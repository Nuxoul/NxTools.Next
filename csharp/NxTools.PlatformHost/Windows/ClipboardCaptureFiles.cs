using NxTools.PlatformHost.Diagnostics;

namespace NxTools.PlatformHost.Windows;

// Owns only files created for one capture, never borrowed source paths or shared caches.
internal sealed class ClipboardCaptureFiles(IPlatformLog? log = null) : IDisposable
{
    private HashSet<string> paths = new(StringComparer.OrdinalIgnoreCase);
    private int? lastCleanupError;

    internal string CreatePath(string directory, string extension)
    {
        Directory.CreateDirectory(directory);
        var path = Path.Combine(directory, Guid.NewGuid().ToString("N") + extension);
        Own(path);
        return path;
    }

    internal void Own(string path) => paths.Add(path);

    internal ClipboardCaptureFiles Detach()
    {
        var next = new ClipboardCaptureFiles(log) { paths = paths };
        paths = new(StringComparer.OrdinalIgnoreCase);
        return next;
    }
    internal void Transfer(IEnumerable<string>? transferred = null)
    {
        if (transferred is null) paths.Clear();
        else foreach (var path in transferred) paths.Remove(path);
    }

    internal void Remove(string path)
    {
        try { File.Delete(path); paths.Remove(path); }
        catch (Exception error) when (error is IOException or UnauthorizedAccessException) { lastCleanupError = error.HResult; }
    }

    public void Dispose()
    {
        foreach (var path in paths.ToArray()) Remove(path);
        if (paths.Count == 0) return;
        try
        {
            log?.Write("warning", "clipboard.staging.cleanup_deferred", fields: new Dictionary<string, object?>
            {
                ["failed_file_count"] = paths.Count,
                ["hresult"] = lastCleanupError
            });
        }
        catch { /* Cleanup diagnostics do not change capture results. */ }
    }
}
