using System.Collections.Concurrent;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Threading.Channels;
using System.Windows.Forms;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal sealed record WindowsCommandResult(bool Success, string? Error = null);

internal sealed class WindowsPlatformAdapter : IDisposable
{
    internal static int NativeInputSize => NativeMethods.InputSize;

    private readonly Channel<WindowsPlatformEvent> _events = Channel.CreateBounded<WindowsPlatformEvent>(
        new BoundedChannelOptions(512)
        {
            FullMode = BoundedChannelFullMode.DropOldest,
            SingleReader = true,
            SingleWriter = true
        });
    private readonly ManualResetEventSlim _started = new(false);
    private readonly Thread _thread;
    private readonly IPlatformLog _log;
    private EventWindow? _window;
    private Exception? _startupError;
    private bool _disposed;

    public WindowsPlatformAdapter(
        ForegroundContextProvider foregroundContextProvider,
        IPlatformLog? log = null)
    {
        _log = log ?? NullPlatformLog.Instance;
        _thread = new Thread(() => RunMessageLoop(foregroundContextProvider))
        {
            IsBackground = true,
            Name = "NxTools.WindowsPlatformAdapter"
        };
        _thread.SetApartmentState(ApartmentState.STA);
        _thread.Start();
        _started.Wait();
        if (_startupError is not null)
        {
            throw new InvalidOperationException("Windows platform adapter failed to start.", _startupError);
        }
    }

    public ChannelReader<WindowsPlatformEvent> Events => _events.Reader;

    public bool ClipboardHotKeyAvailable => _window?.ClipboardHotKeyAvailable ?? false;

