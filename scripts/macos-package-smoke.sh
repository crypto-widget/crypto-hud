#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'EOF'
Usage: scripts/macos-package-smoke.sh [options]

Options are forwarded to package-macos.sh. Use --skip-build to reuse an
existing release binary. The smoke test validates and mounts the DMG, then
launches the bundled executable and waits for its GUI-ready marker.
EOF
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
# shellcheck source=scripts/macos-package-common.sh
source "$SCRIPT_DIR/macos-package-common.sh"
cd "$REPO_ROOT"

VERSION="$(awk '
    /^\[workspace\.package\]$/ { workspace = 1; next }
    /^\[/ { workspace = 0 }
    workspace && /^version[[:space:]]*=/ {
        value = $0
        sub(/^[^=]*=[[:space:]]*"/, "", value)
        sub(/"[[:space:]]*$/, "", value)
        print value
        exit
    }
' Cargo.toml)"
ARCH="current"
MIN_MACOS_VERSION="12.0"
FORWARD_ARGS=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        --version)
            VERSION="${2:?--version requires a value}"
            FORWARD_ARGS+=("$1" "$2")
            shift 2
            ;;
        --arch)
            ARCH="${2:?--arch requires a value}"
            FORWARD_ARGS+=("$1" "$2")
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            FORWARD_ARGS+=("$1")
            shift
            ;;
    esac
done

if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "macos-package-smoke.sh must run on macOS" >&2
    exit 1
fi

if [[ "$ARCH" == "current" ]]; then
    case "$(uname -m)" in
        arm64) ARCH="arm64" ;;
        x86_64) ARCH="x64" ;;
        *) echo "Unsupported Mac architecture: $(uname -m)" >&2; exit 1 ;;
    esac
fi

DISPLAY_VERSION="${VERSION#v}"
SHORT_VERSION="${DISPLAY_VERSION%%-*}"
if [[ ! "$SHORT_VERSION" =~ ^[0-9]+([.][0-9]+){0,2}$ ]]; then
    SHORT_VERSION="0.0.0"
fi

bash "$SCRIPT_DIR/package-macos.sh" "${FORWARD_ARGS[@]}"

APP_PATH="$REPO_ROOT/dist/Crypto HUD.app"
DMG_PATH="$REPO_ROOT/dist/crypto-hud-$VERSION-macos-$ARCH.dmg"
CHECKSUM_PATH="$DMG_PATH.sha256"
MANIFEST_PATH="$REPO_ROOT/dist/crypto-hud-$VERSION-macos-$ARCH.manifest.json"
BINARY_PATH="$APP_PATH/Contents/MacOS/crypto-hud"
STATE_ROOT="$REPO_ROOT/target/macos-package-smoke"
MOUNT_POINT="$STATE_ROOT/mount"
READY_FILE="$STATE_ROOT/ready.json"
APP_PID=""
MOUNTED=0

cleanup() {
    if [[ -n "$APP_PID" ]] && kill -0 "$APP_PID" >/dev/null 2>&1; then
        kill "$APP_PID" >/dev/null 2>&1 || true
        wait "$APP_PID" >/dev/null 2>&1 || true
    fi
    if [[ "$MOUNTED" -eq 1 ]]; then
        hdiutil detach "$MOUNT_POINT" -quiet || true
    fi
    rm -rf "$STATE_ROOT"
}
trap cleanup EXIT

test -x "$BINARY_PATH"
test -f "$APP_PATH/Contents/Info.plist"
test -f "$APP_PATH/Contents/Resources/AppIcon.icns"
test -f "$APP_PATH/Contents/Resources/release-manifest.json"
test -f "$DMG_PATH"
test -f "$CHECKSUM_PATH"
test -f "$MANIFEST_PATH"

plutil -lint "$APP_PATH/Contents/Info.plist" >/dev/null
plutil -lint "$APP_PATH/Contents/Resources/release-manifest.json" >/dev/null
plutil -lint "$MANIFEST_PATH" >/dev/null
[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$APP_PATH/Contents/Info.plist")" == "${CRYPTO_HUD_MACOS_BUNDLE_ID:-com.crypto-hud}" ]]
[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$APP_PATH/Contents/Info.plist")" == "crypto-hud" ]]
[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$APP_PATH/Contents/Info.plist")" == "$SHORT_VERSION" ]]
[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleVersion' "$APP_PATH/Contents/Info.plist")" == "$SHORT_VERSION" ]]
[[ "$(/usr/libexec/PlistBuddy -c 'Print :LSMinimumSystemVersion' "$APP_PATH/Contents/Info.plist")" == "$MIN_MACOS_VERSION" ]]
[[ "$(plutil -extract target raw -o - -- "$MANIFEST_PATH")" == "macos-$ARCH" ]]
[[ "$(plutil -extract diskImage raw -o - -- "$MANIFEST_PATH")" == "$(basename "$DMG_PATH")" ]]
[[ "$(plutil -extract version raw -o - -- "$MANIFEST_PATH")" == "$VERSION" ]]
[[ "$(plutil -extract bundleIdentifier raw -o - -- "$MANIFEST_PATH")" == "${CRYPTO_HUD_MACOS_BUNDLE_ID:-com.crypto-hud}" ]]
[[ "$(plutil -extract codeSigning.signed raw -o - -- "$MANIFEST_PATH")" == "true" ]]
[[ "$(plutil -extract version raw -o - -- "$APP_PATH/Contents/Resources/release-manifest.json")" == "$VERSION" ]]
[[ "$(plutil -extract target raw -o - -- "$APP_PATH/Contents/Resources/release-manifest.json")" == "macos-$ARCH" ]]
[[ "$(plutil -extract minimumSystemVersion raw -o - -- "$APP_PATH/Contents/Resources/release-manifest.json")" == "$MIN_MACOS_VERSION" ]]

BINARY_ARCHS="$(lipo -archs "$BINARY_PATH")"
case "$ARCH" in
    arm64) [[ "$BINARY_ARCHS" == "arm64" ]] ;;
    x64) [[ "$BINARY_ARCHS" == "x86_64" ]] ;;
    universal)
        [[ " $BINARY_ARCHS " == *" arm64 "* ]]
        [[ " $BINARY_ARCHS " == *" x86_64 "* ]]
        ;;
