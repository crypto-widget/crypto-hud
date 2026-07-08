param(
    [int]$Widgets = 2,
    [int]$TimeoutMs = 2200
)

$ErrorActionPreference = "Stop"
$Widgets = [Math]::Max($Widgets, 1)

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$StateDir = Join-Path $RepoRoot "target\tmp\gui-smoke-state"
$ReadyFile = Join-Path $StateDir "ready.json"
$StateFile = Join-Path $StateDir "layouts.json"

if (Test-Path $StateDir) {
    Remove-Item -LiteralPath $StateDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $StateDir | Out-Null

$seedWidgets = @()
for ($i = 1; $i -le $Widgets; $i++) {
    $isMiniTicker = ($i % 2 -eq 0)
    $pluginId = if ($isMiniTicker) { "builtin.mini-ticker" } else { "builtin.quote-board" }
    $prefix = if ($isMiniTicker) { "mini-ticker" } else { "quote-board" }
    $width = if ($isMiniTicker) { 260 } else { 334 }
    $height = if ($isMiniTicker) { 124 } else { 244 }
    $seedWidgets += [ordered]@{
        id = "$prefix-$i"
        plugin_id = $pluginId
        name = "Smoke Widget $i"
        visible = $true
        layout = [ordered]@{
            x = 96 + (($i - 1) * 28)
            y = 96 + (($i - 1) * 32)
            always_on_top = $false
            opacity_percent = 96
            locked = ($i -eq 1)
            width = $width
            height = $height
        }
        symbols = @("BTC", "ETH")
        config = [ordered]@{}
    }
}

$seedState = [ordered]@{
    settings = [ordered]@{
        widgets_always_on_top = $false
        opacity_percent = 96
    }
    selected_widget_id = $seedWidgets[0].id
    next_widget_number = $Widgets + 1
    widgets = $seedWidgets
}
$seedJson = $seedState | ConvertTo-Json -Depth 8
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($StateFile, $seedJson, $utf8NoBom)

$env:CRYPTO_HUD_STATE_DIR = $StateDir
$env:CRYPTO_HUD_GUI_SMOKE_READY_FILE = $ReadyFile
$env:CRYPTO_HUD_INSTANCE_ID = "com.crypto-hud.gui-smoke.$PID"
$env:SLINT_BACKEND = "software"

Push-Location $RepoRoot
try {
    cargo run -p crypto-hud -- --widgets $Widgets --show-settings --gui-smoke-ms $TimeoutMs
    if ($LASTEXITCODE -ne 0) {
        throw "GUI smoke process exited with code $LASTEXITCODE"
    }

    if (-not (Test-Path $ReadyFile)) {
        throw "GUI smoke marker was not written: $ReadyFile"
    }

    $ready = Get-Content -LiteralPath $ReadyFile -Raw | ConvertFrom-Json
    if (-not $ready.ready) {
        throw "GUI smoke marker did not report ready"
    }
    if ([int]$ready.widgetCount -lt $Widgets) {
        throw "Expected at least $Widgets widgets, saw $($ready.widgetCount)"
    }
    if (-not $ready.settingsWindowRequested) {
        throw "Settings window was not requested during GUI smoke"
    }
    $readyWidgets = @($ready.widgets)
    if ($readyWidgets.Count -ne [int]$ready.widgetCount) {
        throw "Ready marker widget details did not match widgetCount"
    }
    if (-not ($readyWidgets | Where-Object { [bool]$_.locked })) {
        throw "GUI smoke did not observe any locked widget layout"
    }
    if ($Widgets -gt 1 -and -not ($readyWidgets | Where-Object { -not [bool]$_.locked })) {
        throw "GUI smoke did not observe any unlocked widget layout"
    }
    foreach ($widget in $readyWidgets) {
        if ([int]$widget.layoutWidth -lt 160 -or [int]$widget.layoutHeight -lt 80) {
            throw "Widget $($widget.id) reported an invalid persisted size"
        }
        if ([int]$widget.scalePercent -le 0) {
            throw "Widget $($widget.id) did not report a persisted scale_percent"
        }
        if ([Math]::Abs([int]$widget.runtimeWidth - [int]$widget.layoutWidth) -gt 1) {
            throw "Widget $($widget.id) runtime width $($widget.runtimeWidth) did not match layout width $($widget.layoutWidth)"
        }
        if ([Math]::Abs([int]$widget.runtimeHeight - [int]$widget.layoutHeight) -gt 1) {
            throw "Widget $($widget.id) runtime height $($widget.runtimeHeight) did not match layout height $($widget.layoutHeight)"
        }
    }
} finally {
    Pop-Location
    Remove-Item Env:\CRYPTO_HUD_STATE_DIR -ErrorAction SilentlyContinue
    Remove-Item Env:\CRYPTO_HUD_GUI_SMOKE_READY_FILE -ErrorAction SilentlyContinue
    Remove-Item Env:\CRYPTO_HUD_INSTANCE_ID -ErrorAction SilentlyContinue
}
