param(
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA "CryptoHud"),
    [switch]$SkipShellIntegration,
    [switch]$RemoveUserData
)

$ErrorActionPreference = "Stop"

$StartMenuDir = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Crypto HUD"
$ShortcutPath = Join-Path $StartMenuDir "Crypto HUD.lnk"
$UninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\CryptoHud"
$LegacyInstallDir = Join-Path $env:LOCALAPPDATA "CryptoWidget\CryptoHud"
$LegacyStartMenuDir = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Crypto Widget"
$LegacyShortcutPath = Join-Path $LegacyStartMenuDir "Crypto Widget.lnk"
$LegacyUninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\CryptoWidget.CryptoHud"
$AutoStartRunKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
$AutoStartApprovalKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run"
$AutoStartValueNames = @("Crypto HUD", "Crypto Widget Slint")
$ScriptDirectory = [System.IO.Path]::GetFullPath((Split-Path -Parent $MyInvocation.MyCommand.Path))

function Assert-CryptoHudInstallDirectory {
    param([string]$Path)

    $fullPath = [System.IO.Path]::GetFullPath($Path)
    $rootPath = [System.IO.Path]::GetPathRoot($fullPath)
    if ($fullPath.TrimEnd('\', '/') -eq $rootPath.TrimEnd('\', '/')) {
        throw "Refusing to uninstall from a filesystem root: $fullPath"
    }

    $expectedExe = Join-Path $fullPath "crypto-hud.exe"
    $manifestPath = Join-Path $fullPath "release-manifest.json"
    $validManifest = $false
    if (Test-Path -LiteralPath $manifestPath) {
        try {
            $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
            $validManifest =
                [int]$manifest.manifestVersion -eq 1 -and
                $manifest.name -eq "crypto-hud" -and
                $manifest.target -eq "windows-x64" -and
                $manifest.executable -eq "crypto-hud.exe" -and
                [string]$manifest.executableSha256 -match "^[a-fA-F0-9]{64}$"
            if ($validManifest -and (Test-Path -LiteralPath $expectedExe)) {
                $actualHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $expectedExe).Hash
                $validManifest = $actualHash -eq [string]$manifest.executableSha256
            } else {
                $validManifest = $false
            }
        } catch {
            $validManifest = $false
        }
    }

    if (-not $validManifest) {
        throw "Refusing to delete an installation without a valid Crypto HUD manifest and executable hash: $fullPath"
    }
    if ($ScriptDirectory.TrimEnd('\', '/') -ne $fullPath.TrimEnd('\', '/')) {
        throw "Refusing to uninstall a different directory from this script location: $fullPath"
    }

    return $fullPath
}

$InstallDir = Assert-CryptoHudInstallDirectory -Path $InstallDir

if (-not $SkipShellIntegration) {
    foreach ($valueName in $AutoStartValueNames) {
        Remove-ItemProperty -LiteralPath $AutoStartRunKey -Name $valueName -Force -ErrorAction SilentlyContinue
        Remove-ItemProperty -LiteralPath $AutoStartApprovalKey -Name $valueName -Force -ErrorAction SilentlyContinue
    }

    Remove-Item -LiteralPath $ShortcutPath -Force -ErrorAction SilentlyContinue
    if (Test-Path $StartMenuDir) {
        $remaining = Get-ChildItem -LiteralPath $StartMenuDir -Force -ErrorAction SilentlyContinue
        if (-not $remaining) {
            Remove-Item -LiteralPath $StartMenuDir -Force -ErrorAction SilentlyContinue
        }
    }
    Remove-Item -LiteralPath $LegacyShortcutPath -Force -ErrorAction SilentlyContinue
    if (Test-Path $LegacyStartMenuDir) {
        $remaining = Get-ChildItem -LiteralPath $LegacyStartMenuDir -Force -ErrorAction SilentlyContinue
        if (-not $remaining) {
            Remove-Item -LiteralPath $LegacyStartMenuDir -Force -ErrorAction SilentlyContinue
        }
    }

    Remove-Item -LiteralPath $UninstallKey -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $LegacyUninstallKey -Recurse -Force -ErrorAction SilentlyContinue
}

Remove-Item -LiteralPath $InstallDir -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $LegacyInstallDir -Recurse -Force -ErrorAction SilentlyContinue

if ($RemoveUserData) {
    Remove-Item -LiteralPath (Join-Path $env:APPDATA "cryptohud\CryptoHud") -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath (Join-Path $env:APPDATA "cryptowidget\CryptoHud") -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath (Join-Path $env:APPDATA "cryptowidget\SlintPoc") -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host "Uninstalled Crypto HUD"
