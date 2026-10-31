# Audit #13 — retained Quick Launch presentation text

## Outcome and scope

Implemented directly in the current session, without delegation. Status: **IMPLEMENTED_BOUNDED_VERIFICATION_WITH_BASELINE_QUALITY_BLOCKERS**. This is not a revision of the global thirteen-finding acceptance count. Unfinished ordinary-rem/cursor work and queued release-chain repairs remain separate.

The rollback basis is the live dirty snapshot at **2026-10-26T09:12:44.222655Z**, not HEAD: 385 source/spec/reference files and 4,293 historical evidence files. Own delivery changes six existing files and adds two Rust modules. The reference index retains its exact prior byte prefix. Historical outcomes, failed logs and manifests are not overwritten.

## References and ownership review

Read AGENTS, current shared/Desktop contracts, component-selection rules, current 0.8.0 release notes before the historical table/index, and the coding-guide retained-text section. The guide requires stable SharedString in the original owning View/Entity, not per-frame String cloning. Locked pre0.3.9 StyledText accepts Into<SharedString>; converting a freshly copied String there does not fix ownership.

Listary was reviewed first at its managed SearchMode.Results/selection and SearchWindowBase construction/events → ShowWindow/base.Show/Activate/Hide/Close/input-focus boundary. Its proprietary native engine is unavailable and was not inferred. SwiftList was reviewed second at the current `App/Views/QuickSearchWindow/Helpers/QuickSearchWindowInputHandler.cs` and `QuickSearchWindowLifecycle.cs`. The older indexed input-handler path without Helpers is absent in this snapshot. Existing show/hide, action stack, IME and focus owners remain authoritative; no native behavior was executed.

## Narrow implementation

- The original QuickLaunchPanel owns one immutable `QuickLaunchResults` snapshot: full original `Vec<SearchResult>` paired with full SharedString title/subtitle presentation values. Preparation occurs at original new/show/search commit/recent-mode/replace boundaries. Read-only access prevents payload/presentation drift. This is not another search controller or an ID/global/per-row lookup cache.
- `render_result` clones retained title/subtitle handles. Exact existing subtitle fallback is preserved: explicit subtitle, CopyText hint, otherwise complete target. Existing truncation styling, highlights, row geometry and rem work are unchanged; no new text truncation or payload cap is introduced.
- Hover closures capture the same immutable result snapshot and index. A full SearchResult is cloned only when the original hover callback actually runs, rather than for every row construction. A retained old frame never resolves its index against a newer list. Snapshot retention can extend the lifetime of the previous complete result set until those callbacks/elements are dropped.
- Original action items/levels retain shared label, section label and level title; action-mode entry prepares a shared target display title. Panel status is shared. InputState already provides a shared read value to render, avoiding a per-frame query String copy.
- Editable filters, parent query/edit/assembly buffers, Finder SearchResult, protocol ShellActionItem, session/path/command data and complete execution payload remain their original types and content. There is no Finder/protocol/vendor/dependency change.
- Search generation, provider merge/order, result selection, pending last-good list, keyboard/pointer behavior, action stack/query restoration, preview controller, original release/close/focus/native boundaries and prior fixed/relative dimensions remain on their original owners. Existing tests were adapted only for the result-container type; original assertions were not weakened or ignored.
- No new user-facing or cross-process operation is introduced. Existing production diagnostics are unchanged. Measurements are test-only, not frame-rate production logging.

## Effective red → green measurement

The regression observes the actual SharedString values passed into production result/action/context/status element composition through real Root/Window draws after production `replace_results` / typed shell-action updates. It holds every observed handle through comparison, so allocator address reuse cannot produce a false stable-identity pass. Final tests require the expected text slots to have actually composed, exact full content and at least three long-text observations with one backing allocation.

| Observed production slot | Effective red handles / distinct allocations | Final handles / distinct allocations | UTF-8 bytes | Repeated backing-text bytes, red → green |
| --- | ---: | ---: | ---: | ---: |
| Action target title | 6 / 6 | 6 / 1 | 548 | 2,740 → 0 |
| Result subtitle 0 | 18 / 18 | 18 / 1 | 466 | 7,922 → 0 |

The two effective red tests fail at the first encountered offending key; HashMap iteration order is not stable. No unobserved baseline count is invented for other slots. Final green also records result title0 18/1 (546 bytes), title1 6/1 (547), subtitle1 6/1 (467), dynamic action label 6/1 (552), submenu title 6/1 (540), status 6/1 (720), and updated title/subtitle0 18/1 (760/880). Counts include actual repeated measurement/draw composition and are not a universal per-frame callback count.

This is a deterministic retained-backing identity/byte-copy proxy, **not** allocator instrumentation, RSS, total allocations or a frame-time SLA. Commit-time construction still copies the full view text once. Shared reference counts, highlight calculation/vectors, layout/GPU/element construction and full payload clones on actual actions remain costs. Short/static strings are not long-allocation measurements. No claim of allocation-free rendering is made.

