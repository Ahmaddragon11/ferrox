// SPDX-License-Identifier: MIT OR Apache-2.0

use ferrox_core::{ComputeMode, FerroxError, Result, TabId};
use ferrox_governor::GovernorPolicy;
use ferrox_shield::{DEFAULT_RULES, ShieldEngine};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc::Sender};
use wry::dpi::{LogicalPosition, LogicalSize};
use wry::{Rect, WebView, WebViewBuilder};

#[derive(Debug, Clone)]
pub enum EngineEvent {
    Ipc {
        tab_id: TabId,
        payload: String,
    },
    TitleChanged {
        tab_id: TabId,
        title: String,
    },
    PageLoad {
        tab_id: TabId,
        url: String,
        finished: bool,
    },
    NewWindow {
        tab_id: TabId,
        url: String,
    },
    NavigationBlocked {
        tab_id: TabId,
        url: String,
    },
    /// A download was accepted by the webview and is being written to disk.
    DownloadStarted {
        tab_id: TabId,
        url: String,
        path: String,
    },
    /// A download finished (successfully or not).
    DownloadCompleted {
        tab_id: TabId,
        url: String,
        path: Option<String>,
        success: bool,
    },
}

pub struct EngineShared {
    pub shield: Arc<ShieldEngine>,
    shield_enabled: AtomicBool,
    tx: Mutex<Sender<EngineEvent>>,
    wake: Arc<dyn Fn() + Send + Sync>,
}

impl EngineShared {
    pub fn new(
        shield: Arc<ShieldEngine>,
        tx: Sender<EngineEvent>,
        wake: Arc<dyn Fn() + Send + Sync>,
    ) -> Self {
        Self {
            shield,
            shield_enabled: AtomicBool::new(true),
            tx: Mutex::new(tx),
            wake,
        }
    }

    pub fn shield_enabled(&self) -> bool {
        self.shield_enabled.load(Ordering::Relaxed)
    }

    pub fn set_shield_enabled(&self, enabled: bool) {
        self.shield_enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn emit(&self, event: EngineEvent) {
        if let Ok(guard) = self.tx.lock() {
            let _ = guard.send(event);
        }
        (self.wake)();
    }
}

pub struct WebViewEngine {
    shared: Arc<EngineShared>,
    views: HashMap<TabId, WebView>,
    bounds: Rect,
    downloads_dir: PathBuf,
}

impl WebViewEngine {
    pub fn new(shared: Arc<EngineShared>, bounds: Rect, downloads_dir: PathBuf) -> Self {
        Self {
            shared,
            views: HashMap::new(),
            bounds,
            downloads_dir,
        }
    }

    pub fn has_view(&self, tab: TabId) -> bool {
        self.views.contains_key(&tab)
    }

    pub fn view_count(&self) -> usize {
        self.views.len()
    }

    pub fn downloads_dir(&self) -> &Path {
        &self.downloads_dir
    }

    pub fn set_layout_bounds(&mut self, bounds: Rect) {
        self.bounds = bounds;
        for view in self.views.values() {
            let _ = view.set_bounds(bounds);
        }
    }

