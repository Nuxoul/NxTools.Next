param(
    [string]$Query = "NxTools.Indexer.exe",
    [ValidateRange(1, 200)]
    [int]$Limit = 20,
    [int]$TimeoutMs = 10000
)

$ErrorActionPreference = "Stop"
$serviceName = 'NxToolsIndexerV1'

function Read-LineWithTimeout(
    [System.IO.StreamReader]$Reader,
    [int]$Timeout,
    [string]$Stage) {
    $read = $Reader.ReadLineAsync()
    if (-not $read.Wait($Timeout)) {
        throw "Indexer $Stage exceeded ${Timeout}ms."
    }
    if ($null -eq $read.Result) {
        throw "Indexer closed the pipe during $Stage."
    }
    return $read.Result
}

$expectedExecutable = [System.IO.Path]::GetFullPath(
    (Join-Path $env:ProgramData 'NxTools\NxTools.Next\Service\NxTools.Indexer.exe'))
$packagedExecutable = [System.IO.Path]::GetFullPath(
    (Join-Path $PSScriptRoot '..\..\artifacts\win-x64\NxTools.Indexer.exe'))
$service = Get-CimInstance Win32_Service -Filter "Name='$serviceName'"
if (-not $service) {
    throw "Service '$serviceName' is not installed."
}
$registeredExecutable = if ($service.PathName -match '^"([^"]+)"') {
    $Matches[1]
}
else {
    ($service.PathName -split '\s+', 2)[0]
}
if ([System.IO.Path]::GetFullPath($registeredExecutable) -ne $expectedExecutable) {
    throw "Service '$serviceName' points to '$registeredExecutable', expected '$expectedExecutable'."
}
$serviceKey = Get-ItemProperty "HKLM:\SYSTEM\CurrentControlSet\Services\$serviceName"
$approvedHash = [string]$serviceKey.NxToolsBinarySha256
$packagedHash = (Get-FileHash -LiteralPath $packagedExecutable -Algorithm SHA256).Hash
$protectedHash = (Get-FileHash -LiteralPath $expectedExecutable -Algorithm SHA256).Hash
if (-not $approvedHash -or
    $approvedHash -ne $packagedHash -or
    $approvedHash -ne $protectedHash) {
    throw "Running protected indexer does not match the packaged indexer."
}
if ($service.StartName -ne 'LocalSystem') {
    throw "Service '$serviceName' runs as '$($service.StartName)', expected LocalSystem."
}
$pipe = [System.IO.Pipes.NamedPipeClientStream]::new(
    ".",
    "nxtools-next-indexer-v1",
    [System.IO.Pipes.PipeDirection]::InOut,
    [System.IO.Pipes.PipeOptions]::Asynchronous)

try {
    $pipe.Connect(3000)
    $writer = [System.IO.StreamWriter]::new(
        $pipe,
        [System.Text.UTF8Encoding]::new($false),
        4096,
        $true)
    $writer.AutoFlush = $true
    $reader = [System.IO.StreamReader]::new(
        $pipe,
        [System.Text.UTF8Encoding]::new($false),
        $false,
        4096,
        $true)

    $hello = @{
        protocol_version = 1
        id = [guid]::NewGuid().ToString()
        correlation_id = $null
        type = "hello"
        payload = @{
            client_name = "indexer_service_smoke"
            client_version = "0.1.0"
            capabilities = @("index_status", "file_search")
        }
    } | ConvertTo-Json -Compress -Depth 8
    $writer.WriteLine($hello)
    $null = Read-LineWithTimeout $reader 3000 "handshake"
    $status = Read-LineWithTimeout $reader 3000 "initial status" |
        ConvertFrom-Json
    $statusData = $status.payload.data
    if ($statusData.phase -ne "ready" -or
        $statusData.indexed_items -le 0 -or
        $statusData.last_error) {
        throw "Indexer is not healthy: phase=$($statusData.phase), indexed_items=$($statusData.indexed_items), last_error=$($statusData.last_error)"
    }

    $search = @{
        protocol_version = 1
        id = [guid]::NewGuid().ToString()
        correlation_id = $null
        type = "indexer_command"
        payload = @{
            command = "search"
            data = @{
                generation = 1
                query = $Query
                limit = $Limit
                context = @{
                    directory = $null
                    include_files = $true
                    include_directories = $true
                }
            }
        }
    } | ConvertTo-Json -Compress -Depth 8
    $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
    $writer.WriteLine($search)
    $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMs)
    $batch = $null
    $items = @()
    while ([DateTime]::UtcNow -lt $deadline -and $null -eq $batch) {
        $remaining = [Math]::Max(1, [int]($deadline - [DateTime]::UtcNow).TotalMilliseconds)
        $responseTask = $reader.ReadLineAsync()
        if (-not $responseTask.Wait($remaining)) {
            break
        }
        if ($null -eq $responseTask.Result) {
            throw "Indexer closed the pipe before returning '$Query'"
        }
        $frame = $responseTask.Result | ConvertFrom-Json
        if ($frame.type -eq 'indexer_event' -and
            $frame.payload.event -eq 'search_batch' -and
            $frame.payload.data.generation -eq 1) {
            $items += @($frame.payload.data.items)
            if ($frame.payload.data.complete) {
                $batch = $frame
            }
        }
    }
    if ($null -eq $batch) {
        throw "Indexer search exceeded ${TimeoutMs}ms for '$Query'"
    }
    $stopwatch.Stop()
    if ($items.Count -eq 0) {
        throw "Indexer returned no file or folder results for '$Query'"
    }

    [pscustomobject]@{
        Phase = $statusData.phase
        IndexedItems = $statusData.indexed_items
        Query = $Query
        ResultCount = $items.Count
        WallMs = $stopwatch.ElapsedMilliseconds
        ServiceElapsedUs = $batch.payload.data.elapsed_us
        FirstPath = $items[0].path
        FirstIsDirectory = $items[0].is_directory
    }
}
finally {
    # A busy service session deliberately closes the pipe after the handshake so
    # that reconnecting clients retry instead of waiting behind an index build.
    # Cleanup must not hide the actual assertion/result with a broken-pipe error.
    if ($reader) {
        try { $reader.Dispose() } catch [System.IO.IOException] { }
    }
    if ($writer) {
        try { $writer.Dispose() } catch [System.IO.IOException] { }
    }
    try { $pipe.Dispose() } catch [System.IO.IOException] { }
}
