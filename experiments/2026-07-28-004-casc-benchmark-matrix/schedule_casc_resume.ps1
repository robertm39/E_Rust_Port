[CmdletBinding(DefaultParameterSetName = "Inspect")]
param(
    [Parameter(Mandatory = $true)]
    [string]$Plan,

    [Parameter(Mandatory = $true, ParameterSetName = "Register")]
    [switch]$Register,

    [Parameter(Mandatory = $true, ParameterSetName = "Audit")]
    [switch]$Audit,

    [Parameter(Mandatory = $true, ParameterSetName = "Launch")]
    [switch]$Launch,

    [Parameter(Mandatory = $true, ParameterSetName = "Launch")]
    [ValidatePattern('^Umlaut-CASC-(J13|CASC2025)-Resume-\d{8}T\d{6}Z$')]
    [string]$ScheduledTaskName,

    [ValidateRange(1, 24)]
    [int]$ExecutionTimeHours = 8
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$retryInterval = New-TimeSpan -Minutes 5
$retryDuration = New-TimeSpan -Hours 24
$retryIntervalIso8601 = "PT5M"
$retryDurationIso8601 = "P1D"
$immediateLaunchDelay = New-TimeSpan -Minutes 5

function Get-RequiredProperty {
    param(
        [Parameter(Mandatory = $true)]
        [object]$InputObject,

        [Parameter(Mandatory = $true)]
        [string]$Name,

        [Parameter(Mandatory = $true)]
        [string]$Context
    )

    if ($InputObject.PSObject.Properties.Name -notcontains $Name) {
        throw "$Context is missing required property '$Name'"
    }
    return $InputObject.PSObject.Properties[$Name].Value
}

function Get-CanonicalPath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,

        [Parameter(Mandatory = $true)]
        [string]$Description,

        [Parameter(Mandatory = $true)]
        [string]$RepositoryRoot,

        [switch]$AllowMissing
    )

    $candidate = [IO.Path]::GetFullPath($Path)
    if (
        (Test-Path -LiteralPath $candidate) -and
        -not (Test-Path -LiteralPath $candidate -PathType Leaf)
    ) {
        throw "$Description is not a plain file: $candidate"
    }
    if (
        -not $AllowMissing -and
        -not (Test-Path -LiteralPath $candidate -PathType Leaf)
    ) {
        throw "$Description is missing: $candidate"
    }
    $rootPrefix = $RepositoryRoot.TrimEnd('\', '/') + [IO.Path]::DirectorySeparatorChar
    if (-not $candidate.StartsWith(
        $rootPrefix,
        [StringComparison]::OrdinalIgnoreCase
    )) {
        throw "$Description must remain inside the repository: $candidate"
    }
    return $candidate
}

function Assert-ExactString {
    param(
        [AllowEmptyString()]
        [string]$Actual,

        [AllowEmptyString()]
        [string]$Expected,

        [Parameter(Mandatory = $true)]
        [string]$Description,

        [switch]$IgnoreCase
    )

    $comparison = if ($IgnoreCase) {
        [StringComparison]::OrdinalIgnoreCase
    }
    else {
        [StringComparison]::Ordinal
    }
    if (-not [string]::Equals($Actual, $Expected, $comparison)) {
        throw "$Description mismatch: '$Actual' != '$Expected'"
    }
}

