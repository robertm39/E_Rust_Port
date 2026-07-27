[CmdletBinding()]
param(
    [ValidateRange(1, 100000)]
    [int]$Count = 1000,
    [string]$OutputPath = (Join-Path $PSScriptRoot 'lambda-lift-corpus\lambda-lift-pdtree.p')
)

$outputDirectory = Split-Path -Parent $OutputPath
if ($outputDirectory) {
    $null = New-Item -ItemType Directory -Force -Path $outputDirectory
}
$lines = [System.Collections.Generic.List[string]]::new((2 * $Count) + 5)
$lines.Add('thf(k_type, type, k: $i > $i > $i).')
$lines.Add('thf(h_type, type, h: ($i > $i) > $o).')
for ($index = 0; $index -lt $Count; $index++) {
    $lines.Add("thf(c_${index}_type, type, c_$($index): `$i).")
    $lines.Add("thf(lambda_$index, axiom, h @ (^[X: `$i]: (k @ c_$index @ X))).")
}
$lines.Add('thf(goal, conjecture, h @ (^[X: $i]: (k @ c_0 @ X))).')

[System.IO.File]::WriteAllLines(
    $OutputPath,
    $lines,
    [System.Text.UTF8Encoding]::new($false)
)
Write-Host "Wrote $($lines.Count) records to $OutputPath"
