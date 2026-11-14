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


## Browser Features

| Feature | How to use it |
| --- | --- |
| Find in page | `Ctrl+F`, then `Enter` / `Shift+Enter` to walk through the matches |
| New / close / restore tab | `Ctrl+T`, `Ctrl+W`, `Ctrl+Shift+T` |
| Focus the address bar | `Ctrl+L` (also `!bang` shortcuts like `!gh ferrox`) |
| Zoom | `Ctrl+=`, `Ctrl+-`, `Ctrl+0` (per tab) |
| Bookmark a page | `Ctrl+D` |
| Developer tools | `≡ Menu → Developer tools` |
| View the page source | `≡ Menu → View source` |
| Pin / mute a tab | Right-click the tab in the strip |
| Downloads | Automatic — see `ferrox://downloads` for progress, checksum and actions |

## Internal Pages

`ferrox://newtab`, `ferrox://settings`, `ferrox://profiler`, `ferrox://history`,
`ferrox://bookmarks`, `ferrox://downloads`, `ferrox://updates`,
`ferrox://view-source?url=…`

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

## Download & Install

**Windows (recommended):** Download `ferrox-windows-x86_64.zip`, extract it, and run `ferrox.exe`. No installer is needed — Ferrox is a portable standalone browser.

1. Download the latest `ferrox-windows-x86_64.zip` from the [Releases page](https://github.com/Ahmaddragon11/ferrox/releases).
2. Verify the checksum:
   ```powershell
   CertUtil -hashfile ferrox-windows-x86_64.zip SHA256
   # compare with the value in ferrox-windows-x86_64.zip.sha256
   ```
3. Extract the ZIP to any folder (e.g. `C:\Apps\Ferrox`).
4. Double-click `ferrox.exe` to launch.

For a persistent install, copy `ferrox.exe` to `C:\Program Files\Ferrox\`.

**Linux:** Download `ferrox-linux-x86_64.tar.gz`, extract it, and run `./ferrox`.

```bash
wget https://github.com/Ahmaddragon11/ferrox/releases/download/v<VERSION>/ferrox-linux-x86_64.tar.gz
tar -xzf ferrox-linux-x86_64.tar.gz
cd ferrox-linux-x86_64
./ferrox
```

## License

Dual-licensed under **MIT** (`LICENSE-MIT`) or **Apache-2.0** (`LICENSE-APACHE`).
