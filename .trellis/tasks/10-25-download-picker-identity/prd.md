# Audit #9 only — download directory picker identity

## Scope and gap
A late PickDirectory result currently broadcasts by Settings/NewTask type and can overwrite another window generation or a newer directory draft. Bind the original actual weak view + GPUI window + directory edit revision + operation ID; revalidate at final deferred application. Keep one DownloadApplication pending slot and original C# picker. No other audit item, layout/theme/shared control, protocol/native/engine/vendor change. Audit #1 accepted keyboard handler is frozen; rerun its eight regressions.

## Expected files / ownership
`downloads.rs` pending/result/disconnect/window-close routing; private `downloads/directory_picker.rs` implementation on the same application owner (not a new controller); original new_download_view.rs/settings_view.rs local directory draft and callbacks; direct directory_picker_tests.rs plus existing picker test integration; append-only download reference and these single-item records. Save per-file dirty baselines in own item09 scratch before editing. Preserve unrelated changes and previous #1/parent-review.

## Reference / component decision
Read download reference first, ABDM desktop LocationTextField -> rememberMyDirectoryPickerLauncher -> FileKit dialog parent attached to current AWT window, then old AddDownloadWindow captured ViewModel picker result and SettingsWindow constructor/animation/Open/Close/Apply. Next WindowsPlatformAdapter -> DirectoryPicker -> StaDialogRunner -> FolderBrowserDialog.ShowDialog remains unchanged. Avalonia/FileKit/AWT native internals unavailable, no native acceptance inferred. Keep released Kit0.8.0 Settings/Form/Input and current outlined picker Button variants/sizes/parent composition, exact pre0.3.9. No added control or geometry.

## Required invariants
- Original live view/window only; type is not identity. No broadcast result.
- Register local owner and application pending before command send with the same command/correlation ID. Preserve one-pending repeated-click suppression; retired/cancelled/failed/disconnected requests allow retry.
- User InputEvent::Change increments revision (including A->B->A); set_value hydration is silent. Picker apply uses silent set_value once, explicitly updates revision/presentation/save_directory_explicit and clears original validation only on actual accepted apply. No autosave.
- Result claim and final deferred apply independently check operation/view/window/revision/current value. Same-turn queued Change must settle before final apply. New pending cannot be cleared by old IDs, stale failures cannot show errors for a replacement.
- Window removal/replacement/native close and disconnect retire logical ownership only; never terminate host/dialog/process. Keep original C# cancellation capability and lifecycle.
- Diagnostics: existing command/correlation, bounded accepted/terminal states, elapsed/native code only; no directory/text/path payload or high-frequency edit logs.

## Verification / stop
Before fix, actual production views/buttons + typed PlatformCommandResult tests go red for close/reopen and ABA/second-hop races. Include normal same-origin once-only success, Settings/NewTask isolation, multiple same-type views, native-close/Cancel/Apply, duplicate, cancellation/failure/disconnect/retry/stale ID/queue-full, real temporary config and in-memory engine. No live picker/network/product/native/user data.
Run focused red/green, Downloads + #1 keyboard, strict Desktop Clippy/workspace tests check/fmt and relevant static gates; at most one final full Desktop integration. Every Rust shell jobs1/incremental0/codegen256, natural idle wait only. Never ask other sessions to pause/reserve/reorder or terminate processes. No package/install/commit/push/delegation. Deliver exact own-delta/manifest via communication, separate source/compiled/synthetic/native/physical/DPI/reader/user acceptance, then stop for parent review before any other item.

## Delivery status
Delivered for parent review, not accepted. Final picker14+acceptedkeyboard8=22/22,
Downloads182/182, strictClippy/workspacecheck/fmt and relevant source gates pass.
Single fullDesktop1173passed/1failed in unchanged Archive Unlock fixture; remains FAILED.
Reference/source/contract/tests/limits/own-deltas are recorded in outcomes.md. No native,
physicalIME/DPI/reader/user acceptance. Stop here; no next item without new single-item dispatch.
