using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal static class ExtensionRuntimePicker
{
    internal static WindowsCommandResult Pick(PickExtensionRuntimeCommand command,
        CancellationToken cancellationToken = default, IPlatformLog? log = null, Guid correlationId = default)
    {
        var clock = Stopwatch.StartNew();
        ExtensionRuntimeDiagnostics.Write(log, "pick", "accepted", correlationId, clock);
        var result = cancellationToken.IsCancellationRequested
            ? new WindowsCommandResult(false, "extension_runtime_cancelled", NativeErrorCode: 995)
            : !PlatformCommandParser.IsExtensionRuntimeProgram(command.Program)
                ? new WindowsCommandResult(false, "extension_runtime_invalid_program", NativeErrorCode: 87)
                : BoundedStaOperationRunner.TryRun("NxTools extension runtime picker", Timeout.InfiniteTimeSpan, token =>
                    StaDialogRunner.RunInline(() => PickerOwner.Run(command.SourceWindowHandle, command.SourceProcessId, token, owner =>
                    {
                        ExtensionRuntimeDiagnostics.Write(log, "pick", "started", correlationId, clock);
                        return PickCore(command, token, File.Exists, Directory.Exists, settings => ExtensionManifestPicker.ShowDialog(settings, owner));
                    }), token), out WindowsCommandResult? picked, out _, cancellationToken)
                    ? picked ?? new(false, "extension_runtime_picker_failed", NativeErrorCode: 31)
                    : new(false, cancellationToken.IsCancellationRequested ? "extension_runtime_cancelled" : "extension_runtime_picker_busy", NativeErrorCode: cancellationToken.IsCancellationRequested ? 995 : 170);
        ExtensionRuntimeDiagnostics.Write(log, "pick", result.Success ? "completed" : result.NativeErrorCode == 995 ? "cancelled" : "failed",
            correlationId, clock, result.Success ? (string.IsNullOrEmpty(result.Data) ? "dismissed" : "selected") : "picker_failure", result.NativeErrorCode);
        return result;
    }

    internal static WindowsCommandResult PickCore(PickExtensionRuntimeCommand command, CancellationToken cancellationToken,
        Func<string, bool> fileExists, Func<string, bool> directoryExists,
        Func<ExtensionManifestPicker.DialogSettings, string?> showDialog)
    {
        if (cancellationToken.IsCancellationRequested) return Cancelled();
        if (!PlatformCommandParser.IsExtensionRuntimeProgram(command.Program))
            return new(false, "extension_runtime_invalid_program", NativeErrorCode: 87);
        try
        {
            var current = Environment.ExpandEnvironmentVariables(command.CurrentPath.Trim());
            var directory = directoryExists(current) ? current : Path.GetDirectoryName(current) ?? "";
            var settings = new ExtensionManifestPicker.DialogSettings(
                directoryExists(directory) ? directory : "",
                IsExe(current) && fileExists(current) ? Path.GetFileName(current) : "",
                Filter: "EXE (*.exe)|*.exe", Title: "选择扩展运行时");
            if (cancellationToken.IsCancellationRequested) return Cancelled();
            var selected = showDialog(settings);
            if (cancellationToken.IsCancellationRequested) return Cancelled();
            if (string.IsNullOrEmpty(selected)) return new(true, Data: "");
            if (!IsExe(selected) || !Path.IsPathFullyQualified(selected) || !fileExists(selected))
                return new(false, "extension_runtime_invalid_selection", NativeErrorCode: 2);
            return new(true, Data: Path.GetFullPath(selected));
        }
        catch (Win32Exception e) { return new(false, "extension_runtime_picker_failed", NativeErrorCode: e.NativeErrorCode); }
        catch (Exception e) when (e is ArgumentException or IOException or UnauthorizedAccessException or InvalidOperationException or ExternalException)
        { return new(false, "extension_runtime_picker_failed", NativeErrorCode: e.HResult); }
    }

    internal static bool IsExe(string path) => string.Equals(Path.GetExtension(path), ".exe", StringComparison.OrdinalIgnoreCase);
    private static WindowsCommandResult Cancelled() => new(false, "extension_runtime_cancelled", NativeErrorCode: 995);
}

internal static class ExtensionRuntimeDiagnostics
{
    internal static void Write(IPlatformLog? log, string operation, string phase, Guid correlationId,
        Stopwatch clock, string? reason = null, int? native = null)
    {
        try
        {
            log?.Write("info", "extension.runtime." + operation + "." + phase, correlationId,
                new Dictionary<string, object?> { ["elapsed_ms"] = clock.ElapsedMilliseconds,
                    ["reason"] = reason, ["native_error_code"] = native });
        }
        catch (Exception) { /* Observational only; no payload or exception text. */ }
    }
}
