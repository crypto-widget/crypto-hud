param(
    [int]$TimeoutMs = 60000
)

$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$SmokeTempRoot = [System.IO.Path]::GetFullPath((Join-Path $RepoRoot "target\tmp"))
$StateDir = [System.IO.Path]::GetFullPath(
    (Join-Path $SmokeTempRoot "gui-plugin-hot-reload-smoke-state")
)
$PluginRoot = Join-Path $StateDir "plugins"
$PluginAId = "com.example.hot-reload-focus-ticker"
$PluginBId = "com.cryptohud.status-strip"
$PluginADir = Join-Path $PluginRoot $PluginAId
$FocusTickerSourceDir = Join-Path `
    $RepoRoot `
    "crates\crypto-hud\plugins\com.cryptohud.focus-ticker"
$FocusTickerSourceFile = Join-Path $FocusTickerSourceDir "ui\main.slint"
$ReadyFile = Join-Path $StateDir "ready.json"
$StateFile = Join-Path $StateDir "layouts.json"
$StdoutFile = Join-Path $StateDir "stdout.log"
$StderrFile = Join-Path $StateDir "stderr.log"
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
$OriginalCardBackgroundLine = `
    "    property <color> card-background: root.light-theme ? #f7fcfff6 : #0d1826f6;"

function Assert-PathInside([string]$Path, [string]$Root, [string]$Label) {
    $fullPath = [System.IO.Path]::GetFullPath($Path)
    $fullRoot = [System.IO.Path]::GetFullPath($Root).TrimEnd('\') + '\'
    if (-not $fullPath.StartsWith(
        $fullRoot,
        [System.StringComparison]::OrdinalIgnoreCase
    )) {
        throw "$Label path is outside the expected root: $fullPath"
    }
}

Assert-PathInside $StateDir $SmokeTempRoot "Smoke state"
Assert-PathInside $PluginADir $StateDir "Plugin A"

if (-not (Test-Path -LiteralPath $FocusTickerSourceFile -PathType Leaf)) {
    throw "Focus Ticker source fixture is missing: $FocusTickerSourceFile"
}
$FocusTickerSourceTemplate = Get-Content -LiteralPath $FocusTickerSourceFile -Raw
if ([regex]::Matches(
        $FocusTickerSourceTemplate,
        [regex]::Escape($OriginalCardBackgroundLine)
    ).Count -ne 1) {
    throw "Focus Ticker card-background fixture line changed unexpectedly"
}

if (Test-Path $StateDir) {
    Remove-Item -LiteralPath $StateDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $PluginRoot | Out-Null

function Write-Utf8File([string]$Path, [string]$Contents) {
    $parent = Split-Path -Parent $Path
    if ($parent) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }
    [System.IO.File]::WriteAllText($Path, $Contents, $utf8NoBom)
}

function Write-FocusTickerRevision(
    [string]$DarkColor,
    [string]$LightColor,
    [string]$Revision
) {
    $replacement = `
        "    property <color> card-background: root.light-theme ? $LightColor : $DarkColor;"
    $source = $FocusTickerSourceTemplate.Replace(
        $OriginalCardBackgroundLine,
        $replacement
    ).TrimEnd()
    $source += "`r`n// hot-reload-smoke revision: $Revision`r`n"
    Write-Utf8File (Join-Path $PluginADir "ui\main.slint") $source
}

function Copy-FocusTickerPlugin(
    [string]$DarkColor,
    [string]$LightColor,
    [string]$Revision
) {
    if (Test-Path -LiteralPath $PluginADir) {
        Remove-Item -LiteralPath $PluginADir -Recurse -Force
    }
    Copy-Item -LiteralPath $FocusTickerSourceDir -Destination $PluginADir -Recurse

    $manifestPath = Join-Path $PluginADir "widget.json"
    $manifest = Get-Content -LiteralPath $manifestPath -Raw
    $originalId = '"id": "com.cryptohud.focus-ticker"'
    $originalName = '"name": "Focus Ticker"'
    if ([regex]::Matches($manifest, [regex]::Escape($originalId)).Count -ne 1 -or
        [regex]::Matches($manifest, [regex]::Escape($originalName)).Count -ne 1) {
        throw "Focus Ticker manifest fixture changed unexpectedly"
    }
    $manifest = $manifest.Replace($originalId, '"id": "' + $PluginAId + '"')
    $manifest = $manifest.Replace(
        $originalName,
        '"name": "Focus Ticker Hot Reload E2E"'
    )
    Write-Utf8File $manifestPath $manifest
    Write-FocusTickerRevision $DarkColor $LightColor $Revision
}

