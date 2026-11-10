# Product archive read lifecycle

Evidence: `repo:rust/crates/nxtools-archive/src/product.rs`,
`repo:rust/crates/nxtools-protocol/src/archive_read.rs`,
`repo:rust/crates/nxtools-ipc/src/archive_read.rs`,
`repo:csharp/NxTools.PlatformHost/Protocol/ArchiveReadSession.cs`.
Exact cross-layer ledger: `repo:.trellis/tasks/10-16-archive-manager/product-read-wire.md`.
This is bounded Phase A open/test infrastructure, not UI/FIFO/write/recycle/full Phase A.

## 1. Scope / Trigger
Any product archive open/test, metadata consumption, cancel, disconnect or session replacement.
Use the callable headless product owner and existing platform pipe/runner. Do not send private
worker JSON as a business result or create another Windows process/Job runtime.

## 2. Signatures
`ArchiveReadClient::new(Arc<PlatformBridge>) -> Result<Self,ArchiveReadError>`;
`start(ArchiveReadCommand) -> ArchiveReadOperation`; `poll().await` processes one event on a
background task. Operation `take_batch/acknowledge(sequence)/cancel/terminal` owns original
operation/session/generation. `PlatformBridge::send_archive_read(command,command_id,generation)`
checks original connection under the existing router lock before pending-before-send admission.
C# `ArchiveReadSession.Dispatch(ProtocolEnvelope)` returns fast typed receipt; Events carries
independent accepted/negotiated/batch/terminal. Existing runner's actual exited cleanup owns the
native terminal, not a private complete frame or command receipt.
`ArchiveReadOperation::connection_generation()` exposes its immutable ORIGINAL admission connection,
retained after terminal removes Pending. Formal drag consumers carry that value through deferred
installation; never sample the shared bridge's current generation as the completed source origin.

## 3. Contracts
Outer envelope v1; archive subprotocol1(default) or explicit2(finite large read) required. Read actions only open/test/ack/cancel;
unknown/missing version/extra password/output/delete/worker path fields reject. No production
helper environment/PATH/current-directory override. C# owns fixed sibling deployment name and
DEBUG-only exact source-layout Cargo-name mapping; fixture selection stays guarded test-only.
Actual worker accepted must negotiate read_contract1 and four known read/credit capabilities;
extensions and build flags alone cannot enable a product capability.

Default/v1 ceilings remain command256KiB, frame1MiB, batch2entries/512KiB,4096entries/4MiB
retained serialized metadata and legacy stdout8MiB. Explicit v2 requires per-operation limits:
<=100000entries/128MiBengine metadata, <=64MiBcumulative transport and <=64MiBcumulative
consumer receipt, batch<=256entries AND256KiBserialized bytes, one exact unacknowledged batch.
Model allocation, cumulative transfer, cumulative retention and sampled memory are distinct.
ACK never refunds historical receipt quota. Build only one bounded batch over the existing
immutable snapshot; charge actual serialized entries plus C#Unicode/HTML-sensitive/DEL expansion.
C#/Rust final serializers enforce byte bounds; no full100k business JSON/second complete Vec.
2process-wide admitted runs and8IPCevents stay unchanged. Original Run reservations remain
rooted across pipe replacement until actualcleanup, never reset by a new session. Job remains
work+metadata+64MiB, <=1GiB/process/aggregate2GiB; original30s product timeout is unchanged.

Production v2 first sends a conservative legacy-shaped private schema2/read_contract1 request,
never v2-only fields to an old strict parser. Actual accepted must include old read/credit/source/
budget capabilities plus product_read_v2/bounded_metadata_v2. Only then send one read_upgrade on
the ORIGINAL control pipe; FIFO before source_acquire acknowledgement. Worker applies/seals
finite limits at that original boundary BEFORE engine/codec reads; duplicate/late/invalid upgrade
cancels. Missing v2 => IncompatibleVersion; missing required flag => MissingCapability. No unsafe
fallback. Controls keep metadata_ack1/exact sequence; business control version matches original.
Version validation precedes version-specific budget validation symmetrically Rust/C#.
Move one batch, validate original snapshot+format/contiguous sequence+ordinal/provenance, then
ack. Paused consumer grants no credit. MetadataCredit is independent of native output ReplyPort;
cancel/EOF wakes it. v2 success requires full counts and bounded transfer stats after cleanup.
Exact contract/matrix: `repo:.trellis/tasks/10-16-archive-manager/product-100k-contract.md`.
Overflow emit and flag/event8/wake1 drain/reset share one short synchronous epoch lock; both
fast and waiting-select paths use the same recovery boundary. Unlock after drain is new-epoch
admission; no lock spans await and no trailing unsynchronized reset can erase a later overflow.

