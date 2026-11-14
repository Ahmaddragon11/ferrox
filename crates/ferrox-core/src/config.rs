// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::error::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SearchEngineId {
    #[default]
    DuckDuckGo,
    Google,
    Brave,
    Startpage,
    Bing,
}

impl SearchEngineId {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::DuckDuckGo => "DuckDuckGo",
            Self::Google => "Google",
            Self::Brave => "Brave Search",
            Self::Startpage => "Startpage",
            Self::Bing => "Bing",
        }
    }

    pub fn search_url(self, query: &str) -> String {
        let encoded = crate::urlenc::encode_query_component(query);
        match self {
            Self::DuckDuckGo => format!("https://duckduckgo.com/?q={encoded}"),
            Self::Google => format!("https://www.google.com/search?q={encoded}"),
            Self::Brave => format!("https://search.brave.com/search?q={encoded}"),
            Self::Startpage => format!("https://www.startpage.com/do/search?query={encoded}"),
            Self::Bing => format!("https://www.bing.com/search?q={encoded}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ComputeMode {
    /// Pure-Rust HTML reader engine with zero JavaScript execution.
    Turbo,
    /// Aggressive throttling of timers, animations, WebGL, and offscreen media.
    Eco,
    /// Smart balance: shields ads/trackers and throttles hidden/background tabs.
    #[default]
    Balanced,
    /// Minimal governor interference for complex web apps.
    Full,
}

impl ComputeMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Turbo => "turbo",
            Self::Eco => "eco",
            Self::Balanced => "balanced",
            Self::Full => "full",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannel {
    #[default]
    Stable,
    Beta,
    Nightly,
}

impl UpdateChannel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Beta => "beta",
            Self::Nightly => "nightly",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FerroxConfig {
    pub homepage: String,
    pub default_search: SearchEngineId,
    pub theme: ThemeMode,
    pub default_compute_mode: ComputeMode,
    pub shield_enabled: bool,
    pub hibernate_after_secs: u64,
    pub max_ram_percent_before_hibernate: f32,
    pub restore_session_on_startup: bool,
    pub update_channel: UpdateChannel,
    pub auto_check_updates: bool,
    pub manifest_url: String,
    pub per_site_compute_modes: HashMap<String, ComputeMode>,
}

impl Default for FerroxConfig {
    fn default() -> Self {
        Self {
            homepage: "ferrox://newtab".to_string(),
            default_search: SearchEngineId::DuckDuckGo,
            theme: ThemeMode::Dark,
            default_compute_mode: ComputeMode::Balanced,
            shield_enabled: true,
            hibernate_after_secs: 300,
            max_ram_percent_before_hibernate: 80.0,
            restore_session_on_startup: true,
            update_channel: UpdateChannel::Stable,
            auto_check_updates: true,
            manifest_url: "https://releases.ferrox.invalid/update-manifest.json".to_string(),
            per_site_compute_modes: HashMap::new(),
        }
    }
}

impl FerroxConfig {
    pub fn project_dirs() -> Option<ProjectDirs> {
        ProjectDirs::from("org", "ferrox", "Ferrox")
    }

    pub fn default_config_path() -> PathBuf {
        Self::project_dirs()
            .map(|d| d.config_dir().join("config.toml"))
            .unwrap_or_else(|| PathBuf::from("ferrox-config.toml"))
    }

    pub fn default_data_dir() -> PathBuf {
        Self::project_dirs()
            .map(|d| d.data_local_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".ferrox-data"))
    }

    pub fn load_or_default(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(content) => toml::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let encoded = toml::to_string_pretty(self)
            .map_err(|e| crate::error::FerroxError::Config(e.to_string()))?;
        std::fs::write(path, encoded)?;
        Ok(())
    }

    pub fn compute_mode_for_url(&self, raw_url: &str) -> ComputeMode {
        if let Ok(parsed) = url::Url::parse(raw_url)
            && let Some(host) = parsed.host_str()
            && let Some(mode) = self.per_site_compute_modes.get(host)
        {
            return *mode;
        }
        self.default_compute_mode
    }
}
