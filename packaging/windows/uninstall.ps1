param(
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA "CryptoHud"),
    [switch]$SkipShellIntegration,
    [switch]$RemoveUserData
)

$ErrorActionPreference = "Stop"

$StartMenuDir = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Crypto HUD"
$ShortcutPath = Join-Path $StartMenuDir "Crypto HUD.lnk"
$UninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\CryptoHud"
$LegacyStartMenuDir = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Crypto Widget"
$LegacyShortcutPath = Join-Path $LegacyStartMenuDir "Crypto Widget.lnk"
$LegacyUninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\CryptoWidget.CryptoHud"
$AutoStartRunKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
$AutoStartApprovalKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run"
$AutoStartValueNames = @("Crypto HUD", "Crypto Widget Slint")
$ScriptDirectory = [System.IO.Path]::GetFullPath((Split-Path -Parent $MyInvocation.MyCommand.Path))

function Assert-UnderDirectory {
    param(
        [string]$Path,
        [string]$Directory,
        [string]$Description
    )

    $fullPath = [System.IO.Path]::GetFullPath($Path).TrimEnd('\', '/')
    $fullDirectory = [System.IO.Path]::GetFullPath($Directory).TrimEnd('\', '/')
    $prefix = "$fullDirectory$([System.IO.Path]::DirectorySeparatorChar)"
    if (-not $fullPath.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to operate outside ${Description}: $fullPath"
    }
    $fullPath
}

function Assert-NoReparsePoints {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path)) {
        return
    }
    $root = Get-Item -LiteralPath $Path -Force
    if (($root.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "Refusing to remove a reparse point: $Path"
    }
    foreach ($item in Get-ChildItem -LiteralPath $Path -Recurse -Force) {
        if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "Refusing to remove a tree containing a reparse point: $($item.FullName)"
        }
    }
}

