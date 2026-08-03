[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [ValidateSet(
        "init",
        "init-reaper",
        "check",
        "up",
        "recover",
        "sync",
        "upload",
        "download",
        "exec",
        "refresh-ip",
        "run",
        "down",
        "status",
        "allowance",
        "reap",
        "gc"
    )]
    [string]$Command = "status",

    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$RunnerArguments
)

$ErrorActionPreference = "Stop"
$secretPath = Join-Path $env:LOCALAPPDATA "E-Rust-Port\linode-token.dpapi"
$runnerStateRoot = Join-Path $env:LOCALAPPDATA "E-Rust-Port\linode-runner"
$parkedStateRoot = Join-Path $runnerStateRoot "parked"
$reaperConfigPath = Join-Path $runnerStateRoot "reaper.json"
$reaperSecretPath = Join-Path $env:LOCALAPPDATA "E-Rust-Port\linode-reaper-token.dpapi"
$controller = Join-Path $PSScriptRoot "tools\linode-runner\linode_runner.py"
$taskPrefix = "Umlaut-Linode-Reaper-"

if (-not (Test-Path -LiteralPath $controller -PathType Leaf)) {
    throw "Linode controller is missing: $controller"
}

$python = Join-Path $PSScriptRoot ".venv\Scripts\python.exe"
if (-not (Test-Path -LiteralPath $python -PathType Leaf)) {
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
else {
    $pythonPrefix = @("-u")
}

function Initialize-RestrictedReaper {
    if (
        $RunnerArguments.Count -ne 2 -or
        $RunnerArguments[0] -ne "--username"
    ) {
        throw "Usage: .\linode-runner.ps1 init-reaper --username NAME"
    }
    $username = $RunnerArguments[1]
    if ($username -notmatch '^[A-Za-z0-9][A-Za-z0-9_-]{1,30}[A-Za-z0-9]$') {
        throw "Invalid restricted Linode username: $username"
    }
    New-Item -ItemType Directory -Path $runnerStateRoot -Force | Out-Null
    $secureReaperToken = Read-Host (
        "Restricted reaper PAT (linodes:read_write and firewall:read_write)"
    ) -AsSecureString
    $secureReaperToken |
        ConvertFrom-SecureString |
        Set-Content -LiteralPath $reaperSecretPath -Encoding UTF8
    $reaperConfig = @{
        schema_version = 1
        username = $username
    } | ConvertTo-Json
    [IO.File]::WriteAllText(
        $reaperConfigPath,
        $reaperConfig,
        [Text.UTF8Encoding]::new($false)
    )
    Write-Host "Restricted reaper credentials saved with user-scoped DPAPI."
    Write-Host "Run '.\linode-runner.ps1 check' to validate them."
}

function Sync-LocalReaperTasks {
    $desiredTasks = [Collections.Generic.HashSet[string]]::new(
        [StringComparer]::OrdinalIgnoreCase
    )
    $stateFiles = @()
    if (Test-Path -LiteralPath $parkedStateRoot -PathType Container) {
        $stateFiles = @(
            Get-ChildItem -LiteralPath $parkedStateRoot -Filter "*.json" -File
        )
    }
    $powershellExecutable = (Get-Command powershell.exe -ErrorAction Stop).Source
    $currentUser = [Security.Principal.WindowsIdentity]::GetCurrent().Name
    $principal = New-ScheduledTaskPrincipal `
        -UserId $currentUser `
        -LogonType Interactive `
        -RunLevel Limited
    foreach ($stateFile in $stateFiles) {
        $state = Get-Content -LiteralPath $stateFile.FullName -Raw |
            ConvertFrom-Json
        if ($state.lifecycle -ne "parked") {
            throw "Invalid parked lifecycle in $($stateFile.FullName)"
        }
        $linodeId = [int64]$state.linode_id
        $leaseId = [string]$state.lease_id
        if ($linodeId -le 0 -or $leaseId -notmatch '^[0-9a-f]{32}$') {
            throw "Invalid parked identity in $($stateFile.FullName)"
        }
        $taskName = "$taskPrefix$linodeId"
        $null = $desiredTasks.Add($taskName)
        $deadline = [DateTimeOffset]::Parse(
            [string]$state.delete_at,
            [Globalization.CultureInfo]::InvariantCulture,
            [Globalization.DateTimeStyles]::AssumeUniversal
        )
        $triggerAt = $deadline.LocalDateTime
        if ($triggerAt -le [DateTime]::Now) {
            $triggerAt = [DateTime]::Now.AddSeconds(5)
        }
        $actionArguments = (
            '-NoProfile -NonInteractive -ExecutionPolicy Bypass ' +
            '-File "{0}" reap --linode-id {1} --lease-id {2}'
        ) -f $PSCommandPath, $linodeId, $leaseId
        $action = New-ScheduledTaskAction `
            -Execute $powershellExecutable `
            -Argument $actionArguments
        $trigger = New-ScheduledTaskTrigger -Once -At $triggerAt
        $settings = New-ScheduledTaskSettingsSet `
            -StartWhenAvailable `
            -WakeToRun `
            -RunOnlyIfNetworkAvailable `
            -RestartCount 10 `
            -RestartInterval (New-TimeSpan -Minutes 2) `
            -ExecutionTimeLimit (New-TimeSpan -Hours 2)
        Register-ScheduledTask `
            -TaskName $taskName `
            -Description "Delete parked Umlaut Linode $linodeId at its billing cutoff" `
            -Action $action `
            -Trigger $trigger `
            -Settings $settings `
            -Principal $principal `
            -Force | Out-Null
    }
    $existingTasks = @(
        Get-ScheduledTask -ErrorAction Stop |
            Where-Object { $_.TaskName.StartsWith($taskPrefix) }
    )
    foreach ($task in $existingTasks) {
        if (-not $desiredTasks.Contains($task.TaskName)) {
            Unregister-ScheduledTask -TaskName $task.TaskName -Confirm:$false
        }
    }
}

