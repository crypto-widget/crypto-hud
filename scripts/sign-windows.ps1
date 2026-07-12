param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [Parameter(Mandatory = $true)]
    [string]$CertificatePath,
    [string]$CertificatePassword = "",
    [string]$TimestampUrl = "https://timestamp.digicert.com",
    [string]$SignToolPath = ""
)

$ErrorActionPreference = "Stop"

function Assert-RegularFile {
    param(
        [string]$Path,
        [string]$Description
    )

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "$Description not found: $Path"
    }
    $item = Get-Item -LiteralPath $Path -Force
    if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "$Description must not be a reparse point: $Path"
    }
    $item.FullName
}

if (-not [string]::IsNullOrEmpty($CertificatePassword)) {
    throw "-CertificatePassword is not accepted because process arguments are observable; use CRYPTO_HUD_SIGN_CERT_PASSWORD"
}
if (-not [string]::IsNullOrWhiteSpace($SignToolPath)) {
    throw "-SignToolPath is no longer used; signing is performed in-process without signtool.exe"
}
if (-not (Test-Path "Env:\CRYPTO_HUD_SIGN_CERT_PASSWORD_PROCESS")) {
    throw "Signing certificate password was not provided through the isolated signing environment"
}

$timestampUri = $null
if (-not [Uri]::TryCreate($TimestampUrl, [UriKind]::Absolute, [ref]$timestampUri) -or
    $timestampUri.Scheme -ne [Uri]::UriSchemeHttps) {
    throw "Timestamp URL must be an absolute HTTPS URL: $TimestampUrl"
}

$resolvedPath = Assert-RegularFile -Path $Path -Description "File to sign"
$resolvedCertificatePath = Assert-RegularFile -Path $CertificatePath -Description "Signing certificate"
$passwordText = (Get-Item "Env:\CRYPTO_HUD_SIGN_CERT_PASSWORD_PROCESS").Value
$securePassword = ConvertTo-SecureString -String $passwordText -AsPlainText -Force
$certificate = $null
try {
    $keyFlags = [System.Security.Cryptography.X509Certificates.X509KeyStorageFlags]::EphemeralKeySet
    $certificate = [System.Security.Cryptography.X509Certificates.X509Certificate2]::new(
        $resolvedCertificatePath,
        $securePassword,
        $keyFlags
    )
    if (-not $certificate.HasPrivateKey) {
        throw "Signing certificate does not contain an accessible private key"
    }
    $now = [DateTime]::UtcNow
    if ($certificate.NotBefore.ToUniversalTime() -gt $now -or
        $certificate.NotAfter.ToUniversalTime() -lt $now) {
        throw "Signing certificate is not currently valid"
    }
    $enhancedKeyUsage = @($certificate.Extensions | Where-Object {
        $_ -is [System.Security.Cryptography.X509Certificates.X509EnhancedKeyUsageExtension]
    })
    if ($enhancedKeyUsage.Count -gt 0) {
        $codeSigningAllowed = @($enhancedKeyUsage[0].EnhancedKeyUsages | Where-Object {
            $_.Value -eq "1.3.6.1.5.5.7.3.3"
        }).Count -gt 0
        if (-not $codeSigningAllowed) {
            throw "Signing certificate is not valid for code signing"
        }
    }

    $signature = Set-AuthenticodeSignature `
        -FilePath $resolvedPath `
        -Certificate $certificate `
        -HashAlgorithm SHA256 `
        -TimestampServer $TimestampUrl
    if ($signature.Status -ne "Valid") {
        throw "Signed file did not validate: $($signature.Status) $($signature.StatusMessage)"
    }
} finally {
    $passwordText = $null
    $securePassword = $null
    if ($certificate) {
        $certificate.Dispose()
    }
}

$verifiedSignature = Get-AuthenticodeSignature -LiteralPath $resolvedPath
if ($verifiedSignature.Status -ne "Valid" -or -not $verifiedSignature.SignerCertificate) {
    throw "Signed file did not pass independent Authenticode verification: $($verifiedSignature.Status)"
}

[ordered]@{
    path = $resolvedPath
    status = [string]$verifiedSignature.Status
    subject = [string]$verifiedSignature.SignerCertificate.Subject
    timestampUrl = $TimestampUrl
    signingBackend = "Set-AuthenticodeSignature"
} | ConvertTo-Json
