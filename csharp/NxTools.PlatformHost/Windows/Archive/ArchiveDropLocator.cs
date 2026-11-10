using System.Diagnostics;

namespace NxTools.PlatformHost.Windows.Archive;

// Destination discovery for an archive dragged as one real folder. The dragged folder carries one
// hidden zero-byte marker with an operation-unique name; whichever receiver performs the copy
// (Explorer, Directory Opus, desktop) creates that marker inside the new folder. The folder that
// holds the copied marker is the extraction destination. No Shell extension, registry entry or
// receiver cooperation beyond an ordinary file copy is involved.
internal sealed class ArchiveDropLocator : IDisposable
{
    internal const string MarkerPrefix = ".nxtools-drop-";
    private readonly string marker;
    private readonly string staging;
    private readonly string wrapper;
    private readonly object gate = new();
    private readonly List<string> hits = [];
    private readonly List<FileSystemWatcher> watchers = [];

    internal string Staging => staging;
    internal string Folder => Path.Combine(staging, wrapper);
    internal string MarkerName => marker;

    private ArchiveDropLocator(string staging, string wrapper, string marker)
    { this.staging = staging; this.wrapper = wrapper; this.marker = marker; }

    // Creates the owned staging folder first, then starts watching, so our own creation is never a hit.
    internal static ArchiveDropLocator Create(Guid operation, string wrapper, string? tempRoot = null, IEnumerable<string>? roots = null)
    {
        var staging = Path.Combine(tempRoot ?? Path.GetTempPath(), "NxTools", "drop-" + operation.ToString("N"));
        var locator = new ArchiveDropLocator(Path.GetFullPath(staging), wrapper, MarkerPrefix + operation.ToString("N"));
        try {
            Directory.CreateDirectory(locator.Folder);
            var markerPath = Path.Combine(locator.Folder, locator.marker);
            File.WriteAllBytes(markerPath, []);
            File.SetAttributes(markerPath, FileAttributes.Hidden);
            foreach (var root in roots ?? WatchRoots()) locator.Watch(root);
            return locator;
        } catch {
            locator.Dispose();
            throw;
        }
    }

    private static IEnumerable<string> WatchRoots() =>
        DriveInfo.GetDrives().Where(drive => drive.DriveType is DriveType.Fixed or DriveType.Removable && drive.IsReady)
            .Select(drive => drive.RootDirectory.FullName);

    private void Watch(string root)
    {
        try {
            var watcher = new FileSystemWatcher(root, marker) {
                IncludeSubdirectories = true, NotifyFilter = NotifyFilters.FileName, InternalBufferSize = 65536
            };
            watcher.Created += (_, change) => Hit(change.FullPath);
            watcher.Renamed += (_, change) => Hit(change.FullPath);
            watcher.EnableRaisingEvents = true;
            watchers.Add(watcher);
        } catch (Exception failure) when (failure is IOException or UnauthorizedAccessException or ArgumentException) {
            // An unreadable volume only narrows discovery; the folder probe still applies.
        }
    }

    private void Hit(string path)
    {
        if (Inside(path, staging)) return;
        lock (gate) hits.Add(path);
    }

    private static bool Inside(string path, string root)
    {
        var full = Path.GetFullPath(path);
        return full.Equals(root, StringComparison.OrdinalIgnoreCase) ||
            full.StartsWith(root.TrimEnd(Path.DirectorySeparatorChar) + Path.DirectorySeparatorChar, StringComparison.OrdinalIgnoreCase);
    }

