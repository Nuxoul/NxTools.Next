# Audit #12 outcome: selected-detail IO leaves the GPUI thread

## Status / scope
IMPLEMENTED_WITH_BOUNDED_VERIFICATION_AND_BASELINE_QUALITY_BLOCKERS. Direct implementation in this session, no delegation. No full Desktop run, external or loopback network tests, real downloads, user data, native Windows/HWND/picker/Shell/OLE/service, packaging/install, extension execution, commit or push. This delivery does not declare global13-finding closure or change the old incomplete #6/#7/#8 history. #13 retained text was not changed.

## Actual original chain and minimal change
Before: render_downloads -> cx.defer -> DownloadApplication::request_task_detail -> window_support::load_download_task_detail -> DownloadEngine::handle_command(TaskDetail) -> engine/projection.rs::command_task_detail -> task() (also queue eligibility storage) -> SqliteDownloadStore::load_transfer_plan and load_task_timeline(300). These methods take the original Mutex<Connection>; defer merely moves to another UI application turn. refresh_snapshot(true) also synchronously reloaded detail.

After: the original Desktop method remains the admission boundary, implemented in its cohesive private detail_request.rs on the same DownloadApplication. It installs taskID/generation/correlation/scope/pending before dispatch. The original engine projection exposes request_task_detail_background, which uses the already-existing download_runtime().spawn_blocking pool and invokes the same original handle_command(TaskDetail). The original store, SQLite connection/schema/transactions/TaskDetail command/engine diagnostics stay unchanged. A bounded one-result async channel returns the owned snapshot/fault. The GPUI foreground future holds no entity lease or storage lock across await, then defers result application to the next application turn.

One actual detail worker remains occupied until physical completion, even after logical cancellation. Rapid selection/refresh changes replace a single latest pending request. TaskID + generation + correlation + live scope + engine session + current task existence qualify publication; stale success/failure is discarded. MainWindow retires its shared local presentation token synchronously on actual row/right-click selection changes, search/filter invisibility, module switch, titlebar Close, window-closed event and entity release, then defers cross-owner cleanup. Presentation retirement does not cancel a download. Closed-window tests deliberately retain the original entity to prove release is not the only guard.

Cache hits run through the original owner without IO or synchronous view writeback. Duplicate current requests do not advance generation/dispatch twice. Last-good detail remains during refresh; cache_dirty prevents a re-bound same-task scope from swallowing a necessary refresh and clears only on current success. Snapshot refresh goes through the same bounded detail owner, not the old synchronous second path. Current live progress is merged into accepted detail before publication. Loading/error content, original five Small underline TabBar tabs, DescriptionList/state composition, header masking, 300-entry timeline, scroll/focus and rem list measurement remain unchanged.

A real failure regression exposed a second edge: handle_command emits CommandRejected for read-only TaskDetail failure. Treating that as task mutation refreshed detail again (observed3dispatch/1publish/2discard instead of2/1/1; persistent errors could loop). The original owner now recognizes its own correlation receipts (bounded8 outstanding) and consumes the read-only rejection without requerying details. Event sequence advancement/stale-event rejection remains; a real sequence gap still refreshes the list snapshot without restarting this failed read. Receipts are claimed at the actual event reader, not by matching error text. No error payload is logged. Actual queries still emit original requested/loaded/failed engine events with the same correlation. Desktop accepted/started/cache/completed/failed/retired events contain IDs/generation/elapsed/typed code/retryable only; no URLs/paths/names/headers/cookies or frame/byte logging.

The shared synchronous load_download_task_detail helper and its existing artifact/task-window/shutdown/Debug preview consumers remain exact baseline bytes. They are not claimed asynchronous by this selected-detail delivery. General compact snapshot IO is also unchanged; this is not a whole-application IO responsiveness claim. No second engine, runtime, store, global cache, lock or list/selection controller was introduced.

