# Six-item UI consistency repair — source outcome (2026-10-28)

## Scope and delivered behavior
User-approved six repairs, on the original Rust/GPUI/C# ownership boundaries:
1. Quick Launch commits nonempty current local results while indexed search remains pending. Empty pending retains protected rows; final empty clears. Original65ms coalescing, IME/generations and stale-result execution guard remain.
2. Attached Quick Switch source row is constrained to the root width; original36DIP rails now include the missing2DIP root-border height in requested bounds. No C# placement/focus change.
3. Download rows show filename plus known size or `大小未知`, not save path/URL. Actual bitmap40DIP, vector fallback30DIP, Shell source48px; original48DIP row/physical selection edge retained. Full failures remain available in status/tooltip/details.
4. Single click selects without detail IO. Right-click `查看详情` opens the original current DownloadDetailScope/inspector. Close is presentation-only; changed selection/navigation/filter/window retirement retains original cancellation/late-reply protection. Ctrl/Shift and configurable double-click/task windows unchanged.
5. Explicit new-download probe/submit and download-settings Save validation use existing NotificationCenter, preserving draft and zero invalid-input effects, without duplicate inline banner or validation-driven growth. Persistent native-layout/security/destructive Alerts remain.
6. Shared official Switch is Medium36x20 in a matching control lane. Routine proxy/disabled-extension/currency/storage/browser copy shortened; trust/restore/clear/privacy warnings retained. Disabled catalog hints are omitted, not replaced with empty rows. Official root-level ListItem::suffix anchors existing action Buttons to content-derived equal-row cells; no fixed252DIP tile height or vendor modification.

## Reference and regression evidence
- Download: note -> ABDM DownloadList/TableDownloadItem first -> legacy NxTools MainWindow selection/context boundary. Quick Launch: note -> Listary managed SearchMode/SearchWindowBase; proprietary native engine unavailable. Exact released Component0.8.0/public sources inspected.
- Pending publication red1FAIL -> green2PASS, including real panel/fake queued indexer.
- Attached source-row red: root288x72, row x=-56.5,width401,bottom73 -> whole-row/root containment green1PASS. Native client/DPI acceptance is separate.
- Core behavior real-red2PASS/3FAIL and bitmap20vs40red used an exact recorded earlier test binary only as red evidence, never latest-source green.
- Download aggregate initially311PASS/6FAIL on superseded inline/Small/icon expectations; updated original assertions to shared notice/no growth/Medium/40px while retaining keyboard/generation/media/footer invariants.
- Settings aggregate initially137PASS/1FAIL: removing disabled hint exposed15.5DIP action-center drift through ListItem's centered internal wrappers. Exact public suffix/root layout fixed it; alignment and natural-height tests unchanged.

## Executed verification
All Rust operations serial at natural idle, jobs1/incremental0/codegenunits256; durable scratch logs use `ux-repair-` prefix. Counts overlap; do not sum.

| Check | Actual result / source scope |
| --- | --- |
| Core `ux_repair_`, green5 | 5PASS/0FAIL/0ignored,0.50s |
| Bitmap/physical-selection geometry, green5 | 1PASS/0FAIL,0.30s |
| Quick Launch aggregate | 163PASS/0FAIL; relevant source unchanged afterward |
| Download-name aggregate, green5 | 317PASS/0FAIL/0ignored,218.57s; before independent catalog repair and additional detail-close assertion |
| Settings-name aggregate, green6 | 138PASS/0FAIL/0ignored,172.06s; includes catalog alignment and short-content tests |
| Latest core/detail-close regression, green7 | 5PASS/0FAIL/0ignored,0.59s; latest source compiled4m12s, actual close/scope/selection/task-state assertions passed |
| `cargo check --workspace --tests`, green6 | PASS,1m10s |
| Strict Desktop bin/tests Clippy, green6 | FAIL exit101 on two non-task-owned diagnostics below |
| Browser popup contract | PASS Node1test/all17 contract rows |
| Specs/catalog/release/editor | PASS114documents/14owners;79historicalpages/78entries;exact two framework deltas;57editorfiles |
| Download/UI-state/UI-token/Desktop-shell source gates | PASS independently |
| Quick Launch lifecycle source gate | FAIL QL11line458, old custom-command-editor token contract |
| Owned Rust format/scoped diff/task contexts | PASS;4implement+2check entries |

Final scratch source hashes and per-command JSON/logs retain stage identity. A temporary summary-script GBK output error was fixed in scratch and complete static checks rerun; it was not a product failure.

## Remaining verification blockers
Strict Clippy is not green:
- `rust/apps/nxtools-desktop/src/main_window.rs:75` unused `SPACING_12` import.
- `rust/apps/nxtools-desktop/src/tools/floating_assistant/media.rs:62` unread `animated` field.
Those owners are concurrent/non-task scope and were not changed or suppressed. QL11source gate requires `CONTROL_HEIGHT_INPUT` in the unrelated custom-command editor; the original strict gate was not relaxed. No claim of full-worktree lint/static acceptance.

## Acceptance and authorization limits
Only owned TestApp/fakeIPC/in-memory/temporary-file/source checks. No package/install/product launch, retained live-window operation, service/Registry change, real user configuration/history/Clipboard/credentials/network, commit/push/archive or delegation. Actual HWND/client clipping, physical pointer/keyboard/IME,125/150/mixedDPI, screenshots/user visual acceptance, reader/provider and end-to-end QuickLaunch latency remain unperformed. Headless bounds/tests do not certify these levels. P4HTTP and formal packaging remain paused; no later capability phase started.

## Final disposition
Source repair completed with affected regressions green and workspace test compilation passing.
Task status is `source_completed_verification_limited`, not full verification/native/user acceptance.
Final green7 ended2026-10-28T09:44:06Z. Owned production/test SHA256 snapshot remained unchanged at
17:44+08 readback. No temporary product probes, broad lint suppression, dependency/vendor edits,
unrelated owner changes or duplicate builds were introduced by this repair. Existing dirty peer
changes, including catalog open-settings behavior outside this slice, were preserved.

## Later user-requested hotkey reset addition
The subsequent approved Clipboard/QuickLaunch changed-hotkey reset addition is recorded separately
in [hotkey-reset-outcomes.md](hotkey-reset-outcomes.md). Latest hotkey9PASS, sharedgeometry1PASS and
workspace--testsPASS supersede earlier-source coverage only for that bounded addition; strictlint
and QL11limits remain. Original six-item counts above are historical stage evidence, not a rerun.
