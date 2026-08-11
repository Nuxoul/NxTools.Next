param(
    [switch]$SourcePreflightOnly,
    [switch]$ContractTestOnly,
    [string]$PackageDirectory = "artifacts\win-x64",
    [string]$OutputDirectory = "",
    [string]$FfprobePath = "",
    [ValidateRange(1, 120)]
    [int]$TimeoutMinutes = 30
)

Set-StrictMode -Version Latest
$ProgressPreference = 'SilentlyContinue'
$ErrorActionPreference = 'Stop'
if ($PSVersionTable.PSVersion.Major -lt 7) {
    throw 'Stage 5 media acceptance requires PowerShell 7 or newer.'
}

$ProcessTimeoutMilliseconds = 15000
$MaximumChildOutputCharacters = 64 * 1024
$ExpectedDesktopProcessId = $null
$ExpectedDesktopPath = ''
$ProcessRunnerSource = Join-Path $PSScriptRoot 'MediaSmokeProcessRunner.cs'
if ($null -eq ('NxTools.MediaSmoke.BoundedProcessRunner' -as [type])) {
    Add-Type -Path $ProcessRunnerSource
}

$AppleHlsMasterUrl = 'https://devstreaming-cdn.apple.com/videos/streaming/examples/bipbop_4x3/bipbop_4x3_variant.m3u8'
$AppleHlsMediaUrl = 'https://devstreaming-cdn.apple.com/videos/streaming/examples/bipbop_4x3/gear1/prog_index.m3u8'
$DashIfWaveUrl = 'https://dash.akamaized.net/WAVE/vectors/2021-06-11/avc_sets/15_30_60/t1/stream.mpd'
$MaximumManifestBytes = 2 * 1024 * 1024
$MinimumHlsSegments = 175
$MaximumHlsSegments = 190
$MinimumHlsDurationSeconds = 1790.0
$MaximumHlsDurationSeconds = 1810.0
$MinimumDashSegmentsPerTrack = 25
$MaximumDashSegmentsPerTrack = 35
$MinimumDashDurationSeconds = 55.0
$MaximumDashDurationSeconds = 65.0

function Read-BoundedRemoteText {
    param(
        [Parameter(Mandatory)]
        [string]$Url,
        [Parameter(Mandatory)]
        [int]$MaximumBytes
    )

    $httpClient = [System.Net.Http.HttpClient]::new()
    $requestDeadline = [System.Threading.CancellationTokenSource]::new([TimeSpan]::FromSeconds(30))
    try {
        $response = $httpClient.GetAsync(
            $Url,
            [System.Net.Http.HttpCompletionOption]::ResponseHeadersRead,
            $requestDeadline.Token
        ).GetAwaiter().GetResult()
        try {
            $null = $response.EnsureSuccessStatusCode()
            $contentLength = $response.Content.Headers.ContentLength
            if ($null -ne $contentLength -and $contentLength -gt $MaximumBytes) {
                throw "Remote manifest exceeds the $MaximumBytes-byte acceptance limit."
            }

            $responseStream = $response.Content.ReadAsStreamAsync().GetAwaiter().GetResult()
            try {
                $buffer = [byte[]]::new(8192)
                $content = [System.IO.MemoryStream]::new()
                try {
                    while (($bytesRead = $responseStream.ReadAsync(
                        $buffer,
                        0,
                        $buffer.Length,
                        $requestDeadline.Token
                    ).GetAwaiter().GetResult()) -gt 0) {
                        if ($content.Length + $bytesRead -gt $MaximumBytes) {
                            throw "Remote manifest exceeds the $MaximumBytes-byte acceptance limit."
                        }
                        $content.Write($buffer, 0, $bytesRead)
                    }
                    return [System.Text.Encoding]::UTF8.GetString($content.ToArray()).TrimStart([char]0xFEFF)
                }
                finally {
                    $content.Dispose()
                }
            }
            finally {
                $responseStream.Dispose()
            }
        }
        finally {
            $response.Dispose()
        }
    }
    finally {
        $requestDeadline.Dispose()
        $httpClient.Dispose()
    }
}

