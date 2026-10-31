namespace NxTools.ShellSelectionHelper;

/// <summary>
/// Dedicated x64 out-of-process COM local server for grouped Explorer selection (P207).
/// It runs only when COM activates it with the standard `-Embedding`/`/Embedding` argument from
/// an authorized LocalServer32 registration. Any other invocation exits without COM side effects.
/// Deployment identity is supplied by the existing release build; deployment is not registration.
/// </summary>
internal static class Program
{
    /// <summary>Generated from this build's Desktop output; unpackaged empty identity refuses.</summary>
    internal const string ExpectedDesktopSha256 = Generated.DesktopReleaseIdentity.Sha256;

    [STAThread]
    private static int Main(string[] args)
    {
        if (!IsComActivation(args)) return 2;
        var directory = AppContext.BaseDirectory;
        var forwarder = new DesktopIntentForwarder(
            new DesktopIdentity(directory, ExpectedDesktopSha256),
            new ProcessDesktopLauncher());
        return LocalServer.Run(forwarder, FileHelperLog.CreateDefault());
    }

    internal static bool IsComActivation(IReadOnlyList<string> args) =>
        args.Count == 1 && (args[0].Equals("-Embedding", StringComparison.OrdinalIgnoreCase) ||
                            args[0].Equals("/Embedding", StringComparison.OrdinalIgnoreCase));
}
