namespace NxTools.PlatformHost.Windows.Archive;

internal enum ArchiveIntegrationScope
{
    CurrentUserClasses,
}

internal enum ArchiveRegistryValueKind
{
    String,
    ExpandString,
    Dword,
    Qword,
    MultiString,
    Binary,
}

internal sealed record ArchiveRegistryValue(ArchiveRegistryValueKind Kind, object Data)
{
    internal static ArchiveRegistryValue String(string value) => new(ArchiveRegistryValueKind.String, value);
    internal static ArchiveRegistryValue ExpandString(string value) => new(ArchiveRegistryValueKind.ExpandString, value);
    internal static ArchiveRegistryValue Dword(int value) => new(ArchiveRegistryValueKind.Dword, value);

    internal bool ContentEquals(ArchiveRegistryValue other)
    {
        if (Kind != other.Kind) return false;
        return Data switch
        {
            byte[] bytes when other.Data is byte[] otherBytes => bytes.SequenceEqual(otherBytes),
            string[] strings when other.Data is string[] otherStrings => strings.SequenceEqual(otherStrings, StringComparer.Ordinal),
            _ => Equals(Data, other.Data),
        };
    }
}

internal sealed record ArchiveIntegrationIntent(
    bool ClassicCascadeEnabled,
    bool OpenWithEnabled,
    IReadOnlyList<string>? Extensions = null)
{
    internal string DoubleClickAction { get; init; } = "open";
    internal string MenuLayout { get; init; } = "cascade";
    internal IReadOnlyList<string> MenuCommands { get; init; } = DefaultMenuCommands;
    internal static string[] DefaultMenuCommands => ["open", "extract_smart", "extract_here", "extract_name", "test"];

    internal void Validate()
    {
        if (DoubleClickAction is not ("open" or "smart" or "name") || MenuLayout is not ("cascade" or "flat") ||
            MenuCommands is null || MenuCommands.Count > 7 ||
            MenuCommands.Distinct(StringComparer.Ordinal).Count() != MenuCommands.Count ||
            MenuCommands.Any(command => command is not ("open" or "test" or "extract_smart" or "extract_here" or "extract_name" or "extract_dialog" or "create")))
            throw new ArgumentException("Invalid archive association configuration.");
    }
    internal static ArchiveIntegrationIntent Disabled { get; } = new(false, false, []);
}

internal sealed record ArchiveRegistryValueTarget(string KeyPath, string ValueName, string CleanupRootPath)
{
    internal string Identity => KeyPath + "\0" + ValueName;
}

internal sealed record ArchiveRegistryValueChange(ArchiveRegistryValueTarget Target, ArchiveRegistryValue Value);

internal sealed record ArchiveRegistryValueSnapshot(
    bool Exists,
    ArchiveRegistryValue? Value,
    IReadOnlyList<string> MissingOwnedKeyPaths);

internal sealed record ArchiveAppliedValueFact(
    ArchiveRegistryValueTarget Target,
    ArchiveRegistryValue AppliedValue,
    ArchiveRegistryValueSnapshot Prior);

internal sealed record ArchiveIntegrationAppliedFacts(
    long DesiredRevision,
    IReadOnlyList<ArchiveAppliedValueFact> Values);

internal sealed record ArchiveIntegrationUninstallResult(
    int RemovedInstalledValues,
    int RestoredPriorValues,
    int PreservedModifiedValues);

internal interface IArchiveIntegrationStore
{
    ArchiveRegistryValueSnapshot Capture(ArchiveRegistryValueTarget target);
    void Write(ArchiveRegistryValueTarget target, ArchiveRegistryValue value);
    void Restore(ArchiveRegistryValueTarget target, ArchiveRegistryValueSnapshot snapshot);
}

