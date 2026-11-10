namespace NxTools.PlatformHost.Windows.Archive;

/// <summary>Pure grouped classic-menu values; explicit product Apply retains release approval before use.</summary>
internal static class ArchiveSelectionHandlerPlan
{
    internal const string HelperExecutableName = "NxTools.ShellSelectionHelper.exe";
    private const string Classes = @"Software\Classes";
    private const string GroupedCascadeId = "NxTools.ArchiveSelection";

    // Historical five-command overload/table remain compatible with P206/P207 source fixtures.
    internal static readonly IReadOnlyList<(string Verb, Guid Clsid, string Label)> Verbs =
    [
        ("open", new Guid("01E01D02-96B5-4338-901E-2C55D944A7A8"), "使用 NxTools 打开"),
        ("extract-smart", new Guid("104E8FCF-4BFC-47CA-9EE2-C8D5FC96D025"), "智能解压"),
        ("extract-here", new Guid("FBBF077F-81DF-4AE8-A56E-89F7FF2A43B7"), "解压到此处"),
        ("extract-name", new Guid("A0E1A23B-AF9E-4C5C-898D-0D486E6D170E"), "解压到同名文件夹"),
        ("test", new Guid("133C1261-3EBC-4B3A-89C6-9AECEB6C352D"), "测试压缩包"),
    ];

    internal sealed record MenuCommand(string Command, string KeyId, Guid Clsid, string Label, string Arguments);
    internal static readonly IReadOnlyList<MenuCommand> Commands =
    [
        new("open", "01open", Verbs[0].Clsid, Verbs[0].Label, "--archive-open"),
        new("extract_smart", "02extract-smart", Verbs[1].Clsid, Verbs[1].Label, "--archive-extract smart"),
        new("extract_here", "03extract-here", Verbs[2].Clsid, Verbs[2].Label, "--archive-extract here"),
        new("extract_name", "04extract-name", Verbs[3].Clsid, Verbs[3].Label, "--archive-extract name"),
        new("test", "05test", Verbs[4].Clsid, Verbs[4].Label, "--archive-test"),
        new("extract_dialog", "06extract-dialog", new("306F1051-91FA-4D1B-87BC-E02E9D68FB46"), "解压到…", "--archive-extract dialog"),
        new("create", "07create", new("BC9707AF-8361-431D-82CC-36554BDDA04C"), "新建压缩包…", "--archive-compress"),
        new("create_zip", "08create-zip", new("0E30B5E0-6767-4FFB-8198-D748035B3E6D"), "", "--archive-compress-zip"),
        new("create_7z", "09create-7z", new("55CCFF86-9D14-4750-9497-7A8D586A6402"), "", "--archive-compress-7z"),
    ];

    internal static ArchiveIntegrationPlan Create(string helperPath, long desiredRevision, IReadOnlyList<string> extensions) =>
        Create(helperPath, desiredRevision, new ArchiveIntegrationIntent(true, false, extensions) { MenuCommands = ArchiveIntegrationIntent.DefaultMenuCommands });

    internal static ArchiveIntegrationPlan Create(string helperPath, long desiredRevision, ArchiveIntegrationIntent intent)
    {
        if (string.IsNullOrWhiteSpace(helperPath) || !Path.IsPathFullyQualified(helperPath) ||
            !Path.GetFileName(helperPath).Equals(HelperExecutableName, StringComparison.OrdinalIgnoreCase) ||
            helperPath.IndexOf('"') >= 0 || helperPath.IndexOf('\0') >= 0)
            throw new ArgumentException("The exact packaged helper path is required.", nameof(helperPath));
        if (desiredRevision < 0) throw new ArgumentOutOfRangeException(nameof(desiredRevision));
        var normalized = ArchiveIntegrationPlan.NormalizeIntent(intent);
        var selected = Commands.Where(command => normalized.ClassicCascadeEnabled && normalized.MenuCommands.Contains(command.Command)).ToArray();
        var changes = new List<ArchiveRegistryValueChange>();
        foreach (var command in selected)
        {
            var root = Classes + @"\CLSID\" + command.Clsid.ToString("B").ToUpperInvariant();
            Add(changes, root, "", "NxTools Archive Selection (" + command.Command + ")", root);
            Add(changes, root + @"\LocalServer32", "", "\"" + helperPath + "\"", root);
        }
        var archiveCommands = selected.Where(command => !IsCreation(command.Command)).ToArray();
        foreach (var extension in normalized.Extensions!)
            AddMenu(changes, Classes + @"\SystemFileAssociations\" + extension, archiveCommands, normalized.MenuLayout);
        var create = selected.Where(command => IsCreation(command.Command)).ToArray();
        AddMenu(changes, Classes + @"\*", create, normalized.MenuLayout);
        AddMenu(changes, Classes + @"\Directory", create, normalized.MenuLayout);
        var identities = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var change in changes)
        {
            WindowsArchiveIntegrationStore.Validate(change.Target);
            if (!identities.Add(change.Target.Identity))
                throw new InvalidOperationException("Grouped selection plan duplicated owned HKCU Classes values.");
        }
        return new(ArchiveIntegrationScope.CurrentUserClasses, desiredRevision, normalized, normalized.Extensions!, changes, null);
    }

    private static void AddMenu(List<ArchiveRegistryValueChange> changes, string parent, IReadOnlyList<MenuCommand> commands, string layout)
    {
        if (commands.Count == 0) return;
        var root = parent + @"\shell\" + GroupedCascadeId;
        if (layout == "cascade")
        {
            Add(changes, root, "MUIVerb", "NxTools", root);
            Add(changes, root, "SubCommands", "", root);
        }
        foreach (var command in commands)
        {
            var key = layout == "cascade" ? root + @"\shell\" + command.KeyId : root + "." + command.KeyId;
            var owned = layout == "cascade" ? root : key;
            if (command.Command is "create_zip" or "create_7z")
                Add(changes, key, "ExplorerCommandHandler", command.Clsid.ToString("B").ToUpperInvariant(), owned);
            else
            {
                Add(changes, key, "MUIVerb", command.Label, owned);
                Add(changes, key + @"\command", "DelegateExecute", command.Clsid.ToString("B").ToUpperInvariant(), owned);
            }
        }
    }

    internal static bool IsCreation(string command) => command is "create" or "create_zip" or "create_7z";

    private static void Add(List<ArchiveRegistryValueChange> changes, string key, string name, string value, string root) =>
        changes.Add(new(new(key, name, root), ArchiveRegistryValue.String(value)));
}
