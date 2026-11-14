// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::policy::GovernorPolicy;
use ferrox_core::ComputeMode;

/// Resolves the correct governor policy for a URL given user workspace defaults
/// and per-site overrides.
pub struct GovernorInjector {
    default_mode: ComputeMode,
}

impl GovernorInjector {
    pub fn new(default_mode: ComputeMode) -> Self {
        Self { default_mode }
    }

    pub fn default_mode(&self) -> ComputeMode {
        self.default_mode
    }

    pub fn set_default_mode(&mut self, mode: ComputeMode) {
        self.default_mode = mode;
    }

    pub fn policy_for(&self, mode_override: Option<ComputeMode>) -> GovernorPolicy {
        GovernorPolicy::for_mode(mode_override.unwrap_or(self.default_mode))
    }

    pub fn script_for(&self, mode_override: Option<ComputeMode>) -> String {
        self.policy_for(mode_override).injection_script()
    }
}
