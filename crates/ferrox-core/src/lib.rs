// SPDX-License-Identifier: MIT OR Apache-2.0

pub mod config;
pub mod engine;
pub mod error;
pub mod profile;
pub mod tab;
pub mod urlenc;

pub use config::{ComputeMode, FerroxConfig, SearchEngineId, ThemeMode, UpdateChannel};
pub use engine::{EngineBackend, EngineKind, EngineMetrics, NavigationTarget};
pub use error::{FerroxError, Result};
pub use profile::{ProfileInfo, ProfileMode};
pub use tab::{TabId, TabInfo, TabManager, TabSnapshot, TabState};
pub use urlenc::{encode_path_segment, encode_query_component};
