# Changelog

All notable changes to **Ferrox Browser** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
