# Audit #9 delivery — directory-picker request identity

Delivered 2026-10-25 for parent review, **not accepted**. Audit #1 remains accepted/frozen. No other audit item is started. Stop after this delivery and await a new parent decision.

## Own change boundary

The repository had extensive pre-existing dirty changes. Separate item09 baselines were saved before writes in this session scratch; do not treat the entire Git diff as this task's changes. Existing files changed only:

- `rust/apps/nxtools-desktop/src/downloads.rs`: register private implementation/tests, replace ID+type pending with original request identity, remove DirectoryPicked broadcast, connect typed result to original owner, retire on disconnect/closed-window observation, bind settings native-close callback to exact window and defer it. Keep thin `handle_directory_result` entry because existing taskbar architecture gate uses that source boundary. +39/-111 relative to item09 baseline.
- `downloads/new_download_view.rs`: directory-local operation/revision state, Change invalidation, remove broadcast subscription, button callback to local `request_directory` using current input. +131/-52 includes rustfmt of the affected confirmed-directory Form chain only; not a layout change.
- `downloads/settings_view.rs`: equivalent local state/Change/request, remove only DirectoryPicked subscription arm; other FFmpeg/taskbar events remain. +39/-38.
- `downloads/tests.rs`: replace two artificial type-broadcast injections in the existing settings Apply/Cancel test with unknown-ID rejection and actual original-view request/typed result; enable synthetic bridge connection. +40/-10.
- `docs/reference-code/download.md`: append-only #9 reference/invariants/result entry. Existing #1/older note prefix preserved.

New files: `downloads/directory_picker.rs` (299 lines, private methods on the SAME DownloadApplication, not another entity/controller); `downloads/directory_picker_tests.rs` (854 lines, 14 production boundary tests); this task PRD/outcomes.

No changes to C#/protocol/IPC/engine/shared controls/dependencies/vendor/native adapter, no browser_handoff_impl.rs modification, no #1 keyboard test/handler change. Byte-equality against item09 baseline verified `default_enter_key_down`, `request`, `reset_schedule_draft`, `begin_probe`, `begin_probe_with_mode`, `submit`, `fixed_confirmation_draft`, `hydrate`; root keyboard capture bindings checked independently and browser_handoff_impl.rs entire bytes equal. No broad formatting, reset, commit, push or unrelated owner repair.

## Reference and composition

Read AGENTS, current spec owners/shared contracts, component selection, 0.8.0 delta/table/API index and matching download note before changes. ABDM first: desktop LocationTextField -> FilePickerUtils.rememberMyDirectoryPickerLauncher -> FileKit parent attached to LocalWindow.current; optional location dialog -> BaseOptionDialog/Compose window. Legacy second: AddDownloadWindow captured ViewModel before awaited StorageProvider picker; constructor -> WindowAnimationService.Attach -> OnOpened/OnClosed/DetachViewModel, SettingsWindow Apply/Cancel/base close and MainWindow.ShowSettingsWindow keyed secondary-show. FileKit/AWT/Avalonia native base unavailable; not claimed inspected. Next exact boundary WindowsPlatformAdapter -> DirectoryPicker -> StaDialogRunner -> FolderBrowserDialog.ShowDialog inspected read-only and unchanged.

Released Kit0.8.0 InputState.set_value is silent hydration and clears undo history, replace_all/real edits emit Change. pre0.3.9 App.flush_effects processes queued Emit/Defer FIFO. Retain actual styled Settings/Form/Input/outlined picker Button tiers and parent composition; no new controls, native dimensions, focus policy, palette or vendor patch.

## Contract and implementation

A view captures a new UUID operation plus directory edit revision/current value and its actual weak entity/window at the original button callback; local token exists before deferred application admission. Original DownloadApplication registers the pending identity BEFORE `send_command_with_id_correlated(command, id, id)`. One global pending policy remains, repeated same-view click does not emit another command. Competing type/view request is rejected and its own local token released without altering the first request.

User Change increments local revision and invalidates the token, including A->B->A; cleanup is deferred beyond the view lease. Silent hydration does not fabricate a user Change. Normal accepted result uses silent set_value once and explicitly advances the local revision, marks NewTask save_directory_explicit, clears only originating validation, and refreshes settings category presentation. Picker does not save config; original Apply/Cancel still owns persistence.

