# Ferrox Browser

**Ferrox** is a high-speed, low-memory, privacy-first desktop web browser written in **Rust** (`MIT OR Apache-2.0`).

## Key Capabilities

- **Dual Rendering Architecture (`ferrox-engine`):**
  - **Full-Web Mode:** System WebView backend (`wry`) tightly wrapped with Rust network shielding and early JS compute governance.
  - **Turbo Mode (`Pure Rust`):** Sub-15ms, <15MB RAM pure-Rust HTML/article renderer (`html5ever` + `scraper`) with zero JavaScript execution.
- **Website Compute Governor (`ferrox-governor`):**
  - `document_start` script injection (`assets/scripts/governor.js`) that coalesces background timers (`setTimeout`/`setInterval`), clamps `requestAnimationFrame`, defers offscreen `<iframe>`/`<img>`/`<video>` via `IntersectionObserver`, prevents autoplay, and supports per-site policies (`Turbo`, `Eco`, `Balanced`, `Full`).
  - Automatic **Smart Tab Hibernation** driven by idle time and system memory pressure (`sysinfo`).
- **Built-in Shield (`ferrox-shield`):**
  - Native Rust ad, tracker, analytics, and cryptominer blocking powered by Brave's `adblock` engine.
- **Smart Omnibox & Search (`ferrox-search`):**
  - DuckDuckGo, Google, Brave Search, Startpage, Bing + Bang shortcuts (`!g`, `!ddg`, `!yt`, `!gh`, `!w`, `!rs`), instant math evaluator, and FTS5 history/bookmark search.
- **Fast Storage (`ferrox-storage`):**
  - SQLite with WAL mode & FTS5 full-text search, plus memory-isolated Incognito mode.
- **Signed Auto-Updater & Release Channels (`ferrox-updater`):**
  - SemVer versioning, `stable` / `beta` / `nightly` channels, `Ed25519` + `SHA-256` verification, and atomic binary replacement (`self-replace`).

## Build & Run

```bash
cargo test --workspace
cargo build --release -p ferrox-app
cargo run --release -p ferrox-app
```

### CLI Utilities
```bash
ferrox --version
ferrox --turbo https://example.com
ferrox --check-update
```

## License

Dual-licensed under **MIT** (`LICENSE-MIT`) or **Apache-2.0** (`LICENSE-APACHE`).
