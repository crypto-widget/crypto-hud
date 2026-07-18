param(
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA "CryptoHud"),
    [switch]$StartAfterInstall,
    [switch]$SkipShellIntegration,
    [switch]$AllowMissingManifest,
    [switch]$AllowUnsignedPackage
)

$ErrorActionPreference = "Stop"

$PackageDir = [System.IO.Path]::GetFullPath((Split-Path -Parent $MyInvocation.MyCommand.Path))
$InstallDir = [System.IO.Path]::GetFullPath($InstallDir)
$ExeName = "crypto-hud.exe"
$ManifestName = "release-manifest.json"
$IntegrityName = "release-integrity.ps1"
$ManifestPath = Join-Path $PackageDir $ManifestName
$IntegrityPath = Join-Path $PackageDir $IntegrityName
$StartMenuDir = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Crypto HUD"
$ShortcutPath = Join-Path $StartMenuDir "Crypto HUD.lnk"
$UninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\CryptoHud"
$LegacyStartMenuDir = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Crypto Widget"
$LegacyShortcutPath = Join-Path $LegacyStartMenuDir "Crypto Widget.lnk"
$LegacyUninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\CryptoWidget.CryptoHud"

function Test-TruthyEnv {
    param([string]$Name)

    if (-not (Test-Path "Env:\$Name")) {
        return $false
    }
    (Get-Item "Env:\$Name").Value -match '^(1|true|yes)$'
}

