using System.Diagnostics;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Threading.Channels;
using NxTools.PlatformHost.Diagnostics;

namespace NxTools.PlatformHost.Windows.Extensions;

internal sealed record CredentialRequest(int Version, string Op, Guid RequestId, string PackageId, string SettingKey, string? Value = null);
internal sealed record CredentialResult(Guid RequestId, bool Success, string? Reason = null, int? NativeErrorCode = null,
    bool? Found = null, string? Value = null, long ElapsedMs = 0, string Op = "credential_result", int Version = 1);

// No config/trust/Save transaction here. Parent captures prior values, records attempted writes
// before dispatch, and compensates all attempts if its atomic nonsecret configuration commit fails.
internal interface ICredentialStore
{
    string? Read(string target);
    void Write(string target, string value);
    void Delete(string target);
}

internal sealed class CredentialAdapter(ICredentialStore store, IPlatformLog? log = null)
{
    internal const int ValueLimit = 32768;
    internal static void Validate(CredentialRequest request)
    {
        if (request.Version != 1 || request.RequestId == Guid.Empty ||
            request.Op is not ("credential_get" or "credential_set" or "credential_delete") ||
            !Key(request.PackageId) || !Key(request.SettingKey) ||
            (request.Op == "credential_set" ? request.Value is null : request.Value is not null))
            throw new ExtensionFailure("invalid_credential_request");
        if (request.Value is not null) ValidateValue(request.Value);
    }
    private static bool Key(string? value) => !string.IsNullOrWhiteSpace(value) && value.Length <= 128 &&
        value.All(c => char.IsAsciiLetterOrDigit(c) || c is '.' or '_' or '-');
    internal static void ValidateValue(string value)
    {
        try
        {
            if (ExtensionProcessProtocol.Utf8.GetByteCount(value) > ValueLimit) throw new ExtensionFailure("credential_value_limit");
        }
        catch (EncoderFallbackException) { throw new ExtensionFailure("credential_invalid_utf8"); }
    }
    // Hash framed UTF8 components: stable case-sensitive identity without delimiter/path collisions.
    // Credential Manager already scopes the store to the current logon user; SID makes it explicit.
    internal static string Target(string sid, string package, string setting)
    {
        using var hash = IncrementalHash.CreateHash(HashAlgorithmName.SHA256);
        foreach (var part in new[] { sid, package, setting })
        {
            var bytes = ExtensionProcessProtocol.Utf8.GetBytes(part);
            hash.AppendData(BitConverter.GetBytes(bytes.Length)); hash.AppendData(bytes);
        }
        return "NxTools.Next/extensions/v1/" + Convert.ToHexString(hash.GetHashAndReset());
    }
    internal CredentialResult Execute(CredentialRequest request)
    {
        var clock = Stopwatch.StartNew();
        CredentialResult result;
        Diagnostic("accepted", request.RequestId, null, null, 0);
        try
        {
            Validate(request);
            ExtensionNative.Ordinary(ExtensionNative.CurrentProcess());
            var sid = ExtensionNative.Identity(ExtensionNative.CurrentProcess()).Sid;
            var target = Target(sid, request.PackageId, request.SettingKey);
            switch (request.Op)
            {
                case "credential_get":
                    var value = store.Read(target);
                    if (value is not null) ValidateValue(value);
                    result = new(request.RequestId, true, Found: value is not null, Value: value);
                    break;
                case "credential_set": store.Write(target, request.Value!); result = new(request.RequestId, true); break;
                default: store.Delete(target); result = new(request.RequestId, true); break;
            }
        }
        catch (ExtensionFailure e) { result = new(request.RequestId, false, e.Reason, e.NativeCode); }
        catch (Exception) { result = new(request.RequestId, false, "credential_store_failure"); }
        result = result with { ElapsedMs = clock.ElapsedMilliseconds };
        Diagnostic(result.Success ? "completed" : "failed", request.RequestId, result.Reason, result.NativeErrorCode, result.ElapsedMs);
        return result;
    }
    private void Diagnostic(string phase, Guid id, string? reason, int? native, long elapsed)
    {
        try { log?.Write("info", "extension.credential." + phase, id, new Dictionary<string, object?>
            { ["reason"] = reason, ["native_error_code"] = native, ["elapsed_ms"] = elapsed }); }
        catch (Exception) { /* No payload in diagnostics; sink never changes result. */ }
    }
}

// One fixed worker, eight pending controls; slow native Credential Manager cannot block
// process cancellation/control input or create unlimited timeout replacements.
internal sealed class CredentialDispatcher : IDisposable
{
    private readonly Channel<CredentialRequest> queue = Channel.CreateBounded<CredentialRequest>(new BoundedChannelOptions(8)
        { SingleReader = false, FullMode = BoundedChannelFullMode.Wait }); // Shutdown also drains queued references.
    private readonly Action<CredentialResult> publish;
    private int stopping;
    internal Task Worker { get; }
    internal CredentialDispatcher(CredentialAdapter adapter, Action<CredentialResult> publish)
    {
        this.publish = publish;
        Worker = Task.Run(async () =>
        {
            await foreach (var request in queue.Reader.ReadAllAsync())
            {
                if (Volatile.Read(ref stopping) != 0) break;
                var result = adapter.Execute(request);
                if (Volatile.Read(ref stopping) == 0) publish(result);
            }
        });
    }
    internal bool TrySchedule(CredentialRequest request)
    {
        if (Volatile.Read(ref stopping) == 0 && queue.Writer.TryWrite(request)) return true;
        publish(new(request.RequestId, false, "credential_queue_limit")); return false;
    }
    public void Dispose()
    {
        Interlocked.Exchange(ref stopping, 1); queue.Writer.TryComplete();
        while (queue.Reader.TryRead(out _)) { } // Release queued secret references, not executing native call.
    }
}
