using System.Buffers.Binary;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;
using System.Threading.Channels;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows.Extensions;
using System.Security.Cryptography;

namespace NxTools.PlatformHost.Windows.Archive;

// Private stdio: u32LE typed JSON, or a negotiated high-bit binary bulk record.
// Rust owns archive state/codecs. Only this boundary owns Windows process/Job lifetime.
internal sealed record ArchiveWorkerRequest(Guid OperationId, string Operation, string Path, int Codepage,
    object Limits, string? Password = null, int MemoryBytes = 0, int TimeoutMs = 30000, string? OutputRoot = null,
    IReadOnlyDictionary<string, string>? ConfirmedOverwrites = null, string? CreateName = null, string? CompressionLevel = null,
    ArchiveZipCreateOptions? ZipOptions = null, IReadOnlyList<string>? SourcePaths = null,
    ArchiveTreeCreateOptions? TreeOptions = null, ArchiveSevenzCreateOptions? SevenzOptions = null, ArchiveExtractOptions? ExtractOptions = null,
    bool? DeleteSource = null, bool ActiveTemporarySource = false, int? ReadContract = null, ArchiveReadTransport? ReadTransport = null,
    bool PasswordInteraction = false, string? ConflictPolicy = null, JsonElement? OutputReadmission = null,
    Protocol.ArchiveEntryCommand? BrowserEntry = null, bool JobExecution = false, Protocol.ArchiveJobExtract? JobExtract = null,
    ulong? VolumeBytes = null, Protocol.ArchiveJobModify? JobModify = null, Protocol.ArchiveBrowserContext? BrowserAuthorize = null,
    ArchiveDragMaterialization? DragMaterialization = null, bool JobLargeCount = false, bool ParallelExtraction = false);
internal sealed record ArchiveReadTransport(ulong MaxTotalBytes, bool BrowserContext = false);
internal sealed record ArchiveZipCreateOptions(string Method = "deflate", string Encryption = "none", bool StripSingleRoot = false,
    IReadOnlyList<string>? ExcludePatterns = null,
    [property: System.Text.Json.Serialization.JsonIgnore(Condition=System.Text.Json.Serialization.JsonIgnoreCondition.WhenWritingDefault)] bool SelfExtracting = false);
internal sealed record ArchiveTreeCreateOptions(bool StripSingleRoot = false, IReadOnlyList<string>? ExcludePatterns = null);
internal sealed record ArchiveSevenzCreateOptions(bool Solid = false, string Encryption = "none", bool EncryptHeader = false);
internal sealed record ArchiveExtractOptions(bool CopyMotw = true,
    [property: System.Text.Json.Serialization.JsonIgnore(Condition=System.Text.Json.Serialization.JsonIgnoreCondition.WhenWritingNull)] Protocol.ArchiveReclamationOptions? Reclamation = null);
internal sealed record ArchiveWorkerEvent(string Kind, Guid OperationId, string? Category = null,
    int? NativeErrorCode = null, long ElapsedMs = 0, JsonElement? Frame = null, uint? ExitCode = null,
    bool CleanupComplete = false, int FilesCommitted = 0, string? CommitState = null, ArchiveSourceDeletionResult? SourceDeletion = null,
    ulong? PrivateBytes = null, ulong? ProcessBudgetBytes = null, ulong FilesSkipped = 0,
    IReadOnlyList<Protocol.ArchiveJobCommit>? Journal = null, ulong BytesCommitted = 0,
    Protocol.ArchiveJobResultLocation? ResultLocation = null, bool? OutputPublished = null,
    Protocol.ArchiveJobAddResult? AddResult = null,
    [property: System.Text.Json.Serialization.JsonIgnore] bool HasNativeFailure = false,
    Protocol.ArchiveJobJournalSummary? JournalSummary = null);

internal sealed partial class ArchiveWorkerRunner : IDisposable
{
    internal const int FrameLimit = 1048576;
    internal const int DefaultMemory = ArchiveWorkerBudget.DefaultProcess;
    private const long AggregateMemory = 2L * 1024 * 1024 * 1024;
    private static readonly object ProductReadGate = new();
    // Root original Runs/leases across pipe-session replacement until actual native cleanup.
    private static readonly Dictionary<Guid, Run> ProductReads = [];
    private readonly object gate = new();
    private readonly Dictionary<Guid, Run> active = [];
    private readonly HashSet<Guid> retired = [];
    private readonly Queue<Guid> recent = new();
    private readonly string executable, argument, directory;
    private readonly Action<ArchiveWorkerEvent> publish;
    private readonly IPlatformLog? log;
    private readonly Action<ExtensionKernelHandle>? admissionProbe;
    private IArchiveRecycleBackend? recycleFixtureBackend;
    private ArchiveReadHelper? readHelper;
    private Func<Guid, JsonElement, CancellationToken, Task<string?>>? readFrame;
    internal Func<Guid,bool,ulong?,ulong,CancellationToken,Task<bool>>? ConfirmDestination { get; set; }
    internal ulong? AvailableSpaceFixture { get; set; }
    internal static ArchiveWorkerRunner ForProductRead(ArchiveReadHelper helper, Action<ArchiveWorkerEvent> publish,
        Func<Guid, JsonElement, CancellationToken, Task<string?>> frame, IPlatformLog? log = null)
    {
        var runner = new ArchiveWorkerRunner(helper.Executable, helper.Argument, publish, log);
        runner.readHelper = helper; runner.readFrame = frame; return runner;
    }
    internal static ArchiveWorkerRunner ForBrowserEntry(ArchiveReadHelper helper, Action<ArchiveWorkerEvent> publish, IPlatformLog? log = null)
    { var runner = new ArchiveWorkerRunner(helper.Executable, helper.Argument, publish, log); runner.readHelper = helper; return runner; }
    private Func<JsonElement, JsonElement>? recycleEvidenceFixtureProbe;
    internal static ArchiveWorkerRunner ForProductJob(ArchiveReadHelper helper, Action<ArchiveWorkerEvent> publish,
        Func<Guid, JsonElement, CancellationToken, Task<string?>> frame, IPlatformLog? log = null) => ForProductRead(helper, publish, frame, log);
    internal static ArchiveWorkerRunner ForRecycleFixture(Action<ArchiveWorkerEvent> publish, IArchiveRecycleBackend backend, IPlatformLog? log = null,
        Func<JsonElement, JsonElement>? evidenceProbe = null)
    {
        if (!string.Equals(Path.GetFileName(Environment.ProcessPath), "NxTools.PlatformHost.Tests.exe", StringComparison.OrdinalIgnoreCase))
            throw new InvalidOperationException("Isolated recycle backend requires current test apphost.");
        var runner = ForPackage(publish, log); runner.recycleFixtureBackend = backend; runner.recycleEvidenceFixtureProbe = evidenceProbe; return runner;
    }
    internal (int FilesCommitted, string? State) InspectOutputCommit(Guid id) {
        lock (gate) return active.TryGetValue(id, out var run) && run.Output is { } output
            ? (output.FilesCommitted, output.LastCommitState?.ToString()) : (0, null);
    }
    internal Action<string>? SourceCheckpointProbe { get; set; }
    internal Action<ArchiveNativeReadStage, ArchiveNativeReadKind, long>? EditSourceReadChunkProbeForFixture { get; set; }
    internal Func<string, string>? SourceNamespacePathProbe { get; set; }
    internal Func<string, string>? SourceOutputNamespacePathProbe { get; set; }
    internal Action<string>? OutputCommitCheckpointProbe { get; set; }
    internal Action<string>? OutputBeginCheckpointProbe { get; set; }
    internal Action? CreateOverwritePreflightProbe { get; set; }
    internal Action<int>? OutputWriteCheckpointProbe { get; set; }
    internal Action<string>? ControlInputCheckpointProbe { get; set; }
    internal Func<bool, uint, uint>? NamedStreamsFlagsProbe { get; set; }
    internal Action? CleanupCheckpointProbe { get; set; }
    internal Action<string, IArchiveRecycleAttempt>? RecycleSessionProbe { get; set; }
    internal static bool IsTarCreation(string operation) => operation is "create_tar" or "create_tgz" or "create_txz";
    internal static bool IsTreeCreation(string operation) => operation is "create_zip" or "create_7z" || IsTarCreation(operation);
    internal static bool IsCreation(string operation) => IsTreeCreation(operation) || operation is "create_gzip" or "create_xz";
    internal static bool IsModification(string operation) => operation is "modify_zip" or "modify_7z" or "modify_tar";
    internal static bool ModificationFormatMatches(string operation, string format) => operation switch {
        "modify_zip" => format == "Zip", "modify_7z" => format == "SevenZ",
        "modify_tar" => format is "Tar" or "TarGz" or "TarXz", _ => false };
    internal static bool SupportsCapabilities(string operation, JsonElement capabilities, int maximum = 24) {
        if (capabilities.ValueKind != JsonValueKind.Array || capabilities.GetArrayLength() > maximum || capabilities.EnumerateArray().Any(value => value.ValueKind != JsonValueKind.String)) return false;
        bool Has(string name) => capabilities.EnumerateArray().Any(value => value.GetString() == name);
        return Has("source_lease_v1") && Has("budget_reservations_v1") &&
            (operation != "extract" || Has("extraction_metadata_v1") && Has("motw_v1")) &&
            (!IsCreation(operation) && !IsModification(operation) || Has("partial_read_v1")) &&
            (!IsModification(operation) || Has(operation switch {"modify_7z"=>"sevenz_modify_v1","modify_tar"=>"tar_modify_v1",_=>"zip_modify_v1"}) && Has("source_tree_v1")) &&
            (!IsTreeCreation(operation) || Has("source_tree_v1") && Has(operation switch { "create_zip" => "zip_create_v1", "create_7z" => "sevenz_create_v1", "create_tar" => "tar_create_v1", "create_tgz" => "tgz_create_v1", _ => "txz_create_v1" }));
    }
    private bool stopped;
    internal int ActiveCount { get { lock (gate) return active.Count; } }