internal sealed record ArchiveIntegrationPlan(
    ArchiveIntegrationScope Scope,
    long DesiredRevision,
    ArchiveIntegrationIntent SavedIntent,
    IReadOnlyList<string> Extensions,
    IReadOnlyList<ArchiveRegistryValueChange> Changes,
    ArchiveIntegrationAppliedFacts? AppliedFacts)
{
    private const string Classes = @"Software\Classes";
    private const string ProgId = "NxTools.Archive";
    private const string CascadeId = "NxTools.Archive";

    internal static ArchiveIntegrationPlan Create(
        string executablePath,
        long desiredRevision,
        ArchiveIntegrationIntent savedIntent,
        string? selectionHelperPath = null)
    {
        if (string.IsNullOrWhiteSpace(executablePath) || !Path.IsPathFullyQualified(executablePath))
            throw new ArgumentException("An absolute executable path is required.", nameof(executablePath));
        if (executablePath.IndexOf('\0') >= 0 || executablePath.IndexOf('"') >= 0)
            throw new ArgumentException("The executable path cannot be represented in a Shell command.", nameof(executablePath));
        if (desiredRevision < 0) throw new ArgumentOutOfRangeException(nameof(desiredRevision));

        var normalized = NormalizeIntent(savedIntent);
        var extensions = normalized.Extensions!;
        var changes = new List<ArchiveRegistryValueChange>();
        if (normalized.OpenWithEnabled)
            AddOpenWith(changes, executablePath, extensions, normalized.DoubleClickAction);
        if (normalized.ClassicCascadeEnabled)
        {
            if (selectionHelperPath is not null)
                changes.AddRange(ArchiveSelectionHandlerPlan.Create(selectionHelperPath, desiredRevision, normalized).Changes);
            else
            {
                AddClassicCascade(changes, executablePath, extensions, normalized);
                // Creation always needs the grouped selection helper, never a per-file %1 claim.
                if (normalized.MenuCommands.Contains("create"))
                    changes.AddRange(ArchiveSelectionHandlerPlan.Create(
                        Path.Combine(Path.GetDirectoryName(executablePath)!, ArchiveSelectionHandlerPlan.HelperExecutableName),
                        desiredRevision, normalized with { MenuCommands = ["create"] }).Changes);
            }
        }

        EnsureUniqueTargets(changes);
        return new(
            ArchiveIntegrationScope.CurrentUserClasses,
            desiredRevision,
            normalized,
            extensions,
            changes,
            null);
    }

    private static void AddOpenWith(
        ICollection<ArchiveRegistryValueChange> changes,
        string executablePath,
        IReadOnlyList<string> extensions,
        string doubleClickAction)
    {
        var root = Classes + "\\" + ProgId;
        Add(changes, root, "", "NxTools Archive", root);
        Add(changes, root + @"\DefaultIcon", "", Quote(executablePath) + ",0", root);
        Add(changes, root + @"\shell\open\command", "", Command(executablePath, doubleClickAction switch
        {
            "smart" => "--archive-extract smart-dialog",
            "name" => "--archive-extract name-dialog",
            _ => "--archive-open",
        }), root);

        foreach (var extension in extensions)
        {
            var openWith = Classes + "\\" + extension + @"\OpenWithProgids";
            Add(changes, openWith, ProgId, "", openWith);
        }
    }

    private static void AddClassicCascade(
        ICollection<ArchiveRegistryValueChange> changes,
        string executablePath,
        IReadOnlyList<string> extensions,
        ArchiveIntegrationIntent intent)
    {
        var selected = ArchiveSelectionHandlerPlan.Commands
            .Where(command => command.Command != "create" && intent.MenuCommands.Contains(command.Command)).ToArray();
        if (selected.Length == 0) return;
        foreach (var extension in extensions)
        {
            var parent = Classes + @"\SystemFileAssociations\" + extension + @"\shell\";
            var root = parent + CascadeId;
            if (intent.MenuLayout == "cascade")
            {
                Add(changes, root, "MUIVerb", "NxTools", root);
                Add(changes, root, "SubCommands", "", root);
                Add(changes, root, "Icon", Quote(executablePath) + ",0", root);
            }
            foreach (var command in selected)
            {
                var verb = intent.MenuLayout == "cascade" ? root + @"\shell\" + command.KeyId : root + "." + command.KeyId;
                var owned = intent.MenuLayout == "cascade" ? root : verb;
                Add(changes, verb, "MUIVerb", command.Label, owned);
                Add(changes, verb + @"\command", "", Command(executablePath, command.Arguments), owned);
            }
        }
    }

    internal static ArchiveIntegrationIntent NormalizeIntent(ArchiveIntegrationIntent intent)
    {
        ArgumentNullException.ThrowIfNull(intent);
        intent.Validate();
        var extensions = NormalizeExtensions(intent.Extensions);
        if ((intent.OpenWithEnabled || (intent.ClassicCascadeEnabled && intent.MenuCommands.Any(command => command != "create"))) && extensions.Count == 0)
            throw new ArgumentException("At least one archive extension is required for enabled integration.", nameof(intent));
        return intent with { Extensions = extensions, MenuCommands = intent.MenuCommands.ToArray() };
    }

    private static void Add(
        ICollection<ArchiveRegistryValueChange> changes,
        string keyPath,
        string valueName,
        string value,
        string cleanupRootPath) =>
        changes.Add(new(new(keyPath, valueName, cleanupRootPath), ArchiveRegistryValue.String(value)));

    private static string Command(string executablePath, string arguments) =>
        $"{Quote(executablePath)} {arguments} \"%1\"";

    private static string Quote(string value) => "\"" + value + "\"";

    private static IReadOnlyList<string> NormalizeExtensions(IReadOnlyList<string>? extensions)
    {
        var normalized = new SortedSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var extension in extensions ?? [])
        {
            if (string.IsNullOrWhiteSpace(extension))
                throw new ArgumentException("Archive extensions cannot be empty.", nameof(extensions));
            var value = extension.StartsWith('.') ? extension : "." + extension;
            if (value.Length < 2 || value.Length > 32 || value.Any(character =>
                    !char.IsAsciiLetterOrDigit(character) && character is not '.' and not '_' and not '-'))
                throw new ArgumentException("Archive extension contains unsupported characters.", nameof(extensions));
            normalized.Add(value.ToLowerInvariant());
        }
        return normalized.ToArray();
    }

    private static void EnsureUniqueTargets(IReadOnlyList<ArchiveRegistryValueChange> changes)
    {
        var targets = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var change in changes)
        {
            if (!change.Target.KeyPath.StartsWith(Classes + "\\", StringComparison.OrdinalIgnoreCase) ||
                change.Target.KeyPath.Contains("UserChoice", StringComparison.OrdinalIgnoreCase) ||
                !targets.Add(change.Target.Identity))
                throw new InvalidOperationException("Archive integration plan escaped or duplicated its owned HKCU Classes values.");
        }
    }
}

