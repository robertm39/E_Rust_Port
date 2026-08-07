[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$CheckpointArchive,

    [Parameter(Mandatory = $true)]
    [ValidatePattern("^[0-9a-fA-F]{64}$")]
    [string]$CheckpointSha256,

    [Parameter(Mandatory = $true)]
    [ValidateRange(0, 5801)]
    [int]$ExpectedInitialResults,

    [ValidateSet("j13", "casc2025")]
    [string]$Release = "j13",

    [ValidateRange(60, 14400)]
    [int]$MaxSessionWallSeconds = 14400,

    [ValidateRange(10, 60)]
    [int]$PollSeconds = 60,

    [string]$NotBeforeUtc,

    [ValidatePattern("^[0-9]{6}-[0-9]{6}-[0-9a-f]{4}$")]
    [string]$ExistingRunnerRunId,

    [switch]$AdoptExistingService,

    [switch]$AdoptCompletedService,

    [switch]$AdoptFailedService,

    [ValidateSet("exit-code", "signal", "timeout", "watchdog", "oom-kill")]
    [string]$ExpectedServiceResult,

    [ValidateRange(1, 255)]
    [int]$ExpectedServiceExecStatus,

    [switch]$ClearFailedCaptureResidue,

    [ValidateRange(1, 2147483647)]
    [int64]$ExpectedServiceMainPid,

    [ValidatePattern("^[0-9a-f]{32}$")]
    [string]$ExpectedServiceInvocationId,

    [switch]$Execute
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$sourceSnapshot = (
    "88bd4fb2010ede33d8f2dd4e6f60957751a0b3183375c57516a6fe06810efa10"
)
$umlautSha256 = (
    "8c093b91e7e0de5f37d2f8066199f9b57aaea3a1041f9fa9eb21d116ae1decda"
)
$vampireSha256 = (
    "3fd88f402d2b74ddf6bf96d49a2bf3c9383710b19d1c9c2c5ecb740265a5c665"
)
$serviceRuntimeSeconds = $MaxSessionWallSeconds + 300
$terminalCaptureAllowanceSeconds = 600
$runnerProbeTimeoutSeconds = 90
$recoveryGraceSeconds = 900

$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..\..")).Path
$releaseConfig = switch ($Release) {
    "j13" {
        [ordered]@{
            contract = (
                "9f29cac72abe79a5a0b31f5135412243f95ec344b5152eadc3d372ac49e8c676"
            )
            contract_file = (
                "4a66c48124cdfb89da5c17ac87229e599ae2dffd92976c0ff89804d362bc6075"
            )
            corpus_file = "casc_2026_corpus.tar.gz"
            corpus_sha256 = (
                "ab89485b9d00b00e1098a3ab3184e47d10e59978320dca1f541480320e2a7fdc"
            )
            manifest_file = "casc_2026_manifest.jsonl"
            run_name = "casc-j13-2026-089e06c8-v2"
            checkpoint_prefix = "j13-checkpoint"
            service_prefix = "casc-j13-v2-resume"
            session_prefix = "j13-resume"
            expected_total_results = 2700
        }
        break
    }
    "casc2025" {
        [ordered]@{
            contract = (
                "e71fc642a15db4528fb915724493b7571798fe40848a4fe0085e62723918d1aa"
            )
            contract_file = (
                "f895aa07141b091060f3ee46d28f91abd6f484f3ad690630af08a7dbe34284c5"
            )
            corpus_file = "casc_2025_corpus.tar.gz"
            corpus_sha256 = (
                "efcebc55298d4c6770113c095e8cefdd77b9e8cbe3afa3078201f541893d1a7d"
            )
            manifest_file = "casc_2025_manifest.jsonl"
            run_name = "casc30-2025-089e06c8-v2"
            checkpoint_prefix = "casc2025-checkpoint"
            service_prefix = "casc2025-v2-resume"
            session_prefix = "casc2025-resume"
            expected_total_results = 5802
        }
        break
    }
}
$expectedContract = [string]$releaseConfig.contract
$expectedContractFile = [string]$releaseConfig.contract_file
$corpusSha256 = [string]$releaseConfig.corpus_sha256
$runName = [string]$releaseConfig.run_name
$runRoot = "/opt/e-rust-port/casc-runs/$runName"
$checkpointPrefix = [string]$releaseConfig.checkpoint_prefix
$servicePrefix = [string]$releaseConfig.service_prefix
$sessionPrefix = [string]$releaseConfig.session_prefix
$expectedTotalResults = [int]$releaseConfig.expected_total_results
$casc2025Contract = (
    "e71fc642a15db4528fb915724493b7571798fe40848a4fe0085e62723918d1aa"
)
$j13Contract = (
    "9f29cac72abe79a5a0b31f5135412243f95ec344b5152eadc3d372ac49e8c676"
)
$casc2025ManifestRelative = "benchmarks/casc_2025_manifest.jsonl"
$casc2025ManifestPath = Join-Path $repoRoot $casc2025ManifestRelative
$casc2025RunName = "casc30-2025-089e06c8-v2"
$casc2025RunRoot = (
    "/opt/e-rust-port/casc-runs/$casc2025RunName"
)
$j13ManifestRelative = "benchmarks/casc_2026_manifest.jsonl"
$j13ManifestPath = Join-Path $repoRoot $j13ManifestRelative
$j13RunName = "casc-j13-2026-089e06c8-v2"
$j13RunRoot = "/opt/e-rust-port/casc-runs/$j13RunName"
$combinedSummary = "/opt/e-rust-port/casc-runs/combined-summary.json"
$runnerPath = Join-Path $repoRoot "linode-runner.ps1"
$validatorPath = Join-Path $PSScriptRoot "validate_casc_checkpoint.py"
$plannerPath = Join-Path $PSScriptRoot "plan_next_casc_resume.py"
$manifestPath = Join-Path (
    $repoRoot
) "benchmarks\$($releaseConfig.manifest_file)"
$corpusPath = Join-Path (
    $repoRoot
) ".artifacts\casc-benchmark\$($releaseConfig.corpus_file)"
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
    "^(j13|casc2025)-checkpoint-[0-9]{6}-[0-9]{6}-[0-9a-f]{4}\.tar\.gz$"
) {
    throw "Checkpoint filename does not identify a guarded CASC run: $checkpointName"
}
$checkpointRootName = $checkpointName.Substring(0, $checkpointName.Length - 7)
$CheckpointSha256 = $CheckpointSha256.ToLowerInvariant()

