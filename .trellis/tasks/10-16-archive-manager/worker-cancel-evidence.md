# Real worker cancellation boundaries — implementation evidence

Date:2026-10-20. **Bounded cancellation regressions complete; pending independent acceptance.**
Canonical **Phase A — Engine contracts and safety foundation** remains incomplete. This covers
only the named ENG09 cooperative receive/observe/wait/cleanup conditions and related precommit
SAFE05/SAFE07, not complete ENG09, arbitrary codec/Win32 interruption, PhaseA or A–F.
[Contract](worker-cancel-contract.md); [results](worker-cancel-results.json).

## Reference-first / existing evidence / minimal gap

Read AGENTS, canonical plan, ENG09/SAFE05/07, owner/shared contracts and the independent
[product P2 cancellation](product-read-p2-acceptance.md), [finite100k](product-100k-post-status-acceptance.md)
and [Windows long-path](windows-long-path-acceptance.md) acceptances. No repeat of their full
matrices/observations. Earlier failure identities and unknown causes are not reinterpreted.

Before code, inspected matching archive note then recorded reference sources: Archiver
core/src/lib.rs152-165 atomic Progress.cancel,417-442 ProgReader underlying read then EOF/1MiB
cancel check/copy mapping; SupaZip nested traits.rs334-337/366-372 ProgressCallback; legacy
FfmpegInstallerService.cs61-85 HTTP cancellation and87-89 synchronous ZIP extraction. No general
legacy archive runtime, reference Windows blocked-I/O cancellation or Next native cleanup proof
is inferred. Updated source-backed reference navigation before test implementation.

Existing Next P2 evidence already covered reliable original-connection full-queue/Drop cancel,
credit wake and actual resource release. Approved publication cancellation remains accepted.
The missing fact was explicit worker input reception versus first work-thread observation,
separate from process exit and native resource cleanup. No cancellation-strategy defect was
found/fixed in this slice; the implementation adds bounded facts and deterministic regressions.
Opening scoped baseline231files plus tracked binary diff/fullstatus retained in scratch.

## Exact production call chain

- Product caller ArchiveReadOperation.cancel -> original-generation PlatformBridge retained cancel
  reservation -> existing typed platform pipe -> ArchiveReadSession.Dispatch cancel -> original
  ArchiveWorkerRunner.Cancel/Run.Fail. Receipt is not worker reception or cleanup.
- Original runner loop enqueues version1/original operation cancel to its existing finite stdin
  queue, SendInput writes through original pipe; one Rust worker control reader read_frame parses
  exact version/operation/op -> receive_cancel records first worker-local elapsed -> original
  OperationControl.cancel sets state/wakes pause Condvar and existing on_cancel hooks.
- Native ReplyPort.wait/MetadataCredit.wait set cancelled via those existing hooks, drop their own
  mutex before recording work-thread first observation; ordinary OperationControl.checkpoint
  records checkpoint/paused_checkpoint on its existing Cancelled return. No lock spans native IO,
  and no new control queue/reader/thread runtime or public pause action was introduced.
- Worker produces its own original `failure/error_code=Cancelled` terminal only after actual
  work returns. One optional private `cancellation` object has received_us/observed_us/fixed enum
  boundary, no payload. Normal success frames omit it; existing business DTO/schema/actions and
  source/output authority are unchanged. First values remain stable on duplicate cancel/observe.
- C# runner retains original source/output/helper leases and active reservation until process
  exit + Jobempty + all pumps drained, then disposes resources and releases slot before exited
  cleanuptrue. Original finite200ms grace/own-Job termination and cleanup-stuck occupied behavior
  remain unchanged. Process exit/worker failure/logicCancelled alone cannot certify cleanup.

## Red-capable actual ZIP streaming regression

New exact fixture `archive cancel boundary fixture actual streaming receive observe cleanup`
uses current built Rust worker, original ForPackage/Job/native output and an independent .NET
Store ZIP containing exactly256KiB from a fixed256byte pattern. Previous target is explicitly
identity-confirmed; external `.nxpart-external` has separate fixed bytes. Only self-owned paths.

