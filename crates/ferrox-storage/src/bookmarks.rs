// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::db::{StorageDb, fts_query, map_err};
use crate::history::parse_datetime;
use chrono::{DateTime, Utc};
use ferrox_core::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub folder: String,
    pub created_at: DateTime<Utc>,
}

impl StorageDb {
    pub fn add_bookmark(&self, url: &str, title: &str, folder: &str) -> Result<i64> {
        let conn = self.conn();
        conn.execute(
            r#"
            INSERT INTO bookmarks (url, title, folder, created_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(url) DO UPDATE SET
                title = excluded.title,
                folder = excluded.folder
            "#,
            rusqlite::params![url, title, folder, Utc::now().to_rfc3339()],
        )
        .map_err(map_err)?;
        Ok(conn.last_insert_rowid())
    }

    pub fn remove_bookmark(&self, id: i64) -> Result<()> {
        self.conn()
            .execute("DELETE FROM bookmarks WHERE id = ?1", [id])
            .map_err(map_err)?;
        Ok(())
    }

    pub fn remove_bookmark_by_url(&self, url: &str) -> Result<bool> {
        let changed = self
            .conn()
            .execute("DELETE FROM bookmarks WHERE url = ?1", [url])
            .map_err(map_err)?;
        Ok(changed > 0)
    }

    pub fn is_bookmarked(&self, url: &str) -> Result<bool> {
        let count: i64 = self
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM bookmarks WHERE url = ?1",
                [url],
                |row| row.get(0),
            )
            .map_err(map_err)?;
        Ok(count > 0)
    }

    pub fn list_bookmarks(&self, folder: Option<&str>) -> Result<Vec<Bookmark>> {
        let conn = self.conn();
        let mut out = Vec::new();
        match folder {
            Some(folder) => {
                let mut stmt = conn
                    .prepare(
                        "SELECT id, url, title, folder, created_at FROM bookmarks
                         WHERE folder = ?1 ORDER BY created_at DESC",
                    )
                    .map_err(map_err)?;
                let rows = stmt
                    .query_map([folder], |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, String>(4)?,
                        ))
                    })
                    .map_err(map_err)?;
                for row in rows {
                    let (id, url, title, folder, created_at) = row.map_err(map_err)?;
                    out.push(Bookmark {
                        id,
                        url,
                        title,
                        folder,
                        created_at: parse_datetime(&created_at),
                    });
                }
            }
            None => {
                let mut stmt = conn
                    .prepare(
                        "SELECT id, url, title, folder, created_at FROM bookmarks
                         ORDER BY created_at DESC",
                    )
                    .map_err(map_err)?;
                let rows = stmt
                    .query_map([], |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, String>(4)?,
                        ))
                    })
                    .map_err(map_err)?;
                for row in rows {
                    let (id, url, title, folder, created_at) = row.map_err(map_err)?;
                    out.push(Bookmark {
                        id,
                        url,
                        title,
                        folder,
                        created_at: parse_datetime(&created_at),
                    });
                }
            }
        }
        Ok(out)
    }

    pub fn search_bookmarks(&self, query: &str, limit: usize) -> Result<Vec<Bookmark>> {
        let Some(match_query) = fts_query(query) else {
            return Ok(Vec::new());
        };
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT b.id, b.url, b.title, b.folder, b.created_at
                 FROM bookmarks_fts f JOIN bookmarks b ON b.id = f.rowid
                 WHERE bookmarks_fts MATCH ?1
                 ORDER BY b.created_at DESC LIMIT ?2",
            )
            .map_err(map_err)?;
        let rows = stmt
            .query_map(rusqlite::params![match_query, limit as i64], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .map_err(map_err)?;
        let mut out = Vec::new();
        for row in rows {
            let (id, url, title, folder, created_at) = row.map_err(map_err)?;
            out.push(Bookmark {
                id,
                url,
                title,
                folder,
                created_at: parse_datetime(&created_at),
            });
        }
        Ok(out)
    }

    pub fn export_bookmarks_html(&self) -> Result<String> {
        let bookmarks = self.list_bookmarks(None)?;
        let mut html = String::from(
            "<!DOCTYPE NETSCAPE-Bookmark-file-1>\n<META HTTP-EQUIV=\"Content-Type\" CONTENT=\"text/html; charset=UTF-8\">\n<TITLE>Bookmarks</TITLE>\n<H1>Ferrox Bookmarks</H1>\n<DL><p>\n",
        );
        for bm in bookmarks {
            let title = bm.title.replace('<', "&lt;").replace('>', "&gt;");
            html.push_str(&format!(
                "    <DT><A HREF=\"{}\" ADD_DATE=\"{}\">{}</A>\n",
                bm.url,
                bm.created_at.timestamp(),
                title
            ));
        }
        html.push_str("</DL><p>\n");
        Ok(html)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bookmark_crud_and_search() {
        let db = StorageDb::open_in_memory().unwrap();
        db.add_bookmark("https://rust-lang.org", "Rust Language", "Dev")
            .unwrap();
        db.add_bookmark("https://docs.rs", "Docs.rs", "Dev")
            .unwrap();

        assert!(db.is_bookmarked("https://rust-lang.org").unwrap());
        assert_eq!(db.list_bookmarks(Some("Dev")).unwrap().len(), 2);
        assert_eq!(db.search_bookmarks("rust", 10).unwrap().len(), 1);

        db.remove_bookmark_by_url("https://docs.rs").unwrap();
        assert_eq!(db.list_bookmarks(None).unwrap().len(), 1);
        assert!(!db.is_bookmarked("https://docs.rs").unwrap());

        let html = db.export_bookmarks_html().unwrap();
        assert!(html.contains("NETSCAPE-Bookmark-file-1"));
        assert!(html.contains("https://rust-lang.org"));
    }
}