Retire original operation on terminal/disconnect; late results from old operation/session/
generation/connection are ignored. Duplicate terminal never changes an existing terminal.
Cancel repeats are idempotent; destruction cancels the original worker even with a full shared
command/ordinary receipt queue. Original connection router reserves at most2read records at start,
coalesces one cancel each and owns it beyond operation/client/local-error/overflow retirement.
The fair original command consumer sends start before cancel, alternates pending cancels with
ordinary commands, and wakes without caller retry/background polling. Records remain until an
exact remote cleanup_complete terminal or original connection retirement; local false cleanup
cannot free them or authorize a new generation. No source/output/ADS/password diagnostics.
If unrelated ordinary notifications fill first, the SAME pipe task must not await them and
starve cancellation. Nonblocking notification failure retires/closes the original connection;
C# EOF cancels/drains every run of that session (and other pending commands are interrupted).
This is explicit connection-wide fail-closed recovery, not local-only Disconnected. Never send
old intent to a new generation, release native slots early, or block pipe close on full notices.

## 4. Validation & Error Matrix
| Boundary | Required result |
| --- | --- |
| Missing/untrusted helper | HelperMissing/HelperUntrusted before spawn, actual native code, no leaked slot |
| Old read version / missing known capability | IncompatibleVersion / MissingCapability, never metadata success |
| Invalid read request / write authority | InvalidRequest, no worker/output/Shell/recycle |
| Source acquire missing/locked/changed | C# native SourceUnavailable/SourceChanged retains actual code over later worker IoFailure |
| CRC/format/codec failure | Typed domain category, not worker_failure/raw backend text |
| Bad sequence/snapshot/ordinal / retained budget | ProtocolViolation / MetadataBudget, cancel original |
| Paused/slow/full consumer | ConsumerTooSlow/ConsumerQueueFull, bounded memory and cancel wake |
| Disconnect/stale generation/worker crash | Disconnected/discard/WorkerCrashed; no new-state writeback |
| Cleanup unconfirmed | CleanupIncomplete/cleanup_complete=false; native slot/leases not declared free |

## 5. Good/Base/Bad Cases
Good: original owner -> real typed pipe -> shared C# session -> existing Job -> actual Rust worker,
independent Python ZIP literal members, exact test counts/bytes and held lease release.
Base: absent optional limits fields default symmetrically, old private requests omit read_contract.
Bad: clone the UI notification receiver expecting broadcast, infer support from extension,
ack on producer enqueue, globally raise legacy stdout or treat private complete as cleanup.

## 6. Tests Required
`product_read_chain` asserts literal names/types/sizes/encoding/snapshot+ordinal, CRC test/corrupt,
paused credit/cancel/retained budget/native source lease, actual pipe EOF and consumer destruction.
`product_read_consumer` asserts exact ack, stale/retired/duplicate terminals, overflow drain and
finite cumulative retention. product_read_cancellation additionally covers full queue operation/
client destruction, metadata/protocol/receipt/overflow retirement, coalescing,2slot cleanup-bound
admission, ordinary receipt pressure, fair progress, original generation rejection and async wake.
IPC epoch tests schedule production emit inside recovery and across a locked concurrent drain;
fresh notifications survive and genuinely later overflows still report. Original-connection send
gate leaves no pending/queued command on old-generation rejection. C# ArchiveProduct fixtures
assert helper/no-reparse/held image, actual read/test, version/capabilities/crash, bounds/unique/
stale/cleanup and sink privacy. Actual pressured typed-pipe Drop tests observe the original
production cancellation terminal and actual source/helper release, not direct runner.Cancel.
Final original119+C# additions, all archive isolated-codecs and shared ZIP regressions remain
required; source/static/build checks are not installed/native-user/release acceptance.

## 7. Wrong vs Correct
Wrong: receive complete -> release source/helper/slot -> reuse another runner on timeout.
Correct: typed admission -> actual negotiation -> original finite metadata credit -> private
complete -> actual process/pump/Job cleanup -> immutable once-only business terminal.
Wrong: C# source acquire native2 -> worker IoFailure -> generic business IoFailure.
Correct: native owner retains SourceUnavailable/native2; later generic projection cannot erase it.

