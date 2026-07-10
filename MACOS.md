# macOS development and release guide

The macOS implementation can be prepared from any platform, but compilation,
bundle validation, signing, notarization, and GUI behavior must be verified on
real macOS. Until the checklist at the end of this document passes, macOS is a
candidate platform rather than a validated release platform.

## Supported targets

- macOS 12.0 or newer.
- Apple Silicon (`aarch64-apple-darwin`, package label `arm64`).
- Intel (`x86_64-apple-darwin`, package label `x64`).
- A universal DMG containing both architectures.

## Prepare a Mac

Install Xcode Command Line Tools and `mise`, then run these commands from the
repository root:

```shell
xcode-select --install
mise trust
mise install
mise run format-check
mise run macos-scripts-check
mise run check
mise run test
```

`mise.toml` is authoritative for the Rust and Zig versions and installs both
Apple Rust targets. Avoid running packaging with an unrelated global Rust
installation.

`mise run macos-scripts-check` combines ShellCheck with platform-independent
tests for thin and universal Mach-O metadata, deployment-target rejection,
system-library filtering, and manifest JSON escaping. On Windows it locates the
Git Bash installed alongside Git automatically.

The Rust-only portions of both macOS targets can also be checked from a
non-macOS host. The task uses the mise-managed Zig compiler for target C/assembly
dependencies. It does not link an application or replace Mac testing:

```shell
mise run macos-cross-check
```

## Run from source

```shell
mise run run-app
```

For isolated state:

```shell
CRYPTO_HUD_STATE_DIR="$PWD/.crypto-hud-state" \
  mise exec -- cargo run -p crypto-hud -- --each-widget
```

The macOS shortcut corresponding to the stored `Alt+C` preference is displayed
as `Option+C`.

## Build packages

Build for the current Mac architecture:

```shell
mise run package-macos
```

Build explicit architecture variants:

```shell
mise run package-macos -- --arch arm64
mise run package-macos -- --arch x64
mise run package-macos -- --arch universal
```

The script creates these artifacts in `dist/`:

- `Crypto HUD.app`
- `crypto-hud-<version>-macos-<arch>.dmg`
- the matching `.dmg.sha256`
- a `.manifest.json` containing target, commit, signing, and notarization status

Without a Developer ID, the app receives an ad-hoc signature for local testing.
An ad-hoc signed DMG is not suitable for public distribution.

## Developer ID signing and notarization

The signing identity must be a `Developer ID Application` certificate available
in the login keychain. Confirm its exact name with:

```shell
security find-identity -v -p codesigning
```

Store App Store Connect credentials in a keychain profile once:

```shell
xcrun notarytool store-credentials "crypto-hud-notary" \
  --apple-id "APPLE_ID" \
  --team-id "TEAM_ID" \
  --password "APP_SPECIFIC_PASSWORD"
```

Then sign, submit, wait for notarization, and staple the ticket:

```shell
CRYPTO_HUD_MACOS_SIGN_IDENTITY="Developer ID Application: Example (TEAMID)" \
CRYPTO_HUD_MACOS_NOTARY_PROFILE="crypto-hud-notary" \
mise run package-macos -- --arch universal --notarize
```

The same settings may be passed through `--sign-identity` and
`--notary-profile`. Do not commit certificate files, passwords, API keys, or
keychain exports.

## Automated Mac smoke test

On the target Mac, run:

```shell
mise run macos-package-smoke
```

This builds the package, validates its plist, manifest, code signature, Mach-O
architectures, deployment target, and system-library dependencies, verifies and
mounts the DMG, checks the Applications shortcut, launches the bundled binary,
and requires the application to write its GUI-ready marker before exiting.

For a previously built target:

```shell
mise run macos-package-smoke -- --arch arm64 --skip-build
```

## GitHub-hosted Mac validation

`.github/workflows/macos.yml` runs the same `mise` checks on GitHub-hosted
Apple Silicon and Intel Macs. The Apple Silicon job also creates and launches a
universal package, while the Intel job launches the native x64 package. The
workflow uses ad-hoc signing and never accesses release signing or notarization
credentials. Each successful job uploads its smoke-tested DMG, checksum, and
manifest as a 14-day workflow artifact, so a candidate can be downloaded even
before a local Mac is available. These ad-hoc artifacts are for testing only;
public distribution still requires Developer ID signing and notarization.

It runs for relevant pull requests and pushes to `main`, `dev`, or `macos`, and
can also be started manually with `workflow_dispatch`. A green workflow is
strong build and smoke-test evidence, but it does not replace the Finder,
Gatekeeper, display, login-item, and interaction checks on the target hardware.

## Manual hardware acceptance checklist

Run the checklist on at least one Apple Silicon Mac. Before publishing an Intel
or universal package, repeat the launch checks on Intel hardware or a genuine
Intel macOS environment.

- [ ] Opening the DMG in Finder shows `Crypto HUD.app` and the Applications link.
- [ ] Dragging the app to Applications and launching it succeeds without a
      Gatekeeper warning for a notarized release.
- [ ] `spctl --assess --type execute --verbose=4 "/Applications/Crypto HUD.app"`
      reports an accepted Developer ID and notarized source.
- [ ] The settings window opens, remains reachable from the tray, and receives
      keyboard focus.
- [ ] The tray icon and menu appear; Settings and Quit both work.
- [ ] Price widgets are transparent, draggable, correctly scaled on Retina, and
      respect pin-to-top.
- [ ] Widget positions remain on screen after restart and after changing display
      resolution, connecting a monitor, or disconnecting a monitor.
- [ ] `Option+C` hides and restores all widgets.
- [ ] Start at login persists after logout/login and can be disabled cleanly.
- [ ] Dark/system and light themes resolve correctly.
- [ ] Market data works both directly and through the configured HTTP/SOCKS proxy.
- [ ] Update notifications select the matching macOS architecture DMG and exact
      checksum rather than a Windows asset.
- [ ] System notifications appear and handle quotes, backslashes, Chinese text,
      and line breaks safely.
- [ ] English and Simplified Chinese UI render correctly.
- [ ] The app exits without leaving a process or stale tray icon.

Known pre-validation limitation: tray-hover detection uses Windows-specific
shell integration today, so the “show on tray hover” preference is not expected
to activate widgets on macOS. Treat this as a documented follow-up rather than
a release-blocking claim until its intended cross-platform behavior is decided.
