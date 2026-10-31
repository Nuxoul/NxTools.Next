using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal sealed partial class WindowsPlatformAdapter
{
    public Task<WindowsCommandResult> PrepareNotificationWindowAsync(PrepareNotificationWindowCommand command,
        Guid correlationId, CancellationToken cancellationToken) =>
        PrepareNotificationWindowCoreAsync(command, correlationId, _log, cancellationToken);

    internal static Task<WindowsCommandResult> PrepareNotificationWindowCoreAsync(PrepareNotificationWindowCommand command,
        Guid correlationId, IPlatformLog log, CancellationToken cancellationToken, INotificationWindowNative? native = null)
    {
        var started = Stopwatch.GetTimestamp();
        WriteNotificationDiagnostic(log, "accepted", correlationId, started, null);
        return Task.Run(() =>
        {
            var completed = BoundedStaOperationRunner.TryRun("NxTools.NotificationWindowPreparation", TimeSpan.FromSeconds(5),
                token =>
                {
                    WriteNotificationDiagnostic(log, "start", correlationId, started, null);
                    return NotificationWindowPlacement.Prepare(command, native ?? new NativeMethods.NotificationNative(), token);
                }, out WindowsCommandResult? applied, out _, cancellationToken);
            var result = completed && applied is not null ? applied :
                new WindowsCommandResult(false, "Notification preparation was cancelled, unavailable or timed out.");
            WriteNotificationDiagnostic(log, "terminal", correlationId, started, result);
            return result;
        }, CancellationToken.None);
    }

    internal static WindowsCommandResult PrepareNotificationWindowForTest(PrepareNotificationWindowCommand command) =>
        NotificationWindowPlacement.Prepare(command, new NativeMethods.NotificationNative(), CancellationToken.None);

    private static void WriteNotificationDiagnostic(IPlatformLog log, string stage, Guid correlationId,
        long started, WindowsCommandResult? result)
    {
        try
        {
            log.Write(result?.Success == false ? "warning" : "info", "ui.notification.surface." + stage, correlationId,
                new Dictionary<string, object?> { ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds,
                    ["success"] = result?.Success, ["native_error_code"] = result?.NativeErrorCode });
        }
        catch { /* Observational diagnostics cannot change native preparation. */ }
    }

    private static partial class NativeMethods
    {
        private const long NotificationToolWindow = 0x80;
        private const long NotificationAppWindow = 0x40000;
        private const long NotificationChildWindow = 0x40000000;

        [DllImport("user32.dll", EntryPoint = "GetWindowThreadProcessId")]
        private static extern uint NotificationGetWindowProcess(IntPtr window, out uint process);
        [DllImport("kernel32.dll", EntryPoint = "ProcessIdToSessionId")]
        private static extern bool NotificationProcessSession(uint process, out uint session);
        [DllImport("user32.dll", EntryPoint = "SetThreadDpiAwarenessContext", SetLastError = true)]
        private static extern IntPtr NotificationSetDpiContext(IntPtr context);
        [DllImport("user32.dll", EntryPoint = "MonitorFromWindow")]
        private static extern IntPtr NotificationMonitorFromWindow(IntPtr window, uint flags);
        [StructLayout(LayoutKind.Sequential)]
        private struct NotificationMonitorInfo
        {
            internal int Size;
            internal NativeWindowRectangle Monitor, Work;
            internal uint Flags;
        }
        [DllImport("user32.dll", EntryPoint = "GetMonitorInfoW", SetLastError = true)]
        private static extern bool NotificationGetMonitorInfo(IntPtr monitor, ref NotificationMonitorInfo info);

        internal sealed class NotificationNative : INotificationWindowNative
        {
            public IDisposable EnterDpiScope() => new NotificationDpiScope();
            public bool ValidateWindows(PrepareNotificationWindowCommand command)
            {
                var target = new IntPtr(command.WindowHandle);
                var source = new IntPtr(command.SourceWindowHandle);
                var root = GetAncestorOrSelf(source);
                var extended = GetWindowLongPtr(target, GwlExStyle).ToInt64();
                var visiblePolicy = command.KeepVisible
                    ? IsWindowVisible(target) && (extended & (WsExNoActivate | WsExTopmost | NotificationToolWindow)) == (WsExNoActivate | WsExTopmost | NotificationToolWindow)
                        && (GetWindowLongPtr(target, -16).ToInt64() & 0x00CF0000L) == 0
                    : !IsWindowVisible(target);
                return target != source && target != root && IsWindow(target) && IsWindow(source) && IsWindow(root) &&
                    visiblePolicy && (GetWindowLongPtr(target, -16).ToInt64() & NotificationChildWindow) == 0 &&
                    NotificationGetWindowProcess(target, out var targetProcess) != 0 && targetProcess == command.SourceProcessId &&
                    NotificationGetWindowProcess(source, out var sourceProcess) != 0 && sourceProcess == command.SourceProcessId &&
                    NotificationGetWindowProcess(root, out var rootProcess) != 0 && rootProcess == command.SourceProcessId &&
                    NotificationProcessSession(sourceProcess, out var sourceSession) && sourceSession != 0 &&
                    NotificationProcessSession((uint)Environment.ProcessId, out var hostSession) && sourceSession == hostSession;
            }

            public NativeWindowRectangle GetWorkArea(long source)
            {
                var monitor = NotificationMonitorFromWindow(GetAncestorOrSelf(new IntPtr(source)), MonitorDefaultToNearest);
                var info = new NotificationMonitorInfo { Size = Marshal.SizeOf<NotificationMonitorInfo>() };
                if (monitor == IntPtr.Zero || !NotificationGetMonitorInfo(monitor, ref info)) ThrowNative();
                return info.Work;
            }

            public void SetPassivePolicy(long target)
            {
                var window = new IntPtr(target);
                var style = GetWindowLongPtr(window, -16).ToInt64();
                Marshal.SetLastPInvokeError(0);
                var oldStyle = SetWindowLongPtr(window, -16, new IntPtr((style | unchecked((long)0x80000000)) & ~0x00CF0000L));
                if (oldStyle == IntPtr.Zero && Marshal.GetLastPInvokeError() != 0) ThrowNative();
                var shadow = SetWindowShadow(window, false);
                if (!shadow.Success) throw new Win32Exception(shadow.NativeErrorCode ?? 0);
                var extended = GetWindowLongPtr(window, GwlExStyle).ToInt64();
                Marshal.SetLastPInvokeError(0);
                var previous = SetWindowLongPtr(window, GwlExStyle,
                    new IntPtr((extended | WsExNoActivate | NotificationToolWindow) & ~NotificationAppWindow));
                if (previous == IntPtr.Zero && Marshal.GetLastPInvokeError() != 0) ThrowNative();
            }

            public void PrepareMonitor(long target, NativeWindowRectangle workArea)
            {
                // Hidden GPUI HWNDs may retain CW_USEDEFAULT/full-sized native bounds. Stage a tiny hidden rectangle,
                // not just an origin with NOSIZE, so the nearest monitor really becomes the source monitor before DPI.
                if (!SetWindowPos(new IntPtr(target), new IntPtr(-1), workArea.Left, workArea.Top,
                    Math.Min(64, workArea.Width), Math.Min(64, workArea.Height), SwpNoActivate | SwpFrameChanged)) ThrowNative();
            }

            public uint GetDpi(long target) => GetDpiForWindow(new IntPtr(target));

            public void ApplyClientBounds(long target, NativeWindowRectangle requested)
            {
                var window = new IntPtr(target);
                if (!AnchoredWindowPlacement.TryGetWindowRectangle(window, out var outer)) ThrowNative();
                if (!TryGetClientScreenRectangle(window, out var client)) ThrowNative();
                var insets = NativeWindowInsets.From(outer, client);
                if (!SetWindowPos(window, new IntPtr(-1), checked(requested.Left - insets.Left), checked(requested.Top - insets.Top),
                    checked(requested.Width + insets.Left + insets.Right), checked(requested.Height + insets.Top + insets.Bottom),
                    SwpNoActivate | SwpFrameChanged)) ThrowNative();
            }

            public NotificationWindowReadback Readback(long target)
            {
                var window = new IntPtr(target);
                if (!TryGetClientScreenRectangle(window, out var client)) ThrowNative();
                var extended = GetWindowLongPtr(window, GwlExStyle).ToInt64();
                return new(client, GetDpiForWindow(window), IsWindowVisible(window), (extended & WsExNoActivate) != 0,
                    (extended & NotificationToolWindow) != 0 && (extended & NotificationAppWindow) == 0, (extended & WsExTopmost) != 0);
            }

            private static void ThrowNative() => throw new Win32Exception(Marshal.GetLastWin32Error());
        }

        private sealed class NotificationDpiScope : IDisposable
        {
            private readonly IntPtr _previous;
            internal NotificationDpiScope()
            {
                _previous = NotificationSetDpiContext(new IntPtr(-4)); // Per-monitor-v2, device-pixel APIs.
                if (_previous == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
            }
            public void Dispose() { if (_previous != IntPtr.Zero) _ = NotificationSetDpiContext(_previous); }
        }
    }
}
