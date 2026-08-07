[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string] $Archive,

    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[0-9a-fA-F]{64}$')]
    [string] $ArchiveSha256,

    [Parameter(Mandatory = $true)]
    [string] $RunName,

    [Parameter(Mandatory = $true)]
    [ValidateRange(0, [int]::MaxValue)]
    [int] $ExpectedResults,

    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[0-9a-fA-F]{64}$')]
    [string] $ContractId,

    [Parameter(Mandatory = $true)]
    [ValidateRange(0, [int]::MaxValue)]
    [int] $ExpectedNewResults,

    [Parameter(Mandatory = $true)]
    [ValidateRange(0, [int]::MaxValue)]
    [int] $ExpectedResumedResults,

    [Parameter(Mandatory = $true)]
    [ValidateRange(0, [long]::MaxValue)]
    [long] $ExpectedContractSequence,

    [Parameter(Mandatory = $true)]
    [ValidateRange(0, [long]::MaxValue)]
    [long] $ExpectedSuccessSequence,

    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[0-9a-fA-F]{32}$')]
    [string] $ExpectedBootId,

    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[0-9a-fA-F]{32}$')]
    [string] $ExpectedInvocationId,

    [Parameter(Mandatory = $true)]
    [string] $ExtractionRoot,

    [string] $Output
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-NormalizedSha256 {
    param(
        [Parameter(Mandatory = $true)]
        [string] $LiteralPath
    )

    return (Get-FileHash -Algorithm SHA256 -LiteralPath $LiteralPath).Hash.ToLowerInvariant()
}

