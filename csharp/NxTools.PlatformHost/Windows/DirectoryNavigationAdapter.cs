using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal sealed record DirectoryNavigationCaptureAttempt(
    DirectoryNavigationContext? Context,
    WindowsCommandResult? Failure = null);

internal static class DirectoryNavigationTarget
{
    public static NavigateDirectoryCommand Normalize(
        NavigateDirectoryCommand command)
    {
        return TryResolve(command.TargetDirectory, out var resolved)
            ? command with { TargetDirectory = resolved }
            : command;
    }

    public static bool TryResolve(
        string? target,
        out string resolved)
    {
        resolved = string.Empty;
        var value = target?.Trim();
        if (string.IsNullOrWhiteSpace(value))
        {
            return false;
        }
        if (Directory.Exists(value))
        {
            resolved = Path.GetFullPath(value);
            return true;
        }
        if (!value.StartsWith("lib://", StringComparison.OrdinalIgnoreCase))
        {
            return false;
        }
        var libraryName = value["lib://".Length..]
            .Trim('\\', '/')
            .Trim();
        var profile = Environment.GetFolderPath(
            Environment.SpecialFolder.UserProfile);
        var candidate = libraryName.ToLowerInvariant() switch
        {
            "desktop" => Environment.GetFolderPath(
                Environment.SpecialFolder.Desktop),
            "documents" => Environment.GetFolderPath(
                Environment.SpecialFolder.MyDocuments),
            "pictures" => Environment.GetFolderPath(
                Environment.SpecialFolder.MyPictures),
            "music" => Environment.GetFolderPath(
                Environment.SpecialFolder.MyMusic),
            "videos" => Environment.GetFolderPath(
                Environment.SpecialFolder.MyVideos),
            "downloads" => Path.Combine(profile, "Downloads"),
            _ => string.Empty
        };
        if (string.IsNullOrWhiteSpace(candidate) ||
            !Directory.Exists(candidate))
        {
            return false;
        }
        resolved = Path.GetFullPath(candidate);
        return true;
    }

    public static IReadOnlyDictionary<string, object?> DiagnosticFields(
        string? target)
    {
        var value = target?.Trim() ?? string.Empty;
        var isLibrary = value.StartsWith(
            "lib://",
            StringComparison.OrdinalIgnoreCase);
        var segments = value.Split(
            ['\\', '/'],
            StringSplitOptions.RemoveEmptyEntries);
        return new Dictionary<string, object?>
        {
            ["target_kind"] = isLibrary
                ? "library_uri"
                : Path.IsPathRooted(value)
                    ? "filesystem_absolute"
                    : "other",
            ["target_root"] = isLibrary
                ? "lib://"
                : Path.GetPathRoot(value) ?? string.Empty,
            ["target_library"] = isLibrary
                ? value["lib://".Length..].Trim('\\', '/')
                : null,
            ["target_length"] = value.Length,
            ["target_segment_count"] = segments.Length,
            ["target_has_trailing_separator"] = value.EndsWith(
                '\\') || value.EndsWith('/')
        };
    }
}

internal static class DirectoryNavigationAdapter
{
    internal const string ExplorerHost = "explorer";
    internal const string DesktopHost = "desktop";
    internal const string FileDialogHost = "file_dialog";
    internal const string ThirdPartyFileManagerHost =
        "third_party_file_manager";

    public static DirectoryNavigationContext? TryCapture(
        ForegroundContext foreground,
        IElevatedDirectoryNavigationChannel? elevatedChannel = null)
    {
        if (elevatedChannel is not null &&
            RequiresElevatedHelper(foreground))
        {
            return elevatedChannel.TryCapture(foreground);
        }
        var local = TryCaptureLocal(foreground);
        if (local is not null)
        {
            return local;
        }
        return elevatedChannel is not null &&
            IsPotentialElevatedHost(foreground)
            ? elevatedChannel.TryCapture(foreground)
            : null;
    }

    internal static DirectoryNavigationContext? TryCaptureLocal(
        ForegroundContext foreground) =>
        TryCaptureLocalAttempt(foreground).Context;

