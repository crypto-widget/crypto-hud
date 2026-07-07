param(
    [int]$TimeoutMs = 2600
)

$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$StateDir = Join-Path $RepoRoot ".gui-dynamic-layout-smoke-state"
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
            width = 274
            height = 80
        }
        symbols = @("BTC")
        config = [ordered]@{
            show_coin_logos = $false
            hide_quote_asset = $false
        }
    },
    [ordered]@{
        id = "plugin-ticker-2"
        plugin_id = "com.cryptohud.focus-ticker"
        name = "Focus Ticker 2"
        visible = $true
        layout = [ordered]@{
            x = 130
            y = 130
            always_on_top = $false
            opacity_percent = 96
            locked = $false
            width = 1025
            height = 195
        }
        symbols = @("ETH")
        config = [ordered]@{}
    },
    [ordered]@{
        id = "plugin-strip-3"
        plugin_id = "com.cryptohud.status-strip"
        name = "Status Strip 3"
        visible = $true
        layout = [ordered]@{
            x = 164
            y = 164
            always_on_top = $false
            opacity_percent = 96
            locked = $true
            width = 624
            height = 138
        }
        symbols = @("BTC", "ETH", "SOL")
        config = [ordered]@{}
    },
    [ordered]@{
        id = "plugin-card-4"
        plugin_id = "com.cryptohud.trust-card"
        name = "Trust Card 4"
        visible = $true
        layout = [ordered]@{
            x = 198
            y = 198
            always_on_top = $false
            opacity_percent = 96
            locked = $false
            width = 520
            height = 410
        }
        symbols = @("BTC")
        config = [ordered]@{}
    }
)

$seedState = [ordered]@{
    settings = [ordered]@{
        widgets_always_on_top = $false
        opacity_percent = 96
    }
    selected_widget_id = "quote-board-1"
    next_widget_number = 5
    widgets = $seedWidgets
}
$seedJson = $seedState | ConvertTo-Json -Depth 8
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($StateFile, $seedJson, $utf8NoBom)

$env:CRYPTO_HUD_STATE_DIR = $StateDir
$env:CRYPTO_HUD_GUI_SMOKE_READY_FILE = $ReadyFile
$env:CRYPTO_HUD_INSTANCE_ID = "com.crypto-hud.gui-dynamic-layout-smoke.$PID"
$env:SLINT_BACKEND = "software"

function Assert-Close([double]$Actual, [double]$Expected, [double]$Tolerance, [string]$Label) {
    if ([Math]::Abs($Actual - $Expected) -gt $Tolerance) {
        throw "$Label expected $Expected, saw $Actual"
    }
}

function Assert-Widget(
    [object]$Widget,
    [int]$LayoutWidth,
    [int]$LayoutHeight,
    [int]$SymbolCount,
    [double]$WidgetScale,
    [int]$ScalePercent
) {
    if ([int]$Widget.layoutWidth -ne $LayoutWidth) {
        throw "Widget $($Widget.id) layout width expected $LayoutWidth, saw $($Widget.layoutWidth)"
    }
    if ([int]$Widget.layoutHeight -ne $LayoutHeight) {
        throw "Widget $($Widget.id) layout height expected $LayoutHeight, saw $($Widget.layoutHeight)"
    }
    if ([int]$Widget.runtimeWidth -ne $LayoutWidth) {
        throw "Widget $($Widget.id) runtime width expected $LayoutWidth, saw $($Widget.runtimeWidth)"
    }
    if ([int]$Widget.runtimeHeight -ne $LayoutHeight) {
        throw "Widget $($Widget.id) runtime height expected $LayoutHeight, saw $($Widget.runtimeHeight)"
    }
    if ([int]$Widget.symbolCount -ne $SymbolCount) {
        throw "Widget $($Widget.id) symbol count expected $SymbolCount, saw $($Widget.symbolCount)"
    }
    if ([int]$Widget.scalePercent -ne $ScalePercent) {
        throw "Widget $($Widget.id) scale_percent expected $ScalePercent, saw $($Widget.scalePercent)"
    }
    Assert-Close ([double]$Widget.widgetScale) $WidgetScale 0.01 "Widget $($Widget.id) scale"
}

Push-Location $RepoRoot
try {
    cargo run -p crypto-hud -- --widgets 4 --show-settings --gui-smoke-ms $TimeoutMs
    if ($LASTEXITCODE -ne 0) {
        throw "GUI dynamic layout smoke process exited with code $LASTEXITCODE"
    }

    if (-not (Test-Path $ReadyFile)) {
        throw "GUI dynamic layout smoke marker was not written: $ReadyFile"
    }

    $ready = Get-Content -LiteralPath $ReadyFile -Raw | ConvertFrom-Json
    if (-not $ready.ready) {
        throw "GUI dynamic layout smoke marker did not report ready"
    }
    if ([int]$ready.widgetCount -ne 4) {
        throw "Expected 4 widgets, saw $($ready.widgetCount)"
    }
    if (-not $ready.settingsWindowRequested) {
        throw "Settings window was not requested during GUI dynamic layout smoke"
    }

    $widgets = @{}
    foreach ($widget in @($ready.widgets)) {
        $widgets[$widget.id] = $widget
    }

    foreach ($id in @("quote-board-1", "plugin-ticker-2", "plugin-strip-3", "plugin-card-4")) {
        if (-not $widgets.ContainsKey($id)) {
            throw "Ready marker did not include $id"
        }
    }

    Assert-Widget $widgets["quote-board-1"] 274 80 1 1.00 100
    Assert-Widget $widgets["plugin-ticker-2"] 1025 195 1 1.25 125
    Assert-Widget $widgets["plugin-strip-3"] 624 138 3 1.50 150
    Assert-Widget $widgets["plugin-card-4"] 520 386 1 1.00 100
} finally {
    Pop-Location
    Remove-Item Env:\CRYPTO_HUD_STATE_DIR -ErrorAction SilentlyContinue
    Remove-Item Env:\CRYPTO_HUD_GUI_SMOKE_READY_FILE -ErrorAction SilentlyContinue
    Remove-Item Env:\CRYPTO_HUD_INSTANCE_ID -ErrorAction SilentlyContinue
}
