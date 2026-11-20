using System.Diagnostics;
using System.Reflection;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

internal static class InstalledApplicationTests
{
    private const string Calculator = @"shell:AppsFolder\Microsoft.WindowsCalculator_8wekyb3d8bbwe!App";

    internal static Task OrdinaryShellDispatch()
    {
        foreach (var target in new[] { Calculator, @"shell:AppsFolder\Microsoft.Paint_8wekyb3d8bbwe!App",
            @"shell:AppsFolder\Future.Vendor_abc123!New.App", "calc.exe", "mspaint.exe", "notepad.exe", "ms-settings:" })
        foreach (var action in target.StartsWith("shell:", StringComparison.Ordinal) ? new[] { "open", "system_tool" } : new[] { "system_tool" })
        {
            Check(ShellTarget.TryCreate(target, action, out var shell, out var error), $"ordinary application target rejected: {error}");
            ProcessStartInfo? observed = null;
            var result = shell.LaunchCaptured(_ => { }, info => { observed = info; return null; });
            Check(result.Success && observed is { UseShellExecute: true } && observed.FileName == target &&
                observed.Arguments.Length == 0 && observed.ArgumentList.Count == 0 && observed.Verb != "runas",
                "ordinary Shell boundary must pass exactly one target and no arguments or elevation");
        }
        return Task.CompletedTask;
    }

    internal static Task RejectUnsafeShellActions()
    {
        foreach (var action in new[] { "run_elevated", "select_in_explorer", "open_in_explorer", "open_with", "paste_files", "registry_key" })
            Check(!ShellTarget.TryCreate(Calculator, action, out _, out _), "AppsFolder is never a filesystem/elevated target");
        foreach (var target in new[] { @"shell:Startup", @"shell:AppsFolder", @"shell:AppsFolder\", @"shell:AppsFolder\Vendor_family",
            @"shell:AppsFolder\Vendor_family!", @"shell:AppsFolder\Vendor_family!App --flag", "shell:AppsFolder\\Vendor_family!App\"",
            @"shell:AppsFolder\Vendor_family!App\other", @"shell:AppsFolder\Vendor_family!App/other", @"shell:AppsFolder\Vendor_family!A!B",
            "shell:AppsFolder\\Vendor_family!A\0", @"shell:AppsFolder\Vendor_family!App?arg", @"shell:AppsFolder\Vendor_family!App:arg" })
        foreach (var action in new[] { "open", "system_tool" })
            Check(!ShellTarget.TryCreate(target, action, out _, out _), "only exact packaged application targets may dispatch");
        return Task.CompletedTask;
    }

    internal static async Task DetachedRouting()
    {
        using var entered = new ManualResetEventSlim();
        using var release = new ManualResetEventSlim();
        var dispatcher = new PlatformCommandSequenceDispatcher();
        try
        {
            var started = Stopwatch.GetTimestamp();
            var admission = dispatcher.DispatchAsync("query_installed_applications", () =>
            {
                entered.Set();
                release.Wait(TimeSpan.FromSeconds(3));
                return Task.CompletedTask;
            });
            Check(admission.IsCompletedSuccessfully && entered.Wait(TimeSpan.FromSeconds(2)) && !release.IsSet,
                "installed application enumeration must detach from serial reader");
            Check(Stopwatch.GetElapsedTime(started) < TimeSpan.FromSeconds(1), "reader admission must return before the blocking delegate finishes");
            var later = false;
            await dispatcher.DispatchAsync("later", () => { later = true; return Task.CompletedTask; });
            Check(later && !release.IsSet, "later commands must not wait for COM");
        }
        finally { release.Set(); await dispatcher.DrainAsync(TimeSpan.FromSeconds(5)); }
    }

