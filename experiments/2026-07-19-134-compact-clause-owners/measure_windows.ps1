[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Binary,

    [Parameter(Mandatory = $true)]
    [string]$Problem,

    [Parameter(Mandatory = $true)]
    [string]$OutputCsv,

    [string]$Label = 'candidate',

    [ValidateRange(1, 20)]
    [int]$Runs = 1,

    [ValidateRange(1, 600)]
    [int]$CpuLimit = 60,

    [switch]$Backtrace
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$binaryPath = (Resolve-Path $Binary).Path
$problemPath = (Resolve-Path $Problem).Path
$outputPath = [System.IO.Path]::GetFullPath($OutputCsv)
[System.IO.Directory]::CreateDirectory([System.IO.Path]::GetDirectoryName($outputPath)) |
    Out-Null

$rows = for ($run = 1; $run -le $Runs; $run++) {
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
    $startInfo.FileName = $binaryPath
    $startInfo.Arguments = ($arguments | ForEach-Object { '"' + $_ + '"' }) -join ' '
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    if ($Backtrace) {
        $env:RUST_BACKTRACE = 'full'
    }

    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
    if (-not $process.Start()) {
        throw "Could not start $binaryPath"
    }
    $moduleBase = $null
    try {
        $module = $process.MainModule
        if ($null -ne $module) {
            $moduleBase = '0x{0:x}' -f $module.BaseAddress.ToInt64()
        }
    }
    catch [System.InvalidOperationException] {
        # Very short-lived processes can exit before module metadata is read.
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
    $stdout = $stdoutTask.Result
    $stderr = $stderrTask.Result
    $statusMatch = [regex]::Matches($stdout, '(?m)^%+ SZS status (\S+)') |
        Select-Object -Last 1

    [pscustomobject]@{
        implementation = $Label
        run = $run
        module_base = $moduleBase
        exit_code = $process.ExitCode
        status = if ($statusMatch) { $statusMatch.Groups[1].Value } else { $null }
        wall_seconds = [Math]::Round($stopwatch.Elapsed.TotalSeconds, 6)
        cpu_seconds = [Math]::Round($process.TotalProcessorTime.TotalSeconds, 6)
        sampled_peak_kib = [Math]::Round($sampledPeak / 1KB)
        stdout_bytes = [System.Text.Encoding]::UTF8.GetByteCount($stdout)
        stderr = $stderr.Trim()
    }
    $process.Dispose()
}

$rows | Export-Csv -NoTypeInformation $outputPath
$rows | Format-Table -AutoSize
