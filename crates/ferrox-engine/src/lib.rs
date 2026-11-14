// SPDX-License-Identifier: MIT OR Apache-2.0

pub mod internal_pages;
pub mod turbo_backend;
pub mod webview_backend;

pub use internal_pages::{
    InternalPageContext, SettingsView, TurboSummary, UpdateView, is_internal, render,
};
pub use turbo_backend::{TurboDocument, TurboReader};
pub use webview_backend::{
    EngineEvent, EngineShared, WebViewEngine, bounds, dom_shield_script, find_script, mute_script,
    sanitize_file_name, shortcut_script, unique_download_path,
};
