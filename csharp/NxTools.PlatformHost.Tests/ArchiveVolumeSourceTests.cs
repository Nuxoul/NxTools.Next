using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Reviewed effects: GUID-owned temporary files under NXTOOLS_ARCHIVE_VOLUME_FIXTURE_ROOT
// (the session scratch directory); no product process, registry, service or native mutation.
internal static class ArchiveVolumeSourceTests
{
    internal static Task ProtectedFamilyAndMissingMembers()
    {
        var scratch = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_VOLUME_FIXTURE_ROOT")
            ?? throw new TestSkippedException("Explicit scratch root required");
        var root = Directory.CreateDirectory(Path.Combine(scratch, "volume-" + Guid.NewGuid().ToString("N")));
        try
        {
            var source = Path.Combine(root.FullName, "sample.7z.001");
            var second = Path.Combine(root.FullName, "sample.7z.002");
            // The checked-in independent upstream archive is segmented only for this
            // Native lease fixture only; codec interoperability needs an independent
            // genuine volume oracle, not this segmentation of a single-file archive.
            var fixture = Path.Combine(Directory.GetCurrentDirectory(), "rust", "crates", "nxtools-archive", "tests", "fixtures", "upstream", "lzma.7z");
            var bytes = File.ReadAllBytes(fixture);
            Require(bytes.Length > 64, "fixture must contain encoded data beyond the signature");
            File.WriteAllBytes(source, bytes[..48]); File.WriteAllBytes(second, bytes[48..]);
            var observedCommand=new NxTools.PlatformHost.Protocol.ArchiveSpaceCommand(2,Guid.NewGuid(),Guid.NewGuid(),1,source,Path.Combine(root.FullName,"output"),null,null);
            var observedResult=ArchiveSpacePreflight.Observe(observedCommand);Require(observedResult.Success,"HZ3 original source-set observation");
            var observed=System.Text.Json.JsonSerializer.Deserialize<NxTools.PlatformHost.Protocol.ArchiveSpaceObservation>(observedResult.Data!,NxTools.PlatformHost.Protocol.JsonDefaults.Options)!;
            Require(observed.SourceSet is {Members.Count:2}&&observed.SourceSet.Primary==source&&observed.SourceSet.Members.Contains(second),"HZ3 observations expose only original validated family members");
            Require(!Directory.Exists(observedCommand.Destination),"HZ3 source-set identification does not create output");
            using (var family = ArchiveVolumeSources.Open(source, 1 << 20, 1 << 20))
            {
                Require(family.Paths.Count == 2 && family.PrimaryPath == source, "ordered sevenz family");
                Require(family.Identity().StartsWith("v1:", StringComparison.Ordinal) && family.Identity().Length <= 256, "bounded path-free stamp");
                family.Verify();
                RejectWriter(second);
                try { Directory.Move(root.FullName, root.FullName + "-moved"); throw new InvalidOperationException("unprotected parent"); }
                catch (IOException) { }
            }
            using (var writer = File.Open(second, FileMode.Open, FileAccess.Write, FileShare.None)) { }
            File.Delete(second);
            ExpectFailure(() => ArchiveVolumeSources.Open(source, 1 << 20, 1 << 20));
            File.WriteAllBytes(second, bytes[48..]);
            File.WriteAllBytes(Path.Combine(root.FullName, "sample.7z.004"), [1, 2, 3]);
            ExpectFailure(() => ArchiveVolumeSources.Open(source, 1 << 20, 1 << 20));
            ExpectFailure(() => ArchiveVolumeSources.Open(second, 1 << 20, 1 << 20));
            return Task.CompletedTask;
        }
        finally { root.Delete(true); }
    }

    internal static Task OrdinarySingleSourceAndExactZipDiskCount()
    {
        var scratch = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_VOLUME_FIXTURE_ROOT")
            ?? throw new TestSkippedException("Explicit scratch root required");
        var root = Directory.CreateDirectory(Path.Combine(scratch, "volume-" + Guid.NewGuid().ToString("N")));
        try
        {
            var zip = Path.Combine(root.FullName, "ordinary.zip");
            using (var archive = System.IO.Compression.ZipFile.Open(zip, System.IO.Compression.ZipArchiveMode.Create))
                archive.CreateEntry("content.txt");
            using (var single = ArchiveVolumeSources.Open(zip, 1 << 20, 1 << 20))
            {
                Require(single.Paths.Count == 1 && single.Identity() == single.Primary.ReadmissionIdentity(), "single-file fallback");
                RejectWriter(zip);
            }
            // The target-output quota is not the size of the encoded ZIP source. Only the
            // private extraction-preflight path may borrow the existing metadata reservation.
            ExpectFailure(() => ArchiveVolumeSources.Open(zip, 1 << 20, 1));
            using (var preflight = ArchiveVolumeSources.Open(zip, 1 << 20, 1, zipOutputPreflight: true))
            {
                Require(preflight.Paths.Count == 1, "finite ZIP preflight keeps original source set");
                RejectWriter(zip); preflight.Verify();
            }
            ExpectFailure(() => ArchiveVolumeSources.Open(zip, 32, 1, zipOutputPreflight: true));
            var impostor = Path.Combine(root.FullName, "impostor.zip");
            File.WriteAllBytes(impostor, new byte[512]);
            ExpectFailure(() => ArchiveVolumeSources.Open(impostor, 1 << 20, 1, zipOutputPreflight: true));
            var z01 = Path.Combine(root.FullName, "multi.Z01");
            var end = Path.Combine(root.FullName, "multi.zip");
            File.WriteAllBytes(z01, "PK\x03\x04"u8.ToArray());
            // EOCD declares disk 1 and disk 0 central; it is a discovery fixture, not a
            // claim of independently decodable ZIP volume content.
            var eocd = new byte[22]; "PK\x05\x06"u8.CopyTo(eocd);
            eocd[4] = 1; File.WriteAllBytes(end, eocd);
            using (var family = ArchiveVolumeSources.Open(end, 1 << 20, 1 << 20))
            {
                Require(family.Paths.Count == 2 && family.Paths[0].EndsWith("multi.z01", StringComparison.OrdinalIgnoreCase), "Windows case-variant disk");
                RejectWriter(z01); family.Verify();
            }
            foreach(var alias in new[]{z01,Path.Combine(root.FullName,"multi.z01")})
                using(var routed=ArchiveVolumeSources.Open(alias,1<<20,1<<20)) {
                    Require(routed.Paths.Count==2,"first ZIP member native route, not old RAR route");routed.Verify();RejectWriter(z01);
                }
            File.Delete(z01);
            ExpectFailure(() => ArchiveVolumeSources.Open(end, 1 << 20, 1 << 20));
            return Task.CompletedTask;
        }
        finally { root.Delete(true); }
    }
    private static void Require(bool yes, string reason) { if (!yes) throw new InvalidOperationException(reason); }
    private static void RejectWriter(string path)
    {
        try { using var writer = new FileStream(path, FileMode.Open, FileAccess.Write, FileShare.ReadWrite | FileShare.Delete); throw new InvalidOperationException("source admitted writer"); }
        catch (IOException) { }
    }
    private static void ExpectFailure(Action action)
    {
        try { action(); }
        catch (ArchiveOutputException) { return; }
        catch (IOException) { return; }
        throw new InvalidOperationException("missing/invalid volume admitted");
    }
}
