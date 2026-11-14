// SPDX-License-Identifier: MIT OR Apache-2.0

//! Download manager: keeps the database, the on-disk files and the browser UI
//! in sync while a file is being fetched by the native webview.

use crate::app::{AsyncEvent, FerroxApp};
use ferrox_core::TabId;
use ferrox_storage::{DownloadItem, DownloadStatus};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// A download that is currently being written to disk.
#[derive(Debug, Clone)]
pub(crate) struct ActiveDownload {
    pub id: String,
    pub url: String,
    pub path: PathBuf,
    pub total: u64,
    pub downloaded: u64,
}

/// All download related state of the browser.
#[derive(Debug, Default)]
pub(crate) struct DownloadState {
    /// Maps a `(tab, url)` pair to the identifier of the matching download.
    pub ids: std::collections::HashMap<(TabId, String), String>,
    /// Downloads that are still in flight.
    pub active: std::collections::HashMap<String, ActiveDownload>,
    /// Last time the file sizes were polled.
    pub last_poll: Option<Instant>,
}

impl FerroxApp {
    /// A download was accepted by the webview.
    pub(crate) fn on_download_started(&mut self, tab_id: TabId, url: String, path: String) {
        let path = PathBuf::from(path);
        let mut item = DownloadItem::new(url.clone(), path.to_string_lossy().to_string());
        item.status = DownloadStatus::InProgress;
        let id = item.id.clone();

        let db = self.active_db();
        if let Err(err) = db.insert_download(&item) {
            tracing::warn!("could not store download record: {err}");
        }

        self.downloads.ids.insert((tab_id, url.clone()), id.clone());
        let file_name = file_name_of(&path);
        self.downloads.active.insert(
            id.clone(),
            ActiveDownload {
                id: id.clone(),
                url: url.clone(),
                path,
                total: 0,
                downloaded: 0,
            },
        );

        // Ask the server for the expected size so the progress bar is meaningful.
        let client = self.net.clone();
        let tx = self.async_tx.clone();
        let target = url.clone();
        self.runtime.spawn(async move {
            let total = client.content_length(&target).await;
            tx.send(AsyncEvent::DownloadSize { id, total });
        });

        self.status(format!("⬇ Downloading {file_name}"));
        self.request_repaint();
    }

    /// A download finished, successfully or not.
    pub(crate) fn on_download_completed(
        &mut self,
        tab_id: TabId,
        url: String,
        path: Option<String>,
        success: bool,
    ) {
        let id = self
            .downloads
            .ids
            .remove(&(tab_id, url.clone()))
            .or_else(|| {
                self.downloads
                    .active
                    .iter()
                    .find(|(_, download)| download.url == url)
                    .map(|(id, _)| id.clone())
            })
            .or_else(|| {
                path.as_ref().and_then(|p| {
                    self.downloads
                        .active
                        .iter()
                        .find(|(_, download)| download.path.to_string_lossy() == *p)
                        .map(|(id, _)| id.clone())
                })
            });

        let Some(id) = id else {
            tracing::debug!("download completed without a matching record: {url}");
            return;
        };

        let download = self.downloads.active.remove(&id);
        let status = if success {
            DownloadStatus::Completed
        } else {
            DownloadStatus::Failed
        };

        let db = self.active_db();
        if let Some(download) = &download {
            let size = std::fs::metadata(&download.path)
                .map(|meta| meta.len())
                .unwrap_or(download.downloaded);
            let _ = db.update_download_progress(&id, size, size);
        }
        let _ = db.set_download_status(&id, status, None);

        if let Some(download) = download {
            if success {
                self.hash_download(&id, &download.path);
                self.status(format!("⬇ Downloaded {}", file_name_of(&download.path)));
            } else {
                self.status(format!(
                    "⬇ Download failed: {}",
                    file_name_of(&download.path)
                ));
            }
        }

        self.refresh_internal_page_if("ferrox://downloads");
        self.push_downloads_to_page();
        self.request_repaint();
    }

    /// The expected size of a download arrived.
    pub(crate) fn on_download_size(&mut self, id: String, total: Option<u64>) {
        let Some(total) = total else {
            return;
        };
        if let Some(download) = self.downloads.active.get_mut(&id) {
            download.total = total;
        }
        let db = self.active_db();
        let _ = db.set_download_total(&id, total);
    }

    /// A freshly computed checksum arrived.
    pub(crate) fn on_download_hash(&mut self, id: String, sha256: String) {
        let db = self.active_db();
        let _ = db.set_download_hash(&id, &sha256);
        self.push_downloads_to_page();
    }

