// SPDX-License-Identifier: MIT OR Apache-2.0

use chrono::{DateTime, Utc};
use ferrox_core::{
    ComputeMode, FerroxConfig, FerroxError, Result, SearchEngineId, TabId, TabManager, ThemeMode,
    UpdateChannel,
};
use ferrox_engine::{
    EngineEvent, EngineShared, TurboDocument, TurboReader, UpdateView, WebViewEngine, bounds,
    is_internal, render as render_internal,
};
use ferrox_governor::HibernationController;
use ferrox_net::FastNetClient;
use ferrox_search::OmniboxResolver;
use ferrox_shield::ShieldEngine;
use ferrox_storage::{SessionTab, StorageDb};
use ferrox_ui::{ChromeAction, ChromeState, chrome};
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::mpsc::TryRecvError;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy};
use winit::window::{Icon, Window, WindowId};

#[derive(Debug, Clone, Copy)]
pub(crate) enum AppEvent {
    Wake,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct StartupOptions {
    pub(crate) url: Option<String>,
    pub(crate) turbo: bool,
    pub(crate) private: bool,
}

/// Messages produced by background tasks and delivered back to the UI thread.
pub(crate) enum AsyncEvent {
    TurboReady {
        tab_id: TabId,
        seq: u64,
        result: Box<Result<TurboDocument>>,
    },
    UpdateChecked(Box<Result<Option<(ferrox_updater::UpdateManifest, ferrox_updater::ReleaseAsset)>>>),
    UpdateDownloaded(Box<Result<PathBuf>>),
}

/// Channel wrapper that wakes the winit event loop after every message.
#[derive(Clone)]
pub(crate) struct AsyncBus {
    tx: tokio::sync::mpsc::UnboundedSender<AsyncEvent>,
    wake: Arc<dyn Fn() + Send + Sync>,
}

impl AsyncBus {
    pub(crate) fn send(&self, event: AsyncEvent) {
        let _ = self.tx.send(event);
        (self.wake)();
    }
}

enum GlobalAction {
    FocusAddress,
    NewTab,
    ReopenTab,
    CloseTab,
    Reload,
    Bookmark,
    History,
    Downloads,
    Incognito,
}

pub(crate) struct FerroxApp {
    pub(crate) window: Option<Arc<Window>>,
    pub(crate) egui_ctx: egui::Context,
    pub(crate) egui_state: Option<egui_winit::State>,
    pub(crate) painter: Option<egui_wgpu::winit::Painter>,
    pub(crate) runtime: tokio::runtime::Runtime,

    pub(crate) tabs: TabManager,
    pub(crate) chrome: ChromeState,
    pub(crate) config: FerroxConfig,
    pub(crate) config_path: PathBuf,
    pub(crate) data_dir: PathBuf,
    pub(crate) shield: Arc<ShieldEngine>,
    pub(crate) engine: WebViewEngine,
    pub(crate) engine_shared: Arc<EngineShared>,
    pub(crate) engine_rx: std::sync::mpsc::Receiver<EngineEvent>,
    pub(crate) omnibox: OmniboxResolver,

    pub(crate) standard_db: Arc<StorageDb>,
    pub(crate) incognito_db: Arc<StorageDb>,
    pub(crate) incognito: bool,

    pub(crate) async_tx: AsyncBus,
    pub(crate) async_rx: tokio::sync::mpsc::UnboundedReceiver<AsyncEvent>,
    pub(crate) net: FastNetClient,

    pub(crate) turbo: TurboReader,
    pub(crate) pending_turbo: HashMap<TabId, (String, u64)>,
    pub(crate) turbo_docs: HashMap<TabId, TurboDocument>,
    pub(crate) turbo_current: HashMap<TabId, String>,
    pub(crate) turbo_stack: HashMap<TabId, Vec<String>>,
    pub(crate) next_seq: u64,

    pub(crate) address_editing: bool,
    pub(crate) update_view: UpdateView,
    pub(crate) pending_update: Option<(
        ferrox_updater::UpdateManifest,
        ferrox_updater::ReleaseAsset,
    )>,
    pub(crate) last_update_check: Option<DateTime<Utc>>,
    pub(crate) update_in_flight: bool,
    pub(crate) update_checked_once: bool,

    pub(crate) hibernation: HibernationController,
    pub(crate) last_hibernate: Instant,
    pub(crate) last_session_save: Instant,
    pub(crate) last_profiler: Instant,
    pub(crate) next_tick: Instant,
    pub(crate) repaint_deadline: Option<Instant>,
    pub(crate) layout: (f64, f64, f64),

    pub(crate) bootstrapped: bool,
    pub(crate) should_exit: bool,
    pub(crate) shutting_down: bool,
    pub(crate) version: String,
    pub(crate) wake: Arc<dyn Fn() + Send + Sync>,
    pub(crate) startup: StartupOptions,
}

impl FerroxApp {
    pub(crate) fn new(proxy: EventLoopProxy<AppEvent>, startup: StartupOptions) -> Result<Self> {
        let version = env!("CARGO_PKG_VERSION").to_string();
        let config_path = FerroxConfig::default_config_path();
        let config = FerroxConfig::load_or_default(&config_path);
        let data_dir = FerroxConfig::default_data_dir();
        std::fs::create_dir_all(&data_dir)?;

        let standard_db = Arc::new(StorageDb::open(&data_dir.join("ferrox.db"))?);
        let incognito_db = Arc::new(StorageDb::open_in_memory()?);

        let shield = Arc::new(ShieldEngine::new());

        let wake_proxy = Arc::new(Mutex::new(proxy));
        let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
            if let Ok(proxy) = wake_proxy.lock() {
                let _ = proxy.send_event(AppEvent::Wake);
            }
        });

