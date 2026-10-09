using System.Diagnostics;
using System.Runtime.InteropServices;
using Microsoft.Win32;

namespace NxTools.PlatformHost.Windows;

internal enum ShellTargetKind
{
    Open,
    SelectInExplorer,
    RunElevated,
    RegistryKey,
    SystemTool
}

internal sealed record ShellTarget(ShellTargetKind Kind, string Target)
{
    private static readonly HashSet<string> AllowedSystemTools = new(StringComparer.OrdinalIgnoreCase)
    {
        "msconfig.exe", "taskmgr.exe", "services.msc", "devmgmt.msc", "diskmgmt.msc",
        "eventvwr.msc", "compmgmt.msc", "gpedit.msc", "secpol.msc", "optionalfeatures.exe",
        "cleanmgr.exe", "dxdiag.exe", "winver.exe", "resmon.exe", "perfmon.exe", "mrt.exe",
        "cmd.exe", "powershell.exe", "regedit.exe", "control.exe"
    };

    private static readonly string[] RegistryRoots =
    [
        "HKEY_LOCAL_MACHINE", "HKEY_CURRENT_USER", "HKEY_CLASSES_ROOT", "HKEY_USERS",
        "HKEY_CURRENT_CONFIG"
    ];

    public static bool TryCreate(
        string target,
        string action,
        out ShellTarget resolved,
        out string? error)
    {
        resolved = null!;
        error = null;
        var value = target.Trim();
        if (value.Length == 0 || value.Contains('\r') || value.Contains('\n'))
        {
            error = "The shell target is empty or contains multiple lines.";
            return false;
        }

        var kind = action switch
        {
            "open" => ShellTargetKind.Open,
            "select_in_explorer" => ShellTargetKind.SelectInExplorer,
            "run_elevated" => ShellTargetKind.RunElevated,
            "registry_key" => ShellTargetKind.RegistryKey,
            "system_tool" => ShellTargetKind.SystemTool,
            _ => (ShellTargetKind?)null
        };
        if (kind is null)
        {
            error = "The shell target action is unsupported.";
            return false;
        }
        if (kind == ShellTargetKind.RegistryKey &&
            !RegistryRoots.Any(root => value.Equals(root, StringComparison.OrdinalIgnoreCase) ||
                value.StartsWith(root + "\\", StringComparison.OrdinalIgnoreCase)))
        {
            error = "The registry key root is unsupported.";
            return false;
        }
        if (kind == ShellTargetKind.SystemTool && !AllowedSystemTools.Contains(value))
        {
            error = "The requested system tool is not allowlisted.";
            return false;
        }
        if (kind == ShellTargetKind.Open && !IsSafeOpenTarget(value))
        {
            error = "The open target must be an HTTP(S) URL or an existing filesystem path.";
            return false;
        }
        if (kind == ShellTargetKind.SelectInExplorer &&
            !File.Exists(value) && !Directory.Exists(value))
        {
            error = "The selected Explorer target does not exist.";
            return false;
        }
        if (kind == ShellTargetKind.RunElevated && !IsSafeElevatedTarget(value))
        {
            error = "The elevated target must be an existing file or an allowlisted system tool.";
            return false;
        }

        resolved = new ShellTarget(kind.Value, value);
        return true;
    }

    private static bool IsSafeOpenTarget(string value) =>
        File.Exists(value) ||
        Directory.Exists(value) ||
        (Uri.TryCreate(value, UriKind.Absolute, out var uri) &&
         uri.Scheme is "http" or "https");

    private static bool IsSafeElevatedTarget(string value)
    {
        if (AllowedSystemTools.Contains(value))
        {
            return true;
        }
        if (!File.Exists(value))
        {
            return false;
        }
        var extension = Path.GetExtension(value);
        return extension.Equals(".exe", StringComparison.OrdinalIgnoreCase) ||
               extension.Equals(".lnk", StringComparison.OrdinalIgnoreCase);
    }

    public WindowsCommandResult Launch(Action<string>? stageChanged = null)
    {
        if (Kind == ShellTargetKind.SelectInExplorer)
        {
            return ExplorerShellActivation.LaunchAndActivate(Target, stageChanged);
        }

        ProcessStartInfo startInfo;
        switch (Kind)
        {
            case ShellTargetKind.RegistryKey:
                using (var key = Registry.CurrentUser.CreateSubKey(
                    @"Software\Microsoft\Windows\CurrentVersion\Applets\Regedit"))
                {
                    var existingLastKey = key?.GetValue("LastKey") as string;
                    key?.SetValue(
                        "LastKey",
                        BuildRegistryLastKey(Target, existingLastKey),
                        RegistryValueKind.String);
                }
                startInfo = new ProcessStartInfo("regedit.exe", "-m") { UseShellExecute = true };
                break;
            case ShellTargetKind.RunElevated:
                startInfo = new ProcessStartInfo(Target)
                {
                    UseShellExecute = true,
                    Verb = "runas"
                };
                break;
            default:
                startInfo = new ProcessStartInfo(Target) { UseShellExecute = true };
                break;
        }

        _ = Process.Start(startInfo);
        return new WindowsCommandResult(true);
    }

    internal static string BuildRegistryLastKey(string target, string? existingLastKey)
    {
        const string rootMarker = "\\HKEY_";
        var markerIndex = existingLastKey?.IndexOf(rootMarker, StringComparison.OrdinalIgnoreCase)
            ?? -1;
        var computerRoot = markerIndex > 0
            ? existingLastKey![..markerIndex]
            : "Computer";
        return $"{computerRoot}\\{target}";
    }
}


internal static class NativeMethods
{
    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern bool SetForegroundWindow(IntPtr hWnd);
}
