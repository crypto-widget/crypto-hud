<p align="center">
  <img src="crates/crypto-hud/ui/icon.png" width="88" alt="Crypto HUD logo">
</p>

<h1 align="center">Crypto HUD — Native Crypto Desktop Widget for Windows</h1>

<p align="center">
  <strong>Your market, always within a glance.</strong><br>
  Native crypto widgets that stay on your Windows desktop without taking over it.
</p>

<p align="center">
  <a href="README.md">English</a> ·
  <a href="README.zh-CN.md">简体中文</a> ·
  <a href="README.zh-TW.md">繁體中文</a> ·
  <a href="README.es.md">Español</a> ·
  <a href="README.pt-BR.md">Português</a> ·
  <a href="README.vi.md">Tiếng Việt</a><br>
  <a href="README.id.md">Bahasa Indonesia</a> ·
  <a href="README.tr.md">Türkçe</a> ·
  <a href="README.ko.md">한국어</a> ·
  <a href="README.ja.md">日本語</a> ·
  <a href="README.ru.md">Русский</a> ·
  <a href="README.ar.md">العربية</a>
</p>

<p align="center">
  <img alt="Platform: Windows" src="https://img.shields.io/badge/platform-Windows-0078d4?style=flat-square&logo=windows11&logoColor=white">
  <img alt="Built with Rust" src="https://img.shields.io/badge/built_with-Rust-dea584?style=flat-square&logo=rust&logoColor=white">
  <a href="https://slint.dev"><img alt="Made with Slint" height="20" src="https://raw.githubusercontent.com/slint-ui/slint/v1.17.0/logo/MadeWithSlint-logo-whitebg.png"></a>
  <img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-22c55e?style=flat-square">
</p>

<p align="center">
  <a href="#quick-start"><strong>Run it locally</strong></a> ·
  <a href="#widget-gallery"><strong>Explore widgets</strong></a> ·
  <a href="CUSTOM_UI_PLUGIN_DEVELOPMENT.md"><strong>Build a widget</strong></a>
</p>

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-dark.png">
    <source media="(prefers-color-scheme: light)" srcset="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-light.png">
    <img src="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-dark.png" width="500" alt="Market Compass widget showing BTC price, a candlestick chart, and surrounding market pairs">
  </picture>
</p>

<p align="center">
  <sub>Live prices at the edge of your workspace. No exchange tab. No wallet connection. No noise.</sub>
</p>

---

Crypto HUD is a lightweight, local-first crypto desktop widget for Windows,
built for people who want to follow a few coins without living inside a trading
terminal. Place a widget where it feels natural, keep working, and glance over
only when the market matters.

<table>
  <tr>
    <td width="25%"><strong>⚡ Native & light</strong><br><sub>Rust + Slint. No Electron, Tauri, WebView, or bundled browser runtime.</sub></td>
    <td width="25%"><strong>👀 Glanceable</strong><br><sub>Draggable, always-on-top widgets keep the important numbers in view.</sub></td>
    <td width="25%"><strong>🔒 Local-first</strong><br><sub>Your layout and preferences stay on your machine. No account or API key.</sub></td>
    <td width="25%"><strong>🙈 Quiet on demand</strong><br><sub>Hide or restore every widget with <kbd>Alt</kbd> + <kbd>C</kbd>.</sub></td>
  </tr>
</table>

<p align="center">
  <strong>Measured in the default test: 0.070% average CPU · about 20 MiB private memory</strong><br>
  <sub>One widget · 3 market pairs · 5-second refresh · <a href="docs/performance-reports/README.md">Full performance report →</a></sub>
</p>

## Widget gallery

Choose a compact ticker, a richer chart, or a multi-market view. Built-in
widgets use the same plugin contract available to custom widgets.

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-dark.png">
    <source media="(prefers-color-scheme: light)" srcset="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-light.png">
    <img src="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-dark.png" width="820" alt="Focus Ticker showing BTC price, daily change, and a sparkline">
  </picture>
</p>

<p align="center"><strong>Focus Ticker</strong> — one market, zero distraction.</p>

