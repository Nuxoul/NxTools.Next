using System.Runtime.InteropServices;

namespace NxTools.ShellSelectionHelper;

// Exact Windows SDK 10.0.26100.0 ShObjIdl_core.h vtable order. Only the methods this helper
// calls are meaningful; earlier slots are declared to keep the COM vtable layout exact.
[ComImport, Guid("43826d1e-e718-42ee-bc55-a1e261c37bfe"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
internal interface IShellItem
{
    [PreserveSig] int BindToHandler(IntPtr bindContext, ref Guid handler, ref Guid iid, out IntPtr result);
    [PreserveSig] int GetParent(out IShellItem parent);
    [PreserveSig] int GetDisplayName(uint sigdnName, out IntPtr name);
    [PreserveSig] int GetAttributes(uint mask, out uint attributes);
    [PreserveSig] int Compare(IShellItem other, uint hint, out int order);
}

[ComImport, Guid("b63ea76d-1f85-456f-a19c-48159efa858b"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
internal interface IShellItemArray
{
    [PreserveSig] int BindToHandler(IntPtr bindContext, ref Guid handler, ref Guid iid, out IntPtr result);
    [PreserveSig] int GetPropertyStore(int flags, ref Guid iid, out IntPtr result);
    [PreserveSig] int GetPropertyDescriptionList(IntPtr keyType, ref Guid iid, out IntPtr result);
    [PreserveSig] int GetAttributes(int attributeFlags, uint mask, out uint attributes);
    [PreserveSig] int GetCount(out uint count);
    [PreserveSig] int GetItemAt(uint index, out IShellItem item);
    [PreserveSig] int EnumItems(out IntPtr enumerator);
}

[ComImport, Guid("1c9cd5bb-98e9-4491-a60f-31aacc72b83c"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
internal interface IObjectWithSelection
{
    [PreserveSig] int SetSelection(IShellItemArray? selection);
    [PreserveSig] int GetSelection(ref Guid iid, out IntPtr result);
}

[ComImport, Guid("7F9185B0-CB92-43c5-80A9-92277A4F7B54"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
internal interface IExecuteCommand
{
    [PreserveSig] int SetKeyState(uint keyState);
    [PreserveSig] int SetParameters([MarshalAs(UnmanagedType.LPWStr)] string? parameters);
    [PreserveSig] int SetPosition(NativePoint point);
    [PreserveSig] int SetShowWindow(int show);
    [PreserveSig] int SetNoShowUI([MarshalAs(UnmanagedType.Bool)] bool noShowUi);
    [PreserveSig] int SetDirectory([MarshalAs(UnmanagedType.LPWStr)] string? directory);
    [PreserveSig] int Execute();
}

[ComImport, Guid("00000001-0000-0000-C000-000000000046"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
internal interface IClassFactory
{
    [PreserveSig] int CreateInstance(IntPtr outer, ref Guid iid, out IntPtr result);
    [PreserveSig] int LockServer([MarshalAs(UnmanagedType.Bool)] bool lockServer);
}

[StructLayout(LayoutKind.Sequential)]
internal struct NativePoint
{
    public int X;
    public int Y;
}

internal enum ShellVerb
{
    Open,
    ExtractSmart,
    ExtractHere,
    ExtractName,
    ExtractDialog,
    Test,
    Compress,
}

internal enum SelectionFailure
{
    None,
    NoSelection,
    Empty,
    TooMany,
    CountFailed,
    ItemFailed,
    NotFileSystem,
    PathTooLong,
    TooManyBytes,
    InvalidPath,
}

internal sealed record ShellSelection(IReadOnlyList<string> Paths, SelectionFailure Failure, int NativeError)
{
    internal bool Succeeded => Failure == SelectionFailure.None;
}

/// <summary>
/// Reads one IShellItemArray completely and in Shell order. Any unreadable, non-filesystem or
/// over-limit item fails the WHOLE selection; nothing is silently dropped or guessed.
/// Bounds equal the existing Desktop archive intent wire limits.
/// </summary>
internal static class ShellSelectionReader
{
    internal const int MaxPaths = 256;
    internal const int MaxPathBytes = 32 * 1024;
    internal const int MaxRequestPathBytes = 256 * 1024;
    internal const uint SigdnFileSysPath = 0x80058000;

    internal static ShellSelection Read(IShellItemArray? selection)
    {
        if (selection is null) return Fail(SelectionFailure.NoSelection, 0);
        var hr = selection.GetCount(out var count);
        if (hr < 0) return Fail(SelectionFailure.CountFailed, hr);
        if (count == 0) return Fail(SelectionFailure.Empty, 0);
        if (count > MaxPaths) return Fail(SelectionFailure.TooMany, 0);
        var paths = new List<string>((int)count);
        var bytes = 0L;
        for (uint index = 0; index < count; index++)
        {
            hr = selection.GetItemAt(index, out var item);
            if (hr < 0 || item is null) return Fail(SelectionFailure.ItemFailed, hr);
            try
            {
                hr = item.GetDisplayName(SigdnFileSysPath, out var name);
                if (hr < 0 || name == IntPtr.Zero) return Fail(SelectionFailure.NotFileSystem, hr);
                string? path;
                try { path = Marshal.PtrToStringUni(name); }
                finally { Marshal.FreeCoTaskMem(name); }
                if (string.IsNullOrEmpty(path) || !IsAbsoluteFileSystemPath(path))
                    return Fail(SelectionFailure.InvalidPath, 0);
                var length = System.Text.Encoding.UTF8.GetByteCount(path);
                if (length > MaxPathBytes) return Fail(SelectionFailure.PathTooLong, 0);
                bytes += length;
                if (bytes > MaxRequestPathBytes) return Fail(SelectionFailure.TooManyBytes, 0);
                paths.Add(path);
            }
            finally
            {
                if (Marshal.IsComObject(item)) Marshal.ReleaseComObject(item);
            }
        }
        return new ShellSelection(paths, SelectionFailure.None, 0);
    }

    /// <summary>Same accepted shapes as the Rust DesktopArchiveIntent validation.</summary>
    internal static bool IsAbsoluteFileSystemPath(string path)
    {
        if (path.Any(char.IsControl)) return false;
        if (path.Length >= 3 && char.IsAsciiLetter(path[0]) && path[1] == ':' && path[2] is '\\' or '/')
            return true;
        if (!path.StartsWith(@"\\", StringComparison.Ordinal) ||
            path.StartsWith(@"\\.\", StringComparison.Ordinal) ||
            path.StartsWith(@"\\?\", StringComparison.Ordinal))
            return false;
        return path[2..].Split('\\', '/').Take(2).Count(part => part.Length > 0) == 2;
    }

    private static ShellSelection Fail(SelectionFailure failure, int nativeError) => new([], failure, nativeError);
}

/// <summary>Forwards one complete selection once into the existing Desktop archive ingress.</summary>
internal interface ISelectionForwarder
{
    ForwardResult Forward(Guid correlationId, ShellVerb verb, IReadOnlyList<string> paths, CancellationToken cancellation);
}

internal enum ForwardStatus
{
    Delivered,
    /// <summary>No running owner: the launched desktop became the owner of its initial intent.
    /// The helper cannot observe that owner's later result and does not claim one.</summary>
    HandedToNewOwner,
    Rejected,
    IdentityRejected,
    Unavailable,
    TimedOut,
    Cancelled,
}

internal sealed record ForwardResult(ForwardStatus Status, int ExitCode);

/// <summary>Bounded path-free diagnostics: counts, codes and elapsed only.</summary>
internal interface IHelperLog
{
    void Write(string eventName, Guid correlationId, IReadOnlyDictionary<string, object?> fields);
}

/// <summary>
/// One COM object per Shell invocation, implementing IExecuteCommand + IObjectWithSelection.
/// SetSelection retains the array; Execute reads it once and forwards one ordered bounded array
/// exactly once. No timer, no coalescing with other invocations, no codec work in callbacks.
/// </summary>
[ComVisible(true), ClassInterface(ClassInterfaceType.None)]
internal sealed class ArchiveSelectionCommand : IExecuteCommand, IObjectWithSelection
{
    internal const int SOk = 0;
    internal const int EFail = unchecked((int)0x80004005);
    internal const int EIllegalMethodCall = unchecked((int)0x8000000E);
    internal const int ENoInterface = unchecked((int)0x80004002);

    private readonly ShellVerb _verb;
    private readonly ISelectionForwarder _forwarder;
    private readonly IHelperLog _log;
    private readonly CancellationToken _cancellation;
    private readonly Action _released;
    private IShellItemArray? _selection;
    private int _executed;
    private int _retired;

    internal ArchiveSelectionCommand(
        ShellVerb verb,
        ISelectionForwarder forwarder,
        IHelperLog log,
        CancellationToken cancellation,
        Action released)
    {
        _verb = verb;
        _forwarder = forwarder;
        _log = log;
        _cancellation = cancellation;
        _released = released;
    }

    internal Guid CorrelationId { get; } = Guid.NewGuid();
    internal IShellItemArray? Selection => Volatile.Read(ref _selection);
    internal ForwardResult? Result { get; private set; }
    internal SelectionFailure Failure { get; private set; }

    public int SetSelection(IShellItemArray? selection)
    {
        if (Volatile.Read(ref _executed) != 0) return EIllegalMethodCall;
        var previous = Interlocked.Exchange(ref _selection, selection);
        if (previous is not null && !ReferenceEquals(previous, selection) && Marshal.IsComObject(previous))
            Marshal.ReleaseComObject(previous);
        return SOk;
    }

    public int GetSelection(ref Guid iid, out IntPtr result)
    {
        result = IntPtr.Zero;
        var selection = Volatile.Read(ref _selection);
        if (selection is null) return EFail;
        var unknown = Marshal.GetIUnknownForObject(selection);
        try { return Marshal.QueryInterface(unknown, in iid, out result); }
        finally { Marshal.Release(unknown); }
    }

    public int SetKeyState(uint keyState) => SOk;
    public int SetParameters(string? parameters) => SOk;
    public int SetPosition(NativePoint point) => SOk;
    public int SetShowWindow(int show) => SOk;
    public int SetNoShowUI(bool noShowUi) => SOk;
    public int SetDirectory(string? directory) => SOk;

    public int Execute()
    {
        if (Interlocked.Exchange(ref _executed, 1) != 0) return EIllegalMethodCall;
        var started = System.Diagnostics.Stopwatch.GetTimestamp();
        Write("shell_selection.accepted", new Dictionary<string, object?> { ["verb"] = _verb.ToString() });
        try
        {
            if (_cancellation.IsCancellationRequested)
                return Finish(new ForwardResult(ForwardStatus.Cancelled, 0), started, EFail);
            var selection = ShellSelectionReader.Read(Volatile.Read(ref _selection));
            Failure = selection.Failure;
            if (!selection.Succeeded)
            {
                Write("shell_selection.terminal", new Dictionary<string, object?>
                {
                    ["verb"] = _verb.ToString(),
                    ["outcome"] = "selection_rejected",
                    ["failure"] = selection.Failure.ToString(),
                    ["native_error_code"] = selection.NativeError,
                    ["elapsed_ms"] = System.Diagnostics.Stopwatch.GetElapsedTime(started).TotalMilliseconds,
                });
                return selection.NativeError < 0 ? selection.NativeError : EFail;
            }
            if (!VerbAccepts(_verb, selection.Paths.Count))
            {
                Failure = SelectionFailure.TooMany;
                Write("shell_selection.terminal", new Dictionary<string, object?>
                {
                    ["verb"] = _verb.ToString(),
                    ["outcome"] = "arity_rejected",
                    ["count"] = selection.Paths.Count,
                    ["elapsed_ms"] = System.Diagnostics.Stopwatch.GetElapsedTime(started).TotalMilliseconds,
                });
                return EFail;
            }
            Write("shell_selection.started", new Dictionary<string, object?>
            {
                ["verb"] = _verb.ToString(),
                ["count"] = selection.Paths.Count,
            });
            var result = _forwarder.Forward(CorrelationId, _verb, selection.Paths, _cancellation);
            var accepted = result.Status is ForwardStatus.Delivered or ForwardStatus.HandedToNewOwner;
            return Finish(result, started, accepted ? SOk : EFail, selection.Paths.Count);
        }
        catch (Exception error)
        {
            Write("shell_selection.terminal", new Dictionary<string, object?>
            {
                ["verb"] = _verb.ToString(),
                ["outcome"] = "exception",
                ["exception_type"] = error.GetType().Name,
                ["native_error_code"] = error.HResult,
                ["elapsed_ms"] = System.Diagnostics.Stopwatch.GetElapsedTime(started).TotalMilliseconds,
            });
            return EFail;
        }
        finally
        {
            Retire();
        }
    }

    /// <summary>Open/Test accept exactly one item, matching DesktopArchiveIntent validation.</summary>
    internal static bool VerbAccepts(ShellVerb verb, int count) =>
        verb is ShellVerb.Open or ShellVerb.Test ? count == 1 : count >= 1;

    /// <summary>Releases the retained selection exactly once and signals server lifetime.</summary>
    internal void Retire()
    {
        if (Interlocked.Exchange(ref _retired, 1) != 0) return;
        var selection = Interlocked.Exchange(ref _selection, null);
        if (selection is not null && Marshal.IsComObject(selection)) Marshal.ReleaseComObject(selection);
        try { _released(); } catch (Exception) { /* lifetime signal cannot change the terminal */ }
    }

    private int Finish(ForwardResult result, long started, int hresult, int count = 0)
    {
        Result = result;
        Write("shell_selection.terminal", new Dictionary<string, object?>
        {
            ["verb"] = _verb.ToString(),
            ["outcome"] = result.Status.ToString(),
            ["count"] = count,
            ["exit_code"] = result.ExitCode,
            ["elapsed_ms"] = System.Diagnostics.Stopwatch.GetElapsedTime(started).TotalMilliseconds,
        });
        return hresult;
    }

    private void Write(string eventName, IReadOnlyDictionary<string, object?> fields)
    {
        try { _log.Write(eventName, CorrelationId, fields); }
        catch (Exception) { /* diagnostics never change the result */ }
    }
}

/// <summary>
/// Class factory for one verb CLSID. Every CreateInstance produces a fresh command object.
/// Server lifetime counts live objects and LockServer locks; the host exits when both reach zero.
/// </summary>
[ComVisible(true), ClassInterface(ClassInterfaceType.None)]
internal sealed class ArchiveSelectionClassFactory : IClassFactory
{
    private static readonly Guid IidIUnknown = new("00000000-0000-0000-C000-000000000046");
    private static readonly Guid IidIExecuteCommand = typeof(IExecuteCommand).GUID;
    private static readonly Guid IidIObjectWithSelection = typeof(IObjectWithSelection).GUID;
    internal const int ClassENoAggregation = unchecked((int)0x80040110);
    private readonly ShellVerb _verb;
    private readonly ISelectionForwarder _forwarder;
    private readonly IHelperLog _log;
    private readonly ServerLifetime _lifetime;

    internal ArchiveSelectionClassFactory(ShellVerb verb, ISelectionForwarder forwarder, IHelperLog log, ServerLifetime lifetime)
    {
        _verb = verb;
        _forwarder = forwarder;
        _log = log;
        _lifetime = lifetime;
    }

    public int CreateInstance(IntPtr outer, ref Guid iid, out IntPtr result)
    {
        result = IntPtr.Zero;
        if (outer != IntPtr.Zero) return ClassENoAggregation;
        if (iid != IidIUnknown && iid != IidIExecuteCommand && iid != IidIObjectWithSelection)
            return ArchiveSelectionCommand.ENoInterface;
        if (!_lifetime.TryAddObject()) return ArchiveSelectionCommand.EFail;
        var command = new ArchiveSelectionCommand(_verb, _forwarder, _log, _lifetime.Cancellation, _lifetime.ReleaseObject);
        // Explicit identity: the final IUnknown::Release (including this creation reference when QI
        // fails) retires the command synchronously; see SelectionCommandIdentity.
        var unknown = SelectionCommandIdentity.Create(command);
        try
        {
            return Marshal.QueryInterface(unknown, in iid, out result);
        }
        finally
        {
            Marshal.Release(unknown);
        }
    }

    public int LockServer(bool lockServer)
    {
        if (lockServer) _lifetime.Lock(); else _lifetime.Unlock();
        return ArchiveSelectionCommand.SOk;
    }
}

/// <summary>Bounded object/lock accounting; Shutdown cancels new work and outstanding forwards.</summary>
internal sealed class ServerLifetime : IDisposable
{
    internal const int MaxLiveObjects = 8;
    /// <summary>Explicit hard server bound (e.g. a client that never releases an object or lock).</summary>
    internal static readonly TimeSpan MaxServerLifetime = TimeSpan.FromMinutes(2);
    private readonly CancellationTokenSource _cancellation = new();
    private readonly ManualResetEventSlim _idle = new(false);
    private int _objects;
    private int _locks;
    private int _shutdown;

    internal CancellationToken Cancellation => _cancellation.Token;
    internal int LiveObjects => Volatile.Read(ref _objects);
    internal int ServerLocks => Volatile.Read(ref _locks);
    internal WaitHandle Idle => _idle.WaitHandle;

    internal bool TryAddObject()
    {
        if (Volatile.Read(ref _shutdown) != 0) return false;
        if (Interlocked.Increment(ref _objects) > MaxLiveObjects)
        {
            ReleaseObject();
            return false;
        }
        _idle.Reset();
        return true;
    }

    internal void ReleaseObject()
    {
        if (Interlocked.Decrement(ref _objects) <= 0 && Volatile.Read(ref _locks) <= 0) _idle.Set();
    }

    internal void Lock()
    {
        Interlocked.Increment(ref _locks);
        _idle.Reset();
    }

    internal void Unlock()
    {
        if (Interlocked.Decrement(ref _locks) <= 0 && Volatile.Read(ref _objects) <= 0) _idle.Set();
    }

    internal void Shutdown()
    {
        if (Interlocked.Exchange(ref _shutdown, 1) != 0) return;
        _cancellation.Cancel();
    }

    public void Dispose()
    {
        _cancellation.Dispose();
        _idle.Dispose();
    }
}

/// <summary>
/// F-AC01: explicit COM identity for one <see cref="ArchiveSelectionCommand"/> exposing exactly
/// IUnknown/IExecuteCommand (block slot 0) and IObjectWithSelection (slot 1) in SDK vtable order.
/// The built-in CCW gives no notification on final IUnknown::Release, and GC/finalizer-based
/// retirement would be nondeterministic and run off the STA. Here the reference count is owned
/// explicitly, so the final Release retires the command (selection + lifetime slot) synchronously
/// on the calling thread, exactly once. An executed command already retired; Retire is idempotent.
/// </summary>
internal static class SelectionCommandIdentity
{
    private static readonly Guid IidIUnknown = new("00000000-0000-0000-C000-000000000046");
    private static readonly Guid IidIExecuteCommand = typeof(IExecuteCommand).GUID;
    private static readonly Guid IidIObjectWithSelection = typeof(IObjectWithSelection).GUID;
    private static readonly int SlotSize = IntPtr.Size;

    [UnmanagedFunctionPointer(CallingConvention.StdCall)] private delegate int QueryInterfaceFn(IntPtr self, ref Guid iid, out IntPtr result);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)] private delegate uint CountFn(IntPtr self);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)] private delegate int UIntFn(IntPtr self, uint value);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)] private delegate int IntFn(IntPtr self, int value);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)] private delegate int PointerFn(IntPtr self, IntPtr value);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)] private delegate int PointFn(IntPtr self, NativePoint value);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)] private delegate int NoArgFn(IntPtr self);

    private sealed class Owner(ArchiveSelectionCommand command)
    {
        internal readonly ArchiveSelectionCommand Command = command;
        internal int References = 1;
    }

    // Delegates stay rooted for the process lifetime; the two tables are never freed.
    internal static readonly Delegate[] Rooted;
    private static readonly IntPtr ExecuteTable;
    private static readonly IntPtr SelectionTable;

    static SelectionCommandIdentity()
    {
        var execute = new Delegate[]
        {
            new QueryInterfaceFn((IntPtr self, ref Guid iid, out IntPtr result) => QueryInterface(self, ref iid, out result)),
            new CountFn(self => AddRef(self)),
            new CountFn(self => Release(self)),
            new UIntFn((self, value) => Call(self, command => command.SetKeyState(value))),
            new PointerFn((self, value) => Call(self, command => command.SetParameters(Marshal.PtrToStringUni(value)))),
            new PointFn((self, value) => Call(self, command => command.SetPosition(value))),
            new IntFn((self, value) => Call(self, command => command.SetShowWindow(value))),
            new IntFn((self, value) => Call(self, command => command.SetNoShowUI(value != 0))),
            new PointerFn((self, value) => Call(self, command => command.SetDirectory(Marshal.PtrToStringUni(value)))),
            new NoArgFn(self => Call(self, command => command.Execute())),
        };
        var selection = new Delegate[]
        {
            new QueryInterfaceFn((IntPtr self, ref Guid iid, out IntPtr result) => QueryInterface(self - SlotSize, ref iid, out result)),
            new CountFn(self => AddRef(self - SlotSize)),
            new CountFn(self => Release(self - SlotSize)),
            new PointerFn((self, value) => Call(self - SlotSize, command => SetSelection(command, value))),
            new GetSelectionFn((IntPtr self, ref Guid iid, out IntPtr result) => GetSelection(self - SlotSize, ref iid, out result)),
        };
        Rooted = execute.Concat(selection).ToArray();
        ExecuteTable = Table(execute);
        SelectionTable = Table(selection);
    }

    [UnmanagedFunctionPointer(CallingConvention.StdCall)] private delegate int GetSelectionFn(IntPtr self, ref Guid iid, out IntPtr result);

    /// <summary>Returns the IUnknown/IExecuteCommand pointer carrying one owned reference.</summary>
    internal static IntPtr Create(ArchiveSelectionCommand command)
    {
        var block = Marshal.AllocHGlobal(3 * SlotSize);
        Marshal.WriteIntPtr(block, 0, ExecuteTable);
        Marshal.WriteIntPtr(block, SlotSize, SelectionTable);
        Marshal.WriteIntPtr(block, 2 * SlotSize, GCHandle.ToIntPtr(GCHandle.Alloc(new Owner(command))));
        return block;
    }

    private static IntPtr Table(Delegate[] methods)
    {
        var table = Marshal.AllocHGlobal(methods.Length * SlotSize);
        for (var index = 0; index < methods.Length; index++)
            Marshal.WriteIntPtr(table, index * SlotSize, Marshal.GetFunctionPointerForDelegate(methods[index]));
        return table;
    }

    private static Owner OwnerOf(IntPtr block) =>
        (Owner)GCHandle.FromIntPtr(Marshal.ReadIntPtr(block, 2 * SlotSize)).Target!;

    private static int QueryInterface(IntPtr block, ref Guid iid, out IntPtr result)
    {
        if (iid == IidIUnknown || iid == IidIExecuteCommand) result = block;
        else if (iid == IidIObjectWithSelection) result = block + SlotSize;
        else
        {
            result = IntPtr.Zero;
            return ArchiveSelectionCommand.ENoInterface;
        }
        AddRef(block);
        return ArchiveSelectionCommand.SOk;
    }

    private static uint AddRef(IntPtr block) => (uint)Interlocked.Increment(ref OwnerOf(block).References);

    private static uint Release(IntPtr block)
    {
        var owner = OwnerOf(block);
        var remaining = Interlocked.Decrement(ref owner.References);
        if (remaining == 0)
        {
            GCHandle.FromIntPtr(Marshal.ReadIntPtr(block, 2 * SlotSize)).Free();
            Marshal.FreeHGlobal(block);
            owner.Command.Retire();
        }
        return (uint)Math.Max(remaining, 0);
    }

    private static int SetSelection(ArchiveSelectionCommand command, IntPtr array)
    {
        IShellItemArray? selection = array == IntPtr.Zero ? null : (IShellItemArray)Marshal.GetObjectForIUnknown(array);
        // The same COM object maps to one cached RCW; drop the extra marshal count instead of keeping two.
        if (selection is not null && Marshal.IsComObject(selection) && ReferenceEquals(selection, command.Selection))
            Marshal.ReleaseComObject(selection);
        var hr = command.SetSelection(selection);
        if (hr < 0 && selection is not null && Marshal.IsComObject(selection) && !ReferenceEquals(selection, command.Selection))
            Marshal.ReleaseComObject(selection);
        return hr;
    }

    private static int GetSelection(IntPtr block, ref Guid iid, out IntPtr result)
    {
        result = IntPtr.Zero;
        try { return OwnerOf(block).Command.GetSelection(ref iid, out result); }
        catch (Exception error) { return error.HResult < 0 ? error.HResult : ArchiveSelectionCommand.EFail; }
    }

    private static int Call(IntPtr block, Func<ArchiveSelectionCommand, int> method)
    {
        try { return method(OwnerOf(block).Command); }
        catch (Exception error) { return error.HResult < 0 ? error.HResult : ArchiveSelectionCommand.EFail; }
    }
}