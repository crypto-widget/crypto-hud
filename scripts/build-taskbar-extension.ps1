param(
    [switch]$Release
)

$ErrorActionPreference = "Stop"
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$hadRustFlags = Test-Path "Env:\RUSTFLAGS"
$previousRustFlags = if ($hadRustFlags) { $env:RUSTFLAGS } else { $null }
$hadEncodedRustFlags = Test-Path "Env:\CARGO_ENCODED_RUSTFLAGS"
$previousEncodedRustFlags = if ($hadEncodedRustFlags) {
    $env:CARGO_ENCODED_RUSTFLAGS
} else {
    $null
}
$staticCrtFlag = "-C target-feature=+crt-static"
$unitSeparator = [char]0x1F

try {
    if ($hadEncodedRustFlags) {
        # Cargo gives CARGO_ENCODED_RUSTFLAGS precedence over RUSTFLAGS. Append
        # the static-CRT option last so even a caller-provided -crt-static is
        # overridden for this Explorer-loaded DLL, while preserving every
        # other encoded rustc argument.
        $encodedParts = @(
            @($previousEncodedRustFlags -split [regex]::Escape([string]$unitSeparator)) |
                Where-Object { -not [string]::IsNullOrEmpty($_) }
        )
        $encodedParts += @("-C", "target-feature=+crt-static")
        $env:CARGO_ENCODED_RUSTFLAGS = [string]::Join($unitSeparator, $encodedParts)
    } elseif ([string]::IsNullOrWhiteSpace($previousRustFlags)) {
        $env:RUSTFLAGS = $staticCrtFlag
    } else {
        # Always append the option: a previous target-feature argument can
        # explicitly disable crt-static, and rustc resolves the last value.
        $env:RUSTFLAGS = "$previousRustFlags $staticCrtFlag"
    }

    Push-Location $RepoRoot
    try {
        $cargoArgs = @("build", "--locked", "-p", "crypto-hud-taskbar")
        if ($Release) {
            $cargoArgs = @("build", "--locked", "--release", "-p", "crypto-hud-taskbar")
        }
        & cargo @cargoArgs
        if ($LASTEXITCODE -ne 0) {
            throw "Taskbar extension build failed with code $LASTEXITCODE"
        }
    } finally {
        Pop-Location
    }
} finally {
    if ($hadRustFlags) {
        $env:RUSTFLAGS = $previousRustFlags
    } else {
        Remove-Item Env:\RUSTFLAGS -ErrorAction SilentlyContinue
    }
    if ($hadEncodedRustFlags) {
        $env:CARGO_ENCODED_RUSTFLAGS = $previousEncodedRustFlags
    } else {
        Remove-Item Env:\CARGO_ENCODED_RUSTFLAGS -ErrorAction SilentlyContinue
    }
}
