using System.Collections.Concurrent;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal sealed record ShellActionMenuData(
    string SessionId,
    string? NavigationDirectory,
    IReadOnlyList<ShellActionItemData> Items);

internal sealed record ShellActionItemData(
    uint? CommandId,
    string Title,
    bool Enabled,
    bool Separator,
    uint? SubmenuId,
    string? IconPath);

internal sealed class ShellContextMenuSessionManager : IDisposable
{
    private readonly object _gate = new();
    private ShellContextMenuSessionWorker? _current;
    private bool _disposed;

    public WindowsCommandResult QueryActions(QueryShellActionsCommand command) =>
        QueryActions(command, CancellationToken.None);

    public WindowsCommandResult QueryActions(
        QueryShellActionsCommand command,
        CancellationToken cancellationToken)
    {
        if (cancellationToken.IsCancellationRequested)
        {
            return CancelledResult();
        }
        var target = command.Target.Trim();
        var navigationDirectory = ShellIconExtractor.ResolveNavigationDirectory(target);
        var shellTarget = File.Exists(target) || Directory.Exists(target)
            ? target
            : ShellIconExtractor.ResolveFilesystemTarget(target);
        if (shellTarget is null)
        {
            return new WindowsCommandResult(
                false,
                "Shell actions require a resolvable local file or directory.");
        }

        var worker = ShellContextMenuSessionWorker.Acquire(shellTarget);
        ShellContextMenuSessionWorker? previous;
        lock (_gate)
        {
            if (_disposed)
            {
                worker.CancelCurrent();
                return new WindowsCommandResult(false, "Shell action sessions are shutting down.");
            }
            previous = _current;
            _current = worker;
        }
        if (previous is not null && !ReferenceEquals(previous, worker))
        {
            previous.CancelCurrent();
        }

        var result = worker.QueryActions(navigationDirectory, cancellationToken);
        if (result.Success && !cancellationToken.IsCancellationRequested)
        {
            worker.Release();
            return result;
        }
        ClearCurrent(worker);
        worker.CancelCurrent();
        return cancellationToken.IsCancellationRequested ? CancelledResult() : result;
    }

    public WindowsCommandResult InvokeAction(InvokeShellActionCommand command) =>
        InvokeAction(command, CancellationToken.None);

    public WindowsCommandResult InvokeAction(
        InvokeShellActionCommand command,
        CancellationToken cancellationToken)
    {
        if (cancellationToken.IsCancellationRequested)
        {
            return CancelledResult();
        }
        if (!Guid.TryParse(command.SessionId, out var sessionId))
        {
            return new WindowsCommandResult(false, "The Shell action session id is invalid.");
        }

        ShellContextMenuSessionWorker? worker;
        lock (_gate)
        {
            worker = _current;
            if (worker is null || worker.SessionId != sessionId)
            {
                return new WindowsCommandResult(false, "The Shell action session has expired.");
            }
            _current = null;
        }

        WindowsCommandResult result;
        try
        {
            result = worker.Invoke(
                command.CommandId,
                new IntPtr(command.OwnerWindowHandle),
                cancellationToken);
            return cancellationToken.IsCancellationRequested ? CancelledResult() : result;
        }
        finally
        {
            if (cancellationToken.IsCancellationRequested)
            {
                worker.CancelCurrent();
            }
            else
            {
                worker.Release();
            }
        }
    }

    public WindowsCommandResult QuerySubmenu(QueryShellSubmenuCommand command) =>
        QuerySubmenu(command, CancellationToken.None);