## Final compiled and synthetic GPUI verification

All actual Rust commands/direct test invocations used jobs=1, incremental=0 and `RUSTFLAGS=-C codegen-units=256`, admitted only when naturally idle and run serially. Desktop tests additionally used command-only `profile.test.package.nxtools-desktop.debug=0` / `_LINK_=/DEBUG:NONE`; no committed build-profile change.

| Final check | Actual result |
| --- | --- |
| New production update/repeated-render ownership tests | **5 passed**, 0 failed/ignored, 0.27 s |
| Original `quick_launch::tests` bounded set | **121 passed**, 0 failed/ignored, 1.30 s |
| Original Quick Launch polish | **11 passed**, 0 failed/ignored, 0.42 s |
| Desktop strict Clippy test targets | **PASS**, 44.16 s final |
| Workspace cargo check | **PASS**, 13.94 s final; two baseline warnings retained |
| Scoped rustfmt, seven changed/new Rust files | **PASS** |
| git diff --check | **PASS**; unrelated CRLF warnings retained |
| Static/document gates | **9 PASS / 1 baseline FAIL** |

Total final selected tests: **137/137**, not a full Desktop run. New tests cover full-result update/repeated render and InputState identity; full dynamic action/context text; same-ID text replacement/reorder/old immutable payload retention; target/CopyText fallback/status; actual submenu pointer gesture and Esc parent/focus restoration. Full CopyText payload tails are asserted, never executed.

Original protection tests cover real result/action rendering, keyboard and pointer selection, key-release dispatch, IME Enter ownership, pinyin/fuzzy highlights, generation/pending/stale batches, original synthetic preview lifecycle/window reuse/focus, hide/activation policies, original row gaps and configuration preservation. Fixtures use synthetic results, temporary catalogs/files and in-memory platform/indexer bridges; typed platform commands are inspected rather than sent to a native Host. Extension `w6_` execution and Archive integration are explicitly excluded. These tests do not certify physical HWND behavior, real IME or DPI.

Passing gates: desktop-shell-lifecycle, ui-token-boundaries, ui-consistency, ui-state-lifecycle, ui-motion-policy, gpui-component-catalog, gpui-release-contracts, markdown-editor-contracts, spec-contracts. They are source/document/provenance checks, not native/user acceptance.

## Retained failures and remaining blockers

1. Production strict Clippy fails on baseline-identical `main_window.rs:75` unused `SPACING_12` and `tools/floating_assistant/media.rs:62` unread `DecodedMedia.animated`. No suppression or unrelated repair was added.
2. Workspace fmt fails in four baseline-identical files: `main_window/page_rem_tests.rs`, `network_settings.rs`, `network_settings/blacklist.rs`, `network_settings/page_rem_tests.rs`. Scoped files pass.
3. Quick Launch lifecycle source gate actually stops at **line 458**, QL-11 custom-command master-detail contract: it requires the literal `CONTROL_HEIGHT_INPUT`, which the baseline-identical custom editor lacks. Both the gate and checked custom editor are unchanged from task start. The queued release-chain assertion at line1765 was **not reached**; it is not claimed to have run or passed. No Queue16 gate repair is folded into this task.
4. First red attempt failed before execution with LLVM `IO failure on output stream: no space on device`. No Rust survivors remained. Exactly 206 object files belonging to this task's failed `nxtools_desktop-5008a12526e9ebe3.nxtools_desktop.c8157c520b212eab-*.o` output were recorded and removed; successful/independent EXE/PDB/artifacts were not removed. The later increase to approximately49.4GB free space was observed, not attributed to this task. The second red is the actual behavioral red (2 failures).
5. The first expanded-five preparation failed E0061 because the mouse-move test call lacked the released Modifiers argument. That log is retained; the corrected API call and all five tests subsequently pass. This compile failure is not a behavior red.
6. The first final-check script found own module ordering/four `.into()` formatting differences, then stopped on a Git CRLF stderr warning before gates ran. Its partial logs remain immutable. Only own formatting was corrected; the second script captures stderr without masking command exit status. No failed attempt is relabeled as passed.

Not run: complete Desktop suite, real user-file search/indexer service, real Clipboard/Shell/native/picker activity, extension install/execution, network, packaging/install, physical mixed-DPI/IME/accessibility reader/visual/user acceptance, commit or push. Original #6/#7 gaps, Queue16 and global acceptance remain outside this delivery. This task stops here; no future background resumption is implied.

## Reproducibility

`evidence/own-deltas.json` contains exact byte-preserving forward/inverse operations relative to the dirty baseline; current snapshots and human-readable diffs are separate. New modules are separately identified. `evidence/owner-checks.json` records unchanged source spans, exact old-test adaptation reversibility, reference prefix and protected owner hashes. `evidence/verify.py` checks frozen artifacts, live owned/protected files and all4,293 historical bytes without editing anything. A successful verifier is an integrity result, not a substitute for the compiled/synthetic/native distinctions above.
