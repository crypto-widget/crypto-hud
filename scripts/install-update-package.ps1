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
$PowerShellExe = (Get-Process -Id $PID).Path
if (-not (Test-Path -LiteralPath $PowerShellExe -PathType Leaf) -or
    (Split-Path -Leaf $PowerShellExe) -notmatch '^(?i:powershell|pwsh)(?:\.exe)?$') {
    throw "Current PowerShell host path is not trusted: $PowerShellExe"
}

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

function Assert-NoReparsePoint {
    param(
        [string]$Path,
        [string]$StopDirectory
    )

    $fullPath = [System.IO.Path]::GetFullPath($Path)
    $stopPath = [System.IO.Path]::GetFullPath($StopDirectory).TrimEnd('\', '/')
    $current = $fullPath
    while ($current -and
        $current.Length -ge $stopPath.Length -and
        $current.StartsWith($stopPath, [System.StringComparison]::OrdinalIgnoreCase)) {
        if (Test-Path -LiteralPath $current) {
            $item = Get-Item -LiteralPath $current -Force
            if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw "Refusing to use a reparse point in an update path: $current"
            }
        }
        if ($current.TrimEnd('\', '/').Equals($stopPath, [System.StringComparison]::OrdinalIgnoreCase)) {
            break
        }
        $current = Split-Path -Parent $current
    }
}

function Assert-TreeHasNoReparsePoints {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path)) {
        return
    }
    foreach ($item in @((Get-Item -LiteralPath $Path -Force)) +
        @(Get-ChildItem -LiteralPath $Path -Recurse -Force)) {
        if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "Refusing to remove an update tree containing a reparse point: $($item.FullName)"
        }
    }
}

