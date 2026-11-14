// SPDX-License-Identifier: MIT OR Apache-2.0

use ferrox_shield::ShieldStats;
use ferrox_storage::{Bookmark, DownloadItem, HistoryEntry};

#[derive(Debug, Clone, Default)]
pub struct UpdateView {
    pub current_version: String,
    pub channel: String,
    pub last_check: String,
    pub available_version: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SettingsView {
    pub search_engine: String,
    pub homepage: String,
    pub theme: String,
    pub compute_mode: String,
    pub shield_enabled: bool,
    pub hibernate_after_secs: u64,
    pub update_channel: String,
    pub auto_check_updates: bool,
    pub restore_session: bool,
    /// Per-site compute overrides as `(host, mode)` pairs.
    pub per_site: Vec<(String, String)>,
}

#[derive(Debug, Clone, Default)]
pub struct TurboSummary {
    pub url: String,
    pub title: String,
    pub words: usize,
    pub read_minutes: usize,
    pub load_ms: u64,
}

/// Source code of a page, rendered by `ferrox://view-source`.
#[derive(Debug, Clone, Default)]
pub struct ViewSource {
    pub url: String,
    pub source: String,
    pub bytes: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone, Default)]
pub struct InternalPageContext {
    pub version: String,
    pub settings: SettingsView,
    pub shield_stats: ShieldStats,
    pub memory_mb: f64,
    pub tab_count: usize,
    pub hibernated_count: usize,
    pub top_sites: Vec<(String, String)>,
    pub history: Vec<HistoryEntry>,
    pub bookmarks: Vec<Bookmark>,
    pub downloads: Vec<DownloadItem>,
    pub update: UpdateView,
    pub turbo: Option<TurboSummary>,
    pub view_source: Option<ViewSource>,
}

pub const INTERNAL_ORIGIN: &str = "ferrox://";

pub fn is_internal(url: &str) -> bool {
    url.starts_with(INTERNAL_ORIGIN)
}

/// Escapes untrusted text for safe HTML interpolation.
pub fn escape_html(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for c in raw.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Serializes JSON that is safe to embed inside a `<script>` element.
pub fn json_script<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .unwrap_or_else(|_| "null".to_string())
        .replace("</", "<\\/")
}

