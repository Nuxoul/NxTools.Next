using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;

namespace NxTools.PlatformHost.Windows;

internal interface IExplorerShellPlatform
{
    // E_NOTIMPL keeps older synthetic platforms off the real Shell boundary.
    int OpenNative(string target, bool isDirectory) => unchecked((int)0x80004001);
    IntPtr FindExplorerWindow(string directory);
    // Legacy synthetic fixtures retain their foreground-only contract. Production
    // must opt into exact active-view observation; folder lookup is never its proof.
    bool SupportsTargetObservation => false;
    IntPtr FindConfirmedTarget(string target, bool isDirectory) => IntPtr.Zero;
    IntPtr FindEnteredDirectory(string directory) => FindExplorerWindow(directory);
    int EnterDirectory(string directory) => OpenNative(directory,true);
    void LaunchExplorer(string arguments);
    bool IsWindow(IntPtr window);
    bool IsIconic(IntPtr window);
    void RestoreWindow(IntPtr window);
    void TryActivateWindow(IntPtr window);
    IntPtr GetForegroundWindow();
    IntPtr GetRootWindow(IntPtr window);
    void WaitForNextAttempt();
    int LastErrorCode { get; }
}

internal sealed record ExplorerTargetView(IntPtr Root, IntPtr View, bool Visible, string Directory, IReadOnlyList<string> SelectedPaths);
internal interface IExplorerTargetObservationPlatform
{
    IEnumerable<ExplorerTargetView> ReadActiveViews();
}

internal static class ExplorerShellActivation
{
    private const int MaxAttempts = 8;

    internal static WindowsCommandResult EnterAndActivate(string target,Action<string>? stages=null) =>
        LaunchAndActivate(target,new NativeExplorerShellPlatform(stages,politeActivation:true),stages,true);
    internal static WindowsCommandResult EnterAndActivate(string target,IExplorerShellPlatform platform) =>
        LaunchAndActivate(target,platform,null,true);
    internal static WindowsCommandResult LaunchAndActivate(string target) =>
        LaunchAndActivate(target, new NativeExplorerShellPlatform(), null);

    internal static WindowsCommandResult LaunchAndActivate(
        string target,
        Action<string>? stageChanged) =>
        LaunchAndActivate(target, new NativeExplorerShellPlatform(stageChanged), stageChanged);

    internal static WindowsCommandResult LaunchAndActivate(
        string target,
        IExplorerShellPlatform platform) =>
        LaunchAndActivate(target, platform, null);

