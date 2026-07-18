param(
    [string]$Version = "v9999.0.1-smoke",
    [string]$DowngradeVersion = "v9999.0.0-smoke",
    [switch]$SkipBuild,
    [switch]$KeepPackage
)

$ErrorActionPreference = "Stop"
$PowerShellExe = (Get-Process -Id $PID).Path
if (-not (Test-Path -LiteralPath $PowerShellExe -PathType Leaf) -or
    (Split-Path -Leaf $PowerShellExe) -notmatch '^(?i:powershell|pwsh)(?:\.exe)?$') {
    throw "Current PowerShell host path is not trusted: $PowerShellExe"
}

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
foreach ($candidateVersion in @($Version, $DowngradeVersion)) {
    if ([System.IO.Path]::IsPathRooted($candidateVersion) -or
        $candidateVersion.IndexOfAny([System.IO.Path]::GetInvalidFileNameChars()) -ge 0 -or
        $candidateVersion.Contains("\") -or
        $candidateVersion.Contains("/") -or
        $candidateVersion.Contains("..") -or
        $candidateVersion -notmatch '^[0-9A-Za-z][0-9A-Za-z.-]{0,63}$') {
        throw "Smoke version must be a safe filename component: $candidateVersion"
    }
}
$DistDir = Join-Path $RepoRoot "dist"
$PackageRoot = Join-Path $DistDir "crypto-hud-$Version-windows-x64"
$ZipPath = "$PackageRoot.zip"
$ChecksumPath = "$ZipPath.sha256"
$InstallDir = Join-Path $RepoRoot "target\tmp\update-smoke-install"
$ExtractRoot = Join-Path ([System.IO.Path]::GetTempPath()) "crypto-hud-update-smoke-$PID"
$RejectedExtractRoot = "$ExtractRoot-rejected"
$DowngradeExtractRoot = "$ExtractRoot-downgrade"
$DowngradePackageRoot = Join-Path $DistDir "crypto-hud-$DowngradeVersion-windows-x64"
$DowngradeZipPath = "$DowngradePackageRoot.zip"
$DowngradeChecksumPath = "$DowngradeZipPath.sha256"
$ShellSandbox = Join-Path $RepoRoot "target\tmp\update-smoke-shell"
$SecurityFixtureRoot = Join-Path ([System.IO.Path]::GetTempPath()) "crypto-hud-update-security-$PID"
$OriginalLocalAppData = $env:LOCALAPPDATA
$OriginalAppData = $env:APPDATA
$OriginalUnsignedSmoke = $env:CRYPTO_HUD_ALLOW_UNSIGNED_SMOKE

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

function New-ZipFixture {
    param([string]$Path, [object[]]$Entries)

    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $fileStream = [System.IO.File]::Open($Path, [System.IO.FileMode]::CreateNew)
    $archive = [System.IO.Compression.ZipArchive]::new(
        $fileStream,
        [System.IO.Compression.ZipArchiveMode]::Create,
        $false
    )
    try {
        foreach ($entryDefinition in $Entries) {
            $entry = $archive.CreateEntry(
                [string]$entryDefinition.Name,
                [System.IO.Compression.CompressionLevel]::Optimal
            )
            $entryStream = $entry.Open()
            try {
                if ([int]$entryDefinition.RepeatMegabytes -gt 0) {
                    $buffer = New-Object byte[] (1MB)
                    for ($index = 0; $index -lt [int]$entryDefinition.RepeatMegabytes; $index++) {
                        $entryStream.Write($buffer, 0, $buffer.Length)
                    }
                } else {
                    $bytes = [System.Text.Encoding]::UTF8.GetBytes([string]$entryDefinition.Content)
                    $entryStream.Write($bytes, 0, $bytes.Length)
                }
            } finally {
                $entryStream.Dispose()
            }
        }
    } finally {
        $archive.Dispose()
        $fileStream.Dispose()
    }
}

function Assert-UnsafeZipRejected {
    param(
        [string]$Name,
        [object[]]$Entries,
        [string]$ExpectedMessage
    )

    $zipPath = Join-Path $SecurityFixtureRoot "$Name.zip"
    $checksumPath = "$zipPath.sha256"
    $extractPath = Join-Path $SecurityFixtureRoot "extract-$Name"
    New-ZipFixture -Path $zipPath -Entries $Entries
    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $zipPath).Hash.ToLowerInvariant()
    "$hash  $(Split-Path -Leaf $zipPath)" | Set-Content -LiteralPath $checksumPath -NoNewline
    $savedErrorActionPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = "Continue"
        $output = (& $PowerShellExe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\install-update-package.ps1" `
            -PackageZip $zipPath `
            -ChecksumPath $checksumPath `
            -InstallDir $InstallDir `
            -ExtractRoot $extractPath `
            -SkipShellIntegration 2>&1 | Out-String)
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $savedErrorActionPreference
    }
    if ($exitCode -eq 0) {
        throw "Update helper accepted unsafe zip fixture: $Name"
    }
    if ($output -notlike "*$ExpectedMessage*") {
        throw "Unsafe zip fixture $Name failed for an unexpected reason: $output"
    }
    if (Test-Path -LiteralPath $extractPath) {
        throw "Unsafe zip fixture $Name created an extraction directory"
    }
}

