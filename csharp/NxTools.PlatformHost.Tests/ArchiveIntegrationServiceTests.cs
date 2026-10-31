using System.Text.Json;
using System.Security.Cryptography;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;
using NxTools.PlatformHost.Windows.Archive;
using static NxTools.PlatformHost.Tests.ArchiveIntegrationPlanTests;

namespace NxTools.PlatformHost.Tests;

// Reviewed effects: fake in-memory store, in-memory or GUID-owned temporary ledger files only.
// Product composition receives only fake effects; native Registry/store/default ledger are never used.
internal static class ArchiveIntegrationServiceTests
{
    private const string Executable = @"C:\NxTools\NxTools.exe";

    internal static Task ProductCompositionAppliesOnlyExplicitRequests()
    {
        var store = new FakeArchiveIntegrationStore();
        var ledger = new MemoryLedger();
        var notified = 0;
        var approvals = 0;
        var service = ArchiveIntegrationService.CreateProduct(NullPlatformLog.Instance, store, ledger,
            () => Executable, () => notified++, _ => { approvals++; return new ApprovalLease(); });
        var status = service.Execute(Command("status", 3, true, true, ".zip"), Guid.NewGuid());
        Check(status.State == "not_applied" && status.AppliedRevision is null && store.OperationCount == 0 && approvals == 0,
            "composition/status/saved desire do not apply anything");
        var applied = service.ExecuteBounded(Command("apply", 3, true, true, ".zip"), Guid.NewGuid());
        Check(applied.State == "applied" && applied.AppliedRevision == 3 && ledger.Saves == 1 && notified == 1 && approvals == 1,
            "explicit product Apply must reach the original transaction, not permanent disabled/apply_not_authorized");
        Check(ledger.Facts!.Values.Any(value => value.Target.ValueName == "DelegateExecute"), "product uses approved grouped helper plan");
        var newer = service.Execute(Command("status", 4, true, true, ".zip"), Guid.NewGuid());
        Check(newer.DesiredRevision == 4 && newer.AppliedRevision == 3, "desired/observed revisions remain separate");
        var reverted = service.ExecuteBounded(Command("revert", 4, false, false), Guid.NewGuid());
        Check(reverted.State == "reverted" && ledger.Facts is null && store.SnapshotValues().Count == 0 && notified == 2,
            "explicit product Revert uses original exact owned transaction");
        return Task.CompletedTask;
    }

    internal static async Task ProductDispatchDoesNotBlockReader()
    {
        using var entered = new ManualResetEventSlim();
        using var release = new ManualResetEventSlim();
        var store = new BlockingStore(entered, release);
        var service = ArchiveIntegrationService.CreateProduct(NullPlatformLog.Instance, store, new MemoryLedger(),
            () => Executable, () => { }, _ => new ApprovalLease());
        var dispatcher = new PlatformCommandSequenceDispatcher();
        var later = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var reader = Task.Run(async () =>
        {
            await dispatcher.DispatchAsync("archive_shell_integration", () =>
            {
                service.ExecuteBounded(Command("apply", 1, false, true, ".zip"), Guid.NewGuid());
                return Task.CompletedTask;
            });
            await dispatcher.DispatchAsync("later", () => { later.SetResult(); return Task.CompletedTask; });
        });
        try
        {
            Check(entered.Wait(TimeSpan.FromSeconds(2)), "product route reaches fixture native work");
            await Task.WhenAny(later.Task, Task.Delay(500));
            Check(later.Task.IsCompletedSuccessfully && store.Apartment == ApartmentState.STA,
                "native integration uses bounded STA without blocking later serial reader commands");
        }
        finally
        {
            release.Set();
            await reader.WaitAsync(TimeSpan.FromSeconds(3));
            await dispatcher.DrainAsync(TimeSpan.FromSeconds(3));
        }
    }

