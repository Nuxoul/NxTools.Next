using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;

namespace NxTools.PlatformHost.Windows;

internal interface IDirectoryOpusWindowOperations
{
    bool IsListerWindow(long windowHandle);

    bool TryCaptureActivePane(
        long windowHandle,
        long focusWindowHandle,
        out string? currentDirectory,
        out DirectoryOpusPaneState? paneState);

    bool IsPaneAvailable(
        long windowHandle,
        DirectoryOpusPaneState paneState);

    bool TryNavigate(
        long windowHandle,
        string targetDirectory,
        DirectoryOpusPaneState paneState,
        out string? error);

    bool TryActivate(
        long windowHandle,
        DirectoryOpusPaneState paneState,
        out string? error);
}

internal sealed record DirectoryOpusPaneState(
    long ContainerWindowHandle,
    long FocusWindowHandle);

internal readonly record struct DirectoryOpusPaneCandidate(
    long WindowHandle,
    string? CurrentDirectory,
    int Left,
    int Top,
    bool ContainsFocus);

internal readonly record struct DirectoryOpusPanePoint(int Left, int Top);

internal static class DirectoryOpusPaneSelector
{
    public static long Select(
        string listerCaption,
        IReadOnlyList<DirectoryOpusPaneCandidate> candidates,
        DirectoryOpusPanePoint? focusedPoint,
        long rememberedWindowHandle)
    {
        var ordered = candidates
            .OrderBy(candidate => candidate.Left)
            .ThenBy(candidate => candidate.Top)
            .ToArray();
        if (ordered.Length == 0)
        {
            return 0;
        }
        if (ordered.Length == 1)
        {
            return ordered[0].WindowHandle;
        }

        var titleMatches = ordered
            .Where(candidate => CaptionMatchesPath(
                listerCaption,
                candidate.CurrentDirectory))
            .ToArray();
        if (titleMatches.Length == 1)
        {
            return titleMatches[0].WindowHandle;
        }

        var focused = ordered.FirstOrDefault(candidate => candidate.ContainsFocus);
        if (focused.WindowHandle != 0)
        {
            return focused.WindowHandle;
        }

        if (focusedPoint is { } point)
        {
            var sameColumn = Math.Abs(ordered[0].Left - ordered[1].Left) <= 10;
            var nearest = ordered.MinBy(candidate => Math.Abs(
                (sameColumn ? candidate.Top : candidate.Left) -
                (sameColumn ? point.Top : point.Left)));
            if (nearest.WindowHandle != 0)
            {
                return nearest.WindowHandle;
            }
        }

        return ordered.FirstOrDefault(candidate =>
            candidate.WindowHandle == rememberedWindowHandle).WindowHandle;
    }

    private static bool CaptionMatchesPath(
        string caption,
        string? currentDirectory)
    {
        if (string.IsNullOrWhiteSpace(caption) ||
            string.IsNullOrWhiteSpace(currentDirectory))
        {
            return false;
        }
        var normalized = currentDirectory.TrimEnd(
            Path.DirectorySeparatorChar,
            Path.AltDirectorySeparatorChar);
        var directoryName = Path.GetFileName(normalized);
        return !string.IsNullOrWhiteSpace(directoryName) &&
            (caption.Contains(normalized, StringComparison.OrdinalIgnoreCase) ||
             caption.StartsWith(
                 directoryName + " ",
                 StringComparison.OrdinalIgnoreCase) ||
             caption.Equals(directoryName, StringComparison.OrdinalIgnoreCase));
    }
}

