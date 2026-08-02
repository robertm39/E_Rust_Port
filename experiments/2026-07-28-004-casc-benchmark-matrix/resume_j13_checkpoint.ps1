[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$CheckpointArchive,

    [Parameter(Mandatory = $true)]
    [ValidatePattern("^[0-9a-fA-F]{64}$")]
    [string]$CheckpointSha256,

    [Parameter(Mandatory = $true)]
    [ValidateRange(1, 2699)]
    [int]$ExpectedInitialResults,

    [ValidateRange(60, 14400)]
    [int]$MaxSessionWallSeconds = 14400,

    [ValidateRange(10, 60)]
    [int]$PollSeconds = 60,

    [string]$NotBeforeUtc,

    [switch]$Execute
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$expectedContract = (
    "9f29cac72abe79a5a0b31f5135412243f95ec344b5152eadc3d372ac49e8c676"
)
$expectedContractFile = (
    "4a66c48124cdfb89da5c17ac87229e599ae2dffd92976c0ff89804d362bc6075"
)
$sourceSnapshot = (
    "88bd4fb2010ede33d8f2dd4e6f60957751a0b3183375c57516a6fe06810efa10"
)
$corpusSha256 = (
    "ab89485b9d00b00e1098a3ab3184e47d10e59978320dca1f541480320e2a7fdc"
)
$umlautSha256 = (
    "8c093b91e7e0de5f37d2f8066199f9b57aaea3a1041f9fa9eb21d116ae1decda"
)
$vampireSha256 = (
    "3fd88f402d2b74ddf6bf96d49a2bf3c9383710b19d1c9c2c5ecb740265a5c665"
)
$runRoot = "/opt/e-rust-port/casc-runs/casc-j13-2026-089e06c8-v2"
$serviceRuntimeSeconds = $MaxSessionWallSeconds + 300

$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..\..")).Path
$runnerPath = Join-Path $repoRoot "linode-runner.ps1"
$corpusPath = Join-Path (
    $repoRoot
) ".artifacts\casc-benchmark\casc_2026_corpus.tar.gz"
$umlautPath = Join-Path (
    $repoRoot
) ".artifacts\casc-benchmark\umlaut-4e87dac3"
$vampirePath = Join-Path (
    $repoRoot
) (
    ".artifacts\vampire\3677326861181f990ce3ef461e90471ba9749225\" +
    "linode-ubuntu24.04-x86_64\vampire"
)
$checkpointPath = if ([IO.Path]::IsPathRooted($CheckpointArchive)) {
    [IO.Path]::GetFullPath($CheckpointArchive)
}
else {
    [IO.Path]::GetFullPath((Join-Path $repoRoot $CheckpointArchive))
}
$checkpointName = [IO.Path]::GetFileName($checkpointPath)
if (
    $checkpointName -notmatch
    "^j13-checkpoint-[0-9]{6}-[0-9]{6}-[0-9a-f]{4}\.tar\.gz$"
) {
    throw "Checkpoint filename does not identify a guarded J13 run: $checkpointName"
}
$checkpointRootName = $checkpointName.Substring(0, $checkpointName.Length - 7)
$CheckpointSha256 = $CheckpointSha256.ToLowerInvariant()

function Get-VerifiedSha256 {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,

        [Parameter(Mandatory = $true)]
        [string]$Expected
    )

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Required input is missing: $Path"
    }
    $actual = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $Expected) {
        throw "SHA-256 mismatch for ${Path}: $actual != $Expected"
    }
    return $actual
}

if (-not (Test-Path -LiteralPath $runnerPath -PathType Leaf)) {
    throw "Linode runner is missing: $runnerPath"
}
$null = Get-VerifiedSha256 -Path $checkpointPath -Expected $CheckpointSha256
$null = Get-VerifiedSha256 -Path $corpusPath -Expected $corpusSha256
$null = Get-VerifiedSha256 -Path $umlautPath -Expected $umlautSha256
$null = Get-VerifiedSha256 -Path $vampirePath -Expected $vampireSha256

