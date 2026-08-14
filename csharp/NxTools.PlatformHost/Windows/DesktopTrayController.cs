namespace NxTools.PlatformHost.Windows;

using System.Drawing;
using System.Windows.Forms;

internal sealed class DesktopTrayController : IDisposable
{
    private readonly Func<string, Action, Action, DesktopTrayCandidate> _createTrayIcon;
    private NotifyIcon? _trayIcon;
    private Icon? _ownedIcon;
    private ContextMenuStrip? _menu;

    internal NotifyIcon? TrayIcon => _trayIcon;

    internal DesktopTrayController()
        : this(WindowsPlatformAdapter.CreateTrayIcon)
    {
    }

    internal DesktopTrayController(
        Func<string, Action, Action, DesktopTrayCandidate> createTrayIcon)
    {
        _createTrayIcon = createTrayIcon;
    }

    internal WindowsCommandResult Configure(
        bool visible,
        string desktopExecutable,
        Action open,
        Action exit)
    {
        if (!visible) return DisposeOwnedResources();
        if (_trayIcon is not null)
        {
            try
            {
                _trayIcon.Visible = true;
                return new WindowsCommandResult(true);
            }
            catch (Exception error)
            {
                var cleanup = DisposeOwnedResources();
                return Failure(error, cleanup.NativeErrorCode);
            }
        }
        NotifyIcon? candidate = null;
        Icon? candidateIcon = null;
        ContextMenuStrip? candidateMenu = null;
        try
        {
            var created = _createTrayIcon(desktopExecutable, open, exit);
            candidate = created.TrayIcon;
            candidateIcon = created.OwnedIcon;
            candidateMenu = candidate.ContextMenuStrip;
            candidate.Visible = true;
            _trayIcon = candidate;
            _ownedIcon = candidateIcon;
            _menu = candidateMenu;
            return new WindowsCommandResult(true);
        }
        catch (Exception error)
        {
            var cleanupCode = DisposeCandidates(candidate, candidateMenu, candidateIcon);
            return Failure(error, cleanupCode);
        }
    }

    public void Dispose() => _ = DisposeOwnedResources();

    private WindowsCommandResult DisposeOwnedResources()
    {
        var tray = _trayIcon;
        var menu = _menu;
        var icon = _ownedIcon;
        _trayIcon = null;
        _menu = null;
        _ownedIcon = null;
        var failure = DisposeCandidates(tray, menu, icon);
        return failure is null
            ? new WindowsCommandResult(true)
            : new WindowsCommandResult(
                false,
                "Windows could not fully release the NxTools notification-area icon.",
                NativeErrorCode: failure);
    }

    private static int? DisposeCandidates(
        NotifyIcon? tray,
        ContextMenuStrip? menu,
        Icon? icon)
    {
        int? failure = null;
        if (tray is not null)
        {
            try { tray.Visible = false; } catch (Exception error) { failure ??= error.HResult; }
            try { tray.Dispose(); } catch (Exception error) { failure ??= error.HResult; }
        }
        try { menu?.Dispose(); } catch (Exception error) { failure ??= error.HResult; }
        try { icon?.Dispose(); } catch (Exception error) { failure ??= error.HResult; }
        return failure;
    }

    private static WindowsCommandResult Failure(Exception error, int? cleanupCode) =>
        new(
            false,
            "Windows could not create or show the NxTools notification-area icon.",
            NativeErrorCode: cleanupCode ?? (error is System.ComponentModel.Win32Exception native
                ? native.NativeErrorCode
                : error.HResult));
}
