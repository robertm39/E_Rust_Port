param(
    [ValidateRange(1, 100)]
    [int]$Iterations = 3
)

$ErrorActionPreference = 'Stop'
$previousTptp = [Environment]::GetEnvironmentVariable('TPTP', 'Process')

function Invoke-Cargo {
    param([string[]]$Arguments)

    & cargo @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "cargo $($Arguments -join ' ') failed with exit code $LASTEXITCODE"
    }
}

try {
    for ($iteration = 1; $iteration -le $Iterations; $iteration++) {
        Write-Host "stress iteration $iteration/$Iterations"

        [Environment]::SetEnvironmentVariable('TPTP', 'Problems', 'Process')
        Invoke-Cargo @(
            'test', '--locked', '--lib',
            'inout::scanner::tests::include_key_splices_included_files_and_resumes_parent_stream',
            '--', '--exact'
        )
        [Environment]::SetEnvironmentVariable('TPTP', $previousTptp, 'Process')

        Invoke-Cargo @(
            'test', '--locked', '--lib',
            'prover::eprover::tests::configured_output_direct_global_write_precedes_pending_stdio_buffer',
            '--', '--exact'
        )
        Invoke-Cargo @(
            'test', '--locked', '--lib',
            'prover::eprover::tests::run_hard_time_limit_uses_cpu_limit_exit_status',
            '--', '--exact'
        )
        Invoke-Cargo @('test', '--locked', '--lib', '--quiet')
    }
}
finally {
    [Environment]::SetEnvironmentVariable('TPTP', $previousTptp, 'Process')
}
