using System.Buffers.Binary;
using System.Text.Json;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// GUID-owned copies of CC0 inputs; original native source leases only.
internal static class ArchiveAlzegVolumeSourceTests
{
    private const ulong Metadata = 1 << 20;
    private const ulong Encoded = 32 << 20;
    private static string Fixtures => Path.Combine(Directory.GetCurrentDirectory(), "rust", "crates", "nxtools-archive", "tests", "fixtures", "alzeg");

    internal static Task AlzGenuineFamiliesProtectedAndReleased()
    {
        foreach (var names in Families(".alz"))
            WithCopies(names, (root, paths) => CheckFamily(root, paths, paths[0]));
        foreach (var name in new[] { "store_old.alz", "deflate_empty.alz" })
            WithCopies([name], (root, paths) => CheckFamily(root, paths, paths[0]));
        return Task.CompletedTask;
    }

    internal static Task EggGenuineFamiliesProtectedAndReleased()
    {
        foreach (var names in Families(".egg"))
            WithCopies(names, (root, paths) =>
            {
                CheckFamily(root, paths, paths[0]);
                CheckFamily(root, paths, paths[paths.Length / 2]);
                CheckFamily(root, paths, paths[^1]);
            });
        WithCopies(["store5.egg"], (root, paths) => CheckFamily(root, paths, paths[0]));
        return Task.CompletedTask;
    }

    internal static Task AlzMissingCorruptAndFailureRelease()
    {
        var names = Families(".alz").First();
        foreach (var missing in new[] { 1, names.Length - 1 })
            WithCopies(names, (_, paths) =>
            {
                File.Delete(paths[missing]);
                Expect("missing_volume", () => ArchiveVolumeSources.Open(paths[0], Metadata, Encoded));
            });
        WithCopies(names, (_, paths) => Expect("missing_volume", () => ArchiveVolumeSources.Open(paths[1], Metadata, Encoded)));
        foreach (var mutation in new Action<string[]>[]
        {
            paths => Mutate(paths[1], bytes => bytes[6] = 7),
            paths => Mutate(paths[0], bytes => bytes[^1] ^= 0x80),
            paths => Mutate(paths[0], bytes => bytes[^9] = 1),
            paths => Mutate(paths[1], bytes => bytes[0] ^= 0x80),
        })
            WithCopies(names, (_, paths) =>
            {
                mutation(paths);
                Expect("corrupt_archive", () => ArchiveVolumeSources.Open(paths[0], Metadata, Encoded));
            });
        return Task.CompletedTask;
    }

    internal static Task EggMissingCorruptAndFailureRelease()
    {
        var names = Families(".egg").First();
        foreach (var missing in new[] { 0, 1, names.Length - 1 })
            WithCopies(names, (_, paths) =>
            {
                File.Delete(paths[missing]);
                Expect("missing_volume", () => ArchiveVolumeSources.Open(paths[missing == 0 ? 1 : 0], Metadata, Encoded));
            });
        foreach (var mutation in new Action<string[]>[]
        {
            paths => Mutate(paths[1], bytes => bytes[21] ^= 0x80),
            paths => Mutate(paths[1], bytes => bytes[29] ^= 0x80),
            paths => Mutate(paths[^1], bytes => BinaryPrimitives.WriteUInt32LittleEndian(bytes.AsSpan(25), ReadId(paths[0]))),
            paths => File.Copy(paths[1], Path.Combine(Path.GetDirectoryName(paths[1])!, "duplicate.part")),
        })
            WithCopies(names, (_, paths) =>
            {
                mutation(paths);
                Expect("corrupt_archive", () => ArchiveVolumeSources.Open(paths[0], Metadata, Encoded));
            });
        return Task.CompletedTask;
    }

    internal static Task EggHeaderIdsNotFilenameOrder()
    {
        WithCopies(Families(".egg").First(), (root, paths) =>
        {
            for (var index = 1; index < paths.Length; index++)
            {
                var renamed = Path.Combine(root.FullName, "continuation-" + (paths.Length - index) + ".part");
                File.Move(paths[index], renamed);
                paths[index] = renamed;
            }
            var unrelated = Path.Combine(root.FullName, "unrelated.egg");
            File.Copy(Path.Combine(Fixtures, "store5.egg"), unrelated);
            CheckFamily(root, paths, paths[0], unrelated);
            CheckFamily(root, paths, paths[1], unrelated);
        });
        return Task.CompletedTask;
    }