function Invoke-BoundedNativeProcess {
    param(
        [Parameter(Mandatory)]
        [string]$Executable,
        [Parameter(Mandatory)]
        [string[]]$Arguments,
        [int]$TimeoutMilliseconds = $ProcessTimeoutMilliseconds
    )

    return [NxTools.MediaSmoke.BoundedProcessRunner]::Run(
        $Executable,
        $Arguments,
        $TimeoutMilliseconds,
        $MaximumChildOutputCharacters
    )
}

function Assert-BoundedProcessRunner {
    $shellPath = [Environment]::ProcessPath
    $success = Invoke-BoundedNativeProcess -Executable $shellPath -Arguments @(
        '-NoLogo', '-NoProfile', '-Command', "[Console]::Out.Write('bounded-runner-ok')"
    ) -TimeoutMilliseconds 5000
    if ($success.TimedOut -or $success.ExitCode -ne 0 -or
        $success.StandardOutput -ne 'bounded-runner-ok') {
        throw 'The bounded child-process runner did not preserve a normal process result.'
    }

    $treeRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("NxTools-MediaSmoke-$([guid]::NewGuid())")
    $null = New-Item -ItemType Directory -Path $treeRoot
    try {
        $sentinelPath = Join-Path $treeRoot 'descendant-survived.txt'
        $childScript = "Start-Sleep -Milliseconds 2500; [System.IO.File]::WriteAllText('$($sentinelPath.Replace("'", "''"))', 'alive')"
        $childEncoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($childScript))
        $parentScript = @"
`$child = Start-Process -FilePath '$($shellPath.Replace("'", "''"))' -ArgumentList @('-NoLogo','-NoProfile','-EncodedCommand','$childEncoded') -PassThru -WindowStyle Hidden
[Console]::Out.Write(`$child.Id)
[Console]::Out.Flush()
Start-Sleep -Seconds 10
"@
        $parentEncoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($parentScript))
        $timeout = Invoke-BoundedNativeProcess -Executable $shellPath -Arguments @(
            '-NoLogo', '-NoProfile', '-EncodedCommand', $parentEncoded
        ) -TimeoutMilliseconds 1000
        $descendantProcessId = 0
        if (-not $timeout.TimedOut -or
            -not [int]::TryParse($timeout.StandardOutput.Trim(), [ref]$descendantProcessId)) {
            throw 'The bounded child-process runner did not expose a live descendant before timeout.'
        }
        Start-Sleep -Milliseconds 3000
        if ($null -ne (Get-Process -Id $descendantProcessId -ErrorAction SilentlyContinue) -or
            (Test-Path -LiteralPath $sentinelPath)) {
            throw 'The bounded child-process runner did not terminate the complete timed-out process tree.'
        }
    }
    finally {
        Remove-Item -LiteralPath $treeRoot -Recurse -Force -ErrorAction SilentlyContinue
    }

    $bounded = Invoke-BoundedNativeProcess -Executable $shellPath -Arguments @(
        '-NoLogo', '-NoProfile', '-Command',
        "[Console]::Out.Write('o' * 100000); [Console]::Error.Write('e' * 100000)"
    ) -TimeoutMilliseconds 5000
    if ($bounded.TimedOut -or $bounded.ExitCode -ne 0 -or
        $bounded.StandardOutput.Length -ne $MaximumChildOutputCharacters -or
        $bounded.StandardError.Length -ne $MaximumChildOutputCharacters) {
        throw 'The bounded child-process runner did not cap and drain both output streams.'
    }
    Write-Output 'PASS bounded child-process runner contract.'
}

