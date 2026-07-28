namespace NxTools.PlatformHost.Windows;

internal sealed class ClipboardCaptureExclusions
{
    private readonly HashSet<string> _processNames;

    private ClipboardCaptureExclusions(HashSet<string> processNames)
    {
        _processNames = processNames;
        ProcessNames = processNames.Order(StringComparer.OrdinalIgnoreCase).ToArray();
    }

    public IReadOnlyList<string> ProcessNames { get; }

    public static ClipboardCaptureExclusions Create(IEnumerable<string> processNames)
    {
        var normalized = processNames
            .Select(Normalize)
            .Where(static name => name.Length > 0)
            .ToHashSet(StringComparer.OrdinalIgnoreCase);
        return new ClipboardCaptureExclusions(normalized);
    }

    public bool Contains(string processName) => _processNames.Contains(Normalize(processName));

    private static string Normalize(string processName)
    {
        var trimmed = processName.Trim();
        return trimmed.EndsWith(".exe", StringComparison.OrdinalIgnoreCase)
            ? trimmed[..^4]
            : trimmed;
    }
}
