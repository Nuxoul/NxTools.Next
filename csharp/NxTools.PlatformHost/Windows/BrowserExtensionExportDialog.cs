using System.Windows.Forms;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

/// <summary>
/// Save-location dialog for the browser extension export. Cancelled dialogs
/// report success with empty data, matching the clipboard archive picker
/// convention the desktop already understands.
/// </summary>
internal static class BrowserExtensionExportDialog
{
    public static WindowsCommandResult PickDestination(PickBrowserExtensionExportPathCommand command,
        CancellationToken cancellationToken)
    {
        if (cancellationToken.IsCancellationRequested)
        {
            return CancelledResult();
        }
        var format = BrowserExtensionPackageService.NormalizeFormat(command.Format);
        var isFirefox = string.Equals(format, BrowserExtensionPackageService.FirefoxFormat, StringComparison.OrdinalIgnoreCase);
        var isStore = string.Equals(format, BrowserExtensionPackageService.ChromeStoreFormat, StringComparison.OrdinalIgnoreCase);
        var completed = BoundedStaOperationRunner.TryRun("NxTools browser extension export picker", Timeout.InfiniteTimeSpan,
            token => StaDialogRunner.RunInline(() => PickerOwner.Run(command.SourceWindowHandle, command.SourceProcessId, token, owner =>
            {
                using var dialog = new SaveFileDialog
                {
                    AddExtension = true,
                    AutoUpgradeEnabled = true,
                    OverwritePrompt = true,
                    RestoreDirectory = true,
                    DefaultExt = isStore ? "zip" : isFirefox ? "xpi" : "crx",
                    FileName = isStore ? BrowserExtensionPackageService.DefaultStoreExportFileName : isFirefox
                        ? BrowserExtensionPackageService.DefaultFirefoxExportFileName
                        : BrowserExtensionPackageService.DefaultExportFileName,
                    Filter = isStore ? "Chrome 商店上传包 (*.zip)|*.zip" : isFirefox
                        ? "NxTools Firefox 扩展 (*.xpi)|*.xpi"
                        : "NxTools Chrome/Edge 扩展 (*.crx)|*.crx",
                    Title = "导出浏览器扩展",
                };
                return dialog.ShowDialog(owner) == DialogResult.OK
                    ? new WindowsCommandResult(true, Data: dialog.FileName)
                    : new WindowsCommandResult(true, Data: string.Empty);
            }), token), out WindowsCommandResult? result, out _, cancellationToken);
        return completed && result is not null ? result : new(false, cancellationToken.IsCancellationRequested ? "picker_cancelled" : "picker_busy", NativeErrorCode: cancellationToken.IsCancellationRequested ? 995 : 170);
    }

    private static WindowsCommandResult CancelledResult() =>
        new(false, "Windows dialog operation was cancelled.");
}
