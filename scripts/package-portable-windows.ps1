param(
    [string]$Version = "",
    [switch]$SkipBuild,
    [switch]$AllowDirty,
    [switch]$AllowDevelopmentVersion
)

$ErrorActionPreference = "Stop"
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$WorkspaceManifest = Join-Path $RepoRoot "Cargo.toml"
$workspaceCargo = Get-Content -LiteralPath $WorkspaceManifest -Raw
$versionMatch = [regex]::Match(
    $workspaceCargo,
    '(?ms)^\[workspace\.package\].*?^version\s*=\s*"([^"]+)"'
)
if (-not $versionMatch.Success) {
    throw "Could not read workspace package version from $WorkspaceManifest"
}

$workspaceVersion = $versionMatch.Groups[1].Value
if ([string]::IsNullOrWhiteSpace($Version)) {
    $Version = "v$workspaceVersion"
}
if ([System.IO.Path]::IsPathRooted($Version) -or
    $Version.IndexOfAny([System.IO.Path]::GetInvalidFileNameChars()) -ge 0 -or
    $Version.Contains("\") -or
    $Version.Contains("/") -or
    $Version.Contains("..") -or
    $Version -notmatch '^[0-9A-Za-z][0-9A-Za-z.-]{0,63}$') {
    throw "Version must be a safe filename component: $Version"
}
if (-not $AllowDevelopmentVersion) {
    if ($Version -notmatch '^v([0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?)$') {
        throw "Portable release version must use v-prefixed SemVer: $Version"
    }
    if ($Matches[1] -ne $workspaceVersion) {
        throw "Portable release version $Version does not match workspace version $workspaceVersion"
    }
}

$AppManifestPath = Join-Path $RepoRoot "crates\crypto-hud\ui\app.manifest"
$appManifest = Get-Content -LiteralPath $AppManifestPath -Raw
$appVersionMatch = [regex]::Match(
    $appManifest,
    '(?s)<assemblyIdentity\b[^>]*\bversion="([^"]+)"'
)
$expectedAppVersion = "$($workspaceVersion.Split('-')[0]).0"
if (-not $appVersionMatch.Success -or
    $appVersionMatch.Groups[1].Value -ne $expectedAppVersion) {
    throw "Windows app manifest version must match workspace version as $expectedAppVersion"
}

$DistDir = Join-Path $RepoRoot "dist"
$PackageRoot = Join-Path $DistDir "crypto-hud-$Version-windows-x64-portable"
$ZipPath = "$PackageRoot.zip"
$ChecksumPath = "$ZipPath.sha256"
$Exe = Join-Path $RepoRoot "target\release\crypto-hud.exe"
$ManifestPath = Join-Path $PackageRoot "portable-manifest.json"

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
    $directoryPrefix = "$fullDirectory$([System.IO.Path]::DirectorySeparatorChar)"
    if (-not $fullPath.StartsWith($directoryPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
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
                throw "Refusing to use a reparse point in a portable release path: $current"
            }
        }
        if ($current.TrimEnd('\', '/').Equals($stopPath, [System.StringComparison]::OrdinalIgnoreCase)) {
            break
        }
        $current = Split-Path -Parent $current
    }
}

function Resolve-PackageTargetPath {
    param([string]$RelativePath)

    if ([string]::IsNullOrWhiteSpace($RelativePath) -or
        [System.IO.Path]::IsPathRooted($RelativePath)) {
        throw "Portable package target path must be relative: $RelativePath"
    }
    $segments = $RelativePath -split '[\\/]'
    if ($segments -contains "" -or $segments -contains "." -or $segments -contains "..") {
        throw "Portable package target path is unsafe: $RelativePath"
    }
    foreach ($segment in $segments) {
        if ($segment.IndexOfAny([System.IO.Path]::GetInvalidFileNameChars()) -ge 0 -or
            $segment.EndsWith('.') -or $segment.EndsWith(' ') -or
            $segment.Split('.')[0] -match '^(?i:CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])$') {
            throw "Portable package target path contains an invalid filename: $RelativePath"
        }
    }
    $target = [System.IO.Path]::GetFullPath((Join-Path $PackageRoot $RelativePath))
    Assert-UnderDirectory -Path $target -Directory $PackageRoot -Description "portable package directory" | Out-Null
    $target
}