    public WindowsCommandResult QuerySubmenu(
        QueryShellSubmenuCommand command,
        CancellationToken cancellationToken)
    {
        if (cancellationToken.IsCancellationRequested)
        {
            return CancelledResult();
        }
        if (!Guid.TryParse(command.SessionId, out var sessionId))
        {
            return new WindowsCommandResult(false, "The Shell action session id is invalid.");
        }

        ShellContextMenuSessionWorker? worker;
        lock (_gate)
        {
            worker = _current;
            if (worker is null || worker.SessionId != sessionId)
            {
                return new WindowsCommandResult(false, "The Shell action session has expired.");
            }
        }
        var result = worker.QuerySubmenu(command.SubmenuId, cancellationToken);
        if (cancellationToken.IsCancellationRequested)
        {
            ClearCurrent(worker);
            worker.CancelCurrent();
            return CancelledResult();
        }
        if (!result.Success)
        {
            ClearCurrent(worker);
            worker.CancelCurrent();
        }
        return result;
    }

    internal void CancelCurrent()
    {
        ShellContextMenuSessionWorker? current;
        lock (_gate)
        {
            current = _current;
            _current = null;
        }
        current?.CancelCurrent();
        ShellContextMenuSessionWorker.CancelCached();
    }

    private void ClearCurrent(ShellContextMenuSessionWorker worker)
    {
        lock (_gate)
        {
            if (ReferenceEquals(_current, worker))
            {
                _current = null;
            }
        }
    }

    public void Dispose()
    {
        ShellContextMenuSessionWorker? current;
        lock (_gate)
        {
            if (_disposed)
            {
                return;
            }
            _disposed = true;
            current = _current;
            _current = null;
        }
        current?.CancelCurrent();
        ShellContextMenuSessionWorker.CancelCached();
    }

    private static WindowsCommandResult CancelledResult() =>
        new(false, "Shell action operation was cancelled.");
}

internal sealed class ShellContextMenuSessionWorker : IDisposable
{
    private const int RequestTimeoutMilliseconds = 5000;
    private const int DisposeJoinMilliseconds = 250;
    // A warm session for the same target answers a repeat right-click without
    // re-running the Shell COM handshake (SHParseDisplayName → GetUIObjectOf →
    // QueryContextMenu), which measured 160–534 ms per cold query and is what
    // the user perceives as the action menu "loading for seconds".
    private const int CacheCapacity = 8;
    private static readonly object CacheGate = new();
    private static readonly Queue<(string Target, ShellContextMenuSessionWorker Worker)> Cache = new();

    private readonly BlockingCollection<Action> _requests = new();
    private readonly ManualResetEventSlim _started = new(false);
    private readonly CancellationTokenSource _retirement = new();
    private readonly Thread _thread;
    private readonly string _target;
    private readonly Action<uint, IntPtr, CancellationToken>? _invokeAction;
    private Exception? _startupError;
    private ShellContextMenuSession? _session;
    private int _disposed;
    private int _leased = 1;

    public static ShellContextMenuSessionWorker Acquire(string target)
    {
        ShellContextMenuSessionWorker? selected = null;
        List<ShellContextMenuSessionWorker>? stale = null;
        lock (CacheGate)
        {
            var count = Cache.Count;
            for (var index = 0; index < count; index++)
            {
                var (cachedTarget, worker) = Cache.Dequeue();
                if (Volatile.Read(ref worker._disposed) != 0)
                {
                    stale ??= new List<ShellContextMenuSessionWorker>();
                    stale.Add(worker);
                    continue;
                }
                if (selected is null &&
                    string.Equals(cachedTarget, target, StringComparison.OrdinalIgnoreCase) &&
                    worker.TryAcquireFromCache())
                {
                    selected = worker;
                    continue;
                }
                Cache.Enqueue((cachedTarget, worker));
            }
        }
        if (stale is not null)
        {
            foreach (var worker in stale)
            {
                worker.Dispose();
            }
        }
        return selected ?? new ShellContextMenuSessionWorker(target);
    }

    internal static void CancelCached()
    {
        ShellContextMenuSessionWorker[] cached;
        lock (CacheGate)
        {
            cached = Cache.Select(entry => entry.Worker).ToArray();
            Cache.Clear();
        }
        foreach (var worker in cached)
        {
            worker.CancelCurrent();
        }
    }