        let (engine_tx, engine_rx) = std::sync::mpsc::channel();
        let engine_shared = Arc::new(EngineShared::new(
            shield.clone(),
            engine_tx,
            wake.clone(),
        ));
        engine_shared.set_shield_enabled(config.shield_enabled);
        let engine = WebViewEngine::new(
            engine_shared.clone(),
            bounds(0.0, 110.0, 1280.0, 710.0),
        );

        let net = FastNetClient::new()?;
        let turbo = TurboReader::new(net.clone());

        let (async_sender, async_rx) = tokio::sync::mpsc::unbounded_channel();
        let async_tx = AsyncBus {
            tx: async_sender,
            wake: wake.clone(),
        };

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("ferrox-net")
            .enable_all()
            .build()
            .map_err(|e| FerroxError::Engine(format!("failed to start network runtime: {e}")))?;

        let omnibox = OmniboxResolver::new(config.default_search);
        let hibernation = HibernationController::new(
            config.hibernate_after_secs,
            config.max_ram_percent_before_hibernate,
        );

        let mut chrome = ChromeState::default();
        chrome.version = version.clone();
        chrome.memory_mb = 42.0;
        chrome.compute_mode = config.default_compute_mode;
        chrome.shield_enabled = config.shield_enabled;
        chrome.incognito = startup.private;

        let update_view = UpdateView {
            current_version: version.clone(),
            channel: config.update_channel.as_str().to_string(),
            last_check: String::new(),
            available_version: None,
        };

