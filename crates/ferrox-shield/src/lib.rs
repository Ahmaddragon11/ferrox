// SPDX-License-Identifier: MIT OR Apache-2.0

use adblock::Engine;
use adblock::lists::{FilterSet, ParseOptions};
use adblock::request::Request;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

pub const DEFAULT_RULES: &str = include_str!("../../../assets/filter-lists/default-rules.txt");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceType {
    Document,
    Script,
    Stylesheet,
    Image,
    Font,
    Media,
    Xhr,
    Subdocument,
    Websocket,
    Other,
}

impl ResourceType {
    pub fn as_adblock_str(self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::Script => "script",
            Self::Stylesheet => "stylesheet",
            Self::Image => "image",
            Self::Font => "font",
            Self::Media => "media",
            Self::Xhr => "xhr",
            Self::Subdocument => "sub_frame",
            Self::Websocket => "websocket",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ShieldStats {
    pub requests_checked: u64,
    pub requests_blocked: u64,
    pub scripts_blocked: u64,
    pub bytes_estimated_saved: u64,
}

#[derive(Debug, Clone)]
pub struct ShieldVerdict {
    pub blocked: bool,
    pub rewritten_url: Option<String>,
    pub redirect: Option<String>,
}

/// High-performance native adblock engine wrapping Brave's `adblock-rust`.
pub struct ShieldEngine {
    engine: RwLock<Engine>,
    stats: ShieldStatsAtomic,
}

#[derive(Debug, Default)]
struct ShieldStatsAtomic {
    checked: AtomicU64,
    blocked: AtomicU64,
    scripts_blocked: AtomicU64,
    bytes_saved: AtomicU64,
}

impl ShieldEngine {
    pub fn new() -> Self {
        Self::with_rules(DEFAULT_RULES)
    }

    pub fn with_rules(rules: &str) -> Self {
        let mut filter_set = FilterSet::new(false);
        filter_set.add_filter_list(rules.to_string(), ParseOptions::default());
        let engine = Engine::new_with_filter_set(filter_set);
        Self {
            engine: RwLock::new(engine),
            stats: ShieldStatsAtomic::default(),
        }
    }

    /// Adds extra filter list text by rebuilding the engine atomically.
    pub fn add_rules(&self, extra_rules: &str) {
        let mut combined = String::with_capacity(DEFAULT_RULES.len() + extra_rules.len() + 1);
        combined.push_str(DEFAULT_RULES);
        combined.push('\n');
        combined.push_str(extra_rules);

        let mut filter_set = FilterSet::new(false);
        filter_set.add_filter_list(combined, ParseOptions::default());
        let new_engine = Engine::new_with_filter_set(filter_set);
        *self.engine.write() = new_engine;
    }

    pub fn check(&self, url: &str, source_url: &str, resource_type: ResourceType) -> ShieldVerdict {
        self.stats.checked.fetch_add(1, Ordering::Relaxed);

        let Ok(request) = Request::new(url, source_url, resource_type.as_adblock_str(), "GET")
        else {
            return ShieldVerdict {
                blocked: false,
                rewritten_url: None,
                redirect: None,
            };
        };

        let result = self.engine.read().check_network_request(&request);
        if result.should_block() {
            self.stats.blocked.fetch_add(1, Ordering::Relaxed);
            if resource_type == ResourceType::Script {
                self.stats.scripts_blocked.fetch_add(1, Ordering::Relaxed);
            }
            self.stats
                .bytes_saved
                .fetch_add(estimate_savings(resource_type), Ordering::Relaxed);

            ShieldVerdict {
                blocked: true,
                rewritten_url: None,
                redirect: result.redirect,
            }
        } else {
            ShieldVerdict {
                blocked: false,
                rewritten_url: result.rewritten_url,
                redirect: None,
            }
        }
    }

    pub fn is_blocked(&self, url: &str, source_url: &str) -> bool {
        self.check(url, source_url, ResourceType::Other).blocked
    }

    pub fn stats(&self) -> ShieldStats {
        ShieldStats {
            requests_checked: self.stats.checked.load(Ordering::Relaxed),
            requests_blocked: self.stats.blocked.load(Ordering::Relaxed),
            scripts_blocked: self.stats.scripts_blocked.load(Ordering::Relaxed),
            bytes_estimated_saved: self.stats.bytes_saved.load(Ordering::Relaxed),
        }
    }

    /// Returns `(blocked_ratio, estimated_saved_mb)` for the profiler UI.
    pub fn savings_summary(&self) -> (f32, f64) {
        let checked = self.stats.checked.load(Ordering::Relaxed);
        let blocked = self.stats.blocked.load(Ordering::Relaxed);
        let saved = self.stats.bytes_saved.load(Ordering::Relaxed);
        let ratio = if checked == 0 {
            0.0
        } else {
            blocked as f32 / checked as f32
        };
        (ratio, saved as f64 / (1024.0 * 1024.0))
    }
}

impl Default for ShieldEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn estimate_savings(resource_type: ResourceType) -> u64 {
    match resource_type {
        ResourceType::Script => 48 * 1024,
        ResourceType::Image => 32 * 1024,
        ResourceType::Media => 512 * 1024,
        ResourceType::Font => 64 * 1024,
        ResourceType::Stylesheet => 16 * 1024,
        ResourceType::Subdocument => 128 * 1024,
        _ => 8 * 1024,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_known_tracker_domains() {
        let shield = ShieldEngine::new();
        assert!(
            shield
                .check(
                    "https://www.google-analytics.com/collect",
                    "https://example.com",
                    ResourceType::Script
                )
                .blocked
        );
        assert!(
            shield
                .check(
                    "https://doubleclick.net/ad.js",
                    "https://news.example.com",
                    ResourceType::Script
                )
                .blocked
        );
        assert!(
            !shield
                .check(
                    "https://example.com/app.js",
                    "https://example.com",
                    ResourceType::Script
                )
                .blocked
        );
    }

    #[test]
    fn stats_accumulate() {
        let shield = ShieldEngine::new();
        shield.check(
            "https://googletagmanager.com/gtm.js",
            "https://example.com",
            ResourceType::Script,
        );
        shield.check("https://example.com/style.css", "https://example.com", ResourceType::Stylesheet);
        let stats = shield.stats();
        assert_eq!(stats.requests_checked, 2);
        assert_eq!(stats.requests_blocked, 1);
        assert_eq!(stats.scripts_blocked, 1);
        assert!(stats.bytes_estimated_saved > 0);
    }
}