function Format-ActionArgument {
    param(
        [Parameter(Mandatory = $true)]
        [AllowEmptyString()]
        [string]$Value,

        [switch]$AlwaysQuote
    )

    if ($Value.Contains('"')) {
        throw "Scheduled action arguments may not contain a double quote"
    }
    if ($AlwaysQuote -or $Value -match '\s') {
        if ($Value.EndsWith('\')) {
            throw "Quoted scheduled action arguments may not end in a backslash"
        }
        return '"' + $Value + '"'
    }
    return $Value
}

function Get-ScheduledActionArguments {
    param(
        [Parameter(Mandatory = $true)]
        [string]$PlanPath,

        [Parameter(Mandatory = $true)]
        [string]$TaskName
    )

    $template = (
        '-WindowStyle Hidden -NoProfile -NonInteractive ' +
        '-ExecutionPolicy Bypass -File {0} -Plan {1} -Launch ' +
        '-ScheduledTaskName {2} -ExecutionTimeHours {3}'
    )
    return $template -f @(
        (Format-ActionArgument -Value $PSCommandPath -AlwaysQuote),
        (Format-ActionArgument -Value $PlanPath -AlwaysQuote),
        (Format-ActionArgument -Value $TaskName -AlwaysQuote),
        $ExecutionTimeHours
    )
}

function Get-AccountSid {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Account
    )

    try {
        return ([Security.Principal.NTAccount]::new($Account)).Translate(
            [Security.Principal.SecurityIdentifier]
        ).Value
    }
    catch {
        throw "Could not resolve scheduled-task principal: $Account"
    }
}

function Get-TaskEvidence {
    param(
        [Parameter(Mandatory = $true)]
        [object]$ValidatedPlan,

        [Parameter(Mandatory = $true)]
        [string]$Status
    )

    return [ordered]@{
        schema_version = 1
        kind = "umlaut-casc-scheduled-resume"
        status = $Status
        observed_at_utc = [DateTimeOffset]::UtcNow.ToString("O")
        plan = [ordered]@{
            path = $ValidatedPlan.plan_path
            sha256 = $ValidatedPlan.plan_sha256
        }
        release = $ValidatedPlan.release
        checkpoint = [ordered]@{
            path = $ValidatedPlan.checkpoint_path
            sha256 = $ValidatedPlan.checkpoint_sha256
            completed_results = $ValidatedPlan.completed_results
            expected_results = $ValidatedPlan.expected_results
        }
        task = [ordered]@{
            name = $ValidatedPlan.task_name
            launch_mode = $ValidatedPlan.launch_mode
            trigger_utc = $ValidatedPlan.not_before.ToString("O")
            retry_interval = $retryIntervalIso8601
            retry_duration = $retryDurationIso8601
            disables_before_controller = $true
            execute = "powershell.exe"
            arguments = $ValidatedPlan.action_arguments
            window_style = "Hidden"
            working_directory = $ValidatedPlan.repo_root
            controller = [ordered]@{
                script = $ValidatedPlan.controller_path
                arguments = $ValidatedPlan.controller_arguments
            }
            principal = $ValidatedPlan.principal
            principal_sid = $ValidatedPlan.principal_sid
            logon_type = "Interactive"
            run_level = "Limited"
            execution_time_hours = $ExecutionTimeHours
            wake_to_run = $true
            run_only_if_network_available = $true
            start_when_available = $true
            multiple_instances = "IgnoreNew"
        }
    }
}

