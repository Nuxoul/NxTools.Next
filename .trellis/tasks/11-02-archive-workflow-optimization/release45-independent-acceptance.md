# Release 45 repair — independent acceptance

## Decision

**Partial acceptance.** Drag identity/title, current/overall progress, concise drag copy and UTF-16 Root-wrapper admission pass the automated/controlled-native scope. The 32 KiB output coalescer is blocked by **R45-AC01 (Medium)** at the supported multi-output failure-cleanup boundary. DOpus visible staging remains unresolved.

Scope is [the drag repair delivery](release45-repair.md), not the later incoming-Add/direct-submit/unsafe_path/notification request. Parent remains **Phase E — Format expansion and ZIP editing / in_progress**. Prior OPT scoped decisions and original Release45 observations are unchanged. No production or checked-in test source was edited, package installed, service/registry changed or physical receiver operated during this acceptance.

HEAD: `0940eec23286c1a750b3aff4284f32e76da68164`. The relevant working-tree source, not HEAD alone, is the inspected implementation. [Evidence summary](../../../.diagnostics/archive-root-drag/20261104-r45-independent-acceptance/verification.json).

## R45-AC01 — abort of an earlier output flushes the current failed member

**Severity: Medium. Blocks acceptance of extraction output coalescing.**

[PrivateOutput::call_payload](../../../rust/crates/nxtools-archive/src/worker_output.rs:565) discards pending bytes only for `abort` of `self.extracted_token`. An abort for another live token enters the ordinary non-write branch, which flushes the current member's pending tail. A failed flush sets sticky `write_failure` and returns before the originally requested abort is sent.

This is reachable through ordinary multi-file 7z extraction. [run_sevenz](../../../rust/crates/nxtools-archive/src/worker_codecs.rs:692) retains output tokens pending whole-decoder/source verification. A later member's size/CRC or decoder failure leaves multiple tokens pending; the [failure-cleanup loop](../../../rust/crates/nxtools-archive/src/worker_codecs.rs:783) aborts them in insertion order. Its first token can therefore differ from the current coalescer token.

### Reproduced boundary

A standalone scratch test crate uses a byte-identical snapshot of all 77 current archive source files, with only a test-module attachment to the copied crate root. No production implementation is substituted. The peer supplies original begin replies and exact binary ACK bindings to the real `PrivateOutput`/`ReplyPort`:

1. Begin first member A, write three valid bytes; beginning B drains and acknowledges A's bytes normally.
2. Begin member B, write a 13-byte tail that remains below the coalescing threshold.
3. Enter the same cleanup order as `run_sevenz`: `abort_file(A)`, then `abort_file(B)`.

| Case | Required | Observed |
| --- | --- | --- |
| Native cleanup-time write succeeds | Discard B's unverified pending tail; issue the existing aborts | One extra binary write of B's tail, then two aborts |
| Native cleanup-time write fails | No cleanup-triggered tail write; do not suppress A's abort because another member's tail failed | One extra write; only B's abort reaches the peer |

The two assertions fail on unchanged production code: [probe](../../../.diagnostics/archive-root-drag/20261104-r45-independent-acceptance/probe/probe.rs), [compiled failure log](../../../.diagnostics/archive-root-drag/20261104-r45-independent-acceptance/r45-cross-member-abort-red.log). They are two manifestations of one finding, not two independent defects. The copied coalescer and production coalescer were compared directly before recording the finding.

[The checked-in abort regression](../../../rust/crates/nxtools-archive/src/worker_output_bulk_tests.rs:475) aborts only the active token, so it cannot detect this multi-token path. Add the real multi-output failure pattern to that regression boundary and retain the format-level failure/cleanup preservation test.

Repair the original failure-cleanup/coalescer boundary so abort/discard does not drain another failed member or allow sticky write failure to suppress required cleanup calls. Keep the first business/native failure, normal successful member barriers, exact ACK order, cancellation and native publication/actual cleanup authority. Do not introduce a second output owner, retry layer or resource increase.

The native [ArchiveOutputSession::Dispose](../../../csharp/NxTools.PlatformHost/Windows/Archive/ArchiveOutputSession.cs:682) still attempts cancellation of every unpublished entry. This acceptance did not reproduce final-file publication, permanent leakage or data loss. The confirmed defect is the added write and skipped explicit abort during Rust failure cleanup; the native disposal fallback does not make the promised Abort/discard behavior correct.

## Accepted scoped behavior

| Repair | Independent evidence | Decision |
| --- | --- | --- |
| R45-UI01 drag title | Explicit `new_drag`/shared `DRAG_TITLE`; original serialized gesture/event route renders `拖出解压`; ordinary Test title preserved | Pass, automated scope |
| R45-UI02 dual progress | Real worker publishes nonzero current bytes before Seal; demand request/index/ordinal and operation/session/generation guards; publication sequence filtering; actual rendered independent current/overall bars, reset/unknown totals and cache-ready versus receiver-complete states | Pass, automated and controlled-native scope |
| R45-UI03 concise copy | Actual drag view excludes cache-boundary/close prose and renders concise lifecycle states | Pass, automated scope |
| R45-N01 name units | Real Rust serialized long-CJK Root wire reaches C# parser/plan/native descriptor HGLOBAL unchanged; original ordinal and complete-path limits retained | Pass, automated and controlled-native scope |
| 32 KiB output optimization | Nominal and single-active-token regressions pass; multi-token cleanup probe fails | Not accepted pending R45-AC01 |
| DOpus visible scatter/collect | Original wrapper-first topology is preserved; added diagnostics do not resolve the visible transition | Open, no physical replay |