function Assert-MediaSources {
    $hlsMaster = Read-BoundedRemoteText -Url $AppleHlsMasterUrl -MaximumBytes $MaximumManifestBytes
    $hlsVariantCount = [regex]::Matches($hlsMaster, '(?m)^#EXT-X-STREAM-INF:').Count
    if ($hlsVariantCount -lt 1 -or $hlsVariantCount -gt 8 -or
        $hlsMaster -notmatch '(?m)^gear1/prog_index\.m3u8\s*$') {
        throw 'The Apple HLS master no longer exposes the expected bounded variant layout.'
    }

    $hlsMedia = Read-BoundedRemoteText -Url $AppleHlsMediaUrl -MaximumBytes $MaximumManifestBytes
    $hlsDurations = @([regex]::Matches(
        $hlsMedia,
        '(?m)^#EXTINF:([0-9]+(?:\.[0-9]+)?)'
    ) | ForEach-Object { [double]$_.Groups[1].Value })
    $hlsDurationSeconds = [double](($hlsDurations | Measure-Object -Sum).Sum)
    if ($hlsMedia -notmatch '(?m)^#EXT-X-ENDLIST\s*$' -or
        $hlsDurations.Count -lt $MinimumHlsSegments -or
        $hlsDurations.Count -gt $MaximumHlsSegments -or
        $hlsDurationSeconds -lt $MinimumHlsDurationSeconds -or
        $hlsDurationSeconds -gt $MaximumHlsDurationSeconds -or
        $hlsMedia -match '(?m)^#EXT-X-(KEY|MAP|BYTERANGE):') {
        throw 'The Apple HLS media playlist no longer matches the finite plaintext TS acceptance profile.'
    }

    $dashManifest = Read-BoundedRemoteText -Url $DashIfWaveUrl -MaximumBytes $MaximumManifestBytes
    [xml]$dashXml = $dashManifest
    $periods = @($dashXml.SelectNodes("//*[local-name()='Period']"))
    $adaptationSets = @($dashXml.SelectNodes("//*[local-name()='AdaptationSet']"))
    $mpdDuration = [System.Xml.XmlConvert]::ToTimeSpan([string]$dashXml.DocumentElement.mediaPresentationDuration).TotalSeconds
    if ($dashManifest -notmatch 'type="static"' -or
        $periods.Count -ne 1 -or
        $adaptationSets.Count -ne 2 -or
        $mpdDuration -lt $MinimumDashDurationSeconds -or
        $mpdDuration -gt $MaximumDashDurationSeconds -or
        $dashManifest -notmatch 'mimeType="video/mp4"' -or
        $dashManifest -notmatch 'mimeType="audio/mp4"' -or
        $dashManifest -notmatch '<SegmentTimeline>' -or
        $dashManifest -match '<(?:ContentProtection|SegmentList|SegmentBase)(?:\s|>)') {
        throw 'The DASH-IF sample no longer matches the supported static dual-track acceptance profile.'
    }
    foreach ($adaptationSet in $adaptationSets) {
        $segmentTemplate = $adaptationSet.SelectSingleNode("./*[local-name()='SegmentTemplate']")
        $timelineEntries = @($segmentTemplate.SelectNodes("./*[local-name()='SegmentTimeline']/*[local-name()='S']"))
        $timescale = [long]$segmentTemplate.timescale
        if ($timescale -le 0) {
            throw 'The DASH-IF sample now has an invalid SegmentTemplate timescale.'
        }
        $expandedSegmentCount = 0L
        $expandedDurationUnits = 0L
        foreach ($entry in $timelineEntries) {
            $repeat = if ($null -eq $entry.Attributes['r']) { 0L } else { [long]$entry.r }
            $duration = [long]$entry.d
            if ($repeat -lt 0 -or $duration -le 0) {
                throw 'The DASH-IF sample now uses an unbounded or invalid SegmentTimeline.'
            }
            $occurrences = $repeat + 1L
            $expandedSegmentCount += $occurrences
            $expandedDurationUnits += $duration * $occurrences
        }
        $trackDurationSeconds = $expandedDurationUnits / [double]$timescale
        if ($expandedSegmentCount -lt $MinimumDashSegmentsPerTrack -or
            $expandedSegmentCount -gt $MaximumDashSegmentsPerTrack -or
            $trackDurationSeconds -lt $MinimumDashDurationSeconds -or
            $trackDurationSeconds -gt $MaximumDashDurationSeconds) {
            throw 'The DASH-IF sample no longer has the expected bounded per-track expansion.'
        }
    }

    Write-Output 'PASS media source preflight: Apple HLS and DASH-IF WAVE manifests remain compatible.'
}

