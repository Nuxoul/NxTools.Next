using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal sealed partial class WindowsPlatformAdapter
{
    internal Task<WindowsCommandResult> PrepareMainWindowAsync(PrepareMainWindowCommand command, Guid correlation, CancellationToken cancellation) =>
        PrepareMainWindowCoreAsync(command, correlation, _log, cancellation);
    internal static Task<WindowsCommandResult> PrepareMainWindowCoreAsync(PrepareMainWindowCommand command, Guid correlation,
        IPlatformLog log, CancellationToken cancellation, IMainWindowNative? native = null)
    {
        var started = Stopwatch.GetTimestamp();
        Diagnostic("accepted", null);
        return Task.Run(() => {
            var completed = BoundedStaOperationRunner.TryRun("NxTools.MainWindowPreparation", TimeSpan.FromSeconds(5), token => {
                Diagnostic("start", null);
                return MainWindowPreparation.Prepare(command, native ?? new NativeMethods.MainWindowNative(), token);
            }, out WindowsCommandResult? applied, out _, cancellation);
            var result = completed && applied is not null ? applied : new(false, "main_window_prepare_unavailable");
            Diagnostic("terminal", result);
            return result;
        }, CancellationToken.None);
        void Diagnostic(string stage, WindowsCommandResult? result)
        {
            try { log.Write(result?.Success == false ? "warning" : "info", "product.main_window.prepare." + stage, correlation,
                new Dictionary<string, object?> { ["window_handle"] = command.WindowHandle, ["requested_width_dip"] = command.Width,
                    ["requested_height_dip"] = command.Height, ["success"] = result?.Success,
                    ["reason"] = result?.Error,
                    ["native_error_code"] = result?.NativeErrorCode, ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds }); } catch { }
        }
    }
    internal static WindowsCommandResult PrepareMainWindowForTest(PrepareMainWindowCommand command) =>
        MainWindowPreparation.Prepare(command, new NativeMethods.MainWindowNative(), CancellationToken.None);

    private static partial class NativeMethods
    {
        [StructLayout(LayoutKind.Sequential)] private struct MainPoint { internal int X, Y; }
        [DllImport("user32.dll", EntryPoint = "MonitorFromPoint")] private static extern IntPtr MainMonitorFromPoint(MainPoint point, uint flags);
        [StructLayout(LayoutKind.Sequential)] private struct MainMonitorInfo { internal int Size; internal NativeWindowRectangle Monitor, Work; internal uint Flags; }
        [DllImport("user32.dll", EntryPoint = "GetMonitorInfoW", SetLastError = true)] private static extern bool MainGetMonitorInfo(IntPtr monitor, ref MainMonitorInfo info);
        [DllImport("user32.dll", EntryPoint = "GetWindowThreadProcessId")] private static extern uint MainGetWindowProcess(IntPtr window, out uint process);
        [DllImport("kernel32.dll", EntryPoint = "ProcessIdToSessionId")] private static extern bool MainProcessSession(uint process, out uint session);
        internal sealed class MainWindowNative : IMainWindowNative
        {
            public IDisposable EnterDpiScope() => new NotificationDpiScope();
            public bool Validate(PrepareMainWindowCommand command)
            {
                var window = new IntPtr(command.WindowHandle);
                return IsWindow(window) && !IsWindowVisible(window) && GetAncestorOrSelf(window) == window
                    && (GetWindowLongPtr(window, -16).ToInt64() & 0x40000000L) == 0
                    && MainGetWindowProcess(window, out var process) != 0 && process == command.SourceProcessId
                    && MainProcessSession(process, out var session) && MainProcessSession((uint)Environment.ProcessId, out var own) && session == own;
            }
            public NativeWindowRectangle PrimaryWorkArea()
            {
                var monitor = MainMonitorFromPoint(default, 1);
                var info = new MainMonitorInfo { Size = Marshal.SizeOf<MainMonitorInfo>() };
                if (monitor == IntPtr.Zero || !MainGetMonitorInfo(monitor, ref info)) Throw();
                return info.Work;
            }
            public void Stage(long window, NativeWindowRectangle work)
            {
                if (!SetWindowPos(new IntPtr(window), IntPtr.Zero, work.Left, work.Top, Math.Min(64, work.Width), Math.Min(64, work.Height),
                    SwpNoZOrder | SwpNoActivate)) Throw();
            }
            public uint Dpi(long window) => GetDpiForWindow(new IntPtr(window));
            public void Apply(long windowHandle, NativeWindowRectangle clientTarget)
            {
                var window = new IntPtr(windowHandle);
                if (!AnchoredWindowPlacement.TryGetWindowRectangle(window, out var outer)) Throw();
                if (!TryGetClientScreenRectangle(window, out var client)) Throw();
                var insets = NativeWindowInsets.From(outer, client);
                if (!SetWindowPos(window, IntPtr.Zero, clientTarget.Left - insets.Left, clientTarget.Top - insets.Top,
                    checked(clientTarget.Width + insets.Left + insets.Right), checked(clientTarget.Height + insets.Top + insets.Bottom),
                    SwpNoZOrder | SwpNoActivate | SwpFrameChanged)) Throw();
            }
            public MainWindowReadback Read(long windowHandle)
            {
                var window = new IntPtr(windowHandle);
                if (!TryGetClientScreenRectangle(window, out var client)) Throw();
                return new(client, GetDpiForWindow(window), IsWindowVisible(window));
            }
            private static void Throw() => throw new Win32Exception(Marshal.GetLastWin32Error());
        }
    }
}
