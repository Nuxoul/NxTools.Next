using System.ComponentModel;
using System.Windows.Forms;
using NxTools.PlatformHost.Protocol;
using System.Runtime.InteropServices;

namespace NxTools.PlatformHost.Windows;

internal static class DirectoryPicker
{
    public static WindowsCommandResult Pick(PickDirectoryCommand command) =>
        Pick(command, CancellationToken.None);

    public static WindowsCommandResult Pick(
        PickDirectoryCommand command,
        CancellationToken cancellationToken) =>
        StaDialogRunner.Run(
            "NxTools directory picker",
            () => PickOnStaThread(command, cancellationToken),
            cancellationToken);

    private static WindowsCommandResult PickOnStaThread(
        PickDirectoryCommand command,
        CancellationToken cancellationToken)
    {
        if (cancellationToken.IsCancellationRequested)
        {
            return CancelledResult();
        }
        try
        {
            using var dialog = new FolderBrowserDialog
            {
                Description = string.IsNullOrWhiteSpace(command.Title)
                    ? "选择目录"
                    : command.Title.Trim(),
                InitialDirectory = Directory.Exists(command.CurrentDirectory)
                    ? command.CurrentDirectory
                    : string.Empty,
                ShowNewFolderButton = true,
                UseDescriptionForTitle = true
            };
            var result = dialog.ShowDialog(NativeForegroundWindow()) == DialogResult.OK
                ? new WindowsCommandResult(true, Data: dialog.SelectedPath)
                : new WindowsCommandResult(true, Data: string.Empty);
            return cancellationToken.IsCancellationRequested ? CancelledResult() : result;
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            return CancelledResult();
        }
        catch (Exception error)
        {
            return new WindowsCommandResult(false, error.Message);
        }
    }

    private static WindowsCommandResult CancelledResult() =>
        new(false, "Windows dialog operation was cancelled.");

    // FolderBrowserDialog opened without an owner on a background STA thread lands
    // behind the GPUI window: the click looks dead while the dialog waits for input
    // the user cannot see. Attaching it to the current foreground window keeps it on
    // top; if the foreground window goes away mid-run the dialog still shows.
    private static IWin32Window? NativeForegroundWindow()
    {
        var foreground = NativeMethods.GetForegroundWindow();
        return foreground == IntPtr.Zero ? null : new HandleWindow(foreground);
    }

    private sealed class HandleWindow(IntPtr handle) : IWin32Window
    {
        public IntPtr Handle => handle;
    }

    internal static class NativeMethods
    {
        [DllImport("user32.dll")]
        internal static extern IntPtr GetForegroundWindow();
    }
}