## Scenario: Resource observation and timeout cause preservation
### 1. Scope / Trigger
Bounded Debug resource probes or worker Cancelled frame after original runner timeout.
### 2. Signatures
`ArchiveReadSession.OnRunnerEvent(ArchiveWorkerEvent)` projects the terminal only after native
cleanup; `tests/performance/archive-resources/run.py ROOT SCENARIO ATTEMPT` is opt-in acceptance.
### 3. Contracts
Retain original timeout cause over later worker cancellation used for cleanup: current wire
has no timeout enum, so status failed/code io_failure, not user Cancelled. CleanupIncomplete
still outranks it; explicit cancel/disconnect retain their existing categories. The timeout slice
itself changes no budget/version; separately authorized readv2 above keeps the same30s deadline.
Source/helper/Job/pipe/reservation release stays native-owned.
Engine default100k admission, direct legacy worker enumeration, runner8MiB transport refusal
and product4096/4MiB ResourceLimit are FOUR distinct observations. Legacy worker snapshot ID
is unavailable; first batch is not first paint. Record Debug/cache/sampling limitations.
### 4. Validation & Error Matrix
Runner timeout plus worker Cancelled -> failed/io_failure after cleanup; explicit user cancel
-> cancelled; unconfirmed cleanup -> cleanup_incomplete. Default/v1 product100k remains
resource_limit; unversioned explicit100k remains invalid_request. Explicit finite v2 may succeed
only after actual capability/upgrade, full count/budget and cleanup checks; never truncated success.
### 5. Good/Base/Bad Cases
Good: original cause retained while observing actualcleanup. Base: small credit cancel on same
pipe. Bad: report30s automatic termination as user cancellation or model budget as measured RSS.
### 6. Tests Required
`ArchiveProductSessionTests.TimeoutIsNotUserCancellation` drives real session/runner30sdeadline
and controlled worker's late Cancelled frame; assert failed/io_failure/cleanuptrue/slots0/helper
release. Existing actual worker/native cancellation/credit/queue/full regression stays required.
Resource results and failed attempts: owning task resource-acceptance-evidence.md; not broadA PASS.
### 7. Wrong vs Correct
Wrong: operation.Failure=Cancelled from cleanup frame overrides runner timeout.
Correct: incomplete cleanup first, original runner timeout cause next, then ordinary domain mapping.

## Scenario: Resource-tool stop and validity (test infrastructure only)
### 1. Scope / Trigger
Opt-in resource probe threshold/deadline, sampler/parent exception or partial initialization.
### 2. Signatures
`run.py ROOT SCENARIO ATTEMPT`; test env `NXTOOLS_RESOURCE_RUN_ID` (UUID) and
`NXTOOLS_RESOURCE_CONTROL` (fresh owned directory); bounded stop.json `{run_id,reason}` and
schema2 status.json `{run_id,state,identities,expected_images,operation,cleanup,stop}`.
Evidence: `repo:tests/performance/archive-resources/control.py`,
`repo:tests/performance/archive-resources/driver_loop.py`,
`repo:rust/crates/nxtools-archive/examples/resource_probe_support/mod.rs`,
`repo:csharp/NxTools.PlatformHost.Tests/ArchiveResourceProbe.cs`.
### 3. Contracts
First-wins idempotent stop targets the exact owned probe; original engine control/product cancel/
runner owns cancellation. StopRequested -> Cancelling -> Draining -> Cleaned requires actual
worker/pumps/Job/source/helper/partial/slot release; receipt/local Cancelled/root death is not
proof. Finite cooperative wait precedes explicit exact-handle forced fallback; force/unconfirmed
cleanup stays Unknown/Incomplete, no recursive scratch deletion or subsequent measurement.
No product command/wire permission or parallel native process/Job owner is added.
Operation success, measurement completeness, stop and cleanup remain separate schema2 fields.
Required PID roles must have exact image/parent provenance and successful WS+Private counter pairs;
retain failed attempts, zero-sample records and null unavailable peaks. Finite startup retry,
bounded error/sample records and actual sampling gaps; sampled peaks are not absolute peaks.
C# setup_ms is separate; runner_dispatch starts immediately before runner.Start and all operation
stamps/differences share that domain. Legacy results remain original/setup-inclusive, not retimed.
### 4. Validation & Error Matrix
Missing root/host/worker, unverified image or zero successful counters -> measurement Incomplete,
never resource PASS even on exit0. Broken stop/status channel -> cancel/failure with retained reason.
Cooperation deadline -> explicit Forced targets, cleanup Unknown unless separately proven; no fake
cleaned flag. Late/duplicate/wrong-run stop cannot cancel a newer operation or dispatch twice.
### 5. Good/Base/Bad Cases
Good: injected threshold at tiny actual native partial then original runner drain and source/slot/
helper release. Base: transient counter failure followed by valid sample. Bad: terminate host first,
count failed reads as samples, conhost as worker, exit0 as resource PASS or delete uncertain partials.
### 6. Tests Required
Python deterministic counters/guards/driver exceptions/concurrent stop and controlled noncooperator;
C# exact listed tool fixtures for partial-init/control/counter-pair/virtual setup-time and real
runner/Job/small partial. Rust example control/guard ordering tests. Independent evidence is additive;
historical review/results must not change. Exact current counts belong to the single checkpoint.
### 7. Wrong vs Correct
Wrong: threshold -> host.terminate -> exit0/empty sample list -> resource PASS.
Correct: unique stop -> original cancel -> actual cleanup facts; exact required-role successful
samples independently determine measurement; force/missing fields/Legacy never imply PASS.

