# Global fullscreen summon suppression

## Result

Global Settings → Application Blacklist owns `全屏时暂不呼出速启和剪贴板`. The old Quick Launch switch/setter is removed. One durable global policy is synchronized to both original configure commands and native pre-publication paths. Exclusive D3D and borderless foreground fullscreen suppress Quick Launch and Clipboard summons; ordinary framed/maximized windows, desktop and failed observations remain non-blocking. Clipboard capture/history are independent.

Missing global values inherit the legacy Quick Launch choice without a load-time write. Explicit global false wins, including after sibling saves and clearing application rules. Original draft/Cancel/Apply, deferred consumers, hotkey recording, target snapshots, focus, reusable panels and previews retain their owners. The compact official Small Switch/Label row shares the original blacklist table slot and 120 DIP row viewport. Settings search includes fullscreen terms.

## Verification

| Check | Result |
| --- | --- |
| Core library and temporary-file fullscreen tests | 98 + 2 passed |
| Protocol library and clipboard fullscreen wire regression | 117 + 1 passed |
| IPC library | 67 passed |
| Desktop Global Settings production tests | 40 passed, including pointer/keyboard, Cancel/Apply/failure, deferred native commands, minimum and enlarged-root long-rule scrolling |
| Quick Launch retained settings interaction | Passed |
| C# focused fullscreen/configuration/diagnostic fixtures | 7 passed, 0 skipped, 0 failed |
| PlatformHost and test-project build | 0 warnings, 0 errors |
| `cargo check --workspace --tests` | Passed |
| Scoped rustfmt for all task-touched Rust files | Passed |
| Spec contracts, GPUI release, Markdown provenance | Passed |
| Clipboard lifecycle architecture | Passed |
| `git diff --check` | Passed |

C# fixtures include independent policies, both Quick Launch triggers and Clipboard, borderless/exclusive classifications, foreground-monitor geometry, ordinary/desktop/invalid/hidden/minimized cases, native query failure, operation correlation and resumed-suppression diagnostics. The Clipboard event retains its original business payload; the shared correlation crosses the existing envelope and Rust IPC receipt diagnostic.

## Known validation blockers

Workspace-wide verification is not clean. The full Desktop run reported 1,946 passed, 24 failed and 1 ignored; the task-owned Global Settings surface has a separate passing 40-test result. Failures outside that surface remain unaddressed:

- Archive: `narrow_tree_folder_name_stays_on_one_line_inside_fixed_row`; `opt01_ac02_retry_restart_and_replacement_attempt`; `opt01_ac02_two_channel_reorder_preserves_phase_and_metric_epoch`; `opt01_ac03_each_frozen_format_uses_its_own_serialized_phases`; `opt03_ac03_post_test_cleanup_failure_preserves_create_facts_and_running_slot`; `opt03_post_test_missing_location_cleanup_and_disconnect`; `opt03_post_test_preserves_native_publication_and_location`; `opt03_post_test_uses_actual_native_primary`; `production_row_drag_read_hydration_and_result_route_share_original_snapshot`.
- Downloads: `failed_task_window_routes_raw_error_to_existing_notification_only`; `queue02_headers_have_one_styled_textarea_frame`; `background_limit_setting_keeps_manual_limit_and_validates_only_when_enabled`; `b4_e2_native_close_retires_held_probe_without_changing_reopened_task`; `b4_ref08_actual_close_and_stale_identity_preserve_reopened_window`; `b4_ref08_actual_first_focused_enter_preserves_writer_and_sqlite`; `b4_ref08_actual_first_focused_space_preserves_writer_and_sqlite`; `b4_ref08_actual_pointer_confirmation_preserves_writer_and_sqlite`; `b4_ref08_actual_refusal_retains_input_retry_and_cancel`; `b4_ref10_actual_paused_window_exposes_written_range_coverage`.
- Main window: `download_context_menu_omits_actions_that_cannot_run_for_the_selection`; `titlebar_move_and_close_reach_transport_without_destroying_the_main_entity`.
- Strict Clippy stops at `rust/crates/nxtools-protocol/src/archive_browser.rs:565` (`unnecessary_map_or`).
- Whole-workspace fmt reports unrelated files, including `quick_launch/file_dialog_search.rs` and `archive_virtual_drag.rs`; task-touched files pass the scoped check.
- `quick-launch-lifecycle.ps1` stops at its third-party file-manager contract near line1735; Clipboard lifecycle passes.

These are recorded test/gate failures, not a claim that their cause predates the task. Their owning behavior is outside fullscreen summon suppression. The full Desktop suite has not been re-certified green.

## Acceptance limits

No package/install, service changes or real product/game launch. Physical exclusive/borderless game hotkeys, mixed-DPI interaction and user visual acceptance remain unverified. The existing user worktree changes are retained; no commit was made.
