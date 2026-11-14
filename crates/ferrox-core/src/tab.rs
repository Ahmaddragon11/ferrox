// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::config::ComputeMode;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TabId(pub Uuid);

impl TabId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TabId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TabState {
    Active,
    Throttled,
    Hibernated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabSnapshot {
    pub url: String,
    pub title: String,
    pub scroll_y: f32,
    pub hibernated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabInfo {
    pub id: TabId,
    pub url: String,
    pub title: String,
    pub pinned: bool,
    pub muted: bool,
    pub loading: bool,
    pub can_go_back: bool,
    pub can_go_forward: bool,
    pub compute_mode: ComputeMode,
    pub state: TabState,
    pub last_active_at: DateTime<Utc>,
    pub estimated_memory_kb: u64,
    pub snapshot: Option<TabSnapshot>,
}

impl TabInfo {
    pub fn new(url: impl Into<String>, compute_mode: ComputeMode) -> Self {
        let url_str = url.into();
        let title = if url_str.starts_with("ferrox://newtab") {
            "New Tab".to_string()
        } else {
            url_str.clone()
        };
        Self {
            id: TabId::new(),
            url: url_str,
            title,
            pinned: false,
            muted: false,
            loading: false,
            can_go_back: false,
            can_go_forward: false,
            compute_mode,
            state: TabState::Active,
            last_active_at: Utc::now(),
            estimated_memory_kb: 24_000,
            snapshot: None,
        }
    }
}

#[derive(Debug, Default)]
pub struct TabManager {
    tabs: Vec<TabInfo>,
    active_index: Option<usize>,
    closed_history: Vec<TabInfo>,
}

impl TabManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_tab(&mut self, url: impl Into<String>, mode: ComputeMode, activate: bool) -> TabId {
        let mut tab = TabInfo::new(url, mode);
        let id = tab.id;
        if activate {
            for t in &mut self.tabs {
                if t.state == TabState::Active {
                    t.state = TabState::Throttled;
                }
            }
            tab.state = TabState::Active;
            self.tabs.push(tab);
            self.active_index = Some(self.tabs.len() - 1);
        } else {
            tab.state = TabState::Throttled;
            self.tabs.push(tab);
            if self.active_index.is_none() {
                self.active_index = Some(0);
            }
        }
        id
    }

    pub fn tabs(&self) -> &[TabInfo] {
        &self.tabs
    }

    pub fn active_tab(&self) -> Option<&TabInfo> {
        self.active_index.and_then(|idx| self.tabs.get(idx))
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut TabInfo> {
        self.active_index.and_then(|idx| self.tabs.get_mut(idx))
    }

    pub fn find_mut(&mut self, id: TabId) -> Option<&mut TabInfo> {
        self.tabs.iter_mut().find(|t| t.id == id)
    }

    pub fn activate(&mut self, id: TabId) -> bool {
        let Some(pos) = self.tabs.iter().position(|t| t.id == id) else {
            return false;
        };
        for (i, t) in self.tabs.iter_mut().enumerate() {
            if i == pos {
                t.state = TabState::Active;
                t.last_active_at = Utc::now();
                t.snapshot = None;
            } else if t.state == TabState::Active {
                t.state = TabState::Throttled;
            }
        }
        self.active_index = Some(pos);
        true
    }

    pub fn close_tab(&mut self, id: TabId) -> Option<TabInfo> {
        let pos = self.tabs.iter().position(|t| t.id == id)?;
        let closed = self.tabs.remove(pos);
        self.closed_history.push(closed.clone());
        if self.closed_history.len() > 25 {
            self.closed_history.remove(0);
        }

        if self.tabs.is_empty() {
            self.active_index = None;
        } else {
            let new_idx = pos.min(self.tabs.len() - 1);
            self.active_index = Some(new_idx);
            if let Some(active) = self.tabs.get_mut(new_idx) {
                active.state = TabState::Active;
                active.last_active_at = Utc::now();
            }
        }
        Some(closed)
    }

    pub fn reopen_last_closed(&mut self) -> Option<TabId> {
        let tab = self.closed_history.pop()?;
        let id = self.open_tab(tab.url, tab.compute_mode, true);
        if let Some(t) = self.find_mut(id) {
            t.title = tab.title;
        }
        Some(id)
    }

    pub fn hibernate_tab(&mut self, id: TabId, scroll_y: f32) -> bool {
        let is_active = self.active_tab().map(|t| t.id == id).unwrap_or(false);
        if is_active {
            return false;
        }
        let Some(tab) = self.find_mut(id) else {
            return false;
        };
        if tab.pinned || tab.state == TabState::Hibernated {
            return false;
        }
        tab.snapshot = Some(TabSnapshot {
            url: tab.url.clone(),
            title: tab.title.clone(),
            scroll_y,
            hibernated_at: Utc::now(),
        });
        tab.state = TabState::Hibernated;
        tab.estimated_memory_kb = 2_048;
        true
    }

    pub fn total_estimated_memory_kb(&self) -> u64 {
        self.tabs.iter().map(|t| t.estimated_memory_kb).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_lifecycle_and_hibernation() {
        let mut mgr = TabManager::new();
        let t1 = mgr.open_tab("https://example.com", ComputeMode::Balanced, true);
        let t2 = mgr.open_tab("https://rust-lang.org", ComputeMode::Balanced, true);

        assert_eq!(mgr.active_tab().unwrap().id, t2);
        assert!(mgr.hibernate_tab(t1, 120.0));
        assert!(!mgr.hibernate_tab(t2, 0.0)); // cannot hibernate active tab
        assert_eq!(
            mgr.tabs().iter().find(|t| t.id == t1).unwrap().state,
            TabState::Hibernated
        );

        assert!(mgr.activate(t1));
        assert_eq!(mgr.active_tab().unwrap().id, t1);
        assert_eq!(mgr.active_tab().unwrap().state, TabState::Active);

        mgr.close_tab(t1);
        assert_eq!(mgr.tabs().len(), 1);
        let reopened = mgr.reopen_last_closed().unwrap();
        assert_ne!(reopened, t2);
        assert_eq!(mgr.tabs().len(), 2);
    }
}
