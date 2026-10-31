# Audit #11 outcome: bounded DownloadPage projection

## Status
IMPLEMENTED_WITH_BOUNDED_REGRESSION_VERIFICATION; production strict and whole-workspace formatting remain blocked by task-start defects outside this delta. No global 13-finding closeout or native/visual/release acceptance is claimed. This session implemented directly without delegation. #12 detail IO/threading and #13 retained text were not changed.

## Production change
The original DownloadApplication now owns `Rc<Vec<DownloadEngineTask>>`. Constructor, authoritative refresh and test replacement publish a new immutable snapshot at their original boundaries. `tasks()` remains a borrowed slice for existing consumers; `task_snapshot()` shares that same allocation, not another list/cache/controller. Production never mutates it in place. A pre-existing floating-assistant consumer uses `.iter()`; its test-only mutation uses explicit Rc::make_mut.

`download_tasks_for_view` returns source indices, retaining the original filter/search predicates and stable comparator, including queue-order early return and tie order. `render_downloads` derives counts, visible IDs and reconciliation from the same captured snapshot. The uniform_list closure owns that snapshot, its indices and existing shared ID sequence. It checks the projected ID before constructing a row; an old index is never applied to a replacement owner's Vec. Live progress and icon resources are looked up by stable ID only inside the framework-requested range. Rows synchronously borrow task/progress, retaining the same necessary owned display strings, tooltip text, image paths, entity handles and ID callbacks.

The selected detail task/progress/detail snapshots remain owned as before (at most one selected task); original deferred request_task_detail lifecycle is unchanged. No new thread, IO path, lock, global cache, selection model, timer, diagnostic stream, dependency or vendor change. Original Ctrl/Shift/anchor, batch/context capabilities, queue operations, double-click/deferred keyed windows, failure states, ScrollHandle and rem measure/paint are retained. Header/InputGroup/Toolbar/PopupMenu/Tabs/Progress/DescriptionList composition and geometry are unchanged. OLE/native adapters are unchanged, not exercised as real native operations.

## Regression-first evidence and actual measurements
Two original implementation reds were run before the production optimization, using real download_tasks_for_view and production Root -> render_downloads -> uniform_list -> download_task_row. No runner/network was started.

| Metric | Original 1,000 | Optimized 1,000 | Original 10,000 | Optimized 10,000 |
| --- | ---: | ---: | ---: | ---: |
| Deep-copied task URL payload allocations in projection (pointer identity) | 1,000 | 0 | 10,000 | 0 |
| Requested / constructed rows in the observed draw batch | 26 / 26 | 26 / 26 | 26 / 26 | 26 / 26 |
| Copied task payloads reaching real row composition | 26 | 0 | 26 | 0 |
| Progress getter accesses in the same draw batch | 2,000 | 26 | 20,000 | 26 |
| Icon getter accesses in the same draw batch | 2,000 | 26 | 20,000 | 26 |
| Single queue projection elapsed microseconds (descriptive only) | 3,654 | 146 | 41,792 | 2,084 |

Original logs: audit11-red-01 and audit11-red-10000. Like-for-like early optimized logs: audit11-green-02 (149/2,115 microseconds). Later final fixture adds real synthetic PNG map entries, checks the original 128-segment progress allocation identity, and corrects category ID/name validity; final timing is not a controlled percentage comparison. Original row element construction was already bounded: this task removes the eager payload/maps and repeated visible-row task/segment cloning, not a falsely claimed eager-all-elements defect. Getters are observed at the real production accessors; task/segment pointer identity is observed at the real row boundary. No custom allocator, RSS, GPU or release-memory benchmark was run. The tests do not rely on timing thresholds.

The observed batch contains two GPUI draw passes, each requesting `0..1` twice for legitimate measurement plus `0..11` for visible rows. Bottom scroll requests `9989..10000` (or `989..1000`) plus those measurement rows: again 26 constructions/accesses and zero task/segment copies. Do not interpret 26 as a universal viewport row limit. Actual range membership, exact construction count, payload identity and bounded resource work are asserted. Original Root14/16/18 row/header/paint/scroll regressions also pass; Root font changes are not Windows DPI.

Five new tests cover: 1,000/10,000 payload identity; real range/resource/segment work and end scroll; fixed expected six-sort/filter/count/tie results; owner replacement exactly after production projection and before deferred range; actual pointer/Ctrl selection, owner reorder and removed-ID retirement. A test-only one-shot post-projection injection is absent from production builds. No fake list/resolver or direct business method substitutes for positive pointer/keyboard evidence.

