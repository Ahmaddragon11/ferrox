// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::db::{StorageDb, map_err};
use ferrox_core::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTab {
    pub tab_id: String,
    pub url: String,
    pub title: String,
    pub position: usize,
    pub pinned: bool,
    pub muted: bool,
}

impl StorageDb {
    pub fn save_session(&self, tabs: &[SessionTab]) -> Result<()> {
        let mut conn = self.conn();
        let tx = conn.transaction().map_err(map_err)?;
        tx.execute("DELETE FROM session_tabs", [])
            .map_err(map_err)?;
        for tab in tabs {
            tx.execute(
                "INSERT INTO session_tabs (tab_id, url, title, position, pinned, muted)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    tab.tab_id,
                    tab.url,
                    tab.title,
                    tab.position as i64,
                    tab.pinned,
                    tab.muted,
                ],
            )
            .map_err(map_err)?;
        }
        tx.commit().map_err(map_err)?;
        Ok(())
    }

    pub fn load_session(&self) -> Result<Vec<SessionTab>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT tab_id, url, title, position, pinned, muted
                 FROM session_tabs ORDER BY position ASC",
            )
            .map_err(map_err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(SessionTab {
                    tab_id: row.get(0)?,
                    url: row.get(1)?,
                    title: row.get(2)?,
                    position: row.get::<_, i64>(3)? as usize,
                    pinned: row.get(4)?,
                    muted: row.get(5)?,
                })
            })
            .map_err(map_err)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(map_err)?);
        }
        Ok(out)
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT value FROM settings_kv WHERE key = ?1")
            .map_err(map_err)?;
        let mut rows = stmt.query([key]).map_err(map_err)?;
        if let Some(row) = rows.next().map_err(map_err)? {
            Ok(Some(row.get::<_, String>(0).map_err(map_err)?))
        } else {
            Ok(None)
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn()
            .execute(
                "INSERT INTO settings_kv (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                [key, value],
            )
            .map_err(map_err)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_roundtrip_and_settings() {
        let db = StorageDb::open_in_memory().unwrap();
        let tabs = vec![
            SessionTab {
                tab_id: "a".into(),
                url: "https://a.example".into(),
                title: "A".into(),
                position: 0,
                pinned: true,
                muted: false,
            },
            SessionTab {
                tab_id: "b".into(),
                url: "https://b.example".into(),
                title: "B".into(),
                position: 1,
                pinned: false,
                muted: true,
            },
        ];
        db.save_session(&tabs).unwrap();
        assert_eq!(db.load_session().unwrap(), tabs);

        assert_eq!(db.get_setting("theme").unwrap(), None);
        db.set_setting("theme", "dark").unwrap();
        assert_eq!(db.get_setting("theme").unwrap().as_deref(), Some("dark"));
        db.set_setting("theme", "light").unwrap();
        assert_eq!(db.get_setting("theme").unwrap().as_deref(), Some("light"));
    }
}
