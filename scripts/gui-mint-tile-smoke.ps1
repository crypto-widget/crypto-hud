param(
    [int]$TimeoutMs = 30000,
    [string]$OutputDir = "",
    [string]$Language = "en",
    [switch]$LockLight,
    [switch]$RedUp,
    [switch]$PaperBackdrop,
    [ValidateRange(30, 200)]
    [int]$ScalePercent = 100,
    [switch]$UpdatePreviews
)

$ErrorActionPreference = "Stop"

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$StateDir = Join-Path $RepoRoot "target\tmp\gui-mint-tile-smoke-state"
$ReadyFile = Join-Path $StateDir "ready.json"
$StateFile = Join-Path $StateDir "layouts.json"
$statePrefix = "$($RepoRoot.TrimEnd('\', '/'))$([System.IO.Path]::DirectorySeparatorChar)"
if (-not [System.IO.Path]::GetFullPath($StateDir).StartsWith($statePrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Smoke state directory escaped the repository"
}
if ($UpdatePreviews -and (-not $PaperBackdrop -or $ScalePercent -ne 100)) {
    throw "Preview generation requires -PaperBackdrop and -ScalePercent 100"
}
$CargoTargetDir = if ([string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
    Join-Path $RepoRoot "target"
} elseif ([System.IO.Path]::IsPathRooted($env:CARGO_TARGET_DIR)) {
    [System.IO.Path]::GetFullPath($env:CARGO_TARGET_DIR)
} else {
    [System.IO.Path]::GetFullPath((Join-Path $RepoRoot $env:CARGO_TARGET_DIR))
}
if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $CaptureDir = Join-Path $StateDir "captures"
} else {
    $CaptureDir = [System.IO.Path]::GetFullPath($OutputDir)
}

if (Test-Path -LiteralPath $StateDir) {
    Remove-Item -LiteralPath $StateDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $StateDir | Out-Null
New-Item -ItemType Directory -Force -Path $CaptureDir | Out-Null
$CoinIconDir = Join-Path $StateDir "coin-icons"
New-Item -ItemType Directory -Force -Path $CoinIconDir | Out-Null
$cachedBtcIcon = Join-Path $RepoRoot "target\tmp\market-compass-single-scale-state\coin-icons\btc.svg"
$cachedEthIcon = Join-Path $RepoRoot "target\tmp\market-compass-single-scale-state\coin-icons\eth.svg"
if (Test-Path -LiteralPath $cachedBtcIcon -PathType Leaf) {
    Copy-Item -LiteralPath $cachedBtcIcon -Destination (Join-Path $CoinIconDir "btc.svg")
}
if (Test-Path -LiteralPath $cachedEthIcon -PathType Leaf) {
    Copy-Item -LiteralPath $cachedEthIcon -Destination (Join-Path $CoinIconDir "eth.svg")
}

$widgets = @(
    [ordered]@{
        id = "mint-tile-light"
        plugin_id = "com.cryptohud.mint-tile"
        name = "Mint Tile Light"
        visible = $true
        layout = [ordered]@{
            x = 90
            y = 80
            always_on_top = $true
            opacity_percent = 100
            locked = [bool]$LockLight
            scale_percent = 100
            width = 360
            height = 440
        }
        symbols = @("BTC")
        config = [ordered]@{ theme = "light" }
    },
    [ordered]@{
        id = "mint-tile-dark"
        plugin_id = "com.cryptohud.mint-tile"
        name = "Mint Tile Dark"
        visible = $true
        layout = [ordered]@{
            x = 490
            y = 80
            always_on_top = $true
            opacity_percent = 100
            locked = $true
            scale_percent = 100
            width = 360
            height = 440
        }
        symbols = @("ETH")
        config = [ordered]@{ theme = "dark" }
    }
)
foreach ($widget in $widgets) {
    $widget.layout.scale_percent = $ScalePercent
    $widget.layout.width = [int][Math]::Round(360 * $ScalePercent / 100.0)
    $widget.layout.height = [int][Math]::Round(440 * $ScalePercent / 100.0)
}

$state = [ordered]@{
    settings = [ordered]@{
        widgets_always_on_top = $true
        opacity_percent = 100
        widget_scale_percent = 100
        theme = "light"
        language = $Language
        red_up_enabled = [bool]$RedUp
        shortcut = "disabled"
        tray_icon_enabled = $false
        auto_start_enabled = $false
    }
    selected_widget_id = "mint-tile-light"
    next_widget_number = 3
    widgets = $widgets
}
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText(
    $StateFile,
    ($state | ConvertTo-Json -Depth 8),
    $utf8NoBom
)

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
if ($PaperBackdrop) {
    Add-Type -AssemblyName System.Windows.Forms
}

if (-not ("CryptoHudMintTileSmokeWin32" -as [type])) {
    Add-Type @'
using System;
using System.Runtime.InteropServices;
using System.Text;

public static class CryptoHudMintTileSmokeWin32 {
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);

    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr hWnd, StringBuilder lpString, int nMaxCount);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdcBlt, uint nFlags);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr hWnd);
    [DllImport("user32.dll", SetLastError = true)] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr value);
    [DllImport("user32.dll", SetLastError = true)] public static extern bool PostMessage(IntPtr hWnd, uint message, IntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT lpPoint);
    [DllImport("user32.dll", SetLastError = true)] public static extern bool SetCursorPos(int X, int Y);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr hWndInsertAfter, int X, int Y, int cx, int cy, uint uFlags);

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

    public const uint PW_RENDERFULLCONTENT = 0x00000002;
    public const uint SWP_NOSIZE = 0x0001;
    public const uint SWP_NOMOVE = 0x0002;
    public const uint SWP_NOACTIVATE = 0x0010;
    public const uint SWP_SHOWWINDOW = 0x0040;
}
'@
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

