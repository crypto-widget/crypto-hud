param(
    [int]$TimeoutMs = 30000,
    # Restrict the input viewport without changing the real monitor or the
    # Slint window's logical layout. Used to reproduce a short CI desktop.
    [int]$SettingsViewportHeight = 0
)

$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$StateDir = Join-Path $RepoRoot "target\tmp\gui-widget-scale-smoke-state"
$ReadyFile = Join-Path $StateDir "ready.json"
$StateFile = Join-Path $StateDir "layouts.json"
$CargoTargetDir = if ([string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
    Join-Path $RepoRoot "target"
} elseif ([System.IO.Path]::IsPathRooted($env:CARGO_TARGET_DIR)) {
    [System.IO.Path]::GetFullPath($env:CARGO_TARGET_DIR)
} else {
    [System.IO.Path]::GetFullPath((Join-Path $RepoRoot $env:CARGO_TARGET_DIR))
}
$Exe = Join-Path $CargoTargetDir "debug\crypto-hud.exe"

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
        shortcut = "disabled"
        tray_icon_enabled = $false
        auto_start_enabled = $false
    }
    selected_widget_id = "quote-board-1"
    next_widget_number = 2
    widgets = $seedWidgets
}
$seedJson = $seedState | ConvertTo-Json -Depth 8
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($StateFile, $seedJson, $utf8NoBom)

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes

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
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")] public static extern IntPtr MonitorFromWindow(IntPtr hWnd, uint flags);
    [DllImport("user32.dll")] public static extern bool GetMonitorInfo(IntPtr monitor, ref MONITORINFO info);
    [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(POINT point);
    [DllImport("user32.dll")] public static extern IntPtr GetAncestor(IntPtr hWnd, uint flags);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdc, uint flags);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr hWnd);
    [DllImport("user32.dll", SetLastError = true)] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr dpiContext);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr hWnd, ref POINT lpPoint);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr hWndInsertAfter, int X, int Y, int cx, int cy, uint uFlags);
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

    [StructLayout(LayoutKind.Sequential)]
    public struct MONITORINFO {
        public uint Size;
        public RECT Monitor;
        public RECT Work;
        public uint Flags;
    }

    public const int SW_RESTORE = 9;
    public static readonly IntPtr HWND_TOPMOST = new IntPtr(-1);
    public static readonly IntPtr HWND_NOTOPMOST = new IntPtr(-2);
    public const uint SWP_NOSIZE = 0x0001;
    public const uint SWP_NOMOVE = 0x0002;
    public const uint SWP_NOACTIVATE = 0x0010;
    public const uint MOUSEEVENTF_LEFTDOWN = 0x0002;
    public const uint MOUSEEVENTF_LEFTUP = 0x0004;

    public static void Scroll(int delta) {
        mouse_event(0x0800, 0, 0, unchecked((uint)delta), UIntPtr.Zero);
    }
}
'@
}

function Get-VisibleRectanglePoint([object]$Bounds, [object]$Viewport) {
    foreach ($rectangle in @($Bounds, $Viewport)) {
        if ($rectangle.Width -le 0 -or $rectangle.Height -le 0) { return $null }
        foreach ($value in @($rectangle.Left, $rectangle.Top, $rectangle.Width, $rectangle.Height)) {
            if ([double]::IsNaN($value) -or [double]::IsInfinity($value)) { return $null }
        }
    }
    $left = [Math]::Max($Bounds.Left, $Viewport.Left)
    $top = [Math]::Max($Bounds.Top, $Viewport.Top)
    $right = [Math]::Min($Bounds.Left + $Bounds.Width, $Viewport.Left + $Viewport.Width)
    $bottom = [Math]::Min($Bounds.Top + $Bounds.Height, $Viewport.Top + $Viewport.Height)
    if ($right - $left -lt 2 -or $bottom - $top -lt 2) { return $null }
    [pscustomobject]@{ X = [int][Math]::Round(($left + $right) / 2); Y = [int][Math]::Round(($top + $bottom) / 2) }
}