internal sealed class DirectoryOpusFileManagerOperations :
    IThirdPartyFileManagerOperations
{
    private readonly IDirectoryOpusWindowOperations _windows;

    public DirectoryOpusFileManagerOperations()
        : this(new Win32DirectoryOpusWindowOperations())
    {
    }

    internal DirectoryOpusFileManagerOperations(
        IDirectoryOpusWindowOperations windows)
    {
        _windows = windows;
    }

    public string Id => "Directory Opus";
    public bool RequiresPostNavigationActivation => false;

    public bool TryCapture(
        ForegroundContext foreground,
        out ThirdPartyFileManagerCapture? capture,
        out WindowsCommandResult? failure)
    {
        capture = null;
        failure = null;
        if (!foreground.ProcessName.Equals(
                "dopus",
                StringComparison.OrdinalIgnoreCase) &&
            !foreground.ProcessName.Equals(
                "dopus.exe",
                StringComparison.OrdinalIgnoreCase) ||
            !_windows.IsListerWindow(foreground.WindowHandle) ||
            !_windows.TryCaptureActivePane(
                foreground.WindowHandle,
                foreground.FocusWindowHandle,
                out var capturedDirectory,
                out var paneState) ||
            !ThirdPartyFileManagerPath.LooksLikeRootedDirectory(
                capturedDirectory) ||
            paneState is null)
        {
            return false;
        }
        var capturedPane = paneState!;
        if (capturedPane.FocusWindowHandle == foreground.WindowHandle)
        {
            capturedPane = capturedPane with { FocusWindowHandle = 0 };
        }
        capture = new ThirdPartyFileManagerCapture(
            capturedDirectory!,
            capturedPane);
        return true;
    }

    public bool CanNavigate(long windowHandle, object paneState) =>
        paneState is DirectoryOpusPaneState directoryOpusPane &&
        _windows.IsPaneAvailable(windowHandle, directoryOpusPane);

    public WindowsCommandResult Navigate(
        long windowHandle,
        string targetDirectory,
        object paneState)
    {
        if (paneState is not DirectoryOpusPaneState directoryOpusPane)
        {
            return new WindowsCommandResult(
                false,
                "The captured Directory Opus pane context is invalid.");
        }
        return _windows.TryNavigate(
                windowHandle,
                targetDirectory,
                directoryOpusPane,
                out var error)
            ? new WindowsCommandResult(true)
            : new WindowsCommandResult(
                false,
                error ??
                "The Directory Opus window could not be navigated.");
    }

    public bool TryActivate(
        long windowHandle,
        object paneState,
        out string? error) =>
        paneState is DirectoryOpusPaneState directoryOpusPane
            ? _windows.TryActivate(
                windowHandle,
                directoryOpusPane,
                out error)
            : FailInvalidPane(out error);

    private static bool FailInvalidPane(out string? error)
    {
        error = "The captured Directory Opus pane context is invalid.";
        return false;
    }
}

