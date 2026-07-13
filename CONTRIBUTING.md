# Contributing to Crypto HUD

Thanks for taking the time to improve Crypto HUD. This project is an alpha
native Rust + Slint desktop app for market-price widgets, so changes should stay
small, testable, and aligned with the existing native shell architecture.

## Development Setup

1. Install `mise`.
2. Review `mise.toml`, then trust and install the pinned toolchain:

```powershell
mise trust
mise install
```

3. Check the workspace:

```powershell
mise run ci
```

## Common Commands

```powershell
mise run run-app
mise run format
mise run clippy
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\gui-smoke.ps1
```

Use `CRYPTO_HUD_STATE_DIR` when you need isolated local state:

```powershell
$env:CRYPTO_HUD_STATE_DIR = "$PWD\.crypto-hud-state"
mise run run-app
```

## Code Style

- Keep the Rust workspace split intact: domain primitives, market feeds,
  runtime contracts, persisted shell state, and the Slint shell live in separate
  crates.
- Prefer native Slint windows and Rust helpers over browser or WebView
  dependencies.
- Keep public state migrations backward compatible.
- For UI text, locale routing, or RTL behavior, follow `LOCALIZATION.md`.
- Run `mise run format` before sending larger code changes.

## Tests

For code changes, run at least:

```powershell
mise run ci
```

For UI, shell, packaging, or installer changes, also run the relevant PowerShell
smoke script from `scripts/`.

## Issues and Pull Requests

- Use issues for bugs, feature proposals, and UX reports.
- Include Windows version, app version or commit, market source, and relevant
  reproduction steps for bug reports.
- Keep pull requests focused on one behavioral change.
- Mention any state migration, packaging, or update-flow impact in the PR
  description.

## Plugin Development

Built-in and local widget plugin contracts are documented in
`crates/crypto-hud/plugins/README.md`.
