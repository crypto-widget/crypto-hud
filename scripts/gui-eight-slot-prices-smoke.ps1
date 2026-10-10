param(
    [ValidateRange(30, 200)][int]$ScalePercent = 100,
    [ValidateRange(1, 8)][int]$SymbolCount = 8,
    [ValidateSet("BTC", "PEPE")][string]$FirstAsset = "PEPE",
    [switch]$SkipBuild,
    [switch]$KeepOpen,
    [ValidateRange(5000, 3600000)][int]$TimeoutMs = 15000
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$stateDir = Join-Path $repoRoot ("target\tmp\eight-slot-prices-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $stateDir -Force | Out-Null
$readyFile = Join-Path $stateDir "ready.json"
$symbols = @($FirstAsset) + @("BTC", "ETH", "SOL", "BNB", "PEPE", "XRP", "DOGE", "ADA" | Where-Object { $_ -ne $FirstAsset })
$plugins = @(
    @{ id = "builtin.quote-board"; name = "Quote Board"; symbolLimit = 20; defaultSize = @{ width = 286; height = 194 }; themes = @(@{ id = "light" }, @{ id = "dark" }) },
    @{ id = "builtin.mini-ticker"; name = "Mini Ticker"; symbolLimit = 1; defaultSize = @{ width = 236; height = 112 }; themes = @(@{ id = "light" }, @{ id = "dark" }) }
)
$plugins += @(Get-ChildItem -LiteralPath (Join-Path $repoRoot "crates\crypto-hud\plugins") -Directory | ForEach-Object {
    $manifest = Join-Path $_.FullName "widget.json"
    if (Test-Path -LiteralPath $manifest) {
        $definition = Get-Content -LiteralPath $manifest -Raw | ConvertFrom-Json
        # These prototypes are deliberately unavailable in the native catalog;
        # widget_price_tests still measures their Slint price renderers.
        if ($definition.id -notin @("com.cryptohud.market-board", "com.example.stage3-price-card")) { $definition }
    }
})
$widgets = @()
$expectedRows = @{}
foreach ($plugin in $plugins) {
    $themes = if ($plugin.themes) { @($plugin.themes | ForEach-Object { $_.id }) } else { @("default") }
    foreach ($theme in $themes) {
        $id = "$($plugin.id)-$theme"
        $expectedRows[$id] = [Math]::Min($SymbolCount, $plugin.symbolLimit)
        $widgets += [ordered]@{
            id = $id
            plugin_id = $plugin.id
            name = "$($plugin.name) $theme"
            visible = $true
            layout = [ordered]@{
                x = 80 + ($widgets.Count % 4) * 300
                y = [int](70 + [Math]::Floor($widgets.Count / 4) * 170)
                always_on_top = $true
                opacity_percent = 100
                locked = $true
                scale_percent = $ScalePercent
                width = [int][Math]::Round($plugin.defaultSize.width * $ScalePercent / 100)
                height = [int][Math]::Round($plugin.defaultSize.height * $ScalePercent / 100)
            }
            symbols = @($symbols[0..($SymbolCount - 1)] | ForEach-Object { "binance:spot:$_/USDT" })
            config = @{ theme = $theme }
        }
    }
}
$state = [ordered]@{
    settings = @{ language = "zh-CN"; show_main_window_on_startup = $false; shortcut = "disabled"; tray_icon_enabled = $false; auto_start_enabled = $false }
    selected_widget_id = $widgets[0].id
    next_widget_number = $widgets.Count + 1
    widgets = $widgets
}
[IO.File]::WriteAllText((Join-Path $stateDir "layouts.json"), ($state | ConvertTo-Json -Depth 10), [Text.UTF8Encoding]::new($false))
$variables = @{
    CRYPTO_HUD_STATE_DIR = $stateDir
    CRYPTO_HUD_GUI_SMOKE_READY_FILE = $readyFile
    CRYPTO_HUD_INSTANCE_ID = "com.crypto-hud.prices.$([guid]::NewGuid().ToString('N'))"
    CRYPTO_HUD_GUI_SMOKE_OFFLINE = "1"
    # Reuse the same deterministic BTC/ETH/SOL/BNB/PEPE prices as Focus Ticker.
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
        if ($LASTEXITCODE -ne 0) { throw "Widget build failed: $LASTEXITCODE" }
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
    # Keep the handle available for ExitCode after a short-lived smoke exits.
    $null = $app.Handle
    $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMs)
    while (-not (Test-Path -LiteralPath $readyFile)) {
        if ($app.HasExited) { throw "Widgets exited before ready. See $stateDir" }
        if ([DateTime]::UtcNow -gt $deadline) { throw "Widget readiness timed out. See $stateDir" }
        Start-Sleep -Milliseconds 100
    }
    $ready = Get-Content -LiteralPath $readyFile -Raw | ConvertFrom-Json
    if (-not $ready.ready -or -not $ready.marketDataReady -or $ready.widgetCount -ne $widgets.Count) {
        throw "Expected $($widgets.Count) native widgets with deterministic prices"
    }
    if (@($ready.catalogErrors).Count -ne 0) { throw "Plugin discovery failed: $($ready.catalogErrors)" }
    foreach ($widget in $ready.widgets) {
        $count = $expectedRows[$widget.id]
        if ($null -eq $count -or $widget.symbolCount -ne $count -or $widget.marketDataRowCount -ne $count) {
            throw "Unexpected price rows: $($widget | ConvertTo-Json -Compress)"
        }
        if ($widget.scalePercent -ne $ScalePercent -or $widget.runtimeWidth -le 0 -or $widget.runtimeHeight -le 0) {
            throw "Invalid widget size: $($widget | ConvertTo-Json -Compress)"
        }
    }
    if (-not $KeepOpen) {
        if (-not $app.WaitForExit($TimeoutMs + 5000)) { throw "Widgets did not exit after smoke timer" }
        if ($app.ExitCode -ne 0) { throw "Widgets exited with code $($app.ExitCode)" }
    }
    $succeeded = $true
    [pscustomobject]@{ Result = "passed"; PluginCount = $plugins.Count; WidgetCount = $widgets.Count; ProcessId = $app.Id; StateDir = $stateDir; ScalePercent = $ScalePercent; RequestedSymbols = $SymbolCount; FirstAsset = $FirstAsset; KeptOpen = [bool]$KeepOpen }
} finally {
    if ($app -and -not $app.HasExited -and (-not $KeepOpen -or -not $succeeded)) { Stop-Process -Id $app.Id }
    foreach ($key in $previous.Keys) { [Environment]::SetEnvironmentVariable($key, $previous[$key], "Process") }
    Pop-Location
}
