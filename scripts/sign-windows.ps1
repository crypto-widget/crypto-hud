param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [Parameter(Mandatory = $true)]
    [string]$CertificatePath,
    [string]$CertificatePassword = "",
    [string]$TimestampUrl = "http://timestamp.digicert.com",
    [string]$SignToolPath = ""
)

$ErrorActionPreference = "Stop"

function Find-SignTool {
    param([string]$ExplicitPath)

    if (-not [string]::IsNullOrWhiteSpace($ExplicitPath)) {
        if (-not (Test-Path $ExplicitPath)) {
            throw "signtool.exe not found: $ExplicitPath"
        }
        return (Resolve-Path $ExplicitPath).Path
    }

    $command = Get-Command signtool.exe -ErrorAction SilentlyContinue
    if ($command) {
        return $command.Source
    }

    $kitsRoot = Join-Path ${env:ProgramFiles(x86)} "Windows Kits\10\bin"
    if (Test-Path $kitsRoot) {
        $candidate = Get-ChildItem -LiteralPath $kitsRoot -Recurse -Filter signtool.exe -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -match "\\x64\\signtool\.exe$" } |
            Sort-Object FullName -Descending |
            Select-Object -First 1
        if ($candidate) {
            return $candidate.FullName
        }
    }

    throw "signtool.exe was not found. Install the Windows SDK or pass -SignToolPath."
}

if (-not (Test-Path $Path)) {
    throw "File to sign not found: $Path"
}
if (-not (Test-Path $CertificatePath)) {
    throw "Signing certificate not found: $CertificatePath"
}

$resolvedPath = (Resolve-Path $Path).Path
$resolvedCertificatePath = (Resolve-Path $CertificatePath).Path
$signTool = Find-SignTool -ExplicitPath $SignToolPath

$args = @("sign", "/fd", "SHA256", "/td", "SHA256", "/tr", $TimestampUrl, "/f", $resolvedCertificatePath)
if (-not [string]::IsNullOrEmpty($CertificatePassword)) {
    $args += @("/p", $CertificatePassword)
}
$args += $resolvedPath

& $signTool @args
if ($LASTEXITCODE -ne 0) {
    throw "signtool failed with code $LASTEXITCODE"
}

$signature = Get-AuthenticodeSignature -LiteralPath $resolvedPath
if ($signature.Status -ne "Valid") {
    throw "Signed file did not validate: $($signature.Status) $($signature.StatusMessage)"
}

$subject = if ($signature.SignerCertificate) {
    $signature.SignerCertificate.Subject
} else {
    ""
}

[ordered]@{
    path = $resolvedPath
    status = [string]$signature.Status
    subject = $subject
    timestampUrl = $TimestampUrl
} | ConvertTo-Json