function Write-InvalidPluginSource {
    Write-Utf8File `
        (Join-Path $PluginADir "ui\main.slint") `
        "export component FocusTicker inherits Window { this is not valid Slint"
}

Copy-FocusTickerPlugin "#4c1622f6" "#ffe4e8f6" "a-v1"

$seedWidgets = @(
    [ordered]@{
        id = "hot-reload-a-1"
        plugin_id = $PluginAId
        name = "Focus Ticker Reload Dark"
        visible = $true
        layout = [ordered]@{
            x = 60
            y = 80
            always_on_top = $true
            opacity_percent = 100
            locked = $true
            scale_percent = 100
            width = 820
            height = 156
        }
        symbols = @("binance:spot:BTC/USDT")
        config = [ordered]@{
            marker = 11
            theme = "dark"
        }
    },
    [ordered]@{
        id = "hot-reload-a-2"
        plugin_id = $PluginAId
        name = "Focus Ticker Reload Light"
        visible = $true
        layout = [ordered]@{
            x = 60
            y = 260
            always_on_top = $true
            opacity_percent = 100
            locked = $true
            scale_percent = 100
            width = 820
            height = 156
        }
        symbols = @("binance:spot:BTC/USDT")
        config = [ordered]@{
            marker = 22
            theme = "light"
        }
    },
    [ordered]@{
        id = "hot-reload-b-1"
        plugin_id = $PluginBId
        name = "Status Strip Stable Control"
        visible = $true
        layout = [ordered]@{
            x = 920
            y = 80
            always_on_top = $true
            opacity_percent = 100
            locked = $true
            scale_percent = 100
            width = 374
            height = 92
        }
        symbols = @(
            "binance:spot:BTC/USDT",
            "binance:spot:ETH/USDT",
            "binance:spot:SOL/USDT"
        )
        config = [ordered]@{
            marker = 33
            theme = "dark"
        }
    }
)
$seedState = [ordered]@{
    schema_version = 1
    settings = [ordered]@{
        widgets_always_on_top = $false
        opacity_percent = 100
        widget_scale_percent = 100
        theme = "dark"
        shortcut = "disabled"
        tray_icon_enabled = $false
        auto_start_enabled = $false
        show_main_window_on_startup = $false
    }
    selected_widget_id = "hot-reload-a-1"
    next_widget_number = 4
    widgets = $seedWidgets
}
Write-Utf8File $StateFile ($seedState | ConvertTo-Json -Depth 8)

Add-Type -AssemblyName System.Drawing

if (-not ("CryptoHudGuiPluginHotReloadSmokeWin32" -as [type])) {
    Add-Type @'
using System;
using System.Runtime.InteropServices;
using System.Text;

public static class CryptoHudGuiPluginHotReloadSmokeWin32 {
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);

    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr hWnd, StringBuilder lpString, int nMaxCount);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdcBlt, uint nFlags);

    [StructLayout(LayoutKind.Sequential)]
    public struct RECT {
        public int Left;
        public int Top;
        public int Right;
        public int Bottom;
    }

    public const uint PW_RENDERFULLCONTENT = 0x00000002;
}
'@
}

