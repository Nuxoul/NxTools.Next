using System.Security.Cryptography;
using System.Collections.Concurrent;
using System.Text;

namespace NxTools.PlatformHost.Windows;

internal sealed record IconCacheKey(
    string TargetPath,
    string Selector,
    uint Size,
    long LastWriteTicks,
    string TargetType)
{
    internal static IconCacheKey Create(
        string targetPath,
        int? iconIndex,
        uint size,
        long lastWriteTicks,
        string targetType)
    {
        var normalizedPath = NormalizePath(targetPath);
        var selector = iconIndex?.ToString(System.Globalization.CultureInfo.InvariantCulture) ?? "shell";
        return new IconCacheKey(
            normalizedPath,
            selector,
            size,
            lastWriteTicks,
            targetType.Trim().ToUpperInvariant());
    }

    internal string Fingerprint()
    {
        var material = string.Join('\0', TargetPath, Selector, Size, LastWriteTicks, TargetType);
        return Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(material))).ToLowerInvariant();
    }

    private static string NormalizePath(string path)
    {
        var fullPath = Path.GetFullPath(path.Trim());
        var root = Path.GetPathRoot(fullPath);
        if (!string.IsNullOrEmpty(root) && fullPath.Length > root.Length)
        {
            fullPath = fullPath.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
        }
        return fullPath.ToUpperInvariant();
    }
}

internal sealed class IconCache
{
    internal const int MaxMemoryEntries = 128;
    private const int MaxDiskEntries = 2048;
    private const int CacheTrimWriteInterval = 128;
    private static readonly Lazy<IconCache> SharedInstance = new(() => new IconCache());

    private readonly object _gate = new();
    private readonly string _directory;
    private readonly int _maxMemoryEntries;
    private readonly Dictionary<string, CachedPath> _memory = [];
    private readonly ConcurrentDictionary<string, Lazy<bool>> _inFlight = new();
    private long _accessSequence;
    private int _writesSinceTrim;

    internal IconCache()
        : this(Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "NxTools", "NxTools.Next", "staging", "icons"),
            MaxMemoryEntries)
    {
    }

    internal IconCache(string directory, int maxMemoryEntries = MaxMemoryEntries)
    {
        _directory = directory;
        _maxMemoryEntries = Math.Max(1, maxMemoryEntries);
    }

    internal static IconCache Shared => SharedInstance.Value;

    internal int MemoryEntryCount
    {
        get
        {
            lock (_gate)
            {
                return _memory.Count;
            }
        }
    }

    internal string GetPath(IconCacheKey key) =>
        Path.Combine(_directory, $"{key.Fingerprint()}.png");

    internal bool TryGetOrCreate(
        IconCacheKey key,
        Func<string, bool> generate,
        out string path)
    {
        var fingerprint = key.Fingerprint();
        path = GetPath(key);
        lock (_gate)
        {
            if (TryGetMemoryPath(fingerprint, path, out var memoryPath))
            {
                path = memoryPath;
                return true;
            }
        }

        Directory.CreateDirectory(_directory);
        if (File.Exists(path))
        {
            lock (_gate)
            {
                Remember(fingerprint, path);
            }
            return true;
        }

        var generatedPath = path;
        var generation = _inFlight.GetOrAdd(
            fingerprint,
            _ => new Lazy<bool>(
                () => Generate(generatedPath, generate),
                LazyThreadSafetyMode.ExecutionAndPublication));
        try
        {
            if (!generation.Value || !File.Exists(path))
            {
                TryDelete(path);
                return false;
            }
            lock (_gate)
            {
                Remember(fingerprint, path);
            }
            if (Interlocked.Increment(ref _writesSinceTrim) >= CacheTrimWriteInterval)
            {
                Interlocked.Exchange(ref _writesSinceTrim, 0);
                TrimDiskCache();
            }
            return true;
        }
        finally
        {
            ((ICollection<KeyValuePair<string, Lazy<bool>>>)_inFlight)
                .Remove(new KeyValuePair<string, Lazy<bool>>(fingerprint, generation));
        }
    }

    private static bool Generate(string path, Func<string, bool> generate)
    {
        try
        {
            if (!generate(path))
            {
                TryDelete(path);
                return false;
            }
            return File.Exists(path);
        }
        catch
        {
            TryDelete(path);
            throw;
        }
    }
    private bool TryGetMemoryPath(string fingerprint, string expectedPath, out string path)
    {
        if (_memory.TryGetValue(fingerprint, out var cached))
        {
            if (cached.Path.Equals(expectedPath, StringComparison.OrdinalIgnoreCase)
                && File.Exists(cached.Path))
            {
                _memory[fingerprint] = cached with { LastAccessSequence = ++_accessSequence };
                path = cached.Path;
                return true;
            }
            _memory.Remove(fingerprint);
        }
        path = string.Empty;
        return false;
    }

    private void Remember(string fingerprint, string path)
    {
        _memory[fingerprint] = new CachedPath(path, ++_accessSequence);
        while (_memory.Count > _maxMemoryEntries)
        {
            var leastRecentlyUsed = _memory
                .OrderBy(pair => pair.Value.LastAccessSequence)
                .First();
            _memory.Remove(leastRecentlyUsed.Key);
        }
    }

    private void TrimDiskCache()
    {
        try
        {
            foreach (var stale in new DirectoryInfo(_directory)
                         .EnumerateFiles("*.png")
                         .OrderByDescending(file => file.LastWriteTimeUtc)
                         .Skip(MaxDiskEntries))
            {
                stale.Delete();
            }
        }
        catch (IOException)
        {
            // Cache cleanup is best effort and must never fail an icon request.
        }
        catch (UnauthorizedAccessException)
        {
        }
    }

    private static void TryDelete(string path)
    {
        try
        {
            if (File.Exists(path))
            {
                File.Delete(path);
            }
        }
        catch (IOException)
        {
        }
        catch (UnauthorizedAccessException)
        {
        }
    }

    private sealed record CachedPath(string Path, long LastAccessSequence);
}