## Regression-first evidence
The first effective red used the original request and real engine/store SQLite connection mutex, not a sleep/result substitute. A fixture thread held the actual connection; the original deferred UI request ran before an unrelated next-turn UI callback that releases it. A separate3second fixture watchdog prevents permanent scheduler deadlock and must never be the passing unlock source.

| Observation | Original effective red | Final same request/store boundary |
| --- | --- | --- |
| Next UI turn ran before watchdog | false | true |
| Watchdog was needed to unlock storage | true | false |
| Observed callback elapsed | 3013ms | 0ms rounded (not a performance SLA) |
| Verdict | 0PASS/1FAIL in3.05s | PASS, real result subsequently awaited |

The deterministic assertion is ordering/lock ownership, not elapsed<arbitrary threshold. Original red: audit12-store-lock-red-no-pdb.log. First optimized run had true/false/0ms but FAILED teardown because the GPUI deterministic scheduler rejected external Tokio wakeups. The fixture now uses released allow_parking() for real IO and condition(...).await with its public bounded deadline; no scheduler/vendor edits. First accepted green:1/1 in0.04s; final9/9 in1.22s. Later lifecycle fixtures use an isolated temporary SQLite engine instead of the initial in-memory engine; both use the same original Mutex<Connection> and command path. Timings are descriptive, not controlled percentage claims.

A second effective red, audit12-refresh-cache-red.log, demonstrated last-good cache erasing an active refresh when re-binding a fresh scope. cache_dirty repaired this at the original owner. Final tests observe actual dispatch/application/discard counts; they never inject a fake successful detail or directly call completion to create positive evidence.

## Nine production-boundary regressions
1. Real request/store-lock ordering and UI continuation; await actual detail result.
2. Duplicate A and A->B->A: one physical worker, one latest pending, intermediateB never dispatched, oldA discarded, latestA published; cache hit dispatch count unchanged.
3. Real temporary SQLite timeline table rename makes original query fail; restore and retry through original request clears error and retains bounded timeline.
4. Retired missing-task failure cannot overwrite current different-task detail; no automatic read-failure refresh loop.
5. Existing detail refresh + new same-task scope retains pending refresh rather than using dirty cache; original queue-only Cancel command eventually publishes Cancelled detail for the same ID.
6. Scope cancel and ID removal retain physical running ownership until result; real inflight success is rejected.
7. Actual production row pointer A->B->A plus actual official detail Tab click remain responsive while storage is held; original loading paints, then current empty-segment state paints after real result.
8. Actual window close with retained MainWindow entity retires scope; late real result does not publish detail/error.
9. Actual original module Button switches to Tools while locked; result is discarded and original queued task remains Created (presentation-only retirement).

## Checks actually run
| Check | Result |
| --- | --- |
| Final formatted audit12 |9PASS/0FAIL/0ignored,1.22s |
| Final DownloadPage |29/29PASS,27.65s; includes audit11 five, sort keyboard and Root14/16/18 rem/scroll protections |
| Download-main filter/sort/count/queue/context/batch/detail/double-click |27/27PASS,3.32s |
| Precisely filtered no-network Downloads lifecycle |48/48PASS,11.57s; includes original keyboard8/picker14, progress/cache/removal/keyed-window ownership; not full Downloads210 |
| Final actual Button name / Clipboard stable-ID protections |18/18PASS,21.99s = names7+identity11 |
| Original engine TaskDetail selected segments/bounded timeline/privacy contract |1/1PASS,0.01s |
| Desktop strict tests |PASS48.17s final |
| Download strict tests / production |PASS30.86s / PASS10.71s |
| Desktop strict production |FinalFAIL only task-start unused SPACING_12 at main_window.rs75 and DecodedMedia.animated at tools/floating_assistant/media.rs62 |
| workspace check --tests |PASS60s final |
| Scoped9Rust files fmt and scoped whitespace |PASS/PASS |
| whole-workspace fmt |FAIL four baseline-identical files: main_window/page_rem_tests.rs, network_settings/blacklist.rs, network_settings/page_rem_tests.rs, network_settings.rs. Touched shell.rs now formatted; no broad unrelated format |
| source/document gates |10/10PASS: Download UI,Desktop shell,UI tokens/consistency/state/motion,catalog,release provenance,Markdown provenance,spec contracts |