Two internal nullable callbacks follow existing test-seam style, default unset:
- after original native session successfully writes first65536bytes, before native reply;
- after original stdin pump's complete frame write (only test sees fixed op, not body).
The test holds that one existing IO reply, calls original runner.Cancel, observes cancel was
written before releasing the reply. FIFO on the original stdin ensures cancel precedes later
native_result; no arbitrary sleep/large-file slowdown/fake worker or hand-created terminal.
SendInput completion is still NOT receive evidence. Actual worker Cancelled terminal supplies
receive/observe facts. No new log stream or artificial worker result is emitted by either seam.

First real run, before new worker diagnostic fields: LIST1/RUN1/PASS0/SKIP0/FAIL1/not_run0/exit1,
**"missing real worker receive/observe facts"**. All earlier runtime assertions below completed,
including fresh readmission. The red is an observation-contract gap, NOT a claim the prior
worker ignored cancellation. Raw red log and inputs/old target/external partial retained.

Same fixture after bounded facts: LIST1/RUN1/PASS1/SKIP0/FAIL0/not_run0/exit0. It verifies:
1. First real native write exactly65536bytes; source protected, active1, no exited event.
2. Original worker cooperatively emits Cancelled with actual reception + native_reply observation;
   final process ExitCode0 (not only forced parent projection).
3. Existing CleanupCheckpointProbe holds AFTER process/Job/pump drain but BEFORE resource disposal:
   ActiveCount1, no exited/cleanuptrue, source still nonwritable, exact native partial still65536bytes,
   previous target/external partial intact. This demonstrates exit is not cleanup.
4. Second actual small test worker occupies second original slot and reaches same cleanup barrier;
   active2, third request rejected concurrency_limit. No logical cancellation frees a reservation.
5. Release original cleanup: true native cleanup, slot0, source writable/hash unchanged, current
   ownpartial gone, foreignpartial/oldtarget retained. Exactly1write/65536bytes,0commit callbacks,
   FilesCommitted0; no later writes/metadata/publication after cancellation observation.
6. Fresh actual same-runner test request admitted after cleanup and succeeds; no replacement
   runtime/source-delete operation. Exact self-copied helper removed only after drained/hash check.

## Metadata credit / paused control / original product boundary

`worker_cancel_contract` launches actual stock Rust worker with tiny checked-in stored.zip;
original schema2/read_contract1 source-acquire rendezvous is acknowledged for this trusted input.
This focused seam is **not C#native source/Job proof**. One bounded stdout observer receives first
batch, sends NO metadata ACK, sends valid original cancel via original stdin. Worker emits one
Cancelled terminal with metadata_credit observation, exits0/EOF and no subsequent frames. No
manual Cancelled or replacement controls. Its same-worker received->observed interval is measured.

`explicit_credit_cancel_stops_batches_then_original_connection_readmits_after_cleanup` runs
actual ArchiveReadClient -> original typed platform IPC -> production ArchiveReadSession ->
original runner/Job/source/helper lease -> actual worker. Tiny independent CPython3member ZIP,
first sequence0 held without ACK, source not writable. Original explicit cancel yields no later
businessbatch, one owner terminal equal original captured remote Cancelled/cleanuptrue. Same
connection generation then admits a real Test (2files/12bytes) and succeeds with cleanuptrue;
source/helper released; endpoint retirement closes original pipe and host exits normally.
This product case measures caller cancel->cleanupterminal only; it does NOT expose or invent
worker reception/observation timestamps from another case or subtract different monotonic clocks.

Paused-control test uses existing OperationControl.set_paused/checkpoint and a cfg(test)-only
waiter count on its actual Condvar to prove the owned work thread really entered pause wait.
Cancel wakes it without resume, returns Cancelled, fixed boundary paused_checkpoint; received_us
remains absent because a local control cancel is not worker-input reception. Existing actual
paused ArchiveEngine.open regression also passes. Private worker pause/resume handlers already
exist; no new public product pause feature or arbitrary blocked codec interruption is claimed.

