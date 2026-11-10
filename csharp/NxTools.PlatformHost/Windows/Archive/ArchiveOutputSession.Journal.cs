using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows.Archive;

internal sealed partial class ArchiveOutputSession
{
    private ulong maximumTransactions = 4096;
    private ulong journalBudget = ulong.MaxValue;
    private ulong journalReservation;
    private int activeFiles;
    private bool compactJournal;
    private int diagnosticCompletions;

    internal void ConfigureJournal(ArchiveAdmittedLimits limits)
    {
        if (transactions.Count != 0 || compactJournal) throw new ArchiveOutputException("invalid_state");
        maximumTransactions = Math.Min(limits.max_entries, limits.max_metadata_bytes / 2048);
        journalBudget = limits.max_metadata_bytes;
        compactJournal = true;
    }

    private void AdmitTransaction(string relative, string? expected)
    {
        // Retained native facts, dictionary/list entries, metadata and path storage are cumulative;
        // open streams/handles have a separate simultaneous ceiling.
        var reservation = 2048UL + (ulong)relative.Length * 4 + (ulong)(expected?.Length ?? 0) * 2;
        if ((ulong)(transactions.Count + pendingExtractionBegins) >= maximumTransactions || activeFiles + pendingExtractionBegins >= 4096 ||
            reservation > journalBudget - journalReservation)
            throw new ArchiveOutputException("session_entry_limit");
        journalReservation += reservation;
    }

    private IEnumerable<ArchiveJobCommit> JournalFacts() => transactions.Where(entry => entry.Journaled)
        .Select((entry, index) => new ArchiveJobCommit((ulong)index, entry.State.ToString(), entry.Replaced,
            (ulong)Math.Max(0, entry.Count), entry.NativeIdentity,
            entry.State is ArchiveCommitState.CommitInFlight or ArchiveCommitState.CommitOutcomeUnknown ? null : entry.Committed,
            entry.Cleanup));
    private static bool OrdinaryCommit(ArchiveJobCommit fact) => fact.State == "Committed" && fact.Published == true &&
        fact.Cleanup is "retained" or "retained_replacement" or "not_requested";

    internal ArchiveJobJournalSummary CommitSummary()
    {
        lock (extractionGate) {
            var groups = new Dictionary<(string State, bool? Published, string Cleanup), (ulong Files, ulong Bytes)>();
            foreach (var fact in JournalFacts()) {
                var key = (fact.State, fact.Published, fact.Cleanup ?? "not_requested");
                var prior = groups.GetValueOrDefault(key);
                groups[key] = (checked(prior.Files + 1), checked(prior.Bytes + fact.Bytes));
            }
            var outcomes = groups.Select(group => new ArchiveJobCommitOutcome(group.Key.State, group.Key.Published,
                group.Key.Cleanup, group.Value.Files, group.Value.Bytes)).ToArray();
            var total = outcomes.Aggregate(0UL, (sum, outcome) => checked(sum + outcome.Files));
            return new(total, total <= 256, outcomes);
        }
    }
}