function Get-WidgetWindow([int]$ProcessId, [string]$Title) {
    $script:result = $null
    $callback = [CryptoHudMintTileSmokeWin32+EnumWindowsProc]{
        param([IntPtr]$WindowHandle, [IntPtr]$Param)

        [uint32]$windowProcessId = 0
        [void][CryptoHudMintTileSmokeWin32]::GetWindowThreadProcessId($WindowHandle, [ref]$windowProcessId)
        if ($windowProcessId -ne [uint32]$ProcessId -or
            -not [CryptoHudMintTileSmokeWin32]::IsWindowVisible($WindowHandle)) {
            return $true
        }
        $windowTitle = [System.Text.StringBuilder]::new(256)
        [void][CryptoHudMintTileSmokeWin32]::GetWindowText($WindowHandle, $windowTitle, 256)
        if ($windowTitle.ToString() -ne $Title) {
            return $true
        }
        $rect = New-Object CryptoHudMintTileSmokeWin32+RECT
        [void][CryptoHudMintTileSmokeWin32]::GetWindowRect($WindowHandle, [ref]$rect)
        $script:result = [pscustomobject]@{
            Handle = $WindowHandle
            Left = $rect.Left
            Top = $rect.Top
            Width = $rect.Right - $rect.Left
            Height = $rect.Bottom - $rect.Top
            ScaleFactor = [double][CryptoHudMintTileSmokeWin32]::GetDpiForWindow($WindowHandle) / 96.0
        }
        return $false
    }
    [void][CryptoHudMintTileSmokeWin32]::EnumWindows($callback, [IntPtr]::Zero)
    return $script:result
}

function Wait-ForWidgetWindow([int]$ProcessId, [string]$Title, [int]$TimeoutMilliseconds) {
    $deadline = (Get-Date).AddMilliseconds($TimeoutMilliseconds)
    while ((Get-Date) -le $deadline) {
        $window = Get-WidgetWindow $ProcessId $Title
        if ($window) {
            return $window
        }
        Start-Sleep -Milliseconds 100
    }
    throw "Timed out waiting for widget window $Title"
}