    /// Polls the size of every running download and refreshes the UI.
    pub(crate) fn poll_download_progress(&mut self) {
        if self.downloads.active.is_empty() {
            return;
        }
        self.downloads.last_poll = Some(Instant::now());

        let mut progress = Vec::new();
        let downloads: Vec<ActiveDownload> = self.downloads.active.values().cloned().collect();
        for download in &downloads {
            let size = std::fs::metadata(&download.path)
                .map(|meta| meta.len())
                .unwrap_or(download.downloaded);
            if size != download.downloaded {
                if let Some(entry) = self.downloads.active.get_mut(&download.id) {
                    entry.downloaded = size;
                }
                let db = self.active_db();
                let _ = db.update_download_progress(&download.id, size, download.total);
            }
            progress.push((
                download.id.clone(),
                download.total,
                size,
                file_name_of(&download.path),
            ));
        }

        if let Some((_, _, _, name)) = progress.first() {
            let active = self.downloads.active.len();
            self.status(format!("⬇ {name} ({active} running)"));
        }

        self.push_downloads_to_page();
        self.request_repaint();
    }

    /// Pushes the current download list into the downloads page.
    pub(crate) fn push_downloads_to_page(&mut self) {
        let Some(tab) = self.tabs.active_tab() else {
            return;
        };
        if !tab.url.starts_with("ferrox://downloads") {
            return;
        }
        let items = self.active_db().list_downloads().unwrap_or_default();
        let list: Vec<serde_json::Value> = items
            .iter()
            .map(|item| {
                serde_json::json!({
                    "id": item.id,
                    "url": item.url,
                    "name": file_name_of(Path::new(&item.file_path)),
                    "downloaded": item.downloaded_bytes,
                    "total": item.total_bytes,
                    "status": ferrox_engine::internal_pages::download_status_label(item),
                })
            })
            .collect();
        let json = serde_json::to_string(&list).unwrap_or_else(|_| "[]".into());
        let js = format!("window.__ferroxSetDownloads && window.__ferroxSetDownloads({json});");
        let _ = self.engine.evaluate(tab.id, &js);
    }

    /// Removes a download record. The downloaded file itself is kept on disk.
    pub(crate) fn remove_download(&mut self, id: &str) {
        let db = self.active_db();
        let _ = db.remove_download(id);
        self.downloads.active.remove(id);
        self.downloads.ids.retain(|_, value| value != id);
        self.refresh_internal_page_if("ferrox://downloads");
        self.push_downloads_to_page();
    }

    /// Restarts a failed download by opening its URL again.
    pub(crate) fn retry_download(&mut self, id: &str) {
        let db = self.active_db();
        let Ok(items) = db.list_downloads() else {
            return;
        };
        let Some(item) = items.iter().find(|item| item.id == id) else {
            return;
        };
        let url = item.url.clone();
        self.remove_download(id);
        if let Some(tab_id) = self.tabs.active_tab().map(|t| t.id) {
            self.navigate_url(tab_id, url);
        }
        self.status("⬇ Download restarted");
    }

    /// Opens the folder that contains a download.
    pub(crate) fn reveal_download(&mut self, id: &str) {
        let db = self.active_db();
        let Ok(items) = db.list_downloads() else {
            return;
        };
        let Some(item) = items.iter().find(|item| item.id == id) else {
            return;
        };
        let path = PathBuf::from(&item.file_path);
        let folder = path.parent().map(Path::to_path_buf).unwrap_or(path);
        crate::app::open_path(&folder.to_string_lossy());
    }

    /// Computes the SHA-256 checksum of a finished download in the background.
    fn hash_download(&self, id: &str, path: &Path) {
        let path = path.to_path_buf();
        let id = id.to_string();
        let tx = self.async_tx.clone();
        self.runtime.spawn(async move {
            let digest = hash_file(&path).await;
            tx.send(AsyncEvent::DownloadHash { id, sha256: digest });
        });
    }

    /// Interval used to poll running downloads.
    pub(crate) fn download_poll_interval(&self) -> Duration {
        if self.downloads.active.is_empty() {
            Duration::from_secs(3600)
        } else {
            Duration::from_millis(500)
        }
    }
}

/// Computes the SHA-256 digest of a file without blocking the UI thread.
async fn hash_file(path: &Path) -> String {
    use sha2::{Digest, Sha256};
    use std::io::Read;

    let path = path.to_path_buf();
    match tokio::task::spawn_blocking(move || {
        let mut file = std::fs::File::open(&path)?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 64 * 1024];
        loop {
            let read = file.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        Ok::<String, std::io::Error>(hex::encode(hasher.finalize()))
    })
    .await
    {
        Ok(Ok(digest)) => digest,
        _ => String::new(),
    }
}

/// Extracts the file name of a path for status messages.
fn file_name_of(path: &std::path::Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string())
}
