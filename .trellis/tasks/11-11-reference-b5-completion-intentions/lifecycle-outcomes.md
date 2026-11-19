# B5 completion lifecycle evidence

Status: `ready_for_acceptance / not_accepted`.

## Original owners

- Queue persistence commits through `SqliteDownloadStore` before `engine/queue.rs` publishes the original `SnapshotRequired` state invalidation. The existing Desktop event pump reloads the authoritative completion snapshot; UI and IPC commands share this path.
- `CompletionActionCoordinator` retains current round membership, consumes attempts once and excludes terminal history from subsequent rounds. A failed/cancelled/removed/moved current member aborts its round; fresh accepted work can start the next one.
- The original `pending_shutdown_command` occupies the single native action slot. The coordinator continues observing facts and directory feedback, but creates no second countdown until matching native result/timeout closes the executing presentation and reevaluates.
- The original engine worker collection is a Tokio `JoinSet`: transfer workers, HTTP probes, resolver futures and the existing managed-tool operation register under the same stop gate. Drain closes admission, requests original pause/cancel controls, invalidates schedule wakes and awaits actual futures outside owner locks. Reaped or current join failures remain failures after all handles retire.
- `DesktopRuntime::shutdown_downloads_and_wait` uses the same completion-exit correlation for engine drain, then stops/retires/joins the existing Download IPC clients/host. It attempts IPC cleanup even when engine drain fails. Main then attempts the original extension and clipboard cleanup before returning any failure.

## Behavioral RED

All logs below are retained in the session scratch `D:\.pi-desktop\data\scratch\0ffe6ffa-9e52-420b-af61-a29fdd0f37b4`.

| Selection | LIST/RUN/PASS/FAIL/SKIP/not_run | Exits | Assertion/log stem |
| --- | --- | --- | --- |
| Download `--test b5_completion_contract b5_queue_mutations_` | 1/1/0/1/0/0 | 0/101 | `committed queue facts must invalidate the original completion subscriber`; `b5-queue-invalidation-red` |
| Download `--lib b5_shutdown_joins_remaining_` | 1/1/0/1/0/0 | 0/101 | remaining worker exited, but `worker panic must not be reported as successful resource drain`; `b5-worker-drain-red` |
| Desktop `--bin nxtools-desktop b5_repair_` | 3/3/0/3/0/0 | 0/101 | historical Failed terminal blocked fresh success; held HTTP probe did not retire at drain; held native reply allowed a second countdown; `b5-round-native-probe-red` |
| Desktop `b5_suspend_reply_outlives` | 1/1/0/1/0/0 | 0/101 | Sleep/Hibernate lost their native action slot after the bounded Shutdown deadline; `b5-suspend-deadline-red` |
| Desktop `b5_platform_disconnect_retires` | 1/1/0/1/0/0 | 0/101 | `disconnect must retire the original executing presentation`; `b5-disconnect-red` |

Each selection used `cargo test --locked --offline -p <package> <target/filter> -- --list`, then identical target/filter with `-- --nocapture --test-threads=1`, through `b5-run-rust.ps1` with the required single-lane/memory/TEMP environment. These are behavioral failures after successful compilation, not compiler prerequisites.

## Current-source verification

- Download completion contract: `b5-backend-pass2`, LIST9/RUN9/PASS9/FAIL0/SKIP0/not_run0; exits0/0. Queue invalidation is GREEN at the original subscription boundary. The other eight cases retain persistence/defaults/concurrent-leaf merge, real writer publication, duplicate admission and actual worker drain coverage.
- `b5-worker-drain-green`: LIST1/RUN1/PASS1/FAIL0/SKIP0/not_run0, exits0/0. Worker panic remains a failure after actual remaining workers retire. Final Desktop/native waiting/disconnect results and first-pass real SQLite/HTTP/runtime coverage are recorded in `verification.md` and `verification.json`.
- `b5-desktop-pass5`: LIST22/RUN22/PASS22/FAIL0/SKIP0/not_run0, exits0/0. Historical-round, held-probe, native-slot, suspend-deadline and disconnect regressions are GREEN. Real SQLite/HTTP/runtime and original event-pump cancellation cases are first-pass supplementary coverage, not invented RED.

## Verification layers

TestPlatform quit is a controlled close request, distinct from actual future/thread joins. Original HTTP listeners/sockets and pipe callers/host are explicitly retired/joined by the relevant fixtures. No physical power action, current Desktop exit, service/registry mutation, product/reference GUI, package/install, commit or push is performed. Native OS-close/IME/DPI/Shell, human visual, installed/Release and physical-process acceptance remain independent.