function Get-ProcessWindows([int]$ProcessId) {
    $handles = [System.Collections.Generic.List[IntPtr]]::new()
    $callback = [CryptoHudGuiPluginHotReloadSmokeWin32+EnumWindowsProc]{
        param([IntPtr]$WindowHandle, [IntPtr]$Param)

        [uint32]$windowProcessId = 0
        [void][CryptoHudGuiPluginHotReloadSmokeWin32]::GetWindowThreadProcessId(
            $WindowHandle,
            [ref]$windowProcessId
        )
        if ($windowProcessId -eq [uint32]$ProcessId -and
            [CryptoHudGuiPluginHotReloadSmokeWin32]::IsWindowVisible($WindowHandle)) {
            $handles.Add($WindowHandle)
        }
        return $true
    }
    [void][CryptoHudGuiPluginHotReloadSmokeWin32]::EnumWindows($callback, [IntPtr]::Zero)

    foreach ($handle in $handles) {
        $title = [System.Text.StringBuilder]::new(256)
        [void][CryptoHudGuiPluginHotReloadSmokeWin32]::GetWindowText($handle, $title, 256)
        $rect = New-Object CryptoHudGuiPluginHotReloadSmokeWin32+RECT
        [void][CryptoHudGuiPluginHotReloadSmokeWin32]::GetWindowRect($handle, [ref]$rect)
        [pscustomobject]@{
            Handle = $handle.ToInt64()
            Title = $title.ToString()
            Left = $rect.Left
            Top = $rect.Top
            Width = $rect.Right - $rect.Left
            Height = $rect.Bottom - $rect.Top
        }
    }
}

function Get-UniqueWidgetWindow([int]$ProcessId, [string]$Title) {
    $matches = @(Get-ProcessWindows $ProcessId | Where-Object { $_.Title -eq $Title })
    if ($matches.Count -eq 1) {
        return $matches[0]
    }
    return $null
}

function Wait-ForFile([string]$Path, [int]$TimeoutMilliseconds) {
    $deadline = (Get-Date).AddMilliseconds($TimeoutMilliseconds)
    while (-not (Test-Path -LiteralPath $Path)) {
        if ((Get-Date) -gt $deadline) {
            throw "Timed out waiting for $Path"
        }
        Start-Sleep -Milliseconds 100
    }
}

function Wait-ForWidgetWindow([int]$ProcessId, [string]$Title, [int]$TimeoutMilliseconds) {
    $deadline = (Get-Date).AddMilliseconds($TimeoutMilliseconds)
    while ((Get-Date) -le $deadline) {
        $window = Get-UniqueWidgetWindow $ProcessId $Title
        if ($window) {
            return $window
        }
        Start-Sleep -Milliseconds 100
    }
    throw "Timed out waiting for the unique $Title window"
}

function Wait-ForReplacedAWindows(
    [int]$ProcessId,
    [long]$OldA1Handle,
    [long]$OldA2Handle,
    [int]$TimeoutMilliseconds
) {
    $deadline = (Get-Date).AddMilliseconds($TimeoutMilliseconds)
    while ((Get-Date) -le $deadline) {
        $a1 = Get-UniqueWidgetWindow $ProcessId "hot-reload-a-1"
        $a2 = Get-UniqueWidgetWindow $ProcessId "hot-reload-a-2"
        if ($a1 -and $a2 -and
            [long]$a1.Handle -ne $OldA1Handle -and
            [long]$a2.Handle -ne $OldA2Handle) {
            return [pscustomobject]@{ A1 = $a1; A2 = $a2 }
        }
        Start-Sleep -Milliseconds 100
    }
    throw "Timed out waiting for both A widget instances to be replaced"
}

function Wait-ForAWindowsAbsent([int]$ProcessId, [int]$TimeoutMilliseconds) {
    $deadline = (Get-Date).AddMilliseconds($TimeoutMilliseconds)
    while ((Get-Date) -le $deadline) {
        $windows = @(Get-ProcessWindows $ProcessId)
        $aWindows = @($windows | Where-Object {
            $_.Title -eq "hot-reload-a-1" -or $_.Title -eq "hot-reload-a-2"
        })
        if ($aWindows.Count -eq 0) {
            return
        }
        Start-Sleep -Milliseconds 100
    }
    throw "Timed out waiting for A widget windows to disappear"
}

function Get-LogText {
    if (-not (Test-Path -LiteralPath $StderrFile)) {
        return ""
    }
    $stream = [System.IO.FileStream]::new(
        $StderrFile,
        [System.IO.FileMode]::Open,
        [System.IO.FileAccess]::Read,
        [System.IO.FileShare]::ReadWrite
    )
    $reader = [System.IO.StreamReader]::new($stream)
    try {
        return $reader.ReadToEnd()
    } finally {
        $reader.Dispose()
        $stream.Dispose()
    }
}

function Get-AppliedReloadCount {
    return [regex]::Matches((Get-LogText), "applied plugin reload generation").Count
}

