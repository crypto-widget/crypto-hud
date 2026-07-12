param(
    [int]$TimeoutMs = 2600
)

$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$StateDir = Join-Path $RepoRoot "target\tmp\gui-settings-interaction-smoke-state"
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
            scale_percent = 150
            width = 429
            height = 152
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

$env:CRYPTO_HUD_STATE_DIR = $StateDir
$env:CRYPTO_HUD_GUI_SMOKE_READY_FILE = $ReadyFile
$env:CRYPTO_HUD_GUI_SMOKE_SETTINGS_INTERACTION = "1"
$env:CRYPTO_HUD_INSTANCE_ID = "com.crypto-hud.gui-settings-interaction-smoke.$PID"
$env:CRYPTO_HUD_GUI_SMOKE_OFFLINE = "1"
$env:CRYPTO_HUD_DISABLE_UPDATE_CHECK = "1"
$env:SLINT_BACKEND = "software"

function Assert-Close([double]$Actual, [double]$Expected, [double]$Tolerance, [string]$Label) {
    if ([Math]::Abs($Actual - $Expected) -gt $Tolerance) {
        throw "$Label expected $Expected, saw $Actual"
    }
}

Push-Location $RepoRoot
try {
    cargo run -p crypto-hud -- --widgets 1 --show-settings --gui-smoke-ms $TimeoutMs
    if ($LASTEXITCODE -ne 0) {
        throw "GUI settings interaction smoke process exited with code $LASTEXITCODE"
    }

    if (-not (Test-Path $ReadyFile)) {
        throw "GUI settings interaction smoke marker was not written: $ReadyFile"
    }

    $ready = Get-Content -LiteralPath $ReadyFile -Raw | ConvertFrom-Json
    if (-not $ready.ready) {
        throw "GUI settings interaction smoke marker did not report ready"
    }
    if (-not $ready.marketDataReady) {
        throw "GUI settings interaction smoke marker did not report market data ready"
    }
    if ([int]$ready.widgetCount -ne 1) {
        throw "Expected 1 widget, saw $($ready.widgetCount)"
    }
    if (-not $ready.settingsWindowRequested) {
        throw "Settings window was not requested during GUI settings interaction smoke"
    }

    $widget = @($ready.widgets)[0]
    if ($widget.id -ne "quote-board-1") {
        throw "Expected quote-board-1, saw $($widget.id)"
    }
    if ([int]$widget.layoutWidth -ne 336) {
        throw "Widget layout width expected 336, saw $($widget.layoutWidth)"
    }
    if ([int]$widget.layoutHeight -ne 152) {
        throw "Widget layout height expected 152, saw $($widget.layoutHeight)"
    }
    if ([int]$widget.runtimeWidth -ne 336) {
        throw "Widget runtime width expected 336, saw $($widget.runtimeWidth)"
    }
    if ([int]$widget.runtimeHeight -ne 152) {
        throw "Widget runtime height expected 152, saw $($widget.runtimeHeight)"
    }
    if ([int]$widget.symbolCount -ne 2) {
        throw "Widget symbol count expected 2, saw $($widget.symbolCount)"
    }
    if ([int]$widget.scalePercent -ne 150) {
        throw "Widget scale_percent expected 150, saw $($widget.scalePercent)"
    }
    Assert-Close ([double]$widget.widgetScale) 1.5 0.01 "Widget scale"
} finally {
    Pop-Location
    Remove-Item Env:\CRYPTO_HUD_STATE_DIR -ErrorAction SilentlyContinue
    Remove-Item Env:\CRYPTO_HUD_GUI_SMOKE_READY_FILE -ErrorAction SilentlyContinue
    Remove-Item Env:\CRYPTO_HUD_GUI_SMOKE_SETTINGS_INTERACTION -ErrorAction SilentlyContinue
    Remove-Item Env:\CRYPTO_HUD_INSTANCE_ID -ErrorAction SilentlyContinue
    Remove-Item Env:\CRYPTO_HUD_GUI_SMOKE_OFFLINE -ErrorAction SilentlyContinue
    Remove-Item Env:\CRYPTO_HUD_DISABLE_UPDATE_CHECK -ErrorAction SilentlyContinue
}
