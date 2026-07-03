param(
    [string]$Version = "dev",
    [switch]$SkipBuild,
    [switch]$Sign,
    [string]$CertificatePath = "",
    [string]$CertificatePassword = "",
    [string]$TimestampUrl = "",
    [string]$SignToolPath = ""
)

$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$DistDir = Join-Path $RepoRoot "dist"
$PackageRoot = Join-Path $DistDir "crypto-hud-$Version-windows-x64"
$Exe = Join-Path $RepoRoot "target\release\crypto-hud.exe"
$ZipPath = "$PackageRoot.zip"
$ChecksumPath = "$ZipPath.sha256"
$PackageManifestPath = Join-Path $PackageRoot "release-manifest.json"
$TempCertificatePath = $null

function New-PackageFileEntry {
    param(
        [string]$PackageRoot,
        [string]$RelativePath
    )

    $path = Join-Path $PackageRoot $RelativePath
    $item = Get-Item -LiteralPath $path
    $hash = Get-FileHash -Algorithm SHA256 -LiteralPath $path
    [ordered]@{
        path = $RelativePath
        sha256 = $hash.Hash.ToLowerInvariant()
        bytes = $item.Length
    }
}

function Test-TruthyEnv {
    param([string]$Name)

    $value = Get-EnvValue -Name $Name
    $value -match "^(1|true|yes)$"
}

function Get-EnvValue {
    param([string]$Name)

    if (-not (Test-Path "Env:\$Name")) {
        return ""
    }
    (Get-Item "Env:\$Name").Value
}

function Initialize-SigningConfig {
    param([string]$DistDir)

    $envCertPath = Get-EnvValue -Name "CRYPTO_HUD_SIGN_CERT_PATH"
    $envCertBase64 = Get-EnvValue -Name "CRYPTO_HUD_SIGN_CERT_BASE64"
    $signRequested = [bool]$Sign -or
        (Test-TruthyEnv -Name "CRYPTO_HUD_SIGN") -or
        (-not [string]::IsNullOrWhiteSpace($envCertPath)) -or
        (-not [string]::IsNullOrWhiteSpace($envCertBase64))

    if (-not $signRequested) {
        return [ordered]@{
            requested = $false
            certificatePath = ""
            certificatePassword = ""
            timestampUrl = ""
            signToolPath = ""
        }
    }

    $certPath = $CertificatePath
    if ([string]::IsNullOrWhiteSpace($certPath) -and -not [string]::IsNullOrWhiteSpace($envCertPath)) {
        $certPath = $envCertPath
    }

    if ([string]::IsNullOrWhiteSpace($certPath) -and -not [string]::IsNullOrWhiteSpace($envCertBase64)) {
        $script:TempCertificatePath = Join-Path $DistDir "crypto-hud-signing-cert.pfx"
        [System.IO.File]::WriteAllBytes(
            $script:TempCertificatePath,
            [Convert]::FromBase64String($envCertBase64)
        )
        $certPath = $script:TempCertificatePath
    }

    if ([string]::IsNullOrWhiteSpace($certPath)) {
        throw "Signing was requested but no certificate path/base64 was provided"
    }

    $password = $CertificatePassword
    $envPassword = Get-EnvValue -Name "CRYPTO_HUD_SIGN_CERT_PASSWORD"
    if ([string]::IsNullOrEmpty($password) -and -not [string]::IsNullOrEmpty($envPassword)) {
        $password = $envPassword
    }

    $timestamp = $TimestampUrl
    $envTimestamp = Get-EnvValue -Name "CRYPTO_HUD_SIGN_TIMESTAMP_URL"
    if ([string]::IsNullOrWhiteSpace($timestamp) -and -not [string]::IsNullOrWhiteSpace($envTimestamp)) {
        $timestamp = $envTimestamp
    }
    if ([string]::IsNullOrWhiteSpace($timestamp)) {
        $timestamp = "http://timestamp.digicert.com"
    }

    $toolPath = $SignToolPath
    $envToolPath = Get-EnvValue -Name "CRYPTO_HUD_SIGNTOOL_PATH"
    if ([string]::IsNullOrWhiteSpace($toolPath) -and -not [string]::IsNullOrWhiteSpace($envToolPath)) {
        $toolPath = $envToolPath
    }

    [ordered]@{
        requested = $true
        certificatePath = $certPath
        certificatePassword = $password
        timestampUrl = $timestamp
        signToolPath = $toolPath
    }
}

