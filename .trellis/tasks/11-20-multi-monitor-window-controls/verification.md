# Verification — 2026-11-20

## Delivery boundary

R1–R7 are implemented through their existing Rust/GPUI/C# owners. Listary6.3.5.94 managed window placement is the primary launcher reference; AB Download Manager and legacy NxTools inform download interaction. Explicit configuration, accepted requests, queue concurrency, retained history/index data and unrelated installer work are preserved.

The source checkpoint is not release or installed-user acceptance. No package, installation, live Registry mutation, service operation or commit was performed.

## Compiled and synthetic evidence

| Check | Result |
| --- | --- |
| Core unit tests | 106 passed, 0 failed |
| Protocol unit tests | 120 passed, 0 failed |
| IPC unit tests | 73 passed, 0 failed |
| Desktop test target | Compiles |
| Quick Launch confirmed presentation, actual panel expansion limits and delayed hide receipt | 3/3 passed |
| Archive progress/reuse/visible completion/native activation retirement | 3/3 passed |
| Main-page feature Switch pointer/keyboard and persisted section | 1/1 passed |
| Clipboard/Quick Launch disablement, save failure, stale events and temporary pause | 2/2 passed |
| Controlled-browser Held/Commit and existing handoff | 9/9 passed |
| Download-window lifecycle and native keyed close | 12/12 passed |
| Installed archive Apply terminal success/warning/failure | 1/1 passed |
| Floating search original input/reuse/session/live edit | 2/2 passed |
| Clipboard link-detection generation/cancellation/worker | 5/5 passed |
| Explicit link position, default screen notification and notification Cancel/Save/failure | 3/3 passed |
| Link-panel position containment | 1/1 passed |
| C# geometry/Clipboard admission/archive restoration selection | 8 run, 8 passed, 0 skipped/failed |
| C# configuration diagnostic/enable/pause/geometry selection | 3 run, 3 passed, 0 skipped/failed |
| Installer uninstall cleanup fixtures | 10/10 passed |
| Inno Code syntax with Output=no | Passed; no installer produced/launched |
| Multi-monitor first-show/reuse/retirement and work-area geometry | 6/6 passed |

The final single Desktop test invocation selects50 related tests: **50 passed, 0 failed, 0 ignored**. This includes the additional controlled-Switch/Settings compatibility regressions selected by the lifecycle filters. Strict lint is recorded at closeout below.

## Static evidence

Passed: locked GPUI release and Markdown editor provenance gates; spec contracts; installer update lifecycle; archive Phase F source gate; Clipboard, Desktop-shell and Download lifecycle source gates; git diff whitespace check.

Two source gates remain failing:

- `quick-launch-lifecycle.ps1:91` requires `.font_family(UI_FONT_FAMILY)`, absent from the committed panel.
- `clipboard-ui-lifecycle.ps1:26` requires `request_clipboard_export(cx)`; the committed page already passes `source_window_handle`.

Task-added Rust code is formatted. Whole-file rustfmt still reports preexisting fragments in `main_window/tests.rs` around302/642, `main_window/tools_page.rs` around296 and `quick_launch/file_dialog_search.rs` around1008/1015. Those unrelated fragments are retained.

## Full Desktop suite

The integrated run reports **2019 passed, 35 failed, 1 ignored**, total2055. It is not a green full-suite result. The feature-disable, Switch, assistant-search and position/default failures have passed focused verification. The two multi-monitor cases have a separate closeout result. The full suite was not rerun after focused fixes.

The remaining integrated failures are retained as unresolved findings, not attributed to this task without evidence:

| Tests / group | Observed failure |
| --- | --- |
| `ac_repair_tests`: opt01_ac02 retry/reorder and opt01_ac03 frozen formats (3) | Required C# production progress-wire export not supplied |
| `opt03_result_tests`: cleanup, missing location, native publication and primary result (4) | Required NXTOOLS_OPT03_CREATE_WIRE export not supplied; archive-worker fixture executable unavailable |
| `browser_host::icon_layout_tests::narrow_tree_folder_name_stays_on_one_line_inside_fixed_row` | Folder label100×26 exceeds one-line expectation |
| `application::tests::production_row_drag_read_hydration_and_result_route_share_original_snapshot` | Receiver-folder ordinal expected None |
| `conflict_tests::failed_task_window_routes_raw_error_to_existing_notification_only` | Expected one error notification; observed zero |
| `polish_tests::queue02_headers_have_one_styled_textarea_frame` | Actual frame713×120 differs from expected756×120 |
| `background_limit_ui_tests::background_limit_setting_keeps_manual_limit_and_validates_only_when_enabled` | Programmatically set zero was not rejected in the immediate draft assertion |
| `settings_task_rem_tests`: b4_e2 native close; b4_ref08 close/stale, first Enter, first Space, pointer confirmation, refusal; b4_ref10 paused coverage (7) | Real16-byte task completed before fixture pause; fixture explicitly uses1 connection |
| `settings_task_rem_tests::b4_ref10_real_parallel_events_refresh_original_window_through_rebalance` | Expected partial rebalanced event not observed |
| Extension environment `runtime_package_override_save_is_independent_from_trust_and_restores_inheritance` | environment-inherit action not reachable |
| Extension settings `remediation_remove_requires_confirmation_and_cancel_preserves_config`, `remediation_stale_removal_confirmation_cannot_remove_newer_package` | Remove action not reachable at minimum viewport |
| Extension settings `w9_manager_pointer_trust_enable_and_revoke_are_separate_explicit_operations` | Permission warning bounds absent |
| `download_context_menu_omits_actions_that_cannot_run_for_the_selection` | Unavailable action omission assertion failed |
| `titlebar_move_and_close_reach_transport_without_destroying_the_main_entity` | Observed59px rather than expected4px |
| `r6_capture_reuses_one_window_and_retires_superseded_six_second_timer` | First capture panel unavailable |

These27 findings exclude the eight task-specific failures covered by focused follow-up. They are not a newly measured27-failure full-suite result.

## Acceptance limits

Not performed: physical mixed-DPI/negative-origin multi-monitor first-frame and focus acceptance; actual native passive preview/link/notification behavior; installed browser progress; normal/silent/launch-unchecked original-user installation; protected redirect/existing-instance Apply against an installed product; actual Registry/Explorer default association and uninstall restoration. TestWindow, fake stores and source gates do not certify these boundaries.

## Closeout

- Related Desktop regressions:50/50 pass; multi-monitor6/6, Core106/106, Protocol120/120 and IPC73/73 pass.
- Strict Clippy with `--tests -- -D warnings` is not green. The Core/Protocol/IPC selection stops on10`unwrap_used` findings in unchanged `nxtools-core/tests/markdown_reader_settings.rs`. Desktop reports103findings in unchanged About emblem, extension activity, existing main-window test, Markdown reader and history-filter code. Categories include `unwrap_used`, `ok_expect`, `unnecessary_lazy_evaluations`, `large_enum_variant`, `nonminimal_bool` and `let_unit_value`. Task-added tests no longer appear in the Desktop diagnostics.
- Workspace-wide `cargo check --workspace --tests` was not run.
- Implementation is complete; whole-suite, lint and physical/installed acceptance remain open. Task status stays in_progress.