The original deferred cross-entity updates, demand-only creation, hidden → no-activate → final bounds → show sequence, one source-HWND/capture/pressed OLE owner and target-held IStream lifetime remain in the inspected paths. Official Component0.8.0 Progress controls are reused. Compiled geometry and controlled COM are not user visual/DPI/physical acceptance.

## Fresh verification

All Cargo invocations used one build job, incremental disabled, codegen-units256 and offline mode. No duplicate Cargo build was started by this acceptance. C# fixture effects were inspected; the final exact LIST and RUN selections are identical.

| Boundary | Fresh result |
| --- | --- |
| Output unit tests | 7 pass |
| Worker progress | 5 pass |
| Drag protocol / IPC | 13 / 3 pass |
| Actual-process bulk / extraction contracts | 8 / 10 pass |
| Desktop original ingress, ordinary progress, preview and Root name | LIST20 / RUN20 / PASS20 / ignored0 |
| C# controlled progress/async/name/preview | LIST7 / RUN7 / PASS7 / SKIP0 / FAIL0 / not_run0 |
| Additional production-boundary failure probe | RUN2 / PASS0 / FAIL2, same R45-AC01 |
| C# Debug build | 0 warnings / 0 errors |
| Strict Clippy | Archive isolated-codecs lib, Protocol lib, Desktop bin pass with `-D warnings` |
| Source/document gates | spec125 documents/14 owners; GPUI release; pinned Markdown editor; Root structure3/3 pass |

Distinct fresh checked-in passes: **66 Rust + 7 C#**. Zero-selection commands are excluded from those counts. [Rust core](../../../.diagnostics/archive-root-drag/20261104-r45-independent-acceptance/r45-acceptance-rust-core.log), [worker](../../../.diagnostics/archive-root-drag/20261104-r45-independent-acceptance/r45-acceptance-worker-tests.log), [Desktop](../../../.diagnostics/archive-root-drag/20261104-r45-independent-acceptance/r45-acceptance-desktop.log), [native LIST](../../../.diagnostics/archive-root-drag/20261104-r45-independent-acceptance/r45-acceptance-native-exact-list.log), [native RUN](../../../.diagnostics/archive-root-drag/20261104-r45-independent-acceptance/r45-acceptance-native-run.log), [Clippy](../../../.diagnostics/archive-root-drag/20261104-r45-independent-acceptance/r45-acceptance-clippy.log), [source gates](../../../.diagnostics/archive-root-drag/20261104-r45-independent-acceptance/r45-acceptance-source-gates.log).

## Performance evidence audit

All six retained raw reports and 20 operation rows were independently recomputed. Summary fields match their original work, native terminal, cache and receiver records; per-route source/catalog work and output bytes agree. The ZIP central metadata independently predicts exactly 6,436 records when each file is split into 32 KiB pieces. Each retained Root operation has 208 Seal and 208 cursor checks over 206,572,163 bytes. Native output counters retain 416 full reads / 413,144,326 bytes per operation; these are not total source/disk I/O.

| Series / route | Baseline mean | 32 KiB mean | Change |
| --- | ---: | ---: | ---: |
| Sequential ordinary Extract | 3,191.11 ms | 2,987.97 ms | 6.37% lower |
| Sequential controlled Root | 7,133.32 ms | 7,442.11 ms | 4.33% higher |
| ABBA ordinary Extract | 4,014.97 ms | 3,251.30 ms | 19.02% lower |
| ABBA controlled Root | 8,106.60 ms | 7,414.17 ms | 8.54% lower |

The reported ABBA improvements and record reduction **16,980 → 6,436** are numerically supported. Both series reduce IPC/ACK time; the sequential Root wall result does not support a stable end-to-end improvement. Inclusive decoder/read/IPC/commit/cache/receiver intervals are not summed. These observations do not override R45-AC01 or establish Bandizip parity.

[Independent audit](../../../.diagnostics/archive-root-drag/20261104-r45-independent-acceptance/r45-independent-measurement-audit.json) and [audit program](../../../.diagnostics/archive-root-drag/20261104-r45-independent-acceptance/audit-r45-measurements.py). The original measurement fixture's independent .NET ZIP payload comparisons and actual cleanup assertions were source-reviewed; this acceptance did not rerun the 20 performance operations or independently rebuild the historical baseline helper.

## Remaining acceptance

- Fix R45-AC01, add the multi-token regression at the original boundary and rerun the affected output/7z cleanup tests before accepting the output-coalescing slice.
- DOpus visible staging, physical pointer/focus/DPI, ordinary button completion-window feedback, actual Bandizip runtime branch and installed Release performance remain separate, unverified scopes.
- The later direct incoming Add, unsafe_path and unified failure-notification requirements were not delivered by this attachment and are not accepted here.
- No complete G-00/G-04/G-18, parent Phase E or Release closeout follows from this scoped decision.

## Directed R45-AC01 closure

[The directed independent acceptance](release45-ac01-independent-acceptance.md) closes R45-AC01 at the original output boundary after the unchanged two-case probe and three checked-in cleanup regressions pass. This report's original finding and failing evidence remain historical. Complete output-slice acceptance remains partial because the native7z low-budget category assertion still fails and blocks its later preservation cases; physical and installed Release scopes remain open.

## Automated and owned-native repair closeout

[Native7z directed acceptance](release45-sevenz-limit-independent-acceptance.md) closes the remaining error-category/preservation blocker after30 fresh Rust cases and the full ten-case native selection pass. R45-AC01's directed closure and prior UI/name scoped acceptance stand. No directed automated/owned-native blocker remains; DOpus physical staging, installed-release gates and prior performance limitations are not silently closed.