    private bool TryAcquireFromCache()
    {
        if (Volatile.Read(ref _disposed) != 0 ||
            Interlocked.Exchange(ref _leased, 1) != 0)
        {
            return false;
        }
        SessionId = Guid.NewGuid();
        return true;
    }

    public void Release()
    {
        if (Volatile.Read(ref _disposed) != 0)
        {
            Dispose();
            return;
        }
        if (Interlocked.Exchange(ref _leased, 0) == 0)
        {
            return;
        }
        List<ShellContextMenuSessionWorker>? evicted = null;
        lock (CacheGate)
        {
            if (Volatile.Read(ref _disposed) != 0)
            {
                evicted = [this];
            }
            else
            {
                Cache.Enqueue((_target, this));
                while (Cache.Count > CacheCapacity)
                {
                    var (_, evictedWorker) = Cache.Dequeue();
                    evicted ??= new List<ShellContextMenuSessionWorker>();
                    evicted.Add(evictedWorker);
                }
            }
        }
        if (evicted is not null)
        {
            foreach (var worker in evicted)
            {
                worker.Dispose();
            }
        }
    }

    public ShellContextMenuSessionWorker(string target)
        : this(target, null)
    {
    }

    internal ShellContextMenuSessionWorker(
        string target,
        Action<uint, IntPtr, CancellationToken>? invokeAction)
    {
        _target = target;
        _invokeAction = invokeAction;
        SessionId = Guid.NewGuid();
        _thread = new Thread(Run)
        {
            IsBackground = true,
            Name = "NxTools.ShellContextMenu"
        };
        _thread.SetApartmentState(ApartmentState.STA);
        _thread.Start();
    }

    public Guid SessionId { get; private set; }

    public WindowsCommandResult QueryActions(string? navigationDirectory) =>
        QueryActions(navigationDirectory, CancellationToken.None);

    public WindowsCommandResult QueryActions(
        string? navigationDirectory,
        CancellationToken cancellationToken)
    {
        if (!TryInvoke(
                () =>
                {
                    cancellationToken.ThrowIfCancellationRequested();
                    _session = ShellContextMenuSession.Open(_target);
                    return _session.Enumerate(cancellationToken);
                },
                cancellationToken,
                out IReadOnlyList<ShellActionItemData>? items,
                out var error))
        {
            if (navigationDirectory is not null && !cancellationToken.IsCancellationRequested)
            {
                return SerializeMenu([], navigationDirectory);
            }
            return new WindowsCommandResult(
                false,
                cancellationToken.IsCancellationRequested
                    ? "Shell action operation was cancelled."
                    : error);
        }

        return cancellationToken.IsCancellationRequested
            ? new WindowsCommandResult(false, "Shell action operation was cancelled.")
            : SerializeMenu(items!, navigationDirectory);
    }

    public WindowsCommandResult QuerySubmenu(uint submenuId) =>
        QuerySubmenu(submenuId, CancellationToken.None);

    public WindowsCommandResult QuerySubmenu(
        uint submenuId,
        CancellationToken cancellationToken)
    {
        if (!TryInvoke(
                () =>
                {
                    if (_session is null)
                    {
                        throw new InvalidOperationException("The Shell action session is not initialized.");
                    }
                    return _session.EnumerateSubmenu(submenuId, cancellationToken);
                },
                cancellationToken,
                out IReadOnlyList<ShellActionItemData>? items,
                out var error))
        {
            return new WindowsCommandResult(
                false,
                cancellationToken.IsCancellationRequested
                    ? "Shell action operation was cancelled."
                    : error);
        }
        return cancellationToken.IsCancellationRequested
            ? new WindowsCommandResult(false, "Shell action operation was cancelled.")
            : SerializeMenu(items!, null);
    }

