using System.Windows.Forms;

namespace NxTools.PlatformHost.Windows;

/// <summary>
/// One-shot error dialog mode (`--error-dialog &lt;report-path&gt;`): the Rust
/// desktop writes a startup failure report into its log directory and asks this
/// host to surface it as a native dialog, because the desktop itself is a
/// GUI-subsystem binary whose startup failures would otherwise be invisible.
/// </summary>
internal static class ErrorDialog
{
    private const int MaxMessageLength = 4000;

    public static bool TryParseArguments(string[] args, out string? reportPath)
    {
        reportPath = null;
        for (var index = 0; index + 1 < args.Length; index++)
        {
            if (!string.Equals(args[index], "--error-dialog", StringComparison.Ordinal))
            {
                continue;
            }
            var candidate = args[index + 1];
            if (string.IsNullOrWhiteSpace(candidate) || !File.Exists(candidate))
            {
                // Recognized mode with an unusable report: stay silent instead
                // of showing a second error about the error report.
                return true;
            }
            reportPath = candidate;
            return true;
        }
        return false;
    }

    public static string ReadMessage(string reportPath)
    {
        var message = File.ReadAllText(reportPath);
        return message.Length <= MaxMessageLength
            ? message
            : message[..(MaxMessageLength - 2)] + "\n…";
    }

    public static int Show(string reportPath)
    {
        var message = ReadMessage(reportPath);
        MessageBox.Show(message, "NxTools", MessageBoxButtons.OK, MessageBoxIcon.Error);
        return 0;
    }
}
