using System.Buffers.Binary;
using System.Text.Json;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Effects: GUID-owned data-only coordinate copies and the original native source leases.
internal static class ArchiveSfxVolumeSourceTests
{
    private const ulong Metadata = 1 << 20;
    private const ulong Encoded = 32 << 20;
    private static string Fixtures => Path.Combine(Directory.GetCurrentDirectory(), "rust", "crates", "nxtools-archive", "tests", "fixtures", "sfx-coordinates");
    private static void Require(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }
    private static string[][] Families(bool split)
    {
        using var manifest = JsonDocument.Parse(File.ReadAllBytes(Path.Combine(Fixtures, "manifest.json")));
        return manifest.RootElement.GetProperty("profiles").EnumerateArray()
            .Where(profile => profile.GetProperty("profile").GetString()!.StartsWith(split ? "split-" : "ordinary-", StringComparison.Ordinal))
            .Select(profile => profile.GetProperty("parts").EnumerateArray().Select(part => part.GetProperty("name").GetString()!).ToArray()).ToArray();
    }
    private static void Owned(string[] names, Action<string, string[]> action)
    {
        var root = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!, "ref20-native-sources", Guid.NewGuid().ToString());
        Directory.CreateDirectory(root);
        var paths = names.Select(name => Path.Combine(root, name)).ToArray();
        foreach (var (name, path) in names.Zip(paths)) File.Copy(Path.Combine(Fixtures, name), path);
        var passed = false;
        try { action(root, paths); passed = true; }
        finally {
            foreach (var path in paths.Where(File.Exists)) using (new FileStream(path, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
            if (passed) Directory.Delete(root, true);
        }
    }
    private static void Protected(string path)
    {
        try { using var file = new FileStream(path, FileMode.Open, FileAccess.Write, FileShare.ReadWrite | FileShare.Delete); }
        catch (IOException) { return; }
        throw new InvalidOperationException("held SFX member denies writer");
    }
    private static void Check(string root, string[] paths, string entry)
    {
        using var family = ArchiveVolumeSources.Open(entry, Metadata, Encoded);
        Require(family.Paths.SequenceEqual(paths, StringComparer.OrdinalIgnoreCase), "complete ordered own SFX family");
        Require(family.PrimaryPath.Equals(paths[0], StringComparison.OrdinalIgnoreCase), "SFX primary executable");
        using var requested = ArchiveSourceLease.Open(entry);
        Require(family.Requested.ReadmissionIdentity() == requested.ReadmissionIdentity(), "requested SFX identity preserved");
        var identity = family.Identity(); family.Verify(); Require(identity == family.Identity(), "stable original family identity");
        foreach (var path in paths) Protected(path);
        try { Directory.Move(root, root + "-moved"); throw new InvalidOperationException("held SFX parent rename admitted"); }
        catch (IOException) { }
    }
    private static void Expect(string reason, Func<ArchiveVolumeSources> open)
    {
        try { using var unexpected = open(); }
        catch (ArchiveOutputException failure) when (failure.Stage == reason) { return; }
        throw new InvalidOperationException("expected typed SFX " + reason);
    }
    internal static Task OrdinaryProtectedAndReleased()
    {
        foreach (var names in Families(false)) Owned(names, (root, paths) => Check(root, paths, paths[0]));
        return Task.CompletedTask;
    }
    internal static Task SplitEntriesProtectedAndReleased()
    {
        foreach (var names in Families(true)) Owned(names, (root, paths) => {
            Check(root, paths, paths[0]); Check(root, paths, paths[^1]);
        });
        return Task.CompletedTask;
    }
    internal static Task MissingSurplusAndCorruptRelease()
    {
        var names = Families(true)[0];
        foreach (var missing in new[] { 0, 1, names.Length - 1 }) Owned(names, (_, paths) => {
            File.Delete(paths[missing]);
            Expect("missing_volume", () => ArchiveVolumeSources.Open(paths[missing == 0 ? paths.Length - 1 : 0], Metadata, Encoded));
        });
        foreach (var mutation in new Action<string[]>[] {
            paths => { var bytes = File.ReadAllBytes(paths[0]); bytes[512 + 16] = 2; File.WriteAllBytes(paths[0], bytes); },
            paths => { var bytes = File.ReadAllBytes(paths[0]); BinaryPrimitives.WriteUInt32LittleEndian(bytes.AsSpan(512 + 24), 1); File.WriteAllBytes(paths[0], bytes); },
            paths => File.WriteAllBytes(Path.ChangeExtension(paths[0], ".z01"), [0, 1, 2, 3]),
        }) Owned(names, (_, paths) => { mutation(paths); Expect("corrupt_archive", () => ArchiveVolumeSources.Open(paths[^1], Metadata, Encoded)); });
        foreach (var namesOrdinary in Families(false)) Owned(namesOrdinary, (_, paths) => {
            var bytes = File.ReadAllBytes(paths[0]); bytes[512 + 16] = 2; File.WriteAllBytes(paths[0], bytes);
            Expect("corrupt_archive", () => ArchiveVolumeSources.Open(paths[0], Metadata, Encoded));
        });
        return Task.CompletedTask;
    }
    internal static Task FiniteAdmissionAndUnrelatedSources()
    {
        Owned(Families(true)[0], (root, paths) => {
            Expect("resource_limit", () => ArchiveVolumeSources.Open(paths[0], 32, Encoded));
            Expect("resource_limit", () => ArchiveVolumeSources.Open(paths[0], Metadata, (ulong)new FileInfo(paths[0]).Length));
            var unrelated = Path.Combine(root, "unrelated.exe"); File.WriteAllBytes(unrelated, new byte[128]);
            Check(root, paths, paths[^1]);
            using var writer = new FileStream(unrelated, FileMode.Open, FileAccess.Write, FileShare.None);
        });
        return Task.CompletedTask;
    }
}