function Resolve-ExactPath {
    param(
        [Parameter(Mandatory)]
        [string]$Path,
        [Parameter(Mandatory)]
        [string]$Description
    )

    if (-not (Test-Path -LiteralPath $Path)) {
        throw "$Description does not exist: $Path"
    }
    return (Resolve-Path -LiteralPath $Path).Path
}

function Invoke-NxToolsCliJson {
    param(
        [Parameter(Mandatory)]
        [string]$CliPath,
        [Parameter(Mandatory)]
        [string[]]$Arguments
    )

    Assert-ExpectedDesktopHost
    $cliArguments = @('--connect-only') + $Arguments
    $processResult = Invoke-BoundedNativeProcess -Executable $CliPath -Arguments $cliArguments
    if ($processResult.TimedOut) {
        throw 'NxTools CLI exceeded its bounded response timeout; no desktop was started.'
    }
    if ($processResult.ExitCode -ne 0) {
        $failureOutput = if ([string]::IsNullOrWhiteSpace($processResult.StandardError)) {
            $processResult.StandardOutput
        } else {
            $processResult.StandardError
        }
        throw "NxTools CLI failed with exit code $($processResult.ExitCode).`n$failureOutput"
    }
    $jsonLine = $processResult.StandardOutput -split '\r?\n' |
        Where-Object { -not [string]::IsNullOrWhiteSpace($_) } |
        Select-Object -Last 1
    if ($null -eq $jsonLine) {
        throw 'NxTools CLI returned no JSON response.'
    }
    return ($jsonLine | ConvertFrom-Json)
}

function Assert-ExpectedDesktopHost {
    if ($null -eq $ExpectedDesktopProcessId) {
        return
    }
    $expectedProcess = Get-CimInstance Win32_Process -Filter "ProcessId = $ExpectedDesktopProcessId" |
        Select-Object -First 1
    if ($null -eq $expectedProcess -or
        [string]::IsNullOrWhiteSpace($expectedProcess.ExecutablePath) -or
        -not [string]::Equals(
            [System.IO.Path]::GetFullPath($expectedProcess.ExecutablePath),
            $ExpectedDesktopPath,
            [StringComparison]::OrdinalIgnoreCase
        )) {
        throw 'The approved desktop exited or changed before the CLI operation; no replacement will be started.'
    }
    $allDesktopProcesses = @(Get-CimInstance Win32_Process |
        Where-Object {
            $_.Name -in @('NxTools.exe', 'nxtools-desktop.exe') -and
            -not [string]::IsNullOrWhiteSpace($_.ExecutablePath)
        })
    if ($allDesktopProcesses.Count -ne 1 -or
        [int]$allDesktopProcesses[0].ProcessId -ne [int]$ExpectedDesktopProcessId) {
        throw 'The desktop host set changed before the CLI operation; no ambiguous host will be used.'
    }
}

function Get-NxToolsMediaTaskMatches {
    param(
        [Parameter(Mandatory)]
        [string]$CliPath,
        [Parameter(Mandatory)]
        [string]$Url,
        [Parameter(Mandatory)]
        [string]$Directory,
        [Parameter(Mandatory)]
        [string]$FileName
    )

    $listResponse = Invoke-NxToolsCliJson -CliPath $CliPath -Arguments @(
        'download', 'list', '--json'
    )
    if (-not $listResponse.ok) {
        throw 'NxTools rejected the task reconciliation snapshot.'
    }
    $expectedDirectory = [System.IO.Path]::GetFullPath($Directory)
    $matchingTasks = @($listResponse.result.tasks | Where-Object {
        $task = $_.task
        if ($null -eq $task -or [string]::IsNullOrWhiteSpace([string]$task.save_directory)) {
            return $false
        }
        $candidateDirectory = [System.IO.Path]::GetFullPath([string]$task.save_directory)
        return [string]::Equals(
            $candidateDirectory,
            $expectedDirectory,
            [System.StringComparison]::OrdinalIgnoreCase
        ) -and
            [string]::Equals(
                [string]$task.file_name,
                $FileName,
                [System.StringComparison]::OrdinalIgnoreCase
            ) -and
            [string]::Equals([string]$task.source_url, $Url, [System.StringComparison]::Ordinal)
    })
    return $matchingTasks
}