    private WindowsCommandResult SerializeMenu(
        IReadOnlyList<ShellActionItemData> items,
        string? navigationDirectory)
    {
        var menu = new ShellActionMenuData(
            SessionId.ToString("D"),
            navigationDirectory,
            items);
        return new WindowsCommandResult(true, Data: JsonSerializer.Serialize(menu, JsonDefaults.Options));
    }

    public WindowsCommandResult Invoke(uint commandId, IntPtr owner) =>
        Invoke(commandId, owner, CancellationToken.None);

    public WindowsCommandResult Invoke(
        uint commandId,
        IntPtr owner,
        CancellationToken cancellationToken)
    {
        if (!TryInvokeHumanAction(
                () =>
                {
                    if (_invokeAction is not null)
                    {
                        _invokeAction(commandId, owner, cancellationToken);
                    }
                    else
                    {
                        if (_session is null)
                        {
                            throw new InvalidOperationException("The Shell action session is not initialized.");
                        }
                        _session.Invoke(commandId, owner, cancellationToken);
                    }
                    return true;
                },
                cancellationToken,
                out _,
                out var error))
        {
            return new WindowsCommandResult(
                false,
                cancellationToken.IsCancellationRequested
                    ? "Shell action operation was cancelled."
                    : error);
        }
        return cancellationToken.IsCancellationRequested
            ? new WindowsCommandResult(false, "Shell action operation was cancelled.")
            : new WindowsCommandResult(true);
    }

    private bool TryInvoke<T>(
        Func<T> operation,
        CancellationToken cancellationToken,
        out T? result,
        out string? error) =>
        TryInvokeCore(
            operation,
            cancellationToken,
            RequestTimeoutMilliseconds,
            out result,
            out error);

    private bool TryInvokeHumanAction<T>(
        Func<T> operation,
        CancellationToken cancellationToken,
        out T? result,
        out string? error) =>
        TryInvokeCore(
            operation,
            cancellationToken,
            Timeout.Infinite,
            out result,
            out error);

    private bool TryInvokeCore<T>(
        Func<T> operation,
        CancellationToken cancellationToken,
        int completionTimeoutMilliseconds,
        out T? result,
        out string? error)
    {
        result = default;
        error = null;
        if (cancellationToken.IsCancellationRequested)
        {
            Dispose();
            error = "Shell action operation was cancelled.";
            return false;
        }
        if (_retirement.IsCancellationRequested)
        {
            error = "The Shell action session has been disposed.";
            return false;
        }
        using var waitCancellation = CancellationTokenSource.CreateLinkedTokenSource(
            cancellationToken,
            _retirement.Token);
        try
        {
            _started.Wait(RequestTimeoutMilliseconds, waitCancellation.Token);
        }
        catch (OperationCanceledException)
        {
            if (cancellationToken.IsCancellationRequested)
            {
                Dispose();
                error = "Shell action operation was cancelled.";
            }
            else
            {
                error = "The Shell action session has been disposed.";
            }
            return false;
        }
        if (!_started.IsSet)
        {
            error = "The Shell action STA worker did not start in time.";
            return false;
        }
        if (_startupError is not null)
        {
            error = _startupError.Message;
            return false;
        }
        if (Volatile.Read(ref _disposed) != 0)
        {
            error = "The Shell action session has been disposed.";
            return false;
        }

        var completion = new TaskCompletionSource<T>(
            TaskCreationOptions.RunContinuationsAsynchronously);
        _ = completion.Task.ContinueWith(
            completedTask => _ = completedTask.Exception,
            CancellationToken.None,
            TaskContinuationOptions.OnlyOnFaulted | TaskContinuationOptions.ExecuteSynchronously,
            TaskScheduler.Default);
        try
        {
            _requests.Add(() =>
            {
                if (_retirement.IsCancellationRequested ||
                    Volatile.Read(ref _disposed) != 0 ||
                    cancellationToken.IsCancellationRequested)
                {
                    return;
                }
                try
                {
                    completion.TrySetResult(operation());
                }
                catch (Exception exception)
                {
                    completion.TrySetException(exception);
                }
            });
        }
        catch (InvalidOperationException)
        {
            error = "The Shell action session is no longer accepting work.";
            return false;
        }

        bool completed;
        try
        {
            completed = completion.Task.Wait(completionTimeoutMilliseconds, waitCancellation.Token);
        }
        catch (OperationCanceledException)
        {
            if (cancellationToken.IsCancellationRequested)
            {
                Dispose();
                error = "Shell action operation was cancelled.";
            }
            else
            {
                error = "The Shell action session has been disposed.";
            }
            return false;
        }
        if (!completed)
        {
            Dispose();
            error = "A Windows Shell extension did not respond within five seconds.";
            return false;
        }
        if (cancellationToken.IsCancellationRequested)
        {
            Dispose();
            error = "Shell action operation was cancelled.";
            return false;
        }
        try
        {
            result = completion.Task.GetAwaiter().GetResult();
            return true;
        }
        catch (Exception exception)
        {
            error = exception.GetBaseException().Message;
            return false;
        }
    }

