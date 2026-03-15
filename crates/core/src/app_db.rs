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
}
