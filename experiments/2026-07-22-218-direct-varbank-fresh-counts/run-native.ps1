param(
    [Parameter(Mandatory = $true)]
    [string]$ParentExe,

    [Parameter(Mandatory = $true)]
    [string]$CandidateExe,

    [int]$Pairs = 64,

    [Parameter(Mandatory = $true)]
    [string]$OutputCsv
)

$ErrorActionPreference = "Stop"
$problem = Join-Path $PSScriptRoot "..\..\eprover\EXAMPLE_PROBLEMS\SMOKETEST\LUSK6.lop"
$arguments = @(
    $problem,
    "--auto",
    "--silent",
    "--cpu-limit=600",
    "--memory-limit=2048",
    "--detsort-rw",
    "--detsort-new"
)

function Invoke-MeasuredRun {
    param(
        [string]$Executable,
        [string]$Variant,
        [int]$Pair,
        [int]$Position
    )

    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = (Resolve-Path $Executable).Path
    $startInfo.Arguments = ($arguments | ForEach-Object { '"' + $_ + '"' }) -join " "
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true

    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
    [void]$process.Start()
    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    $process.WaitForExit()
    $stopwatch.Stop()
    $stdoutTask.GetAwaiter().GetResult() | Out-Null
    $stderrTask.GetAwaiter().GetResult() | Out-Null

    [pscustomobject]@{
        pair = $Pair
        position = $Position
        variant = $Variant
        wall_seconds = $stopwatch.Elapsed.TotalSeconds
        cpu_seconds = $process.TotalProcessorTime.TotalSeconds
        exit_code = $process.ExitCode
    }
}

$results = [System.Collections.Generic.List[object]]::new()
for ($pair = 1; $pair -le $Pairs; $pair++) {
    if (($pair % 2) -eq 1) {
        $order = @(@("parent", $ParentExe), @("candidate", $CandidateExe))
    }
    else {
        $order = @(@("candidate", $CandidateExe), @("parent", $ParentExe))
    }

    for ($position = 0; $position -lt $order.Count; $position++) {
        $variant = $order[$position][0]
        $executable = $order[$position][1]
        $result = Invoke-MeasuredRun $executable $variant $pair ($position + 1)
        $results.Add($result)
        Write-Host ("pair={0} position={1} variant={2} wall={3:F6} cpu={4:F6} exit={5}" -f `
            $result.pair, $result.position, $result.variant, $result.wall_seconds, `
            $result.cpu_seconds, $result.exit_code)
    }
}

$results | Export-Csv -NoTypeInformation -Path $OutputCsv
if (($results | Where-Object { $_.exit_code -ne 0 }).Count -ne 0) {
    throw "At least one benchmark run failed"
}
