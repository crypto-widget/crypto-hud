<h1 align="center">Crypto HUD</h1>

<p align="center">
  A lightweight, local-first market HUD for Windows and macOS.
</p>

<p align="center">
  <a href="README.md">English</a> · <a href="README.zh-CN.md">简体中文</a>
</p>

<p align="center">
  <img alt="Status: alpha" src="https://img.shields.io/badge/status-alpha-f59e0b">
  <img alt="Platform: Windows and macOS" src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS-0078d4">
  <img alt="Runtime: native" src="https://img.shields.io/badge/runtime-native-22c55e">
  <img alt="No account required" src="https://img.shields.io/badge/account-not%20required-14b8a6">
  <img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-111827">
</p>

> Keep prices visible enough to glance at, quiet enough to forget about.

Crypto HUD keeps crypto prices quietly visible on your desktop.

No more switching to an exchange again and again just to check the market. Put a
small price card where it feels comfortable, then keep working, reading, coding,
or living your day. When you care, glance at it. When you do not, it stays out
of the way.

The project is currently in alpha. It is already useful for watching prices, but
it is still early software and may change quickly.

## Highlights

- **Glanceable prices**: keep market prices at the edge of your desktop without
  repeatedly opening an exchange.
- **Light and low-overhead**: native Rust + Slint, with no Electron, Tauri,
  WebView, or bundled browser runtime.
- **Local and permissionless**: layout and preferences stay on your machine; no
  accounts, OAuth, API keys, wallet access, private keys, or seed phrases.
- **One-key hide/show**: press `Alt+C` to hide every widget when you need a
  clean desktop, then bring them back just as quickly.
- **View-only by design**: reads public market data only; no trading, wallet
  connection, or custody.

## What It Does

- Shows draggable, optionally always-on-top desktop price widgets.
- Tracks chosen symbols and fetches live data from Binance, Coinbase, OKX, and
  Hyperliquid.
- Supports widget styles, light/dark themes, English and Simplified Chinese,
  and market color preferences.
- Saves widget positions and settings between launches.

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

Crypto HUD is an alpha native Windows and macOS desktop app built with Rust and
Slint. Windows is the currently validated platform; the macOS implementation and
packaging flow are ready for their first hardware validation.

- Runs as one native desktop process.
- Uses real desktop windows instead of WebView or browser-hosted UI.
- Includes a main window, tray controls, global hide/show shortcut, local
  persistence, plugin loading, and native packaging scripts.
- Default shortcut: `Alt+C` on Windows or `Option+C` on macOS.

## Try It Locally

You need Rust. The project uses `mise` to pin the expected Rust toolchain.

Review `mise.toml` first, then install the toolchain:

```shell
mise trust
mise install
```

Check that the project builds:

```shell
mise run check
```

Run the app:

```shell
mise run run-app
```

To launch a specific number of widgets:

```shell
cargo run -p crypto-hud -- --widgets 3
```

## Basic Use

- Drag a price card to move it.
- Click the tray icon to open the main window.
- Right-click the tray icon for settings and quit actions.
- Use settings to add widgets, choose symbols, change opacity, switch themes,
  configure startup behavior, and change app-level market preferences.
- Use `Alt+C` on Windows or `Option+C` on macOS to hide or show all widgets.

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
powershell -ExecutionPolicy Bypass -File .\scripts\gui-settings-interaction-smoke.ps1
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

Crypto HUD currently uses local release scripts rather than a GitHub Actions
release workflow. Windows packaging is validated locally. macOS packaging must
run on a Mac and still requires the hardware checklist in [MACOS.md](MACOS.md).
The macOS CI workflow builds and smoke-tests ARM64, Intel, and universal
artifacts, but does not publish releases or access signing credentials.

```powershell
cargo test --workspace
powershell -ExecutionPolicy Bypass -File .\scripts\gui-smoke.ps1
powershell -ExecutionPolicy Bypass -File .\scripts\release-process-check.ps1
powershell -ExecutionPolicy Bypass -File .\scripts\package-smoke.ps1 -SkipBuild
powershell -ExecutionPolicy Bypass -File .\scripts\update-smoke.ps1 -SkipBuild
powershell -ExecutionPolicy Bypass -File .\scripts\package-windows.ps1 -Version v0.9.2
```

The package script creates a Windows zip, checksum, and release manifest in
`dist/`. The installer verifies package contents before copying files. Optional
Windows Authenticode signing is supported through the signing environment
variables documented in `scripts/package-windows.ps1`.

On macOS, use the `mise`-managed toolchain to build an app bundle and DMG for
the current architecture:

```shell
mise run package-macos
mise run macos-package-smoke
```

The macOS script also supports `--arch universal`, Developer ID signing, and
Apple notarization. See [MACOS.md](MACOS.md) for setup, commands, release
credentials, generated artifacts, and the pending manual test matrix.

## Roadmap

- Better provider health, stale-data, and error states.
- Price and 24-hour change alerts.
- Duplicate, rename, reorder, and per-widget visibility controls.
- Better first-launch placement and recovery.
- More automated cross-platform release validation.

## License

Crypto HUD is licensed under the MIT License. See `LICENSE`.
