using System.Windows.Forms;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal static class ClipboardArchiveFilePicker
{
    private const string Filter = "NxTools clipboard archive (*.nxtclip)|*.nxtclip";

    public static WindowsCommandResult Pick(PickClipboardArchivePathCommand command) =>
        Pick(command, CancellationToken.None);

    public static WindowsCommandResult Pick(
        PickClipboardArchivePathCommand command,
        CancellationToken cancellationToken)
        => StaDialogRunner.Run(
            "NxTools clipboard archive picker",
            () => PickOnStaThread(command, cancellationToken),
            cancellationToken);

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
                "export" => PickExport(command.SuggestedFileName, cancellationToken),
                "import" => PickImport(cancellationToken),
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
            return new WindowsCommandResult(false, error.Message);
        }
    }

    private static WindowsCommandResult PickExport(
        string suggestedFileName,
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
            FileName = SafeSuggestedName(suggestedFileName),
            Filter = Filter,
            OverwritePrompt = true,
            RestoreDirectory = true,
            Title = "导出剪贴板归档"
        };
        var result = dialog.ShowDialog() == DialogResult.OK
            ? new WindowsCommandResult(true, Data: dialog.FileName)
            : new WindowsCommandResult(true, Data: string.Empty);
        return cancellationToken.IsCancellationRequested ? CancelledResult() : result;
    }

    private static WindowsCommandResult PickImport(CancellationToken cancellationToken)
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
            Title = "导入剪贴板归档"
        };
        var result = dialog.ShowDialog() == DialogResult.OK
            ? new WindowsCommandResult(true, Data: dialog.FileName)
            : new WindowsCommandResult(true, Data: string.Empty);
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