internal static class ArchiveIntegrationTransaction
{
    internal static ArchiveIntegrationAppliedFacts Apply(
        IArchiveIntegrationStore store,
        ArchiveIntegrationPlan plan)
    {
        ArgumentNullException.ThrowIfNull(store);
        ArgumentNullException.ThrowIfNull(plan);

        // Capture the complete pre-state before the first mutation. Mark a value attempted
        // before Write because a store can mutate and then throw.
        var prior = plan.Changes.Select(change => store.Capture(change.Target)).ToArray();
        var attempted = -1;
        try
        {
            for (var index = 0; index < plan.Changes.Count; index++)
            {
                attempted = index;
                store.Write(plan.Changes[index].Target, plan.Changes[index].Value);
            }
        }
        catch (Exception failure)
        {
            var rollbackFailures = new List<Exception>();
            for (var index = attempted; index >= 0; index--)
            {
                try { store.Restore(plan.Changes[index].Target, prior[index]); }
                catch (Exception rollbackFailure) { rollbackFailures.Add(rollbackFailure); }
            }
            if (rollbackFailures.Count > 0)
                throw new AggregateException(
                    "Archive integration apply failed and compensation was incomplete.",
                    new[] { failure }.Concat(rollbackFailures));
            throw;
        }

        return new(
            plan.DesiredRevision,
            plan.Changes.Select((change, index) =>
                new ArchiveAppliedValueFact(change.Target, change.Value, prior[index])).ToArray());
    }

    internal static ArchiveIntegrationUninstallResult Uninstall(
        IArchiveIntegrationStore store,
        ArchiveIntegrationAppliedFacts appliedFacts)
    {
        ArgumentNullException.ThrowIfNull(store);
        ArgumentNullException.ThrowIfNull(appliedFacts);

        var removed = 0;
        var restored = 0;
        var preserved = 0;
        var failures = new List<Exception>();
        foreach (var fact in appliedFacts.Values.Reverse())
        {
            try
            {
                var current = store.Capture(fact.Target);
                if (!current.Exists || current.Value is null || !current.Value.ContentEquals(fact.AppliedValue))
                {
                    preserved++;
                    continue;
                }
                store.Restore(fact.Target, fact.Prior);
                if (fact.Prior.Exists) restored++; else removed++;
            }
            catch (Exception failure)
            {
                failures.Add(failure);
            }
        }
        if (failures.Count > 0)
            throw new AggregateException("Archive integration uninstall was incomplete.", failures);
        return new(removed, restored, preserved);
    }
}
