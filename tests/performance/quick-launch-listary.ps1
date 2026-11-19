param(
    [string]$NxToolsExecutable = (Join-Path $env:ProgramData 'NxTools\NxTools.Next\Service\NxTools.exe'),
    [string]$ListaryExecutable = 'D:\Program Files\Listary\Listary.exe',
    [string]$OutputDirectory,
    [ValidateRange(1, 20)][int]$Repetitions = 5,
    [ValidateRange(100, 5000)][int]$ObservationMs = 1000,
    [ValidateRange(10, 1000)][int]$QuietMs = 250,
    [ValidateRange(500, 30000)][int]$TimeoutMs = 3500,
    [ValidateRange(10, 500)][int]$PollMs = 25,
    [ValidateRange(0, 120)][int]$IndexWaitSeconds = 45,
    [string[]]$CaseId = @(),
    [switch]$PreflightOnly,
    [switch]$SelfTest
)

$ErrorActionPreference = 'Stop'
$workspaceRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$manifest = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'quick-launch-listary-cases.json') -Raw -Encoding UTF8 | ConvertFrom-Json
if ($TimeoutMs -lt ($ObservationMs + $QuietMs)) { throw 'TimeoutMs must cover ObservationMs plus QuietMs.' }

# Windows PowerShell uses the workstation's .NET Framework UI Automation assemblies.
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -AssemblyName WindowsBase
$driverReferences = @(
    'System.dll', 'System.Core.dll',
    [System.Windows.Automation.AutomationElement].Assembly.Location,
    [System.Windows.Automation.ControlType].Assembly.Location,
    [System.Windows.Rect].Assembly.Location
)
Add-Type -TypeDefinition (Get-Content -LiteralPath (Join-Path $PSScriptRoot 'QuickLaunchComparisonDriver.cs') -Raw -Encoding UTF8) -ReferencedAssemblies $driverReferences
[NxTools.QuickLaunchBenchmark.Launcher]::SelfTest()

$targetIds = @{}
foreach ($target in $manifest.targets) { $targetIds.Add($target.id, $true) }
$caseIds = @{}
foreach ($case in $manifest.cases) {
    $caseIds.Add($case.id, $true)
    foreach ($targetId in @($case.expected) + @($case.forbidden)) {
        if (-not $targetIds.ContainsKey($targetId)) { throw "Unknown target '$targetId' in case '$($case.id)'." }
    }
}
if ($SelfTest) {
    Write-Output "PASS driver normalization (5 assertions), manifest ($($targetIds.Count) targets, $($caseIds.Count) cases); no app operations."
    return
}
$selectedCases = @($manifest.cases)
if ($CaseId.Count -gt 0) {
    foreach ($selectedId in $CaseId) { if (-not $caseIds.ContainsKey($selectedId)) { throw "Unknown case '$selectedId'." } }
    $selectedCases = @($manifest.cases | Where-Object { $CaseId -contains $_.id })
}

$runId = [guid]::NewGuid().ToString('N')
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $workspaceRoot ("artifacts\performance\quick-launch-listary-{0}-{1}" -f (Get-Date -Format 'yyyyMMdd-HHmmss'), $runId.Substring(0, 6)) }
$OutputDirectory = [IO.Path]::GetFullPath($OutputDirectory)
$null = New-Item -ItemType Directory -Path $OutputDirectory -Force
$rawPath = Join-Path $OutputDirectory 'raw.json'
$corpusRoot = Join-Path $workspaceRoot 'tests\fixtures\nxqlb'
$records = New-Object 'System.Collections.Generic.List[object]'
$utf8 = New-Object System.Text.UTF8Encoding($false)
$document = [ordered]@{
    schema_version = 1; run_id = $runId; started_at = [DateTimeOffset]::Now.ToString('o'); finished_at = $null
    parameters = [ordered]@{ repetitions = $Repetitions; observation_ms = $ObservationMs; quiet_ms = $QuietMs; timeout_ms = $TimeoutMs; poll_ms = $PollMs; preflight_only = [bool]$PreflightOnly; order = 'AB/BA alternating by repetition; same rotated case order for both products' }
    applications = [ordered]@{}; corpus_root = $corpusRoot; targets = @($manifest.targets); cases = $selectedCases
    samples = $records; failure = $null; cleanup = [ordered]@{ errors = @(); listary_started_by_benchmark = $false }
}
$nxProcess = $null
$adapterProcess = $null
$listaryProcess = $null
$ownedListary = $null
$lastLauncher = $null

function Save-Raw {
    [IO.File]::WriteAllText($rawPath, (ConvertTo-Json -InputObject $document -Depth 16), $utf8)
}