function Get-SettingsScrollPoint([object]$AnchorBounds, [object]$Client) {
    # Use the Scale spinner's actual horizontal span, but not its vertical
    # position: it can be outside the viewport after scrolling. This avoids
    # duplicating Slint's layout/renderer transforms or needing a clickable
    # point on the off-screen switch that we are trying to reveal.
    $scrollColumn = [pscustomobject]@{
        Left = $AnchorBounds.Left
        Top = $Client.Top
        Width = $AnchorBounds.Width
        Height = $Client.Height
    }
    Get-VisibleRectanglePoint $scrollColumn $Client
}

function Get-SettingsClient([IntPtr]$WindowHandle) {
    $rect = New-Object CryptoHudGuiScaleSmokeWin32+RECT
    $origin = New-Object CryptoHudGuiScaleSmokeWin32+POINT
    if (-not [CryptoHudGuiScaleSmokeWin32]::GetClientRect($WindowHandle, [ref]$rect) -or
        -not [CryptoHudGuiScaleSmokeWin32]::ClientToScreen($WindowHandle, [ref]$origin)) {
        throw "Could not read the settings client rectangle"
    }
    [pscustomobject]@{ Left = $origin.X; Top = $origin.Y; Width = $rect.Right; Height = $rect.Bottom; ScaleFactor = [double][CryptoHudGuiScaleSmokeWin32]::GetDpiForWindow($WindowHandle) / 96.0 }
}

function Move-SettingsWindowIntoView([object]$Window) {
    $monitor = [CryptoHudGuiScaleSmokeWin32]::MonitorFromWindow($Window.Handle, 2)
    $info = New-Object CryptoHudGuiScaleSmokeWin32+MONITORINFO
    $info.Size = [Runtime.InteropServices.Marshal]::SizeOf($info)
    if (-not [CryptoHudGuiScaleSmokeWin32]::GetMonitorInfo($monitor, [ref]$info)) { throw "Could not read the settings monitor work area" }
    if ($SettingsViewportHeight -gt 0) {
        $info.Work.Bottom = [Math]::Min($info.Work.Bottom, $info.Work.Top + $SettingsViewportHeight)
    }
    $script:SettingsMonitorWorkArea = [pscustomobject]@{
        Left = $info.Work.Left; Top = $info.Work.Top
        Width = $info.Work.Right - $info.Work.Left; Height = $info.Work.Bottom - $info.Work.Top
    }
    if (-not [CryptoHudGuiScaleSmokeWin32]::SetWindowPos($Window.Handle, [CryptoHudGuiScaleSmokeWin32]::HWND_TOPMOST, $info.Work.Left, $info.Work.Top, 0, 0, [CryptoHudGuiScaleSmokeWin32]::SWP_NOSIZE)) {
        throw "Could not move the settings window to the monitor work area"
    }
    Start-Sleep -Milliseconds 500
}

function Get-SettingsViewport([IntPtr]$WindowHandle) {
    $client = Get-SettingsClient $WindowHandle
    $work = $script:SettingsMonitorWorkArea
    $left = [Math]::Max($client.Left, $work.Left)
    $top = [Math]::Max($client.Top, $work.Top)
    $right = [Math]::Min($client.Left + $client.Width, $work.Left + $work.Width)
    $bottom = [Math]::Min($client.Top + $client.Height, $work.Top + $work.Height)
    [pscustomobject]@{ Left = $left; Top = $top; Width = [Math]::Max(0, $right - $left); Height = [Math]::Max(0, $bottom - $top) }
}

