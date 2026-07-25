[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [ValidateSet(
        "init",
        "check",
        "up",
        "sync",
        "exec",
        "refresh-ip",
        "run",
        "down",
        "status",
        "gc"
    )]
    [string]$Command = "status",

    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$RunnerArguments
)

$ErrorActionPreference = "Stop"
$secretPath = Join-Path $env:LOCALAPPDATA "E-Rust-Port\linode-token.dpapi"
$controller = Join-Path $PSScriptRoot "tools\linode-runner\linode_runner.py"

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

if ($Command -eq "init") {
    & $python @pythonPrefix $controller $Command @RunnerArguments
    exit $LASTEXITCODE
}

if (-not (Test-Path -LiteralPath $secretPath -PathType Leaf)) {
    throw "Encrypted Linode token is missing: $secretPath"
}

$secureToken = Get-Content -LiteralPath $secretPath | ConvertTo-SecureString
$tokenPointer = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secureToken)
try {
    $env:LINODE_TOKEN = [Runtime.InteropServices.Marshal]::PtrToStringBSTR(
        $tokenPointer
    )
    & $python @pythonPrefix $controller $Command @RunnerArguments
    $runnerExitCode = $LASTEXITCODE
}
finally {
    Remove-Item Env:LINODE_TOKEN -ErrorAction SilentlyContinue
    if ($tokenPointer -ne [IntPtr]::Zero) {
        [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($tokenPointer)
    }
}

exit $runnerExitCode
