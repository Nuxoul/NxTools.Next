using System.Diagnostics;
using System.Text;
using System.Text.Json;
using System.Windows.Forms;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal sealed record PickMarkdownDocumentResult(string[] Paths);

internal static class MarkdownDocumentPicker
{
    private const string PickerThreadName = "NxTools Markdown picker";
    private const int MaxPaths = 32;
    // Even at seven encoded bytes per input byte (nested JSON escaping), this plus
    // the bounded vector syntax and envelope stays below the original 1 MiB frame limit.
    private const int MaxPathBytes = 128 * 1024;

    internal static WindowsCommandResult Pick(PickMarkdownDocumentCommand command, Guid correlationId,
        IPlatformLog log, CancellationToken cancellationToken,
        Func<OpenFileDialog, IWin32Window, string[]>? present = null)
    {
        var started = Stopwatch.GetTimestamp();
        var selectedCount = 0;
        void Diagnostic(string stage, string? category = null, int count = 0, int? nativeErrorCode = null)
        {
            try
            {
                log.Write("info", "markdown.reader.picker." + stage, correlationId, new Dictionary<string, object?>
                {
                    ["category"] = category,
                    ["count"] = count,
                    ["native_error_code"] = nativeErrorCode,
                    ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds
                });
            }
            catch { /* Diagnostics must not change picker behavior. */ }
        }

        WindowsCommandResult Complete(WindowsCommandResult result)
        {
            var count = result.Success ? selectedCount : 0;
            Diagnostic("terminal", result.Success ? (count == 0 ? "cancelled" : "selected") : result.Error,
                count, result.NativeErrorCode);
            return result;
        }

        Diagnostic("accepted");
        if (cancellationToken.IsCancellationRequested)
            return Complete(new(false, "picker_cancelled", NativeErrorCode: 995));

        // The shared parser admits only positive, native-representable HWNDs.
        var hwnd = (long)command.SourceWindowHandle;
        var ran = BoundedStaOperationRunner.TryRun(PickerThreadName, Timeout.InfiniteTimeSpan,
            token => StaDialogRunner.RunInline(() => PickerOwner.Run(hwnd, command.SourceProcessId, token, owner =>
            {
                using var dialog = new OpenFileDialog
                {
                    Title = "打开 Markdown 文档",
                    Filter = "Markdown 文档 (*.md;*.markdown)|*.md;*.markdown",
                    Multiselect = true,
                    CheckFileExists = true,
                    CheckPathExists = true,
                    AutoUpgradeEnabled = true,
                    RestoreDirectory = true
                };
                Diagnostic("start");
                var paths = present is null
                    ? (dialog.ShowDialog(owner) == DialogResult.OK ? dialog.FileNames : [])
                    : present(dialog, owner);
                if (token.IsCancellationRequested)
                    return new(false, "picker_cancelled", NativeErrorCode: 995);
                if (ValidateSelection(paths) is { } error)
                    return new(false, error);
                selectedCount = paths.Length;
                return new(true, Data: JsonSerializer.Serialize(new PickMarkdownDocumentResult(paths), JsonDefaults.Options));
            }), token, hwnd, command.SourceProcessId), out WindowsCommandResult? result, out _, cancellationToken);

        if (cancellationToken.IsCancellationRequested)
            return Complete(new(false, "picker_cancelled", NativeErrorCode: 995));
        if (!ran || result is null)
            return Complete(new(false, "picker_busy", NativeErrorCode: 170));
        if (!result.Success && result.Error is not ("picker_invalid_owner" or "picker_limit" or "picker_invalid_selection"))
            result = new(false, "picker_failed", NativeErrorCode: result.NativeErrorCode);
        return Complete(result);
    }