function Assert-UnderDirectory {
    param(
        [string]$Path,
        [string]$Directory,
        [string]$Description,
        [switch]$AllowDirectoryItself
    )

    $fullPath = [System.IO.Path]::GetFullPath($Path).TrimEnd('\', '/')
    $fullDirectory = [System.IO.Path]::GetFullPath($Directory).TrimEnd('\', '/')
    if ($AllowDirectoryItself -and
        $fullPath.Equals($fullDirectory, [System.StringComparison]::OrdinalIgnoreCase)) {
        return $fullPath
    }
    $prefix = "$fullDirectory$([System.IO.Path]::DirectorySeparatorChar)"
    if (-not $fullPath.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to operate outside ${Description}: $fullPath"
    }
    $fullPath
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
                throw "Refusing to use a reparse point in an install path: $current"
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
    $root = Get-Item -LiteralPath $Path -Force
    if (($root.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "Refusing to modify a reparse point: $Path"
    }
    foreach ($item in Get-ChildItem -LiteralPath $Path -Recurse -Force) {
        if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "Refusing to modify a directory tree containing a reparse point: $($item.FullName)"
        }
    }
}

function Resolve-SafeRelativePath {
    param(
        [string]$Root,
        [string]$RelativePath,
        [string]$Description
    )

    if ([string]::IsNullOrWhiteSpace($RelativePath) -or
        [System.IO.Path]::IsPathRooted($RelativePath)) {
        throw "$Description must be a non-empty relative path: $RelativePath"
    }
    $segments = $RelativePath -split '[\\/]'
    if ($segments -contains "" -or $segments -contains "." -or $segments -contains "..") {
        throw "$Description contains an unsafe path segment: $RelativePath"
    }
    foreach ($segment in $segments) {
        if ($segment.IndexOfAny([System.IO.Path]::GetInvalidFileNameChars()) -ge 0 -or
            $segment.EndsWith('.') -or $segment.EndsWith(' ') -or
            $segment.Split('.')[0] -match '^(?i:CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])$') {
            throw "$Description contains invalid filename characters: $RelativePath"
        }
    }
    $fullPath = [System.IO.Path]::GetFullPath((Join-Path $Root $RelativePath))
    Assert-UnderDirectory -Path $fullPath -Directory $Root -Description $Description | Out-Null
    $fullPath
}

function Assert-FileHash {
    param(
        [string]$Path,
        [string]$ExpectedHash
    )

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Package file is missing: $Path"
    }
    if ($ExpectedHash -notmatch '^[a-fA-F0-9]{64}$') {
        throw "Package manifest contains an invalid SHA-256 for $Path"
    }
    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
    if ($actual -ne $ExpectedHash.ToLowerInvariant()) {
        throw "Package file hash mismatch for $Path"
    }
}

function Get-ValidSignerSubject {
    param(
        [string]$Path,
        [string]$Description
    )

    $signature = Get-AuthenticodeSignature -LiteralPath $Path
    if ($signature.Status -ne "Valid" -or -not $signature.SignerCertificate) {
        throw "$Description does not have a valid Authenticode signature: $($signature.Status)"
    }
    [string]$signature.SignerCertificate.Subject
}

function Read-IntegrityMetadata {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Package is missing $IntegrityName"
    }
    $content = Get-Content -LiteralPath $Path -Raw
    $hashMatch = [regex]::Match($content, '(?m)^# CryptoHud-Manifest-SHA256: ([a-fA-F0-9]{64})\r?$')
    $versionMatch = [regex]::Match($content, '(?m)^# CryptoHud-Version: ([0-9A-Za-z][0-9A-Za-z.-]{0,63})\r?$')
    if (-not $hashMatch.Success -or -not $versionMatch.Success) {
        throw "Release integrity metadata is malformed"
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
    $preRelease = if ($match.Groups[4].Success) { @($match.Groups[4].Value -split '\.') } else { @() }
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
    param([string]$Left, [string]$Right)

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
    for ($index = 0; $index -lt [Math]::Max($leftPre.Count, $rightPre.Count); $index++) {
        if ($index -ge $leftPre.Count) { return -1 }
        if ($index -ge $rightPre.Count) { return 1 }
        $leftIdentifier = $leftPre[$index]
        $rightIdentifier = $rightPre[$index]
        $leftNumeric = $leftIdentifier -match '^[0-9]+$'
        $rightNumeric = $rightIdentifier -match '^[0-9]+$'
        if ($leftNumeric -and $rightNumeric) {
            if ($leftIdentifier.Length -lt $rightIdentifier.Length) { return -1 }
            if ($leftIdentifier.Length -gt $rightIdentifier.Length) { return 1 }
            $comparison = [string]::CompareOrdinal($leftIdentifier, $rightIdentifier)
        } elseif ($leftNumeric) {
            return -1
        } elseif ($rightNumeric) {
            return 1
        } else {
            $comparison = [string]::CompareOrdinal($leftIdentifier, $rightIdentifier)
        }
        if ($comparison -lt 0) { return -1 }
        if ($comparison -gt 0) { return 1 }
    }
    0
}

if ($AllowMissingManifest) {
    throw "-AllowMissingManifest is no longer supported; every install requires a complete release manifest"
}
if (-not (Test-Path -LiteralPath $ManifestPath -PathType Leaf)) {
    throw "Package is missing $ManifestName"
}

$manifest = Get-Content -LiteralPath $ManifestPath -Raw | ConvertFrom-Json
if ([int]$manifest.manifestVersion -ne 2) {
    throw "Unsupported release manifest version: $($manifest.manifestVersion)"
}
if ($manifest.name -ne "crypto-hud" -or $manifest.target -ne "windows-x64") {
    throw "Unexpected package identity or target"
}
if ($manifest.executable -ne $ExeName) {
    throw "Manifest executable mismatch: $($manifest.executable)"
}
$version = [string]$manifest.version
if ($version -notmatch '^[0-9A-Za-z][0-9A-Za-z.-]{0,63}$' -or $version.Contains("..")) {
    throw "Package manifest contains an unsafe version: $version"
}
if ($AllowUnsignedPackage -and
    (-not (Test-TruthyEnv -Name "CRYPTO_HUD_ALLOW_UNSIGNED_SMOKE") -or
        ($version -match '^v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$' -and
            $version -notmatch '-smoke$'))) {
    throw "-AllowUnsignedPackage is restricted to explicitly enabled development smoke packages"
}

$packageRoot = [System.IO.Path]::GetPathRoot($PackageDir)
Assert-NoReparsePoint -Path $PackageDir -StopDirectory $packageRoot
$installRoot = [System.IO.Path]::GetPathRoot($InstallDir)
if ($InstallDir.TrimEnd('\', '/').Equals($installRoot.TrimEnd('\', '/'), [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to install into a filesystem root: $InstallDir"
}
Assert-NoReparsePoint -Path $InstallDir -StopDirectory $installRoot
if ($PackageDir.TrimEnd('\', '/').Equals(
    $InstallDir.TrimEnd('\', '/'),
    [System.StringComparison]::OrdinalIgnoreCase
)) {
    throw "Package directory and install directory must be different"
}
if (Test-Path -LiteralPath $InstallDir) {
    Assert-TreeHasNoReparsePoints -Path $InstallDir
}

$manifestPaths = @()
$manifestFilesByPath = @{}
foreach ($file in @($manifest.files)) {
    $relativePath = ([string]$file.path).Replace('\', '/')
    $pathKey = $relativePath.ToLowerInvariant()
    if ($manifestFilesByPath.ContainsKey($pathKey)) {
        throw "Package manifest contains a duplicate file path: $relativePath"
    }
    $sourcePath = Resolve-SafeRelativePath -Root $PackageDir -RelativePath $relativePath -Description "package file path"
    Assert-NoReparsePoint -Path $sourcePath -StopDirectory $PackageDir
    Assert-FileHash -Path $sourcePath -ExpectedHash ([string]$file.sha256
    )
    $manifestFilesByPath[$pathKey] = $file
    $manifestPaths += $relativePath
}

$requiredFiles = @(
    "crypto-hud.exe",
    "install.ps1",
    "uninstall.ps1",
    "install-update-package.ps1",
    "resources/taskbar/crypto_hud_taskbar.dll",
    "LICENSE",
    "plugins/README.md",
    "resources/icon.ico",
    "resources/previews/mini-ticker-dark.png",
    "resources/previews/mini-ticker-light.png",
    "resources/previews/quote-board-dark.png",
    "resources/previews/quote-board-light.png"
)
foreach ($requiredFile in $requiredFiles) {
    if (-not ($manifestPaths -contains $requiredFile)) {
        throw "Package manifest is missing required file: $requiredFile"
    }
}
if (-not (@($manifestPaths | Where-Object { $_ -like 'plugins/*/widget.json' }).Count) -or
    -not (@($manifestPaths | Where-Object { $_ -like 'plugins/*/ui/main.slint' }).Count)) {
    throw "Package manifest does not contain bundled plugin manifests and entries"
}

$sourceExe = Resolve-SafeRelativePath -Root $PackageDir -RelativePath $ExeName -Description "package executable"
Assert-FileHash -Path $sourceExe -ExpectedHash ([string]$manifest.executableSha256)

$integrity = Read-IntegrityMetadata -Path $IntegrityPath
$actualManifestHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $ManifestPath).Hash.ToLowerInvariant()
if ($integrity.manifestSha256 -ne $actualManifestHash -or $integrity.version -ne $version) {
    throw "Release integrity metadata does not match the package manifest"
}
if ([string]$manifest.codeSigning.detachedManifest -ne $IntegrityName) {
    throw "Release manifest does not name the required signed integrity metadata"
}

$rootPrefix = "$($PackageDir.TrimEnd('\', '/'))$([System.IO.Path]::DirectorySeparatorChar)"
$actualPaths = @(Get-ChildItem -LiteralPath $PackageDir -Recurse -File -Force | ForEach-Object {
    if (($_.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "Package contains a reparse point: $($_.FullName)"
    }
    $_.FullName.Substring($rootPrefix.Length).Replace('\', '/')
})
$allowedPaths = @($manifestPaths) + @($ManifestName, $IntegrityName)
$unexpectedPaths = @(Compare-Object -ReferenceObject $allowedPaths -DifferenceObject $actualPaths |
    Where-Object { $_.SideIndicator -eq "=>" })
if ($unexpectedPaths.Count -gt 0) {
    throw "Package contains files not declared by the release manifest: $($unexpectedPaths.InputObject -join ', ')"
}

if (-not $AllowUnsignedPackage) {
    if ($version -notmatch '^v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$') {
        throw "Production package has an invalid version: $version"
    }
    ConvertTo-SemVerParts -Version $version | Out-Null
    if (-not ($manifest.PSObject.Properties.Name -contains "sourceDirty") -or [bool]$manifest.sourceDirty) {
        throw "Production package was built from an unverified or dirty source tree"
    }
    if (-not $manifest.codeSigning -or
        -not [bool]$manifest.codeSigning.required -or
        -not [bool]$manifest.codeSigning.signed) {
        throw "Production package is not marked as fully Authenticode signed"
    }

    $signablePaths = @($manifestPaths | Where-Object {
        [System.IO.Path]::GetExtension($_) -in @('.exe', '.dll', '.ps1', '.msi')
    })
    $declaredSignedPaths = @($manifest.codeSigning.files | ForEach-Object {
        ([string]$_.path).Replace('\', '/')
    })
    foreach ($relativePath in $signablePaths) {
        if (-not ($declaredSignedPaths -contains $relativePath)) {
            throw "Production manifest does not declare a signable file as signed: $relativePath"
        }
    }

    $publisher = Get-ValidSignerSubject -Path $IntegrityPath -Description "Release integrity metadata"
    foreach ($relativePath in $signablePaths) {
        $signedPath = Resolve-SafeRelativePath -Root $PackageDir -RelativePath $relativePath -Description "signed package file"
        $subject = Get-ValidSignerSubject -Path $signedPath -Description "Package file $relativePath"
        if (-not $subject.Equals($publisher, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "Package files were signed by different publishers: $relativePath"
        }
    }
    if (-not ([string]$manifest.codeSigning.subject).Equals(
        $publisher,
        [System.StringComparison]::OrdinalIgnoreCase
    )) {
        throw "Release manifest publisher does not match the signed package"
    }
}

$existingManifestPath = Join-Path $InstallDir $ManifestName
$existingIntegrityPath = Join-Path $InstallDir $IntegrityName
$existingExecutablePath = Join-Path $InstallDir $ExeName
if (Test-Path -LiteralPath $existingManifestPath -PathType Leaf) {
    Assert-NoReparsePoint -Path $existingManifestPath -StopDirectory $InstallDir
    Assert-NoReparsePoint -Path $existingIntegrityPath -StopDirectory $InstallDir
    Assert-NoReparsePoint -Path $existingExecutablePath -StopDirectory $InstallDir
    $existingManifest = Get-Content -LiteralPath $existingManifestPath -Raw | ConvertFrom-Json
    if ([int]$existingManifest.manifestVersion -ne 2 -or
        $existingManifest.name -ne "crypto-hud" -or
        $existingManifest.target -ne "windows-x64" -or
        $existingManifest.executable -ne $ExeName) {
        throw "Existing Crypto HUD installation has an invalid release manifest"
    }
    $existingIntegrity = Read-IntegrityMetadata -Path $existingIntegrityPath
    $existingManifestHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $existingManifestPath).Hash.ToLowerInvariant()
    $existingVersion = [string]$existingManifest.version
    if ($existingIntegrity.manifestSha256 -ne $existingManifestHash -or
        $existingIntegrity.version -ne $existingVersion) {
        throw "Existing release manifest is not bound to its integrity metadata"
    }
    Assert-FileHash `
        -Path $existingExecutablePath `
        -ExpectedHash ([string]$existingManifest.executableSha256)
    if ($version -match '^v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$' -and
        $existingVersion -match '^v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$' -and
        (Compare-SemVer -Left $version -Right $existingVersion) -lt 0) {
        throw "Refusing to downgrade Crypto HUD from $existingVersion to $version"
    }
    if (-not $AllowUnsignedPackage) {
        $existingIntegrityPublisher = Get-ValidSignerSubject `
            -Path $existingIntegrityPath `
            -Description "Existing release integrity metadata"
        $existingExecutablePublisher = Get-ValidSignerSubject `
            -Path $existingExecutablePath `
            -Description "Existing Crypto HUD executable"
        if (-not $existingIntegrityPublisher.Equals($publisher, [System.StringComparison]::OrdinalIgnoreCase) -or
            -not $existingExecutablePublisher.Equals($publisher, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "Existing installation publisher does not match the candidate package"
        }
    }
} elseif ((Test-Path -LiteralPath $existingExecutablePath) -and -not $AllowUnsignedPackage) {
    throw "Refusing to overwrite an existing executable without a trusted release manifest"
}

$installParent = Split-Path -Parent $InstallDir
New-Item -ItemType Directory -Force -Path $installParent | Out-Null
Assert-NoReparsePoint -Path $installParent -StopDirectory $installRoot
$installLeaf = Split-Path -Leaf $InstallDir
if ([string]::IsNullOrWhiteSpace($installLeaf) -or
    $installLeaf.IndexOfAny([System.IO.Path]::GetInvalidFileNameChars()) -ge 0 -or
    $installLeaf.EndsWith('.') -or $installLeaf.EndsWith(' ') -or
    $installLeaf.Split('.')[0] -match '^(?i:CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])$') {
    throw "Install directory must end with a safe filename component: $InstallDir"
}
$operationId = "$PID-$([Guid]::NewGuid().ToString('N'))"
$stagingDir = Join-Path $installParent ".$installLeaf.install-$operationId"
$backupDir = Join-Path $installParent ".$installLeaf.backup-$operationId"
Assert-UnderDirectory -Path $stagingDir -Directory $installParent -Description "install staging directory" | Out-Null
Assert-UnderDirectory -Path $backupDir -Directory $installParent -Description "install backup directory" | Out-Null
New-Item -ItemType Directory -Path $stagingDir | Out-Null

try {
    foreach ($relativePath in $manifestPaths) {
        $sourcePath = Resolve-SafeRelativePath -Root $PackageDir -RelativePath $relativePath -Description "package file"
        $targetPath = Resolve-SafeRelativePath -Root $stagingDir -RelativePath $relativePath -Description "install target"
        $targetDirectory = Split-Path -Parent $targetPath
        New-Item -ItemType Directory -Force -Path $targetDirectory | Out-Null
        Assert-NoReparsePoint -Path $targetDirectory -StopDirectory $stagingDir
        Assert-NoReparsePoint -Path $targetPath -StopDirectory $stagingDir
        Copy-Item -LiteralPath $sourcePath -Destination $targetPath -Force
    }
    $stagedManifestPath = Join-Path $stagingDir $ManifestName
    $stagedIntegrityPath = Join-Path $stagingDir $IntegrityName
    Copy-Item -LiteralPath $ManifestPath -Destination $stagedManifestPath -Force
    Copy-Item -LiteralPath $IntegrityPath -Destination $stagedIntegrityPath -Force
    Assert-FileHash -Path $stagedManifestPath -ExpectedHash $actualManifestHash
    $sourceIntegrityHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $IntegrityPath).Hash
    Assert-FileHash -Path $stagedIntegrityPath -ExpectedHash $sourceIntegrityHash

    foreach ($relativePath in $manifestPaths) {
        $targetPath = Resolve-SafeRelativePath -Root $stagingDir -RelativePath $relativePath -Description "staged install file"
        $file = $manifestFilesByPath[$relativePath.ToLowerInvariant()]
        Assert-FileHash -Path $targetPath -ExpectedHash ([string]$file.sha256)
    }
    if (Test-Path -LiteralPath $InstallDir) {
        Assert-TreeHasNoReparsePoints -Path $InstallDir
        Move-Item -LiteralPath $InstallDir -Destination $backupDir
    }
    try {
        Move-Item -LiteralPath $stagingDir -Destination $InstallDir
    } catch {
        if (Test-Path -LiteralPath $backupDir) {
            Move-Item -LiteralPath $backupDir -Destination $InstallDir
        }
        throw
    }
    if (Test-Path -LiteralPath $backupDir) {
        try {
            Assert-TreeHasNoReparsePoints -Path $backupDir
            Remove-Item -LiteralPath $backupDir -Recurse -Force
        } catch {
            Write-Warning "Installed successfully but could not remove rollback directory ${backupDir}: $($_.Exception.Message)"
        }
    }
} catch {
    if (Test-Path -LiteralPath $stagingDir) {
        Assert-TreeHasNoReparsePoints -Path $stagingDir
        Remove-Item -LiteralPath $stagingDir -Recurse -Force
    }
    throw
}

$targetExe = Join-Path $InstallDir $ExeName
$uninstallScript = Join-Path $InstallDir "uninstall.ps1"
if (-not $SkipShellIntegration) {
    Remove-Item -LiteralPath $LegacyShortcutPath -Force -ErrorAction SilentlyContinue
    if (Test-Path -LiteralPath $LegacyStartMenuDir) {
        $remaining = Get-ChildItem -LiteralPath $LegacyStartMenuDir -Force -ErrorAction SilentlyContinue
        if (-not $remaining) {
            Remove-Item -LiteralPath $LegacyStartMenuDir -Force -ErrorAction SilentlyContinue
        }
    }
    Remove-Item -LiteralPath $LegacyUninstallKey -Recurse -Force -ErrorAction SilentlyContinue

    New-Item -ItemType Directory -Force -Path $StartMenuDir | Out-Null
    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut($ShortcutPath)
    $shortcut.TargetPath = $targetExe
    $shortcut.WorkingDirectory = $InstallDir
    $shortcut.IconLocation = "$(Join-Path $InstallDir 'resources\icon.ico'),0"
    $shortcut.Save()

    New-Item -Path $UninstallKey -Force | Out-Null
    New-ItemProperty -Path $UninstallKey -Name "DisplayName" -Value "Crypto HUD" -PropertyType String -Force | Out-Null
    New-ItemProperty -Path $UninstallKey -Name "DisplayVersion" -Value $version -PropertyType String -Force | Out-Null
    New-ItemProperty -Path $UninstallKey -Name "Publisher" -Value "Crypto HUD Contributors" -PropertyType String -Force | Out-Null
    New-ItemProperty -Path $UninstallKey -Name "InstallLocation" -Value $InstallDir -PropertyType String -Force | Out-Null
    New-ItemProperty -Path $UninstallKey -Name "DisplayIcon" -Value $targetExe -PropertyType String -Force | Out-Null
    New-ItemProperty -Path $UninstallKey -Name "NoModify" -Value 1 -PropertyType DWord -Force | Out-Null
    New-ItemProperty -Path $UninstallKey -Name "NoRepair" -Value 1 -PropertyType DWord -Force | Out-Null
    $systemPowerShell = Join-Path $env:SystemRoot "System32\WindowsPowerShell\v1.0\powershell.exe"
    if (-not (Test-Path -LiteralPath $systemPowerShell -PathType Leaf)) {
        throw "System Windows PowerShell executable was not found: $systemPowerShell"
    }
    New-ItemProperty -Path $UninstallKey -Name "UninstallString" -Value "`"$systemPowerShell`" -NoProfile -ExecutionPolicy AllSigned -File `"$UninstallScript`"" -PropertyType String -Force | Out-Null
}

Write-Host "Installed Crypto HUD to $InstallDir"

if ($StartAfterInstall) {
    Start-Process -FilePath $targetExe
}