function Reveal-SettingsControl([IntPtr]$WindowHandle, [object]$Bounds) {
    $client = Get-SettingsClient $WindowHandle
    $window = New-Object CryptoHudGuiScaleSmokeWin32+RECT
    if (-not [CryptoHudGuiScaleSmokeWin32]::GetWindowRect($WindowHandle, [ref]$window)) { throw "Could not read the settings window bounds" }
    # Native resizing can clip a fixed Slint layout without changing its
    # scroll range. Expand only the isolated test surface when needed.
    $width = $window.Right - $window.Left + [Math]::Max(0, $Bounds.Right - ($client.Left + $client.Width) + 12)
    $height = $window.Bottom - $window.Top + [Math]::Max(0, $Bounds.Bottom - ($client.Top + $client.Height) + 12)
    $point = Get-VisibleRectanglePoint $Bounds $Bounds
    if (-not $point) { throw "The control has invalid bounds" }
    $work = $script:SettingsMonitorWorkArea
    $x = $window.Left
    $y = $window.Top
    if ($point.X -lt $work.Left -or $point.X -ge $work.Left + $work.Width) {
        $x += [int]($work.Left + $work.Width / 2) - $point.X
    }
    if ($point.Y -lt $work.Top -or $point.Y -ge $work.Top + $work.Height) {
        $y += [int]($work.Top + $work.Height / 2) - $point.Y
    }
    if (-not [CryptoHudGuiScaleSmokeWin32]::SetWindowPos($WindowHandle, [CryptoHudGuiScaleSmokeWin32]::HWND_TOPMOST, $x, $y, [int]$width, [int]$height, 0)) {
        throw "Could not reveal the control in the settings test window"
    }
    Start-Sleep -Milliseconds 350
}

function Move-SettingsPointer([IntPtr]$WindowHandle, [object]$Point) {
    $nativePoint = New-Object CryptoHudGuiScaleSmokeWin32+POINT
    $nativePoint.X = $Point.X
    $nativePoint.Y = $Point.Y
    $hit = [CryptoHudGuiScaleSmokeWin32]::WindowFromPoint($nativePoint)
    if ([CryptoHudGuiScaleSmokeWin32]::GetAncestor($hit, 2) -ne $WindowHandle) { throw "The settings input point is covered or outside the test window" }
    if (-not [CryptoHudGuiScaleSmokeWin32]::SetCursorPos($Point.X, $Point.Y)) { throw "Could not move the pointer inside the test window" }
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
            ScaleFactor = [double][CryptoHudGuiScaleSmokeWin32]::GetDpiForWindow($handle) / 96.0
        }
    }
}

function Assert-LogicalWindowSize([object]$Window, [int]$Width, [int]$Height) {
    if ([double]$Window.ScaleFactor -le 0) {
        throw "$($Window.Title) window DPI was not available"
    }
    $physicalWidth = [int][Math]::Round($Width * $Window.ScaleFactor, [MidpointRounding]::AwayFromZero)
    $physicalHeight = [int][Math]::Round($Height * $Window.ScaleFactor, [MidpointRounding]::AwayFromZero)
    if ([int]$Window.Width -ne $physicalWidth -or [int]$Window.Height -ne $physicalHeight) {
        throw "$($Window.Title) expected physical size ${physicalWidth}x${physicalHeight} for logical ${Width}x${Height}, saw $($Window.Width)x$($Window.Height)"
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

function Wait-ForWidgetState(
    [int]$ExpectedWidth,
    [int]$ExpectedHeight,
    [int]$ExpectedScale,
    [hashtable]$ExpectedConfig = @{}
) {
    $deadline = (Get-Date).AddSeconds(5)
    do {
        $state = Get-Content -LiteralPath $StateFile -Raw | ConvertFrom-Json
        $widget = @($state.widgets)[0]
        $configMatches = $true
        foreach ($name in $ExpectedConfig.Keys) {
            $property = $widget.config.PSObject.Properties[$name]
            if ($null -eq $property -or $property.Value -ne $ExpectedConfig[$name]) {
                $configMatches = $false
                break
            }
        }
        if (
            [int]$widget.layout.width -eq $ExpectedWidth -and
            [int]$widget.layout.height -eq $ExpectedHeight -and
            [int]$widget.layout.scale_percent -eq $ExpectedScale -and
            $configMatches
        ) {
            return $widget
        }
        Start-Sleep -Milliseconds 100
    } while ((Get-Date) -lt $deadline)

    $configDescription = $ExpectedConfig | ConvertTo-Json -Compress
    throw "Widget state did not reach ${ExpectedWidth}x${ExpectedHeight} at ${ExpectedScale}% with config $configDescription"
}

function Get-AutomationControl(
    [IntPtr]$WindowHandle,
    [string]$Name,
    [System.Windows.Automation.ControlType]$ControlType,
    [int]$TimeoutMilliseconds = 5000
) {
    $nameCondition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::NameProperty,
        $Name
    )
    $typeCondition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        $ControlType
    )
    $condition = [System.Windows.Automation.AndCondition]::new(
        [System.Windows.Automation.Condition[]]@($nameCondition, $typeCondition)
    )
    $deadline = (Get-Date).AddMilliseconds($TimeoutMilliseconds)
    $lastError = "the control was not present in the automation tree"
    do {
        try {
            $root = [System.Windows.Automation.AutomationElement]::FromHandle($WindowHandle)
            $element = $root.FindFirst(
                [System.Windows.Automation.TreeScope]::Descendants,
                $condition
            )
            if ($element) {
                return $element
            }
            $lastError = "the control was not present in the automation tree"
        } catch {
            $lastError = $_.Exception.Message
        }
        Start-Sleep -Milliseconds 100
    } while ((Get-Date) -lt $deadline)

    throw "Could not find accessible $ControlType control named '$Name' within $TimeoutMilliseconds ms. Last error: $lastError"
}

