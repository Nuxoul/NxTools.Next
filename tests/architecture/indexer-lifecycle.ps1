$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$runtime = Get-Content -Raw -LiteralPath (Join-Path $root 'rust\apps\nxtools-desktop\src\runtime.rs')
$ipc = Get-Content -Raw -LiteralPath (Join-Path $root 'rust\crates\nxtools-ipc\src\lib.rs')
$localPipe = Get-Content -Raw -LiteralPath (Join-Path $root 'rust\crates\nxtools-ipc\src\local_pipe.rs')
$indexer = Get-Content -Raw -LiteralPath (Join-Path $root 'rust\apps\nxtools-indexer\src\lib.rs')
$mft = Get-Content -Raw -LiteralPath (Join-Path $root 'rust\apps\nxtools-indexer\src\mft.rs')
$indexerMain = Get-Content -Raw -LiteralPath (Join-Path $root 'rust\apps\nxtools-indexer\src\main.rs')
$serviceHost = Get-Content -Raw -LiteralPath (Join-Path $root 'rust\apps\nxtools-indexer\src\windows_service_host.rs')
$quickLaunch = Get-Content -Raw -LiteralPath (Join-Path $root 'rust\apps\nxtools-desktop\src\quick_launch.rs')
$finder = Get-Content -Raw -LiteralPath (Join-Path $root 'rust\crates\nxtools-finder\src\lib.rs')

if ($runtime -notmatch 'NxTools\.Indexer\.exe' -or
    $runtime -notmatch 'connect_local_indexer_pipe' -or
    $runtime -notmatch 'supervise_indexer_service' -or
    $runtime -match '(?s)Command::new\([^)]*indexer.*?stdin\(Stdio::piped\(\)\)') {
    throw 'Indexer lifecycle regression: the ordinary desktop must reconnect to the service pipe and must never spawn the MFT indexer as a privileged child.'
}

if ($indexerMain -match 'println!\s*\(' -or
    $indexerMain -notmatch '--service' -or
    $serviceHost -notmatch 'service_dispatcher::start' -or
    $serviceHost -notmatch 'serve_session') {
    throw 'Indexer service regression: the Rust executable must expose the bounded protocol through a real Windows service entrypoint.'
}

if ($localPipe -notmatch 'PIPE_REJECT_REMOTE_CLIENTS' -or
    $localPipe -notmatch 'ConvertStringSecurityDescriptorToSecurityDescriptorW' -or
    $localPipe -notmatch 'AU\)' -or
    $localPipe -notmatch '#!\[allow\(unsafe_code\)\]') {
    throw 'Indexer pipe regression: the LocalSystem service pipe must reject remote clients, use an authenticated-user DACL and isolate Win32 unsafe code.'
}

if ($ipc -notmatch '(?s)read_bounded_sync_frame.*?MAX_FRAME_BYTES' -or
    $indexer -notmatch 'file_system\.eq_ignore_ascii_case\("ntfs"\)' -or
    $indexer -notmatch 'mpsc::sync_channel\(RUNTIME_QUEUE_CAPACITY\)') {
    throw 'Indexer discovery regression: frames must remain bounded and volume status must be based on real NTFS discovery.'
}

if ($runtime -notmatch 'IndexerNotification::Disconnected' -or
    $quickLaunch -notmatch '(?s)IndexerNotification::Disconnected.*?index_status\s*=\s*None') {
    throw 'Indexer lifecycle regression: a disconnect must invalidate stale search results and visible volume status.'
}

if ($quickLaunch -notmatch '(?s)fn apply\(&mut self, batch: IndexSearchBatch\).*?batch\.generation != self\.generation') {
    throw 'Indexer query regression: the desktop must reject search batches from a stale generation.'
}

if ($mft -notmatch 'FSCTL_ENUM_USN_DATA' -or
    $mft -notmatch 'parse_enumeration_batch' -or
    $mft -notmatch 'snapshot_from_records' -or
    $mft -match '(?m)std::fs::read_dir|walkdir|jwalk' -or
    $indexer -notmatch 'spawn_snapshot_build' -or
    $indexer -notmatch 'struct SearchExecutor' -or
    $indexer -notmatch 'pending: Option<SearchJob>' -or
    $indexer -notmatch 'MAX_SEARCH_RESULTS' -or
    $indexer -notmatch 'AtomicU64' -or
    $indexerMain -match 'rebuild_system\(\)' -or
    $quickLaunch -notmatch 'ExternalSearchEntry' -or
    $quickLaunch -notmatch 'search_with_external' -or
    $finder -notmatch 'pub fn search_with_external') {
    throw 'MFT snapshot regression: file results must come from the background bounded Rust MFT reader and reach the standalone launcher without a directory-walk fallback.'
}

$externalEntry = [regex]::Match($finder, '(?s)pub struct ExternalSearchEntry\s*\{.*?\}').Value
if (-not $externalEntry -or $externalEntry -match '\bscore\b') {
    throw 'Finder ranking regression: provider-local scores must not cross the MFT-to-finder boundary.'
}

if ($mft -notmatch '(?s)#\[allow\(unsafe_code\)\]\s*mod windows' -or
    $mft -match '(?s)#\[allow\(unsafe_code\)\].*?#\[allow\(unsafe_code\)\]') {
    throw 'MFT safety regression: Win32 unsafe code must stay isolated to one reviewed adapter module.'
}

$csharpSource = Get-ChildItem -LiteralPath (Join-Path $root 'csharp\NxTools.PlatformHost') -Recurse -Filter '*.cs' |
    ForEach-Object { Get-Content -Raw -LiteralPath $_.FullName }
$joinedCsharp = $csharpSource -join "`n"
if ($joinedCsharp -match 'IndexSearchBatch|IndexSearchRequest|MftSnapshot|FSCTL_ENUM_USN_DATA') {
    throw 'Architecture regression: file indexing and query generations belong to Rust, not PlatformHost.'
}
if ($joinedCsharp -notmatch 'ensure_indexer_service' -or
    $joinedCsharp -notmatch 'obj=", "LocalSystem' -or
    $joinedCsharp -notmatch 'Verb = "runas"') {
    throw 'Indexer deployment regression: only the C# Windows adapter may perform the one-time LocalSystem SCM/UAC installation.'
}
$rustServiceName = [regex]::Match($serviceHost, 'SERVICE_NAME:\s*&str\s*=\s*"([^"]+)"').Groups[1].Value
$csharpServiceName = [regex]::Match($joinedCsharp, 'ServiceName\s*=\s*"([^"]+)"').Groups[1].Value
if (-not $rustServiceName -or $rustServiceName -ne $csharpServiceName) {
    throw 'Indexer deployment regression: Rust dispatcher and C# installer service names must remain identical.'
}

Write-Host 'PASS Rust LocalSystem index service, authenticated local pipe, MFT lifecycle, generation gate and C# deployment boundary'
