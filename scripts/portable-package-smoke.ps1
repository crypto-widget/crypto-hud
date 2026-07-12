param(
    [string]$Version = "v9999.0.1-smoke",
    [switch]$SkipBuild,
    [switch]$KeepPackage
)

$ErrorActionPreference = "Stop"
$PowerShellExe = (Get-Process -Id $PID).Path
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$DistDir = Join-Path $RepoRoot "dist"
$PackageRoot = Join-Path $DistDir "crypto-hud-$Version-windows-x64-portable"
$ZipPath = "$PackageRoot.zip"
$ChecksumPath = "$ZipPath.sha256"
$ExtractRoot = Join-Path ([System.IO.Path]::GetTempPath()) "crypto-hud-portable-smoke-$PID"

function Assert-UnderDirectory {
    param(
        [string]$Path,
        [string]$Directory,
        [string]$Description
    )

    $fullPath = [System.IO.Path]::GetFullPath($Path)
    $fullDirectory = [System.IO.Path]::GetFullPath($Directory).TrimEnd('\', '/')
    $prefix = "$fullDirectory$([System.IO.Path]::DirectorySeparatorChar)"
    if (-not $fullPath.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to operate outside ${Description}: $fullPath"
    }
}

Assert-UnderDirectory -Path $PackageRoot -Directory $DistDir -Description "distribution directory"
Assert-UnderDirectory -Path $ZipPath -Directory $DistDir -Description "distribution directory"
Assert-UnderDirectory -Path $ChecksumPath -Directory $DistDir -Description "distribution directory"
Assert-UnderDirectory -Path $ExtractRoot -Directory ([System.IO.Path]::GetTempPath()) -Description "temporary directory"

Push-Location $RepoRoot
try {
    $packageArgs = @(
        "-ExecutionPolicy", "Bypass",
        "-File", ".\scripts\package-portable-windows.ps1",
        "-Version", $Version,
        "-AllowDirty",
        "-AllowDevelopmentVersion"
    )
    if ($SkipBuild) {
        $packageArgs += "-SkipBuild"
    }
    & $PowerShellExe @packageArgs
    if ($LASTEXITCODE -ne 0) {
        throw "Portable package script failed with code $LASTEXITCODE"
    }

    if (-not (Test-Path -LiteralPath $ZipPath -PathType Leaf) -or
        -not (Test-Path -LiteralPath $ChecksumPath -PathType Leaf)) {
        throw "Portable package did not create the zip and checksum assets"
    }
    $checksumLine = (Get-Content -LiteralPath $ChecksumPath -Raw).Trim()
    $expectedName = Split-Path -Leaf $ZipPath
    if ($checksumLine -notmatch "^([a-f0-9]{64})  $([regex]::Escape($expectedName))$") {
        throw "Portable checksum file has an unexpected format"
    }
    $actualHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $ZipPath).Hash.ToLowerInvariant()
    if ($actualHash -ne $Matches[1]) {
        throw "Portable zip checksum does not match"
    }

    if (Test-Path -LiteralPath $ExtractRoot) {
        Remove-Item -LiteralPath $ExtractRoot -Recurse -Force
    }
    Expand-Archive -LiteralPath $ZipPath -DestinationPath $ExtractRoot
    foreach ($requiredPath in @(
        "crypto-hud.exe",
        "README.md",
        "LICENSE",
        "portable-manifest.json",
        "plugins",
        "resources\previews",
        "resources\icon.ico"
    )) {
        if (-not (Test-Path -LiteralPath (Join-Path $ExtractRoot $requiredPath))) {
            throw "Portable package is missing $requiredPath"
        }
    }
    if (@(Get-ChildItem -LiteralPath $ExtractRoot -Recurse -File -Filter *.ps1).Count -gt 0) {
        throw "Portable package unexpectedly contains a PowerShell script"
    }

    $manifest = Get-Content -LiteralPath (Join-Path $ExtractRoot "portable-manifest.json") -Raw |
        ConvertFrom-Json
    if ([int]$manifest.manifestVersion -ne 1 -or
        [string]$manifest.distribution -ne "portable" -or
        [string]$manifest.version -ne $Version -or
        [bool]$manifest.codeSigning.required -or
        [bool]$manifest.codeSigning.signed) {
        throw "Portable manifest metadata is invalid"
    }
    foreach ($file in @($manifest.files)) {
        $path = [System.IO.Path]::GetFullPath((Join-Path $ExtractRoot ([string]$file.path)))
        Assert-UnderDirectory -Path $path -Directory $ExtractRoot -Description "extracted portable package"
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            throw "Portable manifest names a missing file: $($file.path)"
        }
        $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLowerInvariant()
        if ($hash -ne [string]$file.sha256) {
            throw "Portable manifest hash mismatch: $($file.path)"
        }
    }

    Write-Host "Portable package smoke passed"
} finally {
    Pop-Location
    if (Test-Path -LiteralPath $ExtractRoot) {
        Assert-UnderDirectory -Path $ExtractRoot -Directory ([System.IO.Path]::GetTempPath()) -Description "temporary directory"
        Remove-Item -LiteralPath $ExtractRoot -Recurse -Force
    }
    if (-not $KeepPackage) {
        foreach ($path in @($PackageRoot, $ZipPath, $ChecksumPath)) {
            if (Test-Path -LiteralPath $path) {
                Assert-UnderDirectory -Path $path -Directory $DistDir -Description "distribution directory"
                Remove-Item -LiteralPath $path -Recurse -Force
            }
        }
    }
}
