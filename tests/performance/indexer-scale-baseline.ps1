param(
    [int[]]$EntryCounts = @(1000000, 3000000),
    [string]$OutputPath = "artifacts/performance/indexer-scale-baseline.json"
)

$ErrorActionPreference = "Stop"
$repository = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$results = @()

Push-Location $repository
try {
    foreach ($entryCount in $EntryCounts) {
        if ($entryCount -notin @(1000000, 3000000)) {
            throw "EntryCounts only accepts the phase-two gates 1000000 and 3000000."
        }
        $env:NXTOOLS_INDEX_BENCH_ENTRIES = $entryCount.ToString()
        $output = & cargo test --release -p nxtools-indexer synthetic_index_scale_baseline -- --ignored --nocapture --test-threads=1 2>&1
        if ($LASTEXITCODE -ne 0) {
            $output | Write-Host
            throw "Scale baseline failed for $entryCount entries."
        }
        $line = $output | Where-Object { $_ -match "NXTOOLS_SCALE_BASELINE" } | Select-Object -Last 1
        if (-not $line) {
            throw "Scale baseline emitted no metrics for $entryCount entries."
        }
        $json = [regex]::Match($line, 'NXTOOLS_SCALE_BASELINE (\{.+\})').Groups[1].Value
        $results += $json | ConvertFrom-Json
    }
}
finally {
    Remove-Item Env:NXTOOLS_INDEX_BENCH_ENTRIES -ErrorAction SilentlyContinue
    Pop-Location
}

$absoluteOutput = if ([System.IO.Path]::IsPathRooted($OutputPath)) {
    $OutputPath
} else {
    Join-Path $repository $OutputPath
}
$directory = Split-Path -Parent $absoluteOutput
New-Item -ItemType Directory -Force -Path $directory | Out-Null
$record = [ordered]@{
    recorded_at = [DateTimeOffset]::Now.ToString("o")
    configuration = "release"
    machine = [Environment]::MachineName
    processor_count = [Environment]::ProcessorCount
    results = $results
}
$record | ConvertTo-Json -Depth 6 | Set-Content -Encoding utf8 $absoluteOutput
$record | ConvertTo-Json -Depth 6