function Get-SafeTreeFiles {
    param(
        [string]$SourceRoot,
        [string]$TargetRoot
    )

    $fullSourceRoot = [System.IO.Path]::GetFullPath($SourceRoot)
    Assert-UnderDirectory -Path $fullSourceRoot -Directory $RepoRoot -Description "repository" | Out-Null
    Assert-NoReparsePoint -Path $fullSourceRoot -StopDirectory $RepoRoot
    $sourceItem = Get-Item -LiteralPath $fullSourceRoot -Force
    if (-not $sourceItem.PSIsContainer -or
        ($sourceItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "Portable resource root must be a regular directory: $fullSourceRoot"
    }

    $resolvedSourceRoot = (Resolve-Path -LiteralPath $fullSourceRoot).Path.TrimEnd('\', '/')
    $sourcePrefix = "$resolvedSourceRoot$([System.IO.Path]::DirectorySeparatorChar)"
    foreach ($item in Get-ChildItem -LiteralPath $resolvedSourceRoot -Recurse -Force) {
        if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "Portable resources must not contain reparse points: $($item.FullName)"
        }
        if (-not $item.PSIsContainer) {
            $relativePath = $item.FullName.Substring($sourcePrefix.Length).Replace('\', '/')
            [ordered]@{
                Source = $item.FullName
                Target = "$TargetRoot/$relativePath"
            }
        }
    }
}

function Invoke-ReleaseBuildWithoutSigningSecrets {
    $secretNames = @(
        "CRYPTO_HUD_SIGN_CERT_PATH",
        "CRYPTO_HUD_SIGN_CERT_BASE64",
        "CRYPTO_HUD_SIGN_CERT_PASSWORD",
        "CRYPTO_HUD_SIGN_CERT_PASSWORD_PROCESS"
    )
    $savedValues = @{}
    try {
        foreach ($name in $secretNames) {
            if (Test-Path "Env:\$name") {
                $savedValues[$name] = (Get-Item "Env:\$name").Value
                Remove-Item "Env:\$name"
            }
        }
        cargo build --locked --release -p crypto-hud
        if ($LASTEXITCODE -ne 0) {
            throw "Portable release build failed with code $LASTEXITCODE"
        }
    } finally {
        foreach ($name in $secretNames) {
            Remove-Item "Env:\$name" -ErrorAction SilentlyContinue
            if ($savedValues.ContainsKey($name)) {
                Set-Item "Env:\$name" $savedValues[$name]
            }
        }
    }
}

Push-Location $RepoRoot
try {
    Assert-UnderDirectory -Path $DistDir -Directory $RepoRoot -Description "repository" | Out-Null
    Assert-UnderDirectory -Path $PackageRoot -Directory $DistDir -Description "distribution directory" | Out-Null
    Assert-UnderDirectory -Path $ZipPath -Directory $DistDir -Description "distribution directory" | Out-Null
    Assert-UnderDirectory -Path $ChecksumPath -Directory $DistDir -Description "distribution directory" | Out-Null
    Assert-NoReparsePoint -Path $DistDir -StopDirectory $RepoRoot

    $sourceStatus = @(git status --porcelain --untracked-files=normal)
    if ($LASTEXITCODE -ne 0) {
        throw "Could not inspect Git worktree status"
    }
    $sourceDirty = $sourceStatus.Count -gt 0
    if ($sourceDirty -and -not $AllowDirty) {
        throw "Refusing to create a portable release from a dirty worktree. Commit or stash changes first."
    }

    if (-not $SkipBuild) {
        Invoke-ReleaseBuildWithoutSigningSecrets
    }
    if (-not (Test-Path -LiteralPath $Exe -PathType Leaf)) {
        throw "Portable release executable not found: $Exe"
    }

    New-Item -ItemType Directory -Force -Path $DistDir | Out-Null
    Assert-NoReparsePoint -Path $DistDir -StopDirectory $RepoRoot
    if (Test-Path -LiteralPath $PackageRoot) {
        Assert-NoReparsePoint -Path $PackageRoot -StopDirectory $RepoRoot
        Remove-Item -LiteralPath $PackageRoot -Recurse -Force
    }
    New-Item -ItemType Directory -Path $PackageRoot | Out-Null

    $packageFiles = @(
        @{ Source = $Exe; Target = "crypto-hud.exe" },
        @{ Source = (Join-Path $RepoRoot "README.md"); Target = "README.md" },
        @{ Source = (Join-Path $RepoRoot "LICENSE"); Target = "LICENSE" }
    )
    $packageFiles += @(Get-SafeTreeFiles `
        -SourceRoot (Join-Path $RepoRoot "crates\crypto-hud\plugins") `
        -TargetRoot "plugins")
    $packageFiles += @(Get-SafeTreeFiles `
        -SourceRoot (Join-Path $RepoRoot "crates\crypto-hud\ui\previews") `
        -TargetRoot "resources/previews")
    $packageFiles += @{
        Source = (Join-Path $RepoRoot "crates\crypto-hud\ui\icon.ico")
        Target = "resources/icon.ico"
    }

    foreach ($file in $packageFiles) {
        if (-not (Test-Path -LiteralPath $file.Source -PathType Leaf)) {
            throw "Portable release source file is missing: $($file.Source)"
        }
        Assert-UnderDirectory -Path $file.Source -Directory $RepoRoot -Description "repository" | Out-Null
        Assert-NoReparsePoint -Path $file.Source -StopDirectory $RepoRoot
        $targetPath = Resolve-PackageTargetPath -RelativePath $file.Target
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $targetPath) | Out-Null
        Copy-Item -LiteralPath $file.Source -Destination $targetPath
    }

    $fileEntries = @($packageFiles | ForEach-Object {
        $path = Resolve-PackageTargetPath -RelativePath $_.Target
        $item = Get-Item -LiteralPath $path
        $hash = Get-FileHash -Algorithm SHA256 -LiteralPath $path
        [ordered]@{
            path = $_.Target
            sha256 = $hash.Hash.ToLowerInvariant()
            bytes = $item.Length
        }
    } | Sort-Object { $_.path })
    $executableEntry = $fileEntries | Where-Object { $_.path -eq "crypto-hud.exe" } | Select-Object -First 1
    $commit = (git rev-parse HEAD).Trim()
    $manifest = [ordered]@{
        manifestVersion = 1
        distribution = "portable"
        name = "crypto-hud"
        version = $Version
        target = "windows-x64"
        commit = $commit
        sourceDirty = $sourceDirty
        builtAt = (Get-Date).ToUniversalTime().ToString("o")
        executable = "crypto-hud.exe"
        executableSha256 = $executableEntry.sha256
        stateStorage = "user-profile"
        codeSigning = [ordered]@{
            required = $false
            signed = $false
        }
        files = $fileEntries
    }
    $manifest | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $ManifestPath -Encoding UTF8

    $forbiddenScripts = @(Get-ChildItem -LiteralPath $PackageRoot -Recurse -File -Filter *.ps1)
    if ($forbiddenScripts.Count -gt 0) {
        throw "Portable package must not contain PowerShell installer or updater scripts"
    }

    if (Test-Path -LiteralPath $ZipPath) {
        Assert-NoReparsePoint -Path $ZipPath -StopDirectory $RepoRoot
        Remove-Item -LiteralPath $ZipPath -Force
    }
    Compress-Archive -Path (Join-Path $PackageRoot "*") -DestinationPath $ZipPath
    $hash = Get-FileHash -Algorithm SHA256 -LiteralPath $ZipPath
    "$($hash.Hash.ToLowerInvariant())  $(Split-Path -Leaf $ZipPath)" |
        Set-Content -LiteralPath $ChecksumPath -NoNewline

    Write-Host "Created unsigned portable package $ZipPath"
    Write-Host "Created $ChecksumPath"
} finally {
    Pop-Location
}
