param(
    [string]$HostPath = "$PSScriptRoot\..\..\target\debug\nxtools-cli.exe",
    [string]$DownloadUrl = '',
    [string]$DownloadDirectory = "$env:TEMP\NxTools-Browser-Smoke",
    [switch]$TestBatch,
    [switch]$TestBlob
)

$ErrorActionPreference = 'Stop'
$resolvedHost = [System.IO.Path]::GetFullPath($HostPath)
if (-not (Test-Path -LiteralPath $resolvedHost -PathType Leaf)) {
    throw "Native Messaging host does not exist: $resolvedHost"
}

function Read-Exact([System.IO.Stream]$Stream, [int]$Length) {
    $buffer = [byte[]]::new($Length)
    $offset = 0
    while ($offset -lt $Length) {
        $read = $Stream.Read($buffer, $offset, $Length - $offset)
        if ($read -le 0) {
            throw 'Native Messaging host closed stdout before the frame completed.'
        }
        $offset += $read
    }
    return $buffer
}

function Invoke-NativeRequest(
    [System.IO.Stream]$InputStream,
    [System.IO.Stream]$OutputStream,
    [hashtable]$Request) {
    $json = $Request | ConvertTo-Json -Depth 20 -Compress
    $payload = [System.Text.Encoding]::UTF8.GetBytes($json)
    if ($payload.Length -gt 786432) {
        throw 'Smoke request exceeds the native host frame bound.'
    }
    $length = [BitConverter]::GetBytes([uint32]$payload.Length)
    $InputStream.Write($length, 0, $length.Length)
    $InputStream.Write($payload, 0, $payload.Length)
    $InputStream.Flush()
    $responseLength = [BitConverter]::ToUInt32((Read-Exact $OutputStream 4), 0)
    if ($responseLength -eq 0 -or $responseLength -gt 786432) {
        throw "Native host returned invalid response length $responseLength."
    }
    $responseJson = [System.Text.Encoding]::UTF8.GetString(
        (Read-Exact $OutputStream ([int]$responseLength)))
    return $responseJson | ConvertFrom-Json
}

$startInfo = [System.Diagnostics.ProcessStartInfo]::new()
$startInfo.FileName = $resolvedHost
$startInfo.ArgumentList.Add('browser-native-host')
$startInfo.UseShellExecute = $false
$startInfo.CreateNoWindow = $true
$startInfo.RedirectStandardInput = $true
$startInfo.RedirectStandardOutput = $true
$startInfo.RedirectStandardError = $true
$process = [System.Diagnostics.Process]::Start($startInfo)
if ($null -eq $process) {
    throw 'Windows did not start the Native Messaging host.'
}