$notBefore = $null
if ($PSBoundParameters.ContainsKey("NotBeforeUtc")) {
    try {
        $notBefore = [DateTimeOffset]::Parse(
            $NotBeforeUtc,
            [Globalization.CultureInfo]::InvariantCulture,
            (
                [Globalization.DateTimeStyles]::AssumeUniversal -bor
                [Globalization.DateTimeStyles]::AdjustToUniversal
            )
        )
    }
    catch {
        throw "NotBeforeUtc must be an ISO-8601 timestamp: $NotBeforeUtc"
    }
    if ($notBefore -gt [DateTimeOffset]::UtcNow.AddHours(24)) {
        throw "NotBeforeUtc must be no more than 24 hours in the future"
    }
}

$plan = [ordered]@{
    schema_version = 1
    kind = "umlaut-casc-j13-guarded-resume-plan"
    checkpoint = [ordered]@{
        path = $checkpointPath
        sha256 = $CheckpointSha256
        expected_results = $ExpectedInitialResults
    }
    immutable_inputs = [ordered]@{
        contract_id = $expectedContract
        contract_file_sha256 = $expectedContractFile
        source_snapshot_sha256 = $sourceSnapshot
        corpus_sha256 = $corpusSha256
        umlaut_sha256 = $umlautSha256
        vampire_sha256 = $vampireSha256
    }
    limits = [ordered]@{
        max_session_wall_seconds = $MaxSessionWallSeconds
        service_runtime_seconds = $serviceRuntimeSeconds
        poll_seconds = $PollSeconds
        not_before_utc = if ($null -eq $notBefore) {
            $null
        }
        else {
            $notBefore.ToString("O")
        }
    }
}
if (-not $Execute) {
    $plan | ConvertTo-Json -Depth 5
    Write-Host "Inputs verified. Pass -Execute to provision and resume."
    exit 0
}

if ($null -ne $notBefore -and [DateTimeOffset]::UtcNow -lt $notBefore) {
    Write-Host "Inputs verified. Waiting until $($notBefore.ToString('O'))."
    while ([DateTimeOffset]::UtcNow -lt $notBefore) {
        $remainingSeconds = [Math]::Ceiling(
            ($notBefore - [DateTimeOffset]::UtcNow).TotalSeconds
        )
        $sleepSeconds = [Math]::Min(60, [Math]::Max(1, $remainingSeconds))
        Start-Sleep -Seconds $sleepSeconds
    }
    Write-Host "Not-before boundary reached; rerunning mutable preflights."
}

$branch = (& git -C $repoRoot branch --show-current).Trim()
if ($LASTEXITCODE -ne 0 -or $branch -ne "main") {
    throw "Guarded J13 resumes require the clean main branch; found '$branch'"
}
$worktreeStatus = & git -C $repoRoot status --porcelain=v1
if ($LASTEXITCODE -ne 0) {
    throw "Could not inspect the git worktree"
}
if ($null -ne $worktreeStatus -and @($worktreeStatus).Count -gt 0) {
    throw "Guarded J13 resumes require a clean worktree"
}

$artifactRoot = Join-Path $repoRoot ".artifacts\casc-benchmark"
[IO.Directory]::CreateDirectory($artifactRoot) | Out-Null
$controllerId = (
    [DateTimeOffset]::UtcNow.ToString("yyyyMMddTHHmmssZ") + "-$PID"
)
$logPath = Join-Path $artifactRoot "j13-resume-controller-$controllerId.log"
if (Test-Path -LiteralPath $logPath) {
    throw "Refusing to overwrite controller log: $logPath"
}

function Write-ResumeLog {
    param([Parameter(Mandatory = $true)][string]$Message)

    $timestamp = [DateTimeOffset]::UtcNow.ToString("O")
    $line = "$timestamp $Message"
    [IO.File]::AppendAllText(
        $logPath,
        "$line`r`n",
        [Text.UTF8Encoding]::new($false)
    )
    Write-Host $line
}

function Invoke-Runner {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$CommandArguments
    )

    $output = & $runnerPath @CommandArguments 2>&1
    $exitCode = $LASTEXITCODE
    if ($null -ne $output) {
        foreach ($line in $output) {
            Write-ResumeLog ([string]$line)
        }
    }
    if ($exitCode -ne 0) {
        throw (
            "linode-runner exited with code $exitCode for: " +
            ($CommandArguments -join " ")
        )
    }
    return ($output -join "`n")
}

