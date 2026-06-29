# kopipe

[![CI](https://github.com/lakepuka/kopipe/actions/workflows/ci.yml/badge.svg)](https://github.com/lakepuka/kopipe/actions/workflows/ci.yml)

A lightweight, **fully local** Windows clipboard‑history app. kopipe lives in the
system tray, records what you copy (text, files/folders, and images), and lets
you paste any past item back — fast. Double‑tap **Shift** to summon it anywhere.

Your clipboard is sensitive, so kopipe keeps everything on your PC: no account,
no cloud, no telemetry. See [Privacy](#privacy).

> kopipe is Windows‑only (it relies on Win32 clipboard APIs).

## Features

- **Private by design**: history lives only on your machine — nothing is uploaded
- Automatic clipboard history: text, files/folders (`CF_HDROP`), images (PNG)
- Rich text: keeps HTML so links/formatting survive a paste; view Plain / HTML / Web
- Bookmark items, regex / keyword search, deduplicated history
- Summon with a double‑tap of Shift (or Ctrl, or a custom shortcut)
- Themes, English / 日本語, optional launch at PC login (runs in the tray)

## Install

1. Download the latest `kopipe_x.y.z_x64-setup.exe` from the
   [Releases page](https://github.com/lakepuka/kopipe/releases).
2. Run the installer.
3. On first launch a short setup guide appears (language, welcome, optional
   auto‑start). After that kopipe stays in the tray — press **Shift twice** to open it.

Requirements: Windows 10/11. The WebView2 runtime is preinstalled on Windows 11
(the installer fetches it automatically if missing).

> The installer isn't code‑signed yet, so Windows SmartScreen may warn
> "unknown publisher". Choose **More info → Run anyway**.

## Usage

- **Shift × 2**: show / hide the window (configurable in Settings)
- **Click a row**: paste it into the app you were just using
- **Right‑click a row** (or the ⋮ button): view, copy, paste as plain text,
  bookmark, delete
- **Tray icon**: show window, open settings, quit

## Privacy

kopipe runs entirely on your computer. Your clipboard history is stored in a
local SQLite database at `%APPDATA%\io.github.lakepuka.kopipe\kopipe.db` and
never leaves the device. There are no accounts, no analytics, and no servers —
kopipe makes no network connections during normal use.

> One exception: an **optional update check** (on by default, toggle in
> Settings → Updates) contacts GitHub once on launch to compare version numbers
> and shows a notice if a newer release exists. Nothing from your clipboard is
> ever transmitted, and you can turn it off.

## Build from source

Prerequisites: [Node.js](https://nodejs.org/) + [pnpm](https://pnpm.io/),
[Rust](https://www.rust-lang.org/tools/install), and the
[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for Windows
(MSVC build tools + WebView2).

```bash
pnpm install        # install JS deps
pnpm tauri dev      # run in development
pnpm tauri build    # build the installer
```

The installer is written to `src-tauri/target/release/bundle/nsis/`.

Frontend checks: `pnpm verify` runs format check, lint, TypeScript, and Vitest.
Rust checks: `cargo fmt --check`, `cargo check`, and `cargo test` in `src-tauri/`.

To fill the history with sample data while developing: `pnpm run seed [count]`
(defaults to 1000). It writes directly to the local DB above.

## Tech stack

Tauri 2 · React 19 + TypeScript · Rust · SQLite (rusqlite).

## License

[MIT](LICENSE) © 2026 lakepuka
