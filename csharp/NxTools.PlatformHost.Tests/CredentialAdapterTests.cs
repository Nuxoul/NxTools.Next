using System.Collections.Concurrent;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.ExtensionHost;
using NxTools.PlatformHost.Windows.Extensions;
using static NxTools.PlatformHost.Tests.ExtensionProcessTests;

namespace NxTools.PlatformHost.Tests;

// Fake stores exclusively. Tests call the broker's actual control dispatch/adapter boundaries;
// no CredRead/Write/Delete invocation, user store access, or Core/config transaction owner.
internal static class CredentialAdapterTests
{
    private static CredentialRequest Request(string op = "credential_set", string? value = "synthetic-secret") =>
        new(1, op, Guid.NewGuid(), "fixture.package", "password", value);

    public static async Task TypedRoutingAndValuePrivacy()
    {
        var store = new CredentialFixtureStore(); var log = new FixtureLog();
        var results = new ConcurrentQueue<CredentialResult>();
        using var runner = new ExtensionProcessRunner(_ => throw new InvalidOperationException("unexpected process event"));
        using var dispatcher = new CredentialDispatcher(new CredentialAdapter(store, log), results.Enqueue);
        var set = Request(); Dispatch(set, runner, dispatcher, results);
        await Until(() => results.Count == 1);
        Assert(results.Last().Success && results.Last().Value is null, "set must not echo secret");
        Dispatch(set with { Op = "credential_get", RequestId = Guid.NewGuid(), Value = null }, runner, dispatcher, results);
        await Until(() => results.Count == 2);
        Assert(results.Last().Found == true && results.Last().Value == set.Value, "get is intended secret pipe output");
        Dispatch(set with { Op = "credential_delete", RequestId = Guid.NewGuid(), Value = null }, runner, dispatcher, results);
        await Until(() => results.Count == 3);
        Dispatch(Request("credential_get", null), runner, dispatcher, results);
        await Until(() => results.Count == 4);
        Assert(results.Last().Success && results.Last().Found == false && results.Last().Value is null, "absence distinct from empty string");
        Assert(log.Entries.Count == 8 && log.Entries.All(e => !e.Contains("synthetic-secret") && !e.Contains(set.PackageId) && !e.Contains(set.SettingKey)),
            "logs contain only ID/fixed reason/code/elapsed milestones");
        Assert(CredentialAdapter.Target("sid1", "a", "b") == CredentialAdapter.Target("sid1", "a", "b") &&
            CredentialAdapter.Target("sid1", "a", "b") != CredentialAdapter.Target("sid2", "a", "b") &&
            CredentialAdapter.Target("sid1", "a-b", "c") != CredentialAdapter.Target("sid1", "a", "b-c"), "stable collision-free scoped target");
    }

    public static Task BoundsMalformedAndNativeMetadata()
    {
        var store = new CredentialFixtureStore(); var adapter = new CredentialAdapter(store);
        Assert(adapter.Execute(Request(value: new string('x', 32768))).Success, "32KiB protocol cap inclusive with fake store");
        Assert(adapter.Execute(Request(value: new string('x', 32769))).Reason == "credential_value_limit", "overflow must not mutate");
        Assert(adapter.Execute(Request(value: "\ud800")).Reason == "credential_invalid_utf8", "strict UTF8 before native mutation");
        Assert(!adapter.Execute(Request() with { PackageId = "../escape" }).Success, "no unbounded arbitrary target name");
        var request = Request(); var wire = JsonSerializer.Serialize(request, ExtensionProcessProtocol.Json);
        Assert(ExtensionProcessProtocol.Parse(wire).Credential == request, "typed version1 broker parser");
        foreach (var bad in new[] { wire.Replace("\"version\":1", "\"version\":2"), wire.Replace("\"op\":", "\"extra\":0,\"op\":"),
            wire.Replace("\"version\":1", "\"version\":1,\"version\":1") })
        {
            try { ExtensionProcessProtocol.Parse(bad); throw new InvalidOperationException("malformed control accepted"); }
            catch (ExtensionFailure) { }
        }
        Assert(store.Mutations == 1, "malformed/bounded requests perform no writes");
        var native = typeof(CredentialWindowsStore).GetMethods(System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static)
            .Select(m => m.GetCustomAttributes(typeof(System.Runtime.InteropServices.DllImportAttribute), false)
                .Cast<System.Runtime.InteropServices.DllImportAttribute>().SingleOrDefault()).Where(a => a is not null).ToArray();
        Assert(native.Select(a => a!.EntryPoint).Order().SequenceEqual(new[] { "CredDeleteW", "CredFree", "CredReadW", "CredWriteW" }), "exact native entrypoints");
        Assert(CredentialWindowsStore.NativeBlobLimit == 2560, "Windows native blob cap is explicit, not protocol maximum");
        return Task.CompletedTask;
    }