    private void Run()
    {
        var initialized = false;
        try
        {
            var result = ShellContextMenuNativeMethods.OleInitialize(IntPtr.Zero);
            if (result < 0)
            {
                Marshal.ThrowExceptionForHR(result);
            }
            initialized = true;
            _started.Set();
            foreach (var request in _requests.GetConsumingEnumerable())
            {
                request();
            }
        }
        catch (Exception exception)
        {
            _startupError = exception;
            _started.Set();
        }
        finally
        {
            try
            {
                _session?.Dispose();
            }
            finally
            {
                if (!BoundedStaOperationRunner.TryShutdownCurrentThreadDispatcher(
                        out var dispatcherError) &&
                    _startupError is null)
                {
                    _startupError = new InvalidOperationException(
                        $"Shell action Dispatcher cleanup failed: {dispatcherError}");
                }
                if (initialized)
                {
                    ShellContextMenuNativeMethods.OleUninitialize();
                }
            }
        }
    }

    internal void CancelCurrent() => DisposeCore(waitForThread: false);

    public void Dispose() => DisposeCore(waitForThread: true);

    private void DisposeCore(bool waitForThread)
    {
        if (Interlocked.Exchange(ref _disposed, 1) != 0)
        {
            return;
        }
        _retirement.Cancel();
        lock (CacheGate)
        {
            if (Cache.Count != 0)
            {
                var retained = new Queue<(string Target, ShellContextMenuSessionWorker Worker)>();
                while (Cache.Count != 0)
                {
                    var entry = Cache.Dequeue();
                    if (!ReferenceEquals(entry.Worker, this))
                    {
                        retained.Enqueue(entry);
                    }
                }
                while (retained.Count != 0)
                {
                    Cache.Enqueue(retained.Dequeue());
                }
            }
        }
        _requests.CompleteAdding();
        if (waitForThread && Thread.CurrentThread != _thread)
        {
            try
            {
                _ = _thread.Join(DisposeJoinMilliseconds);
            }
            catch
            {
                // A late STA worker remains background-only and is not cached.
            }
        }
    }
}

internal sealed class ShellContextMenuSession : IDisposable
{
    private const uint FirstCommand = 1;
    private const uint LastCommand = 30000;
    private const uint QueryFlags = 0x00000004 | 0x00000100 | 0x00001000;
    private const int MaximumItems = 192;
    private const int MaximumSubmenus = 512;
    private const int MaximumTitleCharacters = 512;

    private IntPtr _item;
    private IntPtr _parentFolder;
    private IntPtr _contextMenuPointer;
    private IntPtr _menu;
    private ShellContextMenuNativeMethods.IContextMenu? _contextMenu;
    private readonly Dictionary<uint, ShellSubmenuReference> _submenus = new();
    private uint _nextSubmenuId = 1;
    private bool _disposed;

