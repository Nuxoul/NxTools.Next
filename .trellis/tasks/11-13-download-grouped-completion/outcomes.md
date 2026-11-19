# Outcomes

## Delivered

- `BrowserBatchSubmit` marks only newly accepted tasks with its existing correlation in runtime `EngineTaskState`; the nonserialized completion snapshot exposes it. Duplicate handoffs retain their original task membership; no message, schema, configuration or permanent preference changes were introduced.
- `CompletionActionCoordinator` freezes original `(task, attempt)` membership before terminal observation. Independent overlapping groups settle once on terminal, cancellation or removal, counting only observed successes/failures. Attempt0→first dispatch1 remains the same member; later retries use standalone feedback. Historical terminals do not replay. Standalone400ms coalescing and the B5 power coordinator remain separate.
- The original feedback path uses the intake correlation for the grouped notification and bounded platform commands, retaining the existing15second command/result/timeout/disconnect handling and three-directory limit. The sole NotificationCenter/official NotificationList presents result text without task-location or temporary-mute actions. Configured sound, explicit directory actions, permanent preferences and power intentions remain unchanged. Existing keyed task windows still transform in place.
- Reviewed ABDM's AppComponent and BaseSingleDownloadComponent, legacy MainWindowViewModel.HandleDownloadCompleted and the behavioral-only Mister reference documented in `docs/reference-code/mister-download-manager.md`. No Mister code, comments, tests or assets were incorporated. The original official notification composition and passive popup remain the only presentation owners.

## Verification

| Boundary | Result |
| --- | --- |
| Runtime red→green: real browser batch intake/duplicate membership | 1 failed on missing runtime correlation, then 1 passed (`grouped-engine-red.log`, `grouped-engine-green.log`) |
| Real HTTP batch/engine terminal/production GPUI keyed-window red→green | 1 failed on an unwanted individual passive window from compact snapshot attempt0, then 1 passed (`grouped-http-window.log`, `grouped-http-window-green.log`) |
| Browser/engine contract suite | 107 passed (`grouped-engine-contract.log`) |
| Coordinator including B5 intentions, independent groups, failure/removal/cancellation, first dispatch, retry and history | 18 passed (`download-notice-coordinator.log`) |
| Production GPUI intake/HTTP terminal/notice, action-free grouped and standalone results, original native sound correlation and unchanged config | 2 passed (`download-notice-actions-green.log`) |
| Existing bounded platform feedback and official generation/source action for other notices | 1 passed each (`download-notice-platform.log`, `download-notice-other-actions.log`) |
| Strict affected Clippy | download tests passed (`grouped-download-clippy.log`); desktop all targets passed (`download-notice-clippy.log`) |
| Scoped Rustfmt with `skip_children=true`, scoped diff whitespace, download UI lifecycle, spec contracts, GPUI release and pinned editor gates | passed (`download-notice-static.log`) |
| Extended task-window selection | 27 passed, 2 failed (`download-notice-windows.log`) |

## Known limitations

The extended task-window selection has two independently reproducible failures: `failed_task_window_routes_raw_error_to_existing_notification_only` reads the notification list within the publishing callback before its existing deferred update; `production_task_window_reuses_one_skeleton_and_generation_across_phases` fails its Open With pending-state assertion after601seconds. Neither path was changed by the action-free notification work.

Logs are in `D:\.pi-desktop\data\scratch\7ea4a545-4448-4484-afe2-464aa37f261c`. The full download library remains outside this affected check set because its independent B6 `ref13_successful_width_prevents_repeated_extra_connection_refusals` fixture has the previously recorded obsolete native-refusal-only assertion (finishing-progress outcomes:159 passed,1 failed,4 ignored). Compiled TestPlatform, local HTTP and source gates do not establish installed Windows HWND, physical pointer/DPI, visual or user acceptance. No package, install, product launch or commit was performed.
