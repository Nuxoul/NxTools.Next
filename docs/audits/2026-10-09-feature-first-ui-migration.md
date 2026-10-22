# First feature-first local UI migration slice

## Authorization, baseline and scope
After pushing checkpoint `42f60a8ee9e78402461d5c42f5b55ca5cc254c18` and tag `checkpoint/pre-ui-layout-20261009`, user authorized local product UI changes and explicitly required original functions to govern UI, not functions reconstructed from the simplified prototype. This record covers a bounded first implementation slice, not whole-app visual completion. [Feature preservation ledger](../ui/layout-quality/feature-preservation.md) maps original functions; [migration map](../ui/layout-quality/migration-map.md) preserves canonical stage order.

No palette rewrite, vendor/dependency upgrade, C# or protocol change. No package/install, normal product startup, real Clipboard capture/history/config enumeration, user-URL request, real-volume scan, DOpus navigation, service operation, commit or push. Product/controller files untouched where stated below; no rollback/reset of user work.

## Changed presentation
- Foundation/main shell: 176px named module rail,40px single-line entries and opt-in13/20 workbench text. Four original module labels, callbacks, focus/keyboard paths and global settings/titlebar controls retained. Module descriptions remain available as tooltips. Window default1060x650/min920x480 unchanged.
- Added explicit workbench body13/20 and supporting12/18 roles; original compact roles remain. New workbench settings groups/rows are non-shrinking, readable and scrollable, separate from compact dialogs.
- Downloads: main row46->48; name13/20 and path/failure12/18. Downloaded/total,speed,remaining,status,created/category/icon retained. Six filter/counts, all sorting/search/clear, Windows selection/context/queue/batch, double-click preference and five detail tabs retained. Download settings opt into local workbench text; all General/Request/Transfer/Window/Diagnostics fields, categories/request/FFmpeg/overlay controls and save policy remain. New-download/task-window owners and budgets unchanged.
- Quick Find: five-topic settings, hotkey/native state, scope/path/volume/context/command/diagnostics retained. Flexible copy/stable control rail and readable typography; no fake search dashboard. Popup620x44/410,42px search/40px result/26px icon/48px trailing and400x410 passive preview remain. Compact leaf subtitles/path alignment improved; query/actions/native ownership unchanged.
- Clipboard: existing settings arranged into non-shrinking continuous scrollable calling/action,retention/privacy,archive/storage groups. Immediate/deferred setting policy, every modifier/limit/hotkey/import/export/backup/compact/migrate/busy guard retained. History source and metadata/management slots stabilized; actions and context menus unchanged. Compact default300x440/user resize, one passive preview and controller remain.
- Tools: existing Link Detection only, readable entry/status/action slots. Compact460x320 tool settings and420px derived link-panel dimensions/routes/timers retained. Link name/icon alignment and complete target tooltip improved.
- Global settings: opt-in workbench typography and flat groups only. All general/network/per-application rules, Apply/Cancel, single atomic persistence/deferred consumer update and saved-vs-Windows-applied behavior retained.
- Removed unused old split-surface, hotkey-row and compact-group visual wrappers after consumers moved. HotkeyField itself/state/controller retained; compact-setting helper needed only by test fixtures is `cfg(test)`, not warning suppression. No user operation removed.

## References and feature omission check
Before migrated presentation edits, read matching notes and rechecked ABDM DownloadList before old NxTools task list/detail; legacy MainWindow constructor/Open/Closed/Closing/tray and SettingsWindow constructor/animation/Open/Apply/Cancel/Close; Clipboard popup/search/hide/preview and passive style; Listary managed search boundary/SwiftList input/preview; Zed ListItem slots/focus. Missing Avalonia native base and Listary native engine source stay explicit. Rust owns state/layout,GPUI rendering,C# Windows APIs; no new native size/focus/show owner or synchronous cross-owner work.

Static comparison against42f60a8:13 changed view files retain original business-owner call names and settings-member references;9 critical files unchanged (new_download_view,browser_handoff_impl,task_windows_impl,task_window,downloads root,Clipboard root,Quick root,Core,Protocol). No changed C#/Rust core crates/browser extension layer. This is omission screening, not a semantic guarantee. Existing functional tests remain; intentional visual geometry expectations updated with production bounds tests.