    internal static WindowsCommandResult LaunchAndActivate(
        string target,
        IExplorerShellPlatform platform,
        Action<string>? stageChanged, bool enterDirectory=false)
    {
        var stage = "target_resolution";
        void SetStage(string nextStage)
        {
            if (stage == nextStage)
            {
                return;
            }
            stage = nextStage;
            try
            {
                stageChanged?.Invoke(stage);
            }
            catch
            {
                // Diagnostics must never change Shell behavior.
            }
        }

        try
        {
            var isDirectory = Directory.Exists(target);
            enterDirectory &= isDirectory;
            if (!isDirectory && !File.Exists(target))
            {
                return Failure("target_missing", "The Explorer target is no longer available.", 2);
            }
            SetStage("target_resolved");

            var fullTarget = NormalizeDirectory(target);
            var parent = Path.GetDirectoryName(fullTarget);
            var revealRoot = isDirectory && (enterDirectory || string.IsNullOrWhiteSpace(parent));
            var directory = revealRoot ? fullTarget : parent;
            if (string.IsNullOrWhiteSpace(directory))
            {
                return Failure("target_parent_missing", "The Explorer target has no containing directory.", 3);
            }
            directory = NormalizeDirectory(directory);

            SetStage(revealRoot ? "native_directory_open_requested" : "selection_requested");
            var nativeResult = enterDirectory && platform.FindEnteredDirectory(target)!=IntPtr.Zero ? 0
                : enterDirectory ? platform.EnterDirectory(target) : platform.OpenNative(target,isDirectory);
            var fallbackLaunched = false;
            void LaunchFallback()
            {
                SetStage("explorer_fallback_requested");
                platform.LaunchExplorer(revealRoot
                    ? $"\"{target}\""
                    : $"/select,\"{target}\"");
                fallbackLaunched = true;
            }
            if (nativeResult < 0)
            {
                LaunchFallback();
            }
            else
            {
                SetStage("native_request_accepted");
            }
            SetStage("existing_window_lookup");
            var candidate = platform.SupportsTargetObservation
                ? IntPtr.Zero : platform.FindExplorerWindow(directory);

            var lastExpectedRoot = IntPtr.Zero;
            for (var attempt = 0; attempt < MaxAttempts; attempt++)
            {
                if (platform.SupportsTargetObservation)
                {
                    SetStage("target_readback");
                    candidate = enterDirectory ? platform.FindEnteredDirectory(target) : platform.FindConfirmedTarget(target, isDirectory);
                }
                else if (candidate == IntPtr.Zero || !platform.IsWindow(candidate))
                {
                    SetStage("explorer_window_lookup");
                    candidate = platform.FindExplorerWindow(directory);
                }

                if (candidate != IntPtr.Zero && platform.IsWindow(candidate))
                {
                    lastExpectedRoot = platform.GetRootWindow(candidate);
                    if (lastExpectedRoot == IntPtr.Zero)
                    {
                        lastExpectedRoot = candidate;
                    }

                    if (platform.SupportsTargetObservation)
                    {
                        if (!(isDirectory ? Directory.Exists(target) : File.Exists(target)))
                            return Failure("target_missing", "The Explorer target is no longer available.", 2);
                        SetStage("target_confirmed");
                        var foregroundConfirmed = false;
                        int? activationError = null;
                        try
                        {
                            SetStage("foreground_activation_requested");
                            if (platform.IsIconic(candidate)) platform.RestoreWindow(candidate);
                            platform.TryActivateWindow(candidate);
                            SetStage("foreground_readback");
                            var currentForeground = platform.GetForegroundWindow();
                            foregroundConfirmed = platform.IsWindow(currentForeground) &&
                                IsExpectedForegroundRoot(lastExpectedRoot, currentForeground, platform.GetRootWindow);
                        }
                        catch (Win32Exception error)
                        {
                            activationError = error.NativeErrorCode;
                            SetStage("foreground_activation_failed");
                        }
                        catch (Exception error)
                        {
                            // The exact target was observed; activation is now best effort.
                            activationError = error.HResult;
                            SetStage("foreground_activation_failed");
                        }
                        if (!platform.IsWindow(candidate))
                            return Failure("target_window_destroyed", "The observed Explorer window is no longer available.", 1400);
                        if (!(isDirectory ? Directory.Exists(target) : File.Exists(target)))
                            return Failure("target_missing", "The Explorer target is no longer available.", 2);
                        SetStage(foregroundConfirmed ? "foreground_confirmed" : "foreground_not_confirmed");
                        return new WindowsCommandResult(true, Data: foregroundConfirmed
                            ? "foreground_confirmed" : "target_confirmed_foreground_not_confirmed",
                            NativeErrorCode: activationError);
                    }

                    SetStage("foreground_activation_requested");
                    if (platform.IsIconic(candidate))
                    {
                        platform.RestoreWindow(candidate);
                    }
                    platform.TryActivateWindow(candidate);

                    SetStage("foreground_readback");
                    var foreground = platform.GetForegroundWindow();
                    if (platform.IsWindow(candidate) && foreground != IntPtr.Zero &&
                        platform.IsWindow(foreground) && platform.GetRootWindow(foreground) == lastExpectedRoot)
                    {
                        SetStage("foreground_confirmed");
                        return new WindowsCommandResult(true, Data: "foreground_confirmed");
                    }
                }

                // One recovery launch only; API acceptance alone is not success.
                if (!fallbackLaunched && attempt == MaxAttempts / 2 - 1)
                {
                    LaunchFallback();
                    candidate = IntPtr.Zero;
                }
                if (attempt + 1 < MaxAttempts)
                {
                    platform.WaitForNextAttempt();
                }
            }

            if (platform.SupportsTargetObservation)
            {
                SetStage("target_not_confirmed");
                return Failure("target_not_confirmed", "Explorer did not confirm the requested folder or file selection in time.", platform.LastErrorCode);
            }
            var failureStage = candidate == IntPtr.Zero
                ? "explorer_window_not_found"
                : "foreground_not_confirmed";
            SetStage(failureStage);
            var message = candidate == IntPtr.Zero
                ? "Explorer did not publish the requested folder window in time."
                : "Windows did not confirm the requested Explorer window in the foreground.";
            return Failure(failureStage, message, platform.LastErrorCode);
        }
        catch (Win32Exception error)
        {
            return Failure(stage, "Windows could not activate the requested Explorer window.", error.NativeErrorCode);
        }
        catch (Exception error)
        {
            return Failure(stage, "Windows could not activate the requested Explorer window.", error.HResult);
        }
    }