const SHELL: &str = r###"<!doctype html>
<html lang="en"><head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>__TITLE__</title>
<style>
:root{
  --bg:#0b0d10;--panel:#12161d;--panel2:#171c25;--line:#232a35;
  --text:#e9edf2;--muted:#8b96a7;--accent:#ff6a3d;--accent2:#37d6c4;
  --danger:#ff5470;--radius:14px;
}
*{box-sizing:border-box}
html,body{margin:0;padding:0}
body{
  background:radial-gradient(1200px 600px at 15% -10%,#1b2330 0%,var(--bg) 55%) fixed;
  color:var(--text);font-family:Inter,-apple-system,"Segoe UI",system-ui,sans-serif;
  font-size:15px;line-height:1.55;min-height:100vh;
}
a{color:var(--accent2);text-decoration:none}
a:hover{text-decoration:underline}
.wrap{max-width:980px;margin:0 auto;padding:34px 28px 60px}
.brand{display:flex;align-items:center;gap:12px;margin-bottom:26px}
.logo{width:38px;height:38px;border-radius:11px;background:linear-gradient(135deg,var(--accent),#ffb347);
  display:grid;place-items:center;font-weight:800;color:#1b0d06;font-size:19px;box-shadow:0 8px 24px rgba(255,106,61,.25)}
.brand h1{font-size:19px;margin:0;letter-spacing:.4px}
.brand small{color:var(--muted);font-size:11px;letter-spacing:1.6px;text-transform:uppercase}
h2{font-size:15px;letter-spacing:.8px;text-transform:uppercase;color:var(--muted);margin:30px 0 12px}
.card{background:linear-gradient(180deg,var(--panel2),var(--panel));border:1px solid var(--line);
  border-radius:var(--radius);padding:18px}
.grid{display:grid;gap:14px}
.grid.cols-4{grid-template-columns:repeat(auto-fit,minmax(190px,1fr))}
.grid.cols-3{grid-template-columns:repeat(auto-fit,minmax(240px,1fr))}
.stat .value{font-size:26px;font-weight:750;letter-spacing:-.5px}
.stat .label{color:var(--muted);font-size:12px;text-transform:uppercase;letter-spacing:1px}
.stat .hint{color:var(--muted);font-size:12px;margin-top:6px}
.search{display:flex;gap:10px;margin:6px 0 4px}
.search input[type=text],input[type=search],select{
  width:100%;background:#0d1117;border:1px solid var(--line);color:var(--text);
  border-radius:12px;padding:13px 16px;font-size:15px;outline:none;transition:.15s}
.search input:focus,input:focus,select:focus{border-color:var(--accent);box-shadow:0 0 0 3px rgba(255,106,61,.14)}
button{background:var(--panel2);color:var(--text);border:1px solid var(--line);border-radius:10px;
  padding:9px 14px;font-size:13px;cursor:pointer;transition:.15s;font-weight:600}
button:hover{border-color:#3a4657;background:#1d2430}
button.primary{background:linear-gradient(135deg,var(--accent),#ff8f4d);border:none;color:#1c0d05}
button.primary:hover{filter:brightness(1.07)}
button.ghost{background:transparent}
button.danger{color:var(--danger);border-color:#43222b}
.row{display:flex;align-items:center;gap:12px;justify-content:space-between;
  padding:13px 4px;border-bottom:1px solid var(--line)}
.row:last-child{border-bottom:none}
.row .meta{color:var(--muted);font-size:12px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.row .title{font-weight:600;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.item-url{color:var(--muted);font-size:12px;max-width:620px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.pill{display:inline-flex;align-items:center;gap:6px;background:#0d1117;border:1px solid var(--line);
  color:var(--muted);border-radius:999px;padding:4px 10px;font-size:11px;letter-spacing:.6px;text-transform:uppercase}
.dot{width:7px;height:7px;border-radius:50%;background:var(--accent2)}
.chips{display:flex;flex-wrap:wrap;gap:10px}
.chip{flex:1 1 140px;background:#0f131a;border:1px solid var(--line);border-radius:12px;padding:14px;
  display:block;transition:.15s;min-width:0}
.chip:hover{border-color:var(--accent);text-decoration:none;transform:translateY(-1px)}
.chip b{display:block;font-size:13px;color:var(--text);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.chip span{display:block;color:var(--muted);font-size:11px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.empty{color:var(--muted);text-align:center;padding:34px 10px;font-size:13px}
.footer{margin-top:44px;color:var(--muted);font-size:11px;display:flex;justify-content:space-between;gap:10px}
.progress{height:6px;background:#0d1117;border-radius:99px;overflow:hidden;margin-top:8px}
.progress>i{display:block;height:100%;background:linear-gradient(90deg,var(--accent),#ffb347)}
.kv{display:grid;grid-template-columns:150px 1fr;gap:8px 14px;font-size:13px}
.kv dt{color:var(--muted)}
.kv dd{margin:0;overflow:hidden;text-overflow:ellipsis}
label.toggle{display:flex;align-items:center;gap:10px;cursor:pointer;user-select:none;font-size:13px}
label.toggle input{accent-color:var(--accent);width:16px;height:16px}
table{width:100%;border-collapse:collapse;font-size:13px}
th{text-align:left;color:var(--muted);font-weight:600;font-size:11px;text-transform:uppercase;
  letter-spacing:.8px;padding:8px 10px;border-bottom:1px solid var(--line)}
td{padding:10px;border-bottom:1px solid var(--line);vertical-align:middle}
tr:last-child td{border-bottom:none}
code{font-family:ui-monospace,Consolas,monospace;font-size:12px;color:#ffb347}
</style></head>
<body><div class="wrap">__BODY__</div>
<script>
function ferrox(cmd, args){ try{ window.ipc.postMessage(JSON.stringify(Object.assign({cmd:cmd},args||{}))); }catch(e){} }
</script>
</body></html>"###;

fn shell(title: &str, body: &str) -> String {
    SHELL
        .replace("__TITLE__", &escape_html(title))
        .replace("__BODY__", body)
}

fn brand() -> String {
    r###"<div class="brand">
      <div class="logo">F</div>
      <div><h1>Ferrox</h1><small>Fast · Private · Rust</small></div>
    </div>"###
        .to_string()
}

pub fn render(url: &str, ctx: &InternalPageContext) -> String {
    match url.trim_start_matches(INTERNAL_ORIGIN) {
        "newtab" | "" => new_tab_page(ctx),
        "settings" => settings_page(ctx),
        "profiler" => profiler_page(ctx),
        "history" => history_page(ctx),
        "bookmarks" => bookmarks_page(ctx),
        "downloads" => downloads_page(ctx),
        "updates" => updates_page(ctx),
        other if other.starts_with("view-source") => view_source_page(ctx),
        other => not_found_page(other),
    }
}

fn stats_cards(ctx: &InternalPageContext) -> String {
    let saved_mb = ctx.shield_stats.bytes_estimated_saved as f64 / (1024.0 * 1024.0);
    format!(
        r###"<div class="grid cols-4">
  <div class="card stat"><div class="label">Trackers blocked</div><div class="value">{blocked}</div><div class="hint">{ratio:.0}% of requests neutralised</div></div>
  <div class="card stat"><div class="label">Data saved</div><div class="value">{saved:.1} MB</div><div class="hint">Estimated network savings</div></div>
  <div class="card stat"><div class="label">Memory in use</div><div class="value">{mem:.0} MB</div><div class="hint">Across {tabs} open tabs</div></div>
  <div class="card stat"><div class="label">Hibernated tabs</div><div class="value">{hibernated}</div><div class="hint">Suspended to save RAM</div></div>
</div>"###,
        blocked = ctx.shield_stats.requests_blocked,
        ratio = if ctx.shield_stats.requests_checked == 0 {
            0.0
        } else {
            (ctx.shield_stats.requests_blocked as f64 / ctx.shield_stats.requests_checked as f64)
                * 100.0
        },
        saved = saved_mb,
        mem = ctx.memory_mb,
        tabs = ctx.tab_count,
        hibernated = ctx.hibernated_count,
    )
}

fn new_tab_page(ctx: &InternalPageContext) -> String {
    let mut chips = String::new();
    if ctx.top_sites.is_empty() {
        for (title, url) in [
            ("Rust", "https://www.rust-lang.org"),
            ("Docs.rs", "https://docs.rs"),
            ("GitHub", "https://github.com"),
            ("Wikipedia", "https://www.wikipedia.org"),
            ("Hacker News", "https://news.ycombinator.com"),
            ("DuckDuckGo", "https://duckduckgo.com"),
        ] {
            chips.push_str(&format!(
                r###"<a class="chip" href="#" onclick="ferrox('navigate',{{url:'{url}'}});return false"><b>{title}</b><span>{host}</span></a>"###,
                url = escape_html(url),
                title = escape_html(title),
                host = escape_html(url.trim_start_matches("https://"))
            ));
        }
    } else {
        for (url, title) in ctx.top_sites.iter().take(8) {
            let host = url::Url::parse(url)
                .ok()
                .and_then(|u| u.host_str().map(str::to_string))
                .unwrap_or_else(|| url.clone());
            let label = if title.trim().is_empty() {
                &host
            } else {
                title
            };
            chips.push_str(&format!(
                r###"<a class="chip" href="#" onclick="ferrox('navigate',{{url:'{url}'}});return false"><b>{title}</b><span>{host}</span></a>"###,
                url = escape_html(url),
                title = escape_html(label),
                host = escape_html(&host)
            ));
        }
    }

    let body = format!(
        r###"{brand}
<p class="pill"><span class="dot"></span> Compute-governed browsing · v{version}</p>
<h2 style="margin-top:18px">Search the web</h2>
<div class="search card" style="padding:14px">
  <input id="q" type="text" placeholder="Search or enter a URL — try !gh ferrox or = 2^10" autofocus>
  <button class="primary" onclick="submitSearch()">Go</button>
</div>
<script>
document.getElementById('q').addEventListener('keydown', function(e){{
  if(e.key==='Enter') submitSearch();
}});
function submitSearch(){{
  var q=document.getElementById('q').value.trim();
  if(!q) return;
  if(/^https?:\/\//i.test(q) || /^ferrox:\/\//i.test(q)) ferrox('navigate',{{url:q}});
  else ferrox('search',{{query:q}});
}}
</script>
<h2>Performance</h2>
{stats}
<h2>Shortcuts</h2>
<div class="chips">{chips}</div>
<div class="footer"><span>Ferrox {version} · {channel} channel</span><span>Rust-native engine · Zero telemetry</span></div>"###,
        brand = brand(),
        version = escape_html(&ctx.version),
        stats = stats_cards(ctx),
        chips = chips,
        channel = escape_html(&ctx.settings.update_channel),
    );
    shell("New Tab — Ferrox", &body)
}

fn settings_page(ctx: &InternalPageContext) -> String {
    let engines = [
        ("duckduckgo", "DuckDuckGo"),
        ("google", "Google"),
        ("brave", "Brave Search"),
        ("startpage", "Startpage"),
        ("bing", "Bing"),
    ];
    let engine_options: String = engines
        .iter()
        .map(|(id, name)| {
            let sel = if ctx.settings.search_engine == *id {
                " selected"
            } else {
                ""
            };
            format!(r###"<option value="{id}"{sel}>{name}</option>"###)
        })
        .collect();

    let modes = [
        ("turbo", "Turbo — pure-Rust reader, zero JS"),
        ("eco", "Eco — maximum savings"),
        ("balanced", "Balanced — smart default"),
        ("full", "Full — unrestricted"),
    ];
    let mode_options: String = modes
        .iter()
        .map(|(id, name)| {
            let sel = if ctx.settings.compute_mode == *id {
                " selected"
            } else {
                ""
            };
            format!(r###"<option value="{id}"{sel}>{name}</option>"###)
        })
        .collect();

    let themes = [("dark", "Dark"), ("light", "Light"), ("system", "System")];
    let theme_options: String = themes
        .iter()
        .map(|(id, name)| {
            let sel = if ctx.settings.theme == *id {
                " selected"
            } else {
                ""
            };
            format!(r###"<option value="{id}"{sel}>{name}</option>"###)
        })
        .collect();

    let channels = [
        ("stable", "Stable"),
        ("beta", "Beta"),
        ("nightly", "Nightly"),
    ];
    let channel_options: String = channels
        .iter()
        .map(|(id, name)| {
            let sel = if ctx.settings.update_channel == *id {
                " selected"
            } else {
                ""
            };
            format!(r###"<option value="{id}"{sel}>{name}</option>"###)
        })
        .collect();

    let mut site_rows = String::new();
    for (host, mode) in &ctx.settings.per_site {
        site_rows.push_str(&format!(
            r###"<div class="row">
  <div style="min-width:0"><div class="title">{host}</div><div class="item-url">{mode}</div></div>
  <div style="display:flex;gap:8px;align-items:center">
    <button class="danger" onclick="ferrox('remove_site_mode',{{host:'{host}'}});this.closest('.row').remove()">✕</button>
  </div>
</div>"###,
            host = escape_html(host),
            mode = escape_html(mode),
        ));
    }
    if site_rows.is_empty() {
        site_rows.push_str(r###"<div class="empty">No per-site overrides. Every site uses the default compute mode.</div>"###);
    }

    let body = format!(
        r###"{brand}
<h2>Search & Startup</h2>
<div class="card grid">
  <div><label class="pill">Default search engine</label><br><br>
    <select onchange="ferrox('set_search_engine',{{value:this.value}})">{engines}</select></div>
  <div><label class="pill">Homepage</label><br><br>
    <input type="text" value="{homepage}" onchange="ferrox('set_homepage',{{value:this.value}})"></div>
  <div><label class="toggle"><input type="checkbox" {restore} onchange="ferrox('set_restore_session',{{value:this.checked}})"> Restore the last session on startup</label></div>
</div>
<h2>Performance & Privacy</h2>
<div class="card grid">
  <div><label class="pill">Compute mode</label><br><br>
    <select onchange="ferrox('set_compute_mode',{{value:this.value}})">{modes}</select></div>
  <div><label class="pill">Hibernate idle tabs after (seconds)</label><br><br>
    <input type="text" value="{hibernate}" onchange="ferrox('set_hibernate',{{value:parseInt(this.value)||300}})"></div>
  <div><label class="toggle"><input type="checkbox" {shield} onchange="ferrox('set_shield',{{value:this.checked}})"> Enable Shield (ads, trackers, miners)</label></div>
</div>
<h2>Per-site policies</h2>
<div class="card">
  <div class="search">
    <input type="text" id="site-host" placeholder="example.com" oninput="document.getElementById('site-mode').disabled=!this.value.trim()">
    <select id="site-mode" onchange="ferrox('set_site_mode',{{host:document.getElementById('site-host').value.trim(),value:this.value}})" disabled>
      <option value="turbo">Turbo</option>
      <option value="eco">Eco</option>
      <option value="balanced">Balanced</option>
      <option value="full">Full</option>
    </select>
  </div>
  <div style="margin-top:12px">{site_rows}</div>
</div>
<h2>Appearance</h2>
<div class="card grid">
  <div><label class="pill">Theme</label><br><br>
    <select onchange="ferrox('set_theme',{{value:this.value}})">{themes}</select></div>
</div>
<h2>Updates</h2>
<div class="card grid">
  <div><label class="pill">Release channel</label><br><br>
    <select onchange="ferrox('set_update_channel',{{value:this.value}})">{channels}</select></div>
  <div><label class="toggle"><input type="checkbox" {auto} onchange="ferrox('set_auto_update',{{value:this.checked}})"> Check for updates automatically</label></div>
  <div><button class="primary" onclick="ferrox('check_update',{{}})">Check for updates now</button></div>
</div>
<h2>Browsing data</h2>
<div class="card" style="display:flex;gap:10px;flex-wrap:wrap">
  <button class="ghost" onclick="if(confirm('Clear the whole history?')){{ferrox('clear_history',{{}});location.reload()}}">Clear history</button>
  <button class="ghost" onclick="if(confirm('Remove every download record?')){{ferrox('clear_downloads',{{}});location.reload()}}">Clear download list</button>
  <button class="danger" onclick="if(confirm('Erase all browsing data?')){{ferrox('clear_all_data',{{}});location.reload()}}">Erase everything</button>
</div>
<div class="footer"><span>Ferrox {version}</span><span>{profile}</span></div>"###,
        brand = brand(),
        engines = engine_options,
        modes = mode_options,
        themes = theme_options,
        channels = channel_options,
        homepage = escape_html(&ctx.settings.homepage),
        hibernate = ctx.settings.hibernate_after_secs,
        shield = if ctx.settings.shield_enabled {
            "checked"
        } else {
            ""
        },
        restore = if ctx.settings.restore_session {
            "checked"
        } else {
            ""
        },
        site_rows = site_rows,
        auto = if ctx.settings.auto_check_updates {
            "checked"
        } else {
            ""
        },
        version = escape_html(&ctx.version),
        profile = if ctx.settings.theme == "light" {
            "Light theme"
        } else {
            "Dark theme"
        },
    );
    shell("Settings — Ferrox", &body)
}

fn profiler_page(ctx: &InternalPageContext) -> String {
    let body = format!(
        r###"{brand}
<h2>Live performance monitor</h2>
<div class="grid cols-4">
  <div class="card stat"><div class="label">Memory</div><div class="value" id="m-mem">{mem:.0} MB</div><div class="hint">Rust + WebView2 combined</div></div>
  <div class="card stat"><div class="label">Tabs</div><div class="value" id="m-tabs">{tabs}</div><div class="hint"><span id="m-hib">{hib}</span> hibernated</div></div>
  <div class="card stat"><div class="label">Blocked</div><div class="value" id="m-blocked">{blocked}</div><div class="hint">Requests neutralised</div></div>
  <div class="card stat"><div class="label">Saved</div><div class="value" id="m-saved">{saved:.1} MB</div><div class="hint">Estimated transfer</div></div>
</div>
<h2>Compute governor</h2>
<div class="card">
  <dl class="kv">
    <dt>Default mode</dt><dd>{mode}</dd>
    <dt>Scripts blocked</dt><dd id="m-scripts">{scripts}</dd>
    <dt>Requests checked</dt><dd id="m-checked">{checked}</dd>
  </dl>
</div>
<h2>Live document</h2>
<div class="card" id="turbo-card">{turbo}</div>
<div class="footer"><span>Updates every second via native IPC</span><span>ferrox://profiler</span></div>
<script>
window.__ferroxUpdateStats = function(s){{
  function set(id, v){{ var el=document.getElementById(id); if(el) el.textContent=v; }}
  set('m-mem', Math.round(s.memory_mb)+' MB');
  set('m-tabs', s.tabs);
  set('m-hib', s.hibernated);
  set('m-blocked', s.blocked);
  set('m-saved', s.saved_mb.toFixed(1)+' MB');
  set('m-scripts', s.scripts_blocked);
  set('m-checked', s.checked);
}};
ferrox('profiler_ready', {{}});
</script>"###,
        brand = brand(),
        mem = ctx.memory_mb,
        tabs = ctx.tab_count,
        hib = ctx.hibernated_count,
        blocked = ctx.shield_stats.requests_blocked,
        saved = ctx.shield_stats.bytes_estimated_saved as f64 / (1024.0 * 1024.0),
        mode = escape_html(&ctx.settings.compute_mode),
        scripts = ctx.shield_stats.scripts_blocked,
        checked = ctx.shield_stats.requests_checked,
        turbo = match &ctx.turbo {
            Some(t) => format!(
                r###"<dl class="kv"><dt>Title</dt><dd>{}</dd><dt>URL</dt><dd>{}</dd><dt>Words</dt><dd>{} · {} min read</dd><dt>Render time</dt><dd>{} ms (pure Rust)</dd></dl>"###,
                escape_html(&t.title),
                escape_html(&t.url),
                t.words,
                t.read_minutes,
                t.load_ms
            ),
            None => r###"<div class="empty">Open a page in Turbo mode to inspect its pure-Rust render profile.</div>"###.to_string(),
        },
    );
    shell("Profiler — Ferrox", &body)
}

fn history_page(ctx: &InternalPageContext) -> String {
    let mut rows = String::new();
    for entry in ctx.history.iter().take(300) {
        rows.push_str(&format!(
            r###"<div class="row">
  <div style="min-width:0">
    <div class="title">{title}</div>
    <div class="item-url">{url}</div>
  </div>
  <div style="display:flex;gap:8px;align-items:center;white-space:nowrap">
    <span class="meta">{visits} visits · {date}</span>
    <button class="ghost" onclick="ferrox('open_history',{{url:'{url}'}})">Open</button>
    <button class="danger" onclick="ferrox('delete_history',{{id:{id}}});this.closest('.row').remove()">✕</button>
  </div>
</div>"###,
            title = escape_html(if entry.title.is_empty() {
                &entry.url
            } else {
                &entry.title
            }),
            url = escape_html(&entry.url),
            visits = entry.visit_count,
            date = entry.visited_at.format("%b %d, %H:%M"),
            id = entry.id,
        ));
    }
    if rows.is_empty() {
        rows.push_str(
            r###"<div class="empty">No history yet. Your visits stay on this device.</div>"###,
        );
    }

    let body = format!(
        r###"{brand}
<h2>History</h2>
<div class="card">
  <div class="search">
    <input type="search" id="hq" placeholder="Search history (full-text FTS5)…" oninput="doSearch()">
    <button class="danger" onclick="if(confirm('Clear all history?')){{ferrox('clear_history',{{}});location.reload()}}">Clear all</button>
  </div>
  <div id="hlist">{rows}</div>
</div>
<script>
var timer=null;
function doSearch(){{
  clearTimeout(timer);
  timer=setTimeout(function(){{ ferrox('search_history',{{query:document.getElementById('hq').value}}); }},120);
}}
window.__ferroxSetHistory = function(list){{
  var box=document.getElementById('hlist');
  if(!list.length){{ box.innerHTML='<div class="empty">No matches.</div>'; return; }}
  box.innerHTML=list.map(function(e){{
    var t=(e.title||e.url).replace(/[&<>"]/g,function(c){{return {{'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}}[c];}});
    var u=e.url.replace(/[&<>"]/g,function(c){{return {{'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}}[c];}});
    return '<div class="row"><div style="min-width:0"><div class="title">'+t+'</div><div class="item-url">'+u+'</div></div>'+
      '<div style="display:flex;gap:8px"><button class="ghost" onclick="ferrox(\\'open_history\\',{{url:\\''+u+'\\'}})">Open</button>'+
      '<button class="danger" onclick="ferrox(\\'delete_history\\',{{id:'+e.id+'}});this.closest(\\'.row\\').remove()">✕</button></div></div>';
  }}).join('');
}};
</script>
<div class="footer"><span>{count} entries · SQLite FTS5</span><span>Private by default</span></div>"###,
        brand = brand(),
        rows = rows,
        count = ctx.history.len(),
    );
    shell("History — Ferrox", &body)
}

fn bookmarks_page(ctx: &InternalPageContext) -> String {
    let mut rows = String::new();
    for bm in &ctx.bookmarks {
        rows.push_str(&format!(
            r###"<div class="row">
  <div style="min-width:0">
    <div class="title">{title}</div>
    <div class="item-url">{url}</div>
  </div>
  <div style="display:flex;gap:8px;align-items:center;white-space:nowrap">
    <span class="pill">{folder}</span>
    <button class="ghost" onclick="ferrox('navigate',{{url:'{url}'}})">Open</button>
    <button class="danger" onclick="ferrox('delete_bookmark',{{id:{id}}});this.closest('.row').remove()">✕</button>
  </div>
</div>"###,
            title = escape_html(if bm.title.is_empty() {
                &bm.url
            } else {
                &bm.title
            }),
            url = escape_html(&bm.url),
            folder = escape_html(&bm.folder),
            id = bm.id,
        ));
    }
    if rows.is_empty() {
        rows.push_str(
            r###"<div class="empty">No bookmarks yet. Press Ctrl+D on any page.</div>"###,
        );
    }

    let body = format!(
        r###"{brand}
<h2>Bookmarks</h2>
<div class="card">
  <div class="search">
    <input type="search" id="bq" placeholder="Search bookmarks…" oninput="doSearch()">
    <button class="ghost" onclick="ferrox('export_bookmarks',{{}})">Export HTML</button>
  </div>
  <div id="blist">{rows}</div>
</div>
<script>
var timer=null;
function doSearch(){{
  clearTimeout(timer);
  timer=setTimeout(function(){{ ferrox('search_bookmarks',{{query:document.getElementById('bq').value}}); }},120);
}}
window.__ferroxSetBookmarks = function(list){{
  var box=document.getElementById('blist');
  if(!list.length){{ box.innerHTML='<div class="empty">No matches.</div>'; return; }}
  box.innerHTML=list.map(function(e){{
    var esc=function(s){{return String(s).replace(/[&<>"]/g,function(c){{return {{'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}}[c];}});}};
    return '<div class="row"><div style="min-width:0"><div class="title">'+esc(e.title||e.url)+'</div>'+
      '<div class="item-url">'+esc(e.url)+'</div></div><div style="display:flex;gap:8px">'+
      '<button class="ghost" onclick="ferrox(\\'navigate\\',{{url:\\''+esc(e.url)+'\\'}})">Open</button>'+
      '<button class="danger" onclick="ferrox(\\'delete_bookmark\\',{{id:'+e.id+'}});this.closest(\\'.row\\').remove()">✕</button></div></div>';
  }}).join('');
}};
</script>
<div class="footer"><span>{count} bookmarks</span><span>Synced to local SQLite</span></div>"###,
        brand = brand(),
        rows = rows,
        count = ctx.bookmarks.len(),
    );
    shell("Bookmarks — Ferrox", &body)
}

fn downloads_page(ctx: &InternalPageContext) -> String {
    let mut rows = String::new();
    for item in &ctx.downloads {
        rows.push_str(&download_row(item));
    }
    if rows.is_empty() {
        rows.push_str(r###"<div class="empty">No downloads yet.</div>"###);
    }

    let body = format!(
        r###"{brand}
<h2>Downloads</h2>
<div class="card"><div id="dlist">{rows}</div></div>
<script>
window.__ferroxSetDownloads = function(list){{
  var box=document.getElementById('dlist');
  if(!box) return;
  if(!list.length){{ box.innerHTML='<div class="empty">No downloads yet.</div>'; return; }}
  var esc=function(s){{return String(s).replace(/[&<>"]/g,function(c){{return {{'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}}[c];}});}};
  box.innerHTML=list.map(function(d){{
    var pct=d.total>0?Math.min(100,(d.downloaded/d.total)*100):0;
    var size=function(n){{
      if(!n) return '0 B';
      var u=['B','KB','MB','GB']; var i=0;
      while(n>=1024 && i<u.length-1){{ n=n/1024; i++; }}
      return (i===0?n:n.toFixed(1))+' '+u[i];
    }};
    var name=esc(d.name||d.url);
    var actions='<button class="ghost" onclick="ferrox(\'open_download\',{{id:\''+d.id+'\'}})">Open</button>'+
      '<button class="ghost" onclick="ferrox(\'reveal_download\',{{id:\''+d.id+'\'}})">Folder</button>';
    if(d.status==='failed'||d.status==='cancelled'){{
      actions+='<button class="ghost" onclick="ferrox(\'retry_download\',{{id:\''+d.id+'\'}})">Retry</button>';
    }}
    actions+='<button class="danger" onclick="ferrox(\'delete_download\',{{id:\''+d.id+'\'}});this.closest(\'.row\').remove()">✕</button>';
    return '<div class="row"><div style="min-width:0;flex:1">'+
      '<div class="title">'+name+'</div><div class="item-url">'+esc(d.url)+'</div>'+
      '<div class="progress"><i style="width:'+pct.toFixed(1)+'%"></i></div>'+
      '<div class="meta">'+size(d.downloaded)+(d.total?' / '+size(d.total):'')+' · '+pct.toFixed(1)+'%</div></div>'+
      '<div style="display:flex;gap:8px;align-items:center;white-space:nowrap">'+
      '<span class="pill">'+esc(d.status)+'</span>'+actions+'</div></div>';
  }}).join('');
}};
</script>
<div class="footer"><span>{count} items</span><span>{folder}</span></div>"###,
        brand = brand(),
        rows = rows,
        count = ctx.downloads.len(),
        folder = "Ferrox Downloads",
    );
    shell("Downloads — Ferrox", &body)
}

/// Renders a single download row for the downloads page.
fn download_row(item: &DownloadItem) -> String {
    let progress = item.progress_percent();
    let status = download_status_label(item);
    format!(
        r###"<div class="row">
  <div style="min-width:0;flex:1">
    <div class="title">{name}</div>
    <div class="item-url">{url}</div>
    <div class="progress"><i style="width:{progress:.1}%"></i></div>
    <div class="meta">{downloaded} / {total} · {progress:.1}%</div>
  </div>
  <div style="display:flex;gap:8px;align-items:center;white-space:nowrap">
    <span class="pill">{status}</span>
    <button class="ghost" onclick="ferrox('open_download',{{id:'{id}'}})">Open</button>
    <button class="ghost" onclick="ferrox('reveal_download',{{id:'{id}'}})">Folder</button>
    <button class="danger" onclick="ferrox('delete_download',{{id:'{id}'}});this.closest('.row').remove()">✕</button>
  </div>
</div>"###,
        name = escape_html(
            item.file_path
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or(&item.file_path)
        ),
        url = escape_html(&item.url),
        progress = progress,
        downloaded = human_bytes(item.downloaded_bytes),
        total = human_bytes(item.total_bytes),
        status = escape_html(&status),
        id = escape_html(&item.id),
    )
}

/// Human readable label for a download state.
pub fn download_status_label(item: &DownloadItem) -> String {
    match item.status {
        ferrox_storage::DownloadStatus::Pending => "Pending".to_string(),
        ferrox_storage::DownloadStatus::InProgress => "In progress".to_string(),
        ferrox_storage::DownloadStatus::Paused => "Paused".to_string(),
        ferrox_storage::DownloadStatus::Completed => "Completed".to_string(),
        ferrox_storage::DownloadStatus::Failed => "Failed".to_string(),
        ferrox_storage::DownloadStatus::Cancelled => "Cancelled".to_string(),
    }
}

/// Renders the source code of a page.
fn view_source_page(ctx: &InternalPageContext) -> String {
    let source = ctx
        .view_source
        .as_ref()
        .map(|v| {
            let body = escape_html(&v.source);
            format!(
                r###"<div class="card"><pre style="margin:0;white-space:pre-wrap;word-break:break-word;font-family:ui-monospace,Consolas,monospace;font-size:12px;line-height:1.55">{body}</pre></div>
<div class="footer"><span>{bytes} bytes{marker}</span><span>{url}</span></div>"###,
                body = body,
                bytes = v.bytes,
                marker = if v.truncated { " · truncated" } else { "" },
                url = escape_html(&v.url),
            )
        })
        .unwrap_or_else(|| {
            r###"<div class="card empty">The source of this page is being fetched…</div>"###.to_string()
        });

    let body = format!(
        r###"{brand}
<h2>Page source</h2>
{body}
<div class="footer"><span>Fetched by Ferrox · view-source</span><span>{version}</span></div>"###,
        brand = brand(),
        body = source,
        version = escape_html(&ctx.version),
    );
    shell("View source — Ferrox", &body)
}

fn updates_page(ctx: &InternalPageContext) -> String {
    let available = match &ctx.update.available_version {
        Some(v) => format!(
            r###"<span class="pill" style="color:var(--accent2)">Update available: {}</span>"###,
            escape_html(v)
        ),
        None => r###"<span class="pill">You are up to date</span>"###.to_string(),
    };
    let install = match &ctx.update.available_version {
        Some(v) => format!(
            r###"<button class="primary" onclick="ferrox('install_update',{{}})">Install {}</button>"###,
            escape_html(v)
        ),
        None => String::new(),
    };
    let body = format!(
        r###"{brand}
<h2>Updates & releases</h2>
<div class="card">
  <dl class="kv">
    <dt>Installed version</dt><dd>{version}</dd>
    <dt>Release channel</dt><dd>{channel}</dd>
    <dt>Last check</dt><dd>{last}</dd>
    <dt>Status</dt><dd>{available}</dd>
  </dl>
  <div style="margin-top:16px;display:flex;gap:10px">
    <button class="primary" onclick="ferrox('check_update',{{}})">Check for updates</button>
    {install}
    <button class="ghost" onclick="ferrox('open_notes',{{}})">Release notes</button>
  </div>
  <div id="u-msg" class="meta" style="margin-top:12px"></div>
</div>
<h2>Signature & integrity</h2>
<div class="card">
  <p class="meta" style="margin:0">Every Ferrox release is signed with <b>Ed25519</b> and verified against its <b>SHA-256</b> digest before installation. Updates are atomic and can be rolled back.</p>
</div>
<script>
window.__ferroxUpdateResult = function(ok, message){{
  var el=document.getElementById('u-msg');
  el.textContent=message;
  el.style.color = ok ? 'var(--accent2)' : 'var(--danger)';
}};
</script>
<div class="footer"><span>Secure updater · Ed25519</span><span>SemVer {version}</span></div>"###,
        brand = brand(),
        version = escape_html(&ctx.update.current_version),
        channel = escape_html(&ctx.update.channel),
        last = escape_html(if ctx.update.last_check.is_empty() {
            "Never"
        } else {
            &ctx.update.last_check
        }),
        available = available,
        install = install,
    );
    shell("Updates — Ferrox", &body)
}

fn not_found_page(page: &str) -> String {
    let body = format!(
        r###"{brand}
<div class="card empty" style="margin-top:40px">
  <div style="font-size:34px;font-weight:800;letter-spacing:-1px">404</div>
  <p>Unknown internal page: <code>{page}</code></p>
  <p><a href="#" onclick="ferrox('navigate',{{url:'ferrox://newtab'}});return false">Go to new tab</a></p>
</div>"###,
        brand = brand(),
        page = escape_html(page),
    );
    shell("Not Found — Ferrox", &body)
}

fn human_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_user_content() {
        assert_eq!(
            escape_html("<script>alert(1)</script>"),
            "&lt;script&gt;alert(1)&lt;/script&gt;"
        );
    }

    #[test]
    fn json_script_cannot_close_tag() {
        let encoded = json_script(&"</script><script>alert(1)</script>");
        assert!(!encoded.contains("</script"));
    }

    #[test]
    fn renders_every_internal_page() {
        let ctx = InternalPageContext {
            version: "0.1.0".into(),
            settings: SettingsView {
                compute_mode: "balanced".into(),
                update_channel: "stable".into(),
                homepage: "ferrox://newtab".into(),
                ..Default::default()
            },
            ..Default::default()
        };
        for page in [
            "newtab",
            "settings",
            "profiler",
            "history",
            "bookmarks",
            "downloads",
            "updates",
            "view-source",
        ] {
            let html = render(&format!("ferrox://{page}"), &ctx);
            assert!(html.contains("<!doctype html>"), "{page} missing shell");
            assert!(html.contains("Ferrox"), "{page} missing brand");
            assert!(!html.contains("__TITLE__"));
            assert!(!html.contains("__BODY__"));
        }
        assert!(render("ferrox://nope", &ctx).contains("404"));
    }

    #[test]
    fn downloads_page_lists_items_and_offers_actions() {
        let mut item = DownloadItem::new(
            "https://example.com/report.pdf",
            "/tmp/downloads/report.pdf",
        );
        item.total_bytes = 2_000;
        item.downloaded_bytes = 500;
        item.status = ferrox_storage::DownloadStatus::InProgress;

        let ctx = InternalPageContext {
            downloads: vec![item],
            ..Default::default()
        };
        let html = render("ferrox://downloads", &ctx);
        assert!(html.contains("report.pdf"));
        assert!(html.contains("In progress"));
        assert!(html.contains("reveal_download"));
        assert!(html.contains("__ferroxSetDownloads"));
    }

    #[test]
    fn view_source_escapes_the_page_source() {
        let ctx = InternalPageContext {
            view_source: Some(ViewSource {
                url: "https://example.com".into(),
                source: "<script>alert(1)</script>".into(),
                bytes: 25,
                truncated: false,
            }),
            ..Default::default()
        };
        let html = render("ferrox://view-source", &ctx);
        assert!(html.contains("&lt;script&gt;"));
        assert!(!html.contains("<script>alert(1)</script>"));
        assert!(html.contains("25 bytes"));
    }

    #[test]
    fn settings_page_shows_per_site_policies() {
        let ctx = InternalPageContext {
            settings: SettingsView {
                per_site: vec![("example.com".into(), "turbo".into())],
                ..Default::default()
            },
            ..Default::default()
        };
        let html = render("ferrox://settings", &ctx);
        assert!(html.contains("example.com"));
        assert!(html.contains("turbo"));
        assert!(html.contains("remove_site_mode"));
    }
}