        Ok(Self {
            window: None,
            egui_ctx: egui::Context::default(),
            egui_state: None,
            painter: None,
            runtime,
            tabs: TabManager::new(),
            chrome,
            config,
            config_path,
            data_dir,
            shield,
            engine,
            engine_shared,
            engine_rx,
            omnibox,
            standard_db,
            incognito_db,
            incognito: startup.private,
            async_tx,
            async_rx,
            net,
            turbo,
            pending_turbo: HashMap::new(),
            turbo_docs: HashMap::new(),
            turbo_current: HashMap::new(),
            turbo_stack: HashMap::new(),
            next_seq: 0,
            address_editing: false,
            update_view,
            pending_update: None,
            last_update_check: None,
            update_in_flight: false,
            update_checked_once: false,
            hibernation,
            last_hibernate: Instant::now(),
            last_session_save: Instant::now(),
            last_profiler: Instant::now(),
            next_tick: Instant::now() + Duration::from_secs(10),
            repaint_deadline: None,
            layout: (0.0, 0.0, 0.0),
            bootstrapped: false,
            should_exit: false,
            shutting_down: false,
            version,
            wake,
            startup,
        })
    }

    pub(crate) fn next_nav_seq(&mut self) -> u64 {
        self.next_seq = self.next_seq.wrapping_add(1);
        self.next_seq
    }

    fn mode_for_url(&self, url: &str) -> ComputeMode {
        if is_internal(url) {
            ComputeMode::Balanced
        } else {
            self.config.compute_mode_for_url(url)
        }
    }

    fn active_tab_id(&self) -> Option<TabId> {
        self.tabs.active_tab().map(|t| t.id)
    }

    fn request_repaint(&self) {
        self.egui_ctx.request_repaint();
    }

    fn status(&mut self, message: impl Into<String>) {
        self.chrome.status = Some(message.into());
        self.request_repaint();
    }

    // ---------------------------------------------------------------------
    // Startup / session
    // ---------------------------------------------------------------------

    fn bootstrap(&mut self) {
        if self.bootstrapped {
            return;
        }
        self.bootstrapped = true;

        let mut first: Option<TabId> = None;

        if let Some(url) = self.startup.url.clone() {
            let mode = if self.startup.turbo {
                ComputeMode::Turbo
            } else {
                self.mode_for_url(&url)
            };
            let id = self.tabs.open_tab(url, mode, true);
            first = Some(id);
        } else {
            let mut restored = Vec::new();
            if self.config.restore_session_on_startup {
                if let Ok(session) = self.standard_db.load_session() {
                    restored = session;
                }
            }
            if !restored.is_empty() {
                for entry in restored {
                    if entry.url.trim().is_empty() {
                        continue;
                    }
                    let mode = self.mode_for_url(&entry.url);
                    self.tabs.open_tab(entry.url, mode, false);
                }
                if let Some(tab) = self.tabs.tabs().first() {
                    let id = tab.id;
                    self.tabs.activate(id);
                    first = Some(id);
                }
            } else {
                let homepage = self.config.homepage.clone();
                let mode = self.mode_for_url(&homepage);
                let id = self.tabs.open_tab(homepage, mode, true);
                first = Some(id);
            }
        }

        if let Some(id) = first {
            let url = self
                .tabs
                .tabs()
                .iter()
                .find(|t| t.id == id)
                .map(|t| t.url.clone())
                .unwrap_or_else(|| self.config.homepage.clone());
            self.navigate_url(id, url);
        }

        self.update_tab_title();
        if self.config.auto_check_updates {
            self.start_update_check();
        }
    }

    fn save_session(&self) {
        if self.incognito {
            return;
        }
        let tabs: Vec<SessionTab> = self
            .tabs
            .tabs()
            .iter()
            .enumerate()
            .filter(|(_, tab)| !tab.url.is_empty() && tab.url != "about:blank")
            .map(|(index, tab)| SessionTab {
                tab_id: tab.id.0.to_string(),
                url: tab.url.clone(),
                title: tab.title.clone(),
                position: index,
                pinned: tab.pinned,
                muted: tab.muted,
            })
            .collect();
        let _ = self.standard_db.save_session(&tabs);
    }

    // ---------------------------------------------------------------------
    // Tabs
    // ---------------------------------------------------------------------

    fn hide_other_views(&self, keep: TabId) {
        for tab in self.tabs.tabs() {
            if tab.id != keep && self.engine.has_view(tab.id) {
                let _ = self.engine.set_visible(tab.id, false);
            }
        }
    }

    fn show_tab(&mut self, id: TabId) {
        self.hide_other_views(id);
        if self.engine.has_view(id) {
            let _ = self.engine.set_visible(id, true);
            let _ = self.engine.focus(id);
        }
    }

    fn show_tab_and_load(&mut self, id: TabId) {
        self.show_tab(id);
        if self.engine.has_view(id) {
            if let Some(tab) = self.tabs.tabs().iter().find(|t| t.id == id) {
                self.chrome.address = tab.url.clone();
            }
            return;
        }
        let url = self
            .tabs
            .tabs()
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.url.clone());
        if let Some(url) = url {
            self.navigate_url(id, url);
        }
    }

    fn switch_tab(&mut self, id: TabId) {
        if self.active_tab_id() == Some(id) {
            return;
        }
        if !self.tabs.activate(id) {
            return;
        }
        self.address_editing = false;
        self.chrome.show_suggestions = false;
        self.chrome.menu_open = false;
        self.show_tab_and_load(id);
        if let Some(tab) = self.tabs.active_tab() {
            self.chrome.address = tab.url.clone();
        }
        self.update_tab_title();
        self.request_repaint();
    }

    fn open_tab(&mut self, url: String) {
        let mode = self.mode_for_url(&url);
        let id = self.tabs.open_tab(url, mode, true);
        self.address_editing = false;
        self.show_tab_and_load(id);
        self.update_tab_title();
        self.request_repaint();
    }

    fn new_tab_home(&mut self) {
        let url = self.config.homepage.clone();
        self.open_tab(url);
    }

    fn close_active_tab(&mut self) {
        let Some(id) = self.active_tab_id() else {
            self.new_tab_home();
            return;
        };
        let _ = self.engine.destroy(id);
        self.turbo_docs.remove(&id);
        self.turbo_current.remove(&id);
        self.turbo_stack.remove(&id);
        self.pending_turbo.remove(&id);
        self.tabs.close_tab(id);

        if self.tabs.tabs().is_empty() {
            self.new_tab_home();
            return;
        }

        if let Some(tab) = self.tabs.active_tab() {
            let tab_id = tab.id;
            self.chrome.address = tab.url.clone();
            self.show_tab_and_load(tab_id);
        }
        self.update_tab_title();
        self.request_repaint();
    }

    fn reopen_tab(&mut self) {
        if let Some(id) = self.tabs.reopen_last_closed() {
            self.show_tab_and_load(id);
            self.update_tab_title();
            self.request_repaint();
        }
    }

    fn reload_active(&mut self) {
        let Some(id) = self.active_tab_id() else { return };
        if self.turbo_current.contains_key(&id) {
            let url = self
                .turbo_current
                .get(&id)
                .cloned()
                .or_else(|| self.tabs.active_tab().map(|t| t.url.clone()));
            if let Some(url) = url {
                self.start_turbo(id, url, false);
            }
            return;
        }
        let url = self.tabs.active_tab().map(|t| t.url.clone()).unwrap_or_default();
        if url.starts_with("ferrox://") {
            self.open_internal(&url);
        } else if let Err(err) = self.engine.reload(id) {
            self.status(format!("Reload failed: {err}"));
        }
    }

    fn go_back(&mut self) {
        let Some(id) = self.active_tab_id() else { return };
        if self.turbo_current.contains_key(&id) {
            if let Some(stack) = self.turbo_stack.get_mut(&id) {
                if stack.len() > 1 {
                    stack.pop();
                    if let Some(previous) = stack.last().cloned() {
                        self.start_turbo(id, previous, false);
                        return;
                    }
                }
            }
        }
        if let Err(err) = self.engine.go_back(id) {
            self.status(format!("Cannot go back: {err}"));
        }
    }

    fn go_forward(&mut self) {
        let Some(id) = self.active_tab_id() else { return };
        if let Err(err) = self.engine.go_forward(id) {
            self.status(format!("Cannot go forward: {err}"));
        }
    }

    fn apply_compute_mode(&mut self, mode: ComputeMode) {
        self.config.default_compute_mode = mode;
        self.save_config();
        self.chrome.compute_mode = mode;
        let Some(id) = self.active_tab_id() else {
            self.request_repaint();
            return;
        };
        if let Some(tab) = self.tabs.find_mut(id) {
            tab.compute_mode = mode;
        }
        self.engine.destroy(id);
        self.turbo_docs.remove(&id);
        self.turbo_current.remove(&id);
        self.turbo_stack.remove(&id);
        self.pending_turbo.remove(&id);
        let url = self
            .tabs
            .tabs()
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.url.clone());
        if let Some(url) = url {
            self.navigate_url(id, url);
        }
        self.request_repaint();
    }

    fn toggle_bookmark(&mut self) {
        let Some(tab) = self.tabs.active_tab() else { return };
        let url = tab.url.clone();
        if !url.starts_with("http://") && !url.starts_with("https://") {
            self.status("Only web pages can be bookmarked");
            return;
        }
        let title = if tab.title.trim().is_empty() {
            url.clone()
        } else {
            tab.title.clone()
        };
        let db = self.standard_db.clone();
        match db.is_bookmarked(&url) {
            Ok(true) => {
                let _ = db.remove_bookmark_by_url(&url);
                self.status("Bookmark removed");
            }
            Ok(false) => {
                let _ = db.add_bookmark(&url, &title, "Bookmarks");
                self.status("Bookmarked");
            }
            Err(err) => self.status(format!("Bookmark failed: {err}")),
        }
        self.request_repaint();
    }

    fn apply_theme(&self, mode: ThemeMode) {
        match mode {
            ThemeMode::Light => {
                let mut visuals = egui::Visuals::light();
                visuals.panel_fill = egui::Color32::from_rgb(246, 244, 239);
                self.egui_ctx.set_visuals(visuals);
            }
            ThemeMode::Dark | ThemeMode::System => ferrox_ui::theme::apply(&self.egui_ctx),
        }
    }

    // ---------------------------------------------------------------------
    // Engine / IPC events
    // ---------------------------------------------------------------------

    fn drain_events(&mut self) {
        loop {
            match self.engine_rx.try_recv() {
                Ok(event) => self.handle_engine_event(event),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        loop {
            match self.async_rx.try_recv() {
                Ok(event) => self.handle_async_event(event),
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => break,
            }
        }
    }

    fn handle_engine_event(&mut self, event: EngineEvent) {
        match event {
            EngineEvent::Ipc { tab_id, payload } => self.handle_ipc(tab_id, &payload),
            EngineEvent::TitleChanged { tab_id, title } => {
                if let Some(tab) = self.tabs.find_mut(tab_id) {
                    tab.title = title;
                }
                if self.active_tab_id() == Some(tab_id) {
                    self.update_tab_title();
                }
                self.request_repaint();
            }
            EngineEvent::PageLoad {
                tab_id,
                url,
                finished,
            } => {
                if let Some(tab) = self.tabs.find_mut(tab_id) {
                    if finished {
                        tab.loading = false;
                    }
                    if url != "about:blank" && !url.starts_with("ferrox://") && !url.is_empty() {
                        tab.url = url.clone();
                    }
                }

                let is_web = (url.starts_with("http://") || url.starts_with("https://"))
                    && !self.turbo_current.contains_key(&tab_id);
                if finished && is_web {
                    let db = self.active_db();
                    let title = self
                        .tabs
                        .tabs()
                        .iter()
                        .find(|t| t.id == tab_id)
                        .map(|t| t.title.clone())
                        .unwrap_or_default();
                    let _ = db.record_visit(&url, &title);
                }

                if finished && self.active_tab_id() == Some(tab_id) && !self.address_editing {
                    if let Some(tab) = self.tabs.tabs().iter().find(|t| t.id == tab_id) {
                        self.chrome.address = tab.url.clone();
                    }
                }
                self.request_repaint();
            }
            EngineEvent::NewWindow { url, .. } => {
                self.open_tab(url);
            }
            EngineEvent::NavigationBlocked { url, .. } => {
                self.status(format!("Ferrox Shield blocked: {url}"));
            }
        }
    }

    fn handle_async_event(&mut self, event: AsyncEvent) {
        match event {
            AsyncEvent::TurboReady {
                tab_id,
                seq,
                result,
            } => {
                let Some((requested_url, expected)) = self.pending_turbo.get(&tab_id).cloned()
                else {
                    return;
                };
                if expected != seq {
                    return;
                }
                self.pending_turbo.remove(&tab_id);

                match *result {
                    Ok(document) => {
                        self.turbo_current
                            .insert(tab_id, document.final_url.clone());
                        self.turbo_docs.insert(tab_id, document.clone());
                        if let Err(err) = self.engine.load_html(tab_id, &document.html) {
                            tracing::warn!("turbo html load failed: {err}");
                        }
                        if let Some(tab) = self.tabs.find_mut(tab_id) {
                            tab.loading = false;
                            tab.title = document.title.clone();
                            tab.url = document.final_url.clone();
                        }
                        if self.active_tab_id() == Some(tab_id) {
                            self.chrome.address = document.final_url.clone();
                            self.update_tab_title();
                        }
                        if self.active_tab_id() == Some(tab_id) {
                            self.push_profiler_stats();
                        }
                    }
                    Err(err) => {
                        tracing::warn!("turbo reader failed for {requested_url}: {err}");
                        self.status(format!(
                            "Turbo reader could not parse this page ({err}); opened the full site"
                        ));
                        if let Some(tab) = self.tabs.find_mut(tab_id) {
                            tab.compute_mode = ComputeMode::Balanced;
                            tab.loading = false;
                        }
                        self.engine.destroy(tab_id);
                        self.navigate_url(tab_id, requested_url);
                    }
                }
                self.request_repaint();
            }
            AsyncEvent::UpdateChecked(result) => {
                self.update_in_flight = false;
                self.last_update_check = Some(Utc::now());
                self.update_view.last_check = self
                    .last_update_check
                    .map(|t| t.format("%Y-%m-%d %H:%M UTC").to_string())
                    .unwrap_or_default();

                match *result {
                    Ok(Some((manifest, asset))) => {
                        let version = manifest.version.clone();
                        self.update_view.available_version = Some(version.clone());
                        self.pending_update = Some((manifest, asset));
                        self.status(format!("Ferrox {version} is available"));
                    }
                    Ok(None) => {
                        self.update_view.available_version = None;
                        self.pending_update = None;
                        self.status("Ferrox is up to date");
                    }
                    Err(err) => {
                        self.status(format!("Update check failed: {err}"));
                    }
                }
                self.refresh_internal_page_if("ferrox://updates");
                self.request_repaint();
            }
            AsyncEvent::UpdateDownloaded(result) => match *result {
                Ok(path) => {
                    self.status("Installing update…");
                    match ferrox_updater::apply_update(&path) {
                        Ok(_) => {
                            if let Err(err) = ferrox_updater::relaunch() {
                                self.status(format!("Relaunch failed: {err}"));
                                return;
                            }
                            self.should_exit = true;
                        }
                        Err(err) => self.status(format!("Update install failed: {err}")),
                    }
                }
                Err(err) => self.status(format!("Update download failed: {err}")),
            },
        }
    }

    fn handle_ipc(&mut self, tab_id: TabId, payload: &str) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) else {
            tracing::warn!("invalid IPC payload");
            return;
        };
        let cmd = value
            .get("cmd")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let string_arg = |key: &str| {
            value
                .get(key)
                .and_then(|v| v.as_str())
                .map(str::to_string)
        };
        let int_arg = |key: &str| value.get(key).and_then(|v| v.as_i64());
        let bool_arg = |key: &str| value.get(key).and_then(|v| v.as_bool());

        match cmd {
            "navigate" => {
                if let Some(url) = string_arg("url") {
                    self.navigate_url(tab_id, url);
                    self.request_repaint();
                }
            }
            "search" => {
                if let Some(query) = string_arg("query") {
                    let url = ferrox_search::build_search_url(self.config.default_search, &query);
                    self.navigate_url(tab_id, url);
                    self.request_repaint();
                }
            }
            "full_mode" => {
                let url = string_arg("url").or_else(|| {
                    self.tabs
                        .tabs()
                        .iter()
                        .find(|t| t.id == tab_id)
                        .map(|t| t.url.clone())
                });
                if let Some(url) = url {
                    if let Some(tab) = self.tabs.find_mut(tab_id) {
                        tab.compute_mode = ComputeMode::Full;
                    }
                    self.engine.destroy(tab_id);
                    self.turbo_docs.remove(&tab_id);
                    self.turbo_current.remove(&tab_id);
                    self.turbo_stack.remove(&tab_id);
                    self.navigate_url(tab_id, url);
                    self.request_repaint();
                }
            }
            "shortcut" => {
                if let Some(action) = string_arg("action") {
                    self.handle_shortcut(&action);
                }
            }
            "set_search_engine" => {
                if let Some(raw) = string_arg("value") {
                    let engine = match raw.as_str() {
                        "google" => SearchEngineId::Google,
                        "brave" => SearchEngineId::Brave,
                        "startpage" => SearchEngineId::Startpage,
                        "bing" => SearchEngineId::Bing,
                        _ => SearchEngineId::DuckDuckGo,
                    };
                    self.config.default_search = engine;
                    self.omnibox.set_default_engine(engine);
                    self.save_config();
                    self.status("Search engine updated");
                }
            }
            "set_homepage" => {
                if let Some(home) = string_arg("value") {
                    self.config.homepage = home;
                    self.save_config();
                    self.status("Homepage updated");
                }
            }
            "set_compute_mode" => {
                if let Some(raw) = string_arg("value") {
                    let mode = match raw.as_str() {
                        "turbo" => ComputeMode::Turbo,
                        "eco" => ComputeMode::Eco,
                        "full" => ComputeMode::Full,
                        _ => ComputeMode::Balanced,
                    };
                    self.apply_compute_mode(mode);
                }
            }
            "set_hibernate" => {
                if let Some(secs) = int_arg("value") {
                    let secs = secs.clamp(30, 86_400) as u64;
                    self.config.hibernate_after_secs = secs;
                    self.hibernation = HibernationController::new(
                        secs,
                        self.config.max_ram_percent_before_hibernate,
                    );
                    self.save_config();
                    self.status(format!("Tabs hibernate after {secs}s"));
                }
            }
            "set_shield" => {
                if let Some(enabled) = bool_arg("value") {
                    self.config.shield_enabled = enabled;
                    self.engine_shared.set_shield_enabled(enabled);
                    self.save_config();
                    self.status(if enabled {
                        "Ferrox Shield enabled"
                    } else {
                        "Ferrox Shield disabled"
                    });
                }
            }
            "set_theme" => {
                if let Some(raw) = string_arg("value") {
                    self.config.theme = match raw.as_str() {
                        "light" => ThemeMode::Light,
                        "system" => ThemeMode::System,
                        _ => ThemeMode::Dark,
                    };
                    self.apply_theme(self.config.theme);
                    self.save_config();
                }
            }
            "set_update_channel" => {
                if let Some(raw) = string_arg("value") {
                    let channel = match raw.as_str() {
                        "beta" => UpdateChannel::Beta,
                        "nightly" => UpdateChannel::Nightly,
                        _ => UpdateChannel::Stable,
                    };
                    self.config.update_channel = channel;
                    self.update_view.channel = channel.as_str().to_string();
                    self.save_config();
                    self.status(format!("Release channel: {}", channel.as_str()));
                }
            }
            "set_auto_update" => {
                if let Some(enabled) = bool_arg("value") {
                    self.config.auto_check_updates = enabled;
                    self.save_config();
                }
            }
            "check_update" => self.start_update_check(),
            "install_update" => self.install_update(),
            "open_notes" => {
                if let Some((manifest, _)) = &self.pending_update {
                    let url = manifest.notes_url.clone();
                    self.navigate_url(tab_id, url);
                } else {
                    self.open_internal("ferrox://updates");
                    self.status("No release notes are available yet");
                }
            }
            "profiler_ready" => {
                self.last_profiler = Instant::now();
                self.push_profiler_stats();
            }
            "search_history" => {
                if let Some(query) = string_arg("query") {
                    let db = self.active_db();
                    let hits = db.search_history(&query, 300).unwrap_or_default();
                    let list: Vec<serde_json::Value> = hits
                        .iter()
                        .map(|h| {
                            serde_json::json!({
                                "id": h.id,
                                "url": h.url,
                                "title": h.title,
                                "visit_count": h.visit_count,
                            })
                        })
                        .collect();
                    self.evaluate_json_list(
                        tab_id,
                        "__ferroxSetHistory",
                        serde_json::to_string(&list).unwrap_or_else(|_| "[]".into()),
                    );
                }
            }
            "delete_history" => {
                if let Some(id) = int_arg("id") {
                    let db = self.active_db();
                    let _ = db.delete_history_entry(id);
                    self.refresh_internal_page_if("ferrox://history");
                }
            }
            "clear_history" => {
                let db = self.active_db();
                let _ = db.clear_history();
                self.refresh_internal_page_if("ferrox://history");
            }
            "open_history" => {
                if let Some(url) = string_arg("url") {
                    self.navigate_url(tab_id, url);
                }
            }
            "search_bookmarks" => {
                if let Some(query) = string_arg("query") {
                    let hits = self
                        .standard_db
                        .search_bookmarks(&query, 300)
                        .unwrap_or_default();
                    let list: Vec<serde_json::Value> = hits
                        .iter()
                        .map(|b| {
                            serde_json::json!({
                                "id": b.id,
                                "url": b.url,
                                "title": b.title,
                                "folder": b.folder,
                            })
                        })
                        .collect();
                    self.evaluate_json_list(
                        tab_id,
                        "__ferroxSetBookmarks",
                        serde_json::to_string(&list).unwrap_or_else(|_| "[]".into()),
                    );
                }
            }
            "delete_bookmark" => {
                if let Some(id) = int_arg("id") {
                    let _ = self.standard_db.remove_bookmark(id);
                    self.refresh_internal_page_if("ferrox://bookmarks");
                }
            }
            "export_bookmarks" => {
                match self.standard_db.export_bookmarks_html() {
                    Ok(html) => {
                        let target = self.data_dir.join("bookmarks.html");
                        match std::fs::write(&target, html) {
                            Ok(()) => {
                                self.status(format!("Bookmarks exported to {}", target.display()))
                            }
                            Err(err) => self.status(format!("Export failed: {err}")),
                        }
                    }
                    Err(err) => self.status(format!("Export failed: {err}")),
                }
            }
            "delete_download" => {
                if let Some(url_or_id) = string_arg("id") {
                    let _ = self.active_db().remove_download(&url_or_id);
                    self.refresh_internal_page_if("ferrox://downloads");
                }
            }
            "open_download" => {
                if let Some(id) = string_arg("id") {
                    let items = self.active_db().list_downloads().unwrap_or_default();
                    if let Some(item) = items.into_iter().find(|d| d.id == id) {
                        open_path(&item.file_path);
                    }
                }
            }
            other => {
                tracing::debug!("unhandled IPC command: {other}");
            }
        }
    }

    fn evaluate_json_list(&self, tab_id: TabId, function: &str, json: String) {
        let js = format!("window.{function} && window.{function}({json});");
        if let Err(err) = self.engine.evaluate(tab_id, &js) {
            tracing::warn!("failed to evaluate {function}: {err}");
        }
    }

    fn handle_shortcut(&mut self, action: &str) {
        match action {
            "focus_address" => self.focus_address(),
            "new_tab" => self.new_tab_home(),
            "reopen_tab" => self.reopen_tab(),
            "close_tab" => self.close_active_tab(),
            "reload" => self.reload_active(),
            "bookmark" => self.toggle_bookmark(),
            "history" => self.open_internal("ferrox://history"),
            "downloads" => self.open_internal("ferrox://downloads"),
            "incognito" => self.toggle_incognito(),
            other => tracing::debug!("unknown shortcut: {other}"),
        }
    }

    fn focus_address(&mut self) {
        self.chrome.focus_address = true;
        self.address_editing = false;
        if let Some(window) = self.window.clone() {
            window.focus();
        }
        self.request_repaint();
    }

    fn toggle_incognito(&mut self) {
        self.update_view();
        self.save_session();
    }

    // ---------------------------------------------------------------------
    // Updates
    // ---------------------------------------------------------------------

    fn start_update_check(&mut self) {
        if self.update_in_flight {
            return;
        }
        if self.config.manifest_url.trim().is_empty() {
            self.status("No update server is configured");
            return;
        }
        if self.config.manifest_url.contains(".invalid") {
            self.update_checked_once = true;
            if !self.config.auto_check_updates {
                self.status("Updates are not configured for this build");
            }
            return;
        }

        self.update_in_flight = true;
        self.update_checked_once = true;
        self.status("Checking for updates…");

        let client = self.net.raw_client().clone();
        let manifest_url = self.config.manifest_url.clone();
        let current = self.version.clone();
        let channel = self.config.update_channel;
        let tx = self.async_tx.clone();
        self.runtime.spawn(async move {
            let result = ferrox_updater::check_for_update(
                &client,
                &manifest_url,
                &current,
                channel,
            )
            .await;
            let _ = tx.send(AsyncEvent::UpdateChecked(Box::new(result)));
        });
    }

    fn install_update(&mut self) {
        let Some((_, asset)) = self.pending_update.clone() else {
            self.status("No update is available to install");
            return;
        };
        let public_key = option_env!("FERROX_UPDATE_PUBLIC_KEY").unwrap_or_default();
        if public_key.trim().is_empty() {
            self.status(
                "This build has no update signing key; install a signed release from your distributor",
            );
            return;
        }

        self.status("Downloading and verifying update…");
        let client = self.net.raw_client().clone();
        let staging = self.data_dir.join("updates");
        let tx = self.async_tx.clone();
        let key = public_key.to_string();
        self.runtime.spawn(async move {
            let result = ferrox_updater::download_and_verify(
                &client,
                &asset,
                &key,
                &staging,
            )
            .await;
            let _ = tx.send(AsyncEvent::UpdateDownloaded(Box::new(
                result.map(|download| download.path),
            )));
        });
    }

    // ---------------------------------------------------------------------
    // Maintenance
    // ---------------------------------------------------------------------

    fn tick_maintenance(&mut self) {
        let now = Instant::now();

        if now.duration_since(self.last_hibernate) >= Duration::from_secs(30) {
            self.last_hibernate = now;
            let hibernated = self.hibernation.sweep(&mut self.tabs);
            for id in hibernated {
                let _ = self.engine.destroy(id);
                self.turbo_docs.remove(&id);
                self.turbo_current.remove(&id);
                self.pending_turbo.remove(&id);
            }
            if self.engine.view_count() > self.tabs.tabs().len() {
                self.save_session();
            }
        }

        if now.duration_since(self.last_session_save) >= Duration::from_secs(30) {
            self.last_session_save = now;
            self.save_session();
        }

        let on_profiler = self
            .tabs
            .active_tab()
            .map(|t| t.url.starts_with("ferrox://profiler"))
            .unwrap_or(false);
        if on_profiler && now.duration_since(self.last_profiler) >= Duration::from_secs(1) {
            self.last_profiler = now;
            self.push_profiler_stats();
        }

        if self.config.auto_check_updates && !self.update_checked_once {
            self.start_update_check();
        }
    }

    fn refresh_internal_page_if(&mut self, prefix: &str) {
        let Some((id, url)) = self
            .tabs
            .active_tab()
            .map(|t| (t.id, t.url.clone()))
        else {
            return;
        };
        if !url.starts_with(prefix) {
            return;
        }
        let html = render_internal(&url, &self.internal_context(None));
        self.load_html(id, &url, html);
    }

    // ---------------------------------------------------------------------
    // Rendering
    // ---------------------------------------------------------------------

    fn apply_layout(&mut self) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let size = window.inner_size().to_logical::<f64>(window.scale_factor());
        let chrome_height = chrome::chrome_height(&self.chrome) as f64;
        let height = (size.height - chrome_height).max(1.0);
        let width = size.width.max(1.0);
        let layout = (width, height, chrome_height);
        if (self.layout.0 - layout.0).abs() > 0.5
            || (self.layout.1 - layout.1).abs() > 0.5
            || (self.layout.2 - layout.2).abs() > 0.5
        {
            self.layout = layout;
            self.engine
                .set_layout_bounds(bounds(0.0, chrome_height, width, height));
        }
    }

    fn handle_action(&mut self, action: ChromeAction) {
        match action {
            ChromeAction::NewTab => self.new_tab_home(),
            ChromeAction::ActivateTab(id) => self.switch_tab(id),
            ChromeAction::CloseTab(id) => {
                if self.active_tab_id() == Some(id) {
                    self.close_active_tab();
                } else {
                    let _ = self.engine.destroy(id);
                    self.turbo_docs.remove(&id);
                    self.turbo_current.remove(&id);
                    self.turbo_stack.remove(&id);
                    self.tabs.close_tab(id);
                    self.request_repaint();
                }
            }
            ChromeAction::Navigate(target) => {
                self.navigate_address(&target);
                self.request_repaint();
            }
            ChromeAction::AddressChanged => {
                self.address_editing = true;
                self.refresh_suggestions();
                self.request_repaint();
            }
            ChromeAction::Back => self.go_back(),
            ChromeAction::Forward => self.go_forward(),
            ChromeAction::Reload => self.reload_active(),
            ChromeAction::Home => {
                let home = self.config.homepage.clone();
                self.navigate_tab_url(home);
            }
            ChromeAction::ToggleShield => {
                self.config.shield_enabled = !self.config.shield_enabled;
                self.engine_shared
                    .set_shield_enabled(self.config.shield_enabled);
                self.save_config();
                self.status(if self.config.shield_enabled {
                    "Ferrox Shield enabled"
                } else {
                    "Ferrox Shield disabled"
                });
            }
            ChromeAction::CycleComputeMode => {
                let next = match self.config.default_compute_mode {
                    ComputeMode::Turbo => ComputeMode::Eco,
                    ComputeMode::Eco => ComputeMode::Balanced,
                    ComputeMode::Balanced => ComputeMode::Full,
                    ComputeMode::Full => ComputeMode::Turbo,
                };
                self.apply_compute_mode(next);
            }
            ChromeAction::ToggleIncognito => self.toggle_incognito(),
            ChromeAction::OpenInternal(url) => self.open_internal(&url),
            ChromeAction::CheckUpdates => self.start_update_check(),
            ChromeAction::Quit => self.should_exit = true,
        }
    }

    fn detect_global_action(raw: &egui::RawInput) -> Option<GlobalAction> {
        for event in &raw.events {
            if let egui::Event::Key {
                key,
                pressed: true,
                repeat: false,
                modifiers,
                ..
            } = event
            {
                if !modifiers.command {
                    continue;
                }
                let action = match key {
                    egui::Key::L => GlobalAction::FocusAddress,
                    egui::Key::T if modifiers.shift => GlobalAction::ReopenTab,
                    egui::Key::T => GlobalAction::NewTab,
                    egui::Key::W => GlobalAction::CloseTab,
                    egui::Key::R => GlobalAction::Reload,
                    egui::Key::D => GlobalAction::Bookmark,
                    egui::Key::H => GlobalAction::History,
                    egui::Key::J => GlobalAction::Downloads,
                    egui::Key::N if modifiers.shift => GlobalAction::Incognito,
                    _ => continue,
                };
                return Some(action);
            }
        }
        None
    }

    fn handle_global_action(&mut self, action: GlobalAction) {
        match action {
            GlobalAction::FocusAddress => self.focus_address(),
            GlobalAction::NewTab => self.new_tab_home(),
            GlobalAction::ReopenTab => self.reopen_tab(),
            GlobalAction::CloseTab => self.close_active_tab(),
            GlobalAction::Reload => self.reload_active(),
            GlobalAction::Bookmark => self.toggle_bookmark(),
            GlobalAction::History => self.open_internal("ferrox://history"),
            GlobalAction::Downloads => self.open_internal("ferrox://downloads"),
            GlobalAction::Incognito => self.toggle_incognito(),
        }
    }

    fn render(&mut self) {
        let Some(window) = self.window.clone() else {
            return;
        };
        if self.painter.is_none() || self.egui_state.is_none() {
            return;
        }

        self.drain_events();
        self.sync_chrome();
        self.tick_maintenance();

        let raw_input = match self.egui_state.as_mut() {
            Some(state) => state.take_egui_input(&window),
            None => return,
        };
        let global_action = Self::detect_global_action(&raw_input);

        let mut action: Option<ChromeAction> = None;
        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            action = chrome::draw(ctx, &mut self.chrome);
        });

        if let Some(state) = self.egui_state.as_mut() {
            state.handle_platform_output(&window, full_output.platform_output);
        }

        self.apply_layout();

        if let Some(action) = action {
            self.handle_action(action);
        }
        if let Some(global) = global_action {
            self.handle_global_action(global);
        }

        let pixels_per_point = full_output.pixels_per_point;
        let clipped = self
            .egui_ctx
            .tessellate(full_output.shapes, pixels_per_point);
        let mut textures = full_output.textures_delta;
        let repaint_delay = full_output
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .map(|viewport| viewport.repaint_delay)
            .unwrap_or(Duration::MAX);

        if let Some(painter) = self.painter.as_mut() {
            painter.paint_and_update_textures(
                egui::ViewportId::ROOT,
                pixels_per_point,
                [11.0 / 255.0, 13.0 / 255.0, 16.0 / 255.0, 1.0],
                &clipped,
                &mut textures,
                Vec::new(),
                &window,
            );
        }

        if repaint_delay == Duration::ZERO {
            self.repaint_deadline = None;
            window.request_redraw();
        } else if repaint_delay < Duration::from_secs(3600) {
            self.repaint_deadline = Some(Instant::now() + repaint_delay);
        } else {
            self.repaint_deadline = None;
        }
    }

    fn shutdown(&mut self, event_loop: &ActiveEventLoop) {
        if self.shutting_down {
            return;
        }
        self.shutting_down = true;
        self.save_config();
        self.save_session();
        self.engine.destroy_all();
        if let Some(painter) = self.painter.as_mut() {
            painter.destroy();
        }
        event_loop.exit();
    }
}

