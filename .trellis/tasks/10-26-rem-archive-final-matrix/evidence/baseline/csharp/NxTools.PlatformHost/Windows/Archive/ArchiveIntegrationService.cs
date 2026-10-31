using System.Diagnostics;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;

namespace NxTools.PlatformHost.Windows.Archive;

/// <summary>
/// Wire request for the explicit P206 owner. Rust owns the saved desired intent/revision;
/// this command never carries an executable path or registry location.
/// </summary>
[System.Text.Json.Serialization.JsonUnmappedMemberHandling(System.Text.Json.Serialization.JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchiveShellIntegrationCommand(
    int Version,
    string Action,
    long DesiredRevision,
    bool ClassicCascade,
    bool OpenWith,
    string[]? Extensions)
{
    public string DoubleClickAction { get; init; } = "open";
    public string MenuLayout { get; init; } = "cascade";
    public string[] MenuCommands { get; init; } = ArchiveIntegrationIntent.DefaultMenuCommands;

    internal ArchiveIntegrationIntent ToIntent() => new(ClassicCascade, OpenWith, Extensions)
    {
        DoubleClickAction = DoubleClickAction,
        MenuLayout = MenuLayout,
        MenuCommands = MenuCommands,
    };
}

/// <summary>Observed outcome. A saved setting is never reported as an applied state.</summary>
internal sealed record ArchiveShellIntegrationResult(
    int Version,
    string State,
    long? DesiredRevision,
    long? AppliedRevision,
    int AppliedValues,
    int Removed,
    int Restored,
    int Preserved,
    string? Failure);

/// <summary>Persisted observed facts. Owned by the native side because only it observed them.</summary>
internal interface IArchiveIntegrationLedger
{
    ArchiveIntegrationAppliedFacts? Load();
    void Save(ArchiveIntegrationAppliedFacts facts);
    void Delete();
}

internal static class ArchiveShellIntegrationStates
{
    internal const string NotApplied = "not_applied";
    internal const string Applied = "applied";
    internal const string Reverted = "reverted";
    internal const string RevertRequired = "revert_required";
    internal const string Disabled = "disabled";
    internal const string Failed = "failed";
}

/// <summary>
/// Formal P206 owner: explicit Apply/Revert/Status over the existing pure plan and its
/// compensation transaction. Real application is gated off unless separately authorized.
/// </summary>
internal sealed class ArchiveIntegrationService
{
    internal const int WireVersion = 1;

    /// <summary>
    /// Real HKCU application is NOT authorized in this round. Changing this requires a separate
    /// user authorization for the exact executable/user scope and live Shell acceptance.
    /// </summary>
    internal const bool ProductApplyAuthorized = false;

    private static readonly object ServiceGate = new();
    private readonly IArchiveIntegrationStore _store;
    private readonly IArchiveIntegrationLedger _ledger;
    private readonly IPlatformLog _log;
    private readonly Func<string?> _executablePath;
    private readonly Action _notifyAssociationsChanged;
    private readonly bool _applyEnabled;
    private readonly Func<string, IDisposable>? _approveSelectionHelper;

    internal ArchiveIntegrationService(
        IArchiveIntegrationStore store,
        IArchiveIntegrationLedger ledger,
        IPlatformLog log,
        Func<string?> executablePath,
        Action notifyAssociationsChanged,
        bool applyEnabled,
        Func<string, IDisposable>? approveSelectionHelper = null)
    {
        _store = store;
        _ledger = ledger;
        _log = log;
        _executablePath = executablePath;
        _notifyAssociationsChanged = notifyAssociationsChanged;
        _applyEnabled = applyEnabled;
        _approveSelectionHelper = approveSelectionHelper;
    }

    internal static ArchiveIntegrationService CreateProduct(IPlatformLog log) =>
        new(
            new WindowsArchiveIntegrationStore(),
            FileArchiveIntegrationLedger.CreateDefault(),
            log,
            WindowsPlatformAdapter.ResolveApprovedDesktopExecutablePath,
            WindowsArchiveIntegrationStore.NotifyAssociationsChanged,
            ProductApplyAuthorized,
            // Original stable-handle release owner authenticates every helper member. No new trust source.
            executable => IndexerServiceInstaller.OpenArchiveRelease(Path.GetDirectoryName(executable)!));

    internal bool ApplyEnabled => _applyEnabled;

    internal ArchiveShellIntegrationResult Execute(ArchiveShellIntegrationCommand command, Guid correlationId)
    {
        ArgumentNullException.ThrowIfNull(command);
        var started = Stopwatch.GetTimestamp();
        var action = command.Action;
        Write("info", "archive.shell_integration.accepted", correlationId, new Dictionary<string, object?>
        {
            ["action"] = action,
            ["desired_revision"] = command.DesiredRevision,
        });
        ArchiveShellIntegrationResult result;
        Exception? failure = null;
        try
        {
            if (command.Version != WireVersion)
                result = Failed(command.DesiredRevision, "unsupported_version");
            else if (!ValidIntent(command))
                result = Failed(command.DesiredRevision, "invalid_intent");
            else
            {
                lock (ServiceGate)
                {
                    result = action switch
                    {
                        "status" => Status(command.DesiredRevision),
                        "apply" => Apply(command),
                        "revert" => Revert(command.DesiredRevision),
                        _ => Failed(command.DesiredRevision, "invalid_action"),
                    };
                }
            }
        }
        catch (Exception error)
        {
            failure = error;
            result = Failed(command.DesiredRevision, "unexpected_failure");
        }
        var fields = new Dictionary<string, object?>
        {
            ["action"] = action,
            ["state"] = result.State,
            ["failure"] = result.Failure,
            ["desired_revision"] = result.DesiredRevision,
            ["applied_revision"] = result.AppliedRevision,
            ["value_count"] = result.AppliedValues,
            ["removed"] = result.Removed,
            ["restored"] = result.Restored,
            ["preserved"] = result.Preserved,
            ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds,
        };
        if (failure is not null)
        {
            fields["exception_type"] = failure.GetType().Name;
            fields["native_error_code"] = failure.HResult;
        }
        Write(result.State == ArchiveShellIntegrationStates.Failed ? "warn" : "info",
            "archive.shell_integration.terminal", correlationId, fields);
        return result;
    }

    private ArchiveShellIntegrationResult Status(long desiredRevision)
    {
        ArchiveIntegrationAppliedFacts? facts;
        try { facts = _ledger.Load(); }
        catch (Exception) { return Failed(desiredRevision, "ledger_unreadable"); }
        return facts is null
            ? new(WireVersion, ArchiveShellIntegrationStates.NotApplied, desiredRevision, null, 0, 0, 0, 0, null)
            : new(WireVersion, ArchiveShellIntegrationStates.Applied, desiredRevision, facts.DesiredRevision,
                facts.Values.Count, 0, 0, 0, null);
    }

    private static bool ValidIntent(ArchiveShellIntegrationCommand command)
    {
        try
        {
            if (command.DesiredRevision < 0) return false;
            ArchiveIntegrationPlan.NormalizeIntent(command.ToIntent());
            return true;
        }
        catch (ArgumentException) { return false; }
    }

    private ArchiveShellIntegrationResult Apply(ArchiveShellIntegrationCommand command)
    {
        if (!_applyEnabled)
            return new(WireVersion, ArchiveShellIntegrationStates.Disabled, command.DesiredRevision, null,
                0, 0, 0, 0, "apply_not_authorized");

        ArchiveIntegrationAppliedFacts? existing;
        try { existing = _ledger.Load(); }
        catch (Exception) { return Failed(command.DesiredRevision, "ledger_unreadable"); }
        if (existing is not null)
            return new(WireVersion, ArchiveShellIntegrationStates.RevertRequired, command.DesiredRevision,
                existing.DesiredRevision, existing.Values.Count, 0, 0, 0, "revert_required");

        var executable = _executablePath();
        if (string.IsNullOrWhiteSpace(executable))
            return Failed(command.DesiredRevision, "desktop_identity_unavailable");

        IDisposable? helperApproval;
        try
        {
            helperApproval = command.ClassicCascade && command.MenuCommands.Length > 0 && _approveSelectionHelper is not null
                ? _approveSelectionHelper(executable) : null;
        }
        catch (Exception) { return Failed(command.DesiredRevision, "selection_helper_identity_unavailable"); }
        using var retainedHelperApproval = helperApproval;
        if (command.ClassicCascade && command.MenuCommands.Length > 0 && helperApproval is null &&
            (_approveSelectionHelper is not null || command.MenuCommands.Contains("create")))
            return Failed(command.DesiredRevision, "selection_helper_identity_unavailable");
        ArchiveIntegrationPlan plan;
        try
        {
            plan = ArchiveIntegrationPlan.Create(
                executable,
                command.DesiredRevision,
                command.ToIntent(),
                helperApproval is null ? null : Path.Combine(Path.GetDirectoryName(executable)!, ArchiveSelectionHandlerPlan.HelperExecutableName));
        }
        catch (ArgumentException) { return Failed(command.DesiredRevision, "invalid_intent"); }
        if (plan.Changes.Count == 0)
            return new(WireVersion, ArchiveShellIntegrationStates.NotApplied, command.DesiredRevision, null,
                0, 0, 0, 0, null);

        ArchiveIntegrationAppliedFacts facts;
        try { facts = ArchiveIntegrationTransaction.Apply(_store, plan); }
        catch (AggregateException) { return Failed(command.DesiredRevision, "compensation_incomplete"); }
        catch (Exception) { return Failed(command.DesiredRevision, "write_failed_compensated"); }

        // Observed facts become durable before success is reported. If the ledger cannot be
        // persisted, Revert could not prove ownership later, so the applied values are compensated.
        try { _ledger.Save(facts); }
        catch (Exception)
        {
            // Same convention as ArchiveIntegrationTransaction: attempt EVERY value in reverse;
            // one failed Restore never stops independent later restorations.
            var compensationFailures = 0;
            for (var index = facts.Values.Count - 1; index >= 0; index--)
            {
                try { _store.Restore(facts.Values[index].Target, facts.Values[index].Prior); }
                catch (Exception) { compensationFailures++; }
            }
            return compensationFailures > 0
                ? Failed(command.DesiredRevision, "ledger_failed_compensation_incomplete")
                : Failed(command.DesiredRevision, "ledger_failed_compensated");
        }
        NotifyChanged();
        return new(WireVersion, ArchiveShellIntegrationStates.Applied, command.DesiredRevision,
            facts.DesiredRevision, facts.Values.Count, 0, 0, 0, null);
    }

    private ArchiveShellIntegrationResult Revert(long desiredRevision)
    {
        ArchiveIntegrationAppliedFacts? facts;
        try { facts = _ledger.Load(); }
        catch (Exception) { return Failed(desiredRevision, "ledger_unreadable"); }
        if (facts is null)
            return new(WireVersion, ArchiveShellIntegrationStates.NotApplied, desiredRevision, null, 0, 0, 0, 0, null);
        if (!_applyEnabled)
            return new(WireVersion, ArchiveShellIntegrationStates.Disabled, desiredRevision, facts.DesiredRevision,
                facts.Values.Count, 0, 0, 0, "apply_not_authorized");

        ArchiveIntegrationUninstallResult outcome;
        try { outcome = ArchiveIntegrationTransaction.Uninstall(_store, facts); }
        catch (Exception)
        {
            // Keep the ledger: already-restored values no longer match and are skipped on retry.
            NotifyChanged();
            return new(WireVersion, ArchiveShellIntegrationStates.Failed, desiredRevision, facts.DesiredRevision,
                facts.Values.Count, 0, 0, 0, "revert_incomplete");
        }
        try { _ledger.Delete(); }
        catch (Exception)
        {
            NotifyChanged();
            return new(WireVersion, ArchiveShellIntegrationStates.Failed, desiredRevision, facts.DesiredRevision,
                facts.Values.Count, outcome.RemovedInstalledValues, outcome.RestoredPriorValues,
                outcome.PreservedModifiedValues, "ledger_retire_failed");
        }
        NotifyChanged();
        return new(WireVersion, ArchiveShellIntegrationStates.Reverted, desiredRevision, null, 0,
            outcome.RemovedInstalledValues, outcome.RestoredPriorValues, outcome.PreservedModifiedValues, null);
    }

    private void NotifyChanged()
    {
        try { _notifyAssociationsChanged(); }
        catch (Exception) { /* A refresh hint cannot change the observed registry result. */ }
    }

    private static ArchiveShellIntegrationResult Failed(long desiredRevision, string failure) =>
        new(WireVersion, ArchiveShellIntegrationStates.Failed, desiredRevision, null, 0, 0, 0, 0, failure);

    private void Write(string level, string eventName, Guid correlationId, IReadOnlyDictionary<string, object?> fields)
    {
        try { _log.Write(level, eventName, correlationId, fields); }
        catch (Exception) { /* Diagnostics never change the operation result. */ }
    }

    internal static string Serialize(ArchiveShellIntegrationResult result) =>
        JsonSerializer.Serialize(result, NxTools.PlatformHost.Protocol.JsonDefaults.Options);
}

/// <summary>Atomic JSON ledger of observed applied facts beside other per-user NxTools data.</summary>
internal sealed class FileArchiveIntegrationLedger(string path) : IArchiveIntegrationLedger
{
    private const int Version = 1;
    private const int MaxBytes = 1024 * 1024;

    internal static FileArchiveIntegrationLedger CreateDefault() => new(Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
        "NxTools", "NxTools.Next", "data", "archive-shell-integration-applied.json"));

    internal string FilePath => path;

    public ArchiveIntegrationAppliedFacts? Load()
    {
        if (!File.Exists(path)) return null;
        var info = new FileInfo(path);
        if (info.Length > MaxBytes) throw new InvalidDataException("Archive integration ledger exceeds its bound.");
        var document = JsonSerializer.Deserialize<LedgerDocument>(File.ReadAllBytes(path), Protocol.JsonDefaults.Options)
            ?? throw new InvalidDataException("Archive integration ledger was empty.");
        if (document.Version != Version || document.Values is null)
            throw new InvalidDataException("Unsupported archive integration ledger.");
        return new ArchiveIntegrationAppliedFacts(
            document.DesiredRevision,
            document.Values.Select(value => new ArchiveAppliedValueFact(
                new ArchiveRegistryValueTarget(value.KeyPath, value.ValueName, value.CleanupRootPath),
                FromWire(value.Applied),
                new ArchiveRegistryValueSnapshot(
                    value.PriorExists,
                    value.Prior is null ? null : FromWire(value.Prior),
                    value.MissingOwnedKeyPaths ?? []))).ToArray());
    }

    public void Save(ArchiveIntegrationAppliedFacts facts)
    {
        var document = new LedgerDocument(Version, facts.DesiredRevision, facts.Values.Select(fact => new LedgerFact(
            fact.Target.KeyPath,
            fact.Target.ValueName,
            fact.Target.CleanupRootPath,
            ToWire(fact.AppliedValue),
            fact.Prior.Exists,
            fact.Prior.Value is null ? null : ToWire(fact.Prior.Value),
            fact.Prior.MissingOwnedKeyPaths.ToArray())).ToArray());
        var bytes = JsonSerializer.SerializeToUtf8Bytes(document, Protocol.JsonDefaults.Options);
        if (bytes.Length > MaxBytes) throw new InvalidDataException("Archive integration ledger exceeds its bound.");
        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        var temporary = path + "." + Guid.NewGuid().ToString("N") + ".tmp";
        try
        {
            using (var stream = new FileStream(temporary, FileMode.CreateNew, FileAccess.Write, FileShare.None))
            {
                stream.Write(bytes);
                stream.Flush(true);
            }
            File.Move(temporary, path, true);
        }
        finally
        {
            if (File.Exists(temporary)) File.Delete(temporary);
        }
    }

    public void Delete()
    {
        if (File.Exists(path)) File.Delete(path);
    }

    private static LedgerValue ToWire(ArchiveRegistryValue value) => value.Data switch
    {
        string text => new(value.Kind.ToString(), text, null, null, null),
        int number => new(value.Kind.ToString(), null, number, null, null),
        long number => new(value.Kind.ToString(), null, number, null, null),
        string[] strings => new(value.Kind.ToString(), null, null, strings, null),
        byte[] bytes => new(value.Kind.ToString(), null, null, null, Convert.ToBase64String(bytes)),
        _ => throw new InvalidDataException("Unsupported archive integration value data."),
    };

    private static ArchiveRegistryValue FromWire(LedgerValue value)
    {
        if (!Enum.TryParse<ArchiveRegistryValueKind>(value.Kind, false, out var kind))
            throw new InvalidDataException("Unsupported archive integration value kind.");
        object data = kind switch
        {
            ArchiveRegistryValueKind.String or ArchiveRegistryValueKind.ExpandString =>
                value.Text ?? throw new InvalidDataException("Missing string data."),
            ArchiveRegistryValueKind.Dword => checked((int)(value.Number ?? throw new InvalidDataException("Missing DWORD."))),
            ArchiveRegistryValueKind.Qword => value.Number ?? throw new InvalidDataException("Missing QWORD."),
            ArchiveRegistryValueKind.MultiString => value.Strings ?? throw new InvalidDataException("Missing strings."),
            _ => Convert.FromBase64String(value.Base64 ?? throw new InvalidDataException("Missing binary.")),
        };
        return new ArchiveRegistryValue(kind, data);
    }

    private sealed record LedgerDocument(int Version, long DesiredRevision, LedgerFact[]? Values);
    private sealed record LedgerFact(
        string KeyPath,
        string ValueName,
        string CleanupRootPath,
        LedgerValue Applied,
        bool PriorExists,
        LedgerValue? Prior,
        string[]? MissingOwnedKeyPaths);
    private sealed record LedgerValue(string Kind, string? Text, long? Number, string[]? Strings, string? Base64);
}
