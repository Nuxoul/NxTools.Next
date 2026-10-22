# Real worker cancellation boundaries — independent acceptance

Date: 2026-10-20. Canonical **Phase A — Engine contracts and safety foundation** remains incomplete.
Reviewed [contract](worker-cancel-contract.md), [implementation evidence](worker-cancel-evidence.md),
[results](worker-cancel-results.json) and applicable archive/native diagnostic and lifecycle sources.

## Verdict

**PASS for the named small cooperative cancellation receive/observe/credit/pause/native-cleanup
conditions. No demonstrated in-scope blocker remains.** This is not complete ENG09, arbitrary
blocked codec/Win32 interruption, all prompt/finalization cases, PhaseA/A–F, UI or deployment.
No cancellation-strategy bug is claimed fixed: the red-to-green closes missing diagnostic facts,
while the original cancellation and native cleanup strategy remains intact.

Acceptance changed no implementation/tests or submitted evidence/results. It rebuilt current
isolated C#apphost and matching worker, independently ran the bounded regressions ONCE, inspected
new retained outputs and historical hashes, and performed one bounded read-only Rust review.
No100k/1GiB/largeZIP64, full long-path/publication matrix or performance sampling was run.

## Source and review conclusions

- Original control reader checks version/operation before receive_cancel. First received_us and
  first observed_us are distinct worker-local elapsed values, not send receipts or cleanup facts.
- OperationControl checkpoint records observation only on its real Cancelled return; paused wait
  uses original Condvar, and test-only waiter count proves the thread actually entered it.
- ReplyPort/MetadataCredit drop their own mutex before observe_cancelled acquires control state.
  Original cancel collects hooks under control lock then invokes outside it; no new inversion found.
- Optional private cancellation object is assigned only for Cancelled failure, omitted on normal
  successful frames. Fixed enum, first-wins bounded storage; no perchunk logs or payload strings.
- C#native-write seam runs after original session.Write but before reply. Original stdin WriteAsync
  completion is only a send-completion observation, NOT worker receive or a separate flush guarantee.
  FIFO preserves cancel-before-reply; worker's own Cancelled frame supplies receive/observe evidence.
- Existing cleanup probe is after process/Job/pumps drain and before source/output/helper disposal
  and slot release. Actual resource leases/reservations therefore remain testable independently
  of worker process exit or local logicalCancelled.

One read-only reviewer raised a source-only High concern: receive_cancel records received_us,
releases the control mutex, then calls original cancel; another checkpoint may pass in that gap
(control.rs119–126). Parent independently inspected that path, original checkpoint/native-call
boundary and contract27–32. **Not accepted as a demonstrated defect/High blocker**: reception is
explicitly a different stage from cancellation state/first observation; no atomic parse-to-state
transition or ban on work after reception was required. Original cancel invocation could already
be preceded by reader descheduling, and native-call checkpoint is not an atomic write/commit CAS.
The contract forbids subsequent work after cooperative observation, not after reader reception.
No stale-observed continuation or changed cancellation result was demonstrated. Review concern
and this reasoning are retained, not silently called a clean review. No second review/fix loop.
Reviewer found no other concrete deadlock, wait-lock inversion or WorkerFrame compatibility defect.

## Independent execution

All direct Cargo commands inherited CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,
RUSTFLAGS=-C codegen-units=256. Machine lane checked per command; C#apphost built first; final
isolated-codecs worker rebuilt after feature-unifying checks, before C#execution. No overlapping
worker/relink or unrelated process termination. Foreground1800s verification completed normally.

| Actual independent check | Result |
| --- | --- |
| C#Debug testapphost build |exit0 |
| Rust affected lib +7integration targets |77pass/0fail/0ignored,8groups,2filtered,exit0 |
| Filtered tests |existing100k and accepted long-source; neither executed |
| Archive--lib/engine/productcancel/productchain/consumer/workercancel/workerextract/workerwire |13/13/10/22/6/1/1/11pass |
| Strict Clippy Archive tests/examples isolated/default |both exit0,-Dwarnings |
| fmt --all --check |exit0 |
| check --workspace --tests --offline |exit0, compilation ONLY, not workspace tests |
| Matching isolated-codecs worker |build exit0 |
| Exact C#LIST->RUN |selected6/run6/pass6/skip0/fail0/not_run0, exits0 |
| Exact identities |submittednames==LIST==RUN==PASS, all6 |
| Docs/catalog/context/selector/diff |all exit0 |
| Offline own checker |321checks PASS/exit0 |

