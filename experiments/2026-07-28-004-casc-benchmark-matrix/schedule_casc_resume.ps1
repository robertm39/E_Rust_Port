[CmdletBinding(DefaultParameterSetName = "Inspect")]
param(
    [Parameter(Mandatory = $true)]
    [string]$Plan,

    [Parameter(Mandatory = $true, ParameterSetName = "Register")]
    [switch]$Register,

    [Parameter(Mandatory = $true, ParameterSetName = "Audit")]
    [switch]$Audit,

    [ValidateRange(1, 24)]
    [int]$ExecutionTimeHours = 8
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

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
        [string]$RepositoryRoot
    )

    $candidate = [IO.Path]::GetFullPath($Path)
    if (-not (Test-Path -LiteralPath $candidate -PathType Leaf)) {
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
            trigger_utc = $ValidatedPlan.not_before.ToString("O")
            execute = "powershell.exe"
            arguments = $ValidatedPlan.action_arguments
            working_directory = $ValidatedPlan.repo_root
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
    if ([bool](
        Get-RequiredProperty `
            $allowance `
            "required_start_available_now" `
            "allowance"
    )) {
        throw "Immediate-start plans must be executed directly, not scheduled"
    }
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
    $expectedFlags = @(
        "-Release",
        "-CheckpointArchive",
        "-CheckpointSha256",
        "-ExpectedInitialResults",
        "-MaxSessionWallSeconds",
        "-NotBeforeUtc"
    )
    if ($arguments.Count -ne 13) {
        throw "Scheduled controller must have six exact flag/value pairs and -Execute"
    }
    for ($index = 0; $index -lt $expectedFlags.Count; $index++) {
        Assert-ExactString `
            $arguments[$index * 2] `
            $expectedFlags[$index] `
            "controller flag $index"
    }
    Assert-ExactString $arguments[12] "-Execute" "controller terminal flag"
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
    if ($Register) {
        if ($notBefore -le [DateTimeOffset]::UtcNow) {
            throw "Cannot register a CASC resume task in the past"
        }
        if ($notBefore -gt [DateTimeOffset]::UtcNow.AddHours(24)) {
            throw "Cannot register a CASC resume task more than 24 hours ahead"
        }
    }

    $formattedArguments = @(
        for ($index = 0; $index -lt $arguments.Count; $index++) {
            Format-ActionArgument -Value $arguments[$index]
        }
    )
    $actionArguments = (
        '-NoProfile -NonInteractive -ExecutionPolicy Bypass -File {0} {1}' -f
        (Format-ActionArgument -Value $controllerPath -AlwaysQuote),
        ($formattedArguments -join ' ')
    )
    $taskName = "Umlaut-CASC-{0}-Resume-{1}" -f @(
        $release.ToUpperInvariant(),
        $notBefore.ToUniversalTime().ToString("yyyyMMddTHHmmssZ")
    )
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
        not_before = $notBefore
        controller_path = $controllerPath
        controller_arguments = $arguments
        action_arguments = $actionArguments
        task_name = $taskName
        principal = $principal
        principal_sid = $principalSid
    }
}

function Assert-TaskMatches {
    param(
        [Parameter(Mandatory = $true)]
        [object]$ValidatedPlan
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

    $trigger = @($task.Triggers)[0]
    $triggerTime = [DateTimeOffset]::Parse([string]$trigger.StartBoundary)
    if ($triggerTime.ToUniversalTime() -ne $ValidatedPlan.not_before) {
        throw "Scheduled trigger does not match the validated UTC boundary"
    }
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
    if (-not [bool]$task.Settings.Enabled) {
        throw "Scheduled CASC resume task is disabled"
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
        -At $validatedPlan.not_before.ToLocalTime().DateTime
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
