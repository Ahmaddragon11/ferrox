// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::db::{StorageDb, map_err};
use crate::history::parse_datetime;
use chrono::{DateTime, Utc};
use ferrox_core::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DownloadStatus {
    #[default]
    Pending,
    InProgress,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl DownloadStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn from_str(raw: &str) -> Self {
        match raw {
            "pending" => Self::Pending,
            "in_progress" => Self::InProgress,
            "paused" => Self::Paused,
            "completed" => Self::Completed,
            "failed" => Self::Failed,
            "cancelled" => Self::Cancelled,
            _ => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadItem {
    pub id: String,
    pub url: String,
    pub file_path: String,
    pub total_bytes: u64,
    pub downloaded_bytes: u64,
    pub status: DownloadStatus,
    pub sha256: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl DownloadItem {
    pub fn new(url: impl Into<String>, file_path: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            url: url.into(),
            file_path: file_path.into(),
            total_bytes: 0,
            downloaded_bytes: 0,
            status: DownloadStatus::Pending,
            sha256: None,
            created_at: Utc::now(),
        }
    }

    pub fn progress_percent(&self) -> f32 {
        if self.total_bytes == 0 {
            0.0
        } else {
            (self.downloaded_bytes as f32 / self.total_bytes as f32) * 100.0
        }
    }
}

impl StorageDb {
    pub fn insert_download(&self, item: &DownloadItem) -> Result<()> {
        self.conn()
            .execute(
                r#"
                INSERT INTO downloads
                    (id, url, file_path, total_bytes, downloaded_bytes, status, sha256, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                "#,
                rusqlite::params![
                    item.id,
                    item.url,
                    item.file_path,
                    item.total_bytes as i64,
                    item.downloaded_bytes as i64,
                    item.status.as_str(),
                    item.sha256,
                    item.created_at.to_rfc3339(),
                ],
            )
            .map_err(map_err)?;
        Ok(())
    }

    pub fn update_download_progress(&self, id: &str, downloaded: u64, total: u64) -> Result<()> {
        self.conn()
            .execute(
                "UPDATE downloads SET downloaded_bytes = ?2, total_bytes = ?3, status = 'in_progress' WHERE id = ?1",
                rusqlite::params![id, downloaded as i64, total as i64],
            )
            .map_err(map_err)?;
        Ok(())
    }

    pub fn set_download_status(&self, id: &str, status: DownloadStatus, sha256: Option<&str>) -> Result<()> {
        self.conn()
            .execute(
                "UPDATE downloads SET status = ?2, sha256 = COALESCE(?3, sha256) WHERE id = ?1",
                rusqlite::params![id, status.as_str(), sha256],
            )
            .map_err(map_err)?;
        Ok(())
    }

    pub fn list_downloads(&self) -> Result<Vec<DownloadItem>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, url, file_path, total_bytes, downloaded_bytes, status, sha256, created_at
                 FROM downloads ORDER BY created_at DESC",
            )
            .map_err(map_err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(DownloadItem {
                    id: row.get(0)?,
                    url: row.get(1)?,
                    file_path: row.get(2)?,
                    total_bytes: row.get::<_, i64>(3)? as u64,
                    downloaded_bytes: row.get::<_, i64>(4)? as u64,
                    status: DownloadStatus::from_str(&row.get::<_, String>(5)?),
                    sha256: row.get(6)?,
                    created_at: parse_datetime(&row.get::<_, String>(7)?),
                })
            })
            .map_err(map_err)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(map_err)?);
        }
        Ok(out)
    }

    pub fn remove_download(&self, id: &str) -> Result<()> {
        self.conn()
            .execute("DELETE FROM downloads WHERE id = ?1", [id])
            .map_err(map_err)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_lifecycle() {
        let db = StorageDb::open_in_memory().unwrap();
        let mut item = DownloadItem::new("https://example.com/file.zip", "C:/Downloads/file.zip");
        item.total_bytes = 1000;
        db.insert_download(&item).unwrap();

        db.update_download_progress(&item.id, 500, 1000).unwrap();
        let items = db.list_downloads().unwrap();
        assert_eq!(items[0].downloaded_bytes, 500);
        assert_eq!(items[0].status, DownloadStatus::InProgress);
        assert!((items[0].progress_percent() - 50.0).abs() < f32::EPSILON);

        db.set_download_status(&item.id, DownloadStatus::Completed, Some("abc123"))
            .unwrap();
        let items = db.list_downloads().unwrap();
        assert_eq!(items[0].status, DownloadStatus::Completed);
        assert_eq!(items[0].sha256.as_deref(), Some("abc123"));

        db.remove_download(&item.id).unwrap();
        assert!(db.list_downloads().unwrap().is_empty());
    }
}
