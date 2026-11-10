using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

internal static class RawNumberedJoinTests
{
    private static void Check(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }
    private static string Root()
    {
        var root = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR") ?? throw new InvalidOperationException("Scratch required"), "raw-join-" + Guid.NewGuid());
        Directory.CreateDirectory(root);
        return root;
    }
    internal static Task LaterPartPreviewAndByteExactJoin()
    {
        var root = Root(); var passed = false;
        try
        {
            var expected = new byte[320123]; new Random(18).NextBytes(expected);
            File.WriteAllBytes(Path.Combine(root, "ordinary.bin.002"), expected[100003..210010]);
            File.WriteAllBytes(Path.Combine(root, "ordinary.bin.003"), expected[210010..]);
            File.WriteAllBytes(Path.Combine(root, "ordinary.bin.001"), expected[..100003]);
            var selected = Path.Combine(root, "ordinary.bin.003");
            var preview = RawNumberedJoin.Inspect(selected);
            Check(preview.Parts.Select(p => p.Name).SequenceEqual(new[] { "ordinary.bin.001", "ordinary.bin.002", "ordinary.bin.003" }), "later-part selection must preview the numeric family");
            Check(preview.TotalBytes == expected.Length && !File.Exists(preview.SuggestedOutput), "inspect is read-only with exact length");
            var result = RawNumberedJoin.Join(preview, preview.SuggestedOutput, Guid.NewGuid());
            Check(result.Outcome == "succeeded" && result.CleanupComplete && result.BytesWritten == expected.Length, "actual native join terminal");
            Check(File.ReadAllBytes(preview.SuggestedOutput).SequenceEqual(expected), "independent byte-exact output beyond transfer buffer");
            Check(Directory.GetFiles(root).Length == 4 && File.ReadAllBytes(selected).SequenceEqual(expected[210010..]), "sources retained without extra artifacts");
            passed = true;
        }
        finally { if (passed) Directory.Delete(root, true); else Console.Error.WriteLine("PRESERVED FIXTURE " + root); }
        return Task.CompletedTask;
    }
    private static void Refused(Action action, string code)
    {
        try { action(); throw new InvalidOperationException("expected refusal " + code); }
        catch (ArchiveOutputException failure) { Check(failure.Stage == code, "refusal " + failure.Stage + " expected " + code); }
    }
    internal static Task FamilyAndSourceRefusals()
    {
        var root = Root(); var passed = false;
        try
        {
            string Part(int n) => Path.Combine(root, "parts.bin." + n.ToString("D3"));
            File.WriteAllBytes(Part(1), [1, 2]); File.WriteAllBytes(Part(3), [5]);
            Refused(() => RawNumberedJoin.Inspect(Part(3)), "missing_part");
            File.WriteAllBytes(Part(2), [3, 4]);
            foreach (var suffix in new[] { "0001", "000", "1" })
            {
                var alias = Path.Combine(root, "parts.bin." + suffix); File.WriteAllBytes(alias, [9]);
                Refused(() => RawNumberedJoin.Inspect(Part(2)), "ambiguous_family"); File.Delete(alias);
            }
            var classic = Path.Combine(root, "classic.z01"); File.WriteAllBytes(classic, [1]);
            Refused(() => RawNumberedJoin.Inspect(classic), "invalid_family"); File.Delete(classic);
            var preview = RawNumberedJoin.Inspect(Part(2));
            File.WriteAllBytes(Part(1), [7, 8]); File.SetLastWriteTimeUtc(Part(1), DateTime.UtcNow.AddMinutes(-1));
            var changed = RawNumberedJoin.Join(preview, preview.SuggestedOutput, Guid.NewGuid());
            Check(changed.Code == "source_changed" && changed.CleanupComplete && !File.Exists(preview.SuggestedOutput), "frozen source changed before output");
            preview = RawNumberedJoin.Inspect(Part(2)); File.WriteAllBytes(Part(4), [6]);
            Check(RawNumberedJoin.Join(preview, preview.SuggestedOutput, Guid.NewGuid()).Code == "source_changed", "new family member invalidates preview");
            for (var n = 5; n <= 12; n++) File.WriteAllBytes(Part(n), [(byte)n]);
            preview = RawNumberedJoin.Inspect(Part(12));
            Check(preview.Parts.Select(p => p.Name).SequenceEqual(Enumerable.Range(1, 12).Select(n => Path.GetFileName(Part(n)))), "numeric order including 010");
            File.WriteAllBytes(preview.SuggestedOutput, [99]);
            var conflict = RawNumberedJoin.Join(preview, preview.SuggestedOutput, Guid.NewGuid());
            Check(conflict.Outcome == "failed" && conflict.CleanupComplete && File.ReadAllBytes(preview.SuggestedOutput).SequenceEqual(new byte[] { 99 }), "existing output retained");
            var aliasResult = RawNumberedJoin.Join(preview, Part(1), Guid.NewGuid());
            Check(aliasResult.Outcome == "failed" && File.ReadAllBytes(Part(1)).SequenceEqual(new byte[] { 7, 8 }), "source output never overwritten");
            var sourceFamilyOutput = Part(13);
            var preflight = new RawJoinObservation();
            var sourceFamilyResult = RawNumberedJoin.Join(preview, sourceFamilyOutput, Guid.NewGuid(), observation: preflight);
            Check(sourceFamilyResult.Code == "output_is_part" && sourceFamilyResult.BytesWritten == 0 && preflight.Reads == 0 && !File.Exists(sourceFamilyOutput), "numbered family output refused before content I/O");
            var missing = RawNumberedJoin.Join(preview, Path.Combine(root, "missing", "output"), Guid.NewGuid());
            Check(missing.Code == "destination_missing" && !Directory.Exists(Path.Combine(root, "missing")), "no implicit destination creation");
            using (var writer = new FileStream(Part(2), FileMode.Open, FileAccess.ReadWrite, FileShare.ReadWrite))
                Refused(() => RawNumberedJoin.Inspect(Part(2)), "open_component");
            for (var n = 13; n <= 1025; n++) File.WriteAllBytes(Part(n), []);
            Refused(() => RawNumberedJoin.Inspect(Part(1)), "ambiguous_family");
            passed = true;
        }
        finally { if (passed) Directory.Delete(root, true); else Console.Error.WriteLine("PRESERVED FIXTURE " + root); }
        return Task.CompletedTask;
    }
    internal static Task StreamCancellationFailureAndHandleCompletion()
    {
        var root = Root(); var passed = false;
        try
        {
            var source = Path.Combine(root, "large.bin.001"); var bytes = new byte[2 * 1024 * 1024 + 71]; new Random(1818).NextBytes(bytes); File.WriteAllBytes(source, bytes);
            var preview = RawNumberedJoin.Inspect(source);
            var observed = new RawJoinObservation(); var writerRefused = false;
            observed.Checkpoint = phase => { if (phase == "sources_admitted") { try { using var writer = new FileStream(source, FileMode.Open, FileAccess.Write, FileShare.ReadWrite); } catch (IOException) { writerRefused = true; } } };
            var success = RawNumberedJoin.Join(preview, preview.SuggestedOutput, Guid.NewGuid(), observation: observed);
            Check(success.Outcome == "succeeded" && writerRefused && File.ReadAllBytes(preview.SuggestedOutput).SequenceEqual(bytes), "held source and actual stream");
            Check(observed.BufferBytes == 65536 && observed.MaxRead == 65536 && observed.MaxWrite == 65536 && observed.Reads == observed.Writes && observed.Reads > 32 && observed.BytesRead == bytes.Length && observed.BytesWritten == bytes.Length, "one actual 64KiB buffer/read/write bounds and no second content scan");
            File.Delete(preview.SuggestedOutput);
            foreach (var phase in new[] { "write", "before_commit", "committed" })
            {
                using var cancel = new CancellationTokenSource();
                var observation = new RawJoinObservation { Checkpoint = actual => { if (actual == phase) cancel.Cancel(); } };
                var result = RawNumberedJoin.Join(preview, preview.SuggestedOutput, Guid.NewGuid(), cancel.Token, observation);
                Check(result.CleanupComplete && (phase == "committed" ? result.Outcome == "succeeded" && File.Exists(preview.SuggestedOutput) : result.Outcome == "cancelled" && !File.Exists(preview.SuggestedOutput)), "actual pre/late cancellation " + phase);
                if (File.Exists(preview.SuggestedOutput)) File.Delete(preview.SuggestedOutput);
                using var exclusive = new FileStream(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None);
            }
            var failed = RawNumberedJoin.Join(preview, preview.SuggestedOutput, Guid.NewGuid(), observation: new RawJoinObservation {
                ConfigureOutput = output => output.ExtractionWriteCheckpointProbe = _ => throw new IOException("owned write failure", unchecked((int)0x80070070)) });
            Check(failed.Code == "write_file" && failed.NativeErrorCode == 112 && failed.CleanupComplete && !File.Exists(preview.SuggestedOutput), "actual native write failure cleanup/code");
            Check(File.ReadAllBytes(source).SequenceEqual(bytes) && Directory.GetFiles(root).Length == 1, "only owned incomplete output removed; sources retained");
            passed = true;
        }
        finally { if (passed) Directory.Delete(root, true); else Console.Error.WriteLine("PRESERVED FIXTURE " + root); }
        return Task.CompletedTask;
    }
}