## Verification actually run
| Check | Result |
| --- | --- |
| New audit11 final | 5 PASS / 0 FAIL / 0 ignored, 1.33s |
| DownloadPage related (includes new tests, sort keyboard, average rows, rem virtual scroll) | 26/26 PASS, 22.82s |
| Download main filter/sort/count/queue/context/batch/detail/double-click/icon related | 27/27 PASS, 2.48s |
| Downloads related | 210/210 PASS, 185.68s; includes original keyboard8 and picker14 |
| Existing audit04/05/06/08 protections | 69/69 PASS, 168.88s; overlapping counts are not unique coverage |
| Final formatted-source new/name/history-identity group | 23/23 PASS, 22.90s = audit11 5 + names7 + history identity11 |
| Test strict Clippy | Initial FAIL on new seam type_complexity; fixed with a named type, final PASS in 32.46s; no suppression |
| Production strict Clippy | FAIL: baseline `main_window.rs:75` unused SPACING_12, baseline `tools/floating_assistant/media.rs:62` unused DecodedMedia.animated |
| `cargo check --workspace --tests` | PASS, 74s |
| Scoped eight Rust file rustfmt / diff whitespace | PASS / PASS |
| Whole-workspace fmt | FAIL in baseline-identical main_window/page_rem_tests.rs, main_window/shell.rs, network_settings/blacklist.rs, network_settings/page_rem_tests.rs, network_settings.rs |
| Source/document gates | 11/11 PASS: Download UI, Desktop shell, Clipboard UI, UI tokens/consistency/state/motion, catalog, release provenance, Markdown provenance, spec contracts |

Rust commands were naturally idle-admitted and serial, with CARGO_BUILD_JOBS=1 / CARGO_INCREMENTAL=0 / RUSTFLAGS=-C codegen-units=256. One interrupted test call had no terminal result; survivors were inspected before restarting. No full Desktop test run, actual downloads, user files, native pointer/HWND/Windows picker/Shell/OLE/service, packaging/install, extension execution or commit/push was performed. Existing synthetic tests may use local temporary files/SQLite/loopback fixtures; these are not external/user download activity.

## Failed attempts and stall disclosure
- Initial edit payload accidentally inserted operation headers into the new observation region; exact inserted text was removed before compilation. Several line-anchor/new-fixture edits were corrected before compilation. No old assertions were weakened or ignored.
- audit11-green-01: compilation failed on an existing test fixture's direct Vec mutation after Rc ownership; adapted test-only mutation, retaining assertion.
- audit11-final-five-01: new pointer fixture E0499, fixed by reading current bounds before calling simulate_click.
- audit11-final-five-02: compiled, then stalled in the new cross-window ReplacedOwnerPage fixture; tool was interrupted, no terminal result. Follow-up confirmed no surviving Rust/test process.
- A bounded diagnostic demonstrated repeated wrapper render (8-render assertion), 0 PASS / 1 FAIL in 0.88s. Its first shell wrapper captured a null ExitCode and returned0; the stdout failure is authoritative, explicitly not PASS. Replaced the unnecessary cross-window wrapper with a one-shot real production post-projection seam; temporary DEBUG-AUDIT11 markers removed.
- Subsequent new fixture failures: invalid category pairing/length, missing completed timestamp, unregistered queue category, and pointer selection on non-engine synthetic records. Corrected fixture data/use of queue-only engine records, not production validation or detail ownership. Final five all pass.
- A temporary Python log-summary shell command had quoting SyntaxError; no source modification. All command failure/interruption logs and corrective records are retained.

## Dirty-state protection and limits
Rollback basis is current dirty bytes captured 2026-10-26T04:30:58.957751Z, not HEAD: 374 files and 3242 historical evidence files. Eight existing task-owned files plus one new test have exact forward/inverse byte-line deltas/current snapshots. Unchanged callbacks and owner-specific regions are compared against that baseline. Old audit manifests/outcomes remain immutable.

Cargo.lock gained one Desktop dependency-list entry for flate2 during this work, matching the already-present live Desktop manifest. This task never edited the manifest or added a dependency; the lock reconciliation is separately preserved as unowned/toolchain delta, not silently reset or credited as optimization. Its initiating process cannot be proven from passive observations. No external source change is overwritten.

Remaining costs are explicit: full filtering/count/ID/reconciliation traversal remains O(N), nonqueue stable sorting remains O(M log M) and existing lowercase name comparisons may allocate. The captured snapshot can transiently keep one prior authoritative allocation alive through its frame/closure lifetime; it is not stored in a global cache. Progress/icons are read by ID during actual range composition rather than copied as a whole-render map. Selected detail cloning/synchronous IO remains the separate #12 task. No retained-text #13 change.

Source preservation, compiled/synthetic GPUI interaction and deterministic work counts are not native/physical/IME/mixed-DPI/reader/screenshot/user/installed/release acceptance. Original #6/#7/#8 incomplete work and global closure count are not changed by this delivery.