    pub fn ensure_view<W: raw_window_handle::HasWindowHandle>(
        &mut self,
        tab: TabId,
        window: &W,
        mode: ComputeMode,
        initial_url: &str,
        visible: bool,
    ) -> Result<()> {
        if self.views.contains_key(&tab) {
            return Ok(());
        }

        let shared = self.shared.clone();
        let ipc_shared = self.shared.clone();
        let title_shared = self.shared.clone();
        let load_shared = self.shared.clone();
        let nav_shared = self.shared.clone();
        let new_window_shared = self.shared.clone();
        let download_shared = self.shared.clone();
        let download_completed_shared = self.shared.clone();
        let downloads_dir = self.downloads_dir.clone();

        let policy = GovernorPolicy::for_mode(mode).injection_script();
        let inject_shield = self.shared.shield_enabled();
        let shield_script = if inject_shield {
            dom_shield_script()
        } else {
            String::new()
        };

        let builder = WebViewBuilder::new()
            .with_bounds(self.bounds)
            .with_visible(visible)
            .with_focused(visible)
            .with_devtools(true)
            .with_background_color((13, 17, 23, 255))
            .with_url(initial_url)
            .with_initialization_script(policy)
            .with_initialization_script(shield_script)
            .with_initialization_script(shortcut_script())
            .with_initialization_script(find_script())
            .with_initialization_script(mute_script())
            .with_ipc_handler(move |request| {
                ipc_shared.emit(EngineEvent::Ipc {
                    tab_id: tab,
                    payload: request.body().clone(),
                });
            })
            .with_document_title_changed_handler(move |title| {
                if !title.trim().is_empty() {
                    title_shared.emit(EngineEvent::TitleChanged { tab_id: tab, title });
                }
            })
            .with_on_page_load_handler(move |event, url| {
                let finished = matches!(event, wry::PageLoadEvent::Finished);
                load_shared.emit(EngineEvent::PageLoad {
                    tab_id: tab,
                    url,
                    finished,
                });
            })
            .with_navigation_handler(move |url| {
                let parsed = url::Url::parse(&url);
                let allowed = match parsed {
                    Ok(ref u) => matches!(u.scheme(), "http" | "https" | "about"),
                    Err(_) => false,
                };
                if !allowed {
                    nav_shared.emit(EngineEvent::NavigationBlocked {
                        tab_id: tab,
                        url: url.clone(),
                    });
                    return false;
                }
                if nav_shared.shield_enabled() && nav_shared.shield.is_blocked(&url, &url) {
                    nav_shared.emit(EngineEvent::NavigationBlocked {
                        tab_id: tab,
                        url: url.clone(),
                    });
                    return false;
                }
                true
            })
            .with_new_window_req_handler(move |url, _features| {
                new_window_shared.emit(EngineEvent::NewWindow { tab_id: tab, url });
                wry::NewWindowResponse::Deny
            })
            .with_download_started_handler(move |url, path| {
                let target = unique_download_path(&downloads_dir, &url, Some(path.as_path()));
                *path = target.clone();
                download_shared.emit(EngineEvent::DownloadStarted {
                    tab_id: tab,
                    url,
                    path: target.to_string_lossy().to_string(),
                });
                true
            })
            .with_download_completed_handler(move |url, path, success| {
                download_completed_shared.emit(EngineEvent::DownloadCompleted {
                    tab_id: tab,
                    url,
                    path: path.map(|p| p.to_string_lossy().to_string()),
                    success,
                });
            });

        let view = builder
            .build(window)
            .map_err(|e| FerroxError::Engine(format!("failed to create webview: {e}")))?;
        let _ = shared; // keep the shared handle alive for the duration of the view

        self.views.insert(tab, view);
        Ok(())
    }

    pub fn navigate(&mut self, tab: TabId, url: &str) -> Result<()> {
        let view = self.view(tab)?;
        view.load_url(url)
            .map_err(|e| FerroxError::Engine(e.to_string()))
    }

    pub fn load_html(&mut self, tab: TabId, html: &str) -> Result<()> {
        let view = self.view(tab)?;
        view.load_html(html)
            .map_err(|e| FerroxError::Engine(e.to_string()))
    }

    pub fn evaluate(&self, tab: TabId, js: &str) -> Result<()> {
        let view = self.view(tab)?;
        view.evaluate_script(js)
            .map_err(|e| FerroxError::Engine(e.to_string()))
    }

    pub fn reload(&self, tab: TabId) -> Result<()> {
        self.view(tab)?
            .reload()
            .map_err(|e| FerroxError::Engine(e.to_string()))
    }

    pub fn go_back(&self, tab: TabId) -> Result<()> {
        self.view(tab)?
            .go_back()
            .map_err(|e| FerroxError::Engine(e.to_string()))
    }

