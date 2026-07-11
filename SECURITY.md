# Security Policy

## Supported Versions

Crypto HUD is currently in the `0.9.x` alpha line. Security fixes are expected
to target the latest alpha release and the main development branch.

## Reporting a Vulnerability

Please use GitHub private vulnerability reporting when it is available for this
repository.

If private reporting is not available, open a public issue without exploit
details and ask for a security contact. Do not include tokens, private keys,
certificate material, or step-by-step exploit instructions in a public issue.

## Scope

Security-sensitive areas include:

- Update download and package verification.
- Windows install, uninstall, signing, and auto-start behavior.
- Market-data networking, proxy handling, and HTTP response parsing.
- Local plugin manifest validation and renderer loading.
- Persisted settings and layout-state migration.

## Expectations

We aim to acknowledge valid reports promptly, investigate impact, and prepare a
fix before publishing detailed disclosure notes.
