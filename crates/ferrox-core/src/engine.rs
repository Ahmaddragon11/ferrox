// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::config::ComputeMode;
use crate::error::Result;
use crate::tab::TabId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EngineKind {
    FullWeb,
    TurboLite,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineMetrics {
    pub kind: EngineKind,
    pub last_load_ms: u64,
    pub dom_nodes_estimate: usize,
    pub scripts_blocked: u64,
    pub bytes_saved: u64,
}

impl Default for EngineMetrics {
    fn default() -> Self {
        Self {
            kind: EngineKind::FullWeb,
            last_load_ms: 0,
            dom_nodes_estimate: 0,
            scripts_blocked: 0,
            bytes_saved: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum NavigationTarget {
    Url(String),
    Html { url: String, html: String },
}

pub trait EngineBackend {
    fn kind(&self) -> EngineKind;
    fn navigate(&mut self, tab_id: TabId, url: &str, mode: ComputeMode) -> Result<()>;
    fn reload(&mut self, tab_id: TabId) -> Result<()>;
    fn go_back(&mut self, tab_id: TabId) -> Result<()>;
    fn go_forward(&mut self, tab_id: TabId) -> Result<()>;
    fn destroy_tab(&mut self, tab_id: TabId) -> Result<()>;
}
