param(
    [int]$TimeoutMs = 10000
)

$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$StateDir = Join-Path $RepoRoot "target\tmp\gui-widget-scale-smoke-state"
$ReadyFile = Join-Path $StateDir "ready.json"
$StateFile = Join-Path $StateDir "layouts.json"

if (Test-Path $StateDir) {
    Remove-Item -LiteralPath $StateDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $StateDir | Out-Null

$seedWidgets = @(
    [ordered]@{
        id = "quote-board-1"
        plugin_id = "builtin.quote-board"
        name = "Quote Board 1"
        visible = $true
        layout = [ordered]@{
            x = 96
            y = 96
            always_on_top = $false
            opacity_percent = 96
            locked = $false
            scale_percent = 100
            width = 286
            height = 101
        }
        symbols = @("BTC", "ETH")
        config = [ordered]@{
            show_coin_logos = $true
            hide_quote_asset = $false
        }
    }
)

$seedState = [ordered]@{
    settings = [ordered]@{
        widgets_always_on_top = $false
        opacity_percent = 96
        widget_scale_percent = 100
    }
    selected_widget_id = "quote-board-1"
    next_widget_number = 2
    widgets = $seedWidgets
}
$seedJson = $seedState | ConvertTo-Json -Depth 8
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($StateFile, $seedJson, $utf8NoBom)

Add-Type -AssemblyName System.Drawing

if (-not ("CryptoHudGuiScaleSmokeWin32" -as [type])) {
    Add-Type @'
using System;
using System.Runtime.InteropServices;
using System.Text;

public static class CryptoHudGuiScaleSmokeWin32 {
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);

    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr hWnd, StringBuilder lpString, int nMaxCount);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr hWnd, ref POINT lpPoint);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int X, int Y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint dwFlags, uint dx, uint dy, uint dwData, UIntPtr dwExtraInfo);

    [StructLayout(LayoutKind.Sequential)]
    public struct RECT {
        public int Left;
        public int Top;
        public int Right;
        public int Bottom;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct POINT {
        public int X;
        public int Y;
    }

    public const int SW_RESTORE = 9;
    public const uint MOUSEEVENTF_LEFTDOWN = 0x0002;
    public const uint MOUSEEVENTF_LEFTUP = 0x0004;
}
'@
}

function Get-ProcessWindows([int]$ProcessId) {
    $handles = [System.Collections.Generic.List[IntPtr]]::new()
    $callback = [CryptoHudGuiScaleSmokeWin32+EnumWindowsProc]{
        param([IntPtr]$WindowHandle, [IntPtr]$Param)

        [uint32]$windowProcessId = 0
        [void][CryptoHudGuiScaleSmokeWin32]::GetWindowThreadProcessId($WindowHandle, [ref]$windowProcessId)
        if ($windowProcessId -eq [uint32]$ProcessId -and [CryptoHudGuiScaleSmokeWin32]::IsWindowVisible($WindowHandle)) {
            $handles.Add($WindowHandle)
        }
        return $true
    }
    [void][CryptoHudGuiScaleSmokeWin32]::EnumWindows($callback, [IntPtr]::Zero)

    foreach ($handle in $handles) {
        $title = [System.Text.StringBuilder]::new(256)
        [void][CryptoHudGuiScaleSmokeWin32]::GetWindowText($handle, $title, 256)
        $rect = New-Object CryptoHudGuiScaleSmokeWin32+RECT
        [void][CryptoHudGuiScaleSmokeWin32]::GetWindowRect($handle, [ref]$rect)
        [pscustomobject]@{
            Handle = $handle
            Title = $title.ToString()
            Left = $rect.Left
            Top = $rect.Top
            Width = $rect.Right - $rect.Left
            Height = $rect.Bottom - $rect.Top
        }
    }
}

function Wait-ForFile([string]$Path, [int]$TimeoutMilliseconds) {
    $deadline = (Get-Date).AddMilliseconds($TimeoutMilliseconds)
    while (-not (Test-Path $Path)) {
        if ((Get-Date) -gt $deadline) {
            throw "Timed out waiting for $Path"
        }
        Start-Sleep -Milliseconds 100
    }
}

