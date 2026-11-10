using System.ComponentModel;
using System.Windows.Forms;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal static class ProgramPicker
{
    private const string Filter =
        "程序 (*.exe;*.com;*.bat;*.cmd;*.ps1;*.lnk)|*.exe;*.com;*.bat;*.cmd;*.ps1;*.lnk|所有文件 (*.*)|*.*";

    public static WindowsCommandResult Pick(PickProgramCommand command) =>
        Pick(command, CancellationToken.None);

    public static WindowsCommandResult Pick(
        PickProgramCommand command,
        CancellationToken cancellationToken) =>
        BoundedStaOperationRunner.TryRun("NxTools program picker", Timeout.InfiniteTimeSpan,
            token => StaDialogRunner.RunInline(() => PickOnStaThread(command, token), token, command.SourceWindowHandle, command.SourceProcessId),
            out WindowsCommandResult? result, out _, cancellationToken)
            ? result ?? new(false, "picker_failed") : new(false, "picker_busy", NativeErrorCode: 170);

    private static WindowsCommandResult PickOnStaThread(
        PickProgramCommand command,
        CancellationToken cancellationToken)
    {
        if (cancellationToken.IsCancellationRequested)
        {
            return CancelledResult();
        }
        try
        {
            var currentPath = Environment.ExpandEnvironmentVariables(command.CurrentPath.Trim());
            var initialDirectory = Directory.Exists(currentPath)
                ? currentPath
                : Path.GetDirectoryName(currentPath) ?? string.Empty;
            using var dialog = new OpenFileDialog
            {
                AutoUpgradeEnabled = true,
                CheckFileExists = true,
                CheckPathExists = true,
                FileName = File.Exists(currentPath) ? Path.GetFileName(currentPath) : string.Empty,
                Filter = Filter,
                InitialDirectory = Directory.Exists(initialDirectory) ? initialDirectory : string.Empty,
                Multiselect = false,
                RestoreDirectory = true,
                Title = "选择命令程序"
            };
            var result = PickerOwner.Run(command.SourceWindowHandle, command.SourceProcessId, cancellationToken, owner =>
                dialog.ShowDialog(owner) == DialogResult.OK
                    ? new WindowsCommandResult(true, Data: dialog.FileName)
                    : new WindowsCommandResult(true, Data: string.Empty));
            return cancellationToken.IsCancellationRequested ? CancelledResult() : result;
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            return CancelledResult();
        }
        catch (Win32Exception error)
        {
            return new WindowsCommandResult(false, "picker_failed", NativeErrorCode: error.NativeErrorCode);
        }
        catch (Exception error) when (error is InvalidOperationException
                                      or IOException
                                      or UnauthorizedAccessException
                                      or ArgumentException)
        {
            return new WindowsCommandResult(false, "picker_failed", NativeErrorCode: error.HResult);
        }
    }

    private static WindowsCommandResult CancelledResult() =>
        new(false, "Windows dialog operation was cancelled.");
}