function Capture-Widget([object]$Window, [string]$Name) {
    $physicalScale = $Window.ScaleFactor * $ScalePercent / 100.0
    if ($Window.ScaleFactor -le 0 -or
        [Math]::Abs($Window.Width - 360 * $physicalScale) -gt 1 -or
        [Math]::Abs($Window.Height - 440 * $physicalScale) -gt 1) {
        throw "$Name has incorrect physical dimensions: $($Window.Width)x$($Window.Height), scale=$physicalScale"
    }

    $bitmap = [System.Drawing.Bitmap]::new($Window.Width, $Window.Height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    if ($PaperBackdrop) {
        # Put the paper above other widgets before raising this one, so its
        # transparent margins cannot include a neighboring card.
        [void][CryptoHudMintTileSmokeWin32]::SetWindowPos(
            $paperBackdropForm.Handle,
            [IntPtr](-1),
            0,
            0,
            0,
            0,
            [CryptoHudMintTileSmokeWin32]::SWP_NOMOVE -bor
                [CryptoHudMintTileSmokeWin32]::SWP_NOSIZE -bor
                [CryptoHudMintTileSmokeWin32]::SWP_NOACTIVATE -bor
                [CryptoHudMintTileSmokeWin32]::SWP_SHOWWINDOW
        )
        [void][CryptoHudMintTileSmokeWin32]::SetWindowPos(
            [IntPtr]$Window.Handle,
            [IntPtr](-1),
            0,
            0,
            0,
            0,
            [CryptoHudMintTileSmokeWin32]::SWP_NOMOVE -bor
                [CryptoHudMintTileSmokeWin32]::SWP_NOSIZE -bor
                [CryptoHudMintTileSmokeWin32]::SWP_NOACTIVATE -bor
                [CryptoHudMintTileSmokeWin32]::SWP_SHOWWINDOW
        )
        Start-Sleep -Milliseconds 100
        [System.Windows.Forms.Application]::DoEvents()
        $graphics.CopyFromScreen($Window.Left, $Window.Top, 0, 0, $bitmap.Size)
    } else {
        $deviceContext = $graphics.GetHdc()
        try {
            $printed = [CryptoHudMintTileSmokeWin32]::PrintWindow(
                [IntPtr]$Window.Handle,
                $deviceContext,
                [CryptoHudMintTileSmokeWin32]::PW_RENDERFULLCONTENT
            )
        } finally {
            $graphics.ReleaseHdc($deviceContext)
        }
        if (-not $printed) {
            $graphics.CopyFromScreen($Window.Left, $Window.Top, 0, 0, $bitmap.Size)
        }
    }
    $graphics.Dispose()

    if ($PaperBackdrop) {
        foreach ($point in @(
            @(0, 0), @(($bitmap.Width - 1), 0),
            @(0, ($bitmap.Height - 1)), @(($bitmap.Width - 1), ($bitmap.Height - 1))
        )) {
            $pixel = $bitmap.GetPixel($point[0], $point[1])
            if ([Math]::Abs([int]$pixel.R - 247) -gt 3 -or
                [Math]::Abs([int]$pixel.G - 249) -gt 3 -or
                [Math]::Abs([int]$pixel.B - 248) -gt 3) {
                $bitmap.Save((Join-Path $CaptureDir "$Name-rejected.png"), [System.Drawing.Imaging.ImageFormat]::Png)
                $bitmap.Dispose()
                throw "$Name has a clipped or obstructed paper backdrop at ($($point[0]), $($point[1])): $pixel"
            }
        }
    }

    $corner = $bitmap.GetPixel(0, 0)
    if (-not $PaperBackdrop -and $corner.A -eq 255 -and
        [Math]::Max($corner.R, [Math]::Max($corner.G, $corner.B)) -le 8) {
        for ($y = 0; $y -lt $bitmap.Height; $y++) {
            for ($x = 0; $x -lt $bitmap.Width; $x++) {
                $pixel = $bitmap.GetPixel($x, $y)
                if ($pixel.A -eq 255 -and
                    [Math]::Max($pixel.R, [Math]::Max($pixel.G, $pixel.B)) -le 4) {
                    $bitmap.SetPixel($x, $y, [System.Drawing.Color]::Transparent)
                }
            }
        }
    }

    $capturePath = Join-Path $CaptureDir "$Name.png"
    $bitmap.Save($capturePath, [System.Drawing.Imaging.ImageFormat]::Png)

    $uniqueColors = [System.Collections.Generic.HashSet[string]]::new()
    $nonBackgroundPixels = 0
    $corner = $bitmap.GetPixel(0, 0)
    for ($y = 0; $y -lt $bitmap.Height; $y += 4) {
        for ($x = 0; $x -lt $bitmap.Width; $x += 4) {
            $pixel = $bitmap.GetPixel($x, $y)
            [void]$uniqueColors.Add("$($pixel.R):$($pixel.G):$($pixel.B):$($pixel.A)")
            $distance =
                [Math]::Abs([int]$pixel.R - [int]$corner.R) +
                [Math]::Abs([int]$pixel.G - [int]$corner.G) +
                [Math]::Abs([int]$pixel.B - [int]$corner.B)
            if ($distance -gt 36) {
                $nonBackgroundPixels += 1
            }
        }
    }
    $bitmap.Dispose()

    if ($uniqueColors.Count -lt 32 -or $nonBackgroundPixels -lt [Math]::Max(20, 3000 * $physicalScale * $physicalScale)) {
        throw "$Name did not render a detailed mint tile: colors=$($uniqueColors.Count), content=$nonBackgroundPixels"
    }
    return $capturePath
}

function Send-WidgetPointer([object]$Window, [double]$X, [double]$Y, [uint32]$Message, [int]$Buttons = 0) {
    $scale = $Window.ScaleFactor * $ScalePercent / 100.0
    $clientX = [int][Math]::Round($X * $scale)
    $clientY = [int][Math]::Round($Y * $scale)
    if ($Message -eq 0x0200) {
        [void][CryptoHudMintTileSmokeWin32]::SetWindowPos(
            $Window.Handle, [IntPtr](-1), 0, 0, 0, 0,
            [CryptoHudMintTileSmokeWin32]::SWP_NOMOVE -bor
                [CryptoHudMintTileSmokeWin32]::SWP_NOSIZE -bor
                [CryptoHudMintTileSmokeWin32]::SWP_NOACTIVATE -bor
                [CryptoHudMintTileSmokeWin32]::SWP_SHOWWINDOW
        )
        # Native mouse capture reads the real cursor when a dragged window moves.
        # Keep it at the requested physical position instead of posting a second
        # synthetic move that can fight those capture events.
        if (-not [CryptoHudMintTileSmokeWin32]::SetCursorPos($Window.Left + $clientX, $Window.Top + $clientY)) {
            throw "Could not position smoke cursor: $([Runtime.InteropServices.Marshal]::GetLastWin32Error())"
        }
        Start-Sleep -Milliseconds 100
        return
    }
    $position = [IntPtr](($clientY -shl 16) -bor $clientX)
    if (-not [CryptoHudMintTileSmokeWin32]::PostMessage($Window.Handle, $Message, [IntPtr]$Buttons, $position)) {
        throw "Could not post pointer input: $([Runtime.InteropServices.Marshal]::GetLastWin32Error())"
    }
    Start-Sleep -Milliseconds 100
}

function Click-WidgetLock([object]$Window) {
    Send-WidgetPointer $Window 312 55 0x0200
    Send-WidgetPointer $Window 312 55 0x0201 1
    Send-WidgetPointer $Window 312 55 0x0202
}

function Drag-Widget([object]$Window) {
    Send-WidgetPointer $Window 120 230 0x0200
    Send-WidgetPointer $Window 120 230 0x0201 1
    Send-WidgetPointer $Window 150 245 0x0200 1
    Send-WidgetPointer $Window 150 245 0x0202
    Start-Sleep -Milliseconds 400
}

function Assert-WidgetLock([bool]$Expected) {
    $deadline = (Get-Date).AddSeconds(3)
    do {
        $savedState = Get-Content -LiteralPath $StateFile -Raw | ConvertFrom-Json
        $widget = $savedState.widgets | Where-Object id -eq "mint-tile-light" | Select-Object -First 1
        if ($widget -and [bool]$widget.layout.locked -eq $Expected) { return }
        Start-Sleep -Milliseconds 100
    } while ((Get-Date) -lt $deadline)
    throw "Mint Tile lock did not persist expected state $Expected"
}

function Assert-AccessibleLock([object]$Window) {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($Window.Handle)
    $condition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::CheckBox
    )
    $control = $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $condition)
    if (-not $control -or [string]::IsNullOrWhiteSpace($control.Current.Name)) {
        throw "Mint Tile must expose a localized, accessible lock control"
    }
    $pattern = $null
    if ($control.TryGetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern, [ref]$pattern)) {
        ([System.Windows.Automation.TogglePattern]$pattern).Toggle()
    } elseif ($control.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern, [ref]$pattern)) {
        ([System.Windows.Automation.InvokePattern]$pattern).Invoke()
    } else {
        throw "Mint Tile lock exposes no accessibility action"
    }
    Assert-WidgetLock $true
}