    pub fn go_forward(&self, tab: TabId) -> Result<()> {
        self.view(tab)?
            .go_forward()
            .map_err(|e| FerroxError::Engine(e.to_string()))
    }

    pub fn can_go_back(&self, tab: TabId) -> Result<bool> {
        self.view(tab)?
            .can_go_back()
            .map_err(|e| FerroxError::Engine(e.to_string()))
    }

    pub fn can_go_forward(&self, tab: TabId) -> Result<bool> {
        self.view(tab)?
            .can_go_forward()
            .map_err(|e| FerroxError::Engine(e.to_string()))
    }

    pub fn open_devtools(&self, tab: TabId) {
        if let Some(view) = self.views.get(&tab) {
            view.open_devtools();
        }
    }

    pub fn current_url(&self, tab: TabId) -> Option<String> {
        self.views.get(&tab).and_then(|v| v.url().ok())
    }

    pub fn set_visible(&self, tab: TabId, visible: bool) -> Result<()> {
        self.view(tab)?
            .set_visible(visible)
            .map_err(|e| FerroxError::Engine(e.to_string()))
    }

    pub fn focus(&self, tab: TabId) -> Result<()> {
        self.view(tab)?
            .focus()
            .map_err(|e| FerroxError::Engine(e.to_string()))
    }

    /// Moves keyboard focus out of the page and back to the browser chrome.
    pub fn focus_parent(&self, tab: TabId) -> Result<()> {
        self.view(tab)?
            .focus_parent()
            .map_err(|e| FerroxError::Engine(e.to_string()))
    }

    /// Applies a zoom factor (`1.0` is 100%) to a tab.
    pub fn zoom(&self, tab: TabId, factor: f64) -> Result<()> {
        self.view(tab)?
            .zoom(factor)
            .map_err(|e| FerroxError::Engine(e.to_string()))
    }

    pub fn destroy(&mut self, tab: TabId) -> bool {
        self.views.remove(&tab).is_some()
    }

    pub fn destroy_all(&mut self) {
        self.views.clear();
    }

    fn view(&self, tab: TabId) -> Result<&WebView> {
        self.views
            .get(&tab)
            .ok_or_else(|| FerroxError::Engine("tab has no active webview".into()))
    }
}

pub fn bounds(x: f64, y: f64, width: f64, height: f64) -> Rect {
    Rect {
        position: LogicalPosition::new(x, y).into(),
        size: LogicalSize::new(width.max(1.0), height.max(1.0)).into(),
    }
}

/// Builds a collision-free destination for a download inside the download
/// directory. The file name is derived from the suggestion of the webview
/// (usually the `Content-Disposition` name) and falls back to the URL.
pub fn unique_download_path(dir: &Path, url: &str, suggested: Option<&Path>) -> PathBuf {
    let from_url = || {
        url::Url::parse(url)
            .ok()
            .and_then(|u| {
                u.path_segments()
                    .and_then(|mut segments| segments.rfind(|s| !s.is_empty()))
                    .map(str::to_string)
            })
            .unwrap_or_else(|| "download".to_string())
    };

    let raw_name = suggested
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().to_string())
        })
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(from_url);

    let safe = sanitize_file_name(&raw_name);
    let mut candidate = dir.join(&safe);
    let mut counter = 1usize;
    while candidate.exists() && counter < 1_000 {
        let path = Path::new(&safe);
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "download".to_string());
        let extension = path
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();
        candidate = dir.join(format!("{stem} ({counter}){extension}"));
        counter += 1;
    }
    candidate
}

/// Strips every character that is not safe in a file name on all platforms.
pub fn sanitize_file_name(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let cleaned = cleaned.trim().trim_matches('.').to_string();
    if cleaned.is_empty() {
        "download".to_string()
    } else {
        cleaned
    }
}

