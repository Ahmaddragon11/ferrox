// SPDX-License-Identifier: MIT OR Apache-2.0

use chrono::Utc;
use ferrox_core::{ComputeMode, FerroxConfig, TabManager, TabState, UpdateChannel};
use ferrox_engine::{
    InternalPageContext, SettingsView, TurboDocument, TurboSummary, UpdateView, is_internal,
    render as render_internal,
};
use ferrox_storage::StorageDb;
use ferrox_ui::{ChromeState, ChromeTab, SuggestionItem};
use std::sync::Arc;

use crate::app::FerroxApp;

impl FerroxApp {
    pub(crate) fn sync_chrome(&mut self) {
        self.chrome.tabs = self
            .tabs
            .tabs()
            .iter()
            .map(|tab| ChromeTab {
                id: tab.id,
                title: if tab.title.trim().is_empty() {
                    tab.url.clone()
                } else {
                    tab.title.clone()
                },
                url: tab.url.clone(),
                state: tab.state,
                loading: tab.loading,
                muted: tab.muted,
                pinned: tab.pinned,
            })
            .collect();

        self.chrome.active = self.tabs.active_tab().map(|t| t.id);
        if let Some(active) = self.tabs.active_tab() {
            if !self.address_editing {
                self.chrome.address = if active.url.starts_with("about:") {
                    String::new()
                } else {
                    active.url.clone()
                };
            }
            self.chrome.compute_mode = active.compute_mode;
            self.chrome.loading = active.loading;
        }
        self.chrome.blocked = self.shield.stats().requests_blocked;
        self.chrome.shield_enabled = self.config.shield_enabled;
        self.chrome.incognito = self.incognito;
        self.chrome.version = self.version.clone();
        let total_kb = self.tabs.total_estimated_memory_kb();
        self.chrome.memory_mb = total_kb as f64 / 1024.0 + 42.0;
        self.chrome.update_available = self.update_view.available_version.is_some();

        let active_id = self.tabs.active_tab().map(|t| t.id);
        let turbo_back = active_id
            .map(|id| {
                self.turbo_stack
                    .get(&id)
                    .map(|stack| stack.len() > 1)
                    .unwrap_or(false)
            })
            .unwrap_or(false);
        self.chrome.can_back = turbo_back
            || active_id
                .map(|id| self.engine.can_go_back(id).unwrap_or(false))
                .unwrap_or(false);
        self.chrome.can_forward = active_id
            .map(|id| self.engine.can_go_forward(id).unwrap_or(false))
            .unwrap_or(false);
    }

    pub(crate) fn refresh_suggestions(&mut self) {
        let input = self.chrome.address.trim().to_string();
        if input.is_empty() {
            self.chrome.suggestions.clear();
            self.chrome.show_suggestions = false;
            return;
        }
        let db = self.active_db();
        match self.omnibox.suggestions(&db, &input, 6) {
            Ok(items) => {
                self.chrome.suggestions = items
                    .into_iter()
                    .map(|s| SuggestionItem {
                        kind: match s.kind {
                            ferrox_search::SuggestionKind::History => "history".to_string(),
                            ferrox_search::SuggestionKind::Bookmark => "bookmark".to_string(),
                            ferrox_search::SuggestionKind::Search => "search".to_string(),
                            ferrox_search::SuggestionKind::Url => "open".to_string(),
                            ferrox_search::SuggestionKind::Calculation => "calc".to_string(),
                        },
                        title: s.title,
                        url: s.url,
                    })
                    .collect();
                self.chrome.show_suggestions = !self.chrome.suggestions.is_empty();
                self.chrome.selected = 0;
            }
            Err(err) => {
                tracing::warn!("suggestion error: {err}");
            }
        }
    }

