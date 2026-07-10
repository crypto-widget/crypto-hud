#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'EOF'
Usage: scripts/package-macos.sh [options]

Options:
  --version VERSION          Package version (defaults to workspace version)
  --arch ARCH                current, arm64, x64, or universal (default: current)
  --skip-build               Reuse existing target artifacts
  --sign-identity IDENTITY   Developer ID Application identity
  --notarize                 Submit the DMG to Apple's notary service
  --notary-profile PROFILE   notarytool keychain profile
  -h, --help                 Show this help

Environment equivalents:
  CRYPTO_HUD_MACOS_SIGN_IDENTITY
  CRYPTO_HUD_MACOS_NOTARIZE=1
  CRYPTO_HUD_MACOS_NOTARY_PROFILE
  CRYPTO_HUD_MACOS_BUNDLE_ID (default: com.crypto-hud)
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
SKIP_BUILD=0
SIGN_IDENTITY="${CRYPTO_HUD_MACOS_SIGN_IDENTITY:-}"
NOTARIZE="${CRYPTO_HUD_MACOS_NOTARIZE:-0}"
NOTARY_PROFILE="${CRYPTO_HUD_MACOS_NOTARY_PROFILE:-}"
BUNDLE_ID="${CRYPTO_HUD_MACOS_BUNDLE_ID:-com.crypto-hud}"
MIN_MACOS_VERSION="12.0"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --version)
            VERSION="${2:?--version requires a value}"
            shift 2
            ;;
        --arch)
            ARCH="${2:?--arch requires a value}"
            shift 2
            ;;
        --skip-build)
            SKIP_BUILD=1
            shift
            ;;
        --sign-identity)
            SIGN_IDENTITY="${2:?--sign-identity requires a value}"
            shift 2
            ;;
        --notarize)
            NOTARIZE=1
            shift
            ;;
        --notary-profile)
            NOTARY_PROFILE="${2:?--notary-profile requires a value}"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            usage >&2
            exit 2
            ;;
    esac
done

if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "package-macos.sh must run on macOS" >&2
    exit 1
fi
if ! command -v mise >/dev/null 2>&1; then
    echo "mise is required; install it, then run 'mise trust && mise install'" >&2
    exit 1
fi
export MACOSX_DEPLOYMENT_TARGET="$MIN_MACOS_VERSION"

if [[ -z "$VERSION" || ! "$VERSION" =~ ^[0-9A-Za-z._+-]+$ ]]; then
    echo "Invalid package version: $VERSION" >&2
    exit 2
fi
if [[ ! "$BUNDLE_ID" =~ ^[A-Za-z0-9.-]+$ ]]; then
    echo "Invalid bundle identifier: $BUNDLE_ID" >&2
    exit 2
fi
export CRYPTO_HUD_MACOS_BUNDLE_ID="$BUNDLE_ID"

case "$ARCH" in
    current)
        case "$(uname -m)" in
            arm64) ARCH="arm64" ;;
            x86_64) ARCH="x64" ;;
            *) echo "Unsupported Mac architecture: $(uname -m)" >&2; exit 1 ;;
        esac
        ;;
    arm64|x64|universal) ;;
    *) echo "--arch must be current, arm64, x64, or universal" >&2; exit 2 ;;
esac

if [[ "$NOTARIZE" =~ ^(1|true|yes)$ ]]; then
    NOTARIZE=1
else
    NOTARIZE=0
fi
if [[ "$NOTARIZE" -eq 1 && -z "$SIGN_IDENTITY" ]]; then
    echo "Notarization requires a Developer ID Application signing identity" >&2
    exit 2
fi
if [[ "$NOTARIZE" -eq 1 && -z "$NOTARY_PROFILE" ]]; then
    echo "Notarization requires --notary-profile or CRYPTO_HUD_MACOS_NOTARY_PROFILE" >&2
    exit 2
fi

require_command() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "Required macOS packaging command not found: $1" >&2
        exit 1
    fi
}

for command in codesign ditto git hdiutil iconutil lipo otool plutil shasum sips xcode-select; do
    require_command "$command"
done
if ! xcode-select -p >/dev/null 2>&1; then
    echo "Xcode Command Line Tools are not configured; run 'xcode-select --install'" >&2
    exit 1
fi
if [[ "$NOTARIZE" -eq 1 ]]; then
    require_command xcrun
    if ! xcrun --find notarytool >/dev/null 2>&1; then
        echo "notarytool is unavailable in the selected Xcode installation" >&2
        exit 1
    fi
fi
if [[ -n "$SIGN_IDENTITY" ]]; then
    require_command security
    if ! security find-identity -v -p codesigning | grep -F -- "$SIGN_IDENTITY" >/dev/null; then
        echo "Code-signing identity was not found in the keychain: $SIGN_IDENTITY" >&2
        exit 1
    fi
