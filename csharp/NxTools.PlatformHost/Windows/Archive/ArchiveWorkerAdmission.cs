using System.Collections.ObjectModel;
using System.Text;

namespace NxTools.PlatformHost.Windows.Archive;

// One admission owner for every operation. No caller-owned collection or budget object survives
// Start. Counts/strings are charged before copying; two bounded observations must agree, and
// enumerators remain live until the whole capture ends (List's version guard includes overwrites).
internal static class ArchiveWorkerAdmission
{
    private const int MaximumBytes = 4 << 20;
    private sealed class Reservation
    {
        private int bytes;
        internal string Text(string? value, int maximum, string category)
        {
            if (value is null || value.Length > maximum) throw new ArchiveOutputException(category);
            var size = Encoding.UTF8.GetByteCount(value);
            if (size > maximum || size > MaximumBytes - bytes) throw new ArchiveOutputException(category);
            bytes += size;
            return value; // Strings themselves are immutable.
        }
    }
    internal static ArchiveWorkerRequest Capture(ArchiveWorkerRequest request)
    {
        var reservation = new Reservation();
        reservation.Text(request.Path, 32767 * 3, "invalid_request");
        if (request.Path.Length > 32767 || request.OutputRoot?.Length > 32767)
            throw new ArchiveOutputException("invalid_request");
        var creating = ArchiveWorkerRunner.IsCreation(request.Operation);
        if (request.CreateName?.Length > 255) throw new ArchiveOutputException(creating ? "invalid_component" : "invalid_create_request");
        if (creating && request.Password?.Length > 4096) throw new ArchiveOutputException("invalid_create_request");
        if (request.OutputRoot is { } output) reservation.Text(output, 32767 * 3, "invalid_request");
        if (request.CreateName is { } name) reservation.Text(name, 1020, "invalid_create_request");
        if (request.Password is { } password) reservation.Text(password, creating ? 16384 : ArchiveWorkerRunner.FrameLimit, "invalid_request");
        var guards = new List<IDisposable>();
        try {
            var sources = CopyList(request.SourcePaths, 32, 32767 * 3, 32 * 32767 * 3, "invalid_create_request", reservation, guards);
            var patterns = CopyList(request.ZipOptions?.ExcludePatterns, 32, 1024, 8192, "invalid_create_request", reservation, guards);
            var treePatterns = CopyList(request.TreeOptions?.ExcludePatterns, 32, 1024, 8192, "invalid_create_request", reservation, guards);
            var overwrites = CopyTargets(request.ConfirmedOverwrites, reservation, guards);
            var limits = ArchiveWorkerBudget.Capture(request.Limits);
            var browser = request.BrowserEntry is { } selection ? System.Text.Json.JsonSerializer.Deserialize<Protocol.ArchiveEntryCommand>(
                System.Text.Json.JsonSerializer.Serialize(selection, Protocol.JsonDefaults.Options), Protocol.JsonDefaults.Options) : null;
            if (browser is not null && (browser.Browser?.Valid != true || browser.OperationId != request.OperationId || browser.Path != request.Path || browser.Codepage != request.Codepage ||
                limits.max_entry_bytes > Protocol.ArchiveEntrySession.MaterializeEntryLimit || limits.max_output_bytes > Protocol.ArchiveEntrySession.MaterializeOutputLimit || request.ConfirmedOverwrites is not null)) throw new ArchiveOutputException("invalid_request");
            Protocol.ArchiveBrowserContext? authorization = null;
            if (request.BrowserAuthorize is { } context) {
                reservation.Text(context.Comment, 65536, "invalid_request");
                reservation.Text(context.Source.Native, 256, "invalid_request");
                authorization = context with { Source = context.Source with { Digest = context.Source.Digest.ToArray() } };
                if (!authorization.Valid || browser is not null || limits.max_entry_bytes > Protocol.ArchiveEntrySession.MaterializeEntryLimit || limits.max_output_bytes > Protocol.ArchiveEntrySession.MaterializeOutputLimit || request.ConfirmedOverwrites is not null) throw new ArchiveOutputException("invalid_request");
            }
            Protocol.ArchiveJobExtract? jobExtract=null;
            if(request.JobExtract is {} job) {
                if(!request.JobExecution || request.Operation!="extract" || !job.Valid)throw new ArchiveOutputException("invalid_request");
                jobExtract=System.Text.Json.JsonSerializer.Deserialize<Protocol.ArchiveJobExtract>(System.Text.Json.JsonSerializer.Serialize(job,Protocol.JsonDefaults.Options),Protocol.JsonDefaults.Options);
            }
            if (request.ExtractOptions?.Reclamation is { } reclaim && (!reclaim.Valid || request.Operation != "extract" || !request.JobExecution ||
                jobExtract?.Reclamation != reclaim || jobExtract.Valid != true || request.ExtractOptions.CopyMotw != jobExtract.CopyMotw || request.OutputReadmission is not null || request.DeleteSource is not null ||
                request.ConfirmedOverwrites is { Count: > 0 } || request.BrowserEntry is not null || request.BrowserAuthorize is not null || request.Password is not null ||
                request.ConflictPolicy != "ask" || string.IsNullOrEmpty(request.OutputRoot) || limits.max_metadata_bytes < ArchiveReclamationSession.MaximumJournalBytes))
                throw new ArchiveOutputException("reclamation_invalid_request");
            if (jobExtract?.Reclamation is not null && request.ExtractOptions?.Reclamation != jobExtract.Reclamation)
                throw new ArchiveOutputException("reclamation_invalid_request");
            Protocol.ArchiveJobModify? jobModify = null;
            if(request.JobModify is {} edit) {
                if(!ArchiveWorkerRunner.IsModification(request.Operation) || !request.JobExecution || edit.Browser?.Valid!=true || !ArchiveWorkerRunner.ModificationFormatMatches(request.Operation,edit.Browser.Format) || edit.Ordinals is null || edit.Ordinals.Length>4096 || edit.Value?.Length>65535 || sources?.Count>31) throw new ArchiveOutputException("invalid_edit_request");
                jobModify=System.Text.Json.JsonSerializer.Deserialize<Protocol.ArchiveJobModify>(System.Text.Json.JsonSerializer.Serialize(edit,Protocol.JsonDefaults.Options),Protocol.JsonDefaults.Options);
                if(!Protocol.ArchiveJobWire.ValidModify(jobModify,1+(sources?.Count??0)) || (edit.Action is "add" or "replace_content" ? sources is not {Count:>0} : sources is not null))throw new ArchiveOutputException("invalid_edit_request");
            }
            // Verify the complete capture against the caller again, not only each individual copy.
            CheckList(request.SourcePaths, sources);
            CheckList(request.ZipOptions?.ExcludePatterns, patterns);
            CheckList(request.TreeOptions?.ExcludePatterns, treePatterns);
            CheckTargets(request.ConfirmedOverwrites, overwrites);
            if (limits != ArchiveWorkerBudget.Capture(request.Limits)) throw new ArchiveOutputException("request_changed");
            foreach (var guard in guards) {
                if (guard is IEnumerator<string> list) { if (list.MoveNext()) throw new ArchiveOutputException("request_changed"); }
                else if (guard is IEnumerator<KeyValuePair<string, string>> targets) { if (targets.MoveNext()) throw new ArchiveOutputException("request_changed"); }
            }
            return request with { SourcePaths = sources, ZipOptions = request.ZipOptions is { } zip ? zip with { ExcludePatterns = patterns } : null,
                TreeOptions = request.TreeOptions is { } tree ? tree with { ExcludePatterns = treePatterns } : null,
                ConfirmedOverwrites = overwrites, Limits = limits, BrowserEntry = browser, BrowserAuthorize = authorization, JobExtract=jobExtract, JobModify=jobModify };
        }
        catch (Exception failure) when (failure is InvalidOperationException or ArgumentOutOfRangeException or IndexOutOfRangeException)
        { throw new ArchiveOutputException("request_changed"); }
        finally { foreach (var guard in guards) guard.Dispose(); }
    }
    private static IReadOnlyList<string>? CopyList(IReadOnlyList<string>? source, int maximumCount, int maximumString,
        int maximumTotal, string category, Reservation reservation, List<IDisposable> guards)
    {
        if (source is null) return null;
        var count = source.Count;
        if (count < 0 || count > maximumCount) throw new ArchiveOutputException(category);
        var copy = new string[count]; var total = 0;
        var enumerator = source.GetEnumerator(); guards.Add(enumerator);
        for (var index = 0; index < count; index++) {
            if (!enumerator.MoveNext()) throw new ArchiveOutputException("request_changed");
            var text = reservation.Text(enumerator.Current, maximumString, category);
            var size = Encoding.UTF8.GetByteCount(text);
            if (size > maximumTotal - total) throw new ArchiveOutputException(category);
            if (source.Count != count) throw new ArchiveOutputException("request_changed");
            total += size; copy[index] = text;
        }
        if (enumerator.MoveNext() || source.Count != count) throw new ArchiveOutputException("request_changed");
        return Array.AsReadOnly(copy); // Wrapper over our private copy, never over caller storage.
    }
    private static IReadOnlyDictionary<string, string>? CopyTargets(IReadOnlyDictionary<string, string>? source,
        Reservation reservation, List<IDisposable> guards)
    {
        if (source is null) return null;
        var count = source.Count;
        if (count is < 0 or > 4096) throw new ArchiveOutputException("session_entry_limit");
        var copy = new Dictionary<string, string>(count, StringComparer.Ordinal);
        var enumerator = source.GetEnumerator(); guards.Add(enumerator);
        for (var index = 0; index < count; index++) {
            if (!enumerator.MoveNext()) throw new ArchiveOutputException("request_changed");
            var pair = enumerator.Current;
            var key = reservation.Text(pair.Key, 32767 * 3, "invalid_request");
            var identity = reservation.Text(pair.Value, 256, "invalid_request");
            if (!copy.TryAdd(key, identity)) throw new ArchiveOutputException("invalid_request");
        }
        if (enumerator.MoveNext() || source.Count != count) throw new ArchiveOutputException("request_changed");
        return new ReadOnlyDictionary<string, string>(copy);
    }
    private static void CheckList(IReadOnlyList<string>? source, IReadOnlyList<string>? copy)
    {
        if (source is null) return;
        if (source.Count != copy!.Count) throw new ArchiveOutputException("request_changed");
        for (var index = copy.Count - 1; index >= 0; index--)
            if (!string.Equals(source[index], copy[index], StringComparison.Ordinal)) throw new ArchiveOutputException("request_changed");
    }
    private static void CheckTargets(IReadOnlyDictionary<string, string>? source, IReadOnlyDictionary<string, string>? copy)
    {
        if (source is null) return;
        if (source.Count != copy!.Count) throw new ArchiveOutputException("request_changed");
        foreach (var pair in copy)
            if (!source.TryGetValue(pair.Key, out var value) || !string.Equals(value, pair.Value, StringComparison.Ordinal))
                throw new ArchiveOutputException("request_changed");
    }
}

// Exactly the five supported unsigned scalar budgets, immutable and wire-name compatible.
internal sealed record ArchiveAdmittedLimits(ulong max_entries, ulong max_metadata_bytes, ulong max_working_bytes,
    ulong max_entry_bytes, ulong max_output_bytes);