## Phase C browser consumer extension (partial implementation)

Evidence: `repo:rust/apps/nxtools-desktop/src/archive/application.rs`,
`repo:rust/crates/nxtools-protocol/src/archive_browser.rs`,
`repo:csharp/NxTools.PlatformHost/Protocol/ArchiveEntrySession.cs`,
`repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveEntryCache.cs`.
The approved Desktop internal path dependency reuses ArchiveReadClient; archive_client_bridge
shares router/archive transport and deliberately supplies no ordinary Desktop receiver. A second
client cannot take the single dedicated receiver. Browser context is opt-in v2 only, including
empty archives; optional legacy metadata remains omitted by default and actual serialized context
is charged to the existing cumulative receipt budget. Do not synthesize a missing snapshot ID.

Materialize/cancel/release retain one operation correlation while ordinary receipt ID is distinct
and installed before send. Materialize selects one immutable ordinal after source digest/native
revision comparison and ordinary-file safety checks. Materialize uses existing general32GiB-entry/
100GiB-output allowances, not a separate512MiB cap. Preview retains8MiB/text64KiB selected output;
finite source/decode/spool work and Job/memory/time/cancel/cleanup owners remain independent.
The native result never Shell-opens. Only current Rust window/session/generation/snapshot plus
successful outer and typed result and actualcleanup authorize a Shell command. Late success
releases its original operation-owned lease; release cannot invent another owner ID. Failed
materialization after actualcleanup retires its cache admission. Independent live leases have no
32-item business gate; active workers remain2. Sealed cleanup checks identity/hash/metadata/MOTW;
edited/locked files remain. Directory
retirement opens DELETE only after active use ends, rechecks exact identity and deletes only empty
owned ancestry. No recursive arbitrary-path deletion or inference from Shell process receipt.

The Phase C closeout now wires bounded independent window presentation state,24h abandoned-temp
eligibility/startup cleanup and browser password interaction; exact evidence/remaining integration
failures are in `repo:.trellis/tasks/10-16-archive-manager/phase-c-closeout-outcomes.md`.
Public ArchiveReadCommand stays password-free/read-only. Browser-only archive_browser_read uses
the same original read client/router/credit/reservations and private worker password coordinator;
the dedicated password prompt/reply binds original operation/session/view generation/ticket. Secrets
are not read metadata, Debug/log/config/argv; original finite attempts and typed ambiguity remain.
Browser read retries must use ORIGINAL control for native source identity, not a diagnostic attempt
control. Selected extraction keeps its existing no-output preparation/once-only extraction owner.
Retirement authority moves to the fixed sessions parent BEFORE directory deletion; output deletion
alone is never full-retirement success. Recoverable record/matching root identity survives restart;
unknown/edited/locked/active/malformed/legacy/no-record data is retained. Expiry only controls
eligibility and never bypasses original identity/hash/metadata/MOTW checks. Native GUI/Shell/UIA/DPI/
user acceptance and independent Phase C acceptance remain NOT RUN/pending; scoped counts do not
close those rows or concurrent workspace gate failures.

