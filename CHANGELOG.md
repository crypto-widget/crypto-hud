# Changelog

All notable changes to Crypto HUD will be documented in this file.

The format is based on Keep a Changelog, and this project follows SemVer.

## 1.0.0 - Unreleased

### Changed

- Added Coinbase as a public spot market data source.
- Prepared the repository for alpha open-source publication.
- Renamed local development tasks and documentation from prototype wording to
  app-oriented wording.
- Migrated visible layout state naming to `layouts.json`, while preserving
  legacy state loading.
- Simplified the proxy settings panel with a localized HTTP and SOCKS5 example.

### Fixed

- Report partial market-source failures and derive widget freshness from the
  oldest selected pair instead of masking stale quotes with a newer update.
- Preserve every distinct pair required by multiple widgets and fetch pairs in
  bounded parallel batches.
- Keep ticker prices live when chart history is temporarily unavailable.
- Keep transient market-feed failures in the HUD and diagnostic logs instead
  of displaying native operating-system notifications.
- Retry failed coin icons after a cooldown or immediately after proxy changes.
- Activate the running settings window when a second app instance is launched.
- Reject uninstall requests that do not point to a verified Crypto HUD install.
- Removed the unused market fallback setting and its persisted runtime fields.

### Documentation

- Added contribution and security policy documents.
- Clarified the local Windows release packaging process.

## 0.8.3

### Added

- Native Rust + Slint desktop widget shell.
- Frameless, draggable, always-on-top market widgets.
- Live market data from Binance, Coinbase, OKX, and Hyperliquid.
- Native settings window, tray controls, shortcuts, localization, theme
  settings, proxy settings, and Windows packaging scripts.