<table>
  <tr>
    <td width="50%" align="center">
      <picture>
        <source media="(prefers-color-scheme: dark)" srcset="crates/crypto-hud/plugins/com.cryptohud.trust-card/ui/preview-dark.png">
        <source media="(prefers-color-scheme: light)" srcset="crates/crypto-hud/plugins/com.cryptohud.trust-card/ui/preview-light.png">
        <img src="crates/crypto-hud/plugins/com.cryptohud.trust-card/ui/preview-dark.png" width="500" alt="Trust Card widget with BTC price and a larger market chart">
      </picture>
    </td>
    <td width="50%" align="center">
      <picture>
        <source media="(prefers-color-scheme: dark)" srcset="crates/crypto-hud/ui/previews/quote-board-dark.png">
        <source media="(prefers-color-scheme: light)" srcset="crates/crypto-hud/ui/previews/quote-board-light.png">
        <img src="crates/crypto-hud/ui/previews/quote-board-dark.png" width="360" alt="Quote Board widget showing several crypto trading pairs">
      </picture>
    </td>
  </tr>
  <tr>
    <td align="center"><strong>Trust Card</strong><br><sub>More context when one pair deserves attention.</sub></td>
    <td align="center"><strong>Quote Board</strong><br><sub>A compact pulse check across several pairs.</sub></td>
  </tr>
</table>

<details>
  <summary><strong>Prefer a light desktop?</strong> See the light theme</summary>
  <br>
  <table>
    <tr>
      <td width="50%" align="center"><img src="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-light.png" width="440" alt="Market Compass in the light theme"></td>
      <td width="50%" align="center"><img src="crates/crypto-hud/plugins/com.cryptohud.trust-card/ui/preview-light.png" width="480" alt="Trust Card in the light theme"></td>
    </tr>
  </table>
</details>

## Built for the background

- **Real desktop widgets** — move them freely, keep them always on top, and
  restore the same layout next time.
- **Four public market sources** — Binance, Coinbase, OKX, and Hyperliquid.
- **Flexible appearance** — multiple widget styles, light and dark themes,
  opacity controls, and configurable market colors.
- **Global focus switch** — press <kbd>Alt</kbd> + <kbd>C</kbd> to hide or show
  the entire HUD at once.
- **12 interface languages** — including Simplified and Traditional Chinese,
  English, Japanese, Korean, Spanish, Portuguese, and RTL Arabic.
- **Plugin-ready** — create local Slint widgets with declared data requirements,
  themes, parameters, and preview images.

Supported UI languages: `en`, `zh-CN`, `zh-TW`, `es-419`, `pt-BR`, `vi`, `id`,
`tr`, `ko`, `ja`, `ru`, and `ar`.

> [!IMPORTANT]
> Crypto HUD is view-only by design. It reads public market data, but it does
> not place trades, connect to wallets, custody funds, or ask for private keys,
> seed phrases, exchange accounts, or API keys.

## Quick start

Crypto HUD is built for Windows. The repository uses `mise` to pin Rust
`1.96` and provides a one-command local launch task.

```powershell
git clone https://github.com/crypto-widget/crypto-hud.git
cd crypto-hud
mise trust
mise install
mise run run-app
```

That starts one instance of each bundled widget. To launch a specific number
of widgets instead:

```powershell
cargo run -p crypto-hud -- --widgets 3
```

Once running:

1. Drag any widget to place it.
2. Click the tray icon to open settings.
3. Add widgets, select symbols, switch themes, and tune opacity.
4. Press <kbd>Alt</kbd> + <kbd>C</kbd> whenever you want a clean desktop.

Positions and preferences are saved automatically.

## Make it yours

Crypto HUD's bundled widgets are powered by a local plugin system. A widget can
declare its symbols, market-data capabilities, sizes, themes, and settings
without owning network or filesystem access.

- Read the [custom UI plugin guide](CUSTOM_UI_PLUGIN_DEVELOPMENT.md).
- Browse the [plugin contract and bundled examples](crates/crypto-hud/plugins/README.md).
- Start from an existing Slint widget and give the market a different shape.

