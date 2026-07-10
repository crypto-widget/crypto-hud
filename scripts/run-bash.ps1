param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]] $BashArguments
)

$ErrorActionPreference = "Stop"

$bashCommand = Get-Command bash.exe -ErrorAction SilentlyContinue
if ($null -ne $bashCommand) {
    $bashPath = $bashCommand.Source
} else {
    $gitCommand = Get-Command git.exe -ErrorAction SilentlyContinue
    if ($null -eq $gitCommand) {
        throw "Git Bash is required but neither bash.exe nor git.exe was found"
    }

    $gitRoot = Split-Path (Split-Path $gitCommand.Source -Parent) -Parent
    $bashPath = Join-Path $gitRoot "bin\bash.exe"
    if (-not (Test-Path -LiteralPath $bashPath -PathType Leaf)) {
        throw "Git Bash was not found at $bashPath"
    }
}

& $bashPath @BashArguments
exit $LASTEXITCODE