function Get-RunRelativePath {
    param(
        [Parameter(Mandatory = $true)]
        [string] $RunRoot,

        [Parameter(Mandatory = $true)]
        [string] $LiteralPath
    )

    $prefix = $RunRoot.TrimEnd('\') + '\'
    if (-not $LiteralPath.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Path escapes the run root: $LiteralPath"
    }
    return $LiteralPath.Substring($prefix.Length).Replace('\', '/')
}

$resolvedArchive = (Resolve-Path -LiteralPath $Archive).Path
$actualArchiveSha256 = Get-NormalizedSha256 -LiteralPath $resolvedArchive
if ($actualArchiveSha256 -ne $ArchiveSha256.ToLowerInvariant()) {
    throw "Archive SHA-256 mismatch: expected $ArchiveSha256, got $actualArchiveSha256"
}

$resolvedExtractionRoot = [IO.Path]::GetFullPath($ExtractionRoot)
if (Test-Path -LiteralPath $resolvedExtractionRoot) {
    throw "Extraction root already exists: $resolvedExtractionRoot"
}
[void] (New-Item -ItemType Directory -Path $resolvedExtractionRoot)

$outerMembers = @(tar -tzf $resolvedArchive)
if ($LASTEXITCODE -ne 0) {
    throw "Unable to list archive: $resolvedArchive"
}
foreach ($member in $outerMembers) {
    $normalized = $member.Replace('\', '/')
    if (
        $normalized.StartsWith('/') -or
        $normalized -match '^[A-Za-z]:' -or
        @($normalized.Split('/')) -contains '..'
    ) {
        throw "Unsafe outer archive member: $member"
    }
}

tar -xzf $resolvedArchive -C $resolvedExtractionRoot
if ($LASTEXITCODE -ne 0) {
    throw "Unable to extract archive: $resolvedArchive"
}

$outerRoots = @(Get-ChildItem -LiteralPath $resolvedExtractionRoot -Directory)
if ($outerRoots.Count -ne 1) {
    throw "Expected one outer archive root, got $($outerRoots.Count)"
}
$outerRoot = $outerRoots[0].FullName
$sha256SumsPath = Join-Path $outerRoot 'SHA256SUMS'
$sha256SumLines = @(Get-Content -LiteralPath $sha256SumsPath)
$outerHashesVerified = 0
foreach ($line in $sha256SumLines) {
    if ($line -notmatch '^([0-9a-f]{64})  (.+)$') {
        throw "Malformed SHA256SUMS line: $line"
    }
    $memberPath = Join-Path $outerRoot $Matches[2]
    $actual = Get-NormalizedSha256 -LiteralPath $memberPath
    if ($actual -ne $Matches[1]) {
        throw "Outer member SHA-256 mismatch: $($Matches[2])"
    }
    $outerHashesVerified++
}

$outerRegularFiles = @(Get-ChildItem -LiteralPath $outerRoot -File).Count
if ($outerRegularFiles -ne $outerHashesVerified + 1) {
    throw (
        "Outer regular-file count does not equal SHA256SUMS entries plus SHA256SUMS: " +
        "$outerRegularFiles versus $outerHashesVerified"
    )
}

$innerArchive = Join-Path $outerRoot 'casc-runs.tar.gz'
$innerRoot = Join-Path $resolvedExtractionRoot 'inner'
[void] (New-Item -ItemType Directory -Path $innerRoot)
$innerMembers = @(tar -tzf $innerArchive)
if ($LASTEXITCODE -ne 0) {
    throw "Unable to list inner archive: $innerArchive"
}
foreach ($member in $innerMembers) {
    $normalized = $member.Replace('\', '/')
    if (
        $normalized.StartsWith('/') -or
        $normalized -match '^[A-Za-z]:' -or
        @($normalized.Split('/')) -contains '..'
    ) {
        throw "Unsafe inner archive member: $member"
    }
}
tar -xzf $innerArchive -C $innerRoot
if ($LASTEXITCODE -ne 0) {
    throw "Unable to extract inner archive: $innerArchive"
}

$runRoot = Join-Path $innerRoot "casc-runs\$RunName"
if (-not (Test-Path -LiteralPath $runRoot -PathType Container)) {
    throw "Run root is missing: $runRoot"
}

$inventory = @(Get-Content -LiteralPath (Join-Path $outerRoot 'result-files.txt'))
if ($inventory.Count -ne $ExpectedResults) {
    throw "Expected $ExpectedResults inventory paths, got $($inventory.Count)"
}
$sortedInventory = @($inventory | Sort-Object)
if (@(Compare-Object -ReferenceObject $inventory -DifferenceObject $sortedInventory).Count -ne 0) {
    throw 'Outer result inventory is not sorted'
}
$uniqueInventory = @($inventory | Sort-Object -Unique)
if ($uniqueInventory.Count -ne $ExpectedResults) {
    throw "Outer result inventory is not unique: $($uniqueInventory.Count)"
}

$jsonFiles = @(
    Get-ChildItem -LiteralPath (Join-Path $runRoot 'results') -File -Filter '*.json' -Recurse
)
$actualInventory = @(
    $jsonFiles |
        ForEach-Object {
            "/opt/e-rust-port/casc-runs/$RunName/" +
                (Get-RunRelativePath -RunRoot $runRoot -LiteralPath $_.FullName)
        } |
        Sort-Object
)
if (@(Compare-Object -ReferenceObject $inventory -DifferenceObject $actualInventory).Count -ne 0) {
    throw 'Outer result inventory differs from inner result JSON paths'
}

$coordinates = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
$referencedStreams = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
$missingStreams = 0
$mismatchedStreams = 0
$orphanCleanupTrue = 0
foreach ($file in $jsonFiles) {
    $record = Get-Content -Raw -LiteralPath $file.FullName | ConvertFrom-Json
    if (-not $coordinates.Add("$($record.solver)|$($record.problem_id)")) {
        throw "Duplicate solver/problem coordinate in $($file.FullName)"
    }
    if ($record.orphan_cleanup_required -eq $true) {
        $orphanCleanupTrue++
    }
    foreach ($kind in @('stdout', 'stderr')) {
        $pathProperty = "${kind}_path"
        $hashProperty = "${kind}_sha256"
        $relativePath = [string] $record.$pathProperty
        if (-not $referencedStreams.Add($relativePath)) {
            throw "Duplicate referenced stream path: $relativePath"
        }
        $streamPath = Join-Path $runRoot $relativePath
        if (-not (Test-Path -LiteralPath $streamPath -PathType Leaf)) {
            $missingStreams++
            continue
        }
        $streamSha256 = Get-NormalizedSha256 -LiteralPath $streamPath
        if ($streamSha256 -ne [string] $record.$hashProperty) {
            $mismatchedStreams++
        }
    }
}

if ($coordinates.Count -ne $ExpectedResults) {
    throw "Expected $ExpectedResults unique coordinates, got $($coordinates.Count)"
}
$expectedStreams = 2 * $ExpectedResults
if ($referencedStreams.Count -ne $expectedStreams) {
    throw "Expected $expectedStreams referenced streams, got $($referencedStreams.Count)"
}
if ($missingStreams -ne 0 -or $mismatchedStreams -ne 0) {
    throw "Stream audit failed: missing=$missingStreams mismatched=$mismatchedStreams"
}

$actualStreams = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
Get-ChildItem -LiteralPath (Join-Path $runRoot 'results') -File -Recurse |
    Where-Object { $_.Extension -in @('.stdout', '.stderr') } |
    ForEach-Object {
        [void] $actualStreams.Add(
            (Get-RunRelativePath -RunRoot $runRoot -LiteralPath $_.FullName)
        )
    }
$orphanStreams = 0
foreach ($stream in $actualStreams) {
    if (-not $referencedStreams.Contains($stream)) {
        $orphanStreams++
    }
}
if ($actualStreams.Count -ne $expectedStreams -or $orphanStreams -ne 0) {
    throw "Stream inventory failed: actual=$($actualStreams.Count) orphan=$orphanStreams"
}

$sessionRecords = @(
    Get-ChildItem -LiteralPath (Join-Path $runRoot 'sessions') -File -Filter '*.json'
).Count
$journal = @(
    Get-Content -LiteralPath (Join-Path $outerRoot 'service-journal.jsonl') |
        ForEach-Object { $_ | ConvertFrom-Json }
)
$bootIds = @(
    $journal |
        ForEach-Object {
            $property = $_.PSObject.Properties['_BOOT_ID']
            if ($null -ne $property) {
                $property.Value
            }
        } |
        Where-Object { $_ } |
        Sort-Object -Unique
)
$invocationIds = @(
    $journal |
        ForEach-Object {
            $property = $_.PSObject.Properties['INVOCATION_ID']
            if ($null -ne $property) {
                $property.Value
            }
        } |
        Where-Object { $_ } |
        Sort-Object -Unique
)
if ($bootIds.Count -ne 1 -or $bootIds[0] -ne $ExpectedBootId.ToLowerInvariant()) {
    throw "Journal boot identity mismatch: $($bootIds -join ',')"
}
if (
    $invocationIds.Count -ne 1 -or
    $invocationIds[0] -ne $ExpectedInvocationId.ToLowerInvariant()
) {
    throw "Journal invocation identity mismatch: $($invocationIds -join ',')"
}

$expectedContractMessage = (
    "OK: contract $($ContractId.ToLowerInvariant()); " +
    "new=$ExpectedNewResults, resumed=$ExpectedResumedResults"
)
$contractRecords = @(
    $journal | Where-Object {
        $property = $_.PSObject.Properties['MESSAGE']
        $null -ne $property -and $property.Value -eq $expectedContractMessage
    }
)
if (
    $contractRecords.Count -ne 1 -or
    [long] $contractRecords[0].__SEQNUM -ne $ExpectedContractSequence
) {
    throw 'Terminal contract record or sequence mismatch'
}
$successRecords = @(
    $journal | Where-Object {
        $property = $_.PSObject.Properties['MESSAGE_ID']
        $null -ne $property -and $property.Value -eq '7ad2d189f7e94e70a38c781354912448'
    }
)
if (
    $successRecords.Count -ne 1 -or
    [long] $successRecords[0].__SEQNUM -ne $ExpectedSuccessSequence
) {
    throw 'Unique terminal success record or sequence mismatch'
}

$standaloneProcesses = @()
foreach ($line in Get-Content -LiteralPath (Join-Path $outerRoot 'processes.txt')) {
    if ($line -match '^\s*\d+\s+\d+\s+\d+\s+\S+\s+(\S+)\s+(.+)$') {
        $commandName = $Matches[1]
        $arguments = $Matches[2]
        if (
            $commandName -like 'umlaut*' -or
            $commandName -like 'vampire*' -or
            (
                $commandName -eq 'python3' -and
                $arguments -match '/tools/casc_benchmark/batch\.py( |$)'
            )
        ) {
            $standaloneProcesses += $line
        }
    }
}
if ($standaloneProcesses.Count -ne 0) {
    throw "Standalone solver or batch process survived: $($standaloneProcesses -join '; ')"
}

$cgroupResidueBytes = (Get-Item -LiteralPath (Join-Path $outerRoot 'cgroup-residue.txt')).Length
$solverUnitResidueBytes = (Get-Item -LiteralPath (Join-Path $outerRoot 'solver-units.txt')).Length
if ($cgroupResidueBytes -ne 0 -or $solverUnitResidueBytes -ne 0) {
    throw (
        "Captured residue is nonempty: cgroup=$cgroupResidueBytes " +
        "units=$solverUnitResidueBytes"
    )
}

$audit = [ordered] @{
    archive_sha256 = $actualArchiveSha256
    outer_regular_files = $outerRegularFiles
    outer_hashes_verified = $outerHashesVerified
    inner_regular_files = @(Get-ChildItem -LiteralPath $innerRoot -File -Recurse).Count
    result_inventory_count = $inventory.Count
    unique_coordinates = $coordinates.Count
    referenced_streams = $referencedStreams.Count
    missing_streams = $missingStreams
    mismatched_streams = $mismatchedStreams
    orphan_streams = $orphanStreams
    session_records = $sessionRecords
    orphan_cleanup_true = $orphanCleanupTrue
    journal_records = $journal.Count
    boot_id = $bootIds[0]
    invocation_id = $invocationIds[0]
    contract_sequence = [long] $contractRecords[0].__SEQNUM
    success_sequence = [long] $successRecords[0].__SEQNUM
    standalone_solver_or_batch_processes = $standaloneProcesses.Count
    cgroup_residue_bytes = $cgroupResidueBytes
    solver_unit_residue_bytes = $solverUnitResidueBytes
    extraction_root = $resolvedExtractionRoot
}
$json = $audit | ConvertTo-Json
if ($Output) {
    $resolvedOutput = [IO.Path]::GetFullPath($Output)
    $outputParent = Split-Path -Parent $resolvedOutput
    if ($outputParent -and -not (Test-Path -LiteralPath $outputParent)) {
        [void] (New-Item -ItemType Directory -Path $outputParent)
    }
    $utf8NoBom = New-Object Text.UTF8Encoding($false)
    [IO.File]::WriteAllText($resolvedOutput, $json + [Environment]::NewLine, $utf8NoBom)
}
$json
