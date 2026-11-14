// SPDX-License-Identifier: MIT OR Apache-2.0

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ProfileMode {
    #[default]
    Standard,
    Incognito,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileInfo {
    pub name: String,
    pub mode: ProfileMode,
    pub data_dir: Option<PathBuf>,
}

impl ProfileInfo {
    pub fn standard(name: impl Into<String>, data_dir: PathBuf) -> Self {
        Self {
            name: name.into(),
            mode: ProfileMode::Standard,
            data_dir: Some(data_dir),
        }
    }

    pub fn incognito() -> Self {
        Self {
            name: "Incognito".to_string(),
            mode: ProfileMode::Incognito,
            data_dir: None,
        }
    }

    pub fn is_incognito(&self) -> bool {
        self.mode == ProfileMode::Incognito
    }
}