fi

mise exec -- cargo metadata --locked --no-deps --format-version 1 >/dev/null
if [[ "$SKIP_BUILD" -eq 0 ]]; then
    INSTALLED_TARGETS="$(mise exec -- rustup target list --installed)"
    case "$ARCH" in
        arm64) REQUIRED_TARGETS=(aarch64-apple-darwin) ;;
        x64) REQUIRED_TARGETS=(x86_64-apple-darwin) ;;
        universal) REQUIRED_TARGETS=(aarch64-apple-darwin x86_64-apple-darwin) ;;
    esac
    for target in "${REQUIRED_TARGETS[@]}"; do
        if ! grep -Fx -- "$target" <<<"$INSTALLED_TARGETS" >/dev/null; then
            echo "Rust target is not installed in the mise toolchain: $target" >&2
            echo "Run: mise install" >&2
            exit 1
        fi
    done
fi

DISPLAY_VERSION="${VERSION#v}"
SHORT_VERSION="${DISPLAY_VERSION%%-*}"
if [[ ! "$SHORT_VERSION" =~ ^[0-9]+([.][0-9]+){0,2}$ ]]; then
    SHORT_VERSION="0.0.0"
fi
BUNDLE_VERSION="$SHORT_VERSION"

DIST_DIR="$REPO_ROOT/dist"
BUILD_ROOT="$REPO_ROOT/target/macos-package/$ARCH"
APP_NAME="Crypto HUD.app"
APP_PATH="$DIST_DIR/$APP_NAME"
PACKAGE_BASENAME="crypto-hud-$VERSION-macos-$ARCH"
DMG_PATH="$DIST_DIR/$PACKAGE_BASENAME.dmg"
CHECKSUM_PATH="$DMG_PATH.sha256"
MANIFEST_PATH="$DIST_DIR/$PACKAGE_BASENAME.manifest.json"
DMG_ROOT="$BUILD_ROOT/dmg-root"
ICONSET_PATH="$BUILD_ROOT/AppIcon.iconset"
ICNS_PATH="$BUILD_ROOT/AppIcon.icns"

rm -rf "$BUILD_ROOT" "$APP_PATH"
rm -f "$DMG_PATH" "$CHECKSUM_PATH" "$MANIFEST_PATH"
mkdir -p "$BUILD_ROOT" "$DIST_DIR"

build_target() {
    local target="$1"
    if [[ "$SKIP_BUILD" -eq 0 ]]; then
        mise exec -- cargo build --locked --release -p crypto-hud --target "$target"
    fi
    local binary="$REPO_ROOT/target/$target/release/crypto-hud"
    if [[ ! -x "$binary" ]]; then
        echo "Release binary not found: $binary" >&2
        echo "Install the target in the mise-managed Rust toolchain and build it first." >&2
        exit 1
    fi
    printf '%s\n' "$binary"
}

case "$ARCH" in
    arm64)
        APP_BINARY_SOURCE="$(build_target aarch64-apple-darwin)"
        ;;
    x64)
        APP_BINARY_SOURCE="$(build_target x86_64-apple-darwin)"
        ;;
    universal)
        ARM_BINARY="$(build_target aarch64-apple-darwin)"
        X64_BINARY="$(build_target x86_64-apple-darwin)"
        APP_BINARY_SOURCE="$BUILD_ROOT/crypto-hud-universal"
        lipo -create "$ARM_BINARY" "$X64_BINARY" -output "$APP_BINARY_SOURCE"
        chmod 755 "$APP_BINARY_SOURCE"
        ;;
esac

BINARY_ARCHS="$(lipo -archs "$APP_BINARY_SOURCE")"
case "$ARCH" in
    arm64)
        [[ "$BINARY_ARCHS" == "arm64" ]] || {
            echo "Expected an arm64 binary, found: $BINARY_ARCHS" >&2
            exit 1
        }
        ;;
    x64)
        [[ "$BINARY_ARCHS" == "x86_64" ]] || {
            echo "Expected an x86_64 binary, found: $BINARY_ARCHS" >&2
            exit 1
        }
        ;;
    universal)
        if [[ " $BINARY_ARCHS " != *" arm64 "* || " $BINARY_ARCHS " != *" x86_64 "* ]]; then
            echo "Universal binary is missing an architecture: $BINARY_ARCHS" >&2
            exit 1
        fi
        ;;
esac

verify_macos_deployment_target "$APP_BINARY_SOURCE"
verify_macos_system_library_dependencies "$APP_BINARY_SOURCE"

mkdir -p "$APP_PATH/Contents/MacOS" "$APP_PATH/Contents/Resources"
install -m 755 "$APP_BINARY_SOURCE" "$APP_PATH/Contents/MacOS/crypto-hud"

