using System.Collections;
using System.Reflection;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Threading.Channels;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Original Session.OnFrame -> serialized Job event -> Dispatch -> original Run input/pump.
// Only controlled in-memory worker frames and SAME_PROCESS protected memory; no codec or UI.
internal static class Opt07BatchPasswordTests
{
    private const BindingFlags Members = BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic;
    private static readonly byte[] First = Encoding.UTF8.GetBytes("owned-batch-first");
    private static readonly byte[] Second = Encoding.UTF8.GetBytes("owned-batch-second");
    private static void Check(bool value, string message) { if (!value) throw new InvalidOperationException(message); }
    private static object Field(object owner, string name) => owner.GetType().GetField(name, Members)!.GetValue(owner)!;
    private static object? Call(object owner, string name, params object?[] args) => owner.GetType().GetMethod(name, Members)!.Invoke(owner, args);
    private sealed class CaptureLog : IPlatformLog
    {
        internal readonly List<string> Records = [];
        public void Write(string level, string name, Guid? id = null, IReadOnlyDictionary<string, object?>? fields = null) =>
            Records.Add(JsonSerializer.Serialize(new { level, name, id, fields }));
    }
    private sealed record Member(ArchiveJobCommand Command, object Operation, object Run)
    {
        internal Channel<byte[]> Input => (Channel<byte[]>)Field(Run, "Input");
    }
    private sealed class Fixture : IAsyncDisposable
    {
        internal readonly string Root;
        internal readonly CaptureLog Log = new();
        internal readonly ArchiveJobSession Session;
        internal readonly ArchiveWorkerRunner Runner;
        private readonly List<Member> members = [];
        internal Fixture()
        {
            Root = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR") ?? throw new InvalidOperationException("Session scratch required"), "archive-product-opt07-password-" + Guid.NewGuid());
            Directory.CreateDirectory(Root);
            Session = new ArchiveJobSession(Log);
            Runner = (ArchiveWorkerRunner)Field(Session, "runner");
        }
        internal Member Add(Guid? batch, ulong generation = 1, string kind = "test")
        {
            Check(((IDictionary)Field(Session, "active")).Count < 2, "original two-running fixture budget");
            var command = new ArchiveJobCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = generation,
                Action = "start", BatchId = batch, Request = new() { Kind = kind, Paths = [Path.Combine(Root, "owned.zip")],
                    Destination = kind == "extract" ? Path.Combine(Root, "output") : null,
                    Extract = kind == "extract" ? new() { Strategy = "here", SingleFileFolder = false, Conflict = "ask", CopyMotw = false } : null } };
            var operationType = typeof(ArchiveJobSession).GetNestedType("Operation", BindingFlags.NonPublic)!;
            var operation = Activator.CreateInstance(operationType, Members, null, [command], null)!;
            var request = new ArchiveWorkerRequest(command.OperationId, kind, command.Request.Paths[0], 0, new { }, PasswordInteraction: true, JobExecution: true);
            var runType = typeof(ArchiveWorkerRunner).GetNestedType("Run", BindingFlags.NonPublic)!;
            var start = (byte[])typeof(ArchiveWorkerRunner).GetMethod("Pack", BindingFlags.Static | BindingFlags.NonPublic)!.Invoke(null, [new { schema = 1, operation_id = command.OperationId, op = "test" }])!;
            var run = Activator.CreateInstance(runType, Members, null, [request, ArchiveWorkerRunner.DefaultMemory, start], null)!;
            var member = new Member(command, operation, run);
            Check(member.Input.Reader.TryRead(out var initial), "original Run start frame");
            CryptographicOperations.ZeroMemory(initial!);
            ((IDictionary)Field(Session, "active")).Add(command.OperationId, operation);
            ((IDictionary)Field(Runner, "active")).Add(command.OperationId, run);
            members.Add(member);
            return member;
        }
        internal async Task Frame(Member member, object frame)
        {
            var json = JsonSerializer.SerializeToElement(frame, JsonDefaults.Options);
            var result = await (Task<string?>)Call(Session, "OnFrame", member.Command.OperationId, json, CancellationToken.None)!;
            Check(result is null, "original session rejected worker frame");
        }
        internal async Task<ArchiveJobPrompt?> Prompt(Member member, uint attempt = 1, string? code = "PasswordRequired")
        {
            var ticket = new ArchiveJobTicket(member.Command.OperationId, Guid.NewGuid(), attempt);
            await Frame(member, new { op = "password_state", password_state = new { state = "awaiting_password", attempt_id = Guid.NewGuid(), attempt, error_code = code, ticket } });
            var prompts = Events().Where(e => e.Kind == "prompt").ToArray();
            Check(prompts.Length <= 1, "single current prompt");
            return prompts.SingleOrDefault()?.Prompt;
        }
        internal ArchiveJobEvent[] Events()
        {
            var values = new List<ArchiveJobEvent>();
            while (Session.Events.TryRead(out var envelope)) values.Add(envelope.Payload.GetProperty("data").Deserialize<ArchiveJobEvent>(JsonDefaults.Options)!);
            return values.ToArray();
        }
        internal bool Dispatch(ArchiveJobCommand command)
        {
            // Roundtrip the actual numeric-array secret converter and strict production parser.
            var envelope = ProtocolEnvelope.Create("platform_command", new { command = "archive_job", data = command }, command.OperationId);
            envelope = JsonSerializer.Deserialize<ProtocolEnvelope>(JsonSerializer.Serialize(envelope, JsonDefaults.Options), JsonDefaults.Options)!;
            return Session.Dispatch(envelope).Success;
        }
        internal bool Reply(Member member, ArchiveJobPrompt prompt, byte[]? password, bool remember = false, Func<ArchiveJobCommand, ArchiveJobCommand>? change = null)
        {
            var command = member.Command with { Action = "reply", Request = null,
                Reply = new() { Ticket = prompt.Ticket, Kind = "password", ApplyToAll = false, Password = password, RememberBatchPassword = remember } };
            return Dispatch(change is null ? command : change(command));
        }
        internal void Retire(Guid batch) => Check(Dispatch(new() { Version = 1, OperationId = batch, SessionId = batch, Generation = 1,
            Action = "retire_batch", BatchId = batch }), "serialized original batch retirement");
        internal byte[] Protected(Guid batch) => (byte[])Field(((IDictionary)Field(Session, "batchSecrets"))[batch]!, "protectedBytes");
        internal int SecretCount => ((IDictionary)Field(Session, "batchSecrets")).Count;
        internal async Task Input(Member member, byte[]? expected)
        {
            Check(member.Input.Reader.TryPeek(out var frame), "original Run did not receive password reply");
            member.Input.Writer.TryComplete();
            using var sink = new MemoryStream();
            await (Task)Call(Runner, "SendInput", member.Run, sink)!;
            Check(frame!.All(value => value == 0) && sink.Length > 0, "original password input pump must drain/zero");
            using var json = JsonDocument.Parse(sink.GetBuffer().AsMemory(4, checked((int)sink.Length - 4)));
            var reply = json.RootElement.GetProperty("password_reply");
            var ticket = reply.GetProperty("ticket").Deserialize<ArchivePasswordTicket>()!;
            Check(ticket.OperationId == member.Command.OperationId, "Run ticket operation isolation");
            Check(expected is null ? reply.GetProperty("password").ValueKind == JsonValueKind.Null : reply.GetProperty("password").GetString() == Encoding.UTF8.GetString(expected), "Run received another password");
            CryptographicOperations.ZeroMemory(sink.GetBuffer());
        }
        internal void Finish(Member member, string? failure = null)
        {
            if (failure is not null) member.Run.GetType().GetMethod("Fail", Members)!.Invoke(member.Run, [failure, null]);
            var terminal = (ArchiveWorkerEvent)typeof(ArchiveWorkerRunner).GetMethod("ResolveTerminal", BindingFlags.Static | BindingFlags.NonPublic)!.Invoke(null, [member.Run, true, (uint?)0])!;
            Call(Session, "OnRunnerEvent", terminal);
            ((IDictionary)Field(Runner, "active")).Remove(member.Command.OperationId);
        }
        public async ValueTask DisposeAsync()
        {
            Session.Stop();
            foreach (var member in members)
            {
                member.Input.Writer.TryComplete();
                using var sink = new MemoryStream();
                await (Task)Call(Runner, "SendInput", member.Run, sink)!;
                ((IDictionary)Field(Runner, "active")).Remove(member.Command.OperationId);
                foreach (var field in member.Run.GetType().GetFields(Members))
                    if (field.GetValue(member.Run) is CancellationTokenSource cancellation) cancellation.Dispose();
            }
            await Session.DisposeAsync();
            Check(Log.Records.All(row => !row.Contains(Root, StringComparison.Ordinal) && !row.Contains(Encoding.UTF8.GetString(First), StringComparison.Ordinal) && !row.Contains(Encoding.UTF8.GetString(Second), StringComparison.Ordinal)), "password/path in diagnostic output");
            Directory.Delete(Root, true);
        }
    }
    private static async Task Seed(Fixture fixture, Guid batch, bool remember = true, byte[]? password = null)
    {
        var member = fixture.Add(batch);
        var prompt = await fixture.Prompt(member) ?? throw new InvalidOperationException("first explicit prompt required");
        Check(fixture.Reply(member, prompt, password ?? First, remember), "current explicit password reply");
        await fixture.Input(member, password ?? First);
        await fixture.Frame(member, new { op = "complete", files_verified = 1, bytes_verified = 1 });
        fixture.Finish(member);
        fixture.Events();
    }
    public static async Task InitialPromptReuse()
    {
        await using var fixture = new Fixture();
        var batch = Guid.NewGuid();
        await Seed(fixture, batch, false);
        Check(fixture.SecretCount == 0, "unchecked answer was retained");
        await Seed(fixture, batch);
        var protectedBytes = fixture.Protected(batch);
        Check(protectedBytes.Length % 16 == 0 && !protectedBytes.AsSpan(0, First.Length).SequenceEqual(First), "retained bytes must be CryptProtectMemory ciphertext");
        var next = fixture.Add(batch, kind: "extract");
        Check(await fixture.Prompt(next) is null, "eligible original attempt-1 PasswordRequired prompt did not reuse protected batch password");
        await fixture.Input(next, First);
        fixture.Retire(batch);
        Check(protectedBytes.All(value => value == 0) && fixture.SecretCount == 0, "retirement did not zero ciphertext");
    }
    public static async Task RetiredPendingReply()
    {
        await using var fixture = new Fixture();
        var batch = Guid.NewGuid();
        var member = fixture.Add(batch);
        var prompt = (await fixture.Prompt(member))!;
        fixture.Retire(batch);
        Check(!fixture.Reply(member, prompt, First, true), "late opted-in reply repopulated a retired batch");
        Check(fixture.SecretCount == 0 && !member.Input.Reader.TryPeek(out _), "retired answer reached protected owner/Run");
        Check(fixture.Reply(member, prompt, First), "retirement must not cancel a current member-only answer");
        await fixture.Input(member, First);
    }
    public static async Task RetryGenerationIsolation()
    {
        await using var fixture = new Fixture();
        var member = fixture.Add(Guid.NewGuid(), generation: 2);
        var prompt = (await fixture.Prompt(member))!;
        Check(!fixture.Reply(member, prompt, First, true), "retry generation installed an original-batch candidate");
        Check(fixture.SecretCount == 0, "retry leaked password across attempts");
        Check(fixture.Reply(member, prompt, First), "retry still needs current member password support");
        await fixture.Input(member, First);
    }
    public static async Task TicketAndBatchIsolation()
    {
        await using var fixture = new Fixture();
        var first = fixture.Add(Guid.NewGuid()); var second = fixture.Add(Guid.NewGuid());
        var a = (await fixture.Prompt(first))!; var b = (await fixture.Prompt(second))!;
        Check(!fixture.Reply(first, a, First, true, c => c with { SessionId = Guid.NewGuid() }), "wrong session accepted");
        Check(!fixture.Reply(first, a, First, true, c => c with { Generation = 2 }), "wrong generation accepted");
        Check(!fixture.Reply(first, a, First, true, c => c with { BatchId = second.Command.BatchId }), "wrong batch accepted");
        Check(!fixture.Reply(first, a, First, true, c => c with { Reply = c.Reply! with { Ticket = a.Ticket with { PromptId = b.Ticket.PromptId } } }), "another ticket accepted");
        Check(fixture.Reply(first, a, First, true) && !fixture.Reply(first, a, First, true), "first ticket must answer once");
        Check(fixture.Reply(second, b, Second, true) && !fixture.Reply(second, b, Second, true), "second ticket must answer independently once");
        await fixture.Input(first, First); await fixture.Input(second, Second);
        Check(fixture.SecretCount == 2, "independent batch candidates crossed");
        fixture.Retire(first.Command.BatchId!.Value);
        Check(fixture.SecretCount == 1, "retiring one batch cleared another");
    }
    public static async Task WrongPasswordAndAmbiguity()
    {
        await using var fixture = new Fixture();
        var batch = Guid.NewGuid(); await Seed(fixture, batch);
        var protectedBytes = fixture.Protected(batch);
        var member = fixture.Add(batch);
        await fixture.Frame(member, new { op = "password_state", password_state = new { state = "attempt_finished", attempt_id = Guid.NewGuid(), attempt = 2, error_code = "WrongPassword" } });
        Check(fixture.SecretCount == 0 && protectedBytes.All(value => value == 0), "confirmed WrongPassword must invalidate before another reuse");
        var wrong = await fixture.Prompt(member, 2, "WrongPassword");
        Check(wrong?.ErrorCode == "WrongPassword" && !member.Input.Reader.TryPeek(out _), "wrong password was automatically resubmitted");
        Check(fixture.Reply(member, wrong!, Second, true), "fresh different password must remain available");
        await fixture.Input(member, Second);
        await fixture.Frame(member, new { op = "failure", error_code = "AuthenticationOrDataError" });
        fixture.Finish(member, "worker_failure");
        var terminal = fixture.Events().Single(e => e.Kind == "terminal").Terminal!;
        Check(terminal.Code == "authentication_or_data_error" && terminal.Outcome == "failed", "ambiguous authentication/data failure was relabelled WrongPassword");
        var next = fixture.Add(batch);
        Check(await fixture.Prompt(next) is null, "different explicit batch password did not replace the invalid candidate");
        await fixture.Input(next, Second);
        protectedBytes = fixture.Protected(batch);
        await fixture.Frame(next, new { op = "failure", error_code = "WrongPassword" });
        Check(fixture.SecretCount == 0 && protectedBytes.All(value => value == 0), "confirmed terminal frame must invalidate before runner exit");
        fixture.Finish(next, "worker_failure");
        Check(fixture.Events().Single(e => e.Kind == "terminal").Terminal!.Code == "wrong_password", "confirmed wrong-password category changed");
    }
    public static async Task CancelDisconnectAndDrain()
    {
        await using var fixture = new Fixture();
        var batch = Guid.NewGuid(); await Seed(fixture, batch);
        var protectedBytes = fixture.Protected(batch);
        var member = fixture.Add(batch);
        Check(fixture.Dispatch(member.Command with { Action = "cancel", Request = null }), "original cancel route");
        Check(protectedBytes.All(value => value == 0) && fixture.SecretCount == 0, "cancel retained batch password");
        Check(fixture.Runner.ActiveCount == 1, "cancel released original Run before actual drain");
        fixture.Finish(member, "cancelled"); fixture.Events();
        await Seed(fixture, batch);
        protectedBytes = fixture.Protected(batch);
        fixture.Session.Stop();
        Check(protectedBytes.All(value => value == 0) && fixture.SecretCount == 0, "disconnect retained protected batch password");
        await using var independent = new Fixture();
        var isolated = independent.Add(batch);
        Check(await independent.Prompt(isolated) is not null && !isolated.Input.Reader.TryPeek(out _), "password escaped original pipe session");
    }
}
