// SPDX-License-Identifier: MIT OR Apache-2.0

use ferrox_core::ComputeMode;
use serde::{Deserialize, Serialize};

/// Per-tab injection policy produced for a given site and compute mode.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GovernorPolicy {
    pub mode: ComputeMode,
    pub throttle_hidden_timers: bool,
    pub min_hidden_interval_ms: u64,
    pub max_background_fps: u32,
    pub defer_offscreen_media: bool,
    pub reduce_motion: bool,
    pub block_autoplay: bool,
    pub disable_webgl: bool,
    pub coalesce_mutations: bool,
}

impl GovernorPolicy {
    pub fn for_mode(mode: ComputeMode) -> Self {
        match mode {
            ComputeMode::Turbo => Self {
                mode,
                throttle_hidden_timers: true,
                min_hidden_interval_ms: 5_000,
                max_background_fps: 1,
                defer_offscreen_media: true,
                reduce_motion: true,
                block_autoplay: true,
                disable_webgl: true,
                coalesce_mutations: true,
            },
            ComputeMode::Eco => Self {
                mode,
                throttle_hidden_timers: true,
                min_hidden_interval_ms: 2_000,
                max_background_fps: 1,
                defer_offscreen_media: true,
                reduce_motion: true,
                block_autoplay: true,
                disable_webgl: true,
                coalesce_mutations: true,
            },
            ComputeMode::Balanced => Self {
                mode,
                throttle_hidden_timers: true,
                min_hidden_interval_ms: 1_000,
                max_background_fps: 2,
                defer_offscreen_media: true,
                reduce_motion: false,
                block_autoplay: true,
                disable_webgl: false,
                coalesce_mutations: true,
            },
            ComputeMode::Full => Self {
                mode,
                throttle_hidden_timers: false,
                min_hidden_interval_ms: 0,
                max_background_fps: 60,
                defer_offscreen_media: false,
                reduce_motion: false,
                block_autoplay: false,
                disable_webgl: false,
                coalesce_mutations: false,
            },
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// Builds the self-contained initialization script injected before page scripts run.
    pub fn injection_script(&self) -> String {
        let policy = self.to_json();
        format!(
            "window.__FERROX_POLICY__ = {policy};\n{}",
            include_str!("../../../assets/scripts/governor.js")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eco_is_stricter_than_full() {
        let eco = GovernorPolicy::for_mode(ComputeMode::Eco);
        let full = GovernorPolicy::for_mode(ComputeMode::Full);
        assert!(eco.disable_webgl);
        assert!(!full.disable_webgl);
        assert!(full.min_hidden_interval_ms < eco.min_hidden_interval_ms);
    }

    #[test]
    fn injection_contains_policy_and_guard() {
        let script = GovernorPolicy::for_mode(ComputeMode::Balanced).injection_script();
        assert!(script.contains("__FERROX_POLICY__"));
        assert!(script.contains("__ferroxGovernorActive"));
        assert!(script.contains("\"balanced\""));
    }
}