## Causal verification and failures
1. Foundation actual geometry ran red before fix:208px rail !=176. Geometry/keyboard/tool-reuse tests green after correction. Module purposes moved to tooltips, not deleted.
2. Other delegates added production red-capable tests before edits but could not execute Cargo; do not claim their pre-fix red was run. Parent compiled after exclusive writers completed.
3. Integrated compile found GPUI Div tooltip missing stateful ID; added stable ID. This compile error is not behavior-red evidence. Download input-chain punctuation fixed before compile. A log filename containing `::` was corrected without mislabeling product failure.
4. First integrated suite486 passed/2 failed: new Clipboard scroll expectation rendered in default1920x1080 because `window.resize` does not simulate WM_SIZE viewport; test corrected with actual1060x500 resize event. New non-shrinking content/group/row composition preserves full settings height. Quick minimum test also strengthened with explicit920x480 viewport and passed.
5. History pointer management action passed, keyboard follow-up failed because existing Button intentionally prevents mouse-down focus. Test traverses actual row/edit/pin/favorite/delete focus order then Space/KeyUp; one management dispatch and no extra paste asserted. No second activation handler/vendor patch.
6. Strict production lint exposed abandoned visual wrappers; removed only unused wrappers and restricted the legacy test-only helper to tests. No broad dead-code suppression.
7. Static gates synchronized intentional workbench wrappers/type/continuous layout while retaining hotkey native lifecycle, compact result rails, every Clipboard group and all downstream history/preview/cross-entity checks. No footer/content containment assertion removed to obtain green.

## Final checks performed
- Desktop488/488,0 failed/ignored, final serial closeout run after the explicit minimum-view test.
- Production and tests Clippy `-D warnings`, workspace `--tests`,fmt passed. Debug desktop build passed; only a test viewport assertion changed after the build, not production render.
- Eleven source gates: ui-token-boundaries,ui-consistency,ui-state-lifecycle,desktop-shell-lifecycle,download-ui-lifecycle,quick-launch-lifecycle,clipboard-ui-lifecycle,file-dialog-search-visual,file-dialog-search-lifecycle,file-dialog-window-follow,ui-preview-isolation.
- Spec checker87 documents/12 owners and diff checks passed before record update; final document/link/diff checks run at closeout.
- All Cargo serialized with jobs1,incremental0,RUSTFLAGS=-C codegen-units=256,root manifest; no build timeout/duplicate process.
- Static preservation report in session scratch `feature-preservation-check.json`; key logs `feature-first-foundation-red.log`,`feature-first-integrated-desktop.log`,`feature-first-final-*.log`,`feature-first-closeout-*.log` preserve failures and exact outcomes.

## Owned native evidence
Explicit Debug preview intercepted before normal runtime, synthetic in-memory/invalid business owners, owned HWND/PID capture only. Actual100% images under `artifacts/ui-layout-quality/native-foundation/` with hashes in `evidence.json`:1060x650 main,920x480 minimum,Quick settings and Clipboard settings before/after scroll,720x480 global rules and560x278 direct confirmation. Parent inspected shell,Quick settings,Clipboard calling/storage screenshots through browser tool; other captures not blanket visually approved.

Owned native pointer selected Downloads,Tab/Enter selected Quick Find,pointer selected Clipboard. Initial wheel attempt did not change image: fixed script WM_MOUSEWHEEL screen coordinates plus GPUI client pointer; final before/after hashes differ and actual scrolled image shows Export/Import/Backup/Compact/Migrate. No archive or migration action invoked. Preview exited; final process check found no remaining owned preview/Cargo/rustc. Indexer remains Stopped; no service operation.

Direct confirmation screenshot is not integrated probe handoff evidence. HWND-less GPUI tests and captures do not certify physical gestures,mixed DPI,native preview noactivate/OLE,or screen-reader provider.

## Remaining work and acceptance
This slice improves actual local UI but does not complete every page/state. Full native/download-settings/commands/index/history-preview/attached-search/supporting popup state screenshots and100/125/150%/mixed-DPI installed acceptance remain. Browser extension is unchanged. Current screenshot subset is not full feature exercise; preserve diagnostic/native/worker gates during further slices. Pinned GPUI native reader-provider limitation remains out of scope.

The19:30 Validation installer has not been rebuilt and does not contain this migration. Remote42f60a8 remains the rollback baseline; new local changes are uncommitted. Do not state all UI completed/accepted or offer a new installer as if created. Next work follows exact canonical owning phase plan, not automatic phase renumbering.
