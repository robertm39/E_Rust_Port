param(
    [ValidateRange(1, 100)]
    [int]$Iterations = 20
)

$ErrorActionPreference = 'Stop'
$cargo = (Get-Command cargo).Source
$results = @()

for ($iteration = 1; $iteration -le $Iterations; $iteration++) {
    $timer = [Diagnostics.Stopwatch]::StartNew()
    $startInfo = [Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $cargo
    $startInfo.Arguments =
        'test --locked --all-features --test e_ltb_variant_worker --quiet'
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $process = [Diagnostics.Process]::Start($startInfo)
    $stdout = $process.StandardOutput.ReadToEnd()
    $stderr = $process.StandardError.ReadToEnd()
    $process.WaitForExit()
    $timer.Stop()
    $results += [pscustomobject][ordered]@{
        iteration = $iteration
        exit_code = $process.ExitCode
        elapsed_seconds = [Math]::Round($timer.Elapsed.TotalSeconds, 3)
        stdout = if ($process.ExitCode -eq 0) { $null } else { $stdout }
        stderr = if ($process.ExitCode -eq 0) { $null } else { $stderr }
    }
    Write-Host ("iteration {0}/{1}: exit {2}, {3:N3}s" -f `
        $iteration, $Iterations, $process.ExitCode, $timer.Elapsed.TotalSeconds)
}

$summary = [ordered]@{
    command =
        'cargo test --locked --all-features --test e_ltb_variant_worker --quiet'
    concurrent_hidden_workers_per_iteration = 4
    iterations = $Iterations
    passed = @($results | Where-Object { $_.exit_code -eq 0 }).Count
    failed = @($results | Where-Object { $_.exit_code -ne 0 }).Count
    results = $results
}
$summary | ConvertTo-Json -Depth 4 | Set-Content `
    -LiteralPath (Join-Path $PSScriptRoot 'ltb-results-summary.json') `
    -Encoding utf8

if ($summary.failed -ne 0) {
    exit 1
}