## Scenario: Browser-member identity and independent preview budgets
### 1. Scope / Trigger
Browser preview and virtual-file demand materialize one original member into a fresh owned lease.
Evidence: `repo:rust/crates/nxtools-archive/src/worker_browser.rs`,
`repo:rust/crates/nxtools-archive/src/worker_job.rs`, `repo:rust/crates/nxtools-archive/src/planning.rs`,
`repo:rust/crates/nxtools-archive/src/worker_codecs.rs`, `repo:rust/crates/nxtools-archive/src/worker.rs`,
`repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerRunner.cs`,
`repo:rust/crates/nxtools-archive/src/worker_password.rs`.
### 2. Signatures
Private `ExtractionPlan::browser_entry(snapshot, entry)` marks only isolated browser reads;
ordinary `ExtractionPlan::build` retains whole-archive output safety. `worker_browser::selected`
selects the immutable original ordinal. Original `worker_job::spool_limits` supplies finite
intermediate compound-stream allowance; selected entry/output limits remain request-owned.
### 3. Contracts
Optional size/encryption facts and OutputCollision are not source identity. A single fresh lease
can read a colliding regular member by original ordinal without relaxing general extraction.
Validate original path, kind, source revision, record extent and integrity before publication.
7z callback metadata follows the locked reader's block ranges followed by no-block file indices;
identical names never identify members. Preserve first decode failure, full-stream verification,
source recheck and delayed commit. Unknown size is charged by actual bytes, not zero authority.

Encoded source admission, no-output password TEST preparation, intermediate TAR spool, selected
expanded bytes and preview decoder budgets are independent. Browser preparation uses existing
general decode-work entry/output allowances, then restores the original selected extraction limits.
Product preview sets PasswordInteraction=true; browser TAR/non-password codecs route through the
original attempt instead of rejecting that intent. General non-browser password policy is unchanged.
Compound reads use existing metadata/spool reservation for outer expansion, then restore selected
constraints for inner extraction. Do not TEST the whole archive or expand a whole TAR under a text
member's64KiB output quota, or raise that quota to accommodate the container. Native resource errors
retain resource_limit/read_limit/session_entry_limit through runner/session cleanup; consequential
cleanup I/O cannot overwrite the original cause.
Existing correlation/start/terminal/elapsed/native-code diagnostics and owned cleanup remain;
never log member names, contents or credentials.
### 4. Validation & Error Matrix
| Input / stage | Required result |
| --- | --- |
| Same-name ordinary members in separate leases | Each original ordinal yields its own bytes |
| General extraction with unsafe/colliding outputs | Original rejection; no isolated-plan authority |
| 80 B ZIP/TAR member with product PasswordInteraction=true | Independent preparation/spool; exact80B output |
| Selected output exceeds request limit | resource_limit, no published path |
| Encoded source or intermediate spool reaches allowance | Read-stage resource failure, not selected-file-size claim |
| Source/path/kind/integrity failure or cancellation | Typed terminal and original unpublished cleanup |
### 5. Good/Base/Bad Cases
Good: reordered OLE descriptors read two same-name 7z blocks and empty entries by original ordinal.
Base: an 80 B UTF-8 member previews while a 128 KiB member is refused by the original text quota.
Bad: a filename HashMap overwrites metadata, missing estimates reject a whole selection, or the
container's expanded TAR length is reported as the selected text file's size.
### 6. Tests Required
`repo:csharp/NxTools.PlatformHost.Tests/ArchiveVirtualTreeTests.cs` exercises actual worker/source/
lease/native IStream identities, duplicate ZIP/7z members, empty directories and remaining budget.
`repo:csharp/NxTools.PlatformHost.Tests/ArchivePreviewTests.cs` exercises real80BZIP/TAR preview with
product PasswordInteraction=true, oversized selected output, no published path on failure, encoded-
source error preservation and lease release. Retain general engine/extraction-plan path/collision
regressions. Controlled native consumers are not physical
Explorer, foreign-target or installed-product acceptance.
### 7. Wrong vs Correct
Wrong: `metadata_by_name[entry.name]`, whole-archive collision policy for a fresh single-member
lease, or one output budget applied to source/spool/member/decode. Correct: source-bound ordinal
-> isolated validated plan -> stage-owned finite budgets -> full integrity -> actual cleanup.

