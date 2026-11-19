# REF-07 — Task completion intentions

Status: `ready_for_acceptance / not_accepted`.

## Persistence regression

Production TestPlatform regression `b5_ref07_task_intention_survives_original_window_and_engine_reload` clicks the existing Small Checkbox in the original440×184 task window, closes/reopens the keyed presentation, verifies unchanged task execution state, reopens the same temporary SQLite database with DownloadEngine and constructs a new DownloadApplication. No task network execution or power request occurs.

Behavioral RED: nonzero LIST1/RUN1/PASS0/FAIL1/SKIP0/not_run0. The final intention-reload assertion fails: `accepted completion intention must survive original SQLite/engine/application reload`. Earlier pointer/close/reopen/task-state assertions pass. This is not a compiler or fixture prerequisite failure.

```powershell
& "$env:PI_SCRATCH_DIR\b4-run-rust.ps1" -Package nxtools-desktop -Target bin:nxtools-desktop -Filter b5_ref07_task_intention_survives_original_window_and_engine_reload -Name b5-ref07-persistence-red -ExpectedCount 1 -FormatPaths @('rust/apps/nxtools-desktop/src/downloads/settings_task_rem_tests.rs')
```

The wrapper scans the global Cargo lane and sets required memory/TEMP environment in its own PowerShell execution. Actual commands: `cargo test --locked --offline -p nxtools-desktop --bin nxtools-desktop <filter> -- --list`, then the identical selection with `-- --nocapture --test-threads=1`. LIST exit0/RUN exit101; logs `b5-ref07-persistence-red-list.log`, `b5-ref07-persistence-red.log` and result JSON are in this session scratch.

## Original persistence chain

Existing Checkbox's deferred application callback → original `DownloadCommand::Task / SetCompletionIntention` → engine clones current DownloadTask → original SqliteDownloadStore.save_task/save_task_row → stored request metadata → memory replacement → original TaskChanged. Missing fields default to no intention. Reading/closing/reopening does not admit, cancel or pause work. Existing immutable request context, transfer/checkpoints and admission remain untouched. A restored intention is not restored countdown authority.

GREEN: identical filter LIST1/RUN1/PASS1/FAIL0/SKIP0/not_run0, LIST exit0/RUN exit0. Logs `b5-ref07-persistence-green-list.log`, `b5-ref07-persistence-green.log` and result JSON are in this session scratch. The global Cargo lane was empty after completion.

Expanded actions/queue/native/Exit acceptance remain in progress. Controlled TestPlatform/native results do not certify physical Windows power/Exit, user visuals, installed or Release behavior.
