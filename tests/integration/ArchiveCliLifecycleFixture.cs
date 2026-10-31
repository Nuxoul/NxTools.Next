// Compiled ONLY into a scratch source-copy by archive-cli-lifecycle.py; never into product projects.
using System.Diagnostics;
using System.IO.Pipes;
using System.Reflection;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

internal static class ArchiveCliLifecycleFixture
{
    private static readonly JsonElement Settings = JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "lifecycle-control.json"))).RootElement.Clone();
    private static string Root => Settings.GetProperty("root").GetString()!;
    private static string Mode => Settings.GetProperty("mode").GetString()!;
    private static readonly object Gate = new();
    private static void Mark(string stage, object? facts = null)
    {
        lock (Gate) File.AppendAllText(Path.Combine(Root, "checkpoints.jsonl"), JsonSerializer.Serialize(new { stage, process_id = Environment.ProcessId, elapsed_tick = Stopwatch.GetTimestamp(), facts }) + "\n");
    }
    private static void Wait(string name)
    {
        using var signal = EventWaitHandle.OpenExisting(Settings.GetProperty("prefix").GetString()! + name);
        if (!signal.WaitOne(TimeSpan.FromSeconds(8))) throw new TimeoutException("Owned fixture barrier expired.");
    }
    private sealed class SafeSinkFailureWriter : System.IO.TextWriter
    {
        public override System.Text.Encoding Encoding => System.Text.Encoding.UTF8;
        public override void WriteLine(string? value) => Mark("default_sink_write_failure");
    }
    internal static PlatformLog CreateLog()
    {
        if (Settings.GetProperty("isolated_sink").GetBoolean()) return PlatformLog.CreateForDirectory(Path.Combine(Root, "logs"));
        Console.SetError(new SafeSinkFailureWriter());
        return PlatformLog.CreateDefault();
    }
    internal static void Configure(ArchiveJobSession jobs, NamedPipeClientStream pipe)
    {
        var runner = (ArchiveWorkerRunner)typeof(ArchiveJobSession).GetField("runner", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(jobs)!;
        var calls = 0;
        if (Mode is "partial_signal" or "parent_exit") runner.OutputCommitCheckpointProbe = stage => {
            if (stage == "native_published" && Interlocked.Increment(ref calls) == 1) { Mark("work_checkpoint", new { published = 1 }); Wait("release_work"); }
        };
        else if (Mode != "diagnostics") runner.OutputWriteCheckpointProbe = bytes => {
            if (Interlocked.Increment(ref calls) == 1) { Mark("work_checkpoint", new { bytes }); Wait("release_work"); }
        };
        runner.ControlInputCheckpointProbe = op => { if (op == "cancel") Mark("cancel_written"); };
        runner.CleanupCheckpointProbe = () => {
            Mark("cleanup_entered", new { active = runner.ActiveCount });
            if (Mode is "signal" or "partial_signal") Wait("release_cleanup");
        };
        if (Mode == "eof") _ = Task.Run(() => { Wait("close_pipe"); Mark("pipe_closed_by_fixture"); pipe.Dispose(); });
        Mark("configured");
    }
    internal static void Observe(ProtocolEnvelope envelope)
    {
        if (envelope.Type == "platform_command_result") {
            Mark("receipt", new { correlation_id = envelope.CorrelationId, success = envelope.Payload.GetProperty("success").GetBoolean() });
        }
        if (envelope.Type == "platform_event" && envelope.Payload.TryGetProperty("data", out var data)
            && data.TryGetProperty("kind", out var kind) && kind.GetString() == "terminal")
            Mark("terminal", new { operation_id = data.GetProperty("operation_id").GetGuid(), terminal = data.GetProperty("terminal") });
    }
}
