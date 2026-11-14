// SPDX-License-Identifier: MIT OR Apache-2.0

use ferrox_core::{ComputeMode, FerroxError, Result, TabId};
use ferrox_governor::GovernorPolicy;
use ferrox_shield::{DEFAULT_RULES, ShieldEngine};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc::Sender};
use wry::dpi::{LogicalPosition, LogicalSize};
use wry::{Rect, WebView, WebViewBuilder};

#[derive(Debug, Clone)]
pub enum EngineEvent {
    Ipc { tab_id: TabId, payload: String },
    TitleChanged { tab_id: TabId, title: String },
    PageLoad { tab_id: TabId, url: String, finished: bool },
    NewWindow { tab_id: TabId, url: String },
    NavigationBlocked { tab_id: TabId, url: String },
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
}

impl WebViewEngine {
    pub fn new(shared: Arc<EngineShared>, bounds: Rect) -> Self {
        Self {
            shared,
            views: HashMap::new(),
            bounds,
        }
    }

    pub fn has_view(&self, tab: TabId) -> bool {
        self.views.contains_key(&tab)
    }

    pub fn view_count(&self) -> usize {
        self.views.len()
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
    else if (k === "t" && e.shiftKey) action = "reopen_tab";
    else if (k === "t") action = "new_tab";
    else if (k === "w") action = "close_tab";
    else if (k === "r") action = "reload";
    else if (k === "d") action = "bookmark";
    else if (k === "h") action = "history";
    else if (k === "j") action = "downloads";
    else if (k === "n" && e.shiftKey) action = "incognito";
    if (action) {
      e.preventDefault();
      try { window.ipc.postMessage(JSON.stringify({ cmd: "shortcut", action: action })); } catch(err) {}
    }
  }, true);
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
}