function Get-RunnerStatus {
    $raw = Invoke-Runner @("status")
    try {
        return $raw | ConvertFrom-Json
    }
    catch {
        throw "Could not parse linode-runner status JSON"
    }
}

function ConvertFrom-SystemdProperties {
    param([Parameter(Mandatory = $true)][string]$Text)

    $properties = @{}
    foreach ($line in $Text -split "`r?`n") {
        if ($line -match "^(?<name>[A-Za-z0-9]+)=(?<value>.*)$") {
            $properties[$Matches.name] = $Matches.value
        }
    }
    return $properties
}

function Get-ServiceProperties {
    param([Parameter(Mandatory = $true)][string]$Unit)

    $fields = (
        "ActiveState,SubState,MainPID,InvocationID,NRestarts,Result," +
        "ExecMainStatus"
    )
    $raw = Invoke-Runner @(
        "exec",
        "--",
        "systemctl show $Unit --property=$fields"
    )
    return ConvertFrom-SystemdProperties -Text $raw
}

$runnerAcquired = $false
$launchAttempted = $false
$captureSucceeded = $false
$localArchive = $null
$remoteArchive = $null
$unit = $null
$expectedInvocation = $null
$expectedMainPid = $null

try {
    Write-ResumeLog "controller_started"
    $initialStatus = Get-RunnerStatus
    if ($null -ne $initialStatus.active) {
        throw "An active managed runner already exists"
    }
    if (@($initialStatus.parked).Count -ne 0) {
        throw "Parked managed runners must be resolved before a J13 resume"
    }

    Invoke-Runner @("check", "--high-memory") | Out-Null
    Invoke-Runner @("up", "--high-memory") | Out-Null
    $runnerAcquired = $true

    $status = Get-RunnerStatus
    $active = $status.active
    if ($null -eq $active) {
        throw "High-memory runner acquisition did not create active state"
    }
    $runId = [string]$active.run_id
    $runnerLabel = [string]$active.label
    $linodeId = [int64]$active.linode_id
    if ($runId -notmatch "^[0-9]{6}-[0-9]{6}-[0-9a-f]{4}$") {
        throw "Invalid managed runner ID: $runId"
    }
    if ($runnerLabel -ne "e-rust-codex-$runId") {
        throw "Managed runner label does not match its run ID"
    }
    if (
        [string]$active.type -ne "g7-highmem-8" -or
        [string]$active.phase -ne "ready" -or
        [string]$active.live_linode_status -ne "running" -or
        $linodeId -le 0
    ) {
        throw "Managed runner is not a ready canonical high-memory host"
    }

    $sessionId = "j13-resume-$runId"
    $unit = "casc-j13-v2-resume-$runId.service"
    $remoteCheckpointInput = "/root/input-j13-checkpoint.tar.gz"
    $remoteCorpus = "/root/casc_2026_corpus.tar.gz"
    $remoteUmlaut = "/root/umlaut-4e87dac3"
    $remoteVampire = "/root/vampire-5.0.1"
    $remoteRestoreRoot = "/root/j13-restore"
    $remoteCheckpointRoot = "/root/j13-checkpoint-$runId"
    $remoteArchive = "$remoteCheckpointRoot.tar.gz"
    $localArchive = Join-Path (
        $artifactRoot
    ) "j13-checkpoint-$runId.tar.gz"
    if (Test-Path -LiteralPath $localArchive) {
        throw "Refusing to overwrite checkpoint output: $localArchive"
    }

    Write-ResumeLog (
        "runner_ready run_id=$runId label=$runnerLabel linode_id=$linodeId"
    )
    Invoke-Runner @("sync") | Out-Null
    Invoke-Runner @("upload", $corpusPath, $remoteCorpus) | Out-Null
    Invoke-Runner @("upload", $checkpointPath, $remoteCheckpointInput) |
        Out-Null
    Invoke-Runner @("upload", $umlautPath, $remoteUmlaut) | Out-Null
    Invoke-Runner @("upload", $vampirePath, $remoteVampire) | Out-Null

    $preflightCommand = @"
set -Eeuo pipefail
printf '%s  %s\n' '$corpusSha256' '$remoteCorpus' '$CheckpointSha256' '$remoteCheckpointInput' '$umlautSha256' '$remoteUmlaut' '$vampireSha256' '$remoteVampire' | sha256sum -c -
chmod 0755 '$remoteUmlaut' '$remoteVampire'
test ! -e '$remoteRestoreRoot'
test ! -e '/opt/e-rust-port/casc-runs'
install -d -m 0700 '$remoteRestoreRoot'
cd /opt/e-rust-port/source
python3 tools/casc_benchmark/corpus_archive.py extract --archive '$remoteCorpus' --destination /opt/e-rust-port/source --manifest benchmarks/casc_2026_manifest.jsonl
tar -xzf '$remoteCheckpointInput' -C '$remoteRestoreRoot'
cd '$remoteRestoreRoot/$checkpointRootName'
sha256sum -c SHA256SUMS
tar -xzf casc-runs.tar.gz -C /opt/e-rust-port
printf '%s  %s\n' '$expectedContractFile' '$runRoot/contract.json' | sha256sum -c -
cd /opt/e-rust-port/source
python3 tools/casc_benchmark/report.py --manifest benchmarks/casc_2026_manifest.jsonl --run-root '$runRoot' --allow-partial
grep -Fq '"contract_id":"$expectedContract"' '$runRoot/summary.json'
grep -Fq '"completed_results":$ExpectedInitialResults' '$runRoot/summary.json'
test "`$(find '$runRoot/results' -type f -name '*.json' | wc -l)" -eq '$ExpectedInitialResults'
python3 tools/casc_benchmark/batch.py --manifest benchmarks/casc_2026_manifest.jsonl --problem-root /opt/e-rust-port/source --output-root '$runRoot' --umlaut-binary '$remoteUmlaut' --vampire-binary '$remoteVampire' --solvers both --cores 8 --memory-limit-mib 131072 --pids-limit 512 --vampire-seed 1 --wall-grace-seconds 0.25 --terminate-grace-seconds 1 --session-id 'j13-preflight-$runId' --source-snapshot-sha256 '$sourceSnapshot' --runner-label '$runnerLabel' --runner-run-id '$runId' --linode-id '$linodeId' --verify-only
"@
    Invoke-Runner @("exec", "--", $preflightCommand) | Out-Null
    Write-ResumeLog "preflight_passed initial_results=$ExpectedInitialResults"

    $batchCommand = @(
        "/usr/bin/python3",
        "/opt/e-rust-port/source/tools/casc_benchmark/batch.py",
        "--manifest",
        "/opt/e-rust-port/source/benchmarks/casc_2026_manifest.jsonl",
        "--problem-root",
        "/opt/e-rust-port/source",
        "--output-root",
        $runRoot,
        "--umlaut-binary",
        $remoteUmlaut,
        "--vampire-binary",
        $remoteVampire,
        "--solvers",
        "both",
        "--cores",
        "8",
        "--memory-limit-mib",
        "131072",
        "--pids-limit",
        "512",
        "--vampire-seed",
        "1",
        "--wall-grace-seconds",
        "0.25",
        "--terminate-grace-seconds",
        "1",
        "--max-session-wall-seconds",
        [string]$MaxSessionWallSeconds,
        "--session-id",
        $sessionId,
        "--source-snapshot-sha256",
        $sourceSnapshot,
        "--runner-label=$runnerLabel",
        "--runner-run-id=$runId",
        "--linode-id=$linodeId"
    ) -join " "
    $launchCommand = (
        "systemd-run --unit=$unit --service-type=exec " +
        "--property=Restart=no " +
        "--property=RuntimeMaxSec=$serviceRuntimeSeconds " +
        "--property=TimeoutStopSec=30s " +
        "--working-directory=/opt/e-rust-port/source " +
        $batchCommand
    )
    $launchAttempted = $true
    Invoke-Runner @("exec", "--", $launchCommand) | Out-Null

    $identityDeadline = [DateTimeOffset]::UtcNow.AddSeconds(30)
    while ($true) {
        $properties = Get-ServiceProperties -Unit $unit
        $mainPid = 0
        if ($properties.ContainsKey("MainPID")) {
            $mainPid = [int64]$properties.MainPID
        }
        if (
            $properties.ActiveState -eq "active" -and
            $mainPid -gt 0 -and
            [string]$properties.InvocationID -match "^[0-9a-f]{32}$"
        ) {
            break
        }
        if ([DateTimeOffset]::UtcNow -ge $identityDeadline) {
            throw "Could not pin the transient service identity"
        }
        Start-Sleep -Seconds 5
    }
    if ([string]$properties.NRestarts -ne "0") {
        throw "Transient service restarted before its identity was pinned"
    }
    $expectedMainPid = [string]$properties.MainPID
    $expectedInvocation = [string]$properties.InvocationID
    Write-ResumeLog (
        "service_started unit=$unit main_pid=$expectedMainPid " +
        "invocation_id=$expectedInvocation"
    )

    $monitorDeadline = [DateTimeOffset]::UtcNow.AddSeconds(
        $serviceRuntimeSeconds + 300
    )
    while ($true) {
        $properties = Get-ServiceProperties -Unit $unit
        if ([string]$properties.InvocationID -ne $expectedInvocation) {
            throw "Transient service invocation changed"
        }
        if ([string]$properties.NRestarts -ne "0") {
            throw "Transient service restarted"
        }
        if ([string]$properties.ActiveState -ne "active") {
            break
        }
        if ([string]$properties.MainPID -ne $expectedMainPid) {
            throw "Transient service main PID changed while active"
        }
        $resultCount = Invoke-Runner @(
            "exec",
            "--",
            "find '$runRoot/results' -type f -name '*.json' | wc -l"
        )
        Write-ResumeLog "service_active result_count=$($resultCount.Trim())"
        if ([DateTimeOffset]::UtcNow -ge $monitorDeadline) {
            throw "Transient service exceeded its guarded monitoring deadline"
        }
        Start-Sleep -Seconds $PollSeconds
    }

    $serviceSucceeded = (
        [string]$properties.ActiveState -eq "inactive" -and
        [string]$properties.Result -eq "success" -and
        [string]$properties.ExecMainStatus -eq "0" -and
        [string]$properties.InvocationID -eq $expectedInvocation -and
        [string]$properties.NRestarts -eq "0"
    )
    Write-ResumeLog (
        "service_finished active_state=$($properties.ActiveState) " +
        "result=$($properties.Result) exec_status=$($properties.ExecMainStatus)"
    )

    $captureCommand = @"
set -Eeuo pipefail
if pgrep -f '^/root/umlaut-4e87dac3( |$)' || pgrep -f '^/root/vampire-5.0.1( |$)' || pgrep -f '^/usr/bin/python3 /opt/e-rust-port/source/tools/casc_benchmark/batch.py( |$)'; then echo 'solver or batch process remains' >&2; exit 1; fi
cd /opt/e-rust-port/source
python3 tools/casc_benchmark/report.py --manifest benchmarks/casc_2026_manifest.jsonl --run-root '$runRoot' --allow-partial
grep -Fq '"contract_id":"$expectedContract"' '$runRoot/summary.json'
test ! -e '$remoteCheckpointRoot'
install -d -m 0700 '$remoteCheckpointRoot'
tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner -czf '$remoteCheckpointRoot/casc-runs.tar.gz' -C /opt/e-rust-port casc-runs
journalctl -u '$unit' --no-pager -o short-iso > '$remoteCheckpointRoot/service.log'
systemctl show '$unit' --all > '$remoteCheckpointRoot/service-properties.txt'
cp /opt/e-rust-port/package-maintenance-quiescence.json '$remoteCheckpointRoot/package-maintenance-quiescence.json'
ps -eo pid,ppid,etimes,stat,comm,args > '$remoteCheckpointRoot/processes.txt'
find '$runRoot/results' -type f -name '*.json' -print | sort > '$remoteCheckpointRoot/result-files.txt'
wc -l '$remoteCheckpointRoot/result-files.txt' > '$remoteCheckpointRoot/result-count.txt'
find /sys/fs/cgroup -maxdepth 1 -type d -name 'umlaut-casc-*' -print > '$remoteCheckpointRoot/cgroup-residue.txt'
systemctl list-units --all --plain --no-legend 'umlaut-casc-*' > '$remoteCheckpointRoot/solver-units.txt'
test ! -s '$remoteCheckpointRoot/cgroup-residue.txt'
test ! -s '$remoteCheckpointRoot/solver-units.txt'
sha256sum '$remoteCorpus' '$remoteCheckpointInput' '$remoteUmlaut' '$remoteVampire' > '$remoteCheckpointRoot/input-sha256s.txt'
printf '%s\n' 'controller_id=$controllerId' 'parent_checkpoint=$checkpointName' 'parent_checkpoint_sha256=$CheckpointSha256' 'initial_results=$ExpectedInitialResults' 'max_session_wall_seconds=$MaxSessionWallSeconds' 'runner_id=$runId' 'runner_label=$runnerLabel' 'linode_id=$linodeId' 'unit=$unit' 'initial_main_pid=$expectedMainPid' 'initial_invocation_id=$expectedInvocation' > '$remoteCheckpointRoot/resume-metadata.txt'
cd '$remoteCheckpointRoot'
sha256sum -- * > SHA256SUMS
cd /root
tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner -czf '$remoteArchive' 'j13-checkpoint-$runId'
"@
    Invoke-Runner @("exec", "--", $captureCommand) | Out-Null

    $remoteHashLine = Invoke-Runner @("exec", "--", "sha256sum '$remoteArchive'")
    $remoteHash = ($remoteHashLine -split "\s+", 2)[0].ToLowerInvariant()
    if ($remoteHash -notmatch "^[0-9a-f]{64}$") {
        throw "Could not parse the remote checkpoint SHA-256"
    }
    Invoke-Runner @("download", $remoteArchive, $localArchive) | Out-Null
    $localHash = (
        Get-FileHash -LiteralPath $localArchive -Algorithm SHA256
    ).Hash.ToLowerInvariant()
    if ($localHash -ne $remoteHash) {
        throw "Downloaded checkpoint SHA-256 does not match the remote archive"
    }

    $finalCountText = Invoke-Runner @(
        "exec",
        "--",
        "find '$runRoot/results' -type f -name '*.json' | wc -l"
    )
    $finalCount = 0
    if (-not [int]::TryParse($finalCountText.Trim(), [ref]$finalCount)) {
        throw "Could not parse the final result count"
    }
    if ($finalCount -le $ExpectedInitialResults -or $finalCount -gt 2700) {
        throw (
            "Final result count is outside the guarded range: " +
            "$finalCount after $ExpectedInitialResults"
        )
    }

    $captureSucceeded = $true
    Write-ResumeLog (
        "checkpoint_verified path=$localArchive sha256=$localHash " +
        "result_count=$finalCount"
    )
    if (-not $serviceSucceeded) {
        throw "The service failed, but its verified recovery checkpoint was captured"
    }
}
catch {
    if (Test-Path -LiteralPath $logPath) {
        Write-ResumeLog "controller_failed error=$($_.Exception.Message)"
    }
    throw
}
finally {
    if ($runnerAcquired -and $captureSucceeded) {
        try {
            Invoke-Runner @("down", "--all") | Out-Null
            Write-ResumeLog "managed_resources_deleted"
        }
        catch {
            Write-ResumeLog "resource_delete_failed error=$($_.Exception.Message)"
            throw
        }
    }
    elseif ($runnerAcquired -and -not $launchAttempted) {
        try {
            Invoke-Runner @("down", "--all") | Out-Null
            Write-ResumeLog "prelaunch_resources_deleted"
        }
        catch {
            Write-ResumeLog (
                "prelaunch_resource_delete_failed error=$($_.Exception.Message)"
            )
            throw
        }
    }
    elseif ($runnerAcquired) {
        Write-ResumeLog "runner_retained_for_recovery"
    }
}

[ordered]@{
    checkpoint_path = $localArchive
    checkpoint_sha256 = (
        Get-FileHash -LiteralPath $localArchive -Algorithm SHA256
    ).Hash.ToLowerInvariant()
    controller_log = $logPath
} | ConvertTo-Json
