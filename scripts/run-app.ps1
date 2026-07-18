param(
    [string]$StateDir = "target\tmp\demo-run-state",
    [switch]$KeepState,
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$AppArgs = @()
)

$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$statePath = Join-Path $RepoRoot $StateDir
$fullStatePath = [System.IO.Path]::GetFullPath($statePath)
$fullRepoPath = [System.IO.Path]::GetFullPath($RepoRoot)
if (-not $fullRepoPath.EndsWith([System.IO.Path]::DirectorySeparatorChar)) {
    $fullRepoPath = "$fullRepoPath$([System.IO.Path]::DirectorySeparatorChar)"
}
if (-not $fullStatePath.StartsWith($fullRepoPath, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to use state directory outside repository: $fullStatePath"
}
if (-not $KeepState -and (Test-Path -LiteralPath $statePath)) {
    Remove-Item -LiteralPath $statePath -Recurse -Force
}
$env:CRYPTO_HUD_STATE_DIR = $fullStatePath
$taskbarDllPath = Join-Path $RepoRoot "target\debug\crypto_hud_taskbar.dll"
$originalTaskbarDllPresent = Test-Path "Env:\CRYPTO_HUD_TASKBAR_DLL"
$originalTaskbarDll = if ($originalTaskbarDllPresent) {
    (Get-Item "Env:\CRYPTO_HUD_TASKBAR_DLL").Value
} else {
    ""
}

Push-Location $RepoRoot
try {
    & (Join-Path $PSScriptRoot "build-taskbar-extension.ps1")
    if (-not (Test-Path -LiteralPath $taskbarDllPath -PathType Leaf)) {
        throw "Taskbar extension DLL was not created: $taskbarDllPath"
    }
    $env:CRYPTO_HUD_TASKBAR_DLL = $taskbarDllPath

    cargo run --locked -p crypto-hud -- --each-widget @AppArgs
    if ($LASTEXITCODE -ne 0) {
        throw "App exited with code $LASTEXITCODE"
    }
} finally {
    Pop-Location
    Remove-Item Env:\CRYPTO_HUD_STATE_DIR -ErrorAction SilentlyContinue
    Remove-Item Env:\CRYPTO_HUD_TASKBAR_DLL -ErrorAction SilentlyContinue
    if ($originalTaskbarDllPresent) {
        Set-Item Env:\CRYPTO_HUD_TASKBAR_DLL $originalTaskbarDll
    }
}
