using System.Buffers.Binary;
using System.IO.Compression;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Effects: matching worker, GUID-owned source trees, original Jobs and native output only.
// Generated executable archives are data here; recipient execution is a separate native test.
internal static class ArchiveSfxProductTests
{
    private static void Require(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }
    private static string Root()
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        if (string.IsNullOrEmpty(worker) || !File.Exists(worker)) throw new TestSkippedException("Explicit matching worker required.");
        var scratch = Environment.GetEnvironmentVariable("PI_SCRATCH_DIR") ?? throw new InvalidOperationException("Scratch required");
        var root = Path.Combine(scratch, "ref20-sfx", "archive-product-" + Guid.NewGuid());
        Directory.CreateDirectory(root);
        File.Copy(worker, Path.Combine(root, ArchiveReadHelper.DeploymentName));
        return root;
    }
    private static ulong Volume()
    {
        var stub = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_SFX_STUB_FIXTURE");
        if (string.IsNullOrEmpty(stub) || !File.Exists(stub)) throw new TestSkippedException("Explicit reviewed own stub required.");
        return Math.Max(65536UL, checked((ulong)new FileInfo(stub).Length + 48 + 65536));
    }
    private static async Task Owned(Func<string, Task> action)
    {
        var root = Root(); var passed = false;
        try { await action(root); passed = true; }
        finally { if (passed) Directory.Delete(root, true); else Console.Error.WriteLine("PRESERVED FIXTURE " + root); }
    }
    private static Dictionary<string, byte[]> Sources(string root, int count)
    {
        var expected = new Dictionary<string, byte[]> {
            ["资料/data.bin"] = new byte[count], ["empty.txt"] = [],
            ["hello.txt"] = Encoding.UTF8.GetBytes("An owned ZIP-SFX product fixture.\n")
        };
        new Random(20261112).NextBytes(expected["资料/data.bin"]);
        var source = Path.Combine(root, "source"); Directory.CreateDirectory(source);
        Directory.CreateDirectory(Path.Combine(source, "empty-dir"));
        foreach (var (name, bytes) in expected) {
            var path = Path.Combine(source, name); Directory.CreateDirectory(Path.GetDirectoryName(path)!); File.WriteAllBytes(path, bytes);
        }
        return expected;
    }
    private static ArchiveJobRequest Create(string root, string destination, string name, string method = "deflate", ulong? volume = null) => new() {
        Kind = "create", Paths = [Path.Combine(root, "source")], Destination = destination,
        Create = new() { Format = "zip", Level = method == "store" ? "store" : "normal", Method = method,
            StripSingleRoot = true, ExcludePatterns = [], TestAfterCreate = true, Name = name, VolumeBytes = volume }
    };
    private static ArchiveJobCommand Start(ArchiveJobRequest request) => new() {
        Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = 1, Action = "start", Request = request
    };
    private static ProtocolEnvelope Wire(ArchiveJobCommand command, bool sfx = false)
    {
        var data = JsonSerializer.SerializeToNode(command, JsonDefaults.Options)!;
        if (sfx) data["request"]!["create"]!["self_extracting"] = true;
        return ProtocolEnvelope.Create("platform_command", new { command = "archive_job", data }, command.OperationId);
    }
    private static async Task<ArchiveJobTerminal> Finish(ArchiveJobSession jobs)
    {
        while (true) {
            var item = (await jobs.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(90)))
                .Payload.GetProperty("data").Deserialize<ArchiveJobEvent>(JsonDefaults.Options)!;
            Require(item.Prompt is null, "SFX must not request password or overwrite confirmation");
            if (item.Terminal is { } terminal) return terminal;
        }
    }
    private static async Task<ArchiveJobTerminal> Run(ArchiveJobSession jobs, ArchiveJobRequest request, bool sfx = false)
    {
        Require(jobs.Dispatch(Wire(Start(request), sfx)).Success, "explicit SFX Job admission");
        return await Finish(jobs);
    }
    private static void Released(IEnumerable<string> paths)
    {
        foreach (var path in paths) using (new FileStream(path, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
    }
    private static void ProtectedTree(string path)
    {
        var moved = path + ".moved";
        try { Directory.Move(path, moved); }
        catch (IOException) { return; }
        Directory.Move(moved, path);
        throw new InvalidOperationException("original source directory lease must deny rename");
    }
    private static async Task<(ArchiveReadTerminal Terminal, ArchiveReadEntry[] Entries)> Browse(string root, string path)
    {
        await using var read = ArchiveReadSession.ForOwnedTestPackage(root);
        var wire = ArchiveProductReadTests.Command(path: path, version: 2, limits: new() { BrowserContext = true });
        var id = ArchiveReadWire.Parse(wire); Require(read.Dispatch(wire).Success, "SFX Browse admission");
        var entries = new List<ArchiveReadEntry>();
        while (true) {
            var item = (await read.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(90)))
                .Payload.GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!;
            if (item.Batch is { } batch) {
                entries.AddRange(batch.Entries);
                Require(read.Dispatch(ArchiveProductReadTests.Command("ack", id.OperationId, id.SessionId, id.Generation, sequence: batch.Sequence, version: 2)).Success, "SFX metadata credit");
            }
            if (item.Terminal is { } terminal) return (terminal, entries.ToArray());
        }
    }
    private static string[] Family(string destination, bool split)
    {
        var files = Directory.GetFiles(destination);
        if (!split) { Require(files.Length == 1 && files[0].EndsWith(".exe", StringComparison.Ordinal), "ordinary executable only"); return files; }
        var exe = files.Single(path => path.EndsWith(".exe", StringComparison.Ordinal));
        var zip = files.Single(path => path.EndsWith(".zip", StringComparison.Ordinal));
        var middle = files.Where(path => path != exe && path != zip).OrderBy(path => path.Length).ThenBy(path => path, StringComparer.Ordinal).ToArray();
        for (var index = 0; index < middle.Length; index++) Require(Path.GetExtension(middle[index]) == ".z" + (index + 2).ToString("D2"), "exact SFX physical sibling ordinal");
        return new[] { exe }.Concat(middle).Append(zip).ToArray();
    }
    private static void Prefix(string first, int count)
    {
        var bytes = File.ReadAllBytes(first); Require(bytes.AsSpan(0, 2).SequenceEqual("MZ"u8), "executable prefix");
        var stub = File.ReadAllBytes(Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_SFX_STUB_FIXTURE")!);
        Require(bytes.AsSpan(0, stub.Length).SequenceEqual(stub), "exact reviewed own bare stub");
        var descriptor = bytes.AsSpan(stub.Length, 48);
        Require(descriptor[..16].SequenceEqual("NXTOOLS-ZIP-SFX1"u8) && BinaryPrimitives.ReadUInt32LittleEndian(descriptor[16..]) == 1
            && BinaryPrimitives.ReadUInt32LittleEndian(descriptor[20..]) == 48 && BinaryPrimitives.ReadUInt32LittleEndian(descriptor[24..]) == count
            && BinaryPrimitives.ReadUInt32LittleEndian(descriptor[28..]) == 0 && BinaryPrimitives.ReadUInt64LittleEndian(descriptor[32..]) == (ulong)stub.Length + 48
            && BinaryPrimitives.ReadUInt64LittleEndian(descriptor[40..]) == 0, "physical prefix descriptor");
        if (count > 1) Require(bytes.AsSpan(stub.Length + 48, 4).SequenceEqual("PK\x07\x08"u8), "span marker after complete prefix");
    }
    private static void ExactOutput(string directory, Dictionary<string, byte[]> expected)
    {
        var names = Directory.GetFiles(directory, "*", SearchOption.AllDirectories).Select(path => Path.GetRelativePath(directory, path).Replace('\\', '/')).Order();
        Require(names.SequenceEqual(expected.Keys.Order()), "complete exact output names");
        foreach (var (name, bytes) in expected) Require(File.ReadAllBytes(Path.Combine(directory, name)).SequenceEqual(bytes), "exact native member bytes");
        Require(Directory.Exists(Path.Combine(directory, "empty-dir")), "empty directory retained");
    }
    private static void Artifact(string label, string[] parts, Dictionary<string, byte[]> expected)
    {
        var evidence = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_SFX_ARTIFACT_ROOT");
        if (string.IsNullOrEmpty(evidence)) return;
        var destination = Path.Combine(evidence, label); Require(!Directory.Exists(destination), "new product artifact destination"); Directory.CreateDirectory(destination);
        foreach (var part in parts) File.Copy(part, Path.Combine(destination, Path.GetFileName(part)));
        var plain = Path.Combine(destination, "expected"); Directory.CreateDirectory(plain);
        foreach (var (name, bytes) in expected) { var path = Path.Combine(plain, name); Directory.CreateDirectory(Path.GetDirectoryName(path)!); File.WriteAllBytes(path, bytes); }
        File.WriteAllText(Path.Combine(destination, "manifest.json"), JsonSerializer.Serialize(new { parts = parts.Select(Path.GetFileName), expected = expected.Keys, directories = new[] { "资料/", "empty-dir/" } }, JsonDefaults.Options));
    }
    internal static Task ExplicitIntentWireContract()
    {
        var request = Create(@"C:\owned", @"C:\output", "created.exe");
        var legacy = Wire(Start(request));
        var parsedLegacy = ArchiveJobWire.Parse(legacy);
        Require(!JsonSerializer.SerializeToElement(parsedLegacy.Request!.Create, JsonDefaults.Options).TryGetProperty("self_extracting", out _), "false intent omitted on legacy wire");
        var parsed = ArchiveJobWire.Parse(Wire(Start(request), true));
        Require(JsonSerializer.SerializeToElement(parsed.Request!.Create, JsonDefaults.Options).GetProperty("self_extracting").GetBoolean(), "explicit intent survives capture");
        foreach (var invalid in new[] {
            request with { Create = request.Create! with { Name = "created.zip" } },
            request with { Create = request.Create! with { Password = Encoding.UTF8.GetBytes("private-test") } },
            request with { Create = request.Create! with { Format = "seven_z", Method = null } }
        }) {
            var refused = false; try { ArchiveJobWire.Parse(Wire(Start(invalid), true)); } catch (InvalidDataException) { refused = true; }
            Require(refused, "incompatible explicit SFX intent refused");
        }
        return Task.CompletedTask;
    }
    internal static Task OrdinaryStoredDeflate() => Exact(false);
    internal static Task SplitStoredDeflate() => Exact(true);
    private static async Task Exact(bool split)
    {
        foreach (var method in new[] { "store", "deflate" }) await Owned(async root => {
            var volume = Volume(); var expected = Sources(root, split ? checked((int)(2 * volume + 997)) : 8193);
            var destination = Path.Combine(root, "created"); Directory.CreateDirectory(destination);
            await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root);
            var result = await Run(jobs, Create(root, destination, "created.exe", method, split ? volume : null), true);
            var parts = Family(destination, split); Prefix(parts[0], parts.Length);
            Require(result.Outcome == "succeeded" && result.CleanupComplete && result.FilesCommitted == (ulong)parts.Length && result.OutputPublished == true, "actual SFX native publication " + result.Code);
            Require(result.Journal is { } journal && journal.Count == parts.Length && journal.All(item => item.State == "Committed" && !item.Replaced && item.Published == true && item.Cleanup == "retained" && item.NativeIdentity is { Length: > 10 }), "original per-candidate publication journal");
            Require(result.ResultLocation?.Target == parts[^1], "ordinary EXE / split final ZIP result location");
            if (split) Require(parts.Length >= 3 && parts.All(path => (ulong)new FileInfo(path).Length <= volume), "real multi-volume payload and exact maximum");
            else using (var zip = ZipFile.OpenRead(parts[0])) foreach (var (name, bytes) in expected) { using var input = zip.GetEntry(name)!.Open(); using var memory = new MemoryStream(); await input.CopyToAsync(memory); Require(memory.ToArray().SequenceEqual(bytes), "independent .NET ordinary SFX readback"); }
            var (opened, entries) = await Browse(root, parts[^1]);
            Require(opened.Status == "succeeded" && opened.CleanupComplete && opened.Browser is not null && entries.Length == expected.Count + 2, "production final-family Browse " + opened.Code);
            if (split) { var first = await Browse(root, parts[0]); Require(first.Terminal.Status == "succeeded" && first.Entries.Length == entries.Length, "executable entry acquires complete family"); }
            var tested = await Run(jobs, new() { Kind = "test", Paths = [parts[^1]] });
            Require(tested.Outcome == "succeeded" && tested.CleanupComplete && tested.BytesVerified == (ulong)expected.Values.Sum(bytes => bytes.Length), "postcreate read-only Test " + tested.Code);
            var output = Path.Combine(root, "extracted"); Directory.CreateDirectory(output);
            var extracted = await Run(jobs, new() { Kind = "extract", Paths = [parts[^1]], Destination = output, Extract = new() { Strategy = "here", Conflict = "skip", SingleFileFolder = false, CopyMotw = false } });
            Require(extracted.Outcome == "succeeded" && extracted.CleanupComplete && extracted.FilesCommitted == (ulong)expected.Count, "original SFX extraction " + extracted.Code);
            ExactOutput(output, expected); Released(parts.Append(Path.Combine(root, ArchiveReadHelper.DeploymentName)));
            Artifact((split ? "split-" : "ordinary-") + method, parts, expected);
        });
    }
    internal static async Task FreshFamilyNoOverwrite()
    {
        foreach (var split in new[] { false, true }) foreach (var suffix in new[] { ".exe", ".zip", ".z01", ".z02", ".z999" }) await Owned(async root => {
            Sources(root, 8193); var destination = Path.Combine(root, "out"); Directory.CreateDirectory(destination);
            var foreign = Path.Combine(destination, "created" + suffix); var sentinel = Encoding.UTF8.GetBytes("existing family stays unchanged"); File.WriteAllBytes(foreign, sentinel);
            await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root);
            var result = await Run(jobs, Create(root, destination, "created.exe", volume: split ? Volume() : null), true);
            Require(result.Outcome == "failed" && result.Code == "output_collision" && result.FilesCommitted == 0 && result.CleanupComplete, "fresh SFX family conflict " + result.Code);
            Require(Directory.GetFiles(destination).Length == 1 && File.ReadAllBytes(foreign).SequenceEqual(sentinel), "no overwrite or owned partial on conflict");
        });
    }
    internal static async Task CancellationBeforeAndAfterPublication()
    {
        foreach (var late in new[] { false, true }) await Owned(async root => {
            Sources(root, checked((int)(Volume() * 2 + 997))); var destination = Path.Combine(root, "out"); Directory.CreateDirectory(destination);
            await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root); var command = Start(Create(root, destination, "created.exe", volume: Volume())); var requested = false;
            jobs.ProbeOutputCommitForFixture(stage => { if (!requested && stage == (late ? "native_published" : "before_flush")) { requested = true; Require(jobs.Dispatch(Wire(command with { Action = "cancel", Request = null })).Success, "original SFX cancellation"); } });
            Require(jobs.Dispatch(Wire(command, true)).Success, "cancellable SFX admission"); var result = await Finish(jobs);
            Require(requested && result.Outcome == "cancelled" && result.CleanupComplete && (late ? result.FilesCommitted > 0 : result.FilesCommitted == 0), "actual cancellation publication facts " + result.Code);
            Require(!Directory.GetFiles(destination, ".nxpart-*").Any(), "owned candidates removed after actual stop");
            Require(result.Journal is { } journal && journal.Where(item => item.Published == true).All(item => item.State == "Committed" && item.Cleanup == "retained"), "published cancellation facts retained");
        });
    }
    internal static async Task NativeWriteFirstErrorAndProtection()
    {
        await Owned(async root => {
            var expected = Sources(root, 8193); var source = Path.Combine(root, "source", "hello.txt"); var destination = Path.Combine(root, "out"); Directory.CreateDirectory(destination);
            await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root); var wrote = false; var cleanup = false;
            jobs.ProbeLifecycleForFixture(stage => {
                if (stage is "source_validated" or "before_cleanup") ProtectedTree(Path.Combine(root, "source"));
                if (stage == "before_cleanup") cleanup = true;
                if (stage == "candidate_write" && !wrote) { wrote = true; throw new ArchiveOutputException("io_failure", 112); }
            });
            var result = await Run(jobs, Create(root, destination, "created.exe"), true);
            Require(wrote && cleanup && result.Outcome == "failed" && result.Code == "io_failure" && result.NativeErrorCode == 112 && result.FilesCommitted == 0 && result.CleanupComplete, $"native first error: wrote={wrote}, cleanup={cleanup}, outcome={result.Outcome}, code={result.Code}, native={result.NativeErrorCode}, committed={result.FilesCommitted}, clean={result.CleanupComplete}");
            Require(!Directory.GetFiles(destination).Any() && File.ReadAllBytes(source).SequenceEqual(expected["hello.txt"]), "zero publication/source preservation");
            Released([source, Path.Combine(root, ArchiveReadHelper.DeploymentName)]);
        });
    }
    internal static async Task LaterPublicationFailureKeepsJournal()
    {
        await Owned(async root => {
            Sources(root, checked((int)(Volume() * 2 + 997))); var destination = Path.Combine(root, "out"); Directory.CreateDirectory(destination);
            await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root); var count = 0; var final = Path.Combine(destination, "created.zip");
            jobs.ProbeOutputCommitForFixture(stage => { if (stage == "native_published" && ++count == 1) File.WriteAllText(final, "foreign publication"); });
            var result = await Run(jobs, Create(root, destination, "created.exe", volume: Volume()), true);
            Require(result.Outcome == "failed" && result.FilesCommitted > 0 && result.CleanupComplete && File.ReadAllText(final) == "foreign publication", "later no-replace failure retains actual earlier publication");
            Require(result.Journal is { } journal && journal.Count > (int)result.FilesCommitted && journal.Any(item => item.State == "FailedBeforeCommit")
                && journal.All(item => item.NativeIdentity is { Length: > 10 }) && journal.Where(item => item.Published == true).All(item => item.Cleanup == "retained")
                && journal.Where(item => item.Published == false).All(item => item.Cleanup == "partial_removed"), "exact per-volume native journal cleanup facts");
            Require(File.Exists(Path.Combine(destination, "created.exe")) && !Directory.GetFiles(destination, ".nxpart-*").Any(), "published EXE retained; incomplete candidates removed");
        });
    }
    internal static async Task MissingCorruptFamilyTypedCleanup()
    {
        await Owned(async root => {
            Sources(root, checked((int)(Volume() * 2 + 997))); var destination = Path.Combine(root, "out"); Directory.CreateDirectory(destination);
            await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root);
            var created = await Run(jobs, Create(root, destination, "created.exe", volume: Volume()), true);
            Require(created.Outcome == "succeeded" && created.CleanupComplete, "complete family before mutations");
            var parts = Family(destination, true); var middle = parts[1]; File.Move(middle, middle + ".missing");
            var missing = await Run(jobs, new() { Kind = "test", Paths = [parts[^1]] });
            Require(missing.Outcome == "failed" && missing.Code == "missing_volume" && missing.CleanupComplete && missing.FilesCommitted == 0, "typed missing SFX volume " + missing.Code);
            File.Move(middle + ".missing", middle);
            var bytes = File.ReadAllBytes(parts[0]); var original = bytes.ToArray();
            var stubLength = checked((int)new FileInfo(Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_SFX_STUB_FIXTURE")!).Length);
            bytes[stubLength + 16] ^= 0x40; File.WriteAllBytes(parts[0], bytes);
            var corrupt = await Run(jobs, new() { Kind = "test", Paths = [parts[^1]] });
            Require(corrupt.Outcome == "failed" && corrupt.Code == "corrupt_archive" && corrupt.CleanupComplete && corrupt.FilesCommitted == 0, "typed malformed own SFX descriptor " + corrupt.Code);
            File.WriteAllBytes(parts[0], original); Released(parts.Append(Path.Combine(root, ArchiveReadHelper.DeploymentName)));
        });
    }
    internal static async Task CompletePrefixMustFitVolume()
    {
        await Owned(async root => {
            Sources(root, 8193); var destination = Path.Combine(root, "out"); Directory.CreateDirectory(destination);
            var prefix = checked((ulong)new FileInfo(Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_SFX_STUB_FIXTURE")!).Length + 48);
            Require(prefix + 3 >= 65536, "reviewed stub must reach the SFX-specific split admission boundary");
            await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root);
            var result = await Run(jobs, Create(root, destination, "created.exe", volume: prefix + 3), true);
            Require(result.Outcome == "failed" && result.Code == "resource_limit" && result.FilesCommitted == 0 && result.CleanupComplete, "full prefix plus span marker admission " + result.Code);
            Require(!Directory.GetFiles(destination).Any(), "prefix shortfall never publishes a candidate");
        });
    }
    internal static async Task SelectedSmallOutputKeepsSourceAdmission()
    {
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_SFX_ARTIFACT_ROOT");
        if (string.IsNullOrEmpty(fixtures) || !Directory.Exists(fixtures)) throw new TestSkippedException("Explicit independently qualified product SFX fixtures required.");
        foreach (var profile in new[] { "ordinary-store", "ordinary-deflate", "split-store", "split-deflate" }) await Owned(async root => {
            var fixture = Path.Combine(fixtures, profile);
            using var manifest = JsonDocument.Parse(File.ReadAllBytes(Path.Combine(fixture, "manifest.json")));
            var source = Path.Combine(root, "input"); Directory.CreateDirectory(source);
            var parts = manifest.RootElement.GetProperty("parts").EnumerateArray().Select(part => Path.Combine(source, part.GetString()!)).ToArray();
            foreach (var part in parts) File.Copy(Path.Combine(fixture, Path.GetFileName(part)), part);
            var expected = File.ReadAllBytes(Path.Combine(fixture, "expected", "hello.txt"));
            Require(expected.SequenceEqual(Encoding.UTF8.GetBytes("An owned ZIP-SFX product fixture.\n")), "qualified selected literal");
            var (opened, entries) = await Browse(root, parts[^1]);
            Require(opened.Status == "succeeded" && opened.Browser is not null, "Browse before bounded SFX selection " + opened.Code);
            var selected = entries.Single(entry => entry.Name == "hello.txt");
            var destination = Path.Combine(root, "output"); Directory.CreateDirectory(destination);
            await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root);
            var result = await Run(jobs, new() {
                Kind = "extract", Paths = [parts[^1]], Destination = destination,
                Limits = new() { MaxMetadataBytes = 8192, MaxEntryBytes = (ulong)expected.Length, MaxOutputBytes = (ulong)expected.Length },
                Extract = new() { Strategy = "here", Conflict = "skip", SingleFileFolder = false, CopyMotw = false, Selection = new(opened.Browser!, [selected.Ordinal]) }
            });
            Require(result.Outcome == "succeeded" && result.FilesCommitted == 1 && result.BytesCommitted == (ulong)expected.Length && result.CleanupComplete, "selected SFX output credit excludes encoded prefix " + result.Code);
            var output = Directory.GetFiles(destination, "*", SearchOption.AllDirectories);
            Require(output.Length == 1 && Path.GetFileName(output[0]) == "hello.txt" && File.ReadAllBytes(output[0]).SequenceEqual(expected), "only exact selected SFX member");
            Released(parts.Append(Path.Combine(root, ArchiveReadHelper.DeploymentName)));
        });
    }
    internal static async Task SelectedOutputQuotaRemainsExact()
    {
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_SFX_ARTIFACT_ROOT");
        if (string.IsNullOrEmpty(fixtures) || !Directory.Exists(fixtures)) throw new TestSkippedException("Explicit independently qualified product SFX fixtures required.");
        foreach (var profile in new[] { "ordinary-store", "ordinary-deflate", "split-store", "split-deflate" }) await Owned(async root => {
            var fixture = Path.Combine(fixtures, profile);
            using var manifest = JsonDocument.Parse(File.ReadAllBytes(Path.Combine(fixture, "manifest.json")));
            var source = Path.Combine(root, "input"); Directory.CreateDirectory(source);
            var parts = manifest.RootElement.GetProperty("parts").EnumerateArray().Select(part => Path.Combine(source, part.GetString()!)).ToArray();
            foreach (var part in parts) File.Copy(Path.Combine(fixture, Path.GetFileName(part)), part);
            var expected = File.ReadAllBytes(Path.Combine(fixture, "expected", "hello.txt"));
            Require(expected.SequenceEqual(Encoding.UTF8.GetBytes("An owned ZIP-SFX product fixture.\n")), "qualified quota literal");
            var (opened, entries) = await Browse(root, parts[^1]);
            Require(opened.Status == "succeeded" && opened.Browser is not null, "Browse before SFX quota refusal " + opened.Code);
            var selected = entries.Single(entry => entry.Name == "hello.txt");
            var destination = Path.Combine(root, "output"); Directory.CreateDirectory(destination);
            await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root);
            var result = await Run(jobs, new() {
                Kind = "extract", Paths = [parts[^1]], Destination = destination,
                Limits = new() { MaxMetadataBytes = 8192, MaxEntryBytes = (ulong)expected.Length, MaxOutputBytes = (ulong)expected.Length - 1 },
                Extract = new() { Strategy = "here", Conflict = "skip", SingleFileFolder = false, CopyMotw = false, Selection = new(opened.Browser!, [selected.Ordinal]) }
            });
            Require(result.Outcome == "failed" && result.Code == "resource_limit" && result.FilesCommitted == 0 && result.CleanupComplete, "source preparation must not widen selected SFX output " + result.Code);
            Require(!Directory.GetFiles(destination, "*", SearchOption.AllDirectories).Any(), "no selected output beyond quota");
            Released(parts.Append(Path.Combine(root, ArchiveReadHelper.DeploymentName)));
        });
    }
}
