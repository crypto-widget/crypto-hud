param([switch]$SkipRender)

$ErrorActionPreference = "Stop"
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$CaptureDir = Join-Path $RepoRoot "target\tmp\trust-card-previews"
$OriginalCaptureDir = $env:CRYPTO_HUD_TRUST_CARD_CAPTURE_DIR

Push-Location $RepoRoot
try {
    if (-not $SkipRender) {
        $env:CRYPTO_HUD_TRUST_CARD_CAPTURE_DIR = $CaptureDir
        cargo +1.96.0 test --locked -p crypto-hud trust_card_fits_eight_slot_prices_and_preserves_localized_data_states -- --nocapture
        if ($LASTEXITCODE -ne 0) { throw "Trust Card preview rendering test failed with code $LASTEXITCODE" }
    }
    Add-Type -AssemblyName System.Drawing
    # Match the existing previews: one BTC pair, 100% scale and DPI, 260 x 180,
    # both themes, on the focused test's opaque paper-colored background.
    $header = [Text.Encoding]::ASCII.GetBytes("P6`n260 180`n255`n")
    foreach ($theme in @("light", "dark")) {
        $source = Join-Path $CaptureDir "trust-card-BTC-$theme-100.ppm"
        $destination = Join-Path $RepoRoot "crates\crypto-hud\plugins\com.cryptohud.trust-card\ui\preview-$theme.png"
        $bytes = [IO.File]::ReadAllBytes($source)
        if ($bytes.Length -ne $header.Length + 260 * 180 * 3) { throw "Unexpected Trust Card $theme capture size" }
        for ($index = 0; $index -lt $header.Length; $index++) {
            if ($bytes[$index] -ne $header[$index]) { throw "Unexpected Trust Card $theme capture header" }
        }
        $bgra = [byte[]]::new(260 * 180 * 4)
        for ($pixel = 0; $pixel -lt 260 * 180; $pixel++) {
            $sourceOffset = $header.Length + $pixel * 3
            $targetOffset = $pixel * 4
            $bgra[$targetOffset] = $bytes[$sourceOffset + 2]
            $bgra[$targetOffset + 1] = $bytes[$sourceOffset + 1]
            $bgra[$targetOffset + 2] = $bytes[$sourceOffset]
            $bgra[$targetOffset + 3] = 255
        }
        $bitmap = [Drawing.Bitmap]::new(260, 180, [Drawing.Imaging.PixelFormat]::Format32bppArgb)
        try {
            $data = $bitmap.LockBits([Drawing.Rectangle]::new(0, 0, 260, 180), [Drawing.Imaging.ImageLockMode]::WriteOnly, [Drawing.Imaging.PixelFormat]::Format32bppArgb)
            try {
                if ($data.Stride -ne 260 * 4) { throw "Unexpected bitmap stride" }
                [Runtime.InteropServices.Marshal]::Copy($bgra, 0, $data.Scan0, $bgra.Length)
            } finally { $bitmap.UnlockBits($data) }
            $bitmap.Save($destination, [Drawing.Imaging.ImageFormat]::Png)
        } finally { $bitmap.Dispose() }
    }
    Write-Host "Updated Trust Card previews from deterministic software-renderer captures"
} finally {
    $env:CRYPTO_HUD_TRUST_CARD_CAPTURE_DIR = $OriginalCaptureDir
    Pop-Location
}
