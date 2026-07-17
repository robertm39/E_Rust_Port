param(
    [ValidateRange(1, 100)]
    [int]$Iterations = 10
)

$ErrorActionPreference = 'Stop'
$experimentDir = $PSScriptRoot
$rawDir = Join-Path $experimentDir 'raw'
New-Item -ItemType Directory -Force -Path $rawDir | Out-Null
$cargo = (Get-Command cargo).Source
$results = @()

for ($iteration = 1; $iteration -le $Iterations; $iteration++) {
    $stdoutPath = Join-Path $rawDir ("default-parallel-{0:D3}.stdout.log" -f $iteration)
    $stderrPath = Join-Path $rawDir ("default-parallel-{0:D3}.stderr.log" -f $iteration)
    $timer = [Diagnostics.Stopwatch]::StartNew()
    $startInfo = [Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $cargo
    $startInfo.Arguments = 'test --locked --all-targets --all-features --quiet'
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $process = [Diagnostics.Process]::Start($startInfo)
    $stdout = $process.StandardOutput.ReadToEnd()
    $stderr = $process.StandardError.ReadToEnd()
    $process.WaitForExit()
    [IO.File]::WriteAllText($stdoutPath, $stdout)
    [IO.File]::WriteAllText($stderrPath, $stderr)
    $exitCode = $process.ExitCode
    $timer.Stop()
    $stdoutHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $stdoutPath).Hash.ToLowerInvariant()
    $stderrHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $stderrPath).Hash.ToLowerInvariant()
    $failureStdoutLog = $null
    $failureStderrLog = $null
    if ($exitCode -eq 0) {
        Remove-Item -LiteralPath $stdoutPath
        Remove-Item -LiteralPath $stderrPath
    }
    else {
        $failureStdoutLog = "raw/$([IO.Path]::GetFileName($stdoutPath))"
        $failureStderrLog = "raw/$([IO.Path]::GetFileName($stderrPath))"
    }
    $result = [ordered]@{
        iteration = $iteration
        exit_code = $exitCode
        elapsed_seconds = [Math]::Round($timer.Elapsed.TotalSeconds, 3)
        stdout_sha256 = $stdoutHash
        stderr_sha256 = $stderrHash
        failure_stdout_log = $failureStdoutLog
        failure_stderr_log = $failureStderrLog
    }
    $results += [pscustomobject]$result
    Write-Host ("iteration {0}/{1}: exit {2}, {3:N3}s" -f `
        $iteration, $Iterations, $exitCode, $timer.Elapsed.TotalSeconds)
}

$summary = [ordered]@{
    command = 'cargo test --locked --all-targets --all-features --quiet'
    default_parallel = $true
    iterations = $Iterations
    passed = @($results | Where-Object { $_.exit_code -eq 0 }).Count
    failed = @($results | Where-Object { $_.exit_code -ne 0 }).Count
    results = $results
}
$summary | ConvertTo-Json -Depth 4 | Set-Content `
    -LiteralPath (Join-Path $experimentDir 'results-summary.json') `
    -Encoding utf8

if ($summary.failed -ne 0) {
    exit 1
}
