param(
    [int]$Iterations = 100,
    [string]$FinalQuery = "quick-launch-plan",
    [int]$TimeoutMs = 10000
)

$ErrorActionPreference = "Stop"
$pipe = [System.IO.Pipes.NamedPipeClientStream]::new(
    ".",
    "nxtools-next-indexer-v1",
    [System.IO.Pipes.PipeDirection]::InOut,
    [System.IO.Pipes.PipeOptions]::Asynchronous)

try {
    $pipe.Connect(3000)
    $writer = [System.IO.StreamWriter]::new($pipe, [System.Text.UTF8Encoding]::new($false), 4096, $true)
    $writer.AutoFlush = $true
    $reader = [System.IO.StreamReader]::new($pipe, [System.Text.UTF8Encoding]::new($false), $false, 4096, $true)

    $hello = @{
        protocol_version = 1
        id = [guid]::NewGuid().ToString()
        correlation_id = $null
        type = "hello"
        payload = @{
            client_name = "indexer_latest_query_stress"
            client_version = "0.1.0"
            capabilities = @("index_status", "file_search")
        }
    } | ConvertTo-Json -Compress -Depth 8
    $writer.WriteLine($hello)
    $null = $reader.ReadLine()
    $status = $reader.ReadLine() | ConvertFrom-Json
    if ($status.payload.data.phase -ne "ready") {
        throw "Indexer is not ready: $($status.payload.data.phase)"
    }

    for ($generation = 1; $generation -le $Iterations; $generation++) {
        $query = if ($generation -eq $Iterations) {
            $FinalQuery
        } else {
            "quick-launch-plan".Substring(0, 1 + (($generation - 1) % 16))
        }
        $search = @{
            protocol_version = 1
            id = [guid]::NewGuid().ToString()
            correlation_id = $null
            type = "indexer_command"
            payload = @{
                command = "search"
                data = @{
                    generation = $generation
                    query = $query
                    limit = 20
                    context = @{
                        directory = $null
                        include_files = $true
                        include_directories = $true
                    }
                }
            }
        } | ConvertTo-Json -Compress -Depth 8
        $writer.WriteLine($search)
    }

    $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMs)
    $latest = $null
    $batches = 0
    while ([DateTime]::UtcNow -lt $deadline -and $null -eq $latest) {
        $remaining = [Math]::Max(1, [int]($deadline - [DateTime]::UtcNow).TotalMilliseconds)
        $read = $reader.ReadLineAsync()
        if (-not $read.Wait($remaining)) {
            break
        }
        if ($null -eq $read.Result) {
            throw "Indexer closed the pipe before publishing generation $Iterations"
        }
        $frame = $read.Result | ConvertFrom-Json
        if ($frame.type -ne "indexer_event" -or $frame.payload.event -ne "search_batch") {
            continue
        }
        $batches++
        if ($frame.payload.data.generation -eq $Iterations -and $frame.payload.data.complete) {
            $latest = $frame.payload.data
        }
    }
    if ($null -eq $latest) {
        throw "Latest generation $Iterations was not published; observed_batches=$batches"
    }
    $items = @($latest.items)
    if ($items.Count -eq 0) {
        throw "Latest generation $Iterations published no file/folder results for '$FinalQuery'"
    }

    [pscustomobject]@{
        Iterations = $Iterations
        ObservedBatches = $batches
        LatestGeneration = $latest.generation
        ResultCount = $items.Count
        FirstPath = $items[0].path
    }
}
finally {
    if ($reader) { try { $reader.Dispose() } catch [System.IO.IOException] { } }
    if ($writer) { try { $writer.Dispose() } catch [System.IO.IOException] { } }
    try { $pipe.Dispose() } catch [System.IO.IOException] { }
}
