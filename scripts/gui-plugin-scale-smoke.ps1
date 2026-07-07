param(
    [int]$TimeoutMs = 15000
)

$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$StateDir = Join-Path $RepoRoot ".gui-plugin-scale-smoke-state"
$ReadyFile = Join-Path $StateDir "ready.json"
$StateFile = Join-Path $StateDir "layouts.json"

if (Test-Path $StateDir) {
    Remove-Item -LiteralPath $StateDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $StateDir | Out-Null

$seedWidgets = @(
    [ordered]@{
        id = "focus-ticker-10"
        plugin_id = "com.cryptohud.focus-ticker"
        name = "Focus Ticker 10"
        visible = $true
        layout = [ordered]@{
            x = 80
            y = 80
            always_on_top = $false
            opacity_percent = 96
            locked = $false
            scale_percent = 10
            width = 82
            height = 16
        }
        symbols = @("BTC")
        config = [ordered]@{}
    },
    [ordered]@{
        id = "trust-card-10"
        plugin_id = "com.cryptohud.trust-card"
        name = "Trust Card 10"
        visible = $true
        layout = [ordered]@{
            x = 190
            y = 80
            always_on_top = $false
            opacity_percent = 96
            locked = $false
            scale_percent = 10
            width = 52
            height = 41
        }
        symbols = @("ETH")
        config = [ordered]@{}
    },
    [ordered]@{
        id = "status-strip-10"
        plugin_id = "com.cryptohud.status-strip"
        name = "Status Strip 10"
        visible = $true
        layout = [ordered]@{
            x = 270
            y = 80
            always_on_top = $false
            opacity_percent = 96
            locked = $false
            scale_percent = 10
            width = 42
            height = 9
        }
        symbols = @("BTC", "ETH", "SOL")
        config = [ordered]@{}
    }
)

$seedState = [ordered]@{
    settings = [ordered]@{
        widgets_always_on_top = $false
        opacity_percent = 96
        widget_scale_percent = 100
        theme = "dark"
    }
    selected_widget_id = "focus-ticker-10"
    next_widget_number = 4
    widgets = $seedWidgets
}
$seedJson = $seedState | ConvertTo-Json -Depth 8
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($StateFile, $seedJson, $utf8NoBom)

Add-Type -AssemblyName System.Drawing

if (-not ("CryptoHudGuiPluginScaleSmokeWin32" -as [type])) {
    Add-Type @'
using System;
using System.Runtime.InteropServices;
using System.Text;

public static class CryptoHudGuiPluginScaleSmokeWin32 {
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);

    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr hWnd, StringBuilder lpString, int nMaxCount);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);

    [StructLayout(LayoutKind.Sequential)]
    public struct RECT {
        public int Left;
        public int Top;
        public int Right;
        public int Bottom;
    }
}
'@
}

function Get-ProcessWindows([int]$ProcessId) {
    $handles = [System.Collections.Generic.List[IntPtr]]::new()
    $callback = [CryptoHudGuiPluginScaleSmokeWin32+EnumWindowsProc]{
        param([IntPtr]$WindowHandle, [IntPtr]$Param)

        [uint32]$windowProcessId = 0
        [void][CryptoHudGuiPluginScaleSmokeWin32]::GetWindowThreadProcessId($WindowHandle, [ref]$windowProcessId)
        if ($windowProcessId -eq [uint32]$ProcessId -and [CryptoHudGuiPluginScaleSmokeWin32]::IsWindowVisible($WindowHandle)) {
            $handles.Add($WindowHandle)
        }
        return $true
    }
    [void][CryptoHudGuiPluginScaleSmokeWin32]::EnumWindows($callback, [IntPtr]::Zero)

    foreach ($handle in $handles) {
        $title = [System.Text.StringBuilder]::new(256)
        [void][CryptoHudGuiPluginScaleSmokeWin32]::GetWindowText($handle, $title, 256)
        $rect = New-Object CryptoHudGuiPluginScaleSmokeWin32+RECT
        [void][CryptoHudGuiPluginScaleSmokeWin32]::GetWindowRect($handle, [ref]$rect)
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

function Assert-ScaledContentVisible([object]$Window, [string]$Title, [int]$ExpectedWidth, [int]$ExpectedHeight) {
    if ([int]$Window.Width -ne $ExpectedWidth) {
        throw "$Title width expected $ExpectedWidth, saw $($Window.Width)"
    }
    if ([int]$Window.Height -ne $ExpectedHeight) {
        throw "$Title height expected $ExpectedHeight, saw $($Window.Height)"
    }

    $bitmap = [System.Drawing.Bitmap]::new(
        [Math]::Max(1, [int]$Window.Width),
        [Math]::Max(1, [int]$Window.Height)
    )
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen([int]$Window.Left, [int]$Window.Top, 0, 0, $bitmap.Size)
    $graphics.Dispose()

    $capturePath = Join-Path $StateDir "$Title.png"
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

    if ($detailPixels -lt 6) {
        throw "$Title 10% screenshot looks clipped; expected scaled detail pixels, saw $detailPixels. Capture: $capturePath"
    }
}

$env:CRYPTO_HUD_STATE_DIR = $StateDir
$env:CRYPTO_HUD_GUI_SMOKE_READY_FILE = $ReadyFile
$env:CRYPTO_HUD_INSTANCE_ID = "com.crypto-hud.gui-plugin-scale-smoke.$PID"
$env:SLINT_BACKEND = "software"

Push-Location $RepoRoot
try {
    cargo build -p crypto-hud
    if ($LASTEXITCODE -ne 0) {
        throw "GUI plugin scale smoke build exited with code $LASTEXITCODE"
    }

    $app = Start-Process `
        -FilePath (Join-Path $RepoRoot "target\debug\crypto-hud.exe") `
        -ArgumentList @("--widgets", "3", "--show-settings", "--gui-smoke-ms", "$TimeoutMs") `
        -PassThru
    try {
        Wait-ForFile $ReadyFile 5000
        Start-Sleep -Milliseconds 900

        $windows = @(Get-ProcessWindows $app.Id)
        $expected = @(
            [pscustomobject]@{ Title = "focus-ticker-10"; Width = 82; Height = 16 },
            [pscustomobject]@{ Title = "trust-card-10"; Width = 52; Height = 39 },
            [pscustomobject]@{ Title = "status-strip-10"; Width = 42; Height = 9 }
        )

        foreach ($widget in $expected) {
            $window = $windows |
                Where-Object { $_.Title -eq $widget.Title } |
                Select-Object -First 1
            if (-not $window) {
                throw "$($widget.Title) window was not found. Windows: $($windows | ConvertTo-Json -Compress)"
            }
            Assert-ScaledContentVisible $window $widget.Title $widget.Width $widget.Height
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
}