function Wait-NxToolsMediaTaskMatches {
    param(
        [Parameter(Mandatory)]
        [string]$CliPath,
        [Parameter(Mandatory)]
        [string]$Url,
        [Parameter(Mandatory)]
        [string]$Directory,
        [Parameter(Mandatory)]
        [string]$FileName,
        [Parameter(Mandatory)]
        [DateTimeOffset]$Deadline
    )

    while ([DateTimeOffset]::UtcNow -lt $Deadline) {
        try {
            $matchingTasks = @(Get-NxToolsMediaTaskMatches `
                -CliPath $CliPath `
                -Url $Url `
                -Directory $Directory `
                -FileName $FileName)
            if ($matchingTasks.Count -gt 0) {
                return $matchingTasks
            }
        }
        catch {
        }
        Start-Sleep -Milliseconds 250
    }
    return @()
}

function Wait-NxToolsMediaTask {
    param(
        [Parameter(Mandatory)]
        [string]$CliPath,
        [Parameter(Mandatory)]
        [string]$TaskId,
        [Parameter(Mandatory)]
        [DateTimeOffset]$Deadline
    )

    while ([DateTimeOffset]::UtcNow -lt $Deadline) {
        $taskResponse = Invoke-NxToolsCliJson -CliPath $CliPath -Arguments @(
            'download', 'show', $TaskId, '--json'
        )
        if (-not $taskResponse.ok) {
            throw "NxTools rejected task status query for $TaskId."
        }
        $taskStatus = [string]$taskResponse.result.task.status
        switch ($taskStatus.ToLowerInvariant()) {
            'completed' { return $taskResponse.result }
            'failed' { return $taskResponse.result }
            'cancelled' { return $taskResponse.result }
        }
        Start-Sleep -Milliseconds 500
    }
    throw "Media task $TaskId exceeded the $TimeoutMinutes-minute acceptance timeout."
}

function Request-NxToolsTaskCancellation {
    param(
        [Parameter(Mandatory)]
        [string]$CliPath,
        [Parameter(Mandatory)]
        [string]$TaskId
    )

    $cancelDeadline = [DateTimeOffset]::UtcNow.AddSeconds(30)
    $cancelAccepted = $false
    $lastFailure = 'no cancellation response was received'
    while ([DateTimeOffset]::UtcNow -lt $cancelDeadline) {
        if (-not $cancelAccepted) {
            try {
                $cancelResponse = Invoke-NxToolsCliJson -CliPath $CliPath -Arguments @(
                    'download', 'cancel', $TaskId, '--json'
                )
                if ($cancelResponse.ok) {
                    $cancelAccepted = $true
                }
                else {
                    $lastFailure = 'cancel request was rejected'
                }
            }
            catch {
                $lastFailure = "cancel request failed: $($_.Exception.Message)"
            }
        }
        try {
            $taskResponse = Invoke-NxToolsCliJson -CliPath $CliPath -Arguments @(
                'download', 'show', $TaskId, '--json'
            )
            $taskStatus = ([string]$taskResponse.result.task.status).ToLowerInvariant()
            if ($taskStatus -eq 'cancelled') {
                return 'durable Cancelled terminal observed'
            }
            if ($taskStatus -in @('completed', 'failed')) {
                return "durable $taskStatus terminal observed instead of Cancelled"
            }
        }
        catch {
            $lastFailure = "terminal verification failed: $($_.Exception.Message)"
        }
        Start-Sleep -Milliseconds 250
    }
    if ($cancelAccepted) {
        return 'cancel was requested but no durable terminal was observed within 30 seconds'
    }
    return "task outcome is unknown after the bounded cleanup window: $lastFailure"
}

