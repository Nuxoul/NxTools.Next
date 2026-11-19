# Default request headers — execution evidence

Status: **accepted at the [coordinator's stated source/TestPlatform/owned HTTP-settings boundaries](coordinator-acceptance.md)**. Original implementation evidence remains below.

## Actual HTTP default

Original Core DownloadSettings → DownloadHttpPolicy snapshot → engine enqueue/resume → original probe/HTTP transfer/SQLite/writer → final artifact. Production has one default UA definition in Core; Core defaults/missing/whitespace normalization and policy construction use its header-text helper, HTTP client fallback uses the same UA constant. Explicit case-insensitive request priority and dedicated Cookie/Referer handling remain unchanged.

Test `b4_default_headers_reach_actual_probe_and_transfer` binds127.0.0.1:0, captures actual HEAD/ranged probe/content GET, runs original SQLite engine and writer, verifies exact final content and exactly one exact UA on every captured request. It aborts/awaits its sole listener task and requires its request Arc to have no remaining owners. No external HTTP/native/GUI/product process is used.

- Genuine RED: `cargo test --locked --offline -p nxtools-download --test b4_request_policy_contract b4_default_headers_reach_actual_probe_and_transfer -- --exact --list`, followed by identical selection with `--exact --nocapture`. LIST1/RUN1/PASS0/FAIL1/ignored0. Actual header was Chrome120, expected the requested Trident value. Final output/content and real probe/transfer assertions had passed before the target UA assertion. Scratch `b4-headers-red-list.log`, `b4-headers-red.log`, `b4-headers-red-results.json`.
- GREEN: same exact LIST/RUN selection after the default/policy changes. LIST1/RUN1/PASS1/FAIL0/ignored0. Scratch `b4-headers-green-list.log`, `b4-headers-green.log`, `b4-headers-green-results.json`.

Actual custom/global/task/browser precedence, legacy effective defaults, immutable SQLite reopen lifecycle, redirect context, and production Settings interaction/transaction coverage are verified below.

## Production Settings

Regression `b4_headers_reset_edits_original_textarea_and_cancel_discards` opens the original Settings window, selects Request, performs actual Textarea input, finds an official semantic Reset Button, asserts original entity/focus/other-field/disk-byte retention, then Cancel/reopen. Before implementation LIST1/RUN1/PASS0/FAIL1/ignored0 failed at the missing official Reset Button after the actual edit assertion passed (`b4-settings-reset-red-list.log`, `b4-settings-reset-red.log`, `b4-settings-reset-red-results.json`).

The first post-Reset selection `b4_headers_` LIST3/RUN3/PASS1/FAIL2/ignored0 (`b4-settings-reset-green-merge-red2-list.log`, `.log`, `-results.json`) passed mouse Reset/Cancel. Actual Header-only Apply erased a concurrent Cookie (`""` instead of `"concurrent=1"`): genuine transaction RED. The keyboard case stopped before Button activation because the released multiline editor handles Tab as indentation; that failure is a focus prerequisite, not Button activation RED.

The repair carries the window-open baseline through the original draft, merges changed typed leaves under the original Core config lock/reread/atomic writer, and publishes the returned merged configuration. Nested overlay/background leaves are independent; categories retain their original collection semantics. Reset remains one deferred draft edit on the original TextareaState. The keyboard regression uses actual reverse Tab from the adjacent Cookie input and checks semantic Button focus before activation.

Post-merge `b4-settings-merge-green` LIST3/RUN3/PASS3/FAIL0/ignored0 passed mouse Reset/Cancel, first focused Enter/Space/repeat/Escape, and concurrent Cookie/overlay/background leaves in both disk and application configuration. Logs use that prefix with `-list.log`, `.log`, `-results.json`.

Expanded production selection `b4-settings-expanded-background-red` LIST7/RUN7/PASS5/FAIL2/ignored0 passed first-open/new/missing/whitespace/custom actual text with byte-identical disk, failed Apply retaining Reset draft/original entity/error notification/runtime policy, and the original three cases. Header-only Apply changed an untouched enabled1537B/s background limit to2048B/s: genuine precision RED. The narrow Apply case initially checked a cached application handle rather than actual GPUI window removal; the assertion now observes the real window list, then retained disk configuration and reopening.

The background input records original InputEvent::Change in the same view; an untouched value uses exact saved bytes, while edited KiB values retain checked parsing. Empty configured directories keep the original effective fallback.

Production selection `b4-settings-runtime-concurrency-red` LIST8/RUN8/PASS7/FAIL1/ignored0 passed the repaired background precision and narrow blank/Reset→Apply/reopen cases. Header-only Apply cleared a concurrent2049B/s global limit toNone: genuine engine transaction RED. The original draft now compares queue concurrency to its open-time baseline and carries the existing B2 SpeedLimitEdit dirty intent. Untouched runtime leaves do not resend stale values or participate in rollback. Changed leaves retain original command/save/rollback ordering; the original Snapshot supplies the pre-operation queue value.

## Verified focused selections

`b4-header-final-checks.ps1` serializes these selections through `b4-run-rust.ps1`. Each command is `cargo test --locked --offline -p PACKAGE TARGET FILTER -- --list`, then the identical selection with `-- --nocapture --test-threads=1`. Every row has LIST/RUN exit0, FAIL0 and ignored0. Each prefix has `-list.log`, `.log` and `-results.json` in the session scratch; wrapper summary is `b4-header-final-checks-results.json`.

| Package / target | Exact filter | LIST / RUN / PASS | Log prefix |
|---|---|---|---|
| nxtools-desktop / --bin nxtools-desktop | b4_headers_ | 8 / 8 / 8 | b4-header-final-settings |
| nxtools-download / --test b4_request_policy_contract | b4_ | 3 / 3 / 3 | b4-header-final-http |
| nxtools-core / --test b4_download_settings_contract | b4_ | 2 / 2 / 2 | b4-header-final-core |
| nxtools-desktop / --bin nxtools-desktop | ref06_ | 11 / 11 / 11 | b4-header-preserve-ref06 |
| nxtools-desktop / --bin nxtools-desktop | settings_persistence_failure_rolls_back_engine_values | 1 / 1 / 1 | b4-header-preserve-rollback |
| nxtools-desktop / --bin nxtools-desktop | download_settings_save_failure_uses_center_and_keeps_original_draft | 1 / 1 / 1 | b4-header-preserve-failure |
| nxtools-download / --test engine_contract | download_defaults_merge_at_acceptance_and_explicit_request_context_wins | 1 / 1 / 1 | b4-header-preserve-request-context |

Settings8 use actual original Root/Settings navigation, Textarea editing, semantic official Reset focus and first Enter/Space/repeat/Escape, pointer Reset, Cancel/Apply/reopen, stable input entity/focus,720×480 reachability, byte-identical read-only opening, effective blank semantics, corrupt-file failure/error notification/no false policy publication, exact1537B/s background value, and concurrent disk/runtime leaves. HTTP3 use actual local HEAD/ranged probes/content transfers, original SQLite/writer/final byte checks, one exact UA, global/task/original BrowserSubmit precedence, legacy whitespace, policy update and engine reopen with frozen request. Core2 supplement the same original load/locked writer, including nested merge and corrupt-file refusal. New supplemental coverage that passed first time is not claimed as behavioral RED.

The first expanded HTTP run passed default and precedence/snapshot cases; its redirect assertion conflated redirect hops with the separate direct final-range request. The verified original boundary is explicit: cross-origin redirect hops remove sensitive Cookie, while subsequent final-URL range requests use the accepted task context. Default UA and explicit Referer remain unchanged across both boundaries. Final HTTP3 pass with those actual owners distinguished; no Cookie/redirect implementation was changed.

## Limits

Owned temporary configuration/SQLite/output directories and ephemeral loopback listeners only. HTTP listener tasks are aborted/awaited and request-owner drain is asserted. Final B4 format/all-target check/source gates, related preservation and actual resource audit are recorded in outcomes.md. Desktop strict lint retains its two documented baseline findings. Physical Windows IME/DPI/OS-close, human visual, installed/Release and whole-workspace acceptance remain separate.
