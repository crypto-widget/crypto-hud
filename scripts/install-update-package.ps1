param(
    [Parameter(Mandatory = $true)]
    [string]$PackageZip,
    [string]$ChecksumPath = "",
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA "CryptoHud"),
    [string]$ExtractRoot = "",
    [switch]$SkipShellIntegration,
    [switch]$StartAfterInstall,
    [switch]$KeepExtracted,
    [switch]$AllowUnsignedPackage
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

function Resolve-PackageRelativePath {
    param(
        [string]$Root,
        [string]$RelativePath
    )

    if ([string]::IsNullOrWhiteSpace($RelativePath)) {
        throw "Package manifest contains an empty file path"
    }
    if ([System.IO.Path]::IsPathRooted($RelativePath)) {
        throw "Package manifest contains a rooted file path: $RelativePath"
    }
    $segments = $RelativePath -split '[\\/]'
    if ($segments -contains ".." -or $segments -contains "." -or $segments -contains "") {
        throw "Package manifest contains an unsafe file path: $RelativePath"
    }

    $fullRoot = [System.IO.Path]::GetFullPath($Root)
    $rootPrefix = $fullRoot
    if (-not $rootPrefix.EndsWith([System.IO.Path]::DirectorySeparatorChar)) {
        $rootPrefix = "$rootPrefix$([System.IO.Path]::DirectorySeparatorChar)"
    }
    $fullPath = [System.IO.Path]::GetFullPath((Join-Path $fullRoot $RelativePath))
    if (-not $fullPath.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Package file resolves outside the extracted directory: $RelativePath"
    }
    $fullPath
}

function Get-ValidSignerSubject {
    param(
        [string]$Path,
        [string]$Description
    )

    if (-not (Test-Path -LiteralPath $Path)) {
        throw "$Description is missing: $Path"
    }
    $signature = Get-AuthenticodeSignature -LiteralPath $Path
    if ($signature.Status -ne "Valid" -or -not $signature.SignerCertificate) {
        throw "$Description does not have a valid Authenticode signature: $($signature.Status)"
    }
    [string]$signature.SignerCertificate.Subject
}

if (-not (Test-Path -LiteralPath $PackageZip)) {
    throw "Package zip not found: $PackageZip"
}
if (-not (Test-Path -LiteralPath $ChecksumPath)) {
    throw "Checksum file not found: $ChecksumPath"
}

$checksumLine = (Get-Content -LiteralPath $ChecksumPath -Raw).Trim()
if ($checksumLine -notmatch '^([a-fA-F0-9]{64})\s{2}([^\r\n]+)$') {
    throw "Checksum file must contain one SHA-256 entry"
}
$expectedZipHash = $Matches[1]
$checksumFileName = $Matches[2]
if ($checksumFileName -ne (Split-Path -Leaf $PackageZip)) {
    throw "Checksum entry does not match the update package name"
}
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

    if (-not $AllowUnsignedPackage) {
        if ([string]$manifest.version -notmatch '^v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$') {
            throw "Production update package has an invalid version: $($manifest.version)"
        }
        if (-not ($manifest.PSObject.Properties.Name -contains "sourceDirty") -or [bool]$manifest.sourceDirty) {
            throw "Production update package was built from an unverified or dirty source tree"
        }
        if (-not $manifest.codeSigning -or
            -not [bool]$manifest.codeSigning.required -or
            -not [bool]$manifest.codeSigning.signed) {
            throw "Production update package is not marked as fully Authenticode signed"
        }
    }

    if ($manifest.executable -ne "crypto-hud.exe") {
        throw "Unexpected package executable: $($manifest.executable)"
    }
    if ($manifest.installer.script -ne "install.ps1" -or
        $manifest.installer.uninstallScript -ne "uninstall.ps1") {
        throw "Unexpected installer scripts in release manifest"
    }

    $requiredFiles = @(
        "crypto-hud.exe",
        "install.ps1",
        "uninstall.ps1",
        "install-update-package.ps1"
    )
    $manifestPaths = @()
    $seenPaths = @{}
    foreach ($file in @($manifest.files)) {
        $relativePath = ([string]$file.path).Replace('\', '/')
        $pathKey = $relativePath.ToLowerInvariant()
        if ($seenPaths.ContainsKey($pathKey)) {
            throw "Package manifest contains a duplicate file path: $relativePath"
        }
        $seenPaths[$pathKey] = $true
        $resolvedPath = Resolve-PackageRelativePath -Root $ExtractRoot -RelativePath $relativePath
        Assert-Hash -Path $resolvedPath -ExpectedHash ([string]$file.sha256)
        $manifestPaths += $relativePath
    }
    foreach ($requiredFile in $requiredFiles) {
        if (-not ($manifestPaths -contains $requiredFile)) {
            throw "Package manifest is missing required file: $requiredFile"
        }
    }
    Assert-Hash `
        -Path (Join-Path $ExtractRoot "crypto-hud.exe") `
        -ExpectedHash ([string]$manifest.executableSha256)

    $rootPrefix = [System.IO.Path]::GetFullPath($ExtractRoot)
    if (-not $rootPrefix.EndsWith([System.IO.Path]::DirectorySeparatorChar)) {
        $rootPrefix = "$rootPrefix$([System.IO.Path]::DirectorySeparatorChar)"
    }
    $actualPaths = @(Get-ChildItem -LiteralPath $ExtractRoot -Recurse -File | ForEach-Object {
        $_.FullName.Substring($rootPrefix.Length).Replace('\', '/')
    })
    $allowedPaths = @($manifestPaths) + "release-manifest.json"
    $unexpectedPaths = @(Compare-Object -ReferenceObject $allowedPaths -DifferenceObject $actualPaths |
        Where-Object { $_.SideIndicator -eq "=>" })
    if ($unexpectedPaths.Count -gt 0) {
        throw "Update package contains files not declared by the manifest: $($unexpectedPaths.InputObject -join ', ')"
    }

    if (-not $AllowUnsignedPackage) {
        $installedExecutable = Join-Path $InstallDir "crypto-hud.exe"
        $trustedSubject = Get-ValidSignerSubject `
            -Path $installedExecutable `
            -Description "Currently installed Crypto HUD executable"
        $updaterSubject = Get-ValidSignerSubject `
            -Path $PSCommandPath `
            -Description "Installed update helper"
        if (-not $updaterSubject.Equals($trustedSubject, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "Installed update helper publisher does not match the installed application"
        }
        foreach ($relativePath in $requiredFiles) {
            $candidateSubject = Get-ValidSignerSubject `
                -Path (Join-Path $ExtractRoot $relativePath) `
                -Description "Update package file $relativePath"
            if (-not $candidateSubject.Equals($trustedSubject, [System.StringComparison]::OrdinalIgnoreCase)) {
                throw "Update package publisher does not match the installed application: $relativePath"
            }
        }
        if (-not ([string]$manifest.codeSigning.subject).Equals(
            $trustedSubject,
            [System.StringComparison]::OrdinalIgnoreCase
        )) {
            throw "Release manifest publisher does not match the installed application"
        }
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