function Test-MediaArtifact {
    param(
        [Parameter(Mandatory)]
        [string]$ProbePath,
        [Parameter(Mandatory)]
        [string]$ArtifactPath,
        [Parameter(Mandatory)]
        [bool]$RequireAudio,
        [Parameter(Mandatory)]
        [double]$MinimumDurationSeconds,
        [Parameter(Mandatory)]
        [double]$MaximumDurationSeconds
    )

    $probeResult = Invoke-BoundedNativeProcess -Executable $ProbePath -Arguments @(
        '-v', 'error',
        '-show_entries', 'format=duration',
        '-show_entries', 'stream=codec_type',
        '-of', 'json',
        $ArtifactPath
    ) -TimeoutMilliseconds 30000
    if ($probeResult.TimedOut) {
        throw 'ffprobe exceeded its bounded 30-second validation timeout.'
    }
    if ($probeResult.ExitCode -ne 0) {
        throw "ffprobe rejected the completed artifact.`n$($probeResult.StandardError)"
    }
    $probe = ($probeResult.StandardOutput | ConvertFrom-Json)
    $streamKinds = @($probe.streams | ForEach-Object { [string]$_.codec_type })
    if ($streamKinds -notcontains 'video') {
        throw 'Completed media artifact does not contain a video stream.'
    }
    if ($RequireAudio -and $streamKinds -notcontains 'audio') {
        throw 'Completed media artifact does not contain the expected audio stream.'
    }
    $durationSeconds = [double]$probe.format.duration
    if ($durationSeconds -lt $MinimumDurationSeconds -or
        $durationSeconds -gt $MaximumDurationSeconds) {
        throw 'Completed media artifact is outside the expected fixture duration range.'
    }
}

function Invoke-MediaTask {
    param(
        [Parameter(Mandatory)]
        [string]$CliPath,
        [Parameter(Mandatory)]
        [string]$Url,
        [Parameter(Mandatory)]
        [string]$Directory,
        [Parameter(Mandatory)]
        [string]$FileName,
        [Parameter(Mandatory)]
        [DateTimeOffset]$Deadline,
        [string]$ProbePath = '',
        [Parameter(Mandatory)]
        [double]$MinimumDurationSeconds,
        [Parameter(Mandatory)]
        [double]$MaximumDurationSeconds
    )

    $taskId = ''
    $terminalObserved = $false
    $submissionAttempted = $false
    $submissionSettledWithoutTask = $false
    $operationFailure = ''
    $cleanupOutcomes = [System.Collections.Generic.List[string]]::new()
    try {
        $preexistingMatches = @(Get-NxToolsMediaTaskMatches `
            -CliPath $CliPath `
            -Url $Url `
            -Directory $Directory `
            -FileName $FileName)
        if ($preexistingMatches.Count -ne 0) {
            throw 'The explicit acceptance artifact identity already belongs to a durable task.'
        }

        $submissionAttempted = $true
        $addResponse = Invoke-NxToolsCliJson -CliPath $CliPath -Arguments @(
            'download', 'add', $Url, '--dir', $Directory, '--name', $FileName, '--json'
        )
        if (-not $addResponse.ok) {
            $submissionSettledWithoutTask = $true
            throw "NxTools rejected the media task for $FileName."
        }
        $taskId = [string]$addResponse.result.task_id
        if ([string]::IsNullOrWhiteSpace($taskId)) {
            throw 'NxTools accepted the media task without returning a task ID.'
        }

        Write-Output "STARTED $FileName task=$taskId"
        $task = Wait-NxToolsMediaTask -CliPath $CliPath -TaskId $taskId -Deadline $Deadline
        $terminalObserved = $true
        $taskStatus = ([string]$task.task.status).ToLowerInvariant()
        if ($taskStatus -eq 'failed') {
            $faultMessage = [string]$task.error.message
            throw "Media task $taskId failed: $faultMessage"
        }
        if ($taskStatus -eq 'cancelled') {
            throw "Media task $taskId reached a durable Cancelled terminal."
        }

        $artifactPath = [string]$task.final_path
        $expectedArtifactPath = [System.IO.Path]::GetFullPath((Join-Path $Directory $FileName))
        $actualArtifactPath = if ([string]::IsNullOrWhiteSpace($artifactPath)) {
            ''
        }
        else {
            [System.IO.Path]::GetFullPath($artifactPath)
        }
        if (-not [string]::Equals(
                $actualArtifactPath,
                $expectedArtifactPath,
                [System.StringComparison]::OrdinalIgnoreCase
            )) {
            throw "Completed media task $taskId published outside the requested artifact path."
        }
        if ([string]::IsNullOrWhiteSpace($actualArtifactPath) -or
            -not (Test-Path -LiteralPath $actualArtifactPath -PathType Leaf) -or
            (Get-Item -LiteralPath $actualArtifactPath).Length -le 0) {
            throw "Completed media task $taskId has no nonempty final artifact."
        }
        if (-not [string]::IsNullOrWhiteSpace($ProbePath)) {
            Test-MediaArtifact `
                -ProbePath $ProbePath `
                -ArtifactPath $actualArtifactPath `
                -RequireAudio $true `
                -MinimumDurationSeconds $MinimumDurationSeconds `
                -MaximumDurationSeconds $MaximumDurationSeconds
        }
        Write-Output "PASS $FileName task=$taskId artifact=$actualArtifactPath"
    }
    catch {
        $operationFailure = $_.Exception.Message
    }
    finally {
        if ($submissionAttempted -and
            -not $submissionSettledWithoutTask -and
            -not $terminalObserved) {
            $ownedTaskIds = [System.Collections.Generic.HashSet[string]]::new(
                [System.StringComparer]::OrdinalIgnoreCase
            )
            if (-not [string]::IsNullOrWhiteSpace($taskId)) {
                $null = $ownedTaskIds.Add($taskId)
            }
            try {
                $reconciledTasks = @(Wait-NxToolsMediaTaskMatches `
                    -CliPath $CliPath `
                    -Url $Url `
                    -Directory $Directory `
                    -FileName $FileName `
                    -Deadline ([DateTimeOffset]::UtcNow.AddSeconds(30)))
                foreach ($reconciledTask in $reconciledTasks) {
                    $reconciledTaskId = [string]$reconciledTask.task.id
                    if (-not [string]::IsNullOrWhiteSpace($reconciledTaskId)) {
                        $null = $ownedTaskIds.Add($reconciledTaskId)
                    }
                }
            }
            catch {
                $null = $cleanupOutcomes.Add("task reconciliation failed: $($_.Exception.Message)")
            }

            if ($ownedTaskIds.Count -eq 0) {
                $null = $cleanupOutcomes.Add('no durable task could be reconciled; submission outcome is unknown')
            }
            foreach ($ownedTaskId in $ownedTaskIds) {
                $cancelOutcome = Request-NxToolsTaskCancellation -CliPath $CliPath -TaskId $ownedTaskId
                $null = $cleanupOutcomes.Add("task ${ownedTaskId}: $cancelOutcome")
            }
        }
    }

    if (-not [string]::IsNullOrWhiteSpace($operationFailure)) {
        if ($cleanupOutcomes.Count -gt 0) {
            throw "$operationFailure Cleanup outcome: $($cleanupOutcomes -join '; ')."
        }
        throw $operationFailure
    }
}

