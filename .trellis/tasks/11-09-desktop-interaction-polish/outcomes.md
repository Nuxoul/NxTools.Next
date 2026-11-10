# Desktop interaction polish — outcomes

## Delivered behavior

### Archive

- Integration exposes separate selection-named ZIP quick-create, 7z quick-create and editable New Archive commands. Fresh configuration and a missing command field recommend all three; explicit empty, custom and historical command arrays remain exact. Integration still requires explicit Apply.
- The existing Shell helper supplies dynamic selection-aware titles through real `IExplorerCommand::GetTitle` and forwards the complete ordered selection once through `Invoke`. ZIP/7z use the original creation FIFO/Job without opening the editable form. The native correlation ID remains the Job operation/session identity.
- New Archive has a 144 DIP independently scrolling source viewport, filename/size rows, full-path tooltips and original add/paste/remove actions. Ordinary output fields are compact; advanced/password/footer behavior remains intact.
- Archive settings use Download-style filled groups and shared aligned field rows, retaining all navigation, values, actions and section-owned saves.

### Download

- Progress filenames use the 14/20 title tier above 12/16 body text. The collapsed window prioritizes filename, progress/bytes, speed and ETA; technical capabilities, limits, connections and segments remain in the original expansion.
- The task window retains its keyed entity and HWND, fixed footer, 440×184 collapsed size and 440×280 expanded size. The collapsed body does not scroll.
- Server information shows host, short route and observed HTTP request-to-response-headers duration. The duration uses semantic success for 0–100 ms, warning for 101–300 ms and danger for 301+ ms; missing observations show no duration. No peer IP is projected into visible text, tooltips or accessibility copy.
- Category rules heading, help, official Medium searchable Select and Small outlined Add action share one row at ordinary desktop width and wrap at narrow widths. Stable category IDs map into the original settings draft. Real selection, rename, reorder, deletion, Apply and Cancel retain their existing owners. Cross-entity input synchronization is deferred.
- Native icon extraction owns an independent raw image-list reference per query. Corrupt persisted PNGs regenerate through the original bounded cache/STA owner. Real disconnect retires connection-scoped failure budgets; duplicate Connected events cannot bypass retry caps. Successful cache entries and stale-result guards remain intact, with the official neutral File asset as fallback.

### Quick Launch

- Result entrance settings offer Slide Up, Slide From Right, Fade and None. Released Base Timing/EaseOut drives a finite 120 ms Tween; slide distances are 8 DIP from below and 12 DIP from the right.
- Newly admitted visible identities animate once per query generation, including delayed provider batches. Selection, reorder, icon completion, repaint and virtual-row remount do not restart admission. Offscreen arrivals, reduced motion, disabled/unknown effects and retired sessions settle immediately.
- Ordinary typed search uses a stable monitor-clamped 620×530 DIP expanded viewport rather than count-driven resizing. Search-only mode remains 60 DIP; action and recent-directory geometry remain unchanged.
- The fixed official Button keeps hit/focus rails stable around animated inner content. Preview, focus, selection and native placement remain with their original owners. The existing settings saver merges only the entrance field and leaves the live value unchanged on persistence failure.

## Verification

| Boundary | Result |
| --- | --- |
| Core | 105 passed: 96 library, 9 integration |
| Protocol | 116 passed, 1 ignored |
| Desktop archive IPC | 5 passed |
| Desktop targeted archive/download/Quick Launch regressions | 113 passed, 0 failed, 0 ignored |
| HTTP integration | 10 passed |
| C# controlled build | 0 warnings, 0 errors |
| C# selected fixtures | 17 selected/run/passed; 0 skipped or failed |
| Architecture/source gates | `spec-contracts.py`, `gpui-release-contracts.py`, `markdown-editor-contracts.py`, `archive-phase-f-shell.py`, `download-ui-lifecycle.ps1`, `quick-launch-lifecycle.ps1` passed |
| Scoped Rust formatting | 33 files passed; shared Core `lib.rs` was not whole-file reformatted |
| Scoped diff whitespace | 57 paths passed |
| Bounded integration review | No concrete in-scope defects or relevant coverage gaps found |