    private ShellContextMenuSession()
    {
    }

    public static ShellContextMenuSession Open(string target)
    {
        var session = new ShellContextMenuSession();
        try
        {
            var result = ShellContextMenuNativeMethods.SHParseDisplayName(
                target,
                IntPtr.Zero,
                out session._item,
                0,
                out _);
            ThrowIfFailed(result);

            var shellFolderId = new Guid("000214E6-0000-0000-C000-000000000046");
            var relativeItem = IntPtr.Zero;
            result = ShellContextMenuNativeMethods.SHBindToParent(
                session._item,
                ref shellFolderId,
                out session._parentFolder,
                ref relativeItem);
            ThrowIfFailed(result);

            var parent = (ShellContextMenuNativeMethods.IShellFolder)
                Marshal.GetObjectForIUnknown(session._parentFolder);
            try
            {
                var contextMenuId = new Guid("000214E4-0000-0000-C000-000000000046");
                uint reserved = 0;
                parent.GetUIObjectOf(
                    IntPtr.Zero,
                    1,
                    [relativeItem],
                    ref contextMenuId,
                    ref reserved,
                    out session._contextMenuPointer);
            }
            finally
            {
                Marshal.FinalReleaseComObject(parent);
            }

            session._contextMenu = (ShellContextMenuNativeMethods.IContextMenu)
                Marshal.GetObjectForIUnknown(session._contextMenuPointer);
            session._menu = ShellContextMenuNativeMethods.CreatePopupMenu();
            if (session._menu == IntPtr.Zero)
            {
                throw new InvalidOperationException("Windows could not create a Shell action menu.");
            }
            result = session._contextMenu.QueryContextMenu(
                session._menu,
                0,
                FirstCommand,
                LastCommand,
                QueryFlags);
            ThrowIfFailed(result);
            return session;
        }
        catch
        {
            session.Dispose();
            throw;
        }
    }

    public IReadOnlyList<ShellActionItemData> Enumerate() =>
        Enumerate(CancellationToken.None);

    public IReadOnlyList<ShellActionItemData> Enumerate(CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        return EnumerateMenu(_menu, 0, cancellationToken);
    }

    public IReadOnlyList<ShellActionItemData> EnumerateSubmenu(uint submenuId) =>
        EnumerateSubmenu(submenuId, CancellationToken.None);

    public IReadOnlyList<ShellActionItemData> EnumerateSubmenu(
        uint submenuId,
        CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        if (!_submenus.TryGetValue(submenuId, out var submenu))
        {
            throw new InvalidOperationException("The selected Shell submenu is unavailable.");
        }
        return EnumerateMenu(submenu.Menu, submenu.ParentIndex, cancellationToken);
    }