try {
    $ping = Invoke-NativeRequest $process.StandardInput.BaseStream $process.StandardOutput.BaseStream @{
        protocolVersion = 1
        requestId = [Guid]::NewGuid().ToString()
        action = 'ping'
        payload = @{}
    }
    if (-not $ping.ok -or $ping.result.host -ne 'com.nxtools.next.browser') {
        throw "Native host ping failed: $($ping | ConvertTo-Json -Depth 10 -Compress)"
    }
    Write-Host 'PASS Native Messaging ping framing and correlation.'

    if ($DownloadUrl) {
        New-Item -ItemType Directory -Force -Path $DownloadDirectory | Out-Null
        $beforeSnapshot = (& $resolvedHost download list --json) | ConvertFrom-Json
        $beforeTaskIds = @($beforeSnapshot.result.tasks | ForEach-Object { $_.task.id })
        $idempotencyKey = "smoke:$([Guid]::NewGuid())"
        $request = @{
            protocolVersion = 1
            requestId = [Guid]::NewGuid().ToString()
            idempotencyKey = $idempotencyKey
            action = 'create_task'
            payload = @{
                payload = @{
                    url = $DownloadUrl
                    fileName = 'browser-native-smoke.bin'
                    saveDirectory = [System.IO.Path]::GetFullPath($DownloadDirectory)
                    openDialog = $true
                    startImmediately = $false
                    headers = @{ Referer = 'https://example.test/download-page' }
                    cookie = 'smoke_cookie=redacted-fixture'
                    size = 104857600
                    supportsRange = $true
                }
            }
        }
        $first = Invoke-NativeRequest $process.StandardInput.BaseStream $process.StandardOutput.BaseStream $request
        $request.requestId = [Guid]::NewGuid().ToString()
        $second = Invoke-NativeRequest $process.StandardInput.BaseStream $process.StandardOutput.BaseStream $request
        if (-not $first.ok -or -not $second.ok) {
            throw 'Native host did not open the download confirmation window.'
        }
        if (-not $first.result.data.dialogOpened -or
            $first.result.data.dialogRequestId -ne $second.result.data.dialogRequestId) {
            throw 'Duplicate Native Messaging requests did not reuse the correlated dialog request.'
        }
        if ($null -ne $first.result.data.taskId) {
            throw 'Opening the browser confirmation window must not create a queued task.'
        }
        $afterDialogSnapshot = (& $resolvedHost download list --json) | ConvertFrom-Json
        $newDialogTasks = @($afterDialogSnapshot.result.tasks | Where-Object {
            $beforeTaskIds -notcontains $_.task.id
        })
        if ($newDialogTasks.Count -ne 0) {
            throw 'The browser dialog handoff persisted a task before user confirmation.'
        }
        Write-Host "PASS Native Messaging visible dialog replay without premature task creation: dialog=$($first.result.data.dialogRequestId)"
        $dismiss = Invoke-NativeRequest $process.StandardInput.BaseStream $process.StandardOutput.BaseStream @{
            protocolVersion = 1
            requestId = [Guid]::NewGuid().ToString()
            action = 'dismiss_dialog'
            payload = @{ dialogRequestId = $first.result.data.dialogRequestId }
        }
        if (-not $dismiss.ok -or -not $dismiss.result.data.dismissed) {
            throw 'Native host did not compensate the visible download dialog.'
        }
        Write-Host 'PASS Native Messaging dialog dismissal compensation.'

        if ($TestBatch) {
            $batchKey = "smoke-batch:$([Guid]::NewGuid())"
            $batchRequest = @{
                protocolVersion = 1
                requestId = [Guid]::NewGuid().ToString()
                idempotencyKey = $batchKey
                action = 'create_batch'
                payload = @{
                    items = @(
                        @{
                            url = $DownloadUrl
                            fileName = 'browser-batch-smoke-1.bin'
                            saveDirectory = [System.IO.Path]::GetFullPath($DownloadDirectory)
                            openDialog = $false
                            startImmediately = $false
                            headers = @{}
                        },
                        @{
                            url = $DownloadUrl
                            fileName = 'browser-batch-smoke-2.bin'
                            saveDirectory = [System.IO.Path]::GetFullPath($DownloadDirectory)
                            openDialog = $false
                            startImmediately = $false
                            headers = @{}
                        }
                    )
                }
            }
            $batchFirst = Invoke-NativeRequest `
                $process.StandardInput.BaseStream `
                $process.StandardOutput.BaseStream `
                $batchRequest
            $batchRequest.requestId = [Guid]::NewGuid().ToString()
            $batchSecond = Invoke-NativeRequest `
                $process.StandardInput.BaseStream `
                $process.StandardOutput.BaseStream `
                $batchRequest
            $firstIds = @($batchFirst.result.data.results | ForEach-Object { $_.data.taskId })
            $secondIds = @($batchSecond.result.data.results | ForEach-Object { $_.data.taskId })
            if (-not $batchFirst.ok -or -not $batchSecond.ok -or $firstIds.Count -ne 2) {
                throw 'Native host did not accept the two-item batch.'
            }
            if (($firstIds -join ',') -ne ($secondIds -join ',')) {
                throw 'Duplicate Native Messaging batches created different task IDs.'
            }
            if ($firstIds[0] -eq $firstIds[1]) {
                throw 'Native Messaging batch items did not receive distinct task IDs.'
            }
            Write-Host "PASS Native Messaging batch and durable replay: tasks=$($firstIds -join ',')"
        }

        if ($TestBlob) {
            $blobBytes = [System.Text.Encoding]::UTF8.GetBytes('NxTools browser Blob smoke')
            $transferId = [Guid]::NewGuid().ToString()
            $blobName = "$transferId.bin"
            $blobPath = Join-Path ([System.IO.Path]::GetFullPath($DownloadDirectory)) $blobName
            $blobMetadata = @{
                url = 'blob:https://example.test/smoke'
                fileName = $blobName
                saveDirectory = [System.IO.Path]::GetFullPath($DownloadDirectory)
                openDialog = $false
                startImmediately = $true
                headers = @{}
            }
            $begin = Invoke-NativeRequest $process.StandardInput.BaseStream $process.StandardOutput.BaseStream @{
                protocolVersion = 1
                requestId = [Guid]::NewGuid().ToString()
                idempotencyKey = "smoke-blob:$transferId"
                action = 'blob_begin'
                payload = @{
                    transferId = $transferId
                    size = $blobBytes.Length
                    download = $blobMetadata
                }
            }
            $chunk = Invoke-NativeRequest $process.StandardInput.BaseStream $process.StandardOutput.BaseStream @{
                protocolVersion = 1
                requestId = [Guid]::NewGuid().ToString()
                action = 'blob_chunk'
                payload = @{
                    transferId = $transferId
                    index = 0
                    data = [Convert]::ToBase64String($blobBytes)
                }
            }
            $commit = Invoke-NativeRequest $process.StandardInput.BaseStream $process.StandardOutput.BaseStream @{
                protocolVersion = 1
                requestId = [Guid]::NewGuid().ToString()
                action = 'blob_commit'
                payload = @{ transferId = $transferId }
            }
            if (-not $begin.ok -or -not $chunk.ok -or -not $commit.ok) {
                $diagnostic = @{
                    begin = @{ ok = $begin.ok; error = $begin.error }
                    chunk = @{ ok = $chunk.ok; error = $chunk.error }
                    commit = @{ ok = $commit.ok; error = $commit.error }
                } | ConvertTo-Json -Depth 8 -Compress
                throw "Native host did not accept the Blob transfer: $diagnostic"
            }
            $blobTaskId = $commit.result.data.taskId
            $deadline = [DateTime]::UtcNow.AddSeconds(15)
            do {
                $snapshotJson = & $resolvedHost download list --json
                if ($LASTEXITCODE -ne 0) {
                    throw 'NxTools CLI could not read the Blob task snapshot.'
                }
                $snapshot = $snapshotJson | ConvertFrom-Json
                $blobTask = @($snapshot.result.tasks | Where-Object { $_.task.id -eq $blobTaskId }) | Select-Object -First 1
                if ($blobTask -and $blobTask.task.status -eq 'failed') {
                    throw "Blob task failed: $($blobTask.error | ConvertTo-Json -Depth 8 -Compress)"
                }
                if ($blobTask -and $blobTask.task.status -eq 'completed') {
                    break
                }
                Start-Sleep -Milliseconds 100
            } while ([DateTime]::UtcNow -lt $deadline)
            if (-not $blobTask -or $blobTask.task.status -ne 'completed') {
                throw 'Blob task did not complete before the smoke-test deadline.'
            }
            $actualBlob = [System.IO.File]::ReadAllBytes($blobPath)
            if (-not [System.Linq.Enumerable]::SequenceEqual[byte]($blobBytes, $actualBlob)) {
                throw 'Blob task output did not match the staged bytes.'
            }
            Write-Host "PASS Native Messaging Blob staging, range transfer and atomic output: task=$blobTaskId"
        }
    }
}
finally {
    $process.StandardInput.Close()
    if (-not $process.WaitForExit(5000)) {
        $process.Kill($true)
        $process.WaitForExit()
    }
    $process.Dispose()
}