Result first hop matches command/current weak view/window/revision/value/busy state and records result_received once while retaining pending. Two queued defers allow same-update Change effects to settle. Final hop independently rechecks exact ID and current origin/draft before take/apply/failure notice; old results cannot clear new pending. Apply validates again inside original source window/view callback. Window close/removal/replacement/native-close observation/disconnect retires logical ownership and releases only matching local token; never kills host/native modal/process. Actual fixed confirmation replacement uses unchanged original handoff and fresh view; source removal retires picker.

Diagnostics use existing command ID as correlation through IPC: accepted, retired(reason), completed(applied), cancelled, failed(native_error_code/presence), elapsed at terminal states. No directory/path/text payload or raw result Debug logged, no high-frequency edit logs/polling/timers. Existing native API boundary retains its diagnostics/cancellation; no new native measurement claim.

## Red/green chronology and commands

Every Rust command inherited CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, RUSTFLAGS=-C codegen-units=256, serial in this session, full-operation timeouts. Natural Cargo/rustc occupancy was waited out; no request to other conversations to pause/reserve/reorder, no process/session termination, no duplicate own builds, no timed-out surviving wrapper.

1. Initial fixture attempts could not enqueue because bridge had no active synthetic connection. Changed test to mounted actual Button bounds/visibility and enabled `set_connected_for_contract_test(true)`. Those earlier failures are fixture setup, NOT behavioral reds. No native process was launched.
2. **Before production repair:** `cargo test -p nxtools-desktop downloads::directory_picker_tests:: -- --test-threads=1`: **RUN5, 1 pass/4 fail** (1.57s). Real button->PickDirectory->typed CommandResult overwrote replacement Settings, another same-type view, A->B->A draft, and a draft edited after result claim. Normal original draft/Cancel control passed. Log `item09/picker-red.log` SHA256 `8a24aa49ffa9e4da58b9eeab78626fe36af10c11622e0a97d017ea57026d6347`.
3. Initial repair RUN13 (5 picker +8 accepted keyboard) PASS, then expanded matrix RUN19 (11+8) PASS. Additional final three tests and first-hop validation added. Counts were corrected in communication; no invented test cases/results.
4. **Final focused:** `cargo test -p nxtools-desktop -- --test-threads=1 downloads::directory_picker_tests:: new_download_keyboard_`: **22 passed /0 failed/0 ignored** (9.34s), 14 picker +8 keyboard. Log `picker-keyboard-final.log`, SHA256 `f27724a3a73ca56d9fc04060a5ae2a1a043b3d3d416e7942f350668f5af346ce`.
5. **Downloads regression:** `cargo test -p nxtools-desktop downloads:: -- --test-threads=1`: **182 passed/0 failed/0 ignored** (43.37s). Log `downloads-regression.log`, SHA256 `0575bea43828efb1c675a6e571cf77db6c14a8d56e18f3e3b829bdf1554b4df2`. This preceded the final additional first-hop current check; final focused and integrated run validate that final source too.
6. **Strict Clippy** `cargo clippy -p nxtools-desktop --tests -- -D warnings`: PASS final; **workspace** `cargo check --workspace --tests`: PASS; **fmt** `cargo fmt --all -- --check`: PASS. Earlier fmt failure was only added-fragment formatting, corrected locally; no lint suppression. Logs `clippy-final.log`, `workspace-check.log`, empty `fmt-final.log`.
7. **One final integrated Desktop run only:** `cargo test -p nxtools-desktop -- --test-threads=1`: **1173 passed /1 failed /0 ignored**, 189.40s, exit101. This is FAILED, not full-suite acceptance. Log `desktop-full.log`, SHA256 `d5f8e084dfb19a26e409513c47b9f9b98c98efc9c82eed0e11b6b5abb004c58b`.
8. Own-delta-generation ad-hoc Python script initially had a syntax typo, failed before any writes, then corrected; no product/source change from that failure.