    internal static DirectoryNavigationCaptureAttempt TryCaptureLocalAttempt(
        ForegroundContext foreground)
    {
        var desktop = DesktopWindowAdapter.TryCapture(foreground);
        if (desktop is not null)
        {
            return new DirectoryNavigationCaptureAttempt(
                new DirectoryNavigationContext(
                    DesktopHost,
                    desktop.WindowHandle,
                    desktop.CurrentDirectory));
        }
        var explorer = ExplorerWindowAdapter.TryCapture(foreground);
        if (explorer is not null)
        {
            return new DirectoryNavigationCaptureAttempt(
                new DirectoryNavigationContext(
                    ExplorerHost,
                    explorer.WindowHandle,
                    explorer.CurrentDirectory));
        }
        var fileDialog = FileDialogWindowAdapter.TryCapture(foreground);
        if (fileDialog is not null)
        {
            return new DirectoryNavigationCaptureAttempt(
                new DirectoryNavigationContext(
                    FileDialogHost,
                    fileDialog.WindowHandle,
                    fileDialog.CurrentDirectory));
        }
        var thirdPartyAttempt =
            ThirdPartyFileManagerAdapter.TryCapture(foreground);
        var thirdParty = thirdPartyAttempt.Context;
        return thirdParty is null
            ? new DirectoryNavigationCaptureAttempt(
                null,
                thirdPartyAttempt.Failure)
            : new DirectoryNavigationCaptureAttempt(
                new DirectoryNavigationContext(
                    ThirdPartyFileManagerHost,
                    thirdParty.WindowHandle,
                    thirdParty.CurrentDirectory,
                    thirdParty.HostContextId));
    }

    public static WindowsCommandResult Navigate(
        NavigateDirectoryCommand command,
        IElevatedDirectoryNavigationChannel? elevatedChannel = null)
    {
        var normalized = DirectoryNavigationTarget.Normalize(command);
        return ElevatedDirectoryNavigationClient.IsElevatedContext(
                normalized.HostContextId)
            ? elevatedChannel?.Navigate(normalized) ??
                new WindowsCommandResult(
                    false,
                    "The same-integrity directory helper is unavailable.")
            : NavigateLocal(normalized);
    }

    internal static WindowsCommandResult NavigateLocal(
        NavigateDirectoryCommand command)
    {
        command = DirectoryNavigationTarget.Normalize(command);
        return command.Host switch
        {
            ExplorerHost => ExplorerWindowAdapter.Navigate(
                new NavigateExplorerCommand(
                    command.WindowHandle,
                    command.TargetDirectory)),
            DesktopHost => DesktopWindowAdapter.Navigate(
                new NavigateDesktopCommand(
                    command.WindowHandle,
                    command.TargetDirectory)),
            FileDialogHost => FileDialogWindowAdapter.Navigate(
                new NavigateFileDialogCommand(
                    command.WindowHandle,
                    command.TargetDirectory)),
            ThirdPartyFileManagerHost => ThirdPartyFileManagerAdapter.Navigate(
                new NavigateThirdPartyFileManagerCommand(
                    command.WindowHandle,
                    command.TargetDirectory,
                    command.HostContextId)),
            _ => new WindowsCommandResult(
                false,
                $"Unsupported directory navigation host '{command.Host}'.")
        };
    }

    internal static bool RequiresElevatedHelper(
        ForegroundContext foreground) =>
        IsPotentialElevatedHost(foreground) &&
        ElevatedDirectoryHelperPeerVerifier.IsProcessElevated(
            foreground.ProcessId);

    internal static bool IsPotentialElevatedHost(
        ForegroundContext foreground)
    {
        var processName = Path.GetFileNameWithoutExtension(
            foreground.ProcessName);
        return processName.Equals("explorer", StringComparison.OrdinalIgnoreCase) ||
            ThirdPartyFileManagerAdapter.IsPotentialHost(foreground) ||
            FileDialogWindowAdapter.IsPotentialHost(foreground);
    }
}