    internal static Task LocalizedApplicationsAndJson()
    {
        var facts = new[]
        {
            new InstalledApplicationFacts("Microsoft.WindowsCalculator_8wekyb3d8bbwe", "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App", "计算器"),
            new InstalledApplicationFacts("Microsoft.Paint_8wekyb3d8bbwe", "Microsoft.Paint_8wekyb3d8bbwe!App", "画图"),
            new InstalledApplicationFacts("Microsoft.WindowsCamera_8wekyb3d8bbwe", "Microsoft.WindowsCamera_8wekyb3d8bbwe!App", "相机"),
            new InstalledApplicationFacts("Future.Vendor_abc123", "Future.Vendor_abc123!New.App", "未来应用 Ω")
        };
        var log = new RecordingLog();
        var correlation = Guid.NewGuid();
        var result = new InstalledApplicationQuery(token =>
        {
            Check(Thread.CurrentThread.GetApartmentState() == ApartmentState.STA, "enumeration owns STA");
            return facts;
        }).ExecuteBounded(correlation, log, CancellationToken.None);
        Check(result.Success && result.Error is null && result.Data is not null, "complete enumeration succeeds");
        using var document = JsonDocument.Parse(result.Data!);
        Check(document.RootElement.EnumerateObject().Count() == 1, "result has only applications");
        var applications = document.RootElement.GetProperty("applications").EnumerateArray().ToArray();
        Check(applications.Length == facts.Length, "calculator, paint, other and future packages require no allowlist");
        for (var i = 0; i < facts.Length; i++)
        {
            Check(applications[i].EnumerateObject().Count() == 2 && applications[i].GetProperty("name").GetString() == facts[i].Name &&
                applications[i].GetProperty("target").GetString() == @"shell:AppsFolder\" + facts[i].AppUserModelId,
                "localized native name and exact AppsFolder target survive JSON");
        }
        Check(log.Events.Select(e => e.Name).SequenceEqual(new[] { "installed_applications.accepted", "installed_applications.started", "installed_applications.completed" }) &&
            log.Events.All(e => e.Correlation == correlation && e.Fields.ContainsKey("elapsed_ms")), "correlated finite lifecycle");
        Check(!JsonSerializer.Serialize(log.Events).Contains("WindowsCalculator", StringComparison.Ordinal) &&
            !JsonSerializer.Serialize(log.Events).Contains("未来", StringComparison.Ordinal), "diagnostics exclude application identity");
        return Task.CompletedTask;
    }

    internal static Task InvalidFactsAndEmptyResult()
    {
        var valid = new InstalledApplicationFacts("Vendor_family", "Vendor_family!App", "本地应用");
        InstalledApplicationFacts[] facts = [valid, valid, new(null, "Vendor_family!App", "desktop"), new("", "Vendor_family!App", "desktop"),
            new("Vendor_family", null, "missing"), new("Vendor_family", "", "missing"), new("Vendor_family", "Vendor_family!App", null),
            new("Vendor_family", "Vendor_family!App", "  "), new("Vendor_family", "Vendor_family!App --flag", "invalid"),
            new("Other_family", "Vendor_family!Other", "mismatch"), new("Vendor_family", "Vendor_family!App\0", "invalid")];
        var result = new InstalledApplicationQuery(_ => facts).ExecuteBounded(Guid.NewGuid(), NullPlatformLog.Instance, CancellationToken.None);
        using var document = JsonDocument.Parse(result.Data!);
        Check(result.Success && document.RootElement.GetProperty("applications").GetArrayLength() == 1, "nonpackaged, incomplete, unsafe and duplicate facts are not applications");
        var empty = new InstalledApplicationQuery(_ => []).ExecuteBounded(Guid.NewGuid(), new ThrowingLog(), CancellationToken.None);
        Check(empty.Success && empty.Data == "{\"applications\":[]}", "a complete empty enumeration and failing diagnostic sink remain valid");
        return Task.CompletedTask;
    }

    internal static Task FailureNeverPublishesPartial()
    {
        static IEnumerable<InstalledApplicationFacts> Failing(CancellationToken token)
        {
            yield return new("Vendor_family", "Vendor_family!App", "private name");
            throw new COMException("private AUMID", unchecked((int)0x80070005));
        }
        var log = new RecordingLog();
        var result = new InstalledApplicationQuery(Failing).ExecuteBounded(Guid.NewGuid(), log, CancellationToken.None);
        Check(!result.Success && result.Data is null && result.NativeErrorCode == unchecked((int)0x80070005), "COM failure after a valid item must discard partial data and preserve HRESULT");
        var terminal = log.Events.Single(e => e.Name == "installed_applications.failed");
        Check((int?)terminal.Fields["native_error_code"] == result.NativeErrorCode &&
            (string?)terminal.Fields["failure_kind"] == "installed_applications_failed" &&
            !JsonSerializer.Serialize(log.Events).Contains("private", StringComparison.Ordinal), "terminal failure kind and native error without raw exception payload");
        var oversized = new InstalledApplicationQuery(_ => Enumerable.Range(0, 4097).Select(i =>
            new InstalledApplicationFacts("Vendor_family", "Vendor_family!App" + i, "app")))
            .ExecuteBounded(Guid.NewGuid(), NullPlatformLog.Instance, CancellationToken.None);
        Check(!oversized.Success && oversized.Data is null, "enumeration budget exhaustion is failure, not truncated success");
        var largeFrame = new InstalledApplicationQuery(_ => Enumerable.Range(0, 300).Select(i =>
            new InstalledApplicationFacts("Vendor_family", "Vendor_family!App" + i, new string('中', 1000))))
            .ExecuteBounded(Guid.NewGuid(), NullPlatformLog.Instance, CancellationToken.None);
        Check(!largeFrame.Success && largeFrame.Data is null, "escaped outer result must fit the original pipe frame");
        return Task.CompletedTask;
    }

