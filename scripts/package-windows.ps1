param(
    [string]$Version = "",
    [switch]$SkipBuild,
    [switch]$AllowDirty,
    [switch]$AllowDevelopmentVersion,
    [switch]$AllowUnsignedPackage,
    [switch]$Sign,
    [string]$CertificatePath = "",
    [string]$CertificatePassword = "",
    [string]$TimestampUrl = "",
    [string]$SignToolPath = ""
)

$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$WorkspaceManifest = Join-Path $RepoRoot "Cargo.toml"
$workspaceCargo = Get-Content -LiteralPath $WorkspaceManifest -Raw
$versionMatch = [regex]::Match(
    $workspaceCargo,
    '(?ms)^\[workspace\.package\].*?^version\s*=\s*"([^"]+)"'
)
if (-not $versionMatch.Success) {
    throw "Could not read workspace package version from $WorkspaceManifest"
}
$workspaceVersion = $versionMatch.Groups[1].Value
if ([string]::IsNullOrWhiteSpace($Version)) {
    $Version = "v$workspaceVersion"
}
if (-not $AllowDevelopmentVersion) {
    if ($Version -notmatch '^v([0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?)$') {
        throw "Release version must use v-prefixed SemVer: $Version"
    }
    if ($Matches[1] -ne $workspaceVersion) {
        throw "Release version $Version does not match workspace version $workspaceVersion"
    }
}
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
    param([string]$Path)

    $signature = Get-AuthenticodeSignature -LiteralPath $Path
    $subject = if ($signature.SignerCertificate) {
        $signature.SignerCertificate.Subject
    } else {
        ""
    }
    $thumbprint = if ($signature.SignerCertificate) {
        $signature.SignerCertificate.Thumbprint
    } else {
        ""
    }

    [ordered]@{
        path = Split-Path -Leaf $Path
        signed = ($signature.Status -eq "Valid")
        status = [string]$signature.Status
        subject = $subject
        thumbprint = $thumbprint
    }
}

Push-Location $RepoRoot
try {
    $sourceStatus = @(git status --porcelain --untracked-files=normal)
    if ($LASTEXITCODE -ne 0) {
        throw "Could not inspect Git worktree status"
    }
    $sourceDirty = $sourceStatus.Count -gt 0
    if ($sourceDirty -and -not $AllowDirty) {
        throw "Refusing to create a release package from a dirty worktree. Commit or stash changes first."
    }

    if (-not $SkipBuild) {
        cargo build --locked --release -p crypto-hud
        if ($LASTEXITCODE -ne 0) {
            throw "Release build failed with code $LASTEXITCODE"
        }
    }

    if (-not (Test-Path $Exe)) {
        throw "Release executable not found: $Exe"
    }

    if (Test-Path $PackageRoot) {
        Remove-Item -LiteralPath $PackageRoot -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $PackageRoot | Out-Null
    $signingConfig = Initialize-SigningConfig -DistDir $DistDir
    if (-not [bool]$signingConfig.requested -and -not $AllowUnsignedPackage) {
        throw "Production packages must be Authenticode signed. Configure signing or pass -AllowUnsignedPackage for local development only."
    }

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

    $signedTargets = @(
        "crypto-hud.exe",
        "install.ps1",
        "uninstall.ps1",
        "install-update-package.ps1"
    )
    if ($signingConfig.requested) {
        foreach ($relativePath in $signedTargets) {
            $signArgs = @(
                "-ExecutionPolicy", "Bypass",
                "-File", ".\scripts\sign-windows.ps1",
                "-Path", (Join-Path $PackageRoot $relativePath),
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
                throw "Signing $relativePath failed with code $LASTEXITCODE"
            }
        }
    }
    $signatureFiles = @($signedTargets | ForEach-Object {
        Get-SignatureInfo -Path (Join-Path $PackageRoot $_)
    })
    $validSignatureFiles = @($signatureFiles | Where-Object { [bool]$_.signed })
    if ([bool]$signingConfig.requested -and $validSignatureFiles.Count -ne $signedTargets.Count) {
        throw "One or more package executables or scripts do not have a valid Authenticode signature"
    }
    $signerSubjects = @($validSignatureFiles | ForEach-Object { $_.subject } | Sort-Object -Unique)
    if ($signerSubjects.Count -gt 1) {
        throw "Package executables and scripts were signed by different publishers"
    }
    $signatureInfo = [ordered]@{
        required = -not [bool]$AllowUnsignedPackage
        requested = [bool]$signingConfig.requested
        signed = ($validSignatureFiles.Count -eq $signedTargets.Count)
        status = if ($validSignatureFiles.Count -eq $signedTargets.Count) { "Valid" } else { "NotSigned" }
        subject = if ($signerSubjects.Count -eq 1) { $signerSubjects[0] } else { "" }
        thumbprint = if ($validSignatureFiles.Count -gt 0) { $validSignatureFiles[0].thumbprint } else { "" }
        timestampUrl = $signingConfig.timestampUrl
        files = $signatureFiles
    }

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
        sourceDirty = $sourceDirty
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
