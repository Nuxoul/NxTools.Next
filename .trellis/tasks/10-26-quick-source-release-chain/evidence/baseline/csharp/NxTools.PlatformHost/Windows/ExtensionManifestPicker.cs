using System.ComponentModel;
using System.Windows.Forms;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal static class ExtensionManifestPicker
{
    internal sealed record DialogSettings(
        string InitialDirectory,
        string FileName,
        string Filter = "ZIP 扩展包 (*.zip)|*.zip",
        string Title = "导入扩展包",
        bool AutoUpgradeEnabled = true,
        bool CheckFileExists = true,
        bool CheckPathExists = true,
        bool Multiselect = false,
        bool RestoreDirectory = true);

    public static WindowsCommandResult Pick(
        PickExtensionManifestCommand command,
        CancellationToken cancellationToken = default)
    {
        // Do not enter even the STA/native runner after session cancellation.
        if (cancellationToken.IsCancellationRequested)
        {
            return CancelledResult();
        }
        return StaDialogRunner.Run(
            "NxTools extension manifest picker",
            () => PickCore(command, cancellationToken, File.Exists, Directory.Exists, ShowDialog),
            cancellationToken);
    }

    // Inject only native presentation/existence checks. This path never reads or interprets a manifest.
    internal static WindowsCommandResult PickCore(
        PickExtensionManifestCommand command,
        CancellationToken cancellationToken,
        Func<string, bool> fileExists,
        Func<string, bool> directoryExists,
        Func<DialogSettings, string?> showDialog)
    {
        if (cancellationToken.IsCancellationRequested)
        {
            return CancelledResult();
        }
        try
        {
            var currentPath = Environment.ExpandEnvironmentVariables(command.CurrentPath.Trim());
            var initialDirectory = directoryExists(currentPath)
                ? currentPath
                : Path.GetDirectoryName(currentPath) ?? string.Empty;
            var settings = new DialogSettings(
                directoryExists(initialDirectory) ? initialDirectory : string.Empty,
                IsZipPath(currentPath) && fileExists(currentPath) ? Path.GetFileName(currentPath) : string.Empty);
            if (cancellationToken.IsCancellationRequested)
            {
                return CancelledResult();
            }
            var selectedPath = showDialog(settings);
            if (cancellationToken.IsCancellationRequested)
            {
                return CancelledResult();
            }
            if (string.IsNullOrEmpty(selectedPath))
            {
                return new WindowsCommandResult(true, Data: string.Empty);
            }
            if (!IsZipPath(selectedPath) || !Path.IsPathFullyQualified(selectedPath) || !fileExists(selectedPath))
            {
                return new WindowsCommandResult(false, "请选择现有的 ZIP 扩展包文件。", NativeErrorCode: 2);
            }
            return cancellationToken.IsCancellationRequested
                ? CancelledResult()
                : new WindowsCommandResult(true, Data: Path.GetFullPath(selectedPath));
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            return CancelledResult();
        }
        catch (Win32Exception error)
        {
            return new WindowsCommandResult(false, "无法打开 ZIP 扩展包选择窗口。", NativeErrorCode: error.NativeErrorCode);
        }
        catch (Exception error) when (error is InvalidOperationException
                                      or IOException
                                      or UnauthorizedAccessException
                                      or ArgumentException or System.Runtime.InteropServices.ExternalException)
        {
            return new WindowsCommandResult(false, "无法选择 ZIP 扩展包文件。", NativeErrorCode: error.HResult);
        }
    }

    internal static string? ShowDialog(DialogSettings settings)
    {
        using var dialog = new OpenFileDialog
        {
            AutoUpgradeEnabled = settings.AutoUpgradeEnabled,
            CheckFileExists = settings.CheckFileExists,
            CheckPathExists = settings.CheckPathExists,
            FileName = settings.FileName,
            Filter = settings.Filter,
            InitialDirectory = settings.InitialDirectory,
            Multiselect = settings.Multiselect,
            RestoreDirectory = settings.RestoreDirectory,
            Title = settings.Title
        };
        return dialog.ShowDialog() == DialogResult.OK ? dialog.FileName : null;
    }

    private static bool IsZipPath(string path) =>
        string.Equals(Path.GetExtension(path), ".zip", StringComparison.OrdinalIgnoreCase);

    private static WindowsCommandResult CancelledResult() =>
        new(false, "Windows dialog operation was cancelled.", NativeErrorCode: 995);
}