function Save-Preview([string]$Capture, [string]$Theme) {
    $source = [System.Drawing.Image]::FromFile($Capture)
    $preview = [System.Drawing.Bitmap]::new(360, 440)
    $graphics = [System.Drawing.Graphics]::FromImage($preview)
    try {
        $graphics.Clear([System.Drawing.Color]::FromArgb(247, 249, 248))
        $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $graphics.DrawImage($source, 0, 0, 360, 440)
        $pluginDir = Join-Path $RepoRoot "crates/crypto-hud/plugins/com.cryptohud.mint-tile"
        foreach ($directory in @($pluginDir, (Join-Path $pluginDir "ui"))) {
            $preview.Save((Join-Path $directory "preview-$Theme.png"), [System.Drawing.Imaging.ImageFormat]::Png)
        }
    } finally {
        $graphics.Dispose()
        $preview.Dispose()
        $source.Dispose()
    }
}

$smokeEnvironmentNames = @(
    "CRYPTO_HUD_STATE_DIR", "CRYPTO_HUD_GUI_SMOKE_READY_FILE", "CRYPTO_HUD_INSTANCE_ID",
    "CRYPTO_HUD_GUI_SMOKE_OFFLINE", "CRYPTO_HUD_DISABLE_UPDATE_CHECK", "SLINT_BACKEND"
)
$previousEnvironment = @{}
foreach ($name in $smokeEnvironmentNames) {
    $previousEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
}
$env:CRYPTO_HUD_STATE_DIR = $StateDir
$env:CRYPTO_HUD_GUI_SMOKE_READY_FILE = $ReadyFile
$env:CRYPTO_HUD_INSTANCE_ID = "com.crypto-hud.gui-mint-tile-smoke.$PID"
$env:CRYPTO_HUD_GUI_SMOKE_OFFLINE = "1"
$env:CRYPTO_HUD_DISABLE_UPDATE_CHECK = "1"
$env:SLINT_BACKEND = "software"

