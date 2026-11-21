param(
    [string]$InstallDirectory,
    [switch]$LibraryOnly,
    [string]$CorrelationId = [Guid]::NewGuid().ToString('N'),
    [string]$StatePath,
    [switch]$Finalize,
    [switch]$Committed,
    [string]$RollbackNewSha256 = '',
    [string]$NewManifestPath = '',
    [string]$NewSignaturePath = '',
    [string]$BootstrapMigrationPackage = '',
    [string]$DiagnosticPath = '',
    [switch]$RepairMissingBootstrap,
    [switch]$RotateTrustedBootstrap
)
$ErrorActionPreference = 'Stop'

function Get-UpgradeTargets([string]$InstallRoot, [string]$ProtectedRoot, $Approval) {
    $targets = @{}
    foreach ($name in @('NxTools.exe', 'NxTools.PlatformHost.exe', 'NxTools.BrowserHost.exe', 'NxTools.Cli.exe', 'NxTools.ArchiveWorker.exe', 'NxTools.ShellSelectionHelper.exe')) {
        $targets[[IO.Path]::GetFullPath((Join-Path $InstallRoot $name))] = 'installed'
    }
    # A service name or ProgramData path alone is NOT ownership proof. All three
    # registered old hashes must match the actual old tuple before stopping it.
    $tuple = @('NxTools.exe', 'NxTools.PlatformHost.exe', 'NxTools.Indexer.exe')
    if ($Approval -and $Approval.ServicePath -eq ('"' + (Join-Path $ProtectedRoot 'NxTools.Indexer.exe') + '" --service') -and
        $Approval.DesktopPath -eq (Join-Path $ProtectedRoot 'NxTools.exe')) {
        $valid = $true
        foreach ($name in $tuple) {
            $candidate = Join-Path $ProtectedRoot $name
            if (!(Test-Path -LiteralPath $candidate -PathType Leaf) -or !$Approval.Hashes[$name] -or
                (Get-FileHash -LiteralPath $candidate -Algorithm SHA256).Hash -ne $Approval.Hashes[$name]) {
                $valid = $false
            }
        }
        if ($valid) {
            # Indexer is a service owner, not a desktop child. Never terminate it.
            foreach ($name in @('NxTools.exe', 'NxTools.PlatformHost.exe', 'NxTools.BrowserHost.exe')) {
                $candidate = Join-Path $ProtectedRoot $name
                if ($name -eq 'NxTools.BrowserHost.exe') {
                    if (!(Test-Path -LiteralPath $candidate -PathType Leaf) -or !$Approval.Hashes[$name] -or
                        (Get-FileHash -LiteralPath $candidate).Hash -ne $Approval.Hashes[$name]) { continue }
                }
                $targets[[IO.Path]::GetFullPath($candidate)] = 'protected'
            }
            # The new executables require the whole registered archive set, not
            # merely their own names/hashes. Missing approval never downgrades it.
            $oldBrowserApproved = $Approval.Hashes['NxTools.BrowserHost.exe'] -match '^[a-fA-F0-9]{64}$' -and
                (Test-UpgradeFileHash (Join-Path $ProtectedRoot 'NxTools.BrowserHost.exe') $Approval.Hashes['NxTools.BrowserHost.exe'])
            if ($oldBrowserApproved -and (Test-UpgradeArchiveApproval $ProtectedRoot $Approval.ArchiveHashesJson)) {
                foreach ($name in @('NxTools.ArchiveWorker.exe', 'NxTools.ShellSelectionHelper.exe')) {
                    $targets[[IO.Path]::GetFullPath((Join-Path $ProtectedRoot $name))] = 'protected'
                }
            }
        }
    }
    return $targets
}