/// Page-level keyboard bridge so browser shortcuts keep working while the
/// native webview owns focus.
pub fn shortcut_script() -> String {
    r#"
(function(){
  if (window.__ferroxShortcuts) return;
  window.__ferroxShortcuts = true;
  document.addEventListener("keydown", function(e){
    if (!(e.ctrlKey || e.metaKey)) return;
    var k = (e.key || "").toLowerCase();
    var action = null;
    if (k === "l") action = "focus_address";
    else if (k === "f") action = "find";
    else if (k === "t" && e.shiftKey) action = "reopen_tab";
    else if (k === "t") action = "new_tab";
    else if (k === "w") action = "close_tab";
    else if (k === "r") action = "reload";
    else if (k === "d") action = "bookmark";
    else if (k === "h") action = "history";
    else if (k === "j") action = "downloads";
    else if (k === "n" && e.shiftKey) action = "incognito";
    else if (k === "=" || k === "+") action = "zoom_in";
    else if (k === "-") action = "zoom_out";
    else if (k === "0") action = "zoom_reset";
    if (action) {
      e.preventDefault();
      try { window.ipc.postMessage(JSON.stringify({ cmd: "shortcut", action: action })); } catch(err) {}
    }
  }, true);
})();
"#
    .to_string()
}

/// Find-in-page helper injected into every webview. It highlights every match
/// of the current query and keeps the browser UI in sync through IPC.
pub fn find_script() -> String {
    r#"
(function(){
  if (window.__ferroxFindReady) return;
  window.__ferroxFindReady = true;
  var STYLE_ID = "ferrox-find-style";
  var hits = [];
  var current = -1;

  function ensureStyle(){
    if (document.getElementById(STYLE_ID)) return;
    var style = document.createElement("style");
    style.id = STYLE_ID;
    style.textContent = ".ferrox-find-hit{background:#ffe066;color:#111;border-radius:2px}" +
      ".ferrox-find-current{background:#ff9f1c;color:#111;box-shadow:0 0 0 2px rgba(255,159,28,.6)}";
    (document.head || document.documentElement).appendChild(style);
  }

  function report(){
    try {
      window.ipc.postMessage(JSON.stringify({
        cmd: "find_results",
        count: hits.length,
        index: current < 0 ? 0 : current + 1
      }));
    } catch(e) {}
  }

  function clear(){
    var marks = document.querySelectorAll("span.ferrox-find-hit");
    for (var i = 0; i < marks.length; i++) {
      var mark = marks[i];
      var parent = mark.parentNode;
      if (!parent) continue;
      while (mark.firstChild) parent.insertBefore(mark.firstChild, mark);
      parent.removeChild(mark);
      parent.normalize();
    }
    hits = [];
    current = -1;
  }

  function collect(query){
    clear();
    if (!query) { report(); return 0; }
    var root = document.body || document.documentElement;
    if (!root) {
      document.addEventListener("DOMContentLoaded", function(){ window.__ferroxFind(query); }, { once: true });
      report();
      return 0;
    }
    ensureStyle();
    var needle = query.toLowerCase();
    var walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
      acceptNode: function(node){
        var name = node.parentNode ? node.parentNode.nodeName : "";
        if (name === "SCRIPT" || name === "STYLE" || name === "NOSCRIPT" || name === "TEXTAREA") {
          return NodeFilter.FILTER_REJECT;
        }
        if (!node.nodeValue || node.nodeValue.toLowerCase().indexOf(needle) === -1) {
          return NodeFilter.FILTER_REJECT;
        }
        return NodeFilter.FILTER_ACCEPT;
      }
    });
    var node;
    var limit = 2000;
    while ((node = walker.nextNode())) {
      if (hits.length >= limit) break;
      var rest = node.nodeValue;
      var fragment = document.createDocumentFragment();
      var index = rest.toLowerCase().indexOf(needle);
      while (index !== -1) {
        fragment.appendChild(document.createTextNode(rest.slice(0, index)));
        var mark = document.createElement("span");
        mark.className = "ferrox-find-hit";
        mark.textContent = rest.slice(index, index + query.length);
        fragment.appendChild(mark);
        hits.push(mark);
        rest = rest.slice(index + query.length);
        index = rest.toLowerCase().indexOf(needle);
      }
      fragment.appendChild(document.createTextNode(rest));
      if (node.parentNode) node.parentNode.replaceChild(fragment, node);
    }
    current = hits.length ? 0 : -1;
    if (current >= 0) {
      hits[current].classList.add("ferrox-find-current");
      hits[current].scrollIntoView({ block: "center" });
    }
    report();
    return hits.length;
  }

  window.__ferroxFind = function(query){ return collect(query || ""); };

  window.__ferroxFindNext = function(direction){
    if (!hits.length) { report(); return 0; }
    if (current >= 0) hits[current].classList.remove("ferrox-find-current");
    var step = direction < 0 ? -1 : 1;
    current = (current + step + hits.length) % hits.length;
    hits[current].classList.add("ferrox-find-current");
    hits[current].scrollIntoView({ block: "center" });
    report();
    return current + 1;
  };

  window.__ferroxFindClear = function(){ clear(); report(); };
})();
"#
    .to_string()
}

