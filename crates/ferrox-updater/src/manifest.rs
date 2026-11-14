// SPDX-License-Identifier: MIT OR Apache-2.0

use chrono::{DateTime, Utc};
use ferrox_core::{Result, UpdateChannel};
use semver::Version;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseAsset {
    pub platform: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateManifest {
    pub version: String,
    pub channel: UpdateChannel,
    pub published_at: DateTime<Utc>,
    pub notes_url: String,
    #[serde(default)]
    pub min_supported_version: Option<String>,
    pub assets: Vec<ReleaseAsset>,
}

impl UpdateManifest {
    pub fn parse(raw: &str) -> Result<Self> {
        serde_json::from_str(raw)
            .map_err(|e| ferrox_core::FerroxError::Update(format!("invalid manifest: {e}")))
    }

    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| ferrox_core::FerroxError::Update(e.to_string()))
    }

    pub fn version(&self) -> Result<Version> {
        Version::parse(&self.version)
            .map_err(|e| ferrox_core::FerroxError::Update(format!("invalid version: {e}")))
    }

    pub fn asset_for_platform(&self, platform: &str) -> Option<&ReleaseAsset> {
        self.assets.iter().find(|a| a.platform == platform)
    }

    /// Current platform id used in the manifest, e.g. `windows-x86_64`.
    pub fn current_platform() -> String {
        format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
    }

    /// Returns the update when the manifest offers a newer version on the given channel.
    pub fn available_update(
        &self,
        current_version: &str,
        channel: UpdateChannel,
    ) -> Result<Option<Version>> {
        if self.channel != channel {
            return Ok(None);
        }
        let current = Version::parse(current_version)
            .map_err(|e| ferrox_core::FerroxError::Update(format!("invalid current version: {e}")))?;
        let candidate = self.version()?;
        if candidate > current {
            Ok(Some(candidate))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "version": "1.2.0",
        "channel": "stable",
        "publishedAt": "2026-11-13T00:00:00Z",
        "notesUrl": "https://example.com/notes",
        "assets": [
            {
                "platform": "windows-x86_64",
                "url": "https://example.com/ferrox.zip",
                "size": 10,
                "sha256": "aa",
                "signature": "bb"
            }
        ]
    }"#;

    #[test]
    fn parses_manifest_and_selects_asset() {
        let manifest = UpdateManifest::parse(SAMPLE).unwrap();
        assert_eq!(manifest.version().unwrap(), Version::new(1, 2, 0));
        assert!(manifest.asset_for_platform("windows-x86_64").is_some());
        assert!(manifest.asset_for_platform("linux-x86_64").is_none());
    }

    #[test]
    fn detects_updates_respecting_channels() {
        let manifest = UpdateManifest::parse(SAMPLE).unwrap();
        assert_eq!(
            manifest.available_update("1.1.0", UpdateChannel::Stable).unwrap(),
            Some(Version::new(1, 2, 0))
        );
        assert!(
            manifest
                .available_update("1.2.0", UpdateChannel::Stable)
                .unwrap()
                .is_none()
        );
        assert!(
            manifest
                .available_update("1.1.0", UpdateChannel::Beta)
                .unwrap()
                .is_none()
        );
    }
}