function Initialize-UpgradeNative {
    if ('NxToolsUpgradeProcess' -as [type]) { return }
    Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;
public static class NxToolsUpgradeProcess {
    [DllImport("kernel32.dll", SetLastError=true)] static extern IntPtr OpenProcess(uint access, bool inherit, int id);
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)] static extern bool QueryFullProcessImageName(IntPtr process, uint flags, StringBuilder path, ref int size);
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool TerminateProcess(IntPtr process, uint code);
    [DllImport("kernel32.dll", SetLastError=true)] static extern uint WaitForSingleObject(IntPtr process, uint milliseconds);
    [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr handle);
    static Win32Exception ProcessFailure(string api, int code) {
        var failure = new Win32Exception(code);
        failure.Data["native_api"] = api;
        return failure;
    }
    static bool HasExited(IntPtr handle, uint milliseconds = 0) {
        uint result = WaitForSingleObject(handle, milliseconds);
        if (result == 0) return true;
        if (result == 258) return false;
        if (result == 0xFFFFFFFF) throw ProcessFailure("wait", Marshal.GetLastWin32Error());
        throw new InvalidOperationException("Unexpected process wait result.");
    }
    // WMI can omit ExecutablePath while a process is exiting or due to query privileges.
    // Discovery requests no termination right; Stop separately revalidates its own handle.
    public static string QueryImage(int id) {
        IntPtr handle = OpenProcess(0x00101000, false, id);
        if (handle == IntPtr.Zero) {
            int code = Marshal.GetLastWin32Error();
            if (code == 87) return null;
            throw ProcessFailure("open", code);
        }
        try {
            if (HasExited(handle)) return null;
            var path = new StringBuilder(32768); int size = path.Capacity;
            if (!QueryFullProcessImageName(handle, 0, path, ref size)) {
                int code = Marshal.GetLastWin32Error();
                if (HasExited(handle, code == 5 || code == 31 ? 5000u : 0u)) return null;
                throw ProcessFailure("query_image", code);
            }
            return HasExited(handle) ? null : System.IO.Path.GetFullPath(path.ToString());
        } finally { CloseHandle(handle); }
    }
    // Query, exit checks and termination share one handle; PID reuse cannot redirect a kill.
    public static bool Stop(int id, string expected) {
        IntPtr handle = OpenProcess(0x00101001, false, id);
        if (handle == IntPtr.Zero) {
            int code = Marshal.GetLastWin32Error();
            if (code == 87) return false; // process object already gone
            throw ProcessFailure("open", code);
        }
        try {
            // QueryFullProcessImageName returns31 for an exited process whose object is retained.
            // Exit is harmless only when THIS acquired handle is observed signalled.
            if (HasExited(handle)) return false;
            var path = new StringBuilder(32768); int size = path.Capacity;
            if (!QueryFullProcessImageName(handle, 0, path, ref size)) {
                int code = Marshal.GetLastWin32Error();
                if (HasExited(handle, code == 5 || code == 31 ? 5000u : 0u)) return false;
                throw ProcessFailure("query_image", code);
            }
            if (!String.Equals(System.IO.Path.GetFullPath(path.ToString()), expected, StringComparison.OrdinalIgnoreCase)) return false;
            if (HasExited(handle)) return false;
            if (!TerminateProcess(handle, 0)) {
                int code = Marshal.GetLastWin32Error();
                if (HasExited(handle, code == 5 || code == 31 ? 5000u : 0u)) return false;
                throw ProcessFailure("terminate", code);
            }
            uint waited = WaitForSingleObject(handle, 5000);
            if (waited == 0xFFFFFFFF) throw ProcessFailure("wait", Marshal.GetLastWin32Error());
            if (waited != 0) throw new TimeoutException("Owned process termination timed out.");
            return true;
        } finally { CloseHandle(handle); }
    }
    [DllImport("advapi32.dll", CharSet=CharSet.Unicode, SetLastError=true)] static extern IntPtr OpenSCManagerW(string machine, string database, uint access);
    [DllImport("advapi32.dll", CharSet=CharSet.Unicode, SetLastError=true)] static extern IntPtr OpenServiceW(IntPtr manager, string name, uint access);
    [DllImport("advapi32.dll", SetLastError=true)] static extern bool QueryServiceStatusEx(IntPtr service, int level, out ServiceStatusProcess status, int size, out int needed);
    [DllImport("advapi32.dll")] static extern bool CloseServiceHandle(IntPtr handle);
    [StructLayout(LayoutKind.Sequential)] struct ServiceStatusProcess {
        public uint Type, State, Controls, Win32Exit, ServiceExit, CheckPoint, WaitHint, ProcessId, Flags;
    }
    // Same read-only SCM authority as TrustedUpdatePeer: no SYSTEM process handle required.
    public static uint[] ReadUpdaterService() {
        IntPtr manager = OpenSCManagerW(null, null, 1); // SC_MANAGER_CONNECT
        if (manager == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
        try {
            IntPtr service = OpenServiceW(manager, "NxToolsUpdateV1", 4); // SERVICE_QUERY_STATUS
            if (service == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
            try {
                ServiceStatusProcess status; int needed;
                if (!QueryServiceStatusEx(service, 0, out status, Marshal.SizeOf(typeof(ServiceStatusProcess)), out needed))
                    throw new Win32Exception(Marshal.GetLastWin32Error());
                return new uint[] { status.ProcessId, status.State, status.Type };
            } finally { CloseServiceHandle(service); }
        } finally { CloseServiceHandle(manager); }
    }
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
    static extern Microsoft.Win32.SafeHandles.SafeFileHandle CreateFileW(string path, uint access, uint share, IntPtr security, uint creation, uint flags, IntPtr template);
    [DllImport("kernel32.dll", SetLastError=true)]
    static extern bool SetFileInformationByHandle(Microsoft.Win32.SafeHandles.SafeFileHandle file, int info, ref int value, uint size);
    public static void DeleteExactFile(string path, string hash) {
        using (var handle = CreateFileW(path, 0x80010000, 1, IntPtr.Zero, 3, 0x00200000, IntPtr.Zero)) {
            if (handle.IsInvalid) throw new Win32Exception(Marshal.GetLastWin32Error());
            using (var stream = new System.IO.FileStream(handle, System.IO.FileAccess.Read))
            using (var sha = System.Security.Cryptography.SHA256.Create()) {
                string actual = BitConverter.ToString(sha.ComputeHash(stream)).Replace("-", "");
                if (!String.Equals(actual, hash, StringComparison.OrdinalIgnoreCase))
                    throw new System.IO.InvalidDataException("New host recovery identity mismatch.");
                int delete = 1;
                if (!SetFileInformationByHandle(handle, 4, ref delete, 4)) throw new Win32Exception(Marshal.GetLastWin32Error());
            }
        }
    }
}
'@
}

function Resolve-UpgradeUpdaterProcessId($Registration, [string]$ExpectedBootstrap, [string]$ActualHash, $ServiceStatus) {
    if ($null -eq $Registration) { return 0 }
    if (![string]::Equals([string]$Registration.ImagePath, ('"' + $ExpectedBootstrap + '" --trusted-update-service'), [StringComparison]::OrdinalIgnoreCase) -or
        ![string]::Equals([string]$Registration.ObjectName, 'LocalSystem', [StringComparison]::OrdinalIgnoreCase) -or
        $Registration.NxToolsUpdateKeySha256 -notmatch '^[a-fA-F0-9]{64}$' -or
        $ActualHash -notmatch '^[a-fA-F0-9]{64}$' -or
        ![string]::Equals([string]$Registration.NxToolsBootstrapSha256, $ActualHash, [StringComparison]::OrdinalIgnoreCase) -or
        @($ServiceStatus).Count -ne 3 -or [uint32]$ServiceStatus[2] -ne 0x10) {
        throw [IO.InvalidDataException]::new('Updater service identity is not approved for process exclusion.')
    }
    if ([uint32]$ServiceStatus[1] -eq 1 -and [uint32]$ServiceStatus[0] -eq 0) { return 0 }
    if ([uint32]$ServiceStatus[1] -ne 4 -or [uint32]$ServiceStatus[0] -eq 0 -or
        [uint64]$ServiceStatus[0] -gt [int]::MaxValue) {
        throw [IO.InvalidDataException]::new('Updater service identity is not approved for process exclusion.')
    }
    return [int]$ServiceStatus[0]
}

function Get-UpgradeUpdaterProcessId {
    # Exclusion is not mutation authority. Reuse the original fixed updater identity, not a name guess.
    $updaterKey = 'HKLM:\SYSTEM\CurrentControlSet\Services\NxToolsUpdateV1'
    if (!(Test-Path -LiteralPath $updaterKey)) { return 0 }
    $updaterRegistration = Get-ItemProperty -LiteralPath $updaterKey
    $bootstrap = Join-Path $env:ProgramData 'NxTools\NxTools.Next\Updater\Bootstrap\NxTools.PlatformHost.exe'
    Assert-UpgradePlainPath $bootstrap
    $actualHash = (Get-FileHash -LiteralPath $bootstrap -Algorithm SHA256).Hash
    Initialize-UpgradeNative
    $serviceStatus = [NxToolsUpgradeProcess]::ReadUpdaterService()
    Resolve-UpgradeUpdaterProcessId $updaterRegistration $bootstrap $actualHash $serviceStatus
}

