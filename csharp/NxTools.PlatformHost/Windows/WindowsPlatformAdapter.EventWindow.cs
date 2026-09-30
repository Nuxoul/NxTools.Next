using System.Collections.Concurrent;
using System.Text;
using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Threading.Channels;
using System.Windows.Forms;
using Microsoft.Win32;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal sealed partial class WindowsPlatformAdapter
{
    internal sealed class EventWindow : NativeWindow, IWindowsPasteOperations
    {
        private const int WmClipboardUpdate = 0x031D;
        private const int WmExitSizeMove = 0x0232;
        private const int WmHotKey = 0x0312;
        private const int WmClose = 0x0010;
        private const int WmTimer = 0x0113;
        private const int WmExecutePaste = 0x8001;
        private const int WmConfigureClipboard = 0x8002;
        private const int WmSetClipboardHotKeyRecording = 0x8003;
        private const int WmConfigureQuickLaunch = 0x8004;
        private const int WmSetQuickLaunchHotKeyRecording = 0x8005;
        private const int WmQuickLaunchGesture = 0x8006;
        private const int WmDirectorySourceResolved = 0x8007;
        private const int WmFileDialogStructureChanged = 0x8008;
        private const int WmFileDialogGeometryChanged = 0x8009;
        private const int WmTaskbarOverlayChanged = 0x800A;
        private const int WmConfigureDesktopShell = 0x800B;
        private const int WmSettingChange = 0x001A;
        private const int WmDisplayChange = 0x007E;
        private const uint EventSystemForeground = 0x0003;
        private const uint EventSystemMoveSizeEnd = 0x000B;
        private const uint EventObjectCreate = 0x8000;
        private const uint EventObjectDestroy = 0x8001;
        private const uint EventObjectShow = 0x8002;
        private const uint EventObjectHide = 0x8003;
        private const uint EventObjectLocationChange = 0x800B;
        private const uint EventObjectNameChange = 0x800C;
        private const int ObjIdWindow = 0;
        private const uint WineventOutOfContext = 0x0000;
        private const uint RetryTimerId = 1;
        private const uint PointerMonitorTimerId = 2;
        private const uint FileDialogRecognitionTimerId = 3;
        private const uint DirectorySourceRecognitionTimerId = 4;
        private const uint PointerMonitorIntervalMs = 40;
        private const uint FileDialogRecognitionIntervalMs = 30;
        private const int FileDialogRecognitionMaxAttempts = 200;
        private const int DirectorySourceRecognitionMaxAttempts = 20;
        private const int MaxClipboardReadAttempts = 5;
        private const int MaxClipboardPublishTail = 64;
        private const int ClipboardHotKeyId = 1;
        private const int QuickLaunchHotKeyId = 2;
        private const uint ModAlt = 0x0001;
        private const uint ModNoRepeat = 0x4000;
        private const uint VkC = 0x43;
        private const uint VkSpace = 0x20;
        private const int VkLeftControl = 0xA2;
        private const int VkRightControl = 0xA3;
        private static readonly IntPtr HwndMessage = new(-3);

        private readonly ChannelWriter<WindowsPlatformEvent> _events;
        private readonly ForegroundContextProvider _foregroundContextProvider;
        private readonly IElevatedDirectoryNavigationChannel _elevatedDirectoryNavigation;
        private readonly IPlatformLog _log;
        private readonly ClipboardCaptureDropDiagnosticGate _clipboardCaptureDropDiagnosticGate;
        private readonly AutoQuickSwitchTracker _autoQuickSwitch = new();
        private readonly FileDialogRecognitionRetry _fileDialogRecognitionRetry =
            new(FileDialogRecognitionMaxAttempts);
        private readonly FileDialogRecognitionRetry _directorySourceRecognitionRetry =
            new(DirectorySourceRecognitionMaxAttempts);
        private readonly FileDialogGeometryUpdateGate _fileDialogGeometryUpdateGate = new();
        private readonly ConcurrentQueue<PasteRequest> _pasteRequests = new();
        private readonly ConcurrentQueue<ClipboardConfigurationRequest> _configurationRequests = new();
        private readonly ConcurrentQueue<ClipboardHotKeyRecordingRequest> _hotKeyRecordingRequests = new();
        private readonly ConcurrentQueue<QuickLaunchConfigurationRequest> _quickLaunchConfigurationRequests = new();
        private readonly ConcurrentQueue<QuickLaunchHotKeyRecordingRequest> _quickLaunchHotKeyRecordingRequests = new();
        private readonly ConcurrentQueue<DesktopShellConfigurationRequest> _desktopShellConfigurationRequests = new();
        private readonly ConcurrentQueue<(long WindowHandle, string Directory, string DisplayName)>
            _resolvedDirectorySources = new();
        private readonly NativeMethods.WinEventDelegate _foregroundCallback;
        private readonly NativeMethods.WinEventDelegate _fileDialogWindowCallback;
        private IntPtr _foregroundHook;
        private IntPtr _fileDialogCreateHook;
        private IntPtr _fileDialogDestroyHook;
        private IntPtr _fileDialogShowHook;
        private IntPtr _fileDialogHideHook;
        private IntPtr _fileDialogLocationHook;
        private IntPtr _fileDialogNameHook;
        private IntPtr _taskbarOverlayMoveSizeEndHook;
        private ForegroundContext? _pendingClipboardSource;
        private Task _capturePublishTail = Task.CompletedTask;
        private int _capturePublishPending;
        private int _clipboardReadAttempts;
        // A centered resize requested while the window is inside a drag/size
        // modal loop: SetWindowPos is swallowed (or misapplied) during the
        // loop, so it is remembered and re-applied on WM_EXITSIZEMOVE.
        private (int X, int Y, int Width, int Height)? _pendingCenteredResize;
        private bool _suppressNextClipboardUpdate;
        private bool _primaryPointerDown;
        private bool _secondaryPointerDown;
        private int _fileDialogStructureRetryPosted;
        private ulong _platformEventSequence;

        private ulong NextPlatformEventSequence() => ++_platformEventSequence;
        private ClipboardHotKeyRegistration _hotKeyRegistration = null!;
        private ClipboardHotKeyRegistration _quickLaunchHotKeyRegistration = null!;
        private readonly ClipboardHotKeyDebouncer _clipboardHotKeyDebouncer = new();
        // The measured interval spans FIRST press to SECOND press, which
        // includes how long the first Ctrl is held. A deliberate user tap
        // easily exceeds 500 ms of combined hold+gap, so use Listary-grade
        // bounds: relaxed minimum for distinct presses, generous maximum.
        private readonly ModifierDoubleTapDetector _leftControlDoubleTap =
            new(VkLeftControl, 60, 800);
        private readonly ModifierDoubleTapDetector _rightControlDoubleTap =
            new(VkRightControl, 60, 800);
        private LowLevelKeyboardMonitor? _quickLaunchKeyboardMonitor;
        private QuickLaunchTriggerBinding _quickLaunchTrigger = new(
            QuickLaunchTriggerKind.DoubleControl,
            "双击 Ctrl",
            null);
        private bool _quickLaunchTriggerEnabled = true;
        private bool _quickLaunchRecording;
        private bool _fileDialogSearchEnabled;
        private bool _autoQuickSwitchEnabled;
        private FileDialogSearchHostSnapshot? _activeFileDialogSearchHost;
        private AutoQuickSwitchDirectorySource? _publishedFileDialogSearchSource;
        private bool _fileDialogSearchHostVisible;
        private long _fileDialogRecognitionWindowHandle;
        private long _fileDialogRecognitionStartedAt;
        private long _directorySourceRecognitionStartedAt;
        private long _directorySourceRecognitionWindowHandle;
        private PlaceFileDialogSearchWindowCommand? _fileDialogSearchPlacement;
        private PlaceTaskbarProgressOverlayCommand? _taskbarOverlayPlacement;
        private Guid? _taskbarOverlayCorrelationId;
        private string? _taskbarOverlayAppliedBounds;
        private ClipboardCaptureExclusions _captureExclusions = ClipboardCaptureExclusions.Create(
            ["KeePass.exe", "1Password.exe", "Bitwarden.exe"]);
        private readonly DesktopTrayController _desktopTray = new();
        private ClipboardCaptureLimits _captureLimits = ClipboardCaptureLimits.Default;

        public bool ClipboardHotKeyAvailable => _hotKeyRegistration?.IsAvailable == true;
        public bool QuickLaunchHotKeyAvailable => _quickLaunchTriggerEnabled && !_quickLaunchRecording &&
            (_quickLaunchTrigger.Kind == QuickLaunchTriggerKind.DoubleControl
                ? _quickLaunchKeyboardMonitor?.IsInstalled == true
                : _quickLaunchHotKeyRegistration?.IsAvailable == true);

        public EventWindow(
            ChannelWriter<WindowsPlatformEvent> events,
            ForegroundContextProvider foregroundContextProvider,
            IElevatedDirectoryNavigationChannel elevatedDirectoryNavigation,
            IPlatformLog log,
            ClipboardCaptureDropDiagnosticGate clipboardCaptureDropDiagnosticGate)
        {
            _events = events;
            _foregroundContextProvider = foregroundContextProvider;
            _elevatedDirectoryNavigation = elevatedDirectoryNavigation;
            _log = log;
            _clipboardCaptureDropDiagnosticGate = clipboardCaptureDropDiagnosticGate;
            _foregroundCallback = OnForegroundChanged;
            _fileDialogWindowCallback = OnFileDialogWindowChanged;
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
            _fileDialogCreateHook = InstallOptionalWindowEventHook(EventObjectCreate);
            _fileDialogDestroyHook = InstallOptionalWindowEventHook(EventObjectDestroy);
            _fileDialogShowHook = InstallOptionalWindowEventHook(EventObjectShow);
            _fileDialogHideHook = InstallOptionalWindowEventHook(EventObjectHide);
            _fileDialogLocationHook = InstallOptionalWindowEventHook(EventObjectLocationChange);
            _fileDialogNameHook = InstallOptionalWindowEventHook(EventObjectNameChange);
            _taskbarOverlayMoveSizeEndHook = InstallOptionalWindowEventHook(EventSystemMoveSizeEnd);
            SystemEvents.DisplaySettingsChanged += OnSystemDisplaySettingsChanged;
            SystemEvents.UserPreferenceChanged += OnSystemUserPreferenceChanged;

            _hotKeyRegistration = new ClipboardHotKeyRegistration(
                new NativeClipboardHotKeyRegistrar(() => Handle, ClipboardHotKeyId),
                new ClipboardHotKeyBinding(ModAlt, VkC, "Alt+C"));
            var hotKeyInitialization = _hotKeyRegistration.Initialize();
            if (!hotKeyInitialization.Success)
            {
                Console.Error.WriteLine(hotKeyInitialization.Error);
            }
            _quickLaunchHotKeyRegistration = new ClipboardHotKeyRegistration(
                new NativeClipboardHotKeyRegistrar(() => Handle, QuickLaunchHotKeyId),
                new ClipboardHotKeyBinding(ModAlt, VkSpace, "Alt+Space"));
            try
            {
                _quickLaunchKeyboardMonitor = new LowLevelKeyboardMonitor(
                    OnGlobalKeyDown,
                    OnGlobalKeyUp);
            }
            catch (Win32Exception error)
            {
                _quickLaunchTriggerEnabled = false;
                _log.Write("error", "quick_launch.keyboard_hook_failed", null,
                    new Dictionary<string, object?>
                    {
                        ["win32_error"] = error.NativeErrorCode,
                        ["message"] = error.Message
                    });
            }

            _ = NativeMethods.SetTimer(
                Handle,
                (UIntPtr)PointerMonitorTimerId,
                PointerMonitorIntervalMs,
                IntPtr.Zero);

            ProcessForeground(_foregroundContextProvider.Capture());
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

        public Task<WindowsCommandResult> EnqueueClipboardConfiguration(
            ConfigureClipboardCommand command,
            CancellationToken cancellationToken)
        {
            var completion = new TaskCompletionSource<WindowsCommandResult>(
                TaskCreationOptions.RunContinuationsAsynchronously);
            _configurationRequests.Enqueue(new ClipboardConfigurationRequest(
                command, cancellationToken, completion));
            if (!NativeMethods.PostMessage(Handle, WmConfigureClipboard, IntPtr.Zero, IntPtr.Zero))
            {
                completion.TrySetResult(new WindowsCommandResult(
                    false, "Failed to schedule clipboard configuration on the Windows thread."));
            }
            return completion.Task;
        }

        public Task<WindowsCommandResult> EnqueueClipboardHotKeyRecording(
            SetClipboardHotKeyRecordingCommand command,
            CancellationToken cancellationToken)
        {
            var completion = new TaskCompletionSource<WindowsCommandResult>(
                TaskCreationOptions.RunContinuationsAsynchronously);
            _hotKeyRecordingRequests.Enqueue(new ClipboardHotKeyRecordingRequest(
                command, cancellationToken, completion));
            if (!NativeMethods.PostMessage(
                    Handle, WmSetClipboardHotKeyRecording, IntPtr.Zero, IntPtr.Zero))
            {
                completion.TrySetResult(new WindowsCommandResult(
                    false, "Failed to schedule clipboard hotkey recording on the Windows thread."));
            }
            return completion.Task;
        }

        public Task<WindowsCommandResult> EnqueueQuickLaunchConfiguration(
            ConfigureQuickLaunchCommand command,
            CancellationToken cancellationToken)
        {
            var completion = new TaskCompletionSource<WindowsCommandResult>(
                TaskCreationOptions.RunContinuationsAsynchronously);
            _quickLaunchConfigurationRequests.Enqueue(new QuickLaunchConfigurationRequest(
                command, cancellationToken, completion));
            if (!NativeMethods.PostMessage(Handle, WmConfigureQuickLaunch, IntPtr.Zero, IntPtr.Zero))
            {
                completion.TrySetResult(new WindowsCommandResult(
                    false, "Failed to schedule quick-launch configuration on the Windows thread."));
            }
            return completion.Task;
        }

        public Task<WindowsCommandResult> EnqueueQuickLaunchHotKeyRecording(
            SetQuickLaunchHotKeyRecordingCommand command,
            CancellationToken cancellationToken)
        {
            var completion = new TaskCompletionSource<WindowsCommandResult>(
                TaskCreationOptions.RunContinuationsAsynchronously);
            _quickLaunchHotKeyRecordingRequests.Enqueue(new QuickLaunchHotKeyRecordingRequest(
                command, cancellationToken, completion));
            if (!NativeMethods.PostMessage(
                    Handle, WmSetQuickLaunchHotKeyRecording, IntPtr.Zero, IntPtr.Zero))
            {
                completion.TrySetResult(new WindowsCommandResult(
                    false, "Failed to schedule quick-launch hotkey recording on the Windows thread."));
            }
            return completion.Task;
        }

        public void RequestStop()
        {
            _ = NativeMethods.PostMessage(Handle, WmClose, IntPtr.Zero, IntPtr.Zero);
        }

        public Task<WindowsCommandResult> EnqueueDesktopShellConfiguration(
            ConfigureDesktopShellCommand command,
            CancellationToken cancellationToken)
        {
            var completion = new TaskCompletionSource<WindowsCommandResult>(
                TaskCreationOptions.RunContinuationsAsynchronously);
            _desktopShellConfigurationRequests.Enqueue(new DesktopShellConfigurationRequest(
                command, cancellationToken, completion));
            if (!NativeMethods.PostMessage(Handle, WmConfigureDesktopShell, IntPtr.Zero, IntPtr.Zero))
            {
                completion.TrySetResult(new WindowsCommandResult(
                    false,
                    "Failed to schedule desktop-shell configuration on the Windows thread.",
                    NativeErrorCode: Marshal.GetLastWin32Error()));
            }
            return completion.Task;
        }

        public void TrackFileDialogSearchPlacement(
            PlaceFileDialogSearchWindowCommand command)
        {
            Volatile.Write(ref _fileDialogSearchPlacement, command);
        }

        public void TrackTaskbarOverlayPlacement(
            PlaceTaskbarProgressOverlayCommand command,
            Guid correlationId,
            string? appliedBounds)
        {
            Volatile.Write(ref _taskbarOverlayPlacement, command);
            _taskbarOverlayCorrelationId = correlationId;
            _taskbarOverlayAppliedBounds = appliedBounds;
        }

        public void TrackTaskbarOverlayVisibility(
            SetTaskbarProgressOverlayVisibilityCommand command)
        {
            var placement = Volatile.Read(ref _taskbarOverlayPlacement);
            if (placement?.WindowHandle == command.WindowHandle)
            {
                Volatile.Write(
                    ref _taskbarOverlayPlacement,
                    placement with { Visible = command.Visible });
            }
        }

        public void ForgetTaskbarOverlayPlacement(long windowHandle)
        {
            var placement = Volatile.Read(ref _taskbarOverlayPlacement);
            if (placement?.WindowHandle != windowHandle ||
                !ReferenceEquals(
                    Interlocked.CompareExchange(ref _taskbarOverlayPlacement, null, placement),
                    placement))
            {
                return;
            }
            _taskbarOverlayCorrelationId = null;
            _taskbarOverlayAppliedBounds = null;
        }

        protected override void WndProc(ref Message message)
        {
            switch (message.Msg)
            {
                case WmExitSizeMove:
                    var rootWindow = NativeMethods.GetAncestor(Handle, 2 /* GA_ROOT */);
                    WindowsPlatformAdapter.ReapplyPendingCenteredResizeFor(rootWindow);
                    break;
                case WmTimer when message.WParam.ToInt64() == NativeMethods.CenteredResizeRetryTimerId:
                    NativeMethods.KillTimer(Handle, NativeMethods.CenteredResizeRetryTimerId);
                    var retryRoot = NativeMethods.GetAncestor(Handle, 2 /* GA_ROOT */);
                    WindowsPlatformAdapter.RetryPendingCenteredResize(retryRoot);
                    return;
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
                    if (_clipboardHotKeyDebouncer.TryAccept(Environment.TickCount64))
                    {
                        PublishPanelRequest();
                    }
                    return;
                case WmHotKey when message.WParam.ToInt32() == QuickLaunchHotKeyId:
                    PublishQuickLaunchPanelRequest();
                    return;
                case WmTimer when message.WParam.ToInt64() == RetryTimerId:
                    _ = NativeMethods.KillTimer(Handle, (UIntPtr)RetryTimerId);
                    TryPublishClipboardContent();
                    return;
                case WmTimer when message.WParam.ToInt64() == PointerMonitorTimerId:
                    MonitorPointerPress();
                    return;
                case WmTimer when message.WParam.ToInt64() == FileDialogRecognitionTimerId:
                    _ = NativeMethods.KillTimer(
                        Handle,
                        (UIntPtr)FileDialogRecognitionTimerId);
                    RetryFileDialogRecognition();
                    return;
                case WmTimer when message.WParam.ToInt64() == DirectorySourceRecognitionTimerId:
                    _ = NativeMethods.KillTimer(
                        Handle,
                        (UIntPtr)DirectorySourceRecognitionTimerId);
                    RetryDirectorySourceRecognition();
                    return;
                case WmFileDialogGeometryChanged:
                    if (_fileDialogGeometryUpdateGate.TryTake(out var geometryRoot))
                    {
                        FollowFileDialogSearchWindow(geometryRoot);
                    }
                    return;
                case WmTaskbarOverlayChanged:
                case WmSettingChange:
                case WmDisplayChange:
                    ReapplyTaskbarOverlayPlacement();
                    if (message.Msg == WmTaskbarOverlayChanged)
                    {
                        return;
                    }
                    break;
                case WmExecutePaste:
                    DrainPasteRequests();
                    return;
                case WmConfigureClipboard:
                    DrainClipboardConfigurationRequests();
                    return;
                case WmSetClipboardHotKeyRecording:
                    DrainClipboardHotKeyRecordingRequests();
                    return;
                case WmConfigureQuickLaunch:
                    DrainQuickLaunchConfigurationRequests();
                    return;
                case WmSetQuickLaunchHotKeyRecording:
                    DrainQuickLaunchHotKeyRecordingRequests();
                    return;
                case WmConfigureDesktopShell:
                    DrainDesktopShellConfigurationRequests();
                    return;
                case WmQuickLaunchGesture:
                    PublishQuickLaunchPanelRequest();
                    return;
                case WmDirectorySourceResolved:
                    DrainResolvedDirectorySources();
                    return;
                case WmFileDialogStructureChanged:
                    Interlocked.Exchange(
                        ref _fileDialogStructureRetryPosted,
                        0);
                    var changedRoot = message.LParam.ToInt64();
                    if (_fileDialogRecognitionRetry
                        .ShouldRetryForStructureChange(changedRoot))
                    {
                        RetryFileDialogRecognition();
                    }
                    else if ((_fileDialogSearchEnabled ||
                              _autoQuickSwitchEnabled) &&
                             NativeMethods.IsForegroundWindowForTarget(
                                 new IntPtr(changedRoot)))
                    {
                        ProcessForeground(
                            _foregroundContextProvider.Capture(
                                new IntPtr(changedRoot)));
                    }
                    return;
                case WmClose:
                    Stop();
                    return;
            }
            base.WndProc(ref message);
        }

        internal static bool HasUserClipboardContent(
            string? text,
            ClipboardImageData? imageData,
            IReadOnlyCollection<string> paths) =>
            imageData is not null ||
            paths.Count > 0 ||
            !string.IsNullOrWhiteSpace(text);

        private void TryPublishClipboardContent()
        {
            try
            {
                var source = _pendingClipboardSource ?? _foregroundContextProvider.Capture();
                if (_captureExclusions.Contains(source.ProcessName))
                {
                    _log.Write("info", "clipboard.capture_excluded", null,
                        new Dictionary<string, object?>
                        {
                            ["source_process_name"] = source.ProcessName
                        });
                    return;
                }
                var capturedAt = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
                var sequenceBefore = NativeMethods.GetClipboardSequenceNumber();
                var nativeFormats = NativeClipboardFormats.Capture(Handle).ToList();
                var paths = Clipboard.ContainsFileDropList()
                    ? Clipboard.GetFileDropList()
                        .Cast<string>()
                        .Where(static path => !string.IsNullOrWhiteSpace(path))
                        .Take(1024)
                        .ToArray()
                    : [];
                string? promotedFilePath = paths.Length == 1 ? paths[0] : null;
                ClipboardImageData? imageData = null;
                var stagingDirectory = Path.Combine(
                    Path.GetTempPath(), "NxTools.Next", "clipboard-staging");
                var promoted = ClipboardImagePromotion.TryPromote(paths, text: null, stagingDirectory);
                if (promoted is not null)
                {
                    imageData = promoted.Image;
                    paths = promoted.RemainingPaths;
                }
                else if (Clipboard.ContainsImage())
                {
                    using var image = Clipboard.GetImage();
                    if (image is not null)
                    {
                        Directory.CreateDirectory(stagingDirectory);
                        var path = Path.Combine(stagingDirectory, $"{Guid.NewGuid():N}.png");
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
                if (imageData is null)
                {
                    promoted = ClipboardImagePromotion.TryPromote(paths, text, stagingDirectory);
                    if (promoted is not null)
                    {
                        imageData = promoted.Image;
                        paths = promoted.RemainingPaths;
                    }
                }
                if (!string.IsNullOrEmpty(text))
                {
                    var textBytes = (ulong)Encoding.UTF8.GetByteCount(text);
                    if (!_captureLimits.AllowsText(textBytes))
                    {
                        LogCaptureLimitSkip("text", "max_text_bytes", textBytes, _captureLimits.MaxTextBytes);
                        text = null;
                    }
                }
                if (imageData is not null)
                {
                    var outputBytes = File.Exists(imageData.StagingPath)
                        ? (ulong)new FileInfo(imageData.StagingPath).Length
                        : 0;
                    var reason = _captureLimits.ImageRejectionReason(
                        imageData.Width, imageData.Height, outputBytes);
                    if (reason is not null)
                    {
                        LogCaptureLimitSkip(
                            "image",
                            reason,
                            reason == "max_image_pixels"
                                ? checked((ulong)imageData.Width * imageData.Height)
                                : outputBytes,
                            reason == "max_image_pixels"
                                ? _captureLimits.MaxImagePixels
                                : _captureLimits.MaxImageBytes);
                        if (promotedFilePath is not null && paths.Length == 0)
                        {
                            paths = [promotedFilePath];
                        }
                        TryDeleteStagedImage(imageData.StagingPath);
                        imageData = null;
                    }
                }

                if (HasUserClipboardContent(text, imageData, paths))
                {
                    var compatibilityFallbackAdded = TryAddImageCompatibilityFallbacks(
                        sequenceBefore,
                        imageData,
                        paths.Length,
                        text,
                        nativeFormats,
                        source);
                    _log.Write("info", "clipboard.capture_completed", null,
                        new Dictionary<string, object?>
                        {
                            ["sequence_before"] = sequenceBefore,
                            ["sequence_after"] = NativeMethods.GetClipboardSequenceNumber(),
                            ["image_compatibility_fallback_added"] = compatibilityFallbackAdded,
                            ["native_format_count"] = nativeFormats.Count,
                            ["has_image"] = imageData is not null,
                            ["file_count"] = paths.Length,
                            ["has_text"] = !string.IsNullOrEmpty(text),
                            ["source_process_name"] = source.ProcessName
                        });
                    QueueCapturedEvent(text, imageData, paths, nativeFormats, source, capturedAt);
                }
                else
                {
                    _log.Write("info", "clipboard.capture_skipped", null,
                        new Dictionary<string, object?>
                        {
                            ["reason"] = "no_content",
                            ["native_format_count"] = nativeFormats.Count
                        });
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
                    ["hresult"] = $"0x{error.HResult:X8}"
                });
            }
        }
        private void LogCaptureLimitSkip(string kind, string reason, ulong sizeBytesOrPixels, ulong limit)
        {
            _log.Write("info", "clipboard.capture_limit_skipped", null,
                new Dictionary<string, object?>
                {
                    ["kind"] = kind,
                    ["reason"] = reason,
                    ["size"] = sizeBytesOrPixels,
                    ["limit"] = limit
                });
        }

        private static void TryDeleteStagedImage(string path)
        {
            try
            {
                if (File.Exists(path))
                {
                    File.Delete(path);
                }
            }
            catch (IOException)
            {
            }
            catch (UnauthorizedAccessException)
            {
            }
        }


        private bool TryAddImageCompatibilityFallbacks(
            uint expectedSequence,
            ClipboardImageData? imageData,
            int fileCount,
            string? text,
            List<CapturedClipboardNativeFormat> nativeFormats,
            ForegroundContext source)
        {
            var hasText = !string.IsNullOrEmpty(text);
            if (imageData is null ||
                (!ClipboardImageCompatibility.RequiresPngFallback(
                     hasImage: true, hasText, fileCount, nativeFormats) &&
                 !ClipboardImageCompatibility.RequiresFileDropFallback(
                     hasImage: true, hasText, fileCount, nativeFormats) &&
                 !ClipboardImageCompatibility.RequiresCanonicalRewrite(
                     hasImage: true, hasText, fileCount, nativeFormats)))
            {
                return false;
            }
            var actualSequence = NativeMethods.GetClipboardSequenceNumber();
            if (actualSequence != expectedSequence)
            {
                _log.Write("info", "clipboard.image_fallback_skipped", null,
                    new Dictionary<string, object?>
                    {
                        ["reason"] = "clipboard_changed",
                        ["expected_sequence"] = expectedSequence,
                        ["actual_sequence"] = actualSequence
                    });
                return false;
            }

            var pngData = File.ReadAllBytes(imageData.StagingPath);
            var fallbackPath = ClipboardImageFallbackStore.Persist(imageData.StagingPath);
            _suppressNextClipboardUpdate = true;
            if (NativeClipboardFormats.TryRewriteWithImageFallbacks(
                    Handle,
                    nativeFormats,
                    pngData,
                    fallbackPath,
                    expectedSequence,
                    out var rewrittenFormats,
                    out var error,
                    out var hresult))
            {
                nativeFormats.Clear();
                nativeFormats.AddRange(rewrittenFormats);
                _log.Write("info", "clipboard.image_fallback_added", null,
                    new Dictionary<string, object?>
                    {
                        ["sequence_before"] = expectedSequence,
                        ["sequence_after"] = NativeMethods.GetClipboardSequenceNumber(),
                        ["png_bytes"] = pngData.Length,
                        ["file_drop_added"] = true,
                        ["source_process_name"] = source.ProcessName
                    });
                return true;
            }

            _suppressNextClipboardUpdate = false;
            var clipboardChanged = string.Equals(
                error,
                "clipboard_changed",
                StringComparison.Ordinal);
            _log.Write(
                clipboardChanged ? "info" : "warning",
                clipboardChanged
                    ? "clipboard.image_fallback_skipped"
                    : "clipboard.image_fallback_failed",
                null,
                new Dictionary<string, object?>
                {
                    ["hresult"] = $"0x{hresult:X8}",
                    ["reason"] = clipboardChanged ? "clipboard_changed" : "native_format_rewrite_failed",
                    ["source_process_name"] = source.ProcessName
                });
            return false;
        }

        private void QueueCapturedEvent(
            string? text,
            ClipboardImageData? imageData,
            string[] paths,
            IReadOnlyList<CapturedClipboardNativeFormat> nativeFormats,
            ForegroundContext source,
            long capturedAt)
        {
            var pending = Interlocked.Increment(ref _capturePublishPending);
            if (pending > MaxClipboardPublishTail)
            {
                Interlocked.Decrement(ref _capturePublishPending);
                if (imageData is not null)
                {
                    TryDeleteStagedImage(imageData.StagingPath);
                }
                if (_clipboardCaptureDropDiagnosticGate.TryAcquire(out var suppressedDropCount))
                {
                    _log.Write("warning", "clipboard.capture_event_dropped", null,
                        new Dictionary<string, object?>
                        {
                            ["reason"] = "publication_tail_full",
                            ["suppressed_drop_count"] = suppressedDropCount
                        });
                }
                return;
            }
            _capturePublishTail = _capturePublishTail.ContinueWith(
                _ =>
                {
                    string? sourceIconPath = null;
                    ClipboardCapturedEvent? captured = null;
                    try
                    {
                        sourceIconPath = _foregroundContextProvider.TryStageProcessIcon(
                            source.ProcessId,
                            out var iconError);
                        if (iconError is not null)
                        {
                            _log.Write("warning", "clipboard.source_icon_failed", null,
                                new Dictionary<string, object?>
                                {
                                    ["source_process_id"] = source.ProcessId,
                                    ["source_process_name"] = source.ProcessName,
                                    ["reason"] = "source_icon_stage_failed"
                                });
                        }
                        var stagedFormats = StageNativeFormats(nativeFormats);
                        captured = new ClipboardCapturedEvent(
                            text,
                            imageData,
                            paths,
                            sourceIconPath,
                            source,
                            capturedAt,
                            stagedFormats);
                        WindowsPlatformAdapter.TryPublishClipboardCapturedEvent(
                            _events,
                            captured,
                            _log,
                            _clipboardCaptureDropDiagnosticGate);
                    }
                    catch (Exception error)
                    {
                        if (captured is not null)
                        {
                            WindowsPlatformAdapter.CleanupClipboardCapturedStaging(captured);
                        }
                        else
                        {
                            if (imageData is not null)
                            {
                                TryDeleteStagedImage(imageData.StagingPath);
                            }
                            if (!string.IsNullOrWhiteSpace(sourceIconPath))
                            {
                                TryDeleteStagedImage(sourceIconPath);
                            }
                        }
                        PlatformExceptionDiagnostics.Record(
                            _log,
                            "clipboard.capture_publish_failed",
                            error);
                    }
                    finally
                    {
                        Interlocked.Decrement(ref _capturePublishPending);
                    }
                },
                CancellationToken.None,
                TaskContinuationOptions.None,
                TaskScheduler.Default);
        }

        private IReadOnlyList<ClipboardNativeFormatData> StageNativeFormats(
            IReadOnlyList<CapturedClipboardNativeFormat> formats)
        {
            var staged = new List<ClipboardNativeFormatData>(formats.Count);
            var directory = Path.Combine(
                Path.GetTempPath(), "NxTools.Next", "clipboard-format-staging");
            Directory.CreateDirectory(directory);
            foreach (var format in formats)
            {
                try
                {
                    var path = Path.Combine(directory, $"{Guid.NewGuid():N}.bin");
                    File.WriteAllBytes(path, format.Data);
                    staged.Add(new ClipboardNativeFormatData(
                        format.FormatId,
                        format.FormatName,
                        path,
                        checked((ulong)format.Data.LongLength),
                        format.SortOrder));
                }
                catch (Exception error)
                {
                    _log.Write("warning", "clipboard.native_format_stage_failed", null,
                        new Dictionary<string, object?>
                        {
                            ["format_id"] = format.FormatId,
                            ["size_bytes"] = format.Data.LongLength,
                            ["exception_type"] = error.GetType().FullName,
                            ["hresult"] = $"0x{error.HResult:X8}"
                        });
                }
            }
            return staged;
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
            try
            {
                if (window != IntPtr.Zero)
                {
                    ProcessForeground(_foregroundContextProvider.Capture(window));
                }
            }
            catch (Exception error)
            {
                PlatformExceptionDiagnostics.Record(
                    _log,
                    "platform.foreground_callback_failed",
                    error);
            }
        }

        private void ProcessForeground(ForegroundContext context)
        {
            if (FileDialogSearchHost.CanReuseForeground(
                    _fileDialogSearchEnabled,
                    _fileDialogSearchHostVisible,
                    _activeFileDialogSearchHost,
                    context.WindowHandle))
            {
                return;
            }
            PublishForeground(context);
            if ((_fileDialogSearchEnabled || _autoQuickSwitchEnabled) &&
                FileDialogWindowAdapter.IsPotentialHost(context) &&
                _fileDialogRecognitionWindowHandle != context.WindowHandle)
            {
                _fileDialogRecognitionWindowHandle = context.WindowHandle;
                _fileDialogRecognitionStartedAt = Stopwatch.GetTimestamp();
            }
            var fileDialog = FileDialogWindowAdapter.TryCapture(context);
            ProcessFileDialogSearchHost(context, fileDialog);

            if (fileDialog is not null)
            {
                CompleteFileDialogRecognition(fileDialog.WindowHandle);
                ProcessRecognizedFileDialog(fileDialog);
                return;
            }

            var directoryAttempt =
                DirectoryNavigationAdapter.TryCaptureLocalAttempt(context);
            var directorySource = directoryAttempt.Context;
            if (directorySource is not null &&
                !directorySource.Host.Equals(
                    DirectoryNavigationAdapter.FileDialogHost,
                    StringComparison.Ordinal))
            {
                CancelFileDialogRecognition();
                var displayName = DirectorySourceDisplayName(
                    context.ProcessName,
                    directorySource);
                _autoQuickSwitch.ObserveDirectorySource(
                    directorySource,
                    displayName);
                if (directorySource.Host.Equals(
                        DirectoryNavigationAdapter.ExplorerHost,
                        StringComparison.Ordinal) &&
                    string.IsNullOrWhiteSpace(
                        directorySource.CurrentDirectory))
                {
                    QueueExplorerDirectoryResolution(
                        directorySource.WindowHandle,
                        context.ProcessName);
                }
                CancelDirectorySourceRecognition();
                return;
            }
            if ((_fileDialogSearchEnabled || _autoQuickSwitchEnabled) &&
                directoryAttempt.Failure?.Fatal != true &&
                ThirdPartyFileManagerAdapter.IsPotentialHost(context))
            {
                _autoQuickSwitch.DiscardPendingSource();
                ScheduleDirectorySourceRecognition(context.WindowHandle);
                return;
            }

            if ((_fileDialogSearchEnabled || _autoQuickSwitchEnabled) &&
                FileDialogWindowAdapter.IsPotentialHost(context))
            {
                ScheduleFileDialogRecognition(context.WindowHandle);
                return;
            }

            CancelFileDialogRecognition();
            CancelDirectorySourceRecognition();
            if (!IsNxToolsDesktopProcess(context.ProcessName))
            {
                _autoQuickSwitch.ObserveOther();
            }
        }

        private void ScheduleDirectorySourceRecognition(long windowHandle)
        {
            var newTarget = _directorySourceRecognitionWindowHandle != windowHandle;
            if (!_directorySourceRecognitionRetry.Begin(windowHandle))
            {
                return;
            }
            if (newTarget)
            {
                _directorySourceRecognitionWindowHandle = windowHandle;
                _directorySourceRecognitionStartedAt = Stopwatch.GetTimestamp();
                _log.Write(
                    "info",
                    "quick_launch.directory_source_recognition_started",
                    fields: new Dictionary<string, object?>
                    {
                        ["target_window_handle"] = windowHandle,
                        ["max_attempts"] = DirectorySourceRecognitionMaxAttempts
                    });
            }
            if (NativeMethods.SetTimer(
                    Handle,
                    (UIntPtr)DirectorySourceRecognitionTimerId,
                    FileDialogRecognitionIntervalMs,
                    IntPtr.Zero) == UIntPtr.Zero)
            {
                FailDirectorySourceRecognition(
                    "timer_unavailable",
                    Marshal.GetLastWin32Error());
            }
        }

        private void RetryDirectorySourceRecognition()
        {
            var foreground = _foregroundContextProvider.Capture();
            if (!_directorySourceRecognitionRetry.TryBeginPendingAttempt(
                    out var windowHandle))
            {
                _autoQuickSwitch.DiscardPendingSource();
                return;
            }
            var targetContext = foreground.WindowHandle == windowHandle
                ? foreground
                : _foregroundContextProvider.Capture(new IntPtr(windowHandle));
            var attempt = DirectoryNavigationAdapter.TryCaptureLocalAttempt(
                targetContext);
            var source = attempt.Context;
            if (source is not null &&
                source.Host.Equals(
                    DirectoryNavigationAdapter.ThirdPartyFileManagerHost,
                    StringComparison.Ordinal))
            {
                var displayName = DirectorySourceDisplayName(
                    targetContext.ProcessName,
                    source);
                _autoQuickSwitch.ObserveDirectorySource(source, displayName);
                var publishedFields = new Dictionary<string, object?>
                {
                    ["target_window_handle"] = windowHandle
                };
                foreach (var pair in DirectoryNavigationTarget.DiagnosticFields(
                             source.CurrentDirectory))
                {
                    publishedFields[$"source_{pair.Key}"] = pair.Value;
                }
                _log.Write(
                    "info",
                    "quick_launch.directory_source_recognition_published",
                    fields: publishedFields);
                CompleteDirectorySourceRecognition(windowHandle);
                if (foreground.WindowHandle != windowHandle)
                {
                    var fileDialog = FileDialogWindowAdapter.TryCapture(foreground);
                    if (fileDialog is not null)
                    {
                        ProcessFileDialogSearchHost(foreground, fileDialog);
                        ProcessRecognizedFileDialog(fileDialog);
                    }
                }
                return;
            }
            if (attempt.Failure?.Fatal == true)
            {
                FailDirectorySourceRecognition(
                    "fatal_capture_failure",
                    attempt.Failure.NativeErrorCode);
                _autoQuickSwitch.DiscardPendingSource();
                return;
            }
            if (_directorySourceRecognitionRetry.HasPendingAttempt(windowHandle))
            {
                ScheduleDirectorySourceRecognition(windowHandle);
                return;
            }
            FailDirectorySourceRecognition("attempt_budget_exhausted", null);
            _autoQuickSwitch.DiscardPendingSource();
        }

        private void CompleteDirectorySourceRecognition(long windowHandle)
        {
            var attempts = _directorySourceRecognitionRetry.AttemptCount;
            var elapsed = _directorySourceRecognitionStartedAt == 0
                ? 0
                : Stopwatch.GetElapsedTime(_directorySourceRecognitionStartedAt)
                    .TotalMilliseconds;
            _log.Write(
                "info",
                "quick_launch.directory_source_recognition_completed",
                fields: new Dictionary<string, object?>
                {
                    ["target_window_handle"] = windowHandle,
                    ["attempts"] = attempts,
                    ["elapsed_ms"] = elapsed
                });
            _directorySourceRecognitionRetry.Complete(windowHandle);
            _ = NativeMethods.KillTimer(
                Handle,
                (UIntPtr)DirectorySourceRecognitionTimerId);
            _directorySourceRecognitionStartedAt = 0;
            _directorySourceRecognitionWindowHandle = 0;
        }

        private void FailDirectorySourceRecognition(
            string reason,
            int? nativeErrorCode)
        {
            var windowHandle = _directorySourceRecognitionWindowHandle;
            if (windowHandle != 0)
            {
                var attempts = _directorySourceRecognitionRetry.AttemptCount;
                var elapsed = _directorySourceRecognitionStartedAt == 0
                    ? 0
                    : Stopwatch.GetElapsedTime(_directorySourceRecognitionStartedAt)
                        .TotalMilliseconds;
                _log.Write(
                    "warning",
                    "quick_launch.directory_source_recognition_failed",
                    fields: new Dictionary<string, object?>
                    {
                        ["target_window_handle"] = windowHandle,
                        ["attempts"] = attempts,
                        ["elapsed_ms"] = elapsed,
                        ["reason"] = reason,
                        ["native_error_code"] = nativeErrorCode
                    });
            }
            CancelDirectorySourceRecognition();
            _autoQuickSwitch.DiscardPendingSource();
        }

        private void CancelDirectorySourceRecognition()
        {
            _directorySourceRecognitionRetry.Cancel();
            _ = NativeMethods.KillTimer(
                Handle,
                (UIntPtr)DirectorySourceRecognitionTimerId);
            _directorySourceRecognitionStartedAt = 0;
            _directorySourceRecognitionWindowHandle = 0;
        }

        private void ProcessRecognizedFileDialog(FileDialogContext fileDialog)
        {
            var candidate = _autoQuickSwitch.ObserveFileDialog(
                new DirectoryNavigationContext(
                    DirectoryNavigationAdapter.FileDialogHost,
                    fileDialog.WindowHandle,
                    fileDialog.CurrentDirectory));
            if (candidate is not null)
            {
                QueueAutoQuickSwitch(candidate);
            }
        }

        private void QueueExplorerDirectoryResolution(
            long windowHandle,
            string processName)
        {
            _ = Task.Run(() =>
            {
                if (!ExplorerWindowAdapter.TryGetCurrentDirectory(
                        windowHandle,
                        out var directory,
                        out _)
                    || string.IsNullOrWhiteSpace(directory))
                {
                    return;
                }
                var context = new DirectoryNavigationContext(
                    DirectoryNavigationAdapter.ExplorerHost,
                    windowHandle,
                    directory);
                _resolvedDirectorySources.Enqueue((
                    windowHandle,
                    directory,
                    DirectorySourceDisplayName(processName, context)));
                _ = NativeMethods.PostMessage(
                    Handle,
                    WmDirectorySourceResolved,
                    IntPtr.Zero,
                    IntPtr.Zero);
            });
        }

        private void DrainResolvedDirectorySources()
        {
            var changed = false;
            while (_resolvedDirectorySources.TryDequeue(out var source))
            {
                changed |= _autoQuickSwitch.ResolveDirectorySource(
                    source.WindowHandle,
                    source.Directory,
                    source.DisplayName);
            }
            if (changed &&
                _fileDialogSearchHostVisible &&
                _activeFileDialogSearchHost is { } snapshot)
            {
                PublishFileDialogSearchHost(snapshot, force: true);
            }
        }

        private static string DirectorySourceDisplayName(
            string processName,
            DirectoryNavigationContext source)
        {
            var provider = source.Host switch
            {
                DirectoryNavigationAdapter.ExplorerHost => "File Explorer",
                DirectoryNavigationAdapter.DesktopHost => "Desktop",
                DirectoryNavigationAdapter.ThirdPartyFileManagerHost =>
                    Path.GetFileNameWithoutExtension(processName)
                        .ToLowerInvariant() switch
                    {
                        "dopus" => "Directory Opus",
                        "totalcmd" or "totalcmd64" => "Total Commander",
                        "onecommander" => "OneCommander",
                        "files" => "Files",
                        _ => "File Manager"
                    },
                _ => "File Manager"
            };
            var directory = source.CurrentDirectory?
                .TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
            var folder = string.IsNullOrWhiteSpace(directory)
                ? null
                : Path.GetFileName(directory);
            if (string.IsNullOrWhiteSpace(folder))
            {
                folder = directory;
            }
            return string.IsNullOrWhiteSpace(folder)
                ? provider
                : $"{folder} ({provider})";
        }

        private void ScheduleFileDialogRecognition(long windowHandle)
        {
            if (!_fileDialogRecognitionRetry.Begin(windowHandle))
            {
                return;
            }
            if (_fileDialogRecognitionWindowHandle != windowHandle)
            {
                _fileDialogRecognitionWindowHandle = windowHandle;
                _fileDialogRecognitionStartedAt = Stopwatch.GetTimestamp();
            }
            if (NativeMethods.SetTimer(
                    Handle,
                    (UIntPtr)FileDialogRecognitionTimerId,
                    FileDialogRecognitionIntervalMs,
                    IntPtr.Zero) == UIntPtr.Zero)
            {
                _fileDialogRecognitionRetry.Cancel();
                _autoQuickSwitch.ObserveOther();
            }
        }

        private void RetryFileDialogRecognition()
        {
            var foreground = _foregroundContextProvider.Capture();
            if (!_fileDialogRecognitionRetry.TryBeginAttempt(
                    foreground.WindowHandle,
                    out var windowHandle))
            {
                if (windowHandle != 0 &&
                    _fileDialogRecognitionRetry.HasPendingAttempt(windowHandle))
                {
                    ScheduleFileDialogRecognition(windowHandle);
                }
                return;
            }
            var targetContext = foreground.WindowHandle == windowHandle
                ? foreground
                : _foregroundContextProvider.Capture(new IntPtr(windowHandle));
            var fileDialog = FileDialogWindowAdapter.TryCapture(targetContext);
            if (fileDialog is not null)
            {
                CompleteFileDialogRecognition(windowHandle);
                ProcessFileDialogSearchHost(foreground, fileDialog);
                ProcessRecognizedFileDialog(fileDialog);
                return;
            }
            if (_fileDialogRecognitionRetry.HasPendingAttempt(windowHandle))
            {
                ScheduleFileDialogRecognition(windowHandle);
                return;
            }
            _autoQuickSwitch.ObserveOther();
        }

        private void CompleteFileDialogRecognition(long windowHandle)
        {
            _fileDialogRecognitionRetry.Complete(windowHandle);
            if (_fileDialogRecognitionWindowHandle == windowHandle)
            {
                _log.Write(
                    "info",
                    "quick_launch.file_dialog_recognized",
                    fields: new Dictionary<string, object?>
                    {
                        ["window_handle"] = windowHandle,
                        ["elapsed_ms"] = Stopwatch
                            .GetElapsedTime(_fileDialogRecognitionStartedAt)
                            .TotalMilliseconds
                    });
                _fileDialogRecognitionWindowHandle = 0;
                _fileDialogRecognitionStartedAt = 0;
            }
            _ = NativeMethods.KillTimer(
                Handle,
                (UIntPtr)FileDialogRecognitionTimerId);
        }

        private void CancelFileDialogRecognition()
        {
            _fileDialogRecognitionRetry.Cancel();
            _fileDialogRecognitionWindowHandle = 0;
            _fileDialogRecognitionStartedAt = 0;
            _ = NativeMethods.KillTimer(
                Handle,
                (UIntPtr)FileDialogRecognitionTimerId);
        }

        private IntPtr InstallOptionalWindowEventHook(uint eventType)
        {
            var hook = NativeMethods.SetWinEventHook(
                eventType,
                eventType,
                IntPtr.Zero,
                _fileDialogWindowCallback,
                0,
                0,
                WineventOutOfContext);
            if (hook == IntPtr.Zero)
            {
                _log.Write("warning", "quick_launch.file_dialog_window_hook_failed", null,
                    new Dictionary<string, object?>
                    {
                        ["event_type"] = eventType,
                        ["win32_error"] = Marshal.GetLastWin32Error()
                    });
            }
            return hook;
        }

        private void ProcessFileDialogSearchHost(
            ForegroundContext foreground,
            FileDialogContext? fileDialog)
        {
            if (!_fileDialogSearchEnabled)
            {
                return;
            }
            if (fileDialog is not null)
            {
                PublishFileDialogSearchHost(fileDialog);
                return;
            }
            if (IsNxToolsDesktopProcess(foreground.ProcessName))
            {
                return;
            }
            HideFileDialogSearchHost();
        }

        private static bool IsNxToolsDesktopProcess(string processName)
        {
            var name = Path.GetFileNameWithoutExtension(processName);
            return name.Equals("NxTools", StringComparison.OrdinalIgnoreCase) ||
                name.Equals("nxtools-desktop", StringComparison.OrdinalIgnoreCase);
        }

        private void PublishFileDialogSearchHost(FileDialogContext fileDialog)
        {
            var snapshot = FileDialogSearchHost.TryCapture(fileDialog);
            if (snapshot is null)
            {
                HideFileDialogSearchHost();
                return;
            }
            PublishFileDialogSearchHost(snapshot);
        }

        private void PublishFileDialogSearchHost(
            FileDialogSearchHostSnapshot snapshot,
            bool force = false)
        {
            var source = _autoQuickSwitch.LatestSource;
            if (_fileDialogSearchHostVisible &&
                _activeFileDialogSearchHost == snapshot &&
                _publishedFileDialogSearchSource == source &&
                !force)
            {
                return;
            }
            _activeFileDialogSearchHost = snapshot;
            _publishedFileDialogSearchSource = source;
            _fileDialogSearchHostVisible = true;
            _events.TryWrite(new QuickLaunchFileDialogSearchHostChangedEvent(
                snapshot.WindowHandle,
                snapshot.CurrentDirectory,
                snapshot.Left,
                snapshot.Top,
                snapshot.Width,
                snapshot.Height,
                snapshot.MonitorDpi,
                true,
                DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(),
                source));
        }

        private void HideFileDialogSearchHost(bool forget = false)
        {
            var snapshot = _activeFileDialogSearchHost;
            if (snapshot is null)
            {
                return;
            }
            if (_fileDialogSearchHostVisible)
            {
                _events.TryWrite(new QuickLaunchFileDialogSearchHostChangedEvent(
                    snapshot.WindowHandle,
                    snapshot.CurrentDirectory,
                    snapshot.Left,
                    snapshot.Top,
                    snapshot.Width,
                    snapshot.Height,
                    snapshot.MonitorDpi,
                    false,
                    DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(),
                    _autoQuickSwitch.LatestSource));
            }
            _fileDialogSearchHostVisible = false;
            if (forget)
            {
                _fileDialogGeometryUpdateGate.Cancel(snapshot.WindowHandle);
                var placement = Volatile.Read(ref _fileDialogSearchPlacement);
                if (placement?.OwnerWindowHandle == snapshot.WindowHandle)
                {
                    AnchoredWindowPlacement.Forget(new IntPtr(placement.WindowHandle));
                    Volatile.Write(ref _fileDialogSearchPlacement, null);
                }
                _activeFileDialogSearchHost = null;
                _publishedFileDialogSearchSource = null;
            }
        }

        private void OnFileDialogWindowChanged(
            IntPtr hook,
            uint eventType,
            IntPtr window,
            int objectId,
            int childId,
            uint eventThread,
            uint eventTime)
        {
            try
            {
                OnFileDialogWindowChangedCore(
                    hook,
                    eventType,
                    window,
                    objectId,
                    childId,
                    eventThread,
                    eventTime);
            }
            catch (Exception error)
            {
                PlatformExceptionDiagnostics.Record(
                    _log,
                    "platform.file_dialog_callback_failed",
                    error);
            }
        }

        private void OnFileDialogWindowChangedCore(
            IntPtr hook,
            uint eventType,
            IntPtr window,
            int objectId,
            int childId,
            uint eventThread,
            uint eventTime)
        {
            if (eventType == EventSystemMoveSizeEnd)
            {
                var overlay = Volatile.Read(ref _taskbarOverlayPlacement);
                if (overlay is not null && window.ToInt64() == overlay.WindowHandle)
                {
                    _events.TryWrite(new TaskbarOverlayEditGestureFinishedEvent(overlay.WindowHandle));
                }
                return;
            }
            var rootWindowHandle = NativeMethods.GetRootWindowHandle(window);
            if (TaskbarOverlayPlacement.IsTaskbarWindow(window) ||
                TaskbarOverlayPlacement.IsTaskbarWindow(new IntPtr(rootWindowHandle)))
            {
                _ = NativeMethods.PostMessage(
                    Handle,
                    WmTaskbarOverlayChanged,
                    IntPtr.Zero,
                    IntPtr.Zero);
            }
            if (eventType == EventObjectDestroy &&
                objectId == ObjIdWindow &&
                window.ToInt64() == _directorySourceRecognitionWindowHandle)
            {
                FailDirectorySourceRecognition("target_destroyed", null);
                return;
            }
            if ((eventType is EventObjectCreate or EventObjectShow) &&
                (_fileDialogRecognitionRetry.ShouldRetryForStructureChange(
                     rootWindowHandle) ||
                 ((_fileDialogSearchEnabled || _autoQuickSwitchEnabled) &&
                  NativeMethods.IsForegroundWindowForTarget(
                      new IntPtr(rootWindowHandle)))) &&
                Interlocked.Exchange(
                    ref _fileDialogStructureRetryPosted,
                    1) == 0)
            {
                if (!NativeMethods.PostMessage(
                        Handle,
                        WmFileDialogStructureChanged,
                        IntPtr.Zero,
                        new IntPtr(rootWindowHandle)))
                {
                    Interlocked.Exchange(
                        ref _fileDialogStructureRetryPosted,
                        0);
                }
                return;
            }
            var snapshot = _activeFileDialogSearchHost;
            if (!_fileDialogSearchEnabled ||
                snapshot is null ||
                rootWindowHandle != snapshot.WindowHandle)
            {
                return;
            }
            if ((eventType is EventObjectDestroy or EventObjectHide) &&
                objectId == ObjIdWindow &&
                window.ToInt64() == snapshot.WindowHandle)
            {
                HideFileDialogSearchHost(forget: eventType == EventObjectDestroy);
                return;
            }
            if (eventType == EventObjectLocationChange)
            {
                if (_fileDialogGeometryUpdateGate.TrySchedule(
                        snapshot.WindowHandle,
                        rootWindowHandle,
                        window.ToInt64(),
                        objectId,
                        eventType) &&
                    !NativeMethods.PostMessage(
                        Handle,
                        WmFileDialogGeometryChanged,
                        IntPtr.Zero,
                        new IntPtr(snapshot.WindowHandle)))
                {
                    _fileDialogGeometryUpdateGate.Cancel(snapshot.WindowHandle);
                }
                return;
            }
            if (eventType == EventObjectNameChange)
            {
                var foreground = _foregroundContextProvider.Capture(
                    new IntPtr(snapshot.WindowHandle));
                var dialog = FileDialogWindowAdapter.TryCapture(foreground);
                if (dialog is not null)
                {
                    PublishFileDialogSearchHost(dialog);
                }
            }
        }

        private void ReapplyTaskbarOverlayPlacement()
        {
            var placement = Volatile.Read(ref _taskbarOverlayPlacement);
            if (placement is null)
            {
                return;
            }
            var result = TaskbarOverlayPlacement.Place(placement);
            if (result.Success && result.Data == _taskbarOverlayAppliedBounds)
            {
                return;
            }
            _taskbarOverlayAppliedBounds = result.Data;
            _log.Write(
                result.Success ? "info" : "warning",
                result.Success
                    ? "download.taskbar_overlay.repositioned"
                    : "download.taskbar_overlay.reposition_failed",
                _taskbarOverlayCorrelationId,
                new Dictionary<string, object?>
                {
                    ["window_handle"] = placement.WindowHandle,
                    ["applied_bounds"] = result.Data,
                    ["error"] = result.Error
                });
            if (!result.Success)
            {
                ForgetTaskbarOverlayPlacement(placement.WindowHandle);
            }
        }

        private void OnSystemDisplaySettingsChanged(object? sender, EventArgs eventArgs) =>
            RequestTaskbarOverlayReposition();

        private void OnSystemUserPreferenceChanged(
            object? sender,
            UserPreferenceChangedEventArgs eventArgs) =>
            RequestTaskbarOverlayReposition();

        private void RequestTaskbarOverlayReposition()
        {
            _ = NativeMethods.PostMessage(
                Handle,
                WmTaskbarOverlayChanged,
                IntPtr.Zero,
                IntPtr.Zero);
        }

        private void FollowFileDialogSearchWindow(long rootWindowHandle)
        {
            var snapshot = _activeFileDialogSearchHost;
            var placement = Volatile.Read(ref _fileDialogSearchPlacement);
            if (!_fileDialogSearchEnabled ||
                !_fileDialogSearchHostVisible ||
                snapshot is null ||
                snapshot.WindowHandle != rootWindowHandle ||
                placement is null ||
                placement.OwnerWindowHandle != rootWindowHandle)
            {
                return;
            }
            var result = AnchoredWindowPlacement.Follow(
                new IntPtr(placement.WindowHandle),
                new IntPtr(placement.OwnerWindowHandle),
                placement.VisibleWidth,
                placement.VisibleHeight,
                placement.BottomOverlap,
                out var ownerVisible);
            if (!result.Success)
            {
                _log.Write(
                    "warning",
                    "quick_launch.file_dialog_search_follow_failed",
                    fields: new Dictionary<string, object?>
                    {
                        ["window_handle"] = placement.WindowHandle,
                        ["owner_window_handle"] = placement.OwnerWindowHandle,
                        ["error"] = result.Error
                    });
                return;
            }
            var refreshed = FileDialogSearchHost.RefreshGeometry(snapshot, ownerVisible);
            if (refreshed.MonitorDpi != snapshot.MonitorDpi)
            {
                PublishFileDialogSearchHost(refreshed);
            }
            else
            {
                _activeFileDialogSearchHost = refreshed;
            }
        }

        private void QueueAutoQuickSwitch(AutoQuickSwitchCandidate candidate)
        {
            var requestedAtUnixMs = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
            _ = Task.Run(() =>
            {
                if (!_autoQuickSwitch.IsCurrent(candidate))
                {
                    return;
                }
                var sourceDirectory = candidate.Source.CurrentDirectory;
                string? captureError = null;
                if (string.IsNullOrWhiteSpace(sourceDirectory) &&
                    candidate.Source.Host.Equals(
                        DirectoryNavigationAdapter.ExplorerHost,
                        StringComparison.Ordinal) &&
                    ExplorerWindowAdapter.TryGetCurrentDirectory(
                        candidate.SourceWindowHandle,
                        out var resolvedDirectory,
                        out captureError))
                {
                    sourceDirectory = resolvedDirectory;
                    if (!string.IsNullOrWhiteSpace(sourceDirectory))
                    {
                        _resolvedDirectorySources.Enqueue((
                            candidate.SourceWindowHandle,
                            sourceDirectory,
                            DirectorySourceDisplayName(
                                "explorer",
                                candidate.Source with
                                {
                                    CurrentDirectory = sourceDirectory
                                })));
                        _ = NativeMethods.PostMessage(
                            Handle,
                            WmDirectorySourceResolved,
                            IntPtr.Zero,
                            IntPtr.Zero);
                    }
                }
                if (string.IsNullOrWhiteSpace(sourceDirectory))
                {
                    _log.Write("warning", "quick_launch.auto_switch_source_failed", null,
                        new Dictionary<string, object?>
                        {
                            ["source_window_handle"] = candidate.SourceWindowHandle,
                            ["target_window_handle"] = candidate.Target.WindowHandle,
                            ["error"] = captureError
                        });
                    return;
                }
                if (!_autoQuickSwitch.IsCurrent(candidate) ||
                    !NativeMethods.IsForegroundWindowForTarget(
                        new IntPtr(candidate.Target.WindowHandle)))
                {
                    return;
                }
                _events.TryWrite(new QuickLaunchAutoSwitchRequestedEvent(
                    candidate.Target,
                    sourceDirectory,
                    requestedAtUnixMs));
            });
        }

        private void PublishForeground(ForegroundContext context)
        {
            _events.TryWrite(new ForegroundChangedEvent(context));
        }

        private void MonitorPointerPress()
        {
            var primaryDown = NativeMethods.IsPointerButtonDown(0x01);
            var secondaryDown = NativeMethods.IsPointerButtonDown(0x02);
            var pressed = (primaryDown && !_primaryPointerDown) ||
                          (secondaryDown && !_secondaryPointerDown);
            _primaryPointerDown = primaryDown;
            _secondaryPointerDown = secondaryDown;
            if (!pressed)
            {
                return;
            }

            var sampledAtUnixMs = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
            var eventSequence = NextPlatformEventSequence();
            var target = NativeMethods.GetPointerTarget();
            _events.TryWrite(new GlobalPointerPressedEvent(
                target.WindowHandle,
                target.RootWindowHandle,
                target.ProcessId,
                sampledAtUnixMs,
                eventSequence));
        }

        private void PublishPanelRequest()
        {
            var target = _foregroundContextProvider.Capture();
            var cursor = NativeMethods.GetCursorLocation();
            var requestedAtUnixMs = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
            var eventSequence = NextPlatformEventSequence();
            _events.TryWrite(new ClipboardPanelRequestedEvent(
                target,
                cursor.Position.X,
                cursor.Position.Y,
                cursor.DisplayIndex,
                cursor.Dpi,
                requestedAtUnixMs,
                eventSequence));
        }

        private void PublishQuickLaunchPanelRequest()
        {
            var target = _foregroundContextProvider.Capture();
            var cursor = NativeMethods.GetCursorLocation();
            var requestedAtUnixMs = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
            var deferDirectoryCapture =
                DirectoryNavigationAdapter.RequiresElevatedHelper(target);
            var directoryNavigationContext = deferDirectoryCapture
                ? null
                : DirectoryNavigationAdapter.TryCaptureLocal(target);
            _events.TryWrite(new QuickLaunchPanelRequestedEvent(
                target,
                cursor.Position.X,
                cursor.Position.Y,
                cursor.DisplayIndex,
                cursor.Dpi,
                requestedAtUnixMs,
                directoryNavigationContext));
            if (directoryNavigationContext is null &&
                DirectoryNavigationAdapter.IsPotentialElevatedHost(target))
            {
                _ = Task.Run(() =>
                {
                    if (!IsQuickLaunchContextRequestCurrent(
                            requestedAtUnixMs,
                            DateTimeOffset.UtcNow.ToUnixTimeMilliseconds()))
                    {
                        return;
                    }
                    var resolved =
                        _elevatedDirectoryNavigation.TryCapture(target);
                    if (resolved is not null &&
                        IsQuickLaunchContextRequestCurrent(
                            requestedAtUnixMs,
                            DateTimeOffset.UtcNow.ToUnixTimeMilliseconds()))
                    {
                        _events.TryWrite(
                            new QuickLaunchDirectoryContextResolvedEvent(
                                target.WindowHandle,
                                requestedAtUnixMs,
                                resolved));
                    }
                });
            }
        }

        private void OnGlobalKeyDown(int virtualKey, uint timestamp)
        {
            if (!_quickLaunchTriggerEnabled || _quickLaunchRecording ||
                _quickLaunchTrigger.Kind != QuickLaunchTriggerKind.DoubleControl)
            {
                return;
            }

            bool triggered;
            if (virtualKey == VkLeftControl)
            {
                _rightControlDoubleTap.Reset();
                triggered = _leftControlDoubleTap.OnKeyDown(virtualKey, timestamp);
            }
            else if (virtualKey == VkRightControl)
            {
                _leftControlDoubleTap.Reset();
                triggered = _rightControlDoubleTap.OnKeyDown(virtualKey, timestamp);
            }
            else
            {
                _leftControlDoubleTap.Reset();
                _rightControlDoubleTap.Reset();
                return;
            }

            if (triggered)
            {
                // A low-level hook must return quickly. Defer foreground capture and IPC publication
                // to the existing Windows message loop instead of doing either inside the callback.
                _ = NativeMethods.PostMessage(Handle, WmQuickLaunchGesture, IntPtr.Zero, IntPtr.Zero);
            }
        }

        private void OnGlobalKeyUp(int virtualKey)
        {
            var triggered = _leftControlDoubleTap.OnKeyUp(virtualKey) |
                            _rightControlDoubleTap.OnKeyUp(virtualKey);
            if (triggered)
            {
                // The complete gesture has ended, so the new GPUI window will not inherit an
                // unmatched Ctrl key-up as its first input event.
                _ = NativeMethods.PostMessage(Handle, WmQuickLaunchGesture, IntPtr.Zero, IntPtr.Zero);
            }
        }

        private void ResetQuickLaunchGesture()
        {
            _leftControlDoubleTap.Reset();
            _rightControlDoubleTap.Reset();
        }

        /// Re-applies a centered resize that was swallowed by a drag/size
        /// modal loop (the confirmed-download window grow during a drag).
        public void ReapplyPendingCenteredResize()
        {
            var pending = _pendingCenteredResize;
            if (pending is null)
            {
                return;
            }
            _pendingCenteredResize = null;
            var (x, y, width, height) = pending.Value;
            _ = NativeMethods.SetWindowPos(
                Handle,
                IntPtr.Zero,
                x,
                y,
                width,
                height,
                0x0004 | 0x0010); // SWP_NOZORDER | SWP_NOACTIVATE
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

        private void DrainClipboardConfigurationRequests()
        {
            while (_configurationRequests.TryDequeue(out var request))
            {
                if (request.CancellationToken.IsCancellationRequested)
                {
                    request.Completion.TrySetCanceled(request.CancellationToken);
                    continue;
                }
                request.Completion.TrySetResult(ApplyClipboardConfiguration(request.Command));
            }
        }

        private void DrainClipboardHotKeyRecordingRequests()
        {
            while (_hotKeyRecordingRequests.TryDequeue(out var request))
            {
                if (request.CancellationToken.IsCancellationRequested)
                {
                    request.Completion.TrySetCanceled(request.CancellationToken);
                    continue;
                }
                var result = request.Command.Recording
                    ? _hotKeyRegistration.BeginRecording()
                    : _hotKeyRegistration.CancelRecording();
                request.Completion.TrySetResult(result);
            }
        }

        private WindowsCommandResult ApplyClipboardConfiguration(ConfigureClipboardCommand command)
        {
            var requestedExclusions = ClipboardCaptureExclusions.Create(command.ExcludedProcesses);
            var requestedLimits = ClipboardCaptureLimits.From(command);
            if (!ClipboardHotKeyBinding.TryCreate(command, out var binding, out var error))
            {
                return new WindowsCommandResult(false, error);
            }

            // Privacy exclusions do not depend on whether the requested global hotkey is available.
            _captureExclusions = requestedExclusions;

            var registration = _hotKeyRegistration.Apply(binding);
            if (!registration.Success)
            {
                return registration;
            }
            _captureLimits = requestedLimits;
            _log.Write("info", "clipboard.configuration_applied", null,
                new Dictionary<string, object?>
                {
                    ["hotkey"] = binding.Display,
                    ["excluded_process_count"] = _captureExclusions.ProcessNames.Count,
                    ["max_text_bytes"] = _captureLimits.MaxTextBytes,
                    ["max_image_bytes"] = _captureLimits.MaxImageBytes,
                    ["max_image_pixels"] = _captureLimits.MaxImagePixels
                });
            return new WindowsCommandResult(true);
        }

        private void DrainQuickLaunchConfigurationRequests()
        {
            while (_quickLaunchConfigurationRequests.TryDequeue(out var request))
            {
                if (request.CancellationToken.IsCancellationRequested)
                {
                    request.Completion.TrySetCanceled(request.CancellationToken);
                    continue;
                }
                request.Completion.TrySetResult(ApplyQuickLaunchConfiguration(request.Command));
            }
        }

        private void DrainQuickLaunchHotKeyRecordingRequests()
        {
            while (_quickLaunchHotKeyRecordingRequests.TryDequeue(out var request))
            {
                if (request.CancellationToken.IsCancellationRequested)
                {
                    request.Completion.TrySetCanceled(request.CancellationToken);
                    continue;
                }
                var result = SetQuickLaunchRecording(request.Command.Recording);
                request.Completion.TrySetResult(result);
            }
        }

        private void DrainDesktopShellConfigurationRequests()
        {
            while (_desktopShellConfigurationRequests.TryDequeue(out var request))
            {
                if (request.CancellationToken.IsCancellationRequested)
                {
                    request.Completion.TrySetCanceled(request.CancellationToken);
                    continue;
                }
                request.Completion.TrySetResult(ApplyDesktopShellConfiguration(request.Command));
            }
        }

        private WindowsCommandResult ApplyDesktopShellConfiguration(ConfigureDesktopShellCommand command)
        {
            try
            {
                var requiresDesktopExecutable =
                    command.StartOnLoginSilently || command.TrayIconVisible;
                var desktopExecutable = requiresDesktopExecutable
                    ? WindowsPlatformAdapter.ResolveApprovedDesktopExecutablePath()
                    : null;
                if (requiresDesktopExecutable && desktopExecutable is null)
                {
                    return new WindowsCommandResult(
                        false,
                        "The approved protected NxTools desktop executable is unavailable.");
                }

                using (var runKey = Registry.CurrentUser.CreateSubKey(
                           @"Software\Microsoft\Windows\CurrentVersion\Run",
                           writable: true))
                {
                    if (runKey is null)
                    {
                        return new WindowsCommandResult(false, "Windows startup registry is unavailable.");
                    }
                    var startup = ApplyStartupRegistration(
                        runKey,
                        desktopExecutable ?? string.Empty,
                        command.StartOnLoginSilently);
                    if (!startup.Success)
                    {
                        return startup;
                    }
                }

                return ConfigureTrayIcon(
                    command.TrayIconVisible,
                    desktopExecutable ?? string.Empty);
            }
            catch (Exception error)
            {
                var nativeErrorCode = error switch
                {
                    Win32Exception win32 => win32.NativeErrorCode,
                    ExternalException external => external.ErrorCode,
                    _ => error.HResult
                };
                return new WindowsCommandResult(
                    false,
                    error.Message,
                    NativeErrorCode: nativeErrorCode);
            }
        }

        private WindowsCommandResult ConfigureTrayIcon(bool visible, string desktopExecutable)
        {
            return _desktopTray.Configure(
                visible,
                desktopExecutable,
                () => _events.TryWrite(new TrayActionRequestedEvent(Guid.NewGuid(), "open")),
                () => _events.TryWrite(new TrayActionRequestedEvent(Guid.NewGuid(), "exit")));
        }

        private WindowsCommandResult ApplyQuickLaunchConfiguration(ConfigureQuickLaunchCommand command)
        {
            _autoQuickSwitchEnabled = command.Enabled && command.AutoQuickSwitchEnabled;
            var fileDialogSearchEnabled =
                command.Enabled && command.FileDialogSearchEnabled;
            _autoQuickSwitch.Configure(
                _autoQuickSwitchEnabled,
                _autoQuickSwitchEnabled || fileDialogSearchEnabled);
            if (_fileDialogSearchEnabled && !fileDialogSearchEnabled)
            {
                HideFileDialogSearchHost(forget: true);
            }
            _fileDialogSearchEnabled = fileDialogSearchEnabled;
            if (_fileDialogSearchEnabled || _autoQuickSwitchEnabled)
            {
                ProcessForeground(_foregroundContextProvider.Capture());
            }
            else
            {
                CancelFileDialogRecognition();
            }
            if (!command.Enabled)
            {
                _quickLaunchHotKeyRegistration.Stop();
                _quickLaunchTriggerEnabled = false;
                _quickLaunchRecording = false;
                ResetQuickLaunchGesture();
                return new WindowsCommandResult(true);
            }
            if (!QuickLaunchTriggerBinding.TryCreate(command, out var binding, out var error))
            {
                return new WindowsCommandResult(false, error);
            }

            var previous = _quickLaunchTrigger;
            var previousEnabled = _quickLaunchTriggerEnabled;
            WindowsCommandResult registration;
            if (binding.Kind == QuickLaunchTriggerKind.DoubleControl)
            {
                if (_quickLaunchKeyboardMonitor?.IsInstalled != true)
                {
                    return new WindowsCommandResult(
                        false,
                        "The global keyboard monitor required by double Ctrl is unavailable.");
                }
                if (previous.Kind == QuickLaunchTriggerKind.HotKey)
                {
                    _quickLaunchHotKeyRegistration.Stop();
                }
                _quickLaunchTrigger = binding;
                _quickLaunchTriggerEnabled = true;
                _quickLaunchRecording = false;
                ResetQuickLaunchGesture();
                registration = new WindowsCommandResult(true);
            }
            else
            {
                _quickLaunchTriggerEnabled = false;
                ResetQuickLaunchGesture();
                registration = _quickLaunchHotKeyRegistration.Apply(binding.HotKey!);
                if (registration.Success)
                {
                    _quickLaunchTrigger = binding;
                    _quickLaunchTriggerEnabled = true;
                    _quickLaunchRecording = false;
                }
                else
                {
                    _quickLaunchTrigger = previous;
                    _quickLaunchTriggerEnabled = previousEnabled;
                }
            }
            if (registration.Success)
            {
                _log.Write("info", "quick_launch.configuration_applied", null,
                    new Dictionary<string, object?> { ["hotkey"] = binding.Display });
            }
            return registration;
        }

        private WindowsCommandResult SetQuickLaunchRecording(bool recording)
        {
            if (_quickLaunchRecording == recording)
            {
                return new WindowsCommandResult(true);
            }

            if (recording)
            {
                var suspended = _quickLaunchTrigger.Kind == QuickLaunchTriggerKind.HotKey
                    ? _quickLaunchHotKeyRegistration.BeginRecording()
                    : new WindowsCommandResult(true);
                if (!suspended.Success)
                {
                    return suspended;
                }
                _quickLaunchRecording = true;
                _quickLaunchTriggerEnabled = false;
                ResetQuickLaunchGesture();
                return new WindowsCommandResult(true);
            }

            var restored = _quickLaunchTrigger.Kind == QuickLaunchTriggerKind.HotKey
                ? _quickLaunchHotKeyRegistration.CancelRecording()
                : _quickLaunchKeyboardMonitor?.IsInstalled == true
                    ? new WindowsCommandResult(true)
                    : new WindowsCommandResult(false, "The global keyboard monitor is unavailable.");
            if (restored.Success)
            {
                _quickLaunchRecording = false;
                _quickLaunchTriggerEnabled = true;
                ResetQuickLaunchGesture();
            }
            return restored;
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
                            using (var imageData = ClipboardImageDataObject.Load(
                                       imagePayload.ImagePath,
                                       imagePayload.IncludeFileDropFallback))
                            {
                                Clipboard.SetDataObject(imageData.Data, copy: true, retryTimes: 5, retryDelay: 20);
                            }
                            break;
                        case NativeFormatsClipboardPastePayload native:
                            if (!NativeClipboardFormats.TryWrite(
                                    Handle, native.Formats, native.ImagePathFallback,
                                    out var nativeError, out var nativeHresult))
                            {
                                _suppressNextClipboardUpdate = false;
                                throw new ExternalException(
                                    nativeError ?? "Could not restore native clipboard formats.",
                                    nativeHresult);
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
            ConfigureTrayIcon(false, string.Empty);
            SystemEvents.DisplaySettingsChanged -= OnSystemDisplaySettingsChanged;
            SystemEvents.UserPreferenceChanged -= OnSystemUserPreferenceChanged;
            _ = NativeMethods.KillTimer(Handle, (UIntPtr)RetryTimerId);
            _ = NativeMethods.KillTimer(Handle, (UIntPtr)PointerMonitorTimerId);
            _ = NativeMethods.KillTimer(
                Handle,
                (UIntPtr)FileDialogRecognitionTimerId);
            _ = NativeMethods.KillTimer(
                Handle,
                (UIntPtr)DirectorySourceRecognitionTimerId);
            _ = NativeMethods.RemoveClipboardFormatListener(Handle);
            _hotKeyRegistration.Stop();
            _quickLaunchHotKeyRegistration.Stop();
            _quickLaunchTriggerEnabled = false;
            _quickLaunchKeyboardMonitor?.Dispose();
            _quickLaunchKeyboardMonitor = null;
            ResetQuickLaunchGesture();
            if (_foregroundHook != IntPtr.Zero)
            {
                _ = NativeMethods.UnhookWinEvent(_foregroundHook);
                _foregroundHook = IntPtr.Zero;
            }
            CancelFileDialogRecognition();
            foreach (var hook in new[]
                     {
                         _fileDialogCreateHook,
                         _fileDialogDestroyHook,
                         _fileDialogShowHook,
                         _fileDialogHideHook,
                         _fileDialogLocationHook,
                         _fileDialogNameHook,
                         _taskbarOverlayMoveSizeEndHook
                     })
            {
                if (hook != IntPtr.Zero)
                {
                    _ = NativeMethods.UnhookWinEvent(hook);
                }
            }
            _fileDialogCreateHook = IntPtr.Zero;
            _fileDialogDestroyHook = IntPtr.Zero;
            _fileDialogShowHook = IntPtr.Zero;
            _fileDialogHideHook = IntPtr.Zero;
            _fileDialogLocationHook = IntPtr.Zero;
            _fileDialogNameHook = IntPtr.Zero;
            _taskbarOverlayMoveSizeEndHook = IntPtr.Zero;
            while (_pasteRequests.TryDequeue(out var request))
            {
                request.Completion.TrySetResult(new WindowsCommandResult(false, "Windows adapter is stopping."));
            }
            while (_configurationRequests.TryDequeue(out var request))
            {
                request.Completion.TrySetResult(new WindowsCommandResult(
                    false, "Windows adapter is stopping."));
            }
            while (_hotKeyRecordingRequests.TryDequeue(out var request))
            {
                request.Completion.TrySetResult(new WindowsCommandResult(
                    false, "Windows adapter is stopping."));
            }
            while (_quickLaunchConfigurationRequests.TryDequeue(out var request))
            {
                request.Completion.TrySetResult(new WindowsCommandResult(
                    false, "Windows adapter is stopping."));
            }
            while (_quickLaunchHotKeyRecordingRequests.TryDequeue(out var request))
            {
                request.Completion.TrySetResult(new WindowsCommandResult(
                    false, "Windows adapter is stopping."));
            }
            while (_desktopShellConfigurationRequests.TryDequeue(out var request))
            {
                request.Completion.TrySetResult(new WindowsCommandResult(
                    false, "Windows adapter is stopping."));
            }
            DestroyHandle();
            Application.ExitThread();
        }

        private sealed class NativeClipboardHotKeyRegistrar(Func<IntPtr> windowHandle, int hotKeyId)
            : IClipboardHotKeyRegistrar
        {
            public int LastError => Marshal.GetLastWin32Error();

            public bool Register(uint modifiers, uint virtualKey) =>
                NativeMethods.RegisterHotKey(
                    windowHandle(), hotKeyId, modifiers | ModNoRepeat, virtualKey);

            public bool Unregister() =>
                NativeMethods.UnregisterHotKey(windowHandle(), hotKeyId);
        }
    }

    private sealed record PasteRequest(
        ClipboardPastePayload Command,
        Guid CorrelationId,
        CancellationToken CancellationToken,
        TaskCompletionSource<WindowsCommandResult> Completion);

    private sealed record ClipboardConfigurationRequest(
        ConfigureClipboardCommand Command,
        CancellationToken CancellationToken,
        TaskCompletionSource<WindowsCommandResult> Completion);

    private sealed record ClipboardHotKeyRecordingRequest(
        SetClipboardHotKeyRecordingCommand Command,
        CancellationToken CancellationToken,
        TaskCompletionSource<WindowsCommandResult> Completion);

    private sealed record QuickLaunchConfigurationRequest(
        ConfigureQuickLaunchCommand Command,
        CancellationToken CancellationToken,
        TaskCompletionSource<WindowsCommandResult> Completion);

    private sealed record QuickLaunchHotKeyRecordingRequest(
        SetQuickLaunchHotKeyRecordingCommand Command,
        CancellationToken CancellationToken,
        TaskCompletionSource<WindowsCommandResult> Completion);

    private sealed record DesktopShellConfigurationRequest(
        ConfigureDesktopShellCommand Command,
        CancellationToken CancellationToken,
        TaskCompletionSource<WindowsCommandResult> Completion);

}

internal readonly record struct ClipboardCaptureLimits(
    ulong MaxTextBytes,
    ulong MaxImageBytes,
    ulong MaxImagePixels)
{
    public const ulong MinTextBytes = 64 * 1024;
    public const ulong MaxAllowedTextBytes = 64 * 1024 * 1024;
    public const ulong MinImageBytes = 1024 * 1024;
    public const ulong MaxAllowedImageBytes = 256 * 1024 * 1024;
    public const ulong MinImagePixels = 1_000_000;
    public const ulong MaxAllowedImagePixels = 500_000_000;

    public static ClipboardCaptureLimits Default => new(
        8 * 1024 * 1024,
        64 * 1024 * 1024,
        100_000_000);

    public static ClipboardCaptureLimits From(ConfigureClipboardCommand command) => new(
        Clamp(command.MaxTextBytes, MinTextBytes, MaxAllowedTextBytes),
        Clamp(command.MaxImageBytes, MinImageBytes, MaxAllowedImageBytes),
        Clamp(command.MaxImagePixels, MinImagePixels, MaxAllowedImagePixels));

    public bool AllowsText(ulong utf8Bytes) => utf8Bytes <= MaxTextBytes;

    public string? ImageRejectionReason(uint width, uint height, ulong outputBytes)
    {
        var pixels = (ulong)width * height;
        if (pixels > MaxImagePixels)
        {
            return "max_image_pixels";
        }
        return outputBytes > MaxImageBytes ? "max_image_bytes" : null;
    }

    private static ulong Clamp(ulong value, ulong minimum, ulong maximum) =>
        Math.Clamp(value, minimum, maximum);
}