function Get-AppMetadata($AppProcess) {
    $appItem = Get-Item -LiteralPath $AppProcess.ExecutablePath
    return [ordered]@{ process_id = [int]$AppProcess.ProcessId; path = $appItem.FullName; file_version = $appItem.VersionInfo.FileVersion; last_write_time = $appItem.LastWriteTime.ToString('o'); process_created_at = $AppProcess.CreationDate.ToString('o') }
}

function Open-Launcher([string]$Product) {
    $appProcess = if ($Product -eq 'nxtools') { $nxProcess } else { $listaryProcess }
    $launcher = [NxTools.QuickLaunchBenchmark.Launcher]::Find($Product, [uint32]$appProcess.ProcessId)
    if ($null -eq $launcher) {
        if ($Product -eq 'nxtools') { [NxTools.QuickLaunchBenchmark.Native]::ShowNxTools([uint32]$adapterProcess.ProcessId) }
        else { $null = [NxTools.QuickLaunchBenchmark.Native]::ShowListary() }
        $findDeadline = [DateTime]::UtcNow.AddSeconds(6)
        do {
            Start-Sleep -Milliseconds 100
            $launcher = [NxTools.QuickLaunchBenchmark.Launcher]::Find($Product, [uint32]$appProcess.ProcessId)
        } while ($null -eq $launcher -and [DateTime]::UtcNow -lt $findDeadline)
    }
    if ($null -eq $launcher) { throw "$Product did not expose its real launcher input." }
    $script:lastLauncher = $launcher
    if ($launcher.InputValue -ne '') { throw "$Product contains operator input; it was left untouched." }
    if (-not $launcher.Foreground) {
        $null = [NxTools.QuickLaunchBenchmark.Native]::SetForegroundWindow($launcher.Window)
        Start-Sleep -Milliseconds 150
    }
    $launcher.RequireEmpty()
    return $launcher
}

function Invoke-Query($Launcher, [string]$Phase, [int]$Repetition, [string]$SampleCaseId, [string]$Query) {
    $nativeTargets = New-Object 'System.Collections.Generic.List[NxTools.QuickLaunchBenchmark.ExpectedTarget]'
    if ($Phase -eq 'availability') { $expectedIds = @($SampleCaseId.Substring('availability:'.Length)) }
    else { $caseDefinition = $manifest.cases | Where-Object { $_.id -eq $SampleCaseId }; $expectedIds = @($caseDefinition.expected) }
    foreach ($expectedId in $expectedIds) {
        $targetDefinition = $manifest.targets | Where-Object { $_.id -eq $expectedId }
        $nativeTarget = New-Object NxTools.QuickLaunchBenchmark.ExpectedTarget
        $nativeTarget.name = $targetDefinition.name
        $nativeTarget.parent_suffix = $targetDefinition.parent_suffix
        $nativeTargets.Add($nativeTarget)
    }
    $nativeSample = $Launcher.Query($Query, $nativeTargets.ToArray(), $ObservationMs, $QuietMs, $TimeoutMs, $PollMs)
    $record = [ordered]@{
        product = $Launcher.Product; phase = $Phase; repetition = $Repetition; case_id = $SampleCaseId; query = $Query
        error = $nativeSample.error; input_set_ms = $nativeSample.input_set_ms; first_result_ms = $nativeSample.first_result_ms
        expected_target_count = $nativeSample.expected_target_count; first_target_ms = $nativeSample.first_target_ms; all_targets_ms = $nativeSample.all_targets_ms
        last_result_change_ms = $nativeSample.last_result_change_ms; result_change_count = $nativeSample.result_change_count
        observed_ms = $nativeSample.observed_ms; polls = $nativeSample.polls; poll_time_ms = $nativeSample.poll_time_ms; max_poll_ms = $nativeSample.max_poll_ms
        focus_ok = $nativeSample.focus_ok; input_restored = $nativeSample.input_restored; observation_status = $nativeSample.observation_status
        rows = @($nativeSample.rows | ForEach-Object { [ordered]@{ title = $_.title; path = $_.path } })
    }
    $records.Add($record)
    Write-Host ("{0} {1} r{2} {3}: first_ms={4}, rows={5}, status={6}" -f $Phase, $Launcher.Product, $Repetition, $SampleCaseId, $nativeSample.first_result_ms, $record.rows.Count, $nativeSample.observation_status)
    if ($record.error -or -not $record.focus_ok -or -not $record.input_restored) {
        Save-Raw
        throw "Invalid $($Launcher.Product) sample $SampleCaseId : $($record.error)"
    }
    return $record
}

