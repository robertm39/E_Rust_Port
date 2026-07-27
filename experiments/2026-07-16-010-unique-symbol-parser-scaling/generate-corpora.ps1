[CmdletBinding()]
param(
    [ValidateNotNullOrEmpty()]
    [int[]]$Counts = @(100, 1000, 5000, 10000, 20000),
    [ValidateSet('atom', 'implication', 'negated', 'quantified')]
    [string[]]$Shapes = @('atom', 'implication', 'negated', 'quantified'),
    [string]$OutputDirectory = (Join-Path $PSScriptRoot '..\..\.artifacts\experiments\2026-07-16-010-unique-symbol-parser-scaling\corpus')
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$null = New-Item -ItemType Directory -Force -Path $OutputDirectory
foreach ($count in $Counts) {
    if ($count -lt 1 -or $count -gt 1000000) {
        throw "Count $count is outside the supported range 1..1000000"
    }

    foreach ($shape in $Shapes) {
        $lines = [System.Collections.Generic.List[string]]::new($count + 2)
        $lines.Add('% Status : Theorem')
        for ($index = 0; $index -lt $count; $index++) {
            switch ($shape) {
                'atom' { $lines.Add("fof(source_$index, axiom, p_$index(a_$index)).") }
                'implication' { $lines.Add("fof(source_$index, axiom, p_$index(a_$index) => q_$index(b_$index)).") }
                'negated' { $lines.Add("fof(source_$index, axiom, ~p_$index(a_$index)).") }
                'quantified' { $lines.Add("fof(source_$index, axiom, ![X]:p_$index(X)).") }
            }
        }
        switch ($shape) {
            'atom' { $lines.Add('fof(goal, conjecture, p_0(a_0)).') }
            'implication' { $lines.Add('fof(goal, conjecture, p_0(a_0) => q_0(b_0)).') }
            'negated' { $lines.Add('fof(goal, conjecture, ~p_0(a_0)).') }
            'quantified' { $lines.Add('fof(goal, conjecture, ![X]:p_0(X)).') }
        }

        $path = Join-Path $OutputDirectory ('unique-{0}-{1:D5}.p' -f $shape, $count)
        [System.IO.File]::WriteAllLines(
            $path,
            $lines,
            [System.Text.UTF8Encoding]::new($false)
        )
        Write-Host "Wrote $($lines.Count) records to $path"
    }
}