$adoptingExistingService = $AdoptExistingService.IsPresent
$adoptingCompletedService = $AdoptCompletedService.IsPresent
$adoptingFailedService = $AdoptFailedService.IsPresent
$adoptionCount = @(
    $adoptingExistingService,
    $adoptingCompletedService,
    $adoptingFailedService
).Where({ $_ }).Count
$adoptingAnyService = $adoptionCount -eq 1
if ($adoptionCount -gt 1) {
    throw "Service adoption modes are mutually exclusive"
}
if ($adoptingAnyService) {
    if (
        -not $Execute -or
        -not $PSBoundParameters.ContainsKey("ExistingRunnerRunId") -or
        -not $PSBoundParameters.ContainsKey("ExpectedServiceMainPid") -or
        -not $PSBoundParameters.ContainsKey("ExpectedServiceInvocationId") -or
        $PSBoundParameters.ContainsKey("NotBeforeUtc")
    ) {
        throw (
            "Service adoption requires Execute, ExistingRunnerRunId, " +
            "ExpectedServiceMainPid, and ExpectedServiceInvocationId, " +
            "and forbids NotBeforeUtc"
        )
    }
}
if ($adoptingFailedService) {
    if (
        -not $PSBoundParameters.ContainsKey("ExpectedServiceResult") -or
        -not $PSBoundParameters.ContainsKey("ExpectedServiceExecStatus")
    ) {
        throw (
            "Failed-service adoption requires ExpectedServiceResult and " +
            "ExpectedServiceExecStatus"
        )
    }
}
elseif (
    $PSBoundParameters.ContainsKey("ExpectedServiceResult") -or
    $PSBoundParameters.ContainsKey("ExpectedServiceExecStatus") -or
    $ClearFailedCaptureResidue
) {
    throw (
        "Failed-service outcome and residue cleanup require " +
        "AdoptFailedService"
    )
}
elseif (
    -not $adoptingAnyService -and
    (
        $PSBoundParameters.ContainsKey("ExpectedServiceMainPid") -or
        $PSBoundParameters.ContainsKey("ExpectedServiceInvocationId")
    )
) {
    throw (
        "Expected service identity requires AdoptExistingService or " +
        "a terminal adoption mode"
    )
}

$failedTerminalJournalVerifier = @'
import base64
import json
import sys


unit, invocation, pid_text, command_base64, result, status_text = sys.argv[1:]
expected_pid = int(pid_text)
expected_status = int(status_text)
expected_command = base64.b64decode(command_base64).decode("utf-8")
records = []
for line_number, line in enumerate(sys.stdin, 1):
    if not line.strip():
        continue
    try:
        records.append(json.loads(line))
    except json.JSONDecodeError as error:
        raise SystemExit(
            f"invalid journal JSON at line {line_number}: {error}"
        )

service_records = [
    record
    for record in records
    if record.get("_SYSTEMD_UNIT") == unit or record.get("UNIT") == unit
]
if not service_records:
    raise SystemExit("journal contains no records for the expected unit")
observed_invocations = {
    value
    for record in service_records
    for value in (
        record.get("_SYSTEMD_INVOCATION_ID"),
        record.get("INVOCATION_ID"),
    )
    if value
}
if observed_invocations != {invocation}:
    raise SystemExit(
        "journal invocation set mismatch: "
        f"{sorted(observed_invocations)!r}"
    )
commands = {
    record.get("_CMDLINE")
    for record in service_records
    if record.get("_SYSTEMD_UNIT") == unit
    and record.get("_SYSTEMD_INVOCATION_ID") == invocation
    and str(record.get("_PID")) == str(expected_pid)
    and record.get("_CMDLINE")
}
if commands != {expected_command}:
    raise SystemExit("journal command does not match the guarded batch")

exit_records = [
    record
    for record in service_records
    if record.get("UNIT") == unit
    and record.get("INVOCATION_ID") == invocation
    and record.get("MESSAGE_ID") == "98e322203f7a4ed290d09fe03c09fe15"
    and record.get("COMMAND") == "ExecStart"
    and record.get("EXIT_CODE") == "exited"
    and str(record.get("EXIT_STATUS")) == str(expected_status)
]
if len(exit_records) != 1:
    raise SystemExit("journal must contain exactly one matching process exit")
failure_message = f"{unit}: Failed with result '{result}'."
failure_records = [
    record
    for record in service_records
    if record.get("UNIT") == unit
    and record.get("INVOCATION_ID") == invocation
    and record.get("MESSAGE_ID") == "d9b373ed55a64feb8242e02dbe79a49c"
    and record.get("UNIT_RESULT") == result
    and record.get("MESSAGE") == failure_message
]
if len(failure_records) != 1:
    raise SystemExit("journal must contain exactly one matching terminal failure")
exit_sequence = int(exit_records[0]["__SEQNUM"])
failure_sequence = int(failure_records[0]["__SEQNUM"])
if failure_sequence <= exit_sequence:
    raise SystemExit("terminal failure precedes the guarded process exit")
boot_ids = {
    record.get("_BOOT_ID") for record in service_records if record.get("_BOOT_ID")
}
if len(boot_ids) != 1:
    raise SystemExit("journal records span an ambiguous boot identity")
print(
    json.dumps(
        {
            "boot_id": next(iter(boot_ids)),
            "exec_status": expected_status,
            "failure_sequence": failure_sequence,
            "invocation_id": invocation,
            "main_pid": expected_pid,
            "result": result,
            "unit": unit,
        },
        sort_keys=True,
        separators=(",", ":"),
    )
)
'@

$terminalJournalVerifier = @'
import base64
import json
import re
import sys


unit, invocation, pid_text, command_base64, contract = sys.argv[1:]
expected_pid = int(pid_text)
expected_command = base64.b64decode(command_base64).decode("utf-8")
records = []
for line_number, line in enumerate(sys.stdin, 1):
    if not line.strip():
        continue
    try:
        records.append(json.loads(line))
    except json.JSONDecodeError as error:
        raise SystemExit(
            f"invalid journal JSON at line {line_number}: {error}"
        )

service_records = [
    record
    for record in records
    if record.get("_SYSTEMD_UNIT") == unit or record.get("UNIT") == unit
]
if not service_records:
    raise SystemExit("journal contains no records for the expected unit")

observed_invocations = {
    value
    for record in service_records
    for value in (
        record.get("_SYSTEMD_INVOCATION_ID"),
        record.get("INVOCATION_ID"),
    )
    if value
}
if observed_invocations != {invocation}:
    raise SystemExit(
        "journal invocation set mismatch: "
        f"{sorted(observed_invocations)!r}"
    )

commands = {
    record.get("_CMDLINE")
    for record in service_records
    if record.get("_SYSTEMD_UNIT") == unit
    and record.get("_SYSTEMD_INVOCATION_ID") == invocation
    and str(record.get("_PID")) == str(expected_pid)
    and record.get("_CMDLINE")
}
if commands != {expected_command}:
    raise SystemExit("journal command does not match the guarded batch")

success_message_id = "7ad2d189f7e94e70a38c781354912448"
success_message = f"{unit}: Deactivated successfully."
success_records = [
    record
    for record in service_records
    if record.get("UNIT") == unit
    and record.get("INVOCATION_ID") == invocation
    and record.get("MESSAGE_ID") == success_message_id
    and record.get("MESSAGE") == success_message
]
if len(success_records) != 1:
    raise SystemExit(
        "journal must contain exactly one successful terminal record"
    )

