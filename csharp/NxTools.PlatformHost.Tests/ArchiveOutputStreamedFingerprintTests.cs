using System.Security.Cryptography;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// New-file publication fingerprints the written bytes instead of re-reading the candidate and the
// published file. Replacement keeps its independent full reads.
internal static class ArchiveOutputStreamedFingerprintTests
{
    private static void Check(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }

    internal static Task NewFilePublicationUsesWrittenDigest()
    {
        var root = Directory.CreateDirectory(Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!, "archive-product-" + Guid.NewGuid())).FullName;
        try {
            var payload = RandomNumberGenerator.GetBytes(200_003);
            using (var output = ArchiveOutputSession.Open(root, Guid.NewGuid())) {
                var token = output.BeginFile("nested/new.bin");
                for (var offset = 0; offset < payload.Length; offset += 65536)
                    output.Write(token, payload.AsMemory(offset, Math.Min(65536, payload.Length - offset)));
                var result = output.Commit(token);
                Check(result.Category == "committed", "new file committed: " + result.Category);
                var published = Path.Combine(root, "nested", "new.bin");
                Check(File.ReadAllBytes(published).AsSpan().SequenceEqual(payload), "published bytes");
                Check(!output.ReadObservation.Snapshot().Any(c => c.Stage is "OutputFingerprint" or "OutputPostpublication"),
                    "new-file publication performs no candidate or published full read");
                // Verified owned cleanup hashes the real file and must match the streamed fingerprint.
                Check(output.Cancel(token, cleanupNewOutput: true).Category == "new_output_removed" && !File.Exists(published),
                    "streamed fingerprint equals the published content hash");

                var empty = output.BeginFile("empty.bin");
                Check(output.Commit(empty).Category == "committed", "empty new file committed");
                Check(output.Cancel(empty, cleanupNewOutput: true).Category == "new_output_removed", "empty-file fingerprint");

                var existing = Path.Combine(root, "replace.bin");
                File.WriteAllBytes(existing, [1, 2, 3]);
                var replace = output.BeginFile("replace.bin", output.CaptureTargetIdentity("replace.bin"), overwrite: true);
                output.Write(replace, payload.AsMemory(0, 1000));
                Check(output.Commit(replace).Category == "committed", "replacement committed");
                var counts = output.ReadObservation.Snapshot();
                Check(counts.Single(c => c.Stage == "OutputFingerprint").FullReads == 1 &&
                    counts.Single(c => c.Stage == "OutputPostpublication").FullReads == 1,
                    "replacement keeps independent candidate and published reads");
            }
        } finally { Directory.Delete(root, true); }
        return Task.CompletedTask;
    }
}
