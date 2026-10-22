# Real worker cancellation — receive, observe and cleanup contract

Canonical **Phase A — Engine contracts and safety foundation**. Authorized2026-10-20; only
bounded cooperative ENG09 conditions and related SAFE05/SAFE07, not complete ENG09/PhaseA/A–F.
[Long paths independently accepted](windows-long-path-acceptance.md), [finite100k independently
accepted](product-100k-post-status-acceptance.md), [original product cancellation P2 acceptance](product-read-p2-acceptance.md).
No repeat of those full matrices or resource observations; historical failures remain immutable.

## Reference-first review and smallest gap

Read matching archive reference first. Verified reference sources: Archiver core/src/lib.rs
152-165 Progress.check (atomic cancellation),417-442 ProgReader checks after underlying read at
EOF/1MiB and copy_prog maps cancellation; no interruption of a blocked read. SupaZip nested
supazip-core/src/traits.rs334-337/366-372 callback contract checks between entries; its ZIP copy
loops poll cancellation. Legacy NxTools FfmpegInstallerService.cs61-85 token-aware HTTP reads,
87-89 synchronous ZIP extraction without a cancellation argument. No general legacy archive
runtime was found. References do not prove Next native cleanup/slot lifetime or universal200ms.

Existing Next evidence already proves original-connection full-queue/Drop cancellation and
remote cleanup, metadata credit wake, approved commit linearization and tiny native partial
cleanup. Missing evidence: distinguish actual worker control-input reception from work-thread
observation and from native cleanup, and hold a real cleanup barrier to prove source/slot leases
cannot release early. Add regressions before any owning-boundary repair; do not redesign cancel.

## Stages and clock domains

1. Caller issues original cancel; queue admission/receipt is not worker reception.
2. Exactly one worker control reader parses a valid original operation/version cancel frame.
3. Original OperationControl.cancel sets first-wins state, wakes pause Condvar and registered
   ReplyPort/MetadataCredit waits. Work thread first returns Cancelled at a real checkpoint/wait.
4. No subsequent business batch/native write/commit after cooperative observation; previous
   already-issued IO may complete, not a universal kernel interruption or global rollback.
5. Worker failure frame/process exit is still not native cleanup. Original runner waits process,
   Jobempty and all pumps, then disposes original source/output/helper leases and removes slot;
   only then exited/cleanuptrue and product terminal permit same-connection readmission.

Use worker-local monotonic elapsed micros for input-received and first-observed; subtract ONLY
within that shared clock. Caller Stopwatch/Instant measures its own cancel->terminal interval.
Never subtract worker time from caller/runner time. <=200ms is a bounded cooperative observation
TARGET, not arbitrary codec/Win32 interruption or cleanup deadline; do not repeat for best score.

## Change boundary / diagnostics

Expected changes: OperationControl bounded first receive/observe facts; optional private worker
failure-only cancellation fields; fixed observation labels in existing checkpoint/native-reply/
metadata-credit return paths; dedicated C# actual ZIP fixture and internal first-write barrier;
Rust paused-control and original typed IPC cancellation regressions; exact effects registration;
reference/spec/task evidence and unique checkpoint. No public pause/action/version, new reader/
queue/process owner, budgets/deadlines/Job grace, dependency/UI/resource-tool/source-delete change.

Diagnostics are one fixed-category cancellation terminal record with original operation ID,
received/observed worker-local elapsed and boundary. No perchunk/frame/pointer log or raw payload.
No temporary DEBUG probe is needed if bounded production terminal facts suffice. Missing reception
or observation is explicit absent, never synthesized from send/ACK/local Cancelled/root death.
Additive optional private output fields are omitted on ordinary successful/legacy unaffected frames;
product read DTO does not gain native write/pause authority. Throwing diagnostics cannot change result.

## Deterministic small regressions

- ZIP extract actual runner/Job/worker/native: independent tiny Store256KiB input, old approved
  target, external partial. Hold only first successful64KiB native write before its reply; invoke
  original runner.Cancel, release promptly. Require worker original Cancelled frame with reception
  and observed native-reply/checkpoint facts, no subsequent native writes/publication. Original
  cleanup barrier holds after process/Job/pumps exit but BEFORE resource disposal: source still
  nonwritable, partial still owned/protected, ActiveCount retained, no exited/cleanuptrue yet.
  Release barrier, then cleanuptrue/slot0/source writable/ownpartial gone/externalpartial+old target
  intact. One actual subsequent request is admitted after cleanup; never precommit publication.
- Metadata credit cancel through original headless ArchiveReadClient/typedIPC/productionSession/
  runner/actualworker, tiny3member fixture, firstbatch retained without ACK. Explicit original cancel,
  one remote Cancelled/cleanuptrue, no second batch, source/helper released and same generation
  fresh request success. Do not infer separate worker reception timestamp if not captured here.
- Paused original OperationControl.checkpoint wakes and returns Cancelled after cancel, with no
  resume. Use deterministic owned thread/channel/Condvar boundary; no public worker pause added.
  Preserve existing actual engine paused-open test and private worker pause/resume support.

Source identity/leases, native error ownership, original200ms grace/finite own-Job termination,
cleanup-stuck busy reservations and CommitInFlight/postcommit publication semantics remain.
Only exact known copied helper may be removed after cleanup; retain small inputs/outputs/logs.
No real source recycle/delete, unknown sweep,100k/1GiB/largeZIP64/sampling/systemchanges/Desktop/
installedhost/package/sign/install/commit/push. NativeRecycle stays native_protection_unproven/
NotAttempted. Build C#testapphost first then matching isolated worker, one machine-wide Cargo lane
with JOBS1/INCREMENTAL0/RUSTFLAGS=-C codegen-units=256. Exact C#LIST->sameRUN counts/names; affected
small regressions/strictClippy/fmt/compile/docs/context/selector/diff/hash gates, then stop pending
independent acceptance, not a broad review or next feature.
