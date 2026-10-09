# Spotlight for Windows

A personal, Spotlight-style launcher built with **Tauri 2 (Rust) + React**.
Press **Alt + Space** → type → **Enter**.

## Setup (one time)

1. Install the prerequisites:
   - Node.js 18+
   - Rust: https://rustup.rs
   - Microsoft C++ Build Tools ("Desktop development with C++")
   - WebView2 is already on Windows 10/11
2. In this folder:
   ```
   npm install
   npx tauri icon path\to\any-square-logo.png   # generates src-tauri/icons (required to build)
   npm run tauri dev
   ```
3. Release build (fast, small exe + installer): `npm run tauri build`

## How it stays fast

- The window is created once at startup and only **shown/hidden** on the hotkey, so there's no cold start.
- The app index lives **in memory** and is built on a background thread.
- Each keystroke is a fuzzy match in RAM; stale responses are dropped in the UI.
- Results you launch often get a usage boost (`usage.json` in AppData).

## Structure

```
src/                     React UI
  App.tsx                keyboard nav, query → results
  components/            SearchBar, ResultList
  styles/app.css         Spotlight look (blur, rounded, pop-in)
src-tauri/
  tauri.conf.json        borderless, transparent, acrylic, always-on-top, hidden at start
  src/lib.rs             app state + startup
  src/window.rs          Alt+Space hotkey, tray icon, hide on blur
  src/commands.rs        search / execute commands
  src/ranking.rs         usage-based ranking
  src/providers/         one file per feature
    apps.rs              Start Menu apps
    calculator.rs        "45*12", "sqrt(144)"
    system.rs            lock, sleep, shutdown, settings pages
    web.rs               web search fallback
```

## Adding a feature

Create `src-tauri/src/providers/<name>.rs` with a `query(q) -> Vec<SearchResult>`,
register it in `providers/mod.rs`, and call it in `commands::search`.

## Roadmap

- [ ] Copy calculator result to clipboard on Enter
- [ ] UWP / Store apps (shell:AppsFolder)
- [ ] App icons (extract + cache)
- [ ] File search: NTFS MFT/USN journal indexer (Everything-style)
- [ ] Unit & currency conversion
- [ ] Clipboard history
- [ ] Settings window (hotkey, theme, excluded folders)
- [ ] Autostart toggle (plugin already included)
- [ ] AI actions / semantic file search