mkdir -p "$ICONSET_PATH"
create_icon() {
    local pixels="$1"
    local name="$2"
    sips -z "$pixels" "$pixels" "$REPO_ROOT/crates/crypto-hud/ui/icon.png" \
        --out "$ICONSET_PATH/$name" >/dev/null
}
create_icon 16 icon_16x16.png
create_icon 32 icon_16x16@2x.png
create_icon 32 icon_32x32.png
create_icon 64 icon_32x32@2x.png
create_icon 128 icon_128x128.png
create_icon 256 icon_128x128@2x.png
create_icon 256 icon_256x256.png
create_icon 512 icon_256x256@2x.png
create_icon 512 icon_512x512.png
create_icon 1024 icon_512x512@2x.png
iconutil -c icns "$ICONSET_PATH" -o "$ICNS_PATH"
install -m 644 "$ICNS_PATH" "$APP_PATH/Contents/Resources/AppIcon.icns"
install -m 644 README.md README.zh-CN.md LICENSE "$APP_PATH/Contents/Resources/"

sed \
    -e "s|@BUNDLE_ID@|$BUNDLE_ID|g" \
    -e "s|@SHORT_VERSION@|$SHORT_VERSION|g" \
    -e "s|@BUNDLE_VERSION@|$BUNDLE_VERSION|g" \
    -e "s|@MIN_MACOS_VERSION@|$MIN_MACOS_VERSION|g" \
    packaging/macos/Info.plist.in > "$APP_PATH/Contents/Info.plist"
plutil -lint "$APP_PATH/Contents/Info.plist" >/dev/null

cat > "$APP_PATH/Contents/Resources/release-manifest.json" <<EOF
{
  "manifestVersion": 1,
  "name": "crypto-hud",
  "version": "$VERSION",
  "target": "macos-$ARCH",
  "commit": "$(git rev-parse HEAD)",
  "bundleIdentifier": "$BUNDLE_ID",
  "minimumSystemVersion": "$MIN_MACOS_VERSION",
  "updateChannel": "manual-github-release",
  "updateRepository": "crypto-widget/crypto-hud"
}
EOF

if [[ -n "$SIGN_IDENTITY" ]]; then
    codesign --force --options runtime --timestamp --sign "$SIGN_IDENTITY" "$APP_PATH"
    SIGNING_KIND="developer-id"
    SIGNED=true
else
    codesign --force --sign - "$APP_PATH"
    SIGNING_KIND="ad-hoc"
    SIGNED=true
fi
codesign --verify --deep --strict --verbose=2 "$APP_PATH"

mkdir -p "$DMG_ROOT"
ditto "$APP_PATH" "$DMG_ROOT/$APP_NAME"
ln -s /Applications "$DMG_ROOT/Applications"
hdiutil create \
    -volname "Crypto HUD" \
    -srcfolder "$DMG_ROOT" \
    -ov \
    -format UDZO \
    "$DMG_PATH" >/dev/null

if [[ -n "$SIGN_IDENTITY" ]]; then
    codesign --force --timestamp --sign "$SIGN_IDENTITY" "$DMG_PATH"
    codesign --verify --verbose=2 "$DMG_PATH"
fi

NOTARIZED=false
if [[ "$NOTARIZE" -eq 1 ]]; then
    xcrun notarytool submit "$DMG_PATH" --keychain-profile "$NOTARY_PROFILE" --wait
    xcrun stapler staple "$DMG_PATH"
    xcrun stapler validate "$DMG_PATH"
    NOTARIZED=true
fi

DMG_HASH="$(shasum -a 256 "$DMG_PATH" | awk '{print $1}')"
printf '%s  %s\n' "$DMG_HASH" "$(basename "$DMG_PATH")" > "$CHECKSUM_PATH"

JSON_SIGN_IDENTITY="$(json_escape "$SIGN_IDENTITY")"

cat > "$MANIFEST_PATH" <<EOF
{
  "manifestVersion": 1,
  "name": "crypto-hud",
  "version": "$VERSION",
  "target": "macos-$ARCH",
  "commit": "$(git rev-parse HEAD)",
  "builtAt": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "bundle": "$APP_NAME",
  "bundleIdentifier": "$BUNDLE_ID",
  "diskImage": "$(basename "$DMG_PATH")",
  "diskImageSha256": "$DMG_HASH",
  "codeSigning": {
    "signed": $SIGNED,
    "kind": "$SIGNING_KIND",
    "identity": "$JSON_SIGN_IDENTITY"
  },
  "notarized": $NOTARIZED
}
EOF

echo "Created $APP_PATH"
echo "Created $DMG_PATH"
echo "Created $CHECKSUM_PATH"
echo "Created $MANIFEST_PATH"
echo "Binary architectures: $BINARY_ARCHS"