$previousDpiContext = [CryptoHudMintTileSmokeWin32]::SetThreadDpiAwarenessContext([IntPtr](-4))
if ($previousDpiContext -eq [IntPtr]::Zero) {
    throw "Could not enable physical-pixel capture: $([Runtime.InteropServices.Marshal]::GetLastWin32Error())"
}
$previousCursor = New-Object CryptoHudMintTileSmokeWin32+POINT
$restoreCursor = [CryptoHudMintTileSmokeWin32]::GetCursorPos([ref]$previousCursor)

$paperBackdropForm = $null
if ($PaperBackdrop) {
    $paperBackdropForm = [System.Windows.Forms.Form]::new()
    $paperBackdropForm.FormBorderStyle = [System.Windows.Forms.FormBorderStyle]::None
    $paperBackdropForm.StartPosition = [System.Windows.Forms.FormStartPosition]::Manual
    $paperBackdropForm.Location = [System.Drawing.Point]::new(40, 40)
    $paperBackdropForm.Size = [System.Drawing.Size]::new(850, 520)
    $paperBackdropForm.BackColor = [System.Drawing.Color]::FromArgb(247, 249, 248)
    $paperBackdropForm.ShowInTaskbar = $false
    $paperBackdropForm.Show()
    [System.Windows.Forms.Application]::DoEvents()
}