## Scenario: Drag-scoped retained materialization

### 1. Scope / Trigger
Indexed FileContents demands in one admitted archive drag. Original DragSession/EntrySession/runner,
native output/cache and Rust format kernels remain the owners; ordinary one-shot entries are unchanged.
Evidence: `repo:csharp/NxTools.PlatformHost/Protocol/ArchiveEntrySession.Virtual.cs`,
`repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerRunner.Virtual.cs`,
`repo:rust/crates/nxtools-archive/src/worker_drag.rs`.

### 2. Signatures
`ArchiveVirtualPlan.Admit(items)` freezes the once-validated descriptor-index/original-ordinal map.
`CreateVirtualMaterialization(plan,authorization,operation)`, `DemandVirtual(context,index,limit)` and
`CloseVirtual(context)` live in the original EntrySession. Private `VirtualMember` means sealed member
readiness; it is not public `ArchiveEntryResult.CleanupComplete`. Runner `DemandVirtual/EndVirtual`
operate only on the existing Run. Rust `DragPort` serializes demands on the original control pipe.
`ConfirmVirtualMember(operation,request,publish)` validates the post-Seal pending request and unexpired
Run under the same original gate as work-timeout checks, then publishes readiness/stops its work domain.
Seal hashing is outside that gate; a late successful hash cannot grant a timed-out member.

### 3. Contracts
Schema2 extract optionally carries `drag_materialization:{version:1,members:[{index,ordinal}]}` and
requires actual `drag_materialization_v1`, browser/source binding and no Job/edit/read/recycle authority.
Version1 controls retain the Run operation ID: `drag_demand` carries `{request_id,index,ordinal,output_limit}`;
`drag_end` closes demand admission. Native `member_acquire` binds token=request_id to the matching
original cache/output owner; output paths never cross to Rust. `drag_member_ready` carries the same
request/index/ordinal and actual bytes only after complete required format integrity and native commit.
EntrySession Seal verifies that size before a cursor opens. Each cursor independently verifies content.
Descriptor/query-only work creates no worker/cache member. First uncached demand starts one Run/source/
catalog; duplicate indices reuse readiness. Only demanded files acquire outputs. One top-level source
activity hold does not substitute for actual worker source acquisition. Member readiness retains native
Run/global reservations until process, pipes, Job, source and outputs actually drain. Transferred streams
remain target-owned. Original30s work domains cover first startup/catalog, each member and Seal; receiver
idle is excluded. Normal end is bounded and original120s drag cancellation stays registered through drain.
ZIP reuses protected source/catalog/preflight and independent member decoders. Split ZIP admission
retains the original adapter's normalized anonymous source for preflight and all demands, with
external-volume revision/source binding unchanged. Its measured directory work is two admission
passes (original volumes plus normalized catalog), not another pass for each member. 7z reuses its
catalog; public consuming BlockDecoder completes the whole demanded block again for later demands.
RAR retains ordered TEST state, decodes predecessors without output and reopens/replays on backward demand.
Compound TAR retains its first fully verified anonymous spool and validated ordinal records locally.
Actual work events expose ZIP/TAR member invocations, 7z block invocations and RAR opens/replayed predecessors;
internal native source/block work remains unavailable. Test-only envs: explicit matching
`NXTOOLS_ARCHIVE_WORKER_FIXTURE`, prepared `NXTOOLS_ARCHIVE_OPT_FORMAT_ROOT`, immutable completed
`NXTOOLS_ARCHIVE_OPT04_ROOT` for same-path before/after, and `PI_SCRATCH_DIR` owned temporary roots.
`NXTOOLS_ARCHIVE_OPT04_RAR4_ROOT` supplies the marked pinned solid encrypted RAR4 source and independent
expectations prepared by `repo:tests/performance/archive-resources/prepare-opt04-rar4.py`.

### 4. Validation & Error Matrix
| Condition | Required boundary result |
| --- | --- |
| Duplicate/mismatched request/index/ordinal, multiple outstanding demand, late/end-again | Reject original Run; no replacement worker |
| Actual output exceeds positive remaining allowance or descriptor mismatches sealed size | resource_limit/source_changed; no returned medium |
| Required codec/block/CRC/MAC/size verification fails | Original typed failure; no member-ready/committed output |
| Cancel, pipe EOF, timeout or crash | Original drain and truthful terminal; transferred stream remains readable |
| Seal completes after original work deadline | ConfirmVirtualMember refuses medium; timeout cause and actual-drain reservation remain |
| Cleanup unconfirmed | cleanup_incomplete; retain reservation/evidence, never infer cleanup from member-ready |
| Retired/mismatched preview or missing independent encrypted grant | No adoption; no password UI in GetData |