    internal static Task ProductIdentityLeaseAcceptsExactReleaseAndRejectsUnknown()
    {
        var root = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")
            ?? throw new InvalidOperationException("Explicit session scratch is required."), "nxtools-integration-identity-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(root);
        try
        {
            var desktop = Path.Combine(root, "NxTools.exe");
            var platform = Path.Combine(root, "NxTools.PlatformHost.exe");
            File.WriteAllText(desktop, "synthetic current desktop");
            File.WriteAllText(platform, "synthetic current host");
            var desktopHash = Hash(desktop);
            var hashes = new Dictionary<string, string>();
            foreach (var member in TrustedReleaseManifestTests.ArchiveMembers)
            {
                File.WriteAllText(Path.Combine(root, member), "synthetic current " + member);
                hashes[member] = Hash(Path.Combine(root, member));
            }
            string? Resolve(string expected) => WindowsPlatformAdapter.ResolveApprovedDesktopExecutablePath(
                desktop, Path.Combine(root, "missing-protected", "NxTools.exe"), platform, expected, _ => false);
            var store = new FakeArchiveIntegrationStore();
            var ledger = new MemoryLedger();
            var service = ArchiveIntegrationService.CreateProduct(NullPlatformLog.Instance, store, ledger,
                () => Resolve(desktopHash), () => { }, _ => IndexerServiceInstaller.OpenArchiveRelease(root, hashes));
            Check(service.ExecuteBounded(Command("apply", 8, true, true, ".zip"), Guid.NewGuid()).State == "applied",
                "exact current release identity is usable through actual product policy");
            Check(service.ExecuteBounded(Command("revert", 8, false, false), Guid.NewGuid()).State == "reverted", "fixture release reverted");
            foreach (var expected in new[] { "", new string('0', 64) })
            {
                var deniedStore = new FakeArchiveIntegrationStore();
                var denied = ArchiveIntegrationService.CreateProduct(NullPlatformLog.Instance, deniedStore, new MemoryLedger(),
                    () => Resolve(expected), () => { }, _ => IndexerServiceInstaller.OpenArchiveRelease(root, hashes))
                    .ExecuteBounded(Command("apply", 9, true, false, ".zip"), Guid.NewGuid());
                Check(denied.Failure == "desktop_identity_unavailable" && deniedStore.OperationCount == 0,
                    "unpackaged/unknown desktop identity refuses before Registry capture");
            }
            var helper = Path.Combine(root, ArchiveSelectionHandlerPlan.HelperExecutableName);
            File.WriteAllText(helper, "tampered helper");
            var before = store.OperationCount;
            Check(service.ExecuteBounded(Command("apply", 9, true, false, ".zip"), Guid.NewGuid()).Failure == "selection_helper_identity_unavailable"
                && store.OperationCount == before && ledger.Facts is null, "tampered helper never falls back to per-file registration");
            File.Delete(helper);
            Check(service.ExecuteBounded(Command("apply", 9, true, false, ".zip"), Guid.NewGuid()).Failure == "selection_helper_identity_unavailable"
                && store.OperationCount == before, "missing helper refuses before capture");
            var nullApproval = ArchiveIntegrationService.CreateProduct(NullPlatformLog.Instance, store, ledger,
                () => Resolve(desktopHash), () => { }, _ => null!);
            Check(nullApproval.ExecuteBounded(Command("apply", 9, true, false, ".zip"), Guid.NewGuid()).Failure == "selection_helper_identity_unavailable"
                && store.OperationCount == before, "null identity approval remains fail-closed");
        }
        finally { Directory.Delete(root, true); }
        return Task.CompletedTask;
    }

