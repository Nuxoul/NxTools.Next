# Phase A — product read-only archive seam

Status: implementation contract for the bounded open/test slice, not completed acceptance.
Canonical stage: **Phase A — Engine contracts and safety foundation**. No UI, job FIFO,
write operation, password interaction, recycle, package/install or real product launch.

## Verified current chain and smallest gap

`DesktopRuntime::initialize -> platform_bridge -> run_platform_server` owns the Rust pipe
server. `PlatformHost Program -> WindowsPlatformAdapter + PlatformPipeClient.RunAsync ->
RunSessionAsync -> ReadLoopAsync` is the C# client. Its serial command reader must only admit
archive work, never await codec/process lifetime or a metadata consumer. EventWindow is not
an archive execution owner. `ArchiveWorkerRunner.Start -> Task.Run(ExecuteAsync) ->
ExtensionNative.MakeJob/Pipe/Spawn(suspended)/AssignJob/Resume -> ReadFrames` already owns
one native process/Job/source lease. Cancellation retains its slot through actual process,
pumps, Job and handle cleanup. No production caller currently constructs this runner.

Current private worker: `worker_main -> worker::run -> request.validate -> accepted ->
execute -> source_acquire -> ArchiveEngine.open -> entry_batch | ArchiveRequest::test ->
complete/failure -> runner exited`. RAR/7z have isolated codec dispatch under the same owner.
Private u32LE frames/native calls are not the product protocol. Existing entry batches have
no snapshot identity/encoding/consumer credit and a finite8MiB stdout ceiling; that is not
business metadata backpressure. Ordinary build name is `nxtools-archive-worker.exe`, while
ForPackage expects sibling `NxTools.ArchiveWorker.exe`. Fixture copying does not resolve
product deployment responsibility.

Reference reinspection: tracked legacy NxTools grep again identifies only package ZIP,
FFmpeg installation and image/video compression; no general archive-window lifecycle was
identified. BrowserExtensionPackageService.CreateZipFileAsync and
FfmpegInstallerService.DownloadAndInstallAsync are not reusable archive owners. Archiver
core `Entry`, `list` and `test` are synchronous Vec/stream interfaces, not a worker IPC
framework; their filename-keyed/unbounded listing is not copied. No Bandizip disassembly.
No layout/placement/focus/native show-hide interaction is added in this headless slice.

## Intended owner and file boundary

- Protocol: an explicit versioned `ArchiveRead` platform command/event; typed failures,
  IDs, read-only limits, metadata and terminal. No native handle/content/ADS/output authority.
- Archive: callable headless product read owner, original operation/session/generation and
  bounded metadata reducer. Consumers run off UI/app callbacks. No settings persistence.
- IPC: route archive events through a dedicated finite notification channel with actual
  connection generation, not a competing clone of the UI notification receiver.
- C#: one per-pipe archive read session admits typed open/test/control and maps existing
  runner events. EOF retires session and cancels only its owned runs; destruction does not
  release a still-live native slot. A guarded isolated constructor selects a self-owned
  package root; it is never an environment/PATH/request override.
- Helper resolver: fixed deployed sibling name; fixed source-layout Debug mapping to the
  repository target/debug Cargo name only for the exact development host output. No current
  directory, arbitrary parent walk, PATH, downloads or configuration search. Hold no-reparse
  executable protection through runner cleanup. Packaging must deliberately map Cargo name
  to deployed sibling name; packaging/signature/install acceptance is not performed here.
- Private worker: opt-in product-read contract, actual negotiated read capabilities,
  snapshot/encoding and independently acknowledged bounded metadata batches. Legacy request
  omission retains existing fixture behavior. Old/missing/unknown capabilities fail closed.

## Product wire and bounds

Envelope v1 remains unchanged. Archive subprotocol version1 is explicit and required. Every
request/event carries operation_id, session_id and positive generation. Platform command ID
is distinct and correlated to operation_id. Actions are open/test/ack/cancel only; unknown
fields/actions reject. No extract/create/delete/password fields are accepted.

Initial conservative product ceilings (not enlarged archive-engine defaults): request256KiB,
frame1MiB, batch2entries/512KiB, cumulative serialized retained metadata4MiB/4096entries,
one unacknowledged batch per operation, two process-wide admitted read operations, finite8transport events,
finite4096retired IDs. Existing worker stdout8MiB bound is not increased. Engine resource
admission and Job reservation remain independently enforced. Small synthetic cases prove
these limits;100k/million-entry timing/RSS/first paint remain unmeasured/unaccepted.
Original runner read reservations strongly retain original Runs/helper/source leases across
pipe-session replacement until actual cleanup; no reconnect resets occupied/stuck slots.

Worker accepted must actually negotiate product_read_v1, metadata_credit_v1, read_open_v1
and read_test_v1 with the expected read contract version. Product capability is never inferred
from a suffix or only a build constant. Unknown capabilities grant no authority. An old
worker/version mismatch and a missing required capability remain distinct typed failures.
Metadata batches carry actual immutable archive snapshot ID, sequence and ordinal IDs;
encoding provenance is explicit. Completion verifies contiguous sequence/count before success.
The product consumer retains finite metadata, then acknowledges the exact original batch.
A paused consumer grants no further credit. Full/malformed queues fail typed and cancel;
timeouts, cancel and EOF wake waits. Native output acknowledgements are unrelated to this proof.

## Lifecycle and diagnostics

Pending state is installed before dispatch. Admission acknowledgement is not terminal success.
One accepted operation has exactly one terminal, only after actual runner cleanup observation.
Invalid requests/missing helper reject before spawn and occupy no permanent slot. Duplicate
operation/control messages cannot replace state or create another terminal. Cancel is
idempotent; old operation/session/generation and retired results cannot mutate a replacement.
Disconnect cancels originals, retires consumer state and prevents old events after reconnect.

Fixed categories distinguish invalid request, missing/untrusted helper, incompatible version,
missing capability, unsupported operation/format/method/encryption, source unavailable/changed,
resource/metadata/consumer budget, cancellation, disconnect, worker crash and cleanup failure.
No read-only request constructs output/recycle/Shell adapters or changes user state.

Accepted/start/negotiated/terminal diagnostics carry the same operation correlation, elapsed
and actual native code where available; counts/kinds only. No paths, member names, password,
raw frames, child output or data. No per-batch/chunk tick logs. Sink exceptions never affect
outcome. Production null recycle backend and prior P1/P2 protections are unchanged.

## Controlling red → green and acceptance

First tracer: real product owner -> existing typed Rust pipe -> shared C# production archive
session -> original runner/Job -> actual Rust worker -> independent self-owned small ZIP;
literal member/kind/size/encoding and stable snapshot+ordinal assertions. The isolated C#
apphost uses the same dispatcher/session, not a runner-only substitute. First run must fail
before product wiring, then the same assertion must pass. Extend vertically to test CRC,
helper/version/capabilities, cancellation/EOF/crash/lease cleanup, stale/duplicate messages,
metadata budgets and paused consumer. Record actual commands/counts/red/green/cleanup in a
separate evidence file and update the single current checkpoint only after final verification.