### 5. Good/Base/Bad Cases
Good: first miss -> one retained worker -> verified member/Seal -> independent cursors -> receiver end ->
actual drain. Exact current preview may supply all demands with zero workers and a separate cache consumer.
Base: absent drag field preserves ordinary entry behavior. Bad: one new worker per index, early ready before
full block verification, end of OLE treated as async receiver end, or cache-hit bytes exempt from accounting.

### 6. Tests Required
`repo:csharp/NxTools.PlatformHost.Tests/Opt04DragTests.cs`: descriptor-only,1/2/16 demands, three completed
same-path trials, actual starts/source/catalog/occupied first-ready slot, exact outputs/cursors and drain.
`repo:csharp/NxTools.PlatformHost.Tests/Opt04DragTests.Formats.cs`: original ordinal/repeated/nonmonotonic
demands, full-block refusal, counted7z resets/RAR replays/one compound outer decode, no predecessor outputs.
`repo:csharp/NxTools.PlatformHost.Tests/Opt04DragTests.Lifetime.cs`:31s receiver idle, original work timeout,
Seal cancellation, handle-verified owned worker exit/stdin EOF, no hidden restart and actual cleanup.
`repo:csharp/NxTools.PlatformHost.Tests/Opt04DragTests.Encrypted.cs`: genuine solid RAR4 ordinary preview,
separate whole-TEST grant, unauthorized/revoked cached/new demand refusal, counted backward replay,
no GetData prompt, target-stream survival and cache cleanup after actual consumer release.
`repo:csharp/NxTools.PlatformHost.Tests/Opt04DragTests.Preview.cs`: current/mismatched/retired and mixed
preview hit/miss, consumer share survives preview retirement and cache sweep, every cursor still checked.
Keep original async/synchronous/slow/abort/password/budget/full-subtree and OPT-01 tamper/accounting gates.

### 7. Wrong vs Correct
Wrong: private member-ready -> release Run/global slot or publish cleanup=true.
Correct: full codec verification -> native writer close/commit -> Seal-ready -> actual receiver/Run drain.
Wrong: retire preview -> delete borrowed drag output during the next Create/Sweep.
Correct: original cache Release, Dispose and Sweep all honor the distinct drag-consumer share.

## Scenario: Demand-bound drag byte progress

### 1. Scope / Trigger
Original retained worker progress through native materialization and the existing Desktop progress window. Owners: `repo:rust/crates/nxtools-archive/src/worker_job.rs`, `repo:csharp/NxTools.PlatformHost/Protocol/ArchiveEntrySession.Virtual.cs`, `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveVirtualDragSession.cs`, `repo:rust/apps/nxtools-desktop/src/archive/drag_progress.rs`.

### 2. Signatures
`ProgressWriter::with_drag(DragPort)` enables the existing record-boundary/100ms writer. `DragPort::progress_demand` exposes only the initialized active demand. Worker `progress.drag_demand` carries `{request_id,index,ordinal}`. Native `VirtualMemberProgress` forwards through the original Content/Session callback; public `ArchiveVirtualDragProgress` adds optional `current_request_id`, `current_bytes_done`, `current_bytes_total`, `bytes_processed`, `sequence`.

### 3. Contracts
Worker operation plus demand request/index/original ordinal must match the live pending member. Native session/generation and Desktop's frozen descriptor/request mapping remain authoritative. Publication sequences increase under the original progress gate. `bytes_done` remains sealed-cache bytes; `bytes_processed` adds only the active unsealed member's real bytes. Seal charges once. A new demand resets current bytes; unknown member/overall totals stay null. Cache100% is receiver-pending, not destination-write completion. Original window identity uses `new_drag` and title `拖出解压`; ordinary Test retains its default. Official current/overall Progress controls consume separate facts. Background/close retires presentation only; cross-entity updates remain deferred.