esac
verify_macos_deployment_target "$BINARY_PATH"
verify_macos_system_library_dependencies "$BINARY_PATH"
codesign --verify --deep --strict --verbose=2 "$APP_PATH"

SIGNING_KIND="$(plutil -extract codeSigning.kind raw -o - -- "$MANIFEST_PATH")"
SIGNATURE_DETAILS="$(codesign --display --verbose=4 "$APP_PATH" 2>&1)"
case "$SIGNING_KIND" in
    ad-hoc)
        grep -F 'Signature=adhoc' <<<"$SIGNATURE_DETAILS" >/dev/null
        ;;
    developer-id)
        SIGNING_IDENTITY="$(plutil -extract codeSigning.identity raw -o - -- "$MANIFEST_PATH")"
        grep -F "Authority=$SIGNING_IDENTITY" <<<"$SIGNATURE_DETAILS" >/dev/null
        grep -F 'runtime' <<<"$SIGNATURE_DETAILS" >/dev/null
        ;;
    *)
        echo "Unexpected signing kind: $SIGNING_KIND" >&2
        exit 1
        ;;
esac
hdiutil verify "$DMG_PATH" >/dev/null
(
    cd "$(dirname "$DMG_PATH")"
    shasum -a 256 -c "$(basename "$CHECKSUM_PATH")"
)
DMG_HASH="$(shasum -a 256 "$DMG_PATH" | awk '{print $1}')"
[[ "$(plutil -extract diskImageSha256 raw -o - -- "$MANIFEST_PATH")" == "$DMG_HASH" ]]

rm -rf "$STATE_ROOT"
mkdir -p "$MOUNT_POINT"
hdiutil attach "$DMG_PATH" -nobrowse -readonly -mountpoint "$MOUNT_POINT" >/dev/null
MOUNTED=1
test -d "$MOUNT_POINT/Crypto HUD.app"
test -L "$MOUNT_POINT/Applications"
MOUNTED_APP_PATH="$MOUNT_POINT/Crypto HUD.app"
MOUNTED_BINARY_PATH="$MOUNTED_APP_PATH/Contents/MacOS/crypto-hud"
test -x "$MOUNTED_BINARY_PATH"
codesign --verify --deep --strict --verbose=2 "$MOUNTED_APP_PATH"

if [[ "$(plutil -extract notarized raw -o - -- "$MANIFEST_PATH")" == "true" ]]; then
    xcrun stapler validate "$DMG_PATH"
    spctl --assess --type execute --verbose=4 "$MOUNTED_APP_PATH"
fi

mkdir -p "$STATE_ROOT/app-state"
CRYPTO_HUD_STATE_DIR="$STATE_ROOT/app-state" \
CRYPTO_HUD_GUI_SMOKE_READY_FILE="$READY_FILE" \
CRYPTO_HUD_INSTANCE_ID="com.crypto-hud.macos-smoke.$$" \
CRYPTO_HUD_DISABLE_UPDATE_CHECK=1 \
SLINT_BACKEND=software \
"$MOUNTED_BINARY_PATH" --widgets 1 --show-settings --gui-smoke-ms 3000 &
APP_PID=$!

for _ in {1..75}; do
    if [[ -f "$READY_FILE" ]]; then
        break
    fi
    if ! kill -0 "$APP_PID" >/dev/null 2>&1; then
        wait "$APP_PID"
        echo "Crypto HUD exited before writing the GUI-ready marker" >&2
        exit 1
    fi
    sleep 0.2
done

if [[ ! -f "$READY_FILE" ]] \
    || [[ "$(plutil -extract ready raw -o - -- "$READY_FILE")" != "true" ]] \
    || [[ "$(plutil -extract settingsWindowRequested raw -o - -- "$READY_FILE")" != "true" ]] \
    || [[ "$(plutil -extract widgetCount raw -o - -- "$READY_FILE")" -lt 1 ]]; then
    echo "GUI-ready marker was not created" >&2
    exit 1
fi

wait "$APP_PID"
APP_PID=""
echo "Binary architectures: $BINARY_ARCHS"
echo "macOS package smoke passed"
