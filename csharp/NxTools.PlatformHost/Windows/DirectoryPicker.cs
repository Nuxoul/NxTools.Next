using System.ComponentModel;
using System.Windows.Forms;
using NxTools.PlatformHost.Protocol;
using System.Runtime.InteropServices;

namespace NxTools.PlatformHost.Windows;

internal static class DirectoryPicker
{
    public static WindowsCommandResult Pick(PickDirectoryCommand command) =>
        StaDialogRunner.Run("NxTools directory picker", () => PickOnStaThread(command));

    private static WindowsCommandResult PickOnStaThread(PickDirectoryCommand command)
    {
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
            return dialog.ShowDialog(NativeForegroundWindow()) == DialogResult.OK
                ? new WindowsCommandResult(true, Data: dialog.SelectedPath)
                : new WindowsCommandResult(true, Data: string.Empty);
        }
        catch (Exception error)
        {
            return new WindowsCommandResult(false, error.Message);
        }
    }

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
