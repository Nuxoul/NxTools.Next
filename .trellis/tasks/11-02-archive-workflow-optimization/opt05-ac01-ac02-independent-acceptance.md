# OPT-05 — O5-AC01/02 directed independent acceptance

## Decision

**Accept OPT-05 — Reusable format and update plans at its authorized automated/owned-native boundary. O5-AC01 and O5-AC02 are closed.** The prior accepted G-03/G-05/G-24 and touched G-04/G-19 functional evidence is retained; the two diagnostic/regression conditions identified by the original review now pass independent directed verification. No remaining in-scope repair blocker was confirmed.

The [original partial verdict](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/opt05-independent-acceptance.md), [repair evidence](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/opt05-ac01-ac02-repair.md), delivery records and measurement tables retain their original attribution. This report supplies the current directed acceptance decision. Overall task status remains `in_progress`, under **Phase E — Format expansion and ZIP editing**.

Scope: the O5-AC01 production terminal projection in `ArchiveWorkerRunner`, its instance-only cancellation seam and actual terminal fixture, and the O5-AC02 checked-in all-Skip worker assertions. HEAD is `0940eec23286c1a750b3aff4284f32e76da68164`. The review changed acceptance documents only; product/test source and unrelated workspace changes were preserved.

## O5-AC01 — closed

[DiagnosticTerminal](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerRunner.cs:1023) combines the existing `Run.Source.ReadObservation` and primary output observations by Source/Target/Output role. Each role exports attempts, full reads, actual returned bytes and completed bytes; held-partial readback has separate calls/bytes. Aggregation is bounded by the fixed observation table and adds no file reads or source authority.

The [original terminal boundary](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerRunner.cs:643) resolves cleanup/publication/category facts before projecting the same terminal to diagnostics and the event consumer. Counts survive owner disposal. Using `Run.Source` retains established-lease observations when verification fails before bridge binding. Projection and sink exceptions remain isolated from the business result.

The [actual worker terminal fixture](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost.Tests/Opt05NativeObservationTests.Terminal.cs:91) asserts the production sink, correlation, elapsed/native code, commit/cleanup facts and exact role counts. It does not substitute test snapshots for production diagnostic evidence. Independent success/cancellation/rejecting-sink runs produced:

| Case / role | Attempts | Full reads | Actual bytes | Completed bytes |
| --- | ---: | ---: | ---: | ---: |
| Success / source | 5 | 5 | 1,311,330 | 1,311,330 |
| Success / target | 2 | 2 | 524,532 | 524,532 |
| Success / output | 2 | 2 | 524,608 | 524,608 |
| Cancel EditVerification / source | 2 | 1 | 327,802 | 262,266 |
| Cancel EditVerification / target | 1 | 1 | 262,266 | 262,266 |
| Cancel EditVerification / output | 0 | 0 | 0 | 0 |

Original source size was262,266bytes; published candidate size was262,304bytes. Success held-partial readback was6calls/262,304bytes, including EOF; cancelled readback was zero. The interrupted source read contributed65,536 actual bytes without another full-read credit. Success and rejecting-sink modes each published once; cancellation published nothing and preserved the source. All modes completed Run drain, retired owned partials and permitted exclusive reopen. The rejecting sink retained no records and did not change successful publication or cleanup.

The retained pre-export RED log was inspected: both success and established-lease cancellation failed the source-role counter assertion, while business and cleanup outcomes remained valid. This round independently reran the current fixture and reconciled all three saved production terminal packages. The [role-separated diagnostic contract](D:/Code/NxTools.Next-main/.trellis/spec/platformhost/archive-read-observation.md:26) is satisfied.

## O5-AC02 — closed

The [checked-in all-Skip assertions](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/tests/fixtures/eic-edit-repair.py:139) require fresh eligibility exactly once, one structural check per original record and zero payload decodes. Original completion/prompt/summary, unchanged-source, zero incoming payload open, zero candidate and zero publication assertions remain.

Independent execution used the current fixture and current worker, then the retained single-call mutant worker in fresh owned directories:

