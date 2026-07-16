[CmdletBinding()]
param(
    [ValidateRange(1, 1000000)]
    [int]$Count = 20000,
    [string]$OutputPath = (Join-Path $PSScriptRoot '..\..\.artifacts\experiments\2026-07-15-007-formula-set-front-drain\corpus\formula-drain.p')
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$outputDirectory = Split-Path -Parent $OutputPath
if ($outputDirectory) {
    $null = New-Item -ItemType Directory -Force -Path $outputDirectory
}

$lines = [System.Collections.Generic.List[string]]::new($Count + 2)
$lines.Add('% Status : Theorem')
for ($index = 0; $index -lt $Count; $index++) {
    $lines.Add("fof(source_$index, axiom, p(a)).")
}
$lines.Add('fof(goal, conjecture, p(a)).')

[System.IO.File]::WriteAllLines(
    $OutputPath,
    $lines,
    [System.Text.UTF8Encoding]::new($false)
)
Write-Host "Wrote $($lines.Count) records to $OutputPath"