function Click-AutomationSwitch([IntPtr]$WindowHandle, [string]$Name) {
    $point = $null
    for ($attempt = 0; $attempt -lt 5; $attempt++) {
        $element = $null
        try {
            $element = Get-AutomationControl $WindowHandle $Name ([System.Windows.Automation.ControlType]::Button) 500
        } catch {
            if ($attempt -eq 4) { throw }
        }
        if ($element) {
            $viewport = Get-SettingsViewport $WindowHandle
            $point = Get-VisibleRectanglePoint $element.Current.BoundingRectangle $viewport
            if ($point) { break }
            if ($attempt -ge 2) {
                Reveal-SettingsControl $WindowHandle $element.Current.BoundingRectangle
                continue
            }
        }
        Scroll-AutomationPanel $WindowHandle
    }
    if (-not $point) { throw "Accessible switch '$Name' did not become visible after bounded scrolling" }
    Move-SettingsPointer $WindowHandle $point
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

function Scroll-AutomationPanel([IntPtr]$WindowHandle) {
    $anchor = Get-AutomationControl $WindowHandle "Scale" ([System.Windows.Automation.ControlType]::Spinner)
    $point = Get-SettingsScrollPoint $anchor.Current.BoundingRectangle (Get-SettingsViewport $WindowHandle)
    if (-not $point) {
        Reveal-SettingsControl $WindowHandle $anchor.Current.BoundingRectangle
        $anchor = Get-AutomationControl $WindowHandle "Scale" ([System.Windows.Automation.ControlType]::Spinner)
        $point = Get-SettingsScrollPoint $anchor.Current.BoundingRectangle (Get-SettingsViewport $WindowHandle)
    }
    if (-not $point) { throw "The settings scroll column has no visible area" }
    Move-SettingsPointer $WindowHandle $point
    [CryptoHudGuiScaleSmokeWin32]::Scroll(-240)
    Start-Sleep -Milliseconds 350
}

function Save-SettingsFailureDiagnostics([int]$ProcessId, [IntPtr]$WindowHandle) {
    $windows = @(Get-ProcessWindows $ProcessId)
    $controls = @()
    if ($WindowHandle -ne [IntPtr]::Zero) {
        foreach ($name in @("Show coin logos", "Hide quote asset", "Scale")) {
            try {
                $type = if ($name -eq "Scale") { [System.Windows.Automation.ControlType]::Spinner } else { [System.Windows.Automation.ControlType]::Button }
                $element = Get-AutomationControl $WindowHandle $name $type 100
                $controls += [pscustomobject]@{ Name = $name; Bounds = $element.Current.BoundingRectangle; Offscreen = $element.Current.IsOffscreen }
            } catch { $controls += [pscustomobject]@{ Name = $name; Unavailable = $true } }
        }
    }
    $diagnostics = [pscustomobject]@{ Windows = $windows; MonitorWorkArea = $script:SettingsMonitorWorkArea; Controls = $controls }
    [System.IO.File]::WriteAllText((Join-Path $StateDir "failure-geometry.json"), ($diagnostics | ConvertTo-Json -Depth 6), $utf8NoBom)
    Write-Output ($diagnostics | ConvertTo-Json -Depth 6 -Compress)
    if ($WindowHandle -ne [IntPtr]::Zero) {
        $client = Get-SettingsClient $WindowHandle
        $bitmap = [System.Drawing.Bitmap]::new($client.Width, $client.Height)
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
        $dc = $graphics.GetHdc()
        try {
            if ([CryptoHudGuiScaleSmokeWin32]::PrintWindow($WindowHandle, $dc, 1)) {
                $graphics.ReleaseHdc($dc)
                $dc = [IntPtr]::Zero
                $bitmap.Save((Join-Path $StateDir "failure-settings.png"), [System.Drawing.Imaging.ImageFormat]::Png)
            }
        } finally {
            if ($dc -ne [IntPtr]::Zero) { $graphics.ReleaseHdc($dc) }
            $graphics.Dispose()
            $bitmap.Dispose()
        }
    }
}

function Set-AutomationRangeValue([IntPtr]$WindowHandle, [string]$Name, [double]$Value) {
    $element = Get-AutomationControl `
        -WindowHandle $WindowHandle `
        -Name $Name `
        -ControlType ([System.Windows.Automation.ControlType]::Spinner)
    $pattern = $null
    if (-not $element.TryGetCurrentPattern(
            [System.Windows.Automation.RangeValuePattern]::Pattern,
            [ref]$pattern
        )) {
        throw "Accessible control '$Name' does not expose RangeValue"
    }
    ([System.Windows.Automation.RangeValuePattern]$pattern).SetValue($Value)
    Start-Sleep -Milliseconds 350
}

function Assert-ScaledQuoteBoardContentVisible([object]$Window) {
    $positionFlags =
        [CryptoHudGuiScaleSmokeWin32]::SWP_NOSIZE -bor
        [CryptoHudGuiScaleSmokeWin32]::SWP_NOMOVE -bor
        [CryptoHudGuiScaleSmokeWin32]::SWP_NOACTIVATE
    [void][CryptoHudGuiScaleSmokeWin32]::SetWindowPos(
        [IntPtr]$Window.Handle,
        [CryptoHudGuiScaleSmokeWin32]::HWND_TOPMOST,
        0,
        0,
        0,
        0,
        $positionFlags
    )
    Start-Sleep -Milliseconds 500
    $bitmap = [System.Drawing.Bitmap]::new(
        [Math]::Max(1, [int]$Window.Width),
        [Math]::Max(1, [int]$Window.Height)
    )
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen([int]$Window.Left, [int]$Window.Top, 0, 0, $bitmap.Size)
    $graphics.Dispose()
    [void][CryptoHudGuiScaleSmokeWin32]::SetWindowPos(
        [IntPtr]$Window.Handle,
        [CryptoHudGuiScaleSmokeWin32]::HWND_NOTOPMOST,
        0,
        0,
        0,
        0,
        $positionFlags
    )

    $capturePath = Join-Path $StateDir "quote-board-30-scale.png"
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
        throw "Quote Board 30% screenshot looks clipped; expected scaled detail pixels, saw $detailPixels. Capture: $capturePath"
    }
}

$env:CRYPTO_HUD_STATE_DIR = $StateDir
$env:CRYPTO_HUD_GUI_SMOKE_READY_FILE = $ReadyFile
$env:CRYPTO_HUD_INSTANCE_ID = "com.crypto-hud.gui-widget-scale-smoke.$PID"
$env:CRYPTO_HUD_GUI_SMOKE_OFFLINE = "1"
$env:CRYPTO_HUD_DISABLE_UPDATE_CHECK = "1"
$env:SLINT_BACKEND = "software"

$previousDpiContext = [CryptoHudGuiScaleSmokeWin32]::SetThreadDpiAwarenessContext([IntPtr](-4))
if ($previousDpiContext -eq [IntPtr]::Zero) {
    throw "Could not enable physical-pixel window capture: $([Runtime.InteropServices.Marshal]::GetLastWin32Error())"
}
Push-Location $RepoRoot
try {
    cargo build -p crypto-hud
    if ($LASTEXITCODE -ne 0) {
        throw "GUI widget scale smoke build exited with code $LASTEXITCODE"
    }

    $app = Start-Process `
        -FilePath $Exe `
        -ArgumentList @("--widgets", "1", "--show-settings", "--gui-smoke-ms", "$TimeoutMs") `
        -WindowStyle Hidden `
        -PassThru
    try {
        Wait-ForFile $ReadyFile 8000
        $ready = Get-Content -LiteralPath $ReadyFile -Raw | ConvertFrom-Json
        if (-not [bool]$ready.marketDataReady) {
            throw "GUI widget scale smoke marker did not report market data ready"
        }
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
        Move-SettingsWindowIntoView $settingsWindow
        [void][CryptoHudGuiScaleSmokeWin32]::SetForegroundWindow([IntPtr]$settingsWindow.Handle)
        Start-Sleep -Milliseconds 300

        # Slint switches do not expose UIA focus, and the hosted runner can
        # reject Toggle(). Reveal the switch, click its visible bounds, and
        # verify persisted state instead of relying on GetClickablePoint().
        Scroll-AutomationPanel ([IntPtr]$settingsWindow.Handle)
        Click-AutomationSwitch ([IntPtr]$settingsWindow.Handle) "Show coin logos"
        Click-AutomationSwitch ([IntPtr]$settingsWindow.Handle) "Hide quote asset"
        $expectedConfig = @{ show_coin_logos = $false; hide_quote_asset = $true }
        [void](Wait-ForWidgetState 224 101 100 $expectedConfig)
        Set-AutomationRangeValue ([IntPtr]$settingsWindow.Handle) "Scale" 105

        [void](Wait-ForWidgetState 235 106 105 $expectedConfig)

        $liveWidget = @(Get-ProcessWindows $app.Id) |
            Where-Object { $_.Title -eq "quote-board-1" } |
            Select-Object -First 1
        if (-not $liveWidget) {
            throw "Live widget window was not found"
        }
        Assert-LogicalWindowSize $liveWidget 235 106

        Set-AutomationRangeValue ([IntPtr]$settingsWindow.Handle) "Scale" 30

        [void](Wait-ForWidgetState 67 30 30 $expectedConfig)
        $minimumScaleWidget = @(Get-ProcessWindows $app.Id) |
            Where-Object { $_.Title -eq "quote-board-1" } |
            Select-Object -First 1
        if (-not $minimumScaleWidget) {
            throw "Live widget window was not found after scaling down"
        }
        Assert-LogicalWindowSize $minimumScaleWidget 67 30
        Assert-ScaledQuoteBoardContentVisible $minimumScaleWidget
    } catch {
        $smokeError = $_
        try {
            $diagnosticHandle = if ($settingsWindow) { [IntPtr]$settingsWindow.Handle } else { [IntPtr]::Zero }
            Save-SettingsFailureDiagnostics $app.Id $diagnosticHandle
        } catch { Write-Warning "Could not capture the smoke failure geometry: $($_.Exception.Message)" }
        throw $smokeError
    } finally {
        if ($app -and -not $app.HasExited) {
            Stop-Process -Id $app.Id -Force
        }
    }
} finally {
    Pop-Location
    [void][CryptoHudGuiScaleSmokeWin32]::SetThreadDpiAwarenessContext($previousDpiContext)
    Remove-Item Env:\CRYPTO_HUD_STATE_DIR -ErrorAction SilentlyContinue
    Remove-Item Env:\CRYPTO_HUD_GUI_SMOKE_READY_FILE -ErrorAction SilentlyContinue
    Remove-Item Env:\CRYPTO_HUD_INSTANCE_ID -ErrorAction SilentlyContinue
    Remove-Item Env:\CRYPTO_HUD_GUI_SMOKE_OFFLINE -ErrorAction SilentlyContinue
    Remove-Item Env:\CRYPTO_HUD_DISABLE_UPDATE_CHECK -ErrorAction SilentlyContinue
}
