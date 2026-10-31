# Audit #8 — command naming and ellipsis

## Scope
Direct current-session implementation, no delegation. User-selected seams: actual production Button/menu labels, tooltip/a11y synchronization, original synthetic pointer/keyboard single dispatch, cancellation retaining synthetic data, long Chinese content within original clients. No complete Desktop, real import/export/delete/user data/native/picker/Shell/Host/service/package/install/commit/push.

Read AGENTS, component-first/current0.8.0/table/API, original design guide651–718, applicable Desktop/shared contracts and matching Clipboard/Download/Tools references before changes. Guide700–705 requires single U+2026 on commands opening dialog/sheet/separate window or requiring more input; not on immediate work just because it is asynchronous. Result/object wording must match actual owner chain. No business/confirmation/permission/ID changes.

## Source-verified chains and intended boundary
- main_window/clipboard_page.rs Buttons -> deferred ClipboardApplication request_*: Export/Import use PickClipboardArchivePath; Migration uses PickClipboardStorageDirectory. Success+empty data retires picker without worker operation. Backup and Compact immediately dispatch original serialized worker into fixed backups directory; they do not prompt and retain no ellipsis.
- main_window/download_page.rs header -> deferred original open_settings/open_schedule/open_new_download_window. Those entries need ellipsis. Context task-window opens separate surface. Cancel batch is immediate task cancellation, not dialog Cancel. List remove uses original Batch Remove, not filesystem deletion.
- request_delete_confirmation currently removes list only when task_removal_deletes_file=false, otherwise opens original confirmation. Presentation must use current same policy snapshot for accurate label (from list remove versus delete file ellipsis); do not alter that branch or confirm/cancel.
- Tools Configure buttons open/reuse original keyed460x320 settings windows; add ellipsis. Original card-body toggle remains immediate and separate.

Potential source paths are presentation files above plus a narrow read-only DownloadApplication policy getter if needed; original callbacks/state/native/worker/engine policies remain protected. New cfg(test) regressions reuse existing fixture seams rather than new dispatcher/controller. Existing role numbers/rem/layout/colors/vendor/dependencies unchanged. #3 semantic purpose/object names and #4 full identifier prompts cannot be weakened.

## Test strategy
One vertical red/green at a time: Clipboard actual labels and original picker once/cancel; Downloads actual header/menu/conditional policy labels and cancel confirmation retaining sentinel+task, direct remove preserving file, task Cancel retaining record; Tools actual Configure label and keyed-window pointer/keyboard once with original close. Render genuine Chinese labels at default/min clients, observe current actual bounds, preserve IDs and official focus/release owners. No direct business-method shortcuts as positive gesture evidence.

All Rust commands inherit jobs1/incremental0/codegen-units256, start naturally idle and run serially/full-operation timeout. A bounded opening observation first saw independent Cargo83436/test87140, then both naturally ended; idle was observed before own verification. No process kill/pause/yield/lock bypass. Preserve every failed attempt and exact live-dirty delta, not HEAD. Old manifests read-only.

## Baseline
Captured2026-10-26T02:07:18.160673+00:00:373 current source/contract files and2676 prior evidence files. Guard target bytes before each edit, stop an independently moved path, attribute unrelated changes separately. Existing dirty work is the baseline, not owned by this task.