The targeted GPUI set exercises ordinary/narrow settings widths, 14/16/18 root text sizes, real Select popup interaction and last-option scrolling across 24 categories, actual input/Apply persistence, original Cancel/reorder behavior, progress/footer geometry, task close/reopen/cancel, delayed result admission, stable native-bound requests and motion retirement. The C# fixtures cover complete selection and helper lifetime, dynamic quick-create titles and recommendations, parallel native image-list ownership, corrupt PNG regeneration, safe native error categories and completed executable icons. Fixture builds use the existing `ShellSelectionIdentityPath` seam with matching controlled identities; production approval identities remain unchanged.

Verification logs are retained in the session scratch directory under `verify-desktop-focused-complete.log`, `verify-core.log`, `verify-protocol.log`, `verify-ipc.log`, `verify-csharp-bound-build.log`, `verify-csharp-bound-fixtures.log` and `verify-final-source-gates.json`.

## Known issues and acceptance limits

### Full-suite and lint status

The complete Desktop run recorded 1803 passed, 16 failed and 1 ignored, with HTTP integration 10/10 passed. Task-related boundary cases are covered by the passing 113-test targeted set. The full suite is not recorded as green.

Eight preserved baseline failures were observed:

- `archive::application::jobs::tests::ac_repair_tests::opt01_ac02_retry_restart_and_replacement_attempt`
- `archive::application::jobs::tests::ac_repair_tests::opt01_ac02_two_channel_reorder_preserves_phase_and_metric_epoch`
- `archive::application::jobs::tests::ac_repair_tests::opt01_ac03_each_frozen_format_uses_its_own_serialized_phases`
- `archive::application::jobs::tests::opt03_result_tests::opt03_ac03_post_test_cleanup_failure_preserves_create_facts_and_running_slot`
- `archive::application::jobs::tests::opt03_result_tests::opt03_post_test_missing_location_cleanup_and_disconnect`
- `archive::application::jobs::tests::opt03_result_tests::opt03_post_test_preserves_native_publication_and_location`
- `archive::application::jobs::tests::opt03_result_tests::opt03_post_test_uses_actual_native_primary`
- `main_window::tests::download_context_menu_omits_actions_that_cannot_run_for_the_selection`

The independent correctness-task test `clipboard::asset_tests::ref01_row_uses_saved_settings_at_activation_and_stale_close_preserves_reopened_panel` also failed its stale-copy assertion. That implementation is outside this delivery.

Strict combined Clippy did not pass: `downloads/browser_handoff_impl.rs:436` reported `nonminimal_bool`, and `tools/link_panel.rs:286` reported `unnecessary_map_or`. These files were not changed by this task. Production lint stopped before all test-target lint could be established. No lint suppression was added. Full-suite and lint evidence is retained in `verify-desktop.log` and `verify-clippy.log`.

### Native and visual acceptance

An isolated Debug Archive Settings preview created a real 1060×649 window at scale 1. UI Automation observed the settings search, navigation, switches, NumberInput and clear action. The owned preview was closed. This establishes native window/control creation, not screenshot or visual acceptance.

Screenshot interpretation, user visual approval, installed Explorer/Registry integration, physical pointer/keyboard acceptance and mixed-DPI acceptance remain unperformed. Controlled GPUI/COM fixtures do not establish those acceptance levels.

This is a source delivery. No packaging, installation, real Registry/service changes, user-data mutation, commit or push was performed. Existing dirty changes and independent-task ownership were preserved.

## Implementation records

- [Implementation](implement.md)
- [Archive references and component selection](archive-slice.md)
- [Native icon/cache boundary](icon-slice.md)
- [Download layout and settings boundary](download-layout-slice.md)
- [Quick Launch admission and motion](quick-launch-slice.md)
