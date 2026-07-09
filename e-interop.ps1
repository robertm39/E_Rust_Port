[CmdletBinding()]
param(
    [Parameter(Mandatory = $true, Position = 0)]
    [ValidateSet('setup', 'build-reference', 'compare', 'compare-tools', 'benchmark')]
    [string]$Command,

    [string]$RustExe,
    [string]$RustBinDir,
    [string[]]$Tool,
    [string]$Corpus,
    [ValidateRange(1, 100)]
    [int]$Runs = 5,
    [ValidateRange(1, 86400)]
    [int]$TimeoutSeconds = 60,
    [ValidateRange(1, 1048576)]
    [int]$MemoryLimitMb = 2048,
    [ValidateRange(1.0, 100.0)]
    [double]$RegressionThreshold = 1.10,
    [string]$Distro = 'Ubuntu-24.04',
    [switch]$SelfTest
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$RepoRoot = Split-Path -Parent $PSCommandPath
$Driver = Join-Path $RepoRoot 'tools\e-interop\e_interop.py'

function Get-WslDistros {
    $oldPreference = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $output = & wsl.exe --list --quiet 2>$null
        if ($LASTEXITCODE -ne 0) {
            return @()
        }
        return @($output | ForEach-Object { ($_ -replace "`0", '').Trim() } | Where-Object { $_ })
    }
    finally {
        $ErrorActionPreference = $oldPreference
    }
}

function Assert-DistroInstalled {
    $distros = @(Get-WslDistros)
    if ($Distro -notin $distros) {
        throw @"
WSL 2 is enabled, but '$Distro' is not installed.
Run this once from an elevated PowerShell prompt, complete the Linux user setup,
then rerun this command:

    wsl --install -d $Distro
"@
    }

    $verboseList = ((& wsl.exe --list --verbose) -join "`n") -replace "`0", ''
    $match = [regex]::Match(
        $verboseList,
        "(?m)^\s*\*?\s*$([regex]::Escape($Distro))\s+\S+\s+(\d+)\s*$"
    )
    if (-not $match.Success) {
        throw "Could not determine the WSL version for '$Distro'."
    }
    if ($match.Groups[1].Value -ne '2') {
        throw "'$Distro' is not using WSL 2. Run: wsl --set-version $Distro 2"
    }
}

function Convert-ToWslPath([string]$Path) {
    $absolute = [System.IO.Path]::GetFullPath($Path)
    if ($absolute -match '^([A-Za-z]):[\\/](.*)$') {
        $drive = $Matches[1].ToLowerInvariant()
        $tail = ($Matches[2] -replace '\\', '/')
        return "/mnt/$drive/$tail"
    }

    $converted = & wsl.exe -d $Distro -- wslpath -a -- $absolute
    if ($LASTEXITCODE -ne 0) {
        throw "Could not convert '$absolute' to a WSL path."
    }
    return ($converted -replace "`0", '').Trim()
}

function Invoke-WslPython([string[]]$Arguments) {
    $driverWsl = Convert-ToWslPath $Driver
    & wsl.exe -d $Distro -- python3 $driverWsl @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "e-interop $Command failed with exit code $LASTEXITCODE."
    }
}

if ($Command -eq 'setup') {
    Assert-DistroInstalled
    Write-Host "Installing E reference-build dependencies in $Distro..."
    & wsl.exe -d $Distro -- bash -lc 'set -euo pipefail; sudo apt-get update; sudo DEBIAN_FRONTEND=noninteractive apt-get install -y build-essential gawk git python3 time'
    if ($LASTEXITCODE -ne 0) {
        throw "Dependency installation failed with exit code $LASTEXITCODE."
    }
    Invoke-WslPython @('doctor')
    Write-Host 'WSL setup is ready.'
    exit 0
}

Assert-DistroInstalled
$repoRootWsl = Convert-ToWslPath $RepoRoot
$common = @('--repo-root', $repoRootWsl)

switch ($Command) {
    'build-reference' {
        Invoke-WslPython (@('build-reference') + $common)
    }
    'compare' {
        $arguments = @(
            'compare'
        ) + $common + @(
            '--timeout', $TimeoutSeconds.ToString([Globalization.CultureInfo]::InvariantCulture),
            '--memory-limit-mb', $MemoryLimitMb.ToString([Globalization.CultureInfo]::InvariantCulture)
        )

        if ($Corpus) {
            $arguments += @('--corpus', (Convert-ToWslPath (Resolve-Path $Corpus)))
        }

        if ($SelfTest) {
            $arguments += '--self-test'
        }
        else {
            if (-not $RustExe) {
                throw 'compare requires -RustExe <path>, unless -SelfTest is used.'
            }
            $resolvedRustExe = Resolve-Path $RustExe
            if ([System.IO.Path]::GetExtension($resolvedRustExe) -ne '.exe') {
                throw '-RustExe must identify a native Windows .exe.'
            }
            $arguments += @('--rust-windows', (Convert-ToWslPath $resolvedRustExe))
        }

        Invoke-WslPython $arguments
    }
    'compare-tools' {
        $arguments = @(
            'compare-tools'
        ) + $common + @(
            '--timeout', $TimeoutSeconds.ToString([Globalization.CultureInfo]::InvariantCulture)
        )

        foreach ($toolName in @($Tool)) {
            if ($toolName) {
                $arguments += @('--tool', $toolName)
            }
        }

        if ($SelfTest) {
            $arguments += '--self-test'
        }
        else {
            if (-not $RustBinDir) {
                throw 'compare-tools requires -RustBinDir <path>, unless -SelfTest is used.'
            }
            $resolvedRustBinDir = Resolve-Path $RustBinDir
            $arguments += @('--rust-windows-bin-dir', (Convert-ToWslPath $resolvedRustBinDir))
        }

        Invoke-WslPython $arguments
    }
    'benchmark' {
        $arguments = @(
            'benchmark'
        ) + $common + @(
            '--runs', $Runs.ToString([Globalization.CultureInfo]::InvariantCulture),
            '--timeout', $TimeoutSeconds.ToString([Globalization.CultureInfo]::InvariantCulture),
            '--memory-limit-mb', $MemoryLimitMb.ToString([Globalization.CultureInfo]::InvariantCulture),
            '--regression-threshold', $RegressionThreshold.ToString([Globalization.CultureInfo]::InvariantCulture)
        )
        if ($Corpus) {
            $arguments += @('--corpus', (Convert-ToWslPath (Resolve-Path $Corpus)))
        }
        Invoke-WslPython $arguments
    }
}