/// Mutes every media element of a tab, including elements added later on.
pub fn mute_script() -> String {
    r#"
(function(){
  if (window.__ferroxMuteReady) return;
  window.__ferroxMuteReady = true;
  function eachMedia(fn){
    var media = document.querySelectorAll("video, audio");
    for (var i = 0; i < media.length; i++) {
      try { fn(media[i]); } catch(e) {}
    }
  }
  window.__ferroxMute = function(){
    window.__ferroxMuted = true;
    eachMedia(function(m){
      if (m.getAttribute("data-ferrox-volume") === null) {
        m.setAttribute("data-ferrox-volume", String(m.volume));
      }
      m.muted = true;
      m.volume = 0;
    });
  };
  window.__ferroxUnmute = function(){
    window.__ferroxMuted = false;
    eachMedia(function(m){
      m.muted = false;
      var stored = parseFloat(m.getAttribute("data-ferrox-volume"));
      m.volume = isNaN(stored) ? 1 : stored;
    });
  };
  function start(){
    if (window.MutationObserver && document.documentElement) {
      new MutationObserver(function(){
        if (window.__ferroxMuted) window.__ferroxMute();
      }).observe(document.documentElement, { childList: true, subtree: true });
    }
    if (window.__ferroxMuted) window.__ferroxMute();
  }
  if (document.documentElement) start();
  else document.addEventListener("DOMContentLoaded", start, { once: true });
})();
"#
    .to_string()
}

