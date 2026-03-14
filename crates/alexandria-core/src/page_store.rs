// SQLite-backed page store for browser extension captures

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
    pub source_hash: String,
    pub url: String,
    pub title: String,
    pub html: String,
    pub domain: String,
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
                source_hash TEXT PRIMARY KEY,
                url         TEXT NOT NULL,
                title       TEXT NOT NULL DEFAULT '',
                html        BLOB NOT NULL,
                domain      TEXT NOT NULL DEFAULT '',
                captured_at INTEGER NOT NULL,
                indexed_at  INTEGER
            );",
        )?;
        Ok(Self { db })
    }

    pub fn upsert(
        &self,
        url: &str,
        title: &str,
        html: &[u8],
        domain: &str,
        source_hash: &str,
        captured_at: i64,
    ) -> Result<(), PageStoreError> {
        let compressed =
            zstd::encode_all(html, 3).map_err(|e| PageStoreError::Compression(e.to_string()))?;
        self.db.execute(
            "INSERT OR REPLACE INTO pages (source_hash, url, title, html, domain, captured_at, indexed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL)",
            params![source_hash, url, title, compressed, domain, captured_at],
        )?;
        Ok(())
    }

    pub fn pending(&self, limit: usize) -> Result<Vec<StoredPage>, PageStoreError> {
        let mut stmt = self.db.prepare(
            "SELECT source_hash, url, title, html, domain, captured_at
             FROM pages WHERE indexed_at IS NULL
             ORDER BY captured_at
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            let compressed: Vec<u8> = row.get(3)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                compressed,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
            ))
        })?;

        let mut pages = Vec::new();
        for row in rows {
            let (source_hash, url, title, compressed, domain, captured_at) = row?;
            let html = zstd::decode_all(compressed.as_slice())
                .map_err(|e| PageStoreError::Compression(e.to_string()))?;
            let html = String::from_utf8_lossy(&html).into_owned();
            pages.push(StoredPage {
                source_hash,
                url,
                title,
                html,
                domain,
                captured_at,
            });
        }
        Ok(pages)
    }

    pub fn mark_indexed(&self, source_hash: &str) -> Result<(), PageStoreError> {
        let now = chrono::Utc::now().timestamp();
        self.db.execute(
            "UPDATE pages SET indexed_at = ?1 WHERE source_hash = ?2",
            params![now, source_hash],
        )?;
        Ok(())
    }

    pub fn mark_indexed_batch(&self, hashes: &[&str]) -> Result<(), PageStoreError> {
        let now = chrono::Utc::now().timestamp();
        let tx = self.db.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare(
                "UPDATE pages SET indexed_at = ?1 WHERE source_hash = ?2",
            )?;
            for hash in hashes {
                stmt.execute(params![now, hash])?;
            }
        }
        tx.commit()?;
        Ok(())
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

    #[test]
    fn test_upsert_and_pending() {
        let (_path, store) = temp_db();
        store
            .upsert(
                "https://example.com",
                "Example",
                b"<html><body>Hello</body></html>",
                "example.com",
                "abc123",
                1000,
            )
            .unwrap();

        let pending = store.pending(10).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].url, "https://example.com");
        assert_eq!(pending[0].title, "Example");
        assert_eq!(pending[0].html, "<html><body>Hello</body></html>");
        assert_eq!(pending[0].source_hash, "abc123");
    }

    #[test]
    fn test_mark_indexed() {
        let (_path, store) = temp_db();
        store
            .upsert("https://example.com", "Ex", b"<p>hi</p>", "example.com", "h1", 1000)
            .unwrap();
        store.mark_indexed("h1").unwrap();

        let pending = store.pending(10).unwrap();
        assert!(pending.is_empty());
    }

    #[test]
    fn test_upsert_resets_indexed_at() {
        let (_path, store) = temp_db();
        store
            .upsert("https://example.com", "Ex", b"<p>v1</p>", "example.com", "h1", 1000)
            .unwrap();
        store.mark_indexed("h1").unwrap();
        assert!(store.pending(10).unwrap().is_empty());

        // Re-upsert same hash resets indexed_at to NULL
        store
            .upsert("https://example.com", "Ex v2", b"<p>v2</p>", "example.com", "h1", 2000)
            .unwrap();
        let pending = store.pending(10).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].title, "Ex v2");
    }

    #[test]
    fn test_mark_indexed_batch() {
        let (_path, store) = temp_db();
        store.upsert("https://a.com", "A", b"a", "a.com", "h1", 1000).unwrap();
        store.upsert("https://b.com", "B", b"b", "b.com", "h2", 2000).unwrap();
        store.upsert("https://c.com", "C", b"c", "c.com", "h3", 3000).unwrap();

        store.mark_indexed_batch(&["h1", "h3"]).unwrap();
        let pending = store.pending(10).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].source_hash, "h2");
    }

    #[test]
    fn test_zstd_roundtrip() {
        let (_path, store) = temp_db();
        let html = "<html>".repeat(10000);
        store
            .upsert("https://big.com", "Big", html.as_bytes(), "big.com", "big", 1000)
            .unwrap();
        let pending = store.pending(10).unwrap();
        assert_eq!(pending[0].html, html);
    }
}
