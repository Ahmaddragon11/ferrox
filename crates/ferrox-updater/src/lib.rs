// SPDX-License-Identifier: MIT OR Apache-2.0

pub mod installer;
pub mod manifest;
pub mod verify;

pub use installer::{DownloadedUpdate, apply_update, download_and_verify, relaunch};
pub use manifest::{ReleaseAsset, UpdateManifest};
pub use verify::{sha256_hex, signing_payload, verify_sha256, verify_signature};

use ferrox_core::{Result, UpdateChannel};

/// Fetches the update manifest from the configured URL.
pub async fn fetch_manifest(
    client: &reqwest::Client,
    manifest_url: &str,
) -> Result<UpdateManifest> {
    let response = client
        .get(manifest_url)
        .send()
        .await
        .map_err(|e| ferrox_core::FerroxError::Update(format!("manifest fetch failed: {e}")))?;
    if !response.status().is_success() {
        return Err(ferrox_core::FerroxError::Update(format!(
            "manifest fetch returned HTTP {}",
            response.status()
        )));
    }
    let text = response
        .text()
        .await
        .map_err(|e| ferrox_core::FerroxError::Update(format!("manifest body failed: {e}")))?;
    UpdateManifest::parse(&text)
}

/// Full check pipeline: fetch manifest, find a newer version, select the asset.
pub async fn check_for_update(
    client: &reqwest::Client,
    manifest_url: &str,
    current_version: &str,
    channel: UpdateChannel,
) -> Result<Option<(UpdateManifest, ReleaseAsset)>> {
    let manifest = fetch_manifest(client, manifest_url).await?;
    let Some(_new_version) = manifest.available_update(current_version, channel)? else {
        return Ok(None);
    };
    let platform = UpdateManifest::current_platform();
    let asset = manifest
        .asset_for_platform(&platform)
        .cloned()
        .ok_or_else(|| {
            ferrox_core::FerroxError::Update(format!("no release asset for platform {platform}"))
        })?;
    Ok(Some((manifest, asset)))
}