    internal static Task FiniteAdmissionAndContentDispatch()
    {
        foreach (var suffix in new[] { ".alz", ".egg" })
            WithCopies(Families(suffix).First(), (_, paths) =>
            {
                Expect("resource_limit", () => ArchiveVolumeSources.Open(paths[0], 32, Encoded));
                Expect("resource_limit", () => ArchiveVolumeSources.Open(paths[0], Metadata, 1));
                Expect("resource_limit", () => ArchiveVolumeSources.Open(paths[0], Metadata, (ulong)new FileInfo(paths[0]).Length));
            });
        WithCopies(["store_old.alz", "store5.egg"], (root, paths) =>
        {
            for (var index = 0; index < paths.Length; index++)
            {
                var renamed = Path.Combine(root.FullName, "ordinary-" + index + ".bin");
                File.Move(paths[index], renamed);
                CheckFamily(root, [renamed], renamed);
            }
            var impostor = Path.Combine(root.FullName, "impostor.alz");
            File.WriteAllBytes(impostor, new byte[128]);
            File.WriteAllBytes(Path.ChangeExtension(impostor, ".a00"), new byte[128]);
            using var single = ArchiveVolumeSources.Open(impostor, Metadata, Encoded);
            Require(single.Paths.Count == 1, "suffix cannot authorize sibling acquisition");
        });
        return Task.CompletedTask;
    }

    private static void CheckFamily(DirectoryInfo root, string[] expected, string entry, string? unrelated = null)
    {
        string stamp;
        using (var family = ArchiveVolumeSources.Open(entry, Metadata, Encoded))
        {
            Require(family.Paths.SequenceEqual(expected, StringComparer.OrdinalIgnoreCase), "complete ordered genuine family");
            Require(family.PrimaryPath.Equals(expected[0], StringComparison.OrdinalIgnoreCase), "first linked source is primary");
            stamp = family.Identity();
            Require(stamp.StartsWith("v1:", StringComparison.Ordinal) && stamp.Length <= 256, "bounded existing source stamp");
            Require(family.Requested.ReadmissionIdentity() == ReadIdentity(entry), "requested member identity survives ordering");
            family.Verify();
            Require(stamp == family.Identity(), "stable complete-family identity");
            foreach (var path in expected) RejectWriter(path);
            try { Directory.Move(root.FullName, root.FullName + "-moved"); throw new InvalidOperationException("parent rename admitted"); }
            catch (IOException) { }
            if (unrelated is not null) using (var writer = File.Open(unrelated, FileMode.Open, FileAccess.Write, FileShare.None)) { }
        }
        foreach (var path in expected) ExclusiveReopen(path);
        using var reopened = ArchiveVolumeSources.Open(expected[0], Metadata, Encoded);
        Require(reopened.Identity() == stamp, "entry-independent ordered source identity");
    }

    private static string ReadIdentity(string path)
    {
        using var lease = ArchiveSourceLease.Open(path);
        return lease.ReadmissionIdentity();
    }
    private static string[][] Families(string suffix)
    {
        using var manifest = JsonDocument.Parse(File.ReadAllBytes(Path.Combine(Fixtures, "expected", "split", "manifest.json")));
        return manifest.RootElement.GetProperty("families").EnumerateArray()
            .Where(family => family.GetProperty("first").GetString()!.EndsWith(suffix, StringComparison.Ordinal))
            .Select(family => family.GetProperty("parts").EnumerateArray().Select(part => part.GetString()!).ToArray()).ToArray();
    }
    private static void WithCopies(string[] names, Action<DirectoryInfo, string[]> test)
    {
        var scratch = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_VOLUME_FIXTURE_ROOT")
            ?? throw new InvalidOperationException("Explicit session scratch root required");
        var root = Directory.CreateDirectory(Path.Combine(scratch, "ref19-volume-" + Guid.NewGuid().ToString("N")));
        try
        {
            var paths = names.Select(name => Path.Combine(root.FullName, name)).ToArray();
            for (var index = 0; index < names.Length; index++) File.Copy(Path.Combine(Fixtures, names[index]), paths[index]);
            test(root, paths);
            foreach (var path in Directory.EnumerateFiles(root.FullName)) ExclusiveReopen(path);
        }
        finally { root.Delete(true); }
    }
    private static uint ReadId(string path) => BinaryPrimitives.ReadUInt32LittleEndian(File.ReadAllBytes(path).AsSpan(6));
    private static void Mutate(string path, Action<byte[]> mutation)
    {
        var bytes = File.ReadAllBytes(path); mutation(bytes); File.WriteAllBytes(path, bytes);
    }
    private static void RejectWriter(string path)
    {
        try { using var writer = File.Open(path, FileMode.Open, FileAccess.Write, FileShare.ReadWrite | FileShare.Delete); }
        catch (IOException) { return; }
        throw new InvalidOperationException("continuation source admitted writer");
    }
    private static void ExclusiveReopen(string path) { using var writer = File.Open(path, FileMode.Open, FileAccess.ReadWrite, FileShare.None); }
    private static void Expect(string code, Func<ArchiveVolumeSources> open)
    {
        try { using var unexpected = open(); }
        catch (ArchiveOutputException failure) when (failure.Stage == code) { return; }
        throw new InvalidOperationException("expected typed " + code);
    }
    private static void Require(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
}
