// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::db::{StorageDb, fts_query, map_err};
use chrono::{DateTime, Utc};
use ferrox_core::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub visited_at: DateTime<Utc>,
    pub visit_count: i64,
}

impl StorageDb {
    /// Inserts a visit or increments the counter of an existing URL.
    pub fn record_visit(&self, url: &str, title: &str) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            r#"
            INSERT INTO history (url, title, visited_at, visit_count)
            VALUES (?1, ?2, ?3, 1)
            ON CONFLICT(url) DO UPDATE SET
                title = CASE WHEN excluded.title <> '' THEN excluded.title ELSE history.title END,
                visited_at = excluded.visited_at,
                visit_count = history.visit_count + 1
            "#,
            rusqlite::params![url, title, Utc::now().to_rfc3339()],
        )
        .map_err(map_err)?;
        Ok(())
    }

    pub fn recent_history(&self, limit: usize) -> Result<Vec<HistoryEntry>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, url, title, visited_at, visit_count
                 FROM history ORDER BY visited_at DESC LIMIT ?1",
            )
            .map_err(map_err)?;
        let rows = stmt
            .query_map([limit as i64], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            })
            .map_err(map_err)?;

        let mut out = Vec::new();
        for row in rows {
            let (id, url, title, visited_at, visit_count) = row.map_err(map_err)?;
            out.push(HistoryEntry {
                id,
                url,
                title,
                visited_at: parse_datetime(&visited_at),
                visit_count,
            });
        }
        Ok(out)
    }

    pub fn search_history(&self, query: &str, limit: usize) -> Result<Vec<HistoryEntry>> {
        let Some(match_query) = fts_query(query) else {
            return Ok(Vec::new());
        };
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT h.id, h.url, h.title, h.visited_at, h.visit_count
                 FROM history_fts f JOIN history h ON h.id = f.rowid
                 WHERE history_fts MATCH ?1
                 ORDER BY h.visited_at DESC LIMIT ?2",
            )
            .map_err(map_err)?;
        let rows = stmt
            .query_map(rusqlite::params![match_query, limit as i64], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            })
            .map_err(map_err)?;

        let mut out = Vec::new();
        for row in rows {
            let (id, url, title, visited_at, visit_count) = row.map_err(map_err)?;
            out.push(HistoryEntry {
                id,
                url,
                title,
                visited_at: parse_datetime(&visited_at),
                visit_count,
            });
        }
        Ok(out)
    }

    pub fn delete_history_entry(&self, id: i64) -> Result<()> {
        self.conn()
            .execute("DELETE FROM history WHERE id = ?1", [id])
            .map_err(map_err)?;
        Ok(())
    }

    pub fn clear_history(&self) -> Result<()> {
        self.conn()
            .execute_batch("DELETE FROM history; INSERT INTO history_fts(history_fts) VALUES('rebuild');")
            .map_err(map_err)?;
        Ok(())
    }

    pub fn history_count(&self) -> Result<usize> {
        let count: i64 = self
            .conn()
            .query_row("SELECT COUNT(*) FROM history", [], |row| row.get(0))
            .map_err(map_err)?;
        Ok(count as usize)
    }

    /// Returns the most frequently visited hosts for the new-tab shortcuts grid.
    pub fn top_sites(&self, limit: usize) -> Result<Vec<(String, String)>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT url, title FROM history
                 GROUP BY substr(url, 1, instr(substr(url, 9), '/') + 8)
                 ORDER BY SUM(visit_count) DESC, MAX(visited_at) DESC LIMIT ?1",
            )
            .map_err(map_err)?;
        let rows = stmt
            .query_map([limit as i64], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(map_err)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(map_err)?);
        }
        Ok(out)
    }
}

pub(crate) fn parse_datetime(raw: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_upserts_and_searches_history() {
        let db = StorageDb::open_in_memory().unwrap();
        db.record_visit("https://rust-lang.org/learn", "Rust Learn").unwrap();
        db.record_visit("https://rust-lang.org/learn", "Rust Learn").unwrap();
        db.record_visit("https://docs.rs/serde", "Serde Docs").unwrap();

        assert_eq!(db.history_count().unwrap(), 2);
        let hits = db.search_history("rust", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].visit_count, 2);

        let hits = db.search_history("serde docs", 10).unwrap();
        assert_eq!(hits.len(), 1);

        let recent = db.recent_history(10).unwrap();
        assert_eq!(recent.len(), 2);

        db.clear_history().unwrap();
        assert_eq!(db.history_count().unwrap(), 0);
        assert!(db.search_history("rust", 10).unwrap().is_empty());
    }
}
