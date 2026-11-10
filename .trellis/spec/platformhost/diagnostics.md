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
Fix the current native-call failure by first-failure CAS,never restore a mutable last-file error at terminal time. A failed ordinary begin has no token; target_exists remains a conflict result. The terminal log reuses exact category/native_error_code/completion state/files completed/elapsed_ms/cleanup_complete/correlation from the resolved event. Completed/unknown output facts outrank cancellation; diagnostic exceptions cannot change the result.
The same terminal projects bounded native read observations from their owning scopes. Archive Edit/Create candidate source/target/output and held-partial readback follow [protected archive observations](archive-read-observation.md). Ordinary destination extraction has no output fingerprint/postpublication read and retains only journal completion/cleanup plus transport/native counters. Observation counters never become source,conflict or output authority.
Negotiated payload transfer projects numeric JSON/binary bytes/frames,conversion/copy bytes,reservation and bounded buffer high-water. Rust ordinary observations are `output_{begin,close,directory,directory_metadata}_{calls,wait_us}` with begin=begin+begin_overwrite, close=commit and directory=directory+directory_new. Waits start after prior Rust-tail flush, cover serialization/send/reply, and close includes queued payload consumed before close; counts include returned failures/cancel. Native fields are `native_begin_ms` for the whole bridge begin action, inclusive `native_parent_resolution_ms`, `native_final_open_ms` for final-open allocation/syscall/inspection, `native_create_ms` for `NtCreateFile` plus P/Invoke marshalling, `native_open_inspection_ms`, `native_directory_ms` and `native_directory_metadata_ms`. `native_commit_ms` surrounds only `session.Commit` inside the stdout pump after payload drain; `output_close_wait_us` is the full Rust await. `output_write_ack_frames=0`, JSON frames remain `2N + operation overhead`, `output_ipc_ack_us` remains Rust send/backpressure, and fingerprint/postpublication durations are zero. All durations are inclusive/overlapping, non-additive and not exclusive IPC. No per-chunk events, paths or payloads.
### 4. Validation & Error Matrix
Target changed/missing/directory/reparse/busy/denied -> exact category+incomplete-output cleanup+actual native code when supplied. Existing cancel -> cancelled; prior successful closes+cancel -> cancelled_with_committed_output. Unknown close/delete -> retained truthful state/count/native,no retry/rollback. target_exists remains worker OutputCollision/prompt input,not terminal failure promotion. A prior completed file cannot supply a later failure's category/native code.
### 5. Good/Base/Bad Cases
Good: current ordinary write/close fault or Create/Edit candidate fault causes one fixed typed terminal. Base: success logs fixed category/counts. Bad: recover LastCommitCategory regardless of terminating cause,log run.Failure while event resolves another category,or log per-record/path/payload data.
### 6. Tests Required
Actual target/terminal/isolation coverage asserts event/state/native/count/cleanup/bytes, ordinary incomplete outputs and candidate partials separately. Ordinary diagnostics additionally require exact begin/close/directory/directory-metadata counts and waits, native begin/parent/open/create/inspection/directory aggregates, failure/cancel inclusion, post-tail-flush wait start and the `native_commit_ms` versus `output_close_wait_us` boundary. Preserve sticky native write code, completed-first/failed-second, source/candidate/cache observations and throwing-sink/privacy behavior. No raw names/paths/passwords/frames/tokens in logs; final Release performance acceptance remains separate.
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

Keyboard hook lifecycle logs `quick_launch.keyboard_hook.install_started`, `installed`, `install_failed`,
`thread_exited` and `reinstalled`, with the installation/configuration correlation, elapsed_ms and native_error_code.
Normal stop is not an unexpected exit; bounded join exhaustion logs stop_deferred without claiming native termination.
These run outside HookCallback. Key/down/up callbacks never log or publish IPC. No key content, display binding,
heartbeat, idle probe or silent-removal inference is permitted. Typed health forwarding uses the negotiated
quick_launch_keyboard_health_v1 capability; reconnect availability remains the Hello snapshot.
Health snapshot replay and current-session queue recovery preserve the original correlation/native code;
they are transport projection, not a new native operation or lifecycle log. The reader coalesces unchanged
facts without per-event recovery diagnostics; the original observed state transition remains the log owner.

## Bounded ordinary-Host memory observation

Evidence: `repo:scripts/observe-platformhost-memory.ps1`,
`repo:tests/architecture/platformhost-memory-sampler.tests.ps1`,
[runtime audit](../../../docs/audits/2026-11-06-platformhost-memory.md).
This external read-only entry selects one exact Host/Desktop image pair, actual parent PID,
ordinary `--parent-pid` role, session and creation times. It pins and rechecks identity, never
follows a replacement or starts the product. Default60seconds/5secondinterval, maximum600seconds,
minimum5secondinterval. Process private commit/working set/handles/threads and explicit
System.Runtime EventCounters share one observation window; preserve independent timestamps/units.
The counter collector disables runtime resume and exports GC heap, GC committed, naturalGen2count
and allocation rate only. Missing counters/target, changed identity and collector failure cannot
be a successful leak check. Private commit is not GC heap; private-minus-GC is not exact native
allocation. No payload/dump/forcedGC/trim/restart/service/pressure/packaging or production logging
changes. Only the owned collector can be terminated after the bounded shutdown wait. Raw command
lines are not persisted. Absent affected process is a diagnostic blocker, not a repaired incident.
