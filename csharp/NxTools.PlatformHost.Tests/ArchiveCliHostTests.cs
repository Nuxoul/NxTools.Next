using System.Text.Json;
using System.Text.Json.Nodes;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

// Pure arguments/dispatch plus inactive original sessions. No processes, files, UI or native effects.
internal static class ArchiveCliHostTests
{
    private static void Check(bool value, string detail)
    {
        if (!value) throw new InvalidOperationException(detail);
    }

    public static async Task ExactArgumentsAndEarlyMode()
    {
        var session = Guid.NewGuid();
        var pipe = "nxtools-archive-cli-" + session.ToString("D");
        string[] valid = ["--archive-cli", "--parent-pid", "42", "--pipe-name", pipe];
        Check(ArchiveCliHost.IsMode(valid) && ArchiveCliHost.TryParse(valid, out var options) &&
            options.ParentPid == 42 && options.PipeName == pipe && options.SessionId == session, "exact production arguments");
        Check(!ArchiveCliHost.IsMode([]) && !ArchiveCliHost.IsMode(["--archive-cli-other"]), "mode is an exact token");

        string[][] invalid = [
            ["--archive-cli"],
            ["--parent-pid", "42", "--archive-cli", "--pipe-name", pipe],
            ["--extension-host", "--archive-cli"],
            ["--archive-cli", "--parent-pid", "0", "--pipe-name", pipe],
            ["--archive-cli", "--parent-pid", "-1", "--pipe-name", pipe],
            ["--archive-cli", "--parent-pid", "+42", "--pipe-name", pipe],
            ["--archive-cli", "--parent-pid", " 42", "--pipe-name", pipe],
            ["--archive-cli", "--parent-pid", "4294967296", "--pipe-name", pipe],
            ["--archive-cli", "--parent-pid", Environment.ProcessId.ToString(), "--pipe-name", pipe],
            ["--archive-cli", "--parent-pid", "42", "--pipe-name", pipe, "--install-indexer-service"],
            ["--archive-cli", "--parent-pid", "42", "--pipe-name", pipe, "--archive-cli"]
        ];
        foreach (var args in invalid)
        {
            Check(ArchiveCliHost.IsMode(args) && !ArchiveCliHost.TryParse(args, out _), "invalid mode must remain claimed");
            Check(await NxTools.PlatformHost.Program.Main(args) == 2, "invalid mode must reject before normal runtime");
        }
        foreach (var candidate in new[]
        {
            "", "nxtools-next-platform-v1", @"\\.\pipe\" + pipe, pipe + "/other", pipe + " ",
            "NXTOOLS-ARCHIVE-CLI-" + session.ToString("D"), "nxtools-archive-cli-" + session.ToString("N"),
            "nxtools-archive-cli-" + session.ToString("B"), "nxtools-archive-cli-" + Guid.Empty.ToString("D"),
            "nxtools-archive-cli-ABCDEFAB-CDEF-ABCD-EFAB-CDEFABCDEFAB"
        })
        {
            var args = valid.ToArray(); args[4] = candidate;
            Check(!ArchiveCliHost.TryParse(args, out _) && await NxTools.PlatformHost.Program.Main(args) == 2,
                "noncanonical pipe must reject before normal runtime");
        }
        var maximum = valid.ToArray(); maximum[2] = uint.MaxValue.ToString();
        Check(ArchiveCliHost.TryParse(maximum, out _), "PID parser must retain positive uint range");
    }

    private static ArchiveReadCommand Read(string action) => new()
    {
        Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = 1,
        Action = action, Path = action is "open" or "test" ? @"C:\owned\tiny.zip" : null,
        Sequence = action == "ack" ? 1ul : null
    };
    private static ArchiveJobCommand Job(string action = "start") => new()
    {
        Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = 1,
        Action = action, Request = action != "start" ? null : new()
        {
            Kind = "extract", Paths = [@"C:\owned\tiny.zip"], Destination = @"C:\owned\output",
            Extract = new() { Strategy = "here", Conflict = "skip", SingleFileFolder = false, CopyMotw = true }
        }
    };
    private static ProtocolEnvelope Wire(ArchiveReadCommand command) =>
        ProtocolEnvelope.Create("platform_command", new { command = "archive_read", data = command }, command.OperationId);
    private static ProtocolEnvelope Wire(ArchiveJobCommand command) =>
        ProtocolEnvelope.Create("platform_command", new { command = "archive_job", data = command }, command.OperationId);
    private static ProtocolEnvelope Mutate(ProtocolEnvelope envelope, Action<JsonNode> change)
    {
        var node = JsonSerializer.SerializeToNode(envelope, JsonDefaults.Options)!;
        change(node);
        return node.Deserialize<ProtocolEnvelope>(JsonDefaults.Options)!;
    }

