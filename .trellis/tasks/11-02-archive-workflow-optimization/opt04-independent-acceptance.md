# OPT-04 — independent acceptance

## Decision

**Accept OPT-04 — Drag-scoped retained materialization at the authorized automated/owned-native stop point.** G-07/G-08/G-09/G-10/G-27 and touched G-19 pass the independently tested lifecycle, format-integrity, cache/authorization and preview boundaries. No in-scope correctness or ownership repair blocker was confirmed.

This is not complete archive performance or physical acceptance. G-00, OPT-00/01 overall closeout, complete G-04, Phase E and Release remain open. OPT-05 through OPT-07 remain not started. Historical speed measurements retain their delivery attribution; this review independently verifies current behavior and current timings, not a historical before/after speed ratio.

Scope: OPT-04 retained/preview working-tree changes against HEAD `0940eec23286c1a750b3aff4284f32e76da68164`, including the new Rust drag/format helpers and C# virtual operation/plan helpers. Prior accepted OPT-01/02/03 and unrelated workspace changes are preserved. No product/test source was modified by this review.

References: [requirements](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/prd.md), [ordered work](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/implement.md), [acceptance](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/acceptance.md), [delivery and recorded timings](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/opt04-retained-materialization.md).

## Standards

**The accepted scope retains the original runtime and data-protection owners.**

- Rust's private negotiated demand protocol binds operation, request ID, immutable descriptor index and original ordinal, with one outstanding demand and finite actual output limits. Closing/cancellation does not start replacement workers. Original source, codec and format owners perform the work.
- Native [VirtualMaterialization](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Protocol/ArchiveEntrySession.Virtual.cs) separates member readiness from overall cleanup. The original [runner](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerRunner.Virtual.cs) matches demand/output/publication facts and confirms readiness only after Seal under its failure/deadline gate. No public cleanup=true terminal is fabricated while the worker remains alive.
- Native process/Job/pipes/source/output resources and local/global Run reservations retire only at actual drain. Streams already transferred to consumers remain usable across source/session retirement; their consumer lifetime is not replaced by worker lifetime.
- The once-admitted [ArchiveVirtualPlan](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveVirtualPlan.cs) and session activity reservation do not replace actual worker source acquisition. Cache shares stay in the original cache owner and each actual cursor retains full object/content validation.
- Desktop Ready holds its exact original preview lease. [cancel_preview/complete_preview](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/entry.rs:605) retire, transfer or release it through existing generation/selection/close/disconnect paths; stale or owner-lost completion does not publish obsolete content. No new UI layout or component runtime is introduced.

### Remaining format costs

These supported costs are real and remain visible; they are not grounds for removing integrity checks or widening source authority in this acceptance.

| Path | Confirmed cost and boundary |
| --- | --- |
| Split ZIP | The retained normalized source/catalog is reused, but [multi-volume requests do not receive the ordinary single-source NativeReadLease](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/worker.rs:1153). [Per-member conservative revision checks](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/formats/zip.rs:736) can reread all external volumes. Both two-member forward/backward runs recorded40 revision passes across the volume set, despite one worker/source and two admission catalog passes. One retained source acquisition is not constant total source I/O. |
| TAR/compound TAR | The verified spool and validated records survive the operation, but [extract_validated](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/formats/tar.rs:452) rebuilds a full ordinal HashMap for each demand. This is demand-count × catalog-count CPU/index work; full-byte/spool reuse does not eliminate it. |
| 7z | The locked public consuming BlockDecoder traverses the complete required block for each distinct member demand. Two members of the tested solid block cause two real block invocations, not one resumable decode. |
| Solid RAR | Backward/nonmonotonic demands reopen/replay valid TEST state and predecessors. A retained process/catalog does not make arbitrary decoder history reusable. |

PERF-02's revision shortcut is scoped to actual **ordinary** held-source authority; the current split contract explicitly keeps external-volume revision binding unchanged. Validated TAR records are immutable format evidence checked against the snapshot, not a second mutable business state. No supported metadata-budget breach was established. General single-pass indexes/record plans belong to OPT-05/PERF-05; safe multi-volume observation consolidation depends on equivalent original volume-source evidence. These costs remain part of future performance measurement rather than unreported one-pass claims.

## Spec

**The required demand-started and consumer lifecycle behaviors pass within scope.**

| Gate | Independently confirmed boundary |
| --- | --- |
| G-07 | Descriptor/query-only cancellation starts no worker or output. ZIP1/2/16 distinct demands use one worker/source/catalog, repeated indices do not decode again, first ready has an occupied Run and no worker terminal, and actual terminal follows receiver end/drain. No undemanded filesystem outputs. |
| G-08 | Split ZIP forward/backward,12 solid7z/RAR5/TGZ/TXZ order cases, solid encrypted RAR4, duplicate/empty/multiple-block representation and required-block corruption are covered by actual formats or preserved fixtures. Correct original ordinals, independent expected bytes and no predecessor output remain. |
| G-09 | Six actual lifetime modes cover31s receiver idle, member write timeout, Seal timeout, Seal cancellation, owned worker exit and stdin EOF. Old async Start/End/slow-read/abort/close/disconnect/COM ABI and transferred-stream lifetime remain. No failed/expired member returns a medium. |
| G-10 | Ordinary preview hit/mismatch/retired/mixed adoption plus encrypted RAR4 independent grant/revoke use original cache shares. Preview alone grants no encrypted drag permission; revoke refuses new demands without closing transferred streams. Actual-size budget and real-cursor validation persist. |
| G-27 | Four actual Desktop prepare/visible Ready regressions cover retained lease, failed decode, refresh/codepage and stale/closed completion; the current preview-filter group and original native preview/share retirement tests pass. |
| G-19 touched | Original source-versus-output budget, concurrent/repeated actual debit, complete wire/SDK descriptors, first/repeated same-size tamper rejection, cache cursor lifetime and real resource release are preserved. |

