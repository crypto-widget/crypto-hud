param(
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA "CryptoHud"),
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
Remove-Item -LiteralPath $InstallDir -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $LegacyInstallDir -Recurse -Force -ErrorAction SilentlyContinue

if ($RemoveUserData) {
    Remove-Item -LiteralPath (Join-Path $env:APPDATA "cryptohud\CryptoHud") -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath (Join-Path $env:APPDATA "cryptowidget\CryptoHud") -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath (Join-Path $env:APPDATA "cryptowidget\SlintPoc") -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host "Uninstalled Crypto HUD"