function Test-TargetRows($Rows, $Target) {
    foreach ($row in $Rows) {
        if ($row.title -ine $Target.name) { continue }
        if (-not $Target.parent_suffix) { return $true }
        $parent = $row.path.Replace('/', '\').TrimEnd('\')
        if ([IO.Path]::GetFileName($parent) -ieq $Target.name) { $parent = [IO.Path]::GetDirectoryName($parent) }
        if ($parent.EndsWith('\' + $Target.parent_suffix, [StringComparison]::OrdinalIgnoreCase)) { return $true }
    }
    return $false
}

try {
    $expectedNxPath = (Resolve-Path -LiteralPath $NxToolsExecutable).Path
    $expectedListaryPath = (Resolve-Path -LiteralPath $ListaryExecutable).Path
    $nxCandidates = @(Get-CimInstance Win32_Process | Where-Object { $_.Name -eq 'NxTools.exe' -and $_.ExecutablePath -ieq $expectedNxPath })
    if ($nxCandidates.Count -ne 1) { throw 'Expected exactly one running NxTools desktop at NxToolsExecutable; no desktop was started or stopped.' }
    $nxProcess = $nxCandidates[0]
    $adapterPath = Join-Path (Split-Path -Parent $expectedNxPath) 'NxTools.PlatformHost.exe'
    $adapterCandidates = @(Get-CimInstance Win32_Process | Where-Object { $_.Name -eq 'NxTools.PlatformHost.exe' -and $_.ParentProcessId -eq $nxProcess.ProcessId -and $_.ExecutablePath -ieq $adapterPath })
    if ($adapterCandidates.Count -ne 1) { throw 'Expected exactly one matching desktop PlatformHost.' }
    $adapterProcess = $adapterCandidates[0]
    $listaryInstances = @(Get-CimInstance Win32_Process | Where-Object { $_.Name -eq 'Listary.exe' })
    if (@($listaryInstances | Where-Object { $_.ExecutablePath -ine $expectedListaryPath }).Count -gt 0) { throw 'Another Listary installation owns the global URI pipe; no summon was sent.' }
    $listaryCandidates = @($listaryInstances | Where-Object { $_.ExecutablePath -ieq $expectedListaryPath })
    if ($listaryCandidates.Count -gt 1) { throw 'Multiple matching Listary instances are running.' }
    if ($listaryCandidates.Count -eq 0) {
        $ownedListary = Start-Process -FilePath $expectedListaryPath -PassThru
        $listaryProcess = Get-CimInstance Win32_Process -Filter "ProcessId=$($ownedListary.Id)"
        $document.cleanup.listary_started_by_benchmark = $true
        $pipeDeadline = [DateTime]::UtcNow.AddSeconds(15)
        do {
            $pipePresent = @([IO.Directory]::GetFiles('\\.\pipe\') | Where-Object { $_.EndsWith('\ListaryInternalNamedPipe', [StringComparison]::OrdinalIgnoreCase) }).Count -gt 0
            if (-not $pipePresent) { Start-Sleep -Milliseconds 200 }
        } while (-not $pipePresent -and [DateTime]::UtcNow -lt $pipeDeadline)
        if (-not $pipePresent) { throw 'The started Listary instance did not expose its local URI dispatch pipe.' }
    }
    else { $listaryProcess = $listaryCandidates[0] }
    $document.applications.nxtools = Get-AppMetadata $nxProcess
    $document.applications.platformhost = Get-AppMetadata $adapterProcess
    $document.applications.listary = Get-AppMetadata $listaryProcess
    $document.services = @(Get-CimInstance Win32_Service | Where-Object { $_.Name -eq 'NxToolsIndexerV1' -or $_.Name -eq 'ListaryServiceV2' } | Select-Object Name,State,ProcessId,PathName)

    foreach ($target in $manifest.targets) {
        $fixturePath = Join-Path $corpusRoot $target.relative_path
        if ($target.directory) { $null = New-Item -ItemType Directory -Path $fixturePath -Force; continue }
        $null = New-Item -ItemType Directory -Path (Split-Path -Parent $fixturePath) -Force
        $fixtureText = "Quick Launch comparison fixture: $($target.id)`r`n"
        if (Test-Path -LiteralPath $fixturePath) {
            if ([IO.File]::ReadAllText($fixturePath) -ne $fixtureText) { throw "Existing fixture differs; it was not overwritten: $fixturePath" }
        }
        else { [IO.File]::WriteAllText($fixturePath, $fixtureText, $utf8) }
    }
    Save-Raw

    foreach ($product in @('nxtools', 'listary')) {
        $launcher = Open-Launcher $product
        $pendingTargets = @($manifest.targets)
        $availabilityDeadline = [DateTime]::UtcNow.AddSeconds($IndexWaitSeconds)
        $availabilityRound = 0
        do {
            $availabilityRound++
            $stillMissing = @()
            foreach ($target in $pendingTargets) {
                $record = Invoke-Query $launcher 'availability' $availabilityRound ('availability:' + $target.id) $target.name
                if (-not (Test-TargetRows $record.rows $target)) { $stillMissing += $target }
            }
            $pendingTargets = $stillMissing
            Save-Raw
            if ($pendingTargets.Count -gt 0 -and [DateTime]::UtcNow -lt $availabilityDeadline) { Start-Sleep -Seconds 2 }
        } while ($pendingTargets.Count -gt 0 -and [DateTime]::UtcNow -lt $availabilityDeadline)
        Write-Host ("AVAILABILITY {0}: {1}/{2} controlled targets" -f $product, ($manifest.targets.Count - $pendingTargets.Count), $manifest.targets.Count)
    }

    if (-not $PreflightOnly) {
        foreach ($product in @('nxtools', 'listary')) {
            $launcher = Open-Launcher $product
            foreach ($case in $selectedCases) { $null = Invoke-Query $launcher 'warmup' 0 $case.id $case.query }
            Save-Raw
        }
        for ($repetition = 1; $repetition -le $Repetitions; $repetition++) {
            $productOrder = if ($repetition % 2 -eq 1) { @('nxtools', 'listary') } else { @('listary', 'nxtools') }
            $offset = ($repetition - 1) % $selectedCases.Count
            $orderedCases = @($selectedCases[$offset..($selectedCases.Count - 1)])
            if ($offset -gt 0) { $orderedCases += @($selectedCases[0..($offset - 1)]) }
            foreach ($product in $productOrder) {
                $launcher = Open-Launcher $product
                foreach ($case in $orderedCases) { $null = Invoke-Query $launcher 'measurement' $repetition $case.id $case.query }
                Save-Raw
            }
        }
    }
}
catch {
    $document.failure = $_.Exception.Message
    Write-Warning $document.failure
}
finally {
    if ($null -ne $lastLauncher) {
        try {
            if ($lastLauncher.Foreground -and $lastLauncher.InputValue -eq '') {
                [NxTools.QuickLaunchBenchmark.Native]::Escape($lastLauncher.Window)
                Start-Sleep -Milliseconds 200
                $document.cleanup.last_launcher_hidden = -not [NxTools.QuickLaunchBenchmark.Native]::IsWindowVisible($lastLauncher.Window)
            }
        }
        catch { $document.cleanup.errors += $_.Exception.Message }
    }
    if ($null -ne $ownedListary) {
        try {
            $ownedProcess = Get-CimInstance Win32_Process -Filter "ProcessId=$($ownedListary.Id)"
            if ($null -ne $ownedProcess) {
                if ($ownedProcess.ExecutablePath -ine $expectedListaryPath -or $ownedProcess.CreationDate -ne $listaryProcess.CreationDate) { throw 'Started Listary identity changed; process was left untouched.' }
                Stop-Process -Id $ownedListary.Id -ErrorAction Stop
                $ownedListary.WaitForExit(3000) | Out-Null
            }
            $ownedHooks = @(Get-CimInstance Win32_Process | Where-Object { $_.ParentProcessId -eq $ownedListary.Id -and $_.Name -match '^ListaryHookHost(32|64)\.exe$' })
            foreach ($ownedHook in $ownedHooks) { Stop-Process -Id $ownedHook.ProcessId -ErrorAction Stop }
            $document.cleanup.owned_listary_exited = $null -eq (Get-Process -Id $ownedListary.Id -ErrorAction SilentlyContinue)
        }
        catch { $document.cleanup.errors += $_.Exception.Message }
    }
    if ($null -ne $nxProcess) {
        $currentNx = Get-CimInstance Win32_Process -Filter "ProcessId=$($nxProcess.ProcessId)"
        $document.cleanup.nxtools_preserved = $null -ne $currentNx -and $currentNx.CreationDate -eq $nxProcess.CreationDate
    }
    $document.finished_at = [DateTimeOffset]::Now.ToString('o')
    Save-Raw
    Write-Host "RAW=$rawPath"
}
if ($document.failure -or $document.cleanup.errors.Count -gt 0) { throw 'Benchmark did not complete cleanly; inspect raw.json failure/cleanup.' }
Write-Output "PASS external launcher comparison; samples=$($records.Count), output=$OutputDirectory"