Spec106docs/14owners,catalog79pages/79rows/78locked,context80implement/77checkentries.
Existing native-contracts.md35497bytes>32768warning retained; no unrelated cleanup/refactor.

## Independent actual observations

These are single-regression observations, not best-score measurements. No cross-process clocks
were subtracted; <=200ms is only the named cooperative target, not a universal interruption SLA.

| Boundary | Actual independent observation |
| --- | --- |
| Extractworker receive/observe/delta |59434us /59463us /29us, native_reply |
| Caller cancel->stdinwrite completion |2.0056ms, send only |
| Caller cancel->pre-resource-disposal barrier |24.8918ms; held afterward, NOT fullcleanup latency |
| Privatecredit worker receive/observe/delta |1407us /1418us /11us, metadata_credit |
| Privatecredit caller->terminal/exit observation |5574us, not C#Job/source-cleanup evidence |
| Localpaused checkpoint cancel->return |85us; no resume, no received timestamp invented |
| Original typed product caller->cleanupterminal |41355us; sameconnection followup success |

Actual extract proof: independent StoreZIP256KiB/fixedbytepattern; one real65536byte write,
zero commit callbacks/FilesCommitted0, actual worker Cancelled/processExitCode0. Cleanup barrier
held source/partial/active slot after process/Job/pumps exit; second actual worker held the other
slot, third request rejected concurrency_limit. On release, realcleanuptrue, source writable/hash
unchanged, ownedpartialgone, oldtarget/foreignpartial retained; actual fresh same-runner request
admitted/succeeded. Exact copied helper removed only after drained/hashchecked ownership.

Private credit test: actual stock worker originalstdio/source-acquire rendezvous for tiny trusted
fixture, no ACK, own Cancelled/metadata_credit facts, exit0/EOF/no laterframes. This seam explicitly
is NOT native C#source/Job protection evidence. Product test separately covers real typed IPC /
production session/runner, firstbatchheld, explicit originalcancel, no laterbatch, owner=remote
cancelled/cleanuptrue, source/helperrelease and normalhostexit. Samegeneration followup actualTest
succeeds2files/12bytes. No worker timestamps are falsely attributed to the product test.

## Raw evidence / preservation

Independent acceptance scratch:
C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df.
New retained cancellation roots:
- archive-cancel-boundary-b92f253f-9316-4d93-a05e-7091c117012b
- archive-cancel-credit-5a2a5449-d6e5-4d60-83b2-803603d90b5f
- archive-product-chain-779f7b0d-f0da-4df1-8d7f-aacb7c744f73

Current rootlisting also contained a prior longpath root; offline checker excludes it by evidence
file type and does not count it as a new cancellation execution. Original rawdata never modified.

Own offlinechecker directly checked inputZIP bytes, oldtarget/externalpartial/ownedpartial absence,
worker fixedfacts, productremote terminal equality/followup, exacttestidentities/counts, source
baseline and rawhashes. Rehashed26submitted rawfiles,186prior longpath rawfiles and90historical
files unchanged; opening231file baseline has exactly16declared changes/215unchanged. Current
acceptance sourcebaseline unchanged. Acceptednew100k/oldFailedattempt+samples also unchanged.
No claim the old unknown failure cause is explained by these cancellation results.

- cancel-independent-offline.py /cancel-independent-offline-results.json(321PASS), resultSHA256
  `43195f90613efc6dd7007399ce375b727c5971d94de73c2ce5b1df384495b1d7`.
- commandledger cancel-independent-ledger.txt SHA256
  `54a953ec23673c6bf7c7ce143c2d395eb8082c48df62ecd74e7d43c29c866630`.
- exact C#runlog SHA256 `bc5b7bdbdae3d01e49e44dfd533ad0721db8ffaf0c1f3e06a846aee5e4fd7a76`.
- Rusttestlog SHA256 `f7ff55d4bc8aba2862f287ed19d32bcbdb02522be59559605298a22eb33b7554`.

Closing0fixtureprocesses,0archiveproduct/resourcepipes,fixedhelperabsent. No extra realtimingrun,
resource-tool work,UI/FIFO,formalDesktop/installedhost,package/install/sign/commit/push,systemchange
or source recycle/delete API. NativeRecycle stays native_protection_unproven/NotAttempted.
**Accepted and stop: named cooperative boundaries closed; full ENG09/PhaseA/A–F remains incomplete.**