    pub(crate) fn internal_context(&self, turbo: Option<&TurboDocument>) -> InternalPageContext {
        let db = self.active_db();
        let history = db.recent_history(300).unwrap_or_default();
        let bookmarks = db.list_bookmarks(None).unwrap_or_default();
        let downloads = db.list_downloads().unwrap_or_default();
        let top_sites = db.top_sites(8).unwrap_or_default();

        let settings = SettingsView {
            search_engine: match self.config.default_search {
                ferrox_core::SearchEngineId::DuckDuckGo => "duckduckgo",
                ferrox_core::SearchEngineId::Google => "google",
                ferrox_core::SearchEngineId::Brave => "brave",
                ferrox_core::SearchEngineId::Startpage => "startpage",
                ferrox_core::SearchEngineId::Bing => "bing",
            }
            .to_string(),
            homepage: self.config.homepage.clone(),
            theme: match self.config.theme {
                ferrox_core::ThemeMode::Dark => "dark",
                ferrox_core::ThemeMode::Light => "light",
                ferrox_core::ThemeMode::System => "system",
            }
            .to_string(),
            compute_mode: self.config.default_compute_mode.as_str().to_string(),
            shield_enabled: self.config.shield_enabled,
            hibernate_after_secs: self.config.hibernate_after_secs,
            update_channel: self.config.update_channel.as_str().to_string(),
            auto_check_updates: self.config.auto_check_updates,
        };

        let hibernated = self
            .tabs
            .tabs()
            .iter()
            .filter(|t| t.state == TabState::Hibernated)
            .count();

        InternalPageContext {
            version: self.version.clone(),
            settings,
            shield_stats: self.shield.stats(),
            memory_mb: self.chrome.memory_mb,
            tab_count: self.tabs.tabs().len(),
            hibernated_count: hibernated,
            top_sites,
            history,
            bookmarks,
            downloads,
            update: self.update_view.clone(),
            turbo: turbo.map(|doc| TurboSummary {
                url: doc.final_url.clone(),
                title: doc.title.clone(),
                words: doc.word_count,
                read_minutes: doc.read_minutes,
                load_ms: doc.load_ms,
            }),
        }
    }

    pub(crate) fn open_internal(&mut self, url: &str) {
        let Some(tab_id) = self.tabs.active_tab().map(|t| t.id) else {
            return;
        };
        let html = render_internal(url, &self.internal_context(None));
        self.load_html(tab_id, url, html);
    }

    pub(crate) fn navigate_address(&mut self, input: &str) {
        self.address_editing = false;
        let action = self.omnibox.resolve(input);
        match action {
            ferrox_search::OmniboxAction::Navigate(url) => self.navigate_tab_url(url),
            ferrox_search::OmniboxAction::Search { engine, query } => {
                let url = ferrox_search::build_search_url(engine, &query);
                self.navigate_tab_url(url);
            }
            ferrox_search::OmniboxAction::Internal(url) => {
                self.open_internal(&url);
            }
            ferrox_search::OmniboxAction::ShowCalculation { result, .. } => {
                self.chrome.address = result;
                self.address_editing = false;
            }
        }
    }

    pub(crate) fn navigate_tab_url(&mut self, url: String) {
        let Some(tab_id) = self.tabs.active_tab().map(|t| t.id) else {
            return;
        };
        self.navigate_url(tab_id, url);
    }

