# Crypto HUD

[English](README.md) | [简体中文](README.zh-CN.md)

Crypto HUD keeps crypto prices quietly visible on your Windows desktop.

No more switching to an exchange again and again just to check the market. Put a
small price card where it feels comfortable, then keep working, reading, coding,
or living your day. When you care, glance at it. When you do not, it stays out
of the way.

The project is currently in alpha. It is already useful for watching prices, but
it is still early software and may change quickly.

## Highlights

- **Lightweight always-on widgets**: native Rust + Slint desktop windows without
  Electron, Tauri, WebView, or a bundled browser runtime.
- **Low overhead, low interruption**: built for quick startup, modest memory
  use, and low background CPU while staying quietly visible.
- **Local-first experience**: the UI, widget layout, and preferences run and
  stay on your machine, with no cloud account or hosted control plane.
- **No login or authorization**: use it without accounts, OAuth, exchange API
  keys, wallet access, private keys, or seed phrases.
- **One-glance market checks**: keep prices at the edge of your desktop and
  check them without repeatedly opening an exchange.
- **Clear security boundary**: reads public market data only; no trading, wallet
  connection, or custody.

## What It Does

- Shows crypto prices in small floating desktop widgets.
- Lets you pin widgets above other windows or keep them out of the way.
- Supports multiple widget styles, from compact tickers to larger market cards.
- Lets you choose the symbols each widget tracks.
- Fetches live market data from Binance, OKX, and Hyperliquid.
- Supports light/dark themes, English and Simplified Chinese, and green-up or
  red-up market colors.
- Keeps widget positions and settings between launches.

Crypto HUD is only for viewing public market information. It does not place
trades, connect to wallets, custody funds, or ask for exchange API keys. Its
security boundary is intentionally simple: public market data comes from market
providers, while layout and preferences stay local.

## Who It Is For

Crypto HUD is useful if you:

- Watch a few coins throughout the day.
- Feel tired of repeatedly opening an exchange just to check prices.
- Prefer a lightweight native desktop tool over a full trading terminal.
- Like arranging small always-on-top widgets on your desktop.

It is probably not the right tool if you need full charting, portfolio tracking,
order entry, or alert automation today.

## Current Status

Crypto HUD is an alpha native Windows desktop app built with Rust and Slint.

- Runs as one native desktop process.
- Uses real desktop windows instead of WebView or browser-hosted UI.
- Includes a settings window, tray controls, global hide/show shortcut, local
  persistence, plugin loading, and Windows packaging scripts.
- Default shortcut: `Alt+C` to hide or show all widgets.

## Try It Locally

You need Rust. The project uses `mise` to pin the expected Rust toolchain.

Review `mise.toml` first, then install the toolchain:

```powershell
mise trust
mise install
```

Check that the project builds:

```powershell
mise run check
```

Run the app:

```powershell
mise run run-app
```

To launch a specific number of widgets:

```powershell
cargo run -p crypto-hud -- --widgets 3
```

## Basic Use

- Drag a price card to move it.
- Click the tray icon to open settings.
- Right-click the tray icon for settings and quit actions.
- Use settings to add widgets, choose symbols, change opacity, switch themes,
  configure startup behavior, and change app-level market preferences.
- Use `Alt+C` to hide or show all widgets.

Layout and settings are saved automatically. For isolated testing, set a custom
state directory:

```powershell
$env:CRYPTO_HUD_STATE_DIR = "$PWD\.crypto-hud-state"
mise run run-app
```

## For Contributors

Useful development commands:

```powershell
mise run format-check
mise run check
mise run test
mise run format
mise run run-app
powershell -ExecutionPolicy Bypass -File .\scripts\gui-smoke.ps1
```

The code is split into small Rust crates:

```text
crates/
  crypto-hud-core/          market symbols, formatting, alert primitives
  crypto-hud-market/        market data fetching
  crypto-hud-runtime/       widget runtime view contracts
  crypto-hud-shell-state/   settings and persisted layout state
  crypto-hud/              native desktop shell and Slint UI
```

Built-in and local widget plugin contracts live in
`crates/crypto-hud/plugins/README.md`.

See `CONTRIBUTING.md` for contribution guidelines and `SECURITY.md` for
security reporting.

## Release Packaging

Crypto HUD currently uses local Windows release scripts rather than a GitHub
Actions release workflow.

```powershell
cargo test --workspace
powershell -ExecutionPolicy Bypass -File .\scripts\gui-smoke.ps1
powershell -ExecutionPolicy Bypass -File .\scripts\release-process-check.ps1
powershell -ExecutionPolicy Bypass -File .\scripts\package-smoke.ps1 -SkipBuild
powershell -ExecutionPolicy Bypass -File .\scripts\update-smoke.ps1 -SkipBuild
powershell -ExecutionPolicy Bypass -File .\scripts\package-windows.ps1 -Version v0.1.0-alpha.1
```

The package script creates a Windows zip, checksum, and release manifest in
`dist/`. The installer verifies package contents before copying files. Optional
Windows Authenticode signing is supported through the signing environment
variables documented in `scripts/package-windows.ps1`.

## Roadmap

- Better provider health, stale-data, and error states.
- Price and 24-hour change alerts.
- Duplicate, rename, reorder, and per-widget visibility controls.
- Better first-launch placement and recovery.
- A richer installer format.

## License

Crypto HUD is licensed under the MIT License. See `LICENSE`.
