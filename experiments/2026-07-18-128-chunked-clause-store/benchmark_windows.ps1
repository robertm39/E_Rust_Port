[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$BaselineBinary,

    [Parameter(Mandatory = $true)]
    [string]$CandidateBinary,

    [Parameter(Mandatory = $true)]
    [string]$Problem,

    [Parameter(Mandatory = $true)]
    [string]$OutputCsv,

    [ValidateRange(1, 20)]
    [int]$Runs = 5,

    [ValidateRange(1, 600)]
    [int]$CpuLimit = 60
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$implementations = @(
    [pscustomobject]@{
        label = 'baseline'
        binary = (Resolve-Path $BaselineBinary).Path
    },
    [pscustomobject]@{
        label = 'candidate'
        binary = (Resolve-Path $CandidateBinary).Path
    }
)
$problemPath = (Resolve-Path $Problem).Path
$outputPath = [System.IO.Path]::GetFullPath($OutputCsv)
[System.IO.Directory]::CreateDirectory([System.IO.Path]::GetDirectoryName($outputPath)) |
    Out-Null

function Measure-Run {
    param(
        [Parameter(Mandatory = $true)]
        [pscustomobject]$Implementation,

        [Parameter(Mandatory = $true)]
        [int]$Run
    )

    $arguments = @(
        $problemPath,
        '--auto',
        '--silent',
        "--cpu-limit=$CpuLimit",
        '--memory-limit=2048',
        '--detsort-rw',
        '--detsort-new',
        '--proof-object=1'
    )
    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $Implementation.binary
    $startInfo.Arguments = ($arguments | ForEach-Object { '"' + $_ + '"' }) -join ' '
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true

    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
    if (-not $process.Start()) {
        throw "Could not start $($Implementation.binary)"
    }
    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    $sampledPeak = 0L
    while (-not $process.HasExited) {
        try {
            $process.Refresh()
            $sampledPeak = [Math]::Max($sampledPeak, $process.WorkingSet64)
        }
        catch [System.InvalidOperationException] {
            # The process may exit between HasExited and Refresh.
        }
        Start-Sleep -Milliseconds 5
    }
    $process.WaitForExit()
    $stopwatch.Stop()
    $process.Refresh()
    $cpuSeconds = $process.TotalProcessorTime.TotalSeconds
    $exitCode = $process.ExitCode
    $stdout = $stdoutTask.Result
    $stderr = $stderrTask.Result
    $process.Dispose()

    $statusMatch = [regex]::Matches($stdout, '(?m)^%+ SZS status (\S+)') |
        Select-Object -Last 1
    [pscustomobject]@{
        implementation = $Implementation.label
        run = $Run
        exit_code = $exitCode
        status = if ($statusMatch) { $statusMatch.Groups[1].Value } else { $null }
        wall_seconds = [Math]::Round($stopwatch.Elapsed.TotalSeconds, 6)
        cpu_seconds = [Math]::Round($cpuSeconds, 6)
        sampled_peak_kib = [Math]::Round($sampledPeak / 1KB)
        stdout_bytes = [System.Text.Encoding]::UTF8.GetByteCount($stdout)
        stderr = $stderr.Trim()
    }
}

$rows = @()
for ($run = 1; $run -le $Runs; $run++) {
    $order = if ($run % 2 -eq 0) {
        @($implementations[1], $implementations[0])
    }
    else {
        $implementations
    }
    foreach ($implementation in $order) {
        $rows += Measure-Run -Implementation $implementation -Run $run
    }
}

$rows | Export-Csv -NoTypeInformation $outputPath
$rows | Format-Table -AutoSize
