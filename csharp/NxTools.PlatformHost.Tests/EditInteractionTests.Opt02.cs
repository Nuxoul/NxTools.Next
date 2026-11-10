using System.Diagnostics;
using System.IO.Compression;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Effects: GUID-owned PI_SCRATCH_DIR trees, copied hash-pinned worker, original browser/Job/native
// publication, independent Python/7-Zip readers. No external archive, GUI or system mutation.
internal static partial class EditInteractionTests
{
    private sealed class Opt02Fixture
    {
        internal readonly string Root, Archive, Format;
        internal readonly Dictionary<string, string?> Expected;
        internal readonly string? Password;
        internal readonly bool Header;
        private readonly string python, treeOracle, tarOracle;

        internal Opt02Fixture(string format, bool encrypted = false, bool header = false)
        {
            Format = format; Header = header;
            var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
            var marker = Environment.GetEnvironmentVariable(format switch {
                "Zip" => "NXTOOLS_EDIT_INTERACTION_WORKER_SHA256",
                "SevenZ" => "NXTOOLS_ARCHIVE_SEVENZ_EDIT_WORKER_SHA256",
                _ => "NXTOOLS_TAR_EDIT_WORKER_SHA256" });
            python = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_PYTHON_ORACLE") ?? "";
            var sevenzip = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_7ZIP_ORACLE") ?? "";
            treeOracle = Path.Combine(Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_ZIP_FIXTURES") ?? "", "verify-created-tree.py");
            tarOracle = Environment.GetEnvironmentVariable("NXTOOLS_TAR_EDIT_ORACLE") ?? "";
            var scratch = Environment.GetEnvironmentVariable("PI_SCRATCH_DIR");
            var bsdtar = Environment.GetEnvironmentVariable("NXTOOLS_TAR_EDIT_BSDTAR");
            if (string.IsNullOrWhiteSpace(scratch) || worker is null || !File.Exists(worker) || marker is not { Length: 64 } ||
                !File.Exists(python) || !File.Exists(sevenzip) || !File.Exists(treeOracle) ||
                (format.StartsWith("Tar", StringComparison.Ordinal) && (!File.Exists(tarOracle) || bsdtar is null || !File.Exists(bsdtar))))
                throw new TestSkippedException("OPT02 requires owned scratch, frozen worker/hash and explicit Python/stock7Zip/bsdtar oracles");
            Check(Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(worker))).Equals(marker, StringComparison.OrdinalIgnoreCase), "OPT02 frozen worker identity");
            Root = Path.Combine(scratch, "archive-product-opt02-" + Guid.NewGuid());
            Directory.CreateDirectory(Root);
            Directory.CreateDirectory(Path.Combine(Root, "archive"));
            File.Copy(worker, Path.Combine(Root, ArchiveReadHelper.DeploymentName));
            Archive = Path.Combine(Root, "archive", "editable" + (format switch { "Zip" => ".zip", "SevenZ" => ".7z", "TarGz" => ".tgz", "TarXz" => ".txz", _ => ".tar" }));
            Password = encrypted ? "opt02-private-" + Guid.NewGuid().ToString("N") : null;
            Expected = format.StartsWith("Tar", StringComparison.Ordinal)
                ? new() { ["keep.txt"] = "kept-data", ["folder/"] = null, ["folder/child.txt"] = "child-data", ["implicit/child"] = "implicit-data", ["empty/"] = null, ["zero"] = "" }
                : new() { ["keep.txt"] = "kept-data", ["folder/"] = null, ["folder/same.txt"] = "old-content", ["folder/nested/"] = null, ["folder/nested/child.txt"] = "old-child" };
        }

