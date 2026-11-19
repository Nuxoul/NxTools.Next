# Browser handoff lifecycle and external intake

## Outcome

Automatic browser interception uses one correlated ownership transfer:

`pause and verify original -> Pending confirmation -> durable Held task -> verify browser cancellation -> Commit the same task`.

- Pending is not acceptance. Its thirty-second lease is renewed by quiet status demand; Held does not expire automatically.
- Held admission persists the task, transfer and queue together. Ordinary task actions, batch resume, queues, StartAll, schedules and reopen cannot start Held work.
- User cancellation, client/native close, probe or presentation failure and rejected acceptance terminate the same pending identity before browser restoration. Unknown browser cancellation keeps recoverable Held ownership instead of guessing.
- Commit and Abort are identity-bound and idempotent. A verified cancelled original can finish activation after worker recovery or browser-history removal. A committed receipt retains its task UUID after Remove or Redownload.
- Acceptance binds the original browser resource using the existing resolution identity. Destination, filename, category, limits, scheduling and media selection remain editable; a different re-probed URL cannot replace the browser original.
- The default noop resolver uses original Resolve admission without requiring an extension receipt. Published receipts remain checked and reused. Desktop confirmation never commits or opens an active progress window.
- The extension recovery journal is bounded and secret-free. Request context remains isolated by the original Cookie store. Manual, Blob, batch and non-GET behavior retain their original routes.
- External URL/link-recognition/floating/browser-prefilled intake automatically starts the original deferred probe. Manual panel Add, clipboard prefill and Paste remain explicit. Populated, edited-then-cleared and pending confirmation drafts are not replaced.
- Existing GPUI Kit controls, window geometry, focus/placement and the Popup layout remain unchanged. Diagnostics carry correlated, bounded lifecycle transitions rather than payloads or poll traffic.

Native Messaging remains protocol2/256KiB; Download commands are14, IPC6 and SQLite15.

## Verification

Every Rust invocation used `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`. Cargo operations were serialized and did not terminate other sessions' processes.

| Boundary | Result |
| --- | --- |
| Production extension Node suite | PASS318/318 |
| SQLite/localhost controlled lifecycle | PASS12/12 |
| BrowserHost, including actual extension/native/owned-pipe roundtrip | PASS53/53 |
| Download IPC, including draining and cleanup | PASS9/9 |
| Controlled confirmation production GPUI controls/lifecycle | PASS7/7 |
| Link-recognition production GPUI intake | PASS7/7 |
| Floating assistant production intake/projection | PASS2/2 |
| Manual/clipboard intake regression | PASS11/11 |
| Download engine/message/storage integration contracts | PASS105/16/7 |
| Strict Clippy for Download/IPC/Host/Desktop tests | PASS |
| Final Desktop strict Clippy after intake-test changes | PASS |
| `cargo check --workspace --tests` | PASS |
| Spec contracts | PASS130documents/14owners |
| GPUI release and pinned Markdown provenance | PASS |
| Download UI lifecycle / Browser Native Messaging source gates | PASS |
| Scoped formatting and `git diff --check` | PASS |
| Last full Download library invocation |154passed/2failed/4ignored |
| Last full Desktop invocation |1926passed/28failed/1ignored |

The controlled GPUI suite exercises actual footer Cancel, titlebar close, Held acceptance without additional transfer requests, edited/re-probed resource rejection, late confirmation, probe failure and failed native presentation. Link intake uses the original native-presentation acknowledgment boundary, real localhost HTTP and named official controls. Source and fixture results are not installed-browser or human visual acceptance.

## Known issues and acceptance limits

Full-suite acceptance is not green. The observed failures below remain unclassified; they are not asserted to be baseline failures or caused by this change. Intake/handoff cases have their separate current results in the verification table.

Download library:
- `engine::connection_experiments::ref13_successful_width_prevents_repeated_extra_connection_refusals`: refusal-count assertion.
- `engine::fault_recovery::slow_connection_is_shed_and_tail_is_split`: task-limit/reconnect/tail-split assertion; its HTTP fixture also reported a missing-row update.

Other observed Desktop failures:
- `archive::application::browser_host::icon_layout_tests::narrow_tree_folder_name_stays_on_one_line_inside_fixed_row`.
- `archive::application::jobs::tests::ac_repair_tests::{opt01_ac02_retry_restart_and_replacement_attempt,opt01_ac02_two_channel_reorder_preserves_phase_and_metric_epoch,opt01_ac03_each_frozen_format_uses_its_own_serialized_phases}`.
- `archive::application::jobs::tests::opt03_result_tests::{opt03_ac03_post_test_cleanup_failure_preserves_create_facts_and_running_slot,opt03_post_test_missing_location_cleanup_and_disconnect,opt03_post_test_preserves_native_publication_and_location,opt03_post_test_uses_actual_native_primary}`.
- `downloads::polish_tests::queue02_headers_have_one_styled_textarea_frame`.
- `downloads::settings_view::background_limit_ui_tests::background_limit_setting_keeps_manual_limit_and_validates_only_when_enabled`.
- `downloads::settings_view::settings_task_rem_tests::{b4_e2_native_close_retires_held_probe_without_changing_reopened_task,b4_ref08_actual_close_and_stale_identity_preserve_reopened_window,b4_ref08_actual_first_focused_enter_preserves_writer_and_sqlite,b4_ref08_actual_first_focused_space_preserves_writer_and_sqlite,b4_ref08_actual_pointer_confirmation_preserves_writer_and_sqlite,b4_ref08_actual_refusal_retains_input_retry_and_cancel,b4_ref10_actual_paused_window_exposes_written_range_coverage,b4_ref10_real_parallel_events_refresh_original_window_through_rebalance}`.
- `main_window::tests::download_context_menu_omits_actions_that_cannot_run_for_the_selection`.

Installed Chrome/Firefox Save As/pause behavior, authenticated sites, physical input, mixed-DPI, accessibility-provider and human visual acceptance remain unverified. No package/install, browser registration/profile change, product/service launch or real credentials/sites were used. No commit or staging was performed for this task; unrelated worktree changes remain outside its handoff.

## Root cause and regression protection

**Category B: cross-layer contract.** Opening/enqueuing confirmation was treated as the end of automatic handoff, although it was only Pending. Browser ownership was released before later desktop confirmation, so both owners could transfer. A window/channel acknowledgment cannot substitute for durable task admission or verified browser cancellation.

| Contract | Protection |
| --- | --- |
| Pending, Held and Committed have distinct authority | Typed engine/native outcomes; Host projection tests and real roundtrip |
| Accepted is not active | Atomic Held persistence and guards at actual task/queue/schedule/reopen boundaries |
| Browser cancellation precedes activation | Production background cancellation observation, durable phases and same-ID recovery tests |
| UI owner retires with its operation | Deferred cross-owner effects, explicit client/native cancellation and fixed-window owner transfer |
| Browser resource and receipt identity survive UI/task lifecycle | Resource-bound acceptance plus Remove/Redownload replay contracts |
| External parsing is not automatic confirmation | Production named link/floating controls and manual no-probe regression |

The executable domain contract is `.trellis/spec/browser-extension/controlled-handoff.md`; its owning package indexes and Desktop REF-04 link the same behavior. The original DownloadEngine, SQLite store, Native Messaging and NewDownloadView remain the owners; no parallel download manager, raw request journal or native-window framework was introduced.