## Development

```powershell
mise run format-check
mise run check
mise run test
mise run run-app
```

<details>
  <summary><strong>Repository structure and GUI smoke tests</strong></summary>
  <br>

  ```text
  crates/
    crypto-hud-core/          market symbols, formatting, and alert primitives
    crypto-hud-market/        public market-data fetching
    crypto-hud-runtime/       widget runtime view contracts
    crypto-hud-shell-state/   settings and persisted layout state
    crypto-hud/               native Windows shell and Slint UI
  ```

  ```powershell
  powershell -ExecutionPolicy Bypass -File .\scripts\gui-smoke.ps1
  powershell -ExecutionPolicy Bypass -File .\scripts\gui-settings-interaction-smoke.ps1
  powershell -ExecutionPolicy Bypass -File .\scripts\single-instance-smoke.ps1
  ```
</details>

<details>
  <summary><strong>Windows release packaging</strong></summary>
  <br>

  Pushing a `vX.Y.Z` tag that exactly matches the workspace version triggers
  `.github/workflows/release-portable.yml`. It validates the tagged source,
  builds an unsigned Windows x64 portable zip, creates its SHA-256 file, and
  publishes both assets in a GitHub Release. The portable archive contains no
  installer, uninstaller, or update PowerShell scripts. It is installation-free,
  but application state still uses the normal user profile directory. Extract
  it to a fixed directory before enabling autostart.

  ```powershell
  # Run this only after the version commit is on the default branch and CI passes.
  git tag -a v0.9.8 -m "Release v0.9.8"
  git push origin v0.9.8
  ```

  Without Authenticode signing, Windows may show a SmartScreen warning. The
  SHA-256 file detects a changed download but does not authenticate its publisher.
  A local copy of the same portable package can be produced with:

  ```powershell
  powershell -ExecutionPolicy Bypass -File .\scripts\package-portable-windows.ps1 -Version v0.9.8
  ```

  The existing installable package path remains separate. It creates a zip,
  checksum, and signed release manifest in `dist/`. Production installable
  packages must be Authenticode signed; smoke scripts use an explicit local-only
  unsigned override. Bundled widgets are installed under `plugins/`; preview
  images and the application icon are installed under `resources/previews/`
  and `resources/icon.ico`, with every shipped file bound to signed release
  integrity metadata.

  ```powershell
  cargo test --locked --workspace
  cargo audit
  powershell -ExecutionPolicy Bypass -File .\scripts\release-process-check.ps1
  powershell -ExecutionPolicy Bypass -File .\scripts\package-smoke.ps1 -SkipBuild
  powershell -ExecutionPolicy Bypass -File .\scripts\update-smoke.ps1 -SkipBuild
  # Configure CRYPTO_HUD_SIGN_CERT_PATH (or CRYPTO_HUD_SIGN_CERT_BASE64) and
  # CRYPTO_HUD_SIGN_CERT_PASSWORD first. Signed packages always rebuild.
  powershell -ExecutionPolicy Bypass -File .\scripts\package-windows.ps1 -Version v0.9.8 -Sign
  ```

  For a production first install, verify the installer before executing any of
  its code. Confirm that `Status` is `Valid` and that `SignerCertificate.Subject`
  matches the publisher identity published with the release, then use
  `AllSigned` rather than bypassing PowerShell policy:

  ```powershell
  Get-AuthenticodeSignature -LiteralPath .\install.ps1 | Format-List Status,SignerCertificate
  powershell -ExecutionPolicy AllSigned -File .\install.ps1
  ```

  `-ExecutionPolicy Bypass` and `CRYPTO_HUD_ALLOW_UNSIGNED_SMOKE=1` are reserved
  for the repository's isolated unsigned smoke tests; they are not production
  installation options.
</details>

## Roadmap

Current priorities include stronger provider health states, price and 24-hour
change alerts, richer widget management, better first-launch placement, and a
more complete installer.

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md), review the
[changelog](CHANGELOG.md), or read the [security policy](SECURITY.md) before
reporting a vulnerability.

## License

MIT © Crypto HUD Contributors