    private List<ShellActionItemData> EnumerateMenu(
        IntPtr menu,
        uint menuIndex,
        CancellationToken cancellationToken)
    {
        if (menu == IntPtr.Zero)
        {
            return [];
        }
        cancellationToken.ThrowIfCancellationRequested();
        InitializeMenu(menu, menuIndex);
        var count = ShellContextMenuNativeMethods.GetMenuItemCount(menu);
        if (count <= 0)
        {
            return [];
        }

        var items = new List<ShellActionItemData>();
        var titleBuffer = Marshal.AllocHGlobal(MaximumTitleCharacters * sizeof(char));
        try
        {
            for (uint index = 0; index < count && items.Count < MaximumItems; index++)
            {
                cancellationToken.ThrowIfCancellationRequested();
                Marshal.WriteInt16(titleBuffer, 0);
                var info = new ShellContextMenuNativeMethods.MenuItemInfo
                {
                    Size = (uint)Marshal.SizeOf<ShellContextMenuNativeMethods.MenuItemInfo>(),
                    Mask = ShellContextMenuNativeMethods.MenuItemType |
                           ShellContextMenuNativeMethods.MenuItemState |
                           ShellContextMenuNativeMethods.MenuItemId |
                           ShellContextMenuNativeMethods.MenuItemSubMenu |
                           ShellContextMenuNativeMethods.MenuItemString |
                           ShellContextMenuNativeMethods.MenuItemBitmap,
                    Text = titleBuffer,
                    TextLength = MaximumTitleCharacters - 1
                };
                if (!ShellContextMenuNativeMethods.GetMenuItemInfo(menu, index, true, ref info))
                {
                    continue;
                }

                if ((info.Type & ShellContextMenuNativeMethods.MenuTypeSeparator) != 0)
                {
                    items.Add(new ShellActionItemData(
                        null, string.Empty, false, true, null, null));
                    continue;
                }

                var title = NormalizeTitle(Marshal.PtrToStringUni(titleBuffer));
                if (title.Length == 0)
                {
                    continue;
                }
                var enabled = (info.State & ShellContextMenuNativeMethods.MenuStateDisabled) == 0;
                var submenuId = RegisterSubmenu(info.SubMenu, index);
                uint? commandId = submenuId is null &&
                                  info.CommandId is >= FirstCommand and <= LastCommand
                    ? info.CommandId
                    : null;
                if (commandId is null && submenuId is null)
                {
                    enabled = false;
                }
                items.Add(new ShellActionItemData(
                    commandId,
                    title,
                    enabled,
                    false,
                    submenuId,
                    ShellMenuIconStager.Stage(info.ItemBitmap, cancellationToken)));
            }
        }
        finally
        {
            Marshal.FreeHGlobal(titleBuffer);
        }
        return items;
    }

    private uint? RegisterSubmenu(IntPtr menu, uint parentIndex)
    {
        if (menu == IntPtr.Zero || _submenus.Count >= MaximumSubmenus)
        {
            return null;
        }
        var submenuId = _nextSubmenuId++;
        _submenus[submenuId] = new ShellSubmenuReference(menu, parentIndex);
        return submenuId;
    }

    private void InitializeMenu(IntPtr menu, uint index)
    {
        if (_contextMenu is ShellContextMenuNativeMethods.IContextMenu3 contextMenu3)
        {
            _ = contextMenu3.HandleMenuMessage2(
                ShellContextMenuNativeMethods.InitializeMenuPopup,
                menu,
                new IntPtr(index),
                out _);
        }
        else if (_contextMenu is ShellContextMenuNativeMethods.IContextMenu2 contextMenu2)
        {
            _ = contextMenu2.HandleMenuMessage(
                ShellContextMenuNativeMethods.InitializeMenuPopup,
                menu,
                new IntPtr(index));
        }
    }

    public void Invoke(uint commandId, IntPtr owner) =>
        Invoke(commandId, owner, CancellationToken.None);

    public void Invoke(
        uint commandId,
        IntPtr owner,
        CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        if (_contextMenu is null || commandId is < FirstCommand or > LastCommand)
        {
            throw new InvalidOperationException("The selected Shell command is invalid.");
        }
        var command = new ShellContextMenuNativeMethods.CommandInfo
        {
            Size = Marshal.SizeOf<ShellContextMenuNativeMethods.CommandInfo>(),
            Owner = owner,
            Verb = new IntPtr(commandId - FirstCommand),
            Show = 1
        };
        ThrowIfFailed(_contextMenu.InvokeCommand(ref command));
        cancellationToken.ThrowIfCancellationRequested();
    }

    private static string NormalizeTitle(string? title)
    {
        var value = title?.Split('\t', 2)[0].Trim() ?? string.Empty;
        const string escapedAmpersand = "\u0001";
        return value
            .Replace("&&", escapedAmpersand, StringComparison.Ordinal)
            .Replace("&", string.Empty, StringComparison.Ordinal)
            .Replace(escapedAmpersand, "&", StringComparison.Ordinal);
    }