function Get-ValidatedPlan {
    param(
        [Parameter(Mandatory = $true)]
        [string]$PlanPath
    )

    $repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..\..")).Path
    $canonicalPlan = Get-CanonicalPath `
        -Path $PlanPath `
        -Description "resume plan" `
        -RepositoryRoot $repoRoot
    try {
        $document = Get-Content -LiteralPath $canonicalPlan -Raw | ConvertFrom-Json
    }
    catch {
        throw "Resume plan is not valid JSON: $canonicalPlan"
    }

    if ([int](Get-RequiredProperty $document "schema_version" "resume plan") -ne 1) {
        throw "Unsupported resume-plan schema"
    }
    Assert-ExactString `
        ([string](Get-RequiredProperty $document "kind" "resume plan")) `
        "umlaut-casc-next-resume-plan" `
        "resume-plan kind"
    Assert-ExactString `
        ([string](Get-RequiredProperty $document "status" "resume plan")) `
        "ready_to_arm" `
        "resume-plan status"

    $release = [string](Get-RequiredProperty $document "release" "resume plan")
    $expectedTotal = switch ($release) {
        "j13" { 2700 }
        "casc2025" { 5802 }
        default { throw "Unsupported CASC release in resume plan: $release" }
    }

    $checkpoint = Get-RequiredProperty $document "checkpoint" "resume plan"
    $checkpointPath = Get-CanonicalPath `
        -Path ([string](Get-RequiredProperty $checkpoint "path" "checkpoint")) `
        -Description "checkpoint archive" `
        -RepositoryRoot $repoRoot
    $checkpointSha256 = [string](
        Get-RequiredProperty $checkpoint "sha256" "checkpoint"
    )
    if ($checkpointSha256 -notmatch '^[0-9a-f]{64}$') {
        throw "Checkpoint SHA-256 must be canonical lowercase hexadecimal"
    }
    $actualCheckpointSha256 = (
        Get-FileHash -LiteralPath $checkpointPath -Algorithm SHA256
    ).Hash.ToLowerInvariant()
    Assert-ExactString `
        $actualCheckpointSha256 `
        $checkpointSha256 `
        "checkpoint SHA-256"
    $completedResults = [int](
        Get-RequiredProperty $checkpoint "completed_results" "checkpoint"
    )
    $expectedResults = [int](
        Get-RequiredProperty $checkpoint "expected_results" "checkpoint"
    )
    if ($expectedResults -ne $expectedTotal) {
        throw "Checkpoint release total is not canonical for $release"
    }
    if ($completedResults -lt 0 -or $completedResults -ge $expectedResults) {
        throw "Checkpoint is not an incomplete resumable release"
    }

    $allowance = Get-RequiredProperty $document "allowance" "resume plan"
    $requiredStartAvailableNow = [bool](
        Get-RequiredProperty `
            $allowance `
            "required_start_available_now" `
            "allowance"
    )
    $observedAt = [DateTimeOffset]::Parse(
        [string](
            Get-RequiredProperty `
                $allowance `
                "observed_at_utc" `
                "allowance"
        ),
        [Globalization.CultureInfo]::InvariantCulture,
        (
            [Globalization.DateTimeStyles]::AssumeUniversal -bor
            [Globalization.DateTimeStyles]::AdjustToUniversal
        )
    )
    $projectedStart = $null
    if (-not $requiredStartAvailableNow) {
        $projectedStart = [DateTimeOffset]::Parse(
            [string](
                Get-RequiredProperty `
                    $allowance `
                    "projected_earliest_required_start_utc" `
                    "allowance"
            ),
            [Globalization.CultureInfo]::InvariantCulture,
            (
                [Globalization.DateTimeStyles]::AssumeUniversal -bor
                [Globalization.DateTimeStyles]::AdjustToUniversal
            )
        )
    }

    $controller = Get-RequiredProperty $document "controller" "resume plan"
    $controllerPath = Get-CanonicalPath `
        -Path ([string](Get-RequiredProperty $controller "script" "controller")) `
        -Description "resume controller" `
        -RepositoryRoot $repoRoot
    $expectedController = [IO.Path]::GetFullPath(
        (Join-Path $PSScriptRoot "resume_j13_checkpoint.ps1")
    )
    Assert-ExactString `
        $controllerPath `
        $expectedController `
        "resume-controller path" `
        -IgnoreCase

    $arguments = @(
        Get-RequiredProperty $controller "arguments" "controller" |
            ForEach-Object { [string]$_ }
    )
    $expectedFlags = [Collections.Generic.List[string]]@(
        "-Release",
        "-CheckpointArchive",
        "-CheckpointSha256",
        "-ExpectedInitialResults",
        "-MaxSessionWallSeconds"
    )
    if (-not $requiredStartAvailableNow) {
        $expectedFlags.Add("-NotBeforeUtc")
    }
    $terminalArgumentIndex = $expectedFlags.Count * 2
    if ($arguments.Count -ne $terminalArgumentIndex + 1) {
        throw (
            "Scheduled controller must have $($expectedFlags.Count) exact " +
            "flag/value pairs and -Execute"
        )
    }
    for ($index = 0; $index -lt $expectedFlags.Count; $index++) {
        Assert-ExactString `
            $arguments[$index * 2] `
            $expectedFlags[$index] `
            "controller flag $index"
    }
    Assert-ExactString `
        $arguments[$terminalArgumentIndex] `
        "-Execute" `
        "controller terminal flag"
    Assert-ExactString $arguments[1] $release "controller release"
    Assert-ExactString `
        ([IO.Path]::GetFullPath($arguments[3])) `
        $checkpointPath `
        "controller checkpoint path" `
        -IgnoreCase
    Assert-ExactString `
        $arguments[5] `
        $checkpointSha256 `
        "controller checkpoint SHA-256"
    if ([int]$arguments[7] -ne $completedResults) {
        throw "Controller result count does not match the checkpoint"
    }
    $maxSessionWallSeconds = [int]$arguments[9]
    if ($maxSessionWallSeconds -lt 60 -or $maxSessionWallSeconds -gt 14400) {
        throw "Controller session duration is outside the guarded range"
    }
    $requiredSeconds = [int](
        Get-RequiredProperty $allowance "required_seconds" "allowance"
    )
    if ($requiredSeconds -ne $maxSessionWallSeconds + 300) {
        throw "Allowance duration does not match the controller service ceiling"
    }
    $launchMode = "immediate_full_fit"
    $notBefore = $observedAt.Add($immediateLaunchDelay)
    if (-not $requiredStartAvailableNow) {
        $launchMode = "legacy_allowance_boundary"
        $notBefore = [DateTimeOffset]::Parse(
            $arguments[11],
            [Globalization.CultureInfo]::InvariantCulture,
            (
                [Globalization.DateTimeStyles]::AssumeUniversal -bor
                [Globalization.DateTimeStyles]::AdjustToUniversal
            )
        )
        $boundaryGuardSeconds = ($notBefore - $projectedStart).TotalSeconds
        if ($boundaryGuardSeconds -lt 0 -or $boundaryGuardSeconds -gt 60) {
            throw "Scheduled boundary guard must be between zero and 60 seconds"
        }
    }
    if ($Register) {
        if ($notBefore -le [DateTimeOffset]::UtcNow) {
            throw "Cannot register a CASC resume task in the past"
        }
        if ($notBefore -gt [DateTimeOffset]::UtcNow.AddHours(24)) {
            throw "Cannot register a CASC resume task more than 24 hours ahead"
        }
    }

    $taskName = "Umlaut-CASC-{0}-Resume-{1}" -f @(
        $release.ToUpperInvariant(),
        $notBefore.ToUniversalTime().ToString("yyyyMMddTHHmmssZ")
    )
    $actionArguments = Get-ScheduledActionArguments `
        -PlanPath $canonicalPlan `
        -TaskName $taskName
    $principal = [Security.Principal.WindowsIdentity]::GetCurrent().Name
    $principalSid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value

    return [pscustomobject]@{
        repo_root = $repoRoot
        plan_path = $canonicalPlan
        plan_sha256 = (
            Get-FileHash -LiteralPath $canonicalPlan -Algorithm SHA256
        ).Hash.ToLowerInvariant()
        release = $release
        checkpoint_path = $checkpointPath
        checkpoint_sha256 = $checkpointSha256
        completed_results = $completedResults
        expected_results = $expectedResults
        max_session_wall_seconds = $maxSessionWallSeconds
        launch_mode = $launchMode
        not_before = $notBefore
        controller_path = $controllerPath
        controller_arguments = $arguments
        action_arguments = $actionArguments
        task_name = $taskName
        principal = $principal
        principal_sid = $principalSid
    }
}