function Assert-CryptoHudInstallDirectory {
    param([string]$Path)

    $fullPath = [System.IO.Path]::GetFullPath($Path)
    $rootPath = [System.IO.Path]::GetPathRoot($fullPath)
    if ($fullPath.TrimEnd('\', '/') -eq $rootPath.TrimEnd('\', '/')) {
        throw "Refusing to uninstall from a filesystem root: $fullPath"
    }
    Assert-NoReparsePoints -Path $fullPath

    $expectedExe = Join-Path $fullPath "crypto-hud.exe"
    $manifestPath = Join-Path $fullPath "release-manifest.json"
    $integrityPath = Join-Path $fullPath "release-integrity.ps1"
    $validManifest = $false
    if (Test-Path -LiteralPath $manifestPath) {
        try {
            $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
            $validManifest =
                [int]$manifest.manifestVersion -eq 2 -and
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
            if ($validManifest -and (Test-Path -LiteralPath $integrityPath -PathType Leaf)) {
                $integrityContent = Get-Content -LiteralPath $integrityPath -Raw
                $integrityHashMatch = [regex]::Match(
                    $integrityContent,
                    '(?m)^# CryptoHud-Manifest-SHA256: ([a-fA-F0-9]{64})\r?$'
                )
                $integrityVersionMatch = [regex]::Match(
                    $integrityContent,
                    '(?m)^# CryptoHud-Version: ([0-9A-Za-z][0-9A-Za-z.-]{0,63})\r?$'
                )
                $manifestHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $manifestPath).Hash
                $uninstallEntry = @($manifest.files | Where-Object {
                    ([string]$_.path).Replace('\', '/') -eq "uninstall.ps1"
                })
                $validManifest =
                    $integrityHashMatch.Success -and
                    $integrityVersionMatch.Success -and
                    $integrityHashMatch.Groups[1].Value -eq $manifestHash -and
                    $integrityVersionMatch.Groups[1].Value -eq [string]$manifest.version -and
                    $uninstallEntry.Count -eq 1 -and
                    [string]$uninstallEntry[0].sha256 -match '^[a-fA-F0-9]{64}$' -and
                    (Get-FileHash -Algorithm SHA256 -LiteralPath $PSCommandPath).Hash -eq
                        [string]$uninstallEntry[0].sha256
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

function Remove-RegistryValueVerified {
    param(
        [string]$Path,
        [string]$Name
    )

    if (-not (Test-Path -LiteralPath $Path)) {
        return
    }
    $properties = Get-ItemProperty -LiteralPath $Path -ErrorAction Stop
    if ($properties.PSObject.Properties.Name -contains $Name) {
        Remove-ItemProperty -LiteralPath $Path -Name $Name -Force -ErrorAction Stop
    }
    $properties = Get-ItemProperty -LiteralPath $Path -ErrorAction Stop
    if ($properties.PSObject.Properties.Name -contains $Name) {
        throw "Failed to remove registry value '$Name' from $Path"
    }
}

function Remove-PathVerified {
    param(
        [string]$Path,
        [switch]$Recurse
    )

    if (-not (Test-Path -LiteralPath $Path)) {
        return
    }
    Remove-Item -LiteralPath $Path -Force -Recurse:$Recurse -ErrorAction Stop
    if (Test-Path -LiteralPath $Path) {
        throw "Failed to remove shell integration path: $Path"
    }
}

function Remove-EmptyDirectoryVerified {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path)) {
        return
    }
    $remaining = @(Get-ChildItem -LiteralPath $Path -Force -ErrorAction Stop)
    if ($remaining.Count -eq 0) {
        Remove-PathVerified -Path $Path
    }
}

$InstallDir = Assert-CryptoHudInstallDirectory -Path $InstallDir

# Remove the validated application tree before unregistering its recovery
# entry. If file locks or ACLs block deletion, the installed app remains
# discoverable and the script exits non-zero without a half-uninstalled shell.
Remove-Item -LiteralPath $InstallDir -Recurse -Force -ErrorAction Stop
if (Test-Path -LiteralPath $InstallDir) {
    throw "Failed to remove the Crypto HUD installation directory: $InstallDir"
}

if (-not $SkipShellIntegration) {
    $shellCleanupErrors = @()
    foreach ($valueName in $AutoStartValueNames) {
        foreach ($registryPath in @($AutoStartRunKey, $AutoStartApprovalKey)) {
            try {
                Remove-RegistryValueVerified -Path $registryPath -Name $valueName
            } catch {
                $shellCleanupErrors += $_.Exception.Message
            }
        }
    }

    foreach ($shortcut in @($ShortcutPath, $LegacyShortcutPath)) {
        try {
            Remove-PathVerified -Path $shortcut
        } catch {
            $shellCleanupErrors += $_.Exception.Message
        }
    }
    foreach ($startMenuDirectory in @($StartMenuDir, $LegacyStartMenuDir)) {
        try {
            Remove-EmptyDirectoryVerified -Path $startMenuDirectory
        } catch {
            $shellCleanupErrors += $_.Exception.Message
        }
    }

    foreach ($registryKey in @($UninstallKey, $LegacyUninstallKey)) {
        try {
            Remove-PathVerified -Path $registryKey -Recurse
        } catch {
            $shellCleanupErrors += $_.Exception.Message
        }
    }
    if ($shellCleanupErrors.Count -gt 0) {
        throw "Crypto HUD files were removed, but shell cleanup failed: $($shellCleanupErrors -join '; ')"
    }
}

if ($RemoveUserData) {
    foreach ($userDataPath in @(
        (Join-Path $env:APPDATA "cryptohud\CryptoHud"),
        (Join-Path $env:APPDATA "cryptowidget\CryptoHud"),
        (Join-Path $env:APPDATA "cryptowidget\SlintPoc")
    )) {
        $safeUserDataPath = Assert-UnderDirectory `
            -Path $userDataPath `
            -Directory $env:APPDATA `
            -Description "application data directory"
        Assert-NoReparsePoints -Path $safeUserDataPath
        if (Test-Path -LiteralPath $safeUserDataPath) {
            Remove-Item -LiteralPath $safeUserDataPath -Recurse -Force -ErrorAction Stop
            if (Test-Path -LiteralPath $safeUserDataPath) {
                throw "Failed to remove Crypto HUD user data: $safeUserDataPath"
            }
        }
    }
}

Write-Host "Uninstalled Crypto HUD"