    internal static bool IsExpectedForegroundRoot(
        IntPtr expectedRoot,
        IntPtr foregroundWindow,
        Func<IntPtr, IntPtr> getRootWindow)
    {
        return expectedRoot != IntPtr.Zero && foregroundWindow != IntPtr.Zero &&
            getRootWindow(foregroundWindow) == expectedRoot;
    }

    internal static string NormalizeDirectory(string directory)
    {
        var fullPath = Path.GetFullPath(directory);
        // Shell automation publishes ordinary DOS/UNC paths even when the request
        // came from a canonical extended-length filesystem path. Normalize only
        // these explicit aliases; device/volume namespaces retain their identity.
        if (fullPath.StartsWith(@"\\?\UNC\", StringComparison.OrdinalIgnoreCase))
            fullPath = @"\\" + fullPath[8..];
        else if (fullPath.Length >= 7 && fullPath.StartsWith(@"\\?\", StringComparison.Ordinal) &&
                 char.IsAsciiLetter(fullPath[4]) && fullPath[5] == ':' && fullPath[6] == '\\')
            fullPath = fullPath[4..];
        var root = Path.GetPathRoot(fullPath);
        return !string.IsNullOrEmpty(root) &&
            fullPath.Length <= root.Length
                ? root
                : fullPath.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
    }

    private static WindowsCommandResult Failure(
        string stage,
        string message,
        int nativeErrorCode) =>
        new(
            false,
            message,
            Data: stage,
            NativeErrorCode: nativeErrorCode == 0 ? null : nativeErrorCode);

    internal static IntPtr ObserveTarget(string target, bool isDirectory, IExplorerTargetObservationPlatform platform, bool enterDirectory=false)
    {
        var fullTarget = NormalizeDirectory(target);
        var parent = Path.GetDirectoryName(fullTarget);
        var revealRoot = isDirectory && (enterDirectory || string.IsNullOrWhiteSpace(parent));
        var expectedDirectory = NormalizeDirectory(revealRoot ? fullTarget : parent!);
        var expectedFile = revealRoot ? null : fullTarget;
        foreach (var view in platform.ReadActiveViews())
        {
            if (view.Root == IntPtr.Zero || view.View == IntPtr.Zero || !view.Visible ||
                !string.Equals(NormalizeDirectory(view.Directory), expectedDirectory, StringComparison.OrdinalIgnoreCase)) continue;
            if (revealRoot || view.SelectedPaths.Any(path => string.Equals(
                NormalizeDirectory(path), expectedFile, StringComparison.OrdinalIgnoreCase))) return view.Root;
        }
        return IntPtr.Zero;
    }

    // IShellFolderViewDual.Folder returns base Folder dispatch. Self belongs to
    // Folder2, so query that interface instead of asking the base dispatch for it.
    internal static object ReadFolderSelf(object folder)
    {
        var result = ((IExplorerFolder2)folder).GetSelf(out var self);
        Marshal.ThrowExceptionForHR(result);
        return self ?? throw new COMException("The active Shell folder has no self item.", unchecked((int)0x80004005));
    }

    // SDK ShlDisp.h: IUnknown(3) + IDispatch(4) + Folder(10) precede
    // Folder2.get_Self at zero-based native slot17. Only get_Self is invoked.
    [ComImport, Guid("f0d2d8ef-3890-11d2-bf8b-00c04fb93661"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    internal interface IExplorerFolder2
    {
        [PreserveSig] int GetTypeInfoCount(out uint count);
        [PreserveSig] int GetTypeInfo(uint index, uint locale, out IntPtr info);
        [PreserveSig] int GetIDsOfNames(ref Guid iid, IntPtr names, uint count, uint locale, IntPtr ids);
        [PreserveSig] int Invoke(int id, ref Guid iid, uint locale, ushort flags, IntPtr parameters,
            IntPtr result, IntPtr exception, IntPtr argumentError);
        [PreserveSig] int GetTitle(out IntPtr title);
        [PreserveSig] int GetApplication(out IntPtr application);
        [PreserveSig] int GetParent(out IntPtr parent);
        [PreserveSig] int GetParentFolder(out IntPtr folder);
        [PreserveSig] int Items(out IntPtr items);
        [PreserveSig] int ParseName(IntPtr name, out IntPtr item);
        [PreserveSig] int NewFolder(IntPtr name, [MarshalAs(UnmanagedType.Struct)] object options);
        [PreserveSig] int MoveHere([MarshalAs(UnmanagedType.Struct)] object item, [MarshalAs(UnmanagedType.Struct)] object options);
        [PreserveSig] int CopyHere([MarshalAs(UnmanagedType.Struct)] object item, [MarshalAs(UnmanagedType.Struct)] object options);
        [PreserveSig] int GetDetailsOf([MarshalAs(UnmanagedType.Struct)] object item, int column, out IntPtr details);
        [PreserveSig] int GetSelf([MarshalAs(UnmanagedType.Interface)] out object? self);
    }

    internal sealed class NativeTargetObservationPlatform : IExplorerTargetObservationPlatform
    {
        private readonly Action<int> _nativeFailure;
        private readonly Func<object> _createWindows;
        private readonly Action<string>? _observationStage;
        internal NativeTargetObservationPlatform(Action<int> nativeFailure,
            Func<object>? createWindows = null, Action<string>? observationStage = null)
        {
            _nativeFailure = nativeFailure;
            _observationStage = observationStage;
            _createWindows = createWindows ?? (() => Activator.CreateInstance(Type.GetTypeFromCLSID(
                new Guid("9BA05972-F6A8-11CF-A442-00A0C90A8F39"))!)!);
        }
        public IEnumerable<ExplorerTargetView> ReadActiveViews()
        {
            using var apartment = ExplorerShellApartment.Enter();
            Marshal.ThrowExceptionForHR(apartment.Result);
            object? windows = null;
            try
            {
                windows = _createWindows();
                dynamic collection = windows;
                // Snapshot count once; bound a corrupt/unexpected provider, not replacement workers.
                var count = Math.Clamp((int)collection.Count, 0, 256);
                Report(count == 0 ? "observation_no_shell_windows" : "observation_shell_windows_present");
                for (var index = 0; index < count; index++)
                {
                    ExplorerTargetView? view = null;
                    object? candidate = null;
                    try
                    {
                        candidate = collection.Item(index);
                        if (candidate is not IExplorerServiceProvider services)
                        {
                            Report("observation_service_interface_missing");
                            continue;
                        }
                        Report("observation_service_interface_acquired");
                        // Service-only candidates are supported when their native
                        // browser still proves an actual Explorer root and active view.
                        var expectedRoot = IntPtr.Zero;
                        if (candidate is IExplorerBrowserDispatch dispatch)
                        {
                            var hwnd = new IntPtr(dispatch.WindowHandle);
                            expectedRoot = NativeExplorerShellPlatform.ReadRootWindow(hwnd);
                            if (expectedRoot != hwnd) Report("observation_browser_child_hwnd");
                            if (!NativeExplorerShellPlatform.IsExplorerRoot(expectedRoot))
                            { Report("observation_explorer_root_rejected"); continue; }
                        }
                        else Report("observation_browser_dispatch_unavailable_using_service");
                        view = ReadActiveView(expectedRoot, services);
                    }
                    catch (Exception error) when (error is COMException or InvalidCastException or
                        InvalidOperationException or ArgumentException or MissingMemberException or
                        Microsoft.CSharp.RuntimeBinder.RuntimeBinderException)
                    {
                        // One closing/incompatible candidate cannot abort later candidates.
                        Report(error is COMException ? "observation_candidate_com_failure" : "observation_candidate_managed_failure");
                        _nativeFailure(error.HResult);
                    }
                    finally { ReleaseCom(candidate); }
                    if (view is not null) yield return view;
                }
            }
            finally { ReleaseCom(windows); }
        }

        private void Report(string stage)
        {
            try { _observationStage?.Invoke(stage); }
            catch { /* Diagnostics cannot change observation. */ }
        }

        private ExplorerTargetView? ReadActiveView(IntPtr root, IExplorerServiceProvider services)
        {
            object? shellBrowser = null;
            IExplorerShellView? shellView = null;
            object? background = null;
            object? folder = null;
            object? folderSelf = null;
            object? selected = null;
            try
            {
                var service = new Guid("4C96BE40-915C-11CF-99D3-00AA004AE837"); // SID_STopLevelBrowser
                var browserId = typeof(IExplorerShellBrowser).GUID;
                var result = services.QueryService(ref service, ref browserId, out shellBrowser);
                if (result < 0) { Report("observation_query_service_failed"); _nativeFailure(result); return null; }
                if (shellBrowser is not IExplorerShellBrowser browser)
                { Report("observation_shell_browser_interface_missing"); return null; }
                Report("observation_query_service_succeeded");
                result = browser.GetWindow(out var browserWindow);
                if (result < 0) { Report("observation_browser_window_failed"); _nativeFailure(result); return null; }
                var actualRoot = NativeExplorerShellPlatform.ReadRootWindow(browserWindow);
                if (!NativeExplorerShellPlatform.IsExplorerRoot(actualRoot) || (root != IntPtr.Zero && root != actualRoot))
                { Report("observation_browser_root_mismatch"); return null; }
                root = actualRoot;
                result = browser.QueryActiveShellView(out shellView);
                if (result < 0) { Report("observation_active_view_failed"); _nativeFailure(result); return null; }
                if (shellView is null) { Report("observation_active_view_missing"); return null; }
                Report("observation_active_view_acquired");
                result = shellView.GetWindow(out var viewWindow);
                if (result < 0) { Report("observation_view_window_failed"); _nativeFailure(result); return null; }
                if (!NativeExplorerShellPlatform.IsVisibleView(viewWindow, root)) { Report("observation_view_visibility_rejected"); return null; }
                var dispatchId = new Guid("00020400-0000-0000-C000-000000000046");
                result = shellView.GetItemObject(0, ref dispatchId, out background);
                if (result < 0) { Report("observation_background_failed"); _nativeFailure(result); return null; }
                if (background is null) return null;
                // SVGIO_BACKGROUND is the active view's ShellFolderView dispatch, not
                // IWebBrowser2.Document (which can describe an inactive same-root tab).
                dynamic active = background;
                folder = active.Folder;
                folderSelf = ReadFolderSelf(folder);
                var directory = (string)((dynamic)folderSelf).Path;
                if (!Path.IsPathFullyQualified(directory)) { Report("observation_folder_not_filesystem"); return null; }
                Report("observation_filesystem_folder_decoded");
                selected = active.SelectedItems();
                dynamic items = selected;
                var count = (int)items.Count;
                if (count < 0 || count > 4096) return null; // fail closed on unsupported selection size
                Report(count == 0 ? "observation_selection_empty" : "observation_selection_present");
                var paths = new List<string>(count);
                for (var index = 0; index < count; index++)
                {
                    object? item = null;
                    try
                    {
                        item = items.Item(index);
                        var path = (string)((dynamic)item).Path;
                        if (Path.IsPathFullyQualified(path)) paths.Add(path);
                    }
                    finally { ReleaseCom(item); }
                }
                Report("observation_active_view_snapshot_ready");
                return new(root, viewWindow, NativeExplorerShellPlatform.IsVisibleView(viewWindow, root), directory, paths);
            }
            finally
            {
                ReleaseCom(selected); ReleaseCom(folderSelf); ReleaseCom(folder);
                ReleaseCom(background); ReleaseCom(shellView); ReleaseCom(shellBrowser);
            }
        }

        private static void ReleaseCom(object? value)
        {
            if (value is not null && Marshal.IsComObject(value)) Marshal.ReleaseComObject(value);
        }
    }

    [ComImport, Guid("6D5140C1-7436-11CE-8034-00AA006009FA"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    internal interface IExplorerServiceProvider
    {
        [PreserveSig] int QueryService(ref Guid service, ref Guid iid, [MarshalAs(UnmanagedType.Interface)] out object? value);
    }

    // IUnknown + IOleWindow slots MUST precede the methods used below.
    [ComImport, Guid("000214E2-0000-0000-C000-000000000046"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    internal interface IExplorerShellBrowser
    {
        [PreserveSig] int GetWindow(out IntPtr window);
        [PreserveSig] int ContextSensitiveHelp([MarshalAs(UnmanagedType.Bool)] bool enter);
        [PreserveSig] int InsertMenusSB(IntPtr menu, IntPtr widths);
        [PreserveSig] int SetMenuSB(IntPtr menu, IntPtr reserved, IntPtr window);
        [PreserveSig] int RemoveMenusSB(IntPtr menu);
        [PreserveSig] int SetStatusTextSB([MarshalAs(UnmanagedType.LPWStr)] string text);
        [PreserveSig] int EnableModelessSB([MarshalAs(UnmanagedType.Bool)] bool enable);
        [PreserveSig] int TranslateAcceleratorSB(IntPtr message, ushort id);
        [PreserveSig] int BrowseObject(IntPtr pidl, uint flags);
        [PreserveSig] int GetViewStateStream(uint mode, out IntPtr stream);
        [PreserveSig] int GetControlWindow(uint id, out IntPtr window);
        [PreserveSig] int SendControlMsg(uint id, uint message, IntPtr wparam, IntPtr lparam, out IntPtr result);
        [PreserveSig] int QueryActiveShellView(out IExplorerShellView? view);
        [PreserveSig] int OnViewWindowActive(IExplorerShellView view);
        [PreserveSig] int SetToolbarItems(IntPtr buttons, uint count, uint flags);
    }

    [ComImport, Guid("000214E3-0000-0000-C000-000000000046"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    internal interface IExplorerShellView
    {
        [PreserveSig] int GetWindow(out IntPtr window);
        [PreserveSig] int ContextSensitiveHelp([MarshalAs(UnmanagedType.Bool)] bool enter);
        [PreserveSig] int TranslateAccelerator(IntPtr message);
        [PreserveSig] int EnableModeless([MarshalAs(UnmanagedType.Bool)] bool enable);
        [PreserveSig] int UIActivate(uint state);
        [PreserveSig] int Refresh();
        [PreserveSig] int CreateViewWindow(IntPtr previous, IntPtr settings, IntPtr browser, IntPtr rectangle, out IntPtr window);
        [PreserveSig] int DestroyViewWindow();
        [PreserveSig] int GetCurrentInfo(IntPtr settings);
        [PreserveSig] int AddPropertySheetPages(uint reserved, IntPtr callback, IntPtr lparam);
        [PreserveSig] int SaveViewState();
        [PreserveSig] int SelectItem(IntPtr pidl, uint flags);
        [PreserveSig] int GetItemObject(uint item, ref Guid iid, [MarshalAs(UnmanagedType.Interface)] out object? value);
    }

    private sealed class NativeExplorerShellPlatform : IExplorerShellPlatform
    {
        private const int SwRestore = 9;
        private const uint GaRoot = 2;
        private readonly Action<string>? _observationStage;
        private readonly HashSet<string> _reportedObservationStages = [];
        private readonly bool _politeActivation;
        internal NativeExplorerShellPlatform(Action<string>? observationStage = null,bool politeActivation=false) { _observationStage=observationStage;_politeActivation=politeActivation; }
        private void ReportObservationStage(string stage)
        {
            // Fixed vocabulary, once per command rather than per window/attempt.
            if (!_reportedObservationStages.Add(stage)) return;
            try { _observationStage?.Invoke(stage); }
            catch { /* A diagnostic sink cannot change the native result. */ }
        }
        private int _lastNativeError;
        public int LastErrorCode => _lastNativeError;

        private NativeDirectoryOpusTargetPlatform? _directoryOpus;
        public bool SupportsTargetObservation => true;
        public IntPtr FindConfirmedTarget(string target, bool isDirectory)
        {
            try
            {
                var root = ObserveTarget(target, isDirectory, new NativeTargetObservationPlatform(
                    code => _lastNativeError = code, observationStage: ReportObservationStage));
                if (root != IntPtr.Zero) return root;
            }
            catch (Exception error) when (error is COMException or InvalidCastException or
                Microsoft.CSharp.RuntimeBinder.RuntimeBinderException)
            {
                _lastNativeError = error.HResult;
                ReportObservationStage("observation_explorer_reader_failed");
            }
            _directoryOpus ??= new NativeDirectoryOpusTargetPlatform(ReportObservationStage);
            return ObserveTarget(target, isDirectory, new DirectoryOpusTargetObservation(
                target, isDirectory, _directoryOpus, ReportObservationStage, code => _lastNativeError = code));
        }
        public IntPtr FindExplorerWindow(string directory) => FindConfirmedTarget(directory, true);

        public IntPtr FindEnteredDirectory(string target) => ObserveTarget(target,true,new NativeTargetObservationPlatform(code=>_lastNativeError=code,observationStage:ReportObservationStage),true);
        public int EnterDirectory(string target) => ExplorerNativeFolder.Enter(target);
        public int OpenNative(string target, bool isDirectory)
        {
            int result;
            try { result = ExplorerNativeFolder.Open(target, isDirectory); }
            catch (Win32Exception error) { result = error.HResult; }
            catch (Exception error) { result = error.HResult; }
            if (result < 0) _lastNativeError = result;
            return result;
        }

        public void LaunchExplorer(string arguments)
        {
            using var process = Process.Start(new ProcessStartInfo("explorer.exe", arguments)
            {
                UseShellExecute = true,
            });
        }

        public bool IsWindow(IntPtr window) => IsWindowNative(window);
        public bool IsIconic(IntPtr window) => IsIconicNative(window);
        public void RestoreWindow(IntPtr window) => _ = ShowWindow(window, SwRestore);

        public void TryActivateWindow(IntPtr window)
        {
            if (_politeActivation) { _=SetForegroundWindow(window);return; }
            var currentThread = GetCurrentThreadId();
            var targetThread = GetWindowThreadProcessId(window, out _);
            var foreground = GetForegroundWindowNative();
            var foregroundThread = foreground == IntPtr.Zero
                ? 0
                : GetWindowThreadProcessId(foreground, out _);
            var targetAttached = targetThread != 0 && targetThread != currentThread &&
                AttachThreadInput(currentThread, targetThread, true);
            var foregroundAttached = foregroundThread != 0 && foregroundThread != currentThread &&
                foregroundThread != targetThread && AttachThreadInput(currentThread, foregroundThread, true);
            try
            {
                _ = BringWindowToTop(window);
                _ = SetForegroundWindow(window);
            }
            finally
            {
                if (foregroundAttached)
                {
                    _ = AttachThreadInput(currentThread, foregroundThread, false);
                }
                if (targetAttached)
                {
                    _ = AttachThreadInput(currentThread, targetThread, false);
                }
            }
        }

        public IntPtr GetForegroundWindow() => GetForegroundWindowNative();
        public IntPtr GetRootWindow(IntPtr window) => ReadRootWindow(window);
        internal static IntPtr ReadRootWindow(IntPtr window) => GetAncestor(window, GaRoot);
        public void WaitForNextAttempt() => Thread.Sleep(75);

        // Exact target evidence comes from the visible active view, never LocationURL
        // or the first browser with a matching HWND (Windows11 tabs share that root).
        internal static bool IsExplorerRoot(IntPtr window) => IsWindowNative(window) &&
            (IsWindowVisible(window) || IsIconicNative(window)) && IsExplorerClass(window);
        internal static bool IsVisibleView(IntPtr view, IntPtr root) => view != IntPtr.Zero &&
            IsWindowNative(view) && IsWindowVisible(view) && GetAncestor(view, GaRoot) == root;


        private static bool IsExplorerClass(IntPtr window)
        {
            var className = new char[256];
            var length = GetClassName(window, className, className.Length);
            if (length <= 0)
            {
                return false;
            }
            var value = new string(className, 0, length);
            return value is "CabinetWClass" or "ExploreWClass";
        }

        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool IsWindowVisible(IntPtr window);
        [DllImport("user32.dll", EntryPoint = "IsWindow", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool IsWindowNative(IntPtr window);
        [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern int GetClassName(IntPtr window, [Out] char[] className, int maxCount);
        [DllImport("user32.dll", EntryPoint = "IsIconic", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool IsIconicNative(IntPtr window);
        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool ShowWindow(IntPtr window, int command);
        [DllImport("user32.dll", SetLastError = true)]
        private static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);
        [DllImport("kernel32.dll")]
        private static extern uint GetCurrentThreadId();
        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool AttachThreadInput(uint attach, uint attachTo, bool isAttached);
        [DllImport("user32.dll", EntryPoint = "GetForegroundWindow", SetLastError = true)]
        private static extern IntPtr GetForegroundWindowNative();
        [DllImport("user32.dll", SetLastError = true)]
        private static extern IntPtr GetAncestor(IntPtr window, uint flags);
        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool BringWindowToTop(IntPtr window);
        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool SetForegroundWindow(IntPtr window);
    }

}
