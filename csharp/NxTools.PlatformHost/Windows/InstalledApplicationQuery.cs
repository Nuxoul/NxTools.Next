using System.ComponentModel;
using System.Diagnostics;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal sealed record InstalledApplicationFacts(string? PackageFamilyName, string? AppUserModelId, string? Name);

internal static class PackagedApplicationTarget
{
    internal const string Prefix = @"shell:AppsFolder\";

    internal static bool IsValid(string target) => target.StartsWith(Prefix, StringComparison.Ordinal) &&
        IsValidAppUserModelId(target[Prefix.Length..]);

    internal static bool IsValidAppUserModelId(string value)
    {
        if (value.Length is 0 or > 128) return false;
        var separator = value.IndexOf('!');
        if (separator <= 0 || separator == value.Length - 1 || value.LastIndexOf('!') != separator) return false;
        return value.All(c => char.IsAsciiLetterOrDigit(c) || c is '.' or '_' or '-' or '!');
    }
}

internal sealed class InstalledApplicationQuery
{
    private const string WorkerName = "NxTools.InstalledApplications";
    private const int MaximumItems = 4096;
    private readonly Func<CancellationToken, IEnumerable<InstalledApplicationFacts>> _enumerate;
    private readonly TimeSpan _timeout;

    internal InstalledApplicationQuery(
        Func<CancellationToken, IEnumerable<InstalledApplicationFacts>>? enumerate = null,
        TimeSpan? timeout = null)
    {
        _enumerate = enumerate ?? AppsFolderApplicationSource.Enumerate;
        _timeout = timeout ?? TimeSpan.FromSeconds(5);
    }

    internal WindowsCommandResult ExecuteBounded(Guid correlationId, IPlatformLog log, CancellationToken cancellationToken)
    {
        var started = Stopwatch.GetTimestamp();
        WindowsCommandResult result;
        if (cancellationToken.IsCancellationRequested)
        {
            result = new(false, "installed_applications_cancelled", NativeErrorCode: 995);
        }
        else if (!BoundedStaOperationRunner.TryRun(WorkerName, _timeout, token =>
        {
            WriteDiagnostic(log, "accepted", correlationId, started);
            WriteDiagnostic(log, "started", correlationId, started);
            try { return ReadComplete(token); }
            catch (OperationCanceledException) when (token.IsCancellationRequested)
            {
                return new WindowsCommandResult(false, "installed_applications_cancelled", NativeErrorCode: 995);
            }
            catch (Exception error)
            {
                return new WindowsCommandResult(false, "installed_applications_failed", NativeErrorCode:
                    error is Win32Exception native ? native.NativeErrorCode : error.HResult);
            }
        }, out var completed, out _, cancellationToken))
        {
            // A logical timeout/cancel does not release the runner's actual native-operation slot.
            result = new(false, "installed_applications_unconfirmed");
        }
        else
        {
            result = completed ?? new(false, "installed_applications_failed");
        }
        WriteDiagnostic(log, result.Success ? "completed" : "failed", correlationId, started, result.NativeErrorCode, result.Error);
        return result;
    }

    private WindowsCommandResult ReadComplete(CancellationToken token)
    {
        var applications = new List<object>();
        var targets = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        var examined = 0;
        token.ThrowIfCancellationRequested();
        foreach (var facts in _enumerate(token))
        {
            token.ThrowIfCancellationRequested();
            if (++examined > MaximumItems)
                return new(false, "installed_applications_limit_exceeded");
            if (string.IsNullOrWhiteSpace(facts.PackageFamilyName) || string.IsNullOrWhiteSpace(facts.AppUserModelId) ||
                string.IsNullOrWhiteSpace(facts.Name) || !PackagedApplicationTarget.IsValidAppUserModelId(facts.AppUserModelId)) continue;
            var separator = facts.AppUserModelId.IndexOf('!');
            if (!facts.AppUserModelId.AsSpan(0, separator).Equals(facts.PackageFamilyName.AsSpan(), StringComparison.OrdinalIgnoreCase)) continue;
            if (facts.Name.Length > 1024) return new(false, "installed_applications_limit_exceeded");
            var target = PackagedApplicationTarget.Prefix + facts.AppUserModelId;
            if (targets.Add(target)) applications.Add(new { name = facts.Name, target });
        }
        token.ThrowIfCancellationRequested();
        var data = JsonSerializer.Serialize(new { applications }, JsonDefaults.Options);
        // data is itself a JSON string in the original result envelope; account for that escaping.
        if (JsonSerializer.SerializeToUtf8Bytes(data, JsonDefaults.Options).Length > ProtocolConstants.MaxFrameBytes - 1024)
            return new(false, "installed_applications_limit_exceeded");
        return new(true, Data: data);
    }

    private static void WriteDiagnostic(IPlatformLog log, string phase, Guid correlationId, long started, int? nativeCode = null, string? failureKind = null)
    {
        try
        {
            log.Write(phase == "failed" ? "warning" : "info", "installed_applications." + phase, correlationId,
                new Dictionary<string, object?>
                {
                    ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds,
                    ["native_error_code"] = nativeCode,
                    ["failure_kind"] = failureKind
                });
        }
        catch { /* Diagnostics do not change enumeration results. */ }
    }
}
