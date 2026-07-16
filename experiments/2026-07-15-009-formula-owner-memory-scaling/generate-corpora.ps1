[CmdletBinding()]
param(
    [ValidateNotNullOrEmpty()]
    [int[]]$Counts = @(100, 1000, 5000, 10000, 20000),
    [string]$OutputDirectory = (Join-Path $PSScriptRoot '..\..\.artifacts\experiments\2026-07-15-009-formula-owner-memory-scaling\corpus')
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$null = New-Item -ItemType Directory -Force -Path $OutputDirectory
foreach ($count in $Counts) {
    if ($count -lt 1 -or $count -gt 1000000) {
        throw "Count $count is outside the supported range 1..1000000"
    }

    foreach ($shape in 'repeated', 'unique') {
        $lines = [System.Collections.Generic.List[string]]::new($count + 2)
        $lines.Add('% Status : Theorem')
        for ($index = 0; $index -lt $count; $index++) {
            if ($shape -eq 'repeated') {
                $lines.Add("fof(source_$index, axiom, p(a)).")
            }
            else {
                $lines.Add("fof(source_$index, axiom, p_$index(a_$index)).")
            }
        }
        if ($shape -eq 'repeated') {
            $lines.Add('fof(goal, conjecture, p(a)).')
        }
        else {
            $lines.Add('fof(goal, conjecture, p_0(a_0)).')
        }

        $path = Join-Path $OutputDirectory ('{0}-{1:D5}.p' -f $shape, $count)
        [System.IO.File]::WriteAllLines(
            $path,
            $lines,
            [System.Text.UTF8Encoding]::new($false)
        )
        Write-Host "Wrote $($lines.Count) records to $path"
    }
}
