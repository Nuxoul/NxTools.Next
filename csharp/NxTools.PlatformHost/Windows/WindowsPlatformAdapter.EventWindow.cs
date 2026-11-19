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
    internal sealed partial class EventWindow : NativeWindow, IWindowsPasteOperations
    {
        private const int WmHotKey = 0x0312;
        private const int WmClipboardUpdate = 0x031D;
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
        private const int WmDirectoryOpusSourceRefresh = 0x800C;
        private const int WmCenteredResizeRecoveryRequested = 0x800D;
        private const int WmRefreshSystemMotion = 0x800E;
        private const int WmDisplayChange = 0x007E;
        private const int WmFlushContextMenuPointer = 0x800F;
        private const int WmClipboardPause = 0x8010;
        private const int WmReadClipboardFileDrop = 0x8011;
        private const uint EventSystemForeground = 0x0003;
        private const uint EventSystemMoveSizeEnd = 0x000B;
        private const uint EventObjectCreate = 0x8000;
        private const uint EventObjectDestroy = 0x8001;
        private const uint EventObjectShow = 0x8002;
        private const uint EventObjectHide = 0x8003;
        private const uint EventObjectLocationChange = 0x800B;
        private const uint EventObjectNameChange = 0x800C;
        private const uint EventObjectFocus = 0x8005;
        private const uint EventObjectSelection = 0x8006;
        private const int ObjIdWindow = 0;
        private const uint WineventOutOfContext = 0x0000;
        private const uint RetryTimerId = 1;
        private const uint PointerMonitorTimerId = 2;
        private const uint FileDialogRecognitionTimerId = 3;
        private const uint CenteredResizeRecoveryTimerId = 5;
        private const uint CenteredResizeRecoveryIntervalMs = 50;
        private const uint DirectorySourceRecognitionTimerId = 4;
        private const uint DirectoryOpusSourceRefreshTimerId = 6;
        private const uint SystemMotionRetryTimerId = 7;
        private const uint DirectoryOpusSourceRefreshIntervalMs = 50;
        private const long CenteredResizeRecoveryMaximumWaitMs = 30_000;
        private const int PointerMonitorIntervalMs = 40;
        private const int FileDialogRecognitionIntervalMs = 30;
        private const int FileDialogRecognitionMaxAttempts = 200;
        private const int DirectorySourceRecognitionMaxAttempts = 20;
        private const int MaxClipboardReadAttempts = 5;
        private const int MaxClipboardPublishTail = 64;
        internal const long DefaultClipboardPublicationBytes = 32L * 1024 * 1024;
        private readonly long _clipboardPublicationByteLimit = DefaultClipboardPublicationBytes;
        internal long ClipboardPublicationByteLimit => _clipboardPublicationByteLimit;
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
        private readonly SystemMotionPreference _systemMotion = new(new WindowsSystemMotionReader());
        private bool _systemMotionStopped;
        private bool _systemMotionRetryFailed;
        private readonly PendingMotionPublication _pendingMotion = new();
        internal MotionPreferenceSnapshot SystemMotionSnapshot => _systemMotion.Snapshot;
        private readonly SystemThemePreference _systemTheme = new(new WindowsSystemThemeReader());
        private readonly PendingThemePublication _pendingTheme = new();
        internal ThemePreferenceSnapshot SystemThemeSnapshot => _systemTheme.Snapshot;
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
        private readonly QuickLaunchSummonDiagnosticGate _quickLaunchSummonDiagnosticGate = new();
        private bool _quickLaunchSuppressInExclusiveFullscreen = true;
        private bool _clipboardSuppressInFullscreen = true;
        private readonly ConcurrentQueue<DesktopShellConfigurationRequest> _desktopShellConfigurationRequests = new();
        // Event-thread owned: one physical capture and one coalesced source/return intent.
        // A logical cancellation never frees the occupied native runner slot early.
        private readonly ConcurrentQueue<ExplorerCaptureCompletion> _resolvedDirectorySources = new();
        private ExplorerCaptureOperation? _explorerCapture;
        private DirectoryNavigationContext? _explorerSource;
        private ExplorerReturnIntent? _explorerReturn;
        private long _explorerSourceEpoch;
        private bool _explorerSourceNeedsCapture;
        private bool _explorerCaptureStopped;
        private sealed record ExplorerReturnIntent(AutoQuickSwitchCandidate Candidate, Guid CorrelationId, long StartedAt, long RequestedAtUnixMs);
        private sealed record ExplorerCaptureOperation(long Epoch, long WindowHandle, bool ForReturn, Guid CorrelationId, long StartedAt);
        private sealed record ExplorerCaptureCompletion(ExplorerCaptureOperation Operation, string? Directory, string? FailureKind);
        private readonly Func<long, string?> _captureExplorerDirectory = CaptureExplorerDirectory;
        private readonly Action<Action> _runExplorerWork = work => { _ = Task.Run(work); };
        private readonly Action<Action> _deferExplorerDelivery;
        private readonly Func<long, bool> _isExplorerReturnForeground = window =>
            NativeMethods.IsForegroundWindowForTarget(new IntPtr(window));
        private readonly NativeMethods.WinEventDelegate _foregroundCallback;
        private readonly NativeMethods.WinEventDelegate _fileDialogWindowCallback;
        private IntPtr _foregroundHook;
        private IntPtr _fileDialogCreateHook;
        private IntPtr _fileDialogDestroyHook;
        private IntPtr _fileDialogShowHook;
        private IntPtr _fileDialogHideHook;
        private IntPtr _fileDialogLocationHook;
        private IntPtr _fileDialogNameHook;
        private IntPtr _directoryOpusSelectionHook;
        private IntPtr _directoryOpusFocusHook;
        private IntPtr _taskbarOverlayMoveSizeEndHook;
        private ForegroundContext? _pendingClipboardSource;
        private readonly CenteredResizeRecovery _centeredResizeRecovery = new();
        private readonly System.Collections.Concurrent.ConcurrentQueue<(IntPtr Root, CenteredResizeReplayResult Replay)> _resizeCompletions = new();
        private Task _capturePublishTail = Task.CompletedTask;
        private int _capturePublishPending;
        private readonly object _clipboardPublicationGate = new();
        private long _capturePublishRetainedBytes;
        private static readonly ForegroundContext EmptyClipboardCaptureSource = new(0, 0, 0, "", "");
        internal (int Pending, long RetainedBytes) ClipboardPublicationState
        {
            get { lock (_clipboardPublicationGate) return (_capturePublishPending, _capturePublishRetainedBytes); }
        }
        private int _clipboardReadAttempts;
        private bool _suppressNextClipboardUpdate;
        private bool _primaryPointerDown;
        private bool _secondaryPointerDown;
        private int _fileDialogStructureRetryPosted;
        private int _directoryOpusSourceRefreshPosted;
        private long _pendingDirectoryOpusSourceRefreshWindow;
        private bool _directoryOpusSourceRefreshTimerActive;
        private ulong _platformEventSequence;

        private ulong NextPlatformEventSequence() => ++_platformEventSequence;
        private ClipboardHotKeyRegistration _hotKeyRegistration = null!;
        private ClipboardHotKeyRegistration _quickLaunchHotKeyRegistration = null!;
        private readonly ClipboardHotKeyDebouncer _clipboardHotKeyDebouncer = new();
        private readonly QuickLaunchKeyboardGesture _quickLaunchKeyboardGesture = new();
        private readonly IntPtr _quickLaunchGestureWindow;
        private long _quickLaunchGestureResetGeneration;
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
        private readonly ClipboardContentOperations _clipboardContent = ClipboardContentOperations.Windows(new ForegroundContextProvider());
        private readonly Lazy<ClipboardStagingDirectory> _clipboardStaging = new(() => ClipboardStagingDirectory.Create(ClipboardStagingDirectory.DefaultRoot));
        internal Task ClipboardPublication => _capturePublishTail;

        public bool ClipboardHotKeyAvailable => _hotKeyRegistration?.IsAvailable == true;
        public QuickLaunchKeyboardHealthChangedEvent? QuickLaunchKeyboardHealthSnapshot => _quickLaunchKeyboardMonitor?.HealthSnapshot;
        public bool QuickLaunchHotKeyAvailable => _quickLaunchTriggerEnabled && !_quickLaunchRecording &&
            (_quickLaunchTrigger.Kind == QuickLaunchTriggerKind.DoubleControl
                ? _quickLaunchKeyboardMonitor?.IsInstalled == true
                : _quickLaunchHotKeyRegistration?.IsAvailable == true);

        private readonly NativeContextMenuCoordinator? _contextMenus;

        public EventWindow(
            ChannelWriter<WindowsPlatformEvent> events,
            ForegroundContextProvider foregroundContextProvider,
            IElevatedDirectoryNavigationChannel elevatedDirectoryNavigation,
            IPlatformLog log,
            ClipboardCaptureDropDiagnosticGate clipboardCaptureDropDiagnosticGate,
            NativeContextMenuCoordinator? contextMenus = null, int? clipboardConsumerProcessId = null)
        {
            _events = events;
            _foregroundContextProvider = foregroundContextProvider;
            _elevatedDirectoryNavigation = elevatedDirectoryNavigation;
            _log = log;
            _clipboardCaptureDropDiagnosticGate = clipboardCaptureDropDiagnosticGate;
            _contextMenus = contextMenus;
            _foregroundCallback = OnForegroundChanged;
            _fileDialogWindowCallback = OnFileDialogWindowChanged;
            _clipboardCapture = CreateClipboardCaptureOperations();
            _clipboardContent = ClipboardContentOperations.Windows(foregroundContextProvider);
            _clipboardStaging = new(() => ClipboardStagingDirectory.Create(ClipboardStagingDirectory.DefaultRoot, clipboardConsumerProcessId));
            ClipboardStagingDirectory.Recover(ClipboardStagingDirectory.DefaultRoot, log);
            _deferExplorerDelivery = delivery =>
            {
                _ = NativeMethods.PostMessage(Handle, WmDirectorySourceResolved, IntPtr.Zero, IntPtr.Zero);
            };
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
            _directoryOpusSelectionHook = InstallOptionalWindowEventHook(EventObjectSelection);
            _fileDialogNameHook = InstallOptionalWindowEventHook(EventObjectNameChange);
            _directoryOpusFocusHook = InstallOptionalWindowEventHook(EventObjectFocus);
            _taskbarOverlayMoveSizeEndHook = InstallOptionalWindowEventHook(EventSystemMoveSizeEnd);
            SystemEvents.DisplaySettingsChanged += OnSystemDisplaySettingsChanged;
            SystemEvents.UserPreferenceChanged += OnSystemUserPreferenceChanged;
            _systemMotion.Refresh(out _);
            _systemTheme.Refresh(out _);
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
            _quickLaunchGestureWindow = Handle;
            ResetQuickLaunchGesture();
            _quickLaunchKeyboardMonitor = new LowLevelKeyboardMonitor(
                OnGlobalKeyDown,
                OnGlobalKeyUp,
                healthChanged: health => _events.TryWrite(health),
                log: _log);

            _ = NativeMethods.SetTimer(
                Handle,
                (UIntPtr)PointerMonitorTimerId,
                PointerMonitorIntervalMs,
                IntPtr.Zero);

            ProcessForeground(_foregroundContextProvider.Capture());
        }

        // Isolated caller seam: no HWND, hooks, clipboard listener, COM or global input.
        internal EventWindow(
            ChannelWriter<WindowsPlatformEvent> events,
            IPlatformLog log,
            Func<long, string?> captureExplorerDirectory,
            Action<Action> runExplorerWork,
            Action<Action> deferExplorerDelivery,
            Func<long, bool> isReturnForeground,
            IElevatedDirectoryNavigationChannel? elevatedDirectoryNavigation = null)
        {
            _events = events;
            _log = log;
            _foregroundContextProvider = new ForegroundContextProvider();
            _elevatedDirectoryNavigation = elevatedDirectoryNavigation!;
            _clipboardCaptureDropDiagnosticGate = new();
            _foregroundCallback = OnForegroundChanged;
            _fileDialogWindowCallback = OnFileDialogWindowChanged;
            _captureExplorerDirectory = captureExplorerDirectory;
            _runExplorerWork = runExplorerWork;
            _deferExplorerDelivery = deferExplorerDelivery;
            _isExplorerReturnForeground = isReturnForeground;
            _clipboardCapture = CreateClipboardCaptureOperations();
            _autoQuickSwitch.Configure(true);
        }

        private static string? CaptureExplorerDirectory(long windowHandle) =>
            ExplorerWindowAdapter.TryGetCurrentDirectory(windowHandle, out var directory, out _)
                ? directory : null;

        internal void ObserveExplorerDirectorySource(DirectoryNavigationContext source, string processName)
        {
            CancelExplorerDirectoryCapture("superseded");
            _autoQuickSwitch.ObserveDirectorySource(source, DirectorySourceDisplayName(processName, source));
            _explorerSource = source;
            _explorerSourceNeedsCapture = true;
            QueueExplorerDirectoryResolution(source.WindowHandle);
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
            Guid correlationId,
            CancellationToken cancellationToken)
        {
            var completion = new TaskCompletionSource<WindowsCommandResult>(
                TaskCreationOptions.RunContinuationsAsynchronously);
            _quickLaunchConfigurationRequests.Enqueue(new QuickLaunchConfigurationRequest(
                command, correlationId, cancellationToken, completion));
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
            CancelClipboardPauseRequests();
            StopFileDropReads();
            _ = NativeMethods.PostMessage(Handle, WmClose, IntPtr.Zero, IntPtr.Zero);
        }

        public Task<WindowsCommandResult> EnqueueDesktopShellConfiguration(
            ConfigureDesktopShellCommand command,
            Guid correlationId,
            CancellationToken cancellationToken)
        {
            var completion = new TaskCompletionSource<WindowsCommandResult>(
                TaskCreationOptions.RunContinuationsAsynchronously);
            _desktopShellConfigurationRequests.Enqueue(new DesktopShellConfigurationRequest(
                command, correlationId, cancellationToken, completion));
            if (!NativeMethods.PostMessage(Handle, WmConfigureDesktopShell, IntPtr.Zero, IntPtr.Zero))
            {
                var errorCode = Marshal.GetLastWin32Error();
                completion.TrySetResult(new WindowsCommandResult(
                    false,
                    "Failed to schedule desktop-shell configuration on the Windows thread.",
                    NativeErrorCode: errorCode,
                    Data: "windows_shell_enqueue_failed"));
                WriteDesktopShellDiagnostic("error", "desktop_shell.enqueue_failed", correlationId,
                    new Dictionary<string, object?>
                    {
                        ["stage"] = "enqueue",
                        ["native_error_code"] = errorCode
                    });
            }
            else
            {
                WriteDesktopShellDiagnostic("info", "desktop_shell.accepted", correlationId,
                    new Dictionary<string, object?> { ["stage"] = "accepted" });
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
                case WmFlushContextMenuPointer:
                    MonitorPointerPress();
                    return;
                case WmClipboardUpdate:
                    HandleClipboardUpdate();
                    return;
                case WmHotKey when message.WParam.ToInt32() == ClipboardHotKeyId:
                    if (_clipboardHotKeyDebouncer.TryAccept(Environment.TickCount64))
                    {
                        PublishPanelRequest();
                    }
                    return;
                case WmHotKey when message.WParam.ToInt32() == QuickLaunchHotKeyId:
                    PublishQuickLaunchPanelRequest(QuickLaunchSummonTrigger.HotKey);
                    return;
                case WmTimer when message.WParam.ToInt64() == RetryTimerId:
                    HandleClipboardRetry();
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
                case WmTimer when message.WParam.ToInt64() == CenteredResizeRecoveryTimerId:
                    _ = NativeMethods.KillTimer(Handle, (UIntPtr)CenteredResizeRecoveryTimerId);
                    RetryCenteredResizeRecovery();
                    return;
                case WmCenteredResizeRecoveryRequested:
                    RegisterCenteredResizeRecovery(new IntPtr(message.LParam.ToInt64()));
                    return;
                case WmTimer when message.WParam.ToInt64() == DirectoryOpusSourceRefreshTimerId:
                    _ = NativeMethods.KillTimer(Handle, (UIntPtr)DirectoryOpusSourceRefreshTimerId);
                    _directoryOpusSourceRefreshTimerActive = false;
                    RefreshPendingDirectoryOpusSource();
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
                case WmRefreshSystemMotion:
                    RefreshSystemMotionPreference();
                    return;
                case WmTimer when message.WParam.ToInt64() == SystemMotionRetryTimerId:
                    PublishPendingSystemMotion();
                    return;
                case WmExecutePaste:
                    DrainPasteRequests();
                    return;
                case WmConfigureClipboard:
                    DrainClipboardConfigurationRequests();
                    return;
                case WmReadClipboardFileDrop:
                    DrainFileDropReads();
                    return;
                case WmClipboardPause:
                    DrainClipboardPauseRequests();
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
                    HandleQuickLaunchGesture(0, message.LParam.ToInt64(), unchecked((uint)message.WParam.ToInt64()));
                    return;
                case ElevatedInputMessages.Gesture:
                    HandleQuickLaunchGesture(unchecked((int)(uint)message.WParam.ToInt64()),
                        message.LParam.ToInt64(), unchecked((uint)((ulong)message.WParam.ToInt64() >> 32)));
                    return;
                case ElevatedInputMessages.Unavailable:
                    (_elevatedDirectoryNavigation as IElevatedInputChannel)?.InputUnavailable(
                        message.WParam.ToInt32(), message.LParam.ToInt64());
                    return;
                case ElevatedInputMessages.PointerPressed:
                    HandleElevatedPointerPress(message.WParam.ToInt64(),
                        unchecked((int)(uint)message.LParam.ToInt64()),
                        unchecked((uint)((ulong)message.LParam.ToInt64() >> 32)));
                    return;
                case WmDirectorySourceResolved:
                    DrainResolvedDirectorySources();
                    return;
                case WmDirectoryOpusSourceRefresh:
                    Interlocked.Exchange(ref _directoryOpusSourceRefreshPosted, 0);
                    Volatile.Write(ref _pendingDirectoryOpusSourceRefreshWindow, message.WParam.ToInt64());
                    SchedulePendingDirectoryOpusSourceRefresh();
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

        public bool RequestCenteredResizeRecovery(IntPtr rootWindow)
        {
            if (rootWindow == IntPtr.Zero)
            {
                return false;
            }
            if (NativeMethods.PostMessage(
                    Handle,
                    WmCenteredResizeRecoveryRequested,
                    IntPtr.Zero,
                    new IntPtr(rootWindow.ToInt64())))
            {
                return true;
            }
            var nativeErrorCode = Marshal.GetLastWin32Error();
            if (NativeMethods.TryGetPendingCenteredResize(rootWindow, out var snapshot))
            {
                WriteCenteredResizeDiagnostic(
                    "warning",
                    "download.window_resize_recovery_failed",
                    new CenteredResizeReplayResult(
                        true,
                        snapshot.CorrelationId,
                        snapshot.WindowHandle,
                        snapshot.StartedAtTimestamp,
                        snapshot.AttemptCount,
                        false,
                        true,
                        nativeErrorCode),
                    "recovery_request_unavailable");
            }
            return false;
        }

        private void RegisterCenteredResizeRecovery(IntPtr rootWindow)
        {
            while (_resizeCompletions.TryDequeue(out var completed))
            {
                if (NativeMethods.TryGetPendingCenteredResize(completed.Root, out var live))
                    _centeredResizeRecovery.TrackPending(live);
                var decision = _centeredResizeRecovery.Observe(completed.Root, completed.Replay);
                HandleCenteredResizeRecoveryDecision(completed.Replay, decision);
            }
            if (NativeMethods.TryGetPendingCenteredResize(rootWindow, out var snapshot))
            {
                _centeredResizeRecovery.TrackPending(snapshot);
                ScheduleCenteredResizeRecoveryTimer();
            }
        }

        private void RetryCenteredResizeRecovery()
        {
            foreach (var rootWindow in _centeredResizeRecovery.PendingRoots)
            {
                RetryCenteredResizeForRoot(rootWindow);
            }
            ScheduleCenteredResizeRecoveryTimer();
        }

        private void RetryCenteredResizeForRoot(IntPtr rootWindow)
        {
            if (!NativeMethods.TryGetPendingCenteredResize(rootWindow, out var snapshot))
            {
                _centeredResizeRecovery.Cancel(rootWindow);
                ScheduleCenteredResizeRecoveryTimer();
                return;
            }

            _centeredResizeRecovery.TrackPending(snapshot);
            var waitDecision = _centeredResizeRecovery.WaitForMoveEnd(
                rootWindow,
                Stopwatch.GetTimestamp(),
                NativeMethods.IsDraggingOrSizing(new IntPtr(snapshot.WindowHandle)));
            if (waitDecision == CenteredResizeRecoveryDecision.WaitingForMoveEnd)
            {
                ScheduleCenteredResizeRecoveryTimer();
                return;
            }
            if (waitDecision == CenteredResizeRecoveryDecision.TimedOut)
            {
                NativeMethods.CancelPendingCenteredResize(rootWindow, snapshot.CorrelationId, snapshot.Generation);
                WriteCenteredResizeDiagnostic(
                    "warning",
                    "download.window_resize_recovery_failed",
                    new CenteredResizeReplayResult(
                        true,
                        snapshot.CorrelationId,
                        snapshot.WindowHandle,
                        snapshot.StartedAtTimestamp,
                        snapshot.AttemptCount,
                        false,
                        false,
                        null),
                    "move_end_timeout");
                ScheduleCenteredResizeRecoveryTimer();
                return;
            }
            if (waitDecision != CenteredResizeRecoveryDecision.Retry)
            {
                return;
            }

            if (snapshot.AttemptCount >= CenteredResizeRecovery.MaximumAttempts)
            {
                NativeMethods.CancelPendingCenteredResize(rootWindow, snapshot.CorrelationId, snapshot.Generation);
                _centeredResizeRecovery.Cancel(rootWindow);
                LogCenteredResizeRecoveryTerminal(new(true, snapshot.CorrelationId, snapshot.WindowHandle,
                    snapshot.StartedAtTimestamp, snapshot.AttemptCount, false, false, null) { Generation = snapshot.Generation },
                    CenteredResizeRecoveryDecision.Exhausted);
                return;
            }
            _ = CenteredResizeWorkers.TrySchedule(rootWindow, () =>
            {
                CenteredResizeReplayResult replay;
                try { replay = WindowsPlatformAdapter.ReapplyPendingCenteredResizeFor(rootWindow); }
                catch (Exception failure)
                {
                    replay = new(true, snapshot.CorrelationId, snapshot.WindowHandle, snapshot.StartedAtTimestamp,
                        snapshot.AttemptCount, false, false, failure.HResult) { Generation = snapshot.Generation };
                    NativeMethods.CancelPendingCenteredResize(rootWindow, snapshot.CorrelationId, snapshot.Generation);
                }
                if (replay.HadPendingResize) _resizeCompletions.Enqueue((rootWindow, replay));
                _ = RequestCenteredResizeRecovery(rootWindow);
            });
        }

        private void HandleCenteredResizeRecoveryDecision(
            CenteredResizeReplayResult replay,
            CenteredResizeRecoveryDecision decision)
        {
            if (decision is CenteredResizeRecoveryDecision.Completed or
                CenteredResizeRecoveryDecision.Failed or
                CenteredResizeRecoveryDecision.Exhausted)
            {
                if (decision is CenteredResizeRecoveryDecision.Failed or CenteredResizeRecoveryDecision.Exhausted)
                {
                    NativeMethods.CancelPendingCenteredResize(
                        NativeMethods.GetAncestorOrSelf(new IntPtr(replay.WindowHandle)),
                        replay.CorrelationId, replay.Generation);
                }
                LogCenteredResizeRecoveryTerminal(replay, decision);
            }
            ScheduleCenteredResizeRecoveryTimer();
        }

        private void LogCenteredResizeRecoveryTerminal(
            CenteredResizeReplayResult replay,
            CenteredResizeRecoveryDecision decision)
        {
            var succeeded = decision == CenteredResizeRecoveryDecision.Completed;
            WriteCenteredResizeDiagnostic(
                succeeded ? "info" : "warning",
                succeeded
                    ? "download.window_resize_recovered"
                    : "download.window_resize_recovery_failed",
                replay,
                decision switch
                {
                    CenteredResizeRecoveryDecision.Completed => "client_size_settled",
                    CenteredResizeRecoveryDecision.Exhausted => "attempt_budget_exhausted",
                    CenteredResizeRecoveryDecision.Failed when replay.NativeErrorCode is not null => "native_error",
                    _ => "replay_not_applied"
                });
        }

        private void ScheduleCenteredResizeRecoveryTimer()
        {
            if (!_centeredResizeRecovery.HasPending)
            {
                _ = NativeMethods.KillTimer(Handle, (UIntPtr)CenteredResizeRecoveryTimerId);
                return;
            }
            if (NativeMethods.SetTimer(
                    Handle,
                    (UIntPtr)CenteredResizeRecoveryTimerId,
                    CenteredResizeRecoveryIntervalMs,
                    IntPtr.Zero) != UIntPtr.Zero)
            {
                return;
            }

            var nativeErrorCode = Marshal.GetLastWin32Error();
            foreach (var rootWindow in _centeredResizeRecovery.PendingRoots)
            {
                if (_centeredResizeRecovery.TryGetSnapshot(rootWindow, out var snapshot))
                {
                    NativeMethods.CancelPendingCenteredResize(rootWindow, snapshot.CorrelationId, snapshot.Generation);
                    WriteCenteredResizeDiagnostic(
                        "warning",
                        "download.window_resize_recovery_failed",
                        new CenteredResizeReplayResult(
                            true,
                            snapshot.CorrelationId,
                            snapshot.WindowHandle,
                            snapshot.StartedAtTimestamp,
                            snapshot.AttemptCount,
                            false,
                            true,
                            nativeErrorCode),
                        "recovery_timer_unavailable");
                }
                _centeredResizeRecovery.Cancel(rootWindow);
            }
        }

        private void WriteCenteredResizeDiagnostic(
            string level,
            string eventName,
            CenteredResizeReplayResult replay,
            string reason)
        {
            try
            {
                var presentation = NativeMethods.CaptureWindowPresentation(
                    new IntPtr(replay.WindowHandle));
                var settledNow = eventName == "download.window_resize_recovered" && replay.MatchesCurrentClient(presentation);
                if (eventName == "download.window_resize_recovered" && !settledNow)
                {
                    eventName = "download.window_resize_recovery_failed";
                    level = "warning";
                    reason = "client_changed_before_terminal_publication";
                }
                _log.Write(level, eventName, replay.CorrelationId,
                    new Dictionary<string, object?>
                    {
                        ["window_handle"] = replay.WindowHandle,
                        ["attempt_count"] = replay.AttemptCount,
                        ["elapsed_ms"] = replay.StartedAtTimestamp == 0
                            ? 0
                            : Stopwatch.GetElapsedTime(replay.StartedAtTimestamp).TotalMilliseconds,
                        ["success"] = settledNow,
                        ["reason"] = reason,
                        ["native_error_code"] = replay.NativeErrorCode,
                        ["generation"] = replay.Generation,
                        ["requested_client_width"] = replay.RequestedWidth,
                        ["requested_client_height"] = replay.RequestedHeight,
                        ["readback_client_width"] = replay.ReadbackWidth,
                        ["readback_client_height"] = replay.ReadbackHeight,
                        ["final_client_width"] = presentation.Exists ? presentation.ClientBounds.Width : 0,
                        ["final_client_height"] = presentation.Exists ? presentation.ClientBounds.Height : 0
                    });
            }
            catch
            {
                // Diagnostics cannot change the native resize result.
            }
        }


        internal static bool HasUserClipboardContent(
            string? text,
            ClipboardImageData? imageData,
            IReadOnlyCollection<string> paths) =>
            imageData is not null ||
            paths.Count > 0 ||
            !string.IsNullOrWhiteSpace(text);

        internal void ReadClipboardContent(ForegroundContext source, uint sequenceBefore)
        {
            // Keep the gate at the actual native read boundary as well as update/retry dispatch.
            if (Volatile.Read(ref _clipboardCaptureStopped) || !_clipboardPause.MayReadSequence(sequenceBefore)) return;
            using var files = new ClipboardCaptureFiles(_log);
            var capturedAt = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
            var nativeFormats = _clipboardContent.Formats(Handle).ToList();
            var paths = _clipboardContent.Files();
            ClipboardImageData? imageData = null;
            string? pendingImagePath = null;
            var imageRejected = false;
            var stagingDirectory = _clipboardStaging.Value.DirectoryPath;
            void RejectImage(ClipboardImageBudgetExceededException budget)
            {
                imageRejected = true;
                if (pendingImagePath is not null) files.Remove(pendingImagePath);
                LogCaptureLimitSkip("image", budget.Reason, budget.Observed, budget.Maximum);
            }
            try
            {
                var promoted = ClipboardImagePromotion.TryPromote(paths, text: null, stagingDirectory,
                    _clipboardContent.SaveImage, files, _captureLimits);
                if (promoted is not null)
                {
                    imageData = promoted.Image;
                    paths = promoted.RemainingPaths;
                }
                else
                {
                    using var image = _clipboardContent.Image();
                    if (image is not null)
                    {
                        ClipboardImageEncoding.CheckPixels(image.Width, image.Height, _captureLimits.MaxImagePixels);
                        pendingImagePath = files.CreatePath(stagingDirectory, ".png");
                        _clipboardContent.SaveImage(image, pendingImagePath, _captureLimits.MaxImageBytes);
                        imageData = new(pendingImagePath, checked((uint)image.Width), checked((uint)image.Height));
                    }
                }
            }
            catch (ClipboardImageBudgetExceededException budget) { RejectImage(budget); }
            var text = _clipboardContent.Text();
            if (imageData is null && !imageRejected)
            {
                try
                {
                    var promoted = ClipboardImagePromotion.TryPromote(paths, text, stagingDirectory,
                        _clipboardContent.SaveImage, files, _captureLimits);
                    if (promoted is not null)
                    {
                        imageData = promoted.Image;
                        paths = promoted.RemainingPaths;
                    }
                }
                catch (ClipboardImageBudgetExceededException budget) { RejectImage(budget); }
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
                        ["sequence_after"] = _clipboardCapture.Sequence(),
                        ["image_compatibility_fallback_added"] = compatibilityFallbackAdded,
                        ["native_format_count"] = nativeFormats.Count,
                        ["has_image"] = imageData is not null,
                        ["file_count"] = paths.Length,
                        ["has_text"] = !string.IsNullOrEmpty(text),
                        ["source_process_name"] = source.ProcessName
                    });
                QueueCapturedEvent(text, imageData, paths, nativeFormats, source, capturedAt, files);
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

        private bool MayPublishClipboardCapture(long epoch) =>
            !Volatile.Read(ref _clipboardCaptureStopped) && _clipboardPause.MayRead && _clipboardPause.PublicationEpoch == epoch;

        private static long ClipboardPublicationPayloadBytes(string? text, ClipboardImageData? image,
            IReadOnlyList<string> paths, IReadOnlyList<CapturedClipboardNativeFormat> formats, ForegroundContext source)
        {
            static long StringBytes(string? value) => (long)(value?.Length ?? 0) * sizeof(char);
            var bytes = StringBytes(text) + StringBytes(image?.StagingPath) +
                StringBytes(source.ProcessName) + StringBytes(source.WindowTitle);
            foreach (var path in paths) bytes += StringBytes(path);
            foreach (var format in formats) bytes += format.Data.LongLength + StringBytes(format.FormatName);
            return bytes;
        }

        private bool TryReserveClipboardPublication(long bytes, out string reason, out long retainedBytes)
        {
            lock (_clipboardPublicationGate)
            {
                retainedBytes = _capturePublishRetainedBytes;
                if (_capturePublishPending >= MaxClipboardPublishTail) { reason = "publication_tail_full"; return false; }
                // Preserve one admitted large capture, but never stack other captures behind it.
                if (_capturePublishPending != 0 && (bytes > _clipboardPublicationByteLimit ||
                    _capturePublishRetainedBytes > _clipboardPublicationByteLimit - bytes))
                { reason = "publication_byte_budget_full"; return false; }
                _capturePublishPending++;
                _capturePublishRetainedBytes += bytes;
                reason = "";
                return true;
            }
        }

        private void ReleaseClipboardPublication(long bytes)
        {
            lock (_clipboardPublicationGate)
            {
                _capturePublishPending--;
                _capturePublishRetainedBytes -= bytes;
            }
        }

        internal void QueueCapturedEvent(
            string? text, ClipboardImageData? imageData, string[] paths,
            IReadOnlyList<CapturedClipboardNativeFormat> nativeFormats,
            ForegroundContext source, long capturedAt, ClipboardCaptureFiles? captureFiles = null)
        {
            using var files = captureFiles?.Detach() ?? new ClipboardCaptureFiles(_log);
            if (imageData is not null) files.Own(imageData.StagingPath);
            var epoch = _clipboardPause.PublicationEpoch;
            if (!MayPublishClipboardCapture(epoch)) return;
            var payloadBytes = ClipboardPublicationPayloadBytes(text, imageData, paths, nativeFormats, source);
            if (!TryReserveClipboardPublication(payloadBytes, out var reason, out var retainedBytes))
            {
                if (_clipboardCaptureDropDiagnosticGate.TryAcquire(out var suppressedDropCount))
                {
                    try
                    {
                        _log.Write("warning", "clipboard.capture_event_dropped", null,
                            new Dictionary<string, object?> { ["reason"] = reason, ["suppressed_drop_count"] = suppressedDropCount,
                                ["payload_bytes"] = payloadBytes, ["retained_payload_bytes"] = retainedBytes,
                                ["payload_byte_limit"] = _clipboardPublicationByteLimit });
                    }
                    catch { /* Drop diagnostics cannot change admission or staging cleanup. */ }
                }
                return;
            }
            void RetirePayload()
            {
                nativeFormats = [];
                text = null;
                paths = [];
                imageData = null;
                source = EmptyClipboardCaptureSource;
                ReleaseClipboardPublication(payloadBytes);
            }
            ClipboardCaptureFiles? scheduledOwnership = null;
            try
            {
                var owned = files.Detach();
                scheduledOwnership = owned;
                _capturePublishTail = _capturePublishTail.ContinueWith(_ =>
                {
                    try
                    {
                        using (owned)
                        {
                            try
                            {
                                if (!MayPublishClipboardCapture(epoch)) return;
                                var borrowedIcon = _clipboardContent.SourceIcon(source.ProcessId, out var iconError);
                                string? sourceIconPath = null;
                                if (borrowedIcon is not null)
                                {
                                    sourceIconPath = owned.CreatePath(_clipboardStaging.Value.DirectoryPath, ".png");
                                    File.Copy(borrowedIcon, sourceIconPath);
                                }
                                if (iconError is not null)
                                    _log.Write("warning", "clipboard.source_icon_failed", null,
                                        new Dictionary<string, object?> { ["reason"] = "source_icon_stage_failed" });
                                var stagedFormats = StageNativeFormats(nativeFormats, ownership: owned);
                                var captured = new ClipboardCapturedEvent(text, imageData, paths, sourceIconPath, source, capturedAt, stagedFormats);
                                if (MayPublishClipboardCapture(epoch) && WindowsPlatformAdapter.TryPublishClipboardCapturedEvent(
                                        _events, captured, _log, _clipboardCaptureDropDiagnosticGate))
                                {
                                    if (imageData is not null) owned.Transfer([imageData.StagingPath]);
                                    if (sourceIconPath is not null) owned.Transfer([sourceIconPath]);
                                    owned.Transfer(stagedFormats.Select(format => format.DataPath));
                                }
                            }
                            catch (Exception error)
                            {
                                PlatformExceptionDiagnostics.Record(_log, "clipboard.capture_publish_failed", error);
                            }
                        }
                    }
                    finally { RetirePayload(); }
                }, CancellationToken.None, TaskContinuationOptions.None, TaskScheduler.Default);
            }
            catch
            {
                try { scheduledOwnership?.Dispose(); files.Dispose(); }
                finally { RetirePayload(); }
                throw;
            }
        }

        internal IReadOnlyList<ClipboardNativeFormatData> StageNativeFormats(
            IReadOnlyList<CapturedClipboardNativeFormat> formats, string? stagingDirectory = null,
            Action<string, byte[]>? write = null, ClipboardCaptureFiles? ownership = null)
        {
            using var files = new ClipboardCaptureFiles(_log);
            var staged = new List<ClipboardNativeFormatData>(formats.Count);
            var directory = stagingDirectory ?? _clipboardStaging.Value.DirectoryPath;
            foreach (var format in formats)
            {
                var path = files.CreatePath(directory, ".bin");
                ownership?.Own(path);
                try
                {
                    (write ?? _clipboardContent.WriteFormat)(path, format.Data);
                    staged.Add(new(format.FormatId, format.FormatName, path, checked((ulong)format.Data.LongLength), format.SortOrder));
                }
                catch (Exception error)
                {
                    files.Remove(path);
                    _log.Write("warning", "clipboard.native_format_stage_failed", null,
                        new Dictionary<string, object?> { ["format_id"] = format.FormatId, ["size_bytes"] = format.Data.LongLength,
                            ["exception_type"] = error.GetType().FullName, ["hresult"] = $"0x{error.HResult:X8}" });
                }
            }
            files.Transfer(staged.Select(format => format.DataPath));
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
            var observeOneCommander = _fileDialogSearchEnabled || _autoQuickSwitchEnabled;
            ThirdPartyFileManagerAdapter.ObserveForeground(observeOneCommander ? context : EmptyClipboardCaptureSource);
            if (_explorerReturn is { } pendingReturn && context.WindowHandle != pendingReturn.Candidate.Target.WindowHandle)
            {
                ClearExplorerReturn("foreground_left");
            }
            if (FileDialogSearchHost.CanReuseForeground(
                    _fileDialogSearchEnabled,
                    _fileDialogSearchHostVisible,
                    _activeFileDialogSearchHost,
                    context.WindowHandle))
            {
                if (_activeFileDialogSearchHost is { } snapshot &&
                    FileDialogSearchHost.ShouldRefreshDirectorySourceOnReuse(
                        _autoQuickSwitchEnabled,
                        _fileDialogSearchEnabled,
                        _fileDialogSearchHostVisible,
                        snapshot,
                        context.WindowHandle,
                        _autoQuickSwitch.LatestSource is { } source &&
                        ThirdPartyFileManagerAdapter.IsDirectoryOpusSource(
                            source.Context.WindowHandle,
                            source.Context.HostContextId)))
                {
                    var reusedFileDialog = FileDialogWindowAdapter.TryCapture(context);
                    if (reusedFileDialog is not null)
                    {
                        ProcessRecognizedFileDialog(reusedFileDialog);
                    }
                }
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

            var directoryAttempt = !observeOneCommander && Path.GetFileNameWithoutExtension(context.ProcessName)
                .Equals("OneCommander", StringComparison.OrdinalIgnoreCase)
                ? new DirectoryNavigationCaptureAttempt(null)
                : DirectoryNavigationAdapter.TryCaptureLocalAttempt(context);
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
                if (directorySource.Host.Equals(DirectoryNavigationAdapter.ExplorerHost, StringComparison.Ordinal))
                {
                    ObserveExplorerDirectorySource(directorySource, context.ProcessName);
                }
                else
                {
                    CancelExplorerDirectoryCapture("source_superseded");
                    _autoQuickSwitch.ObserveDirectorySource(directorySource, displayName);
                }
                CancelDirectorySourceRecognition();
                return;
            }
            if ((_fileDialogSearchEnabled || _autoQuickSwitchEnabled) &&
                directoryAttempt.Failure?.Fatal != true &&
                ThirdPartyFileManagerAdapter.IsPotentialHost(context))
            {
                CancelExplorerDirectoryCapture("source_superseded");
                if (!IsTrackedDirectoryOpusSource(context.WindowHandle) ||
                    !_autoQuickSwitch.MarkSourcePending(context.WindowHandle))
                {
                    _autoQuickSwitch.DiscardPendingSource();
                }
                ScheduleDirectorySourceRecognition(context.WindowHandle);
                return;
            }

            if ((_fileDialogSearchEnabled || _autoQuickSwitchEnabled) &&
                FileDialogWindowAdapter.IsPotentialHost(context))
            {
                ScheduleFileDialogRecognition(context.WindowHandle);
                return;
            }

            // An unrelated foreground (taskbar, Alt+Tab switcher, the dialog owner's main window)
            // is not a bound host: like Listary, it neither consumes nor clears the pending
            // source → dialog synchronization.
            CancelFileDialogRecognition();
            CancelDirectorySourceRecognition();
        }

        private bool IsTrackedDirectoryOpusSource(long windowHandle) =>
            windowHandle != 0 &&
            _autoQuickSwitch.LatestSource is { } source &&
            source.Context.WindowHandle == windowHandle &&
            ThirdPartyFileManagerAdapter.IsDirectoryOpusSource(
                source.Context.WindowHandle,
                source.Context.HostContextId);

        private void DiscardUnrecognizedDirectorySource(long windowHandle)
        {
            // A tracked Directory Opus lister keeps its pending state; the dialog return refresh
            // re-reads the active pane and discards only when that read fails.
            if (!IsTrackedDirectoryOpusSource(windowHandle))
            {
                _autoQuickSwitch.DiscardPendingSource();
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
                DiscardUnrecognizedDirectorySource(windowHandle);
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
                return;
            }
            if (_directorySourceRecognitionRetry.HasPendingAttempt(windowHandle))
            {
                ScheduleDirectorySourceRecognition(windowHandle);
                return;
            }
            FailDirectorySourceRecognition("attempt_budget_exhausted", null);
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
            DiscardUnrecognizedDirectorySource(windowHandle);
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

        private bool RefreshDirectoryOpusSourceFromWindow(
            AutoQuickSwitchDirectorySource previous,
            string stage)
        {
            if (string.IsNullOrWhiteSpace(previous.Context.HostContextId))
            {
                return false;
            }
            var started = Stopwatch.GetTimestamp();
            var correlationId = Guid.NewGuid();
            _log.Write(
                "info",
                "quick_launch.directory_opus_source_refresh_started",
                correlationId,
                new Dictionary<string, object?>
                {
                    ["source_window_handle"] = previous.Context.WindowHandle,
                    ["stage"] = stage
                });
            var foreground = _foregroundContextProvider.Capture(
                new IntPtr(previous.Context.WindowHandle));
            var refreshed = ThirdPartyFileManagerAdapter.TryRefreshDirectoryOpusSource(
                previous.Context.WindowHandle,
                previous.Context.HostContextId,
                foreground);
            if (refreshed is null)
            {
                _log.Write(
                    "warning",
                    "quick_launch.directory_opus_source_refresh_failed",
                    correlationId,
                    fields: new Dictionary<string, object?>
                    {
                        ["source_window_handle"] = previous.Context.WindowHandle,
                        ["stage"] = stage,
                        ["success"] = false,
                        ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds
                    });
                return false;
            }

            var refreshedContext = previous.Context with
            {
                CurrentDirectory = refreshed.CurrentDirectory,
                HostContextId = refreshed.HostContextId
            };
            var changed = _autoQuickSwitch.RefreshDirectorySource(
                refreshedContext,
                DirectorySourceDisplayName(foreground.ProcessName, refreshedContext));
            _log.Write(
                "info",
                changed
                    ? "quick_launch.directory_opus_source_refresh_succeeded"
                    : "quick_launch.directory_opus_source_refresh_unchanged",
                correlationId,
                fields: new Dictionary<string, object?>
                {
                    ["source_window_handle"] = refreshed.WindowHandle,
                    ["stage"] = stage,
                    ["success"] = true,
                    ["path_changed"] = changed,
                    ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds
                });
            if (changed &&
                _fileDialogSearchHostVisible &&
                _activeFileDialogSearchHost is { } snapshot)
            {
                PublishFileDialogSearchHost(snapshot, force: true);
            }
            return true;
        }

        private void QueueDirectoryOpusSourceRefresh(
            long eventRootWindowHandle,
            uint eventType,
            int objectId)
        {
            var source = _autoQuickSwitch.LatestSource;
            if (source is null)
            {
                return;
            }
            var isDirectoryOpus = ThirdPartyFileManagerAdapter.IsDirectoryOpusSource(
                source.Context.WindowHandle,
                source.Context.HostContextId);
            var refreshWindow = DirectoryOpusSourceRefresh.PendingFocusRefreshWindow(
                source.Context.WindowHandle,
                isDirectoryOpus,
                eventRootWindowHandle,
                eventType,
                objectId);
            if (refreshWindow == 0)
            {
                return;
            }
            Volatile.Write(ref _pendingDirectoryOpusSourceRefreshWindow, refreshWindow);
            if (Interlocked.Exchange(ref _directoryOpusSourceRefreshPosted, 1) == 0 &&
                !NativeMethods.PostMessage(
                    Handle,
                    WmDirectoryOpusSourceRefresh,
                    new IntPtr(refreshWindow),
                    IntPtr.Zero))
            {
                Interlocked.Exchange(ref _directoryOpusSourceRefreshPosted, 0);
                Volatile.Write(ref _pendingDirectoryOpusSourceRefreshWindow, 0);
            }
        }

        private void SchedulePendingDirectoryOpusSourceRefresh()
        {
            var pendingWindow = Volatile.Read(ref _pendingDirectoryOpusSourceRefreshWindow);
            var source = _autoQuickSwitch.LatestSource;
            var latestWindow = source?.Context.WindowHandle ?? 0;
            var isDirectoryOpus = source is not null &&
                ThirdPartyFileManagerAdapter.IsDirectoryOpusSource(
                    source.Context.WindowHandle,
                    source.Context.HostContextId);
            var foreground = _foregroundContextProvider.Capture();
            var foregroundRoot = NativeMethods.GetRootWindowHandle(
                new IntPtr(foreground.WindowHandle));
            if (pendingWindow == 0 ||
                !DirectoryOpusSourceRefresh.ShouldRefreshPending(
                    pendingWindow,
                    foregroundRoot,
                    isDirectoryOpus,
                    latestWindow) ||
                source is null)
            {
                Volatile.Write(ref _pendingDirectoryOpusSourceRefreshWindow, 0);
                return;
            }
            if (_directoryOpusSourceRefreshTimerActive)
            {
                return;
            }
            if (NativeMethods.SetTimer(
                    Handle,
                    (UIntPtr)DirectoryOpusSourceRefreshTimerId,
                    DirectoryOpusSourceRefreshIntervalMs,
                    IntPtr.Zero) != UIntPtr.Zero)
            {
                _directoryOpusSourceRefreshTimerActive = true;
                return;
            }
            _log.Write(
                "warning",
                "quick_launch.directory_opus_source_refresh_failed",
                fields: new Dictionary<string, object?>
                {
                    ["source_window_handle"] = pendingWindow,
                    ["stage"] = "focus_timer_unavailable",
                    ["success"] = false,
                    ["native_error_code"] = Marshal.GetLastWin32Error()
                });
            Volatile.Write(ref _pendingDirectoryOpusSourceRefreshWindow, 0);
        }

        private void RefreshPendingDirectoryOpusSource()
        {
            var pendingWindow = Interlocked.Exchange(
                ref _pendingDirectoryOpusSourceRefreshWindow,
                0);
            var source = _autoQuickSwitch.LatestSource;
            var latestWindow = source?.Context.WindowHandle ?? 0;
            var isDirectoryOpus = source is not null &&
                ThirdPartyFileManagerAdapter.IsDirectoryOpusSource(
                    source.Context.WindowHandle,
                    source.Context.HostContextId);
            var foreground = _foregroundContextProvider.Capture();
            var foregroundRoot = NativeMethods.GetRootWindowHandle(
                new IntPtr(foreground.WindowHandle));
            if (!DirectoryOpusSourceRefresh.ShouldRefreshPending(
                    pendingWindow,
                    foregroundRoot,
                    isDirectoryOpus,
                    latestWindow) ||
                source is null)
            {
                return;
            }
            _ = RefreshDirectoryOpusSourceFromWindow(
                source,
                "lister_focus_change");
        }


        internal void ProcessRecognizedFileDialog(FileDialogContext fileDialog)
        {
            if (_explorerReturn is { } pending && pending.Candidate.Target.WindowHandle != fileDialog.WindowHandle)
            {
                ClearExplorerReturn("target_superseded");
            }
            var target = new DirectoryNavigationContext(
                DirectoryNavigationAdapter.FileDialogHost,
                fileDialog.WindowHandle,
                fileDialog.CurrentDirectory);
            var candidate = DirectoryOpusDialogReturnRefresh.Observe(
                _autoQuickSwitch,
                target,
                source => ThirdPartyFileManagerAdapter.IsDirectoryOpusSource(
                    source.Context.WindowHandle,
                    source.Context.HostContextId),
                source => RefreshDirectoryOpusSourceFromWindow(
                    source,
                    "dialog_return"));
            if (candidate is not null)
            {
                QueueAutoQuickSwitch(candidate);
            }
        }

        private void QueueExplorerDirectoryResolution(long windowHandle)
        {
            if (_explorerCaptureStopped || _explorerCapture is not null ||
                _explorerSource is not { } source || source.WindowHandle != windowHandle ||
                (!_explorerSourceNeedsCapture && _explorerReturn is null))
            {
                return;
            }
            var intent = _explorerReturn;
            var operation = new ExplorerCaptureOperation(_explorerSourceEpoch, windowHandle,
                intent is not null, intent?.CorrelationId ?? Guid.NewGuid(), intent?.StartedAt ?? Stopwatch.GetTimestamp());
            _explorerCapture = operation; // Install ownership before dispatch, including immediate completions.
            _explorerSourceNeedsCapture = false;
            if (intent is null) LogExplorerCapture(operation.CorrelationId, operation.StartedAt, "accepted", "activation");
            _runExplorerWork(() =>
            {
                LogExplorerCapture(operation.CorrelationId, operation.StartedAt, "start", operation.ForReturn ? "return" : "activation");
                string? directory = null;
                string? failure = null;
                try
                {
                    directory = _captureExplorerDirectory(windowHandle);
                    if (string.IsNullOrWhiteSpace(directory)) failure = "capture_failed";
                }
                catch (Exception error)
                {
                    failure = error.GetType().Name; // Never a raw COM message/path.
                }
                _resolvedDirectorySources.Enqueue(new(operation, directory, failure));
                _deferExplorerDelivery(DrainResolvedDirectorySources);
            });
        }

        internal void DrainResolvedDirectorySources()
        {
            var changed = false;
            while (_resolvedDirectorySources.TryDequeue(out var completion))
            {
                var operation = completion.Operation;
                if (!ReferenceEquals(_explorerCapture, operation)) continue;
                _explorerCapture = null;
                // Cancellation already emitted this return's terminal. A late completion cannot
                // borrow a newer intent or emit a second terminal for the retired correlation.
                if (operation.ForReturn && _explorerReturn?.CorrelationId != operation.CorrelationId) continue;
                if (_explorerCaptureStopped || operation.Epoch != _explorerSourceEpoch ||
                    _explorerSource?.WindowHandle != operation.WindowHandle)
                {
                    if (operation.ForReturn) ClearExplorerReturn("stale");
                    else LogExplorerCapture(operation.CorrelationId, operation.StartedAt, "terminal", "stale");
                    continue;
                }
                if (completion.FailureKind is not null || string.IsNullOrWhiteSpace(completion.Directory))
                {
                    if (operation.ForReturn) ClearExplorerReturn(completion.FailureKind ?? "capture_failed");
                    else
                    {
                        LogExplorerCapture(operation.CorrelationId, operation.StartedAt, "terminal", completion.FailureKind ?? "capture_failed");
                        ClearExplorerReturn("source_failed");
                    }
                    continue;
                }
                LogExplorerCapture(operation.CorrelationId, operation.StartedAt, "ready", operation.ForReturn ? "return" : "activation");
                changed |= _autoQuickSwitch.ResolveDirectorySource(operation.WindowHandle, completion.Directory,
                    DirectorySourceDisplayName("explorer", _explorerSource with { CurrentDirectory = completion.Directory }));
                if (operation.ForReturn && _explorerReturn is { } intent)
                {
                    _explorerReturn = null;
                    var candidate = intent.Candidate;
                    var current = _autoQuickSwitch.IsCurrent(candidate) &&
                        _isExplorerReturnForeground(candidate.Target.WindowHandle);
                    var published = current && _events.TryWrite(new QuickLaunchAutoSwitchRequestedEvent(
                        candidate.Target, completion.Directory, intent.RequestedAtUnixMs));
                    LogExplorerCapture(intent.CorrelationId, intent.StartedAt, "terminal",
                        published ? "requested" : current ? "event_queue_full" : "stale_target");
                }
                else
                {
                    LogExplorerCapture(operation.CorrelationId, operation.StartedAt, "terminal", "source_published");
                }
            }
            if (_explorerReturn is { } pending &&
                (!_autoQuickSwitch.IsCurrent(pending.Candidate) || !_isExplorerReturnForeground(pending.Candidate.Target.WindowHandle)))
            {
                ClearExplorerReturn("stale_target");
            }
            if (_explorerSource is { } next) QueueExplorerDirectoryResolution(next.WindowHandle);
            if (changed && _fileDialogSearchHostVisible && _activeFileDialogSearchHost is { } snapshot)
            {
                PublishFileDialogSearchHost(snapshot, force: true);
            }
        }

        private void LogExplorerCapture(Guid correlationId, long startedAt, string stage, string outcome)
        {
            // Diagnostic failure cannot change admission/completion. No paths, titles or raw errors.
            try
            {
                _log.Write("info", "quick_launch.explorer_source_" + stage, correlationId,
                    new Dictionary<string, object?>
                    {
                        ["source_epoch"] = _explorerSourceEpoch,
                        ["elapsed_ms"] = Stopwatch.GetElapsedTime(startedAt).TotalMilliseconds,
                        ["outcome"] = outcome
                    });
            }
            catch { }
        }

        private void ClearExplorerReturn(string reason)
        {
            if (_explorerReturn is not { } intent) return;
            _explorerReturn = null;
            LogExplorerCapture(intent.CorrelationId, intent.StartedAt, "terminal", reason);
        }

        internal void CancelExplorerDirectoryCapture(string reason, bool stop = false)
        {
            ClearExplorerReturn(reason);
            _explorerSourceEpoch++;
            _explorerSource = null;
            _explorerSourceNeedsCapture = false;
            _explorerCaptureStopped |= stop;
        }

        internal void ExplorerCaptureWindowUnavailable(long windowHandle)
        {
            if (_explorerSource?.WindowHandle == windowHandle) CancelExplorerDirectoryCapture("source_destroyed");
            else if (_explorerReturn?.Candidate.Target.WindowHandle == windowHandle) ClearExplorerReturn("target_unavailable");
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
                var moveSizeRoot = new IntPtr(NativeMethods.GetRootWindowHandle(window));
                RetryCenteredResizeForRoot(moveSizeRoot);
                var overlay = Volatile.Read(ref _taskbarOverlayPlacement);
                if (overlay is not null && window.ToInt64() == overlay.WindowHandle)
                {
                    _events.TryWrite(new TaskbarOverlayEditGestureFinishedEvent(
                        overlay.WindowHandle));
                }
                return;
            }
            if (eventType == EventObjectDestroy && objectId == ObjIdWindow && childId == 0)
                ThirdPartyFileManagerAdapter.RetireWindow(window.ToInt64());

            var rootWindowHandle = NativeMethods.GetRootWindowHandle(window);
            if ((eventType is EventObjectDestroy or EventObjectHide) && objectId == ObjIdWindow)
            {
                ExplorerCaptureWindowUnavailable(window.ToInt64());
            }
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

            if (eventType is EventObjectFocus or EventObjectNameChange or EventObjectSelection)
            {
                QueueDirectoryOpusSourceRefresh(rootWindowHandle, eventType, objectId);
            }


            if ((eventType is EventObjectCreate or EventObjectShow) &&
                (_fileDialogRecognitionRetry.ShouldRetryForStructureChange(rootWindowHandle) ||
                 ((_fileDialogSearchEnabled || _autoQuickSwitchEnabled) &&
                  NativeMethods.IsForegroundWindowForTarget(new IntPtr(rootWindowHandle)))) &&
                Interlocked.Exchange(ref _fileDialogStructureRetryPosted, 1) == 0)
            {
                if (!NativeMethods.PostMessage(
                        Handle,
                        WmFileDialogStructureChanged,
                        IntPtr.Zero,
                        new IntPtr(rootWindowHandle)))
                {
                    Interlocked.Exchange(ref _fileDialogStructureRetryPosted, 0);
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
            UserPreferenceChangedEventArgs eventArgs)
        {
            RequestTaskbarOverlayReposition();
            // SystemEvents owns a broadcast window; marshal motion/theme reads to our STA.
            if (!_systemMotionStopped)
            {
                _ = NativeMethods.PostMessage(Handle, WmRefreshSystemMotion, IntPtr.Zero, IntPtr.Zero);
            }
        }

        private void RefreshSystemMotionPreference()
        {
            if (_systemMotionStopped) return;
            if (_systemMotion.Refresh(out var snapshot)) _pendingMotion.Stage(snapshot);
            if (_systemTheme.Refresh(out var themeSnapshot)) _pendingTheme.Stage(themeSnapshot);
            PublishPendingSystemMotion();
        }

        private void PublishPendingSystemMotion()
        {
            if (_systemMotionStopped) return;
            var themeStarted = Stopwatch.GetTimestamp();
            var themeDelivered = _pendingTheme.TryPublish(value => _events.TryWrite(value));
            var delivered = _pendingMotion.TryPublish(value => _events.TryWrite(value));
            if (_pendingMotion.HasPending || _pendingTheme.HasPending)
            {
                var timer = NativeMethods.SetTimer(Handle, (UIntPtr)SystemMotionRetryTimerId, 250, IntPtr.Zero);
                if (timer == UIntPtr.Zero)
                {
                    var nativeError = Marshal.GetLastWin32Error();
                    if (!_systemMotionRetryFailed)
                    {
                        _systemMotionRetryFailed = true;
                        try
                        {
                            _log.Write("warning", "system.preference.retry_timer_failed",
                                _pendingTheme.PendingCorrelationId ?? _pendingMotion.PendingCorrelationId ?? Guid.Empty,
                                new Dictionary<string, object?>
                                {
                                    ["native_error_code"] = nativeError == 0 ? -1 : nativeError,
                                    ["pending_retained"] = true
                                });
                        }
                        catch { /* Diagnostics never retire the pending preference. */ }
                    }
                }
                else _systemMotionRetryFailed = false;
            }
            else
            {
                _ = NativeMethods.KillTimer(Handle, (UIntPtr)SystemMotionRetryTimerId);
                _systemMotionRetryFailed = false;
            }
            if (themeDelivered is not null)
            {
                try
                {
                    _log.Write(themeDelivered.NativeErrorCode == 0 ? "info" : "warning",
                        "system.theme.preference_changed", themeDelivered.CorrelationId,
                        new Dictionary<string, object?>
                        {
                            ["dark"] = themeDelivered.Dark,
                            ["native_error_code"] = themeDelivered.NativeErrorCode,
                            ["elapsed_ms"] = Stopwatch.GetElapsedTime(themeStarted).TotalMilliseconds
                        });
                }
                catch { /* Diagnostic delivery cannot affect preference state. */ }
            }
            if (delivered is not null)
            {
                var started = Stopwatch.StartNew();
                try
                {
                    _log.Write(delivered.NativeErrorCode == 0 ? "info" : "warning",
                        "system.motion.preference_changed", delivered.CorrelationId,
                        new Dictionary<string, object?>
                        {
                            ["reduced_motion"] = delivered.ReducedMotion,
                            ["native_error_code"] = delivered.NativeErrorCode,
                            ["elapsed_ms"] = started.ElapsedMilliseconds
                        });
                }
                catch { /* Diagnostic delivery cannot affect preference state. */ }
            }
        }

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
            if (candidate.Source.Host.Equals(DirectoryNavigationAdapter.ExplorerHost, StringComparison.Ordinal))
            {
                if (_explorerCaptureStopped || !_autoQuickSwitch.IsCurrent(candidate) ||
                    _explorerSource?.WindowHandle != candidate.SourceWindowHandle) return;
                ClearExplorerReturn("superseded");
                _explorerReturn = new(candidate, Guid.NewGuid(), Stopwatch.GetTimestamp(), DateTimeOffset.UtcNow.ToUnixTimeMilliseconds());
                LogExplorerCapture(_explorerReturn.CorrelationId, _explorerReturn.StartedAt, "accepted", "return");
                // Even a ready activation snapshot may predate Explorer navigation. Serialize one fresh
                // return read AFTER any in-flight activation read, rather than racing the same runner key.
                QueueExplorerDirectoryResolution(candidate.SourceWindowHandle);
                return;
            }
            var requestedAtUnixMs = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
            _runExplorerWork(() =>
            {
                if (!_autoQuickSwitch.IsCurrent(candidate))
                {
                    return;
                }
                var sourceDirectory = candidate.Source.CurrentDirectory;
                if (string.IsNullOrWhiteSpace(sourceDirectory))
                {
                    _log.Write("warning", "quick_launch.auto_switch_source_failed", null,
                        new Dictionary<string, object?>
                        {
                            ["source_window_handle"] = candidate.SourceWindowHandle,
                            ["target_window_handle"] = candidate.Target.WindowHandle,
                            ["failure_kind"] = "capture_failed"
                        });
                    return;
                }
                if (!_autoQuickSwitch.IsCurrent(candidate) ||
                    !_isExplorerReturnForeground(candidate.Target.WindowHandle))
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

        internal void FlushContextMenuPointerPress()
        {
            // Worker-only, bounded message barrier; no buffers whose lifetime can outlive timeout.
            _ = SendMessageTimeoutW(Handle, WmFlushContextMenuPointer, IntPtr.Zero, IntPtr.Zero,
                0x0002, 200, out _);
        }

        [DllImport("user32.dll", SetLastError = true)]
        private static extern IntPtr SendMessageTimeoutW(IntPtr window, int message, IntPtr wParam,
            IntPtr lParam, uint flags, uint timeout, out UIntPtr result);

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
            var target = NativeMethods.GetPointerTarget();
            PublishPointerPress(target.WindowHandle, target.RootWindowHandle, target.ProcessId, sampledAtUnixMs);
        }

        private void PublishPanelRequest()
        {
            var decision = QuickLaunchSummonGate.DispatchObserved(
                triggerEnabled: true,
                suppressInFullscreen: _clipboardSuppressInFullscreen,
                trigger: QuickLaunchSummonTrigger.ClipboardHotKey,
                queryFullscreenState: QuickLaunchSummonGate.QueryWindowsFullscreenState,
                publish: correlationId => PublishPanelRequestCore(correlationId));
            if (!_quickLaunchSummonDiagnosticGate.TryAcquire(decision, out var suppressedFailureCount))
            {
                return;
            }
            try
            {
                _log.Write(
                    decision.QueryHresult is null && decision.ForegroundHresult is null ? "info" : "warning",
                    "clipboard.summon_decision",
                    decision.CorrelationId,
                    new Dictionary<string, object?>
                    {
                        ["decision"] = decision.Suppressed ? "suppressed" : "published",
                        ["fullscreen_kind"] = decision.FullscreenKind.ToString(),
                        ["fullscreen_reason"] = decision.FullscreenReason,
                        ["notification_state"] = decision.NotificationState,
                        ["native_hresult"] = decision.QueryHresult,
                        ["foreground_native_error_code"] = decision.ForegroundHresult,
                        ["elapsed_ms"] = decision.ElapsedMilliseconds,
                        ["suppressed_failure_count"] = suppressedFailureCount
                    });
            }
            catch
            {
                // Logging must never change clipboard capture or panel publication.
            }
        }

        private void PublishPanelRequestCore(Guid correlationId)
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
                eventSequence,
                correlationId));
        }

        private void PublishQuickLaunchPanelRequest(QuickLaunchSummonTrigger trigger)
        {
            var decision = QuickLaunchSummonGate.DispatchObserved(
                _quickLaunchTriggerEnabled && !_quickLaunchRecording,
                _quickLaunchSuppressInExclusiveFullscreen,
                trigger,
                QuickLaunchSummonGate.QueryWindowsFullscreenState,
                correlationId => PublishQuickLaunchPanelRequestCore(correlationId));
            if (!_quickLaunchSummonDiagnosticGate.TryAcquire(decision, out var suppressedFailureCount))
            {
                return;
            }
            try
            {
                _log.Write(
                    decision.QueryHresult is null && decision.ForegroundHresult is null ? "info" : "warning",
                    "quick_launch.summon_decision",
                    decision.CorrelationId,
                    new Dictionary<string, object?>
                    {
                        ["trigger"] = decision.Trigger == QuickLaunchSummonTrigger.HotKey
                            ? "registered_hotkey"
                            : "double_control",
                        ["decision"] = !decision.TriggerEnabled
                            ? "disabled"
                            : decision.Suppressed ? "suppressed" : "published",
                        ["fullscreen_kind"] = decision.FullscreenKind.ToString(),
                        ["fullscreen_reason"] = decision.FullscreenReason,
                        ["notification_state"] = decision.NotificationState,
                        ["native_hresult"] = decision.QueryHresult,
                        ["foreground_native_error_code"] = decision.ForegroundHresult,
                        ["elapsed_ms"] = decision.ElapsedMilliseconds,
                        ["suppressed_failure_count"] = suppressedFailureCount
                    });
            }
            catch
            {
                // Logging must never change whether the panel summon is dispatched.
            }
        }

        private void PublishQuickLaunchPanelRequestCore(Guid correlationId)
        {
            var target = _foregroundContextProvider.Capture();
            var cursor = NativeMethods.GetCursorLocation();
            var requestedAtUnixMs = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
            var deferDirectoryCapture =
                DirectoryNavigationAdapter.RequiresElevatedHelper(target);
            var oneCommander = Path.GetFileNameWithoutExtension(target.ProcessName)
                .Equals("OneCommander", StringComparison.OrdinalIgnoreCase);
            var directoryNavigationContext = deferDirectoryCapture
                ? oneCommander ? _elevatedDirectoryNavigation.TryCapture(target) : null
                : DirectoryNavigationAdapter.TryCaptureLocal(target);
            _events.TryWrite(new QuickLaunchPanelRequestedEvent(
                target,
                cursor.Position.X,
                cursor.Position.Y,
                cursor.DisplayIndex,
                cursor.Dpi,
                requestedAtUnixMs,
                directoryNavigationContext,
                correlationId));
            if (directoryNavigationContext is null && !oneCommander &&
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
            if (_quickLaunchKeyboardGesture.OnKeyDown(virtualKey, timestamp))
                PostOrdinaryQuickLaunchGesture();
        }

        private void OnGlobalKeyUp(int virtualKey)
        {
            if (_quickLaunchKeyboardGesture.OnKeyUp(virtualKey))
                PostOrdinaryQuickLaunchGesture();
        }

        private void PostOrdinaryQuickLaunchGesture() =>
            _ = NativeMethods.PostMessage(_quickLaunchGestureWindow, WmQuickLaunchGesture,
                new IntPtr(_quickLaunchKeyboardGesture.Timestamp),
                new IntPtr(_quickLaunchKeyboardGesture.Generation));

        private void ResetQuickLaunchGesture()
        {
            var configuration = new QuickLaunchKeyboardConfiguration(
                _quickLaunchTriggerEnabled,
                _quickLaunchRecording,
                _quickLaunchTrigger.Kind,
                ++_quickLaunchGestureResetGeneration);
            _quickLaunchKeyboardGesture.Publish(configuration);
            (_elevatedDirectoryNavigation as IElevatedInputChannel)?.ConfigureInput(
                new ElevatedInputConfiguration(Handle.ToInt64(), configuration, Guid.NewGuid()));
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
                    new WindowsPasteCoordinator(this, _log,
                        _elevatedDirectoryNavigation as IElevatedPasteInput).Execute(
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

        private void DrainQuickLaunchConfigurationRequests()
        {
            while (_quickLaunchConfigurationRequests.TryDequeue(out var request))
            {
                if (request.CancellationToken.IsCancellationRequested)
                {
                    request.Completion.TrySetCanceled(request.CancellationToken);
                    continue;
                }

                var startedAt = Stopwatch.GetTimestamp();
                WriteQuickLaunchDiagnostic("info", "quick_launch.configuration.started", request.CorrelationId,
                    new Dictionary<string, object?>
                    {
                        ["stage"] = "windows_apply",
                        ["suppress_in_exclusive_fullscreen"] = request.Command.SuppressInExclusiveFullscreen
                    });
                var result = ApplyQuickLaunchConfiguration(request.Command, request.CorrelationId);
                if (result.Success)
                {
                    _quickLaunchSuppressInExclusiveFullscreen = request.Command.SuppressInExclusiveFullscreen;
                }
                WriteQuickLaunchDiagnostic(result.Success ? "info" : "error",
                    result.Success ? "quick_launch.configuration.applied" : "quick_launch.configuration.failed",
                    request.CorrelationId,
                    new Dictionary<string, object?>
                    {
                        ["stage"] = result.Data ?? "windows_apply",
                        ["success"] = result.Success,
                        ["native_hresult"] = result.NativeErrorCode,
                        ["elapsed_ms"] = Stopwatch.GetElapsedTime(startedAt).TotalMilliseconds
                    });
                request.Completion.TrySetResult(result);
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

        private void WriteQuickLaunchDiagnostic(
            string level,
            string eventName,
            Guid correlationId,
            IReadOnlyDictionary<string, object?> fields)
        {
            try
            {
                _log.Write(level, eventName, correlationId, fields);
            }
            catch
            {
                // Diagnostics cannot change hotkey registration or summon results.
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
            _clipboardSuppressInFullscreen = command.SuppressInFullscreen;
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


        private void WriteDesktopShellDiagnostic(
            string level,
            string eventName,
            Guid correlationId,
            IReadOnlyDictionary<string, object?> fields)
        {
            try
            {
                _log.Write(level, eventName, correlationId, fields);
            }
            catch
            {
                // Diagnostics must not affect desktop-shell configuration results.
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

                var started = Stopwatch.GetTimestamp();
                WriteDesktopShellDiagnostic("info", "desktop_shell.started", request.CorrelationId,
                    new Dictionary<string, object?> { ["stage"] = "windows_apply" });
                var result = ApplyDesktopShellConfiguration(request.Command, request.CorrelationId);
                WriteDesktopShellDiagnostic(result.Success ? "info" : "error",
                    result.Success ? "desktop_shell.applied" : "desktop_shell.apply_failed",
                    request.CorrelationId,
                    new Dictionary<string, object?>
                    {
                        ["stage"] = result.Data ?? "unknown",
                        ["success"] = result.Success,
                        ["native_error_code"] = result.NativeErrorCode,
                        ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds
                    });
                request.Completion.TrySetResult(result);
            }
        }


        private WindowsCommandResult ApplyDesktopShellConfiguration(
            ConfigureDesktopShellCommand command,
            Guid correlationId)
        {
            var failureStage = "desktop_executable_lookup";
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
                        "The approved protected NxTools desktop executable is unavailable.",
                        Data: "desktop_executable_unavailable");
                }

                failureStage = "startup_registration";
                using (var runKey = Registry.CurrentUser.CreateSubKey(
                           @"Software\Microsoft\Windows\CurrentVersion\Run",
                           writable: true))
                {
                    if (runKey is null)
                    {
                        return new WindowsCommandResult(
                            false,
                            "Windows startup registry is unavailable.",
                            Data: "startup_registration_failed");
                    }
                    var startup = ApplyStartupRegistration(
                        runKey,
                        desktopExecutable ?? string.Empty,
                        command.StartOnLoginSilently);
                    if (!startup.Success)
                    {
                        WriteDesktopShellDiagnostic("error", "desktop_shell.startup_registration_failed", correlationId,
                            new Dictionary<string, object?>
                            {
                                ["stage"] = startup.Data ?? "startup_registration_failed",
                                ["native_error_code"] = startup.NativeErrorCode
                            });
                        return startup;
                    }
                    WriteDesktopShellDiagnostic("info", "desktop_shell.startup_registration_applied", correlationId,
                        new Dictionary<string, object?>
                        {
                            ["stage"] = "startup_registration_applied",
                            ["start_on_login_silently"] = command.StartOnLoginSilently
                        });
                }

                failureStage = "tray_icon";
                var tray = ConfigureTrayIcon(
                    command.TrayIconVisible,
                    desktopExecutable ?? string.Empty);
                WriteDesktopShellDiagnostic(tray.Success ? "info" : "error",
                    tray.Success ? "desktop_shell.tray_icon_applied" : "desktop_shell.tray_icon_failed",
                    correlationId,
                    new Dictionary<string, object?>
                    {
                        ["stage"] = tray.Data ?? (tray.Success ? "tray_icon_applied" : "tray_icon_failed"),
                        ["visible"] = command.TrayIconVisible,
                        ["native_error_code"] = tray.NativeErrorCode
                    });
                return tray.Success
                    ? tray with { Data = "windows_shell_apply_succeeded" }
                    : tray;
            }
            catch (Exception error)
            {
                var result = WindowsPlatformAdapter.CreateDesktopShellApplyFailure(error, failureStage);
                WriteDesktopShellDiagnostic("error", "desktop_shell.native_exception", correlationId,
                    new Dictionary<string, object?>
                    {
                        ["stage"] = failureStage,
                        ["exception_type"] = error.GetType().FullName,
                        ["native_error_code"] = result.NativeErrorCode
                    });
                return result;
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

        private WindowsCommandResult ApplyQuickLaunchConfiguration(ConfigureQuickLaunchCommand command, Guid correlationId)
        {
            CancelExplorerDirectoryCapture("configuration_changed");
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
                ThirdPartyFileManagerAdapter.ObserveForeground(EmptyClipboardCaptureSource);
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
                ResetQuickLaunchGesture();
                if (_quickLaunchKeyboardMonitor?.TryInstall(correlationId) != true)
                {
                    return new WindowsCommandResult(
                        false,
                        "The global keyboard monitor required by double Ctrl is unavailable.",
                        NativeErrorCode: _quickLaunchKeyboardMonitor?.LastNativeErrorCode ?? 0);
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
            ResetQuickLaunchGesture();
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
            StopClipboardCapture();
            StopFileDropReads();
            CancelExplorerDirectoryCapture("stopped", stop: true);
            ConfigureTrayIcon(false, string.Empty);
            _systemMotionStopped = true;
            _pendingMotion.Stop();
            _pendingTheme.Stop();
            _ = NativeMethods.KillTimer(Handle, (UIntPtr)SystemMotionRetryTimerId);
            SystemEvents.DisplaySettingsChanged -= OnSystemDisplaySettingsChanged;
            SystemEvents.UserPreferenceChanged -= OnSystemUserPreferenceChanged;
            _ = NativeMethods.KillTimer(Handle, (UIntPtr)PointerMonitorTimerId);
            _ = NativeMethods.KillTimer(
                Handle,
                (UIntPtr)FileDialogRecognitionTimerId);
            _ = NativeMethods.KillTimer(
                Handle,
                (UIntPtr)DirectorySourceRecognitionTimerId);
            _ = NativeMethods.KillTimer(Handle, (UIntPtr)CenteredResizeRecoveryTimerId);
            _ = NativeMethods.KillTimer(Handle, (UIntPtr)DirectoryOpusSourceRefreshTimerId);
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
                        _directoryOpusFocusHook,
                        _directoryOpusSelectionHook,
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
            _directoryOpusFocusHook = IntPtr.Zero;
            _directoryOpusSelectionHook = IntPtr.Zero;
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
        Guid CorrelationId,
        CancellationToken CancellationToken,
        TaskCompletionSource<WindowsCommandResult> Completion);

    private sealed record QuickLaunchHotKeyRecordingRequest(
        SetQuickLaunchHotKeyRecordingCommand Command,
        CancellationToken CancellationToken,
        TaskCompletionSource<WindowsCommandResult> Completion);

    private sealed record DesktopShellConfigurationRequest(
        ConfigureDesktopShellCommand Command,
        Guid CorrelationId,
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