    public static Task CancelAndAttemptedWriteCompensationSeam()
    {
        // A caller fixture models attempted writes only; product Save/config composition stays Rust.
        var store = new CredentialFixtureStore(); var adapter = new CredentialAdapter(store);
        Assert(store.Mutations == 0, "Cancel without dispatch does not touch native store");
        var original = Request(value: "prior"); Assert(adapter.Execute(original).Success, "seed prior value");
        var before = adapter.Execute(original with { Op = "credential_get", Value = null }).Value;
        store.FailMutation = store.Mutations + 1;
        var attempted = original with { RequestId = Guid.NewGuid(), Value = "synthetic-secret" };
        var failure = adapter.Execute(attempted);
        Assert(!failure.Success && failure.Reason == "credential_store_failure" && failure.Value is null, "mutate-then-throw is truthful and payload safe");
        Assert(adapter.Execute(original with { Op = "credential_get", Value = null }).Value == "synthetic-secret", "failure does not imply no side effect");
        store.FailMutation = null;
        Assert(adapter.Execute(original with { RequestId = Guid.NewGuid(), Value = before }).Success, "parent can compensate failed attempted write");
        Assert(adapter.Execute(original with { Op = "credential_get", Value = null }).Value == "prior", "previous value restored through adapter");
        store.FailMutation = store.Mutations + 1;
        Assert(!adapter.Execute(original with { Op = "credential_delete", Value = null }).Success, "delete can mutate then throw too");
        Assert(adapter.Execute(original with { Op = "credential_get", Value = null }).Found == false, "failed delete must be compensated by caller");
        store.FailMutation = null;
        Assert(adapter.Execute(original with { RequestId = Guid.NewGuid(), Value = before }).Success, "failed-delete attempt can restore prior value");
        store.FailMutation = store.Mutations + 1;
        Assert(!adapter.Execute(original with { RequestId = Guid.NewGuid(), Value = before }).Success,
            "compensation itself can fail and must not be reported as recovered success");
        return Task.CompletedTask;
    }

    public static async Task BoundedWorkerKeepsControlResponsive()
    {
        using var release = new ManualResetEventSlim(); var entered = 0;
        var store = new CredentialFixtureStore { BeforeRead = () => { Interlocked.Increment(ref entered); release.Wait(); } };
        var results = new ConcurrentQueue<CredentialResult>(); var process = new ConcurrentQueue<ExtensionProcessEvent>();
        using var runner = new ExtensionProcessRunner(process.Enqueue);
        using var dispatcher = new CredentialDispatcher(new CredentialAdapter(store), results.Enqueue);
        try
        {
            Dispatch(Request("credential_get", null), runner, dispatcher, results, process);
            await Until(() => Volatile.Read(ref entered) == 1);
            for (var i = 0; i < 9; i++) Dispatch(Request("credential_get", null), runner, dispatcher, results, process);
            Assert(results.Any(r => r.Reason == "credential_queue_limit") && entered == 1, "one worker/eight pending reject instead of replacement workers");
            ExtensionHostBroker.DispatchControl(JsonSerializer.Serialize(new ExtensionControlRequest(1, "cancel", Guid.NewGuid()), ExtensionProcessProtocol.Json),
                runner, dispatcher, process.Enqueue, results.Enqueue);
            Assert(process.Any(e => e.Reason == "unknown_run"), "credential native call must not block process controls");
            dispatcher.Dispose();
        }
        finally { release.Set(); await dispatcher.Worker.WaitAsync(TimeSpan.FromSeconds(5)); }
        Assert(store.Mutations == 0, "queued reads/disconnect never write secrets");
    }

