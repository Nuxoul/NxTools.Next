# Phase 7 baseline: CPU, memory, handles, disk write and database WAL
# measurements for the packaged desktop + indexer while a real download
# runs. Run against the packaged artifacts (no -RestartService needed).
param(
    [string]$RepositoryRoot = '',
    [string]$CliPath = '',
    [switch]$Json
)

$ErrorActionPreference = 'Stop'

if (-not $RepositoryRoot) {
    $RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
}
if (-not $CliPath) {
    $CliPath = Join-Path $RepositoryRoot "artifacts\win-x64\NxTools.Cli.exe"
}

$packageDir = Join-Path $RepositoryRoot "artifacts\win-x64"
if (-not (Test-Path $CliPath)) {
    throw "CLI not found at $CliPath - build the package first."
}

function Get-ProcessSample([string]$Name) {
    $processes = Get-Process -Name $Name -ErrorAction SilentlyContinue
    if (-not $processes) { return $null }
    $handles = ($processes | Measure-Object -Property Handles -Sum).Sum
    $memoryMb = [math]::Round(
        (($processes | Measure-Object -Property WorkingSet64 -Sum).Sum / 1MB), 1)
    $cpuSeconds = [math]::Round(
        (($processes | Measure-Object -Property CPU -Sum).Sum), 1)
    return [PSCustomObject]@{
        processes = @($processes).Count
        handles = $handles
        memory_mb = $memoryMb
        cpu_seconds = $cpuSeconds
    }
}

function Get-WalSize([string]$DataDir) {
    $wal = Join-Path $DataDir 'downloads.schema-4.sqlite3-wal'
    if (Test-Path $wal) { return [math]::Round((Get-Item $wal).Length / 1KB, 1) }
    return 0
}

$desktopData = Join-Path $env:LOCALAPPDATA 'NxTools\NxTools.Next\data'
$testDir = Join-Path $env:TEMP ("nxtools-baseline-" + [guid]::NewGuid().ToString('N').Substring(0, 8))
New-Item -ItemType Directory -Path $testDir -Force | Out-Null

$baseline = [PSCustomObject]@{
    started_at = (Get-Date).ToString('o')
    idle = $null
    download = $null
    download_url = 'https://github.com/git-lfs/git-lfs/releases/download/v3.5.1/git-lfs-windows-amd64-v3.5.1.zip'
}

# idle sample
Start-Sleep -Seconds 3
$baseline.idle = [PSCustomObject]@{
    desktop = Get-ProcessSample 'NxTools'
    platform_host = Get-ProcessSample 'NxTools.PlatformHost'
    indexer = Get-ProcessSample 'NxTools.Indexer'
    browser_host = Get-ProcessSample 'NxTools.BrowserHost'
    wal_kb = Get-WalSize $desktopData
}

# download sample (10s into a real download)
$task = & $CliPath download add `
    'https://github.com/git-lfs/git-lfs/releases/download/v3.5.1/git-lfs-windows-amd64-v3.5.1.zip' `
    --dir $testDir --name 'baseline-git-lfs.zip' --json | ConvertFrom-Json
if (-not $task.ok) { throw 'CLI rejected the baseline download task.' }
$taskId = $task.result.task_id
Start-Sleep -Seconds 10
$baseline.download = [PSCustomObject]@{
    desktop = Get-ProcessSample 'NxTools'
    platform_host = Get-ProcessSample 'NxTools.PlatformHost'
    indexer = Get-ProcessSample 'NxTools.Indexer'
    wal_kb = Get-WalSize $desktopData
    task_status = (& $CliPath download show $taskId --json | ConvertFrom-Json).result.task.status
}

# cleanup: cancel + remove + delete the staging file
& $CliPath download cancel $taskId --json | Out-Null
& $CliPath download remove $taskId --delete-file --json | Out-Null
try { Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue } catch {}

if ($Json) {
    $baseline | ConvertTo-Json -Depth 5
} else {
    Write-Host '=== Phase 7 baseline ==='
    Write-Host ('idle : desktop=' + ($baseline.idle.desktop | ConvertTo-Json -Compress))
    Write-Host ('       indexer=' + ($baseline.idle.indexer | ConvertTo-Json -Compress))
    Write-Host ('       wal_kb=' + $baseline.idle.wal_kb)
    Write-Host ('dl10s: desktop=' + ($baseline.download.desktop | ConvertTo-Json -Compress))
    Write-Host ('       wal_kb=' + $baseline.download.wal_kb + ' status=' + $baseline.download.task_status)
    Write-Host 'Baseline captured.'
}