    public static Task DispatchAllowlistAndReceipts()
    {
        var readCalls = 0; var jobCalls = 0;
        ProtocolEnvelope? received = null;
        WindowsCommandResult ReadDispatch(ProtocolEnvelope envelope) { readCalls++; received = envelope; return new(true, Data: "original_read_receipt"); }
        WindowsCommandResult JobDispatch(ProtocolEnvelope envelope) { jobCalls++; received = envelope; return new(true, Data: "original_job_receipt"); }
        foreach (var action in new[] { "open", "test", "ack", "cancel" })
        {
            var request = Wire(Read(action));
            var result = ArchiveCliHost.DispatchCommand(request, ReadDispatch, JobDispatch);
            Check(ReferenceEquals(request, received) && result.Type == "platform_command_result" &&
                result.CorrelationId == request.Id && result.Payload.GetProperty("success").GetBoolean() &&
                result.Payload.GetProperty("data").GetString() == "original_read_receipt", "read must retain original wire/receipt");
        }
        foreach (var action in new[] { "start", "cancel", "pause", "resume", "reply" })
        {
            var command = Job(action);
            if (action == "reply") command = command with
            {
                Reply = new() { Ticket = new(command.OperationId, Guid.NewGuid(), 1), Kind = "conflict", Decision = "skip", ApplyToAll = false }
            };
            var request = Wire(command);
            var result = ArchiveCliHost.DispatchCommand(request, ReadDispatch, JobDispatch);
            Check(ReferenceEquals(request, received) && result.CorrelationId == request.Id &&
                result.Payload.GetProperty("success").GetBoolean() && result.Payload.GetProperty("data").GetString() == "original_job_receipt",
                "extract and controls must retain original wire/receipt");
        }
        Check(readCalls == 4 && jobCalls == 5, "only two original owners may receive commands");
        return Task.CompletedTask;
    }

    public static Task DeniedCommandsHaveNoEffects()
    {
        var calls = 0;
        WindowsCommandResult Forbidden(ProtocolEnvelope _) { calls++; throw new InvalidOperationException("denied command reached an owner"); }
        var extract = Job();
        var create = extract with { Request = new()
        {
            Kind = "create", Paths = [@"C:\owned\input.txt"], Destination = @"C:\owned\output",
            Create = new() { Format = "zip", Level = "normal", Method = "deflate", StripSingleRoot = false,
                ExcludePatterns = [], TestAfterCreate = false, Name = "out.zip" }
        } };
        var edit = extract with { Request = new() { Kind = "modify", Paths = [@"C:\owned\tiny.zip"] } };
        var denied = new List<ProtocolEnvelope>
        {
            Wire(create), Wire(edit), Wire(extract with { Request = new() { Kind = "test", Paths = [@"C:\owned\tiny.zip"] } }),
            Wire(extract with { Request = extract.Request! with { Readmission = new(Guid.NewGuid(), null) } }),
            Wire(Read("open") with { Version = 2, Limits = new() { BrowserContext = true } }),
            Wire(Read("extract")), Wire(Job("unknown")), Wire(Read("open")) with { ProtocolVersion = 2 },
            Wire(Read("open")) with { Type = "platform_event" },
            Mutate(Wire(Read("open")), n => n["payload"]!["data"]!["worker_path"] = @"C:\untrusted.exe"),
            Mutate(Wire(Read("open")), n => n["payload"]!["data"]!["password"] = "private-sentinel"),
            Mutate(Wire(extract), n => n["payload"]!["data"]!["request"]!["delete_source"] = true),
            Mutate(Wire(extract), n => n["payload"]!["data"]!["request"]!["kind"] = "Extract"),
            ProtocolEnvelope.Create("platform_command", new { command = 17, data = new { } }),
            ProtocolEnvelope.Create("platform_command", new { data = new { } })
        };
        foreach (var command in new[] { "archive_entry", "archive_picker", "archive_browser_read", "archive_password_reply",
            "archive_virtual_drag", "archive_virtual_drag_spike", "open_shell_target", "install_indexer_service",
            "apply_archive_integration", "floating_assistant", "set_clipboard_text", "configure_global_hotkeys" })
            denied.Add(ProtocolEnvelope.Create("platform_command", new { command, data = new { } }));
        foreach (var request in denied)
        {
            var result = ArchiveCliHost.DispatchCommand(request, Forbidden, Forbidden);
            Check(result.Type == "platform_command_result" && result.CorrelationId == request.Id &&
                !result.Payload.GetProperty("success").GetBoolean() &&
                result.Payload.GetProperty("error").GetString() == "archive_cli_command_denied" &&
                result.Payload.GetProperty("data").ValueKind == JsonValueKind.Null, "fixed correlated rejection without private data");
        }
        Check(calls == 0, "denied source shapes must have zero effects");
        return Task.CompletedTask;
    }

    public static async Task StaleControlsUseOriginalSessions()
    {
        await using var reads = new ArchiveReadSession();
        await using var jobs = new ArchiveJobSession();
        foreach (var action in new[] { "ack", "cancel" })
        {
            var result = ArchiveCliHost.DispatchCommand(Wire(Read(action)), e => reads.Dispatch(e), jobs.Dispatch);
            Check(!result.Payload.GetProperty("success").GetBoolean() &&
                result.Payload.GetProperty("error").GetString() == "invalid_request", "read ownership must remain in original session");
        }
        foreach (var action in new[] { "cancel", "pause", "resume" })
        {
            var result = ArchiveCliHost.DispatchCommand(Wire(Job(action)), e => reads.Dispatch(e), jobs.Dispatch);
            Check(!result.Payload.GetProperty("success").GetBoolean() &&
                result.Payload.GetProperty("error").GetString() == "stale_operation", "job controls must target an original active job");
        }
        Check(reads.ActiveCount == 0 && !reads.Events.TryRead(out _) && !jobs.Events.TryRead(out _), "stale controls must not start workers");
    }
}