summary_pattern = re.compile(
    rf"^OK: contract {re.escape(contract)}; new=(\d+), resumed=(\d+)$"
)
summary_records = []
for record in service_records:
    if (
        record.get("_SYSTEMD_UNIT") != unit
        or record.get("_SYSTEMD_INVOCATION_ID") != invocation
        or str(record.get("_PID")) != str(expected_pid)
    ):
        continue
    match = summary_pattern.fullmatch(str(record.get("MESSAGE", "")))
    if match:
        summary_records.append((record, int(match.group(1)), int(match.group(2))))
if len(summary_records) != 1:
    raise SystemExit(
        "journal must contain exactly one guarded batch completion summary"
    )

success_sequence = int(success_records[0]["__SEQNUM"])
summary_record, new_results, resumed_results = summary_records[0]
if success_sequence <= int(summary_record["__SEQNUM"]):
    raise SystemExit("terminal success precedes the batch completion summary")

boot_ids = {
    record.get("_BOOT_ID") for record in service_records if record.get("_BOOT_ID")
}
if len(boot_ids) != 1:
    raise SystemExit("journal records span an ambiguous boot identity")

print(
    json.dumps(
        {
            "boot_id": next(iter(boot_ids)),
            "invocation_id": invocation,
            "main_pid": expected_pid,
            "new_results": new_results,
            "resumed_results": resumed_results,
            "reported_results": new_results + resumed_results,
            "success_sequence": success_sequence,
            "unit": unit,
        },
        sort_keys=True,
        separators=(",", ":"),
    )
)
'@

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
if (-not (Test-Path -LiteralPath $validatorPath -PathType Leaf)) {
    throw "Checkpoint validator is missing: $validatorPath"
}
if (-not (Test-Path -LiteralPath $plannerPath -PathType Leaf)) {
    throw "Checkpoint campaign planner is missing: $plannerPath"
}
if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
    throw "CASC manifest is missing: $manifestPath"
}
if ($ExpectedInitialResults -ge $expectedTotalResults) {
    throw (
        "ExpectedInitialResults must be smaller than the $Release total " +
        "$expectedTotalResults"
    )
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

function Invoke-CampaignInspector {
    $python = Join-Path $repoRoot ".venv\Scripts\python.exe"
    if (Test-Path -LiteralPath $python -PathType Leaf) {
        $pythonPrefix = @("-u")
    }
    else {
        $pythonCommand = Get-Command py -ErrorAction SilentlyContinue
        if ($null -ne $pythonCommand) {
            $python = $pythonCommand.Source
            $pythonPrefix = @("-3", "-u")
        }
        else {
            $pythonCommand = Get-Command python -ErrorAction Stop
            $python = $pythonCommand.Source
            $pythonPrefix = @("-u")
        }
    }
    $output = & $python @pythonPrefix $plannerPath `
        --checkpoint $checkpointPath `
        --checkpoint-sha256 $CheckpointSha256 `
        --max-session-wall-seconds $MaxSessionWallSeconds `
        --inspect-only 2>&1
    $exitCode = $LASTEXITCODE
    if ($exitCode -ne 0) {
        throw (
            "Checkpoint campaign inspection failed: " +
            (($output | ForEach-Object { [string]$_ }) -join "`n")
        )
    }
    try {
        $state = (($output | ForEach-Object { [string]$_ }) -join "`n") |
            ConvertFrom-Json
    }
    catch {
        throw "Checkpoint campaign inspector did not emit valid JSON"
    }
    if (
        [int]$state.schema_version -ne 1 -or
        [string]$state.kind -ne "umlaut-casc-checkpoint-state" -or
        [string]$state.status -ne "resume_candidate" -or
        [string]$state.release -ne $Release -or
        [string]$state.checkpoint.path -ne $checkpointPath -or
        [string]$state.checkpoint.sha256 -ne $CheckpointSha256 -or
        [int]$state.checkpoint.completed_results -ne $ExpectedInitialResults -or
        [int]$state.checkpoint.expected_results -ne $expectedTotalResults
    ) {
        throw (
            "Requested release/count does not match validated campaign state"
        )
    }
    return $state
}

$plan = [ordered]@{
    schema_version = 1
    kind = "umlaut-casc-guarded-resume-plan"
    release = $Release
    checkpoint = [ordered]@{
        path = $checkpointPath
        sha256 = $CheckpointSha256
        expected_results = $ExpectedInitialResults
        total_results = $expectedTotalResults
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
    existing_runner_run_id = if (
        $PSBoundParameters.ContainsKey("ExistingRunnerRunId")
    ) {
        $ExistingRunnerRunId
    }
    else {
        $null
    }
}
$campaignState = Invoke-CampaignInspector
$plan.checkpoint["outer_release"] = [string]$campaignState.outer_release
$plan.checkpoint["campaign_completed_results"] = (
    $campaignState.campaign_completed_results
)
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
    throw "Guarded CASC resumes require the clean main branch; found '$branch'"
}
$worktreeStatus = & git -C $repoRoot status --porcelain=v1
if ($LASTEXITCODE -ne 0) {
    throw "Could not inspect the git worktree"
}
if ($null -ne $worktreeStatus -and @($worktreeStatus).Count -gt 0) {
    throw "Guarded CASC resumes require a clean worktree"
}
$null = Invoke-CampaignInspector

$artifactRoot = Join-Path $repoRoot ".artifacts\casc-benchmark"
[IO.Directory]::CreateDirectory($artifactRoot) | Out-Null
$controllerId = (
    [DateTimeOffset]::UtcNow.ToString("yyyyMMddTHHmmssZ") + "-$PID"
)
$logPath = Join-Path $artifactRoot "$Release-resume-controller-$controllerId.log"
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
            $text = [string]$line
            if (-not [string]::IsNullOrEmpty($text)) {
                Write-ResumeLog $text
            }
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

function Invoke-RunnerProbe {
    param([Parameter(Mandatory = $true)][string]$RemoteCommand)

    return Invoke-Runner @(
        "exec",
        "--timeout-seconds",
        [string]$runnerProbeTimeoutSeconds,
        "--",
        $RemoteCommand
    )
}

function Invoke-CheckpointValidator {
    param(
        [Parameter(Mandatory = $true)][string]$Archive,
        [Parameter(Mandatory = $true)][string]$ArchiveSha256,
        [Parameter(Mandatory = $true)][int]$ResultCount
    )

    $python = Join-Path $repoRoot ".venv\Scripts\python.exe"
    if (Test-Path -LiteralPath $python -PathType Leaf) {
        $pythonPrefix = @("-u")
    }
    else {
        $pythonCommand = Get-Command py -ErrorAction SilentlyContinue
        if ($null -ne $pythonCommand) {
            $python = $pythonCommand.Source
            $pythonPrefix = @("-3", "-u")
        }
        else {
            $pythonCommand = Get-Command python -ErrorAction Stop
            $python = $pythonCommand.Source
            $pythonPrefix = @("-u")
        }
    }
    $validationOutput = "$Archive.validation.json"
    if (Test-Path -LiteralPath $validationOutput) {
        throw "Refusing to overwrite validation output: $validationOutput"
    }
    $output = & $python @pythonPrefix $validatorPath `
        --archive $Archive `
        --archive-sha256 $ArchiveSha256 `
        --manifest $manifestPath `
        --run-name $runName `
        --contract-id $expectedContract `
        --expected-results $ResultCount `
        --combined-run CASC-2025 $casc2025ManifestPath `
            $casc2025RunName $casc2025Contract `
        --combined-run CASC-J13 $j13ManifestPath `
            $j13RunName $j13Contract `
        --output $validationOutput 2>&1
    $exitCode = $LASTEXITCODE
    if ($null -ne $output) {
        foreach ($line in $output) {
            $text = [string]$line
            if (-not [string]::IsNullOrEmpty($text)) {
                Write-ResumeLog $text
            }
        }
    }
    if ($exitCode -ne 0) {
        throw "Downloaded checkpoint failed streaming validation"
    }
    if (-not (Test-Path -LiteralPath $validationOutput -PathType Leaf)) {
        throw "Checkpoint validator did not write its evidence sidecar"
    }
    $validationHash = (
        Get-FileHash -LiteralPath $validationOutput -Algorithm SHA256
    ).Hash.ToLowerInvariant()
    Write-ResumeLog (
        "validation_sidecar path=$validationOutput sha256=$validationHash"
    )
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

function Get-RequiredHighMemoryAllowance {
    param(
        [ValidateRange(0, 1)]
        [int]$ExpectedActiveManagedHighMemory = 0,

        [ValidateRange(1, 86400)]
        [int]$RequiredSeconds = $serviceRuntimeSeconds
    )

    $raw = Invoke-Runner @(
        "allowance",
        "--required-seconds",
        [string]$RequiredSeconds
    )
    try {
        $allowance = $raw | ConvertFrom-Json
    }
    catch {
        throw "Could not parse high-memory allowance JSON"
    }
    $requiredBilledSeconds = [int](
        [Math]::Ceiling($RequiredSeconds / 3600.0) * 3600
    )
    if (
        [int]$allowance.schema_version -ne 2 -or
        [string]$allowance.kind -ne "umlaut-linode-high-memory-allowance" -or
        [int]$allowance.required_seconds -ne $RequiredSeconds -or
        [int]$allowance.required_billed_seconds -ne $requiredBilledSeconds -or
        [int]$allowance.remaining_seconds -lt $requiredBilledSeconds -or
        [int]$allowance.active_managed_high_memory -ne
            $ExpectedActiveManagedHighMemory -or
        (
            $ExpectedActiveManagedHighMemory -eq 0 -and
            -not [bool]$allowance.required_start_available_now
        )
    ) {
        throw (
            "Trusted allowance does not permit the required " +
            "$RequiredSeconds-second high-memory operation"
        )
    }
    return $allowance
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
        "LoadState,ActiveState,SubState,MainPID,InvocationID,NRestarts," +
        "Result,ExecMainStatus"
    )
    $raw = Invoke-RunnerProbe -RemoteCommand (
        "systemctl show $Unit --property=$fields"
    )
    return ConvertFrom-SystemdProperties -Text $raw
}

function Get-VerifiedCompletedServiceEvidence {
    param(
        [Parameter(Mandatory = $true)][string]$Unit,
        [Parameter(Mandatory = $true)][hashtable]$Properties,
        [Parameter(Mandatory = $true)][string]$ExpectedInvocation,
        [Parameter(Mandatory = $true)][string]$ExpectedMainPid,
        [Parameter(Mandatory = $true)][string]$ExpectedCommand
    )

    if (
        [string]$Properties.ActiveState -ne "inactive" -or
        [string]$Properties.SubState -ne "dead" -or
        [string]$Properties.MainPID -ne "0" -or
        [string]$Properties.NRestarts -ne "0" -or
        [string]$Properties.Result -ne "success" -or
        [string]$Properties.ExecMainStatus -ne "0" -or
        (
            [string]$Properties.InvocationID -ne "" -and
            [string]$Properties.InvocationID -ne $ExpectedInvocation
        ) -or
        (
            [string]$Properties.LoadState -ne "loaded" -and
            [string]$Properties.LoadState -ne "not-found"
        )
    ) {
        throw "Completed service properties do not match a clean terminal state"
    }

    $verifierBase64 = [Convert]::ToBase64String(
        [Text.Encoding]::UTF8.GetBytes($terminalJournalVerifier)
    )
    $commandBase64 = [Convert]::ToBase64String(
        [Text.Encoding]::UTF8.GetBytes($ExpectedCommand)
    )
    $launcher = "import base64;exec(base64.b64decode('$verifierBase64'))"
    $remoteCommand = (
        "journalctl -u '$Unit' -o json --no-pager | " +
        "python3 -c `"$launcher`" '$Unit' '$ExpectedInvocation' " +
        "'$ExpectedMainPid' '$commandBase64' '$expectedContract'"
    )
    $raw = Invoke-RunnerProbe -RemoteCommand $remoteCommand
    try {
        $evidence = $raw | ConvertFrom-Json
    }
    catch {
        throw "Could not parse completed-service journal evidence"
    }
    if (
        [string]$evidence.unit -ne $Unit -or
        [string]$evidence.invocation_id -ne $ExpectedInvocation -or
        [string]$evidence.main_pid -ne $ExpectedMainPid -or
        [string]$evidence.boot_id -notmatch "^[0-9a-f]{32}$" -or
        [int]$evidence.reported_results -le 0 -or
        [int64]$evidence.success_sequence -le 0
    ) {
        throw "Completed-service journal evidence is inconsistent"
    }
    return $evidence
}