impl ApplicationHandler<AppEvent> for FerroxApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title("Ferrox")
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 820.0))
            .with_min_inner_size(winit::dpi::LogicalSize::new(720.0, 480.0))
            .with_window_icon(Some(window_icon()));

        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(err) => {
                tracing::error!("failed to create window: {err}");
                event_loop.exit();
                return;
            }
        };
        self.window = Some(window.clone());
        self.chrome.memory_mb = 42.0;

        let mut painter = self.runtime.block_on(egui_wgpu::winit::Painter::new(
            self.egui_ctx.clone(),
            egui_wgpu::WgpuConfiguration::default(),
            false,
            egui_wgpu::RendererOptions::default(),
        ));

        if let Err(err) = self
            .runtime
            .block_on(painter.set_window(egui::ViewportId::ROOT, Some(window.clone())))
        {
            tracing::error!("failed to initialise GPU surface: {err}");
            event_loop.exit();
            return;
        }
        let max_texture_side = painter.max_texture_side();
        self.painter = Some(painter);

        ferrox_ui::theme::apply(&self.egui_ctx);
        self.apply_theme(self.config.theme);

        self.egui_state = Some(egui_winit::State::new(
            self.egui_ctx.clone(),
            egui::ViewportId::ROOT,
            window.as_ref(),
            Some(window.scale_factor() as f32),
            window.theme(),
            max_texture_side,
        ));

        self.bootstrap();
        window.request_redraw();
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::Wake => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.window.as_ref().map(|window| window.id()) != Some(window_id) {
            return;
        }

        if matches!(&event, WindowEvent::RedrawRequested) {
            self.render();
            if self.should_exit {
                self.shutdown(event_loop);
            }
            return;
        }

        if let (Some(state), Some(window)) = (self.egui_state.as_mut(), self.window.clone()) {
            let response = state.on_window_event(&window, &event);
            if response.repaint {
                self.egui_ctx.request_repaint();
            }
        }

        match event {
            WindowEvent::CloseRequested => self.shutdown(event_loop),
            WindowEvent::Resized(size) => {
                if let Some(painter) = self.painter.as_mut() {
                    if let (Some(width), Some(height)) =
                        (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                    {
                        painter.on_window_resized(egui::ViewportId::ROOT, width, height);
                    }
                }
                self.layout = (f64::NAN, f64::NAN, f64::NAN);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                self.layout = (f64::NAN, f64::NAN, f64::NAN);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.should_exit {
            self.shutdown(event_loop);
            return;
        }

        let now = Instant::now();
        let profiler_active = self
            .tabs
            .active_tab()
            .map(|t| t.url.starts_with("ferrox://profiler"))
            .unwrap_or(false);
        let interval = if profiler_active {
            Duration::from_secs(1)
        } else {
            Duration::from_secs(15)
        };

        if now >= self.next_tick {
            self.next_tick = now + interval;
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }

        let mut wake_at = self.next_tick;
        if let Some(deadline) = self.repaint_deadline {
            if deadline <= now {
                self.repaint_deadline = None;
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            } else if deadline < wake_at {
                wake_at = deadline;
            }
        }

        event_loop.set_control_flow(ControlFlow::WaitUntil(wake_at));
    }
}

fn window_icon() -> Icon {
    let size = 64u32;
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let dx = (x as f32 - size as f32 / 2.0) / (size as f32 / 2.0);
            let dy = (y as f32 - size as f32 / 2.0) / (size as f32 / 2.0);
            let distance = (dx * dx + dy * dy).sqrt();
            let inside = distance <= 0.96;
            let (r, g, b, a) = if inside {
                let t = (dy + 1.0) * 0.5;
                (
                    255,
                    (90.0 + 95.0 * t) as u8,
                    (45.0 + 60.0 * t) as u8,
                    255,
                )
            } else {
                (0, 0, 0, 0)
            };
            rgba.extend_from_slice(&[r, g, b, a]);
        }
    }
    Icon::from_rgba(rgba, size, size).unwrap_or_else(|_| {
        Icon::from_rgba(vec![255, 106, 61, 255], 1, 1).expect("1x1 icon")
    })
}

fn open_path(path: &str) {
    #[cfg(windows)]
    let mut command = {
        let mut command = std::process::Command::new("explorer");
        command.arg(path);
        command
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = std::process::Command::new("open");
        command.arg(path);
        command
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = std::process::Command::new("xdg-open");
        command.arg(path);
        command
    };

    if let Err(err) = command.spawn() {
        tracing::warn!("failed to open {}: {err}", path);
    }
}