    // Never accept a caller-supplied production path from a platform command.
    internal static ArchiveWorkerRunner ForPackage(Action<ArchiveWorkerEvent> publish, IPlatformLog? log = null)
    {
        // Historical codec fixtures own this exact test-apphost sibling. Never an environment/PATH
        // override for a production PlatformHost; formal Read/Job always supply their original helper.
        if (string.Equals(Path.GetFileName(Environment.ProcessPath), "NxTools.PlatformHost.Tests.exe", StringComparison.OrdinalIgnoreCase))
            return new(Path.Combine(AppContext.BaseDirectory, ArchiveReadHelper.DeploymentName), "--archive-worker", publish, log);
        var helper = ArchiveReadHelper.Production();
        return new(helper.Executable, helper.Argument, publish, log) { readHelper = helper };
    }
    // Only the test harness may select its own apphost as an executable.
    internal static ArchiveWorkerRunner ForFixture(string testApphost, Action<ArchiveWorkerEvent> publish,
        IPlatformLog? log = null, Action<ExtensionKernelHandle>? probe = null)
    {
        if (!string.Equals(Path.GetFullPath(testApphost), Path.GetFullPath(Environment.ProcessPath!), StringComparison.OrdinalIgnoreCase) ||
            !string.Equals(Path.GetFileName(testApphost), "NxTools.PlatformHost.Tests.exe", StringComparison.OrdinalIgnoreCase))
            throw new ArgumentException("Only the current test apphost is a fixture executable.", nameof(testApphost));
        return new(testApphost, "--archive-worker-fixture", publish, log, probe);
    }
    private ArchiveWorkerRunner(string executable, string argument, Action<ArchiveWorkerEvent> publish,
        IPlatformLog? log, Action<ExtensionKernelHandle>? probe = null)
    {
        this.executable = executable; this.argument = argument;
        directory = Path.GetDirectoryName(executable)!;
        this.publish = publish; this.log = log; admissionProbe = probe;
    }
    internal bool Start(ArchiveWorkerRequest request, Func<(string Signature, long Size), CancellationToken, Task<bool>>? confirmCreateOverwrite = null, byte[]? browserPassword = null)
    {
        string? reason = null;
        int? rejectionNative = null;
        byte[]? input = null;
        if (request.BrowserAuthorize is { } authorization && (!authorization.Valid || authorization.Format is not ("Zip" or "SevenZ" or "Rar4" or "Rar5") || request.Operation != "test" || !request.PasswordInteraction || request.ReadContract is not null || request.BrowserEntry is not null || request.OutputRoot is not null || request.Password is not null || request.JobExecution || request.DeleteSource is not null || request.ConflictPolicy is not null || request.OutputReadmission is not null || readHelper is null)) reason = "invalid_request";
        if (browserPassword is not null && (request.BrowserEntry is null || request.PasswordInteraction || request.BrowserAuthorize is not null || !ValidPasswordBytes(browserPassword))) reason = "invalid_request";
        if (request.BrowserEntry is { } entry && (entry.Action != "materialize" || entry.Browser?.Valid != true || entry.Ordinal is null || request.Operation != "extract" || request.ReadContract is not null || readHelper is null || request.Password is not null || request.DeleteSource is not null || request.ConflictPolicy is not null || request.OutputReadmission is not null)) reason = "invalid_request";
        if (request.JobExecution && (readHelper is null || request.ReadContract is not null || request.BrowserEntry is not null || request.DeleteSource is not null)) reason = "invalid_request";
        if (request.JobLargeCount && (!request.JobExecution || request.Operation != "extract" || request.JobExtract is null || request.ExtractOptions?.Reclamation is not null)) reason = "invalid_request";
        if (confirmCreateOverwrite is not null && (!request.JobExecution || !IsCreation(request.Operation) || request.ConfirmedOverwrites is not null)) reason = "invalid_request";
        if (request.ConflictPolicy is not null && (request.ConflictPolicy is not ("ask" or "overwrite" or "skip" or "auto_rename") || request.Operation != "extract" && !(IsModification(request.Operation) && request.JobExecution && request.ConflictPolicy == "ask") || request.ReadContract is not null || request.ConfirmedOverwrites is not null)) reason = "invalid_request";
        if (request.OutputReadmission is not null && (request.Operation != "extract" || request.ReadContract is not null || request.DeleteSource is not null)) reason = "invalid_request";
        int memory = 0;
        if (request.PasswordInteraction && (request.ReadContract is not null && !(request.ReadContract == 2 && request.Operation == "open" && request.ReadTransport?.BrowserContext == true) || request.Operation is not ("open" or "test" or "extract") && !IsModification(request.Operation) || argument == "--archive-worker-fixture")) reason = "invalid_request";
        if (request.ReadContract is not null && (request.ReadContract is not (1 or 2) || request.Operation is not ("open" or "test") || readHelper is null || request.Password is not null) ||
            (request.ReadContract == 2) != (request.ReadTransport is not null) ||
            request.ReadTransport is { MaxTotalBytes: 0 or > 67108864 }) reason = "invalid_request";
        if (request.OperationId == Guid.Empty || request.Operation is not ("open" or "test" or "extract") && !IsModification(request.Operation) && !IsCreation(request.Operation) ||
            string.IsNullOrWhiteSpace(request.Path) || request.Path.Length > 32767 || request.Codepage < 0 ||
            (request.TimeoutMs == 0 ? !request.JobLargeCount : request.TimeoutMs < 200 || request.TimeoutMs > (request.Operation == "create_zip" ? 600000 : 120000)))
            reason = "invalid_request";
        else if ((request.Operation == "extract" || IsModification(request.Operation) || IsCreation(request.Operation)) != !string.IsNullOrEmpty(request.OutputRoot))
            reason = "invalid_request";
        else
        {
            try
            {
                request = ArchiveWorkerAdmission.Capture(request);
                request = request with { DragMaterialization = CaptureVirtual(request) };
                if (request.VolumeBytes is {} volume && (volume < 65536 || volume > ((ArchiveAdmittedLimits)request.Limits).max_output_bytes || request.Operation is not ("create_zip" or "create_7z") || request.ConfirmedOverwrites is not null || confirmCreateOverwrite is not null)) throw new ArchiveOutputException("invalid_create_request");
                if (IsModification(request.Operation)) {
                    if (!request.JobExecution || request.JobModify?.Browser.Valid != true || !ModificationFormatMatches(request.Operation,request.JobModify.Browser.Format) || request.ReadContract is not null || request.CreateName is not null || request.CompressionLevel is not null || request.VolumeBytes is not null || request.ConfirmedOverwrites is not null ||
                        !string.Equals(Path.GetDirectoryName(request.Path),request.OutputRoot,StringComparison.OrdinalIgnoreCase)) throw new ArchiveOutputException("invalid_edit_request");
                } else if (request.JobModify is not null) throw new ArchiveOutputException("invalid_edit_request");
                if (IsCreation(request.Operation)) {
                    var isZip = request.Operation == "create_zip";
                    var isSevenz = request.Operation == "create_7z";
                    var isTar = request.Operation == "create_tar";
                    if (argument == "--archive-worker-fixture" || !isZip && !isSevenz && request.Password is not null ||
                        !(isTar ? request.CompressionLevel is null : request.CompressionLevel is "fast" or "normal" or "maximum" || isZip && request.CompressionLevel == "store") || string.IsNullOrEmpty(request.CreateName))
                        throw new ArchiveOutputException("invalid_create_request");
                    if (!isSevenz && request.SevenzOptions is not null || !isSevenz && !IsTarCreation(request.Operation) && request.TreeOptions is not null) throw new ArchiveOutputException("invalid_create_request");
                    if (isZip) {
                        var zip = request.ZipOptions ?? new();
                        if (zip.SelfExtracting && (zip.Encryption != "none" || request.Password is not null ||
                            request.CreateName!.Length <= 4 || !request.CreateName.EndsWith(".exe",StringComparison.OrdinalIgnoreCase) ||
                            request.VolumeBytes > 4294967295UL || request.ConfirmedOverwrites is not null || confirmCreateOverwrite is not null))
                            throw new ArchiveOutputException("invalid_create_request");
                        if (zip.Method is not ("store" or "deflate") || zip.Encryption is not ("none" or "aes256") ||
                            (zip.Method == "store") != (request.CompressionLevel == "store") ||
                            (zip.Encryption == "aes256") != !string.IsNullOrEmpty(request.Password) ||
                            zip.Encryption == "none" && request.Password is not null || request.Password?.Length > 4096 || request.Password?.Contains('\0') == true ||
                            request.SourcePaths?.Count is < 1 or > 32 || request.SourcePaths is { } inputs && inputs[0] != request.Path ||
                            zip.ExcludePatterns?.Count > 32 || zip.ExcludePatterns?.Sum(p => p.Length) > 8192)
                            throw new ArchiveOutputException("invalid_create_request");
                    } else if (isSevenz) {
                        var sevenz = request.SevenzOptions ?? new();
                        if (request.ZipOptions is not null || sevenz.Encryption is not ("none" or "aes256") ||
                            (sevenz.Encryption == "aes256") != !string.IsNullOrEmpty(request.Password) ||
                            sevenz.Encryption == "none" && (request.Password is not null || sevenz.EncryptHeader) ||
                            request.Password?.Length > 4096 || request.Password?.Contains('\0') == true ||
                            request.SourcePaths?.Count is < 1 or > 32 || request.SourcePaths is { } inputs && inputs[0] != request.Path ||
                            request.TreeOptions?.ExcludePatterns?.Count > 32 || request.TreeOptions?.ExcludePatterns?.Sum(p => p.Length) > 8192)
                            throw new ArchiveOutputException("invalid_create_request");
                    } else if (IsTarCreation(request.Operation)) {
                        if (request.ZipOptions is not null || request.SourcePaths?.Count is < 1 or > 32 ||
                            request.SourcePaths is { } inputs && inputs[0] != request.Path || request.TreeOptions?.ExcludePatterns?.Count > 32 ||
                            request.TreeOptions?.ExcludePatterns?.Sum(p => p.Length) > 8192) throw new ArchiveOutputException("invalid_create_request");
                    } else if (request.ZipOptions is not null || request.SourcePaths is not null) throw new ArchiveOutputException("invalid_create_request");
                    ArchiveOutputSession.Validate(request.CreateName);
                    if (request.ConfirmedOverwrites is { } targets && (targets.Count > 1 || targets.Keys.Any(key => key != request.CreateName)))
                        throw new ArchiveOutputException("invalid_create_request");
                    if (string.Equals(Path.GetFullPath(request.Path), Path.GetFullPath(Path.Combine(request.OutputRoot!, request.CreateName)), StringComparison.OrdinalIgnoreCase))
                        throw new ArchiveOutputException("source_output_collision");
                } else if (request.CreateName is not null || request.CompressionLevel is not null || request.ZipOptions is not null || request.SourcePaths is not null && !IsModification(request.Operation) || request.TreeOptions is not null || request.SevenzOptions is not null) throw new ArchiveOutputException("invalid_create_request");
                if (request.ExtractOptions is not null && request.Operation != "extract") throw new ArchiveOutputException("invalid_extract_request");
                if (request.DeleteSource is not null && request.Operation != "extract" && !IsCreation(request.Operation)) throw new ArchiveOutputException("invalid_delete_request");
                var budget = ArchiveWorkerBudget.Approve(request);
                memory = budget.Memory;
                input = request.Operation == "create_zip"
                    ? Pack(new { schema = 2, operation_id = request.OperationId, op = "start", operation = request.Operation,
                        path = request.Path, codepage = request.Codepage, limits = budget.Limits, password = request.Password,
                        create_name = request.CreateName, compression_level = request.CompressionLevel,
                        zip_options = new { method = (request.ZipOptions ?? new()).Method, encryption = (request.ZipOptions ?? new()).Encryption,
                            strip_single_root = request.ZipOptions?.StripSingleRoot ?? false,
                            exclude_patterns = request.ZipOptions?.ExcludePatterns ?? ["Thumbs.db", "desktop.ini"] } })
                    : request.Operation == "create_7z"
                    ? Pack(new { schema = 2, operation_id = request.OperationId, op = "start", operation = request.Operation,
                        path = request.Path, codepage = request.Codepage, limits = budget.Limits, password = request.Password,
                        create_name = request.CreateName, compression_level = request.CompressionLevel,
                        tree_options = new { strip_single_root = request.TreeOptions?.StripSingleRoot ?? false,
                            exclude_patterns = request.TreeOptions?.ExcludePatterns ?? ["Thumbs.db", "desktop.ini"] },
                        sevenz_options = new { solid = request.SevenzOptions?.Solid ?? false, encryption = request.SevenzOptions?.Encryption ?? "none", encrypt_header = request.SevenzOptions?.EncryptHeader ?? false } })
                    : request.Operation == "create_tar"
                    ? Pack(new { schema = 2, operation_id = request.OperationId, op = "start", operation = request.Operation,
                        path = request.Path, codepage = request.Codepage, limits = budget.Limits, password = request.Password,
                        create_name = request.CreateName, tree_options = new { strip_single_root = request.TreeOptions?.StripSingleRoot ?? false,
                            exclude_patterns = request.TreeOptions?.ExcludePatterns ?? ["Thumbs.db", "desktop.ini"] } })
                    : request.Operation is "create_tgz" or "create_txz"
                    ? Pack(new { schema = 2, operation_id = request.OperationId, op = "start", operation = request.Operation,
                        path = request.Path, codepage = request.Codepage, limits = budget.Limits, password = request.Password,
                        create_name = request.CreateName, compression_level = request.CompressionLevel,
                        tree_options = new { strip_single_root = request.TreeOptions?.StripSingleRoot ?? false,
                            exclude_patterns = request.TreeOptions?.ExcludePatterns ?? ["Thumbs.db", "desktop.ini"] } })
                    : request.Operation is "create_gzip" or "create_xz"
                    ? Pack(new { schema = 2, operation_id = request.OperationId, op = "start", operation = request.Operation,
                        path = request.Path, codepage = request.Codepage, limits = budget.Limits, password = request.Password,
                        create_name = request.CreateName, compression_level = request.CompressionLevel })
                    : request.Operation == "extract" && argument != "--archive-worker-fixture"
                    ? Pack(new { schema = 2, operation_id = request.OperationId, op = "start", operation = request.Operation,
                        path = request.Path, codepage = request.Codepage, limits = budget.Limits, password = request.Password,
                        extract_options = new { copy_motw = request.ExtractOptions?.CopyMotw ?? true } })
                    : Pack(new { schema = argument == "--archive-worker-fixture" ? 1 : 2, operation_id = request.OperationId, op = "start",
                        operation = request.Operation, path = request.Path, codepage = request.Codepage,
                        limits = budget.Limits, password = request.Password });
                if (request.ExtractOptions?.Reclamation is { } reclamation) {
                    var privateRequest = System.Text.Json.Nodes.JsonNode.Parse(input.AsSpan(4))!;
                    privateRequest["extract_options"]!["reclamation"] = System.Text.Json.Nodes.JsonNode.Parse(JsonSerializer.Serialize(reclamation, Protocol.JsonDefaults.Options));
                    CryptographicOperations.ZeroMemory(input); input = Pack(privateRequest);
                }
                if (request.ConflictPolicy is not null) {
                    var privateRequest = System.Text.Json.Nodes.JsonNode.Parse(input.AsSpan(4))!;
                    privateRequest["conflict_policy"] = request.ConflictPolicy;
                    CryptographicOperations.ZeroMemory(input); input = Pack(privateRequest);
                }
                if (request.OutputReadmission is { } readmission) {
                    var privateRequest = System.Text.Json.Nodes.JsonNode.Parse(input.AsSpan(4))!;
                    privateRequest["output_readmission"] = System.Text.Json.Nodes.JsonNode.Parse(readmission.GetRawText());
                    CryptographicOperations.ZeroMemory(input); input = Pack(privateRequest);
                }
                AddRiskFixture(ref input, request);
                if (request.VolumeBytes is not null || request.JobModify is not null || request.ZipOptions?.SelfExtracting == true) {
                    var privateRequest = System.Text.Json.Nodes.JsonNode.Parse(input.AsSpan(4))!;
                    if (request.VolumeBytes is {} size) privateRequest["volume_bytes"] = size;
                    if (request.JobModify is {} edit) privateRequest["job_modify"] = JsonSerializer.SerializeToNode(edit,Protocol.JsonDefaults.Options);
                    if (request.ZipOptions?.SelfExtracting == true) privateRequest["zip_options"]!["self_extracting"] = true;
                    CryptographicOperations.ZeroMemory(input); input=Pack(privateRequest);
                }
                if (request.DeleteSource is { } consent) {
                    var privateRequest = System.Text.Json.Nodes.JsonNode.Parse(input.AsSpan(4))!;
                    privateRequest["delete_source"] = consent;
                    input = Pack(privateRequest);
                }
                if (request.PasswordInteraction) {
                    var privateRequest = System.Text.Json.Nodes.JsonNode.Parse(input.AsSpan(4))!;
                    privateRequest["password_interaction"] = true;
                    CryptographicOperations.ZeroMemory(input);
                    input = Pack(privateRequest);
                }
                if (request.BrowserEntry is { } selection) {
                    var privateRequest = System.Text.Json.Nodes.JsonNode.Parse(input.AsSpan(4))!;
                    privateRequest["browser_entry"] = JsonSerializer.SerializeToNode(new { ordinal = selection.Ordinal, expected = selection.Browser }, Protocol.JsonDefaults.Options);
                    CryptographicOperations.ZeroMemory(input); input = Pack(privateRequest);
                }
                if (request.DragMaterialization is { } drag) {
                    var privateRequest = System.Text.Json.Nodes.JsonNode.Parse(input.AsSpan(4))!;
                    privateRequest["drag_materialization"] = JsonSerializer.SerializeToNode(new { version = 1, members = drag.Members });
                    CryptographicOperations.ZeroMemory(input); input = Pack(privateRequest);
                }
                if (request.JobExecution) {
                    var privateRequest = System.Text.Json.Nodes.JsonNode.Parse(input.AsSpan(4))!;
                    privateRequest["job_execution"] = true;
                    if (request.JobLargeCount) privateRequest["job_large_count"] = true;
                    if (request.JobExtract is { } jobExtract) privateRequest["job_extract"] = JsonSerializer.SerializeToNode(jobExtract, Protocol.JsonDefaults.Options);
                    CryptographicOperations.ZeroMemory(input); input = Pack(privateRequest);
                }
                if (request.ReadContract is { } contract) {
                    var privateRequest = System.Text.Json.Nodes.JsonNode.Parse(input.AsSpan(4))!;
                    // Bootstrap is byte-compatible with old read workers. Never send v2-only
                    // fields before observing actual accepted capabilities from this process.
                    privateRequest["read_contract"] = request.ReadContract == 2 ? 1 : contract;
                    if (request.ReadContract == 2) privateRequest["limits"] = JsonSerializer.SerializeToNode(
                        new ArchiveAdmittedLimits(4096, 4194304, 67108864, 34359738368, 107374182400));
                    input = Pack(privateRequest);
                }
                if (request.BrowserAuthorize is { } authorizationContext) {
                    var privateRequest = System.Text.Json.Nodes.JsonNode.Parse(input.AsSpan(4))!;
                    privateRequest["browser_authorize"] = JsonSerializer.SerializeToNode(authorizationContext, Protocol.JsonDefaults.Options);
                    CryptographicOperations.ZeroMemory(input); input = Pack(privateRequest);
                }
                if (request.ParallelExtraction && (request.Operation != "extract" || request.BrowserEntry is not null ||
                    request.DragMaterialization is not null || request.ExtractOptions?.Reclamation is not null))
                    throw new ArchiveOutputException("invalid_request");
                if (argument != "--archive-worker-fixture" && BulkOperation(request)) {
                    var privateRequest = System.Text.Json.Nodes.JsonNode.Parse(input.AsSpan(4))!;
                    privateRequest["bulk_transfer"] = 1;
                    if (request.ParallelExtraction) privateRequest["parallel_extraction"] = true;
                    CryptographicOperations.ZeroMemory(input); input = Pack(privateRequest);
                }
                // Insert bytes LAST, never through JsonNode/string or a retained Run.Request field.
                if (browserPassword is not null) AddBrowserPassword(ref input, browserPassword);
            }
            catch (ArchiveBudgetException failure) { reason = failure.Category; }
            catch (ArchiveOutputException failure) { reason = failure.Stage; }
            catch (Exception) { reason = "invalid_request"; }
        }
        IDisposable? helperLease = null;
        if (reason is null && readHelper is not null) {
            try { helperLease = readHelper!.Acquire(); }
            catch (ArchiveOutputException failure) { reason = failure.Stage; rejectionNative = failure.NativeCode; }
        }
        ArchiveSourceActivity.Reservation? sourceReservation = null;
        if (reason is null && readHelper is not null && argument == "--archive-worker" && !IsCreation(request.Operation)) {
            try { sourceReservation = ArchiveSourceLease.ReserveActivity(request.Path, request.ExtractOptions?.Reclamation is not null, request.OperationId, request.ExtractOptions?.Reclamation?.Mode == "continue"); }
            catch (ArchiveOutputException failure) { reason = failure.Stage.StartsWith("reclamation_", StringComparison.Ordinal) ? failure.Stage : request.ExtractOptions?.Reclamation is not null ? "reclamation_source_unavailable" : "source_unavailable"; rejectionNative = failure.NativeCode; }
        }
        Run? run = null;
        lock (gate)
        {
            if (reason is null)
            {
                if (stopped) reason = "stopping";
                else if (active.ContainsKey(request.OperationId) || retired.Contains(request.OperationId)) reason = "duplicate_operation";
                else if (active.Count >= 2) reason = "concurrency_limit";
                else if (active.Values.Sum(r => (long)r.Memory) + memory > AggregateMemory) reason = "aggregate_memory_limit";
                else if (request.ReadContract is not null || request.BrowserEntry is not null || request.BrowserAuthorize is not null || request.JobExecution) {
                    lock (ProductReadGate) {
                        if (ProductReads.Count >= 2 || ProductReads.ContainsKey(request.OperationId) ||
                            ProductReads.Values.Sum(item => (long)item.Memory) + memory > AggregateMemory)
                            reason = "concurrency_limit";
                        else {
                            run = new Run(request, memory, input!) { HelperLease = helperLease, ConfirmCreateOverwrite = confirmCreateOverwrite };
                            ProductReads.Add(request.OperationId, run); active.Add(request.OperationId, run);
                        }
                    }
                }
                else { run = new Run(request, memory, input!) { HelperLease = helperLease }; active.Add(request.OperationId, run); }
            }
        }
        if (run is null) { if (input is not null) CryptographicOperations.ZeroMemory(input); sourceReservation?.Dispose(); helperLease?.Dispose(); Emit(new("rejected", request.OperationId, reason, rejectionNative)); return false; }
        run.SourceReservation = sourceReservation;
        Emit(new("accepted", request.OperationId)); Diagnostic("accepted", run);
        _ = Task.Run(() => ExecuteAsync(run));
        return true;
    }
    // Write only a bounded JSON control record; it is not interpreted as a codec instruction here.
    internal bool Write(Guid id, JsonElement frame)
    {
        byte[] bytes;
        try
        {
            if (frame.ValueKind != JsonValueKind.Object || frame.GetProperty("operation_id").GetGuid() != id ||
                frame.GetProperty("schema").GetInt32() != 1 || frame.GetProperty("op").GetString() != "write") return false;
            bytes = Pack(frame);
        }
        catch (Exception) { return false; }
        lock (gate)
        {
            if (!active.TryGetValue(id, out var run) || run.Failure is not null) return false;
            // Reserve one of four queue slots for cancellation even under sustained writes.
            if (run.InputBytes + bytes.Length > 4 * FrameLimit || run.PendingWrites >= 2 || !run.Input.Writer.TryWrite(bytes))
            { run.Fail("input_limit"); return false; }
            run.PendingWrites++; run.InputBytes += bytes.Length; return true;
        }
    }
    internal bool SetPaused(Guid id, bool paused) {
        lock (gate) {
            if (!active.TryGetValue(id,out var run) || !run.Request.JobExecution || run.Failure is not null || !run.Negotiated) return false;
            if (run.Paused == paused) return true;
            if (run.PendingWrites >= 2 || !run.Input.Writer.TryWrite(Pack(new {version=1,operation_id=id,op=paused ? "pause" : "resume"}))) return false;
            run.PendingWrites++; run.SetHold(paused,run.PromptWaiting); return true;
        }
    }
    internal void SetPromptWaiting(Guid id,bool waiting) { lock(gate) { if(active.TryGetValue(id,out var run)) run.SetHold(run.Paused,waiting); } }
    internal bool Cancel(Guid id)
    {
        lock (gate)
        {
            if (!active.TryGetValue(id, out var run)) return false;
            if (run.Recycling) run.RecycleCancellation.Cancel(); else run.Fail("cancelled"); return true;
        }
    }
    internal void Stop(string reason = "disconnected")
    {
        lock (gate) { stopped = true; foreach (var run in active.Values) run.Fail(reason); }
    }
    public void Dispose() => Stop(); // Worker keeps leases until actual process and pipe drainage.
    internal async Task<bool> DrainAsync(int milliseconds = 10000)
    {
        var clock = Stopwatch.StartNew();
        while (ActiveCount != 0 && clock.ElapsedMilliseconds < milliseconds) await Task.Delay(20);
        return ActiveCount == 0;
    }
    private sealed partial class Run
    {
        internal readonly ArchiveWorkerRequest Request;
        internal readonly int Memory;
        internal readonly Stopwatch Clock = Stopwatch.StartNew();
        internal readonly Channel<byte[]> Input = Channel.CreateBounded<byte[]>(new BoundedChannelOptions(4)
            { FullMode = BoundedChannelFullMode.Wait, SingleReader = true });
        internal readonly List<IDisposable> Resources = [];
        internal IDisposable? HelperLease;
        internal Func<(string Signature, long Size), CancellationToken, Task<bool>>? ConfirmCreateOverwrite;
        internal bool CleanupFailed;
        internal readonly CancellationTokenSource InputStop = new();
        internal readonly CancellationTokenSource MetadataStop = new();
        internal int InputBytes, OutputBytes, StderrBytes, PendingWrites;
        internal long TransportStdinBytes, TransportStdoutBytes;
        internal bool SawTerminal;
        internal bool Negotiated;
        internal ulong FilesSkipped;
        internal bool Paused, PromptWaiting;
        internal bool DestinationChecked;
        private readonly object timingGate=new();
        internal long ExcludedMs, HoldStartedMs;
        internal IReadOnlyList<Protocol.ArchiveJobCommit>? Journal;
        internal Protocol.ArchiveJobJournalSummary? JournalSummary;
        internal ulong BytesCommitted;
        internal Protocol.ArchiveJobResultLocation? ResultLocation;
        internal bool? OutputPublished;
        internal Protocol.ArchiveJobAddResult? AddResult;
        internal long ActiveElapsedMs { get {lock(timingGate){var elapsed=Clock.ElapsedMilliseconds;return elapsed-ExcludedMs-(Paused || PromptWaiting?elapsed-HoldStartedMs:0);}} }
        internal void SetHold(bool paused, bool prompt) {
            lock(timingGate) {
                var held=Paused || PromptWaiting;
                var next=paused || prompt;
                if (!held && next) HoldStartedMs=Clock.ElapsedMilliseconds;
                if (held && !next) ExcludedMs += Clock.ElapsedMilliseconds-HoldStartedMs;
                Paused=paused; PromptWaiting=prompt;
            }
        }
        internal ArchiveSourceActivity.Reservation? SourceReservation;
        internal ArchiveFault? Failure;
        internal ArchiveWorkerOutputBridge? Output;
        internal ArchiveSourceLease? Source;
        internal ArchiveCreationSources? CreationSources;
        internal ArchiveVolumeSources? VolumeSources;
        internal string? EditTargetIdentity;
        internal JsonElement? DeleteEvidence;
        internal RecycleSession? DeleteSession;
        internal ArchiveSourceDeletionResult? SourceDeletion;
        internal ArchiveWorkerEvent? ArchiveTerminal;
        internal volatile bool Recycling;
        internal readonly CancellationTokenSource RecycleCancellation = new();
        internal Run(ArchiveWorkerRequest request, int memory, byte[] first)
        {
            Request = request; Memory = memory; InputBytes = first.Length;
            using var start = JsonDocument.Parse(first.AsMemory(4));
            BulkRequested = start.RootElement.TryGetProperty("bulk_transfer", out _);
            Input.Writer.TryWrite(first); InitializeVirtual();
        }
        internal void Fail(string reason, int? native = null, bool operationFailure = false) {
            if (Recycling) { RecycleCancellation.Cancel(); return; }
            var failure = new ArchiveFault(reason, native);
            if (operationFailure) RecordOperationFailure(failure);
            else Interlocked.CompareExchange(ref Failure, failure, null);
            if (Request.ReadContract is not null || Request.JobExecution) MetadataStop.Cancel();
            Output?.RequestCancellation();
        }
        private void RecordOperationFailure(ArchiveFault failure) {
            while (true) {
                var current = Volatile.Read(ref Failure);
                if (current is not null && !(Request.ParallelExtraction && current.Category == "cancelled" && failure.Category != "cancelled")) return;
                if (ReferenceEquals(Interlocked.CompareExchange(ref Failure, failure, current), current)) return;
            }
        }
        internal void FailNative(ArchiveNativeOutputFailure failure) {
            if (failure.OperationId != Request.OperationId) throw new ArchiveFault("stdout_invalid_frame");
            if (Request.ParallelExtraction && failure.Category != "target_exists") {
                RecordOperationFailure(new ArchiveFault(failure.Category, failure.NativeCode, failure));
                Output?.RequestCancellation(); return;
            }
            if (!(IsModification(Request.Operation) && failure.Action == "commit") && failure.Category is not ("write_file" or "flush_file" or "flush_or_commit" or "io_failure") && !failure.Category.StartsWith("reclamation_", StringComparison.Ordinal) && !failure.Category.StartsWith("metadata_", StringComparison.Ordinal) && !failure.Category.StartsWith("motw_", StringComparison.Ordinal) && failure.Category is not ("target_changed" or "target_missing" or "target_directory" or
                "target_reparse" or "target_busy" or "target_denied" or "overwrite_not_supported" or "selected_output_limit" or
                "commit_outcome_unknown" or "committed_but_verification_failed" or "target_changed_after_commit" or "cancelled")) return;
            // Fail-stop on this exact acknowledged call, never recover from a mutable last error.
            Interlocked.CompareExchange(ref Failure, new ArchiveFault(failure.Category, failure.NativeCode, failure), null);
            Output?.RequestCancellation();
        }
    }
    private sealed class ArchiveFault(string reason, int? native = null, ArchiveNativeOutputFailure? outputFailure = null) : Exception
    { internal string Category => reason; internal int? NativeCode => native; internal ArchiveNativeOutputFailure? OutputFailure => outputFailure; }
    private static byte[] Pack(object frame)
    {
        var bytes = JsonSerializer.SerializeToUtf8Bytes(frame);
        if (bytes.Length is < 1 or > FrameLimit) throw new ArchiveFault("input_frame_limit");
        var result = new byte[4 + bytes.Length];
        BinaryPrimitives.WriteUInt32LittleEndian(result, (uint)bytes.Length);
        bytes.CopyTo(result.AsSpan(4)); CryptographicOperations.ZeroMemory(bytes); return result;
    }
    private async Task ExecuteAsync(Run run)
    {
        ExtensionKernelHandle? job = null, process = null;
        FileStream? input = null, output = null, error = null;
        Task outPump = Task.CompletedTask, errPump = Task.CompletedTask, inPump = Task.CompletedTask;
        var assigned = false; var drained = false; uint? exit = null;
        try
        {
            if (run.Failure is not null) throw run.Failure;
            if (run.Request.Operation == "extract" || IsModification(run.Request.Operation) || IsCreation(run.Request.Operation)) {
                ArchiveOutputSession? prepared = null;
                var outputRequest = run.Request;
                if (run.Request.DragMaterialization is { } drag) {
                    var limits = (ArchiveAdmittedLimits)run.Request.Limits;
                    outputRequest = outputRequest with { Limits = limits with { max_entry_bytes = Math.Min(limits.max_entry_bytes, drag.First.OutputLimit), max_output_bytes = drag.First.OutputLimit } };
                }
                if (run.ConfirmCreateOverwrite is { } confirm) {
                    // Original global/max2 Run already owns this blocking preflight and every
                    // handle until actual cleanup. Never run it on the serial command reader.
                    CreateOverwritePreflightProbe?.Invoke();
                    if (run.Failure is not null) throw run.Failure;
                    prepared = ArchiveOutputSession.Prepare(run.Request.OutputRoot!, run.Request.OperationId, log);
                    run.Resources.Add(prepared);
                    var observed = prepared.ObservePreparedCreateTarget(run.Request.CreateName!, run.MetadataStop.Token);
                    if (run.Failure is not null) throw run.Failure;
                    if (observed is { } target) {
                        if (!await confirm(target, run.MetadataStop.Token)) throw new ArchiveFault("cancelled");
                        outputRequest = run.Request with { ConfirmedOverwrites = new Dictionary<string, string>(StringComparer.Ordinal) { [run.Request.CreateName!] = target.Signature } };
                    }
                    if (run.Failure is not null) throw run.Failure;
                }
                run.Output = new ArchiveWorkerOutputBridge(outputRequest, log, prepared, run.BulkRequested);
                if (run.Request.VolumeBytes is not null || run.Request.ZipOptions?.SelfExtracting == true) run.Output.ConfigureVolumeOutput(run.Request);
                if (prepared is not null) run.Resources.Remove(prepared); // Bridge now owns the SAME held namespace.
                run.Output.CommitCheckpointProbe = stage => OutputCommitCheckpointProbe?.Invoke(stage);
                run.Output.ParallelOrdinary = run.Request.ParallelExtraction;
                run.Output.BeginCheckpointProbe = OutputBeginCheckpointProbe;
                run.Output.WriteCheckpointProbe = OutputWriteCheckpointProbe;
                run.Output.CreationOutput.NamedStreamsFlagsProbe = NamedStreamsFlagsProbe;
                run.Resources.Add(run.Output);
            }
            job = ExtensionNative.MakeJob(run.Memory, 1);
            run.Resources.Add(job);
            var stdin = ExtensionNative.Pipe(false); using var childInput = stdin.Child;
            run.Resources.Add(stdin.Ours);
            var stdout = ExtensionNative.Pipe(true); using var childOutput = stdout.Child;
            run.Resources.Add(stdout.Ours);
            var stderr = ExtensionNative.Pipe(true); using var childError = stderr.Child;
            run.Resources.Add(stderr.Ours);
            var child = ExtensionNative.Spawn(executable, [argument], directory, childInput, childOutput, childError);
            process = child.Process; run.Resources.Add(process);
            using var thread = child.Thread;
            ExtensionNative.Require(ExtensionNative.AssignJob(job, process.DangerousGetHandle()), "assign_job");
            assigned = true;
            admissionProbe?.Invoke(process);
            CheckWorkTimeout(run);
            if (run.Failure is not null) throw run.Failure;
            // The unique input pump sends complete envelopes; small control frames must not remain buffered.
            input = new FileStream(stdin.Ours, FileAccess.Write, 1, false); run.Resources.Add(input);
            output = new FileStream(stdout.Ours, FileAccess.Read, 8192, false); run.Resources.Add(output);
            error = new FileStream(stderr.Ours, FileAccess.Read, 8192, false); run.Resources.Add(error);
            if (ExtensionNative.Resume(thread) == uint.MaxValue)
                throw new ArchiveFault("resume_thread", Marshal.GetLastWin32Error());
            childInput.Dispose(); childOutput.Dispose(); childError.Dispose();
            Emit(new("started", run.Request.OperationId, ElapsedMs: run.Clock.ElapsedMilliseconds));
            Diagnostic("started", run);
            outPump = Task.Run(() => ReadFrames(run, output));
            errPump = Task.Run(() => DrainStderr(run, error));
            inPump = Task.Run(() => SendInput(run, input));
            while (run.Failure is null)
            {
                var wait = ExtensionNative.Wait(process, 0);
                if (wait == 0) break;
                if (wait == uint.MaxValue) throw new ArchiveFault("wait_process", Marshal.GetLastWin32Error());
                CheckWorkTimeout(run);
                await Task.Delay(20);
            }
            if (run.Failure is not null && ExtensionNative.Wait(process, 0) != 0)
            {
                // One queue slot is reserved for this cooperative request; no path or secret argv.
                run.Input.Writer.TryWrite(Pack(new { version = 1, operation_id = run.Request.OperationId, op = "cancel" }));
                var grace = Stopwatch.StartNew();
                while (grace.ElapsedMilliseconds < 200 && ExtensionNative.Wait(process, 0) != 0)
                    await Task.Delay(10);
            }
        }
        catch (ArchiveFault e) { run.Fail(e.Category, e.NativeCode); }
        catch (ArchiveOutputException e) { run.Fail(e.Stage, e.NativeCode); }
        catch (OperationCanceledException) when (run.MetadataStop.IsCancellationRequested) { run.Fail("cancelled"); }
        catch (ExtensionFailure e) { run.Fail(e.Reason, e.NativeCode); }
        catch (Exception) { run.Fail("runner_failure"); }
        finally
        {
            run.MetadataStop.Cancel();
            run.Input.Writer.TryComplete(); run.InputStop.Cancel();
            try
            {
                // Assignment failure still owns a suspended process, which must be killed explicitly.
                if (job is not null && !ExtensionNative.TerminateJob(job, 1))
                    run.Fail("terminate_job", Marshal.GetLastWin32Error());
                if (!assigned && process is not null && ExtensionNative.Wait(process, 0) != 0 &&
                    !ExtensionNative.TerminateProcess(process, 1))
                    run.Fail("terminate_process", Marshal.GetLastWin32Error());
                var cleanup = Stopwatch.StartNew(); var pumps = Task.WhenAll(outPump, errPump, inPump);
                while (cleanup.ElapsedMilliseconds < 5000)
                {
                    if ((job is null || ExtensionNative.JobEmpty(job)) &&
                        (process is null || ExtensionNative.Wait(process, 0) == 0) && pumps.IsCompleted)
                    { await pumps; drained = true; break; }
                    await Task.Delay(20);
                }
                if (drained && process is not null)
                {
                    ExtensionNative.Require(ExtensionNative.ExitCode(process, out var code), "exit_code");
                    exit = code; if (code != 0) run.Fail("process_exit");
                }
            }
            catch (ExtensionFailure e) { run.Fail(e.Reason, e.NativeCode); }
            catch (Exception) { run.Fail("cleanup_failure"); }
            if (drained)
            {
                CleanupCheckpointProbe?.Invoke();
                run.SourceDeletion = await ResolveSourceDeletionAsync(run, exit);
                run.DeleteSession?.Retire();
                if (run.Request.JobExecution && run.Request.Operation is "extract" or "create_zip" or "create_7z" or "create_tar" or "create_tgz" or "create_txz" or "create_gzip" or "create_xz") {
                    try {
                        var preferred = run.Request.Operation == "extract" ? null : run.Request.Operation == "create_7z" && run.Request.VolumeBytes is not null
                            ? run.Request.CreateName + ".001" : run.Request.ZipOptions?.SelfExtracting == true && run.Request.VolumeBytes is not null
                            ? Path.GetFileNameWithoutExtension(run.Request.CreateName) + ".zip" : run.Request.CreateName;
                        run.ResultLocation = run.Output?.CreationOutput.ResultLocation(preferred);
                    }
                    catch (Exception) { run.ResultLocation = null; } // Presentation failure never changes publication.
                }
                foreach (var resource in run.Resources)
                {
                    try { resource.Dispose(); }
                    catch (Exception) { run.Fail("dispose_failure"); if (run.Request.ReadContract is not null || run.Request.BrowserEntry is not null || run.Request.BrowserAuthorize is not null || run.Request.JobExecution) run.CleanupFailed = true; }
                }
                if (!run.CleanupFailed) run.SourceReservation?.Dispose();
                run.Journal = run.Output?.CreationOutput.CommitJournal(run.Request.JobLargeCount ? 256 : int.MaxValue);
                run.JournalSummary = run.Request.JobLargeCount ? run.Output?.CreationOutput.CommitSummary() : null;
                run.BytesCommitted = run.Output?.BytesCommitted ?? 0;
                run.OutputPublished = run.Request.DragMaterialization is null ? run.Output is null ? false : run.Output.CreationOutput.OutputPublished()
                    : run.VirtualOutputs.Any(member => member.CreationOutput.OutputPublished() == true);
                run.InputStop.Dispose();
                if (!run.CleanupFailed) {
                    try { run.HelperLease?.Dispose(); }
                    catch (Exception) { run.Fail("dispose_failure"); run.CleanupFailed = true; }
                }
                run.RecycleCancellation.Dispose();
                lock (gate)
                {
                    if (!run.CleanupFailed) {
                        active.Remove(run.Request.OperationId);
                        if (run.Request.ReadContract is not null || run.Request.BrowserEntry is not null || run.Request.BrowserAuthorize is not null || run.Request.JobExecution) lock (ProductReadGate) ProductReads.Remove(run.Request.OperationId);
                    }
                    retired.Add(run.Request.OperationId); recent.Enqueue(run.Request.OperationId);
                    if (recent.Count > 4096) retired.Remove(recent.Dequeue());
                }
                if (!run.CleanupFailed) run.MetadataStop.Dispose();
            }
            else run.Fail("cleanup_stuck"); // Leases remain rooted in the occupied slot.
            var terminal = ResolveTerminal(run, drained && !run.CleanupFailed, exit);
            DiagnosticTerminal(run, terminal);
            Emit(terminal);
        }
    }
    private static ArchiveWorkerEvent ResolveTerminal(Run run, bool drained, uint? exit)
    {
        var failure = run.Request.ParallelExtraction ? run.Failure : run.OutputWriteFailure is { } firstWrite
            ? new ArchiveFault(firstWrite.Category, firstWrite.NativeCode, firstWrite) : run.Failure;
        if (run.ArchiveTerminal is { } frozen) return frozen with { CleanupComplete = drained, SourceDeletion = run.SourceDeletion, ElapsedMs = run.Clock.ElapsedMilliseconds, OutputPublished = run.OutputPublished };
        var files = run.PublishedFiles;
        var state = run.Output?.LastCommitState;
        // Publication/uncertainty outranks cancellation. Ordinary failures use only their own call.
        var authoritativeCommit = state is ArchiveCommitState.CommitOutcomeUnknown or ArchiveCommitState.CommitInFlight or
            ArchiveCommitState.TargetChangedAfterCommit or ArchiveCommitState.CommittedButVerificationFailed;
        if (!authoritativeCommit && failure?.OutputFailure is { } nativeFailure) state = nativeFailure.State;
        var category = state is ArchiveCommitState.CommitOutcomeUnknown or ArchiveCommitState.CommitInFlight ? "commit_outcome_unknown"
            : state == ArchiveCommitState.TargetChangedAfterCommit ? "target_changed_after_commit"
            : state == ArchiveCommitState.CommittedButVerificationFailed ? "committed_but_verification_failed"
            : failure?.Category == "cancelled" && files > 0 ? "cancelled_with_committed_output" : failure?.Category ?? (run.FilesSkipped > 0 ? "succeeded_with_warnings" : "success");
        return new("exited", run.Request.OperationId, category,
            authoritativeCommit ? run.Output?.LastCommitNativeCode : failure?.NativeCode,
            run.Clock.ElapsedMilliseconds, ExitCode: exit, CleanupComplete: drained,
            FilesCommitted: files, CommitState: state?.ToString(), SourceDeletion: run.SourceDeletion,
            PrivateBytes: run.Request.ReadContract == 2 ? (ulong)run.OutputBytes : null,
            ProcessBudgetBytes: run.Request.ReadContract == 2 ? (ulong)run.Memory : null, FilesSkipped: run.FilesSkipped,
            Journal: run.Journal, BytesCommitted: run.PublishedBytes, ResultLocation:run.ResultLocation, OutputPublished:run.OutputPublished,
            AddResult: drained && (category is "success" or "succeeded_with_warnings") && state == ArchiveCommitState.Committed && run.OutputPublished == true && files > 0 && run.SawTerminal ? run.AddResult : null,
            HasNativeFailure: failure?.OutputFailure is not null, JournalSummary: run.JournalSummary);
    }
    private async Task ReadFrames(Run run, Stream stream)
    {
        var header = new byte[4];
        try
        {
            while (true)
            {
                // CreatePipe supplies synchronous handles; this independent pump owns their blocking reads.
                var count = stream.Read(header.AsSpan(0, 4));
                if (count == 0)
                {
                    await JoinNative(run);
                    if (!run.SawTerminal) run.Fail("stdout_missing_terminal");
                    return;
                }
                if (count < 4) stream.ReadExactly(header.AsSpan(count, 4 - count));
                var size = BinaryPrimitives.ReadUInt32LittleEndian(header);
                if ((size & ArchiveBulkFrame.Discriminator) != 0) {
                    await ReadBulkWrite(run, stream, size & ~ArchiveBulkFrame.Discriminator);
                    continue;
                }
                if (size is 0 or > FrameLimit) throw new ArchiveFault("stdout_frame_limit");
                var body = new byte[size];
                try {
                stream.ReadExactly(body);
                Interlocked.Add(ref run.TransportStdoutBytes, size + 4L);
                Interlocked.Increment(ref run.StdoutJsonFrames);
                using var doc = JsonDocument.Parse(body, new JsonDocumentOptions { MaxDepth = 32 });
                var root = doc.RootElement;
                var cumulativeLimit = run.Request.ReadContract == 2 && run.Negotiated
                    ? (long)run.Request.ReadTransport!.MaxTotalBytes : 8L * FrameLimit;
                if ((run.Output is null || root.GetProperty("op").GetString() != "native_call") &&
                    Interlocked.Add(ref run.OutputBytes, checked((int)size + 4)) > cumulativeLimit) throw new ArchiveFault("stdout_total_limit");
                if (root.GetProperty("schema").GetInt32() != 1 ||
                    root.GetProperty("operation_id").GetGuid() != run.Request.OperationId ||
                    root.GetProperty("op").GetString() is not ("accepted" or "progress" or "entry_batch" or "complete" or "failure" or "native_call" or "password_state" or "conflict_prompt" or "extraction_summary" or "risk_prompt" or "risk_confirmed" or "output_budget_shortfall" or "browser_context" or "browser_selected" or "add_result" or "drag_member_ready"))
                    throw new ArchiveFault("stdout_invalid_frame");
                if (run.SawTerminal) throw new ArchiveFault("stdout_invalid_frame");
                if (run.BulkRequested && !run.Negotiated && root.GetProperty("op").GetString() != "accepted")
                    throw new ArchiveFault("worker_capability_mismatch");
                if (root.GetProperty("op").GetString() == "accepted" && run.Request.ReadContract is not null) {
                    if (!root.TryGetProperty("read_contract", out var contract) || contract.ValueKind != JsonValueKind.Number || contract.GetInt32() != 1)
                        throw new ArchiveFault("incompatible_version");
                    if (!root.TryGetProperty("capabilities", out var readCaps) || readCaps.ValueKind != JsonValueKind.Array ||
                        readCaps.GetArrayLength() > 32 || readCaps.EnumerateArray().Any(c => c.ValueKind != JsonValueKind.String) ||
                        Protocol.ArchiveReadWire.RequiredCapabilities.Any(c => !readCaps.EnumerateArray().Any(v => v.GetString() == c)))
                        throw new ArchiveFault("missing_capability");
                    if (run.Request.ReadContract == 2) {
                        if (!readCaps.EnumerateArray().Any(v => v.GetString() == "product_read_v2")) throw new ArchiveFault("incompatible_version");
                        if (run.Request.ReadTransport!.BrowserContext && !readCaps.EnumerateArray().Any(v => v.GetString() == "browser_entry_v1")) throw new ArchiveFault("missing_capability");
                        if (Protocol.ArchiveReadWire.LargeCapabilities.Any(c => !readCaps.EnumerateArray().Any(v => v.GetString() == c)))
                            throw new ArchiveFault("missing_capability");
                        var transport = new System.Text.Json.Nodes.JsonObject { ["max_total_bytes"] = run.Request.ReadTransport!.MaxTotalBytes };
                        if (run.Request.ReadTransport.BrowserContext) transport["browser_context"] = true;
                        if (!run.Input.Writer.TryWrite(Pack(new { version = 1, operation_id = run.Request.OperationId, op = "read_upgrade",
                            upgrade = new { limits = run.Request.Limits, transport } }))) throw new ArchiveFault("input_limit");
                    }
                }
                if (root.GetProperty("op").GetString() == "accepted" && argument != "--archive-worker-fixture") {
                    if (run.Negotiated || !root.TryGetProperty("capabilities", out var capabilities) ||
                        !SupportsCapabilities(run.Request.Operation, capabilities, run.Request.ParallelExtraction ? 32 : 24))
                        throw new ArchiveFault(run.Request.ReadContract is not null ? "missing_capability" : "worker_capability_mismatch");
                    if (run.BulkRequested && !HasBulkCapability(capabilities)) throw new ArchiveFault("worker_capability_mismatch");
                    if (run.Request.ParallelExtraction && !capabilities.EnumerateArray().Any(item => item.GetString() == "parallel_extraction_v1"))
                        throw new ArchiveFault("worker_capability_mismatch");
                    if (run.BulkRequested && run.Output?.PipelinedExtraction == true && !capabilities.EnumerateArray().Any(item => item.GetString() == "extraction_output_v1"))
                        throw new ArchiveFault("worker_capability_mismatch");
                    run.Negotiated = true;
                    if (run.Request.JobExecution && !capabilities.EnumerateArray().Any(item=>item.GetString()=="product_job_v1"))throw new ArchiveFault("missing_capability");
                    if (run.Request.JobLargeCount && !capabilities.EnumerateArray().Any(item=>item.GetString()=="job_large_count_v1"))throw new ArchiveFault("missing_capability");
                    if (run.Request.BrowserEntry is not null && !capabilities.EnumerateArray().Any(item => item.GetString() == "browser_entry_v1")) throw new ArchiveFault("missing_capability");
                    if (run.Request.DragMaterialization is not null) {
                        if (!capabilities.EnumerateArray().Any(item => item.GetString() == "drag_materialization_v1")) throw new ArchiveFault("missing_capability");
                        lock (gate) { if (!SendVirtualDemand(run)) throw new ArchiveFault("input_limit"); }
                    }
                    if (run.Request.BrowserAuthorize is not null && !capabilities.EnumerateArray().Any(item => item.GetString() == "browser_authorize_v1")) throw new ArchiveFault("missing_capability");
                    if (run.Request.DeleteSource is not null && !capabilities.EnumerateArray().Any(item => item.GetString() == "source_delete_eligibility_v1")) throw new ArchiveFault("worker_capability_mismatch");
                    if (run.Request.PasswordInteraction && !capabilities.EnumerateArray().Any(item => item.GetString() == "password_interaction_v1")) throw new ArchiveFault("worker_capability_mismatch");
                    if (run.Request.PasswordInteraction && run.Request.Operation == "extract" && !capabilities.EnumerateArray().Any(item => item.GetString() == "password_extract_v1")) throw new ArchiveFault("worker_capability_mismatch");
                    if (run.Request.ConflictPolicy is not null && !capabilities.EnumerateArray().Any(item => item.GetString() == "extraction_conflict_v1")) throw new ArchiveFault("worker_capability_mismatch");
                    if (run.Request.Operation == "extract" && !capabilities.EnumerateArray().Any(item => item.GetString() == "zip_risk_v1")) throw new ArchiveFault("worker_capability_mismatch");
                    if (run.Request.OutputReadmission is not null && !capabilities.EnumerateArray().Any(item => item.GetString() == "zip_output_readmission_v1")) throw new ArchiveFault("worker_capability_mismatch");
                    if (run.Request.VolumeBytes is not null && !capabilities.EnumerateArray().Any(item=>item.GetString()=="volume_create_v1"))throw new ArchiveFault("missing_capability");
                    if (run.Request.ZipOptions?.SelfExtracting == true && !capabilities.EnumerateArray().Any(item=>item.GetString()=="zip_sfx_v1"))throw new ArchiveFault("missing_capability");
                    if (run.Request.ReadContract == 2 && (ulong)run.OutputBytes > run.Request.ReadTransport!.MaxTotalBytes)
                        throw new ArchiveFault("stdout_total_limit");
                }
                if (root.GetProperty("op").GetString() == "drag_member_ready") { ReadyVirtualMember(run, root); continue; }
                if (root.GetProperty("op").GetString() == "password_state") {
                    if (!run.Request.PasswordInteraction || !run.Negotiated) throw new ArchiveFault("stdout_invalid_frame");
                    DiagnosticPassword(run, root.GetProperty("password_state"));
                }
                if (root.GetProperty("op").GetString() is "conflict_prompt" or "extraction_summary") {
                    if (run.Request.ConflictPolicy is null || !run.Negotiated) throw new ArchiveFault("stdout_invalid_frame");
                    if (root.GetProperty("op").GetString() == "extraction_summary") run.FilesSkipped = root.GetProperty("extraction_summary").GetProperty("files_skipped").GetUInt64();
                    Diagnostic(root.GetProperty("op").GetString() == "conflict_prompt" ? "awaiting_conflict" : "conflict_finished", run);
                }
                if (root.GetProperty("op").GetString() == "add_result") {
                    if (!run.Request.JobExecution || !IsModification(run.Request.Operation) || run.Request.JobModify?.Action != "add" ||
                        !run.Negotiated || run.AddResult is not null || run.Output?.FilesCommitted != 1 || run.Failure is not null)
                        throw new ArchiveFault("stdout_invalid_frame");
                    var result = root.GetProperty("add_result").Deserialize<Protocol.ArchiveJobAddResult>(Protocol.ArchiveJobWire.Strict);
                    if (result?.Valid != true) throw new ArchiveFault("stdout_invalid_frame");
                    run.AddResult = result;
                }
                if (root.GetProperty("op").GetString() is "risk_prompt" or "risk_confirmed") {
                    if (run.Request.Operation != "extract" || !run.Negotiated) throw new ArchiveFault("stdout_invalid_frame");
                    Diagnostic(root.GetProperty("op").GetString() == "risk_prompt" ? "awaiting_risk" : "risk_confirmed", run);
                }
                if (root.GetProperty("op").GetString() == "output_budget_shortfall") {
                    if (run.Request.OutputReadmission is null || !run.Negotiated) throw new ArchiveFault("stdout_invalid_frame");
                    Diagnostic("output_budget_shortfall", run);
                }
                if (root.GetProperty("op").GetString() == "native_call") {
                    var nativeCall = root.GetProperty("native");
                    var action = nativeCall.GetProperty("action").GetString();
                    if (run.BulkRequested && action == "write") throw new ArchiveFault("stdout_invalid_frame");
                    if (run.BulkRequested && action is ("source_read" or "read")) {
                        if (action == "source_read" && run.CreationSources is null) throw new ArchiveFault("source_lease_required");
                        await ReadBulkRequest(run, nativeCall, action == "source_read");
                        continue;
                    }
                    if (!run.BulkRequested && action == "write") run.PayloadConversionInputBytes += body.Length;
                    if (action == "begin" && run.Request.BrowserEntry is not null) Emit(new("browser_output", run.Request.OperationId, Frame: nativeCall.Clone()));
                    if (action == "source_acquire") {
                        if (!run.Negotiated || run.Source is not null || run.Output?.HasReclamation == true || run.CreationSources is not null || run.Failure is not null) throw new ArchiveFault("stdout_invalid_frame");
                        object sourceReply;
                        try {
                            if (IsTreeCreation(run.Request.Operation)) {
                                var limits = (ArchiveAdmittedLimits)run.Request.Limits;
                                var sourceOutputRoot = SourceOutputNamespacePathProbe?.Invoke(run.Request.OutputRoot!) ?? run.Request.OutputRoot!;
                                var heldOutput = run.Output?.CreationOutput ?? throw new ArchiveFault("source_lease_required");
                                run.CreationSources = ArchiveCreationSources.Open(run.Request.SourcePaths ?? [run.Request.Path], sourceOutputRoot, run.Request.CreateName!,
                                    limits.max_entries, limits.max_metadata_bytes, limits.max_entry_bytes, limits.max_output_bytes, heldOutput);
                                run.CreationSources.NamespacePathProbe = SourceNamespacePathProbe;
                                run.Resources.Add(run.CreationSources);
                                if(run.Request.JobExecution)await CheckDestination(run,null);
                                if (run.Request.VolumeBytes is not null || run.Request.ZipOptions?.SelfExtracting == true) heldOutput.RequireNewVolumeFamily(run.Request.CreateName!,run.Request.Operation=="create_7z",run.Request.ZipOptions?.SelfExtracting==true);
                                heldOutput.CreatePreparedRoot(() => {
                                    SourceCheckpointProbe?.Invoke("creation_output_preparing");
                                    if (run.Failure is not null) throw new ArchiveOutputException(run.Failure.Category);
                                    run.CreationSources.RevalidateRoots();
                                });
                                run.CreationSources.RejectOutputCollisions(sourceOutputRoot, run.Request.CreateName!, heldOutput);
                            } else if (run.Request.ExtractOptions?.Reclamation is { } reclaim) {
                                if (reclaim.Mode == "start") await CheckDestination(run, null);
                                run.Output!.AcquireReclamation(run.Request, run.SourceReservation?.Identity);
                            } else {
                                if (IsModification(run.Request.Operation)) {
                                    run.Source = ArchiveSourceLease.OpenForEdit(run.Request.Path, run.Output!.OperationCancellation);
                                    run.Resources.Add(run.Source);
                                    run.Source.ReadObservation.ReadChunkProbeForFixture = EditSourceReadChunkProbeForFixture;
                                    if (run.Request.JobModify!.Browser.Source.Native != run.Source.ReadmissionIdentity()) throw new ArchiveOutputException("source_changed");
                                    run.EditTargetIdentity = run.Output!.CreationOutput.CaptureTargetIdentity(Path.GetFileName(run.Request.Path), run.Output.OperationCancellation);
                                    run.Output.BindEditSource(run.Source,run.EditTargetIdentity);
                                    if (run.Request.SourcePaths is {Count:>0} additions) {
                                        var limits=(ArchiveAdmittedLimits)run.Request.Limits;
                                        run.CreationSources=ArchiveCreationSources.Open(additions,run.Request.OutputRoot!,Path.GetFileName(run.Request.Path),limits.max_entries,limits.max_metadata_bytes,limits.max_entry_bytes,limits.max_output_bytes,run.Output.CreationOutput);
                                        run.Resources.Add(run.CreationSources);
                                        if(run.Request.JobModify.Action=="replace_content")run.CreationSources.RequireSingleRegularFile();
                                    }
                                } else {
                                    var limits=(ArchiveAdmittedLimits)run.Request.Limits;
                                    // Selected-member output can be narrowed without shrinking source admission.
                                    var sourceLimit=run.Request.BrowserEntry is null?limits.max_output_bytes:Protocol.ArchiveEntrySession.MaterializeOutputLimit;
                                    run.VolumeSources=ArchiveVolumeSources.Open(run.Request.Path,limits.max_metadata_bytes,sourceLimit,
                                        sevenzOutputPreflight:run.Request.Operation=="extract" && run.Request.OutputReadmission is not null,
                                        rarOutputPreflight:run.Request.Operation=="extract" && run.Request.OutputReadmission is not null,
                                        tarStreamOutputPreflight:run.Request.Operation=="extract" && run.Request.OutputReadmission is not null,
                                        zipOutputPreflight:run.Request.Operation=="extract" && (run.Request.OutputReadmission is not null || run.Request.JobExtract?.Selection is not null),
                                        alzegOutputPreflight:run.Request.Operation=="extract" && (run.Request.OutputReadmission is not null || run.Request.JobExtract?.Selection is not null));
                                    run.Source=run.VolumeSources.Primary;run.Resources.Add(run.VolumeSources);
                                }
                                if (run.SourceReservation is { } reservation && (run.VolumeSources?.Requested ?? run.Source).ActivityIdentity != reservation.Identity) throw new ArchiveOutputException("source_changed");
                                var originalIdentity=run.VolumeSources?.Identity() ?? run.Source.ReadmissionIdentity();
                                if (run.Request.JobExtract?.Selection is { } selected && selected.Browser.Source.Native != originalIdentity) throw new ArchiveOutputException("source_changed");
                                if (run.Request.BrowserEntry is { } browser && browser.Browser!.Source.Native != originalIdentity) throw new ArchiveOutputException("source_changed");
                                if (run.Request.BrowserAuthorize is { } authorization && authorization.Source.Native != originalIdentity) throw new ArchiveOutputException("source_changed");
                                if (run.Request.OutputReadmission is { } binding && binding.TryGetProperty("expected_source", out var expected) && expected.ValueKind != JsonValueKind.Null && expected.GetProperty("native").GetString() != originalIdentity)
                                    throw new ArchiveOutputException("source_changed");
                                if (run.Request.Operation == "extract" && (run.Request.ExtractOptions?.CopyMotw ?? true)) {
                                    var mark = run.Source.CaptureZoneIdentifier(NamedStreamsFlagsProbe);
                                    run.Output!.BindZoneIdentifier(mark);
                                    run.VirtualZone = mark;
                                    Diagnostic(mark is null ? "motw_absent" : "motw_captured", run);
                                } else if (run.Request.Operation == "extract") {
                                    Diagnostic("motw_disabled", run);
                                }
                            }
                            if (run.Request.DeleteSource == true) {
                                var originalSources = run.CreationSources is { } tree ? tree.RecycleSources() :
                                    run.Source is { } single ? new[] { single.RecycleSource(run.Request.OperationId) } : [];
                                run.DeleteSession = new RecycleSession(run, originalSources, recycleFixtureBackend, log);
                                RecycleSessionProbe?.Invoke("captured", run.DeleteSession);
                            }
                            Diagnostic("source_acquired", run);
                            sourceReply = new { version = 1, operation_id = run.Request.OperationId, op = "native_result",
                                call_id = nativeCall.GetProperty("call_id").GetGuid(), success = true,
                                read_data = run.Request.ExtractOptions?.Reclamation is not null ? JsonSerializer.Serialize(new {path=run.Request.Path,volume_paths=new[]{run.Request.Path}}) : run.VolumeSources is {} volumes ? JsonSerializer.Serialize(new {path=volumes.PrimaryPath,volume_paths=volumes.Paths}) : null };
                        }
                        catch (ArchiveOutputException failure) {
                            if (failure.Stage.StartsWith("reclamation_", StringComparison.Ordinal)) {
                                run.FailNative(new(run.Request.OperationId,nativeCall.GetProperty("call_id").GetGuid(),"source_acquire",null,failure.Stage,failure.NativeCode,failure.State));
                            }
                            if (run.Request.ReadContract is not null || run.Request.OutputReadmission is not null || run.Request.BrowserEntry is not null || run.Request.BrowserAuthorize is not null || run.Request.JobExecution)
                                run.Fail(failure.Stage is "source_changed" or "missing_volume" or "corrupt_archive" or "unsupported_format" or "unsupported_method" or "unsupported_encryption" or "target_exists" ? failure.Stage :
                                    failure.Stage is "resource_limit" or "read_limit" or "session_entry_limit" ? "resource_limit" : failure.NativeCode is 32 or 33 ? "source_changed" : "source_unavailable", failure.NativeCode);
                            if (failure.Stage.StartsWith("motw_", StringComparison.Ordinal)) run.Fail(failure.Stage, failure.NativeCode);
                            DiagnosticSourceFailure(run, failure);
                            sourceReply = new { version = 1, operation_id = run.Request.OperationId, op = "native_result",
                                call_id = nativeCall.GetProperty("call_id").GetGuid(), success = false,
                                error_code = failure.Stage.StartsWith("reclamation_", StringComparison.Ordinal) ? failure.Stage : failure.Stage is "missing_volume" or "corrupt_archive" or "unsupported_format" or "unsupported_method" or "unsupported_encryption" or "resource_limit" or "session_entry_limit" or "read_limit" or "reparse_component" or "source_output_collision" or "source_changed" or "cancelled" or "target_exists" ? failure.Stage : failure.NativeCode is 32 or 33 ? "source_changed" : "source_unavailable",
                                native_error_code = failure.NativeCode };
                        }
                        if (!run.Input.Writer.TryWrite(Pack(sourceReply))) throw new ArchiveFault("input_limit");
                        continue;
                    }
                    if (run.Source is null && run.Output?.HasReclamation != true && run.CreationSources is null && argument != "--archive-worker-fixture") throw new ArchiveFault("source_lease_required");
                    if (action == "member_acquire") {
                        var acquired = AcquireVirtualMember(run, nativeCall);
                        if (!run.Input.Writer.TryWrite(Pack(acquired))) throw new ArchiveFault("input_limit");
                        continue;
                    }
                    if (action == "source_stamp" && (run.Request.OutputReadmission is not null || run.Request.ReadContract == 2 || run.Request.BrowserEntry is not null || run.Request.BrowserAuthorize is not null || run.Request.JobExtract?.Selection is not null || run.Request.JobModify is not null)) {
                        if (run.Source is null || run.Failure is not null) throw new ArchiveFault("source_lease_required");
                        if (!run.Input.Writer.TryWrite(Pack(new { version = 1, operation_id = run.Request.OperationId, op = "native_result", call_id = nativeCall.GetProperty("call_id").GetGuid(), success = true, read_data = run.VolumeSources?.Identity() ?? run.Source.ReadmissionIdentity() }))) throw new ArchiveFault("input_limit");
                        continue;
                    }
                    if (action == "output_ready" && run.Request.OutputReadmission is not null) {
                        if (run.Source is null || run.Failure is not null || run.Output is null) throw new ArchiveFault("source_lease_required");
                        if(!run.Request.JobExecution)run.Output.CreationOutput.CreatePreparedRoot();
                        if (!run.Input.Writer.TryWrite(Pack(new { version = 1, operation_id = run.Request.OperationId, op = "native_result", call_id = nativeCall.GetProperty("call_id").GetGuid(), success = true }))) throw new ArchiveFault("input_limit");
                        continue;
                    }
                    if (action is "source_roots" or "source_next" or "source_open" or "source_read" or "source_close" or "source_check") {
                        if (run.CreationSources is null || run.Failure is not null) throw new ArchiveFault("source_lease_required");
                        var sourceResult = run.CreationSources.Execute(run.Request.OperationId, nativeCall);
                        if (!run.Input.Writer.TryWrite(Pack(sourceResult))) throw new ArchiveFault("input_limit");
                        CryptographicOperations.ZeroMemory(body);
                        continue;
                    }
                    if (action is "source_validated" or "codec_opened" or "source_decoding" or "source_precommit" or "creation_planning" or "creation_encoding" or "creation_verifying" or "edit_generating" or "edit_verifying" || run.Request.Operation is ("modify_7z" or "modify_tar") && action is ("edit_planning" or "edit_reading" or "edit_reencoding")) {
                        run.VolumeSources?.Verify();
                        if (run.Output?.HasReclamation == true) run.Output.VerifyReclamationSource();
                        SourceCheckpointProbe?.Invoke(action);
                        if (action is "creation_planning" or "creation_encoding" or "creation_verifying" || IsCreation(run.Request.Operation) && action == "source_precommit")
                            Diagnostic(action == "source_precommit" ? "creation_committing" : action, run);
                        if(run.Request.Operation is ("modify_7z" or "modify_tar") && action is ("edit_planning" or "edit_reading" or "edit_reencoding" or "edit_verifying")) Diagnostic(action,run);
                        if (run.Request.JobExecution && action is ("creation_encoding" or "creation_verifying" or "source_precommit"))
                            Emit(new("job_state",run.Request.OperationId,Category:action=="creation_verifying"?"verifying":action=="source_precommit"?"committing":"running",ElapsedMs:run.Clock.ElapsedMilliseconds));
                        if (run.Request.JobExecution && action is ("edit_generating" or "edit_verifying")) Emit(new("job_state",run.Request.OperationId,Category:action=="edit_verifying"?"verifying":"running",ElapsedMs:run.Clock.ElapsedMilliseconds));
                        if (!run.Input.Writer.TryWrite(Pack(new { version = 1, operation_id = run.Request.OperationId,
                            op = "native_result", call_id = nativeCall.GetProperty("call_id").GetGuid(), success = true })))
                            throw new ArchiveFault("input_limit");
                        continue;
                    }
                    if (run.Output is null || run.Failure is not null) throw new ArchiveFault("stdout_invalid_frame");
                    if(run.Request.JobExecution && action=="destination_preflight"){
                        var required=nativeCall.TryGetProperty("offset",out var expected)&&expected.ValueKind==JsonValueKind.Number?expected.GetUInt64():(ulong?)null;
                        await CheckDestination(run,required);
                        if(!run.Input.Writer.TryWrite(Pack(new{version=1,operation_id=run.Request.OperationId,op="native_result",call_id=nativeCall.GetProperty("call_id").GetGuid(),success=true})))throw new ArchiveFault("input_limit");
                        continue;
                    }
                    if(run.Request.JobExecution&&!run.DestinationChecked&&action is ("directory" or "directory_new" or "begin" or "begin_overwrite" or "observe_target"))await CheckDestination(run,null);
                    if (run.Request.ParallelExtraction && action is ("begin" or "begin_overwrite" or "commit")) {
                        if (!run.Negotiated || run.Source is null) throw new ArchiveFault("source_lease_required");
                        if (action is "begin" or "begin_overwrite") run.Output.PrepareParallelRoot();
                        await DispatchNative(run, nativeCall);
                        continue;
                    }
                    if (run.Request.ParallelExtraction) await JoinNative(run);
                    var reply = run.Output.Execute(run.Request.OperationId, root.GetProperty("native"), out var nativeFailure);
                    if (nativeFailure is not null) run.FailNative(nativeFailure);
                    if (!run.Input.Writer.TryWrite(Pack(reply))) throw new ArchiveFault("input_limit");
                    CryptographicOperations.ZeroMemory(body);
                    continue;
                }
                if (root.GetProperty("op").GetString() is "complete" or "failure")
                {
                    if (run.Request.ParallelExtraction) await JoinNative(run);
                    run.SawTerminal = true;
                    DiagnosticWork(run, root);
                    if (root.TryGetProperty("source_delete", out var deleteEvidence)) run.DeleteEvidence = recycleEvidenceFixtureProbe?.Invoke(deleteEvidence.Clone()) ?? deleteEvidence.Clone();
                    if (root.GetProperty("op").GetString() == "failure") {
                        var errorCode = root.GetProperty("error_code").GetString();
                        var resourceLimit = errorCode == "ResourceLimit";
                        var selectedOutputLimit = errorCode == "SelectedOutputLimit";
                        var passwordCancelled = (run.Request.PasswordInteraction || run.Request.Operation == "extract") && errorCode == "Cancelled";
                        run.Fail(passwordCancelled ? "cancelled" : selectedOutputLimit ? "selected_output_limit" : resourceLimit ? "resource_limit" : "worker_failure",
                            root.TryGetProperty("native_error_code", out var nativeCode) ? nativeCode.GetInt32() : null, operationFailure: true);
                    }
                }
                if (run.Request.ReadContract is not null || run.Request.JobExecution) {
                    if (!run.Negotiated) throw new ArchiveFault("protocol_violation");
                    var reason = await readFrame!(run.Request.OperationId, root.Clone(), run.MetadataStop.Token);
                    if (reason is not null) throw new ArchiveFault(reason);
                    if (run.Request.ReadContract is not null && root.GetProperty("op").GetString() == "entry_batch" &&
                        !run.Input.Writer.TryWrite(Pack(new { version = 1, operation_id = run.Request.OperationId,
                            op = "metadata_ack", batch_sequence = root.GetProperty("batch_sequence").GetUInt64() })))
                        throw new ArchiveFault("input_limit");
                } else Emit(new("frame", run.Request.OperationId, Frame: root.Clone(), ElapsedMs: run.Clock.ElapsedMilliseconds));
                }
                finally { CryptographicOperations.ZeroMemory(body); }
            }
        }
        catch (ArchiveFault e) { run.Fail(e.Category, e.NativeCode); }
        catch (OperationCanceledException) when (run.MetadataStop.IsCancellationRequested) { }
        catch (Exception) { run.Fail("stdout_invalid_frame"); }
        finally {
            await JoinNative(run);
            lock (run.BulkGate) { run.BulkPending?.Sent.TrySetResult(false); run.BulkPending = null; }
        }
    }
    private static async Task DrainStderr(Run run, Stream stream)
    {
        var buffer = new byte[8192];
        try
        {
            int n;
            while ((n = await stream.ReadAsync(buffer)) != 0)
                if (Interlocked.Add(ref run.StderrBytes, n) > 65536) run.Fail("stderr_total_limit");
        }
        catch (Exception) { run.Fail("stderr_io_failure"); }
    }
    private async Task SendInput(Run run, Stream stream)
    {
        try
        {
            await foreach (var data in run.Input.Reader.ReadAllAsync(run.InputStop.Token))
            {
                var sent = false;
                try {
                    run.InputStop.Token.ThrowIfCancellationRequested();
                    stream.Write(data);
                    ObserveInputWire(run, data);
                    if (ControlInputCheckpointProbe is { } probe) {
                        if ((BinaryPrimitives.ReadUInt32LittleEndian(data) & ArchiveBulkFrame.Discriminator) != 0) probe("binary_bulk");
                        else {
                            using var control = JsonDocument.Parse(data.AsMemory(4));
                            probe(control.RootElement.GetProperty("op").GetString()!);
                        }
                    }
                    sent = true;
                }
                finally { CryptographicOperations.ZeroMemory(data); run.CompleteBulkInput(data, sent); run.CompleteNativeInput(data, sent); }
                lock (gate) { if (run.PendingWrites > 0) run.PendingWrites--; }
            }
        }
        catch (OperationCanceledException) when (run.InputStop.IsCancellationRequested) { }
        catch (Exception) { run.Fail("stdin_io_failure"); }
        finally {
            while (run.Input.Reader.TryRead(out var pending)) { CryptographicOperations.ZeroMemory(pending); run.CompleteBulkInput(pending, false); run.CompleteNativeInput(pending, false); }
        }
    }
    private async Task<ArchiveSourceDeletionResult> ResolveSourceDeletionAsync(Run run, uint? exit)
    {
        if (run.Request.DeleteSource != true) return new("consent_missing", []);
        var session = run.DeleteSession;
        if (session is null) return new("source_unavailable", []);
        string reason;
        lock (gate) {
            var terminal = ResolveTerminal(run, true, exit);
            reason = session.Complete(run, terminal);
            if (reason == "eligible") { run.ArchiveTerminal = terminal; run.Recycling = true; }
        }
        ArchiveRecycleDiagnostics.SafeLog(log, "qualified", run.Request.OperationId, run.Clock, reason, null, 0);
        if (reason != "eligible") return new(reason, []);
        RecycleSessionProbe?.Invoke("qualified", session);
        return await session.ExecuteOwnedAsync();
    }
    private void Emit(ArchiveWorkerEvent value)
    {
        try { publish(value); }
        catch (Exception) { Stop("transport_failure"); }
    }
    private void DiagnosticWork(Run run, JsonElement frame)
    {
        try {
            if (log is null || !frame.TryGetProperty("work", out var work) || work.ValueKind != JsonValueKind.Object) return;
            var fields = new Dictionary<string, object?> { ["elapsed_ms"] = run.Clock.ElapsedMilliseconds };
            foreach (var name in new[] { "revision_passes", "revision_bytes", "revision_us", "catalog_loads", "eligibility_checks", "outer_decodes" }) {
                if (!work.TryGetProperty(name, out var value) || !value.TryGetUInt64(out var count)) return;
                fields[name] = count;
            }
            foreach (var name in new[] { "zip_member_decodes", "tar_member_reads", "sevenz_block_decodes", "rar_decoder_opens", "rar_replayed_entries",
                "zip_local_checks", "zip_descriptor_checks", "zip_record_seeks", "zip_source_payload_decodes", "zip_addition_payload_decodes", "zip_candidate_payload_decodes",
                "tar_index_builds", "tar_record_scans", "tar_header_seeks", "tar_payload_seeks", "tar_plan_bytes", "outer_decoded_bytes",
                "zip_inflate_us", "zip_inflate_calls", "zip_payload_read_us", "output_ipc_ack_us", "output_write_calls", "output_write_bytes",
                "output_begin_calls", "output_begin_wait_us", "output_close_calls", "output_close_wait_us",
                "output_directory_calls", "output_directory_wait_us", "output_directory_metadata_calls", "output_directory_metadata_wait_us",
                "parallel_workers_planned", "parallel_workers_high_water", "parallel_members_completed" })
                if (work.TryGetProperty(name, out var value) && value.TryGetUInt64(out var count)) fields[name] = count;
            if (work.TryGetProperty("selection", out var selection) && selection.ValueKind == JsonValueKind.Object)
                fields["selection"] = selection.Clone();
            log.Write("info", "archive.worker.work", run.Request.OperationId, fields);
        }
        catch (Exception) { /* Diagnostics do not determine the operation. */ }
    }