Integrated failure: `archive::application::tests::production_password_application_original_dialog_ticket_and_deferred_grant_never_start_ole`, panic `archive/application_tests.rs:954:10`, `official unlock Button`. No backtrace enabled in this run. Same unresolved owner/test was recorded by #1 parent review; this task did not modify Archive or initialize NewDownloadView in that fixture. Do not treat Extension's pass here as proof its previous integrated reentry defect was repaired. No full rerun/green hunting or unrelated fixes.

## Acceptance-to-test map (all 14 final picker tests passed)

Names below have prefix `downloads::directory_picker_tests::directory_picker_`:

| Requirement | Actual regression suffix / assertions |
| --- | --- |
| Original same-window success once; Cancel keeps disk | `normal_result_changes_only_original_draft_and_cancel_keeps_disk`: actual Button command, directory changed once, duplicate ID no second presentation change, temp config unchanged and actual Cancel |
| Closed/reopened same type | `closed_settings_result_cannot_write_reopened_draft`: actual simulate_close, different EntityId, old success does not write replacement |
| Multiple same-type views | `same_type_views_do_not_share_a_result`: two actual settings views, only origin changes |
| Edit before result, same-turn ABA | `same_turn_aba_user_edit_rejects_old_result`: original InputState.replace_all B->A, typed result same update, original draft retained |
| Result before deferred edit | `result_then_edit_before_deferred_apply_keeps_new_draft`: claim first, actual edit second, newest draft preserved |
| Claim->close/reopen/new pending/old failure | `result_claim_then_close_reopen_cannot_apply_or_fail_replacement`: retained old view unchanged, new view unchanged, old failure neither clears new pending nor adds notice |
| Repeat/cancel/native fail/disconnect/new-ID retry | `cancel_failure_disconnect_retry_and_old_ids_are_isolated`: real repeated click no second command, empty success cancel, native failure notice, disconnect cleanup, old success IDs inert against new pending |
| Queue full/disconnected send | `dispatch_rejection_releases_original_token_and_can_retry`: bounded synthetic queue truly full, failed send leaves no pending/draft mutation, later real clicks succeed |
| Settings vs NewTask; explicit/save/Apply | `new_task_settings_isolation_explicit_draft_and_apply_policy`: one global pending, rejected competing type retry allowed, correct typed result only updates origin, explicit/validation preserved, neither picker saves; actual Apply persists selected settings directory while QuickLaunch section remains unchanged, no download task |
| NewTask ABA/close/reopen/hydration | `new_task_aba_edit_close_reopen_and_silent_hydration`: actual InputState ABA rejection, silent same-value hydration not fake edit, old source result cannot alter replacement/new pending |
| Delayed old close callback | `old_settings_close_callback_preserves_current_window_and_request`: old exact window close notification cannot clear replacement registration/pending |
| Claim->ABA in same turn | `result_claim_then_same_turn_aba_is_not_applied`: catches Emit/Defer ordering, later retry succeeds |
| Claim failure->close | `failure_claim_then_closed_origin_has_no_current_notice`: final failure ownership check suppresses stale notice |
| Actual fixed-stage replacement | `fixed_confirmation_replacement_retires_original_view_request`: original create_fixed_confirmation (HWND-less synthetic path), source/target IDs differ, pending retired, neither retained source nor target receives late path |

Every emitted picker command is checked to carry correlation_id==id. Tests use actual DownloadApplication + original Button/InputState/window lifecycle + typed results, memory engine and temporary synthetic config only; helper predicates are not substituted for acceptance.

## Static checks / own evidence

PASS component catalog, release contracts, Markdown editor contracts, spec contracts, Download lifecycle, taskbar overlay lifecycle, UI token boundaries and scoped diff check. These are source/provenance checks only. Own baseline/delivery manifest and complete own-delta texts are generated in this session item09 scratch and returned by session communication to parent; no cross-session scratch access assumption. Existing #1 parent-review is untouched.

## Remaining limits and stop

No product/PlatformHost/service/native picker launch, real user configuration/database, registry, packaging/install, commit/push or recursive delegation. Native/physical IME/keyboard/mouse, actual HWND/modal lifetime, focus restoration, DPI/reader/screenshots/installed/user acceptance NOT executed. HWND-less fixed-stage fixture is synthetic lifecycle evidence only. Single fullDesktop remains FAILED as above; parent review and scoped acceptance pending. This slice is complete for review, not release acceptance. Stop now; do not start another item.