    public static async Task SyntheticInvokeAndHostCallProtocol()
    {
        foreach (var mode in new[] { "protocol", "protocol-host-call" })
        {
            var events = new ConcurrentQueue<ExtensionProcessEvent>(); using var runner = new ExtensionProcessRunner(events.Enqueue);
            var request = ExtensionProcessTests.Request(mode);
            request = request with { Stdin = JsonSerializer.Serialize(new { protocol_version = 1, op = "invoke", run_id = request.RunId }) };
            Assert(runner.Start(request), "protocol fixture start");
            if (mode == "protocol-host-call")
            {
                await Until(() => events.Any(e => e.Op == "stdout"));
                using var call = JsonDocument.Parse(events.First(e => e.Op == "stdout").Data!);
                Assert(call.RootElement.GetProperty("method").GetString() == "host.describe", "typed fixture host call");
                Assert(runner.Write(request.RunId, JsonSerializer.Serialize(new { protocol_version = 1, op = "host_result", run_id = request.RunId,
                    call_id = "fixture-call", success = true, value = new { } })), "host_result delivered");
            }
            await Until(() => events.Any(e => e.Op == "exited"));
            using var complete = JsonDocument.Parse(events.Last(e => e.Op == "stdout").Data!);
            Assert(events.Last().Success && complete.RootElement.GetProperty("run_id").GetGuid() == request.RunId &&
                complete.RootElement.GetProperty("status").GetString() == "success" && complete.RootElement.GetProperty("value").GetProperty("text").GetString() == "fixture",
                "parent-readable version1 completion");
        }
    }
    private static void Dispatch(CredentialRequest request, ExtensionProcessRunner runner, CredentialDispatcher dispatcher,
        ConcurrentQueue<CredentialResult> results, ConcurrentQueue<ExtensionProcessEvent>? process = null) =>
        ExtensionHostBroker.DispatchControl(JsonSerializer.Serialize(request, ExtensionProcessProtocol.Json), runner, dispatcher,
            e => { if (process is null) throw new InvalidOperationException("unexpected process route"); process.Enqueue(e); }, results.Enqueue);

    public static async Task BrokerPipeFakeStoreRoundTrip()
    {
        using var broker = ExtensionProcessFixture.Start("--extension-fixture-broker", Environment.ProcessId.ToString(), Environment.ProcessPath!);
        try
        {
            var set = Request();
            var index = 0;
            foreach (var request in new[] { set, set with { Op = "credential_get", RequestId = Guid.NewGuid(), Value = null },
                set with { Op = "credential_delete", RequestId = Guid.NewGuid(), Value = null },
                set with { Op = "credential_get", RequestId = Guid.NewGuid(), Value = null } })
            {
                await broker.StandardInput.WriteLineAsync(JsonSerializer.Serialize(request, ExtensionProcessProtocol.Json));
                await broker.StandardInput.FlushAsync();
                var line = await broker.StandardOutput.ReadLineAsync().WaitAsync(TimeSpan.FromSeconds(5));
                var response = JsonSerializer.Deserialize<CredentialResult>(line!, ExtensionProcessProtocol.Json)!;
                Assert(response.Op == "credential_result" && response.RequestId == request.RequestId && response.Success, "authenticated broker fake store response");
                if (request.Op == "credential_get") Assert(response.Found == (index == 1) && response.Value == (index == 1 ? set.Value : null),
                    "actual broker get distinguishes presence and deletion without truncating pipe value");
                else Assert(response.Value is null, "mutation response must never echo secret");
                index++;
            }
            broker.StandardInput.Close(); await broker.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(5));
        }
        finally { if (!broker.HasExited) { broker.Kill(entireProcessTree: true); broker.WaitForExit(5000); } }
    }
    private sealed class FixtureLog : IPlatformLog
    {
        internal readonly ConcurrentQueue<string> Entries = new();
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null) =>
            Entries.Enqueue(eventName + correlationId + JsonSerializer.Serialize(fields));
    }
}

internal sealed class CredentialFixtureStore : ICredentialStore
{
    internal readonly Dictionary<string, string> Values = new(StringComparer.Ordinal);
    internal int Mutations;
    internal int? FailMutation;
    internal Action? BeforeRead;
    public string? Read(string target) { BeforeRead?.Invoke(); return Values.GetValueOrDefault(target); }
    public void Write(string target, string value)
    {
        Values[target] = value;
        if (++Mutations == FailMutation) throw new InvalidOperationException("synthetic-secret native raw message");
    }
    public void Delete(string target)
    {
        Values.Remove(target);
        if (++Mutations == FailMutation) throw new InvalidOperationException("synthetic-secret native raw message");
    }
}
