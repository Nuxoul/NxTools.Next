using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;
using NxTools.PlatformHost.Windows.Archive;
using NxTools.ShellSelectionHelper;
using static NxTools.PlatformHost.Tests.ArchiveIntegrationPlanTests;

namespace NxTools.PlatformHost.Tests;

// Fake-only: JSON, pure plans, memory stores/ledgers, managed Shell items and injected launches.
// No real settings launch, Registry, COM activation/registration, service or product process.
internal static class ArchiveAssociationSettingsTests
{
    private const string Executable = @"C:\NxTools\NxTools.exe";
    private const string Helper = @"C:\NxTools\NxTools.ShellSelectionHelper.exe";
    private const string Legacy = "\"version\":1,\"action\":\"apply\",\"desired_revision\":1,\"classic_cascade\":true,\"open_with\":false,\"extensions\":[\".zip\"]";

    internal static Task StrictWireRejectsInvalidConfiguration()
    {
        // This exact actual-parser assertion was run RED before implementation (old code ignored it).
        RejectArchive("\"menu_commands\":[\"test\",\"test\"]");
        foreach (var invalid in new[]
        {
            "\"menu_commands\":null", "\"menu_commands\":[null]", "\"menu_commands\":[\"unknown\"]",
            "\"menu_commands\":[\"open\",\"test\",\"extract_smart\",\"extract_here\",\"extract_name\",\"extract_dialog\",\"create\",\"open\"]",
            "\"double_click_action\":null", "\"double_click_action\":\"here\"",
            "\"menu_layout\":null", "\"menu_layout\":\"windows11\"", "\"unknown\":true",
        }) RejectArchive(invalid);
        var legacy = ParseArchive();
        Check(legacy.DoubleClickAction == "open" && legacy.MenuLayout == "cascade" &&
            legacy.MenuCommands.SequenceEqual(ArchiveIntegrationIntent.DefaultMenuCommands), "missing new fields preserve old five defaults");
        Check(ParseArchive("\"menu_commands\":[]").MenuCommands.Length == 0, "explicit empty remains empty");
        var all = ParseArchive("\"double_click_action\":\"name\",\"menu_layout\":\"flat\",\"menu_commands\":[\"open\",\"test\",\"extract_smart\",\"extract_here\",\"extract_name\",\"extract_dialog\",\"create\"]");
        Check(all.MenuCommands.Length == 7 && all.DoubleClickAction == "name" && all.MenuLayout == "flat", "all seven exact fields parsed");
        using (var wire = JsonDocument.Parse(JsonSerializer.Serialize(all, JsonDefaults.Options)))
            Check(wire.RootElement.GetProperty("double_click_action").GetString() == "name" &&
                wire.RootElement.GetProperty("menu_layout").GetString() == "flat" &&
                wire.RootElement.GetProperty("menu_commands").GetArrayLength() == 7 &&
                wire.RootElement.GetProperty("version").GetInt32() == 1, "exact snake_case version1 field alignment");
        return Task.CompletedTask;
    }