    // Folders that commonly receive drops and are cheap to inspect when the change journal buffer
    // overflowed: desktops plus every open Explorer window captured right after the drop.
    internal static IReadOnlyList<string> ProbeFolders(IEnumerable<string> explorerFolders) =>
        new[] { Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory),
                Environment.GetFolderPath(Environment.SpecialFolder.CommonDesktopDirectory) }
            .Concat(explorerFolders).Where(path => !string.IsNullOrEmpty(path))
            .Distinct(StringComparer.OrdinalIgnoreCase).ToArray();

    // Returns the destination folder that now holds the copied marker, or null when the receiver
    // did not copy it within the window (non-filesystem target, declined merge, cancelled copy).
    // The folder probe starts after the change notifications had a fair chance to report it.
    internal string? Wait(TimeSpan timeout, CancellationToken stop, Func<IReadOnlyList<string>> probeFolders)
    {
        var clock = Stopwatch.StartNew();
        var nextProbe = TimeSpan.FromSeconds(1);
        IReadOnlyList<string>? probes = null;
        while (true) {
            string[] snapshot;
            lock (gate) snapshot = hits.ToArray();
            foreach (var hit in snapshot.Reverse()) if (Accept(hit) is {} folder) return folder;
            if (clock.Elapsed >= nextProbe) {
                probes ??= probeFolders();
                foreach (var probe in probes) if (Probe(probe) is {} folder) return folder;
                nextProbe = clock.Elapsed + TimeSpan.FromMilliseconds(500);
            }
            if (clock.Elapsed >= timeout || stop.WaitHandle.WaitOne(100)) return null;
        }
    }

    // Every open Explorer window's filesystem folder, bounded and best-effort.
    internal static IReadOnlyList<string> ExplorerFolders()
    {
        var folders = new List<string>();
        object? shell = null;
        try {
            if (Type.GetTypeFromProgID("Shell.Application") is not {} type) return folders;
            shell = Activator.CreateInstance(type);
            dynamic windows = ((dynamic)shell!).Windows();
            var count = Math.Min((int)windows.Count, 64);
            for (var index = 0; index < count; index++) {
                try {
                    string? path = windows.Item(index)?.Document?.Folder?.Self?.Path;
                    if (!string.IsNullOrEmpty(path) && Path.IsPathFullyQualified(path)) folders.Add(path);
                } catch { }
            }
        } catch { }
        finally { if (shell is not null) try { System.Runtime.InteropServices.Marshal.FinalReleaseComObject(shell); } catch { } }
        return folders;
    }

    private string? Accept(string markerPath)
    {
        try {
            if (Path.GetFileName(markerPath) != marker || !File.Exists(markerPath)) return null;
            var folder = Path.GetDirectoryName(Path.GetFullPath(markerPath));
            if (folder is null || Inside(folder, staging) || !Directory.Exists(folder)) return null;
            var attributes = File.GetAttributes(folder);
            return (attributes & FileAttributes.ReparsePoint) != 0 ? null : folder;
        } catch (Exception failure) when (failure is IOException or UnauthorizedAccessException or ArgumentException or NotSupportedException) {
            return null;
        }
    }

    private string? Probe(string parent)
    {
        try {
            if (!Directory.Exists(parent) || Inside(parent, staging)) return null;
            foreach (var candidate in Directory.EnumerateDirectories(parent, wrapper + "*"))
                if (Accept(Path.Combine(candidate, marker)) is {} folder) return folder;
        } catch (Exception failure) when (failure is IOException or UnauthorizedAccessException or ArgumentException) { }
        return null;
    }

    // The receiver may still hold the just-copied marker briefly; retry before reporting failure.
    internal static bool RemoveMarker(string folder, string markerName, CancellationToken stop)
    {
        var path = Path.Combine(folder, markerName);
        for (var attempt = 0; attempt < 30; attempt++) {
            try {
                if (!File.Exists(path)) return true;
                File.SetAttributes(path, FileAttributes.Normal);
                File.Delete(path);
                return true;
            } catch (Exception failure) when (failure is IOException or UnauthorizedAccessException) {
                if (stop.WaitHandle.WaitOne(100)) return false;
            }
        }
        return false;
    }

    public void Dispose()
    {
        foreach (var watcher in watchers) { try { watcher.Dispose(); } catch { } }
        watchers.Clear();
        try { if (Directory.Exists(staging)) Directory.Delete(staging, true); } catch (IOException) { } catch (UnauthorizedAccessException) { }
    }
}
