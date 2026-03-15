//! Application metadata stored in a separate `app.db` SQLite database.
//!
//! Holds the indexing watermark (highest SQLite rowid that has been indexed into
//! Tantivy) and the Tantivy schema revision, keeping this bookkeeping separate
//! from the page data in `pages.db`.

use rusqlite::{params, Connection};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppDbError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

pub struct AppDb {
    db: Connection,
}

impl AppDb {
    pub fn open(path: &Path) -> Result<Self, AppDbError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let db = Connection::open(path)?;
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;")?;
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS meta (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS ingest_log (
                id        INTEGER PRIMARY KEY,
                timestamp TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                page_id   INTEGER NOT NULL,
                url       TEXT NOT NULL,
                domain    TEXT NOT NULL,
                reason    TEXT NOT NULL
            );",
        )?;
        Ok(Self { db })
    }

    pub fn get_watermark(&self) -> Result<i64, AppDbError> {
        let mut stmt = self
            .db
            .prepare("SELECT value FROM meta WHERE key = 'watermark'")?;
        let mut rows = stmt.query([])?;
        match rows.next()? {
            Some(row) => {
                let val: String = row.get(0)?;
                Ok(val.parse::<i64>().unwrap_or(0))
            }
            None => Ok(0),
        }
    }

    pub fn set_watermark(&self, rowid: i64) -> Result<(), AppDbError> {
        self.db.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('watermark', ?1)",
            params![rowid.to_string()],
        )?;
        Ok(())
    }

    pub fn get_schema_revision(&self) -> Result<Option<i64>, AppDbError> {
        let mut stmt = self
            .db
            .prepare("SELECT value FROM meta WHERE key = 'schema_revision'")?;
        let mut rows = stmt.query([])?;
        match rows.next()? {
            Some(row) => {
                let val: String = row.get(0)?;
                Ok(val.parse::<i64>().ok())
            }
            None => Ok(None),
        }
    }

    pub fn set_schema_revision(&self, rev: i64) -> Result<(), AppDbError> {
        self.db.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('schema_revision', ?1)",
            params![rev.to_string()],
        )?;
        Ok(())
    }

    pub fn log_ingest_failure(
        &self,
        page_id: i64,
        url: &str,
        domain: &str,
        reason: &str,
    ) -> Result<(), AppDbError> {
        self.db.execute(
            "INSERT INTO ingest_log (page_id, url, domain, reason) VALUES (?1, ?2, ?3, ?4)",
            params![page_id, url, domain, reason],
        )?;
        Ok(())
    }

    pub fn recent_ingest_failures(&self, limit: u32) -> Result<Vec<LogEntry>, AppDbError> {
        let mut stmt = self.db.prepare(
            "SELECT id, timestamp, page_id, url, domain, reason
             FROM ingest_log ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok(LogEntry {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                page_id: row.get(2)?,
                url: row.get(3)?,
                domain: row.get(4)?,
                reason: row.get(5)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn clear_ingest_log(&self) -> Result<(), AppDbError> {
        self.db.execute("DELETE FROM ingest_log", [])?;
        Ok(())
    }
}

pub struct LogEntry {
    pub id: i64,
    pub timestamp: String,
    pub page_id: i64,
    pub url: String,
    pub domain: String,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_db() -> (PathBuf, AppDb) {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "alexandria-appdb-test-{}-{}",
            std::process::id(),
            id
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("app.db");
        let db = AppDb::open(&path).unwrap();
        (path, db)
    }

    #[test]
    fn test_watermark_default() {
        let (_path, db) = temp_db();
        assert_eq!(db.get_watermark().unwrap(), 0);
    }

    #[test]
    fn test_watermark_roundtrip() {
        let (_path, db) = temp_db();
        db.set_watermark(42).unwrap();
        assert_eq!(db.get_watermark().unwrap(), 42);
        db.set_watermark(100).unwrap();
        assert_eq!(db.get_watermark().unwrap(), 100);
    }

    #[test]
    fn test_schema_revision() {
        let (_path, db) = temp_db();
        assert_eq!(db.get_schema_revision().unwrap(), None);
        db.set_schema_revision(1).unwrap();
        assert_eq!(db.get_schema_revision().unwrap(), Some(1));
    }

    #[test]
    fn test_log_ingest_failure_and_query() {
        let (_path, db) = temp_db();
        db.log_ingest_failure(1, "https://example.com/a", "example.com", "deeply nested HTML").unwrap();
        db.log_ingest_failure(2, "https://example.com/b", "example.com", "stack overflow").unwrap();

        let entries = db.recent_ingest_failures(10).unwrap();
        assert_eq!(entries.len(), 2);
        // Most recent first
        assert_eq!(entries[0].page_id, 2);
        assert_eq!(entries[1].page_id, 1);
        assert_eq!(entries[0].reason, "stack overflow");
        assert_eq!(entries[1].url, "https://example.com/a");
        assert!(!entries[0].timestamp.is_empty());
    }

    #[test]
    fn test_log_ingest_failure_limit() {
        let (_path, db) = temp_db();
        for i in 0..5 {
            db.log_ingest_failure(i, &format!("https://example.com/{i}"), "example.com", "fail").unwrap();
        }
        let entries = db.recent_ingest_failures(3).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].page_id, 4);
    }

    #[test]
    fn test_clear_ingest_log() {
        let (_path, db) = temp_db();
        db.log_ingest_failure(1, "https://example.com", "example.com", "fail").unwrap();
        assert_eq!(db.recent_ingest_failures(10).unwrap().len(), 1);
        db.clear_ingest_log().unwrap();
        assert_eq!(db.recent_ingest_failures(10).unwrap().len(), 0);
    }
}