function Wait-ForWidgetState([int]$ExpectedWidth, [int]$ExpectedHeight, [int]$ExpectedScale) {
    $deadline = (Get-Date).AddSeconds(5)
    do {
        $state = Get-Content -LiteralPath $StateFile -Raw | ConvertFrom-Json
        $widget = @($state.widgets)[0]
        if (
            [int]$widget.layout.width -eq $ExpectedWidth -and
            [int]$widget.layout.height -eq $ExpectedHeight -and
            [int]$widget.layout.scale_percent -eq $ExpectedScale
        ) {
            return $widget
        }
        Start-Sleep -Milliseconds 100
    } while ((Get-Date) -lt $deadline)

    throw "Widget state did not reach ${ExpectedWidth}x${ExpectedHeight} at ${ExpectedScale}%"
}

function Click-ClientPoint([IntPtr]$WindowHandle, [int]$X, [int]$Y, [string]$Label) {
    $point = New-Object CryptoHudGuiScaleSmokeWin32+POINT
    $point.X = $X
    $point.Y = $Y
    [void][CryptoHudGuiScaleSmokeWin32]::ClientToScreen($WindowHandle, [ref]$point)
    [void][CryptoHudGuiScaleSmokeWin32]::SetCursorPos($point.X, $point.Y)
    Start-Sleep -Milliseconds 80
    [CryptoHudGuiScaleSmokeWin32]::mouse_event(
        [CryptoHudGuiScaleSmokeWin32]::MOUSEEVENTF_LEFTDOWN,
        0,
        0,
        0,
        [UIntPtr]::Zero
    )
    Start-Sleep -Milliseconds 80
    [CryptoHudGuiScaleSmokeWin32]::mouse_event(
        [CryptoHudGuiScaleSmokeWin32]::MOUSEEVENTF_LEFTUP,
        0,
        0,
        0,
        [UIntPtr]::Zero
    )
    Start-Sleep -Milliseconds 350
}

function Assert-ScaledQuoteBoardContentVisible([object]$Window) {
    Start-Sleep -Milliseconds 500
    $bitmap = [System.Drawing.Bitmap]::new(
        [Math]::Max(1, [int]$Window.Width),
        [Math]::Max(1, [int]$Window.Height)
    )
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen([int]$Window.Left, [int]$Window.Top, 0, 0, $bitmap.Size)
    $graphics.Dispose()

    $capturePath = Join-Path $StateDir "quote-board-10-scale.png"
    $bitmap.Save($capturePath, [System.Drawing.Imaging.ImageFormat]::Png)

    $colorCounts = @{}
    for ($x = 0; $x -lt $bitmap.Width; $x += 1) {
        for ($y = 0; $y -lt $bitmap.Height; $y += 1) {
            $pixel = $bitmap.GetPixel($x, $y)
            $key = "{0},{1},{2}" -f $pixel.R, $pixel.G, $pixel.B
            if ($colorCounts.ContainsKey($key)) {
                $colorCounts[$key] += 1
            } else {
                $colorCounts[$key] = 1
            }
        }
    }
    $dominantColor = $colorCounts.GetEnumerator() | Sort-Object Value -Descending | Select-Object -First 1
    $dominantRgb = @($dominantColor.Key.Split(",") | ForEach-Object { [int]$_ })

    $detailPixels = 0
    $startX = [Math]::Max(1, [int]($bitmap.Width / 4))
    for ($x = $startX; $x -lt $bitmap.Width; $x += 1) {
        for ($y = 1; $y -lt $bitmap.Height; $y += 1) {
            $pixel = $bitmap.GetPixel($x, $y)
            $colorDistance =
                [Math]::Abs([int]$pixel.R - $dominantRgb[0]) +
                [Math]::Abs([int]$pixel.G - $dominantRgb[1]) +
                [Math]::Abs([int]$pixel.B - $dominantRgb[2])
            if ($colorDistance -gt 20) {
                $detailPixels += 1
            }
        }
    }
    $bitmap.Dispose()

    if ($detailPixels -lt 8) {
        throw "Quote Board 10% screenshot looks clipped; expected scaled detail pixels, saw $detailPixels. Capture: $capturePath"
    }
}

