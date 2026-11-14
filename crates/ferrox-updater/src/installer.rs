// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::manifest::ReleaseAsset;
use crate::verify::{sha256_hex, verify_sha256, verify_signature};
use ferrox_core::{FerroxError, Result};
use std::path::{Path, PathBuf};

const MAX_UPDATE_BYTES: usize = 256 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct DownloadedUpdate {
    pub path: PathBuf,
    pub sha256: String,
    pub bytes_written: usize,
}

/// Downloads, verifies, and stages an update asset in the given directory.
pub async fn download_and_verify(
    client: &reqwest::Client,
    asset: &ReleaseAsset,
    public_key_hex: &str,
    staging_dir: &Path,
) -> Result<DownloadedUpdate> {
    if asset.signature.trim().is_empty() {
        return Err(FerroxError::Update(
            "missing release signature; refusing to install".into(),
        ));
    }
    // Verify the signature over the advertised hash before downloading.
    verify_signature(public_key_hex, &asset.url, &asset.sha256, &asset.signature)?;

    let response = client
        .get(&asset.url)
        .send()
        .await
        .map_err(|e| FerroxError::Update(format!("download failed: {e}")))?;

    if !response.status().is_success() {
        return Err(FerroxError::Update(format!(
            "download returned HTTP {}",
            response.status()
        )));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| FerroxError::Update(format!("download body failed: {e}")))?;

    if bytes.len() > MAX_UPDATE_BYTES {
        return Err(FerroxError::Update("update asset exceeds size limit".into()));
    }

    verify_sha256(&bytes, &asset.sha256)?;

    std::fs::create_dir_all(staging_dir)?;
    let file_name = asset
        .url
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or("ferrox-update.bin");
    let target = staging_dir.join(file_name);
    std::fs::write(&target, &bytes)?;

    Ok(DownloadedUpdate {
        path: target,
        sha256: sha256_hex(&bytes),
        bytes_written: bytes.len(),
    })
}

/// Atomically replaces the running executable with the staged update.
/// Returns the path of the new executable.
pub fn apply_update(staged_binary: &Path) -> Result<PathBuf> {
    if !staged_binary.is_file() {
        return Err(FerroxError::Update(format!(
            "staged update not found: {}",
            staged_binary.display()
        )));
    }
    self_replace::self_replace(staged_binary)
        .map_err(|e| FerroxError::Update(format!("self-replace failed: {e}")))?;
    std::env::current_exe()
        .map_err(|e| FerroxError::Update(format!("cannot resolve current exe: {e}")))
}

/// Relaunches the freshly updated executable after a short delay.
pub fn relaunch() -> Result<()> {
    let exe = std::env::current_exe()
        .map_err(|e| FerroxError::Update(format!("cannot resolve current exe: {e}")))?;
    std::process::Command::new(exe)
        .spawn()
        .map_err(|e| FerroxError::Update(format!("relaunch failed: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_missing_signature_before_network() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let asset = ReleaseAsset {
            platform: "test".into(),
            url: "https://127.0.0.1:9/none.zip".into(),
            size: 1,
            sha256: "00".into(),
            signature: String::new(),
        };
        let result = runtime.block_on(download_and_verify(
            &reqwest::Client::new(),
            &asset,
            "00",
            Path::new("unused"),
        ));
        assert!(result.is_err());
    }

    #[test]
    fn apply_update_validates_missing_file() {
        let result = apply_update(Path::new("definitely-not-here.bin"));
        assert!(result.is_err());
    }
}
