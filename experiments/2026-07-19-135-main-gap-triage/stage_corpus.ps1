[CmdletBinding()]
param(
    [string]$OutputDirectory = '.artifacts\e-corpus\main-gap-triage-135'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$output = [System.IO.Path]::GetFullPath((Join-Path $repoRoot $OutputDirectory))
[System.IO.Directory]::CreateDirectory($output) | Out-Null

$fixtures = @(
    'eprover\EXAMPLE_PROBLEMS\TPTP\GEO288+1.p',
    'eprover\EXAMPLE_PROBLEMS\SMOKETEST\LUSK6ext.lop'
)

foreach ($fixture in $fixtures) {
    $source = Join-Path $repoRoot $fixture
    $destination = Join-Path $output ([System.IO.Path]::GetFileName($fixture))
    Copy-Item -LiteralPath $source -Destination $destination -Force
}

Get-ChildItem -LiteralPath $output | Select-Object Name, Length