if ($Command -eq "init") {
    $previousErrorActionPreference = $ErrorActionPreference
    try {
        # Windows PowerShell promotes native stderr to NativeCommandError
        # records.  The Python controller's exit code is authoritative; tools
        # such as systemctl may emit harmless progress on stderr while still
        # succeeding.
        $ErrorActionPreference = "Continue"
        & $python @pythonPrefix $controller $Command @RunnerArguments
        $runnerExitCode = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }
    exit $runnerExitCode
}

if ($Command -eq "init-reaper") {
    Initialize-RestrictedReaper
    exit 0
}

if (-not (Test-Path -LiteralPath $secretPath -PathType Leaf)) {
    throw "Encrypted Linode token is missing: $secretPath"
}

$pythonRunnerArguments = $RunnerArguments
if ($Command -eq "exec") {
    $remoteArguments = @($RunnerArguments)
    if ($remoteArguments.Count -gt 0 -and $remoteArguments[0] -eq "--") {
        $remoteArguments = @($remoteArguments | Select-Object -Skip 1)
    }
    if ($remoteArguments.Count -eq 0) {
        throw "Provide a remote command after 'exec --'"
    }
    $remoteCommand = $remoteArguments -join " "
    $encodedCommand = [Convert]::ToBase64String(
        [Text.Encoding]::UTF8.GetBytes($remoteCommand)
    )
    # Windows PowerShell's native argument marshalling removes embedded quote
    # characters.  Encode the complete command before crossing that boundary.
    $pythonRunnerArguments = @("--", "--encoded-command", $encodedCommand)
}

$tokenPointer = [IntPtr]::Zero
$reaperTokenPointer = [IntPtr]::Zero
$runnerExitCode = 1
try {
    $secureToken = Get-Content -LiteralPath $secretPath | ConvertTo-SecureString
    $tokenPointer = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secureToken)
    $env:LINODE_TOKEN = [Runtime.InteropServices.Marshal]::PtrToStringBSTR(
        $tokenPointer
    )
    if (Test-Path -LiteralPath $reaperSecretPath -PathType Leaf) {
        $secureReaperToken = Get-Content -LiteralPath $reaperSecretPath |
            ConvertTo-SecureString
        $reaperTokenPointer = [Runtime.InteropServices.Marshal]::SecureStringToBSTR(
            $secureReaperToken
        )
        $env:LINODE_REAPER_TOKEN = [Runtime.InteropServices.Marshal]::PtrToStringBSTR(
            $reaperTokenPointer
        )
    }
    $previousErrorActionPreference = $ErrorActionPreference
    try {
        # Preserve native stderr without letting Windows PowerShell terminate
        # this wrapper before the controller can report its real exit code.
        $ErrorActionPreference = "Continue"
        & $python @pythonPrefix $controller $Command @pythonRunnerArguments
        $runnerExitCode = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }
}
finally {
    Remove-Item Env:LINODE_TOKEN -ErrorAction SilentlyContinue
    Remove-Item Env:LINODE_REAPER_TOKEN -ErrorAction SilentlyContinue
    if ($tokenPointer -ne [IntPtr]::Zero) {
        [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($tokenPointer)
    }
    if ($reaperTokenPointer -ne [IntPtr]::Zero) {
        [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($reaperTokenPointer)
    }
}

if ($Command -in @("up", "down", "run", "reap")) {
    try {
        Sync-LocalReaperTasks
    }
    catch {
        [Console]::Error.WriteLine(
            "URGENT: could not synchronize local reaper tasks: $($_.Exception.Message)"
        )
        if ($runnerExitCode -eq 0) {
            $runnerExitCode = 1
        }
    }
}

exit $runnerExitCode