    internal static Task PlansConsumeActionsFilteringAndLayout()
    {
        foreach (var (action, arguments) in new[] { ("open", "--archive-open"), ("smart", "--archive-extract smart-dialog"), ("name", "--archive-extract name-dialog") })
        {
            var plan = ArchiveIntegrationPlan.Create(Executable, 1, new(false, true, [".zip"]) { DoubleClickAction = action });
            Check((string)plan.Changes.Single(change => change.Target.KeyPath.EndsWith(@"\shell\open\command")).Value.Data ==
                "\"" + Executable + "\" " + arguments + " \"%1\"", "double-click exact confirmation-form ingress: " + action);
        }
        foreach (var layout in new[] { "cascade", "flat" })
        {
            var intent = new ArchiveIntegrationIntent(true, false, [".zip", ".7z"]) { MenuLayout = layout, MenuCommands = ["test", "extract_dialog", "create"] };
            foreach (var grouped in new[] { false, true })
            {
                var plan = ArchiveIntegrationPlan.Create(Executable, 2, intent, grouped ? Helper : null);
                var leaves = plan.Changes.Where(change => change.Target.KeyPath.EndsWith(@"\command")).ToArray();
                Check(leaves.Length == 6, "two archive commands per extension and create only on * and Directory");
                Check(!plan.Changes.Any(change => change.Target.KeyPath.Contains("01open") || change.Target.KeyPath.Contains("02extract-smart")), "unselected commands absent");
                Check(plan.Changes.Any(change => change.Target.KeyPath.StartsWith(@"Software\Classes\*\shell\")) &&
                    plan.Changes.Any(change => change.Target.KeyPath.StartsWith(@"Software\Classes\Directory\shell\")), "create source menus both present");
                Check(plan.Changes.Any(change => change.Target.ValueName == "SubCommands") == (layout == "cascade"), "classic layout truly consumed");
                if (layout == "flat") Check(plan.Changes.All(change => !change.Target.CleanupRootPath.EndsWith(@"\shell") &&
                    (change.Target.KeyPath == change.Target.CleanupRootPath || change.Target.KeyPath.StartsWith(change.Target.CleanupRootPath + "\\"))), "flat cleanup owns only each exact verb, never shared shell");
                if (grouped) Check(leaves.All(change => change.Target.ValueName == "DelegateExecute" && !((string)change.Value.Data).Contains("%1")), "grouped uses only helper DelegateExecute");
                foreach (var change in plan.Changes) WindowsArchiveIntegrationStore.Validate(change.Target);
                Check(plan.AppliedFacts is null, "pure planning does not claim applied");
            }
        }
        var empty = new ArchiveIntegrationIntent(true, false, []) { MenuCommands = [] };
        Check(ArchiveIntegrationPlan.Create(Executable, 1, empty).Changes.Count == 0 &&
            ArchiveSelectionHandlerPlan.Create(Helper, 1, empty).Changes.Count == 0, "empty commands means no empty menu/CLSID writes");
        Check(ArchiveIntegrationPlan.Create(Executable, 1, empty with { MenuCommands = ["create"] }, Helper).Changes.Count == 10, "create-only needs no archive extension");
        var seven = new ArchiveIntegrationIntent(true, false, [".zip"]) { MenuCommands = ArchiveSelectionHandlerPlan.Commands.Select(command => command.Command).ToArray() };
        var allPlan = ArchiveSelectionHandlerPlan.Create(Helper, 1, seven);
        Check(allPlan.Changes.Count(change => change.Target.ValueName == "DelegateExecute") == 8 &&
            allPlan.Changes.Count(change => change.Target.KeyPath.EndsWith(@"\LocalServer32")) == 7, "all seven finite identities / six archive plus two source verbs");
        foreach (var command in ArchiveSelectionHandlerPlan.Commands)
        {
            var verb = command.Command switch
            {
                "open" => ShellVerb.Open, "test" => ShellVerb.Test, "extract_smart" => ShellVerb.ExtractSmart,
                "extract_here" => ShellVerb.ExtractHere, "extract_name" => ShellVerb.ExtractName,
                "extract_dialog" => ShellVerb.ExtractDialog, _ => ShellVerb.Compress,
            };
            Check(HelperIdentity.VerbClasses[verb] == command.Clsid, "stable helper identity " + command.Command);
        }
        return Task.CompletedTask;
    }

    internal static Task GroupedServiceCompensationAndGate()
    {
        var command = ParseArchive("\"double_click_action\":\"smart\",\"menu_layout\":\"flat\",\"menu_commands\":[\"test\",\"create\"]") with { OpenWith = true };
        var expected = ArchiveIntegrationPlan.Create(Executable, command.DesiredRevision, command.ToIntent(), Helper);
        var store = new FakeArchiveIntegrationStore();
        var ledger = new MemoryLedger();
        var lease = new FakeLease();
        var result = Service(store, ledger, true, _ => lease).Execute(command, Guid.NewGuid());
        Check(result.State == "applied" && ledger.Facts?.Values.Count == expected.Changes.Count && lease.Disposed, "service consumes grouped configuration and persists complete facts");
        Check(ledger.Facts!.Values.Any(fact => fact.Target.ValueName == "DelegateExecute") &&
            ledger.Facts.Values.Any(fact => Equals(fact.AppliedValue.Data, "\"" + Executable + "\" --archive-extract smart-dialog \"%1\"")), "service does not drop double/menu fields");
        var foreign = new ArchiveRegistryValueTarget(@"Software\Classes\*\shell\Other.App", "MUIVerb", @"Software\Classes\*\shell\Other.App");
        store.SeedValue(foreign, ArchiveRegistryValue.String("foreign"));
        var modified = ledger.Facts.Values.First(fact => fact.Target.ValueName == "DelegateExecute").Target;
        store.SeedValue(modified, ArchiveRegistryValue.String("user edit"));
        var reverted = Service(store, ledger, true, _ => new FakeLease()).Execute(command with { Action = "revert" }, Guid.NewGuid());
        Check(reverted.State == "reverted" && reverted.Preserved == 1 && store.Value(foreign) is not null &&
            Equals(store.Value(modified)?.Data, "user edit"), "flat exact uninstall preserves shared foreign and modified owned values");
        for (var index = 1; index <= expected.Changes.Count; index++)
        {
            var failing = new FakeArchiveIntegrationStore { FailWriteAt = index };
            var outcome = Service(failing, new MemoryLedger(), true, _ => new FakeLease()).Execute(command, Guid.NewGuid());
            Check(outcome.Failure == "write_failed_compensated" && failing.SnapshotValues().Count == 0 && failing.FirstWriteLogIndex == expected.Changes.Count, "every grouped write-then-throw compensates after full capture");
        }
        var failedLedgerStore = new FakeArchiveIntegrationStore { FailRestoreIdentity = expected.Changes[^1].Target.Identity };
        var failedLedger = Service(failedLedgerStore, new MemoryLedger { FailSave = true }, true, _ => new FakeLease()).Execute(command, Guid.NewGuid());
        Check(failedLedger.Failure == "ledger_failed_compensation_incomplete" && failedLedgerStore.RestoreAttempts == expected.Changes.Count, "grouped ledger compensation attempts all despite restore failure");
        var gatedStore = new FakeArchiveIntegrationStore();
        var gatedLedger = new MemoryLedger();
        var approvals = 0;
        var gated = Service(gatedStore, gatedLedger, false, _ => { approvals++; throw new InvalidOperationException(); }).Execute(command, Guid.NewGuid());
        Check(gated.State == "disabled" && gatedStore.OperationCount == 0 && gatedLedger.Loads == 0 && approvals == 0, "unauthorized Apply has zero capture/write/ledger/identity work");
        var deniedStore = new FakeArchiveIntegrationStore();
        var denied = Service(deniedStore, new MemoryLedger(), true, _ => throw new InvalidDataException()).Execute(command, Guid.NewGuid());
        Check(denied.Failure == "selection_helper_identity_unavailable" && deniedStore.OperationCount == 0, "helper approval failure never falls back to per-file plan");
        var nullApprovalStore = new FakeArchiveIntegrationStore();
        var nullApproval = Service(nullApprovalStore, new MemoryLedger(), true, _ => null!).Execute(command, Guid.NewGuid());
        Check(nullApproval.Failure == "selection_helper_identity_unavailable" && nullApprovalStore.OperationCount == 0, "null approval never falls back to legacy per-file commands");
        var unapprovedStore = new FakeArchiveIntegrationStore();
        var unapproved = new ArchiveIntegrationService(unapprovedStore, new MemoryLedger(), NullPlatformLog.Instance,
            () => Executable, () => { }, true).Execute(command, Guid.NewGuid());
        Check(unapproved.Failure == "selection_helper_identity_unavailable" && unapprovedStore.OperationCount == 0, "legacy service constructor cannot authorize grouped creation");
        var invalidStore = new FakeArchiveIntegrationStore();
        var invalid = Service(invalidStore, new MemoryLedger(), true, _ => new FakeLease()).Execute(command with { MenuCommands = ["bad"] }, Guid.NewGuid());
        Check(invalid.Failure == "invalid_intent" && invalidStore.OperationCount == 0, "direct service callers cannot bypass strict validation");
        var emptyStore = new FakeArchiveIntegrationStore();
        var empty = Service(emptyStore, new MemoryLedger(), true, _ => throw new InvalidOperationException()).Execute(command with { OpenWith = false, MenuCommands = [] }, Guid.NewGuid());
        Check(empty.State == "not_applied" && emptyStore.OperationCount == 0, "empty service plan neither approves helper nor writes");
        return Task.CompletedTask;
    }

    internal static Task CreateForwardsWholeGroupWithoutComOrLaunch()
    {
        var root = Path.Combine(Path.GetTempPath(), "nxtools-selection-settings-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(root);
        try
        {
            var desktop = Path.Combine(root, "NxTools.exe");
            File.WriteAllBytes(desktop, [1, 2, 3]);
            var hash = Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(desktop)));
            var launcher = new FakeLauncher();
            var forwarder = new DesktopIntentForwarder(new DesktopIdentity(root, hash), launcher);
            var paths = new[] { @"C:\first.txt", @"C:\folder", @"C:\last.zip" };
            var retired = 0;
            var command = new ArchiveSelectionCommand(ShellVerb.Compress, forwarder, new HelperLog(), CancellationToken.None, () => retired++);
            command.SetSelection(new ManagedArray(paths));
            Check(command.Execute() == 0 && launcher.Calls == 1 && launcher.Arguments.SequenceEqual(new[] { "--archive-compress" }.Concat(paths)), "original helper forwards complete ordered group once into original create ingress");
            Check(command.Execute() < 0 && launcher.Calls == 1 && retired == 1, "one invocation / one retire, no timer or union");
            var rejected = new DesktopIntentForwarder(new DesktopIdentity(root, ""), launcher).Forward(Guid.NewGuid(), ShellVerb.Compress, paths, CancellationToken.None);
            Check(rejected.Status == ForwardStatus.IdentityRejected && launcher.Calls == 1, "source-build empty identity remains fail-closed");
            foreach (var count in new[] { ShellSelectionReader.MaxPaths, ShellSelectionReader.MaxPaths + 1 })
            {
                var beforeCalls = launcher.Calls;
                var bounded = new ArchiveSelectionCommand(ShellVerb.Compress, forwarder, new HelperLog(), CancellationToken.None, () => { });
                bounded.SetSelection(new ManagedArray(Enumerable.Range(0, count).Select(index => $@"C:\{index}.txt").ToArray()));
                var outcome = bounded.Execute();
                Check(count == ShellSelectionReader.MaxPaths
                    ? outcome == 0 && launcher.Calls == beforeCalls + 1 && launcher.Arguments.Count == count + 1
                    : outcome < 0 && launcher.Calls == beforeCalls, "original complete selection count bound retained");
            }
        }
        finally { Directory.Delete(root, true); }
        return Task.CompletedTask;
    }

    internal static async Task FixedDefaultAppsWireResultsAndDetachedReader()
    {
        var valid = PlatformCommandParser.ParseOpenDefaultAppsSettings(Envelope("{\"command\":\"open_default_apps_settings\",\"data\":{}}"));
        foreach (var invalid in new[]
        {
            "{}", "{\"command\":\"open_default_apps_settings\"}", "{\"command\":\"other\",\"data\":{}}",
            "{\"command\":null,\"data\":{}}", "{\"command\":\"open_default_apps_settings\",\"data\":null}",
            "{\"command\":\"open_default_apps_settings\",\"data\":[]}", "{\"command\":\"open_default_apps_settings\",\"data\":\"ms-settings:defaultapps\"}",
            "{\"command\":\"open_default_apps_settings\",\"data\":{\"uri\":\"ms-settings:evil\"}}",
            "{\"command\":\"open_default_apps_settings\",\"data\":{\"path\":\"C:\\\\x\"}}",
            "{\"command\":\"open_default_apps_settings\",\"data\":{\"unknown\":false}}",
            "{\"command\":\"open_default_apps_settings\",\"data\":{},\"uri\":\"evil\"}",
        }) ExpectRejected(() => PlatformCommandParser.ParseOpenDefaultAppsSettings(Envelope(invalid)));
        var calls = 0;
        var correlation = Guid.NewGuid();
        var log = new RecordingLog();
        var opener = new DefaultAppsSettingsOpener(log, () => calls++);
        Check(opener.Open(valid, correlation, CancellationToken.None).Success && calls == 1, "fixed no-argument native operation invoked once");
        Check(log.Entries.Count == 3 && log.Entries.All(entry => entry.Correlation == correlation) &&
            log.Entries.Select(entry => entry.Event).SequenceEqual(["default_apps_settings.accepted", "default_apps_settings.started", "default_apps_settings.terminal"]), "correlated accepted/start/terminal lifecycle");
        using var cancelled = new CancellationTokenSource();
        cancelled.Cancel();
        var stopped = opener.Open(valid, correlation, cancelled.Token);
        Check(!stopped.Success && stopped.Data == "cancelled" && calls == 1, "pre-cancel does not launch");
        var failure = new DefaultAppsSettingsOpener(log, () => throw new Win32Exception(5, "secret path")).Open(valid, correlation, CancellationToken.None);
        Check(!failure.Success && failure.NativeErrorCode == 5 && !failure.Error!.Contains("secret"), "native failure returns exact code and fixed safe error");
        Check(new DefaultAppsSettingsOpener(new RecordingLog { Throw = true }, () => { }).Open(valid, correlation, CancellationToken.None).Success, "throwing log never changes result");
        Check(log.Entries.All(entry => entry.Fields.ContainsKey("elapsed_ms") && entry.Fields.ContainsKey("native_error_code") &&
            entry.Fields.Values.All(value => value is not string text || !text.Contains("secret"))), "bounded path-free elapsed/native diagnostics");
        using var entered = new ManualResetEventSlim();
        using var release = new ManualResetEventSlim();
        var dispatcher = new PlatformCommandSequenceDispatcher();
        var blocking = new DefaultAppsSettingsOpener(NullPlatformLog.Instance, () => { entered.Set(); release.Wait(TimeSpan.FromSeconds(5)); });
        try
        {
            var accepted = dispatcher.DispatchAsync("open_default_apps_settings", () => { blocking.Open(valid, correlation, CancellationToken.None); return Task.CompletedTask; });
            Check(accepted.IsCompletedSuccessfully && entered.Wait(TimeSpan.FromSeconds(2)), "fixed settings native call detached before blocking");
            var later = false;
            await dispatcher.DispatchAsync("later", () => { later = true; return Task.CompletedTask; });
            Check(later && !release.IsSet, "later reader command does not wait for Settings launch");
        }
        finally { release.Set(); await dispatcher.DrainAsync(TimeSpan.FromSeconds(3)); }
    }

    private static ProtocolEnvelope Envelope(string payload) => ProtocolEnvelope.Create("platform_command", JsonSerializer.Deserialize<JsonElement>(payload));
    private static ArchiveShellIntegrationCommand ParseArchive(string? extra = null) => PlatformCommandParser.ParseArchiveShellIntegration(
        Envelope("{\"command\":\"archive_shell_integration\",\"data\":{" + Legacy + (extra is null ? "" : "," + extra) + "}}"));
    private static void RejectArchive(string extra) => ExpectRejected(() => ParseArchive(extra));
    private static void ExpectRejected(Action action)
    {
        try { action(); }
        catch (Exception error) when (error is InvalidDataException or JsonException or ArgumentException) { return; }
        throw new InvalidOperationException("actual parser accepted invalid payload");
    }
    private static ArchiveIntegrationService Service(FakeArchiveIntegrationStore store, MemoryLedger ledger, bool enabled, Func<string, IDisposable> approve) =>
        new(store, ledger, NullPlatformLog.Instance, () => Executable, () => { }, enabled, approve);
    private static void Check(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }
    private sealed class FakeLease : IDisposable { internal bool Disposed; public void Dispose() => Disposed = true; }
    private sealed class MemoryLedger : IArchiveIntegrationLedger
    {
        internal ArchiveIntegrationAppliedFacts? Facts; internal bool FailSave; internal int Loads;
        public ArchiveIntegrationAppliedFacts? Load() { Loads++; return Facts; }
        public void Save(ArchiveIntegrationAppliedFacts facts) { if (FailSave) throw new IOException(); Facts = facts; }
        public void Delete() => Facts = null;
    }
    private sealed class RecordingLog : IPlatformLog
    {
        internal bool Throw;
        internal readonly List<(string Event, Guid? Correlation, IReadOnlyDictionary<string, object?> Fields)> Entries = [];
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null)
        { if (Throw) throw new IOException(); Entries.Add((eventName, correlationId, fields!)); }
    }
    private sealed class HelperLog : IHelperLog { public void Write(string eventName, Guid correlationId, IReadOnlyDictionary<string, object?> fields) { } }
    private sealed class FakeLauncher : IDesktopLauncher
    {
        internal int Calls; internal IReadOnlyList<string> Arguments = [];
        public int? Run(string executable, IReadOnlyList<string> arguments, TimeSpan timeout, CancellationToken cancellation)
        { Calls++; Arguments = arguments; return 0; }
    }
    private sealed class ManagedArray(string[] paths) : IShellItemArray
    {
        public int GetCount(out uint count) { count = (uint)paths.Length; return 0; }
        public int GetItemAt(uint index, out IShellItem item) { item = new ManagedItem(paths[index]); return 0; }
        public int BindToHandler(IntPtr context, ref Guid handler, ref Guid iid, out IntPtr result) { result = IntPtr.Zero; return -1; }
        public int GetPropertyStore(int flags, ref Guid iid, out IntPtr result) { result = IntPtr.Zero; return -1; }
        public int GetPropertyDescriptionList(IntPtr key, ref Guid iid, out IntPtr result) { result = IntPtr.Zero; return -1; }
        public int GetAttributes(int flags, uint mask, out uint attributes) { attributes = 0; return -1; }
        public int EnumItems(out IntPtr result) { result = IntPtr.Zero; return -1; }
    }
    private sealed class ManagedItem(string path) : IShellItem
    {
        public int GetDisplayName(uint sigdn, out IntPtr name) { name = Marshal.StringToCoTaskMemUni(path); return 0; }
        public int BindToHandler(IntPtr context, ref Guid handler, ref Guid iid, out IntPtr result) { result = IntPtr.Zero; return -1; }
        public int GetParent(out IShellItem parent) { parent = null!; return -1; }
        public int GetAttributes(uint mask, out uint attributes) { attributes = 0; return -1; }
        public int Compare(IShellItem other, uint hint, out int order) { order = 0; return -1; }
    }
}
