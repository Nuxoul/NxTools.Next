using System.Windows.Forms;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal static class ClipboardArchiveFilePicker
{
    private const string Filter = "NxTools 剪贴板压缩包 (*.nxtclip)|*.nxtclip";

    public static WindowsCommandResult Pick(PickClipboardArchivePathCommand command) =>
        Pick(command, CancellationToken.None);

    public static WindowsCommandResult Pick(
        PickClipboardArchivePathCommand command,
        CancellationToken cancellationToken)
        => BoundedStaOperationRunner.TryRun("NxTools clipboard archive picker", Timeout.InfiniteTimeSpan,
            token => StaDialogRunner.RunInline(() => PickOnStaThread(command, token), token, command.SourceWindowHandle, command.SourceProcessId),
            out WindowsCommandResult? result, out _, cancellationToken)
            ? result ?? new(false, "picker_failed") : new(false, "picker_busy", NativeErrorCode: 170);

    private static WindowsCommandResult PickOnStaThread(
        PickClipboardArchivePathCommand command,
        CancellationToken cancellationToken)
    {
        if (cancellationToken.IsCancellationRequested)
        {
            return CancelledResult();
        }
        try
        {
            var result = command.Mode.ToLowerInvariant() switch
            {
                "export" => PickExport(command, cancellationToken),
                "import" => PickImport(command, cancellationToken),
                _ => new WindowsCommandResult(false, $"Unsupported clipboard archive picker mode '{command.Mode}'.")
            };
            return cancellationToken.IsCancellationRequested ? CancelledResult() : result;
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            return CancelledResult();
        }
        catch (Exception error) when (error is InvalidOperationException or IOException or UnauthorizedAccessException)
        {
            return new WindowsCommandResult(false, "picker_failed", NativeErrorCode: error.HResult);
        }
    }

    private static WindowsCommandResult PickExport(
        PickClipboardArchivePathCommand command,
        CancellationToken cancellationToken)
    {
        if (cancellationToken.IsCancellationRequested)
        {
            return CancelledResult();
        }
        using var dialog = new SaveFileDialog
        {
            AddExtension = true,
            AutoUpgradeEnabled = true,
            CheckPathExists = true,
            DefaultExt = "nxtclip",
            FileName = SafeSuggestedName(command.SuggestedFileName),
            Filter = Filter,
            OverwritePrompt = true,
            RestoreDirectory = true,
            Title = "导出剪贴板压缩包"
        };
        var result = PickerOwner.Run(command.SourceWindowHandle, command.SourceProcessId, cancellationToken, owner =>
            dialog.ShowDialog(owner) == DialogResult.OK
                ? new WindowsCommandResult(true, Data: dialog.FileName)
                : new WindowsCommandResult(true, Data: string.Empty));
        return cancellationToken.IsCancellationRequested ? CancelledResult() : result;
    }

    private static WindowsCommandResult PickImport(PickClipboardArchivePathCommand command, CancellationToken cancellationToken)
    {
        if (cancellationToken.IsCancellationRequested)
        {
            return CancelledResult();
        }
        using var dialog = new OpenFileDialog
        {
            AddExtension = true,
            AutoUpgradeEnabled = true,
            CheckFileExists = true,
            CheckPathExists = true,
            DefaultExt = "nxtclip",
            Filter = Filter,
            Multiselect = false,
            RestoreDirectory = true,
            Title = "导入剪贴板压缩包"
        };
        var result = PickerOwner.Run(command.SourceWindowHandle, command.SourceProcessId, cancellationToken, owner =>
            dialog.ShowDialog(owner) == DialogResult.OK
                ? new WindowsCommandResult(true, Data: dialog.FileName)
                : new WindowsCommandResult(true, Data: string.Empty));
        return cancellationToken.IsCancellationRequested ? CancelledResult() : result;
    }

    private static WindowsCommandResult CancelledResult() =>
        new(false, "Windows dialog operation was cancelled.");

    private static string SafeSuggestedName(string value)
    {
        var name = Path.GetFileName(value.Trim());
        return string.IsNullOrWhiteSpace(name) ? "NxTools-clipboard.nxtclip" : name;
    }
}
