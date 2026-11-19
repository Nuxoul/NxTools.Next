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
    /// <summary>Claims each extension's current-user class default so icons and double-click use the owned ProgID.</summary>
    internal bool SetDefault { get; init; }
    internal string MenuLayout { get; init; } = "cascade";
    internal IReadOnlyList<string> MenuCommands { get; init; } = RecommendedMenuCommands;
    internal static string[] RecommendedMenuCommands => [.. DefaultMenuCommands, "create_zip", "create_7z", "create"];
    // Historical explicit five-command plans keep their exact selection.
    internal static string[] DefaultMenuCommands => ["open", "extract_smart", "extract_here", "extract_name", "test"];

    internal void Validate()
    {
        if (DoubleClickAction is not ("open" or "smart" or "name") || MenuLayout is not ("cascade" or "flat") ||
            MenuCommands is null || MenuCommands.Count > 9 ||
            MenuCommands.Distinct(StringComparer.Ordinal).Count() != MenuCommands.Count ||
            MenuCommands.Any(command => command is not ("open" or "test" or "extract_smart" or "extract_here" or "extract_name" or "extract_dialog" or "create" or "create_zip" or "create_7z")))
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

/// <param name="PriorUserChoices">
/// Explorer default program (UserChoice ProgId, or null when none) of each extension captured
/// before an explicit default claim; null when the revision made no claim.
/// </param>
internal sealed record ArchiveIntegrationAppliedFacts(
    long DesiredRevision,
    IReadOnlyList<ArchiveAppliedValueFact> Values,
    IReadOnlyDictionary<string, string?>? PriorUserChoices = null);

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

/// <summary>Explorer per-user default program (FileExts UserChoice) for one extension.</summary>
internal interface IArchiveDefaultClaim
{
    /// <summary>Current remembered ProgId, or null when Explorer has none.</summary>
    string? Current(string extension);
    /// <summary>Writes a valid remembered choice for <paramref name="progId"/>, or removes it when null.</summary>
    void Set(string extension, string? progId);
    /// <summary>Whether Explorer's live association resolves the extension to <paramref name="progId"/>.</summary>
    bool Resolves(string extension, string progId);
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
    internal const string ProgId = "NxTools.Archive";
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
        if (normalized.OpenWithEnabled || normalized.SetDefault)
            AddOpenWith(changes, executablePath, extensions, normalized.DoubleClickAction, normalized.SetDefault);
        if (normalized.ClassicCascadeEnabled)
        {
            if (selectionHelperPath is not null)
                changes.AddRange(ArchiveSelectionHandlerPlan.Create(selectionHelperPath, desiredRevision, normalized).Changes);
            else
            {
                AddClassicCascade(changes, executablePath, extensions, normalized);
                // Creation always needs the grouped selection helper, never a per-file %1 claim.
                if (normalized.MenuCommands.Any(ArchiveSelectionHandlerPlan.IsCreation))
                    changes.AddRange(ArchiveSelectionHandlerPlan.Create(
                        Path.Combine(Path.GetDirectoryName(executablePath)!, ArchiveSelectionHandlerPlan.HelperExecutableName),
                        desiredRevision, normalized with { MenuCommands = normalized.MenuCommands.Where(ArchiveSelectionHandlerPlan.IsCreation).ToArray() }).Changes);
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
        string doubleClickAction,
        bool setDefault)
    {
        var root = Classes + "\\" + ProgId;
        Add(changes, root, "", "NxTools Archive", root);
        // Negative index selects RT_GROUP_ICON ID2, never the blue application group1.
        Add(changes, root + @"\DefaultIcon", "", Quote(executablePath) + ",-2", root);
        Add(changes, root + @"\shell\open\command", "", Command(executablePath, doubleClickAction switch
        {
            "smart" => "--archive-extract smart-dialog",
            "name" => "--archive-extract name-dialog",
            _ => "--archive-open",
        }), root);

        foreach (var extension in extensions)
        {
            // The class default precedes OpenWithProgids so reverse restoration empties the
            // child first and a created extension key can be retired when it becomes empty.
            if (setDefault)
            {
                var extensionKey = Classes + "\\" + extension;
                Add(changes, extensionKey, "", ProgId, extensionKey);
            }
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
            .Where(command => !ArchiveSelectionHandlerPlan.IsCreation(command.Command) && intent.MenuCommands.Contains(command.Command)).ToArray();
        if (selected.Length == 0) return;
        foreach (var extension in extensions)
        {
            var parent = Classes + @"\SystemFileAssociations\" + extension + @"\shell\";
            var root = parent + CascadeId;
            if (intent.MenuLayout == "cascade")
            {
                Add(changes, root, "MUIVerb", "NxTools", root);
                Add(changes, root, "SubCommands", "", root);
                Add(changes, root, "Icon", Quote(executablePath) + ",-2", root);
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
        if ((intent.OpenWithEnabled || intent.SetDefault || (intent.ClassicCascadeEnabled && intent.MenuCommands.Any(command => !ArchiveSelectionHandlerPlan.IsCreation(command)))) && extensions.Count == 0)
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
        ArchiveIntegrationPlan plan,
        CancellationToken cancellation = default)
    {
        ArgumentNullException.ThrowIfNull(store);
        ArgumentNullException.ThrowIfNull(plan);

        // Capture the complete pre-state before the first mutation. Mark a value attempted
        // before Write because a store can mutate and then throw.
        var prior = plan.Changes.Select(change =>
        {
            cancellation.ThrowIfCancellationRequested();
            return store.Capture(change.Target);
        }).ToArray();
        var attempted = -1;
        try
        {
            for (var index = 0; index < plan.Changes.Count; index++)
            {
                cancellation.ThrowIfCancellationRequested();
                attempted = index;
                store.Write(plan.Changes[index].Target, plan.Changes[index].Value);
            }
            cancellation.ThrowIfCancellationRequested();
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
