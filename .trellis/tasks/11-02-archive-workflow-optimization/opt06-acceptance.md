# OPT-06 — Implementation verification and independent handoff

## Status

**OPT-06 — Bounded binary bulk transfer** is implemented and verified within the authorized automated/owned-native boundary. Independent acceptance is pending. The archive remains **Phase E — Format expansion and ZIP editing**; the overall task remains `in_progress`. OPT-02/03/04/05 retain independent scoped acceptance and O5-AC01/02 remain closed. OPT-07 is not started.

Contracts: [private frames](opt06-binary-transfer.md), [executable owner spec](../../spec/nxtools-archive/backend/binary-transfer.md), [saved reference chain](../../../docs/reference-code/archive.md#opt-06-bounded-binary-bulk-transfer). Measurements: [results](opt06-measurements.md), [18 raw samples](opt06-measurements.json). No new public IPC, dependency, format/backend, Job runtime, scheduler or UI surface.

## Product call chains and invariants

| Route | Original owner chain |
| --- | --- |
| Create/Add source payload | Original Job/Run -> `worker_sources::Sources::read_file` -> sequenced JSON `source_read` -> C# `ReadBulkRequest` -> original `ArchiveCreationSources.SourceReadBytes` protected handle -> raw kind2 reply -> original format encoder |
| Extract/retained output | Original format/retained controller -> `PrivateOutput::write_file` -> borrowed kind1 frame -> C# original stdout owner -> `ArchiveWorkerOutputBridge` token/offset/budget -> original `ArchiveOutputSession.Write` -> exact typed ACK |
| Held candidate readback | Original candidate/created validation -> `PrivateOutput::read_partial` -> sequenced JSON `read` -> original session token/read -> raw kind3 reply -> complete original integrity/encoded-equality/native completion checks |
| Output readmission | `product_job::execute_with_output_readmission` builds the same negotiated private schema2 request -> original finite `output_readmission::execute` -> unchanged typed Job callback -> actual native exited/cleanup terminal |

One pending call supplies backpressure. The operation-wide sequence survives new tokens, attempts and retained members; exact operation/call/token/offset/request length/kind/sequence is required. Metadata/source-close/finish/commit/member-ready cannot overtake data acknowledgement. Pause/cancel/EOF wake original waiters; first native failure and committed/unknown/late-cancel truth remain authoritative. Host raw replies briefly own native bytes plus one envelope; writes use the received body. Payloads are <=64KiB and each active direction retains <=2 transport payload buffers. The fixed1MiB reservation is inside existing64MiB overhead; original process/codec/work/output/concurrency/deadline ceilings remain.

Modern schema2 data operations require `bulk_transfer:1` and unique exact `binary_bulk_v1`; negotiation failure precedes payload/output effects. Schema1 ZIP, read-only operations and explicit HZ5 retain separate contracts. Existing modern test peers explicitly negotiate and adapt to raw frames without weakening their original integrity/publication assertions.

Full source freshness, CRC/MAC/actual-size/codec end, metadata/MOTW, removed/replaced source payload responsibility, complete final candidate, exact held readback, five fresh native stages, single-use reader completion, FILE-only handoff, one publication/postverification and actual cleanup remain unchanged. Retained member-ready is not Run cleanup; preview shares, transferred cursors, source-set protection and independent encrypted-drag authorization retain their original lifetimes.

## Effective regression evidence

All logs below are retained under session scratch `D:\.pi-desktop\data\scratch\a2072588-1f47-4232-be7a-3951a25df615`. Compiler/setup failures are not behavioral RED.

| Boundary | Effective RED | GREEN |
| --- | --- | --- |
| Actual Rust worker negotiated output | `opt06-worker-negotiation-red.log`: JSON-only worker does not provide negotiated raw frame | First actual write control, then `opt06-worker-symmetric-green.log`:8/8 groups,35 process scenarios |
| Actual Rust wrong-operation ACK | `opt06-worker-symmetric-first.log`: protocol refusal incorrectly becomes Cancelled | Wrong ACK/stale duplicate group returns failure/zero commit in symmetric GREEN |
| Original Rust rendezvous native failure priority | `opt06-worker-native-priority-red.log`: accepted source_changed is lost to later cancel | `opt06-worker-bulk-unit-green.log`:5/5 literal/binding/priority tests; this helper evidence complements actual pipes/native fixtures |
| Actual C# original Run raw output | `opt06-csharp-red-run.log`: raw frame hits stdout_frame_limit after successful build | `opt06-csharp-integrated-run.log`: exact13/13,0skip |
| Actual C# missing required capability | `opt06-csharp-cap-red-run.log`: missing capability reaches success | Current capability selection refuses before output namespace mutation |
| Original product readmission conversion | `opt06-worker-preservation-remaining.log`: original adapter rejects its unnegotiated private request | `opt06-product-readmission-green.log`: original finite two-attempt regression passes |

## Completed checks

| Selection | Executed result | Evidence |
| --- | --- | --- |
| Rust worker-related library tests |28PASS/0fail/0ignored; includes5new literal/binding tests | `opt06-worker-unit-preservation.log` |
| New actual Rust worker/private peer |8PASS/0fail/0ignored,35process scenarios | `opt06-worker-symmetric-green.log`; repeated within current directed integration |
| Directed Rust integration contracts |95distinct tests passed across27targets; overlapping reruns are not added | `opt06-worker-preservation-integrated.log`, `opt06-worker-add-sevenz-preservation.log`, `opt06-worker-preservation-remaining.log`, `opt06-product-readmission-green.log`, `opt06-worker-preservation-rest.log`, `opt06-worker-final-readmission-options.log`, `opt06-worker-tar-plan-preservation.log` |
| C# new wire and original legacy compatibility |Exact LIST13/RUN13/PASS13/SKIP0/fail0/not_run0 | `opt06-csharp-integrated-{list,run}.log` |
| Actual matching Rust/C# original native routes |Exact LIST5/RUN5/PASS5/SKIP0, then6/6 and2/2; each fail0/not_run0 | `opt06-native-integration-{list,run}.log`, `opt06-native-preservation-{list,run}.log`, `opt06-native-split-encrypted-{list,run}.log` |
| Principal before and after measurements |Each exact LIST1/RUN1/PASS1/SKIP0/fail0/not_run0;9complete Jobs per side | `opt06-before-measure-{list,run}.log`, `opt06-delivery-measure-{list,run}.log` |
| C# final Debug build |0warnings/0errors | `opt06-delivery-csharp-build.log` |
| Rust strict lint and formatting |Archive isolated-codecs all-target Clippy `-D warnings`; scoped rustfmt37files | `opt06-clippy-final.log`, `opt06-rustfmt-final.log` |
| Source/document gates |Spec123documents/14owners; locked Kit/editor; CLI16assertions; tree-root-drag3tests; Phase F source boundary | `opt06-static-gates.log` |

Rust integration targets include worker framing/extraction/cancellation, Create ZIP/7z/TAR/streams, ZIP/7z/TAR edit/readback/source failure, original Add conflicts/results/all-Skip qualification, metadata, finite readmission and seven-format options. The95count includes typed/in-process contract cases; it does not mean95native publications or independent scenarios. Pure reclamation/library controls do not execute destructive HZ5/native effects.

The actual native5selection covers Gzip creation, empty Gzip/XZ create/read/extract, creation safety/candidate readback, production terminal success/established-lease cancel/throwing sink, and retained idle/work/Seal/process/EOF lifetime. The6selection covers metadata/MOTW, corrupt full-block refusal before Ready,12retained demand-order scenarios across solid7z/RAR5/TGZ/TXZ, preview adoption/consumer lifetimes, and original TAR/7z failure/unknown/no-retry/late-cancel handoff. The2selection covers genuine split ZIP creation/source-set/retained reads and pinned encrypted solid RAR4 preview versus separate drag authorization. Every selected fixture asserts real output or refusal plus original actual cleanup; none uses a real Explorer/foreign OLE target.

## Comparable results

| Complete original route | Before median ms [min–max] | After median ms [min–max] | Bidirectional wire median before -> after |
| --- | ---: | ---: | ---: |
| ZIP Store Create |691.335 [601.668–972.028] |183.681 [183.278–335.040] |6,789,553 ->3,172,163bytes |
| ZIP Extract |542.544 [517.313–571.784] |246.609 [244.524–279.812] |3,755,971 ->1,059,473bytes |
| ZIP Add |1,260.297 [1,209.646–1,315.571] |495.964 [494.115–498.746] |12,054,485 ->5,280,000bytes |

All18completed samples preserve exactly equal source/output/partial payload calls/bytes and native source/target/output actual/full/completed read counts across corresponding trials. Add retains source5/target2/output2 full reads. Each sample has one publication/Committed journal, actual cleanup/zero active Runs, exclusive reopen, exact independent decode/source preservation and no owned partial. Nine after samples have zero observed modern conversion input. Host payload high-water is2/131072bytes for Create/Add and1/65536bytes for Extract. Raw counts, all trials, CPU/memory samples/native sampling failures and unavailable fields are in the measurement JSON.

## Frozen source and binary attribution

- HEAD remains `0940eec23286c1a750b3aff4284f32e76da68164`; no commit, reset or unrelated-tree cleanup.
- `opt06-original` retains pre-slice source/pair. `opt06-before` retains observed-before source/Debug pair; its immutable `reports/before.json` is complete9samples.
- Final `opt06-delivery/source` and `opt06-delivery/Debug` retain Rust source/tests, C# source/tests, Host/test apphost/dependencies and worker. Worker12,376,576bytes; Host DLL2,186,752bytes; test DLL3,745,792bytes. `pair.json` records mode/build parameters and artifact sizes/timestamps.
- Measurement input/root pointer: `opt06-measure-root.txt`; actual worker execution stays at that root's `NxTools.ArchiveWorker.exe`. After report is `reports/after-complete.json`. Inputs/stable route paths and both completed raw reports remain intact.
- One bounded read-only correctness pass found no actionable defect in negotiation, framing, binding, ordering, bounded buffers or error precedence. This is implementation review, not independent acceptance.

## Independent handoff and open gates

Review G-14 and touched G-01/G-02/G-04/G-07/G-19/G-23/G-25 against the owner spec, current source, saved effective REDs, exact selections and completed raw measurements. Use only reviewed GUID-owned fixtures and matching frozen worker; LIST must precede an identical RUN, with nonzero selection and zero skips. All Cargo/direct Rust invocations retain jobs1/incremental0/codegen-units256/offline and one build lane. Do not overwrite completed evidence or user sources.

G-00, OPT-00/01 overall, complete G-04, overall Phase E and physical/Release remain open. Complete/internal source I/O, exclusive codec/transport CPU, exact process memory peaks and separately timed cleanup remain unavailable; process sampling is not exclusive performance attribution. No reference execution, product GUI/Explorer/system clipboard/picker, user archive/source mutation/recycle/HZ5, service/registry, packaging/signing/install or commit/push was performed. Stop at OPT-06; OPT-07 remains unstarted.
