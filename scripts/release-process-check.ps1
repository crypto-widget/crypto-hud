param(
    [int]$Widgets = 0,
    [int[]]$ScenarioWidgets = @(1, 5, 10),
    [int]$TimeoutMs = 10000,
    [int]$WarmupSeconds = 2,
    [int]$CpuSampleSeconds = 3,
    [int]$CpuSampleIntervalMs = 500,
    [int]$MaxStartupMs = 5000,
    [int]$MaxPrivateMemoryMb = 140,
    [double]$MaxCpuPercent = 25.0,
    [int]$MaxChildProcesses = 0,
    [string]$ReportPath = "",
    [switch]$SkipBuild,
    [switch]$KeepState
)

$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$Exe = Join-Path $RepoRoot "target\release\crypto-hud.exe"
$StateRoot = Join-Path $RepoRoot "target\tmp\release-process-state"

if ([string]::IsNullOrWhiteSpace($ReportPath)) {
    $ReportPath = Join-Path $RepoRoot "dist\release-process-check.json"
}

function Assert-UnderRepo {
    param([string]$Path)

    $fullPath = [System.IO.Path]::GetFullPath($Path)
    $repoPath = [System.IO.Path]::GetFullPath($RepoRoot)
    if (-not $fullPath.StartsWith($repoPath, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to operate outside repository: $fullPath"
    }
}

function Get-DescendantProcesses {
    param(
        [uint32]$RootProcessId,
        [object[]]$AllProcesses
    )

    $children = @($AllProcesses | Where-Object { $_.ParentProcessId -eq $RootProcessId })
    foreach ($child in $children) {
        $child
        Get-DescendantProcesses -RootProcessId ([uint32]$child.ProcessId) -AllProcesses $AllProcesses
    }
}

function Get-MemoryTargetMb {
    param([int]$WidgetCount)

    switch ($WidgetCount) {
        1 { return 60 }
        5 { return 90 }
        10 { return 130 }
        default { return $MaxPrivateMemoryMb }
    }
}

function Get-StartupTargetMs {
    param([int]$WidgetCount)

    switch ($WidgetCount) {
        1 { return 500 }
        5 { return 1000 }
        10 { return 1500 }
        default { return $MaxStartupMs }
    }
}

function Measure-ProcessCpu {
    param(
        [System.Diagnostics.Process]$Process,
        [int]$SampleSeconds,
        [int]$IntervalMs
    )

    $samples = @()
    if ($SampleSeconds -le 0) {
        return [ordered]@{
            averagePercent = 0.0
            peakPercent = 0.0
            samples = @()
        }
    }

    $processorCount = [Environment]::ProcessorCount
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    $Process.Refresh()
    $previousCpu = $Process.TotalProcessorTime
    $previousElapsed = $watch.Elapsed

    while ($watch.Elapsed.TotalSeconds -lt $SampleSeconds) {
        Start-Sleep -Milliseconds $IntervalMs
        if ($Process.HasExited) {
            break
        }

        $Process.Refresh()
        $currentCpu = $Process.TotalProcessorTime
        $currentElapsed = $watch.Elapsed
        $elapsedSeconds = ($currentElapsed - $previousElapsed).TotalSeconds
        if ($elapsedSeconds -gt 0) {
            $cpuSeconds = ($currentCpu - $previousCpu).TotalSeconds
            $percent = ($cpuSeconds / ($elapsedSeconds * $processorCount)) * 100.0
            $samples += [Math]::Round([Math]::Max(0.0, $percent), 2)
        }
        $previousCpu = $currentCpu
        $previousElapsed = $currentElapsed
    }

    if ($samples.Count -eq 0) {
        return [ordered]@{
            averagePercent = 0.0
            peakPercent = 0.0
            samples = @()
        }
    }

    $average = ($samples | Measure-Object -Average).Average
    $peak = ($samples | Measure-Object -Maximum).Maximum
    [ordered]@{
        averagePercent = [Math]::Round($average, 2)
        peakPercent = [Math]::Round($peak, 2)
        samples = @($samples)
    }
}

function Invoke-ReleaseScenario {
    param([int]$WidgetCount)

    $stateDir = Join-Path $StateRoot "widgets-$WidgetCount"
    $readyFile = Join-Path $stateDir "ready.json"
    $process = $null
    $failures = @()

    if (Test-Path $stateDir) {
        Remove-Item -LiteralPath $stateDir -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $stateDir | Out-Null

    try {
        $psi = [System.Diagnostics.ProcessStartInfo]::new()
        $psi.FileName = $Exe
        $psi.Arguments = "--widgets $WidgetCount --show-settings --gui-smoke-ms $TimeoutMs"
        $psi.WorkingDirectory = $RepoRoot
        $psi.UseShellExecute = $false
        $psi.Environment["CRYPTO_HUD_STATE_DIR"] = $stateDir
        $psi.Environment["CRYPTO_HUD_GUI_SMOKE_READY_FILE"] = $readyFile
        $psi.Environment["CRYPTO_HUD_INSTANCE_ID"] = "com.crypto-hud.release-process.$PID.$WidgetCount"
        $psi.Environment["CRYPTO_HUD_DISABLE_UPDATE_CHECK"] = "1"
        $psi.Environment["SLINT_BACKEND"] = "software"

        $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
        $process = [System.Diagnostics.Process]::Start($psi)

        while (-not (Test-Path $readyFile)) {
            if ($process.HasExited) {
                throw "Release process exited before ready marker with code $($process.ExitCode)"
            }
            if ($stopwatch.ElapsedMilliseconds -gt $MaxStartupMs) {
                throw "Ready marker was not written within $MaxStartupMs ms"
            }
            Start-Sleep -Milliseconds 100
        }
        $startupMs = [int]$stopwatch.ElapsedMilliseconds

        $ready = Get-Content -LiteralPath $readyFile -Raw | ConvertFrom-Json
        if (-not $ready.ready) {
            $failures += "ready marker did not report ready"
        }
        if ([int]$ready.widgetCount -lt $WidgetCount) {
            $failures += "expected at least $WidgetCount widgets, saw $($ready.widgetCount)"
        }

        Start-Sleep -Seconds $WarmupSeconds
        if ($process.HasExited) {
            throw "Release process exited before metrics collection with code $($process.ExitCode)"
        }

        $appProcess = Get-Process -Id $process.Id -ErrorAction Stop
        $privateMemoryMb = [Math]::Round($appProcess.PrivateMemorySize64 / 1MB, 2)
        $cpu = Measure-ProcessCpu -Process $appProcess -SampleSeconds $CpuSampleSeconds -IntervalMs $CpuSampleIntervalMs
        $allProcesses = @(Get-CimInstance Win32_Process)
        $descendants = @(Get-DescendantProcesses -RootProcessId ([uint32]$process.Id) -AllProcesses $allProcesses)
        $repoPath = [System.IO.Path]::GetFullPath($RepoRoot)
        $statePath = [System.IO.Path]::GetFullPath($stateDir)
        $attributedWebView2 = @(
            $allProcesses | Where-Object {
                $_.Name -ieq "msedgewebview2.exe" -and (
                    ($_.ParentProcessId -eq $process.Id) -or
                    (($_.CommandLine -as [string]) -like "*$repoPath*") -or
                    (($_.CommandLine -as [string]) -like "*$statePath*")
                )
            }
        )

        $memoryTargetMb = Get-MemoryTargetMb -WidgetCount $WidgetCount
        $startupTargetMs = Get-StartupTargetMs -WidgetCount $WidgetCount
        if ($startupMs -gt $startupTargetMs) {
            $failures += "startup ${startupMs}ms exceeded ${startupTargetMs}ms target"
        }
        if ($privateMemoryMb -gt $memoryTargetMb) {
            $failures += "private memory ${privateMemoryMb}MB exceeded ${memoryTargetMb}MB target"
        }
        if ($cpu.averagePercent -gt $MaxCpuPercent) {
            $failures += "average CPU $($cpu.averagePercent)% exceeded ${MaxCpuPercent}% gate"
        }
        if ($descendants.Count -gt $MaxChildProcesses) {
            $failures += "child process count $($descendants.Count) exceeded $MaxChildProcesses"
        }
        if ($attributedWebView2.Count -gt 0) {
            $failures += "found attributed msedgewebview2.exe process"
        }

        if (-not $process.WaitForExit($TimeoutMs + 5000)) {
            $process.Kill()
            $failures += "release process did not exit after GUI smoke timeout"
        } elseif ($process.ExitCode -ne 0) {
            $failures += "release process exited with code $($process.ExitCode)"
        }

        [ordered]@{
            ready = [bool]$ready.ready
            widgetCount = [int]$ready.widgetCount
            requestedWidgets = $WidgetCount
            settingsWindowRequested = [bool]$ready.settingsWindowRequested
            startupMs = $startupMs
            startupTargetMs = $startupTargetMs
            warmupSeconds = $WarmupSeconds
            cpuSampleSeconds = $CpuSampleSeconds
            cpu = $cpu
            privateMemoryMb = $privateMemoryMb
            privateMemoryTargetMb = $memoryTargetMb
            processId = $process.Id
            childProcessCount = $descendants.Count
            childProcesses = @($descendants | ForEach-Object {
                [ordered]@{
                    processId = $_.ProcessId
                    parentProcessId = $_.ParentProcessId
                    name = $_.Name
                    commandLine = $_.CommandLine
                }
            })
            webView2ProcessCount = $attributedWebView2.Count
            webView2Processes = @($attributedWebView2 | ForEach-Object {
                [ordered]@{
                    processId = $_.ProcessId
                    parentProcessId = $_.ParentProcessId
                    commandLine = $_.CommandLine
                }
            })
            failures = @($failures)
        }
    } catch {
        [ordered]@{
            ready = $false
            widgetCount = 0
            requestedWidgets = $WidgetCount
            settingsWindowRequested = $false
            failures = @($_.Exception.Message)
        }
    } finally {
        if ($process -and -not $process.HasExited) {
            $process.Kill()
            $process.WaitForExit()
        }
    }
}

Assert-UnderRepo -Path $StateRoot
Assert-UnderRepo -Path $ReportPath

if ($TimeoutMs -lt (($WarmupSeconds + $CpuSampleSeconds) * 1000 + 1500)) {
    throw "TimeoutMs must leave enough time for ready detection, warmup, and CPU sampling"
}

$scenarioWidgetsToRun = if ($Widgets -gt 0) {
    @($Widgets)
} else {
    @($ScenarioWidgets | Sort-Object -Unique)
}

if ($scenarioWidgetsToRun.Count -eq 0) {
    throw "At least one scenario must be requested"
}

if (Test-Path $StateRoot) {
    Remove-Item -LiteralPath $StateRoot -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $StateRoot | Out-Null
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $ReportPath) | Out-Null

Push-Location $RepoRoot
try {
    if (-not $SkipBuild) {
        cargo build --locked --release -p crypto-hud
        if ($LASTEXITCODE -ne 0) {
            throw "Release build failed with code $LASTEXITCODE"
        }
    }

    if (-not (Test-Path $Exe)) {
        throw "Release executable not found: $Exe"
    }

    $scenarioReports = @()
    foreach ($widgetCount in $scenarioWidgetsToRun) {
        Write-Host "Checking release process scenario: $widgetCount widget(s)"
        $scenarioReports += Invoke-ReleaseScenario -WidgetCount $widgetCount
    }

    $allFailures = @()
    foreach ($scenario in $scenarioReports) {
        foreach ($failure in $scenario.failures) {
            $allFailures += "$($scenario.requestedWidgets) widget(s): $failure"
        }
    }

    $report = [ordered]@{
        ready = ($allFailures.Count -eq 0)
        generatedAt = (Get-Date).ToUniversalTime().ToString("o")
        executable = $Exe
        scenarios = @($scenarioReports)
        thresholds = [ordered]@{
            maxStartupMs = $MaxStartupMs
            maxCpuPercent = $MaxCpuPercent
            maxChildProcesses = $MaxChildProcesses
            startupTargetsMs = [ordered]@{
                widgets1 = 500
                widgets5 = 1000
                widgets10 = 1500
                fallback = $MaxStartupMs
            }
            memoryTargetsMb = [ordered]@{
                widgets1 = 60
                widgets5 = 90
                widgets10 = 130
                fallback = $MaxPrivateMemoryMb
            }
        }
        failures = @($allFailures)
    }
    $report | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $ReportPath

    if ($allFailures.Count -gt 0) {
        throw "Release process check failed: $($allFailures -join '; ')"
    }

    Write-Host "Release process check passed"
    Write-Host "Report: $ReportPath"
} finally {
    Pop-Location
    if (-not $KeepState -and (Test-Path $StateRoot)) {
        Remove-Item -LiteralPath $StateRoot -Recurse -Force
    }
}