## Actual observations, both first green and final regression retained

All worker elapsed micros share that OperationControl's Instant origin. Caller times use the
caller's own clock; compare only durations in a single domain. No repeated timing for best score.

| Observation | Initial directed green | Final affected regression |
| --- | --- | --- |
| Actual extract worker received_us /observed_us /delta |75468 /75494 /26us |51235 /51289 /54us |
| Actual extract observed boundary |native_reply |native_reply |
| Caller cancel->stdin frame write completion |26.0136ms |1.7726ms |
| Caller cancel->pre-resource-disposal cleanup barrier |41.8893ms |27.8808ms |
| Actual private worker credit received/observed/delta |1804 /1818 /14us |1758 /1771 /13us |
| Product caller cancel->remote cleanup terminal |44.134ms |42.138ms |
| Local paused checkpoint cancel->returned result |79us (directed nocapture log) |passed target; individual value not serialized |

These satisfy the <=200ms cooperative observation TARGET in these bounded cases. Cleanup barrier
is intentionally held for assertions afterward, so it is NOT final cleanup latency. No universal
200ms codec/OS interruption, Release performance, sampled memory or first-paint claim. Arbitrary
slow native IO/codec behavior stays governed by original busy/cleanup/finite termination policy,
not accepted merely because these small cases pass. Full prompt/finalization/codec ENG09 remains open.

## Actual verification and scope

C#isolated apphost built before matching isolated-codecs worker. Current C#build0warnings/0errors,
exit0. Every direct Cargo command used CARGO_BUILD_JOBS1,CARGO_INCREMENTAL0,RUSTFLAGS=-C codegen-units=256;
one machine-wide lane checked, worker execution/relink serial. Foreground full-operation timeout;
no background process/test driver or unrelated build termination this slice.

Final precise C# LIST=RUN=PASS6names,selected6/run6/pass6/skip0/fail0/not_run0,exits0. Newactual
case plus existing product credit/capturedcleanup,runner cancellation/stuckslot and native ZIP.
No complete overwrite or long-path matrix. Explicit worker/upstreamfixture/Python paths configured.

Rust final77PASS/0fail/0ignored over8groups,2deliberatelyfiltered(100k and acceptedlongsource).
Groups: Archive--lib13,engine13,productcancellation10,productchain22,consumer6,workercancel1,
workerextraction1,workerwire11. StrictClippy tests+examples isolated/default both exit0,-Dwarnings;
globalfmt exit0; workspacecheck--tests--offline exit0 (**compilation,NOTworkspace tests**).
Exact command arrays/start/end/exits and names in scratch verification.jsonl/C#results. No changed
shared ZIP dependency/reader path, so no unrelated full-format/shared-package suites were added.

No optional subagent review was needed; direct source/lock-order/compatibility checks and relevant
real tests completed. No dependency/framework rewrite,resource tool,100k/1GiB/largeZIP64/sampling,
UI/FIFO/PhaseB,formalDesktop/installedhost,package/sign/install/commit/push/systemchange or source
recycle/delete. NativeRecycle remains **native_protection_unproven/NotAttempted**.

Final offline40assertions PASS:7new roots/26rawfiles retained,90historicaltaskfiles unchanged,
prior longpath186rawfiles and100k9new+12olderresourcefiles hash-identical; unrelateddirtydiff retained.
Spec106docs/14owners,context80implement/77checkentries,fixtureselector and gitdiff--check all exit0.
Reference32766bytes remains below32768; existingnative-contracts35497byte injection warning and
unrelatedGitCRLF warnings retained. Final0archiveprocesses/0testpipes/0fixedhelpers. One scratch
offline-reader encoding incident (PowerShell UTF16 log, not productfailure) was corrected by BOM
decoding without changing original logs/inputs or rerunning products. Exact closing scope/hashes
live in cancel-boundary-final-hygiene.json. All red/green data retained; no further actual run.
**Bounded cancellation regressions complete; pending independent acceptance.** Stop; no next
feature or full ENG09/PhaseA/A–F signoff.
