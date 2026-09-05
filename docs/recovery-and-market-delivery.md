# Recovery and market delivery

## Installation and removal

The classic installer accepts a missing or empty installation directory. An
existing nonempty directory must have a valid Crypto HUD release manifest and
integrity metadata. All existing files and directories must belong to that
manifest before the installer replaces the directory. Move personal files out
of an installation directory before upgrading; the installer refuses to discard
unowned content. It checks ownership again before the directory swap and before
deleting its rollback copy. A rollback copy that contains unexpected content is
retained with a warning.

The installed uninstaller defaults to its own directory. This also applies to
the Windows Apps uninstall command, which supplies the script path without an
explicit `-InstallDir`. An explicit target must still match the script's location
and pass the existing manifest and hash checks.

`scripts/package-smoke.ps1` exercises refusal of a non-install directory with a
sentinel file, refusal of an upgrade containing an unowned file, a clean
reinstall, and uninstalling a custom path with spaces without `-InstallDir`.
These checks use isolated state and skip real Shell registration. Authenticode
verification remains a separate signed-release check.

## State preservation

If state cannot be read or parsed, the loader attempts to preserve the original
before using defaults. When that backup also fails, the fallback `LayoutStore`
carries a runtime-only write guard. Cloning the store for settings transactions
or layout changes retains the same protection; it is not serialized and does
not change the state schema.

Every `save_layout_store` call checks the guard. Until a complete, flushed
backup can be created, saving fails without replacing the original state. Once
the file becomes accessible, a save can create the backup and proceed. Later
saves do not replace that recovery copy. A restart can instead load the original
configuration if it has become readable.

Shell-state regression tests cover backup failure on every platform and a
temporary exclusive Windows file lock. They also verify that repeated saves
through cloned stores preserve the original recovery copy.

## Market delivery and fallback

Market requests share at most four workers. Workers take the next pair when
they finish; each completed snapshot is delivered immediately without waiting
for unrelated requests. Configuration changes and cancellation suppress obsolete
results. A cycle's final status reports any ticker or chart failures; individual
successful snapshots do not clear an existing error before the cycle completes.

`MarketSnapshot.updated_at` records when the ticker was parsed, before optional
candle requests and queue delivery. The desktop bridge preserves that timestamp
in `QuoteState`, so delayed delivery cannot reset the age used for stale data.

A fallback route is validated for its current pair set. Adding or replacing a
pair resets that source to its preferred provider and allows normal catalog
validation and fallback selection to run again. Removing pairs, reordering them,
or changing only candle requirements preserves the current route. Other sources
are unaffected.

Market regression tests use fixed inputs and channels to hold one request while
later healthy requests finish. They cover delivery, cancellation, concurrency,
fallback subscription changes, and final cycle status without live exchanges.

## Dependency checks

The desktop manifest constrains Slint's existing Unix dependencies to
`webbrowser >=1.2.2` and `event-listener >=5.4.2`, addressing
RUSTSEC-2026-0257 and RUSTSEC-2026-0221. Keep these constraints and `Cargo.lock`
aligned and run `cargo audit` after dependency changes. Upstream maintenance
warnings for other Slint dependencies remain visible; they are not suppressed.
