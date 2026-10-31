using System.Windows.Forms;

namespace NxTools.PlatformHost.Windows;

/// <summary>
/// Save-location dialog for the browser extension export. Cancelled dialogs
/// report success with empty data, matching the clipboard archive picker
/// convention the desktop already understands.
/// </summary>
internal static class BrowserExtensionExportDialog
{
    public static WindowsCommandResult PickDestination(string format) =>
        PickDestination(format, CancellationToken.None);

    public static WindowsCommandResult PickDestination(
        string format,
        CancellationToken cancellationToken)
    {
        if (cancellationToken.IsCancellationRequested)
        {
            return CancelledResult();
        }
        var isFirefox = string.Equals(
            format,
            BrowserExtensionPackageService.FirefoxFormat,
            StringComparison.OrdinalIgnoreCase);
        var isStore = string.Equals(format, BrowserExtensionPackageService.ChromeStoreFormat, StringComparison.OrdinalIgnoreCase);
        return StaDialogRunner.Run(
            "NxTools browser extension export",
            () =>
            {
                if (cancellationToken.IsCancellationRequested)
                {
                    return CancelledResult();
                }
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
                var result = dialog.ShowDialog() == DialogResult.OK
                    ? new WindowsCommandResult(true, Data: dialog.FileName)
                    : new WindowsCommandResult(true, Data: string.Empty);
                return cancellationToken.IsCancellationRequested ? CancelledResult() : result;
            },
            cancellationToken);
    }

    private static WindowsCommandResult CancelledResult() =>
        new(false, "Windows dialog operation was cancelled.");
}
