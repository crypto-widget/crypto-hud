#!/usr/bin/env bash

# Shared validation helpers for the macOS packager and smoke test. This file is
# sourced by scripts that already enable `set -euo pipefail`.

normalize_macos_version() {
    local major minor patch extra
    IFS=. read -r major minor patch extra <<<"$1"
    if [[ ! "$major" =~ ^[0-9]+$ ]] \
        || [[ -n "$minor" && ! "$minor" =~ ^[0-9]+$ ]] \
        || [[ -n "$patch" && ! "$patch" =~ ^[0-9]+$ ]] \
        || [[ -n "$extra" ]]; then
        return 1
    fi
    printf '%s.%s.%s\n' "$major" "${minor:-0}" "${patch:-0}"
}

verify_macos_deployment_target() {
    local binary="$1"
    local versions expected version normalized
    : "${MIN_MACOS_VERSION:?MIN_MACOS_VERSION must be set}"

    versions="$(otool -l "$binary" | awk '
        $1 == "cmd" { command = $2 }
        command == "LC_BUILD_VERSION" && $1 == "minos" { print $2 }
        command == "LC_VERSION_MIN_MACOSX" && $1 == "version" { print $2 }
    ')"
    if [[ -z "$versions" ]]; then
        echo "Mach-O deployment target was not found in $binary" >&2
        return 1
    fi
    expected="$(normalize_macos_version "$MIN_MACOS_VERSION")" || return 1
    while IFS= read -r version; do
        normalized="$(normalize_macos_version "$version")" || {
            echo "Invalid Mach-O deployment target in $binary: $version" >&2
            return 1
        }
        if [[ "$normalized" != "$expected" ]]; then
            echo "Expected macOS deployment target $MIN_MACOS_VERSION, found $version in $binary" >&2
            return 1
        fi
    done <<<"$versions"
}

verify_macos_system_library_dependencies() {
    local binary="$1"
    local dependencies dependency
    dependencies="$(otool -L "$binary" | awk '
        /^[[:space:]]+.*\(compatibility version/ { print $1 }
    ')"
    if [[ -z "$dependencies" ]]; then
        echo "Mach-O dynamic library dependencies were not found in $binary" >&2
        return 1
    fi
    while IFS= read -r dependency; do
        case "$dependency" in
            /System/Library/*|/usr/lib/*) ;;
            *)
                echo "Non-system dynamic library dependency in $binary: $dependency" >&2
                return 1
                ;;
        esac
    done <<<"$dependencies"
}

json_escape() {
    local value="$1"
    value="${value//\\/\\\\}"
    value="${value//\"/\\\"}"
    value="${value//$'\r'/\\r}"
    value="${value//$'\n'/\\n}"
    value="${value//$'\t'/\\t}"
    printf '%s' "$value"
}
