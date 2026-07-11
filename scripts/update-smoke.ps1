param(
    [string]$Version = "update-smoke",
    [switch]$SkipBuild,
    [switch]$KeepPackage
)

$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$DistDir = Join-Path $RepoRoot "dist"
$PackageRoot = Join-Path $DistDir "crypto-hud-$Version-windows-x64"
$ZipPath = "$PackageRoot.zip"
$ChecksumPath = "$ZipPath.sha256"
$InstallDir = Join-Path $RepoRoot "target\tmp\update-smoke-install"
$ExtractRoot = Join-Path ([System.IO.Path]::GetTempPath()) "crypto-hud-update-smoke-$PID"
$RejectedExtractRoot = "$ExtractRoot-rejected"

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

function Assert-UnderRepo {
    param([string]$Path)

    Assert-UnderDirectory -Path $Path -Directory $RepoRoot -Description "repository"
}

function Assert-UnderTemp {
    param([string]$Path)

    Assert-UnderDirectory -Path $Path -Directory ([System.IO.Path]::GetTempPath()) -Description "temporary update directory"
}

function Assert-Hash {
    param(
        [string]$Path,
        [string]$ExpectedHash
    )

    if (-not (Test-Path -LiteralPath $Path)) {
        throw "Missing file: $Path"
    }
    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
    if ($actual -ne $ExpectedHash.ToLowerInvariant()) {
        throw "Hash mismatch for $Path"
    }
}

Assert-UnderRepo -Path $InstallDir
Assert-UnderTemp -Path $ExtractRoot
Assert-UnderTemp -Path $RejectedExtractRoot
if (Test-Path -LiteralPath $InstallDir) {
    Remove-Item -LiteralPath $InstallDir -Recurse -Force
}

Push-Location $RepoRoot
try {
    $packageArgs = @(
        "-ExecutionPolicy", "Bypass",
        "-File", ".\scripts\package-windows.ps1",
        "-Version", $Version,
        "-AllowDirty",
        "-AllowDevelopmentVersion",
        "-AllowUnsignedPackage"
    )
    if ($SkipBuild) {
        $packageArgs += "-SkipBuild"
    }
    powershell @packageArgs
    if ($LASTEXITCODE -ne 0) {
        throw "Package script failed with code $LASTEXITCODE"
    }

    powershell -ExecutionPolicy Bypass -File ".\scripts\install-update-package.ps1" `
        -PackageZip $ZipPath `
        -ChecksumPath $ChecksumPath `
        -InstallDir $InstallDir `
        -ExtractRoot $RejectedExtractRoot `
        -SkipShellIntegration
    if ($LASTEXITCODE -eq 0) {
        throw "Update helper accepted an unsigned development package without an explicit override"
    }
    if (Test-Path -LiteralPath $RejectedExtractRoot) {
        Assert-UnderTemp -Path $RejectedExtractRoot
        Remove-Item -LiteralPath $RejectedExtractRoot -Recurse -Force
    }

    powershell -ExecutionPolicy Bypass -File ".\scripts\install-update-package.ps1" `
        -PackageZip $ZipPath `
        -ChecksumPath $ChecksumPath `
        -InstallDir $InstallDir `
        -ExtractRoot $ExtractRoot `
        -SkipShellIntegration `
        -AllowUnsignedPackage
    if ($LASTEXITCODE -ne 0) {
        throw "Update install smoke failed with code $LASTEXITCODE"
    }

    $manifestPath = Join-Path $InstallDir "release-manifest.json"
    if (-not (Test-Path -LiteralPath $manifestPath)) {
        throw "Installed update manifest was not copied"
    }
    $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
    if ($manifest.version -ne $Version) {
        throw "Unexpected installed update version: $($manifest.version)"
    }

    $installedExe = Join-Path $InstallDir "crypto-hud.exe"
    Assert-Hash -Path $installedExe -ExpectedHash ([string]$manifest.executableSha256)
    if (-not (Test-Path -LiteralPath (Join-Path $InstallDir "install-update-package.ps1"))) {
        throw "Installed update handoff script was not copied"
    }

    $uninstallScript = Join-Path $InstallDir "uninstall.ps1"
    powershell -ExecutionPolicy Bypass -File $uninstallScript -InstallDir $InstallDir -SkipShellIntegration
    if ($LASTEXITCODE -ne 0) {
        throw "Update uninstall smoke failed with code $LASTEXITCODE"
    }
    if (Test-Path -LiteralPath $installedExe) {
        throw "Update uninstall smoke left the executable behind"
    }

    Write-Host "Update smoke passed"
} finally {
    Pop-Location
    if (Test-Path -LiteralPath $InstallDir) {
        Assert-UnderRepo -Path $InstallDir
        Remove-Item -LiteralPath $InstallDir -Recurse -Force
    }
    if (Test-Path -LiteralPath $ExtractRoot) {
        Assert-UnderTemp -Path $ExtractRoot
        Remove-Item -LiteralPath $ExtractRoot -Recurse -Force
    }
    if (Test-Path -LiteralPath $RejectedExtractRoot) {
        Assert-UnderTemp -Path $RejectedExtractRoot
        Remove-Item -LiteralPath $RejectedExtractRoot -Recurse -Force
    }
    if (-not $KeepPackage) {
        foreach ($path in @($PackageRoot, $ZipPath, $ChecksumPath)) {
            if (Test-Path -LiteralPath $path) {
                Assert-UnderRepo -Path $path
                Remove-Item -LiteralPath $path -Recurse -Force
            }
        }
    }
}
