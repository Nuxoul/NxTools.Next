using System.Diagnostics;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;

namespace NxTools.PlatformHost.Windows.Archive;

internal enum ArchiveRecycleKind { Recycled, Cancelled, Unsupported, Failed, OutcomeUnknown, NotAttempted }
internal sealed record ArchiveRecycleNativeResult(ArchiveRecycleKind Kind, int? NativeCode = null);
internal sealed record ArchiveRecycleOutcome(Guid SourceId, ArchiveRecycleKind Kind, int? NativeCode = null);
internal sealed record ArchiveSourceDeletionResult(string Reason, IReadOnlyList<ArchiveRecycleOutcome> Sources);

// No worker path, parent or recursive authority. The original C# owner keeps its leases live.
internal sealed class ArchiveRecycleSource
{
    internal Guid Id { get; }
    private readonly Action verify;
    internal ArchiveRecycleSource(Guid id, Action verify) { Id = id; this.verify = verify; }
    internal void Verify() => verify();
}
internal interface IArchiveRecycleBackend
{
    ArchiveRecycleNativeResult Recycle(ArchiveRecycleSource source, CancellationToken cancellation);
}

// Payload-safe observational diagnostics, not a factory or holder of delete authority.
internal static class ArchiveRecycleDiagnostics
{
    internal static void SafeLog(IPlatformLog? log, string phase, Guid operation, Stopwatch clock, string reason, int? nativeCode, int count)
    {
        try { log?.Write("info", "archive.recycle." + phase, operation, new Dictionary<string, object?> {
            ["elapsed_ms"] = clock.ElapsedMilliseconds, ["reason"] = reason, ["native_error_code"] = nativeCode, ["source_count"] = count }); }
        catch (Exception) { /* Diagnostics cannot affect qualification, archive or recycle outcome. */ }
    }
}

internal static class ArchiveRecycleVersion
{
    [System.Runtime.InteropServices.StructLayout(System.Runtime.InteropServices.LayoutKind.Sequential)]
    private struct BasicInfo { internal long Creation, Access, Modified, Changed; internal uint Attributes; }
    [System.Runtime.InteropServices.DllImport("kernel32.dll", EntryPoint = "GetFileInformationByHandleEx", ExactSpelling = true, SetLastError = true)]
    [return: System.Runtime.InteropServices.MarshalAs(System.Runtime.InteropServices.UnmanagedType.Bool)]
    private static extern bool ReadBasic(Microsoft.Win32.SafeHandles.SafeFileHandle handle, int kind, out BasicInfo info, uint size);
    internal static object Capture(Microsoft.Win32.SafeHandles.SafeFileHandle handle)
    {
        if (!ReadBasic(handle, 0, out var before, (uint)System.Runtime.InteropServices.Marshal.SizeOf<BasicInfo>()))
            throw new ArchiveOutputException("source_unavailable", System.Runtime.InteropServices.Marshal.GetLastWin32Error());
        var identity = OutputNative.Identity(handle);
        if (!ReadBasic(handle, 0, out var after, (uint)System.Runtime.InteropServices.Marshal.SizeOf<BasicInfo>()))
            throw new ArchiveOutputException("source_unavailable", System.Runtime.InteropServices.Marshal.GetLastWin32Error());
        if (before.Modified != after.Modified || before.Changed != after.Changed || before.Attributes != after.Attributes ||
            after.Modified != identity.Modified || (after.Attributes & 0x410) != 0) throw new ArchiveOutputException("source_changed");
        return (identity, after.Changed, after.Attributes);
    }
}