function Stop-UpgradeTargets($Targets, [string]$ProtectedRoot = '') {
    Initialize-UpgradeNative
    $stopped = 0
    # Desktop first; then hosts. Bounded repeat covers an already-starting child.
    for ($round = 0; $round -lt 3; $round++) {
        $remaining = 0
        foreach ($name in @('NxTools.exe', 'NxTools.PlatformHost.exe', 'NxTools.BrowserHost.exe', 'NxTools.Cli.exe', 'NxTools.ArchiveWorker.exe', 'NxTools.ShellSelectionHelper.exe')) {
            foreach ($entry in @(Get-CimInstance Win32_Process -Filter "Name='$name'")) {
                try {
                    $image = $entry.ExecutablePath
                    if (!$image) {
                        # Independent updater exclusion is query-only SCM authority, never a kill.
                        if ($name -eq 'NxTools.PlatformHost.exe' -and
                            [int]$entry.ProcessId -eq (Get-UpgradeUpdaterProcessId)) { continue }
                        $image = [NxToolsUpgradeProcess]::QueryImage([int]$entry.ProcessId)
                        if (!$image) { continue } # exact native object gone/signalled
                    }
                    $image = [IO.Path]::GetFullPath($image)
                    if (!$Targets.ContainsKey($image)) {
                        if ($ProtectedRoot -and $name -eq 'NxTools.exe' -and
                            $image -eq [IO.Path]::GetFullPath((Join-Path $ProtectedRoot $name))) {
                            throw [InvalidOperationException]::new('Running protected desktop identity is not approved.')
                        }
                        continue
                    }
                    $remaining++
                    if ([NxToolsUpgradeProcess]::Stop([int]$entry.ProcessId, $image)) { $stopped++ }
                } catch {
                    $identityFailure = $_.Exception
                    while ($identityFailure.InnerException) { $identityFailure = $identityFailure.InnerException }
                    $identityFailure.Data['process_id'] = [int]$entry.ProcessId
                    $identityFailure.Data['process_name'] = $name
                    throw
                }
            }
        }
        if ($remaining -eq 0) { return $stopped }
        Start-Sleep -Milliseconds 150
    }
    foreach ($entry in @(Get-CimInstance Win32_Process)) {
        if ($entry.ExecutablePath -and $Targets.ContainsKey([IO.Path]::GetFullPath($entry.ExecutablePath))) {
            throw [TimeoutException]::new('Owned process remains active.')
        }
    }
    return $stopped
}

function Suspend-UpgradeBrowserHost([string]$InstallRoot, [string]$StateFile, [string]$OperationId,
    [string]$NewManifestPath = '', [string]$NewSignaturePath = '') {
    if ($NewManifestPath) {
        Suspend-UpgradeOwnedSet $InstallRoot $StateFile $OperationId $NewManifestPath $NewSignaturePath
        return
    }
    if (!$StateFile) { throw [ArgumentException]::new('Missing upgrade state.') }
    $source = [IO.Path]::GetFullPath((Join-Path $InstallRoot 'NxTools.BrowserHost.exe'))
    if (!(Test-Path -LiteralPath $source -PathType Leaf)) { return }
    $safeId = $OperationId -replace '[^a-zA-Z0-9-]', ''
    if (!$safeId) { throw [ArgumentException]::new('Invalid upgrade identity.') }
    $backupName = '.NxTools.BrowserHost-' + $safeId + '.upgrade-old'
    $backup = Join-Path $InstallRoot $backupName
    $state = @{ backup_name=$backupName; old_sha256=(Get-FileHash -LiteralPath $source).Hash }
    if (Test-Path -LiteralPath $StateFile) {
        $previous = Get-Content -LiteralPath $StateFile -Raw | ConvertFrom-Json
        if ($previous.backup_name -ne $backupName -or $previous.old_sha256 -ne $state.old_sha256) {
            throw [InvalidOperationException]::new('Upgrade recovery identity changed.')
        }
    }
    # Write the recovery record before the atomic rename. The source path is not
    # supplied by the record: both names are restricted to this installation.
    [IO.File]::WriteAllText($StateFile, ($state | ConvertTo-Json))
    $targets = @{ $source = 'installed' }
    for ($attempt = 0; $attempt -lt 10; $attempt++) {
        Stop-UpgradeTargets $targets | Out-Null
        try {
            [IO.File]::Move($source, $backup)
            return
        } catch [IO.IOException] {
            if ($attempt -eq 9) { throw }
        } catch [UnauthorizedAccessException] {
            if ($attempt -eq 9) { throw [ComponentModel.Win32Exception]::new(5) }
        }
        Start-Sleep -Milliseconds 150
    }
}

function Complete-UpgradeBrowserHost([string]$InstallRoot, [string]$StateFile, [switch]$Committed, [string]$RollbackNewSha256 = '') {
    Assert-UpgradePlainPath $StateFile
    if (!(Test-Path -LiteralPath $StateFile -PathType Leaf)) {
        if (Test-Path -LiteralPath ($StateFile + '.pending')) { throw [IO.InvalidDataException]::new('Interrupted journal write requires recovery.') }
        return
    }
    $state = Get-Content -LiteralPath $StateFile -Raw | ConvertFrom-Json
    if ($state.schema -eq 2) {
        Complete-UpgradeOwnedSet $InstallRoot $StateFile $state -Committed:$Committed
        return
    }
    if ($state.backup_name -notmatch '^\.NxTools\.BrowserHost-[a-zA-Z0-9-]+\.upgrade-old$' -or
        $state.old_sha256 -notmatch '^[a-fA-F0-9]{64}$') { throw [IO.InvalidDataException]::new('Invalid recovery record.') }
    $source = Join-Path $InstallRoot 'NxTools.BrowserHost.exe'
    $backup = Join-Path $InstallRoot $state.backup_name
    if (Test-Path -LiteralPath $backup -PathType Leaf) {
        if ((Get-FileHash -LiteralPath $backup).Hash -ne $state.old_sha256) { throw [IO.InvalidDataException]::new('Recovery identity mismatch.') }
        if ($Committed) {
            if (!(Test-Path -LiteralPath $source -PathType Leaf)) { throw [InvalidOperationException]::new('New host missing at commit.') }
            [IO.File]::Delete($backup)
        } else {
            if (Test-Path -LiteralPath $source) {
                if ($RollbackNewSha256 -notmatch '^[a-fA-F0-9]{64}$') { throw [InvalidOperationException]::new('Rollback refuses to overwrite an existing host.') }
                Initialize-UpgradeNative
                [NxToolsUpgradeProcess]::DeleteExactFile([IO.Path]::GetFullPath($source), $RollbackNewSha256)
            }
            [IO.File]::Move($backup, $source)
        }
    }
    [IO.File]::Delete($StateFile)
}