### 4. Validation & Error Matrix
| Condition | Result |
| --- | --- |
| Stale operation/session/generation/request/index/ordinal or preinitialized/post-Seal snapshot | Ignore presentation snapshot; preserve execution authority |
| Older sequence or decreasing same-member/aggregate bytes | Ignore; no visible rollback |
| Missing total | Indeterminate progress; no invented percentage/ETA |
| Cache sealed, receiver active | Waiting state; no receiver success |
| Native receiver error/cancel or incomplete cleanup | Original terminal/failure state and cleanup ownership |

### 5. Good/Base/Bad Cases
Good: member A1/3, total1/10; member B1/7 after A seals, total4/10. Base: an unknown member retains unknown current/overall totals. Bad: count A again at Seal, accept A's late progress as B, or label sealed bytes as target completion.

### 6. Tests Required
`repo:csharp/NxTools.PlatformHost.Tests/ArchiveVirtualAsyncLifecycleTests.cs` requires actual worker nonzero current bytes before Seal, injected wrong demand refusal, reset/unknown totals and both End/OLE return orders. `repo:rust/apps/nxtools-desktop/src/archive/application_tests.rs` drives original serialized ingress and actual rendered dual bars, title, concise text, stale sequence/identity, terminal/background behavior. Preserve ordinary Test/Extract, preview lease and async cancel/target-held-stream tests.

### 7. Wrong vs Correct
Wrong: copy sealed totals into both bars or infer completion from cache100%. Correct: initialized worker demand -> original bound native callback -> ordered public snapshot -> deferred original view; receiver completion stays independent.

## Scenario: Cache retirement without an intermediate durable deletion flag

### 1. Scope / Trigger
Original preview/entry/drag cache lease release and restart cleanup. Owners: `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveEntryCache.cs#Release`, `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveEntryCache.cs#RetireEmpty` and `repo:csharp/NxTools.PlatformHost.Tests/ArchiveCacheRetirementTests.cs`.

### 2. Signatures
`Lease.Release() -> bool`, private `RetireEmpty() -> bool`, existing version2 `ManifestRecord { Retired, OutputRemoved, RootIdentity, Output, Directories, ... }`. The original fixed-parent `<lease>.retire.json` name and held-relative native operations remain unchanged.

### 3. Contracts
Persist/Flush the original `Retired=true` before output deletion. Verify full original output identity, size/mtime, attributes, MOTW and content before deleting it. After deletion or confirmed absence, set `OutputRemoved=true` only in memory and proceed to original directory/manifest retirement; do not persist a state immediately before destroying its manifest. A restarted fixed-parent record may therefore contain `Retired=true, OutputRemoved=false`: reopen through the same validated manifest/root identities, verify or observe the original output absent, then retire. If the original root is already absent, the validated retired record can finish its own deletion without opening a nonexistent output. Root/ancestor replacement, unknown children and edited/locked output remain preserved. Live consumer shares and transferred native streams keep their original ownership; no cursor/Seal/publication/content checks are removed.

### 4. Validation & Error Matrix
| Condition | Required result |
| --- | --- |
| Output deleted, unknown child blocks retirement | Return false; durable retired record survives with OutputRemoved still false |
| Restart with original root and pending flag | Original output check/absence detection, verified empty-directory cleanup |
| Restart after original root deletion | Retire only the validated fixed-parent record |
| Changed root/ancestor/output or malformed record | Preserve foreign/edited data and recovery evidence |
| Active consumer or native stream | Original retention/sharing rules; no premature deletion |

### 5. Good/Base/Bad Cases
Good: one durable retired stamp supports deletion and restart recovery, with no redundant post-delete Flush. Base: previously persisted OutputRemoved=true records remain readable. Bad: omit the retired stamp or accept pending retired records while assuming their output has already been deleted.

### 6. Tests Required
`ArchiveCacheRetirementTests.AncestryAndMalformedRecovery` asserts the actual pending persisted flags, obstructed nested output and restart with both existing and removed roots. Keep obstructed retirement/restart, expiry/edited/locked/live owner, replacement ancestor/root, malformed records and fair local rotation. Original first/repeated cursor tamper, borrowed preview, target-held stream and real drag worker/receiver cleanup tests remain mandatory.

### 7. Wrong vs Correct
Wrong: delete output, Flush OutputRemoved, immediately delete the manifest; or remove that Flush without accepting its restart state. Correct: durable retirement -> verified owned-output deletion -> in-memory deletion flag -> original retirement; restart repeats the original validation/absence decision from the durable retired record.