if ($ContractTestOnly) {
    Assert-BoundedProcessRunner
    return
}

Assert-MediaSources
if ($SourcePreflightOnly) {
    return
}

if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    throw 'Full media acceptance requires an explicit -OutputDirectory.'
}

$packageRoot = Resolve-ExactPath -Path $PackageDirectory -Description 'Package directory'
$desktopPath = Resolve-ExactPath -Path (Join-Path $packageRoot 'NxTools.exe') -Description 'Packaged desktop'
$cliPath = Resolve-ExactPath -Path (Join-Path $packageRoot 'NxTools.Cli.exe') -Description 'Packaged CLI'
$packagedIndexerPath = Resolve-ExactPath -Path (Join-Path $packageRoot 'NxTools.Indexer.exe') -Description 'Packaged indexer'
$packagedPlatformHostPath = Resolve-ExactPath -Path (Join-Path $packageRoot 'NxTools.PlatformHost.exe') -Description 'Packaged PlatformHost'
$expectedDesktopPath = [System.IO.Path]::GetFullPath($desktopPath)
$programDataRoot = if ([string]::IsNullOrWhiteSpace($env:ProgramData)) {
    'C:\ProgramData'
} else {
    $env:ProgramData
}
$protectedRoot = Join-Path $programDataRoot 'NxTools\NxTools.Next\Service'
$protectedDesktopPath = [System.IO.Path]::GetFullPath((Join-Path $protectedRoot 'NxTools.exe'))
$protectedMatchesPackage = $true
foreach ($packagePair in @(
    [pscustomobject]@{ Release = $desktopPath; Protected = (Join-Path $protectedRoot 'NxTools.exe') },
    [pscustomobject]@{ Release = $packagedIndexerPath; Protected = (Join-Path $protectedRoot 'NxTools.Indexer.exe') },
    [pscustomobject]@{ Release = $packagedPlatformHostPath; Protected = (Join-Path $protectedRoot 'NxTools.PlatformHost.exe') }
)) {
    if (-not (Test-Path -LiteralPath $packagePair.Protected -PathType Leaf) -or
        (Get-FileHash -LiteralPath $packagePair.Release -Algorithm SHA256).Hash -ne
        (Get-FileHash -LiteralPath $packagePair.Protected -Algorithm SHA256).Hash) {
        $protectedMatchesPackage = $false
        break
    }
}
$desktopProcesses = @(Get-CimInstance Win32_Process |
    Where-Object {
        $_.Name -in @('NxTools.exe', 'nxtools-desktop.exe') -and
        -not [string]::IsNullOrWhiteSpace($_.ExecutablePath)
    })