## Independent verification

All Rust calls used jobs1/incremental0/codegen-units256 in the same PowerShell invocation, one Cargo lane and full-operation timeouts. Native effects were reviewed and exact LIST→identical RUN selections used the freshly built matching worker. TEMP/TMP, generated/copied fixture inputs, expected data and result artifacts were confined to this session scratch. Only fixed public test data and independent decoding tools were used; no archive payload was executed.

| Check | Observed result |
| --- | --- |
| Archive isolated-codecs library tests |59/59 |
| Current worker build and C# test-host build |PASS; C#0warnings/0errors |
| Six retained native fixtures |LIST6/RUN6/PASS6/SKIP0: split ZIP, RAR4 grant, full-block rejection,12 format-order cases, six lifetime modes and ordinary preview adoption |
| Current-only control and retained-context fixtures |Each LIST/RUN/PASS1/SKIP0; descriptor-only plus nine completed ZIP1/2/16 trials per mode. Both modes used the same current code and are not historical before/after data. |
| Original native async/password/wire/SDK/tamper/cache/budget preservation |LIST12/RUN12/PASS12/SKIP0 |
| Desktop `opt04_preview_` / `preview` selections |4/4 and122/122; current observed counts, overlapping and not additive |
| Protocol `archive_` / IPC `archive` |50/50 and17/17 |
| Archive isolated-codecs + Desktop all-target strict Clippy, offline |PASS |
| Current result reconciliation |20 current-only lifecycle/control observations,12 format-order reports,2 split orders and3 RAR4 authorization scenarios consistent with their assertions |

The broader current `preview` filter includes the four new tests and other matching workspace tests; it is not represented as122 new archive features. Unrelated integration targets selecting zero tests are not counted as coverage. The native preservation selection additionally exercises complete tree wire/SDK limits, original async state references, slow consumers, password grant/revoke, actual remaining-byte accounting and source/output admission separation.

### Current independent lifecycle observations

These are medians/ranges from the current retained run, not an optimization comparison:

| Distinct ZIP demands | Worker/source/catalog | Revision passes | All-ready median ms | Receiver-end→actual-cleanup range ms |
| --- | --- | ---: | ---: | ---: |
|1|1/1/1|4|1643.264|56.469–86.047|
|2|1/1/1|4|1675.932|45.705–70.753|
|16|1/1/1|4|2631.322|98.126–142.855|

Each demanded trial retained an active worker at first ready, had zero worker terminals at that point, then exactly one actual terminal and completed cleanup/source exclusive reopen. Output counts/bytes were1/262144,2/524288 and16/4194304; actual ZIP member decode counts were1/2/16. Descriptor-only had zero worker/source/new output and no invented catalog count.

The original test collector names its fresh setup file `measurements-before.json`; this acceptance deliberately ran that mode with the **current** worker solely to establish the existing fixture's immutable source/path control before the retained assertion mode. `independent-acceptance-scope.json` records that purpose. Both reports have identical current pairing, and neither is used as historical optimization-before evidence.

Independent format reports confirm12 unique format/order pairs, one worker/source/catalog each, actual full7z block invocations, TGZ/TXZ outer decode1, first-ready source protection and occupied slot, final slots0, worker exit, surviving transferred stream and cache removal after target release. Split orders each have one worker/source, catalog2 and40 per-volume revisions. The three RAR4 scenarios retain cleanup and independent authorization; revoked cached requests are rejected while streams already transferred survive.

### Evidence and timing limits

Current frozen worker SHA256: `C0221616253C7EEE6C80B3B6B40B737FDF1E8522DCDA8B03706B36E8A3A0FB8D`. It was built from the reviewed workspace with isolated codecs and copied to `opt04-independent-acceptance/Debug/NxTools.ArchiveWorker.exe` below the current session scratch. Current and frozen copies matched for native runs. The host/test pairing was rebuilt from the same workspace. No claim is made that byte size alone proves equality with an old historical binary.

This review's artifacts/logs are under `D:\.pi-desktop\data\scratch\aab2f188-352d-4f1c-b7a3-5375fe0df2d8`, named `opt04-acceptance-*`, with the current lifecycle/control in `archive-product-opt04-7e8e7027-e063-46c7-8d68-e6310e55576a`. Generated sources/provenance/expected bytes and bounded result evidence remain owned artifacts; successful copied helpers/partials retire through original test owners.

The delivery's historical before/after raw files are not present in this acceptance session directory. Its27.68s→2.45s table remains a delivery-reported historical measurement, **not independently recomputed here**. The current1/2/16 results and reduced worker/catalog repetition are independently confirmed. No new speed ratio, cold-cache result, Bandizip equivalence or current Release performance is certified.

## Stop point

No OPT-04 correctness/ownership repair is requested. The disclosed per-format repeated work remains visible for G-00/OPT-05 planning; it is not permission to remove volume freshness, full-block integrity or required resets. This acceptance preserves all existing data guarantees and does not introduce a broader source capability or backend change.

G-00 total source I/O/internal phase/block measurement and original large ZIP comparison, OPT-00/01 overall closeout, complete G-04, Phase E and physical/Release acceptance remain open. Controlled COM does not certify actual Explorer/GPUI gesture, DragOver, external target disk completion, DPI/accessibility or user visual behavior. No product/test source change, reference-program execution, real GUI/clipboard/picker, user archive mutation, source recycling/HZ5, service/registry, package/sign/install, commit or push occurred. OPT-05 through OPT-07 are not started; a new implementation instruction is required to proceed.
