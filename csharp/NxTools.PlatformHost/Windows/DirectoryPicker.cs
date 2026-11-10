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
        BoundedStaOperationRunner.TryRun("NxTools directory picker", Timeout.InfiniteTimeSpan,
            token => StaDialogRunner.RunInline(() => PickOnStaThread(command, token), token, command.SourceWindowHandle, command.SourceProcessId),
            out WindowsCommandResult? result, out _, cancellationToken)
            ? result ?? new(false, "picker_failed") : new(false, "picker_busy", NativeErrorCode: 170);

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
            var result = PickerOwner.Run(command.SourceWindowHandle, command.SourceProcessId, cancellationToken, owner =>
                dialog.ShowDialog(owner) == DialogResult.OK
                    ? new WindowsCommandResult(true, Data: dialog.SelectedPath)
                    : new WindowsCommandResult(true, Data: string.Empty));
            return cancellationToken.IsCancellationRequested ? CancelledResult() : result;
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            return CancelledResult();
        }
        catch (Exception error)
        {
            return new WindowsCommandResult(false, "picker_failed", NativeErrorCode:
                error is Win32Exception native ? native.NativeErrorCode : error.HResult);
        }
    }

    private static WindowsCommandResult CancelledResult() =>
        new(false, "Windows dialog operation was cancelled.");

}