# Phase F extends the existing suspend/finalize owner, not a second updater.
# Licenses and browser_extension assets retain their original Inno file owner.
function Get-UpgradeArchiveNames {
    @('NxTools.ArchiveWorker.exe', 'NxTools.ShellSelectionHelper.exe', 'NxTools.ShellSelectionHelper.dll',
      'NxTools.ShellSelectionHelper.deps.json', 'NxTools.ShellSelectionHelper.runtimeconfig.json')
}

function Get-UpgradeOwnedNames {
    @('NxTools.exe', 'NxTools.PlatformHost.exe', 'NxTools.Indexer.exe', 'NxTools.Cli.exe', 'NxTools.BrowserHost.exe') +
        @(Get-UpgradeArchiveNames) + @('NxTools.release.json', 'NxTools.release.sig')
}

function Test-UpgradeArchiveApproval([string]$ProtectedRoot, [string]$Json) {
    if (!$Json -or $Json.Length -gt 4096) { return $false }
    try {
        $hashes = $Json | ConvertFrom-Json
        $names = @(Get-UpgradeArchiveNames)
        if (@($hashes.PSObject.Properties).Count -ne $names.Count) { return $false }
        foreach ($name in $names) {
            $hash = $hashes.$name
            $path = Join-Path $ProtectedRoot $name
            if ($hash -notmatch '^[a-fA-F0-9]{64}$' -or !(Test-Path -LiteralPath $path -PathType Leaf) -or
                (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ne $hash) { return $false }
        }
        return $true
    } catch { return $false }
}

function Assert-UpgradePlainPath([string]$Path) {
    $cursor = [IO.Path]::GetFullPath($Path)
    while ($cursor) {
        if (Test-Path -LiteralPath $cursor) {
            if ((Get-Item -LiteralPath $cursor -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) {
                throw [IO.InvalidDataException]::new('Recovery refuses reparse points.')
            }
        }
        $cursor = [IO.Path]::GetDirectoryName($cursor)
    }
}

function Test-UpgradeFileHash([string]$Path, [string]$Hash) {
    Assert-UpgradePlainPath $Path
    (Test-Path -LiteralPath $Path -PathType Leaf) -and
        ((Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash -eq $Hash)
}

function Write-UpgradeJournal([string]$StateFile, $State) {
    Assert-UpgradePlainPath $StateFile
    $pending = $StateFile + '.pending'
    $bytes = [Text.UTF8Encoding]::new($false).GetBytes(($State | ConvertTo-Json -Depth 8))
    # CreateNew never truncates an interrupted write. Keep such evidence and fail
    # closed. Flush(true) precedes publication and every subsequent file mutation.
    $stream = [IO.FileStream]::new($pending, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write,
        [IO.FileShare]::None, 4096, [IO.FileOptions]::WriteThrough)
    try { $stream.Write($bytes, 0, $bytes.Length); $stream.Flush($true) } finally { $stream.Dispose() }
    if (Test-Path -LiteralPath $StateFile) { [IO.File]::Replace($pending, $StateFile, [NullString]::Value) }
    else { [IO.File]::Move($pending, $StateFile) }
}

function Move-UpgradeFile([string]$Source, [string]$Destination) { [IO.File]::Move($Source, $Destination) }
function Restore-UpgradeFile([string]$Source, [string]$Destination) { [IO.File]::Move($Source, $Destination) }
function Remove-UpgradeExactFile([string]$Path, [string]$ExpectedHash) {
    Assert-UpgradePlainPath $Path
    Initialize-UpgradeNative
    # Never hash an unknown post-failure destination to manufacture deletion rights.
    [NxToolsUpgradeProcess]::DeleteExactFile([IO.Path]::GetFullPath($Path), $ExpectedHash)
}

function Assert-UpgradeSetRecord([string]$InstallRoot, $State) {
    $names = @(Get-UpgradeOwnedNames)
    if ($State.schema -ne 2 -or $State.root -ne [IO.Path]::GetFullPath($InstallRoot) -or
        $State.operation -notmatch '^[a-zA-Z0-9-]{1,100}$' -or @($State.entries).Count -ne $names.Count -or
        $State.failure_count -isnot [int] -or $State.failure_count -lt 0 -or
        $State.phase -notin @('preparing','suspended','rolling_back','rolled_back','failed','committing','committed')) {
        throw [IO.InvalidDataException]::new('Invalid owned-set recovery record.')
    }
    for ($index = 0; $index -lt $names.Count; $index++) {
        $entry = $State.entries[$index]
        if ($entry.name -cne $names[$index] -or $entry.backup_name -cne ('.' + $entry.name + '-' + $State.operation + '.upgrade-old') -or
            $entry.new_sha256 -notmatch '^[a-fA-F0-9]{64}$' -or $entry.existed -isnot [bool] -or
            $entry.attempted -isnot [bool] -or $entry.restore_attempted -isnot [bool] -or
            $entry.cleanup_attempted -isnot [bool] -or $entry.restored -isnot [bool] -or
            ($entry.existed -and $entry.old_sha256 -notmatch '^[a-fA-F0-9]{64}$') -or
            (!$entry.existed -and $entry.old_sha256 -ne '')) { throw [IO.InvalidDataException]::new('Invalid owned-set member.') }
    }
}

function Undo-UpgradeOwnedSet([string]$InstallRoot, [string]$StateFile, $State) {
    $failures = [Collections.Generic.List[Exception]]::new()
    $State.phase = 'rolling_back'
    try { Write-UpgradeJournal $StateFile $State } catch { $failures.Add($_.Exception) }
    for ($index = $State.entries.Count - 1; $index -ge 0; $index--) {
        $entry = $State.entries[$index]
        if (!$entry.attempted) { continue }
        try {
            $source = Join-Path $InstallRoot $entry.name
            $backup = Join-Path $InstallRoot $entry.backup_name
            Assert-UpgradePlainPath $source
            Assert-UpgradePlainPath $backup
            $entry.restore_attempted = $true
            Write-UpgradeJournal $StateFile $State
            if ($entry.existed) {
                if (Test-Path -LiteralPath $backup) {
                    if (!(Test-UpgradeFileHash $backup $entry.old_sha256)) { throw [IO.InvalidDataException]::new('Old backup hash mismatch.') }
                    if (Test-Path -LiteralPath $source) {
                        Remove-UpgradeExactFile $source $entry.new_sha256
                    }
                    Restore-UpgradeFile $backup $source
                }
                # Missing backup is NOT successful recovery unless the old bytes
                # are actually back (covers crash/mutation-then-throw on restore).
                if (!(Test-UpgradeFileHash $source $entry.old_sha256)) { throw [IO.InvalidDataException]::new('Old member not restored.') }
            } else {
                if (Test-Path -LiteralPath $backup) { throw [IO.InvalidDataException]::new('Unexpected absent-member backup.') }
                if (Test-Path -LiteralPath $source) { Remove-UpgradeExactFile $source $entry.new_sha256 }
            }
            $entry.restored = $true
            Write-UpgradeJournal $StateFile $State
        } catch { $failures.Add($_.Exception) }
    }
    $State.phase = if ($failures.Count) { 'failed' } else { 'rolled_back' }
    $State.failure_count += $failures.Count
    try { Write-UpgradeJournal $StateFile $State } catch { $failures.Add($_.Exception) }
    if ($failures.Count) { throw [AggregateException]::new('Owned-set compensation incomplete; journal retained.', [Exception[]]$failures.ToArray()) }
}

function Suspend-UpgradeOwnedSet([string]$InstallRoot, [string]$StateFile, [string]$OperationId,
    [string]$ManifestPath, [string]$SignaturePath) {
    $installPath = [IO.Path]::GetFullPath($InstallRoot)
    if (!$StateFile -or [IO.Path]::GetDirectoryName([IO.Path]::GetFullPath($StateFile)) -ne $installPath -or
        $OperationId -notmatch '^[a-zA-Z0-9-]{1,100}$') { throw [ArgumentException]::new('Invalid owned-set identity.') }
    Assert-UpgradePlainPath $installPath
    if ((Test-Path -LiteralPath $StateFile) -or (Test-Path -LiteralPath ($StateFile + '.pending'))) {
        throw [InvalidOperationException]::new('Previous owned-set journal requires recovery; refusing new transaction.')
    }
    Assert-UpgradePlainPath $ManifestPath
    Assert-UpgradePlainPath $SignaturePath
    if ((Get-Item -LiteralPath $ManifestPath).Length -gt 1MB) { throw [IO.InvalidDataException]::new('New manifest exceeds bound.') }
    $manifest = Get-Content -LiteralPath $ManifestPath -Raw | ConvertFrom-Json
    $newHashes = @{}
    if (@($manifest.files).Count -gt 4096) { throw [IO.InvalidDataException]::new('Too many new manifest members.') }
    foreach ($file in $manifest.files) {
        if (!$file.path -or $newHashes.ContainsKey($file.path) -or $file.sha256 -notmatch '^[a-fA-F0-9]{64}$') {
            throw [IO.InvalidDataException]::new('Invalid new manifest member.')
        }
        $newHashes[$file.path] = $file.sha256
    }
    # Sidecars cannot hash themselves in the signed manifest. Their expected
    # hashes also come from the extracted NEW package, before any old-file move.
    $newHashes['NxTools.release.json'] = (Get-FileHash -LiteralPath $ManifestPath -Algorithm SHA256).Hash
    $newHashes['NxTools.release.sig'] = (Get-FileHash -LiteralPath $SignaturePath -Algorithm SHA256).Hash
    $entries = @()
    foreach ($name in @(Get-UpgradeOwnedNames)) {
        if (!$newHashes[$name]) { throw [IO.InvalidDataException]::new('Incomplete new release manifest.') }
        $source = Join-Path $installPath $name
        $backupName = '.' + $name + '-' + $OperationId + '.upgrade-old'
        Assert-UpgradePlainPath $source
        if (Test-Path -LiteralPath (Join-Path $installPath $backupName)) { throw [IO.IOException]::new('Recovery backup already exists.') }
        $existed = Test-Path -LiteralPath $source -PathType Leaf
        if ((Test-Path -LiteralPath $source) -and !$existed) { throw [IO.InvalidDataException]::new('Owned member is not a file.') }
        $oldHash = if ($existed) { (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash } else { '' }
        $entries += @{ name=$name; backup_name=$backupName; existed=[bool]$existed; old_sha256=$oldHash;
            new_sha256=$newHashes[$name]; attempted=$false; restore_attempted=$false; restored=$false; cleanup_attempted=$false }
    }
    $state = @{schema=2; root=$installPath; operation=$OperationId; phase='preparing'; failure_count=0; entries=$entries}
    if (!(Test-Path -LiteralPath $installPath)) { [IO.Directory]::CreateDirectory($installPath) | Out-Null }
    Write-UpgradeJournal $StateFile $state
    Write-Output "installer.upgrade_set.accepted correlation=$OperationId count=$($entries.Count)"
    try {
        foreach ($entry in $entries) {
            $entry.attempted = $true
            Write-UpgradeJournal $StateFile $state
            if ($entry.existed) {
                $source = Join-Path $installPath $entry.name
                if (!(Test-UpgradeFileHash $source $entry.old_sha256)) { throw [IO.InvalidDataException]::new('Old member changed before suspension.') }
                # In particular removes BrowserHost's launch path before Inno can
                # replace it, preserving the old late-browser-relaunch invariant.
                if ($entry.name -eq 'NxTools.BrowserHost.exe') {
                    $hostTargets = @{ $source = 'installed' }
                    for ($attempt = 0; $attempt -lt 10; $attempt++) {
                        Stop-UpgradeTargets $hostTargets | Out-Null
                        if (!(Test-UpgradeFileHash $source $entry.old_sha256)) { throw [IO.InvalidDataException]::new('Browser host changed during suspension retry.') }
                        try { Move-UpgradeFile $source (Join-Path $installPath $entry.backup_name); break }
                        catch [IO.IOException] {
                            if ($attempt -eq 9 -or !(Test-Path -LiteralPath $source)) { throw }
                        } catch [UnauthorizedAccessException] {
                            if ($attempt -eq 9) { throw [ComponentModel.Win32Exception]::new(5) }
                        }
                        Start-Sleep -Milliseconds 150
                    }
                    continue
                }
                Move-UpgradeFile $source (Join-Path $installPath $entry.backup_name)
            }
        }
        $state.phase = 'suspended'
        Write-UpgradeJournal $StateFile $state
        Write-Output "installer.upgrade_set.suspended correlation=$OperationId count=$($entries.Count)"
    } catch {
        $failures = [Collections.Generic.List[Exception]]::new()
        $failures.Add($_.Exception)
        $state.failure_count++
        try { Undo-UpgradeOwnedSet $installPath $StateFile $state } catch { $failures.Add($_.Exception) }
        Write-Output "installer.upgrade_set.failed correlation=$OperationId stage=suspend failures=$($failures.Count)"
        throw [AggregateException]::new('Owned-set suspension failed; compensation attempted; journal retained.', [Exception[]]$failures.ToArray())
    }
}

function Assert-UpgradeNewSet([string]$InstallRoot, $State) {
    foreach ($entry in $State.entries) {
        if (!$entry.attempted -or $entry.restore_attempted -or $entry.restored -or
            !(Test-UpgradeFileHash (Join-Path $InstallRoot $entry.name) $entry.new_sha256)) {
            throw [IO.InvalidDataException]::new('New release identity mismatch at commit.')
        }
    }
}

function Remove-UpgradeJournal([string]$StateFile) { [IO.File]::Delete($StateFile) }

function Complete-UpgradeSetCleanup([string]$InstallRoot, [string]$StateFile, $State) {
    # The durable committing record is the point of no return. Cleanup can be
    # retried, but must NEVER compensate or mutate any installed new member.
    try {
        Assert-UpgradeNewSet $InstallRoot $State
        foreach ($entry in $State.entries) {
            $backup = Join-Path $InstallRoot $entry.backup_name
            Assert-UpgradePlainPath $backup
            if (!$entry.existed) {
                if ($entry.cleanup_attempted -or (Test-Path -LiteralPath $backup)) { throw [IO.InvalidDataException]::new('Unexpected absent-member cleanup record.') }
                continue
            }
            $present = Test-Path -LiteralPath $backup
            if ((!$present -and !$entry.cleanup_attempted) -or
                ($State.phase -eq 'committed' -and (!$entry.cleanup_attempted -or $present))) {
                throw [IO.InvalidDataException]::new('Incomplete backup cleanup evidence.')
            }
            if ($present -and !(Test-UpgradeFileHash $backup $entry.old_sha256)) {
                throw [IO.InvalidDataException]::new('Old cleanup identity mismatch.')
            }
        }
        foreach ($entry in $State.entries) {
            if (!$entry.existed) { continue }
            $backup = Join-Path $InstallRoot $entry.backup_name
            if (!(Test-Path -LiteralPath $backup)) {
                if (!$entry.cleanup_attempted) { throw [IO.InvalidDataException]::new('Backup disappeared without durable cleanup attempt.') }
                continue
            }
            $entry.cleanup_attempted = $true
            Write-UpgradeJournal $StateFile $State
            Remove-UpgradeExactFile $backup $entry.old_sha256
        }
        Assert-UpgradeNewSet $InstallRoot $State
        foreach ($entry in $State.entries) {
            if (Test-Path -LiteralPath (Join-Path $InstallRoot $entry.backup_name)) { throw [IO.InvalidDataException]::new('Backup remains after exact cleanup.') }
        }
        if ($State.phase -ne 'committed') {
            $State.phase = 'committed'
            Write-UpgradeJournal $StateFile $State
        }
        Remove-UpgradeJournal $StateFile
        Write-Output "installer.upgrade_set.committed correlation=$($State.operation) count=$($State.entries.Count)"
    } catch {
        Write-Output "installer.upgrade_set.failed correlation=$($State.operation) stage=cleanup failures=1"
        throw [AggregateException]::new('Owned-set cleanup failed after commit point; new set and journal retained.', [Exception[]]@($_.Exception))
    }
}

function Complete-UpgradeOwnedSet([string]$InstallRoot, [string]$StateFile, $State, [switch]$Committed) {
    Assert-UpgradeSetRecord $InstallRoot $State
    Assert-UpgradePlainPath $StateFile
    if ($State.phase -in @('committing','committed')) {
        if (!$Committed) { throw [InvalidOperationException]::new('Rollback refused after durable commit point; new set and journal retained.') }
        Complete-UpgradeSetCleanup $InstallRoot $StateFile $State
        return
    }
    if (!$Committed) {
        if ($State.phase -ne 'suspended') { $State.failure_count++ }
        Undo-UpgradeOwnedSet $InstallRoot $StateFile $State
        if ($State.failure_count -eq 0) { [IO.File]::Delete($StateFile) }
        else { throw [InvalidOperationException]::new('Prior owned-set failure recovered but journal retained; not successful finalization.') }
        Write-Output "installer.upgrade_set.rolled_back correlation=$($State.operation) failures=$($State.failure_count)"
        return
    }
    try {
        if ($State.phase -ne 'suspended' -or $State.failure_count -ne 0) { throw [InvalidOperationException]::new('Set is not eligible to commit.') }
        # Before the commit point, validation failures still compensate the whole
        # attempted set in reverse. No backup is discarded in this region.
        Assert-UpgradeNewSet $InstallRoot $State
        foreach ($entry in $State.entries) {
            $backup = Join-Path $InstallRoot $entry.backup_name
            if ($entry.cleanup_attempted -or ($entry.existed -and !(Test-UpgradeFileHash $backup $entry.old_sha256)) -or
                (!$entry.existed -and (Test-Path -LiteralPath $backup))) {
                throw [IO.InvalidDataException]::new('Old recovery identity mismatch at commit.')
            }
        }
        $State.phase = 'committing'
        Write-UpgradeJournal $StateFile $State
    } catch {
        $failures = [Collections.Generic.List[Exception]]::new()
        $failures.Add($_.Exception)
        if ($State.phase -eq 'committing') {
            # Publication may mutate then throw. Re-read only our known journal;
            # if publication is uncertain, retain all bytes instead of undoing a
            # potentially durable commit decision.
            try {
                $durable = Get-Content -LiteralPath $StateFile -Raw | ConvertFrom-Json
                Assert-UpgradeSetRecord $InstallRoot $durable
                if ($durable.operation -ne $State.operation -or $durable.phase -ne 'suspended') {
                    throw [IO.InvalidDataException]::new('Commit decision published or uncertain.')
                }
            } catch {
                $failures.Add($_.Exception)
                throw [AggregateException]::new('Owned-set commit publication uncertain; new set and journal retained.', [Exception[]]$failures.ToArray())
            }
        }
        $State.failure_count++
        try { Undo-UpgradeOwnedSet $InstallRoot $StateFile $State } catch { $failures.Add($_.Exception) }
        Write-Output "installer.upgrade_set.failed correlation=$($State.operation) stage=commit failures=$($failures.Count)"
        throw [AggregateException]::new('Owned-set commit failed before commit point; compensation attempted; journal retained.', [Exception[]]$failures.ToArray())
    }
    # Deliberately outside the pre-commit compensation catch.
    Complete-UpgradeSetCleanup $InstallRoot $StateFile $State
}

function Prepare-UpgradeBrowserHost([string]$InstallRoot, [string]$StateFile, [string]$OperationId,
    [string]$NewManifestPath = '', [string]$NewSignaturePath = '') {
    $installPath = [IO.Path]::GetFullPath($InstallRoot)
    $statePath = [IO.Path]::GetFullPath($StateFile)
    if (![string]::Equals($statePath, (Join-Path $installPath '.NxTools.BrowserHost-upgrade.json'), [StringComparison]::OrdinalIgnoreCase) -or
        $OperationId -notmatch '^[a-zA-Z0-9-]{1,100}$') { throw [ArgumentException]::new('Invalid preparation identity.') }
    Assert-UpgradePlainPath $statePath
    if (Test-Path -LiteralPath ($statePath + '.pending')) {
        throw [IO.InvalidDataException]::new('Interrupted journal publication cannot be proved; recovery record retained.')
    }
    if (Test-Path -LiteralPath $statePath) {
        if (!(Test-Path -LiteralPath $statePath -PathType Leaf) -or (Get-Item -LiteralPath $statePath).Length -gt 1MB) {
            throw [IO.InvalidDataException]::new('Invalid preparation recovery journal.')
        }
        $state = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
        if ($state.schema -isnot [int] -or $state.schema -ne 2) {
            throw [IO.InvalidDataException]::new('Unknown preparation recovery schema; journal retained.')
        }
        Assert-UpgradeSetRecord $installPath $state
        Write-Output "installer.upgrade_set.recovery_started correlation=$OperationId original_operation=$($state.operation) phase=$($state.phase)"
        # Recovery authority is exclusively the OLD journal and actual old transaction files.
        # Never consult this new install's manifest to authorize deleting an old transaction member.
        if ($state.phase -in @('committing','committed')) {
            Complete-UpgradeOwnedSet $installPath $statePath $state -Committed
        } else {
            if (@($state.entries | Where-Object { $_.cleanup_attempted }).Count) {
                throw [IO.InvalidDataException]::new('Precommit journal contains commit cleanup evidence; retained.')
            }
            # Unlike a failed invocation's Finalize, a new preparation may retire a historically
            # failed journal only AFTER the original compensation and complete old-set readback.
            Undo-UpgradeOwnedSet $installPath $statePath $state
            foreach ($entry in $state.entries) {
                $source = Join-Path $installPath $entry.name
                $backup = Join-Path $installPath $entry.backup_name
                Assert-UpgradePlainPath $source
                Assert-UpgradePlainPath $backup
                if ((Test-Path -LiteralPath $backup) -or
                    ($entry.existed -and !(Test-UpgradeFileHash $source $entry.old_sha256)) -or
                    (!$entry.existed -and (Test-Path -LiteralPath $source))) {
                    throw [IO.InvalidDataException]::new('Previous complete old set not recovered; journal retained.')
                }
            }
            Remove-UpgradeJournal $statePath
        }
        if ((Test-Path -LiteralPath $statePath) -or (Test-Path -LiteralPath ($statePath + '.pending'))) {
            throw [IO.InvalidDataException]::new('Previous journal recovery incomplete; new transaction refused.')
        }
        Write-Output "installer.upgrade_set.recovered correlation=$OperationId original_operation=$($state.operation)"
    }
    Suspend-UpgradeBrowserHost $installPath $statePath $OperationId -NewManifestPath $NewManifestPath -NewSignaturePath $NewSignaturePath
}
function Invoke-UpgradeBootstrapMigration([string]$Package, [string]$OperationId, [switch]$RepairMissing, [switch]$RotateTrusted) {
    $migrationHost = Join-Path $Package 'NxTools.PlatformHost.exe'
    Assert-UpgradePlainPath $migrationHost
    $repairId=[Guid]::NewGuid()
    Write-Output "installer.bootstrap_migration.started correlation=$OperationId repair_id=$repairId missing_root=$([int][bool]$RepairMissing) trust_rotation=$([int][bool]$RotateTrusted)"
    if ($RepairMissing) {
        & $migrationHost --repair-missing-trusted-bootstrap $repairId.ToString('D')
    } elseif ($RotateTrusted) {
        & $migrationHost --rotate-trusted-bootstrap
    } else {
        & $migrationHost --maintain-trusted-bootstrap
    }
    if ($LASTEXITCODE -ne 0) { throw [InvalidOperationException]::new('Updater migration failed before application file replacement.') }
    Write-Output "installer.bootstrap_migration.completed correlation=$OperationId repair_id=$repairId"
}

function Write-UpgradeDiagnostic([string]$EventName, [string]$Stage, $FailureRecord = $null, [int]$NativeCode = 0) {
    # Observational only: bounded metadata, never exception messages, member paths or child output.
    if (!$DiagnosticPath -or $CorrelationId -notmatch '^[a-zA-Z0-9-]{1,100}$') { return }
    try {
        Assert-UpgradePlainPath $DiagnosticPath
        if ((Test-Path -LiteralPath $DiagnosticPath) -and
            (!(Test-Path -LiteralPath $DiagnosticPath -PathType Leaf) -or (Get-Item -LiteralPath $DiagnosticPath).Length -gt 61440)) { return }
        $record = [ordered]@{ schema=1; correlation=$CorrelationId; event=$EventName; stage=$Stage;
            elapsed_ms=$watch.ElapsedMilliseconds; native_code=$NativeCode; process_bits=([IntPtr]::Size * 8) }
        $record.elevated = ([Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
        if ($FailureRecord) {
            $exception = $FailureRecord.Exception
            $chain = @()
            while ($exception -and $chain.Count -lt 8) {
                $chain += @{ type=$exception.GetType().FullName; hresult=$exception.HResult }
                $exception = $exception.InnerException
            }
            $record.exception_type = $FailureRecord.Exception.GetType().FullName
            $record.script_line = $FailureRecord.InvocationInfo.ScriptLineNumber
            $record.exception_chain = $chain
            $nativeFailure = $FailureRecord.Exception
            while ($nativeFailure.InnerException) { $nativeFailure = $nativeFailure.InnerException }
            $nativeApi = [string]$nativeFailure.Data['native_api']
            if ($nativeFailure.Data['process_id'] -is [int] -and $nativeFailure.Data['process_id'] -gt 0) {
                $record.process_id = $nativeFailure.Data['process_id']
            }
            $processName = [string]$nativeFailure.Data['process_name']
            if ($processName -in @('NxTools.exe','NxTools.PlatformHost.exe','NxTools.BrowserHost.exe','NxTools.Cli.exe','NxTools.ArchiveWorker.exe','NxTools.ShellSelectionHelper.exe')) {
                $record.process_name = $processName
            }
            if ($nativeApi -in @('open','query_image','terminate','wait')) { $record.native_api = $nativeApi }
        }
        $line = ($record | ConvertTo-Json -Depth 5 -Compress) + [Environment]::NewLine
        if ([Text.Encoding]::UTF8.GetByteCount($line) -le 4096) {
            [IO.File]::AppendAllText($DiagnosticPath, $line, [Text.UTF8Encoding]::new($false))
        }
    } catch { # A diagnostic sink failure cannot change preparation, recovery or its exit code.
    }
}

if ($LibraryOnly) { return }
$watch = [Diagnostics.Stopwatch]::StartNew()
$diagnosticStage = 'entry'
Write-UpgradeDiagnostic 'started' $diagnosticStage
Write-Output "installer.upgrade_close.started correlation=$CorrelationId"
try {
    if ($Finalize) {
        $diagnosticStage = 'finalization'
        Write-UpgradeDiagnostic 'stage' $diagnosticStage
        Complete-UpgradeBrowserHost $InstallDirectory $StatePath -Committed:$Committed -RollbackNewSha256 $RollbackNewSha256
        Write-Output "installer.upgrade_files.finalized correlation=$CorrelationId committed=$Committed elapsed_ms=$($watch.ElapsedMilliseconds)"
        Write-UpgradeDiagnostic 'completed' $diagnosticStage
        exit 0
    }
    if ($BootstrapMigrationPackage) {
        $diagnosticStage = 'bootstrap_migration'
        Invoke-UpgradeBootstrapMigration $BootstrapMigrationPackage $CorrelationId -RepairMissing:$RepairMissingBootstrap -RotateTrusted:$RotateTrustedBootstrap
    }
    if (!$InstallDirectory -or !(Test-Path -LiteralPath $InstallDirectory -PathType Container)) {
        # First installation has no prior application directory, but the protected
        # tuple may still belong to an existing installation.
        if (!$InstallDirectory) { throw [ArgumentException]::new('Missing installation directory.') }
    }
    $protectedRoot = Join-Path $env:ProgramData 'NxTools\NxTools.Next\Service'
    $diagnosticStage = 'registration'
    Write-UpgradeDiagnostic 'stage' $diagnosticStage
    $approval = $null
    $serviceKey = 'HKLM:\SYSTEM\CurrentControlSet\Services\NxToolsIndexerV1'
    if (Test-Path $serviceKey) {
        $registration = Get-ItemProperty $serviceKey
        $approval = @{ ServicePath = $registration.ImagePath; DesktopPath = $registration.NxToolsDesktopPath; ArchiveHashesJson = $registration.NxToolsArchiveReleaseSha256; Hashes = @{
            'NxTools.exe' = $registration.NxToolsDesktopSha256
            'NxTools.PlatformHost.exe' = $registration.NxToolsPlatformHostSha256
            'NxTools.Indexer.exe' = $registration.NxToolsBinarySha256
            'NxTools.BrowserHost.exe' = $registration.NxToolsBrowserHostSha256
        }}
    }
    $diagnosticStage = 'target_identity'
    Write-UpgradeDiagnostic 'stage' $diagnosticStage
    $targets = Get-UpgradeTargets $InstallDirectory $protectedRoot $approval
    $diagnosticStage = 'process_close'
    Write-UpgradeDiagnostic 'stage' $diagnosticStage
    $closedCount = Stop-UpgradeTargets $targets $protectedRoot
    $diagnosticStage = 'file_preparation'
    Write-UpgradeDiagnostic 'stage' $diagnosticStage
    Prepare-UpgradeBrowserHost $InstallDirectory $StatePath $CorrelationId -NewManifestPath $NewManifestPath -NewSignaturePath $NewSignaturePath
    Write-Output "installer.upgrade_close.completed correlation=$CorrelationId count=$closedCount elapsed_ms=$($watch.ElapsedMilliseconds)"
    Write-UpgradeDiagnostic 'completed' $diagnosticStage
    exit 0
} catch {
    $failure = $_.Exception
    while ($failure.InnerException) { $failure = $failure.InnerException }
    $nativeCode = if ($failure -is [ComponentModel.Win32Exception]) { $failure.NativeErrorCode } else { 0 }
    if ($failure -is [UnauthorizedAccessException]) { $nativeCode = 5 }
    Write-UpgradeDiagnostic 'failed' $diagnosticStage $_ $nativeCode
    Write-Output "installer.upgrade_close.failed correlation=$CorrelationId native_code=$nativeCode elapsed_ms=$($watch.ElapsedMilliseconds)"
    if ($nativeCode -eq 5) { exit 5 }
    exit 1
}
