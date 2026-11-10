using System.Runtime.InteropServices;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Windows.Forms;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record PickArchivePathCommand(long SourceHwnd);
internal sealed record ArchivePickerResult(string Status, [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] string? Path = null);

internal static class ArchivePathPicker
{
    private const string OperationName = "NxTools archive open picker";
    private sealed record Owner(IntPtr Handle) : IWin32Window;

    internal static WindowsCommandResult Pick(PickArchivePathCommand command, uint desktopProcessId, CancellationToken cancellationToken)
    {
        if (cancellationToken.IsCancellationRequested) return Result("cancelled");
        if (!PickerOwner.IsLive(command.SourceHwnd, desktopProcessId)) return Result("failed", native: 1400);
        var completed = BoundedStaOperationRunner.TryRun(
            OperationName, Timeout.InfiniteTimeSpan,
            token => StaDialogRunner.RunInline(() => Show(command, desktopProcessId, token), token, command.SourceHwnd, desktopProcessId),
            out WindowsCommandResult? result, out _, cancellationToken);
        return completed && result is not null ? result : Result(cancellationToken.IsCancellationRequested ? "cancelled" : "failed");
    }

    private static WindowsCommandResult Show(PickArchivePathCommand command, uint desktopProcessId, CancellationToken cancellationToken)
    {
        if (cancellationToken.IsCancellationRequested) return Result("cancelled");
        if (!PickerOwner.IsLive(command.SourceHwnd, desktopProcessId)) return Result("failed", native: 1400);
        using var dialog = new OpenFileDialog
        {
            Title = "打开压缩包", Multiselect = false, CheckFileExists = true,
            CheckPathExists = true, RestoreDirectory = true, AutoUpgradeEnabled = true,
            Filter = "压缩包|*.zip;*.7z;*.rar;*.tar;*.tgz;*.tbz;*.tbz2;*.txz;*.tzst;*.gz;*.bz2;*.xz;*.zst|所有文件|*.*"
        };
        var chosen = PickerOwner.Run(command.SourceHwnd, desktopProcessId, cancellationToken, owner =>
            dialog.ShowDialog(owner) == DialogResult.OK
                ? new WindowsCommandResult(true, Data: dialog.FileName)
                : new WindowsCommandResult(true, Data: string.Empty));
        if (!chosen.Success) return Result("failed", native: chosen.NativeErrorCode);
        return cancellationToken.IsCancellationRequested || string.IsNullOrEmpty(chosen.Data) ? Result("cancelled") : Result("selected", chosen.Data);
    }

    internal static WindowsCommandResult Result(string status, string? path = null, int? native = null) =>
        new(status != "failed", status == "failed" ? "archive_picker_failed" : null, NativeErrorCode: native,
            Data: JsonSerializer.Serialize(new ArchivePickerResult(status, path), JsonDefaults.Options));

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsWindow(IntPtr window);
}