    private static void ThrowIfFailed(int result)
    {
        if (result < 0)
        {
            Marshal.ThrowExceptionForHR(result);
        }
    }

    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }
        _disposed = true;
        if (_menu != IntPtr.Zero)
        {
            _ = ShellContextMenuNativeMethods.DestroyMenu(_menu);
            _menu = IntPtr.Zero;
        }
        if (_contextMenu is not null && Marshal.IsComObject(_contextMenu))
        {
            Marshal.FinalReleaseComObject(_contextMenu);
            _contextMenu = null;
        }
        if (_contextMenuPointer != IntPtr.Zero)
        {
            _ = Marshal.Release(_contextMenuPointer);
            _contextMenuPointer = IntPtr.Zero;
        }
        if (_parentFolder != IntPtr.Zero)
        {
            _ = Marshal.Release(_parentFolder);
            _parentFolder = IntPtr.Zero;
        }
        if (_item != IntPtr.Zero)
        {
            ShellContextMenuNativeMethods.CoTaskMemFree(_item);
            _item = IntPtr.Zero;
        }
    }

    private sealed record ShellSubmenuReference(IntPtr Menu, uint ParentIndex);
}

internal static class ShellMenuIconStager
{
    private const int MaximumCachedIcons = 2048;
    private static int _stageCount;

    public static string? Stage(IntPtr bitmapHandle) =>
        Stage(bitmapHandle, CancellationToken.None);

    public static string? Stage(
        IntPtr bitmapHandle,
        CancellationToken cancellationToken)
    {
        // HBMMENU_* pseudo handles occupy the small integer range and are not GDI bitmaps.
        if (bitmapHandle == IntPtr.Zero || bitmapHandle.ToInt64() is >= -1 and <= 12)
        {
            return null;
        }
        cancellationToken.ThrowIfCancellationRequested();
        try
        {
            var bitmap = System.Windows.Interop.Imaging.CreateBitmapSourceFromHBitmap(
                bitmapHandle,
                IntPtr.Zero,
                System.Windows.Int32Rect.Empty,
                System.Windows.Media.Imaging.BitmapSizeOptions.FromEmptyOptions());
            bitmap.Freeze();
            using var encoded = new MemoryStream();
            var encoder = new System.Windows.Media.Imaging.PngBitmapEncoder();
            encoder.Frames.Add(System.Windows.Media.Imaging.BitmapFrame.Create(bitmap));
            encoder.Save(encoded);
            cancellationToken.ThrowIfCancellationRequested();
            var bytes = encoded.ToArray();
            var fingerprint = Convert.ToHexString(SHA256.HashData(bytes)).ToLowerInvariant();
            var directory = Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
                "NxTools", "NxTools.Next", "staging", "quick-launch-action-icons");
            Directory.CreateDirectory(directory);
            var destination = Path.Combine(directory, $"{fingerprint}.png");
            if (!File.Exists(destination))
            {
                File.WriteAllBytes(destination, bytes);
            }
            if ((Interlocked.Increment(ref _stageCount) & 63) == 1)
            {
                TrimCache(directory);
            }
            return destination;
        }
        catch (Exception error) when (error is ArgumentException or ExternalException or IOException)
        {
            return null;
        }
    }

    private static void TrimCache(string directory)
    {
        try
        {
            foreach (var stale in new DirectoryInfo(directory)
                         .EnumerateFiles("*.png")
                         .OrderByDescending(file => file.LastWriteTimeUtc)
                         .Skip(MaximumCachedIcons))
            {
                stale.Delete();
            }
        }
        catch (Exception error) when (error is IOException or UnauthorizedAccessException)
        {
            // Cache cleanup is best effort and never blocks the action list.
        }
    }
}
