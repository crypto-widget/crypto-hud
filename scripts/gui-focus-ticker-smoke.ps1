param(
    [ValidateRange(30, 200)][int]$ScalePercent = 100,
    [ValidateRange(1, 100)][int]$OpacityPercent = 100,
    [ValidateRange(1, 8)][int]$SymbolCount = 1,
    [string]$Language = "zh-CN",
    [switch]$RedUp,
    [switch]$PriceMatrix,
    [switch]$SkipBuild,
    [switch]$KeepOpen,
    [ValidateRange(5000, 3600000)][int]$TimeoutMs = 15000
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
# A unique isolated directory avoids overwriting user state or earlier evidence.
$stateDir = Join-Path $repoRoot ("target\tmp\focus-ticker-smoke-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $stateDir -Force | Out-Null
$readyFile = Join-Path $stateDir "ready.json"
$symbols = @("PEPE", "BTC", "ETH", "SOL", "DOGE", "ADA", "XRP", "BNB")
$widgets = @()
$assets = if ($PriceMatrix) { @("BTC", "ETH", "SOL", "BNB", "PEPE") } else { @("PEPE") }
foreach ($asset in $assets) {
    foreach ($theme in @("light", "dark")) {
        $widgetId = if ($PriceMatrix) { "focus-ticker-$($asset.ToLowerInvariant())-$theme" } else { "focus-ticker-$theme" }
        $widgets += [ordered]@{
            id = $widgetId
            plugin_id = "com.cryptohud.focus-ticker"
            name = "Focus Ticker $theme"
            visible = $true
            layout = [ordered]@{
                x = if ($PriceMatrix -and $theme -eq "dark") { 800 } else { 100 }
                y = if ($PriceMatrix) { 70 + [array]::IndexOf($assets, $asset) * 165 } elseif ($theme -eq "light") { 120 } else { 160 + [int][Math]::Round(100 * $ScalePercent / 100) }
                always_on_top = $true
                opacity_percent = $OpacityPercent
                locked = $true
                scale_percent = $ScalePercent
                # Deliberately seed the previous 820 x 156 size to verify layout migration.
                width = [int][Math]::Round(820 * $ScalePercent / 100)
                height = [int][Math]::Round(156 * $ScalePercent / 100)
            }
            symbols = @(if ($PriceMatrix) { "binance:spot:$asset/USDT" } else { $symbols[0..($SymbolCount - 1)] | ForEach-Object { "binance:spot:$_/USDT" } })
            config = @{ theme = $theme }
        }
    }
}
$state = [ordered]@{
    settings = [ordered]@{
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
    CRYPTO_HUD_INSTANCE_ID = "com.crypto-hud.focus-smoke.$([guid]::NewGuid().ToString('N'))"
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
        if ($LASTEXITCODE -ne 0) { throw "Focus Ticker build failed: $LASTEXITCODE" }
    }
    foreach ($key in $variables.Keys) {
        $previous[$key] = [Environment]::GetEnvironmentVariable($key, "Process")
        [Environment]::SetEnvironmentVariable($key, $variables[$key], "Process")
    }
    $duration = if ($KeepOpen) { 3600000 } else { $TimeoutMs }
    $app = Start-Process -FilePath (Join-Path $repoRoot "target\debug\crypto-hud.exe") `
        -ArgumentList @("--widgets", "$($widgets.Count)", "--gui-smoke-ms", "$duration") `
        -WindowStyle Hidden -PassThru `
        -RedirectStandardOutput (Join-Path $stateDir "stdout.log") `
        -RedirectStandardError (Join-Path $stateDir "stderr.log")
    $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMs)
    while (-not (Test-Path -LiteralPath $readyFile)) {
        if ($app.HasExited) { throw "Focus Ticker exited before ready. See $stateDir" }
        if ([DateTime]::UtcNow -gt $deadline) { throw "Focus Ticker readiness timed out. See $stateDir" }
        Start-Sleep -Milliseconds 100
    }
    $ready = Get-Content -LiteralPath $readyFile -Raw | ConvertFrom-Json
    if (-not $ready.ready -or -not $ready.marketDataReady -or $ready.widgetCount -ne $widgets.Count) {
        throw "Expected $($widgets.Count) native widgets with deterministic market data"
    }
    if (@($ready.catalogErrors).Count -ne 0) { throw "Plugin discovery failed: $($ready.catalogErrors)" }
    foreach ($widget in $ready.widgets) {
        if ($widget.pluginId -ne "com.cryptohud.focus-ticker" -or $widget.symbolCount -ne 1 -or $widget.marketDataRowCount -ne 1) {
            throw "Focus Ticker must normalize $SymbolCount requested symbols to one populated row"
        }
        $expectedWidth = [int][Math]::Round(490 * $ScalePercent / 100, [MidpointRounding]::AwayFromZero)
        $expectedHeight = [int][Math]::Round(100 * $ScalePercent / 100, [MidpointRounding]::AwayFromZero)
        if ($widget.layoutWidth -ne $expectedWidth -or $widget.layoutHeight -ne $expectedHeight -or $widget.scalePercent -ne $ScalePercent) {
            throw "Focus Ticker size migration failed: $($widget | ConvertTo-Json -Compress)"
        }
        $dpiScale = [double]$widget.windowScaleFactor
        if ($dpiScale -le 0 -or [Math]::Abs($widget.runtimeWidth - $expectedWidth * $dpiScale) -gt 1 -or [Math]::Abs($widget.runtimeHeight - $expectedHeight * $dpiScale) -gt 1) {
            throw "Focus Ticker logical/physical size mismatch"
        }
    }
    if (-not $KeepOpen) {
        if (-not $app.WaitForExit($TimeoutMs + 5000)) { throw "Focus Ticker did not exit after smoke timer" }
        if ($app.ExitCode -ne 0) { throw "Focus Ticker exited with code $($app.ExitCode)" }
    }
    $succeeded = $true
    [pscustomobject]@{ Result = "passed"; ProcessId = $app.Id; StateDir = $stateDir; ScalePercent = $ScalePercent; Language = $Language; RequestedSymbols = $SymbolCount; PriceMatrix = [bool]$PriceMatrix; KeptOpen = [bool]$KeepOpen }
} finally {
    if ($app -and -not $app.HasExited -and (-not $KeepOpen -or -not $succeeded)) { Stop-Process -Id $app.Id }
    foreach ($key in $previous.Keys) { [Environment]::SetEnvironmentVariable($key, $previous[$key], "Process") }
    Pop-Location
}
