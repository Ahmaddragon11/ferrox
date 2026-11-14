// SPDX-License-Identifier: MIT OR Apache-2.0

use chrono::{DateTime, Utc};
use ferrox_core::{TabInfo, TabManager, TabState};
use sysinfo::System;

/// Decides when background tabs should be hibernated based on idle time and
/// live system memory pressure, reclaiming renderer processes to keep RAM low.
pub struct HibernationController {
    idle_timeout_secs: u64,
    memory_pressure_percent: f32,
}

impl Default for HibernationController {
    fn default() -> Self {
        Self::new(300, 80.0)
    }
}

impl HibernationController {
    pub fn new(idle_timeout_secs: u64, memory_pressure_percent: f32) -> Self {
        Self {
            idle_timeout_secs,
            memory_pressure_percent,
        }
    }

    pub fn system_memory_usage_percent(&self) -> f32 {
        let mut sys = System::new();
        sys.refresh_memory();
        let total = sys.total_memory();
        if total == 0 {
            return 0.0;
        }
        let used = sys.used_memory();
        (used as f32 / total as f32) * 100.0
    }

    pub fn should_hibernate(&self, tab: &TabInfo, now: DateTime<Utc>, memory_percent: f32) -> bool {
        if tab.pinned || tab.state == TabState::Hibernated || tab.state == TabState::Active {
            return false;
        }
        let idle_secs = (now - tab.last_active_at).num_seconds().max(0) as u64;
        let idle_trigger = idle_secs >= self.idle_timeout_secs;
        let pressure_trigger = memory_percent >= self.memory_pressure_percent && idle_secs >= 30;
        idle_trigger || pressure_trigger
    }

    /// Sweeps all tabs and hibernates eligible ones. Returns the IDs that were suspended.
    pub fn sweep(&self, manager: &mut TabManager) -> Vec<ferrox_core::TabId> {
        let now = Utc::now();
        let memory = self.system_memory_usage_percent();
        let candidates: Vec<ferrox_core::TabId> = manager
            .tabs()
            .iter()
            .filter(|t| self.should_hibernate(t, now, memory))
            .map(|t| t.id)
            .collect();

        let mut hibernated = Vec::new();
        for id in candidates {
            if manager.hibernate_tab(id, 0.0) {
                hibernated.push(id);
            }
        }
        hibernated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrox_core::ComputeMode;
    use chrono::Duration;

    #[test]
    fn hibernates_idle_background_tabs_only() {
        let mut mgr = TabManager::new();
        let background = mgr.open_tab("https://a.example", ComputeMode::Balanced, true);
        let active = mgr.open_tab("https://b.example", ComputeMode::Balanced, true);

        if let Some(tab) = mgr.find_mut(background) {
            tab.last_active_at = Utc::now() - Duration::seconds(3600);
        }

        let controller = HibernationController::new(300, 101.0);
        let hibernated = controller.sweep(&mut mgr);

        assert!(hibernated.contains(&background));
        assert!(!hibernated.contains(&active));
        assert_eq!(
            mgr.tabs().iter().find(|t| t.id == background).unwrap().state,
            TabState::Hibernated
        );
    }
}
