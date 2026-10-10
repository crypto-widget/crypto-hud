param(
    [ValidateRange(30, 200)][int]$ScalePercent = 100,
    [ValidateRange(1, 100)][int]$OpacityPercent = 100,
    [ValidateSet("BTC", "PEPE")][string]$FirstAsset = "BTC",
    [ValidateSet("both", "light", "dark")][string]$Theme = "both",
    [ValidateRange(1, 8)][int]$SymbolCount = 3,
    [string]$Language = "zh-CN",
    [switch]$RedUp,
    [switch]$Locked,
    [switch]$SkipBuild,
    [switch]$KeepOpen,
    [ValidateRange(5000, 3600000)][int]$TimeoutMs = 15000
)

# Validate native plugin loading and window geometry through the application's
# ready marker. This smoke does not capture the desktop or send desktop input.
$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$stateDir = Join-Path $repoRoot ("target\tmp\status-strip-runtime-smoke-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $stateDir -Force | Out-Null
$readyFile = Join-Path $stateDir "ready.json"
$cargoTargetDir = if ([string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
    Join-Path $repoRoot "target"
} elseif ([IO.Path]::IsPathRooted($env:CARGO_TARGET_DIR)) {
    [IO.Path]::GetFullPath($env:CARGO_TARGET_DIR)
} else {
    [IO.Path]::GetFullPath((Join-Path $repoRoot $env:CARGO_TARGET_DIR))
}
$themes = if ($Theme -eq "both") { @("light", "dark") } else { @($Theme) }
$visibleCount = [Math]::Min($SymbolCount, 5)
$stripWidth = [int][Math]::Round((122 * $visibleCount + 8) * $ScalePercent / 100, [MidpointRounding]::AwayFromZero)
$stripHeight = [int][Math]::Round(92 * $ScalePercent / 100, [MidpointRounding]::AwayFromZero)
$assets = @($FirstAsset) + @('BTC', 'ETH', 'SOL', 'BNB', 'PEPE', 'DOGE', 'ADA', 'XRP' | Where-Object { $_ -ne $FirstAsset })
$symbols = @($assets | Select-Object -First $SymbolCount | ForEach-Object { "binance:spot:$_/USDT" })
$widgets = @()
foreach ($item in $themes) {
    $widgets += [ordered]@{
        id = "status-strip-$item"
        plugin_id = "com.cryptohud.status-strip"
        name = "Status Strip $item"
        visible = $true
        layout = [ordered]@{
            x = 80 + $widgets.Count * ($stripWidth + 40)
            y = 100
            always_on_top = $true
            opacity_percent = $OpacityPercent
            locked = [bool]$Locked
            scale_percent = $ScalePercent
            width = $stripWidth
            height = $stripHeight
        }
        symbols = $symbols
        config = @{
            theme = if ($item -eq "light") { "default" } else { $item }
        }
    }
}
$state = [ordered]@{
    settings = @{
        language = $Language
        show_main_window_on_startup = $false
        red_up_enabled = [bool]$RedUp
        shortcut = "disabled"
        tray_icon_enabled = $false
        auto_start_enabled = $false
    }
    selected_widget_id = $widgets[0].id
    next_widget_number = $widgets.Count + 1
    widgets = $widgets
}
[IO.File]::WriteAllText((Join-Path $stateDir "layouts.json"), ($state | ConvertTo-Json -Depth 10), [Text.UTF8Encoding]::new($false))
$variables = @{
    CRYPTO_HUD_STATE_DIR = $stateDir
    CRYPTO_HUD_GUI_SMOKE_READY_FILE = $readyFile
    CRYPTO_HUD_INSTANCE_ID = "com.crypto-hud.status-strip-runtime-smoke.$([guid]::NewGuid().ToString('N'))"
    CRYPTO_HUD_GUI_SMOKE_OFFLINE = "1"
    CRYPTO_HUD_GUI_SMOKE_FOCUS_TICKER = "1"
    CRYPTO_HUD_DISABLE_UPDATE_CHECK = "1"
    SLINT_BACKEND = "software"
}
$previous = @{}
$app = $null
$succeeded = $false
Push-Location $repoRoot
try {
    if (-not $SkipBuild) {
        cargo +1.96.0 build --locked -p crypto-hud
        if ($LASTEXITCODE -ne 0) { throw "Status Strip build failed: $LASTEXITCODE" }
    }
    foreach ($key in $variables.Keys) {
        $previous[$key] = [Environment]::GetEnvironmentVariable($key, "Process")
        [Environment]::SetEnvironmentVariable($key, $variables[$key], "Process")
    }
    $duration = if ($KeepOpen) { 3600000 } else { $TimeoutMs }
    $app = Start-Process -FilePath (Join-Path $cargoTargetDir "debug\crypto-hud.exe") `
        -ArgumentList @("--widgets", "$($widgets.Count)", "--gui-smoke-ms", "$duration") `
        -WindowStyle Hidden -PassThru `
        -RedirectStandardOutput (Join-Path $stateDir "stdout.log") `
        -RedirectStandardError (Join-Path $stateDir "stderr.log")
    # Keep the handle available for ExitCode after a short-lived smoke exits.
    $null = $app.Handle
    $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMs)
    while (-not (Test-Path -LiteralPath $readyFile)) {
        if ($app.HasExited) { throw "Status Strip exited before ready. See $stateDir" }
        if ([DateTime]::UtcNow -gt $deadline) { throw "Status Strip readiness timed out. See $stateDir" }
        Start-Sleep -Milliseconds 100
    }
    $ready = Get-Content -LiteralPath $readyFile -Raw | ConvertFrom-Json
    if (-not $ready.ready -or -not $ready.marketDataReady -or $ready.widgetCount -ne $widgets.Count -or @($ready.widgets).Count -ne $widgets.Count) {
        throw "Expected $($widgets.Count) native Status Strip windows with deterministic prices"
    }
    if (@($ready.pluginIds) -notcontains "com.cryptohud.status-strip") {
        throw "Status Strip was missing from the runtime plugin catalog"
    }
    if (@($ready.catalogErrors).Count -ne 0) { throw "Plugin discovery failed: $($ready.catalogErrors)" }
    foreach ($widget in $ready.widgets) {
        if (@($widgets.id) -notcontains $widget.id -or $widget.pluginId -ne "com.cryptohud.status-strip" -or $widget.symbolCount -ne $visibleCount -or $widget.marketDataRowCount -ne $visibleCount) {
            throw "Status Strip did not populate its normalized pair count: $($widget | ConvertTo-Json -Compress)"
        }
        if ([bool]$widget.locked -ne [bool]$Locked) { throw "Status Strip did not preserve the requested lock state" }
        if ($widget.layoutWidth -ne $stripWidth -or $widget.layoutHeight -ne $stripHeight -or $widget.scalePercent -ne $ScalePercent) {
            throw "Status Strip logical dimensions do not match the requested scale"
        }
        $dpiScale = [double]$widget.windowScaleFactor
        if ($dpiScale -le 0 -or [Math]::Abs($widget.runtimeWidth - $stripWidth * $dpiScale) -gt 1 -or [Math]::Abs($widget.runtimeHeight - $stripHeight * $dpiScale) -gt 1) {
            throw "Status Strip physical dimensions do not match the desktop DPI"
        }
    }
    if (-not $KeepOpen) {
        if (-not $app.WaitForExit($TimeoutMs + 5000)) { throw "Status Strip did not exit after the smoke timer" }
        if ($app.ExitCode -ne 0) { throw "Status Strip exited with code $($app.ExitCode)" }
    }
    $succeeded = $true
    [pscustomobject]@{
        Result = "passed"
        ProcessId = $app.Id
        StateDir = $stateDir
        ScalePercent = $ScalePercent
        Theme = $Theme
        FirstAsset = $FirstAsset
        RequestedPairs = $SymbolCount
        VisiblePairs = $visibleCount
        KeptOpen = [bool]$KeepOpen
    }
} finally {
    if ($app -and -not $app.HasExited -and (-not $KeepOpen -or -not $succeeded)) { Stop-Process -Id $app.Id }
    foreach ($key in $previous.Keys) { [Environment]::SetEnvironmentVariable($key, $previous[$key], "Process") }
    Pop-Location
}
