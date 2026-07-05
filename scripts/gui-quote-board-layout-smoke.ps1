param(
    [int]$TimeoutMs = 2600
)

$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$StateDir = Join-Path $RepoRoot ".gui-quote-board-layout-smoke-state"
$ReadyFile = Join-Path $StateDir "ready.json"
$StateFile = Join-Path $StateDir "layouts.json"

if (Test-Path $StateDir) {
    Remove-Item -LiteralPath $StateDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $StateDir | Out-Null

$seedWidgets = @(
    [ordered]@{
        id = "quote-board-full-1"
        plugin_id = "builtin.quote-board"
        name = "Quote Board Full"
        visible = $true
        layout = [ordered]@{
            x = 96
            y = 96
            always_on_top = $false
            opacity_percent = 96
            locked = $false
            width = 0
            height = 0
        }
        symbols = @("BTC", "ETH", "SOL", "BNB", "DOGE")
        config = [ordered]@{
            show_coin_logos = $true
            hide_quote_asset = $false
        }
    },
    [ordered]@{
        id = "quote-board-no-icon-2"
        plugin_id = "builtin.quote-board"
        name = "Quote Board No Icon"
        visible = $true
        layout = [ordered]@{
            x = 128
            y = 128
            always_on_top = $false
            opacity_percent = 96
            locked = $false
            width = 0
            height = 0
        }
        symbols = @("1000S")
        config = [ordered]@{
            show_coin_logos = $false
            hide_quote_asset = $false
        }
    },
    [ordered]@{
        id = "quote-board-no-quote-3"
        plugin_id = "builtin.quote-board"
        name = "Quote Board No Quote"
        visible = $true
        layout = [ordered]@{
            x = 160
            y = 160
            always_on_top = $false
            opacity_percent = 96
            locked = $false
            width = 0
            height = 0
        }
        symbols = @("BTC", "ETH")
        config = [ordered]@{
            show_coin_logos = $true
            hide_quote_asset = $true
        }
    },
    [ordered]@{
        id = "quote-board-compact-4"
        plugin_id = "builtin.quote-board"
        name = "Quote Board Compact"
        visible = $true
        layout = [ordered]@{
            x = 192
            y = 192
            always_on_top = $false
            opacity_percent = 96
            locked = $true
            width = 0
            height = 0
        }
        symbols = @("1000S", "ETH", "SOL", "BNB", "DOGE")
        config = [ordered]@{
            show_coin_logos = $false
            hide_quote_asset = $true
        }
    }
)

$seedState = [ordered]@{
    settings = [ordered]@{
        widgets_always_on_top = $false
        opacity_percent = 96
        widget_scale_percent = 100
    }
    selected_widget_id = "quote-board-full-1"
    next_widget_number = 5
    widgets = $seedWidgets
}
$seedJson = $seedState | ConvertTo-Json -Depth 8
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($StateFile, $seedJson, $utf8NoBom)

$env:CRYPTO_HUD_STATE_DIR = $StateDir
$env:CRYPTO_HUD_GUI_SMOKE_READY_FILE = $ReadyFile
$env:CRYPTO_HUD_INSTANCE_ID = "com.crypto-hud.gui-quote-board-layout-smoke.$PID"
$env:SLINT_BACKEND = "software"

function Assert-Close([double]$Actual, [double]$Expected, [double]$Tolerance, [string]$Label) {
    if ([Math]::Abs($Actual - $Expected) -gt $Tolerance) {
        throw "$Label expected $Expected, saw $Actual"
    }
}

function Assert-Widget(
    [object]$Widget,
    [int]$ExpectedWidth,
    [int]$ExpectedHeight,
    [int]$ExpectedSymbols,
    [bool]$ExpectedLocked
) {
    if ([int]$Widget.layoutWidth -ne $ExpectedWidth) {
        throw "Widget $($Widget.id) layout width expected $ExpectedWidth, saw $($Widget.layoutWidth)"
    }
    if ([int]$Widget.layoutHeight -ne $ExpectedHeight) {
        throw "Widget $($Widget.id) layout height expected $ExpectedHeight, saw $($Widget.layoutHeight)"
    }
    if ([int]$Widget.runtimeWidth -ne $ExpectedWidth) {
        throw "Widget $($Widget.id) runtime width expected $ExpectedWidth, saw $($Widget.runtimeWidth)"
    }
    if ([int]$Widget.runtimeHeight -ne $ExpectedHeight) {
        throw "Widget $($Widget.id) runtime height expected $ExpectedHeight, saw $($Widget.runtimeHeight)"
    }
    if ([int]$Widget.symbolCount -ne $ExpectedSymbols) {
        throw "Widget $($Widget.id) symbol count expected $ExpectedSymbols, saw $($Widget.symbolCount)"
    }
    if ([bool]$Widget.locked -ne $ExpectedLocked) {
        throw "Widget $($Widget.id) locked expected $ExpectedLocked, saw $($Widget.locked)"
    }
    if ([int]$Widget.scalePercent -ne 100) {
        throw "Widget $($Widget.id) scale_percent expected 100, saw $($Widget.scalePercent)"
    }
    Assert-Close ([double]$Widget.widgetScale) 1.0 0.01 "Widget $($Widget.id) scale"
}

Push-Location $RepoRoot
try {
    cargo run -p crypto-hud -- --widgets 4 --show-settings --gui-smoke-ms $TimeoutMs
    if ($LASTEXITCODE -ne 0) {
        throw "GUI quote board layout smoke process exited with code $LASTEXITCODE"
    }

    if (-not (Test-Path $ReadyFile)) {
        throw "GUI quote board layout smoke marker was not written: $ReadyFile"
    }

    $ready = Get-Content -LiteralPath $ReadyFile -Raw | ConvertFrom-Json
    if (-not $ready.ready) {
        throw "GUI quote board layout smoke marker did not report ready"
    }
    if ([int]$ready.widgetCount -ne 4) {
        throw "Expected 4 widgets, saw $($ready.widgetCount)"
    }
    if (-not $ready.settingsWindowRequested) {
        throw "Settings window was not requested during GUI quote board layout smoke"
    }

    $widgets = @{}
    foreach ($widget in @($ready.widgets)) {
        $widgets[$widget.id] = $widget
    }

    Assert-Widget $widgets["quote-board-full-1"] 286 194 5 $false
    Assert-Widget $widgets["quote-board-no-icon-2"] 274 80 1 $false
    Assert-Widget $widgets["quote-board-no-quote-3"] 246 101 2 $false
    Assert-Widget $widgets["quote-board-compact-4"] 224 194 5 $true
} finally {
    Pop-Location
    Remove-Item Env:\CRYPTO_HUD_STATE_DIR -ErrorAction SilentlyContinue
    Remove-Item Env:\CRYPTO_HUD_GUI_SMOKE_READY_FILE -ErrorAction SilentlyContinue
    Remove-Item Env:\CRYPTO_HUD_INSTANCE_ID -ErrorAction SilentlyContinue
}
