using System.Collections;
using System.Reflection;
using System.Text.Json;

namespace NxTools.PlatformHost.Windows.Archive;

internal sealed class ArchiveBudgetException(string category) : Exception
{ internal string Category { get; } = category; }

// Reservations are separate: codec workspace, retained metadata, fixed runtime/IPC overhead.
// A Job is the final process-commit ceiling, NOT a measurement of dictionary allocation.
internal static class ArchiveWorkerBudget
{
    internal const ulong Overhead = 64UL << 20;
    // Included in Overhead: four 64KiB transport payloads plus fixed headers/single-binding state.
    // This reservation does not enlarge process, codec, metadata or output admission.
    internal const ulong BulkTransportReservation = 1UL << 20;
    internal const int DefaultProcess = 832 << 20;
    internal static ArchiveAdmittedLimits Capture(object source)
    {
        if (source is ArchiveAdmittedLimits admitted) return admitted;
        var values = new Dictionary<string, ulong>(5, StringComparer.Ordinal);
        void Add(string key, object? value) {
            if (values.Count >= 5 || key is not ("max_entries" or "max_metadata_bytes" or "max_working_bytes" or "max_entry_bytes" or "max_output_bytes"))
                throw new ArchiveBudgetException("invalid_budget");
            var number = value switch {
                byte n => n, ushort n => n, uint n => n, ulong n => n,
                sbyte n when n >= 0 => (ulong)n, short n when n >= 0 => (ulong)n,
                int n when n >= 0 => (ulong)n, long n when n >= 0 => (ulong)n,
                JsonElement n when n.ValueKind == JsonValueKind.Number && n.TryGetUInt64(out var parsed) => parsed,
                _ => throw new ArchiveBudgetException("invalid_budget"),
            };
            if (!values.TryAdd(key, number)) throw new ArchiveBudgetException("invalid_budget");
        }
        if (source is JsonDocument document) source = document.RootElement;
        if (source is JsonElement element) {
            if (element.ValueKind != JsonValueKind.Object) throw new ArchiveBudgetException("invalid_budget");
            foreach (var property in element.EnumerateObject()) Add(property.Name, property.Value);
        } else if (source is IDictionary dictionary) {
            if (dictionary.Count > 5) throw new ArchiveBudgetException("invalid_budget");
            foreach (DictionaryEntry entry in dictionary) {
                if (entry.Key is not string key) throw new ArchiveBudgetException("invalid_budget");
                Add(key, entry.Value);
            }
            if (dictionary.Count != values.Count) throw new ArchiveBudgetException("request_changed");
        } else {
            if (source is null) throw new ArchiveBudgetException("invalid_budget");
            var properties = source.GetType().GetProperties(BindingFlags.Instance | BindingFlags.Public);
            if (properties.Length > 5) throw new ArchiveBudgetException("invalid_budget");
            foreach (var property in properties) {
                if (property.GetIndexParameters().Length != 0 || !property.CanRead) throw new ArchiveBudgetException("invalid_budget");
                Add(property.Name, property.GetValue(source));
            }
        }
        ulong Get(string key, ulong fallback) => values.GetValueOrDefault(key, fallback);
        return new(Get("max_entries", 1_000_000), Get("max_metadata_bytes", 256UL << 20),
            Get("max_working_bytes", 512UL << 20), Get("max_entry_bytes", 32UL << 30), Get("max_output_bytes", 100UL << 30));
    }
    internal static (int Memory, ArchiveAdmittedLimits Limits) Approve(ArchiveWorkerRequest request)
    {
        var limits = Capture(request.Limits);
        var work = limits.max_working_bytes; var metadata = limits.max_metadata_bytes;
        if (work is 0 or > (1UL << 30) || metadata is 0 or > (512UL << 20) || limits.max_entries is 0 or > 1_000_000 ||
            limits.max_entry_bytes == 0 || limits.max_output_bytes == 0)
            throw new ArchiveBudgetException("invalid_budget");
        if (work < (64UL << 20) || work + metadata + Overhead > (1UL << 30))
            throw new ArchiveBudgetException("resource_limit");
        var memory = checked((int)(work + metadata + Overhead));
        if (request.MemoryBytes != 0 && request.MemoryBytes != memory) throw new ArchiveBudgetException("budget_mismatch");
        return (memory, limits);
    }
}