        internal string Input(string directory, string name, string text)
        {
            var path = Path.Combine(Root, "inputs", directory, name);
            Directory.CreateDirectory(Path.GetDirectoryName(path)!);
            File.WriteAllText(path, text, new UTF8Encoding(false));
            return path;
        }
        internal string Folder(string directory, string name)
        {
            var path = Path.Combine(Root, "inputs", directory, name);
            Directory.CreateDirectory(path);
            return path;
        }
        internal async Task<string> Process(string executable, params string[] args)
        {
            var info = new ProcessStartInfo(executable) { UseShellExecute = false, RedirectStandardInput = true, RedirectStandardOutput = true, RedirectStandardError = true };
            foreach (var arg in args) info.ArgumentList.Add(arg);
            using var child = System.Diagnostics.Process.Start(info)!;
            var output = child.StandardOutput.ReadToEndAsync(); var error = child.StandardError.ReadToEndAsync();
            if (Password is not null) await child.StandardInput.WriteAsync(string.Concat(Enumerable.Repeat(Password + "\n", 8)));
            child.StandardInput.Close();
            await child.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(60));
            var text = await output; Check(child.ExitCode == 0, "OPT02 independent tool: " + await error);
            return text;
        }
        internal async Task Seed()
        {
            if (Format.StartsWith("Tar", StringComparison.Ordinal)) { await Process(python, tarOracle, "seed", Archive, Format); return; }
            var seed = Path.Combine(Root, "seed");
            Directory.CreateDirectory(Path.Combine(seed, "folder", "nested"));
            File.WriteAllText(Path.Combine(seed, "keep.txt"), "kept-data");
            File.WriteAllText(Path.Combine(seed, "folder", "same.txt"), "old-content");
            File.WriteAllText(Path.Combine(seed, "folder", "nested", "child.txt"), "old-child");
            await using var jobs = ArchiveJobSession.ForOwnedTestPackage(Root);
            var request = new ArchiveJobRequest { Kind = "create", Paths = [seed], Destination = Path.GetDirectoryName(Archive), Create = new() {
                Format = Format == "Zip" ? "zip" : "seven_z", Name = Path.GetFileName(Archive), Level = "fast", Method = Format == "Zip" ? "deflate" : null,
                Password = Password is null ? null : Encoding.UTF8.GetBytes(Password), Solid = Format == "SevenZ" ? false : null,
                EncryptHeader = Format == "SevenZ" ? Header : null, StripSingleRoot = true, ExcludePatterns = [], TestAfterCreate = false } };
            var seedResult = await Run(jobs, request);
            Check(seedResult.Outcome == "succeeded" && seedResult.FilesCommitted == 1 && seedResult.CleanupComplete, "OPT02 original native seed");
        }
        internal async Task<ArchiveBrowserContext> Browse()
        {
            await using var reads = ArchiveReadSession.ForOwnedTestPackage(Root);
            var wire = ArchiveProductReadTests.Command(path: Archive, version: 2, limits: new() { BrowserContext = true });
            var command = ArchiveReadWire.Parse(wire);
            Check(reads.Dispatch(wire, browserInteraction: Password is not null).Success, "OPT02 browser admission");
            while (true)
            {
                var envelope = await reads.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(40));
                if (envelope.Payload.GetProperty("event").GetString() == "archive_password") {
                    var prompt = envelope.Payload.GetProperty("data").Deserialize<ArchiveBrowserPasswordPrompt>(JsonDefaults.Options)!;
                    Check(Password is not null && reads.RespondPassword(prompt, Password), "OPT02 browser password"); continue;
                }
                var item = envelope.Payload.GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!;
                if (item.Batch is { } batch) Check(reads.Dispatch(ArchiveProductReadTests.Command("ack", command.OperationId, command.SessionId, command.Generation, sequence: batch.Sequence, version: 2)).Success, "OPT02 browser credit");
                if (item.Terminal is not { } terminal) continue;
                Check(terminal.Status == "succeeded" && terminal.CleanupComplete && terminal.Browser?.Format == Format, "OPT02 actual browser context");
                return terminal.Browser!;
            }
        }
        internal ArchiveJobRequest Add(ArchiveBrowserContext browser, string? target, params string[] sources) =>
            new() { Kind = "modify", Paths = new[] { Archive }.Concat(sources).ToArray(), Modify = new(browser, "add", [], target) };

        internal async Task Oracle(Dictionary<string, string?> expected)
        {
            if (Format.StartsWith("Tar", StringComparison.Ordinal)) {
                using var doc = JsonDocument.Parse(await Process(python, tarOracle, "inspect", Archive));
                var actual = doc.RootElement;
                Check(actual.EnumerateObject().Count() == expected.Count, "OPT02 TAR exact member count");
                foreach (var pair in expected) {
                    Check(actual.TryGetProperty(pair.Key, out var entry) && entry.GetProperty("content").GetString() == pair.Value, "OPT02 TAR source/name binding " + pair.Key);
                    if (pair.Key == "keep.txt") Check(entry.GetProperty("uid").GetInt32() == 123 && entry.GetProperty("gid").GetInt32() == 456 && entry.GetProperty("mode").GetInt32() == 0x1a0 && entry.GetProperty("pax").GetProperty("mtime").GetString() == "1700000000.123456789", "OPT02 original TAR metadata");
                    if (pair.Key == "folder/") Check(entry.GetProperty("uid").GetInt32() == 123 && entry.GetProperty("gid").GetInt32() == 456 && entry.GetProperty("mode").GetInt32() == 0x1e8, "OPT02 original TAR directory metadata");
                }
                return;
            }
            if (Format == "Zip" && Password is null) {
                using var zip = ZipFile.OpenRead(Archive);
                Check(zip.Entries.Count == expected.Count, "OPT02 ZIP exact count");
                foreach (var pair in expected) {
                    var entry = zip.GetEntry(pair.Key); Check(entry is not null, "OPT02 ZIP name " + pair.Key);
                    if (pair.Value is null) continue;
                    using var reader = new StreamReader(entry!.Open());
                    Check(reader.ReadToEnd() == pair.Value, "OPT02 ZIP source/name binding " + pair.Key);
                    if (!Expected.ContainsKey(pair.Key)) Check(entry.CompressedLength > 0 || pair.Value.Length == 0, "OPT02 new plain Deflate member");
                }
                return;
            }
            if (Format == "Zip") {
                var decoded = Path.Combine(Root, "independent-zip");
                await Process(Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_7ZIP_ORACLE")!, "x", "-y", "-o" + decoded, Archive);
                var actual = Directory.EnumerateFileSystemEntries(decoded, "*", SearchOption.AllDirectories).ToDictionary(
                    path => Path.GetRelativePath(decoded, path).Replace('\\', '/') + (Directory.Exists(path) ? "/" : ""),
                    path => Directory.Exists(path) ? null : File.ReadAllText(path));
                Check(actual.Count == expected.Count && expected.All(pair => actual.TryGetValue(pair.Key, out var content) && content == pair.Value),
                    "OPT02 independent encrypted ZIP exact tree/content");
                return;
            }
            var info = new ProcessStartInfo(python) { UseShellExecute = false, RedirectStandardInput = true, RedirectStandardOutput = true, RedirectStandardError = true };
            info.ArgumentList.Add(treeOracle);
            using var child = System.Diagnostics.Process.Start(info)!;
            var entries = expected.ToDictionary(pair => pair.Key, pair => new { size = Encoding.UTF8.GetByteCount(pair.Value ?? ""), sha256 = Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(pair.Value ?? ""))).ToLowerInvariant() });
            await child.StandardInput.WriteAsync(JsonSerializer.Serialize(new { path = Archive, entries, format = Format == "Zip" ? "zip" : "7z", password = Password, encrypt_header = Format == "SevenZ" && Header })); child.StandardInput.Close();
            var output = child.StandardOutput.ReadToEndAsync(); var errors = child.StandardError.ReadToEndAsync();
            await child.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(60));
            Check(child.ExitCode == 0, "OPT02 independent 7Zip/Python exact manifest: " + await errors);
            using var parsed = JsonDocument.Parse(await output);
            Check(parsed.RootElement.GetProperty("verified_entries").GetInt32() == expected.Count, "OPT02 independent entry count");
        }
    }

    private static async Task<ArchiveJobTerminal> Run(ArchiveJobSession jobs, ArchiveJobRequest request,
        IReadOnlyList<string>? decisions = null, bool applyAll = false, int? expectedPrompts = null, Action<ArchiveJobPrompt, int>? onPrompt = null,
        string? password = null, bool wrongPassword = false, bool cancelPassword = false, int? expectedPasswordPrompts = null)
    {
        var command = new ArchiveJobCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = 1, Action = "start", Request = request };
        Check(jobs.Dispatch(Wire(command)).Success, "OPT02 original Job admission");
        var prompts = 0; var passwordPrompts = 0;
        while (true) {
            var envelope = await jobs.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(60));
            var item = envelope.Payload.GetProperty("data").Deserialize<ArchiveJobEvent>(JsonDefaults.Options)!;
            if (item.Prompt is { } prompt) {
                if (prompt.Kind == "password") {
                    var secret = request.Create?.Password ?? (password is null ? null : Encoding.UTF8.GetBytes(password));
                    Check(secret is not null, "OPT02 unexpected password prompt");
                    Check(++passwordPrompts <= 3, "OPT02 bounded password attempts");
                    if (cancelPassword) secret = null;
                    else if (wrongPassword) secret = Encoding.UTF8.GetBytes("wrong-opt02-password");
                    Check(jobs.Dispatch(Wire(command with { Action = "reply", Request = null, Reply = new() { Ticket = prompt.Ticket, Kind = "password", Password = secret, ApplyToAll = false } })).Success, "OPT02 password reply");
                } else {
                    Check(prompt.Kind == "conflict" && decisions is not null && prompts < decisions.Count && prompt.Display is not null, "OPT02 original conflict ticket");
                    onPrompt?.Invoke(prompt, prompts);
                    var answer = decisions![prompts++];
                    Check(jobs.Dispatch(Wire(command with { Action = "reply", Request = null, Reply = new() { Ticket = prompt.Ticket, Kind = "conflict", Decision = answer == "cancel" ? null : answer, ApplyToAll = applyAll } })).Success, "OPT02 original conflict reply");
                }
            }
            if (item.Terminal is not { } result) continue;
            if (expectedPrompts is { } count) Check(prompts == count, "OPT02 exact conflict prompt count: " + prompts + " != " + count + "; terminal=" + result.Outcome + "/" + result.Code);
            if (expectedPasswordPrompts is { } passwordCount) Check(passwordPrompts == passwordCount, "OPT02 exact password prompt count");
            return result;
        }
    }

    private static async Task Opt02Matrix(string format)
    {
        var tar = format.StartsWith("Tar", StringComparison.Ordinal);
        var originalName = tar ? "child.txt" : "same.txt";
        var oldName = "folder/" + originalName;
        // Each scenario has a fresh protected archive, browser revision and attempt-local ticket state.
        foreach (var mode in new[] { "overwrite", "skip", "auto_rename", "cancel", "apply_all", "future", "future_reverse", "future_case", "merge", "implicit_skip", "explicit_skip", "merge_only", "empty", "mismatch" })
        {
            var f = new Opt02Fixture(format); await f.Seed();
            Console.WriteLine("OPT02 scenario " + format + "/" + mode);
            if (mode == "implicit_skip" && format == "Zip") {
                f.Expected["implicit/child.txt"] = "implicit-data";
                using var file = new FileStream(f.Archive, FileMode.Create, FileAccess.ReadWrite, FileShare.None);
                using var zip = new ZipArchive(file, ZipArchiveMode.Create);
                foreach (var pair in f.Expected) {
                    var entry = zip.CreateEntry(pair.Key);
                    if (pair.Value is null) continue;
                    using var writer = new StreamWriter(entry.Open());
                    writer.Write(pair.Value);
                }
            }
            var expected = new Dictionary<string, string?>(f.Expected);
            var original = File.ReadAllBytes(f.Archive);
            var context = await f.Browse();
            await using var jobs = ArchiveJobSession.ForOwnedTestPackage(f.Root);
            var flush = 0; var publish = 0; var commit = 0;
            jobs.ProbeOutputCommitForFixture(stage => { if (stage == "before_flush") flush++; if (stage == "commit_in_flight") commit++; if (stage == "native_published") publish++; });
            var first = f.Input("root-a", originalName, "source-A");
            var second = f.Input("root-b", originalName, "source-B");
            string[] sources = [first, second]; string? target = "folder/";
            IReadOnlyList<string>? decisions = mode switch {
                "overwrite" => ["overwrite", "overwrite"], "skip" or "implicit_skip" or "explicit_skip" => ["skip", "skip"],
                "auto_rename" => ["auto_rename", "auto_rename"], "cancel" => ["cancel"], "apply_all" => ["overwrite"],
                "future" or "future_reverse" or "future_case" => ["auto_rename"], "merge" => ["overwrite"], _ => null };
            var promptCount = mode switch { "cancel" or "apply_all" or "future" or "future_reverse" or "future_case" or "merge" => 1,
                "overwrite" or "skip" or "auto_rename" or "implicit_skip" or "explicit_skip" => 2, _ => 0 };
            if (mode.StartsWith("future", StringComparison.Ordinal)) {
                var reserved = f.Input("root-c", originalName[..^4] + " (2).txt", "future-2");
                if (mode == "future_case") { first = f.Input("root-a", originalName.ToUpperInvariant(), "source-A"); sources = [first, reserved]; }
                else sources = mode == "future_reverse" ? [reserved, first] : [first, reserved];
                if (mode == "future_reverse") promptCount = 1;
            }
            if (mode == "merge" || mode is "implicit_skip" or "explicit_skip" or "merge_only") {
                if (mode == "implicit_skip" && (tar || format == "Zip")) {
                    var implicitFolder = f.Folder("tree", "implicit");
                    f.Input(Path.Combine("tree", "implicit"), tar ? "child" : "child.txt", "nested-incoming");
                    sources = [implicitFolder]; target = null; decisions = ["skip"]; promptCount = 1;
                } else {
                    var folder = f.Folder("tree", "folder");
                    if (mode != "merge_only") f.Input(Path.Combine("tree", "folder"), originalName, "nested-incoming");
                    sources = [folder]; target = null;
                    if (mode == "merge") promptCount = 1;
                    else if (mode == "merge_only") promptCount = 0;
                    else { decisions = ["skip"]; promptCount = 1; }
                }
            }
            if (mode == "empty") { sources = [f.Folder("tree", "new-empty")]; target = null; }
            if (mode == "mismatch") { sources = [f.Folder("tree", "keep.txt")]; target = null; }
            var sourceBytes = sources.SelectMany(path => File.Exists(path) ? new[] { path } : Directory.GetFiles(path, "*", SearchOption.AllDirectories)).ToDictionary(path => path, File.ReadAllBytes);
            var result = await Run(jobs, f.Add(context, target, sources), decisions, mode == "apply_all", promptCount,
                (prompt, _) => {
                    Check(prompt.Display?.Target is not null && prompt.Display.IncomingMember is not null && flush == 0 && publish == 0,
                        "OPT02 Ask before candidate flush/publication with archive-relative context");
                });
            File.WriteAllText(Path.Combine(f.Root, "result.json"), JsonSerializer.Serialize(new { format, scenario = mode, terminal = result, flush, commit, publish }, JsonDefaults.Options));
            Check(result.CleanupComplete && sourceBytes.All(pair => File.ReadAllBytes(pair.Key).SequenceEqual(pair.Value)), "OPT02 source bytes and actual cleanup");
            foreach (var path in sourceBytes.Keys) using (var sourceRead = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.None)) Check(sourceRead.Length == sourceBytes[path].Length, "OPT02 protected source released");
            Check(Directory.GetFiles(Path.GetDirectoryName(f.Archive)!, ".nxpart-*").Length == 0, "OPT02 no owned candidate remains");
            if (mode is "cancel" or "mismatch" or "skip" or "implicit_skip" or "explicit_skip" or "merge_only") {
                Check(result.FilesCommitted == 0 && (result.Journal?.Count ?? 0) == 0 && flush == 0 && commit == 0 && publish == 0 &&
                    File.ReadAllBytes(f.Archive).SequenceEqual(original), "OPT02 zero candidate/publication and exact original");
                Check(mode == "cancel" ? result.Outcome == "cancelled" : mode == "mismatch" ? result.Outcome == "failed" :
                    (result.Outcome is "succeeded" or "succeeded_with_warnings"), "OPT02 no-change/refusal terminal");
                if (mode != "mismatch") await f.Oracle(expected);
                continue;
            }
            Check(result.Outcome == "succeeded" && result.FilesCommitted == 1 && result.Journal is { Count: 1 } &&
                result.Journal[0] is { State: "Committed", Published: true, Replaced: true } && flush == 1 && commit == 1 && publish == 1,
                "OPT02 exactly one verified native candidate and atomic publication: " + mode + "/" + format + "/" + result.Code);
            switch (mode) {
                case "overwrite": case "apply_all": expected[oldName] = "source-B"; break;
                case "auto_rename": expected["folder/" + originalName[..^4] + " (2).txt"] = "source-A"; expected["folder/" + originalName[..^4] + " (3).txt"] = "source-B"; break;
                case "future": case "future_reverse": case "future_case":
                    expected["folder/" + originalName[..^4] + " (2).txt"] = "future-2";
                    expected["folder/" + (mode == "future_case" ? originalName[..^4].ToUpperInvariant() + " (3).TXT" : originalName[..^4] + " (3).txt")] = "source-A"; break;
                case "merge": expected[oldName] = "nested-incoming"; break;
                case "empty": expected["new-empty/"] = null; break;
            }
            await f.Oracle(expected);
            using var exclusive = new FileStream(f.Archive, FileMode.Open, FileAccess.Read, FileShare.None);
        }
        if (format is "Zip" or "SevenZ") await Opt02Encrypted(format);
    }

    private static async Task Opt02Encrypted(string format)
    {
        foreach (var header in format == "Zip" ? new[] { false } : new[] { false, true })
        foreach (var mode in new[] { "correct", "wrong_password", "password_cancel" }) {
            Console.WriteLine("OPT02 encrypted " + format + "/header=" + header + "/" + mode);
            var f = new Opt02Fixture(format, encrypted: true, header: header); await f.Seed();
            var before = File.ReadAllBytes(f.Archive); var context = await f.Browse();
            await using var jobs = ArchiveJobSession.ForOwnedTestPackage(f.Root);
            var publication = 0; var flush = 0; var commit = 0;
            jobs.ProbeOutputCommitForFixture(stage => { if (stage == "native_published") publication++; if (stage == "before_flush") flush++; if (stage == "commit_in_flight") commit++; });
            var source = f.Input("encrypted-add", "keep.txt", "new-encrypted-add");
            var cancelled = mode == "password_cancel";
            var result = await Run(jobs, f.Add(context, null, source), ["auto_rename"], expectedPrompts: mode == "correct" ? 1 : 0, password: f.Password,
                wrongPassword: mode == "wrong_password", cancelPassword: cancelled, expectedPasswordPrompts: 1);
            File.WriteAllText(Path.Combine(f.Root, "result.json"), JsonSerializer.Serialize(new { format, header, scenario = mode, terminal = result, flush, commit, publish = publication }, JsonDefaults.Options));
            Check(result.CleanupComplete && File.ReadAllText(source) == "new-encrypted-add" && Directory.GetFiles(Path.GetDirectoryName(f.Archive)!, ".nxpart-*").Length == 0, "OPT02 encrypted source/cleanup");
            using (var released = new FileStream(source, FileMode.Open, FileAccess.Read, FileShare.None)) Check(released.Length > 0, "OPT02 encrypted input released");
            using (var released = new FileStream(f.Archive, FileMode.Open, FileAccess.Read, FileShare.None)) Check(released.Length > 0, "OPT02 encrypted archive released");
            if (mode != "correct") {
                Check(cancelled ? result.Outcome == "cancelled" : result.Outcome == "failed" && result.Code == "authentication_or_data_error", "OPT02 password cancellation/ambiguous authentication truth");
                Check(result.FilesCommitted == 0 && (result.Journal?.Count ?? 0) == 0 && flush == 0 && commit == 0 && publication == 0 && File.ReadAllBytes(f.Archive).SequenceEqual(before), "OPT02 password refusal keeps exact source and publishes nothing");
                continue;
            }
            Check(result.Outcome == "succeeded" && result.FilesCommitted == 1 && flush == 1 && commit == 1 && publication == 1 &&
                result.Journal is { Count: 1 } && result.Journal[0] is { State: "Committed", Published: true }, "OPT02 encrypted one-shot publication");
            var expected = new Dictionary<string, string?>(f.Expected) { ["keep (2).txt"] = "new-encrypted-add" };
            await f.Oracle(expected);
            if (format == "Zip") {
                using var zip = ZipFile.OpenRead(f.Archive);
                using var plain = new StreamReader(zip.GetEntry("keep (2).txt")!.Open());
                Check(plain.ReadToEnd() == "new-encrypted-add", "OPT02 ZIP new member is independently readable without original password");
                var listing = await f.Process(Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_7ZIP_ORACLE")!, "l", "-slt", f.Archive);
                var sections = listing.Replace("\r\n", "\n").Split("\n\n", StringSplitOptions.RemoveEmptyEntries);
                var added = sections.Single(section => section.Contains("Path = keep (2).txt\n", StringComparison.Ordinal));
                var retained = sections.Single(section => section.Contains("Path = keep.txt\n", StringComparison.Ordinal));
                Check(added.Contains("Method = Deflate\n", StringComparison.Ordinal) && added.Contains("Encrypted = -\n", StringComparison.Ordinal) &&
                    retained.Contains("Encrypted = +\n", StringComparison.Ordinal), "OPT02 new ZIP Deflate/plain and retained old encryption");
            }
            Check(!File.ReadAllBytes(f.Archive).SequenceEqual(before) && File.ReadAllText(source) == "new-encrypted-add", "OPT02 encrypted archive changed, source retained");
        }
    }

    internal static Task Opt02AddNamespaceCorrectnessZip() => Opt02Matrix("Zip");
    internal static Task Opt02AddNamespaceCorrectnessSevenz() => Opt02Matrix("SevenZ");
    internal static async Task Opt02AddNamespaceCorrectnessTarFamily() {
        foreach (var format in new[] { "Tar", "TarGz", "TarXz" }) await Opt02Matrix(format);
    }
}
