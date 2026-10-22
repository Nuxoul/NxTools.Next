param(
    [string]$InstallDirectory,
    [switch]$LibraryOnly,
    [string]$CorrelationId = [Guid]::NewGuid().ToString('N'),
    [string]$StatePath,
    [switch]$Finalize,
    [switch]$Committed,
    [string]$RollbackNewSha256 = ''
)
$ErrorActionPreference = 'Stop'

function Get-UpgradeTargets([string]$InstallRoot, [string]$ProtectedRoot, $Approval) {
    $targets = @{}
    foreach ($name in @('NxTools.exe', 'NxTools.PlatformHost.exe', 'NxTools.BrowserHost.exe', 'NxTools.Cli.exe')) {
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
    // Query and termination share the same handle: PID reuse cannot redirect a kill.
    public static bool Stop(int id, string expected) {
        IntPtr handle = OpenProcess(0x00101001, false, id);
        if (handle == IntPtr.Zero) {
            int code = Marshal.GetLastWin32Error();
            if (code == 87) return false; // process already exited
            throw new Win32Exception(code);
        }
        try {
            var path = new StringBuilder(32768); int size = path.Capacity;
            if (!QueryFullProcessImageName(handle, 0, path, ref size)) throw new Win32Exception(Marshal.GetLastWin32Error());
            if (!String.Equals(System.IO.Path.GetFullPath(path.ToString()), expected, StringComparison.OrdinalIgnoreCase)) return false;
            if (WaitForSingleObject(handle, 0) == 0) return false;
            if (!TerminateProcess(handle, 0)) throw new Win32Exception(Marshal.GetLastWin32Error());
            if (WaitForSingleObject(handle, 5000) != 0) throw new TimeoutException("Owned process termination timed out.");
            return true;
        } finally { CloseHandle(handle); }
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

function Stop-UpgradeTargets($Targets, [string]$ProtectedRoot = '') {
    Initialize-UpgradeNative
    $stopped = 0
    # Desktop first; then hosts. Bounded repeat covers an already-starting child.
    for ($round = 0; $round -lt 3; $round++) {
        $remaining = 0
        foreach ($name in @('NxTools.exe', 'NxTools.PlatformHost.exe', 'NxTools.BrowserHost.exe', 'NxTools.Cli.exe')) {
            foreach ($entry in @(Get-CimInstance Win32_Process -Filter "Name='$name'")) {
                if (!$entry.ExecutablePath) {
                    # Cannot establish ownership at current privilege. Retry elevated,
                    # never guess from the name or silently certify cleanup.
                    throw [ComponentModel.Win32Exception]::new(5)
                }
                $image = [IO.Path]::GetFullPath($entry.ExecutablePath)
                if (!$Targets.ContainsKey($image)) {
                    if ($ProtectedRoot -and $name -eq 'NxTools.exe' -and
                        $image -eq [IO.Path]::GetFullPath((Join-Path $ProtectedRoot $name))) {
                        throw [InvalidOperationException]::new('Running protected desktop identity is not approved.')
                    }
                    continue
                }
                $remaining++
                if ([NxToolsUpgradeProcess]::Stop([int]$entry.ProcessId, $image)) { $stopped++ }
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

function Suspend-UpgradeBrowserHost([string]$InstallRoot, [string]$StateFile, [string]$OperationId) {
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
    if (!(Test-Path -LiteralPath $StateFile -PathType Leaf)) { return }
    $state = Get-Content -LiteralPath $StateFile -Raw | ConvertFrom-Json
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

if ($LibraryOnly) { return }
$watch = [Diagnostics.Stopwatch]::StartNew()
Write-Output "installer.upgrade_close.started correlation=$CorrelationId"
try {
    if ($Finalize) {
        Complete-UpgradeBrowserHost $InstallDirectory $StatePath -Committed:$Committed -RollbackNewSha256 $RollbackNewSha256
        Write-Output "installer.upgrade_files.finalized correlation=$CorrelationId committed=$Committed elapsed_ms=$($watch.ElapsedMilliseconds)"
        exit 0
    }
    if (!$InstallDirectory -or !(Test-Path -LiteralPath $InstallDirectory -PathType Container)) {
        # First installation has no prior application directory, but the protected
        # tuple may still belong to an existing installation.
        if (!$InstallDirectory) { throw [ArgumentException]::new('Missing installation directory.') }
    }
    $protectedRoot = Join-Path $env:ProgramData 'NxTools\NxTools.Next\Service'
    $approval = $null
    $serviceKey = 'HKLM:\SYSTEM\CurrentControlSet\Services\NxToolsIndexerV1'
    if (Test-Path $serviceKey) {
        $registration = Get-ItemProperty $serviceKey
        $approval = @{ ServicePath = $registration.ImagePath; DesktopPath = $registration.NxToolsDesktopPath; Hashes = @{
            'NxTools.exe' = $registration.NxToolsDesktopSha256
            'NxTools.PlatformHost.exe' = $registration.NxToolsPlatformHostSha256
            'NxTools.Indexer.exe' = $registration.NxToolsBinarySha256
            'NxTools.BrowserHost.exe' = $registration.NxToolsBrowserHostSha256
        }}
    }
    $targets = Get-UpgradeTargets $InstallDirectory $protectedRoot $approval
    $closedCount = Stop-UpgradeTargets $targets $protectedRoot
    Suspend-UpgradeBrowserHost $InstallDirectory $StatePath $CorrelationId
    Write-Output "installer.upgrade_close.completed correlation=$CorrelationId count=$closedCount elapsed_ms=$($watch.ElapsedMilliseconds)"
    exit 0
} catch {
    $failure = $_.Exception
    while ($failure.InnerException) { $failure = $failure.InnerException }
    $nativeCode = if ($failure -is [ComponentModel.Win32Exception]) { $failure.NativeErrorCode } else { 0 }
    if ($failure -is [UnauthorizedAccessException]) { $nativeCode = 5 }
    Write-Output "installer.upgrade_close.failed correlation=$CorrelationId native_code=$nativeCode elapsed_ms=$($watch.ElapsedMilliseconds)"
    if ($nativeCode -eq 5) { exit 5 }
    exit 1
}