    public Task<WindowsCommandResult> PasteTextAsync(
        PasteTextCommand command,
        Guid correlationId,
        CancellationToken cancellationToken)
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
        return (_window ?? throw new InvalidOperationException("Windows message window is unavailable."))
            .EnqueuePaste(new TextClipboardPastePayload(
                command.Text, command.TargetWindowHandle,
                command.TargetFocusWindowHandle), correlationId, cancellationToken);
    }

    public Task<WindowsCommandResult> PasteFilesAsync(
        PasteFilesCommand command, Guid correlationId, CancellationToken cancellationToken)
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
        return (_window ?? throw new InvalidOperationException("Windows message window is unavailable."))
            .EnqueuePaste(new FilesClipboardPastePayload(
                command.Paths, command.TargetWindowHandle,
                command.TargetFocusWindowHandle), correlationId, cancellationToken);
    }

    public Task<WindowsCommandResult> PasteImageAsync(
        PasteImageCommand command, Guid correlationId, CancellationToken cancellationToken)
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
        return (_window ?? throw new InvalidOperationException("Windows message window is unavailable."))
            .EnqueuePaste(new ImageClipboardPastePayload(
                command.ImagePath, command.TargetWindowHandle,
                command.TargetFocusWindowHandle), correlationId, cancellationToken);
    }

    public WindowsCommandResult SetWindowTopmost(SetWindowTopmostCommand command) =>
        NativeMethods.SetWindowTopmost(new IntPtr(command.WindowHandle), command.Enabled);

    public WindowsCommandResult SetWindowVisibility(SetWindowVisibilityCommand command) =>
        NativeMethods.SetWindowVisibility(new IntPtr(command.WindowHandle), command.Visible);

    private void RunMessageLoop(ForegroundContextProvider foregroundContextProvider)
    {
        try
        {
            _window = new EventWindow(_events.Writer, foregroundContextProvider, _log);
            _started.Set();
            Application.Run();
        }
        catch (Exception error)
        {
            _startupError = error;
            _started.Set();
        }
        finally
        {
            _events.Writer.TryComplete(_startupError);
        }
    }

    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }
        _disposed = true;
        _window?.RequestStop();
        if (!_thread.Join(TimeSpan.FromSeconds(5)))
        {
            Console.Error.WriteLine("Windows platform adapter did not stop within five seconds.");
        }
        _started.Dispose();
    }

    private sealed class EventWindow : NativeWindow, IWindowsPasteOperations
    {
        private const int WmClipboardUpdate = 0x031D;
        private const int WmHotKey = 0x0312;
        private const int WmClose = 0x0010;
        private const int WmTimer = 0x0113;
        private const int WmExecutePaste = 0x8001;
        private const uint EventSystemForeground = 0x0003;
        private const uint WineventOutOfContext = 0x0000;
        private const uint RetryTimerId = 1;
        private const int MaxClipboardReadAttempts = 5;
        private const int ClipboardHotKeyId = 1;
        private const uint ModAlt = 0x0001;
        private const uint ModNoRepeat = 0x4000;
        private const uint VkC = 0x43;
        private static readonly IntPtr HwndMessage = new(-3);

        private readonly ChannelWriter<WindowsPlatformEvent> _events;
        private readonly ForegroundContextProvider _foregroundContextProvider;
        private readonly IPlatformLog _log;
        private readonly ConcurrentQueue<PasteRequest> _pasteRequests = new();
        private readonly NativeMethods.WinEventDelegate _foregroundCallback;
        private IntPtr _foregroundHook;
        private ForegroundContext? _pendingClipboardSource;
        private int _clipboardReadAttempts;
        private bool _suppressNextClipboardUpdate;

        public bool ClipboardHotKeyAvailable { get; }

        public EventWindow(
            ChannelWriter<WindowsPlatformEvent> events,
            ForegroundContextProvider foregroundContextProvider,
            IPlatformLog log)
        {
            _events = events;
            _foregroundContextProvider = foregroundContextProvider;
            _log = log;
            _foregroundCallback = OnForegroundChanged;
            CreateHandle(new CreateParams
            {
                Caption = "NxTools.PlatformHost.Events",
                Parent = HwndMessage
            });

            if (!NativeMethods.AddClipboardFormatListener(Handle))
            {
                throw new Win32Exception(Marshal.GetLastWin32Error());
            }
            _foregroundHook = NativeMethods.SetWinEventHook(
                EventSystemForeground,
                EventSystemForeground,
                IntPtr.Zero,
                _foregroundCallback,
                0,
                0,
                WineventOutOfContext);
            if (_foregroundHook == IntPtr.Zero)
            {
                _ = NativeMethods.RemoveClipboardFormatListener(Handle);
                throw new Win32Exception(Marshal.GetLastWin32Error());
            }

            ClipboardHotKeyAvailable = NativeMethods.RegisterHotKey(
                    Handle,
                    ClipboardHotKeyId,
                    ModAlt | ModNoRepeat,
                    VkC);
            if (!ClipboardHotKeyAvailable)
            {
                Console.Error.WriteLine(
                    $"Could not register clipboard hotkey Alt+C: Win32={Marshal.GetLastWin32Error()}");
            }

            PublishForeground(_foregroundContextProvider.Capture());
        }

        public Task<WindowsCommandResult> EnqueuePaste(
            ClipboardPastePayload command,
            Guid correlationId,
            CancellationToken cancellationToken)
        {
            var completion = new TaskCompletionSource<WindowsCommandResult>(
                TaskCreationOptions.RunContinuationsAsynchronously);
            _pasteRequests.Enqueue(new PasteRequest(command, correlationId, cancellationToken, completion));
            if (!NativeMethods.PostMessage(Handle, WmExecutePaste, IntPtr.Zero, IntPtr.Zero))
            {
                completion.TrySetResult(new WindowsCommandResult(false, "Failed to schedule paste on the Windows thread."));
            }
            return completion.Task;
        }

        public void RequestStop()
        {
            _ = NativeMethods.PostMessage(Handle, WmClose, IntPtr.Zero, IntPtr.Zero);
        }

        protected override void WndProc(ref Message message)
        {
            switch (message.Msg)
            {
                case WmClipboardUpdate:
                    if (_suppressNextClipboardUpdate)
                    {
                        _suppressNextClipboardUpdate = false;
                        return;
                    }
                    _pendingClipboardSource = _foregroundContextProvider.Capture();
                    _clipboardReadAttempts = 0;
                    TryPublishClipboardContent();
                    return;
                case WmHotKey when message.WParam.ToInt32() == ClipboardHotKeyId:
                    PublishPanelRequest();
                    return;
                case WmTimer when message.WParam.ToInt64() == RetryTimerId:
                    _ = NativeMethods.KillTimer(Handle, (UIntPtr)RetryTimerId);
                    TryPublishClipboardContent();
                    return;
                case WmExecutePaste:
                    DrainPasteRequests();
                    return;
                case WmClose:
                    Stop();
                    return;
            }
            base.WndProc(ref message);
        }

        private void TryPublishClipboardContent()
        {
            try
            {
                var source = _pendingClipboardSource ?? _foregroundContextProvider.Capture();
                var capturedAt = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
                var paths = Clipboard.ContainsFileDropList()
                    ? Clipboard.GetFileDropList()
                        .Cast<string>()
                        .Where(static path => !string.IsNullOrWhiteSpace(path))
                        .Take(1024)
                        .ToArray()
                    : [];
                ClipboardImageData? imageData = null;
                if (Clipboard.ContainsImage())
                {
                    using var image = Clipboard.GetImage();
                    if (image is not null)
                    {
                        var directory = Path.Combine(Path.GetTempPath(), "NxTools.Next", "clipboard-staging");
                        Directory.CreateDirectory(directory);
                        var path = Path.Combine(directory, $"{Guid.NewGuid():N}.png");
                        image.Save(path, System.Drawing.Imaging.ImageFormat.Png);
                        imageData = new ClipboardImageData(
                            path,
                            checked((uint)image.Width),
                            checked((uint)image.Height));
                    }
                }
                string? text = null;
                if (Clipboard.ContainsText(TextDataFormat.UnicodeText))
                {
                    text = Clipboard.GetText(TextDataFormat.UnicodeText);
                }
                if (paths.Length > 0 || imageData is not null || !string.IsNullOrEmpty(text))
                {
                    var sourceIconPath = _foregroundContextProvider.TryStageProcessIcon(source.ProcessId);
                    _events.TryWrite(new ClipboardCapturedEvent(
                        text, imageData, paths, sourceIconPath, source, capturedAt));
                }
            }
            catch (ExternalException) when (++_clipboardReadAttempts < MaxClipboardReadAttempts)
            {
                _ = NativeMethods.SetTimer(Handle, (UIntPtr)RetryTimerId, 40, IntPtr.Zero);
            }
            catch (Exception error)
            {
                _log.Write("error", "clipboard.capture_failed", null, new Dictionary<string, object?>
                {
                    ["exception_type"] = error.GetType().FullName,
                    ["message"] = error.Message
                });
            }
        }

        private void OnForegroundChanged(
            IntPtr hook,
            uint eventType,
            IntPtr window,
            int objectId,
            int childId,
            uint eventThread,
            uint eventTime)
        {
            if (window != IntPtr.Zero)
            {
                PublishForeground(_foregroundContextProvider.Capture(window));
            }
        }

        private void PublishForeground(ForegroundContext context)
        {
            _events.TryWrite(new ForegroundChangedEvent(context));
        }

        private void PublishPanelRequest()
        {
            var target = _foregroundContextProvider.Capture();
            var cursor = NativeMethods.GetCursorLocation();
            _events.TryWrite(new ClipboardPanelRequestedEvent(
                target,
                cursor.Position.X,
                cursor.Position.Y,
                cursor.DisplayIndex,
                cursor.Dpi,
                DateTimeOffset.UtcNow.ToUnixTimeMilliseconds()));
        }

        private void DrainPasteRequests()
        {
            while (_pasteRequests.TryDequeue(out var request))
            {
                if (request.CancellationToken.IsCancellationRequested)
                {
                    request.Completion.TrySetCanceled(request.CancellationToken);
                    continue;
                }
                request.Completion.TrySetResult(
                    new WindowsPasteCoordinator(this, _log).Execute(
                        request.Command,
                        request.CorrelationId));
            }
        }

        private ClipboardWriteResult WriteClipboard(ClipboardPastePayload payload)
        {
            var attempts = new List<ClipboardWriteAttempt>();
            for (var attempt = 0; attempt < 5; attempt++)
            {
                try
                {
                    _suppressNextClipboardUpdate = true;
                    switch (payload)
                    {
                        case TextClipboardPastePayload text:
                            Clipboard.SetText(text.Text, TextDataFormat.UnicodeText);
                            break;
                        case FilesClipboardPastePayload files:
                            var paths = new System.Collections.Specialized.StringCollection();
                            paths.AddRange(files.Paths.Where(static path =>
                                File.Exists(path) || Directory.Exists(path)).ToArray());
                            if (paths.Count == 0)
                            {
                                _suppressNextClipboardUpdate = false;
                                return new ClipboardWriteResult(false, "No pasted file paths still exist.", attempts);
                            }
                            Clipboard.SetFileDropList(paths);
                            break;
                        case ImageClipboardPastePayload imagePayload:
                            if (!File.Exists(imagePayload.ImagePath))
                            {
                                _suppressNextClipboardUpdate = false;
                                return new ClipboardWriteResult(false, "The clipboard image file no longer exists.", attempts);
                            }
                            using (var source = System.Drawing.Image.FromFile(imagePayload.ImagePath))
                            using (var image = new System.Drawing.Bitmap(source))
                            {
                                Clipboard.SetImage(image);
                            }
                            break;
                        default:
                            _suppressNextClipboardUpdate = false;
                            return new ClipboardWriteResult(false, "Unsupported clipboard paste payload.", attempts);
                    }
                    attempts.Add(new ClipboardWriteAttempt(attempt + 1, true, 0, null));
                    return new ClipboardWriteResult(true, null, attempts);
                }
                catch (ExternalException exception)
                {
                    _suppressNextClipboardUpdate = false;
                    attempts.Add(new ClipboardWriteAttempt(
                        attempt + 1,
                        false,
                        exception.ErrorCode,
                        exception.Message));
                    if (attempt == 4)
                    {
                        return new ClipboardWriteResult(
                            false,
                            $"Could not open the Windows clipboard: {exception.Message}",
                            attempts);
                    }
                    Thread.Sleep(20);
                }
                catch (Exception exception)
                {
                    _suppressNextClipboardUpdate = false;
                    attempts.Add(new ClipboardWriteAttempt(
                        attempt + 1,
                        false,
                        exception.HResult,
                        exception.Message));
                    return new ClipboardWriteResult(false, exception.Message, attempts);
                }
            }
            return new ClipboardWriteResult(false, "Could not open the Windows clipboard.", attempts);
        }

        bool IWindowsPasteOperations.IsWindow(long windowHandle) =>
            NativeMethods.IsWindow(new IntPtr(windowHandle));

        uint IWindowsPasteOperations.GetClipboardSequenceNumber() =>
            NativeMethods.GetClipboardSequenceNumber();

        ClipboardWriteResult IWindowsPasteOperations.WriteClipboard(ClipboardPastePayload payload) =>
            WriteClipboard(payload);

        bool IWindowsPasteOperations.TryActivateWindow(
            long targetWindowHandle,
            long focusWindowHandle) =>
            NativeMethods.ActivateWindow(
                new IntPtr(targetWindowHandle),
                new IntPtr(focusWindowHandle));

        bool IWindowsPasteOperations.IsForegroundWindowForTarget(long targetWindowHandle) =>
            NativeMethods.IsForegroundWindowForTarget(new IntPtr(targetWindowHandle));

        PasteWindowSnapshot IWindowsPasteOperations.CaptureWindowState(long targetWindowHandle) =>
            NativeMethods.CaptureWindowState(new IntPtr(targetWindowHandle));

        ModifierReleaseResult IWindowsPasteOperations.ReleaseActivationModifiers() =>
            NativeMethods.ReleaseActivationModifiers();

        PasteShortcutResult IWindowsPasteOperations.SendPasteShortcut() =>
            NativeMethods.SendPasteShortcut();

        PostPasteResult IWindowsPasteOperations.PostPaste(long targetWindowHandle) =>
            NativeMethods.PostPaste(new IntPtr(targetWindowHandle));

        void IWindowsPasteOperations.Delay(int milliseconds) => Thread.Sleep(milliseconds);

        private void Stop()
        {
            _ = NativeMethods.KillTimer(Handle, (UIntPtr)RetryTimerId);
            _ = NativeMethods.RemoveClipboardFormatListener(Handle);
            if (ClipboardHotKeyAvailable)
            {
                _ = NativeMethods.UnregisterHotKey(Handle, ClipboardHotKeyId);
            }
            if (_foregroundHook != IntPtr.Zero)
            {
                _ = NativeMethods.UnhookWinEvent(_foregroundHook);
                _foregroundHook = IntPtr.Zero;
            }
            while (_pasteRequests.TryDequeue(out var request))
            {
                request.Completion.TrySetResult(new WindowsCommandResult(false, "Windows adapter is stopping."));
            }
            DestroyHandle();
            Application.ExitThread();
        }
    }

    private sealed record PasteRequest(
        ClipboardPastePayload Command,
        Guid CorrelationId,
        CancellationToken CancellationToken,
        TaskCompletionSource<WindowsCommandResult> Completion);

    private static class NativeMethods
    {
        internal static int InputSize => Marshal.SizeOf<Input>();

        private const int SwRestore = 9;
        private const uint InputKeyboard = 1;
        private const uint KeyeventfKeyup = 0x0002;
        private const ushort VkControl = 0x11;
        private const ushort VkV = 0x56;
        private const ushort VkShift = 0x10;
        private const ushort VkMenu = 0x12;
        private const ushort VkLWin = 0x5B;
        private const ushort VkRWin = 0x5C;
        private const ushort VkLShift = 0xA0;
        private const ushort VkRShift = 0xA1;
        private const ushort VkLControl = 0xA2;
        private const ushort VkRControl = 0xA3;
        private const ushort VkLMenu = 0xA4;
        private const ushort VkRMenu = 0xA5;
        private const int WmPaste = 0x0302;
        private const uint GaRoot = 2;
        private const uint SwpNoSize = 0x0001;
        private const uint SwpNoMove = 0x0002;
        private const uint SwpNoActivate = 0x0010;
        private const int GwlExStyle = -20;
        private const long WsExTopmost = 0x00000008L;
        private const uint MonitorDefaultToNearest = 2;
        private const int MonitorDpiTypeEffective = 0;
        private const uint DefaultDpi = 96;
        private const int SwHide = 0;
        private const int SwShowNoActivate = 4;

        [UnmanagedFunctionPointer(CallingConvention.Winapi)]
        internal delegate void WinEventDelegate(
            IntPtr hook,
            uint eventType,
            IntPtr window,
            int objectId,
            int childId,
            uint eventThread,
            uint eventTime);

        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool AddClipboardFormatListener(IntPtr window);

        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool RemoveClipboardFormatListener(IntPtr window);

        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool RegisterHotKey(
            IntPtr window,
            int id,
            uint modifiers,
            uint virtualKey);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool UnregisterHotKey(IntPtr window, int id);

        [DllImport("user32.dll", SetLastError = true)]
        internal static extern IntPtr SetWinEventHook(
            uint eventMin,
            uint eventMax,
            IntPtr module,
            WinEventDelegate callback,
            uint processId,
            uint threadId,
            uint flags);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool UnhookWinEvent(IntPtr hook);

        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool PostMessage(IntPtr window, int message, IntPtr wParam, IntPtr lParam);

        [DllImport("user32.dll")]
        internal static extern UIntPtr SetTimer(IntPtr window, UIntPtr timerId, uint milliseconds, IntPtr callback);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool KillTimer(IntPtr window, UIntPtr timerId);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool IsWindow(IntPtr window);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool IsWindowVisible(IntPtr window);

        [DllImport("user32.dll")]
        internal static extern uint GetClipboardSequenceNumber();

        [DllImport("user32.dll")]
        private static extern IntPtr GetForegroundWindow();

        [DllImport("user32.dll")]
        private static extern IntPtr GetAncestor(IntPtr window, uint flags);

        [DllImport("user32.dll")]
        private static extern short GetAsyncKeyState(int virtualKey);

        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool SetWindowPos(
            IntPtr window,
            IntPtr insertAfter,
            int x,
            int y,
            int width,
            int height,
            uint flags);

        [DllImport("user32.dll", EntryPoint = "GetWindowLongPtrW", SetLastError = true)]
        private static extern IntPtr GetWindowLongPtr(IntPtr window, int index);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool GetCursorPos(out Point point);

        [DllImport("user32.dll")]
        private static extern IntPtr MonitorFromPoint(Point point, uint flags);

        [DllImport("shcore.dll")]
        private static extern int GetDpiForMonitor(
            IntPtr monitor,
            int dpiType,
            out uint dpiX,
            out uint dpiY);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool EnumDisplayMonitors(
            IntPtr deviceContext,
            IntPtr clipRect,
            MonitorEnumDelegate callback,
            IntPtr data);

        [DllImport("user32.dll")]
        private static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);

        [DllImport("kernel32.dll")]
        private static extern uint GetCurrentThreadId();

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool AttachThreadInput(uint attach, uint attachTo, bool isAttached);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool ShowWindow(IntPtr window, int command);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool IsIconic(IntPtr window);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool BringWindowToTop(IntPtr window);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool SetForegroundWindow(IntPtr window);

        [DllImport("user32.dll")]
        private static extern IntPtr GetFocus();

        [DllImport("user32.dll")]
        private static extern IntPtr SetFocus(IntPtr window);

        [DllImport("user32.dll", SetLastError = true)]
        private static extern uint SendInput(uint inputCount, Input[] inputs, int inputSize);

        internal static bool ActivateWindow(IntPtr target, IntPtr focusWindow)
        {
            if (IsIconic(target))
            {
                _ = ShowWindow(target, SwRestore);
            }
            var currentThread = GetCurrentThreadId();
            var targetThread = GetWindowThreadProcessId(target, out _);
            var foreground = GetForegroundWindow();
            var foregroundThread = foreground == IntPtr.Zero
                ? 0
                : GetWindowThreadProcessId(foreground, out _);

            var attachedToTarget = targetThread != 0 && targetThread != currentThread
                && AttachThreadInput(currentThread, targetThread, true);
            var attachedToForeground = foregroundThread != 0
                && foregroundThread != currentThread
                && foregroundThread != targetThread
                && AttachThreadInput(currentThread, foregroundThread, true);
            try
            {
                _ = BringWindowToTop(target);
                _ = SetForegroundWindow(target);
                if (focusWindow != IntPtr.Zero &&
                    focusWindow != target &&
                    IsWindow(focusWindow) &&
                    GetFocus() != focusWindow)
                {
                    _ = SetFocus(focusWindow);
                }
            }
            finally
            {
                if (attachedToForeground)
                {
                    _ = AttachThreadInput(currentThread, foregroundThread, false);
                }
                if (attachedToTarget)
                {
                    _ = AttachThreadInput(currentThread, targetThread, false);
                }
            }
            return IsForegroundWindowForTarget(target);
        }

        internal static bool IsForegroundWindowForTarget(IntPtr target)
        {
            var foreground = GetForegroundWindow();
            if (foreground == target)
            {
                return true;
            }
            if (foreground == IntPtr.Zero || target == IntPtr.Zero)
            {
                return false;
            }
            var targetRoot = GetAncestor(target, GaRoot);
            var foregroundRoot = GetAncestor(foreground, GaRoot);
            return targetRoot != IntPtr.Zero && targetRoot == foregroundRoot;
        }

        internal static PasteWindowSnapshot CaptureWindowState(IntPtr target)
        {
            var currentThread = GetCurrentThreadId();
            uint targetProcess = 0;
            var targetThread = target == IntPtr.Zero
                ? 0
                : GetWindowThreadProcessId(target, out targetProcess);
            var foreground = GetForegroundWindow();
            uint foregroundProcess = 0;
            var foregroundThread = foreground == IntPtr.Zero
                ? 0
                : GetWindowThreadProcessId(foreground, out foregroundProcess);
            return new PasteWindowSnapshot(
                foreground.ToInt64(),
                GetAncestor(foreground, GaRoot).ToInt64(),
                GetAncestor(target, GaRoot).ToInt64(),
                currentThread,
                targetThread,
                foregroundThread,
                targetProcess,
                foregroundProcess);
        }

        internal static ModifierReleaseResult ReleaseActivationModifiers()
        {
            ushort[] keys =
            [
                VkShift, VkControl, VkMenu,
                VkLShift, VkRShift, VkLControl, VkRControl, VkLMenu, VkRMenu,
                VkLWin, VkRWin
            ];
            var initiallyPressed = keys.Where(IsKeyDown).ToArray();
            var inputSteps = new List<KeyboardInputStep>();
            var attempts = 0;
            for (; attempts < 12; attempts++)
            {
                var pressed = keys.Where(IsKeyDown).ToArray();
                if (pressed.Length == 0)
                {
                    return new ModifierReleaseResult(true, initiallyPressed, [], attempts, inputSteps);
                }
                foreach (var key in pressed)
                {
                    inputSteps.Add(SendKeyboardInput("modifier_up", key, KeyeventfKeyup));
                }
                Thread.Sleep(3);
            }
            var finallyPressed = keys.Where(IsKeyDown).ToArray();
            return new ModifierReleaseResult(
                finallyPressed.Length == 0,
                initiallyPressed,
                finallyPressed,
                attempts,
                inputSteps);
        }

        internal static PasteShortcutResult SendPasteShortcut()
        {
            KeyboardInputStep[] steps =
            [
                SendKeyboardInput("ctrl_down", VkControl, 0),
                DelayThenSend(10, "v_down", VkV, 0),
                DelayThenSend(10, "v_up", VkV, KeyeventfKeyup),
                DelayThenSend(5, "ctrl_up", VkControl, KeyeventfKeyup)
            ];
            return new PasteShortcutResult(
                steps.Aggregate(0U, static (sent, step) => sent + step.Sent),
                Marshal.SizeOf<Input>(),
                steps);
        }

        private static KeyboardInputStep DelayThenSend(
            int milliseconds,
            string name,
            ushort key,
            uint flags)
        {
            Thread.Sleep(milliseconds);
            return SendKeyboardInput(name, key, flags);
        }

        internal static PostPasteResult PostPaste(IntPtr target)
        {
            if (target == IntPtr.Zero || !IsWindow(target))
            {
                return new PostPasteResult(false, 1400, "The paste target is no longer a valid window.");
            }
            Marshal.SetLastPInvokeError(0);
            if (PostMessage(target, WmPaste, IntPtr.Zero, IntPtr.Zero))
            {
                return new PostPasteResult(true, 0, null);
            }
            var error = Marshal.GetLastPInvokeError();
            return new PostPasteResult(
                false,
                error,
                $"PostMessage(WM_PASTE) failed with Win32={error}.");
        }

        internal static WindowsCommandResult SetWindowTopmost(IntPtr window, bool enabled)
        {
            if (window == IntPtr.Zero || !IsWindow(window))
            {
                return new WindowsCommandResult(false, "The GPUI window handle is invalid.");
            }
            var insertAfter = enabled ? new IntPtr(-1) : new IntPtr(-2);
            var lastError = 0;
            for (var attempt = 0; attempt < 3; attempt++)
            {
                if (!SetWindowPos(
                    window,
                    insertAfter,
                    0,
                    0,
                    0,
                    0,
                    SwpNoMove | SwpNoSize | SwpNoActivate))
                {
                    lastError = Marshal.GetLastWin32Error();
                }
                else if (IsTopmost(window) == enabled)
                {
                    return new WindowsCommandResult(true);
                }
                Thread.Sleep(10);
            }
            return new WindowsCommandResult(
                false,
                lastError == 0
                    ? "Windows did not apply the requested topmost state."
                    : $"SetWindowPos failed with Win32={lastError}.");
        }

        internal static WindowsCommandResult SetWindowVisibility(IntPtr window, bool visible)
        {
            if (window == IntPtr.Zero || !IsWindow(window))
            {
                return new WindowsCommandResult(false, "The GPUI window handle is invalid.");
            }

            _ = ShowWindow(window, visible ? SwShowNoActivate : SwHide);
            return IsWindowVisible(window) == visible
                ? new WindowsCommandResult(true)
                : new WindowsCommandResult(false, "Windows did not apply the requested visibility state.");
        }

        private static bool IsTopmost(IntPtr window) =>
            (GetWindowLongPtr(window, GwlExStyle).ToInt64() & WsExTopmost) != 0;

        internal static CursorLocation GetCursorLocation()
        {
            var position = GetCursorPos(out var point) ? point : default;
            var monitor = MonitorFromPoint(position, MonitorDefaultToNearest);
            var dpi = DefaultDpi;
            if (monitor != IntPtr.Zero &&
                GetDpiForMonitor(monitor, MonitorDpiTypeEffective, out var dpiX, out _) == 0 &&
                dpiX > 0)
            {
                dpi = dpiX;
            }
            return new CursorLocation(position, GetDisplayIndex(monitor), dpi);
        }

        private static uint GetDisplayIndex(IntPtr targetMonitor)
        {
            uint current = 0;
            uint? result = null;
            _ = EnumDisplayMonitors(
                IntPtr.Zero,
                IntPtr.Zero,
                (monitor, _, _, _) =>
                {
                    if (monitor == targetMonitor)
                    {
                        result = current;
                        return false;
                    }
                    current++;
                    return true;
                },
                IntPtr.Zero);
            return result ?? 0;
        }

        private static Input KeyboardInput(ushort key, uint flags) => new()
        {
            Type = InputKeyboard,
            Union = new InputUnion
            {
                Keyboard = new KeyboardInputData
                {
                    VirtualKey = key,
                    Flags = flags
                }
            }
        };

        private static bool IsKeyDown(ushort key) => (GetAsyncKeyState(key) & 0x8000) != 0;

        private static KeyboardInputStep SendKeyboardInput(string name, ushort key, uint flags)
        {
            var input = new[] { KeyboardInput(key, flags) };
            Marshal.SetLastPInvokeError(0);
            var sent = SendInput(1, input, Marshal.SizeOf<Input>());
            var error = sent == 0 ? Marshal.GetLastPInvokeError() : 0;
            return new KeyboardInputStep(name, key, flags, sent, error);
        }

        [StructLayout(LayoutKind.Sequential)]
        internal struct Point
        {
            public int X;
            public int Y;
        }

        internal readonly record struct CursorLocation(Point Position, uint DisplayIndex, uint Dpi);

        private delegate bool MonitorEnumDelegate(
            IntPtr monitor,
            IntPtr deviceContext,
            IntPtr monitorRect,
            IntPtr data);

        [StructLayout(LayoutKind.Sequential)]
        private struct Input
        {
            public uint Type;
            public InputUnion Union;
        }

        [StructLayout(LayoutKind.Explicit)]
        private struct InputUnion
        {
            [FieldOffset(0)]
            public KeyboardInputData Keyboard;

            [FieldOffset(0)]
            public MouseInputData Mouse;

            [FieldOffset(0)]
            public HardwareInputData Hardware;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct MouseInputData
        {
            public int X;
            public int Y;
            public uint MouseData;
            public uint Flags;
            public uint Time;
            public UIntPtr ExtraInfo;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct HardwareInputData
        {
            public uint Message;
            public ushort ParameterLow;
            public ushort ParameterHigh;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct KeyboardInputData
        {
            public ushort VirtualKey;
            public ushort ScanCode;
            public uint Flags;
            public uint Time;
            public UIntPtr ExtraInfo;
        }
    }
}
