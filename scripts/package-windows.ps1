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
$PowerShellExe = (Get-Process -Id $PID).Path
if (-not (Test-Path -LiteralPath $PowerShellExe -PathType Leaf) -or
    (Split-Path -Leaf $PowerShellExe) -notmatch '^(?i:powershell|pwsh)(?:\.exe)?$') {
    throw "Current PowerShell host path is not trusted: $PowerShellExe"
}

if (-not [string]::IsNullOrEmpty($CertificatePassword)) {
    throw "-CertificatePassword is not accepted because process arguments are observable; use CRYPTO_HUD_SIGN_CERT_PASSWORD"
}
if (-not [string]::IsNullOrWhiteSpace($SignToolPath) -or
    (Test-Path "Env:\CRYPTO_HUD_SIGNTOOL_PATH")) {
    throw "signtool.exe configuration is no longer used; signing runs in-process"
}

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
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
if ([System.IO.Path]::IsPathRooted($Version) -or
    $Version.IndexOfAny([System.IO.Path]::GetInvalidFileNameChars()) -ge 0 -or
    $Version.Contains("\") -or
    $Version.Contains("/") -or
    $Version.Contains("..") -or
    $Version -notmatch '^[0-9A-Za-z][0-9A-Za-z.-]{0,63}$') {
    throw "Version must be a safe filename component: $Version"
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
$TempCertificateDirectory = $null
$OriginalProcessPasswordPresent = Test-Path "Env:\CRYPTO_HUD_SIGN_CERT_PASSWORD_PROCESS"
$OriginalProcessPassword = if ($OriginalProcessPasswordPresent) {
    (Get-Item "Env:\CRYPTO_HUD_SIGN_CERT_PASSWORD_PROCESS").Value
} else {
    ""
}
Remove-Item "Env:\CRYPTO_HUD_SIGN_CERT_PASSWORD_PROCESS" -ErrorAction SilentlyContinue

function Assert-UnderDirectory {
    param(
        [string]$Path,
        [string]$Directory,
        [string]$Description,
        [switch]$AllowDirectoryItself
    )

    $fullPath = [System.IO.Path]::GetFullPath($Path).TrimEnd('\', '/')
    $fullDirectory = [System.IO.Path]::GetFullPath($Directory).TrimEnd('\', '/')
    if ($AllowDirectoryItself -and
        $fullPath.Equals($fullDirectory, [System.StringComparison]::OrdinalIgnoreCase)) {
        return $fullPath
    }
    $directoryPrefix = "$fullDirectory$([System.IO.Path]::DirectorySeparatorChar)"
    if (-not $fullPath.StartsWith($directoryPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to operate outside ${Description}: $fullPath"
    }
    $fullPath
}

function Assert-NoReparsePoint {
    param(
        [string]$Path,
        [string]$StopDirectory
    )

    $fullPath = [System.IO.Path]::GetFullPath($Path)
    $stopPath = [System.IO.Path]::GetFullPath($StopDirectory).TrimEnd('\', '/')
    $current = $fullPath
    while ($current -and
        $current.Length -ge $stopPath.Length -and
        $current.StartsWith($stopPath, [System.StringComparison]::OrdinalIgnoreCase)) {
        if (Test-Path -LiteralPath $current) {
            $item = Get-Item -LiteralPath $current -Force
            if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw "Refusing to use a reparse point in a release path: $current"
            }
        }
        if ($current.TrimEnd('\', '/').Equals($stopPath, [System.StringComparison]::OrdinalIgnoreCase)) {
            break
        }
        $current = Split-Path -Parent $current
    }
}

function Resolve-PackageTargetPath {
    param([string]$RelativePath)

    if ([string]::IsNullOrWhiteSpace($RelativePath) -or
        [System.IO.Path]::IsPathRooted($RelativePath)) {
        throw "Package target path must be relative: $RelativePath"
    }
    $segments = $RelativePath -split '[\\/]'
    if ($segments -contains "" -or $segments -contains "." -or $segments -contains "..") {
        throw "Package target path is unsafe: $RelativePath"
    }
    foreach ($segment in $segments) {
        if ($segment.IndexOfAny([System.IO.Path]::GetInvalidFileNameChars()) -ge 0 -or
            $segment.EndsWith('.') -or $segment.EndsWith(' ') -or
            $segment.Split('.')[0] -match '^(?i:CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])$') {
            throw "Package target path contains an invalid filename: $RelativePath"
        }
    }
    $target = [System.IO.Path]::GetFullPath((Join-Path $PackageRoot $RelativePath))
    Assert-UnderDirectory -Path $target -Directory $PackageRoot -Description "package directory" | Out-Null
    $target
}

function Get-SafeTreeFiles {
    param(
        [string]$SourceRoot,
        [string]$TargetRoot
    )

    $fullSourceRoot = [System.IO.Path]::GetFullPath($SourceRoot)
    Assert-UnderDirectory -Path $fullSourceRoot -Directory $RepoRoot -Description "repository" | Out-Null
    Assert-NoReparsePoint -Path $fullSourceRoot -StopDirectory $RepoRoot
    $sourceRootItem = Get-Item -LiteralPath $fullSourceRoot -Force
    if (-not $sourceRootItem.PSIsContainer) {
        throw "Release resource root is not a directory: $fullSourceRoot"
    }
    if (($sourceRootItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "Release resource root must not be a reparse point: $fullSourceRoot"
    }

    $resolvedSourceRoot = (Resolve-Path -LiteralPath $fullSourceRoot).Path.TrimEnd('\', '/')
    Assert-UnderDirectory -Path $resolvedSourceRoot -Directory $RepoRoot -Description "repository" | Out-Null
    $sourcePrefix = "$resolvedSourceRoot$([System.IO.Path]::DirectorySeparatorChar)"
    foreach ($item in Get-ChildItem -LiteralPath $resolvedSourceRoot -Recurse -Force) {
        if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "Release resources must not contain reparse points: $($item.FullName)"
        }
        if (-not $item.PSIsContainer) {
            $relativePath = $item.FullName.Substring($sourcePrefix.Length).Replace('\', '/')
            [ordered]@{
                Source = $item.FullName
                Target = "$TargetRoot/$relativePath"
            }
        }
    }
}

function Protect-SigningDirectory {
    param([string]$Path)

    $security = [System.Security.AccessControl.DirectorySecurity]::new()
    $security.SetAccessRuleProtection($true, $false)
    $inheritance = [System.Security.AccessControl.InheritanceFlags]::ContainerInherit -bor
        [System.Security.AccessControl.InheritanceFlags]::ObjectInherit
    foreach ($sidValue in @("S-1-5-18", "S-1-5-32-544", ([System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value))) {
        $sid = [System.Security.Principal.SecurityIdentifier]::new($sidValue)
        $rule = [System.Security.AccessControl.FileSystemAccessRule]::new(
            $sid,
            [System.Security.AccessControl.FileSystemRights]::FullControl,
            $inheritance,
            [System.Security.AccessControl.PropagationFlags]::None,
            [System.Security.AccessControl.AccessControlType]::Allow
        )
        $security.AddAccessRule($rule)
    }
    Set-Acl -LiteralPath $Path -AclObject $security
}

function New-PackageFileEntry {
    param(
        [string]$PackageRoot,
        [string]$RelativePath
    )

    $path = Resolve-PackageTargetPath -RelativePath $RelativePath
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

function Test-SigningRequested {
    $envCertPath = Get-EnvValue -Name "CRYPTO_HUD_SIGN_CERT_PATH"
    $envCertBase64 = Get-EnvValue -Name "CRYPTO_HUD_SIGN_CERT_BASE64"
    [bool]$Sign -or
        (Test-TruthyEnv -Name "CRYPTO_HUD_SIGN") -or
        (-not [string]::IsNullOrWhiteSpace($CertificatePath)) -or
        (-not [string]::IsNullOrWhiteSpace($envCertPath)) -or
        (-not [string]::IsNullOrWhiteSpace($envCertBase64))
}

function Initialize-SigningConfig {
    param([string]$DistDir)

    $envCertPath = Get-EnvValue -Name "CRYPTO_HUD_SIGN_CERT_PATH"
    $envCertBase64 = Get-EnvValue -Name "CRYPTO_HUD_SIGN_CERT_BASE64"
    $signRequested = [bool]$Sign -or
        (Test-TruthyEnv -Name "CRYPTO_HUD_SIGN") -or
        (-not [string]::IsNullOrWhiteSpace($CertificatePath)) -or
        (-not [string]::IsNullOrWhiteSpace($envCertPath)) -or
        (-not [string]::IsNullOrWhiteSpace($envCertBase64))

    if (-not $signRequested) {
        return [ordered]@{
            requested = $false
            certificatePath = ""
            certificateBase64 = ""
            certificatePassword = ""
            timestampUrl = ""
            signToolPath = ""
        }
    }

    $certPath = $CertificatePath
    $pendingCertificateBase64 = ""
    if ([string]::IsNullOrWhiteSpace($certPath) -and -not [string]::IsNullOrWhiteSpace($envCertPath)) {
        $certPath = $envCertPath
    }

    if ([string]::IsNullOrWhiteSpace($certPath) -and -not [string]::IsNullOrWhiteSpace($envCertBase64)) {
        $certPath = "base64-pending"
        $pendingCertificateBase64 = $envCertBase64
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
        $timestamp = "https://timestamp.digicert.com"
    }

    [ordered]@{
        requested = $true
        certificatePath = $certPath
        certificateBase64 = $pendingCertificateBase64
        certificatePassword = $password
        timestampUrl = $timestamp
        signToolPath = ""
    }
}

function Materialize-SigningCertificate {
    param([System.Collections.IDictionary]$SigningConfig)

    if (-not [bool]$SigningConfig.requested -or
        [string]::IsNullOrWhiteSpace([string]$SigningConfig.certificateBase64)) {
        return
    }
    $script:TempCertificateDirectory = Join-Path `
        ([System.IO.Path]::GetTempPath()) `
        ([Guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $script:TempCertificateDirectory | Out-Null
    Protect-SigningDirectory -Path $script:TempCertificateDirectory
    $script:TempCertificatePath = Join-Path $script:TempCertificateDirectory "certificate.pfx"
    [System.IO.File]::WriteAllBytes(
        $script:TempCertificatePath,
        [Convert]::FromBase64String([string]$SigningConfig.certificateBase64)
    )
    $SigningConfig.certificatePath = $script:TempCertificatePath
    $SigningConfig.certificateBase64 = ""
}

function Invoke-ReleaseBuildWithoutSigningSecrets {
    $secretNames = @(
        "CRYPTO_HUD_SIGN_CERT_PATH",
        "CRYPTO_HUD_SIGN_CERT_BASE64",
        "CRYPTO_HUD_SIGN_CERT_PASSWORD",
        "CRYPTO_HUD_SIGN_CERT_PASSWORD_PROCESS"
    )
    $savedValues = @{}
    try {
        foreach ($name in $secretNames) {
            if (Test-Path "Env:\$name") {
                $savedValues[$name] = (Get-Item "Env:\$name").Value
                Remove-Item "Env:\$name"
            }
        }
        cargo build --locked --release -p crypto-hud
        if ($LASTEXITCODE -ne 0) {
            throw "Release build failed with code $LASTEXITCODE"
        }
    } finally {
        foreach ($name in $secretNames) {
            Remove-Item "Env:\$name" -ErrorAction SilentlyContinue
            if ($savedValues.ContainsKey($name)) {
                Set-Item "Env:\$name" $savedValues[$name]
            }
        }
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
    Assert-UnderDirectory -Path $DistDir -Directory $RepoRoot -Description "repository" | Out-Null
    Assert-UnderDirectory -Path $PackageRoot -Directory $DistDir -Description "distribution directory" | Out-Null
    Assert-UnderDirectory -Path $ZipPath -Directory $DistDir -Description "distribution directory" | Out-Null
    Assert-UnderDirectory -Path $ChecksumPath -Directory $DistDir -Description "distribution directory" | Out-Null
    Assert-NoReparsePoint -Path $DistDir -StopDirectory $RepoRoot
    Assert-NoReparsePoint -Path $PackageRoot -StopDirectory $RepoRoot

    $signRequested = Test-SigningRequested
    if ($signRequested -and $SkipBuild) {
        throw "Production signing requires a fresh release build; -SkipBuild is not allowed when signing"
    }
    if ($signRequested -and
        [string]::IsNullOrEmpty((Get-EnvValue -Name "CRYPTO_HUD_SIGN_CERT_PASSWORD"))) {
        throw "Production signing requires CRYPTO_HUD_SIGN_CERT_PASSWORD; plaintext password parameters are not accepted"
    }
    if ($AllowUnsignedPackage -and
        (-not $AllowDevelopmentVersion -or
            -not (Test-TruthyEnv -Name "CRYPTO_HUD_ALLOW_UNSIGNED_SMOKE") -or
            ($Version -match '^v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$' -and
                $Version -notmatch '-smoke$'))) {
        throw "-AllowUnsignedPackage is restricted to local smoke tests; set CRYPTO_HUD_ALLOW_UNSIGNED_SMOKE=1 and use -AllowDevelopmentVersion"
    }
    $signingConfig = Initialize-SigningConfig -DistDir $DistDir
    if (-not [bool]$signingConfig.requested -and -not $AllowUnsignedPackage) {
        throw "Production packages must be Authenticode signed. Configure signing or pass -AllowUnsignedPackage for local smoke tests only."
    }
    foreach ($signingSecretName in @(
        "CRYPTO_HUD_SIGN_CERT_PATH",
        "CRYPTO_HUD_SIGN_CERT_BASE64",
        "CRYPTO_HUD_SIGN_CERT_PASSWORD",
        "CRYPTO_HUD_SIGN_CERT_PASSWORD_PROCESS"
    )) {
        Remove-Item "Env:\$signingSecretName" -ErrorAction SilentlyContinue
    }

    $sourceStatus = @(git status --porcelain --untracked-files=normal)
    if ($LASTEXITCODE -ne 0) {
        throw "Could not inspect Git worktree status"
    }
    $sourceDirty = $sourceStatus.Count -gt 0
    if ($sourceDirty -and -not $AllowDirty) {
        throw "Refusing to create a release package from a dirty worktree. Commit or stash changes first."
    }
    if ($sourceDirty -and $signRequested) {
        throw "Refusing to sign a production package from a dirty worktree"
    }

    if (-not $SkipBuild) {
        Invoke-ReleaseBuildWithoutSigningSecrets
    }

    Materialize-SigningCertificate -SigningConfig $signingConfig

    if (-not (Test-Path $Exe)) {
        throw "Release executable not found: $Exe"
    }

    New-Item -ItemType Directory -Force -Path $DistDir | Out-Null
    Assert-NoReparsePoint -Path $DistDir -StopDirectory $RepoRoot
    if (Test-Path -LiteralPath $PackageRoot) {
        Assert-NoReparsePoint -Path $PackageRoot -StopDirectory $RepoRoot
        Remove-Item -LiteralPath $PackageRoot -Recurse -Force
    }
    New-Item -ItemType Directory -Path $PackageRoot | Out-Null
    $packageFiles = @(
        @{ Source = $Exe; Target = "crypto-hud.exe" },
        @{ Source = (Join-Path $RepoRoot "README.md"); Target = "README.md" },
        @{ Source = (Join-Path $RepoRoot "LICENSE"); Target = "LICENSE" },
        @{ Source = (Join-Path $RepoRoot "packaging\windows\install.ps1"); Target = "install.ps1" },
        @{ Source = (Join-Path $RepoRoot "packaging\windows\uninstall.ps1"); Target = "uninstall.ps1" },
        @{ Source = (Join-Path $RepoRoot "scripts\install-update-package.ps1"); Target = "install-update-package.ps1" }
    )
    $packageFiles += @(Get-SafeTreeFiles `
        -SourceRoot (Join-Path $RepoRoot "crates\crypto-hud\plugins") `
        -TargetRoot "plugins")
    $packageFiles += @(Get-SafeTreeFiles `
        -SourceRoot (Join-Path $RepoRoot "crates\crypto-hud\ui\previews") `
        -TargetRoot "resources/previews")
    $packageFiles += @{
        Source = (Join-Path $RepoRoot "crates\crypto-hud\ui\icon.ico")
        Target = "resources/icon.ico"
    }
    foreach ($file in $packageFiles) {
        if (-not (Test-Path -LiteralPath $file.Source -PathType Leaf)) {
            throw "Release package source file is missing: $($file.Source)"
        }
        Assert-UnderDirectory -Path $file.Source -Directory $RepoRoot -Description "repository" | Out-Null
        Assert-NoReparsePoint -Path $file.Source -StopDirectory $RepoRoot
        $sourceItem = Get-Item -LiteralPath $file.Source -Force
        if (($sourceItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "Release package source file must not be a reparse point: $($file.Source)"
        }
        $targetPath = Resolve-PackageTargetPath -RelativePath $file.Target
        $targetDirectory = Split-Path -Parent $targetPath
        New-Item -ItemType Directory -Force -Path $targetDirectory | Out-Null
        Copy-Item -LiteralPath $file.Source -Destination $targetPath
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
                Set-Item "Env:\CRYPTO_HUD_SIGN_CERT_PASSWORD_PROCESS" $signingConfig.certificatePassword
            } else {
                Remove-Item "Env:\CRYPTO_HUD_SIGN_CERT_PASSWORD_PROCESS" -ErrorAction SilentlyContinue
            }
            & $PowerShellExe @signArgs | Out-Host
            if ($LASTEXITCODE -ne 0) {
                throw "Signing $relativePath failed with code $LASTEXITCODE"
            }
        }
        Remove-Item "Env:\CRYPTO_HUD_SIGN_CERT_PASSWORD_PROCESS" -ErrorAction SilentlyContinue
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
        detachedManifest = "release-integrity.ps1"
    }

    $fileEntries = @($packageFiles | ForEach-Object {
        New-PackageFileEntry -PackageRoot $PackageRoot -RelativePath $_.Target
    })
    $executableEntry = $fileEntries | Where-Object { $_.path -eq "crypto-hud.exe" } | Select-Object -First 1
    $commit = (git rev-parse HEAD).Trim()
    $packageManifest = [ordered]@{
        manifestVersion = 2
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
    $packageManifest | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $PackageManifestPath -Encoding UTF8

    $manifestHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $PackageManifestPath).Hash.ToLowerInvariant()
    $integrityPath = Join-Path $PackageRoot "release-integrity.ps1"
    @(
        "# Crypto HUD signed release integrity metadata. This file contains no executable code.",
        "# CryptoHud-Manifest-SHA256: $manifestHash",
        "# CryptoHud-Version: $Version"
    ) | Set-Content -LiteralPath $integrityPath -Encoding UTF8

    if ($signingConfig.requested) {
        if (-not [string]::IsNullOrEmpty($signingConfig.certificatePassword)) {
            Set-Item "Env:\CRYPTO_HUD_SIGN_CERT_PASSWORD_PROCESS" $signingConfig.certificatePassword
        }
        try {
            $integritySignArgs = @(
                "-ExecutionPolicy", "Bypass",
                "-File", ".\scripts\sign-windows.ps1",
                "-Path", $integrityPath,
                "-CertificatePath", $signingConfig.certificatePath,
                "-TimestampUrl", $signingConfig.timestampUrl
            )
            & $PowerShellExe @integritySignArgs | Out-Host
            if ($LASTEXITCODE -ne 0) {
                throw "Signing release-integrity.ps1 failed with code $LASTEXITCODE"
            }
        } finally {
            Remove-Item "Env:\CRYPTO_HUD_SIGN_CERT_PASSWORD_PROCESS" -ErrorAction SilentlyContinue
        }

        $integritySignature = Get-SignatureInfo -Path $integrityPath
        if (-not [bool]$integritySignature.signed -or
            -not ([string]$integritySignature.subject).Equals(
                [string]$signatureInfo.subject,
                [System.StringComparison]::OrdinalIgnoreCase
            )) {
            throw "Signed release integrity metadata does not match the package publisher"
        }
    }

    if (Test-Path -LiteralPath $ZipPath) {
        Assert-NoReparsePoint -Path $ZipPath -StopDirectory $RepoRoot
        Remove-Item -LiteralPath $ZipPath -Force
    }
    Compress-Archive -Path (Join-Path $PackageRoot "*") -DestinationPath $ZipPath

    $hash = Get-FileHash -Algorithm SHA256 -LiteralPath $ZipPath
    "$($hash.Hash.ToLowerInvariant())  $(Split-Path -Leaf $ZipPath)" | Set-Content -LiteralPath $ChecksumPath -NoNewline

    Write-Host "Created $ZipPath"
    Write-Host "Created $ChecksumPath"
} finally {
    Remove-Item "Env:\CRYPTO_HUD_SIGN_CERT_PASSWORD_PROCESS" -ErrorAction SilentlyContinue
    if ($OriginalProcessPasswordPresent) {
        Set-Item "Env:\CRYPTO_HUD_SIGN_CERT_PASSWORD_PROCESS" $OriginalProcessPassword
    }
    if ($TempCertificatePath -and (Test-Path -LiteralPath $TempCertificatePath)) {
        Remove-Item -LiteralPath $TempCertificatePath -Force
    }
    if ($TempCertificateDirectory -and (Test-Path -LiteralPath $TempCertificateDirectory)) {
        Remove-Item -LiteralPath $TempCertificateDirectory -Force
    }
    Pop-Location
}
