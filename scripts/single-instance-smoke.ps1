param(
    [int]$TimeoutMs = 7000,
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$Exe = Join-Path $RepoRoot "target\debug\crypto-hud.exe"
$StateDir = Join-Path $RepoRoot "target\tmp\single-instance-smoke-state"
$ReadyFile = Join-Path $StateDir "ready.json"
$ActivationFile = Join-Path $StateDir "activated.txt"
$StateFile = Join-Path $StateDir "layouts.json"
$InstanceId = "com.crypto-hud.single-instance-smoke.$PID"
$process = $null

function Wait-ForFile {
    param(
        [string]$Path,
        [int]$Timeout
    )

    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    while (-not (Test-Path -LiteralPath $Path)) {
        if ($watch.ElapsedMilliseconds -gt $Timeout) {
            throw "Timed out waiting for $Path"
        }
        Start-Sleep -Milliseconds 50
    }
}

if (Test-Path -LiteralPath $StateDir) {
    Remove-Item -LiteralPath $StateDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $StateDir | Out-Null

$seedState = [ordered]@{
    settings = [ordered]@{
        show_main_window_on_startup = $false
        shortcut = "disabled"
        tray_icon_enabled = $false
        auto_start_enabled = $false
    }
    selected_widget_id = "quote-board-1"
    next_widget_number = 2
    widgets = @(
        [ordered]@{
            id = "quote-board-1"
            plugin_id = "builtin.quote-board"
            name = "Quote Board 1"
            visible = $true
            layout = [ordered]@{
                x = 96
                y = 96
                always_on_top = $true
                opacity_percent = 96
                locked = $false
                scale_percent = 100
                width = 286
                height = 194
            }
            symbols = @("binance:spot:BTC/USDT")
            config = [ordered]@{}
        }
    )
}
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText(
    $StateFile,
    ($seedState | ConvertTo-Json -Depth 8),
    $utf8NoBom
)

Push-Location $RepoRoot
try {
    if (-not $SkipBuild) {
        cargo build -p crypto-hud
        if ($LASTEXITCODE -ne 0) {
            throw "Debug build failed with code $LASTEXITCODE"
        }
    }
    if (-not (Test-Path -LiteralPath $Exe)) {
        throw "Application executable not found: $Exe"
    }

    $first = [System.Diagnostics.ProcessStartInfo]::new()
    $first.FileName = $Exe
    $first.Arguments = "--gui-smoke-ms $TimeoutMs"
    $first.WorkingDirectory = $RepoRoot
    $first.UseShellExecute = $false
    $first.Environment["CRYPTO_HUD_STATE_DIR"] = $StateDir
    $first.Environment["CRYPTO_HUD_GUI_SMOKE_READY_FILE"] = $ReadyFile
    $first.Environment["CRYPTO_HUD_GUI_SMOKE_ACTIVATION_FILE"] = $ActivationFile
    $first.Environment["CRYPTO_HUD_INSTANCE_ID"] = $InstanceId
    $first.Environment["CRYPTO_HUD_DISABLE_UPDATE_CHECK"] = "1"
    $first.Environment["CRYPTO_HUD_GUI_SMOKE_OFFLINE"] = "1"
    $first.Environment["SLINT_BACKEND"] = "software"
    $process = [System.Diagnostics.Process]::Start($first)

    Wait-ForFile -Path $ReadyFile -Timeout 5000
    $ready = Get-Content -LiteralPath $ReadyFile -Raw | ConvertFrom-Json
    if (-not [bool]$ready.marketDataReady) {
        throw "Primary instance marker did not report market data ready"
    }
    if ($process.HasExited) {
        throw "Primary instance exited before activation with code $($process.ExitCode)"
    }

    $second = [System.Diagnostics.ProcessStartInfo]::new()
    $second.FileName = $Exe
    $second.WorkingDirectory = $RepoRoot
    $second.UseShellExecute = $false
    $second.Environment["CRYPTO_HUD_STATE_DIR"] = $StateDir
    $second.Environment["CRYPTO_HUD_INSTANCE_ID"] = $InstanceId
    $second.Environment["CRYPTO_HUD_DISABLE_UPDATE_CHECK"] = "1"
    $second.Environment["CRYPTO_HUD_GUI_SMOKE_OFFLINE"] = "1"
    $second.Environment["SLINT_BACKEND"] = "software"
    $secondaryProcess = [System.Diagnostics.Process]::Start($second)
    if (-not $secondaryProcess.WaitForExit(5000)) {
        $secondaryProcess.Kill()
        throw "Secondary instance did not exit after signaling the primary instance"
    }
    if ($secondaryProcess.ExitCode -ne 0) {
        throw "Secondary instance exited with code $($secondaryProcess.ExitCode)"
    }

    Wait-ForFile -Path $ActivationFile -Timeout 3000
    Write-Host "Single-instance activation smoke passed"
} finally {
    Pop-Location
    if ($process -and -not $process.HasExited) {
        $process.Kill()
        $process.WaitForExit()
    }
}