| Worker / directory representation | Business terminal | Eligibility | Local / descriptor checks | Begin / commit | Source unchanged | Fixture result |
| --- | --- | ---: | --- | --- | --- | --- |
| Current / implicit | complete | 1 | 2 / 2 | 0 / 0 | Yes | PASS |
| Current / explicit | complete | 1 | 3 / 3 | 0 / 0 | Yes | PASS |
| Qualification-bypass mutant / implicit | complete | 0 | 0 / 0 | 0 / 0 | Yes | Expected fresh-qualification assertion failure |
| Qualification-bypass mutant / explicit | complete | 0 | 0 / 0 | 0 / 0 | Yes | Expected fresh-qualification assertion failure |

All four had zero member/source/addition/candidate payload decodes and one Skip prompt. Correct/mutant saved source comparison confirmed that the mutation removes only the no-candidate fresh qualification call. Current `worker_zip_edit.rs` exactly matches the saved correct source; the saved Python fixture exactly matches the checked-in fixture. No product mutation was performed by this review. The regression now turns red at the actual worker boundary when its required qualification is absent.

## Independent verification

| Check | Observed result |
| --- | --- |
| `cargo test -p nxtools-archive --features isolated-codecs --offline --test eic_repair_worker_contract -- --test-threads=1 --nocapture` |4/4PASS,0ignored; includes both all-Skip forms, real additions and existing cancellation/native-error semantics. |
| Direct current/mutant all-Skip control |Current2/2PASS; mutant2/2expected assertion failures, with business completion/zero publication/source preservation verified separately. |
| C# Debug test-host build, `--no-restore -m:1 -p:UseSharedCompilation=false` |0warnings/0errors. |
| Effects-reviewed actual apphost selection |LIST9/RUN9/PASS9/SKIP0/FAIL0/not_run0, identical nine name filters. |
| Production terminal reconciliation |Success, established-lease cancellation and rejecting sink agree with resolved results, exact role bytes and actual lifecycle. |
| Spec/locked dependency contracts |PASS:122spec documents/14owners; Kit0.8.0/pre0.3.9; pinned editor57files/16serializer cases. Source/provenance scope. |

The nine native selections were the original worker terminal diagnostics and operation isolation, OPT05 identity/version/eight interrupted-read controls, the new actual worker terminal observations, and five ZIP held-source/same-path-cancellation/changed-revision/handoff/failure-unknown-no-retry fixtures. Exact LIST preceded identical RUN. No skipped or zero-selection result is counted as acceptance.

Cargo used one lane, offline mode, scratch TEMP/TMP and `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256` in the same invocation with a2400-second timeout. Native runs used the freshly built test apphost and an explicit current matching worker frozen at `opt05-ac-independent/Debug/NxTools.ArchiveWorker.exe`, SHA256 `BF2968F19474ED32EA3F4C71BBAD9B9F55FF5E7D354B9833EF74A4D4E27B68BF`.

Evidence root: `D:\.pi-desktop\data\scratch\aab2f188-352d-4f1c-b7a3-5375fe0df2d8`. Logs use `opt05-ac-independent-*`. Production terminal reconciliation is `opt05-ac-independent-terminal-audit.json`; its packages are `archive-product-opt05-terminal-dd4b9769-c51e-4072-905b-cfa0ed5a485b`, `archive-product-opt05-terminal-ac852f7c-5a9e-405d-9716-c32982feb176` and `archive-product-opt05-terminal-cae94503-6eba-44a5-a5d6-f33068189bd7`. The all-Skip control is `opt05-ac02-independent-f174974e-5ed5-46a4-99d7-d727783b94b1/independent-verification.json`.

## Limitations and stop point

This is directed acceptance of O5-AC01/02, not another complete format/performance review. Earlier green engine/Desktop/protocol/native results and the audited240raw samples/118summaries retain their prior scope. Counts across rounds overlap and are not added. No new performance trial, universal speedup or total source/disk I/O claim is introduced; unobserved total/internal I/O and peak memory remain unavailable.

G-00, OPT-00/01 overall closeout, complete G-04, overall Phase E and physical/Release acceptance remain open. OPT-06/07 remain unstarted. The original native freshness, single-use read completion, FILE-only handoff, publication precedence and actual cleanup contracts remain unchanged. No real GUI/Explorer/clipboard/picker, user archive mutation/recycling/HZ5, service/registry, packaging/signing/install, commit or push occurred.