function Get-SignatureInfo {
    param(
        [string]$Path,
        [object]$SigningConfig
    )

    $signature = Get-AuthenticodeSignature -LiteralPath $Path
    $subject = if ($signature.SignerCertificate) {
        $signature.SignerCertificate.Subject
    } else {
        ""
    }

    [ordered]@{
        requested = [bool]$SigningConfig.requested
        signed = ($signature.Status -eq "Valid")
        status = [string]$signature.Status
        subject = $subject
        timestampUrl = $SigningConfig.timestampUrl
    }
}

Push-Location $RepoRoot
try {
    if (-not $SkipBuild) {
        cargo build --release -p crypto-hud
    }

    if (-not (Test-Path $Exe)) {
        throw "Release executable not found: $Exe"
    }

    if (Test-Path $PackageRoot) {
        Remove-Item -LiteralPath $PackageRoot -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $PackageRoot | Out-Null
    $signingConfig = Initialize-SigningConfig -DistDir $DistDir

    $packageFiles = @(
        @{ Source = $Exe; Target = "crypto-hud.exe" },
        @{ Source = (Join-Path $RepoRoot "README.md"); Target = "README.md" },
        @{ Source = (Join-Path $RepoRoot "packaging\windows\install.ps1"); Target = "install.ps1" },
        @{ Source = (Join-Path $RepoRoot "packaging\windows\uninstall.ps1"); Target = "uninstall.ps1" },
        @{ Source = (Join-Path $RepoRoot "scripts\install-update-package.ps1"); Target = "install-update-package.ps1" }
    )
    foreach ($file in $packageFiles) {
        Copy-Item -LiteralPath $file.Source -Destination (Join-Path $PackageRoot $file.Target)
    }

    $packageExe = Join-Path $PackageRoot "crypto-hud.exe"
    if ($signingConfig.requested) {
        $signArgs = @(
            "-ExecutionPolicy", "Bypass",
            "-File", ".\scripts\sign-windows.ps1",
            "-Path", $packageExe,
            "-CertificatePath", $signingConfig.certificatePath,
            "-TimestampUrl", $signingConfig.timestampUrl
        )
        if (-not [string]::IsNullOrEmpty($signingConfig.certificatePassword)) {
            $signArgs += @("-CertificatePassword", $signingConfig.certificatePassword)
        }
        if (-not [string]::IsNullOrWhiteSpace($signingConfig.signToolPath)) {
            $signArgs += @("-SignToolPath", $signingConfig.signToolPath)
        }
        powershell @signArgs | Out-Host
        if ($LASTEXITCODE -ne 0) {
            throw "Signing failed with code $LASTEXITCODE"
        }
    }
    $signatureInfo = Get-SignatureInfo -Path $packageExe -SigningConfig $signingConfig

    $fileEntries = @($packageFiles | ForEach-Object {
        New-PackageFileEntry -PackageRoot $PackageRoot -RelativePath $_.Target
    })
    $executableEntry = $fileEntries | Where-Object { $_.path -eq "crypto-hud.exe" } | Select-Object -First 1
    $commit = (git rev-parse HEAD).Trim()
    $packageManifest = [ordered]@{
        manifestVersion = 1
        name = "crypto-hud"
        version = $Version
        target = "windows-x64"
        commit = $commit
        builtAt = (Get-Date).ToUniversalTime().ToString("o")
        executable = "crypto-hud.exe"
        executableSha256 = $executableEntry.sha256
        appUserModelId = "CryptoHud"
        updateChannel = "manual-github-release"
        updateRepository = "crypto-widget/crypto-hud"
        updateApiUrl = "https://api.github.com/repos/crypto-widget/crypto-hud/releases/latest"
        codeSigning = $signatureInfo
        files = $fileEntries
        installer = [ordered]@{
            script = "install.ps1"
            uninstallScript = "uninstall.ps1"
            supportsSkipShellIntegration = $true
        }
    }
    $packageManifest | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $PackageManifestPath

    if (Test-Path $ZipPath) {
        Remove-Item -LiteralPath $ZipPath -Force
    }
    Compress-Archive -Path (Join-Path $PackageRoot "*") -DestinationPath $ZipPath

    $hash = Get-FileHash -Algorithm SHA256 -LiteralPath $ZipPath
    "$($hash.Hash.ToLowerInvariant())  $(Split-Path -Leaf $ZipPath)" | Set-Content -LiteralPath $ChecksumPath -NoNewline

    Write-Host "Created $ZipPath"
    Write-Host "Created $ChecksumPath"
} finally {
    if ($TempCertificatePath -and (Test-Path $TempCertificatePath)) {
        Remove-Item -LiteralPath $TempCertificatePath -Force
    }
    Pop-Location
}
