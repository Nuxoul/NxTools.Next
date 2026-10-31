# PlatformHost diagnostics

Evidence: `repo:csharp/NxTools.PlatformHost/Diagnostics/PlatformLog.cs#Write`, `repo:csharp/NxTools.PlatformHost/Diagnostics/PlatformExceptionDiagnostics.cs`, `repo:csharp/NxTools.PlatformHost/Protocol/PlatformPipeClient.cs`.

IPlatformLog.Write receives level/event/correlation/structured fields. Observed sink capacity4096, TryWrite drop counting, daily JSONL/14-day cleanup and recursive sensitive-field redaction. Do not rely on redactor to sanitize arbitrary string values: fixed stage/exception type/HRESULT/stack/native code only; raw Exception.Message, command result data and child/URL/path/user payload forbidden. Pipe completion records has_data, not data itself. No pointer/frame/hotkey polling/byte-tick event streams; log state transitions/rate-limited repeated failures. Sink failure must not alter feature result.

Write is nonblocking; observed Dispose drains/joins synchronously, so do not claim bounded shutdown from TryWrite alone. No generic logging redesign in documentation scope. Tests use CreateForDirectory and structured/exception privacy assertions; do not read real logs. Safe compiled harness filter results are distinct from live native acceptance. [Shared privacy](../shared/engineering.md).

Trusted-update client emits accepted,peer-verified,completed or fixed failurephase(enrollment_validation/package_validation/pipe_connect/peer_service_validation/request_reply/published_verification) with original correlation/nativecode/elapsed. Both --prepare-trusted-update and EnsureIndexerService forward IPlatformLog; do not silently collapse all failures into misleading signature diagnosis. Observational logging never changes authentication/update result or includes package paths/replies/raw exceptions.

## Scenario: Archive worker terminal projection
### 1. Scope / Trigger
Native output failure crosses the private Rust worker rendezvous and becomes the final event/log.
### 2. Signatures
`ArchiveWorkerOutputBridge.Execute(..., out ArchiveNativeOutputFailure?)` returns operation/call/action/token/category/native/state; `ArchiveWorkerRunner.ResolveTerminal` produces one immutable ArchiveWorkerEvent consumed by DiagnosticTerminal and publish. Evidence: `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerRunner.cs`, `repo:csharp/NxTools.PlatformHost.Tests/ArchiveWorkerOutcomeTests.cs`.
### 3. Contracts
Fix the current native-call failure by first-failure CAS, never restore a mutable last-file error at terminal time. Begin has no allocated token. Six target failures and existing overwrite special states are promoted; unrelated codec/domain mappings stay unchanged. No private wire/schema change. Terminal log reuses exact category/native_error_code/commit_state/files_committed/elapsed_ms/cleanup_complete/correlation from the event. Publication/unknown/postcommit facts outrank cancellation; diagnostic exceptions cannot change the result.
### 4. Validation & Error Matrix
Target changed/missing/directory/reparse/busy/denied→exact category+FailedBeforeCommit+actual native code when supplied. Existing cancel→cancelled, publication+cancel→cancelled_with_committed_output. Unknown/postcommit→retained truthful state/count/native, no retry/delete. target_exists remains worker OutputCollision, not this promotion. Prior operation/earlier committed file cannot supply a later failure's category/native code.
### 5. Good/Base/Bad Cases
Good: current Begin/Commit fault causes fixed typed terminal. Base: success logs category success. Bad: recover LastCommitCategory regardless of terminating cause or log run.Failure while event resolves another category.
### 6. Tests Required
ActualTargetFailures12cases assert event/state/native/count/cleanup/bytes/owned partials; ActualTerminalDiagnostics8cases compare structured log/event fields and throwing-sink/privacy behavior; ActualOperationIsolation4cases cover committed-first/failed-second and next-job success. Preserve original29fixtures. No raw names/paths/passwords/frames/tokens in logs.
### 7. Wrong vs Correct
Wrong: terminal event resolves overwrite_not_supported but failed log says worker_failure. Correct: resolve once after real cleanup→write safe fields from terminal→publish same terminal.

HZ4-TERM01 Job handoff: `ArchiveWorkerEvent.HasNativeFailure` is an internal `[JsonIgnore]` provenance bit, set only from the current first-failure `ArchiveFault.OutputFailure` in `ResolveTerminal`. Job terminal projection uses that owning native cause ahead of the worker's consequential Cancelled frame; it must not infer ownership merely from a nonnull native code or from a mutable last error. Cleanup-incomplete remains first, publication/unknown/postcommit and timeout priorities remain; actual native category cancelled stays cancelled. No new IPC/worker/public DTO/diagnostic field or fault switch. `ArchiveJobTerminalTests.NativeFailureBeforeWorkerCancelled` forces real FailNative -> worker Cancelled frame -> original Job exit; its12same-boundary controls preserve genuine/native cancel, incidental nativecode, source/target,64unknown,timeout,cleanup/domain and published-latecancel facts.

Archive read timeout projection: original runner category timeout must not become user Cancelled
because cooperative shutdown later emits worker Cancelled. Current unchanged wire has no timeout
enum; product terminal is failed/io_failure after actualcleanup, while runner diagnostics retain
timeout. No budget/deadline expansion. Red-capable real session/runner fixture and exact contract:
[resource/timeout lifecycle](../nxtools-archive/backend/product-read.md#scenario-resource-observation-and-timeout-cause-preservation).

Resource probes are test-only observation owners, not product diagnostic permissions. Their
schema2 separately records operation/measurement/stop/cleanup; failed native counter pairs never
increment successful samples or peaks. Setup and runner-dispatch timing domains are separate.
Safe-stop/real cleanup and Legacy-result rules are owned by
[resource-tool contract](../nxtools-archive/backend/product-read.md#scenario-resource-tool-stop-and-validity-test-infrastructure-only).
No raw payload/secret diagnostics, per-sample production events or new PlatformCommand are added.

Archive CLI-only host emits archive.cli_host.start/connected/stopped/failure with GUID pipe-session
correlation, fixed stage/count/elapsed/native fields; original read/job/worker operation correlation
is unchanged. No adapter/UI or raw frame/path/error output. EOF sets stop before original channel
completion so normal shutdown cannot be diagnosed as a failed event pump. Child exit still reflects
actual transport/session drain failure. CLI structured product output is separate from diagnostics.
