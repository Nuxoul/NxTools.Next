# B5 — Completion intentions

Status: **accepted at the [coordinator's stated source/TestPlatform/owned HTTP-SQLite-writer-pipe boundaries](coordinator-acceptance.md)**. B5-01/B5-02/B5-03 are closed after independent repair checks; original implementation results below retain their scope and prerequisites.

Scope: **REF-07 and REF-11 only**. Acceptance owner: coordinator. B1–B4 retain their independently accepted boundaries. Stop at B5; no B6/B7. Canonical Archive remains **Phase E — Format expansion and ZIP editing / in_progress**.

## Delivered

- Task completion intention adds Sleep, Hibernate and normal application Exit while retaining applicable Shutdown and independent directory feedback. The original task/request metadata persists intention and attempt with missing-field defaults. Progress close/reopen/reload neither clears intention nor mutates task execution state.
- Queue intention persists on the original queue in SQLite schema14. Original transactional baseline merge changes only owned intention leaves; failed save retains draft/error. Queue membership and accepted-work revisions remain original engine/store facts, not presentation state.
- The original single coordinator distinguishes persistent intention, current successful attempt/round and one-shot authority. Nonempty current rounds require actual successful completion of all accepted members. Waiting/scheduled/failed/exhausted/cancelled/removal are not success. Historical terminal reload cannot replay; retry/membership changes/new accepted work enter the original event/revision path.
- One stable arbitration is **Shutdown > Hibernate > Sleep > ExitApplication**, independent of callback order. This favors machine-level intentions when sources conflict; it does not run several competing actions. The original bounded max-three case-insensitive directory batch remains independent.
- One action/countdown slot is consumed once. Explicit/new-work cancellation and stale generation/correlation handling preserve current authority. While the original native command is held, facts still update but no second countdown starts. Sleep/Hibernate retain the slot through suspension until actual native terminal; Shutdown keeps its bounded acknowledgement. Disconnect closes matching executing presentation and reports an unconfirmed result without substitution/retry.
- C# owns capability/privilege/SetSuspendState and exact Windows codes through the original typed parser/pipe/adapter. The original detached command dispatcher remains responsive and drains actual delegates. Rust owns intentions/coordination; official Kit0.8.0 components own presentation/input.
- Normal Exit calls original `cx.quit()` and carries its correlation into post-run cleanup. Original engine futures retire before Download IPC stop/client/host joins, followed by original extension/clipboard cleanup and runtime release. Cleanup attempts continue across earlier failure and do not report false drain success. A close request is not physical process exit.

## Original owner chain

`DownloadTask/DownloadQueue` → original request metadata/`SqliteDownloadStore` → original Task/Queue commands and accepted-work events → `DownloadEngine::completion_snapshot` → `CompletionActionCoordinator::evaluate_completion_in_slot` → original countdown window → `execute_shutdown` → typed PlatformBridge/PlatformPipeClient/parser → WindowsPlatformAdapter/CompletionSystemActions.

Exit uses `DesktopRuntime::shutdown_downloads_and_wait` → `shutdown_download_resources` → `DownloadEngine::shutdown_and_wait_correlated` → original Download IPC retirement/joins → original extension/clipboard cleanup. `design.md`, `backend-outcomes.md`, `ui-outcomes.md`, `native-outcomes.md` and `exit-outcomes.md` retain the exact boundary and component decisions. References were inspected ABDM → legacy NxTools → Mister before implementation; `reference-review.md` records provenance and rejected failure/empty-queue semantics.

## Verification

- Current B5 Desktop: LIST22/RUN22/PASS22/FAIL0/SKIP0/not_run0.
- Download completion contract9 and original Download lib128/engine105 pass; Download-IPC9, Core98, IPC62, Protocol116, CLI15 and BrowserHost48 pass.
- B4 UI30, original shutdown12/coordinator5, exact-rate11, Server7, theme10 and task-window/settings preservation selections pass. Groups overlap and are not added together.
- Safe C# exact `B5 completion` selection: LIST2/RUN2/PASS2/FAIL0/SKIP0/not_run0. No capability/token/power API or user Desktop exit is invoked.
- Real behavioral RED/GREEN covers task persistence, queue invalidation, truthful worker drain, historical rounds, held probe, held native serialization, suspend deadline and disconnect presentation. Compiler/fixture prerequisites and first-pass supplementary coverage remain separately classified.
- Scoped Rust formatting and affected eight-package all-target check pass. Six affected packages pass strict all-target Clippy. Protocol/Desktop retain three unchanged historical diagnostics, with no suppression.
- Four spec/component/Kit/editor gates and the Download UI lifecycle gate pass. Final actual resource audit observes no retained workspace tests, B5 wrappers, archive workers, test TCP or B5 completion pipes; fixtures separately assert actual future/socket/listener/client/host retirement.

`verification.md` contains all33 current-source groups, exact command rules and counts. `verification.json` indexes original logs, machine selections and audits in this session scratch. `changed-paths.txt` is the exact B5 delta against original before-copies, preserving the dirty accepted B1–B4 baseline. Protected coordinator files are byte-identical; branch/HEAD are unchanged.

## Limitations

`limitations.md` records the three historical strict-lint diagnostics and seven existing Archive tests that fail because their mandatory native wire exports are absent (Archive129: PASS122/FAIL7). Neither group is labeled passing. Physical power/current-user Desktop Exit, native OS-close/IME/DPI/Shell, human visual, installed/Release and whole-workspace acceptance remain independent.

No product/reference GUI, connection/performance experiment, package/install, service/registry mutation, real power action, current Desktop termination, commit or push is performed. Independent B5 reacceptance is complete with verdict accepted at the stated boundaries; see [final acceptance](coordinator-acceptance.md). The separate PlatformHost memory root repair is planned for the next task, not completed here.