    internal static async Task ProductCancelTimeoutRetainsBoundedSlotAndNoAppliedReply()
    {
        using var entered = new ManualResetEventSlim();
        using var release = new ManualResetEventSlim();
        using var cancellation = new CancellationTokenSource();
        var store = new BlockingStore(entered, release);
        var ledger = new MemoryLedger();
        var log = new RecordingLog();
        var service = ArchiveIntegrationService.CreateProduct(log, store, ledger,
            () => Executable, () => { }, _ => new ApprovalLease());
        var correlation = Guid.NewGuid();
        var operation = Task.Run(() => service.ExecuteBounded(Command("apply", 12, false, true, ".zip"), correlation,
            cancellation.Token, TimeSpan.FromSeconds(2)));
        try
        {
            Check(entered.Wait(TimeSpan.FromSeconds(2)), "isolated capture entered on native owner");
            cancellation.Cancel();
            var cancelled = await operation.WaitAsync(TimeSpan.FromSeconds(2));
            Check(cancelled.State == "failed" && cancelled.Failure == "cancelled" && cancelled.AppliedRevision is null,
                "cancel/close cannot be relabeled applied");
            var blocked = service.ExecuteBounded(Command("apply", 13, false, true, ".zip"), correlation, timeout: TimeSpan.FromMilliseconds(50));
            Check(blocked.State == "failed" && blocked.Failure == "native_operation_unconfirmed" && store.Writes == 0,
                "cancelled-but-still-running native work retains the only STA slot");
        }
        finally { release.Set(); await operation.WaitAsync(TimeSpan.FromSeconds(3)); }
        Check(SpinWait.SpinUntil(() => log.HasTerminal(correlation), TimeSpan.FromSeconds(3)), "actual native cleanup/terminal observed");
        Check(store.Writes == 0 && ledger.Facts is null, "cancel during pre-capture makes no writes or applied ledger");
        Check(log.Entries.All(entry => entry.Correlation == correlation && entry.Fields.Values.All(value =>
            value is not string text || !text.Contains("C:\\", StringComparison.Ordinal))), "correlated payload-free native diagnostics");

        // Real bounded timeout, not just the occupied-slot path. Source is still only a memory store.
        using var timedEntered = new ManualResetEventSlim();
        using var timedRelease = new ManualResetEventSlim();
        var timedStore = new BlockingStore(timedEntered, timedRelease);
        var timedLedger = new MemoryLedger();
        var timedCorrelation = Guid.NewGuid();
        var timedService = ArchiveIntegrationService.CreateProduct(log, timedStore, timedLedger,
            () => Executable, () => { }, _ => new ApprovalLease());
        var timedOperation = Task.Run(() => timedService.ExecuteBounded(Command("apply", 14, false, true, ".zip"),
            timedCorrelation, timeout: TimeSpan.FromMilliseconds(300)));
        try
        {
            Check(timedEntered.Wait(TimeSpan.FromSeconds(2)), "timed fixture work entered");
            var timed = await timedOperation.WaitAsync(TimeSpan.FromSeconds(2));
            Check(timed.State == "failed" && timed.Failure == "native_operation_unconfirmed" && timed.AppliedRevision is null,
                "actual timeout does not report applied or assert Windows was unchanged");
        }
        finally { timedRelease.Set(); await timedOperation.WaitAsync(TimeSpan.FromSeconds(3)); }
        Check(SpinWait.SpinUntil(() => log.HasTerminal(timedCorrelation), TimeSpan.FromSeconds(3)) &&
            timedStore.Writes == 0 && timedLedger.Facts is null, "timed worker drains without writes after capture returns");

        using var preCancelled = new CancellationTokenSource();
        preCancelled.Cancel();
        var untouched = new FakeArchiveIntegrationStore();
        var preService = ArchiveIntegrationService.CreateProduct(log, untouched, new MemoryLedger(),
            () => Executable, () => { }, _ => new ApprovalLease());
        Check(preService.Execute(Command("apply", 15, true, true, ".zip"), Guid.NewGuid(), preCancelled.Token).Failure == "cancelled" &&
            untouched.OperationCount == 0, "pre-cancel/closed request has zero native effects");

        using var duringWrite = new CancellationTokenSource();
        var cancellingStore = new CancelOnWriteStore(duringWrite);
        var cancellingLedger = new MemoryLedger();
        var cancellingService = ArchiveIntegrationService.CreateProduct(log, cancellingStore, cancellingLedger,
            () => Executable, () => { }, _ => new ApprovalLease());
        var compensated = cancellingService.Execute(Command("apply", 16, false, true, ".zip"), Guid.NewGuid(), duringWrite.Token);
        Check(compensated.State == "failed" && compensated.AppliedRevision is null && cancellingLedger.Facts is null &&
            cancellingStore.Inner.RestoreAttempts == 1 && cancellingStore.Inner.SnapshotValues().Count == 0,
            "cancellation after one mutation uses original attempt-before-write reverse compensation");
    }

