# Archive continuation workflow

Status: object-scoped continuation is implemented. Archive, home, protocol and read-only native regressions pass. Global verification and native/user acceptance limits are listed below.

## Product behavior
- No unconditional continuation button on the archive home, blank-source continuation form or manual recovery UUID.
- A started reclamation task exposes check/continue only after a failed/cancelled terminal with actual cleanup. The callback rechecks the original job, attempt and generation.
- Reopening a top-level source may inspect recovery records once after failed/rejected read cleanup. Cancelled, disconnected and nested reads do not probe. An absent record preserves the original read failure.
- The existing continuation form fixes the source. A native-identified original target is readonly; an unresolved target requires selecting the original folder and reinspection. Recovery ID and original copy_motw remain internal facts.
- Pending, failed, completed, missing-target and disconnected states cannot submit through Start or Enter. Discovery never submits. A ready form still requires the independent irreversible confirmation and a fresh original Job Start.
- Completed records report recorded completion, not current output validity. They expose no browser continuation action; the form disables submission and omits future-operation warnings/preferences.
- Ordinary extraction, create/edit, queue, preview, Shell drag and full native execution validation retain their original owners. Continue has no ordinary space query, source chooser, range/strategy/conflict editing or remembered defaults.

## Boundaries
Rust owns state/layout and correlated pending discovery; C# owns held read-only source/parent/ADS/companion observation; GPUI renders the existing views. Pending requests are bounded, registered before dispatch and retired on close/refresh/cancel/disconnect/deadline. Results validate request IDs, generation, source/target context and connection, with deferred view delivery.

The additive inspection command does not grant replay/write authority. No journal schema, reclamation release order, format support, output limit, dependency/vendor, task-history database or directory scanning change is included. No destructive recovery, sparse mutation, process-kill recovery matrix, user original, package, installation, service configuration or protected runtime operation was performed.

Quick Launch retains the original result presentation and behavior from its separate checkpoint; this continuation work does not modify it.

## Executed verification — 2026-10-31
| Check | Result |
| --- | --- |
| Rust protocol tests | 96 unit + 2 integration passed |
| Desktop archive aggregate | 345 passed, 0 failed |
| Archive home/settings production tests | 20 passed, 0 failed |
| Desktop/protocol check, including tests | Passed |
| PlatformHost + test-project build | Passed, 0 warnings/errors |
| Exact `archive recovery inspection fixture` selection | RUN4 / PASS4 / SKIP0 / FAIL0 |
| Scoped Rust formatting and whitespace | Passed |
| GPUI release/editor/catalog and spec contracts | Passed |

All Rust commands use jobs1/incremental0/codegen-units256. Native fixtures use only GUID-owned synthetic source/marker/journal records and verify byte/identity preservation. They include absent/mismatched/missing/corrupt/torn/completed records, invalid plan names through the execution validator, original-target identity, bounded detached work and private diagnostics.

Production-view/fake-IPC regressions cover contextual task and browser actions, actual cleanup, stale attempts/generations/closed objects, disconnect/timeout, no autosubmit, hidden UUID/source chooser, original-target reinspection, internal recovery facts, Start/Enter guards, consent and fixed actions. Settings tests use actual independently scrollable sidebar/content areas and real controls; save/cancel and failed-save assertions remain.

## Actual isolated windows
The compiled scratch-copy preview opens the real OperationForm without runtime, platform, capture or indexer initialization. Actual observed scale is100%; checked default560x600 and minimum480x360, long synthetic source/target paths, Light/Dark and the following states:
- [Ready, Dark](assets/continuation-found-dark.png): fixed readonly objects, identified state, concise warning, reachable footer; no UUID or ordinary extraction controls.
- [Independent consent](assets/continuation-confirmation.png): pointer Start opens the irreversible confirmation, not an immediate job. Pointer “保留原压缩包，不开始” returns to the original form, and Cancel closes it.
- [Missing target, Light/minimum](assets/continuation-missing-light-min.png): original-folder selection and reason visible, submission disabled, Cancel visible.
- [Inspection failure, Dark/minimum](assets/continuation-error-dark-min.png): visible error/retry/original-folder selection, disabled submission and fixed Cancel.
- [Pending, Dark](assets/continuation-pending-dark.png): truthful readonly-check status and disabled submission.
- [Completed, Light](assets/continuation-completed-light.png): recorded completion/current-output limitation, disabled submission and no future-operation settings.

## Limitations
- The complete Desktop run recorded1570passed/1failed: `tools::polish_tests::r6_capture_reuses_one_window_and_retires_superseded_six_second_timer` at `tools/polish_tests.rs:548`. It is outside archive continuation; the full suite is not reported green.
- Strict global Clippy final verification is deferred while the shared Cargo lane is occupied by another workstream. No global all-pass result is claimed.
- Compiled Enter/Escape/focus regressions pass. Actual Escape, system picker lifecycle, other DPI/font scales, screen-reader/provider and installed-product interaction remain unverified.
- Screenshots and synthetic fixtures are not user visual acceptance or proof that an arbitrary recovery record can continue. Fresh execution still performs the original full native/plan/output validation.
