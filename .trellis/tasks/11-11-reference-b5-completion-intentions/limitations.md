# B5 remaining acceptance layers

Acceptance owner: coordinator. Acceptance status: `not_accepted`.

## Existing strict-lint failures

Strict all-target `-D warnings` retains these unrelated diagnostics:

| Owner | Location | Diagnostic |
| --- | --- | --- |
| Protocol | `rust/crates/nxtools-protocol/src/archive_browser.rs:563` | `unnecessary_map_or` in the existing preview-options test |
| Desktop | `rust/apps/nxtools-desktop/src/downloads/browser_handoff_impl.rs:436` | `nonminimal_bool` in the existing extension-export result owner |
| Desktop | `rust/apps/nxtools-desktop/src/downloads/settings_task_rem_tests.rs:614` | `unwrap_used` in existing official-popup pointer scrolling |

The three expressions match the pre-B5 source record `b4-baseline-lint-source.txt` in the session log root. Protocol archive_browser is unchanged against HEAD. Browser handoff is outside the B5 delta; the existing settings expression is unchanged against its B5 before-copy. No lint suppression is added. Exact final exits and logs are retained in `verification.json`; strict Protocol/Desktop are not reported as passing.

## Archive preservation prerequisites

The existing original job selection is LIST129/RUN129/PASS122/FAIL7/SKIP0/not_run0, exits0/101. The seven failures require real pre-existing C# wire exports:

- `NXTOOLS_ARCHIVE_AC_PROGRESS_WIRE`: `opt01_ac02_retry_restart_and_replacement_attempt`, `opt01_ac02_two_channel_reorder_preserves_phase_and_metric_epoch`, `opt01_ac03_each_frozen_format_uses_its_own_serialized_phases`.
- `NXTOOLS_OPT03_CREATE_WIRE`: `opt03_ac03_post_test_cleanup_failure_preserves_create_facts_and_running_slot`, `opt03_post_test_missing_location_cleanup_and_disconnect`, `opt03_post_test_preserves_native_publication_and_location`, `opt03_post_test_uses_actual_native_primary`.

Each fails at its mandatory export-environment read. The same absent-export condition is recorded in the existing Archive `root-drop-and-read-passes.md` and `preview-budget-results.md`. The relevant Archive source/tests are outside the B5 delta. This group remains failed, not skipped or passed. Draft12 and the other122 job cases pass at their original boundaries. No synthetic terminal replaces a native export. Canonical Archive remains **Phase E — Format expansion and ZIP editing / in_progress**.

## Independent layers

- Controlled typed native success is not real Shutdown/Sleep/Hibernate acceptance. Native unsupported/denied/error code propagation and held-delegate lifecycle are tested without capability, process-token or power API calls.
- TestPlatform quit request, actual Download future/client/host retirement and physical process exit are distinct. The first two have separate evidence; current-user Desktop exit and Indexer/Platform process-lifetime reader joins are not established by these tests.
- Native OS-close/IME/DPI/Shell, human visual, installed/Release and whole-workspace acceptance remain independent.
- B1–B4 retain their recorded acceptance. B5 stops for independent coordinator review; B6/B7 are outside this handoff.
