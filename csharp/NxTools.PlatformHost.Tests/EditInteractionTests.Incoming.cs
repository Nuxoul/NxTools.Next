using System.IO.Compression;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Tests;

internal static partial class EditInteractionTests
{
    // Owned temporary files, original Browse/Job/worker/native output; no GUI or user archive.
    internal static async Task IncomingBackslashDecisions()
    {
        var worker = MatchingWorker();
        foreach (var decision in new[] { "overwrite", "skip", "auto_rename", "cancel" })
        {
            var root = OwnedRoot(worker);
            var path = Path.Combine(root, "archive", "decisions.zip");
            using (var zip = ZipFile.Open(path, ZipArchiveMode.Create))
            {
                zip.CreateEntry("folder\\");
                using (var writer = new StreamWriter(zip.CreateEntry("folder\\same.txt").Open())) writer.Write("old-content");
                using var keep = new StreamWriter(zip.CreateEntry("keep.txt").Open()); keep.Write("keep");
            }
            var source = Path.Combine(root, "same.txt"); File.WriteAllText(source, "incoming");
            var before = File.ReadAllBytes(path);
            var current = await IncomingBrowse(root, path);
            await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root);
            var publications = 0;
            jobs.ProbeOutputCommitForFixture(stage => { if (stage == "native_published") publications++; });
            var command = new ArchiveJobCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = 1, Action = "start", Request = new() { Kind = "modify", Paths = [path, source], Modify = new(current.Context, "add", [], "folder/") } };
            Check(jobs.Dispatch(Wire(command)).Success, "original admission");
            ArchiveJobTerminal? terminal = null;
            var prompts = 0;
            while (terminal is null)
            {
                var wire = await jobs.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(40));
                var item = wire.Payload.GetProperty("data").Deserialize<ArchiveJobEvent>(JsonDefaults.Options)!;
                if (item.Prompt is { } prompt)
                {
                    prompts++;
                    Check(prompt.Kind == "conflict" && prompt.Display is { IncomingMember: "folder/same.txt", Target: "folder/same.txt" }, "normalized namespace retains actual conflict");
                    Check(jobs.Dispatch(Wire(command with { Action = "reply", Request = null, Reply = new() { Ticket = prompt.Ticket, Kind = "conflict", Decision = decision == "cancel" ? null : decision, ApplyToAll = decision != "cancel" } })).Success, "original ticket decision");
                }
                terminal = item.Terminal;
            }
            Check(prompts == 1 && terminal.CleanupComplete && File.ReadAllText(source) == "incoming", "one prompt, source retained, cleanup complete");
            if (decision is "skip" or "cancel")
            {
                Check(publications == 0 && terminal.OutputPublished == false && File.ReadAllBytes(path).SequenceEqual(before), "affirmative zero publication and exact original");
                Check(decision == "skip" ? terminal.Outcome == "succeeded_with_warnings" && terminal.FilesSkipped == 1 : terminal.Outcome == "cancelled", "truthful decision outcome");
            }
            else
            {
                Check(terminal.Outcome == "succeeded" && publications == 1 && terminal.Journal is { Count: 1 } && terminal.Journal[0].Published == true, "single publication");
                using var zip = ZipFile.OpenRead(path);
                using (var reader = new StreamReader(zip.GetEntry(decision == "overwrite" ? "folder/same.txt" : "folder/same (2).txt")!.Open())) Check(reader.ReadToEnd() == "incoming", "independent content binding");
                Check(zip.GetEntry("folder\\") is not null, "unchanged directory record retained");
                Check(zip.Entries.Count == (decision == "overwrite" ? 3 : 4), "no duplicate or deleted unrelated member");
            }
            Check(Directory.GetFiles(Path.GetDirectoryName(path)!, ".nxpart-*").Length == 0, "actual candidate cleanup");
            await jobs.DisposeAsync();
            using (File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
            Directory.Delete(root, true);
            Console.WriteLine($"INCOMING_DECISION decision={decision} publications={publications} cleanup=true");
        }
    }

    private static async Task<(ArchiveBrowserContext Context, List<ArchiveReadEntry> Entries)> IncomingBrowse(string root, string path)
    {
        await using var reads = ArchiveReadSession.ForOwnedTestPackage(root);
        var wire = ArchiveProductReadTests.Command(path: path, version: 2, limits: new() { BrowserContext = true });
        var command = ArchiveReadWire.Parse(wire);
        var entries = new List<ArchiveReadEntry>();
        Check(reads.Dispatch(wire).Success, "original browse admission");
        while (true)
        {
            var envelope = await reads.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(40));
            var item = envelope.Payload.GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!;
            if (item.Batch is { } batch)
            {
                entries.AddRange(batch.Entries);
                Check(reads.Dispatch(ArchiveProductReadTests.Command("ack", command.OperationId, command.SessionId, command.Generation, sequence: batch.Sequence, version: 2)).Success, "browse ACK");
            }
            if (item.Terminal is { } terminal)
            {
                Check(terminal.Status == "succeeded" && terminal.CleanupComplete && terminal.Browser is not null, "actual browsed context");
                return (terminal.Browser!, entries);
            }
        }
    }

    internal static async Task IncomingSampleCopy()
    {
        var sample = Environment.GetEnvironmentVariable("NXTOOLS_INCOMING_ADD_READONLY_SAMPLE");
        if (sample is null || !File.Exists(sample)) throw new TestSkippedException("Explicit read-only sample required");
        var worker = MatchingWorker();
        foreach (var subdirectory in new[] { false, true })
        {
            var root = OwnedRoot(worker);
            var path = Path.Combine(root, "archive", "sample-copy.zip");
            File.Copy(sample, path);
            var before = File.ReadAllBytes(path);
            var current = await IncomingBrowse(root, path);
            var prefix = subdirectory ? current.Entries.First(e => e.Name.Contains('/') || e.Name.Contains('\\')).Name.Replace('\\', '/').Split('/')[0] + "/" : null;
            var source = Path.Combine(root, "正常 拖入.txt");
            File.WriteAllText(source, "incoming-normal-file");
            await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root);
            var publications = 0;
            jobs.ProbeOutputCommitForFixture(stage => { if (stage == "native_published") publications++; });
            var command = new ArchiveJobCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = 1, Action = "start", Request = new() { Kind = "modify", Paths = [path, source], Modify = new(current.Context, "add", [], prefix) } };
            var terminal = await Terminal(jobs, command);
            Console.WriteLine($"INCOMING_SAMPLE subdirectory={subdirectory} outcome={terminal.Outcome} code={terminal.Code} publications={publications} cleanup={terminal.CleanupComplete}");
            Check(terminal.CleanupComplete && File.ReadAllText(source) == "incoming-normal-file", "cleanup/source preservation");
            if (terminal.Outcome != "succeeded") Check(publications == 0 && File.ReadAllBytes(path).SequenceEqual(before), "failed copy unchanged");
            var evidence = Environment.GetEnvironmentVariable("NXTOOLS_INCOMING_ADD_EVIDENCE");
            if (evidence is not null) { Directory.CreateDirectory(evidence); File.WriteAllText(Path.Combine(evidence, $"sample-{subdirectory}.json"), JsonSerializer.Serialize(terminal, JsonDefaults.Options)); }
            Check(terminal.Outcome == "succeeded" && publications == 1, "normal sample-copy Add must succeed");
            using (var zip = ZipFile.OpenRead(path))
            using (var reader = new StreamReader(zip.GetEntry(prefix + "正常 拖入.txt")!.Open())) Check(reader.ReadToEnd() == "incoming-normal-file", "independent added bytes");
            await jobs.DisposeAsync();
            Directory.Delete(root, true);
        }
    }

    internal static Task IncomingNormalPaths() => IncomingNormalPaths(false);
    internal static Task IncomingDesktopNativeAdd() => IncomingNormalPaths(true);

    private static async Task<ArchiveJobCommand> DesktopAdd(string root, string path, string[] sources, string folder, ArchiveBrowserContext browser, List<ArchiveReadEntry> entries)
    {
        var desktop = Environment.GetEnvironmentVariable("NXTOOLS_INCOMING_ADD_DESKTOP_FIXTURE");
        if (desktop is null || !File.Exists(desktop)) throw new TestSkippedException("Explicit Desktop test apphost required");
        File.WriteAllText(Path.Combine(root, "ui-input.json"), JsonSerializer.Serialize(new { path, sources, folder, browser, entries }, JsonDefaults.Options));
        var start = new System.Diagnostics.ProcessStartInfo(desktop) { UseShellExecute = false, RedirectStandardOutput = true, RedirectStandardError = true };
        foreach (var arg in new[] { "--exact", "archive::application::tests::incoming_add_tests::incoming_add_native_request_fixture", "--ignored", "--nocapture", "--test-threads=1" }) start.ArgumentList.Add(arg);
        start.Environment["NXTOOLS_INCOMING_ADD_UI_ROOT"] = root;
        using var child = System.Diagnostics.Process.Start(start)!;
        var stdout = child.StandardOutput.ReadToEndAsync();
        var stderr = child.StandardError.ReadToEndAsync();
        await child.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(40));
        var output = await stdout;
        File.WriteAllText(Path.Combine(root, "desktop-test.log"), output + await stderr);
        Check(child.ExitCode == 0 && output.Contains("test result: ok. 1 passed", StringComparison.Ordinal) && output.Contains("INCOMING_DESKTOP request_emitted=1 forms=0", StringComparison.Ordinal), "real Desktop button fixture must run once and export original request");
        var command = JsonSerializer.Deserialize<ArchiveJobCommand>(File.ReadAllText(Path.Combine(root, "ui-command.json")), JsonDefaults.Options)!;
        Check(command.Request is { Kind: "modify", Modify.Action: "add" } && command.Request.Paths.SequenceEqual(new[] { path }.Concat(sources)), "complete original Desktop paths");
        Console.WriteLine("INCOMING_DESKTOP request_emitted=1 forms=0 sources=" + sources.Length);
        return command;
    }

    private static async Task IncomingNormalPaths(bool desktop)
    {
        var worker = MatchingWorker();
        var failures = new List<string>();
        foreach (var separators in new[] { "slash", "backslash" })
        foreach (var prefix in new[] { "", "已有 目录/" })
        {
            var root = OwnedRoot(worker);
            var path = Path.Combine(root, "archive", "正常 压缩包.zip");
            var oldName = separators == "slash" ? "已有 目录/保留.txt" : "已有 目录\\保留.txt";
            using (var zip = ZipFile.Open(path, ZipArchiveMode.Create))
            {
                zip.CreateEntry(separators == "slash" ? "已有 目录/" : "已有 目录\\");
                using (var writer = new StreamWriter(zip.CreateEntry("folder/same.txt").Open())) writer.Write("old-content");
                using var keep = new StreamWriter(zip.CreateEntry(oldName).Open());
                keep.Write("retained");
            }
            var source = Path.Combine(root, "输入 文件.txt");
            File.WriteAllText(source, "incoming");
            var folder = Path.Combine(root, "输入 目录");
            Directory.CreateDirectory(folder);
            File.WriteAllText(Path.Combine(folder, "子 文件.txt"), "child");
            Directory.CreateDirectory(Path.Combine(folder, "空 目录"));
            var original = File.ReadAllBytes(path);
            var current = await IncomingBrowse(root, path);
            await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root);
            var publications = 0;
            jobs.ProbeOutputCommitForFixture(stage => { if (stage == "native_published") publications++; });
            var command = new ArchiveJobCommand
            {
                Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = 1, Action = "start",
                Request = new() { Kind = "modify", Paths = [path, source, folder], Modify = new(current.Context, "add", [], prefix.Length == 0 ? null : prefix) }
            };
            if (desktop) command = await DesktopAdd(root, path, [source, folder], prefix.TrimEnd('/'), current.Context, current.Entries);
            var terminal = await Terminal(jobs, command);
            Console.WriteLine($"INCOMING_ADD separators={separators} subdirectory={prefix.Length != 0} outcome={terminal.Outcome} code={terminal.Code} publications={publications} cleanup={terminal.CleanupComplete}");
            Check(terminal.CleanupComplete, "actual cleanup");
            Check(File.ReadAllText(source) == "incoming" && File.ReadAllText(Path.Combine(folder, "子 文件.txt")) == "child", "external sources preserved");
            Check(Directory.GetFiles(Path.GetDirectoryName(path)!, ".nxpart-*").Length == 0, "no owned candidate retained");
            if (terminal.Outcome != "succeeded")
            {
                Check(publications == 0 && File.ReadAllBytes(path).SequenceEqual(original), "failed attempt preserves original, measured zero publication");
                failures.Add($"{separators}/subdirectory={prefix.Length != 0}: {terminal.Code}");
            }
            else
            {
                Check(publications == 1 && terminal.Journal is { Count: 1 } && terminal.Journal[0].Published == true, "one native publication");
                using var zip = ZipFile.OpenRead(path);
                using (var reader = new StreamReader(zip.GetEntry(oldName)!.Open())) Check(reader.ReadToEnd() == "retained", "unchanged original record spelling and bytes");
                using (var reader = new StreamReader(zip.GetEntry(prefix + "输入 文件.txt")!.Open())) Check(reader.ReadToEnd() == "incoming", "independent file oracle");
                using (var reader = new StreamReader(zip.GetEntry(prefix + "输入 目录/子 文件.txt")!.Open())) Check(reader.ReadToEnd() == "child", "independent folder oracle");
                Check(zip.GetEntry(prefix + "输入 目录/空 目录/") is not null, "empty directory retained");
            }
            var evidence = Environment.GetEnvironmentVariable("NXTOOLS_INCOMING_ADD_EVIDENCE");
            if (evidence is not null)
            {
                Directory.CreateDirectory(evidence);
                File.WriteAllText(Path.Combine(evidence, $"{(desktop ? "desktop" : "native")}-{separators}-{(prefix.Length == 0 ? "root" : "folder")}.json"), JsonSerializer.Serialize(terminal, JsonDefaults.Options));
            }
            await jobs.DisposeAsync();
            Directory.Delete(root, true);
        }
        Check(failures.Count == 0, "normal Add must succeed: " + string.Join("; ", failures));
    }
}
