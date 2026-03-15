//! SQLite-backed page store for browser extension captures.
//!
//! Pages are stored with zstd-compressed HTML. Indexing progress is tracked
//! via a watermark in the separate `app.db` (see [`crate::app_db`]).

use rusqlite::{params, Connection};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PageStoreError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("compression error: {0}")]
    Compression(String),
}

pub struct StoredPage {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub html: String,
    pub domain: String,
    pub site_group: String,
    pub captured_at: i64,
}

pub struct PageStore {
    db: Connection,
}

impl PageStore {
    pub fn open(path: &Path) -> Result<Self, PageStoreError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let db = Connection::open(path)?;
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;")?;
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS pages (
                id            INTEGER PRIMARY KEY,
                url           TEXT NOT NULL,
                title         TEXT NOT NULL DEFAULT '',
                html          BLOB NOT NULL,
                domain        TEXT NOT NULL DEFAULT '',
                site_group    TEXT NOT NULL DEFAULT '',
                captured_at   INTEGER NOT NULL,
                content_hash  BLOB NOT NULL
            );",
        )?;
        // Migration: add site_group column to existing databases
        let has_site_group: bool = db
            .prepare("SELECT COUNT(*) FROM pragma_table_info('pages') WHERE name='site_group'")?
            .query_row([], |row| row.get::<_, i64>(0))
            .map(|c| c > 0)?;
        if !has_site_group {
            db.execute_batch("ALTER TABLE pages ADD COLUMN site_group TEXT NOT NULL DEFAULT ''")?;
        }
        Ok(Self { db })
    }

    pub fn insert(
        &self,
        url: &str,
        title: &str,
        html: &[u8],
        domain: &str,
        site_group: &str,
        captured_at: i64,
        content_hash: &[u8; 16],
    ) -> Result<(), PageStoreError> {
        let compressed =
            zstd::encode_all(html, 3).map_err(|e| PageStoreError::Compression(e.to_string()))?;
        self.db.execute(
            "INSERT INTO pages (url, title, html, domain, site_group, captured_at, content_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![url, title, compressed, domain, site_group, captured_at, &content_hash[..]],
        )?;
        Ok(())
    }

    /// Load content hashes from the most recent `limit` rows, oldest first.
    pub fn recent_content_hashes(&self, limit: usize) -> Result<Vec<[u8; 16]>, PageStoreError> {
        let mut stmt = self.db.prepare(
            "SELECT content_hash FROM (
                SELECT content_hash, id FROM pages ORDER BY id DESC LIMIT ?1
            ) ORDER BY id",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            let blob: Vec<u8> = row.get(0)?;
            Ok(blob)
        })?;
        let mut hashes = Vec::new();
        for row in rows {
            let blob = row?;
            if let Ok(arr) = <[u8; 16]>::try_from(blob.as_slice()) {
                hashes.push(arr);
            }
        }
        Ok(hashes)
    }

    pub fn pages_after(
        &self,
        watermark: i64,
        limit: usize,
    ) -> Result<Vec<StoredPage>, PageStoreError> {
        let mut stmt = self.db.prepare(
            "SELECT id, url, title, html, domain, site_group, captured_at
             FROM pages WHERE id > ?1
             ORDER BY id
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![watermark, limit as i64], |row| {
            let compressed: Vec<u8> = row.get(3)?;
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                compressed,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, i64>(6)?,
            ))
        })?;

        let mut pages = Vec::new();
        for row in rows {
            let (id, url, title, compressed, domain, site_group, captured_at) = row?;
            let html = zstd::decode_all(compressed.as_slice())
                .map_err(|e| PageStoreError::Compression(e.to_string()))?;
            let html = String::from_utf8_lossy(&html).into_owned();
            pages.push(StoredPage {
                id,
                url,
                title,
                html,
                domain,
                site_group,
                captured_at,
            });
        }
        Ok(pages)
    }

    /// Returns (count, oldest_captured_at) for pages after the watermark, or (0, None) if none.
    pub fn pages_after_count(
        &self,
        watermark: i64,
    ) -> Result<(u64, Option<i64>), PageStoreError> {
        let mut stmt = self.db.prepare(
            "SELECT COUNT(*), MIN(captured_at) FROM pages WHERE id > ?1",
        )?;
        let (count, oldest): (i64, Option<i64>) =
            stmt.query_row(params![watermark], |row| Ok((row.get(0)?, row.get(1)?)))?;
        Ok((count as u64, oldest))
    }

    /// Look up the raw HTML for a page by its rowid.
    pub fn get_html_by_id(&self, id: i64) -> Result<Option<String>, PageStoreError> {
        let mut stmt = self
            .db
            .prepare("SELECT html FROM pages WHERE id = ?1")?;
        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => {
                let compressed: Vec<u8> = row.get(0)?;
                let html = zstd::decode_all(compressed.as_slice())
                    .map_err(|e| PageStoreError::Compression(e.to_string()))?;
                Ok(Some(String::from_utf8_lossy(&html).into_owned()))
            }
            None => Ok(None),
        }
    }

    pub fn delete_all(&self) -> Result<(), PageStoreError> {
        self.db.execute_batch("DELETE FROM pages")?;
        Ok(())
    }

    /// Returns (date_string, count) pairs for each day with captured pages.
    pub fn daily_page_counts(&self) -> Result<Vec<(String, i64)>, PageStoreError> {
        let mut stmt = self.db.prepare(
            "SELECT date(captured_at, 'unixepoch', 'localtime') as day, COUNT(*)
             FROM pages GROUP BY day ORDER BY day",
        )?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Returns (date_string, total_compressed_bytes) pairs for each day.
    pub fn daily_byte_counts(&self) -> Result<Vec<(String, i64)>, PageStoreError> {
        let mut stmt = self.db.prepare(
            "SELECT date(captured_at, 'unixepoch', 'localtime') as day, SUM(length(html))
             FROM pages GROUP BY day ORDER BY day",
        )?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get::<_, i64>(1)?)))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Returns (day_of_week, hour, visits, distinct_domains, bytes) for each (dow, hour) bucket.
    pub fn day_hour_breakdown(&self) -> Result<Vec<(i32, i32, i64, i64, i64)>, PageStoreError> {
        let mut stmt = self.db.prepare(
            "SELECT CAST(strftime('%w', captured_at, 'unixepoch', 'localtime') AS INTEGER),
                    CAST(strftime('%H', captured_at, 'unixepoch', 'localtime') AS INTEGER),
                    COUNT(*),
                    COUNT(DISTINCT site_group)
             FROM pages GROUP BY 1, 2",
        )?;
        let visit_rows: Vec<(i32, i32, i64, i64)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        let mut stmt2 = self.db.prepare(
            "SELECT CAST(strftime('%w', captured_at, 'unixepoch', 'localtime') AS INTEGER),
                    CAST(strftime('%H', captured_at, 'unixepoch', 'localtime') AS INTEGER),
                    SUM(length(html))
             FROM pages WHERE length(html) > 512 GROUP BY 1, 2",
        )?;
        let byte_rows: std::collections::HashMap<(i32, i32), i64> = stmt2
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get::<_, i64>(2)?)))?
            .filter_map(|r| r.ok())
            .map(|(d, h, b)| ((d, h), b))
            .collect();

        Ok(visit_rows
            .into_iter()
            .map(|(d, h, v, dd)| {
                let bytes = byte_rows.get(&(d, h)).copied().unwrap_or(0);
                (d, h, v, dd, bytes)
            })
            .collect())
    }

    /// Returns (site_group, visit_count, total_compressed_bytes) for top site groups.
    pub fn top_domains(&self, limit: i64) -> Result<Vec<(String, i64, i64)>, PageStoreError> {
        let mut stmt = self.db.prepare(
            "SELECT site_group, COUNT(*), SUM(length(html))
             FROM pages GROUP BY site_group ORDER BY COUNT(*) DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get::<_, i64>(2)?))
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Returns (total, today, this_week, this_month, this_year).
    pub fn summary_counts(&self) -> Result<(i64, i64, i64, i64, i64), PageStoreError> {
        let mut stmt = self.db.prepare(
            "SELECT COUNT(*),
               SUM(CASE WHEN date(captured_at,'unixepoch','localtime') = date('now','localtime') THEN 1 ELSE 0 END),
               SUM(CASE WHEN captured_at >= strftime('%s', date('now','localtime','weekday 0','-6 days'), 'utc') THEN 1 ELSE 0 END),
               SUM(CASE WHEN date(captured_at,'unixepoch','localtime') >= date('now','start of month','localtime') THEN 1 ELSE 0 END),
               SUM(CASE WHEN date(captured_at,'unixepoch','localtime') >= date('now','start of year','localtime') THEN 1 ELSE 0 END)
             FROM pages",
        )?;
        stmt.query_row([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<i64>>(1)?.unwrap_or(0),
                row.get::<_, Option<i64>>(2)?.unwrap_or(0),
                row.get::<_, Option<i64>>(3)?.unwrap_or(0),
                row.get::<_, Option<i64>>(4)?.unwrap_or(0),
            ))
        })
        .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_db() -> (PathBuf, PageStore) {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "alexandria-test-{}-{}",
            std::process::id(),
            id
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.db");
        let store = PageStore::open(&path).unwrap();
        (path, store)
    }

    fn hash(data: &[u8]) -> [u8; 16] {
        xxhash_rust::xxh3::xxh3_128(data).to_le_bytes()
    }

    #[test]
    fn test_insert_and_pages_after() {
        let (_path, store) = temp_db();
        let html = b"<html><body>Hello</body></html>";
        store
            .insert("https://example.com", "Example", html, "example.com", "example.com", 1000, &hash(html))
            .unwrap();

        let pages = store.pages_after(0, 10).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].url, "https://example.com");
        assert_eq!(pages[0].title, "Example");
        assert_eq!(pages[0].html, "<html><body>Hello</body></html>");
        assert!(pages[0].id > 0);
    }

    #[test]
    fn test_watermark_filtering() {
        let (_path, store) = temp_db();
        store.insert("https://a.com", "A", b"a", "a.com", "a.com", 1000, &hash(b"a")).unwrap();
        store.insert("https://b.com", "B", b"b", "b.com", "b.com", 2000, &hash(b"b")).unwrap();
        store.insert("https://c.com", "C", b"c", "c.com", "c.com", 3000, &hash(b"c")).unwrap();

        let all = store.pages_after(0, 10).unwrap();
        assert_eq!(all.len(), 3);

        let after_first = store.pages_after(all[0].id, 10).unwrap();
        assert_eq!(after_first.len(), 2);
        assert_eq!(after_first[0].url, "https://b.com");
    }

    #[test]
    fn test_pages_after_count() {
        let (_path, store) = temp_db();
        store.insert("https://a.com", "A", b"a", "a.com", "a.com", 1000, &hash(b"a")).unwrap();
        store.insert("https://b.com", "B", b"b", "b.com", "b.com", 2000, &hash(b"b")).unwrap();

        let (count, oldest) = store.pages_after_count(0).unwrap();
        assert_eq!(count, 2);
        assert_eq!(oldest, Some(1000));

        let all = store.pages_after(0, 10).unwrap();
        let (count, _) = store.pages_after_count(all[1].id).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_get_html_by_id() {
        let (_path, store) = temp_db();
        store
            .insert("https://example.com", "Ex", b"<p>hi</p>", "example.com", "example.com", 1000, &hash(b"<p>hi</p>"))
            .unwrap();

        let pages = store.pages_after(0, 10).unwrap();
        let html = store.get_html_by_id(pages[0].id).unwrap();
        assert_eq!(html, Some("<p>hi</p>".to_string()));

        let missing = store.get_html_by_id(99999).unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn test_zstd_roundtrip() {
        let (_path, store) = temp_db();
        let html = "<html>".repeat(10000);
        store
            .insert("https://big.com", "Big", html.as_bytes(), "big.com", "big.com", 1000, &hash(html.as_bytes()))
            .unwrap();
        let pages = store.pages_after(0, 10).unwrap();
        assert_eq!(pages[0].html, html);
    }

    #[test]
    fn test_recent_content_hashes() {
        let (_path, store) = temp_db();
        let h1 = hash(b"a");
        let h2 = hash(b"b");
        let h3 = hash(b"c");
        store.insert("https://a.com", "A", b"a", "a.com", "a.com", 1000, &h1).unwrap();
        store.insert("https://b.com", "B", b"b", "b.com", "b.com", 2000, &h2).unwrap();
        store.insert("https://c.com", "C", b"c", "c.com", "c.com", 3000, &h3).unwrap();

        // Fetch last 2 — should be b and c in insertion order
        let hashes = store.recent_content_hashes(2).unwrap();
        assert_eq!(hashes.len(), 2);
        assert_eq!(hashes[0], h2);
        assert_eq!(hashes[1], h3);

        // Fetch all
        let hashes = store.recent_content_hashes(10).unwrap();
        assert_eq!(hashes.len(), 3);
        assert_eq!(hashes[0], h1);
    }

    #[test]
    fn test_daily_page_counts() {
        let (_path, store) = temp_db();
        // Two pages on same day, one on a different day
        store.insert("https://a.com", "A", b"a", "a.com", "a.com", 1700000000, &hash(b"a")).unwrap();
        store.insert("https://b.com", "B", b"b", "b.com", "b.com", 1700000100, &hash(b"b")).unwrap();
        store.insert("https://c.com", "C", b"c", "c.com", "c.com", 1700100000, &hash(b"c")).unwrap();

        let counts = store.daily_page_counts().unwrap();
        assert!(counts.len() >= 1);
        let total: i64 = counts.iter().map(|(_, c)| c).sum();
        assert_eq!(total, 3);
    }

    #[test]
    fn test_day_hour_breakdown() {
        let (_path, store) = temp_db();
        store.insert("https://a.com", "A", b"a", "a.com", "a.com", 1700000000, &hash(b"a")).unwrap();
        store.insert("https://b.com", "B", b"b", "b.com", "b.com", 1700000100, &hash(b"b")).unwrap();

        let breakdown = store.day_hour_breakdown().unwrap();
        assert!(!breakdown.is_empty());
        let total_visits: i64 = breakdown.iter().map(|(_, _, v, _, _)| v).sum();
        assert_eq!(total_visits, 2);
    }

    #[test]
    fn test_top_domains() {
        let (_path, store) = temp_db();
        store.insert("https://a.com/1", "A1", b"a1", "a.com", "a.com", 1000, &hash(b"a1")).unwrap();
        store.insert("https://a.com/2", "A2", b"a2", "a.com", "a.com", 2000, &hash(b"a2")).unwrap();
        store.insert("https://b.com/1", "B1", b"b1", "b.com", "b.com", 3000, &hash(b"b1")).unwrap();

        let top = store.top_domains(10).unwrap();
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0, "a.com");
        assert_eq!(top[0].1, 2);
        assert_eq!(top[1].0, "b.com");
        assert_eq!(top[1].1, 1);
    }

    #[test]
    fn test_summary_counts() {
        let (_path, store) = temp_db();
        store.insert("https://a.com", "A", b"a", "a.com", "a.com", 1000, &hash(b"a")).unwrap();

        let (total, today, this_week, this_month, this_year) = store.summary_counts().unwrap();
        assert_eq!(total, 1);
        // The old timestamp won't be today/this week/etc
        assert_eq!(today, 0);
        assert_eq!(this_week, 0);
        assert_eq!(this_month, 0);
        assert_eq!(this_year, 0);
    }

    #[test]
    fn test_summary_counts_empty() {
        let (_path, store) = temp_db();
        let (total, today, this_week, this_month, this_year) = store.summary_counts().unwrap();
        assert_eq!(total, 0);
        assert_eq!(today, 0);
        assert_eq!(this_week, 0);
        assert_eq!(this_month, 0);
        assert_eq!(this_year, 0);
    }
}