function Get-VerifiedFailedServiceEvidence {
    param(
        [Parameter(Mandatory = $true)][string]$Unit,
        [Parameter(Mandatory = $true)][hashtable]$Properties,
        [Parameter(Mandatory = $true)][string]$ExpectedInvocation,
        [Parameter(Mandatory = $true)][string]$ExpectedMainPid,
        [Parameter(Mandatory = $true)][string]$ExpectedCommand,
        [Parameter(Mandatory = $true)][string]$ExpectedResult,
        [Parameter(Mandatory = $true)][int]$ExpectedExecStatus
    )

    if (
        [string]$Properties.LoadState -ne "loaded" -or
        [string]$Properties.ActiveState -ne "failed" -or
        [string]$Properties.SubState -ne "failed" -or
        [string]$Properties.MainPID -ne "0" -or
        [string]$Properties.InvocationID -ne $ExpectedInvocation -or
        [string]$Properties.NRestarts -ne "0" -or
        [string]$Properties.Result -ne $ExpectedResult -or
        [string]$Properties.ExecMainStatus -ne [string]$ExpectedExecStatus
    ) {
        throw "Failed service properties do not match the expected terminal state"
    }

    $verifierBase64 = [Convert]::ToBase64String(
        [Text.Encoding]::UTF8.GetBytes($failedTerminalJournalVerifier)
    )
    $commandBase64 = [Convert]::ToBase64String(
        [Text.Encoding]::UTF8.GetBytes($ExpectedCommand)
    )
    $launcher = "import base64;exec(base64.b64decode('$verifierBase64'))"
    $remoteCommand = (
        "journalctl -u '$Unit' -o json --no-pager | " +
        "python3 -c `"$launcher`" '$Unit' '$ExpectedInvocation' " +
        "'$ExpectedMainPid' '$commandBase64' '$ExpectedResult' " +
        "'$ExpectedExecStatus'"
    )
    $raw = Invoke-RunnerProbe -RemoteCommand $remoteCommand
    try {
        $evidence = $raw | ConvertFrom-Json
    }
    catch {
        throw "Could not parse failed-service journal evidence"
    }
    if (
        [string]$evidence.unit -ne $Unit -or
        [string]$evidence.invocation_id -ne $ExpectedInvocation -or
        [string]$evidence.main_pid -ne $ExpectedMainPid -or
        [string]$evidence.result -ne $ExpectedResult -or
        [int]$evidence.exec_status -ne $ExpectedExecStatus -or
        [string]$evidence.boot_id -notmatch "^[0-9a-f]{32}$" -or
        [int64]$evidence.failure_sequence -le 0
    ) {
        throw "Failed-service journal evidence is inconsistent"
    }
    return $evidence
}

$runnerAcquired = $false
$launchAttempted = $false
$captureSucceeded = $false
$localArchive = $null
$remoteArchive = $null
$unit = $null
$expectedInvocation = $null
$expectedMainPid = $null
$terminalEvidence = $null
$terminalReportedResults = $null

try {
    Write-ResumeLog "controller_started"
    $initialStatus = Get-RunnerStatus
    if (@($initialStatus.parked).Count -ne 0) {
        throw "Parked managed runners must be resolved before a CASC resume"
    }
    $useExistingRunner = $PSBoundParameters.ContainsKey(
        "ExistingRunnerRunId"
    )
    $expectedRunnerPhase = if ($adoptingAnyService) {
        "synced"
    }
    else {
        "ready"
    }
    if ($useExistingRunner) {
        $status = $initialStatus
        $candidate = $status.active
        if (
            $null -eq $candidate -or
            [string]$candidate.run_id -ne $ExistingRunnerRunId -or
            [string]$candidate.label -ne
                "e-rust-codex-$ExistingRunnerRunId" -or
            [string]$candidate.lifecycle -ne "active" -or
            [string]$candidate.type -ne "g7-highmem-8" -or
            [string]$candidate.phase -ne $expectedRunnerPhase -or
            [string]$candidate.live_linode_status -ne "running" -or
            [string]$candidate.live_firewall_status -ne "enabled"
        ) {
            throw "Requested existing runner is not the exact ready host"
        }
        $requiredExistingSeconds = if (
            $adoptingCompletedService -or $adoptingFailedService
        ) {
            $terminalCaptureAllowanceSeconds
        }
        else {
            $serviceRuntimeSeconds
        }
        $allowance = Get-RequiredHighMemoryAllowance `
            -ExpectedActiveManagedHighMemory 1 `
            -RequiredSeconds $requiredExistingSeconds
        Write-ResumeLog (
            "allowance_verified_existing observed_at=" +
            "$($allowance.observed_at_utc) remaining_seconds=" +
            "$($allowance.remaining_seconds) required_seconds=" +
            "$requiredExistingSeconds"
        )
        $runnerAcquired = $true
        Write-ResumeLog "existing_runner_claimed run_id=$ExistingRunnerRunId"
    }
    else {
        if ($null -ne $initialStatus.active) {
            throw "An active managed runner already exists"
        }
        $allowance = Get-RequiredHighMemoryAllowance
        Write-ResumeLog (
            "allowance_verified observed_at=$($allowance.observed_at_utc) " +
            "remaining_seconds=$($allowance.remaining_seconds)"
        )
        Invoke-Runner @("check", "--high-memory") | Out-Null
        Invoke-Runner @("up", "--high-memory") | Out-Null
        $runnerAcquired = $true
        $status = Get-RunnerStatus
    }

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
        [string]$active.phase -ne $expectedRunnerPhase -or
        [string]$active.live_linode_status -ne "running" -or
        $linodeId -le 0
    ) {
        throw "Managed runner is not a ready canonical high-memory host"
    }

    $sessionId = "$sessionPrefix-$runId"
    $unit = "$servicePrefix-$runId.service"
    $remoteCheckpointInput = "/root/input-casc-checkpoint.tar.gz"
    $remoteCorpus = "/root/$($releaseConfig.corpus_file)"
    $remoteManifestRelative = "benchmarks/$($releaseConfig.manifest_file)"
    $remoteManifest = "/opt/e-rust-port/source/$remoteManifestRelative"
    $remoteUmlaut = "/root/umlaut-4e87dac3"
    $remoteVampire = "/root/vampire-5.0.1"
    $remoteRestoreRoot = "/root/casc-restore"
    $remoteCheckpointRoot = "/root/$checkpointPrefix-$runId"
    $remoteArchive = "$remoteCheckpointRoot.tar.gz"
    $localArchive = Join-Path (
        $artifactRoot
    ) "$checkpointPrefix-$runId.tar.gz"
    if (Test-Path -LiteralPath $localArchive) {
        throw "Refusing to overwrite checkpoint output: $localArchive"
    }

    $batchCommand = @(
        "/usr/bin/python3",
        "/opt/e-rust-port/source/tools/casc_benchmark/batch.py",
        "--manifest",
        $remoteManifest,
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
        "--expected-contract-id",
        $expectedContract,
        "--runner-label=$runnerLabel",
        "--runner-run-id=$runId",
        "--linode-id=$linodeId"
    ) -join " "

    Write-ResumeLog (
        "runner_ready run_id=$runId label=$runnerLabel linode_id=$linodeId"
    )
    if ($adoptingAnyService) {
        $launchAttempted = $true
        $expectedMainPid = [string]$ExpectedServiceMainPid
        $expectedInvocation = $ExpectedServiceInvocationId
        $properties = Get-ServiceProperties -Unit $unit
        if ($adoptingCompletedService) {
            $terminalEvidence = Get-VerifiedCompletedServiceEvidence `
                -Unit $unit `
                -Properties $properties `
                -ExpectedInvocation $expectedInvocation `
                -ExpectedMainPid $expectedMainPid `
                -ExpectedCommand $batchCommand
            $terminalReportedResults = [int]$terminalEvidence.reported_results
        }
        elseif ($adoptingFailedService) {
            $terminalEvidence = Get-VerifiedFailedServiceEvidence `
                -Unit $unit `
                -Properties $properties `
                -ExpectedInvocation $expectedInvocation `
                -ExpectedMainPid $expectedMainPid `
                -ExpectedCommand $batchCommand `
                -ExpectedResult $ExpectedServiceResult `
                -ExpectedExecStatus $ExpectedServiceExecStatus
            if ($ClearFailedCaptureResidue) {
                $cleanupFailedCaptureCommand = @"
set -Eeuo pipefail
test ! -e '$remoteArchive'
python3 - '$remoteCheckpointRoot' '$expectedMainPid' '$runRoot' <<'PY'
import os
import shutil
import sys
from pathlib import Path

root = Path(sys.argv[1])
expected_pid = sys.argv[2]
run_root = Path(sys.argv[3])
if str(root) != os.path.realpath(root):
    raise SystemExit(f"partial checkpoint path is not exact: {root}")
if not root.is_dir() or root.is_symlink():
    raise SystemExit(f"partial checkpoint is not a plain directory: {root}")
allowed = {
    "casc-runs.tar.gz",
    "cgroup-residue.txt",
    "package-maintenance-quiescence.json",
    "processes.txt",
    "result-count.txt",
    "result-files.txt",
    "service-journal.jsonl",
    "service-properties.txt",
    "service.log",
    "solver-units.txt",
}
children = list(root.iterdir())
names = {child.name for child in children}
if names != allowed or any(not child.is_file() or child.is_symlink() for child in children):
    raise SystemExit(f"unexpected partial checkpoint inventory: {sorted(names)!r}")
partial_count = int((root / "result-count.txt").read_text().split()[0])
live_count = sum(1 for _path in (run_root / "results").rglob("*.json"))
if partial_count != live_count:
    raise SystemExit(
        f"partial/live result count mismatch: {partial_count} != {live_count}"
    )
residues = [
    Path(line)
    for line in (root / "cgroup-residue.txt").read_text().splitlines()
    if line
]
if len(residues) != 1:
    raise SystemExit(f"expected one captured cgroup residue, got {residues!r}")
for residue in residues:
    if (
        residue.parent != Path("/sys/fs/cgroup")
        or not residue.name.startswith(f"umlaut-casc-{expected_pid}-")
        or str(residue) != os.path.realpath(residue)
        or not residue.is_dir()
        or residue.is_symlink()
    ):
        raise SystemExit(f"cgroup residue identity mismatch: {residue}")
    procs = (residue / "cgroup.procs").read_text().split()
    threads = (residue / "cgroup.threads").read_text().split()
    events = dict(
        line.split(maxsplit=1)
        for line in (residue / "cgroup.events").read_text().splitlines()
    )
    if procs or threads or events.get("populated") != "0":
        raise SystemExit(
            f"cgroup residue remains populated: {residue} "
            f"procs={procs!r} threads={threads!r} events={events!r}"
        )
    residue.rmdir()
shutil.rmtree(root)
print(f"cleared_failed_capture_result_count={live_count}")
PY
test ! -e '$remoteCheckpointRoot'
"@
                $cleanupOutput = Invoke-RunnerProbe `
                    -RemoteCommand $cleanupFailedCaptureCommand
                if (
                    $cleanupOutput -notmatch
                        "cleared_failed_capture_result_count=(\d+)"
                ) {
                    throw "Failed-capture cleanup did not return a result count"
                }
                $clearedCaptureResultCount = [int]$Matches[1]
                Write-ResumeLog (
                    "failed_capture_residue_cleared unit=$unit " +
                    "result_count=$clearedCaptureResultCount"
                )
            }
        }
        else {
            if (
                [string]$properties.LoadState -ne "loaded" -or
                [string]$properties.ActiveState -ne "active" -or
                [string]$properties.SubState -ne "running" -or
                [string]$properties.MainPID -ne $expectedMainPid -or
                [string]$properties.InvocationID -ne
                    $expectedInvocation -or
                [string]$properties.NRestarts -ne "0" -or
                [string]$properties.Result -ne "success" -or
                [string]$properties.ExecMainStatus -ne "0"
            ) {
                throw "Existing service does not match the required live identity"
            }
            $execStart = (
                Invoke-RunnerProbe -RemoteCommand (
                    "systemctl show $unit --property=ExecStart --value"
                )
            ).Trim()
            $expectedExecStart = "argv[]=$batchCommand ;"
            if (-not $execStart.Contains($expectedExecStart)) {
                throw "Existing service command does not match the guarded batch"
            }
        }
        $adoptionCommand = @"