    internal static async Task CancellationAndRetainedAdmission()
    {
        using var entered = new ManualResetEventSlim();
        using var release = new ManualResetEventSlim();
        using var exited = new ManualResetEventSlim();
        var calls = 0;
        var query = new InstalledApplicationQuery(_ =>
        {
            Interlocked.Increment(ref calls); entered.Set();
            try { release.Wait(TimeSpan.FromSeconds(5)); return []; }
            finally { exited.Set(); }
        }, TimeSpan.FromMilliseconds(100));
        try
        {
            var pending = Task.Run(() => query.ExecuteBounded(Guid.NewGuid(), NullPlatformLog.Instance, CancellationToken.None));
            Check(entered.Wait(TimeSpan.FromSeconds(2)), "worker entered");
            var timeout = await pending;
            Check(!timeout.Success && timeout.Data is null, "logical timeout returns no partial data");
            var overlap = query.ExecuteBounded(Guid.NewGuid(), NullPlatformLog.Instance, CancellationToken.None);
            Check(!overlap.Success && calls == 1, "timed-out uncooperative COM retains singleflight admission without replacement threads");
        }
        finally { release.Set(); Check(exited.Wait(TimeSpan.FromSeconds(2)), "fixture worker exits"); }
        var cleanupProbe = new InstalledApplicationQuery(_ => []);
        Check(SpinWait.SpinUntil(() => cleanupProbe.ExecuteBounded(Guid.NewGuid(), NullPlatformLog.Instance, CancellationToken.None).Success,
            TimeSpan.FromSeconds(2)), "actual cleanup releases the original STA slot");
        using var cancel = new CancellationTokenSource(); cancel.Cancel();
        var preCancelled = new InstalledApplicationQuery(_ => throw new InvalidOperationException("must not enumerate"))
            .ExecuteBounded(Guid.NewGuid(), NullPlatformLog.Instance, cancel.Token);
        Check(!preCancelled.Success && preCancelled.Data is null, "cancelled command never enumerates");
    }

    internal static async Task ExactPipeCommandAndResult()
    {
        var adapter = (WindowsPlatformAdapter)RuntimeHelpers.GetUninitializedObject(typeof(WindowsPlatformAdapter));
        var calls = 0;
        var query = new InstalledApplicationQuery(_ => { calls++; return [new("Future_family", "Future_family!App", "未来应用")]; });
        typeof(WindowsPlatformAdapter).GetField("_installedApplications", BindingFlags.Instance | BindingFlags.NonPublic)!.SetValue(adapter, query);
        typeof(WindowsPlatformAdapter).GetField("_log", BindingFlags.Instance | BindingFlags.NonPublic)!.SetValue(adapter, NullPlatformLog.Instance);
        var client = new PlatformPipeClient("fixture-unused", adapter, NullPlatformLog.Instance);
        var execute = typeof(PlatformPipeClient).GetMethod("ExecutePlatformCommandAsync", BindingFlags.Instance | BindingFlags.NonPublic)!;
        var request = ProtocolEnvelope.Create("platform_command", new { command = "query_installed_applications" }, Guid.NewGuid());
        var result = await (Task<WindowsCommandResult>)execute.Invoke(client, [request, "query_installed_applications", request.CorrelationId!.Value, CancellationToken.None, 0u])!;
        Check(result.Success && calls == 1, "exact parameterless pipe command calls adapter query without parser payload");
        var reply = PlatformPipeClient.CreateCommandResultEnvelope(request, result);
        Check(reply.Type == "platform_command_result" && reply.CorrelationId == request.Id && reply.Payload.GetProperty("success").GetBoolean() &&
            reply.Payload.GetProperty("data").ValueKind == JsonValueKind.String, "original correlated result carries a JSON string, not an envelope schema change");
        using var data = JsonDocument.Parse(reply.Payload.GetProperty("data").GetString()!);
        Check(data.RootElement.GetProperty("applications")[0].GetProperty("name").GetString() == "未来应用", "result projects complete applications JSON");
        var unknown = await (Task<WindowsCommandResult>)execute.Invoke(client, [request, "query_installed_application", Guid.NewGuid(), CancellationToken.None, 0u])!;
        Check(!unknown.Success && calls == 1, "wire command spelling is exact");
    }

    private sealed record DiagnosticEvent(string Name, Guid? Correlation, IReadOnlyDictionary<string, object?> Fields);

    private sealed class RecordingLog : IPlatformLog
    {
        public readonly List<DiagnosticEvent> Events = [];
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null) =>
            Events.Add(new(eventName, correlationId, fields ?? new Dictionary<string, object?>()));
    }

    private sealed class ThrowingLog : IPlatformLog
    {
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null) => throw new IOException("fixture sink");
    }

    private static void Check(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