    pub(crate) fn navigate_url(&mut self, tab_id: ferrox_core::TabId, url: String) {
        if is_internal(&url) && !url.starts_with("ferrox://") {
            return;
        }
        let Some(window) = self.window.clone() else {
            return;
        };

        let mode = self
            .tabs
            .find_mut(tab_id)
            .map(|t| t.compute_mode)
            .unwrap_or(self.config.default_compute_mode);

        if let Some(tab) = self.tabs.find_mut(tab_id) {
            tab.loading = true;
            tab.url = url.clone();
            tab.title = if is_internal(&url) {
                "New Tab".to_string()
            } else {
                url.clone()
            };
        }

        if is_internal(&url) {
            self.turbo_docs.remove(&tab_id);
            self.turbo_current.remove(&tab_id);
            self.turbo_stack.remove(&tab_id);
            let html = render_internal(&url, &self.internal_context(None));
            if let Err(err) = self.ensure_view(tab_id, &window, mode, "about:blank") {
                tracing::error!("view error: {err}");
                return;
            }
            self.load_html(tab_id, &url, html);
            let _ = self.engine.focus(tab_id);
            return;
        }

        if mode == ComputeMode::Turbo {
            self.start_turbo(tab_id, url, true);
            return;
        }

        self.turbo_docs.remove(&tab_id);
        self.turbo_current.remove(&tab_id);
        self.turbo_stack.remove(&tab_id);
        if let Err(err) = self.ensure_view(tab_id, &window, mode, &url) {
            tracing::error!("view error: {err}");
            return;
        }
        if self.engine.current_url(tab_id).as_deref() != Some(url.as_str()) {
            if let Err(err) = self.engine.navigate(tab_id, &url) {
                tracing::error!("navigation error: {err}");
            }
        }
        let _ = self.engine.focus(tab_id);
    }

    pub(crate) fn ensure_view(
        &mut self,
        tab_id: ferrox_core::TabId,
        window: &Arc<winit::window::Window>,
        mode: ComputeMode,
        initial_url: &str,
    ) -> ferrox_core::Result<()> {
        if self.engine.has_view(tab_id) {
            return Ok(());
        }
        let visible = self.tabs.active_tab().map(|t| t.id) == Some(tab_id);
        self.engine
            .ensure_view(tab_id, window.as_ref(), mode, initial_url, visible)
    }

    pub(crate) fn load_html(&mut self, tab_id: ferrox_core::TabId, display_url: &str, html: String) {
        if let Err(err) = self.engine.load_html(tab_id, &html) {
            tracing::error!("load_html error: {err}");
            return;
        }
        if let Some(tab) = self.tabs.find_mut(tab_id) {
            tab.url = display_url.to_string();
            tab.loading = false;
        }
        self.chrome.address = display_url.to_string();
    }

    pub(crate) fn start_turbo(&mut self, tab_id: ferrox_core::TabId, url: String, push_history: bool) {
        let Some(window) = self.window.clone() else {
            return;
        };
        if let Err(err) = self.ensure_view(tab_id, &window, ComputeMode::Turbo, "about:blank") {
            tracing::error!("view error: {err}");
            return;
        }
        let _ = self.engine.load_html(tab_id, turbo_loading_page(&url, &self.version));

        if push_history {
            let stack = self.turbo_stack.entry(tab_id).or_default();
            if stack.last() != Some(&url) {
                stack.push(url.clone());
                if stack.len() > 50 {
                    stack.remove(0);
                }
            }
        }

        let seq = self.next_nav_seq();
        self.pending_turbo.insert(tab_id, (url.clone(), seq));

        let reader = self.turbo.clone();
        let tx = self.async_tx.clone();
        self.runtime.spawn(async move {
            let result = reader.fetch(&url).await;
            let _ = tx.send(crate::app::AsyncEvent::TurboReady {
                tab_id,
                seq,
                result: Box::new(result),
            });
        });

        if let Some(tab) = self.tabs.find_mut(tab_id) {
            tab.loading = true;
            tab.url = url;
        }
    }

    pub(crate) fn active_db(&self) -> Arc<StorageDb> {
        if self.incognito {
            self.incognito_db.clone()
        } else {
            self.standard_db.clone()
        }
    }

    pub(crate) fn save_config(&self) {
        if let Err(err) = self.config.save(&self.config_path) {
            tracing::warn!("failed to save config: {err}");
        }
    }

    pub(crate) fn update_view(&mut self) {
        self.incognito = !self.incognito;
        self.chrome.incognito = self.incognito;
        self.chrome.status = Some(if self.incognito {
            "Incognito mode on — no history is written".to_string()
        } else {
            "Incognito mode off".to_string()
        });
        self.open_internal("ferrox://newtab");
    }