internal sealed class Win32DirectoryOpusWindowOperations :
    IDirectoryOpusWindowOperations
{
    private const uint WmCopyData = 0x004A;
    private const uint SmtoAbortIfHung = 0x0002;
    private const uint CommandTimeoutMs = 1_000;
    private const int CommandDataId = 0x14;

    public bool IsListerWindow(long windowHandle)
    {
        var hwnd = new IntPtr(windowHandle);
        return hwnd != IntPtr.Zero &&
            IsWindow(hwnd) &&
            GetWindowClass(hwnd).Equals(
                "dopus.lister",
                StringComparison.OrdinalIgnoreCase) &&
            WindowBelongsToProcess(hwnd, "dopus");
    }

    public bool TryCaptureActivePane(
        long windowHandle,
        long focusWindowHandle,
        out string? currentDirectory,
        out DirectoryOpusPaneState? paneState)
    {
        currentDirectory = null;
        paneState = null;
        var lister = new IntPtr(windowHandle);
        if (!IsListerWindow(windowHandle))
        {
            return false;
        }
        var containers = GetVisibleFileDisplayContainers(lister);
        if (containers.Count == 0)
        {
            return false;
        }

        var focused = new IntPtr(focusWindowHandle);
        var candidates = containers
            .Select(container =>
            {
                var hasPosition = TryGetWindowPosition(
                    container,
                    out var position);
                return new DirectoryOpusPaneCandidate(
                    container.ToInt64(),
                    GetContainerPath(container),
                    hasPosition ? position.Left : 0,
                    hasPosition ? position.Top : 0,
                    focused != IntPtr.Zero &&
                    (container == focused || IsChild(container, focused)));
            })
            .ToArray();
        DirectoryOpusPanePoint? focusedPoint = null;
        if (focused != IntPtr.Zero &&
            TryGetWindowPosition(focused, out var focusedPosition))
        {
            focusedPoint = focusedPosition;
        }
        var selected = new IntPtr(DirectoryOpusPaneSelector.Select(
            GetWindowCaption(lister),
            candidates,
            focusedPoint,
            LastActivePane(lister, paneState, containers).ToInt64()));
        var candidate = candidates.FirstOrDefault(item =>
            item.WindowHandle == selected.ToInt64());
        if (selected == IntPtr.Zero ||
            !ThirdPartyFileManagerPath.LooksLikeRootedDirectory(
                candidate.CurrentDirectory))
        {
            return false;
        }
        currentDirectory = candidate.CurrentDirectory;
        RememberActivePane(new IntPtr(windowHandle), selected);
        paneState = new DirectoryOpusPaneState(
            selected.ToInt64(),
            candidate.ContainsFocus ? focusWindowHandle : 0);
        return true;
    }

    // Per-lister last-active pane. Tab switches move OS focus to the lister
    // frame; the remembered pane plus the container caption (which DOpus
    // updates to the active tab's path) keep the directory current.
    private static readonly System.Collections.Concurrent.ConcurrentDictionary<
        IntPtr,
        IntPtr> LastActivePaneByLister = new();

    private static IntPtr LastActivePane(
        IntPtr lister,
        DirectoryOpusPaneState? paneState,
        List<IntPtr> containers)
    {
        if (paneState is not null)
        {
            var remembered = new IntPtr(paneState.ContainerWindowHandle);
            if (remembered != IntPtr.Zero &&
                IsWindow(remembered) &&
                IsWindowVisible(remembered) &&
                containers.Contains(remembered))
            {
                return remembered;
            }
        }
        return LastActivePaneByLister.TryGetValue(lister, out var cached) &&
            containers.Contains(cached)
                ? cached
                : IntPtr.Zero;
    }

    private static void RememberActivePane(IntPtr lister, IntPtr container)
    {
        LastActivePaneByLister[new IntPtr(lister)] = container;
    }

    public bool IsPaneAvailable(
        long windowHandle,
        DirectoryOpusPaneState paneState)
    {
        if (!IsListerWindow(windowHandle))
        {
            return false;
        }
        var lister = new IntPtr(windowHandle);
        var container = new IntPtr(paneState.ContainerWindowHandle);
        var focus = new IntPtr(paneState.FocusWindowHandle);
        var focusAvailable = focus == IntPtr.Zero ||
            (IsWindow(focus) &&
             (container == focus || IsChild(container, focus)));
        return container != IntPtr.Zero &&
            IsWindow(container) &&
            IsWindowVisible(container) &&
            IsChild(lister, container) &&
            focusAvailable;
    }

    public bool TryNavigate(
        long windowHandle,
        string targetDirectory,
        DirectoryOpusPaneState paneState,
        out string? error)
    {
        if (!IsPaneAvailable(windowHandle, paneState))
        {
            error = "The captured Directory Opus pane is no longer available.";
            return false;
        }
        if (!TryActivate(windowHandle, paneState, out error))
        {
            return false;
        }

        var commandWindow = FindCommandWindowForLister(
            new IntPtr(windowHandle));
        if (commandWindow == IntPtr.Zero)
        {
            error = "Directory Opus did not expose its command window.";
            return false;
        }
        var bytes = Encoding.Unicode.GetBytes(
            $"Go \"{targetDirectory}\"\0");
        var pinned = GCHandle.Alloc(bytes, GCHandleType.Pinned);
        try
        {
            var data = new CopyDataStruct
            {
                DataId = new IntPtr(CommandDataId),
                ByteCount = bytes.Length,
                Data = pinned.AddrOfPinnedObject()
            };
            var sent = SendMessageTimeout(
                commandWindow,
                WmCopyData,
                IntPtr.Zero,
                ref data,
                SmtoAbortIfHung,
                CommandTimeoutMs,
                out _);
            if (sent == IntPtr.Zero)
            {
                error = "Directory Opus did not accept the directory command before timeout.";
                return false;
            }
        }
        finally
        {
            pinned.Free();
        }

        var deadline = Stopwatch.StartNew();
        while (deadline.ElapsedMilliseconds < 1_000)
        {
            var currentDirectory = GetContainerPath(
                new IntPtr(paneState.ContainerWindowHandle));
            if (ThirdPartyFileManagerPath.SameDirectory(
                    currentDirectory,
                    targetDirectory))
            {
                error = null;
                return true;
            }
            Thread.Sleep(25);
        }
        error = "Directory Opus did not switch the captured lister to the requested directory.";
        return false;
    }

    public bool TryActivate(
        long windowHandle,
        DirectoryOpusPaneState paneState,
        out string? error)
    {
        if (!IsPaneAvailable(windowHandle, paneState))
        {
            error = "The captured Directory Opus pane is no longer available.";
            return false;
        }
        if (WindowsPlatformAdapter.TryActivateNativeWindow(
                windowHandle,
                paneState.FocusWindowHandle))
        {
            error = null;
            return true;
        }
        error = "Windows did not restore the captured Directory Opus lister.";
        return false;
    }

    private static bool TryGetWindowPosition(
        IntPtr window,
        out DirectoryOpusPanePoint point)
    {
        if (!GetWindowRect(window, out var rectangle))
        {
            point = default;
            return false;
        }
        point = new DirectoryOpusPanePoint(
            rectangle.Left,
            rectangle.Top);
        return true;
    }

    private static List<IntPtr> GetVisibleFileDisplayContainers(IntPtr lister)
    {
        var containers = new List<IntPtr>();
        _ = EnumChildWindows(
            lister,
            (window, _) =>
            {
                if (IsWindowVisible(window) &&
                    GetWindowClass(window).Equals(
                        "dopus.filedisplaycontainer",
                        StringComparison.OrdinalIgnoreCase))
                {
                    containers.Add(window);
                }
                return true;
            },
            IntPtr.Zero);
        return containers;
    }

    private static string? GetContainerPath(IntPtr container)
    {
        var caption = GetWindowCaption(container);
        string? candidate;
        if (ThirdPartyFileManagerPath.LooksLikeRootedDirectory(caption))
        {
            candidate = caption;
        }
        else
        {
            var pathControl = FindDescendant(container, "dopus.ctl.treepath");
            var path = pathControl == IntPtr.Zero
                ? null
                : GetWindowCaption(pathControl);
            candidate = ThirdPartyFileManagerPath.LooksLikeRootedDirectory(path)
                ? path
                : null;
        }
        if (candidate is null)
        {
            return null;
        }
        // Directory Opus renders profile folders in their localized display
        // form (e.g. C:\用户\Administrator\桌面 on a zh-CN system); resolve
        // those segments back to the real parsing paths every downstream
        // exists-check requires.
        return ThirdPartyFileManagerDisplayPath.Canonicalize(
            candidate,
            Directory.Exists,
            ThirdPartyFileManagerDisplayPath.KnownFolderDisplayAliases());
    }

    private static IntPtr FindDescendant(IntPtr parent, string className)
    {
        var found = IntPtr.Zero;
        _ = EnumChildWindows(
            parent,
            (window, _) =>
            {
                if (!GetWindowClass(window).Equals(
                        className,
                        StringComparison.OrdinalIgnoreCase))
                {
                    return true;
                }
                found = window;
                return false;
            },
            IntPtr.Zero);
        return found;
    }

    private static IntPtr FindCommandWindowForLister(IntPtr lister)
    {
        _ = GetWindowThreadProcessId(lister, out var targetProcessId);
        if (targetProcessId == 0)
        {
            return IntPtr.Zero;
        }
        var found = IntPtr.Zero;
        _ = EnumWindows(
            (window, parameter) =>
            {
                if (!GetWindowClass(window).Equals(
                        "DOpus.ParentWindow",
                        StringComparison.OrdinalIgnoreCase))
                {
                    return true;
                }
                _ = GetWindowThreadProcessId(window, out var processId);
                if (processId != targetProcessId)
                {
                    return true;
                }
                found = window;
                return false;
            },
            IntPtr.Zero);
        return found;
    }

    private static string GetWindowClass(IntPtr window)
    {
        var value = new StringBuilder(256);
        return GetClassName(window, value, value.Capacity) > 0
            ? value.ToString()
            : string.Empty;
    }

    private static string GetWindowCaption(IntPtr window)
    {
        var value = new StringBuilder(2_048);
        return GetWindowText(window, value, value.Capacity) > 0
            ? value.ToString().Trim()
            : string.Empty;
    }

    private static bool WindowBelongsToProcess(
        IntPtr window,
        string expectedProcessName)
    {
        _ = GetWindowThreadProcessId(window, out var processId);
        if (processId == 0)
        {
            return false;
        }
        try
        {
            using var process = Process.GetProcessById((int)processId);
            return process.ProcessName.Equals(
                expectedProcessName,
                StringComparison.OrdinalIgnoreCase);
        }
        catch (ArgumentException)
        {
            return false;
        }
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct WindowRectangle
    {
        public int Left;
        public int Top;
        public int Right;
        public int Bottom;
    }

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetWindowRect(
        IntPtr window,
        out WindowRectangle rectangle);

    [StructLayout(LayoutKind.Sequential)]
    private struct CopyDataStruct
    {
        public IntPtr DataId;
        public int ByteCount;
        public IntPtr Data;
    }

    private delegate bool EnumWindowsProc(IntPtr window, IntPtr parameter);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool EnumChildWindows(
        IntPtr parent,
        EnumWindowsProc callback,
        IntPtr parameter);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetClassName(
        IntPtr window,
        StringBuilder value,
        int maximum);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetWindowText(
        IntPtr window,
        StringBuilder value,
        int maximum);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsWindow(IntPtr window);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsWindowVisible(IntPtr window);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsChild(IntPtr parent, IntPtr window);

    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(
        IntPtr window,
        out uint processId);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool EnumWindows(
        EnumWindowsProc callback,
        IntPtr parameter);

    [DllImport("user32.dll", SetLastError = true)]
    private static extern IntPtr SendMessageTimeout(
        IntPtr window,
        uint message,
        IntPtr wParam,
        ref CopyDataStruct lParam,
        uint flags,
        uint timeout,
        out IntPtr result);
}