function Get-ValidatedLaunchEnvelope {
    param(
        [Parameter(Mandatory = $true)]
        [string]$PlanPath,

        [Parameter(Mandatory = $true)]
        [string]$TaskName
    )

    $taskPattern = '^Umlaut-CASC-(J13|CASC2025)-Resume-(\d{8}T\d{6}Z)$'
    if ($TaskName -notmatch $taskPattern) {
        throw "Scheduled launch task name is not canonical: $TaskName"
    }
    $release = $Matches[1].ToLowerInvariant()
    $triggerText = $Matches[2]
    try {
        $notBefore = [DateTimeOffset]::ParseExact(
            $triggerText,
            "yyyyMMdd'T'HHmmss'Z'",
            [Globalization.CultureInfo]::InvariantCulture,
            (
                [Globalization.DateTimeStyles]::AssumeUniversal -bor
                [Globalization.DateTimeStyles]::AdjustToUniversal
            )
        )
    }
    catch {
        throw "Scheduled launch task timestamp is invalid: $triggerText"
    }

    $repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..\..")).Path
    $canonicalPlan = Get-CanonicalPath `
        -Path $PlanPath `
        -Description "resume plan" `
        -RepositoryRoot $repoRoot `
        -AllowMissing
    $principal = [Security.Principal.WindowsIdentity]::GetCurrent().Name
    $principalSid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value

    return [pscustomobject]@{
        repo_root = $repoRoot
        plan_path = $canonicalPlan
        release = $release
        not_before = $notBefore
        action_arguments = Get-ScheduledActionArguments `
            -PlanPath $canonicalPlan `
            -TaskName $TaskName
        task_name = $TaskName
        principal = $principal
        principal_sid = $principalSid
    }
}