    pub(crate) fn current_update_view(&self) -> UpdateView {
        UpdateView {
            current_version: self.version.clone(),
            channel: match self.config.update_channel {
                UpdateChannel::Stable => "stable",
                UpdateChannel::Beta => "beta",
                UpdateChannel::Nightly => "nightly",
            }
            .to_string(),
            last_check: self
                .last_update_check
                .map(|t| t.format("%Y-%m-%d %H:%M UTC").to_string())
                .unwrap_or_default(),
            available_version: self.update_view.available_version.clone(),
        }
    }

    pub(crate) fn push_profiler_stats(&mut self) {
        if let Some(tab) = self.tabs.active_tab()
            && tab.url.starts_with("ferrox://profiler")
        {
            let stats = self.shield.stats();
            let payload = serde_json::json!({
                "memory_mb": self.chrome.memory_mb,
                "tabs": self.tabs.tabs().len(),
                "hibernated": self.tabs.tabs().iter().filter(|t| t.state == TabState::Hibernated).count(),
                "blocked": stats.requests_blocked,
                "saved_mb": stats.bytes_estimated_saved as f64 / (1024.0 * 1024.0),
                "scripts_blocked": stats.scripts_blocked,
                "checked": stats.requests_checked,
            });
            let js = format!("window.__ferroxUpdateStats && window.__ferroxUpdateStats({payload});");
            let _ = self.engine.evaluate(tab.id, &js);
        }
    }

    pub(crate) fn update_tab_title(&self) {
        if let Some(window) = &self.window {
            let title = self
                .tabs
                .active_tab()
                .map(|t| format!("{} — Ferrox", t.title))
                .unwrap_or_else(|| "Ferrox".to_string());
            window.set_title(&title);
        }
    }

    pub(crate) fn now(&self) -> chrono::DateTime<Utc> {
        Utc::now()
    }

    pub(crate) fn config_snapshot(&self) -> FerroxConfig {
        self.config.clone()
    }

    pub(crate) fn tab_manager(&self) -> &TabManager {
        &self.tabs
    }
}

/// Lightweight placeholder shown while the pure-Rust reader fetches a page.
pub(crate) fn turbo_loading_page(url: &str, version: &str) -> String {
    let safe_url = ferrox_engine::internal_pages::escape_html(url);
    format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><title>Turbo — Ferrox</title>
<style>
body{{margin:0;min-height:100vh;display:grid;place-items:center;background:#0b0d10;color:#e9edf2;
font-family:Inter,-apple-system,"Segoe UI",system-ui,sans-serif}}
.card{{max-width:540px;padding:34px;border:1px solid #232a35;border-radius:16px;
background:linear-gradient(180deg,#171c25,#12161d);text-align:center}}
.spin{{width:34px;height:34px;margin:0 auto 18px;border-radius:50%;
border:3px solid #232a35;border-top-color:#37d6c4;animation:sp 1s linear infinite}}
@keyframes sp{{to{{transform:rotate(360deg)}}}}
h1{{font-size:17px;margin:0 0 8px;letter-spacing:.3px}}
p{{color:#8b96a7;font-size:12.5px;margin:0 0 8px;line-height:1.6}}
code{{color:#ffb347;font-size:11px;word-break:break-all}}
.badge{{display:inline-block;margin-top:14px;padding:4px 10px;border-radius:999px;
border:1px solid #232a35;color:#37d6c4;font-size:10px;letter-spacing:1.4px;text-transform:uppercase}}
</style></head><body><div class="card">
<div class="spin"></div>
<h1>Turbo engine is extracting this page</h1>
<p>Pure-Rust parsing with zero JavaScript execution · target under 15&nbsp;ms render</p>
<code>{safe_url}</code>
<div class="badge">Ferrox {version} · Turbo</div>
</div></body></html>"#
    )
}
