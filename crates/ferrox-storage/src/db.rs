// SPDX-License-Identifier: MIT OR Apache-2.0

use ferrox_core::{FerroxError, Result};
use parking_lot::Mutex;
use rusqlite::Connection;
use std::path::Path;

const SCHEMA: &str = r#"
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA temp_store = MEMORY;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS history (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    url         TEXT NOT NULL,
    title       TEXT NOT NULL DEFAULT '',
    visited_at  TEXT NOT NULL,
    visit_count INTEGER NOT NULL DEFAULT 1
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_history_url ON history(url);
CREATE INDEX IF NOT EXISTS idx_history_visited ON history(visited_at DESC);

CREATE VIRTUAL TABLE IF NOT EXISTS history_fts USING fts5(
    url, title, content='history', content_rowid='id', tokenize='unicode61'
);
CREATE TRIGGER IF NOT EXISTS history_ai AFTER INSERT ON history BEGIN
    INSERT INTO history_fts(rowid, url, title) VALUES (new.id, new.url, new.title);
END;
CREATE TRIGGER IF NOT EXISTS history_ad AFTER DELETE ON history BEGIN
    INSERT INTO history_fts(history_fts, rowid, url, title) VALUES ('delete', old.id, old.url, old.title);
END;
CREATE TRIGGER IF NOT EXISTS history_au AFTER UPDATE ON history BEGIN
    INSERT INTO history_fts(history_fts, rowid, url, title) VALUES ('delete', old.id, old.url, old.title);
    INSERT INTO history_fts(rowid, url, title) VALUES (new.id, new.url, new.title);
END;

CREATE TABLE IF NOT EXISTS bookmarks (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    url        TEXT NOT NULL UNIQUE,
    title      TEXT NOT NULL DEFAULT '',
    folder     TEXT NOT NULL DEFAULT 'Bookmarks',
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_bookmarks_folder ON bookmarks(folder);

CREATE VIRTUAL TABLE IF NOT EXISTS bookmarks_fts USING fts5(
    url, title, folder, content='bookmarks', content_rowid='id', tokenize='unicode61'
);
CREATE TRIGGER IF NOT EXISTS bookmarks_ai AFTER INSERT ON bookmarks BEGIN
    INSERT INTO bookmarks_fts(rowid, url, title, folder) VALUES (new.id, new.url, new.title, new.folder);
END;
CREATE TRIGGER IF NOT EXISTS bookmarks_ad AFTER DELETE ON bookmarks BEGIN
    INSERT INTO bookmarks_fts(bookmarks_fts, rowid, url, title, folder) VALUES ('delete', old.id, old.url, old.title, old.folder);
END;
CREATE TRIGGER IF NOT EXISTS bookmarks_au AFTER UPDATE ON bookmarks BEGIN
    INSERT INTO bookmarks_fts(bookmarks_fts, rowid, url, title, folder) VALUES ('delete', old.id, old.url, old.title, old.folder);
    INSERT INTO bookmarks_fts(rowid, url, title, folder) VALUES (new.id, new.url, new.title, new.folder);
END;

CREATE TABLE IF NOT EXISTS downloads (
    id               TEXT PRIMARY KEY,
    url              TEXT NOT NULL,
    file_path        TEXT NOT NULL,
    total_bytes      INTEGER NOT NULL DEFAULT 0,
    downloaded_bytes INTEGER NOT NULL DEFAULT 0,
    status           TEXT NOT NULL DEFAULT 'pending',
    sha256           TEXT,
    created_at       TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS session_tabs (
    tab_id   TEXT PRIMARY KEY,
    url      TEXT NOT NULL,
    title    TEXT NOT NULL DEFAULT '',
    position INTEGER NOT NULL,
    pinned   INTEGER NOT NULL DEFAULT 0,
    muted    INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS settings_kv (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
"#;

pub struct StorageDb {
    conn: Mutex<Connection>,
}

impl StorageDb {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path).map_err(map_err)?;
        Self::init(conn)
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().map_err(map_err)?;
        Self::init(conn)
    }

    fn init(conn: Connection) -> Result<Self> {
        conn.execute_batch(SCHEMA).map_err(map_err)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn conn(&self) -> parking_lot::MutexGuard<'_, Connection> {
        self.conn.lock()
    }
}

pub(crate) fn map_err(err: rusqlite::Error) -> FerroxError {
    FerroxError::Storage(err.to_string())
}

/// Converts arbitrary user input into a safe FTS5 prefix query.
pub(crate) fn fts_query(raw: &str) -> Option<String> {
    let tokens: Vec<String> = raw
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| format!("\"{}\"*", t.replace('"', "")))
        .collect();
    if tokens.is_empty() {
        None
    } else {
        Some(tokens.join(" AND "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_initializes_with_fts5() {
        let db = StorageDb::open_in_memory().expect("in-memory db");
        let conn = db.conn();
        let version: String = conn
            .query_row(
                "SELECT sqlite_version() || ':' || (SELECT count(*) FROM pragma_compile_options WHERE compile_options LIKE '%FTS5%')",
                [],
                |row| row.get(0),
            )
            .expect("sqlite version");
        assert!(!version.is_empty());
        assert!(version.ends_with(":1"), "FTS5 must be compiled in: {version}");
    }

    #[test]
    fn fts_query_escapes_specials() {
        assert_eq!(
            fts_query("rust browser"),
            Some("\"rust\"* AND \"browser\"*".to_string())
        );
        assert_eq!(fts_query("   "), None);
        assert_eq!(fts_query("a\"b"), Some("\"ab\"*".to_string()));
    }
}