function Assert-TaskMatches {
    param(
        [Parameter(Mandatory = $true)]
        [object]$ValidatedPlan,

        [bool]$ExpectedEnabled = $true
    )

    $task = Get-ScheduledTask `
        -TaskName $ValidatedPlan.task_name `
        -ErrorAction SilentlyContinue
    if ($null -eq $task) {
        throw "Scheduled CASC resume task is missing: $($ValidatedPlan.task_name)"
    }
    if (@($task.Actions).Count -ne 1 -or @($task.Triggers).Count -ne 1) {
        throw "Scheduled CASC resume task must have one action and one trigger"
    }
    $action = @($task.Actions)[0]
    Assert-ExactString `
        ([string]$action.Execute) `
        "powershell.exe" `
        "scheduled executable" `
        -IgnoreCase
    Assert-ExactString `
        ([string]$action.Arguments) `
        $ValidatedPlan.action_arguments `
        "scheduled arguments"
    Assert-ExactString `
        ([IO.Path]::GetFullPath([string]$action.WorkingDirectory)) `
        $ValidatedPlan.repo_root `
        "scheduled working directory" `
        -IgnoreCase
    Assert-ExactString `
        ([string]$task.Description) `
        "Guarded immutable $($ValidatedPlan.release) checkpoint resume for Umlaut" `
        "scheduled description"

    $trigger = @($task.Triggers)[0]
    $triggerTime = [DateTimeOffset]::Parse([string]$trigger.StartBoundary)
    if ($triggerTime.ToUniversalTime() -ne $ValidatedPlan.not_before) {
        throw "Scheduled trigger does not match the validated UTC boundary"
    }
    Assert-ExactString `
        ([string]$trigger.Repetition.Interval) `
        $retryIntervalIso8601 `
        "scheduled retry interval"
    Assert-ExactString `
        ([string]$trigger.Repetition.Duration) `
        $retryDurationIso8601 `
        "scheduled retry duration"
    $taskPrincipalSid = Get-AccountSid -Account ([string]$task.Principal.UserId)
    Assert-ExactString `
        $taskPrincipalSid `
        $ValidatedPlan.principal_sid `
        "scheduled principal SID" `
        -IgnoreCase
    Assert-ExactString `
        ([string]$task.Principal.LogonType) `
        "Interactive" `
        "scheduled logon type"
    Assert-ExactString `
        ([string]$task.Principal.RunLevel) `
        "Limited" `
        "scheduled run level"
    if (
        -not [bool]$task.Settings.WakeToRun -or
        -not [bool]$task.Settings.RunOnlyIfNetworkAvailable -or
        -not [bool]$task.Settings.StartWhenAvailable -or
        [string]$task.Settings.MultipleInstances -ne "IgnoreNew" -or
        [string]$task.Settings.ExecutionTimeLimit -ne "PT${ExecutionTimeHours}H" -or
        [bool]$task.Settings.DisallowStartIfOnBatteries -or
        [bool]$task.Settings.StopIfGoingOnBatteries
    ) {
        throw "Scheduled CASC resume settings do not match the guarded policy"
    }
    if ([bool]$task.Settings.Enabled -ne $ExpectedEnabled) {
        throw "Scheduled CASC resume task enabled state does not match policy"
    }
}