Push-Location $RepoRoot
try {
    cargo +1.96.0 build --locked -p crypto-hud
    if ($LASTEXITCODE -ne 0) {
        throw "Mint Tile smoke build exited with code $LASTEXITCODE"
    }

    $app = Start-Process `
        -FilePath (Join-Path $CargoTargetDir "debug\crypto-hud.exe") `
        -ArgumentList @("--widgets", "2", "--gui-smoke-ms", "$TimeoutMs") `
        -WindowStyle Hidden `
        -PassThru
    try {
        Wait-ForFile $ReadyFile 10000
        $ready = Get-Content -LiteralPath $ReadyFile -Raw | ConvertFrom-Json
        if (-not [bool]$ready.ready -or -not [bool]$ready.marketDataReady) {
            throw "Mint Tile smoke did not report ready market data"
        }
        if (@($ready.pluginIds) -notcontains "com.cryptohud.mint-tile") {
            throw "Mint Tile was missing from the runtime plugin catalog"
        }
        if (@($ready.catalogErrors).Count -ne 0) {
            throw "Mint Tile catalog errors: $($ready.catalogErrors -join '; ')"
        }

        Start-Sleep -Milliseconds 1000
        $lightWindow = Wait-ForWidgetWindow $app.Id "mint-tile-light" 5000
        $darkWindow = Wait-ForWidgetWindow $app.Id "mint-tile-dark" 5000
        if ($paperBackdropForm) {
            $margin = [int][Math]::Ceiling([Math]::Max(30, 60 * $lightWindow.ScaleFactor * $ScalePercent / 100.0))
            $left = [Math]::Min($lightWindow.Left, $darkWindow.Left) - $margin
            $top = [Math]::Min($lightWindow.Top, $darkWindow.Top) - $margin
            $right = [Math]::Max($lightWindow.Left + $lightWindow.Width, $darkWindow.Left + $darkWindow.Width) + $margin
            $bottom = [Math]::Max($lightWindow.Top + $lightWindow.Height, $darkWindow.Top + $darkWindow.Height) + $margin
            $paperBackdropForm.Location = [System.Drawing.Point]::new($left, $top)
            $paperBackdropForm.Size = [System.Drawing.Size]::new($right - $left, $bottom - $top)
            [void][CryptoHudMintTileSmokeWin32]::SetWindowPos(
                $paperBackdropForm.Handle,
                [IntPtr](-1),
                0,
                0,
                0,
                0,
                [CryptoHudMintTileSmokeWin32]::SWP_NOMOVE -bor
                    [CryptoHudMintTileSmokeWin32]::SWP_NOSIZE -bor
                    [CryptoHudMintTileSmokeWin32]::SWP_NOACTIVATE -bor
                    [CryptoHudMintTileSmokeWin32]::SWP_SHOWWINDOW
            )
            [System.Windows.Forms.Application]::DoEvents()
        }
        $lightCapture = Capture-Widget $lightWindow "mint-tile-light"
        $darkCapture = Capture-Widget $darkWindow "mint-tile-dark"
        Send-WidgetPointer $lightWindow 312 55 0x0200
        $hoverCapture = Capture-Widget $lightWindow "mint-tile-light-hover"
        if ($LockLight) { Click-WidgetLock $lightWindow; Assert-WidgetLock $false }
        Click-WidgetLock $lightWindow
        Assert-WidgetLock $true
        $lockedCapture = Capture-Widget $lightWindow "mint-tile-light-locked"
        Drag-Widget $lightWindow
        $afterLockedDrag = Get-WidgetWindow $app.Id "mint-tile-light"
        if ($afterLockedDrag.Left -ne $lightWindow.Left -or $afterLockedDrag.Top -ne $lightWindow.Top) {
            throw "Locked Mint Tile moved"
        }
        Click-WidgetLock $lightWindow
        Assert-WidgetLock $false
        Drag-Widget $lightWindow
        $afterDrag = Get-WidgetWindow $app.Id "mint-tile-light"
        $physicalScale = $lightWindow.ScaleFactor * $ScalePercent / 100.0
        $expectedX = [Math]::Round(150 * $physicalScale) - [Math]::Round(120 * $physicalScale)
        $expectedY = [Math]::Round(245 * $physicalScale) - [Math]::Round(230 * $physicalScale)
        if ([Math]::Abs($afterDrag.Left - $lightWindow.Left - $expectedX) -gt 1 -or
            [Math]::Abs($afterDrag.Top - $lightWindow.Top - $expectedY) -gt 1) {
            throw "Unlocked Mint Tile did not follow the pointer: expected ($expectedX, $expectedY), got ($($afterDrag.Left - $lightWindow.Left), $($afterDrag.Top - $lightWindow.Top))"
        }
        $movedCapture = Capture-Widget $afterDrag "mint-tile-light-moved"
        Assert-AccessibleLock $afterDrag
        if ($UpdatePreviews) {
            Save-Preview $lightCapture "light"
            Save-Preview $darkCapture "dark"
        }
        Write-Output "Light capture: $lightCapture"
        Write-Output "Dark capture: $darkCapture"
        Write-Output "Hover capture: $hoverCapture"
        Write-Output "Lock and moved captures: $lockedCapture; $movedCapture"
        Write-Output "Mint Tile DPI, mouse lock/unlock, dragging and accessibility smoke passed"
    } finally {
        if ($app -and -not $app.HasExited) {
            Stop-Process -Id $app.Id -Force
        }
    }
} finally {
    Pop-Location
    if ($restoreCursor) {
        [void][CryptoHudMintTileSmokeWin32]::SetCursorPos($previousCursor.X, $previousCursor.Y)
    }
    [void][CryptoHudMintTileSmokeWin32]::SetThreadDpiAwarenessContext($previousDpiContext)
    if ($paperBackdropForm) {
        $paperBackdropForm.Close()
        $paperBackdropForm.Dispose()
    }
    foreach ($name in $smokeEnvironmentNames) {
        [Environment]::SetEnvironmentVariable($name, $previousEnvironment[$name], "Process")
    }
}
