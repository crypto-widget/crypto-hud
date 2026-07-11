param(
    [string]$Version = "smoke",
    [switch]$SkipBuild,
    [switch]$KeepPackage
)

$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$DistDir = Join-Path $RepoRoot "dist"
$PackageRoot = Join-Path $DistDir "crypto-hud-$Version-windows-x64"
$ZipPath = "$PackageRoot.zip"
$ChecksumPath = "$ZipPath.sha256"
$InstallDir = Join-Path $RepoRoot "target\tmp\package-smoke-install"

function Assert-UnderRepo {
    param([string]$Path)

    $fullPath = [System.IO.Path]::GetFullPath($Path)
    $repoPath = [System.IO.Path]::GetFullPath($RepoRoot)
    if (-not $fullPath.StartsWith($repoPath, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to operate outside repository: $fullPath"
    }
}

function Assert-Hash {
    param(
        [string]$Path,
        [string]$ExpectedHash
    )

    if (-not (Test-Path $Path)) {
        throw "Missing file: $Path"
    }
    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
    if ($actual -ne $ExpectedHash.ToLowerInvariant()) {
        throw "Hash mismatch for $Path"
    }
}

function Resolve-PackageFile {
    param([string]$RelativePath)

    if ([System.IO.Path]::IsPathRooted($RelativePath) -or $RelativePath.Contains("..")) {
        throw "Unsafe package path in manifest: $RelativePath"
    }
    Join-Path $PackageRoot $RelativePath
}

Assert-UnderRepo -Path $PackageRoot
Assert-UnderRepo -Path $InstallDir

if (Test-Path $InstallDir) {
    Remove-Item -LiteralPath $InstallDir -Recurse -Force
}

Push-Location $RepoRoot
try {
    $packageArgs = @(
        "-ExecutionPolicy", "Bypass",
        "-File", ".\scripts\package-windows.ps1",
        "-Version", $Version,
        "-AllowDirty",
        "-AllowDevelopmentVersion"
    )
    if ($SkipBuild) {
        $packageArgs += "-SkipBuild"
    }
    powershell @packageArgs
    if ($LASTEXITCODE -ne 0) {
        throw "Package script failed with code $LASTEXITCODE"
    }

    if (-not (Test-Path $ZipPath)) {
        throw "Package zip was not created: $ZipPath"
    }
    if (-not (Test-Path $ChecksumPath)) {
        throw "Package checksum was not created: $ChecksumPath"
    }

    $checksumLine = Get-Content -LiteralPath $ChecksumPath -Raw
    $expectedZipHash = ($checksumLine -split "\s+")[0]
    Assert-Hash -Path $ZipPath -ExpectedHash $expectedZipHash

    $manifestPath = Join-Path $PackageRoot "release-manifest.json"
    if (-not (Test-Path $manifestPath)) {
        throw "Package manifest was not created"
    }
    $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
    if ([int]$manifest.manifestVersion -ne 1) {
        throw "Unexpected manifest version: $($manifest.manifestVersion)"
    }
    if ($manifest.executable -ne "crypto-hud.exe") {
        throw "Unexpected manifest executable: $($manifest.executable)"
    }
    if (-not (@($manifest.files).path -contains "install-update-package.ps1")) {
        throw "Package manifest is missing install-update-package.ps1"
    }
    if (-not $manifest.codeSigning) {
        throw "Package manifest is missing codeSigning metadata"
    }
    if ([bool]$manifest.codeSigning.requested -and -not [bool]$manifest.codeSigning.signed) {
        throw "Package manifest says signing was requested but the executable is not signed"
    }
    if (-not [bool]$manifest.codeSigning.requested -and [bool]$manifest.codeSigning.signed) {
        throw "Package manifest says signing was not requested but the executable is signed"
    }
    foreach ($file in @($manifest.files)) {
        Assert-Hash -Path (Resolve-PackageFile -RelativePath ([string]$file.path)) -ExpectedHash ([string]$file.sha256)
    }

    $protectedDir = Join-Path $RepoRoot "target\tmp\package-smoke-protected"
    Assert-UnderRepo -Path $protectedDir
    New-Item -ItemType Directory -Force -Path $protectedDir | Out-Null
    $sentinel = Join-Path $protectedDir "keep.txt"
    Set-Content -LiteralPath $sentinel -Value "keep"
    powershell -ExecutionPolicy Bypass -File (Join-Path $PackageRoot "uninstall.ps1") -InstallDir $protectedDir -SkipShellIntegration
    if ($LASTEXITCODE -eq 0) {
        throw "Uninstall safety check accepted a non-install directory"
    }
    if (-not (Test-Path -LiteralPath $sentinel)) {
        throw "Uninstall safety check removed a protected directory"
    }
    Remove-Item -LiteralPath $protectedDir -Recurse -Force

    powershell -ExecutionPolicy Bypass -File (Join-Path $PackageRoot "install.ps1") -InstallDir $InstallDir -SkipShellIntegration
    if ($LASTEXITCODE -ne 0) {
        throw "Install smoke failed with code $LASTEXITCODE"
    }

    $installedExe = Join-Path $InstallDir "crypto-hud.exe"
    Assert-Hash -Path $installedExe -ExpectedHash ([string]$manifest.executableSha256)
    if (-not (Test-Path (Join-Path $InstallDir "release-manifest.json"))) {
        throw "Installed manifest was not copied"
    }
    if (-not (Test-Path (Join-Path $InstallDir "install-update-package.ps1"))) {
        throw "Installed update handoff script was not copied"
    }

    powershell -ExecutionPolicy Bypass -File (Join-Path $InstallDir "uninstall.ps1") -InstallDir $InstallDir -SkipShellIntegration
    if ($LASTEXITCODE -ne 0) {
        throw "Uninstall smoke failed with code $LASTEXITCODE"
    }
    if (Test-Path $installedExe) {
        throw "Uninstall smoke left the executable behind"
    }

    Write-Host "Package smoke passed"
} finally {
    Pop-Location
    if (Test-Path $InstallDir) {
        Remove-Item -LiteralPath $InstallDir -Recurse -Force
    }
    if (-not $KeepPackage) {
        Remove-Item -LiteralPath $PackageRoot -Recurse -Force -ErrorAction SilentlyContinue
        Remove-Item -LiteralPath $ZipPath -Force -ErrorAction SilentlyContinue
        Remove-Item -LiteralPath $ChecksumPath -Force -ErrorAction SilentlyContinue
    }
}
