using System.Collections.Concurrent;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Threading.Channels;
using System.Windows.Forms;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal sealed record PasteResult(bool Success, string? Error = null);

internal sealed class WindowsPlatformAdapter : IDisposable
{
    private readonly Channel<WindowsPlatformEvent> _events = Channel.CreateBounded<WindowsPlatformEvent>(
        new BoundedChannelOptions(512)
        {
            FullMode = BoundedChannelFullMode.DropOldest,
            SingleReader = true,
            SingleWriter = true
        });
    private readonly ManualResetEventSlim _started = new(false);
    private readonly Thread _thread;
    private EventWindow? _window;
    private Exception? _startupError;
    private bool _disposed;

    public WindowsPlatformAdapter(ForegroundContextProvider foregroundContextProvider)
    {
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

    public Task<PasteResult> PasteTextAsync(
        PasteTextCommand command,
        CancellationToken cancellationToken)
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
        return (_window ?? throw new InvalidOperationException("Windows message window is unavailable."))
            .EnqueuePaste(command, cancellationToken);
    }

    private void RunMessageLoop(ForegroundContextProvider foregroundContextProvider)
    {
        try
        {
            _window = new EventWindow(_events.Writer, foregroundContextProvider);
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

    private sealed class EventWindow : NativeWindow
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
        private readonly ConcurrentQueue<PasteRequest> _pasteRequests = new();
        private readonly ClipboardWriteSuppressor _writeSuppressor = new();
        private readonly NativeMethods.WinEventDelegate _foregroundCallback;
        private IntPtr _foregroundHook;
        private ForegroundContext? _pendingClipboardSource;
        private int _clipboardReadAttempts;

        public bool ClipboardHotKeyAvailable { get; }

        public EventWindow(
            ChannelWriter<WindowsPlatformEvent> events,
            ForegroundContextProvider foregroundContextProvider)
        {
            _events = events;
            _foregroundContextProvider = foregroundContextProvider;
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

        public Task<PasteResult> EnqueuePaste(
            PasteTextCommand command,
            CancellationToken cancellationToken)
        {
            var completion = new TaskCompletionSource<PasteResult>(
                TaskCreationOptions.RunContinuationsAsynchronously);
            _pasteRequests.Enqueue(new PasteRequest(command, cancellationToken, completion));
            if (!NativeMethods.PostMessage(Handle, WmExecutePaste, IntPtr.Zero, IntPtr.Zero))
            {
                completion.TrySetResult(new PasteResult(false, "Failed to schedule paste on the Windows thread."));
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
                    _pendingClipboardSource = _foregroundContextProvider.Capture();
                    _clipboardReadAttempts = 0;
                    TryPublishClipboardText();
                    return;
                case WmHotKey when message.WParam.ToInt32() == ClipboardHotKeyId:
                    PublishPanelRequest();
                    return;
                case WmTimer when message.WParam.ToInt64() == RetryTimerId:
                    _ = NativeMethods.KillTimer(Handle, (UIntPtr)RetryTimerId);
                    TryPublishClipboardText();
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

        private void TryPublishClipboardText()
        {
            try
            {
                if (!Clipboard.ContainsText(TextDataFormat.UnicodeText))
                {
                    return;
                }
                var text = Clipboard.GetText(TextDataFormat.UnicodeText);
                if (text.Length == 0 || _writeSuppressor.ConsumeIfMatches(text))
                {
                    return;
                }
                var source = _pendingClipboardSource ?? _foregroundContextProvider.Capture();
                _events.TryWrite(new ClipboardTextCapturedEvent(
                    text,
                    source,
                    DateTimeOffset.UtcNow.ToUnixTimeMilliseconds()));
            }
            catch (ExternalException) when (++_clipboardReadAttempts < MaxClipboardReadAttempts)
            {
                _ = NativeMethods.SetTimer(Handle, (UIntPtr)RetryTimerId, 40, IntPtr.Zero);
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
                request.Completion.TrySetResult(ExecutePaste(request.Command));
            }
        }

        private PasteResult ExecutePaste(PasteTextCommand command)
        {
            var target = new IntPtr(command.TargetWindowHandle);
            if (target == IntPtr.Zero || !NativeMethods.IsWindow(target))
            {
                return new PasteResult(false, "The target window no longer exists.");
            }
            if (!TrySetClipboardText(command.Text, out var clipboardError))
            {
                return new PasteResult(false, clipboardError);
            }
            if (!NativeMethods.ActivateWindow(
                    target,
                    new IntPtr(command.TargetFocusWindowHandle)))
            {
                return new PasteResult(false, "Windows refused to activate the target window.");
            }

            Thread.Sleep(25);
            return NativeMethods.SendPasteShortcut()
                ? new PasteResult(true)
                : new PasteResult(false, "Windows failed to send the paste shortcut.");
        }

        private bool TrySetClipboardText(string text, out string? error)
        {
            for (var attempt = 0; attempt < 5; attempt++)
            {
                _writeSuppressor.SuppressNext(text);
                try
                {
                    Clipboard.SetText(text, TextDataFormat.UnicodeText);
                    error = null;
                    return true;
                }
                catch (ExternalException exception)
                {
                    _writeSuppressor.Cancel(text);
                    if (attempt == 4)
                    {
                        error = $"Could not open the Windows clipboard: {exception.Message}";
                        return false;
                    }
                    Thread.Sleep(20);
                }
            }
            error = "Could not open the Windows clipboard.";
            return false;
        }

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
                request.Completion.TrySetResult(new PasteResult(false, "Windows adapter is stopping."));
            }
            DestroyHandle();
            Application.ExitThread();
        }
    }

    private sealed class ClipboardWriteSuppressor
    {
        private string? _expectedText;

        public void SuppressNext(string text) => _expectedText = text;

        public bool ConsumeIfMatches(string text)
        {
            if (_expectedText is null || !string.Equals(_expectedText, text, StringComparison.Ordinal))
            {
                return false;
            }
            _expectedText = null;
            return true;
        }

        public void Cancel(string text)
        {
            if (string.Equals(_expectedText, text, StringComparison.Ordinal))
            {
                _expectedText = null;
            }
        }
    }

    private sealed record PasteRequest(
        PasteTextCommand Command,
        CancellationToken CancellationToken,
        TaskCompletionSource<PasteResult> Completion);

    private static class NativeMethods
    {
        private const int SwRestore = 9;
        private const uint InputKeyboard = 1;
        private const uint KeyeventfKeyup = 0x0002;
        private const ushort VkControl = 0x11;
        private const ushort VkV = 0x56;
        private const uint MonitorDefaultToNearest = 2;
        private const int MonitorDpiTypeEffective = 0;
        private const uint DefaultDpi = 96;

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
        private static extern IntPtr GetForegroundWindow();

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
            return GetForegroundWindow() == target;
        }

        internal static bool SendPasteShortcut()
        {
            var inputs = new[]
            {
                KeyboardInput(VkControl, 0),
                KeyboardInput(VkV, 0),
                KeyboardInput(VkV, KeyeventfKeyup),
                KeyboardInput(VkControl, KeyeventfKeyup)
            };
            return SendInput((uint)inputs.Length, inputs, Marshal.SizeOf<Input>()) == inputs.Length;
        }

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
