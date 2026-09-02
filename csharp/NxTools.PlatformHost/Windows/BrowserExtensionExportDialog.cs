using System.Windows.Forms;

namespace NxTools.PlatformHost.Windows;

/// <summary>
/// Save-location dialog for the browser extension export. Cancelled dialogs
/// report success with empty data, matching the clipboard archive picker
/// convention the desktop already understands.
/// </summary>
internal static class BrowserExtensionExportDialog
{
    public static WindowsCommandResult PickDestination(string format)
    {
        var isFirefox = string.Equals(
            format,
            BrowserExtensionPackageService.FirefoxFormat,
            StringComparison.OrdinalIgnoreCase);
        return StaDialogRunner.Run(
            "NxTools browser extension export",
            () =>
            {
                using var dialog = new SaveFileDialog
                {
                    AddExtension = true,
                    AutoUpgradeEnabled = true,
                    OverwritePrompt = true,
                    RestoreDirectory = true,
                    DefaultExt = isFirefox ? "xpi" : "crx",
                    FileName = isFirefox
                        ? BrowserExtensionPackageService.DefaultFirefoxExportFileName
                        : BrowserExtensionPackageService.DefaultExportFileName,
                    Filter = isFirefox
                        ? "NxTools Firefox 扩展 (*.xpi)|*.xpi"
                        : "NxTools Chrome/Edge 扩展 (*.crx)|*.crx",
                    Title = "导出浏览器扩展",
                };
                return dialog.ShowDialog() == DialogResult.OK
                    ? new WindowsCommandResult(true, Data: dialog.FileName)
                    : new WindowsCommandResult(true, Data: string.Empty);
            });
    }
}
