param(
    [string]$Version = "v9999.0.1-smoke",
    [switch]$SkipBuild,
    [switch]$KeepPackage
)

$ErrorActionPreference = "Stop"
$PowerShellExe = (Get-Process -Id $PID).Path
if (-not (Test-Path -LiteralPath $PowerShellExe -PathType Leaf) -or
    (Split-Path -Leaf $PowerShellExe) -notmatch '^(?i:powershell|pwsh)(?:\.exe)?$') {
    throw "Current PowerShell host path is not trusted: $PowerShellExe"
}

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$TaskbarDllSmokeScript = Join-Path $PSScriptRoot "taskbar-dll-smoke.ps1"
if ([System.IO.Path]::IsPathRooted($Version) -or
    $Version.IndexOfAny([System.IO.Path]::GetInvalidFileNameChars()) -ge 0 -or
    $Version.Contains("\") -or
    $Version.Contains("/") -or
    $Version.Contains("..") -or
    $Version -notmatch '^[0-9A-Za-z][0-9A-Za-z.-]{0,63}$') {
    throw "Smoke version must be a safe filename component: $Version"
}
$DistDir = Join-Path $RepoRoot "dist"
$PackageRoot = Join-Path $DistDir "crypto-hud-$Version-windows-x64"
$ZipPath = "$PackageRoot.zip"
$ChecksumPath = "$ZipPath.sha256"
$TempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
$IsolatedRoot = Join-Path $TempRoot "crypto-hud-package-smoke-$PID"
$IsolatedPackageRoot = Join-Path $IsolatedRoot "package"
$InstallDir = Join-Path $IsolatedRoot "install"
$ShellSandbox = Join-Path $IsolatedRoot "shell"
$ActivePackageRoot = $PackageRoot
$OriginalLocalAppData = $env:LOCALAPPDATA
$OriginalAppData = $env:APPDATA
$OriginalUnsignedSmoke = $env:CRYPTO_HUD_ALLOW_UNSIGNED_SMOKE

function Assert-UnderRepo {
    param([string]$Path)

    $fullPath = [System.IO.Path]::GetFullPath($Path)
    $repoPath = [System.IO.Path]::GetFullPath($RepoRoot).TrimEnd('\', '/')
    $repoPrefix = "$repoPath$([System.IO.Path]::DirectorySeparatorChar)"
    if (-not $fullPath.StartsWith($repoPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to operate outside repository: $fullPath"
    }
}

function Assert-Hash {
    param(
        [string]$Path,
        [string]$ExpectedHash
    )

    if (-not (Test-Path $Path)) {
        throw "Missing file: $Path"
    }
    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
    if ($actual -ne $ExpectedHash.ToLowerInvariant()) {
        throw "Hash mismatch for $Path"
    }
}

function Resolve-PackageFile {
    param([string]$RelativePath)

    $segments = $RelativePath -split '[\\/]'
    if ([System.IO.Path]::IsPathRooted($RelativePath) -or
        $segments -contains "" -or $segments -contains "." -or $segments -contains "..") {
        throw "Unsafe package path in manifest: $RelativePath"
    }
    $path = [System.IO.Path]::GetFullPath((Join-Path $ActivePackageRoot $RelativePath))
    $packagePrefix = "$([System.IO.Path]::GetFullPath($ActivePackageRoot).TrimEnd('\', '/'))$([System.IO.Path]::DirectorySeparatorChar)"
    if (-not $path.StartsWith($packagePrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Package path escaped the active package root: $RelativePath"
    }
    $path
}

function Assert-UnderTemp {
    param([string]$Path)

    $fullPath = [System.IO.Path]::GetFullPath($Path)
    $tempPath = $TempRoot.TrimEnd('\', '/')
    $tempPrefix = "$tempPath$([System.IO.Path]::DirectorySeparatorChar)"
    if (-not $fullPath.StartsWith($tempPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to operate outside temporary directory: $fullPath"
    }
}

function Assert-ExecutableDoesNotEmbedSourceResourcePath {
    param([string]$Path)

    $bytes = [System.IO.File]::ReadAllBytes($Path)
    $ascii = [System.Text.Encoding]::ASCII.GetString($bytes)
    $unicode = [System.Text.Encoding]::Unicode.GetString($bytes)
    foreach ($sourcePath in @(
        (Join-Path $RepoRoot "crates\crypto-hud\plugins"),
        (Join-Path $RepoRoot "crates\crypto-hud\ui\previews"),
        (Join-Path $RepoRoot "crates\crypto-hud\ui\icon.ico")
    )) {
        $forwardPath = $sourcePath.Replace('\', '/')
        if ($ascii.Contains($sourcePath) -or $ascii.Contains($forwardPath) -or
            $unicode.Contains($sourcePath) -or $unicode.Contains($forwardPath)) {
            throw "Release executable embeds a source-tree runtime resource path: $sourcePath"
        }
    }
}

function Assert-PackagedResourceHeaders {
    param([string]$Root)

    $iconBytes = [System.IO.File]::ReadAllBytes((Join-Path $Root "resources\icon.ico"))
    if ($iconBytes.Length -lt 6 -or
        $iconBytes[0] -ne 0 -or $iconBytes[1] -ne 0 -or
        $iconBytes[2] -ne 1 -or $iconBytes[3] -ne 0 -or
        $iconBytes[4] -eq 0) {
        throw "Packaged resources/icon.ico does not have a valid ICO header"
    }
    $pngSignature = @(137, 80, 78, 71, 13, 10, 26, 10)
    foreach ($relativePath in @(
        "resources\previews\mini-ticker-dark.png",
        "resources\previews\mini-ticker-light.png",
        "resources\previews\quote-board-dark.png",
        "resources\previews\quote-board-light.png"
    )) {
        $bytes = [System.IO.File]::ReadAllBytes((Join-Path $Root $relativePath))
        if ($bytes.Length -lt $pngSignature.Count) {
            throw "Packaged preview is truncated: $relativePath"
        }
        for ($index = 0; $index -lt $pngSignature.Count; $index++) {
            if ($bytes[$index] -ne $pngSignature[$index]) {
                throw "Packaged preview does not have a PNG header: $relativePath"
            }
        }
    }
}

function Invoke-IsolatedPluginRuntimeSmoke {
    param(
        [string]$Executable,
        [string]$WorkingDirectory,
        [string]$Scenario
    )

    $expectedPluginIds = @(
        "com.cryptohud.focus-ticker",
        "com.cryptohud.market-compass",
        "com.cryptohud.trust-card",
        "com.cryptohud.status-strip"
    )
    $stateDir = Join-Path $IsolatedRoot "state-$Scenario"
    $readyFile = Join-Path $stateDir "ready.json"
    New-Item -ItemType Directory -Force -Path $stateDir | Out-Null
    $sizes = @{
        "com.cryptohud.focus-ticker" = @(820, 156)
        "com.cryptohud.market-compass" = @(480, 480)
        "com.cryptohud.trust-card" = @(520, 386)
        "com.cryptohud.status-strip" = @(618, 92)
    }
    $widgets = @()
    for ($index = 0; $index -lt $expectedPluginIds.Count; $index++) {
        $pluginId = $expectedPluginIds[$index]
        $size = $sizes[$pluginId]
        $widgets += [ordered]@{
            id = "packaged-plugin-$index"
            plugin_id = $pluginId
            name = "Packaged plugin $index"
            visible = $true
            layout = [ordered]@{
                x = 80 + ($index * 30)
                y = 80 + ($index * 30)
                always_on_top = $false
                opacity_percent = 96
                locked = $false
                width = $size[0]
                height = $size[1]
            }
            symbols = @("binance:spot:BTC/USDT")
            config = [ordered]@{}
        }
    }
    $state = [ordered]@{
        settings = [ordered]@{
            widgets_always_on_top = $false
            opacity_percent = 96
            shortcut = "disabled"
            tray_icon_enabled = $false
            auto_start_enabled = $false
        }
        selected_widget_id = $widgets[0].id
        next_widget_number = 5
        widgets = $widgets
    }
    $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText(
        (Join-Path $stateDir "layouts.json"),
        ($state | ConvertTo-Json -Depth 8),
        $utf8NoBom
    )

    $process = $null
    try {
        $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
        $startInfo.FileName = $Executable
        $startInfo.Arguments = "--show-settings --gui-smoke-ms 5000"
        $startInfo.WorkingDirectory = $WorkingDirectory
        $startInfo.UseShellExecute = $false
        $startInfo.CreateNoWindow = $true
        $startInfo.EnvironmentVariables["CRYPTO_HUD_STATE_DIR"] = $stateDir
        $startInfo.EnvironmentVariables["CRYPTO_HUD_GUI_SMOKE_READY_FILE"] = $readyFile
        $startInfo.EnvironmentVariables["CRYPTO_HUD_INSTANCE_ID"] = "com.crypto-hud.package-smoke.$PID.$Scenario"
        $startInfo.EnvironmentVariables["CRYPTO_HUD_GUI_SMOKE_OFFLINE"] = "1"
        $startInfo.EnvironmentVariables["CRYPTO_HUD_DISABLE_UPDATE_CHECK"] = "1"
        $startInfo.EnvironmentVariables["SLINT_BACKEND"] = "software"
        $process = [System.Diagnostics.Process]::Start($startInfo)
        $deadline = [DateTime]::UtcNow.AddSeconds(15)
        while (-not (Test-Path -LiteralPath $readyFile)) {
            if ($process.HasExited) {
                throw "$Scenario executable exited before its ready marker with code $($process.ExitCode)"
            }
            if ([DateTime]::UtcNow -ge $deadline) {
                throw "$Scenario executable did not write its ready marker"
            }
            Start-Sleep -Milliseconds 100
        }
        $ready = Get-Content -LiteralPath $readyFile -Raw | ConvertFrom-Json
        if (-not [bool]$ready.ready -or -not [bool]$ready.marketDataReady) {
            throw "$Scenario runtime did not reach ready state with deterministic market data"
        }
        $runtimePluginIds = @($ready.widgets | ForEach-Object { [string]$_.pluginId })
        foreach ($pluginId in $expectedPluginIds) {
            if (-not ($runtimePluginIds -contains $pluginId)) {
                throw "$Scenario runtime did not instantiate bundled plugin $pluginId"
            }
            if (-not (@($ready.pluginIds) -contains $pluginId)) {
                throw "$Scenario catalog did not discover bundled plugin $pluginId"
            }
        }
        if (@($ready.catalogErrors).Count -gt 0) {
            throw "$Scenario catalog reported errors: $(@($ready.catalogErrors) -join '; ')"
        }
        if (-not $process.WaitForExit(15000)) {
            throw "$Scenario executable did not exit after the GUI smoke timeout"
        }
        if ($process.ExitCode -ne 0) {
            throw "$Scenario executable exited with code $($process.ExitCode)"
        }
    } finally {
        if ($process -and -not $process.HasExited) {
            $process.Kill()
            $process.WaitForExit()
        }
    }
}

function Assert-PowerShellChildProcessesDisableProfiles {
    $scriptRoots = @(
        (Join-Path $RepoRoot "scripts"),
        (Join-Path $RepoRoot "packaging\windows")
    )
    foreach ($scriptPath in @(
        Get-ChildItem -LiteralPath $scriptRoots -Filter "*.ps1" -File -Recurse |
            Select-Object -ExpandProperty FullName
    )) {
        $tokens = $null
        $parseErrors = $null
        $ast = [System.Management.Automation.Language.Parser]::ParseFile(
            $scriptPath,
            [ref]$tokens,
            [ref]$parseErrors
        )
        if ($parseErrors.Count -gt 0) {
            throw "PowerShell syntax validation failed for ${scriptPath}: $($parseErrors[0].Message)"
        }

        $childPowerShellCommands = $ast.FindAll({
            param($node)

            if ($node -isnot [System.Management.Automation.Language.CommandAst] -or
                $node.CommandElements.Count -eq 0) {
                return $false
            }
            $command = $node.CommandElements[0]
            if ($command -is [System.Management.Automation.Language.VariableExpressionAst]) {
                return $command.VariablePath.UserPath -match '(?i)powershell'
            }
            if ($command -is [System.Management.Automation.Language.StringConstantExpressionAst]) {
                return (Split-Path -Leaf $command.Value) -match '^(?i:powershell|pwsh)(?:\.exe)?$'
            }
            return $false
        }, $true)

        foreach ($command in $childPowerShellCommands) {
            $profileIsDisabledFirst = $command.CommandElements.Count -gt 1 -and
                $command.CommandElements[1].Extent.Text -ieq '-NoProfile'
            if (-not $profileIsDisabledFirst) {
                throw "Child PowerShell invocation must use -NoProfile: ${scriptPath}:$($command.Extent.StartLineNumber)"
            }
        }
    }

    $installerPath = Join-Path $RepoRoot "packaging\windows\install.ps1"
    $installerSource = Get-Content -LiteralPath $installerPath -Raw
    $uninstallCommand = [regex]::Match(
        $installerSource,
        '(?m)^.*-Name\s+"UninstallString".*$'
    )
    if (-not $uninstallCommand.Success -or
        $uninstallCommand.Value -notmatch '(?i)\s-NoProfile\s') {
        throw "Registered uninstall PowerShell command must use -NoProfile: $installerPath"
    }
}

Assert-UnderRepo -Path $PackageRoot
Assert-UnderTemp -Path $IsolatedRoot
Assert-PowerShellChildProcessesDisableProfiles

if (Test-Path -LiteralPath $IsolatedRoot) {
    Assert-UnderTemp -Path $IsolatedRoot
    Remove-Item -LiteralPath $IsolatedRoot -Recurse -Force
}

Push-Location $RepoRoot
try {
    $env:LOCALAPPDATA = Join-Path $ShellSandbox "local-app-data"
    $env:APPDATA = Join-Path $ShellSandbox "roaming-app-data"
    $env:CRYPTO_HUD_ALLOW_UNSIGNED_SMOKE = "1"
    New-Item -ItemType Directory -Force -Path $env:LOCALAPPDATA, $env:APPDATA | Out-Null

    foreach ($unsafeVersion in @('..\escape', 'x/escape', 'C:\escape', 'x..y')) {
        & $PowerShellExe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\package-windows.ps1" `
            -Version $unsafeVersion `
            -SkipBuild `
            -AllowDirty `
            -AllowDevelopmentVersion `
            -AllowUnsignedPackage
        if ($LASTEXITCODE -eq 0) {
            throw "Package script accepted an unsafe version: $unsafeVersion"
        }
    }

    & $PowerShellExe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\package-windows.ps1" `
        -SkipBuild `
        -Sign `
        -CertificatePath (Join-Path $IsolatedRoot "missing-signing-cert.pfx")
    if ($LASTEXITCODE -eq 0) {
        throw "Package script allowed -SkipBuild for a formally signed package"
    }
    & $PowerShellExe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\package-windows.ps1" `
        -SkipBuild `
        -CertificatePassword "smoke-placeholder"
    if ($LASTEXITCODE -eq 0) {
        throw "Package script accepted a plaintext certificate password argument"
    }

    $reparseVersion = "reparse-smoke"
    $reparsePackageRoot = Join-Path $DistDir "crypto-hud-$reparseVersion-windows-x64"
    $reparseTarget = Join-Path $IsolatedRoot "reparse-protected"
    New-Item -ItemType Directory -Force -Path $reparseTarget | Out-Null
    $reparseSentinel = Join-Path $reparseTarget "keep.txt"
    Set-Content -LiteralPath $reparseSentinel -Value "keep"
    if (Test-Path -LiteralPath $reparsePackageRoot) {
        $existingReparseItem = Get-Item -LiteralPath $reparsePackageRoot -Force
        if (($existingReparseItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -eq 0) {
            throw "Reparse smoke fixture path already exists as a regular directory: $reparsePackageRoot"
        }
        [System.IO.Directory]::Delete($reparsePackageRoot, $false)
    }
    New-Item -ItemType Junction -Path $reparsePackageRoot -Target $reparseTarget | Out-Null
    try {
        & $PowerShellExe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\package-windows.ps1" `
            -Version $reparseVersion `
            -SkipBuild `
            -AllowDirty `
            -AllowDevelopmentVersion `
            -AllowUnsignedPackage
        if ($LASTEXITCODE -eq 0) {
            throw "Package script accepted a reparse point as its package directory"
        }
        if (-not (Test-Path -LiteralPath $reparseSentinel)) {
            throw "Package script followed a reparse point and removed external content"
        }
    } finally {
        if (Test-Path -LiteralPath $reparsePackageRoot) {
            $reparseItem = Get-Item -LiteralPath $reparsePackageRoot -Force
            if (($reparseItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -eq 0) {
                throw "Reparse smoke fixture was unexpectedly replaced with a regular directory"
            }
            [System.IO.Directory]::Delete($reparsePackageRoot, $false)
        }
    }

    $packageArgs = @(
        "-ExecutionPolicy", "Bypass",
        "-File", ".\scripts\package-windows.ps1",
        "-Version", $Version,
        "-AllowDirty",
        "-AllowDevelopmentVersion",
        "-AllowUnsignedPackage"
    )
    if ($SkipBuild) {
        $packageArgs += "-SkipBuild"
    }
    $hadEncodedRustFlags = Test-Path "Env:\CARGO_ENCODED_RUSTFLAGS"
    $previousEncodedRustFlags = if ($hadEncodedRustFlags) {
        $env:CARGO_ENCODED_RUSTFLAGS
    } else {
        $null
    }
    try {
        if (-not $SkipBuild) {
            # Exercise the build helper under Cargo's highest-priority rustflags
            # environment variable. The packaged DLL smoke below must still
            # prove that the companion uses the static MSVC runtime.
            $unitSeparator = [char]0x1F
            $env:CARGO_ENCODED_RUSTFLAGS = [string]::Join(
                $unitSeparator,
                @("-C", "target-feature=-crt-static")
            )
        }
        & $PowerShellExe -NoProfile @packageArgs
        if ($LASTEXITCODE -ne 0) {
            throw "Package script failed with code $LASTEXITCODE"
        }
    } finally {
        if ($hadEncodedRustFlags) {
            $env:CARGO_ENCODED_RUSTFLAGS = $previousEncodedRustFlags
        } else {
            Remove-Item Env:\CARGO_ENCODED_RUSTFLAGS -ErrorAction SilentlyContinue
        }
    }

    if (-not (Test-Path $ZipPath)) {
        throw "Package zip was not created: $ZipPath"
    }
    if (-not (Test-Path $ChecksumPath)) {
        throw "Package checksum was not created: $ChecksumPath"
    }

    $checksumLine = Get-Content -LiteralPath $ChecksumPath -Raw
    $expectedZipHash = ($checksumLine -split "\s+")[0]
    Assert-Hash -Path $ZipPath -ExpectedHash $expectedZipHash

    New-Item -ItemType Directory -Force -Path $IsolatedRoot | Out-Null
    Copy-Item -LiteralPath $PackageRoot -Destination $IsolatedPackageRoot -Recurse
    $ActivePackageRoot = $IsolatedPackageRoot
    Assert-ExecutableDoesNotEmbedSourceResourcePath `
        -Path (Join-Path $ActivePackageRoot "crypto-hud.exe")
    Assert-PackagedResourceHeaders -Root $ActivePackageRoot

    $manifestPath = Join-Path $ActivePackageRoot "release-manifest.json"
    if (-not (Test-Path $manifestPath)) {
        throw "Package manifest was not created"
    }
    $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
    if ([int]$manifest.manifestVersion -ne 2) {
        throw "Unexpected manifest version: $($manifest.manifestVersion)"
    }
    if ($manifest.executable -ne "crypto-hud.exe") {
        throw "Unexpected manifest executable: $($manifest.executable)"
    }
    if (-not (@($manifest.files).path -contains "install-update-package.ps1")) {
        throw "Package manifest is missing install-update-package.ps1"
    }
    if (-not (@($manifest.files).path -contains "LICENSE")) {
        throw "Package manifest is missing LICENSE"
    }
    $taskbarDllRelativePath = "resources/taskbar/crypto_hud_taskbar.dll"
    if (-not (@($manifest.files).path -contains $taskbarDllRelativePath)) {
        throw "Package manifest is missing $taskbarDllRelativePath"
    }
    if (-not $manifest.codeSigning) {
        throw "Package manifest is missing codeSigning metadata"
    }
    if (-not (@($manifest.codeSigning.files).path -contains $taskbarDllRelativePath)) {
        throw "Package signing metadata is missing $taskbarDllRelativePath"
    }
    if ([string]$manifest.codeSigning.detachedManifest -ne "release-integrity.ps1") {
        throw "Package manifest is missing signed integrity metadata binding"
    }
    $integrityPath = Join-Path $ActivePackageRoot "release-integrity.ps1"
    if (-not (Test-Path -LiteralPath $integrityPath -PathType Leaf)) {
        throw "Package is missing release-integrity.ps1"
    }
    $integrityContent = Get-Content -LiteralPath $integrityPath -Raw
    $expectedManifestHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $manifestPath).Hash.ToLowerInvariant()
    if ($integrityContent -notmatch "(?m)^# CryptoHud-Manifest-SHA256: $expectedManifestHash`r?$") {
        throw "Release integrity metadata is not bound to the package manifest"
    }
    if ([bool]$manifest.codeSigning.requested -and -not [bool]$manifest.codeSigning.signed) {
        throw "Package manifest says signing was requested but the executable is not signed"
    }
    if (-not [bool]$manifest.codeSigning.requested -and [bool]$manifest.codeSigning.signed) {
        throw "Package manifest says signing was not requested but the executable is signed"
    }
    foreach ($file in @($manifest.files)) {
        Assert-Hash -Path (Resolve-PackageFile -RelativePath ([string]$file.path)) -ExpectedHash ([string]$file.sha256)
    }
    $sourceTrees = @(
        @{
            Root = Join-Path $RepoRoot "crates\crypto-hud\plugins"
            Target = "plugins"
        },
        @{
            Root = Join-Path $RepoRoot "crates\crypto-hud\ui\previews"
            Target = "resources/previews"
        }
    )
    foreach ($tree in $sourceTrees) {
        $sourceRoot = [System.IO.Path]::GetFullPath($tree.Root).TrimEnd('\', '/')
        $sourcePrefix = "$sourceRoot$([System.IO.Path]::DirectorySeparatorChar)"
        foreach ($sourceFile in Get-ChildItem -LiteralPath $sourceRoot -Recurse -File -Force) {
            $relativeSource = $sourceFile.FullName.Substring($sourcePrefix.Length).Replace('\', '/')
            $expectedPackagePath = "$($tree.Target)/$relativeSource"
            if (-not (@($manifest.files).path -contains $expectedPackagePath)) {
                throw "Package manifest omitted release resource: $expectedPackagePath"
            }
        }
    }
    if (-not (@($manifest.files).path -contains "resources/icon.ico")) {
        throw "Package manifest omitted resources/icon.ico"
    }
    $taskbarDllEntry = @($manifest.files | Where-Object {
        [string]$_.path -eq $taskbarDllRelativePath
    })[0]
    Assert-Hash `
        -Path (Join-Path $ActivePackageRoot $taskbarDllRelativePath) `
        -ExpectedHash ([string]$taskbarDllEntry.sha256)
    & $PowerShellExe -NoProfile -ExecutionPolicy Bypass -File $TaskbarDllSmokeScript `
        -DllPath (Join-Path $ActivePackageRoot $taskbarDllRelativePath)
    if ($LASTEXITCODE -ne 0) {
        throw "Packaged taskbar extension DLL smoke failed with code $LASTEXITCODE"
    }
    Invoke-IsolatedPluginRuntimeSmoke `
        -Executable (Join-Path $ActivePackageRoot "crypto-hud.exe") `
        -WorkingDirectory $ActivePackageRoot `
        -Scenario "package"

    $protectedDir = Join-Path $IsolatedRoot "protected"
    Assert-UnderTemp -Path $protectedDir
    New-Item -ItemType Directory -Force -Path $protectedDir | Out-Null
    $sentinel = Join-Path $protectedDir "keep.txt"
    Set-Content -LiteralPath $sentinel -Value "keep"
    & $PowerShellExe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $ActivePackageRoot "uninstall.ps1") -InstallDir $protectedDir -SkipShellIntegration
    if ($LASTEXITCODE -eq 0) {
        throw "Uninstall safety check accepted a non-install directory"
    }
    if (-not (Test-Path -LiteralPath $sentinel)) {
        throw "Uninstall safety check removed a protected directory"
    }
    Remove-Item -LiteralPath $protectedDir -Recurse -Force

    & $PowerShellExe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $ActivePackageRoot "install.ps1") `
        -InstallDir $InstallDir `
        -SkipShellIntegration `
        -AllowUnsignedPackage
    if ($LASTEXITCODE -ne 0) {
        throw "Install smoke failed with code $LASTEXITCODE"
    }

    $installedExe = Join-Path $InstallDir "crypto-hud.exe"
    Assert-Hash -Path $installedExe -ExpectedHash ([string]$manifest.executableSha256)
    if (-not (Test-Path (Join-Path $InstallDir "release-manifest.json"))) {
        throw "Installed manifest was not copied"
    }
    if (-not (Test-Path (Join-Path $InstallDir "install-update-package.ps1"))) {
        throw "Installed update handoff script was not copied"
    }
    if (-not (Test-Path (Join-Path $InstallDir "LICENSE"))) {
        throw "Installed license was not copied"
    }
    if (-not (Test-Path -LiteralPath (Join-Path $InstallDir $taskbarDllRelativePath) -PathType Leaf)) {
        throw "Installed taskbar extension DLL was not copied"
    }
    foreach ($file in @($manifest.files)) {
        $installedPath = [System.IO.Path]::GetFullPath((Join-Path $InstallDir ([string]$file.path)))
        Assert-Hash -Path $installedPath -ExpectedHash ([string]$file.sha256)
    }
    & $PowerShellExe -NoProfile -ExecutionPolicy Bypass -File $TaskbarDllSmokeScript `
        -DllPath (Join-Path $InstallDir $taskbarDllRelativePath)
    if ($LASTEXITCODE -ne 0) {
        throw "Installed taskbar extension DLL smoke failed with code $LASTEXITCODE"
    }
    Assert-PackagedResourceHeaders -Root $InstallDir
    Invoke-IsolatedPluginRuntimeSmoke `
        -Executable $installedExe `
        -WorkingDirectory $InstallDir `
        -Scenario "installed"

    if ($Version -eq "v9999.0.1-smoke") {
        $installedManifestPath = Join-Path $InstallDir "release-manifest.json"
        $installedIntegrityPath = Join-Path $InstallDir "release-integrity.ps1"
        $higherVersion = "v9999.0.2-smoke"
        $higherManifest = Get-Content -LiteralPath $installedManifestPath -Raw | ConvertFrom-Json
        $higherManifest.version = $higherVersion
        $higherManifest | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $installedManifestPath -Encoding UTF8
        $higherManifestHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $installedManifestPath).Hash.ToLowerInvariant()
        $higherIntegrity = Get-Content -LiteralPath $installedIntegrityPath -Raw
        $higherIntegrity = [regex]::Replace(
            $higherIntegrity,
            '(?m)^# CryptoHud-Manifest-SHA256: [a-fA-F0-9]{64}\r?$',
            "# CryptoHud-Manifest-SHA256: $higherManifestHash"
        )
        $higherIntegrity = [regex]::Replace(
            $higherIntegrity,
            '(?m)^# CryptoHud-Version: [0-9A-Za-z][0-9A-Za-z.-]{0,63}\r?$',
            "# CryptoHud-Version: $higherVersion"
        )
        Set-Content -LiteralPath $installedIntegrityPath -Value $higherIntegrity -Encoding UTF8 -NoNewline

        & $PowerShellExe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $ActivePackageRoot "install.ps1") `
            -InstallDir $InstallDir `
            -SkipShellIntegration `
            -AllowUnsignedPackage
        if ($LASTEXITCODE -eq 0) {
            throw "Direct installer accepted a downgrade from $higherVersion to $Version"
        }
        $retainedManifest = Get-Content -LiteralPath $installedManifestPath -Raw | ConvertFrom-Json
        if ([string]$retainedManifest.version -ne $higherVersion) {
            throw "Rejected direct installer downgrade modified the existing installation"
        }
    }

    $sandboxLegacyDir = Join-Path $env:LOCALAPPDATA "CryptoWidget\CryptoHud"
    New-Item -ItemType Directory -Force -Path $sandboxLegacyDir | Out-Null
    $legacySentinel = Join-Path $sandboxLegacyDir "keep.txt"
    Set-Content -LiteralPath $legacySentinel -Value "keep"

    & $PowerShellExe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $InstallDir "uninstall.ps1") -InstallDir $InstallDir -SkipShellIntegration
    if ($LASTEXITCODE -ne 0) {
        throw "Uninstall smoke failed with code $LASTEXITCODE"
    }
    if (Test-Path $installedExe) {
        throw "Uninstall smoke left the executable behind"
    }
    if (-not (Test-Path -LiteralPath $legacySentinel)) {
        throw "SkipShellIntegration removed the isolated legacy installation"
    }

    Write-Host "Package smoke passed"
} finally {
    Pop-Location
    $env:LOCALAPPDATA = $OriginalLocalAppData
    $env:APPDATA = $OriginalAppData
    $env:CRYPTO_HUD_ALLOW_UNSIGNED_SMOKE = $OriginalUnsignedSmoke
    if (-not $KeepPackage) {
        Remove-Item -LiteralPath $PackageRoot -Recurse -Force -ErrorAction SilentlyContinue
        Remove-Item -LiteralPath $ZipPath -Force -ErrorAction SilentlyContinue
        Remove-Item -LiteralPath $ChecksumPath -Force -ErrorAction SilentlyContinue
    }
    if (Test-Path -LiteralPath $IsolatedRoot) {
        Assert-UnderTemp -Path $IsolatedRoot
        Remove-Item -LiteralPath $IsolatedRoot -Recurse -Force
    }
}