function Test-TruthyEnv {
    param([string]$Name)

    if (-not (Test-Path "Env:\$Name")) {
        return $false
    }
    (Get-Item "Env:\$Name").Value -match '^(1|true|yes)$'
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
    foreach ($segment in $segments) {
        if ($segment.IndexOfAny([System.IO.Path]::GetInvalidFileNameChars()) -ge 0 -or
            $segment.EndsWith('.') -or $segment.EndsWith(' ') -or
            $segment.Split('.')[0] -match '^(?i:CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])$') {
            throw "Package manifest contains invalid filename characters: $RelativePath"
        }
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

function Copy-FileSnapshot {
    param(
        [string]$Source,
        [string]$Destination,
        [int64]$MaxBytes
    )

    $sourceStream = [System.IO.File]::Open(
        $Source,
        [System.IO.FileMode]::Open,
        [System.IO.FileAccess]::Read,
        [System.IO.FileShare]::Read
    )
    try {
        if ($MaxBytes -le 0 -or $sourceStream.Length -gt $MaxBytes) {
            throw "Update source exceeds the $MaxBytes byte snapshot limit: $Source"
        }
        $destinationStream = [System.IO.File]::Open(
            $Destination,
            [System.IO.FileMode]::CreateNew,
            [System.IO.FileAccess]::Write,
            [System.IO.FileShare]::None
        )
        try {
            $buffer = [byte[]]::new(1MB)
            [int64]$copiedBytes = 0
            while (($readBytes = $sourceStream.Read($buffer, 0, $buffer.Length)) -gt 0) {
                $copiedBytes += $readBytes
                if ($copiedBytes -gt $MaxBytes) {
                    throw "Update source changed beyond the $MaxBytes byte snapshot limit: $Source"
                }
                $destinationStream.Write($buffer, 0, $readBytes)
            }
            $destinationStream.Flush($true)
        } finally {
            $destinationStream.Dispose()
        }
    } finally {
        $sourceStream.Dispose()
    }
}

function Assert-SafeZipEntries {
    param([string]$Path)

    $maxEntryCount = 4096
    $maxEntryBytes = 128MB
    $maxTotalBytes = 512MB
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = [System.IO.Compression.ZipFile]::OpenRead($Path)
    try {
        $seen = @{}
        $entryCount = 0
        [int64]$totalExpandedBytes = 0
        foreach ($entry in $archive.Entries) {
            $entryCount++
            if ($entryCount -gt $maxEntryCount) {
                throw "Update zip contains more than $maxEntryCount entries"
            }
            $relativePath = $entry.FullName.Replace('\', '/')
            if ([string]::IsNullOrWhiteSpace($relativePath)) {
                throw "Update zip contains an empty entry name"
            }
            if ($relativePath.Length -gt 240) {
                throw "Update zip entry path is too long: $relativePath"
            }
            if ([System.IO.Path]::IsPathRooted($relativePath) -or $relativePath.StartsWith('/')) {
                throw "Update zip contains a rooted entry: $relativePath"
            }
            $segments = $relativePath.TrimEnd('/') -split '/'
            if ($segments -contains "" -or $segments -contains "." -or $segments -contains "..") {
                throw "Update zip contains an unsafe entry: $relativePath"
            }
            foreach ($segment in $segments) {
                if ($segment -match '[\x00-\x1f<>:"|?*]' -or
                    $segment.EndsWith('.') -or $segment.EndsWith(' ') -or
                    $segment.Split('.')[0] -match '^(?i:CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])$') {
                    throw "Update zip contains a Windows-unsafe entry: $relativePath"
                }
            }
            $key = $relativePath.TrimEnd('/').ToLowerInvariant()
            if ($seen.ContainsKey($key)) {
                throw "Update zip contains a duplicate entry: $relativePath"
            }
            $seen[$key] = $true
            $unixMode = ($entry.ExternalAttributes -shr 16) -band 0xF000
            if ($unixMode -eq 0xA000) {
                throw "Update zip contains a symbolic link: $relativePath"
            }
            if ($entry.Length -gt $maxEntryBytes) {
                throw "Update zip entry exceeds the $maxEntryBytes byte limit: $relativePath"
            }
            $totalExpandedBytes += $entry.Length
            if ($totalExpandedBytes -gt $maxTotalBytes) {
                throw "Update zip exceeds the $maxTotalBytes byte expanded-size limit"
            }
        }
    } finally {
        $archive.Dispose()
    }
}

function Read-IntegrityMetadata {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Release integrity metadata is missing: $Path"
    }
    $content = Get-Content -LiteralPath $Path -Raw
    $hashMatch = [regex]::Match($content, '(?m)^# CryptoHud-Manifest-SHA256: ([a-fA-F0-9]{64})\r?$')
    $versionMatch = [regex]::Match($content, '(?m)^# CryptoHud-Version: ([0-9A-Za-z][0-9A-Za-z.-]{0,63})\r?$')
    if (-not $hashMatch.Success -or -not $versionMatch.Success) {
        throw "Release integrity metadata is malformed: $Path"
    }
    [ordered]@{
        manifestSha256 = $hashMatch.Groups[1].Value.ToLowerInvariant()
        version = $versionMatch.Groups[1].Value
    }
}

function ConvertTo-SemVerParts {
    param([string]$Version)

    $match = [regex]::Match(
        $Version,
        '^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:-([0-9A-Za-z.-]+))?$'
    )
    if (-not $match.Success) {
        throw "Invalid release SemVer: $Version"
    }
    $preRelease = if ($match.Groups[4].Success) {
        @($match.Groups[4].Value -split '\.')
    } else {
        @()
    }
    foreach ($identifier in $preRelease) {
        if ([string]::IsNullOrEmpty($identifier) -or
            ($identifier -match '^[0-9]+$' -and $identifier.Length -gt 1 -and $identifier.StartsWith('0'))) {
            throw "Invalid release SemVer pre-release identifier: $Version"
        }
    }
    [ordered]@{
        major = [uint64]$match.Groups[1].Value
        minor = [uint64]$match.Groups[2].Value
        patch = [uint64]$match.Groups[3].Value
        preRelease = $preRelease
    }
}

function Compare-SemVer {
    param(
        [string]$Left,
        [string]$Right
    )

    $leftParts = ConvertTo-SemVerParts -Version $Left
    $rightParts = ConvertTo-SemVerParts -Version $Right
    foreach ($field in @('major', 'minor', 'patch')) {
        if ($leftParts[$field] -lt $rightParts[$field]) { return -1 }
        if ($leftParts[$field] -gt $rightParts[$field]) { return 1 }
    }
    $leftPre = @($leftParts.preRelease)
    $rightPre = @($rightParts.preRelease)
    if ($leftPre.Count -eq 0 -and $rightPre.Count -eq 0) { return 0 }
    if ($leftPre.Count -eq 0) { return 1 }
    if ($rightPre.Count -eq 0) { return -1 }

    $identifierCount = [Math]::Max($leftPre.Count, $rightPre.Count)
    for ($index = 0; $index -lt $identifierCount; $index++) {
        if ($index -ge $leftPre.Count) { return -1 }
        if ($index -ge $rightPre.Count) { return 1 }
        $leftIdentifier = $leftPre[$index]
        $rightIdentifier = $rightPre[$index]
        $leftNumeric = $leftIdentifier -match '^[0-9]+$'
        $rightNumeric = $rightIdentifier -match '^[0-9]+$'
        if ($leftNumeric -and $rightNumeric) {
            if ($leftIdentifier.Length -lt $rightIdentifier.Length) { return -1 }
            if ($leftIdentifier.Length -gt $rightIdentifier.Length) { return 1 }
            $numericComparison = [string]::CompareOrdinal($leftIdentifier, $rightIdentifier)
            if ($numericComparison -lt 0) { return -1 }
            if ($numericComparison -gt 0) { return 1 }
        } elseif ($leftNumeric) {
            return -1
        } elseif ($rightNumeric) {
            return 1
        } else {
            $comparison = [string]::CompareOrdinal($leftIdentifier, $rightIdentifier)
            if ($comparison -lt 0) { return -1 }
            if ($comparison -gt 0) { return 1 }
        }
    }
    0
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

Assert-NoReparsePoint -Path $PackageZip -StopDirectory ([System.IO.Path]::GetPathRoot($PackageZip))
Assert-NoReparsePoint -Path $ChecksumPath -StopDirectory ([System.IO.Path]::GetPathRoot($ChecksumPath))
Assert-NoReparsePoint -Path $InstallDir -StopDirectory ([System.IO.Path]::GetPathRoot($InstallDir))

$SnapshotRoot = Join-Path $TempRoot ("crypto-hud-update-snapshot-{0}-{1}" -f $PID, [guid]::NewGuid().ToString("N"))
$PackageSnapshot = Join-Path $SnapshotRoot "package.zip"
$ChecksumSnapshot = Join-Path $SnapshotRoot "package.sha256"
Assert-UnderDirectory -Path $SnapshotRoot -Directory $TempRoot -Description "temporary update snapshot directory"

try {
    New-Item -ItemType Directory -Path $SnapshotRoot | Out-Null
    Assert-NoReparsePoint -Path $SnapshotRoot -StopDirectory $TempRoot

    # Lock each source while copying, then perform every validation and extraction
    # against the immutable local snapshot to close check/use races on remote paths.
    Copy-FileSnapshot -Source $PackageZip -Destination $PackageSnapshot -MaxBytes 512MB
    Copy-FileSnapshot -Source $ChecksumPath -Destination $ChecksumSnapshot -MaxBytes 1MB
    Assert-NoReparsePoint -Path $PackageSnapshot -StopDirectory $TempRoot
    Assert-NoReparsePoint -Path $ChecksumSnapshot -StopDirectory $TempRoot

    $checksumLine = (Get-Content -LiteralPath $ChecksumSnapshot -Raw).Trim()
    if ($checksumLine -notmatch '^([a-fA-F0-9]{64})\s{2}([^\r\n]+)$') {
        throw "Checksum file must contain one SHA-256 entry"
    }
    $expectedZipHash = $Matches[1]
    $checksumFileName = $Matches[2]
    if ($checksumFileName -ne (Split-Path -Leaf $PackageZip)) {
        throw "Checksum entry does not match the update package name"
    }
    Assert-Hash -Path $PackageSnapshot -ExpectedHash $expectedZipHash
    Assert-SafeZipEntries -Path $PackageSnapshot

    Assert-UnderDirectory -Path $ExtractRoot -Directory $TempRoot -Description "temporary update directory"
    if (Test-Path -LiteralPath $ExtractRoot) {
        Assert-NoReparsePoint -Path $ExtractRoot -StopDirectory $TempRoot
        Assert-TreeHasNoReparsePoints -Path $ExtractRoot
        Remove-Item -LiteralPath $ExtractRoot -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $ExtractRoot | Out-Null
    Assert-NoReparsePoint -Path $ExtractRoot -StopDirectory $TempRoot

    try {
        Expand-Archive -LiteralPath $PackageSnapshot -DestinationPath $ExtractRoot -Force

    $manifestPath = Join-Path $ExtractRoot "release-manifest.json"
    $integrityPath = Join-Path $ExtractRoot "release-integrity.ps1"
    $installScript = Join-Path $ExtractRoot "install.ps1"
    if (-not (Test-Path -LiteralPath $manifestPath)) {
        throw "Extracted update package is missing release-manifest.json"
    }
    if (-not (Test-Path -LiteralPath $installScript)) {
        throw "Extracted update package is missing install.ps1"
    }

    $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
    if ([int]$manifest.manifestVersion -ne 2) {
        throw "Unsupported release manifest version: $($manifest.manifestVersion)"
    }
    if ($manifest.name -ne "crypto-hud") {
        throw "Unexpected package name: $($manifest.name)"
    }
    if ($manifest.target -ne "windows-x64") {
        throw "Unexpected package target: $($manifest.target)"
    }

    $candidateVersion = [string]$manifest.version
    if ($AllowUnsignedPackage -and
        (-not (Test-TruthyEnv -Name "CRYPTO_HUD_ALLOW_UNSIGNED_SMOKE") -or
            ($candidateVersion -match '^v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$' -and
                $candidateVersion -notmatch '-smoke$'))) {
        throw "-AllowUnsignedPackage is restricted to explicitly enabled development smoke packages"
    }

    $integrity = Read-IntegrityMetadata -Path $integrityPath
    $manifestHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $manifestPath).Hash.ToLowerInvariant()
    if ($integrity.manifestSha256 -ne $manifestHash -or
        $integrity.version -ne $candidateVersion) {
        throw "Release integrity metadata does not match the update manifest"
    }
    if ([string]$manifest.codeSigning.detachedManifest -ne "release-integrity.ps1") {
        throw "Release manifest does not name the required signed integrity metadata"
    }

    if (-not $AllowUnsignedPackage) {
        if ($candidateVersion -notmatch '^v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$') {
            throw "Production update package has an invalid version: $candidateVersion"
        }
        ConvertTo-SemVerParts -Version $candidateVersion | Out-Null
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
        "install-update-package.ps1",
        "LICENSE",
        "plugins/README.md",
        "resources/icon.ico",
        "resources/previews/mini-ticker-dark.png",
        "resources/previews/mini-ticker-light.png",
        "resources/previews/quote-board-dark.png",
        "resources/previews/quote-board-light.png"
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
    $actualPaths = @(Get-ChildItem -LiteralPath $ExtractRoot -Recurse -File -Force | ForEach-Object {
        if (($_.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "Update package contains a reparse point: $($_.FullName)"
        }
        $_.FullName.Substring($rootPrefix.Length).Replace('\', '/')
    })
    $allowedPaths = @($manifestPaths) + @("release-manifest.json", "release-integrity.ps1")
    $unexpectedPaths = @(Compare-Object -ReferenceObject $allowedPaths -DifferenceObject $actualPaths |
        Where-Object { $_.SideIndicator -eq "=>" })
    if ($unexpectedPaths.Count -gt 0) {
        throw "Update package contains files not declared by the manifest: $($unexpectedPaths.InputObject -join ', ')"
    }

    $installedExecutable = Join-Path $InstallDir "crypto-hud.exe"
    $installedManifestPath = Join-Path $InstallDir "release-manifest.json"
    $installedIntegrityPath = Join-Path $InstallDir "release-integrity.ps1"
    $installedManifest = $null
    $installedVersion = ""
    if (Test-Path -LiteralPath $installedManifestPath -PathType Leaf) {
        $installedManifest = Get-Content -LiteralPath $installedManifestPath -Raw | ConvertFrom-Json
        if ([int]$installedManifest.manifestVersion -ne 2 -or
            $installedManifest.name -ne "crypto-hud" -or
            $installedManifest.target -ne "windows-x64" -or
            $installedManifest.executable -ne "crypto-hud.exe") {
            throw "Currently installed Crypto HUD manifest is invalid"
        }
        $installedIntegrity = Read-IntegrityMetadata -Path $installedIntegrityPath
        $installedManifestHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $installedManifestPath).Hash.ToLowerInvariant()
        $installedVersion = [string]$installedManifest.version
        if ($installedIntegrity.manifestSha256 -ne $installedManifestHash -or
            $installedIntegrity.version -ne $installedVersion) {
            throw "Currently installed release manifest is not bound to its integrity metadata"
        }
        Assert-Hash `
            -Path $installedExecutable `
            -ExpectedHash ([string]$installedManifest.executableSha256)
        if ($candidateVersion -match '^v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$' -and
            $installedVersion -match '^v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$' -and
            (Compare-SemVer -Left $candidateVersion -Right $installedVersion) -lt 0) {
            throw "Refusing to downgrade Crypto HUD from $installedVersion to $candidateVersion"
        }
    }

    if (-not $AllowUnsignedPackage) {
        if (-not $installedManifest) {
            throw "Currently installed Crypto HUD manifest is missing"
        }
        $trustedSubject = Get-ValidSignerSubject `
            -Path $installedExecutable `
            -Description "Currently installed Crypto HUD executable"
        $updaterSubject = Get-ValidSignerSubject `
            -Path $PSCommandPath `
            -Description "Installed update helper"
        if (-not $updaterSubject.Equals($trustedSubject, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "Installed update helper publisher does not match the installed application"
        }

        $candidateIntegritySubject = Get-ValidSignerSubject `
            -Path $integrityPath `
            -Description "Update package release integrity metadata"
        if (-not $candidateIntegritySubject.Equals(
            $trustedSubject,
            [System.StringComparison]::OrdinalIgnoreCase
        )) {
            throw "Update package integrity metadata publisher does not match the installed application"
        }

        $signablePaths = @($manifestPaths | Where-Object {
            [System.IO.Path]::GetExtension($_) -in @('.exe', '.dll', '.ps1', '.msi')
        })
        $declaredSignedPaths = @($manifest.codeSigning.files | ForEach-Object {
            ([string]$_.path).Replace('\', '/')
        })
        foreach ($relativePath in $signablePaths) {
            if (-not ($declaredSignedPaths -contains $relativePath)) {
                throw "Update manifest does not declare a signable file as signed: $relativePath"
            }
            $candidateSubject = Get-ValidSignerSubject `
                -Path (Resolve-PackageRelativePath -Root $ExtractRoot -RelativePath $relativePath) `
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

        $installedIntegritySubject = Get-ValidSignerSubject `
            -Path $installedIntegrityPath `
            -Description "Currently installed release integrity metadata"
        if (-not $installedIntegritySubject.Equals(
            $trustedSubject,
            [System.StringComparison]::OrdinalIgnoreCase
        )) {
            throw "Installed release integrity metadata publisher does not match the application"
        }
        if (-not ([string]$installedManifest.codeSigning.subject).Equals(
            $trustedSubject,
            [System.StringComparison]::OrdinalIgnoreCase
        )) {
            throw "Installed release manifest publisher does not match the application"
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
    if ($AllowUnsignedPackage) {
        $installArgs += "-AllowUnsignedPackage"
    }

    & $PowerShellExe -NoProfile @installArgs
    if ($LASTEXITCODE -ne 0) {
        throw "Update package install failed with code $LASTEXITCODE"
    }

    Write-Host "Installed update package to $InstallDir"
    } finally {
        if (-not $KeepExtracted -and (Test-Path -LiteralPath $ExtractRoot)) {
            Assert-UnderDirectory -Path $ExtractRoot -Directory $TempRoot -Description "temporary update directory"
            Assert-NoReparsePoint -Path $ExtractRoot -StopDirectory $TempRoot
            Assert-TreeHasNoReparsePoints -Path $ExtractRoot
            Remove-Item -LiteralPath $ExtractRoot -Recurse -Force
        }
    }
} finally {
    if (Test-Path -LiteralPath $SnapshotRoot) {
        Assert-UnderDirectory -Path $SnapshotRoot -Directory $TempRoot -Description "temporary update snapshot directory"
        Assert-NoReparsePoint -Path $SnapshotRoot -StopDirectory $TempRoot
        Assert-TreeHasNoReparsePoints -Path $SnapshotRoot
        Remove-Item -LiteralPath $SnapshotRoot -Recurse -Force
    }
}
