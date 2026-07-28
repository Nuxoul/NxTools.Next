param(
    [int]$Iterations = 50,
    [string]$Query = "quick-launch-plan",
    [int]$TimeoutMs = 10000
)

$ErrorActionPreference = "Stop"

for ($iteration = 1; $iteration -le $Iterations; $iteration++) {
    $pipe = [System.IO.Pipes.NamedPipeClientStream]::new(
        ".",
        "nxtools-next-indexer-v1",
        [System.IO.Pipes.PipeDirection]::InOut,
        [System.IO.Pipes.PipeOptions]::Asynchronous)
    $reader = $null
    $writer = $null
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
                client_name = "indexer_reconnect_stress"
                client_version = "0.1.0"
                capabilities = @("index_status", "file_search")
            }
        } | ConvertTo-Json -Compress -Depth 8
        $writer.WriteLine($hello)

        $helloAck = $reader.ReadLine()
        $statusLine = $reader.ReadLine()
        if ($null -eq $helloAck -or $null -eq $statusLine) {
            throw "Session $iteration ended during handshake"
        }
        $status = $statusLine | ConvertFrom-Json
        $readyDeadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMs)
        while ($status.payload.data.phase -ne "ready" -and [DateTime]::UtcNow -lt $readyDeadline) {
            $remaining = [Math]::Max(
                1,
                [int]($readyDeadline - [DateTime]::UtcNow).TotalMilliseconds)
            $read = $reader.ReadLineAsync()
            if (-not $read.Wait($remaining) -or $null -eq $read.Result) {
                break
            }
            $candidate = $read.Result | ConvertFrom-Json
            if ($candidate.type -eq "indexer_event" -and $candidate.payload.event -eq "status") {
                $status = $candidate
            }
        }
        if ($status.payload.data.phase -ne "ready") {
            throw "Session $iteration did not become ready: $($status.payload.data.phase)"
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
                    limit = 20
                    context = @{
                        directory = $null
                        include_files = $true
                        include_directories = $true
                    }
                }
            }
        } | ConvertTo-Json -Compress -Depth 8
        $searchTimer = [Diagnostics.Stopwatch]::StartNew()
        $writer.WriteLine($search)

        $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMs)
        $batch = $null
        while ([DateTime]::UtcNow -lt $deadline -and $null -eq $batch) {
            $remaining = [Math]::Max(1, [int]($deadline - [DateTime]::UtcNow).TotalMilliseconds)
            $read = $reader.ReadLineAsync()
            if (-not $read.Wait($remaining)) {
                break
            }
            if ($null -eq $read.Result) {
                throw "Session $iteration closed before returning the search batch"
            }
            $frame = $read.Result | ConvertFrom-Json
            if ($frame.type -eq "indexer_event" -and
                $frame.payload.event -eq "search_batch" -and
                $frame.payload.data.complete) {
                $batch = $frame.payload.data
            }
        }
        if ($null -eq $batch) {
            throw "Session $iteration timed out waiting for its search batch"
        }
        if (@($batch.items).Count -eq 0) {
            throw "Session $iteration returned no results for '$Query'"
        }
        $searchTimer.Stop()
        $lastSearchElapsedUs = [long]$batch.elapsed_us
        $lastClientElapsedMs = $searchTimer.Elapsed.TotalMilliseconds
    }
    finally {
        if ($reader) { try { $reader.Dispose() } catch [System.IO.IOException] { } }
        if ($writer) { try { $writer.Dispose() } catch [System.IO.IOException] { } }
        try { $pipe.Dispose() } catch [System.IO.IOException] { }
    }
}

[pscustomobject]@{
    Iterations = $Iterations
    Query = $Query
    SearchElapsedUs = $lastSearchElapsedUs
    ClientElapsedMs = [math]::Round($lastClientElapsedMs, 2)
    Status = "all reconnect sessions returned file/folder results"
}
