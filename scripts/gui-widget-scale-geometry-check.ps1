$ErrorActionPreference = "Stop"

$sourcePath = Join-Path $PSScriptRoot "gui-widget-scale-smoke.ps1"
$tokens = $null
$errors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile($sourcePath, [ref]$tokens, [ref]$errors)
if ($errors.Count -gt 0) { throw ($errors -join "; ") }

# Load only the pure geometry helpers, without building or starting the app.
foreach ($name in @("Get-SettingsScrollPoint", "Get-VisibleRectanglePoint")) {
    $function = $ast.Find({ param($node) $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq $name }, $true)
    if (-not $function) { throw "Missing geometry helper: $name" }
    . ([scriptblock]::Create($function.Extent.Text))
}

function Assert-Close([double]$Actual, [double]$Expected) {
    if ([Math]::Abs($Actual - $Expected) -gt 0.01) { throw "Expected $Expected, saw $Actual" }
}

foreach ($dpi in @(1.0, 1.25, 1.5, 2.0)) {
    $client = [pscustomobject]@{ Left = -1920; Top = 40; Width = 1120 * $dpi; Height = 720 * $dpi; ScaleFactor = $dpi }
    $anchor = [pscustomobject]@{ Left = -1920 + 828 * $dpi; Top = 9000; Width = 150 * $dpi; Height = 34 * $dpi }
    $point = Get-SettingsScrollPoint $anchor $client
    Assert-Close $point.X ([Math]::Round(-1920 + 903 * $dpi))
    Assert-Close $point.Y ([Math]::Round(40 + 360 * $dpi))
}

# A short client still scrolls from its visible middle, even when the anchor
# is vertically outside the viewport. Clip partially visible columns as well.
$smallClient = [pscustomobject]@{ Left = 20; Top = 40; Width = 1120; Height = 500 }
$anchor = [pscustomobject]@{ Left = 1100; Top = -9000; Width = 150; Height = 34 }
$point = Get-SettingsScrollPoint $anchor $smallClient
if ($point.X -ne 1120 -or $point.Y -ne 290) { throw "Scroll point must stay inside the visible client" }
$anchor.Left = 1200
if ($null -ne (Get-SettingsScrollPoint $anchor $smallClient)) { throw "An off-screen scroll column must not produce an input point" }

$viewport = [pscustomobject]@{ Left = -100; Top = 20; Width = 80; Height = 60 }
$visible = [pscustomobject]@{ Left = -90; Top = 30; Width = 20; Height = 10 }
$point = Get-VisibleRectanglePoint $visible $viewport
if ($point.X -ne -80 -or $point.Y -ne 35) { throw "Visible rectangle center is incorrect" }
$clipped = [pscustomobject]@{ Left = -40; Top = 70; Width = 50; Height = 30 }
$point = Get-VisibleRectanglePoint $clipped $viewport
if ($point.X -ne -30 -or $point.Y -ne 75) { throw "Clipped rectangle center must remain inside the viewport" }
foreach ($bounds in @(
    [pscustomobject]@{ Left = 0; Top = 100; Width = 20; Height = 20 },
    [pscustomobject]@{ Left = -90; Top = 30; Width = 0; Height = 10 },
    [pscustomobject]@{ Left = [double]::PositiveInfinity; Top = 30; Width = 20; Height = 10 }
)) {
    if ($null -ne (Get-VisibleRectanglePoint $bounds $viewport)) { throw "An empty, off-screen, or invalid rectangle must not produce an input point" }
}

$scroll = $ast.Find({ param($node) $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq "Scroll-AutomationPanel" }, $true)
if ($scroll.Extent.Text -match 'GetClickablePoint|Show coin logos|Hide quote asset') { throw "Scrolling must not depend on the target switch already being clickable" }
Write-Output "Widget scale geometry checks passed"
