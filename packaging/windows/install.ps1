param(
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA "CryptoHud"),
    [switch]$StartAfterInstall,
    [switch]$SkipShellIntegration,
    [switch]$AllowMissingManifest
)

$ErrorActionPreference = "Stop"

$PackageDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ExeName = "crypto-hud.exe"
$SourceExe = Join-Path $PackageDir $ExeName
$TargetExe = Join-Path $InstallDir $ExeName
$ManifestPath = Join-Path $PackageDir "release-manifest.json"
$StartMenuDir = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Crypto HUD"
$ShortcutPath = Join-Path $StartMenuDir "Crypto HUD.lnk"
$UninstallScript = Join-Path $InstallDir "uninstall.ps1"
$UpdateScript = Join-Path $InstallDir "install-update-package.ps1"
$UninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\CryptoHud"
$LegacyStartMenuDir = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Crypto Widget"
$LegacyShortcutPath = Join-Path $LegacyStartMenuDir "Crypto Widget.lnk"
$LegacyUninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\CryptoWidget.CryptoHud"

if (-not (Test-Path $SourceExe)) {
    throw "Package is missing $ExeName"
}

$version = "dev"
$manifest = $null
if (Test-Path $ManifestPath) {
    $manifest = Get-Content -LiteralPath $ManifestPath -Raw | ConvertFrom-Json
    if ([int]$manifest.manifestVersion -ne 1) {
        throw "Unsupported release manifest version: $($manifest.manifestVersion)"
    }
    if ($manifest.name -ne "crypto-hud") {
        throw "Unexpected package name: $($manifest.name)"
    }
    if ($manifest.target -ne "windows-x64") {
        throw "Unexpected package target: $($manifest.target)"
    }
    if ($manifest.executable -ne $ExeName) {
        throw "Manifest executable mismatch: $($manifest.executable)"
    }
    if ($manifest.version) {
        $version = [string]$manifest.version
    }
} elseif (-not $AllowMissingManifest) {
    throw "Package is missing release-manifest.json"
}

function Resolve-PackageRelativePath {
    param([string]$RelativePath)

    if ([string]::IsNullOrWhiteSpace($RelativePath)) {
        throw "Manifest file path is empty"
    }
    if ([System.IO.Path]::IsPathRooted($RelativePath) -or $RelativePath.Contains("..")) {
        throw "Manifest file path must stay inside the package: $RelativePath"
    }
    Join-Path $PackageDir $RelativePath
}

function Assert-FileHash {
    param(
        [string]$Path,
        [string]$ExpectedHash
    )

    if (-not (Test-Path $Path)) {
        throw "Package file is missing: $Path"
    }
    if ([string]::IsNullOrWhiteSpace($ExpectedHash)) {
        throw "Package manifest is missing SHA-256 for $Path"
    }
    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
    if ($actual -ne $ExpectedHash.ToLowerInvariant()) {
        throw "Package file hash mismatch for $Path"
    }
}

if ($manifest) {
    foreach ($file in @($manifest.files)) {
        $path = Resolve-PackageRelativePath -RelativePath ([string]$file.path)
        Assert-FileHash -Path $path -ExpectedHash ([string]$file.sha256)
    }
    Assert-FileHash -Path $SourceExe -ExpectedHash ([string]$manifest.executableSha256)
}

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Copy-Item -LiteralPath $SourceExe -Destination $TargetExe -Force
Copy-Item -LiteralPath (Join-Path $PackageDir "README.md") -Destination (Join-Path $InstallDir "README.md") -Force -ErrorAction SilentlyContinue
Copy-Item -LiteralPath $ManifestPath -Destination (Join-Path $InstallDir "release-manifest.json") -Force -ErrorAction SilentlyContinue
Copy-Item -LiteralPath (Join-Path $PackageDir "uninstall.ps1") -Destination $UninstallScript -Force
Copy-Item -LiteralPath (Join-Path $PackageDir "install-update-package.ps1") -Destination $UpdateScript -Force

if (-not $SkipShellIntegration) {
    Remove-Item -LiteralPath $LegacyShortcutPath -Force -ErrorAction SilentlyContinue
    if (Test-Path $LegacyStartMenuDir) {
        $remaining = Get-ChildItem -LiteralPath $LegacyStartMenuDir -Force -ErrorAction SilentlyContinue
        if (-not $remaining) {
            Remove-Item -LiteralPath $LegacyStartMenuDir -Force -ErrorAction SilentlyContinue
        }
    }
    Remove-Item -LiteralPath $LegacyUninstallKey -Recurse -Force -ErrorAction SilentlyContinue

    New-Item -ItemType Directory -Force -Path $StartMenuDir | Out-Null
    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut($ShortcutPath)
    $shortcut.TargetPath = $TargetExe
    $shortcut.WorkingDirectory = $InstallDir
    $shortcut.IconLocation = "$TargetExe,0"
    $shortcut.Save()

    New-Item -Path $UninstallKey -Force | Out-Null
    New-ItemProperty -Path $UninstallKey -Name "DisplayName" -Value "Crypto HUD" -PropertyType String -Force | Out-Null
    New-ItemProperty -Path $UninstallKey -Name "DisplayVersion" -Value $version -PropertyType String -Force | Out-Null
    New-ItemProperty -Path $UninstallKey -Name "Publisher" -Value "Crypto HUD Contributors" -PropertyType String -Force | Out-Null
    New-ItemProperty -Path $UninstallKey -Name "InstallLocation" -Value $InstallDir -PropertyType String -Force | Out-Null
    New-ItemProperty -Path $UninstallKey -Name "DisplayIcon" -Value $TargetExe -PropertyType String -Force | Out-Null
    New-ItemProperty -Path $UninstallKey -Name "NoModify" -Value 1 -PropertyType DWord -Force | Out-Null
    New-ItemProperty -Path $UninstallKey -Name "NoRepair" -Value 1 -PropertyType DWord -Force | Out-Null
    New-ItemProperty -Path $UninstallKey -Name "UninstallString" -Value "powershell -ExecutionPolicy Bypass -File `"$UninstallScript`"" -PropertyType String -Force | Out-Null
}

Write-Host "Installed Crypto HUD to $InstallDir"

if ($StartAfterInstall) {
    Start-Process -FilePath $TargetExe
}