set -Eeuo pipefail
printf '%s  %s\n' '$corpusSha256' '$remoteCorpus' '$CheckpointSha256' '$remoteCheckpointInput' '$umlautSha256' '$remoteUmlaut' '$vampireSha256' '$remoteVampire' | sha256sum -c -
printf '%s  %s\n' '$expectedContractFile' '$runRoot/contract.json' | sha256sum -c -
test ! -e '$remoteCheckpointRoot'
test ! -e '$remoteArchive'
python3 - '$runRoot' '$expectedContract' '$ExpectedInitialResults' '$expectedTotalResults' <<'PY'
import json
import sys
from pathlib import Path

run_root = Path(sys.argv[1])
expected_contract = sys.argv[2]
initial_results = int(sys.argv[3])
expected_total = int(sys.argv[4])
contract = json.loads((run_root / "contract.json").read_text(encoding="utf-8"))
if contract.get("contract_id") != expected_contract:
    raise SystemExit(
        f"contract mismatch: {contract.get('contract_id')!r}"
    )
result_count = sum(1 for _path in (run_root / "results").rglob("*.json"))
if result_count <= initial_results or result_count > expected_total:
    raise SystemExit(
        f"result count outside recovery range: {result_count}"
    )
print(f"adoption_result_count={result_count}")
PY
"@
        $adoptionOutput = Invoke-RunnerProbe -RemoteCommand $adoptionCommand
        if ($adoptionOutput -notmatch "adoption_result_count=(\d+)") {
            throw "Existing service adoption did not return a result count"
        }
        $adoptedResultCount = [int]$Matches[1]
        if (
            $adoptingCompletedService -and
            $adoptedResultCount -ne $terminalReportedResults
        ) {
            throw (
                "Completed-service journal count does not match the result " +
                "inventory: $terminalReportedResults != $adoptedResultCount"
            )
        }
        if (
            $ClearFailedCaptureResidue -and
            $adoptedResultCount -ne $clearedCaptureResultCount
        ) {
            throw (
                "Cleared partial checkpoint count does not match the result " +
                "inventory: $clearedCaptureResultCount != $adoptedResultCount"
            )
        }
        $adoptionKind = if ($adoptingCompletedService) {
            "completed_service_adopted"
        }
        elseif ($adoptingFailedService) {
            "failed_service_adopted"
        }
        else {
            "existing_service_adopted"
        }
        Write-ResumeLog (
            "$adoptionKind unit=$unit main_pid=$expectedMainPid " +
            "invocation_id=$expectedInvocation result_count=$adoptedResultCount"
        )
    }
    else {
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
python3 tools/casc_benchmark/corpus_archive.py extract --archive '$remoteCorpus' --destination /opt/e-rust-port/source --manifest '$remoteManifestRelative'
tar -xzf '$remoteCheckpointInput' -C '$remoteRestoreRoot'
cd '$remoteRestoreRoot/$checkpointRootName'
sha256sum -c SHA256SUMS
tar -xzf casc-runs.tar.gz -C /opt/e-rust-port
printf '%s  %s\n' '$expectedContractFile' '$runRoot/contract.json' | sha256sum -c -
cd /opt/e-rust-port/source
python3 tools/casc_benchmark/report.py --manifest '$remoteManifestRelative' --run-root '$runRoot' --allow-partial
python3 - '$runRoot' '$expectedContract' '$ExpectedInitialResults' <<'PY'
import json
import sys
from pathlib import Path

run_root = Path(sys.argv[1])
expected_contract = sys.argv[2]
expected_results = int(sys.argv[3])
summary = json.loads((run_root / "summary.json").read_text(encoding="utf-8"))
if summary.get("contract_id") != expected_contract:
    raise SystemExit(
        f"summary contract mismatch: {summary.get('contract_id')!r}"
    )
if summary.get("completed_results") != expected_results:
    raise SystemExit(
        "summary completed-result mismatch: "
        f"{summary.get('completed_results')!r}"
    )
actual_results = sum(1 for _path in (run_root / "results").rglob("*.json"))
if actual_results != expected_results:
    raise SystemExit(
        f"result inventory mismatch: {actual_results} != {expected_results}"
    )
print(
    f"OK: summary contract/count/result inventory {expected_results}"
)
PY
"@
    Invoke-Runner @("exec", "--", $preflightCommand) | Out-Null
    Write-ResumeLog (
        "checkpoint_restore_verified initial_results=$ExpectedInitialResults"
    )

    $verifyCommand = "cd /opt/e-rust-port/source && " +
        "python3 tools/casc_benchmark/batch.py " +
        "--manifest '$remoteManifestRelative' " +
        "--problem-root /opt/e-rust-port/source " +
        "--output-root '$runRoot' " +
        "--umlaut-binary '$remoteUmlaut' " +
        "--vampire-binary '$remoteVampire' " +
        "--solvers both --cores 8 --memory-limit-mib 131072 " +
        "--pids-limit 512 --vampire-seed 1 " +
        "--wall-grace-seconds 0.25 --terminate-grace-seconds 1 " +
        "--session-id '$Release-preflight-$runId' " +
        "--source-snapshot-sha256 '$sourceSnapshot' " +
        "--expected-contract-id '$expectedContract' " +
        "--runner-label '$runnerLabel' --runner-run-id '$runId' " +
        "--linode-id '$linodeId' --verify-only"
    Invoke-Runner @("exec", "--", $verifyCommand) | Out-Null
    Write-ResumeLog "preflight_passed initial_results=$ExpectedInitialResults"

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
    }

    $monitorDeadline = [DateTimeOffset]::UtcNow.AddSeconds(
        $serviceRuntimeSeconds + 300
    )
    if ($null -eq $terminalEvidence) {
        while ($true) {
            $properties = Get-ServiceProperties -Unit $unit
            if ([string]$properties.ActiveState -eq "active") {
                if (
                    [string]$properties.LoadState -ne "loaded" -or
                    [string]$properties.InvocationID -ne
                        $expectedInvocation
                ) {
                    throw "Transient service invocation changed"
                }
                if ([string]$properties.NRestarts -ne "0") {
                    throw "Transient service restarted"
                }
                if ([string]$properties.MainPID -ne $expectedMainPid) {
                    throw "Transient service main PID changed while active"
                }
                $resultCount = Invoke-RunnerProbe -RemoteCommand (
                    "find '$runRoot/results' -type f -name '*.json' | wc -l"
                )
                Write-ResumeLog (
                    "service_active result_count=$($resultCount.Trim())"
                )
                if ([DateTimeOffset]::UtcNow -ge $monitorDeadline) {
                    throw (
                        "Transient service exceeded its guarded monitoring " +
                        "deadline"
                    )
                }
                Start-Sleep -Seconds $PollSeconds
                continue
            }

            if ([string]$properties.InvocationID -eq $expectedInvocation) {
                if ([string]$properties.NRestarts -ne "0") {
                    throw "Transient service restarted"
                }
                if (
                    [string]$properties.ActiveState -eq "inactive" -and
                    [string]$properties.Result -eq "success" -and
                    [string]$properties.ExecMainStatus -eq "0"
                ) {
                    $terminalEvidence = Get-VerifiedCompletedServiceEvidence `
                        -Unit $unit `
                        -Properties $properties `
                        -ExpectedInvocation $expectedInvocation `
                        -ExpectedMainPid $expectedMainPid `
                        -ExpectedCommand $batchCommand
                    $terminalReportedResults = [int](
                        $terminalEvidence.reported_results
                    )
                }
                break
            }
            if ([string]$properties.InvocationID -ne "") {
                throw "Transient service invocation changed"
            }
            $terminalEvidence = Get-VerifiedCompletedServiceEvidence `
                -Unit $unit `
                -Properties $properties `
                -ExpectedInvocation $expectedInvocation `
                -ExpectedMainPid $expectedMainPid `
                -ExpectedCommand $batchCommand
            $terminalReportedResults = [int]$terminalEvidence.reported_results
            break
        }
    }

    $serviceSucceeded = if ($adoptingFailedService) {
        $false
    }
    else {
        (
            $null -ne $terminalEvidence -or
            (
                [string]$properties.ActiveState -eq "inactive" -and
                [string]$properties.Result -eq "success" -and
                [string]$properties.ExecMainStatus -eq "0" -and
                [string]$properties.InvocationID -eq $expectedInvocation -and
                [string]$properties.NRestarts -eq "0"
            )
        )
    }
    if ($null -ne $terminalEvidence) {
        Write-ResumeLog (
            "terminal_service_identity_verified unit=$unit " +
            "main_pid=$expectedMainPid invocation_id=$expectedInvocation " +
            "boot_id=$($terminalEvidence.boot_id) " +
            "reported_results=$terminalReportedResults"
        )
    }
    Write-ResumeLog (
        "service_finished active_state=$($properties.ActiveState) " +
        "result=$($properties.Result) exec_status=$($properties.ExecMainStatus)"
    )

    $terminalBootId = if ($null -ne $terminalEvidence) {
        [string]$terminalEvidence.boot_id
    }
    else {
        ""
    }
    $terminalResultCount = if ($null -ne $terminalReportedResults) {
        [string]$terminalReportedResults
    }
    else {
        ""
    }
$captureCommand = @"
set -Eeuo pipefail
if pgrep -f '^/root/umlaut-4e87dac3( |$)' || pgrep -f '^/root/vampire-5.0.1( |$)' || pgrep -f '^/usr/bin/python3 /opt/e-rust-port/source/tools/casc_benchmark/batch.py( |$)'; then echo 'solver or batch process remains' >&2; exit 1; fi
python3 - '$runRoot/results' <<'PY'
import sys
from pathlib import Path

results_root = Path(sys.argv[1])
if not results_root.is_dir() or results_root.is_symlink():
    raise SystemExit(f"result root is not a plain directory: {results_root}")
artifacts = {}
for path in results_root.rglob("*"):
    if path.is_symlink():
        raise SystemExit(f"result tree contains a symlink: {path}")
    if path.is_file() and path.suffix in {".stdout", ".stderr"}:
        artifacts.setdefault(path.with_suffix(""), set()).add(path.suffix)
removed = []
for base, suffixes in sorted(artifacts.items(), key=lambda item: str(item[0])):
    result = base.with_suffix(".json")
    if result.exists():
        continue
    if suffixes != {".stdout", ".stderr"}:
        raise SystemExit(
            f"incomplete result has an ambiguous artifact set: {base} {suffixes!r}"
        )
    for suffix in sorted(suffixes):
        artifact = base.with_suffix(suffix)
        artifact.unlink()
        removed.append(str(artifact))
print(
    f"incomplete_result_artifacts_removed={len(removed)} "
    f"incomplete_result_bases={len(removed) // 2}"
)
PY
cd /opt/e-rust-port/source
python3 tools/casc_benchmark/report.py --manifest '$casc2025ManifestRelative' --run-root '$casc2025RunRoot' --allow-partial
python3 tools/casc_benchmark/report.py --manifest '$j13ManifestRelative' --run-root '$j13RunRoot' --allow-partial
python3 tools/casc_benchmark/combined_report.py --allow-partial --input CASC-2025 '$casc2025ManifestRelative' '$casc2025RunRoot' --input CASC-J13 '$j13ManifestRelative' '$j13RunRoot' --output '$combinedSummary'
grep -Fq '"contract_id":"$expectedContract"' '$runRoot/summary.json'
grep -Fq '"expected_results":8502' '$combinedSummary'
grep -Fq '"targeted_problems":4251' '$combinedSummary'
grep -Fq '"csv_count":66' '$combinedSummary'
test ! -e '$remoteCheckpointRoot'
install -d -m 0700 '$remoteCheckpointRoot'
tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner -czf '$remoteCheckpointRoot/casc-runs.tar.gz' -C /opt/e-rust-port casc-runs
journalctl -u '$unit' --no-pager -o short-iso > '$remoteCheckpointRoot/service.log'
journalctl -u '$unit' --no-pager -o json > '$remoteCheckpointRoot/service-journal.jsonl'
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
printf '%s\n' 'controller_id=$controllerId' 'release=$Release' 'parent_checkpoint=$checkpointName' 'parent_checkpoint_sha256=$CheckpointSha256' 'initial_results=$ExpectedInitialResults' 'expected_total_results=$expectedTotalResults' 'max_session_wall_seconds=$MaxSessionWallSeconds' 'runner_id=$runId' 'runner_label=$runnerLabel' 'linode_id=$linodeId' 'unit=$unit' 'initial_main_pid=$expectedMainPid' 'initial_invocation_id=$expectedInvocation' 'terminal_boot_id=$terminalBootId' 'terminal_reported_results=$terminalResultCount' > '$remoteCheckpointRoot/resume-metadata.txt'
cd '$remoteCheckpointRoot'
sha256sum -- * > SHA256SUMS
cd /root
tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner -czf '$remoteArchive' '$checkpointPrefix-$runId'
"@
    $captureOutput = Invoke-Runner @("exec", "--", $captureCommand)
    foreach ($line in $captureOutput) {
        $text = [string]$line
        if (-not [string]::IsNullOrEmpty($text)) {
            Write-ResumeLog $text
        }
    }

    $remoteHashLine = Invoke-RunnerProbe -RemoteCommand (
        "sha256sum '$remoteArchive'"
    )
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

    $finalCountText = Invoke-RunnerProbe -RemoteCommand (
        "find '$runRoot/results' -type f -name '*.json' | wc -l"
    )
    $finalCount = 0
    if (-not [int]::TryParse($finalCountText.Trim(), [ref]$finalCount)) {
        throw "Could not parse the final result count"
    }
    if (
        $finalCount -le $ExpectedInitialResults -or
        $finalCount -gt $expectedTotalResults
    ) {
        throw (
            "Final result count is outside the guarded range: " +
            "$finalCount after $ExpectedInitialResults"
        )
    }
    if (
        $null -ne $terminalReportedResults -and
        $finalCount -ne $terminalReportedResults
    ) {
        throw (
            "Final result count does not match the terminal journal " +
            "summary: $finalCount != $terminalReportedResults"
        )
    }

    Invoke-CheckpointValidator `
        -Archive $localArchive `
        -ArchiveSha256 $localHash `
        -ResultCount $finalCount

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
        try {
            Invoke-Runner @(
                "guard-recovery",
                "--grace-seconds",
                [string]$recoveryGraceSeconds
            ) | Out-Null
            $guardStatus = Invoke-Runner @("status") | ConvertFrom-Json
            if (
                $null -eq $guardStatus.active -or
                [string]$guardStatus.active.lifecycle -ne "guarded-recovery"
            ) {
                throw "Recovery guard did not retain an exact guarded runner"
            }
            $guardDeadline = [DateTimeOffset]::Parse(
                [string]$guardStatus.active.delete_at,
                [Globalization.CultureInfo]::InvariantCulture,
                [Globalization.DateTimeStyles]::AssumeUniversal
            )
            if ($guardDeadline -le [DateTimeOffset]::UtcNow) {
                throw "Recovery guard returned an expired deletion deadline"
            }
            Write-ResumeLog (
                "runner_guarded_for_recovery grace_seconds=" +
                "$recoveryGraceSeconds delete_at=$($guardDeadline.ToString('O'))"
            )
        }
        catch {
            Write-ResumeLog (
                "recovery_guard_failed error=$($_.Exception.Message)"
            )
            try {
                Invoke-Runner @("down", "--all") | Out-Null
                Write-ResumeLog "recovery_guard_fallback_deleted"
            }
            catch {
                Write-ResumeLog (
                    "URGENT_recovery_guard_cleanup_failed " +
                    "error=$($_.Exception.Message)"
                )
                throw
            }
            throw
        }
    }
}

[ordered]@{
    checkpoint_path = $localArchive
    checkpoint_sha256 = (
        Get-FileHash -LiteralPath $localArchive -Algorithm SHA256
    ).Hash.ToLowerInvariant()
    controller_log = $logPath
} | ConvertTo-Json
