# Release 45 — R45-AC01 directed independent acceptance

## Decision

**R45-AC01 is closed at the directed original-output boundary. The complete output-coalescing slice remains partially accepted.** The unchanged original acceptance probe now passes, and the checked-in multi-token cleanup regressions cover both original manifestations plus a preexisting sticky write failure. The native 7z preservation fixture still fails its low-output-budget error-category assertion.

This decision covers [the AC01 repair](release45-ac01-repair.md), not a new drag/UI/performance acceptance or the later direct incoming-Add/unsafe_path/notification request. [The original independent report](release45-independent-acceptance.md) and its failing evidence retain their historical scope. Parent remains **Phase E — Format expansion and ZIP editing / in_progress**.

HEAD remains `0940eec23286c1a750b3aff4284f32e76da68164`. No production or checked-in test source was modified by this acceptance; only independent records and owned test evidence were created. No user archive, service/registry, physical GUI/DOpus/Bandizip, package/install or commit/push operation was performed.

## Verified production delta

Direct comparison against the original acceptance's 77-file archive snapshot finds exactly two changed source files: `worker_output.rs` and `worker_output_bulk_tests.rs`.

The only production change is [PrivateOutput::call_payload](../../../rust/crates/nxtools-archive/src/worker_output.rs:565): every `abort` now discards pending bytes/current-token staging before entering the original control checkpoint and native-call path. It no longer requires the aborted token to equal the current staged token. The existing `write_failure` remains unchanged; ordinary non-abort operations still return that first failure, while abort can perform cleanup without flushing or replaying the failed member.

The successful begin/metadata/source-verification/commit barriers, exact token/sequence/ACK, resource limits and original 7z forward cleanup order are unchanged. The binary-transfer owner contract describes this explicit failure-discard boundary. The C# fixture's diagnostic text now distinguishes synthetic sample and expected/actual result without changing its assertions.

## Independent regression evidence

The original two-token acceptance probe was copied without modification and bound to a current 77-file source snapshot. Its two assertions, previously red, now both pass:

| Original failure | Fresh observation |
| --- | --- |
| `abort(A)` flushes B's pending 13-byte tail | Cleanup writes = 0; A and B both reach abort |
| Rejected cleanup-triggered B write suppresses A's abort | No cleanup write occurs; A and B both reach abort |

[Unchanged probe](../../../.diagnostics/archive-root-drag/20261104-r45-ac01-independent-acceptance/probe/probe.rs), [fresh Rust/probe log](../../../.diagnostics/archive-root-drag/20261104-r45-ac01-independent-acceptance/r45-ac01-independent-rust.log), [original red evidence](../../../.diagnostics/archive-root-drag/20261104-r45-independent-acceptance/r45-cross-member-abort-red.log).

The three [checked-in regressions](../../../rust/crates/nxtools-archive/src/worker_output_bulk_tests.rs:660) exercise actual begin/write/ReplyPort/abort with two nonempty members. The added sticky-failure case acknowledges a failed 32 KiB B write, checks that both aborts remain callable, then checks that commit still returns the original IoFailure with no payload replay. The retained [three-case red log](../../../.diagnostics/archive-root-drag/20261104-r45-ac01/r45-ac01-red.log) fails on the intended extra-write/missing-abort assertions; the fresh current run passes all three.

| Fresh check | Result |
| --- | --- |
| Complete output unit boundary, including the three new cases | 10 pass |
| Actual-process bulk transfer | 8 pass |
| Actual-worker extraction contracts | 10 pass |
| Unchanged original acceptance probe | 2 pass |
| Archive strict Clippy, isolated-codecs/tests | Pass, `-D warnings` |
| Scoped rustfmt | Pass |
| Final Debug worker build | Pass |
| C# Debug build | 0 warnings / 0 errors |
| Spec contracts | Pass, 125 documents / 14 owners |

Fresh coverage is **28 distinct checked-in Rust cases plus 2 unchanged independent probe cases**. Every Cargo command used offline mode, one job, incremental disabled and codegen-units256. [Build log](../../../.diagnostics/archive-root-drag/20261104-r45-ac01-independent-acceptance/r45-ac01-independent-csharp-build.log), [lint/format/spec log](../../../.diagnostics/archive-root-drag/20261104-r45-ac01-independent-acceptance/r45-ac01-independent-gates.log), [verification summary](../../../.diagnostics/archive-root-drag/20261104-r45-ac01-independent-acceptance/verification.json).

## Native 7z preservation blocker remains open

Effects-reviewed exact selection: `archive extraction actual Rust 7z handle-session fixture`. It uses the freshly built isolated-codecs worker, attributed repository 7z samples, GUID-owned temporary targets and the original runner/native output. The temporary test-app worker slot was empty before execution and removed afterward; no protected installation was modified.

Fresh result: **LIST1 / RUN1 / PASS0 / FAIL1 / SKIP0 / not_run0**. At [the low-budget call](../../../csharp/NxTools.PlatformHost.Tests/ArchiveExtractionWorkerTests.cs:108), `lzma.7z` with `max_output_bytes=1` still expects `ResourceLimit` but receives `worker_failure/IoFailure`, no native error code, and `CleanupComplete=true`.

[Exact LIST](../../../.diagnostics/archive-root-drag/20261104-r45-ac01-independent-acceptance/r45-ac01-independent-native-list.log), [fresh RUN](../../../.diagnostics/archive-root-drag/20261104-r45-ac01-independent-acceptance/r45-ac01-independent-native-run.log), [retained old/new comparison](../../../.diagnostics/archive-root-drag/20261104-r45-ac01/r45-ac01-native-comparison.log).

The sequential fixture reaches this seventh Verify call only after its preceding plain, solid, non-solid, encrypted, missing-password and wrong-password checks. Its later existing-target collision and plain/encrypted corruption cases are not executed. The low-budget case's output-count/sentinel assertions also follow the failing category assertion and are not claimed as passed.

The retained comparison records the same mismatch for workers labeled `baseline-before-coalescing` and `final-ac01`. This acceptance independently confirms the current failure; it does not independently reconstruct that historical binary pairing or completely diagnose the failure's origin. Neither identical failures nor cleanup=true convert the fixture into a pass. No expected category, assertion, resource budget or production mapping was changed during this acceptance.

## Acceptance boundary

- **Closed:** R45-AC01's extra cleanup write and suppressed earlier abort, including sticky-first-failure preservation, at the original production output boundary.
- **Open:** the native 7z low-budget category mismatch and the format-level preservation cases hidden behind its fail-fast stop. Resolve or causally isolate that blocker without weakening assertions, then execute the remaining preservation cases before signing off the complete output slice.
- Prior title/dual-progress/copy/name scoped decisions stand; physical DOpus staging, installed Release behavior and actual Bandizip runtime comparison remain unverified.
- No performance trials were added. The prior sequential/ABBA measurements and their variability retain their original scope.
- No full G-00/G-04/G-18, parent Phase E or Release closeout follows from this directed closure.

## Native preservation directed closure

[The subsequent independent acceptance](release45-sevenz-limit-independent-acceptance.md) closes the native7z low-budget category blocker: current ResourceLimit mapping,30 Rust regressions and all ten original native7z cases pass. R45-AC01 remains closed. This document retains the earlier failed preservation result as historical evidence; physical and installed Release acceptance remain separate.