function Assert-OversizedSourceZipRejected {
    $zipPath = Join-Path $SecurityFixtureRoot "oversized-source.zip"
    $checksumPath = "$zipPath.sha256"
    $extractPath = Join-Path $SecurityFixtureRoot "extract-oversized-source"
    $stream = [System.IO.File]::Open(
        $zipPath,
        [System.IO.FileMode]::CreateNew,
        [System.IO.FileAccess]::Write,
        [System.IO.FileShare]::None
    )
    try {
        $stream.SetLength([int64](512MB) + 1)
    } finally {
        $stream.Dispose()
    }
    $zeroHash = "".PadLeft(64, '0')
    "$zeroHash  $(Split-Path -Leaf $zipPath)" | Set-Content -LiteralPath $checksumPath -NoNewline

    $savedErrorActionPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = "Continue"
        $output = (& $PowerShellExe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\install-update-package.ps1" `
            -PackageZip $zipPath `
            -ChecksumPath $checksumPath `
            -InstallDir $InstallDir `
            -ExtractRoot $extractPath `
            -SkipShellIntegration 2>&1 | Out-String)
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $savedErrorActionPreference
    }
    if ($exitCode -eq 0 -or $output -notlike "*snapshot limit*") {
        throw "Update helper did not reject an oversized source zip before copying: $output"
    }
    if (Test-Path -LiteralPath $extractPath) {
        throw "Oversized source zip created an extraction directory"
    }
}

Assert-UnderRepo -Path $InstallDir
Assert-UnderTemp -Path $ExtractRoot
Assert-UnderTemp -Path $RejectedExtractRoot
Assert-UnderTemp -Path $DowngradeExtractRoot
Assert-UnderTemp -Path $SecurityFixtureRoot
Assert-UnderRepo -Path $DowngradePackageRoot
Assert-UnderRepo -Path $ShellSandbox
if (Test-Path -LiteralPath $InstallDir) {
    Remove-Item -LiteralPath $InstallDir -Recurse -Force
}