/// Supplemental document-level neutralisation of known tracker hosts. This does
/// not replace network-level shielding; it stops dynamically injected scripts
/// from executing inside the webview.
pub fn dom_shield_script() -> String {
    let domains: Vec<String> = DEFAULT_RULES
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let host = line.strip_prefix("||")?.strip_suffix('^')?;
            if host.is_empty() {
                None
            } else {
                Some(format!("\"{host}\""))
            }
        })
        .collect();

    format!(
        r#"
(function(){{
  if (window.__ferroxDomShield) return;
  window.__ferroxDomShield = true;
  window.__ferroxBlocked = 0;
  var hosts = [{}];
  function hostOf(raw) {{
    try {{ var u = new URL(raw, location.href); return u.hostname; }} catch(e) {{ return ""; }}
  }}
  function isBlocked(raw) {{
    if (!raw) return false;
    var h = hostOf(raw);
    if (!h) return false;
    for (var i = 0; i < hosts.length; i++) {{
      var d = hosts[i];
      if (h === d || h.endsWith("." + d)) return true;
    }}
    return false;
  }}
  function strip(node) {{
    if (!node || node.nodeType !== 1) return;
    var attrs = ["src", "href"];
    for (var i = 0; i < attrs.length; i++) {{
      var value = node.getAttribute && node.getAttribute(attrs[i]);
      if (value && isBlocked(value)) {{
        window.__ferroxBlocked++;
        if (node.tagName === "SCRIPT" || node.tagName === "IFRAME") {{ node.remove(); return; }}
        if (node.tagName === "IMG" || node.tagName === "LINK") {{ node.remove(); return; }}
        node.setAttribute(attrs[i], "about:blank");
      }}
    }}
  }}
  function sweep(root) {{
    if (!root) return;
    if (root.querySelectorAll) {{
      var all = root.querySelectorAll("script[src],iframe[src],img[src],link[href]");
      for (var i = 0; i < all.length; i++) strip(all[i]);
    }}
    strip(root);
  }}
  var mo = new MutationObserver(function(muts) {{
    for (var i = 0; i < muts.length; i++) {{
      var m = muts[i];
      for (var j = 0; j < m.addedNodes.length; j++) sweep(m.addedNodes[j]);
    }}
  }});
  function start() {{
    sweep(document.documentElement);
    mo.observe(document.documentElement, {{ childList: true, subtree: true }});
  }}
  if (document.documentElement) start();
  else document.addEventListener("DOMContentLoaded", start, {{ once: true }});
}})();
"#,
        domains.join(",")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dom_shield_contains_known_hosts_and_guards() {
        let script = dom_shield_script();
        assert!(script.contains("\"doubleclick.net\""));
        assert!(script.contains("\"google-analytics.com\""));
        assert!(script.contains("__ferroxDomShield"));
        assert!(script.contains("MutationObserver"));
    }

    #[test]
    fn governor_script_embeds_mode() {
        let script = GovernorPolicy::for_mode(ComputeMode::Eco).injection_script();
        assert!(script.contains("\"eco\""));
        assert!(script.contains("disableWebgl"));
    }

    #[test]
    fn bounds_are_clamped_to_positive_size() {
        let rect = bounds(0.0, 92.0, 0.0, -10.0);
        assert!(matches!(rect.size, wry::dpi::Size::Logical(_)));
    }

    #[test]
    fn download_paths_are_unique_and_safe() {
        let dir = std::env::temp_dir().join("ferrox-download-test");
        std::fs::create_dir_all(&dir).unwrap();
        let first = unique_download_path(&dir, "https://example.com/files/report.pdf", None);
        assert_eq!(first.file_name().unwrap(), "report.pdf");

        std::fs::write(&first, b"x").unwrap();
        let second = unique_download_path(&dir, "https://example.com/files/report.pdf", None);
        assert_ne!(first, second);
        assert_eq!(second.file_name().unwrap(), "report (1).pdf");

        let traversal = unique_download_path(&dir, "https://example.com/../../etc/passwd", None);
        assert_eq!(traversal.parent().unwrap(), dir.as_path());
        assert!(!traversal.to_string_lossy().contains(".."));

        // A suggestion from the webview wins over the URL.
        let suggested = unique_download_path(
            &dir,
            "https://example.com/download?id=7",
            Some(Path::new("/tmp/holiday photo.png")),
        );
        assert_eq!(suggested.file_name().unwrap(), "holiday photo.png");

        std::fs::remove_file(&first).ok();
        std::fs::remove_file(&second).ok();
    }

    #[test]
    fn file_names_are_stripped_of_dangerous_characters() {
        assert_eq!(sanitize_file_name("a/b\\c.txt"), "a_b_c.txt");
        assert_eq!(sanitize_file_name("   "), "download");
        assert_eq!(sanitize_file_name("..."), "download");
    }

    #[test]
    fn find_script_exposes_the_browser_api() {
        let script = find_script();
        assert!(script.contains("__ferroxFind"));
        assert!(script.contains("__ferroxFindNext"));
        assert!(script.contains("ferrox-find-hit"));
        assert!(script.contains("find_results"));
    }

    #[test]
    fn shortcut_script_covers_find_and_zoom() {
        let script = shortcut_script();
        assert!(script.contains("\"find\""));
        assert!(script.contains("\"zoom_in\""));
        assert!(script.contains("\"zoom_reset\""));
    }
}
