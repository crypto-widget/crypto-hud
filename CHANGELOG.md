# Changelog

All notable changes to Crypto HUD will be documented in this file.

The format is based on Keep a Changelog, and this project follows SemVer.

## 1.0.5 - 2026-10-13

### Fixed

- Smooth downscaled widget artwork and coin icons with alpha-aware sampling
  across widget scales and display DPI.
- Preserve vector SVG icons and reuse bounded raster icon caches during quote
  refreshes.

### Changed

- Add optional, backward-compatible image and icon resampling callbacks for
  local Slint plugins.
- Move native GUI, packaging, and update smoke checks to tag-triggered releases
  while retaining daily code quality checks and dependency auditing.

### Documentation

- Refresh Market Compass and Trust Card previews and document image sampling
  and reproducible preview generation.

## 1.0.4 - 2026-10-10

### Added

- Add the Mint Tile widget with light and dark themes and accessible layout locking.
- Add a compact feedback entry to the settings window.

### Changed

- Refresh Focus Ticker, Market Compass, Status Strip, and Trust Card with compact
  layouts, matching themes, and floating keyboard-accessible lock controls.
- Format widget and taskbar prices consistently within eight display characters
  while preserving cached values and alert calculations.
- Expose optional per-quote chart endpoints and scoped market-health warnings
  without breaking older plugin contracts or serialized runtime views.

### Fixed

- Check rounded corners using alpha-preserving Slint snapshots while retaining
  native DPI and scaled-content checks.
- Improve widget-management spacing, keyboard access, localized default names,
  and previews that follow each selected widget theme.
- Show explicit symbol-catalog loading and failure states and reset transient
  menus and scroll positions when navigating or creating widgets.
- Keep native smoke interactions reliable in constrained viewports and align
  legacy-layout checks with the compact Trust Card and square Mint Tile designs.

### Documentation

- Add Microsoft Store links across all supported README languages and document
  the compact widget presentation contract.

## 1.0.3 - 2026-10-01

### Changed

- Load symbol catalogs with bounded concurrent requests and show completed
  results without waiting for slower market sources.
- Keep cached symbol choices available during refreshes and discard obsolete
  catalog results after settings change.

### Fixed

- Account for monitor DPI when checking native window layouts and capturing
  screenshots in GUI smoke tests.
- Preserve Status Strip's rounded corners at small widget scales and fractional
  monitor DPI without clipping its decorative gradients.

### Security

- Reject external and nested SVG image references in plugin resources while
  retaining support for local vector fragments and embedded PNG/JPEG images.
- Require a patched rustls dependency for TLS handshake validation.

## 1.0.2 - 2026-09-05

### Fixed

- Preserve unreadable layouts until a recovery backup can be safely created.
- Deliver completed market quotes without waiting for unrelated requests and
  preserve the original quote timestamps when updating the UI.
- Revalidate temporary fallback routes when subscribed pairs change.
- Protect unrelated files in custom installation directories during upgrades
  and resolve the default uninstall target from the installed script location.

### Security

- Require patched webbrowser and event-listener dependencies used by Slint.

### Documentation

- Document installation ownership checks, state recovery, and market delivery.

## 1.0.1 - Unreleased

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
