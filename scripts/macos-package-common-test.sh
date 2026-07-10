#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/macos-package-common.sh
source "$SCRIPT_DIR/macos-package-common.sh"

TEMP_DIR="$(mktemp -d)"
cleanup() {
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

cat > "$TEMP_DIR/otool" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

case "$1:$2" in
    -l:thin-good)
        cat <<'OUTPUT'
Load command 10
      cmd LC_BUILD_VERSION
  cmdsize 32
 platform MACOS
    minos 12.0
      sdk 15.2
OUTPUT
        ;;
    -l:universal-good)
        cat <<'OUTPUT'
universal-good (architecture x86_64):
Load command 10
      cmd LC_VERSION_MIN_MACOSX
  cmdsize 16
  version 12.0.0
      sdk 15.2
universal-good (architecture arm64):
Load command 10
      cmd LC_BUILD_VERSION
  cmdsize 32
 platform MACOS
    minos 12.0
      sdk 15.2
OUTPUT
        ;;
    -l:wrong-version)
        cat <<'OUTPUT'
Load command 10
      cmd LC_BUILD_VERSION
  cmdsize 32
 platform MACOS
    minos 13.0
      sdk 15.2
OUTPUT
        ;;
    -l:universal-mixed)
        cat <<'OUTPUT'
universal-mixed (architecture x86_64):
Load command 10
      cmd LC_BUILD_VERSION
  cmdsize 32
 platform MACOS
    minos 12.0
      sdk 15.2
universal-mixed (architecture arm64):
Load command 10
      cmd LC_BUILD_VERSION
  cmdsize 32
 platform MACOS
    minos 13.0
      sdk 15.2
OUTPUT
        ;;
    -l:no-version)
        printf '%s\n' 'Load command 0' '      cmd LC_SEGMENT_64'
        ;;
    -L:thin-good)
        cat <<'OUTPUT'
thin-good:
	/usr/lib/libSystem.B.dylib (compatibility version 1.0.0, current version 1351.0.0)
OUTPUT
        ;;
    -L:universal-good)
        cat <<'OUTPUT'
universal-good (architecture x86_64):
	/usr/lib/libSystem.B.dylib (compatibility version 1.0.0, current version 1351.0.0)
universal-good (architecture arm64):
	/System/Library/Frameworks/AppKit.framework/Versions/C/AppKit (compatibility version 45.0.0, current version 2487.20.6)
OUTPUT
        ;;
    -L:bad-dependency)
        cat <<'OUTPUT'
bad-dependency:
	@rpath/libUnexpected.dylib (compatibility version 1.0.0, current version 1.0.0)
OUTPUT
        ;;
    -L:no-dependency)
        printf '%s\n' 'no-dependency:'
        ;;
    *)
        echo "Unexpected fake otool invocation: $*" >&2
        exit 2
        ;;
esac
EOF
chmod +x "$TEMP_DIR/otool"
export PATH="$TEMP_DIR:$PATH"
export MIN_MACOS_VERSION="12.0"

[[ "$(normalize_macos_version 12)" == "12.0.0" ]]
[[ "$(normalize_macos_version 12.0)" == "12.0.0" ]]
[[ "$(normalize_macos_version 12.0.0)" == "12.0.0" ]]
if normalize_macos_version 12.0.0.1 >/dev/null 2>&1; then
    echo "A four-component macOS version unexpectedly normalized" >&2
    exit 1
fi

verify_macos_deployment_target thin-good
verify_macos_deployment_target universal-good
if verify_macos_deployment_target wrong-version >/dev/null 2>&1; then
    echo "A wrong deployment target unexpectedly passed" >&2
    exit 1
fi
if verify_macos_deployment_target universal-mixed >/dev/null 2>&1; then
    echo "A universal binary with mixed deployment targets unexpectedly passed" >&2
    exit 1
fi
if verify_macos_deployment_target no-version >/dev/null 2>&1; then
    echo "A missing deployment target unexpectedly passed" >&2
    exit 1
fi

verify_macos_system_library_dependencies thin-good
verify_macos_system_library_dependencies universal-good
if verify_macos_system_library_dependencies bad-dependency >/dev/null 2>&1; then
    echo "A non-system dependency unexpectedly passed" >&2
    exit 1
fi
if verify_macos_system_library_dependencies no-dependency >/dev/null 2>&1; then
    echo "A missing dependency list unexpectedly passed" >&2
    exit 1
fi

[[ "$(json_escape 'A"B')" == 'A\"B' ]]
[[ "$(json_escape 'A\B')" == 'A\\B' ]]
[[ "$(json_escape $'A\nB')" == 'A\nB' ]]
TAB="$(printf '\t')"
CARRIAGE_RETURN="$(printf '\r')"
[[ "$(json_escape "A${TAB}B")" == 'A\tB' ]]
[[ "$(json_escape "A${CARRIAGE_RETURN}B")" == 'A\rB' ]]

echo "macOS package helper tests passed"
