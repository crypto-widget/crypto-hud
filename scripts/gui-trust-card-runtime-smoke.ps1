param(
    [ValidateRange(30, 200)][int]$ScalePercent = 100,
    [ValidateRange(1, 100)][int]$OpacityPercent = 100,
    [ValidateRange(1, 8)][int]$SymbolCount = 1,
    [ValidateSet("BTC", "ETH", "SOL", "BNB", "PEPE")][string]$FirstAsset = "BTC",
    [ValidateSet("both", "light", "dark")][string]$Theme = "both",
    [string]$Language = "zh-CN",
    [string]$IconCacheDir,
    [switch]$PreviewMatrix,
    [switch]$NoLogos,
    [switch]$RedUp,
    [switch]$Locked,
    [switch]$SkipBuild,
    [switch]$KeepOpen,
    [ValidateRange(5000, 3600000)][int]$TimeoutMs = 15000
)

# Native loading, saved-layout normalization, and DPI geometry. This smoke
# reads the app's ready marker; it does not capture or automate desktop input.
$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$stateDir = Join-Path $repoRoot ("target\tmp\trust-card-runtime-smoke-" + [guid]::NewGuid().ToString("N"))
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
$assets = if ($PreviewMatrix) { @("BTC", "PEPE") } else { @($FirstAsset) }
$expectedWidth = [int][Math]::Round(260 * $ScalePercent / 100, [MidpointRounding]::AwayFromZero)
$expectedHeight = [int][Math]::Round(180 * $ScalePercent / 100, [MidpointRounding]::AwayFromZero)
$widgets = @()
foreach ($asset in $assets) {
    $symbols = @($asset) + @("BTC", "ETH", "SOL", "BNB", "PEPE", "DOGE", "ADA", "XRP" | Where-Object { $_ -ne $asset })
    foreach ($item in $themes) {
        $widgets += [ordered]@{
            id = "trust-card-$($asset.ToLowerInvariant())-$item"
            plugin_id = "com.cryptohud.trust-card"
            name = "Trust Card $asset $item"
            visible = $true
            layout = [ordered]@{
                x = 80 + [array]::IndexOf($themes, $item) * ($expectedWidth + 40)
                y = 80 + [array]::IndexOf($assets, $asset) * ($expectedHeight + 40)
                always_on_top = $true
                opacity_percent = $OpacityPercent
                locked = [bool]$Locked
                scale_percent = $ScalePercent
                # Seed the old card dimensions to check real saved-layout migration.
                width = [int][Math]::Round(520 * $ScalePercent / 100, [MidpointRounding]::AwayFromZero)
                height = [int][Math]::Round(386 * $ScalePercent / 100, [MidpointRounding]::AwayFromZero)
            }
            symbols = @($symbols[0..($SymbolCount - 1)] | ForEach-Object { "binance:spot:$_/USDT" })
            config = @{ theme = $item; show_coin_logos = -not [bool]$NoLogos }
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
    # Copy only the requested public artwork, never user settings.
    foreach ($asset in $assets) {
        foreach ($extension in @("svg", "png")) {
            $icon = Join-Path $iconSource ($asset.ToLowerInvariant() + "." + $extension)
            if (Test-Path -LiteralPath $icon -PathType Leaf) {
                Copy-Item -LiteralPath $icon -Destination $iconTarget
            }
        }
    }
}
$variables = @{
    CRYPTO_HUD_STATE_DIR = $stateDir
    CRYPTO_HUD_GUI_SMOKE_READY_FILE = $readyFile
    CRYPTO_HUD_INSTANCE_ID = "com.crypto-hud.trust-card-runtime-smoke.$([guid]::NewGuid().ToString('N'))"
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
        if ($LASTEXITCODE -ne 0) { throw "Trust Card build failed: $LASTEXITCODE" }
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
    # Retain the process handle so Windows PowerShell can read ExitCode after
    # the short smoke process exits instead of returning a null exit code.
    $null = $app.Handle
    $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMs)
    while (-not (Test-Path -LiteralPath $readyFile)) {
        if ($app.HasExited) { throw "Trust Card exited before ready. See $stateDir" }
        if ([DateTime]::UtcNow -gt $deadline) { throw "Trust Card readiness timed out. See $stateDir" }
        Start-Sleep -Milliseconds 100
    }
    $ready = Get-Content -LiteralPath $readyFile -Raw | ConvertFrom-Json
    if (-not $ready.ready -or -not $ready.marketDataReady -or $ready.widgetCount -ne $widgets.Count -or @($ready.widgets).Count -ne $widgets.Count) {
        throw "Expected $($widgets.Count) native Trust Card windows with deterministic prices"
    }
    if (@($ready.pluginIds) -notcontains "com.cryptohud.trust-card") { throw "Trust Card was missing from the plugin catalog" }
    if (@($ready.catalogErrors).Count -ne 0) { throw "Plugin discovery failed: $($ready.catalogErrors)" }
    foreach ($widget in $ready.widgets) {
        if (@($widgets.id) -notcontains $widget.id -or $widget.pluginId -ne "com.cryptohud.trust-card" -or $widget.symbolCount -ne 1 -or $widget.marketDataRowCount -ne 1) {
            throw "Trust Card did not normalize $SymbolCount requested symbols to one populated row"
        }
        if ([bool]$widget.locked -ne [bool]$Locked) { throw "Trust Card did not preserve the requested lock state" }
        if ($widget.layoutWidth -ne $expectedWidth -or $widget.layoutHeight -ne $expectedHeight -or $widget.scalePercent -ne $ScalePercent) {
            throw "Trust Card saved-layout size migration failed"
        }
        $dpiScale = [double]$widget.windowScaleFactor
        if ($dpiScale -le 0 -or [Math]::Abs($widget.runtimeWidth - $expectedWidth * $dpiScale) -gt 1 -or [Math]::Abs($widget.runtimeHeight - $expectedHeight * $dpiScale) -gt 1) {
            throw "Trust Card physical dimensions do not match the desktop DPI"
        }
    }
    if (-not $KeepOpen) {
        if (-not $app.WaitForExit($TimeoutMs + 5000)) { throw "Trust Card did not exit after the smoke timer" }
        if ($app.ExitCode -ne 0) { throw "Trust Card exited with code $($app.ExitCode)" }
    }
    $succeeded = $true
    [pscustomobject]@{
        Result = "passed"; ProcessId = $app.Id; StateDir = $stateDir
        ScalePercent = $ScalePercent; Theme = $Theme; FirstAsset = $FirstAsset
        RequestedSymbols = $SymbolCount; PreviewMatrix = [bool]$PreviewMatrix
        LogosEnabled = -not [bool]$NoLogos; KeptOpen = [bool]$KeepOpen
    }
} finally {
    if ($app -and -not $app.HasExited -and (-not $KeepOpen -or -not $succeeded)) { Stop-Process -Id $app.Id }
    foreach ($key in $previous.Keys) { [Environment]::SetEnvironmentVariable($key, $previous[$key], "Process") }
    Pop-Location
}