    private void DiagnosticSourceFailure(Run run, ArchiveOutputException failure)
    {
        try { log?.Write("info", "archive.worker.source_failed", run.Request.OperationId,
            new Dictionary<string, object?> { ["native_error_code"] = failure.NativeCode,
                ["elapsed_ms"] = run.Clock.ElapsedMilliseconds, ["stage"] = "source_acquire", ["recovery_id"] = run.Request.ExtractOptions?.Reclamation?.RecoveryId }); }
        catch (Exception) { /* Diagnostics do not determine the operation. */ }
    }
    private void DiagnosticTerminal(Run run, ArchiveWorkerEvent terminal)
    {
        var phase = terminal.Category is "success" or "succeeded_with_warnings" ? "completed" :
            terminal.Category is "cancelled" or "cancelled_with_committed_output" ? "cancelled" : "failed";
        try {
            if (log is null) return;
            var fields = new Dictionary<string, object?> {
                ["elapsed_ms"] = terminal.ElapsedMs, ["category"] = terminal.Category,
                ["native_error_code"] = terminal.NativeErrorCode, ["files_committed"] = terminal.FilesCommitted, ["files_skipped"] = terminal.FilesSkipped,
                ["commit_state"] = terminal.CommitState, ["cleanup_complete"] = terminal.CleanupComplete,
                ["native_pending_high_water"] = run.NativePendingHighWater,
                ["transport_stdin_json_bytes"] = Interlocked.Read(ref run.TransportStdinBytes),
                ["transport_stdout_json_bytes"] = Interlocked.Read(ref run.TransportStdoutBytes) };
            // Numeric observations survive disposal; the Run also owns sources whose bridge bind failed.
            ArchiveWorkerOutputBridge[] outputs = run.Request.DragMaterialization is null || run.VirtualOutputs.Count == 0
                ? run.Output is null ? [] : [run.Output]
                : run.VirtualOutputs.ToArray();
            var reads = (run.Source?.ReadObservation.Snapshot() ?? [])
                .Concat(outputs.SelectMany(item => item.CreationOutput.ReadObservation.Snapshot())).ToArray();
            foreach (var kind in new[] { "Source", "Target", "Output" }) {
                var role = reads.Where(read => read.Kind == kind);
                var prefix = "native_" + kind.ToLowerInvariant();
                fields[prefix + "_attempts"] = role.Sum(read => read.Attempts);
                fields[prefix + "_full_reads"] = role.Sum(read => read.FullReads);
                fields[prefix + "_read_bytes"] = role.Sum(read => read.BytesRead);
                fields[prefix + "_completed_bytes"] = role.Sum(read => read.CompletedBytes);
            }
            fields["partial_read_calls"] = outputs.Sum(item => item.CreationOutput.ReadObservation.PartialReadCalls);
            fields["partial_read_bytes"] = outputs.Sum(item => item.CreationOutput.ReadObservation.PartialReadBytes);
            fields["source_payload_read_calls"] = run.CreationSources?.PayloadReadCalls ?? 0;
            fields["source_payload_read_bytes"] = run.CreationSources?.PayloadReadBytes ?? 0;
            fields["output_payload_write_calls"] = outputs.Sum(item => item.PayloadWriteCalls);
            fields["output_payload_write_bytes"] = outputs.Aggregate(0ul, (sum, item) => sum + item.PayloadWriteBytes);
            fields["output_write_ack_frames"] = Interlocked.Read(ref run.OutputWriteAckFrames);
            fields["native_write_ms"] = outputs.Sum(item => item.CreationOutput.NativeWriteMs);
            fields["native_commit_ms"] = outputs.Sum(item => item.NativeCommitMs);
            fields["native_begin_ms"] = outputs.Sum(item => item.NativeBeginMs);
            fields["native_parent_resolution_ms"] = outputs.Sum(item => item.CreationOutput.ParentResolutionMs);
            fields["native_final_open_ms"] = outputs.Sum(item => item.CreationOutput.FinalOpenMs);
            fields["native_create_ms"] = outputs.Sum(item => item.CreationOutput.NativeCreateMs);
            fields["native_open_inspection_ms"] = outputs.Sum(item => item.CreationOutput.NativeOpenInspectionMs);
            fields["native_directory_ms"] = outputs.Sum(item => item.NativeDirectoryMs);
            fields["native_directory_metadata_ms"] = outputs.Sum(item => item.NativeDirectoryMetadataMs);
            fields["native_fingerprint_ms"] = outputs.Sum(item => item.CreationOutput.FingerprintMs);
            fields["native_postpublication_ms"] = outputs.Sum(item => item.CreationOutput.PostpublicationMs);
            fields["transport_stdin_json_frames"] = Interlocked.Read(ref run.StdinJsonFrames);
            fields["transport_stdout_json_frames"] = Interlocked.Read(ref run.StdoutJsonFrames);
            fields["transport_stdin_binary_bytes"] = Interlocked.Read(ref run.StdinBinaryBytes);
            fields["transport_stdout_binary_bytes"] = Interlocked.Read(ref run.StdoutBinaryBytes);
            fields["transport_stdin_binary_frames"] = Interlocked.Read(ref run.StdinBinaryFrames);
            fields["transport_stdout_binary_frames"] = Interlocked.Read(ref run.StdoutBinaryFrames);
            fields["payload_conversion_input_bytes"] = run.PayloadConversionInputBytes +
                (run.CreationSources?.PayloadConversionInputBytes ?? 0) + outputs.Sum(item => item.PayloadConversionInputBytes);
            fields["transport_payload_copy_bytes"] = run.PayloadEnvelopeCopyBytes;
            fields["transport_reserved_bytes"] = run.BulkRequested ? ArchiveWorkerBudget.BulkTransportReservation : 0;
            fields["transport_payload_buffers_high_water"] = run.BulkBufferHighWater;
            fields["transport_payload_buffer_bytes_high_water"] = run.BulkBufferBytesHighWater;
            log.Write("info", "archive.worker." + phase, terminal.OperationId, fields);
        }
        catch (Exception) { /* Diagnostics never change the resolved business outcome. */ }
    }
    private void Diagnostic(string phase, Run run)
    {
        try
        {
            log?.Write("info", "archive.worker." + phase, run.Request.OperationId, new Dictionary<string, object?>
            { ["elapsed_ms"] = run.Clock.ElapsedMilliseconds, ["category"] = run.Failure?.Category,
                ["recovery_id"] = run.Request.ExtractOptions?.Reclamation?.RecoveryId,
                ["native_error_code"] = run.Output?.LastCommitNativeCode ?? run.Failure?.NativeCode,
                ["files_committed"] = run.Output?.FilesCommitted ?? 0, ["commit_state"] = run.Output?.LastCommitState?.ToString(),
                ["logical_id"] = run.Request.OutputReadmission is { } readmission ? readmission.GetProperty("logical_id").GetGuid() : run.Request.OperationId });
        }
        catch (Exception) { /* A diagnostic sink never affects worker outcome. */ }
    }

    private async Task CheckDestination(Run run,ulong? required){
        if(run.DestinationChecked)return;
        var output=run.Output??throw new ArchiveFault("invalid_output_request");
        (bool Missing,ulong Available) observed;
        try{observed=output.CreationOutput.InspectDestination();}
        catch(ArchiveOutputException failure){throw new ArchiveFault(failure.Stage,failure.NativeCode);}
        var available=AvailableSpaceFixture??observed.Available;
        if((observed.Missing||required>available)&&ConfirmDestination is {} confirm){if(!await confirm(run.Request.OperationId,observed.Missing,required,available,run.MetadataStop.Token))throw new ArchiveFault("cancelled");}
        else if(observed.Missing||required>available)throw new ArchiveFault("destination_confirmation_required");
        if(run.Failure is not null)throw run.Failure;
        run.DestinationChecked=true;
    }
}