Counts overlap and are not unique coverage. Existing tests whose names mention native use synthetic platform command/GPUI fixtures here; no real native acceptance. Test list and exact selected filters are retained. HTTP/media/probe/FFmpeg suites, full Download lib and full Desktop were not run because this authorization forbids network/full Desktop; the original one TaskDetail lib contract is storage-only. No ignore/assertion weakening/lint suppression.

## Build/tooling failures and precise recovery
- Initial original red attempt did not run: MSVC LNK1318 PDB LIMIT(12). Second attempt with Desktop-test debug0 still failed LNK1106. Inspection found D: free2961408bytes. No Rust/link survivors.
- Only this task's two failed output hashes da7215c5e26ccf8d /5008a12526e9ebe3 were recorded and removed (523files/2670014859bytes). Existing successful binaries and other-session artifacts were not cleaned. Full cleanup manifest retained in commands/own-failed-build-cleanup.json.
- Successful Desktop tests use command-line `--config profile.test.package.nxtools-desktop.debug=0` plus `_LINK_=/DEBUG:NONE`, preserving required CARGO_BUILD_JOBS=1/CARGO_INCREMENTAL=0/RUSTFLAGS=-Ccodegen-units=256. This is verification-only, no committed profile or framework change. Lack of PDB means diagnostic stacks have unknown frames; exact source assertion lines/logs retained. One read-only `cargo rustc --help` inspection did not explicitly set these variables in that command; it compiled or ran nothing. This command-policy omission is disclosed rather than claimed compliant. All actual build/check/lint/fmt/test commands set the required variables.
- First optimized lock test: UI ordering green but overallFAIL on real external-thread scheduler wake; fixed fixture with official allow_parking/condition, not production bypass.
- New lifecycle compile attempt failed17E0308 from new assertions passing TestAppContext to Entity::read; changed to released read_with, no product change.
- Six-test run5PASS/1FAIL exposed the real CommandRejected refresh interaction above; retained then repaired, six all green.
- Cache refresh regression red retained before cache_dirty fix; final nine all green.
- One attempted Rust batch refused before starting because an independent NxProxy Cargo was active. Observed its PID/path, never killed/paused/contacted it; two bounded samples later naturally idle. No claim its commands followed this session's scope, and no credit for its work.
- Initial quality pass revealed an own mistake: cfg(test) on the shared synchronous helper broke five production consumers (E0425), hidden by test builds. Reverted that one attribute; helper exact baseline restored. Production strict rerun has only pre-existing two warnings. Initial storage lock-seam/shell formatting failures fixed only in touched files. Every failed/aborted/preparation log remains.

## Dirty baseline, evidence and remaining limits
Live baseline398files/3750historyfiles at2026-10-26T07:39:57.113195Z; not HEAD. Expected delivery:9existing-file reversible dirty deltas (reference,2manifests,Desktop owner/MainWindow/page/shell,engine projection,test-only store seam) and3new cohesive modules (request owner,owner tests,real page tests). No old evidence is rewritten. Existing engine.rs/lib/message/window_support and artifact/task-window/shutdown consumers, audit11 tests, theme/rem tokens/vendor and unrelated files remain hash-protected. A test-support feature/dev dependency exposes holding the actual synthetic connection mutex only; production store query/schema/lock implementation unchanged. Cargo.lock baseline remains unchanged. Evidence verifier replays both directions and checks source/history/current/result hashes.

SQLite IO cannot be physically interrupted; cancelled work finishes and is ignored. One running + one pending bounds selected-detail IO, but long underlying storage contention still delays the latest detail. General compact snapshots/actions/other synchronous detail consumers can still perform IO and are explicitly outside this slice. Tests prove synthetic application-turn/gesture/lifecycle behavior on this workstation, not full product responsiveness, real Windows IME/DPI/readers/physical input/installed/native/visual/release acceptance. No global13closure or next task is implied.
