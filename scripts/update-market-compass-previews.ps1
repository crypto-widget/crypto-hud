param()

$ErrorActionPreference = "Stop"
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$OriginalPreviewFlag = $env:CRYPTO_HUD_UPDATE_MARKET_COMPASS_PREVIEWS

Push-Location $RepoRoot
try {
    $env:CRYPTO_HUD_UPDATE_MARKET_COMPASS_PREVIEWS = "1"
    cargo +1.96.0 test --locked -p crypto-hud market_compass_preserves_price_layout_rotation_and_controls -- --nocapture
    if ($LASTEXITCODE -ne 0) {
        throw "Market Compass preview rendering test failed with code $LASTEXITCODE"
    }
    Add-Type -AssemblyName System.Drawing
    foreach ($theme in @("light", "dark")) {
        $SnapshotPath = Join-Path $RepoRoot "target\tmp\market-compass-preview-$theme.rgba"
        $PreviewPath = Join-Path $RepoRoot "crates\crypto-hud\plugins\com.cryptohud.market-compass\ui\preview-$theme.png"
        $bytes = [System.IO.File]::ReadAllBytes($SnapshotPath)
        if ($bytes.Length -lt 8) { throw "Missing Market Compass $theme snapshot header" }
        $width = [BitConverter]::ToUInt32($bytes, 0)
        $height = [BitConverter]::ToUInt32($bytes, 4)
        if ($width -ne 480 -or $height -ne 480 -or $bytes.Length -ne 8 + $width * $height * 4) {
            throw "Unexpected Market Compass $theme preview dimensions or pixel buffer length"
        }
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
    }
    Write-Host "Updated Market Compass previews from alpha-preserving software-renderer snapshots"
} finally {
    $env:CRYPTO_HUD_UPDATE_MARKET_COMPASS_PREVIEWS = $OriginalPreviewFlag
    Pop-Location
}
