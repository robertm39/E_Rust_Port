[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Binary,

    [Parameter(Mandatory = $true)]
    [string]$Problem,

    [Parameter(Mandatory = $true)]
    [string]$OutputDirectory,

    [ValidateRange(1, 100)]
    [int]$Runs = 1
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$resolvedBinary = (Resolve-Path $Binary).Path
$resolvedProblem = (Resolve-Path $Problem).Path
$resolvedOutput = [System.IO.Path]::GetFullPath($OutputDirectory)
[System.IO.Directory]::CreateDirectory($resolvedOutput) | Out-Null

$records = @()
for ($run = 1; $run -le $Runs; $run++) {
    $stdout = Join-Path $resolvedOutput ("run-{0:D2}.stdout" -f $run)
    $stderr = Join-Path $resolvedOutput ("run-{0:D2}.stderr" -f $run)
    $arguments = @(
        $resolvedProblem,
        '--auto',
        '--silent',
        '--cpu-limit=60',
        '--memory-limit=2048',
        '--detsort-rw',
        '--detsort-new',
        '--proof-object=1'
    )

    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $resolvedBinary
    $startInfo.Arguments = ($arguments | ForEach-Object { '"' + $_ + '"' }) -join ' '
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true

    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
    if (-not $process.Start()) {
        throw "Could not start $resolvedBinary"
    }
    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    $process.WaitForExit()
    $exitCode = $process.ExitCode
    $peakWorkingSet = $process.PeakWorkingSet64
    $stopwatch.Stop()
    [System.IO.File]::WriteAllText($stdout, $stdoutTask.Result)
    [System.IO.File]::WriteAllText($stderr, $stderrTask.Result)

    $status = Select-String -Path $stdout -Pattern '^# SZS status (\S+)' |
        Select-Object -Last 1
    $records += [pscustomobject]@{
        run = $run
        exit_code = $exitCode
        wall_seconds = [Math]::Round($stopwatch.Elapsed.TotalSeconds, 6)
        peak_working_set_bytes = $peakWorkingSet
        status = if ($status) { $status.Matches[0].Groups[1].Value } else { $null }
        stderr_bytes = (Get-Item $stderr).Length
    }
}

$records | Export-Csv -NoTypeInformation (Join-Path $resolvedOutput 'results.csv')
$records | Format-Table -AutoSize
