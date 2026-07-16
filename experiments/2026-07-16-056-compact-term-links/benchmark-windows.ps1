[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$BaselineBinary,

    [Parameter(Mandatory = $true)]
    [string]$CandidateBinary,

    [Parameter(Mandatory = $true)]
    [string]$OutputCsv,

    [ValidateRange(1, 100)]
    [int]$Runs = 5
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
$problems = @(
    [pscustomobject]@{
        shape = 'repeated'
        path = (Resolve-Path '.artifacts/experiments/2026-07-15-009-formula-owner-memory-scaling/corpus/repeated-20000.p').Path
    },
    [pscustomobject]@{
        shape = 'unique-atom'
        path = (Resolve-Path '.artifacts/experiments/2026-07-16-010-unique-symbol-parser-scaling/corpus/unique-atom-20000.p').Path
    }
)

$outputPath = [System.IO.Path]::GetFullPath($OutputCsv)
[System.IO.Directory]::CreateDirectory([System.IO.Path]::GetDirectoryName($outputPath)) |
    Out-Null

function Measure-Run {
    param(
        [Parameter(Mandatory = $true)]
        [pscustomobject]$Implementation,

        [Parameter(Mandatory = $true)]
        [pscustomobject]$Problem,

        [Parameter(Mandatory = $true)]
        [int]$Run
    )

    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $Implementation.binary
    $startInfo.Arguments = '--cnf --silent --output-file=NUL "' + $Problem.path + '"'
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
        Start-Sleep -Milliseconds 2
    }
    $process.WaitForExit()
    $stopwatch.Stop()
    $exitCode = $process.ExitCode
    $stdout = $stdoutTask.Result
    $stderr = $stderrTask.Result
    $process.Dispose()

    if ($exitCode -ne 0) {
        throw "$($Implementation.label)/$($Problem.shape) run $Run exited $exitCode`: $stderr"
    }
    [pscustomobject]@{
        implementation = $Implementation.label
        shape = $Problem.shape
        run = $Run
        exit_code = $exitCode
        wall_seconds = [Math]::Round($stopwatch.Elapsed.TotalSeconds, 6)
        sampled_peak_kib = [Math]::Round($sampledPeak / 1KB)
        stdout_bytes = [System.Text.Encoding]::UTF8.GetByteCount($stdout)
        stderr_bytes = [System.Text.Encoding]::UTF8.GetByteCount($stderr)
    }
}

$rows = @()
foreach ($problem in $problems) {
    foreach ($implementation in $implementations) {
        $null = Measure-Run -Implementation $implementation -Problem $problem -Run 0
    }
    for ($run = 1; $run -le $Runs; $run++) {
        $orderedImplementations = if ($run % 2 -eq 0) {
            @($implementations[1], $implementations[0])
        }
        else {
            $implementations
        }
        foreach ($implementation in $orderedImplementations) {
            $rows += Measure-Run -Implementation $implementation -Problem $problem -Run $run
        }
    }
}

$rows | Export-Csv -NoTypeInformation $outputPath
$rows | Format-Table -AutoSize
$rows | Group-Object implementation, shape | ForEach-Object {
    $wall = @($_.Group.wall_seconds | Sort-Object)
    $rss = @($_.Group.sampled_peak_kib | Sort-Object)
    [pscustomobject]@{
        implementation = $_.Group[0].implementation
        shape = $_.Group[0].shape
        median_wall_seconds = $wall[[Math]::Floor($wall.Count / 2)]
        median_sampled_peak_kib = $rss[[Math]::Floor($rss.Count / 2)]
    }
} | Format-Table -AutoSize