function Wait-ForAppliedReloadCount([int]$MinimumCount, [int]$TimeoutMilliseconds) {
    $deadline = (Get-Date).AddMilliseconds($TimeoutMilliseconds)
    while ((Get-Date) -le $deadline) {
        if ((Get-AppliedReloadCount) -ge $MinimumCount) {
            return
        }
        Start-Sleep -Milliseconds 100
    }
    throw "Timed out waiting for plugin reload $MinimumCount. Log: $(Get-LogText)"
}

function Get-LastReloadLogLine {
    return @((Get-LogText) -split "`r?`n" | Where-Object {
        $_ -like "*applied plugin reload generation*"
    })[-1]
}

function Assert-LastReloadLog([string]$Pattern) {
    $line = Get-LastReloadLogLine
    if ($line -notmatch $Pattern) {
        throw "Latest reload log did not match '$Pattern': $line"
    }
}

function Assert-SameHandle([object]$Actual, [object]$Expected, [string]$Label) {
    if ([long]$Actual.Handle -ne [long]$Expected.Handle) {
        throw "$Label HWND changed from $($Expected.Handle) to $($Actual.Handle)"
    }
}

function Assert-SameGeometry([object]$Actual, [object]$Expected, [string]$Label) {
    foreach ($property in @("Left", "Top", "Width", "Height")) {
        if ([int]$Actual.$property -ne [int]$Expected.$property) {
            throw "$Label $property changed from $($Expected.$property) to $($Actual.$property)"
        }
    }
}

