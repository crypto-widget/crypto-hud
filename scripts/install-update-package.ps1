param(
    [Parameter(Mandatory = $true)]
    [string]$PackageZip,
    [string]$ChecksumPath = "",
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA "CryptoHud"),
    [string]$ExtractRoot = "",
    [switch]$SkipShellIntegration,
    [switch]$StartAfterInstall,
    [switch]$KeepExtracted
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($ChecksumPath)) {
    $ChecksumPath = "$PackageZip.sha256"
}
if ([string]::IsNullOrWhiteSpace($ExtractRoot)) {
    $ExtractRoot = Join-Path ([System.IO.Path]::GetTempPath()) "crypto-hud-update-$PID"
}

$PackageZip = [System.IO.Path]::GetFullPath($PackageZip)
$ChecksumPath = [System.IO.Path]::GetFullPath($ChecksumPath)
$InstallDir = [System.IO.Path]::GetFullPath($InstallDir)
$ExtractRoot = [System.IO.Path]::GetFullPath($ExtractRoot)
$TempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())

function Assert-UnderDirectory {
    param(
        [string]$Path,
        [string]$Directory,
        [string]$Description
    )

    $fullPath = [System.IO.Path]::GetFullPath($Path)
    $fullDirectory = [System.IO.Path]::GetFullPath($Directory)
    if (-not $fullDirectory.EndsWith([System.IO.Path]::DirectorySeparatorChar)) {
        $fullDirectory = "$fullDirectory$([System.IO.Path]::DirectorySeparatorChar)"
    }
    if (-not $fullPath.StartsWith($fullDirectory, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to operate outside ${Description}: $fullPath"
    }
}

function Assert-Hash {
    param(
        [string]$Path,
        [string]$ExpectedHash
    )

    if (-not (Test-Path -LiteralPath $Path)) {
        throw "Missing file: $Path"
    }
    if ($ExpectedHash -notmatch "^[a-fA-F0-9]{64}$") {
        throw "Invalid SHA-256 checksum: $ExpectedHash"
    }

    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
    if ($actual -ne $ExpectedHash.ToLowerInvariant()) {
        throw "Hash mismatch for $Path"
    }
}

if (-not (Test-Path -LiteralPath $PackageZip)) {
    throw "Package zip not found: $PackageZip"
}
if (-not (Test-Path -LiteralPath $ChecksumPath)) {
    throw "Checksum file not found: $ChecksumPath"
}

$checksumLine = Get-Content -LiteralPath $ChecksumPath -Raw
$expectedZipHash = ($checksumLine -split "\s+")[0]
Assert-Hash -Path $PackageZip -ExpectedHash $expectedZipHash

Assert-UnderDirectory -Path $ExtractRoot -Directory $TempRoot -Description "temporary update directory"
if (Test-Path -LiteralPath $ExtractRoot) {
    Remove-Item -LiteralPath $ExtractRoot -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $ExtractRoot | Out-Null

try {
    Expand-Archive -LiteralPath $PackageZip -DestinationPath $ExtractRoot -Force

    $manifestPath = Join-Path $ExtractRoot "release-manifest.json"
    $installScript = Join-Path $ExtractRoot "install.ps1"
    if (-not (Test-Path -LiteralPath $manifestPath)) {
        throw "Extracted update package is missing release-manifest.json"
    }
    if (-not (Test-Path -LiteralPath $installScript)) {
        throw "Extracted update package is missing install.ps1"
    }

    $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
    if ([int]$manifest.manifestVersion -ne 1) {
        throw "Unsupported release manifest version: $($manifest.manifestVersion)"
    }
    if ($manifest.name -ne "crypto-hud") {
        throw "Unexpected package name: $($manifest.name)"
    }
    if ($manifest.target -ne "windows-x64") {
        throw "Unexpected package target: $($manifest.target)"
    }

    $installArgs = @(
        "-ExecutionPolicy", "Bypass",
        "-File", $installScript,
        "-InstallDir", $InstallDir
    )
    if ($SkipShellIntegration) {
        $installArgs += "-SkipShellIntegration"
    }
    if ($StartAfterInstall) {
        $installArgs += "-StartAfterInstall"
    }

    powershell @installArgs
    if ($LASTEXITCODE -ne 0) {
        throw "Update package install failed with code $LASTEXITCODE"
    }

    Write-Host "Installed update package to $InstallDir"
} finally {
    if (-not $KeepExtracted -and (Test-Path -LiteralPath $ExtractRoot)) {
        Assert-UnderDirectory -Path $ExtractRoot -Directory $TempRoot -Description "temporary update directory"
        Remove-Item -LiteralPath $ExtractRoot -Recurse -Force
    }
}
