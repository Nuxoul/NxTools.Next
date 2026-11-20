param(
    [ValidateSet('Uninstall', 'Upgrade', 'Repair')]
    [string]$Operation = 'Uninstall',
    [string]$InstallDirectory,
    [string]$ConfigRoot,
    [string]$DataRoot,
    [string]$CacheRoot,
    [string]$LogRoot,
    [string]$LocalProductRoot,
    [string]$CloseLibraryPath,
    [switch]$RemovePersonalData,
    [string]$CorrelationId = [Guid]::NewGuid().ToString('D'),
    [string]$FixtureRoot = '',
    [string]$FixtureEffectsPath = ''
)
$ErrorActionPreference = 'Stop'

function Get-NxToolsFullPath([string]$Path) {
    if ([string]::IsNullOrWhiteSpace($Path)) { throw [ArgumentException]::new('A required uninstall path was empty.') }
    [IO.Path]::GetFullPath($Path)
}

function Test-NxToolsAbsolutePath([string]$Path) {
    return $Path -match '^[A-Za-z]:[\\/]' -or $Path.StartsWith('\\', [StringComparison]::Ordinal)
}
function Test-NxToolsSameOrDescendant([string]$Path, [string]$Root) {
    $candidate = (Get-NxToolsFullPath $Path).TrimEnd('\')
    $owner = (Get-NxToolsFullPath $Root).TrimEnd('\')
    return $candidate.Equals($owner, [StringComparison]::OrdinalIgnoreCase) -or
        $candidate.StartsWith($owner + '\', [StringComparison]::OrdinalIgnoreCase)
}

function Stop-NxToolsOwnedProcesses([string]$InstallDirectory, [string]$CloseLibraryPath) {
    if (!(Test-Path -LiteralPath $CloseLibraryPath -PathType Leaf)) {
        throw [IO.FileNotFoundException]::new('The installed exact-process cleanup library is unavailable.')
    }
    . $CloseLibraryPath -LibraryOnly
    $protectedRoot = Join-Path $env:ProgramData 'NxTools\NxTools.Next\Service'
    $approval = $null
    $serviceKey = 'HKLM:\SYSTEM\CurrentControlSet\Services\NxToolsIndexerV1'
    if (Test-Path -LiteralPath $serviceKey) {
        $registration = Get-ItemProperty -LiteralPath $serviceKey
        $approval = @{ ServicePath = $registration.ImagePath; DesktopPath = $registration.NxToolsDesktopPath; ArchiveHashesJson = $registration.NxToolsArchiveReleaseSha256; Hashes = @{
            'NxTools.exe' = $registration.NxToolsDesktopSha256
            'NxTools.PlatformHost.exe' = $registration.NxToolsPlatformHostSha256
            'NxTools.Indexer.exe' = $registration.NxToolsBinarySha256
            'NxTools.BrowserHost.exe' = $registration.NxToolsBrowserHostSha256
        }}
    }
    $targets = Get-UpgradeTargets $InstallDirectory $protectedRoot $approval
    Stop-UpgradeTargets $targets $protectedRoot | Out-Null
}

function Initialize-NxToolsUninstallNative {
    if ('NxToolsUninstallNative' -as [type]) { return }
    Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;
using Microsoft.Win32.SafeHandles;
public static class NxToolsUninstallNative {
    const int SQLITE_OK = 0, SQLITE_ROW = 100, SQLITE_DONE = 101;
    [DllImport("winsqlite3.dll")] static extern int sqlite3_open_v2(IntPtr name, out IntPtr database, int flags, IntPtr vfs);
    [DllImport("winsqlite3.dll")] static extern int sqlite3_prepare_v2(IntPtr database, IntPtr sql, int bytes, out IntPtr statement, IntPtr tail);
    [DllImport("winsqlite3.dll")] static extern int sqlite3_step(IntPtr statement);
    [DllImport("winsqlite3.dll")] static extern IntPtr sqlite3_column_text(IntPtr statement, int column);
    [DllImport("winsqlite3.dll")] static extern int sqlite3_column_bytes(IntPtr statement, int column);
    [DllImport("winsqlite3.dll")] static extern int sqlite3_finalize(IntPtr statement);
    [DllImport("winsqlite3.dll")] static extern int sqlite3_close(IntPtr database);
    static IntPtr Utf8(string value) {
        byte[] bytes = Encoding.UTF8.GetBytes(value + "\0");
        IntPtr result = Marshal.AllocHGlobal(bytes.Length);
        Marshal.Copy(bytes, 0, result, bytes.Length);
        return result;
    }
    static string[] QueryText(string path, string query) {
        IntPtr name = Utf8(path), database = IntPtr.Zero, sql = IntPtr.Zero, statement = IntPtr.Zero;
        try {
            int code = sqlite3_open_v2(name, out database, 1, IntPtr.Zero);
            if (code != SQLITE_OK) throw new Win32Exception(code, "Unable to open the application database.");
            sql = Utf8(query);
            code = sqlite3_prepare_v2(database, sql, -1, out statement, IntPtr.Zero);
            if (code != SQLITE_OK) throw new Win32Exception(code, "Unable to inspect application-owned paths.");
            var values = new System.Collections.Generic.List<string>();
            while ((code = sqlite3_step(statement)) == SQLITE_ROW) {
                IntPtr text = sqlite3_column_text(statement, 0); int length = sqlite3_column_bytes(statement, 0);
                if (text == IntPtr.Zero || length <= 0) continue;
                byte[] bytes = new byte[length]; Marshal.Copy(text, bytes, 0, length);
                values.Add(Encoding.UTF8.GetString(bytes));
            }
            if (code != SQLITE_DONE) throw new Win32Exception(code, "Unable to read application-owned paths.");
            return values.ToArray();
        } finally {
            if (statement != IntPtr.Zero) sqlite3_finalize(statement);
            if (database != IntPtr.Zero) sqlite3_close(database);
            if (sql != IntPtr.Zero) Marshal.FreeHGlobal(sql);
            if (name != IntPtr.Zero) Marshal.FreeHGlobal(name);
        }
    }
    public static string[] ReadCommittedOutputs(string path) {
        return QueryText(path, "SELECT path FROM download_artifacts WHERE kind='final' AND state='committed'");
    }
    public static string[] ReadClipboardAssets(string path) {
        return QueryText(path, "SELECT image_path FROM clipboard_items WHERE image_path != '' UNION SELECT source_process_icon_path FROM clipboard_items WHERE source_process_icon_path != '' UNION SELECT data_path FROM clipboard_item_formats WHERE data_path != ''");
    }
    [StructLayout(LayoutKind.Sequential)] struct FileIdInfo {
        public ulong VolumeSerialNumber;
        [MarshalAs(UnmanagedType.ByValArray, SizeConst=16)] public byte[] FileId;
    }
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
    static extern SafeFileHandle CreateFileW(string path, uint access, uint share, IntPtr security, uint creation, uint flags, IntPtr template);
    [DllImport("kernel32.dll", SetLastError=true)]
    static extern bool GetFileInformationByHandleEx(SafeFileHandle file, int infoClass, out FileIdInfo info, uint size);
    public static string DirectoryIdentity(string path) {
        using (var handle = CreateFileW(path, 0x80, 7, IntPtr.Zero, 3, 0x02200000, IntPtr.Zero)) {
            if (handle.IsInvalid) throw new Win32Exception(Marshal.GetLastWin32Error());
            FileIdInfo info;
            if (!GetFileInformationByHandleEx(handle, 18, out info, (uint)Marshal.SizeOf(typeof(FileIdInfo))))
                throw new Win32Exception(Marshal.GetLastWin32Error());
            ulong low = BitConverter.ToUInt64(info.FileId, 0), high = BitConverter.ToUInt64(info.FileId, 8);
            return info.VolumeSerialNumber.ToString() + ":" + low.ToString() + ":" + high.ToString();
        }
    }
}
'@
}

function Get-NxToolsFinalDownloadPaths([string]$DatabasePath) {
    if (!(Test-Path -LiteralPath $DatabasePath -PathType Leaf)) { return @() }
    Initialize-NxToolsUninstallNative
    @([NxToolsUninstallNative]::ReadCommittedOutputs((Get-NxToolsFullPath $DatabasePath)))
}

function Get-NxToolsDirectoryIdentity([string]$Path) {
    Initialize-NxToolsUninstallNative
    [NxToolsUninstallNative]::DirectoryIdentity((Get-NxToolsFullPath $Path))
}

function Restore-NxToolsArchiveIntegration([string]$InstallDirectory, [string]$CorrelationId) {
    $archiveCorrelation = [Guid]::ParseExact($CorrelationId, 'D')
    $platformHostPath = Join-Path $InstallDirectory 'NxTools.PlatformHost.exe'
    $archiveWatch = [Diagnostics.Stopwatch]::StartNew()
    Write-Output "installer.archive_uninstall.started correlation=$archiveCorrelation"
    $archiveProcess = Start-Process -FilePath $platformHostPath -ArgumentList "--uninstall-archive-integration $archiveCorrelation" -WindowStyle Hidden -Wait -PassThru
    try {
        $archiveExit = $archiveProcess.ExitCode
        Write-Output "installer.archive_uninstall.terminal correlation=$archiveCorrelation exit_code=$archiveExit elapsed_ms=$($archiveWatch.ElapsedMilliseconds)"
        if ($archiveExit -ne 0) {
            $archiveFailure = [InvalidOperationException]::new('Current-user archive integration restoration was not confirmed.')
            $archiveFailure.Data['ArchiveIntegrationExit'] = $archiveExit
            throw $archiveFailure
        }
    } finally {
        $archiveProcess.Dispose()
    }
}

function Remove-NxToolsBrowserRegistration([string]$InstallDirectory, [string]$LocalProductRoot) {
    $manifestRoot = Join-Path $LocalProductRoot 'browser-native-host'
    $installedHost = Get-NxToolsFullPath (Join-Path $InstallDirectory 'NxTools.BrowserHost.exe')
    $registrations = @(
        @{ Key='Software\Google\Chrome\NativeMessagingHosts\com.nxtools.next.browser'; Manifest=(Join-Path $manifestRoot 'chromium-host.json') },
        @{ Key='Software\Microsoft\Edge\NativeMessagingHosts\com.nxtools.next.browser'; Manifest=(Join-Path $manifestRoot 'chromium-host.json') },
        @{ Key='Software\Mozilla\NativeMessagingHosts\com.nxtools.next.browser'; Manifest=(Join-Path $manifestRoot 'firefox-host.json') }
    )
    foreach ($registration in $registrations) {
        $manifest = Get-NxToolsFullPath $registration.Manifest
        $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($registration.Key, $false)
        try {
            if ($null -eq $key) { continue }
            $registered = [string]$key.GetValue('', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
            if (!$registered -or !(Get-NxToolsFullPath $registered).Equals($manifest, [StringComparison]::OrdinalIgnoreCase)) { continue }
            if (!(Test-Path -LiteralPath $manifest -PathType Leaf)) { throw [IO.InvalidDataException]::new('Owned native messaging manifest is missing.') }
            $document = Get-Content -LiteralPath $manifest -Raw -Encoding UTF8 | ConvertFrom-Json
            if ([string]$document.name -ne 'com.nxtools.next.browser' -or
                !(Get-NxToolsFullPath ([string]$document.path)).Equals($installedHost, [StringComparison]::OrdinalIgnoreCase)) {
                throw [IO.InvalidDataException]::new('Native messaging manifest identity changed.')
            }
        } finally {
            if ($null -ne $key) { $key.Dispose() }
        }
        [Microsoft.Win32.Registry]::CurrentUser.DeleteSubKeyTree($registration.Key, $false)
    }
    foreach ($manifestName in @('chromium-host.json', 'firefox-host.json')) {
        $manifest = Join-Path $manifestRoot $manifestName
        if (Test-Path -LiteralPath $manifest -PathType Leaf) {
            $document = Get-Content -LiteralPath $manifest -Raw -Encoding UTF8 | ConvertFrom-Json
            if ([string]$document.name -eq 'com.nxtools.next.browser' -and
                (Get-NxToolsFullPath ([string]$document.path)).Equals($installedHost, [StringComparison]::OrdinalIgnoreCase)) {
                Remove-Item -LiteralPath $manifest -Force
            }
        }
    }
    if ((Test-Path -LiteralPath $manifestRoot -PathType Container) -and
        @(Get-ChildItem -LiteralPath $manifestRoot -Force).Count -eq 0) { Remove-Item -LiteralPath $manifestRoot -Force }
}

function Remove-NxToolsOwnedRegistrations([string]$InstallDirectory, [string]$LocalProductRoot) {
    Restore-NxToolsArchiveIntegration $InstallDirectory $CorrelationId
    Remove-NxToolsBrowserRegistration $InstallDirectory $LocalProductRoot
}

function Get-NxToolsRecordedArchiveRoots([string]$CacheRoot) {
    $inventory = Join-Path $CacheRoot 'archive\temp-roots'
    $roots = @()
    for ($slot = 0; $slot -lt 16; $slot++) {
        $recordPath = Join-Path $inventory ($slot.ToString() + '.json')
        if (!(Test-Path -LiteralPath $recordPath -PathType Leaf)) { continue }
        if ((Get-Item -LiteralPath $recordPath).Length -gt 256KB) { throw [IO.InvalidDataException]::new('Archive temporary-root record exceeds its bound.') }
        $record = Get-Content -LiteralPath $recordPath -Raw -Encoding UTF8 | ConvertFrom-Json
        $base = [string]$record.BaseDirectory
        if ([int]$record.Version -ne 1 -or $null -eq $record.Identity -or
            $base -notmatch '^[A-Za-z]:\\' -or $base.IndexOfAny([char[]]'*?"<>|') -ge 0) {
            throw [IO.InvalidDataException]::new('Archive temporary-root record is invalid.')
        }
        $sessions = Join-Path $base 'NxTools.Next\archive\sessions'
        if (!(Test-Path -LiteralPath $sessions -PathType Container)) { continue }
        $expected = ([uint64]$record.Identity.Volume).ToString() + ':' + ([uint64]$record.Identity.Low).ToString() + ':' + ([uint64]$record.Identity.High).ToString()
        if ((Get-NxToolsDirectoryIdentity $sessions) -ne $expected) {
            throw [IO.InvalidDataException]::new('Recorded archive temporary-root identity changed.')
        }
        $roots += Get-NxToolsFullPath $sessions
    }
    @($roots | Select-Object -Unique)
}

function Get-NxToolsConfiguredDownloadRoots($Configuration, [string]$DataRoot) {
    if ($null -eq $Configuration -or $null -eq $Configuration.download) { return @() }
    $configuredDefault = [string]$Configuration.download.default_directory
    if ([string]::IsNullOrWhiteSpace($configuredDefault)) {
        $defaultRoot = if ($env:USERPROFILE) { Join-Path $env:USERPROFILE 'Downloads' } else { Join-Path $DataRoot 'downloads' }
    } else {
        if (!(Test-NxToolsAbsolutePath $configuredDefault)) {
            throw [IO.InvalidDataException]::new('Configured default download directory is not absolute.')
        }
        $defaultRoot = $configuredDefault
    }
    $roots = @(Get-NxToolsFullPath $defaultRoot)
    foreach ($category in @($Configuration.download.categories)) {
        $directory = [string]$category.save_directory
        if ([string]::IsNullOrWhiteSpace($directory)) { continue }
        if (Test-NxToolsAbsolutePath $directory) {
            $roots += Get-NxToolsFullPath $directory
        } else {
            $roots += Get-NxToolsFullPath (Join-Path $defaultRoot $directory)
        }
    }
    @($roots | Select-Object -Unique)
}

function Remove-NxToolsOwnedFile([string]$Path) {
    if (Test-Path -LiteralPath $Path -PathType Leaf) { Remove-Item -LiteralPath $Path -Force }
}

function Test-NxToolsOutputExclusion([string]$Path, [string[]]$PreservedPaths, [string[]]$PreservedDirectories) {
    $candidate = Get-NxToolsFullPath $Path
    if (@($PreservedPaths | Where-Object {
        (Get-NxToolsFullPath $_).Equals($candidate, [StringComparison]::OrdinalIgnoreCase)
    }).Count) { return $true }
    return @($PreservedDirectories | Where-Object {
        Test-NxToolsSameOrDescendant $candidate (Get-NxToolsFullPath $_)
    }).Count -gt 0
}

function Get-NxToolsClipboardAssetPaths([string]$DatabasePath) {
    if (!(Test-Path -LiteralPath $DatabasePath -PathType Leaf)) { return @() }
    try {
        Initialize-NxToolsUninstallNative
        @([NxToolsUninstallNative]::ReadClipboardAssets((Get-NxToolsFullPath $DatabasePath)))
    } catch {
        # An unreadable inventory cannot authorize asset deletion. The database itself remains an exact owner.
        @()
    }
}

function Remove-NxToolsClipboardStorage([string]$StorageRoot, [string[]]$PreservedPaths,
    [string[]]$PreservedDirectories) {
    if (!(Test-Path -LiteralPath $StorageRoot -PathType Container)) { return }
    $database = Join-Path $StorageRoot 'clipboard.db'
    $assetRoot = Join-Path $StorageRoot 'clipboard-images'
    foreach ($asset in @(Get-NxToolsClipboardAssetPaths $database | Select-Object -Unique)) {
        if ([string]::IsNullOrWhiteSpace($asset)) { continue }
        $assetPath = Get-NxToolsFullPath $asset
        if ((Test-NxToolsSameOrDescendant $assetPath $assetRoot) -and
            !(Test-NxToolsOutputExclusion $assetPath $PreservedPaths $PreservedDirectories)) {
            Remove-NxToolsOwnedFile $assetPath
        }
    }
    foreach ($name in @('clipboard.db', 'clipboard.db-wal', 'clipboard.db-shm')) {
        Remove-NxToolsOwnedFile (Join-Path $StorageRoot $name)
    }
}

function Remove-NxToolsExtensionStorage([string]$Root, [string[]]$PreservedPaths,
    [string[]]$PreservedDirectories) {
    if (!(Test-Path -LiteralPath $Root -PathType Container)) { return }
    $owned = @('catalog.json', 'catalog.tmp', 'initialized', 'quota.lock')
    $catalogPath = Join-Path $Root 'catalog.json'
    if (Test-Path -LiteralPath $catalogPath -PathType Leaf) {
        try {
            if ((Get-Item -LiteralPath $catalogPath).Length -le 1MB) {
                $catalog = Get-Content -LiteralPath $catalogPath -Raw -Encoding UTF8 | ConvertFrom-Json
                if ([int]$catalog.version -eq 1 -and $null -ne $catalog.namespaces) {
                    foreach ($namespace in $catalog.namespaces.PSObject.Properties) {
                        $namespaceId = [Guid]::Empty
                        if ([Guid]::TryParseExact($namespace.Name, 'D', [ref]$namespaceId) -and $namespaceId -ne [Guid]::Empty) {
                            $owned += $namespace.Name + '.lock'
                        }
                        $fileName = [string]$namespace.Value.file
                        $fileId = [Guid]::Empty
                        if ([Guid]::TryParseExact($fileName, 'D', [ref]$fileId) -and $fileId -ne [Guid]::Empty) {
                            $owned += $fileName + '.json'
                        }
                    }
                    $pendingName = [string]$catalog.pending
                    $pendingId = [Guid]::Empty
                    if ([Guid]::TryParseExact($pendingName, 'D', [ref]$pendingId) -and $pendingId -ne [Guid]::Empty) {
                        $owned += $pendingName + '.json'
                    }
                }
            }
        } catch {
            # Retain unattributed namespace payloads when the catalog cannot prove them.
        }
    }
    foreach ($name in @($owned | Select-Object -Unique)) {
        $path = Join-Path $Root $name
        if (!(Test-NxToolsOutputExclusion $path $PreservedPaths $PreservedDirectories) -or
            $name -in @('catalog.json', 'catalog.tmp', 'initialized', 'quota.lock')) {
            Remove-NxToolsOwnedFile $path
        }
    }
}

function Remove-NxToolsArchiveSessionMetadata([string]$SessionsRoot) {
    if (!(Test-Path -LiteralPath $SessionsRoot -PathType Container)) { return }
    foreach ($directory in @(Get-ChildItem -LiteralPath $SessionsRoot -Directory -Force -ErrorAction SilentlyContinue)) {
        $leaseId = [Guid]::Empty
        if (![Guid]::TryParseExact($directory.Name, 'D', [ref]$leaseId) -or $leaseId -eq [Guid]::Empty) { continue }
        $manifestPath = Join-Path $directory.FullName 'manifest.json'
        if (!(Test-Path -LiteralPath $manifestPath -PathType Leaf) -or
            (Get-Item -LiteralPath $manifestPath).Length -gt 64KB) { continue }
        try {
            $manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
            $manifestMatchesLease = [int]$manifest.Version -eq 2 -and [Guid]$manifest.LeaseId -eq $leaseId
        } catch { continue }
        if ($manifestMatchesLease) { Remove-NxToolsOwnedFile $manifestPath }
    }
    foreach ($record in @(Get-ChildItem -LiteralPath $SessionsRoot -Filter '*.retire.json' -File -Force -ErrorAction SilentlyContinue)) {
        $leaf = $record.Name.Substring(0, $record.Name.Length - '.retire.json'.Length)
        $leaseId = [Guid]::Empty
        if (![Guid]::TryParseExact($leaf, 'D', [ref]$leaseId) -or $leaseId -eq [Guid]::Empty -or $record.Length -gt 64KB) { continue }
        try {
            $manifest = Get-Content -LiteralPath $record.FullName -Raw -Encoding UTF8 | ConvertFrom-Json
            $manifestMatchesLease = [int]$manifest.Version -eq 2 -and [Guid]$manifest.LeaseId -eq $leaseId
        } catch { continue }
        if ($manifestMatchesLease) { Remove-NxToolsOwnedFile $record.FullName }
    }
}

function Remove-NxToolsArchiveMetadata([string]$CacheRoot, [string[]]$RecordedRoots) {
    $roots = @((Join-Path $CacheRoot 'archive\sessions')) + @($RecordedRoots)
    foreach ($root in @($roots | Select-Object -Unique)) { Remove-NxToolsArchiveSessionMetadata $root }
    $inventory = Join-Path $CacheRoot 'archive\temp-roots'
    for ($slot = 0; $slot -lt 16; $slot++) { Remove-NxToolsOwnedFile (Join-Path $inventory ($slot.ToString() + '.json')) }
    Remove-NxToolsOwnedFile (Join-Path $inventory 'admission.lock')
}

function Remove-NxToolsManagedFfmpeg($Configuration, [string]$DataRoot,
    [string[]]$PreservedPaths, [string[]]$PreservedDirectories) {
    if ($null -eq $Configuration -or $null -eq $Configuration.download) { return }
    $executable = [string]$Configuration.download.ffmpeg_executable_path
    if ([string]::IsNullOrWhiteSpace($executable) -or !(Test-NxToolsAbsolutePath $executable)) { return }
    $path = Get-NxToolsFullPath $executable
    $directory = Split-Path -Parent $path
    $managedRoot = Get-NxToolsFullPath (Join-Path $DataRoot 'tools\ffmpeg')
    if ((Split-Path -Parent $directory).Equals($managedRoot, [StringComparison]::OrdinalIgnoreCase) -and
        (Split-Path -Leaf $directory).StartsWith('managed-', [StringComparison]::OrdinalIgnoreCase) -and
        !(Test-NxToolsOutputExclusion $path $PreservedPaths $PreservedDirectories)) {
        Remove-NxToolsOwnedFile $path
    }
}

function Remove-NxToolsKnownProjectData([string]$ConfigRoot, [string]$DataRoot, [string]$CacheRoot,
    [string]$LogRoot) {
    foreach ($name in @('config.json', 'config.json.lock')) { Remove-NxToolsOwnedFile (Join-Path $ConfigRoot $name) }
    foreach ($entry in @(Get-ChildItem -LiteralPath $ConfigRoot -Filter '.config.json.*.tmp' -File -Force -ErrorAction SilentlyContinue)) {
        Remove-NxToolsOwnedFile $entry.FullName
    }
    foreach ($name in @('downloads.sqlite3', 'downloads.sqlite3-wal', 'downloads.sqlite3-shm',
        'quick-launch.db', 'quick-launch.db-wal', 'quick-launch.db-shm')) {
        Remove-NxToolsOwnedFile (Join-Path $DataRoot $name)
    }
    foreach ($name in @('desktop.lock', 'desktop.hints', 'indexer.lock', 'indexer.hints')) {
        Remove-NxToolsOwnedFile (Join-Path $CacheRoot $name)
    }
    foreach ($pattern in @('desktop.log.*', 'download-host.log.*', 'browser-host.log.*', 'indexer.log.*',
        'platform-host.log.*.jsonl')) {
        foreach ($entry in @(Get-ChildItem -LiteralPath $LogRoot -Filter $pattern -File -Force -ErrorAction SilentlyContinue)) {
            Remove-NxToolsOwnedFile $entry.FullName
        }
    }
    Remove-NxToolsOwnedFile (Join-Path $LogRoot 'startup-failure.log')
    $installerLogs = Join-Path $LogRoot 'installer'
    foreach ($name in @('launch.txt', 'preparation.jsonl', 'setup.log', 'bootstrap-detection.jsonl')) {
        foreach ($entry in @(Get-ChildItem -LiteralPath $installerLogs -Filter $name -File -Force -Recurse -ErrorAction SilentlyContinue)) {
            Remove-NxToolsOwnedFile $entry.FullName
        }
    }
}

function Invoke-NxToolsPersonalDataCleanup([string]$ConfigRoot, [string]$DataRoot, [string]$CacheRoot, [string]$LogRoot) {
    $configPath = Join-Path $ConfigRoot 'config.json'
    $configuration = $null
    if (Test-Path -LiteralPath $configPath -PathType Leaf) {
        $configuration = Get-Content -LiteralPath $configPath -Raw -Encoding UTF8 | ConvertFrom-Json
    }
    $downloadDatabase = Join-Path $DataRoot 'downloads.sqlite3'
    $preservedOutputs = @(Get-NxToolsFinalDownloadPaths $downloadDatabase | ForEach-Object { Get-NxToolsFullPath $_ } | Where-Object {
        Test-Path -LiteralPath $_ -PathType Leaf
    } | Select-Object -Unique)
    $downloadRoots = @(Get-NxToolsConfiguredDownloadRoots $configuration $DataRoot)
    $archiveRoots = @(Get-NxToolsRecordedArchiveRoots $CacheRoot)
    $customClipboard = if ($null -ne $configuration -and $null -ne $configuration.clipboard) {
        [string]$configuration.clipboard.storage_directory
    } else { '' }
    Remove-NxToolsClipboardStorage $DataRoot $preservedOutputs $downloadRoots
    if (![string]::IsNullOrWhiteSpace($customClipboard) -and
        !(Get-NxToolsFullPath $customClipboard).Equals((Get-NxToolsFullPath $DataRoot), [StringComparison]::OrdinalIgnoreCase)) {
        Remove-NxToolsClipboardStorage (Get-NxToolsFullPath $customClipboard) $preservedOutputs $downloadRoots
    }
    Remove-NxToolsExtensionStorage (Join-Path $DataRoot 'extensions-data') $preservedOutputs $downloadRoots
    Remove-NxToolsArchiveMetadata $CacheRoot $archiveRoots
    Remove-NxToolsManagedFfmpeg $configuration $DataRoot $preservedOutputs $downloadRoots
    Remove-NxToolsKnownProjectData $ConfigRoot $DataRoot $CacheRoot $LogRoot
}

if ($Operation -ne 'Uninstall') {
    Write-Output "installer.uninstall_cleanup.skipped correlation=$CorrelationId operation=$Operation"
    exit 0
}

$InstallDirectory = Get-NxToolsFullPath $InstallDirectory
$ConfigRoot = Get-NxToolsFullPath $ConfigRoot
$DataRoot = Get-NxToolsFullPath $DataRoot
$CacheRoot = Get-NxToolsFullPath $CacheRoot
$LogRoot = Get-NxToolsFullPath $LogRoot
if ([string]::IsNullOrWhiteSpace($LocalProductRoot)) { $LocalProductRoot = Split-Path -Parent $LogRoot | Split-Path -Parent }
$LocalProductRoot = Get-NxToolsFullPath $LocalProductRoot
$CloseLibraryPath = Get-NxToolsFullPath $CloseLibraryPath

if ($FixtureEffectsPath) {
    if (!$FixtureRoot) { throw [ArgumentException]::new('Fixture effects require an owned fixture root.') }
    $FixtureRoot = Get-NxToolsFullPath $FixtureRoot
    foreach ($fixturePath in @($InstallDirectory, $ConfigRoot, $DataRoot, $CacheRoot, $LogRoot, $LocalProductRoot, $FixtureEffectsPath)) {
        if (!(Test-NxToolsSameOrDescendant $fixturePath $FixtureRoot)) { throw [ArgumentException]::new('Fixture path escaped its owned root.') }
    }
    . (Get-NxToolsFullPath $FixtureEffectsPath)
}

$watch = [Diagnostics.Stopwatch]::StartNew()
Write-Output "installer.uninstall_cleanup.started correlation=$CorrelationId remove_personal=$([int][bool]$RemovePersonalData)"
try {
    Stop-NxToolsOwnedProcesses $InstallDirectory $CloseLibraryPath
    Remove-NxToolsOwnedRegistrations $InstallDirectory $LocalProductRoot
    if ($RemovePersonalData) { Invoke-NxToolsPersonalDataCleanup $ConfigRoot $DataRoot $CacheRoot $LogRoot }
    Write-Output "installer.uninstall_cleanup.completed correlation=$CorrelationId remove_personal=$([int][bool]$RemovePersonalData) elapsed_ms=$($watch.ElapsedMilliseconds)"
    exit 0
} catch {
    $failure = $_.Exception
    while ($failure.InnerException) { $failure = $failure.InnerException }
    if ($failure.Data.Contains('ArchiveIntegrationExit')) {
        $archiveExit = [int]$failure.Data['ArchiveIntegrationExit']
        Write-Output "installer.uninstall_cleanup.failed correlation=$CorrelationId archive_exit_code=$archiveExit elapsed_ms=$($watch.ElapsedMilliseconds)"
        if ($archiveExit -eq 12) { exit 12 }
        exit 11
    }
    $nativeCode = if ($failure -is [ComponentModel.Win32Exception]) { $failure.NativeErrorCode } elseif ($failure -is [UnauthorizedAccessException]) { 5 } else { 0 }
    Write-Output "installer.uninstall_cleanup.failed correlation=$CorrelationId native_code=$nativeCode exception_type=$($failure.GetType().FullName) elapsed_ms=$($watch.ElapsedMilliseconds)"
    if ($nativeCode -eq 5) { exit 5 }
    exit 1
}
