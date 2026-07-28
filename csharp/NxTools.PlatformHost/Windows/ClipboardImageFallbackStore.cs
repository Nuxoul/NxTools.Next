using System.Security.Cryptography;

namespace NxTools.PlatformHost.Windows;

internal static class ClipboardImageFallbackStore
{
    private static readonly TimeSpan DefaultRetention = TimeSpan.FromDays(7);

    public static string Persist(string sourcePath)
    {
        var localData = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
        var directory = Path.Combine(
            localData, "NxTools", "NxTools.Next", "data", "clipboard-paste-fallback");
        return Persist(sourcePath, directory, DateTimeOffset.UtcNow, DefaultRetention);
    }

    internal static string Persist(
        string sourcePath,
        string directory,
        DateTimeOffset now,
        TimeSpan retention)
    {
        using var source = File.OpenRead(sourcePath);
        var name = Convert.ToHexString(SHA256.HashData(source)).ToLowerInvariant() + ".png";
        Directory.CreateDirectory(directory);
        var destination = Path.Combine(directory, name);
        if (!File.Exists(destination))
        {
            source.Position = 0;
            using var output = new FileStream(destination, FileMode.CreateNew, FileAccess.Write, FileShare.Read);
            source.CopyTo(output);
        }
        File.SetLastWriteTimeUtc(destination, now.UtcDateTime);
        RemoveExpiredFiles(directory, destination, now - retention);
        return destination;
    }

    private static void RemoveExpiredFiles(
        string directory,
        string protectedPath,
        DateTimeOffset cutoff)
    {
        foreach (var candidate in Directory.EnumerateFiles(directory, "*.png", SearchOption.TopDirectoryOnly))
        {
            if (string.Equals(candidate, protectedPath, StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }
            try
            {
                if (File.GetLastWriteTimeUtc(candidate) < cutoff.UtcDateTime)
                {
                    File.Delete(candidate);
                }
            }
            catch (IOException)
            {
                // A concurrent paste may still be reading this file. A later persist will retry cleanup.
            }
            catch (UnauthorizedAccessException)
            {
                // Cleanup is best effort and must never break a paste operation.
            }
        }
    }
}