function Invoke-LoggedController {
    param(
        [Parameter(Mandatory = $true)]
        [string]$ControllerPath,

        [Parameter(Mandatory = $true)]
        [hashtable]$ControllerParameters,

        [Parameter(Mandatory = $true)]
        [string]$LogPath
    )

    $writeLog = {
        param([Parameter(Mandatory = $true)][string]$Message)

        $timestamp = [DateTimeOffset]::UtcNow.ToString("O")
        [IO.File]::AppendAllText($LogPath, "$timestamp $Message`r`n")
    }
    & $writeLog "controller_invocation_started path=$ControllerPath"
    try {
        & $ControllerPath @ControllerParameters *>&1 | ForEach-Object {
            $text = [string]$_
            if (-not [string]::IsNullOrEmpty($text)) {
                & $writeLog "controller_output $text"
            }
        }
        & $writeLog "controller_invocation_completed"
    }
    catch {
        & $writeLog "controller_invocation_failed error=$($_.Exception.Message)"
        throw
    }
}

if ($Launch) {
    $launchEnvelope = Get-ValidatedLaunchEnvelope `
        -PlanPath $Plan `
        -TaskName $ScheduledTaskName
    $artifactRoot = Join-Path $launchEnvelope.repo_root ".artifacts\casc-benchmark"
    [IO.Directory]::CreateDirectory($artifactRoot) | Out-Null
    $launchId = (
        [DateTimeOffset]::UtcNow.ToString("yyyyMMddTHHmmssZ") + "-$PID"
    )
    $launchLog = Join-Path (
        $artifactRoot
    ) "scheduled-launch-$($launchEnvelope.release)-$launchId.log"
    if (Test-Path -LiteralPath $launchLog) {
        throw "Refusing to overwrite scheduled-launch log: $launchLog"
    }
    $startedAt = [DateTimeOffset]::UtcNow.ToString("O")
    [IO.File]::WriteAllText(
        $launchLog,
        (
            "$startedAt task_launch_started " +
            "task=$($launchEnvelope.task_name) " +
            "plan=$($launchEnvelope.plan_path)`r`n"
        )
    )
    try {
        Assert-TaskMatches -ValidatedPlan $launchEnvelope
        Disable-ScheduledTask -TaskName $launchEnvelope.task_name | Out-Null
        Assert-TaskMatches `
            -ValidatedPlan $launchEnvelope `
            -ExpectedEnabled $false
        [IO.File]::AppendAllText(
            $launchLog,
            "$([DateTimeOffset]::UtcNow.ToString('O')) task_disabled`r`n"
        )

        $validatedPlan = Get-ValidatedPlan -PlanPath $Plan
        Assert-ExactString `
            $validatedPlan.task_name `
            $launchEnvelope.task_name `
            "validated launch task name"
        Assert-ExactString `
            $validatedPlan.plan_path `
            $launchEnvelope.plan_path `
            "validated launch plan path" `
            -IgnoreCase
        Assert-TaskMatches `
            -ValidatedPlan $validatedPlan `
            -ExpectedEnabled $false
        [IO.File]::AppendAllText(
            $launchLog,
            (
                "$([DateTimeOffset]::UtcNow.ToString('O')) plan_validated " +
                "sha256=$($validatedPlan.plan_sha256)`r`n"
            )
        )

        $controllerArguments = @($validatedPlan.controller_arguments)
        $controllerParameters = @{}
        for (
            $index = 0;
            $index -lt $controllerArguments.Count - 1;
            $index += 2
        ) {
            $parameterName = $controllerArguments[$index].Substring(1)
            $controllerParameters[$parameterName] = $controllerArguments[$index + 1]
        }
        $controllerParameters["Execute"] = $true
        Invoke-LoggedController `
            -ControllerPath $validatedPlan.controller_path `
            -ControllerParameters $controllerParameters `
            -LogPath $launchLog
        [IO.File]::AppendAllText(
            $launchLog,
            "$([DateTimeOffset]::UtcNow.ToString('O')) task_launch_completed`r`n"
        )
        exit 0
    }
    catch {
        [IO.File]::AppendAllText(
            $launchLog,
            (
                "$([DateTimeOffset]::UtcNow.ToString('O')) " +
                "task_launch_failed error=$($_.Exception.Message)`r`n"
            )
        )
        throw
    }
}

