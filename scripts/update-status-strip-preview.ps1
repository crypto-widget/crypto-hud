param()

$ErrorActionPreference = "Stop"
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$SnapshotPath = Join-Path $RepoRoot "target\tmp\status-strip-preview.rgba"
$PreviewPath = Join-Path $RepoRoot "crates\crypto-hud\plugins\com.cryptohud.status-strip\ui\preview-light.png"
$OriginalPreviewFlag = $env:CRYPTO_HUD_UPDATE_STATUS_STRIP_PREVIEW

Push-Location $RepoRoot
try {
    $env:CRYPTO_HUD_UPDATE_STATUS_STRIP_PREVIEW = "1"
    cargo +1.96.0 test --locked -p crypto-hud status_strip_keeps_rounded_corners_in_software_snapshots -- --nocapture
    if ($LASTEXITCODE -ne 0) {
        throw "Status Strip preview rendering test failed with code $LASTEXITCODE"
    }
    $bytes = [System.IO.File]::ReadAllBytes($SnapshotPath)
    $width = [BitConverter]::ToUInt32($bytes, 0)
    $height = [BitConverter]::ToUInt32($bytes, 4)
    if ($width -ne 374 -or $height -ne 92 -or $bytes.Length -ne 8 + $width * $height * 4) {
        throw "Unexpected Status Strip preview dimensions or pixel buffer length"
    }
    Add-Type -AssemblyName System.Drawing
    $bitmap = [System.Drawing.Bitmap]::new([int]$width, [int]$height, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    try {
        $bgra = [byte[]]::new($bytes.Length - 8)
        for ($offset = 0; $offset -lt $bgra.Length; $offset += 4) {
            $bgra[$offset] = $bytes[$offset + 10]
            $bgra[$offset + 1] = $bytes[$offset + 9]
            $bgra[$offset + 2] = $bytes[$offset + 8]
            $bgra[$offset + 3] = $bytes[$offset + 11]
        }
        $data = $bitmap.LockBits(
            [System.Drawing.Rectangle]::new(0, 0, [int]$width, [int]$height),
            [System.Drawing.Imaging.ImageLockMode]::WriteOnly,
            [System.Drawing.Imaging.PixelFormat]::Format32bppArgb
        )
        try {
            if ($data.Stride -ne $width * 4) { throw "Unexpected bitmap stride" }
            [System.Runtime.InteropServices.Marshal]::Copy($bgra, 0, $data.Scan0, $bgra.Length)
        } finally { $bitmap.UnlockBits($data) }
        $bitmap.Save($PreviewPath, [System.Drawing.Imaging.ImageFormat]::Png)
    } finally { $bitmap.Dispose() }
    Write-Host "Updated Status Strip preview from an alpha-preserving software-renderer snapshot"
} finally {
    $env:CRYPTO_HUD_UPDATE_STATUS_STRIP_PREVIEW = $OriginalPreviewFlag
    Pop-Location
}
