param(
    [ValidateRange(30, 200)][int]$ScalePercent = 100,
    [ValidateRange(1, 100)][int]$OpacityPercent = 100,
    [ValidateRange(1, 8)][int]$SymbolCount = 8,
    [ValidateRange(1, 60)][int]$SwitchIntervalSeconds = 5,
    [ValidateSet("BTC", "PEPE")][string]$FirstAsset = "BTC",
    [ValidateSet("both", "light", "dark")][string]$Theme = "both",
    [string]$Language = "zh-CN",
    [string]$IconCacheDir,
    [switch]$NoLogos,
    [switch]$RedUp,
    [switch]$Locked,
    [switch]$SkipBuild,
    [switch]$KeepOpen,
    [ValidateRange(5000, 3600000)][int]$TimeoutMs = 15000
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$stateDir = Join-Path $repoRoot ("target\tmp\market-compass-smoke-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $stateDir -Force | Out-Null
$readyFile = Join-Path $stateDir "ready.json"
$symbols = @($FirstAsset) + @("BTC", "ETH", "SOL", "BNB", "XRP", "DOGE", "ADA", "PEPE" | Where-Object { $_ -ne $FirstAsset })
$themes = if ($Theme -eq "both") { @("light", "dark") } else { @($Theme) }
$size = [int][Math]::Round(480 * $ScalePercent / 100, [MidpointRounding]::AwayFromZero)
$widgets = @()
foreach ($item in $themes) {
    $widgets += [ordered]@{
        id = "market-compass-$item"
        plugin_id = "com.cryptohud.market-compass"
        name = "Market Compass $item"
        visible = $true
        layout = [ordered]@{
            x = 80 + $widgets.Count * ($size + 40)
            y = 100
            always_on_top = $true
            opacity_percent = $OpacityPercent
            locked = [bool]$Locked
            scale_percent = $ScalePercent
            width = $size
            height = $size
        }
        symbols = @($symbols[0..($SymbolCount - 1)] | ForEach-Object { "binance:spot:$_/USDT" })
        config = @{
            theme = $item
            show_coin_logos = -not [bool]$NoLogos
            plugin_parameters = @{ "switch-interval-seconds" = $SwitchIntervalSeconds }
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
if ($IconCacheDir) {
    $iconSource = (Resolve-Path -LiteralPath $IconCacheDir).Path
    $iconTarget = Join-Path $stateDir "coin-icons"
    New-Item -ItemType Directory -Path $iconTarget -Force | Out-Null
    # Only copy named public coin artwork, never the caller's configuration.
    foreach ($asset in $symbols) {
        $icon = Join-Path $iconSource ($asset.ToLowerInvariant() + ".svg")
        if (Test-Path -LiteralPath $icon -PathType Leaf) {
            Copy-Item -LiteralPath $icon -Destination $iconTarget
        }
    }
}
$variables = @{
    CRYPTO_HUD_STATE_DIR = $stateDir
    CRYPTO_HUD_GUI_SMOKE_READY_FILE = $readyFile
    CRYPTO_HUD_INSTANCE_ID = "com.crypto-hud.compass-smoke.$([guid]::NewGuid().ToString('N'))"
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
        if ($LASTEXITCODE -ne 0) { throw "Market Compass build failed: $LASTEXITCODE" }
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
        if ($app.HasExited) { throw "Market Compass exited before ready. See $stateDir" }
        if ([DateTime]::UtcNow -gt $deadline) { throw "Market Compass readiness timed out. See $stateDir" }
        Start-Sleep -Milliseconds 100
    }
    $ready = Get-Content -LiteralPath $readyFile -Raw | ConvertFrom-Json
    if (-not $ready.ready -or -not $ready.marketDataReady -or $ready.widgetCount -ne $widgets.Count) {
        throw "Expected $($widgets.Count) native Market Compass windows with deterministic prices"
    }
    if (@($ready.catalogErrors).Count -ne 0) { throw "Plugin discovery failed: $($ready.catalogErrors)" }
    foreach ($widget in $ready.widgets) {
        if ($widget.pluginId -ne "com.cryptohud.market-compass" -or $widget.symbolCount -ne $SymbolCount -or $widget.marketDataRowCount -ne $SymbolCount) {
            throw "Market Compass did not populate $SymbolCount rows: $($widget | ConvertTo-Json -Compress)"
        }
        if ($widget.layoutWidth -ne $size -or $widget.layoutHeight -ne $size -or $widget.scalePercent -ne $ScalePercent) {
            throw "Market Compass logical dimensions do not match the requested scale"
        }
        $dpiScale = [double]$widget.windowScaleFactor
        if ($dpiScale -le 0 -or [Math]::Abs($widget.runtimeWidth - $size * $dpiScale) -gt 1 -or [Math]::Abs($widget.runtimeHeight - $size * $dpiScale) -gt 1) {
            throw "Market Compass physical dimensions do not match the desktop DPI"
        }
    }
    if (-not $KeepOpen) {
        if (-not $app.WaitForExit($TimeoutMs + 5000)) { throw "Market Compass did not exit after the smoke timer" }
        if ($app.ExitCode -ne 0) { throw "Market Compass exited with code $($app.ExitCode)" }
    }
    $succeeded = $true
    [pscustomobject]@{
        Result = "passed"
        ProcessId = $app.Id
        StateDir = $stateDir
        ScalePercent = $ScalePercent
        SymbolCount = $SymbolCount
        Theme = $Theme
        FirstAsset = $FirstAsset
        LogosEnabled = -not [bool]$NoLogos
        KeptOpen = [bool]$KeepOpen
    }
} finally {
    if ($app -and -not $app.HasExited -and (-not $KeepOpen -or -not $succeeded)) { Stop-Process -Id $app.Id }
    foreach ($key in $previous.Keys) { [Environment]::SetEnvironmentVariable($key, $previous[$key], "Process") }
    Pop-Location
}