$validatedPlan = Get-ValidatedPlan -PlanPath $Plan

if ($Register) {
    if ($null -ne (Get-ScheduledTask `
        -TaskName $validatedPlan.task_name `
        -ErrorAction SilentlyContinue
    )) {
        throw "Refusing to replace scheduled task: $($validatedPlan.task_name)"
    }
    $action = New-ScheduledTaskAction `
        -Execute "powershell.exe" `
        -Argument $validatedPlan.action_arguments `
        -WorkingDirectory $validatedPlan.repo_root
    $trigger = New-ScheduledTaskTrigger `
        -Once `
        -At $validatedPlan.not_before.ToLocalTime().DateTime `
        -RepetitionInterval $retryInterval `
        -RepetitionDuration $retryDuration
    $settings = New-ScheduledTaskSettingsSet `
        -ExecutionTimeLimit (New-TimeSpan -Hours $ExecutionTimeHours) `
        -StartWhenAvailable `
        -WakeToRun `
        -RunOnlyIfNetworkAvailable `
        -AllowStartIfOnBatteries `
        -DontStopIfGoingOnBatteries `
        -MultipleInstances IgnoreNew
    $principal = New-ScheduledTaskPrincipal `
        -UserId $validatedPlan.principal `
        -LogonType Interactive `
        -RunLevel Limited
    Register-ScheduledTask `
        -TaskName $validatedPlan.task_name `
        -Description "Guarded immutable $($validatedPlan.release) checkpoint resume for Umlaut" `
        -Action $action `
        -Trigger $trigger `
        -Settings $settings `
        -Principal $principal | Out-Null
    Assert-TaskMatches -ValidatedPlan $validatedPlan
    Get-TaskEvidence -ValidatedPlan $validatedPlan -Status "registered" |
        ConvertTo-Json -Depth 6
    exit 0
}

if ($Audit) {
    Assert-TaskMatches -ValidatedPlan $validatedPlan
    Get-TaskEvidence -ValidatedPlan $validatedPlan -Status "audit_passed" |
        ConvertTo-Json -Depth 6
    exit 0
}

Get-TaskEvidence -ValidatedPlan $validatedPlan -Status "ready_to_register" |
    ConvertTo-Json -Depth 6
