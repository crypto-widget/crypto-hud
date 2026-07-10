# Changelog

All notable changes to Crypto HUD will be documented in this file.

The format is based on Keep a Changelog, and this project follows SemVer while
it remains in the `0.x` alpha series.

## Unreleased

### Changed

- Added the initial macOS app bundle and DMG workflow, including Apple Silicon,
  Intel, universal binary, Developer ID signing, notarization, and package smoke
  test support.
- Added GitHub-hosted Apple Silicon and Intel build, test, and package smoke
  validation using the mise-managed toolchain.
- Added reproducible Zig-backed Apple target checks for non-macOS hosts and
  Mach-O deployment-target, system-library, signature, and manifest validation
  to the macOS package smoke test.
- Added macOS platform behavior for display sizing, system theme detection,
  usable work-area placement, settings-window activation, notifications,
  shortcut labels, and architecture-specific update assets.
- Scoped Windows-only Rust dependencies to Windows targets.
- Added Coinbase as a public spot market data source.
- Prepared the repository for alpha open-source publication.
- Renamed local development tasks and documentation from prototype wording to
  app-oriented wording.
- Migrated visible layout state naming to `layouts.json`, while preserving
  legacy state loading.

### Documentation

- Added macOS development, packaging, signing, notarization, and hardware-test
  instructions.
- Added contribution and security policy documents.
- Clarified the local Windows release packaging process.

## 0.8.3 - Unreleased

### Added

- Native Rust + Slint desktop widget shell.
- Frameless, draggable, always-on-top market widgets.
- Live market data from Binance, Coinbase, OKX, and Hyperliquid.
- Native settings window, tray controls, shortcuts, localization, theme
  settings, proxy settings, and Windows packaging scripts.
