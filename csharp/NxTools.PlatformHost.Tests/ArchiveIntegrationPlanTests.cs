using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Reviewed effects: in-memory fake registry only. No Registry API, Explorer, COM,
// process, service, filesystem or system mutation.
internal static class ArchiveIntegrationPlanTests
{
    internal static Task DefaultsAreDisabledAndPlanIsPureHkcu()
    {
        var disabled = ArchiveIntegrationIntent.Disabled;
        Check(!disabled.ClassicCascadeEnabled && !disabled.OpenWithEnabled, "integration must default off");

        var plan = ArchiveIntegrationPlan.Create(
            @"C:\Program Files\NxTools\NxTools.exe",
            7,
            disabled);

        Check(plan.Changes.Count == 0, "disabled intent must produce no writes");
        Check(!plan.SavedIntent.ClassicCascadeEnabled && !plan.SavedIntent.OpenWithEnabled && plan.SavedIntent.Extensions!.Count == 0, "saved intent is retained without becoming an applied fact");
        Check(plan.AppliedFacts is null, "a pure plan must not claim observed registry state");
        Check(plan.Scope == ArchiveIntegrationScope.CurrentUserClasses, "only HKCU Software\\Classes is planned");

        Check(disabled.MenuCommands.Contains("create"), "fresh recommended menu includes creation without enabling integration");
        foreach (var layout in new[] { "cascade", "flat" })
        {
            var recommended = ArchiveIntegrationPlan.Create(@"C:\NxTools\NxTools.exe", 1,
                new ArchiveIntegrationIntent(true, false, [".zip"]) { MenuLayout = layout },
                @"C:\NxTools\NxTools.ShellSelectionHelper.exe");
            foreach (var source in new[] { @"Software\Classes\*\shell\", @"Software\Classes\Directory\shell\" })
                Check(recommended.Changes.Any(change => change.Target.KeyPath.StartsWith(source, StringComparison.OrdinalIgnoreCase)
                    && change.Target.ValueName == "DelegateExecute"), "recommended creation covers ordinary files and folders");
            var custom = ArchiveIntegrationPlan.Create(@"C:\NxTools\NxTools.exe", 1,
                new ArchiveIntegrationIntent(true, false, [".zip"]) { MenuLayout = layout, MenuCommands = ["test"] },
                @"C:\NxTools\NxTools.ShellSelectionHelper.exe");
            Check(!custom.Changes.Any(change => change.Target.KeyPath.StartsWith(@"Software\Classes\*\shell\")
                || change.Target.KeyPath.StartsWith(@"Software\Classes\Directory\shell\")), "saved custom choices are not expanded");
        }
        return Task.CompletedTask;
    }

    internal static Task ClassicCascadeAndOpenWithProgidsHaveExactOwnedValues()
    {
        var plan = ArchiveIntegrationPlan.Create(
            @"C:\Program Files\NxTools\NxTools.exe",
            11,
            new ArchiveIntegrationIntent(true, true, [".ZIP", ".7z", ".zip"]) { MenuCommands = ArchiveIntegrationIntent.DefaultMenuCommands });

        Check(plan.Extensions.SequenceEqual([".7z", ".zip"]), "extensions must be normalized and deduplicated");
        Check(plan.Changes.All(change => change.Target.KeyPath.StartsWith(@"Software\Classes\", StringComparison.OrdinalIgnoreCase)), "HKCU Classes only");
        Check(plan.Changes.All(change => !change.Target.KeyPath.Contains("UserChoice", StringComparison.OrdinalIgnoreCase)), "UserChoice is never planned");
        Check(plan.Changes.Select(change => change.Target.Identity).Distinct(StringComparer.OrdinalIgnoreCase).Count() == plan.Changes.Count, "owned targets must be unique");

        Value(plan, @"Software\Classes\NxTools.Archive", "", "NxTools Archive");
        Value(plan, @"Software\Classes\NxTools.Archive\DefaultIcon", "", "\"C:\\Program Files\\NxTools\\NxTools.exe\",-2");
        Value(plan, @"Software\Classes\SystemFileAssociations\.zip\shell\NxTools.Archive", "Icon", "\"C:\\Program Files\\NxTools\\NxTools.exe\",-2");
        Value(plan, @"Software\Classes\NxTools.Archive\shell\open\command", "", "\"C:\\Program Files\\NxTools\\NxTools.exe\" --archive-open \"%1\"");
        Value(plan, @"Software\Classes\.zip\OpenWithProgids", "NxTools.Archive", "");
        Value(plan, @"Software\Classes\SystemFileAssociations\.7z\shell\NxTools.Archive", "SubCommands", "");
        Value(plan, @"Software\Classes\SystemFileAssociations\.zip\shell\NxTools.Archive\shell\03extract-here\command", "", "\"C:\\Program Files\\NxTools\\NxTools.exe\" --archive-extract here \"%1\"");

        Check(plan.Changes.All(change => !change.Target.KeyPath.Contains("CLSID", StringComparison.OrdinalIgnoreCase)), "P206 must not smuggle COM registration into the classic plan");
        Check(plan.Changes.Where(change => change.Target.KeyPath.EndsWith("\\command", StringComparison.OrdinalIgnoreCase))
            .All(change => ((string)change.Value.Data).Contains("\"%1\"", StringComparison.Ordinal)), "classic commands are explicitly per-file");
        return Task.CompletedTask;
    }

    internal static Task ApplyCapturesAllBeforeWriteAndCompensatesWriteThenThrow()
    {
        var plan = ArchiveIntegrationPlan.Create(
            @"C:\NxTools\NxTools.exe",
            12,
            new ArchiveIntegrationIntent(true, true, [".zip"]));

        for (var failureAt = 1; failureAt <= plan.Changes.Count; failureAt++)
        {
            var store = new FakeArchiveIntegrationStore { FailWriteAt = failureAt };
            var owned = plan.Changes[0].Target;
            store.SeedValue(owned, ArchiveRegistryValue.Dword(73));
            var foreign = new ArchiveRegistryValueTarget(
                owned.CleanupRootPath,
                "ForeignValue",
                owned.CleanupRootPath);
            store.SeedValue(foreign, ArchiveRegistryValue.String("Foreign"));
            var before = store.SnapshotValues();

            Expect<IOException>(() => ArchiveIntegrationTransaction.Apply(store, plan));

            Check(store.FirstWriteLogIndex == plan.Changes.Count, "all targets must be captured before the first write");
            Check(store.ValuesEqual(before), $"write-then-throw at {failureAt} must restore exact values and kinds");
            Check(store.Value(foreign)?.ContentEquals(ArchiveRegistryValue.String("Foreign")) == true, "foreign values survive rollback");
        }
        return Task.CompletedTask;
    }

    internal static Task UninstallRestoresOwnedValuesAndPreservesForeignOrModifiedEntries()
    {
        var plan = ArchiveIntegrationPlan.Create(
            @"C:\NxTools\NxTools.exe",
            13,
            new ArchiveIntegrationIntent(true, true, [".zip"]));
        var store = new FakeArchiveIntegrationStore();
        var priorTarget = plan.Changes.Single(change => change.Target.KeyPath == @"Software\Classes\NxTools.Archive" && change.Target.ValueName == "").Target;
        store.SeedValue(priorTarget, ArchiveRegistryValue.ExpandString("Prior %LOCALAPPDATA%"));

        var facts = ArchiveIntegrationTransaction.Apply(store, plan);
        var modifiedTarget = plan.Changes.Single(change => change.Target.KeyPath.EndsWith(@"\02extract-smart\command", StringComparison.OrdinalIgnoreCase)).Target;
        store.SeedValue(modifiedTarget, ArchiveRegistryValue.String("user replacement"));
        var cascadeRoot = @"Software\Classes\SystemFileAssociations\.zip\shell\NxTools.Archive";
        var foreign = new ArchiveRegistryValueTarget(cascadeRoot, "ForeignValue", cascadeRoot);
        var foreignChild = cascadeRoot + @"\Foreign.Child";
        store.SeedValue(foreign, ArchiveRegistryValue.String("keep"));
        store.SeedKey(foreignChild);

        var result = ArchiveIntegrationTransaction.Uninstall(store, facts);

        Check(store.Value(priorTarget)?.ContentEquals(ArchiveRegistryValue.ExpandString("Prior %LOCALAPPDATA%")) == true, "prior value and kind restored");
        Check(store.Value(modifiedTarget)?.ContentEquals(ArchiveRegistryValue.String("user replacement")) == true, "user-modified owned value preserved");
        Check(store.Value(foreign)?.ContentEquals(ArchiveRegistryValue.String("keep")) == true && store.HasKey(foreignChild), "foreign value and child key under an owned key preserved");
        Check(result.PreservedModifiedValues == 1, "modified owned value is reported, not silently removed");
        Check(plan.Changes.Where(change => change.Target != modifiedTarget && change.Target != priorTarget)
            .All(change => store.Value(change.Target) is null), "unchanged newly-owned values removed exactly");
        Check(result.RemovedInstalledValues == plan.Changes.Count - 2 && result.RestoredPriorValues == 1, "uninstall accounting matches removed/restored ownership");
        return Task.CompletedTask;
    }

    internal static Task SavedIntentAndAppliedFactsRemainSeparate()
    {
        var intent = new ArchiveIntegrationIntent(false, true, [".zip"]);
        var plan = ArchiveIntegrationPlan.Create(@"C:\NxTools\NxTools.exe", 99, intent);
        var store = new FakeArchiveIntegrationStore();

        Check(plan.AppliedFacts is null && plan.SavedIntent.OpenWithEnabled, "saved desired state is not observed state");
        var facts = ArchiveIntegrationTransaction.Apply(store, plan);
        Check(plan.AppliedFacts is null, "apply must not mutate the saved plan into an applied claim");
        Check(facts.DesiredRevision == 99 && facts.Values.Count == plan.Changes.Count, "facts are separately returned from successful store writes");
        Check(facts.Values.All(fact => store.Value(fact.Target)?.ContentEquals(fact.AppliedValue) == true), "facts match observed fake-store values");

        var uninstall = ArchiveIntegrationTransaction.Uninstall(store, facts);
        Check(uninstall.RemovedInstalledValues == facts.Values.Count && store.SnapshotValues().Count == 0, "exact owned uninstall removes unchanged values");
        Check(intent.OpenWithEnabled, "uninstalling applied facts does not rewrite saved intent");
        return Task.CompletedTask;
    }

    private static void Value(ArchiveIntegrationPlan plan, string keyPath, string name, string expected)
    {
        var actual = plan.Changes.Single(change => change.Target.KeyPath == keyPath && change.Target.ValueName == name).Value;
        Check(actual.Kind == ArchiveRegistryValueKind.String && (string)actual.Data == expected, $"exact value {keyPath}::{name}");
    }

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

    internal sealed class FakeArchiveIntegrationStore : IArchiveIntegrationStore
    {
        private readonly Dictionary<string, (ArchiveRegistryValueTarget Target, ArchiveRegistryValue Value)> _values = new(StringComparer.OrdinalIgnoreCase);
        private readonly HashSet<string> _keys = new(StringComparer.OrdinalIgnoreCase) { @"Software", @"Software\Classes" };
        private readonly List<string> _log = [];
        private int _writes;

        internal int? FailWriteAt { get; init; }
        internal int OperationCount => _log.Count;
        internal string? FailRestoreIdentity { get; init; }
        internal int RestoreAttempts { get; private set; }
        internal int FirstWriteLogIndex => _log.FindIndex(entry => entry.StartsWith("write:", StringComparison.Ordinal));

        public ArchiveRegistryValueSnapshot Capture(ArchiveRegistryValueTarget target)
        {
            _log.Add("capture:" + target.Identity);
            var missing = OwnedPath(target.CleanupRootPath, target.KeyPath).Where(path => !_keys.Contains(path)).ToArray();
            return _values.TryGetValue(target.Identity, out var entry)
                ? new ArchiveRegistryValueSnapshot(true, entry.Value, missing)
                : new ArchiveRegistryValueSnapshot(false, null, missing);
        }

        public void Write(ArchiveRegistryValueTarget target, ArchiveRegistryValue value)
        {
            _log.Add("write:" + target.Identity);
            EnsureKeys(target.KeyPath);
            _values[target.Identity] = (target, value);
            if (++_writes == FailWriteAt) throw new IOException("synthetic write-after-mutation failure");
        }

        public void Restore(ArchiveRegistryValueTarget target, ArchiveRegistryValueSnapshot snapshot)
        {
            RestoreAttempts++;
            if (FailRestoreIdentity is not null && target.Identity.Equals(FailRestoreIdentity, StringComparison.OrdinalIgnoreCase))
                throw new IOException("synthetic restore failure");
            if (snapshot.Exists)
            {
                EnsureKeys(target.KeyPath);
                _values[target.Identity] = (target, snapshot.Value!);
            }
            else
            {
                _values.Remove(target.Identity);
                foreach (var path in snapshot.MissingOwnedKeyPaths.OrderByDescending(path => path.Length))
                {
                    if (!_values.Values.Any(entry => IsAtOrBelow(entry.Target.KeyPath, path)) &&
                        !_keys.Any(key => !key.Equals(path, StringComparison.OrdinalIgnoreCase) && IsAtOrBelow(key, path)))
                        _keys.Remove(path);
                }
            }
        }

        internal void SeedValue(ArchiveRegistryValueTarget target, ArchiveRegistryValue value)
        {
            EnsureKeys(target.KeyPath);
            _values[target.Identity] = (target, value);
        }

        internal void SeedKey(string keyPath) => EnsureKeys(keyPath);
        internal bool HasKey(string keyPath) => _keys.Contains(keyPath);
        internal ArchiveRegistryValue? Value(ArchiveRegistryValueTarget target) =>
            _values.TryGetValue(target.Identity, out var value) ? value.Value : null;

        internal Dictionary<string, ArchiveRegistryValue> SnapshotValues() =>
            _values.ToDictionary(pair => pair.Key, pair => pair.Value.Value, StringComparer.OrdinalIgnoreCase);

        internal bool ValuesEqual(IReadOnlyDictionary<string, ArchiveRegistryValue> expected) =>
            expected.Count == _values.Count && expected.All(pair => ValueByIdentity(pair.Key)?.ContentEquals(pair.Value) == true);

        private ArchiveRegistryValue? ValueByIdentity(string identity) =>
            _values.TryGetValue(identity, out var value) ? value.Value : null;

        private void EnsureKeys(string keyPath)
        {
            var current = "";
            foreach (var component in keyPath.Split('\\'))
            {
                current = current.Length == 0 ? component : current + "\\" + component;
                _keys.Add(current);
            }
        }

        private static IEnumerable<string> OwnedPath(string root, string leaf)
        {
            var current = root;
            yield return current;
            if (leaf.Equals(root, StringComparison.OrdinalIgnoreCase)) yield break;
            var suffix = leaf[(root.Length + 1)..];
            foreach (var component in suffix.Split('\\'))
            {
                current += "\\" + component;
                yield return current;
            }
        }

        private static bool IsAtOrBelow(string candidate, string root) =>
            candidate.Equals(root, StringComparison.OrdinalIgnoreCase) ||
            candidate.StartsWith(root + "\\", StringComparison.OrdinalIgnoreCase);
    }
}