$env:CRYPTO_HUD_STATE_DIR = $StateDir
$env:CRYPTO_HUD_GUI_SMOKE_READY_FILE = $ReadyFile
$env:CRYPTO_HUD_INSTANCE_ID = "com.crypto-hud.gui-widget-scale-smoke.$PID"
$env:SLINT_BACKEND = "software"

Push-Location $RepoRoot
try {
    cargo build -p crypto-hud
    if ($LASTEXITCODE -ne 0) {
        throw "GUI widget scale smoke build exited with code $LASTEXITCODE"
    }

    $app = Start-Process `
        -FilePath (Join-Path $RepoRoot "target\debug\crypto-hud.exe") `
        -ArgumentList @("--widgets", "1", "--show-settings", "--gui-smoke-ms", "$TimeoutMs") `
        -PassThru
    try {
        Wait-ForFile $ReadyFile 5000
        Start-Sleep -Milliseconds 700

        $windows = @(Get-ProcessWindows $app.Id)
        $settingsWindow = $windows |
            Where-Object { $_.Title -eq "Crypto HUD" -and $_.Width -ge 1000 } |
            Select-Object -First 1
        if (-not $settingsWindow) {
            throw "Settings window was not found. Windows: $($windows | ConvertTo-Json -Compress)"
        }

        [void][CryptoHudGuiScaleSmokeWin32]::ShowWindow(
            [IntPtr]$settingsWindow.Handle,
            [CryptoHudGuiScaleSmokeWin32]::SW_RESTORE
        )
        [void][CryptoHudGuiScaleSmokeWin32]::SetForegroundWindow([IntPtr]$settingsWindow.Handle)
        Start-Sleep -Milliseconds 300

        Click-ClientPoint ([IntPtr]$settingsWindow.Handle) 832 489 "show icon toggle"
        Click-ClientPoint ([IntPtr]$settingsWindow.Handle) 832 539 "hide quote asset toggle"
        [void](Wait-ForWidgetState 224 101 100)
        Click-ClientPoint ([IntPtr]$settingsWindow.Handle) 885 598 "scale increase"

        $widgetState = Wait-ForWidgetState 235 106 105
        if ([bool]$widgetState.config.show_coin_logos) {
            throw "show_coin_logos expected false after GUI click"
        }
        if (-not [bool]$widgetState.config.hide_quote_asset) {
            throw "hide_quote_asset expected true after GUI click"
        }

        $liveWidget = @(Get-ProcessWindows $app.Id) |
            Where-Object { $_.Title -eq "quote-board-1" } |
            Select-Object -First 1
        if (-not $liveWidget) {
            throw "Live widget window was not found"
        }
        if ([int]$liveWidget.Width -ne 235) {
            throw "Live widget width expected 235, saw $($liveWidget.Width)"
        }
        if ([int]$liveWidget.Height -ne 106) {
            throw "Live widget height expected 106, saw $($liveWidget.Height)"
        }

        for ($click = 0; $click -lt 19; $click += 1) {
            Click-ClientPoint ([IntPtr]$settingsWindow.Handle) 783 598 "scale decrease"
        }

        [void](Wait-ForWidgetState 22 10 10)
        $minimumScaleWidget = @(Get-ProcessWindows $app.Id) |
            Where-Object { $_.Title -eq "quote-board-1" } |
            Select-Object -First 1
        if (-not $minimumScaleWidget) {
            throw "Live widget window was not found after scaling down"
        }
        if ([int]$minimumScaleWidget.Width -ne 22) {
            throw "Live widget width at 10% expected 22, saw $($minimumScaleWidget.Width)"
        }
        if ([int]$minimumScaleWidget.Height -ne 10) {
            throw "Live widget height at 10% expected 10, saw $($minimumScaleWidget.Height)"
        }
        Assert-ScaledQuoteBoardContentVisible $minimumScaleWidget
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
}
