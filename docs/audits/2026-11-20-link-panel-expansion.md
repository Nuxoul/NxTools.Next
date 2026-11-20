# Link panel expansion

## Behavior

- The footer shows the link count only.
- Detection retains every match within the existing 64-line / 128-distinct-candidate budget, rather than stopping at eight matches.
- The official Small ghost expander defers sizing to the original ClipboardApplication/native placement owner. Logical bounds fit the retained monitor work area and convert once to device pixels.
- Short lists expand to their natural client height. Longer lists use one official vertical scroll viewport with a fixed count footer; the last row and Copy action remain reachable.
- Existing row, Copy, Download, passive-window, capture-generation and auto-hide behavior remains unchanged.

Source contract: [link detector reference](../reference-code/link-detector.md#complete-expansion-and-count-only-footer).

## Verification

| Check | Result |
| --- | --- |
| Production detector with 32 distinct copied URLs | All 32 retained in source order |
| `cargo test -p nxtools-desktop link_ -- --test-threads=1` | 65 passed |
| Production expander/native bounds fixture | Passed: three and 32 rows, 144DPI, negative-origin work area |
| Production fixed-client and scroll fixture | Passed: last row and Copy reachable; footer stationary |
| Scoped Link section of `clipboard-ui-lifecycle.ps1` | Passed |
| GPUI release, Markdown editor provenance and spec contracts | Passed |
| Scoped rustfmt and diff whitespace | Passed |
| Full desktop unit suite | 2054 passed, 27 failed, 1 ignored |
| Strict desktop Clippy including tests | Failed: 103 diagnostics; none reference the files changed for this slice |
| Complete clipboard UI source gate | Failed before Link checks: expects `application.request_clipboard_export(cx)`, while the committed call includes `source_window_handle` |

## Full-suite failures

These are the observed full-suite failures, not a claim that all were reproduced on an unmodified baseline.

```text
archive::application::browser_host::icon_layout_tests::narrow_tree_folder_name_stays_on_one_line_inside_fixed_row
archive::application::jobs::tests::ac_repair_tests::opt01_ac02_retry_restart_and_replacement_attempt
archive::application::jobs::tests::ac_repair_tests::opt01_ac02_two_channel_reorder_preserves_phase_and_metric_epoch
archive::application::jobs::tests::ac_repair_tests::opt01_ac03_each_frozen_format_uses_its_own_serialized_phases
archive::application::jobs::tests::opt03_result_tests::opt03_ac03_post_test_cleanup_failure_preserves_create_facts_and_running_slot
archive::application::jobs::tests::opt03_result_tests::opt03_post_test_missing_location_cleanup_and_disconnect
archive::application::jobs::tests::opt03_result_tests::opt03_post_test_preserves_native_publication_and_location
archive::application::jobs::tests::opt03_result_tests::opt03_post_test_uses_actual_native_primary
archive::application::tests::production_row_drag_read_hydration_and_result_route_share_original_snapshot
downloads::conflict_tests::failed_task_window_routes_raw_error_to_existing_notification_only
downloads::polish_tests::queue02_headers_have_one_styled_textarea_frame
downloads::settings_view::background_limit_ui_tests::background_limit_setting_keeps_manual_limit_and_validates_only_when_enabled
downloads::settings_view::settings_task_rem_tests::b4_e2_native_close_retires_held_probe_without_changing_reopened_task
downloads::settings_view::settings_task_rem_tests::b4_ref08_actual_close_and_stale_identity_preserve_reopened_window
downloads::settings_view::settings_task_rem_tests::b4_ref08_actual_first_focused_enter_preserves_writer_and_sqlite
downloads::settings_view::settings_task_rem_tests::b4_ref08_actual_first_focused_space_preserves_writer_and_sqlite
downloads::settings_view::settings_task_rem_tests::b4_ref08_actual_pointer_confirmation_preserves_writer_and_sqlite
downloads::settings_view::settings_task_rem_tests::b4_ref08_actual_refusal_retains_input_retry_and_cancel
downloads::settings_view::settings_task_rem_tests::b4_ref10_actual_paused_window_exposes_written_range_coverage
main_window::extensions_page::environment::package_tests::runtime_package_override_save_is_independent_from_trust_and_restores_inheritance
main_window::extensions_page::extensions_settings::tests::extension_purpose_and_clipboard_usage_render_in_overview_and_first_detail_page
main_window::extensions_page::extensions_settings::tests::remediation_remove_requires_confirmation_and_cancel_preserves_config
main_window::extensions_page::extensions_settings::tests::remediation_stale_removal_confirmation_cannot_remove_newer_package
main_window::extensions_page::extensions_settings::tests::w9_manager_pointer_trust_enable_and_revoke_are_separate_explicit_operations
main_window::tests::download_context_menu_omits_actions_that_cannot_run_for_the_selection
main_window::tests::titlebar_move_and_close_reach_transport_without_destroying_the_main_entity
tools::polish_tests::r6_capture_reuses_one_window_and_retires_superseded_six_second_timer
```

The unchanged `r6_capture...` fixture also fails in isolation at `first capture panel`. It sends a capture and waits for detection but supplies no reply to the current native monitor query. Both the fixture and the monitor-query-first presentation chain are present in the committed baseline. The production expander/native-result test and independent window-timer regression pass.

## Limitations

The single-capture detection work budget remains 64 lines and 128 distinct candidates. Synthetic platform/IPC fixtures do not certify installed Windows pointer behavior, physical mixed-DPI behavior or user visual acceptance. No Release package or installed application was updated. Full-suite, strict-Clippy and complete clipboard-gate failures above remain open.
