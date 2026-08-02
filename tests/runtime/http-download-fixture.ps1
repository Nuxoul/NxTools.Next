param(
    [Parameter(Mandatory = $true)]
    [int]$Port,
    [Parameter(Mandatory = $true)]
    [string]$ReadyFile,
    [int]$PayloadBytes = 33554432,
    [int]$ChunkBytes = 65536,
    [int]$DelayMilliseconds = 25
)

$ErrorActionPreference = 'Stop'
$listener = [System.Net.HttpListener]::new()
$listener.Prefixes.Add("http://127.0.0.1:$Port/")
$listener.Start()
Set-Content -LiteralPath $ReadyFile -Value "ready" -Encoding ascii
$buffer = [byte[]]::new($ChunkBytes)

try {
    while ($listener.IsListening) {
        $context = $listener.GetContext()
        try {
            if ($context.Request.Url.AbsolutePath -ne '/nxtools-ui-smoke.bin') {
                $context.Response.StatusCode = 404
                $context.Response.Close()
                continue
            }
            $context.Response.StatusCode = 200
            $context.Response.ContentType = 'application/octet-stream'
            $context.Response.ContentLength64 = $PayloadBytes
            $context.Response.Headers['Content-Disposition'] =
                'attachment; filename="nxtools-ui-smoke.bin"'
            if ($context.Request.HttpMethod -eq 'HEAD') {
                $context.Response.Close()
                continue
            }
            $remaining = $PayloadBytes
            while ($remaining -gt 0) {
                $count = [Math]::Min($remaining, $buffer.Length)
                $context.Response.OutputStream.Write($buffer, 0, $count)
                $context.Response.OutputStream.Flush()
                $remaining -= $count
                Start-Sleep -Milliseconds $DelayMilliseconds
            }
            $context.Response.Close()
        }
        catch {
            try { $context.Response.Abort() } catch { }
        }
    }
}
finally {
    $listener.Stop()
    $listener.Close()
}