function Capture-WindowVisual([object]$Window, [string]$CaptureName) {
    $bitmap = [System.Drawing.Bitmap]::new(
        [Math]::Max(1, [int]$Window.Width),
        [Math]::Max(1, [int]$Window.Height)
    )
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $deviceContext = $graphics.GetHdc()
    try {
        $printed = [CryptoHudGuiPluginHotReloadSmokeWin32]::PrintWindow(
            [IntPtr][long]$Window.Handle,
            $deviceContext,
            [CryptoHudGuiPluginHotReloadSmokeWin32]::PW_RENDERFULLCONTENT
        )
    } finally {
        $graphics.ReleaseHdc($deviceContext)
    }
    if (-not $printed) {
        $graphics.CopyFromScreen(
            [int]$Window.Left,
            [int]$Window.Top,
            0,
            0,
            $bitmap.Size
        )
    }
    $graphics.Dispose()

    if ($CaptureName) {
        $capturePath = Join-Path $StateDir "$CaptureName.png"
        $bitmap.Save($capturePath, [System.Drawing.Imaging.ImageFormat]::Png)
    }

    $probeRed = 0
    $probeGreen = 0
    $probeBlue = 0
    $probeSamples = 0
    $probeY = [Math]::Min(
        $bitmap.Height - 1,
        [Math]::Max(0, [int]($bitmap.Height * 0.20))
    )
    foreach ($xRatio in @(0.28, 0.38, 0.58, 0.70)) {
        $x = [Math]::Min(
            $bitmap.Width - 1,
            [Math]::Max(0, [int]($bitmap.Width * $xRatio))
        )
        $pixel = $bitmap.GetPixel($x, $probeY)
        $probeRed += [int]$pixel.R
        $probeGreen += [int]$pixel.G
        $probeBlue += [int]$pixel.B
        $probeSamples += 1
    }

    $uniqueColors = [System.Collections.Generic.HashSet[string]]::new()
    $minimumLuminance = 255
    $maximumLuminance = 0
    $edgeTransitions = 0
    $xStep = [Math]::Max(1, [int][Math]::Floor($bitmap.Width / 96.0))
    $yStep = [Math]::Max(1, [int][Math]::Floor($bitmap.Height / 48.0))
    for ($y = 0; $y -lt $bitmap.Height; $y += $yStep) {
        $previousPixel = $null
        for ($x = 0; $x -lt $bitmap.Width; $x += $xStep) {
            $pixel = $bitmap.GetPixel($x, $y)
            $bucket = "{0}:{1}:{2}" -f `
                [int]([int]$pixel.R / 16), `
                [int]([int]$pixel.G / 16), `
                [int]([int]$pixel.B / 16)
            [void]$uniqueColors.Add($bucket)
            $luminance = [int](
                (299 * [int]$pixel.R +
                    587 * [int]$pixel.G +
                    114 * [int]$pixel.B) / 1000
            )
            $minimumLuminance = [Math]::Min($minimumLuminance, $luminance)
            $maximumLuminance = [Math]::Max($maximumLuminance, $luminance)
            if ($null -ne $previousPixel) {
                $distance =
                    [Math]::Abs([int]$pixel.R - [int]$previousPixel.R) +
                    [Math]::Abs([int]$pixel.G - [int]$previousPixel.G) +
                    [Math]::Abs([int]$pixel.B - [int]$previousPixel.B)
                if ($distance -ge 24) {
                    $edgeTransitions += 1
                }
            }
            $previousPixel = $pixel
        }
    }
    $bitmap.Dispose()

    return [pscustomobject]@{
        ProbeR = [int]($probeRed / $probeSamples)
        ProbeG = [int]($probeGreen / $probeSamples)
        ProbeB = [int]($probeBlue / $probeSamples)
        UniqueColorBuckets = $uniqueColors.Count
        LuminanceRange = $maximumLuminance - $minimumLuminance
        EdgeTransitions = $edgeTransitions
    }
}

function Assert-RealPluginVisual([object]$Capture, [string]$Label) {
    if ([int]$Capture.UniqueColorBuckets -lt 12 -or
        [int]$Capture.LuminanceRange -lt 30 -or
        [int]$Capture.EdgeTransitions -lt 24) {
        throw "$Label looked like a flat-color fixture: $($Capture | ConvertTo-Json -Compress)"
    }
}

function Assert-FocusTickerRevision(
    [object]$Window,
    [int]$ExpectedRed,
    [int]$ExpectedGreen,
    [int]$ExpectedBlue,
    [string]$CaptureName
) {
    $deadline = (Get-Date).AddSeconds(5)
    $actual = $null
    while ((Get-Date) -le $deadline) {
        $actual = Capture-WindowVisual $Window $CaptureName
        $distance =
            [Math]::Abs([int]$actual.ProbeR - $ExpectedRed) +
            [Math]::Abs([int]$actual.ProbeG - $ExpectedGreen) +
            [Math]::Abs([int]$actual.ProbeB - $ExpectedBlue)
        if ([int]$actual.UniqueColorBuckets -ge 12 -and
            [int]$actual.LuminanceRange -ge 30 -and
            [int]$actual.EdgeTransitions -ge 24 -and
            $distance -le 75) {
            return
        }
        Start-Sleep -Milliseconds 100
    }
    throw "$CaptureName did not render the expected real Focus Ticker revision: $($actual | ConvertTo-Json -Compress)"
}

$env:CRYPTO_HUD_STATE_DIR = $StateDir
$env:CRYPTO_HUD_GUI_SMOKE_READY_FILE = $ReadyFile
$env:CRYPTO_HUD_INSTANCE_ID = "com.crypto-hud.gui-plugin-hot-reload-smoke.$PID"
$env:CRYPTO_HUD_GUI_SMOKE_OFFLINE = "1"
$env:CRYPTO_HUD_DISABLE_UPDATE_CHECK = "1"
$env:SLINT_BACKEND = "software"

Push-Location $RepoRoot
try {
    cargo build -p crypto-hud
    if ($LASTEXITCODE -ne 0) {
        throw "GUI plugin hot reload smoke build exited with code $LASTEXITCODE"
    }

    $app = Start-Process `
        -FilePath (Join-Path $RepoRoot "target\debug\crypto-hud.exe") `
        -ArgumentList @("--widgets", "3", "--gui-smoke-ms", "$TimeoutMs") `
        -RedirectStandardOutput $StdoutFile `
        -RedirectStandardError $StderrFile `
        -PassThru
    try {
        Wait-ForFile $ReadyFile 15000
        $ready = Get-Content -LiteralPath $ReadyFile -Raw | ConvertFrom-Json
        if (-not [bool]$ready.ready -or -not [bool]$ready.marketDataReady) {
            throw "GUI plugin hot reload smoke marker did not report ready market data"
        }
        if ([int]$ready.widgetCount -ne 3) {
            throw "Expected 3 widget runtimes, saw $($ready.widgetCount)"
        }
        foreach ($pluginId in @($PluginAId, $PluginBId)) {
            if (@($ready.pluginIds) -notcontains $pluginId) {
                throw "Ready marker did not contain plugin $pluginId"
            }
        }
        $readyWidgets = @($ready.widgets)
        if ($readyWidgets.Count -ne [int]$ready.widgetCount) {
            throw "Ready marker widget details did not match widgetCount"
        }
        foreach ($widget in $readyWidgets) {
            if ([int]$widget.marketDataRowCount -le 0 -or
                [int]$widget.marketDataRowCount -ne [int]$widget.symbolCount) {
                throw "Widget $($widget.id) received $($widget.marketDataRowCount) market rows for $($widget.symbolCount) symbols"
            }
        }
        if (@($ready.catalogErrors).Count -ne 0) {
            throw "Initial plugin catalog errors: $($ready.catalogErrors -join '; ')"
        }

        $baselineA1 = Wait-ForWidgetWindow $app.Id "hot-reload-a-1" 10000
        $baselineA2 = Wait-ForWidgetWindow $app.Id "hot-reload-a-2" 10000
        $baselineB1 = Wait-ForWidgetWindow $app.Id "hot-reload-b-1" 10000
        Assert-FocusTickerRevision $baselineA1 76 22 34 "initial-a1-dark"
        Assert-FocusTickerRevision $baselineA2 255 228 232 "initial-a2-light"
        $baselineBVisual = Capture-WindowVisual $baselineB1 "stable-b1-status-strip"
        Assert-RealPluginVisual $baselineBVisual "stable-b1-status-strip"
        $baselineStateHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $StateFile).Hash

        $reloadCount = Get-AppliedReloadCount
        Write-InvalidPluginSource
        Wait-ForAppliedReloadCount ($reloadCount + 1) 10000
        Assert-LastReloadLog "0 plugins changed, 0 instances replaced, 1 last-known-good plugins retained"
        $reloadCount += 1

        $invalidA1 = Wait-ForWidgetWindow $app.Id "hot-reload-a-1" 3000
        $invalidA2 = Wait-ForWidgetWindow $app.Id "hot-reload-a-2" 3000
        $invalidB1 = Wait-ForWidgetWindow $app.Id "hot-reload-b-1" 3000
        Assert-SameHandle $invalidA1 $baselineA1 "A1 after invalid save"
        Assert-SameHandle $invalidA2 $baselineA2 "A2 after invalid save"
        Assert-SameHandle $invalidB1 $baselineB1 "B1 after invalid save"
        Assert-SameGeometry $invalidA1 $baselineA1 "A1 after invalid save"
        Assert-SameGeometry $invalidA2 $baselineA2 "A2 after invalid save"
        Assert-SameGeometry $invalidB1 $baselineB1 "B1 after invalid save"
        Assert-FocusTickerRevision $invalidA1 76 22 34 "invalid-a1-dark-lkg"
        Assert-FocusTickerRevision $invalidA2 255 228 232 "invalid-a2-light-lkg"

        Write-FocusTickerRevision "#5a2a0cf6" "#ffedd5f6" "a-v2"
        Start-Sleep -Milliseconds 900
        Write-FocusTickerRevision "#0d3b2af6" "#dcfce7f6" "a-v3"
        $latestA = Wait-ForReplacedAWindows `
            $app.Id `
            ([long]$baselineA1.Handle) `
            ([long]$baselineA2.Handle) `
            12000
        Assert-FocusTickerRevision $latestA.A1 13 59 42 "latest-a1-dark"
        Assert-FocusTickerRevision $latestA.A2 220 252 231 "latest-a2-light"
        Wait-ForAppliedReloadCount ($reloadCount + 1) 3000
        Start-Sleep -Milliseconds 1800
        $actualReloadCount = Get-AppliedReloadCount
        if ($actualReloadCount -ne ($reloadCount + 1)) {
            throw "Rapid saves applied an intermediate generation; expected $($reloadCount + 1) reloads, saw $actualReloadCount"
        }
        Assert-LastReloadLog "1 plugins changed, 2 instances replaced, 0 last-known-good plugins retained"
        $reloadCount += 1

        $latestB1 = Wait-ForWidgetWindow $app.Id "hot-reload-b-1" 3000
        Assert-SameHandle $latestB1 $baselineB1 "B1 after A rapid saves"
        Assert-SameGeometry $latestA.A1 $baselineA1 "A1 after rapid saves"
        Assert-SameGeometry $latestA.A2 $baselineA2 "A2 after rapid saves"
        Assert-SameGeometry $latestB1 $baselineB1 "B1 after A rapid saves"

        Remove-Item -LiteralPath $PluginADir -Recurse -Force
        Wait-ForAppliedReloadCount ($reloadCount + 1) 10000
        Wait-ForAWindowsAbsent $app.Id 5000
        Assert-LastReloadLog "1 plugins changed, 0 instances replaced, 0 last-known-good plugins retained"
        $reloadCount += 1
        $deletedB1 = Wait-ForWidgetWindow $app.Id "hot-reload-b-1" 3000
        Assert-SameHandle $deletedB1 $baselineB1 "B1 after deleting A"
        Assert-SameGeometry $deletedB1 $baselineB1 "B1 after deleting A"

        Copy-FocusTickerPlugin "#3b1b5af6" "#f3e8fff6" "a-v4"
        Wait-ForAppliedReloadCount ($reloadCount + 1) 10000
        Assert-LastReloadLog "1 plugins changed, 2 instances replaced, 0 last-known-good plugins retained"
        $recreatedA1 = Wait-ForWidgetWindow $app.Id "hot-reload-a-1" 5000
        $recreatedA2 = Wait-ForWidgetWindow $app.Id "hot-reload-a-2" 5000
        $recreatedB1 = Wait-ForWidgetWindow $app.Id "hot-reload-b-1" 3000
        Assert-FocusTickerRevision $recreatedA1 59 27 90 "recreated-a1-dark"
        Assert-FocusTickerRevision $recreatedA2 243 232 255 "recreated-a2-light"
        Assert-SameGeometry $recreatedA1 $baselineA1 "A1 after recreation"
        Assert-SameGeometry $recreatedA2 $baselineA2 "A2 after recreation"
        Assert-SameHandle $recreatedB1 $baselineB1 "B1 after recreating A"
        Assert-SameGeometry $recreatedB1 $baselineB1 "B1 after recreating A"

        $finalStateHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $StateFile).Hash
        if ($finalStateHash -ne $baselineStateHash) {
            throw "Plugin hot reload mutated layouts.json"
        }
        $finalState = Get-Content -LiteralPath $StateFile -Raw | ConvertFrom-Json
        $markers = @{}
        foreach ($widget in @($finalState.widgets)) {
            $markers[[string]$widget.id] = [int]$widget.config.marker
        }
        if ($markers["hot-reload-a-1"] -ne 11 -or
            $markers["hot-reload-a-2"] -ne 22 -or
            $markers["hot-reload-b-1"] -ne 33) {
            throw "Plugin hot reload did not preserve widget configuration"
        }
        $themes = @{}
        foreach ($widget in @($finalState.widgets)) {
            $themes[[string]$widget.id] = [string]$widget.config.theme
        }
        if ($themes["hot-reload-a-1"] -ne "dark" -or
            $themes["hot-reload-a-2"] -ne "light" -or
            $themes["hot-reload-b-1"] -ne "dark") {
            throw "Plugin hot reload did not preserve widget theme configuration"
        }
    } finally {
        if ($app -and -not $app.HasExited) {
            Stop-Process -Id $app.Id -Force
        }
    }
} finally {
    Pop-Location
    Remove-Item Env:\CRYPTO_HUD_STATE_DIR -ErrorAction SilentlyContinue
    Remove-Item Env:\CRYPTO_HUD_GUI_SMOKE_READY_FILE -ErrorAction SilentlyContinue
    Remove-Item Env:\CRYPTO_HUD_INSTANCE_ID -ErrorAction SilentlyContinue
    Remove-Item Env:\CRYPTO_HUD_GUI_SMOKE_OFFLINE -ErrorAction SilentlyContinue
    Remove-Item Env:\CRYPTO_HUD_DISABLE_UPDATE_CHECK -ErrorAction SilentlyContinue
}
