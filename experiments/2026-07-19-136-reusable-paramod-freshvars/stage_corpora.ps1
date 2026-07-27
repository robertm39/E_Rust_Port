[CmdletBinding()]
param(
    [string]$OutputRoot = '.artifacts\e-corpus\reusable-paramod-freshvars-136'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$output = [System.IO.Path]::GetFullPath((Join-Path $repoRoot $OutputRoot))
$corpora = @{
    proof = @(
        'eprover\EXAMPLE_PROBLEMS\TPTP\GEO288+1.p'
        'eprover\EXAMPLE_PROBLEMS\TPTP\HEN011-2.p'
        'eprover\EXAMPLE_PROBLEMS\SMOKETEST\LUSK6.lop'
        'eprover\EXAMPLE_PROBLEMS\SMOKETEST\LUSK6ext.lop'
    )
    proof_lusk = @(
        'eprover\EXAMPLE_PROBLEMS\SMOKETEST\LUSK6.lop'
    )
    resource = @(
        'eprover\EXAMPLE_PROBLEMS\SMOKETEST\BOO020-1.p'
        'eprover\EXAMPLE_PROBLEMS\TPTP\SWV851-1.p'
    )
    resource_boo = @(
        'eprover\EXAMPLE_PROBLEMS\SMOKETEST\BOO020-1.p'
    )
    resource_swv = @(
        'eprover\EXAMPLE_PROBLEMS\TPTP\SWV851-1.p'
    )
}

foreach ($corpus in $corpora.GetEnumerator()) {
    $directory = Join-Path $output $corpus.Key
    [System.IO.Directory]::CreateDirectory($directory) | Out-Null
    foreach ($fixture in $corpus.Value) {
        $source = Join-Path $repoRoot $fixture
        $destination = Join-Path $directory ([System.IO.Path]::GetFileName($fixture))
        Copy-Item -LiteralPath $source -Destination $destination -Force
    }
}

$proofAxioms = Join-Path $output 'proof\Axioms'
[System.IO.Directory]::CreateDirectory($proofAxioms) | Out-Null
Copy-Item `
    -LiteralPath (Join-Path $repoRoot 'eprover\EXAMPLE_PROBLEMS\TPTP\Axioms\HEN001-0.ax') `
    -Destination (Join-Path $proofAxioms 'HEN001-0.ax') `
    -Force

Get-ChildItem -LiteralPath $output -Recurse -File |
    Select-Object FullName, Length