if ($desktopProcesses.Count -ne 1) {
    throw 'Start exactly one current NxTools desktop before the smoke test; debug or duplicate desktop hosts are not accepted.'
}
$runningDesktopPath = [System.IO.Path]::GetFullPath($desktopProcesses[0].ExecutablePath)
$runningPackagedDesktop = [string]::Equals(
    $runningDesktopPath,
    $expectedDesktopPath,
    [StringComparison]::OrdinalIgnoreCase
)
$runningProtectedDesktop = $protectedMatchesPackage -and [string]::Equals(
    $runningDesktopPath,
    $protectedDesktopPath,
    [StringComparison]::OrdinalIgnoreCase
)
if (-not $runningPackagedDesktop -and -not $runningProtectedDesktop) {
    throw 'The running desktop is not the selected package or its byte-identical protected atomic copy.'
}
$ExpectedDesktopProcessId = [int]$desktopProcesses[0].ProcessId
$ExpectedDesktopPath = $runningDesktopPath

if (-not (Test-Path -LiteralPath $OutputDirectory)) {
    $null = New-Item -ItemType Directory -Path $OutputDirectory
}
$outputRoot = Resolve-ExactPath -Path $OutputDirectory -Description 'Output directory'
if ($null -ne (Get-ChildItem -LiteralPath $outputRoot -Force | Select-Object -First 1)) {
    throw 'Full media acceptance requires a new empty output directory so existing files cannot satisfy the artifact checks.'
}

$resolvedProbePath = ''
if (-not [string]::IsNullOrWhiteSpace($FfprobePath)) {
    $resolvedProbePath = Resolve-ExactPath -Path $FfprobePath -Description 'ffprobe executable'
}

Invoke-MediaTask `
    -CliPath $cliPath `
    -Url $AppleHlsMediaUrl `
    -Directory $outputRoot `
    -FileName 'apple-bipbop.ts' `
    -Deadline ([DateTimeOffset]::UtcNow.AddMinutes($TimeoutMinutes)) `
    -ProbePath $resolvedProbePath `
    -MinimumDurationSeconds $MinimumHlsDurationSeconds `
    -MaximumDurationSeconds $MaximumHlsDurationSeconds
Invoke-MediaTask `
    -CliPath $cliPath `
    -Url $DashIfWaveUrl `
    -Directory $outputRoot `
    -FileName 'dashif-wave.mp4' `
    -Deadline ([DateTimeOffset]::UtcNow.AddMinutes($TimeoutMinutes)) `
    -ProbePath $resolvedProbePath `
    -MinimumDurationSeconds $MinimumDashDurationSeconds `
    -MaximumDurationSeconds $MaximumDashDurationSeconds

Write-Output 'PASS Stage 5 media runtime smoke. Open both retained artifacts and verify picture, sound, duration and seeking.'