    private static string Hash(string path) => Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path)));

    internal static Task EnabledApplyRecordsFactsAndRevertsExactly()
    {
        var store = new FakeArchiveIntegrationStore();
        var ledger = new MemoryLedger();
        var notified = 0;
        var service = Enabled(store, ledger, () => notified++);

        var applied = service.Execute(Command("apply", 21, true, true, ".zip", ".7z"), Guid.NewGuid());
        Check(applied.State == ArchiveShellIntegrationStates.Applied && applied.AppliedRevision == 21 && applied.AppliedValues > 0,
            "explicit apply reports observed facts and revision");
        Check(ledger.Facts?.DesiredRevision == 21 && ledger.Facts.Values.Count == applied.AppliedValues, "facts persisted before success");
        Check(notified == 1, "association refresh hint after observed apply");

        var again = service.Execute(Command("apply", 22, true, false, ".zip"), Guid.NewGuid());
        Check(again.State == ArchiveShellIntegrationStates.RevertRequired && again.AppliedRevision == 21,
            "a changed desired revision cannot overwrite unreverted applied facts");

        var modified = ledger.Facts!.Values.First(fact => fact.Target.KeyPath.EndsWith(@"\05test\command", StringComparison.OrdinalIgnoreCase)).Target;
        store.SeedValue(modified, ArchiveRegistryValue.String("user edit"));
        var foreign = new ArchiveRegistryValueTarget(@"Software\Classes\.zip\OpenWithProgids", "Other.App", @"Software\Classes\.zip\OpenWithProgids");
        store.SeedValue(foreign, ArchiveRegistryValue.String(""));

        var reverted = service.Execute(Command("revert", 22, false, false), Guid.NewGuid());
        Check(reverted.State == ArchiveShellIntegrationStates.Reverted && reverted.Preserved == 1
            && reverted.Removed == applied.AppliedValues - 1, "revert removes exact owned values and reports preserved modification");
        Check(store.Value(modified)?.ContentEquals(ArchiveRegistryValue.String("user edit")) == true, "user-modified value preserved");
        Check(store.Value(foreign) is not null, "another application's OpenWithProgids value preserved");
        Check(ledger.Facts is null && ledger.Deletes == 1, "applied facts retired only after exact revert");
        Check(service.Execute(Command("status", 22, false, false), Guid.NewGuid()).State == ArchiveShellIntegrationStates.NotApplied,
            "observed status returns to not applied");
        return Task.CompletedTask;
    }

    internal static Task ApplyFailuresCompensateAndNeverReportApplied()
    {
        var plan = ArchiveIntegrationPlan.Create(Executable, 5, new ArchiveIntegrationIntent(true, true, [".zip"]));
        for (var failAt = 1; failAt <= plan.Changes.Count; failAt += Math.Max(1, plan.Changes.Count / 4))
        {
            var store = new FakeArchiveIntegrationStore { FailWriteAt = failAt };
            var ledger = new MemoryLedger();
            var before = store.SnapshotValues();
            var result = Enabled(store, ledger, () => { }).Execute(Command("apply", 5, true, true, ".zip"), Guid.NewGuid());
            Check(result.State == ArchiveShellIntegrationStates.Failed && result.Failure == "write_failed_compensated",
                $"write-then-throw at {failAt} is a compensated failure");
            Check(store.ValuesEqual(before) && ledger.Facts is null && ledger.Saves == 0, "no facts and exact compensation");
        }

        var ledgerFailStore = new FakeArchiveIntegrationStore();
        var failingLedger = new MemoryLedger { FailSave = true };
        var ledgerFailure = Enabled(ledgerFailStore, failingLedger, () => { })
            .Execute(Command("apply", 6, true, true, ".zip"), Guid.NewGuid());
        Check(ledgerFailure.State == ArchiveShellIntegrationStates.Failed && ledgerFailure.Failure == "ledger_failed_compensated",
            "unpersisted facts cannot be reported applied");
        Check(ledgerFailStore.SnapshotValues().Count == 0, "values compensated when ownership cannot be persisted");

        var noIdentity = new ArchiveIntegrationService(new FakeArchiveIntegrationStore(), new MemoryLedger(), NullPlatformLog.Instance,
            () => null, () => { }, applyEnabled: true).Execute(Command("apply", 7, true, false, ".zip"), Guid.NewGuid());
        Check(noIdentity.Failure == "desktop_identity_unavailable", "no approved desktop identity, no plan");
        var invalid = Enabled(new FakeArchiveIntegrationStore(), new MemoryLedger(), () => { })
            .Execute(Command("apply", 8, true, false, "z ip"), Guid.NewGuid());
        Check(invalid.Failure == "invalid_intent", "invalid extension rejected before capture");
        var version = Enabled(new FakeArchiveIntegrationStore(), new MemoryLedger(), () => { })
            .Execute(Command("apply", 8, true, false, ".zip") with { Version = 2 }, Guid.NewGuid());
        Check(version.Failure == "unsupported_version", "unknown wire version rejected");
        return Task.CompletedTask;
    }

    // F-AC03: after a ledger failure every attempted value is restored in reverse even if one Restore fails.
    internal static Task LedgerFailureCompensatesEveryValueDespiteRestoreFailure()
    {
        var plan = ArchiveIntegrationPlan.Create(Executable, 41, new ArchiveIntegrationIntent(true, true, [".zip"]));
        Check(plan.Changes.Count == 17, $"probe-equivalent plan has 17 values ({plan.Changes.Count})");
        var failing = plan.Changes[plan.Changes.Count - 1].Target; // first one restored (reverse order)
        var store = new FakeArchiveIntegrationStore { FailRestoreIdentity = failing.Identity };
        var result = Enabled(store, new MemoryLedger { FailSave = true }, () => { })
            .Execute(Command("apply", 41, true, true, ".zip"), Guid.NewGuid());
        Check(store.RestoreAttempts == plan.Changes.Count, $"all {plan.Changes.Count} values attempted ({store.RestoreAttempts})");
        Check(result.State == ArchiveShellIntegrationStates.Failed && result.Failure == "ledger_failed_compensation_incomplete"
            && result.AppliedRevision is null, "incomplete compensation is reported, never applied/compensated");
        var remaining = store.SnapshotValues();
        Check(remaining.Count == 1 && remaining.ContainsKey(failing.Identity), "only the failed value remains; recoverable values restored");

        var clean = new FakeArchiveIntegrationStore();
        var cleanResult = Enabled(clean, new MemoryLedger { FailSave = true }, () => { })
            .Execute(Command("apply", 42, true, true, ".zip"), Guid.NewGuid());
        Check(clean.RestoreAttempts == plan.Changes.Count && clean.SnapshotValues().Count == 0
            && cleanResult.Failure == "ledger_failed_compensated", "complete compensation unchanged");
        return Task.CompletedTask;
    }
    internal static Task LedgerRoundTripWireAndRegistryTargetGuards()
    {
        var directory = Path.Combine(Path.GetTempPath(), "nxtools-archive-ledger-" + Guid.NewGuid().ToString("N"));
        try
        {
            var ledger = new FileArchiveIntegrationLedger(Path.Combine(directory, "applied.json"));
            Check(ledger.Load() is null, "missing ledger means not applied");
            var store = new FakeArchiveIntegrationStore();
            var prior = ArchiveIntegrationPlan.Create(Executable, 9, new ArchiveIntegrationIntent(false, true, [".zip"])).Changes[0].Target;
            store.SeedValue(prior, ArchiveRegistryValue.ExpandString("%LOCALAPPDATA%\\prior"));
            var facts = ArchiveIntegrationTransaction.Apply(store,
                ArchiveIntegrationPlan.Create(Executable, 9, new ArchiveIntegrationIntent(false, true, [".zip"])));
            ledger.Save(facts);
            var loaded = ledger.Load() ?? throw new InvalidOperationException("ledger reload");
            Check(loaded.DesiredRevision == 9 && loaded.Values.Count == facts.Values.Count, "ledger roundtrip count/revision");
            for (var index = 0; index < facts.Values.Count; index++)
            {
                var expected = facts.Values[index];
                var actual = loaded.Values[index];
                Check(actual.Target == expected.Target && actual.AppliedValue.ContentEquals(expected.AppliedValue)
                    && actual.Prior.Exists == expected.Prior.Exists
                    && (actual.Prior.Value?.ContentEquals(expected.Prior.Value!) ?? expected.Prior.Value is null)
                    && actual.Prior.MissingOwnedKeyPaths.SequenceEqual(expected.Prior.MissingOwnedKeyPaths), "exact ledger fact");
            }
            Check(Directory.GetFiles(directory).Length == 1, "atomic ledger leaves no temporary file");
            File.WriteAllText(ledger.FilePath, "{\"version\":99,\"desired_revision\":1,\"values\":[]}");
            Expect<InvalidDataException>(() => ledger.Load());
            var unreadable = new ArchiveIntegrationService(new FakeArchiveIntegrationStore(), ledger, NullPlatformLog.Instance,
                () => Executable, () => { }, applyEnabled: true).Execute(Command("revert", 1, false, false), Guid.NewGuid());
            Check(unreadable.Failure == "ledger_unreadable", "corrupt ledger is explicit, never a silent revert");
            ledger.Delete();
            Check(ledger.Load() is null, "delete retires ledger");
        }
        finally
        {
            if (Directory.Exists(directory)) Directory.Delete(directory, true);
        }

        var envelope = ProtocolEnvelope.Create("platform_command", new
        {
            command = "archive_shell_integration",
            data = new { version = 1, action = "apply", desired_revision = 4, classic_cascade = true, open_with = false, extensions = new[] { ".zip" } },
        });
        var parsed = PlatformCommandParser.ParseArchiveShellIntegration(envelope);
        Check(parsed is { Version: 1, Action: "apply", DesiredRevision: 4, ClassicCascade: true, OpenWith: false }
            && parsed.Extensions!.SequenceEqual([".zip"]), "snake_case wire parsed exactly");
        var wire = ArchiveIntegrationService.Serialize(new ArchiveShellIntegrationResult(1, "applied", 4, 4, 3, 0, 0, 0, null));
        using (var json = JsonDocument.Parse(wire))
            Check(json.RootElement.GetProperty("state").GetString() == "applied"
                && json.RootElement.GetProperty("applied_revision").GetInt64() == 4, "observed result wire");

        foreach (var escaped in new[]
        {
            new ArchiveRegistryValueTarget(@"Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.zip\UserChoice", "ProgId", @"Software\Microsoft"),
            new ArchiveRegistryValueTarget(@"Software\Classes\.zip\UserChoice", "ProgId", @"Software\Classes\.zip"),
            new ArchiveRegistryValueTarget(@"Software\Classes\NxTools.Archive", "", @"Software\Classes\Other"),
            new ArchiveRegistryValueTarget(@"Software\Classes\\NxTools.Archive", "", @"Software\Classes\"),
        })
            Expect<InvalidOperationException>(() => WindowsArchiveIntegrationStore.Validate(escaped));
        foreach (var change in ArchiveIntegrationPlan.Create(Executable, 1, new ArchiveIntegrationIntent(true, true, [".zip", ".rar"])).Changes)
            WindowsArchiveIntegrationStore.Validate(change.Target);
        return Task.CompletedTask;
    }

    private static ArchiveIntegrationService Enabled(IArchiveIntegrationStore store, IArchiveIntegrationLedger ledger, Action notify) =>
        new(store, ledger, NullPlatformLog.Instance, () => Executable, notify, applyEnabled: true);

    private static ArchiveShellIntegrationCommand Command(string action, long revision, bool cascade, bool openWith, params string[] extensions) =>
        new(ArchiveIntegrationService.WireVersion, action, revision, cascade, openWith, extensions);

    private static void Check(bool value, string detail)
    {
        if (!value) throw new InvalidOperationException(detail);
    }

    private static void Expect<T>(Action action) where T : Exception
    {
        try { action(); }
        catch (T) { return; }
        throw new InvalidOperationException($"Expected {typeof(T).Name}");
    }

    private sealed class ApprovalLease : IDisposable { public void Dispose() { } }
    private sealed class BlockingStore(ManualResetEventSlim entered, ManualResetEventSlim release) : IArchiveIntegrationStore
    {
        private readonly FakeArchiveIntegrationStore _inner = new();
        internal ApartmentState Apartment;
        internal int Writes;
        public ArchiveRegistryValueSnapshot Capture(ArchiveRegistryValueTarget target)
        { Apartment = Thread.CurrentThread.GetApartmentState(); entered.Set(); release.Wait(TimeSpan.FromSeconds(5)); return _inner.Capture(target); }
        public void Write(ArchiveRegistryValueTarget target, ArchiveRegistryValue value) { Writes++; _inner.Write(target, value); }
        public void Restore(ArchiveRegistryValueTarget target, ArchiveRegistryValueSnapshot snapshot) => _inner.Restore(target, snapshot);
    }

    private sealed class CancelOnWriteStore(CancellationTokenSource cancellation) : IArchiveIntegrationStore
    {
        internal readonly FakeArchiveIntegrationStore Inner = new();
        public ArchiveRegistryValueSnapshot Capture(ArchiveRegistryValueTarget target) => Inner.Capture(target);
        public void Write(ArchiveRegistryValueTarget target, ArchiveRegistryValue value) { Inner.Write(target, value); cancellation.Cancel(); }
        public void Restore(ArchiveRegistryValueTarget target, ArchiveRegistryValueSnapshot snapshot) => Inner.Restore(target, snapshot);
    }

    private sealed class RecordingLog : IPlatformLog
    {
        internal readonly System.Collections.Concurrent.ConcurrentQueue<(string Event, Guid? Correlation, IReadOnlyDictionary<string, object?> Fields)> Entries = new();
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null) =>
            Entries.Enqueue((eventName, correlationId, fields!));
        internal bool HasTerminal(Guid correlation) => Entries.Any(entry => entry.Correlation == correlation && entry.Event == "archive.shell_integration.terminal");
    }

    private sealed class MemoryLedger : IArchiveIntegrationLedger
    {
        internal ArchiveIntegrationAppliedFacts? Facts { get; private set; }
        internal int Saves { get; private set; }
        internal int Deletes { get; private set; }
        internal bool FailSave { get; init; }
        public ArchiveIntegrationAppliedFacts? Load() => Facts;
        public void Save(ArchiveIntegrationAppliedFacts facts)
        {
            if (FailSave) throw new IOException("synthetic ledger failure");
            Saves++;
            Facts = facts;
        }
        public void Delete()
        {
            Deletes++;
            Facts = null;
        }
    }
}