Push-Location $RepoRoot
try {
    $env:LOCALAPPDATA = Join-Path $ShellSandbox "local-app-data"
    $env:APPDATA = Join-Path $ShellSandbox "roaming-app-data"
    $env:CRYPTO_HUD_ALLOW_UNSIGNED_SMOKE = "1"
    New-Item -ItemType Directory -Force -Path $env:LOCALAPPDATA, $env:APPDATA | Out-Null
    New-Item -ItemType Directory -Force -Path $SecurityFixtureRoot | Out-Null

    Assert-OversizedSourceZipRejected

    Assert-UnsafeZipRejected `
        -Name "traversal" `
        -Entries @(@{ Name = "../outside.txt"; Content = "unsafe" }) `
        -ExpectedMessage "unsafe entry"
    Assert-UnsafeZipRejected `
        -Name "absolute" `
        -Entries @(@{ Name = "C:/outside.txt"; Content = "unsafe" }) `
        -ExpectedMessage "rooted entry"
    Assert-UnsafeZipRejected `
        -Name "ads" `
        -Entries @(@{ Name = "file.txt:payload"; Content = "unsafe" }) `
        -ExpectedMessage "Windows-unsafe entry"
    Assert-UnsafeZipRejected `
        -Name "duplicate" `
        -Entries @(
            @{ Name = "duplicate.txt"; Content = "first" },
            @{ Name = "duplicate.txt"; Content = "second" }
        ) `
        -ExpectedMessage "duplicate entry"
    Assert-UnsafeZipRejected `
        -Name "oversized" `
        -Entries @(@{ Name = "large.bin"; RepeatMegabytes = 129 }) `
        -ExpectedMessage "exceeds"

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
    & $PowerShellExe -NoProfile @packageArgs
    if ($LASTEXITCODE -ne 0) {
        throw "Package script failed with code $LASTEXITCODE"
    }

    & $PowerShellExe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\install-update-package.ps1" `
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

    & $PowerShellExe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\install-update-package.ps1" `
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
    if ([int]$manifest.manifestVersion -ne 2) {
        throw "Unexpected installed manifest version: $($manifest.manifestVersion)"
    }
    if ($manifest.version -ne $Version) {
        throw "Unexpected installed update version: $($manifest.version)"
    }

    $installedExe = Join-Path $InstallDir "crypto-hud.exe"
    Assert-Hash -Path $installedExe -ExpectedHash ([string]$manifest.executableSha256)
    $installedTaskbarDll = Join-Path $InstallDir "resources\taskbar\crypto_hud_taskbar.dll"
    if (-not (Test-Path -LiteralPath $installedTaskbarDll -PathType Leaf)) {
        throw "Installed update is missing the taskbar extension DLL"
    }
    if (-not (Test-Path -LiteralPath (Join-Path $InstallDir "install-update-package.ps1"))) {
        throw "Installed update handoff script was not copied"
    }
    foreach ($file in @($manifest.files)) {
        $installedPath = [System.IO.Path]::GetFullPath((Join-Path $InstallDir ([string]$file.path)))
        Assert-Hash -Path $installedPath -ExpectedHash ([string]$file.sha256)
    }

    if ($Version -match '^v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$' -and
        $DowngradeVersion -match '^v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$') {
        & $PowerShellExe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\package-windows.ps1" `
            -Version $DowngradeVersion `
            -SkipBuild `
            -AllowDirty `
            -AllowDevelopmentVersion `
            -AllowUnsignedPackage
        if ($LASTEXITCODE -ne 0) {
            throw "Downgrade fixture package failed with code $LASTEXITCODE"
        }
        & $PowerShellExe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\install-update-package.ps1" `
            -PackageZip $DowngradeZipPath `
            -ChecksumPath $DowngradeChecksumPath `
            -InstallDir $InstallDir `
            -ExtractRoot $DowngradeExtractRoot `
            -SkipShellIntegration `
            -AllowUnsignedPackage
        if ($LASTEXITCODE -eq 0) {
            throw "Update helper accepted a downgrade from $Version to $DowngradeVersion"
        }
        $retainedManifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
        if ([string]$retainedManifest.version -ne $Version) {
            throw "Rejected downgrade changed the installed release manifest"
        }
    }

    $sandboxLegacyDir = Join-Path $env:LOCALAPPDATA "CryptoWidget\CryptoHud"
    New-Item -ItemType Directory -Force -Path $sandboxLegacyDir | Out-Null
    $legacySentinel = Join-Path $sandboxLegacyDir "keep.txt"
    Set-Content -LiteralPath $legacySentinel -Value "keep"

    $uninstallScript = Join-Path $InstallDir "uninstall.ps1"
    & $PowerShellExe -NoProfile -ExecutionPolicy Bypass -File $uninstallScript -InstallDir $InstallDir -SkipShellIntegration
    if ($LASTEXITCODE -ne 0) {
        throw "Update uninstall smoke failed with code $LASTEXITCODE"
    }
    if (Test-Path -LiteralPath $installedExe) {
        throw "Update uninstall smoke left the executable behind"
    }
    if (-not (Test-Path -LiteralPath $legacySentinel)) {
        throw "SkipShellIntegration removed the isolated legacy installation"
    }

    Write-Host "Update smoke passed"
} finally {
    Pop-Location
    $env:LOCALAPPDATA = $OriginalLocalAppData
    $env:APPDATA = $OriginalAppData
    $env:CRYPTO_HUD_ALLOW_UNSIGNED_SMOKE = $OriginalUnsignedSmoke
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
    if (Test-Path -LiteralPath $DowngradeExtractRoot) {
        Assert-UnderTemp -Path $DowngradeExtractRoot
        Remove-Item -LiteralPath $DowngradeExtractRoot -Recurse -Force
    }
    if (Test-Path -LiteralPath $SecurityFixtureRoot) {
        Assert-UnderTemp -Path $SecurityFixtureRoot
        Remove-Item -LiteralPath $SecurityFixtureRoot -Recurse -Force
    }
    if (-not $KeepPackage) {
        foreach ($path in @($PackageRoot, $ZipPath, $ChecksumPath)) {
            if (Test-Path -LiteralPath $path) {
                Assert-UnderRepo -Path $path
                Remove-Item -LiteralPath $path -Recurse -Force
            }
        }
        foreach ($path in @($DowngradePackageRoot, $DowngradeZipPath, $DowngradeChecksumPath)) {
            if (Test-Path -LiteralPath $path) {
                Assert-UnderRepo -Path $path
                Remove-Item -LiteralPath $path -Recurse -Force
            }
        }
    }
    if (Test-Path -LiteralPath $ShellSandbox) {
        Assert-UnderRepo -Path $ShellSandbox
        Remove-Item -LiteralPath $ShellSandbox -Recurse -Force
    }
}
