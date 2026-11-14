# Changelog

All notable changes to **Ferrox Browser** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Download manager**: native download interception with a dedicated `Ferrox/Downloads` folder, live progress tracking (file size polling plus a `HEAD` request for the expected size), `SHA-256` checksums, and a `ferrox://downloads` page that updates in place. Records can be opened, revealed in the file manager, retried or removed.
- **Find in page** (`Ctrl+F`): a find bar in the browser chrome with match highlighting, a live match counter and next/previous navigation, implemented with an injected `document_start` helper.
- **Tab actions**: right-click a tab to pin it (never hibernates), mute its media, duplicate it or close every other tab. Middle-click closes a tab.
- **Per-tab zoom** (`Ctrl+=`, `Ctrl+-`, `Ctrl+0`) with the current level shown in the tab strip.
- **Developer tools** and **view source** (`ferrox://view-source`) which fetches and renders the raw markup of the current page.
- **Per-site compute policies** in the settings page: every host can be pinned to `Turbo`, `Eco`, `Balanced` or `Full`.
- **Browsing data controls**: clear the history, the download list or everything from the settings page.
- Session restore now keeps the pinned and muted state of every tab.

### Fixed
- The workspace no longer compiles with a moved-value error in the Turbo reader, and `cargo clippy -- -D warnings` is clean again.
- Search queries are percent-encoded with `%20` instead of `+`, so engines no longer receive a literal plus sign.
- The Turbo reader keeps page scripts and inline handlers out of the extracted article body.

## [0.1.0] - 2026-11-13

### Added
- Initial multi-crate workspace architecture (`ferrox-core`, `ferrox-net`, `ferrox-shield`, `ferrox-governor`, `ferrox-storage`, `ferrox-search`, `ferrox-updater`, `ferrox-engine`, `ferrox-ui`, `ferrox-app`).
- Dual rendering architecture: `WebViewEngine` (`wry`) + pure-Rust `TurboReader` (`html5ever` + `scraper`).
- `ferrox-shield` ad, tracker, and cryptominer blocking powered by Brave's `adblock` engine.
- `ferrox-governor` compute-saving engine: timer/RAF coalescing, offscreen iframe/media deferral, reduced motion, per-site compute modes (`Turbo`, `Eco`, `Balanced`, `Full`), and automatic tab hibernation.
- `ferrox-search` omnibox with multi-engine search, Bang shortcuts (`!g`, `!ddg`, `!yt`, `!gh`, `!w`, `!rs`), and built-in math calculator.
- `ferrox-storage` SQLite WAL + FTS5 full-text search for history, bookmarks, downloads, and sessions with Incognito isolation.
- `ferrox-updater` SemVer release channels (`stable`, `beta`, `nightly`), Ed25519 signature verification, SHA-256 checksums, and atomic self-replace.
- Internal pages: `ferrox://newtab`, `ferrox://profiler`, `ferrox://settings`, `ferrox://history`, `ferrox://bookmarks`, `ferrox://downloads`, `ferrox://updates`.