    internal static WindowsCommandResult PickExport(PickMarkdownExportCommand command, Guid correlationId,
        IPlatformLog log, CancellationToken cancellationToken,
        Func<SaveFileDialog, IWin32Window, string?>? present = null)
    {
        var started = Stopwatch.GetTimestamp();
        void Diagnostic(string stage, string? category = null, int? nativeCode = null) =>
            WriteExportDiagnostic(log, correlationId, stage, category, nativeCode, Stopwatch.GetElapsedTime(started).TotalMilliseconds);

        Diagnostic("accepted");
        var hwnd = (long)command.SourceWindowHandle;
        WindowsCommandResult? result = null;
        var ran = !cancellationToken.IsCancellationRequested &&
            BoundedStaOperationRunner.TryRun(PickerThreadName, Timeout.InfiniteTimeSpan,
                token => StaDialogRunner.RunInline(() => PickerOwner.Run(hwnd, command.SourceProcessId, token, owner =>
                {
                    var pdf = command.Format == MarkdownExportFormat.Pdf;
                    var extension = pdf ? ".pdf" : ".html";
                    var suggested = string.IsNullOrWhiteSpace(command.SuggestedName) ? "Markdown" : command.SuggestedName;
                    using var dialog = new SaveFileDialog
                    {
                        Title = pdf ? "导出 PDF" : "导出 HTML",
                        Filter = pdf ? "PDF 文档 (*.pdf)|*.pdf" : "HTML 文档 (*.html)|*.html",
                        DefaultExt = extension[1..],
                        FileName = suggested.EndsWith(extension, StringComparison.OrdinalIgnoreCase) ? suggested : suggested + extension,
                        AddExtension = true,
                        CheckPathExists = true,
                        OverwritePrompt = true,
                        AutoUpgradeEnabled = true,
                        RestoreDirectory = true
                    };
                    Diagnostic("start");
                    var path = present is null
                        ? (dialog.ShowDialog(owner) == DialogResult.OK ? dialog.FileName : null)
                        : present(dialog, owner);
                    if (path is null)
                        return new(true);
                    if (path.Length >= 32767 || path.Contains('\0') || !Path.IsPathFullyQualified(path) ||
                        !string.Equals(Path.GetExtension(path), extension, StringComparison.OrdinalIgnoreCase))
                        return new(false, "picker_invalid_selection");
                    return new(true, Data: path);
                }), token, hwnd, command.SourceProcessId), out result, out _, cancellationToken);

        if (cancellationToken.IsCancellationRequested)
            result = new(false, "picker_cancelled", NativeErrorCode: 995);
        else if (!ran || result is null)
            result = new(false, "picker_busy", NativeErrorCode: 170);
        else if (!result.Success && result.Error is not ("picker_invalid_owner" or "picker_invalid_selection" or "picker_cancelled"))
            result = new(false, "picker_failed", NativeErrorCode: result.NativeErrorCode);
        Diagnostic("terminal", result.Success ? (result.Data is null ? "cancelled" : "selected") : result.Error, result.NativeErrorCode);
        return result;
    }

    internal static void WriteExportDiagnostic(IPlatformLog log, Guid id, string stage,
        string? category, int? nativeCode, double elapsedMs)
    {
        try
        {
            log.Write("info", "markdown.reader.export_picker." + stage, id, new Dictionary<string, object?>
            {
                ["category"] = category, ["native_error_code"] = nativeCode, ["elapsed_ms"] = elapsedMs
            });
        }
        catch { /* Diagnostics must not change picker behavior. */ }
    }

    private static string? ValidateSelection(string[] paths)
    {
        if (paths.Length > MaxPaths)
            return "picker_limit";
        var remaining = MaxPathBytes;
        foreach (var path in paths)
        {
            if (string.IsNullOrEmpty(path) || path.Contains('\0') || !Path.IsPathFullyQualified(path))
                return "picker_invalid_selection";
            var extension = Path.GetExtension(path);
            if (!string.Equals(extension, ".md", StringComparison.OrdinalIgnoreCase) &&
                !string.Equals(extension, ".markdown", StringComparison.OrdinalIgnoreCase))
                return "picker_invalid_selection";
            remaining -= Encoding.UTF8.GetByteCount(path);
            if (remaining < 0)
                return "picker_limit";
        }
        return null;
    }
}
