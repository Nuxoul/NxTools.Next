# Onboarding and native interaction outcomes

## Delivered boundaries

| Requirement | Implementation and evidence |
| --- | --- |
| R1 first-build indexing | The existing serial worker publishes each completed, persisted and exactly reopened volume while later volumes build. Global Ready still waits for final completion. Bounded stage/count diagnostics distinguish first-search readiness from all-volume completion. [Indexer evidence](indexer-r1.md) |
| R2 settings scrolling | Theme previews, category form and extension tags have natural height under the existing SettingPage. Category selection uses the released official scrollable dropdown; its temporary popup owns only menu scrolling. The original drafts and Apply/Cancel owners remain. [UI evidence](ui-outcomes.md) |
| R3 theme identity | Light/Latte and Dark/Frappe each appear once. Saved aliases select the same effective preview without rewriting preferences on opening or cancelling. Follow System and other palettes remain available. |
| R4 Create Archive | Fresh recommendations include Create for ordinary files, directories and mixed selections through the existing grouped `--archive-compress` route. Integration remains disabled until explicit Apply. Missing-field legacy defaults, explicit empty lists and saved custom selections are not expanded. |
| R5 temporary path | Settings display and copy the effective absolute session path derived from the current draft, including default/custom policy. Blank config remains the default. Rendering does not probe/create/migrate/delete cache data. |
| R6 uninstall | Standard removal and explicit personal-data retention are separate from upgrade/repair. Exact-owned processes, registrations and signed protected components use existing ownership/deployment boundaries. Known application data is cleaned without sweeping arbitrary ProjectDirs/custom parents; unattributed payload preservation and remaining runtime data are recorded in [uninstall evidence](uninstall-outcomes.md). |
| R7 native selection | Initiating HWND/PID, authenticated peer ownership, unlimited human wait and retained bounded-STA admission cross the existing protocol. Owner retirement, cancellation and stale results are isolated. Browser export destination selection is separate from bounded package generation. [Picker evidence](native-picker-outcomes.md) |

The ordinary archive extraction-output pipeline, its codecs, journal, publication and cancellation semantics were not changed by this task. Shared Shell/default/label/test-registration and picker contract edits are scoped in [coordination](coordination.md).

## Verification

| Check | Recorded result |
| --- | --- |
| Protocol library tests | 112 passed |
| IPC/router library tests | 61 passed |
| Protocol/IPC strict Clippy | Passed |
| Indexer focused synthetic tests | 10 distinct tests passed, including blocked-later-volume publication and final Ready admission |
| Core fresh/legacy Shell recommendation tests | 2 passed |
| C# production Shell plan fixtures | 5 passed |
| C# safe uninstall/association/OPT07 picker group | 16 passed |
| Final C# Host/tests build | 0 warnings, 0 errors |
| Final protected-uninstall fixtures | 4 passed, including retry after service deletion and exact-owned missing updater root |
| Personal-data cleanup script fixtures | 9 passed, including unknown runtime output preservation and metadata-deletion failure propagation |
| Protected-uninstall source entry gate | Passed |
| FFmpeg manual picker draft isolation | 2 focused tests passed |
| Category follow-up | 5 focused tests passed |
| Final real-button/export, official category dropdown and saved custom Shell selection | 3 focused tests passed |
| Required document/component gates | spec-contracts, gpui-component-catalog, gpui-release-contracts and markdown-editor-contracts passed |
| UI terminology | 2 tests passed |
| Formatting and whitespace | Scoped Rustfmt and task-file diff checks passed |

The one complete Desktop run recorded **1765 passed, 15 failed, 1 ignored**. Its seven in-scope failures passed subsequent focused checks. The recorded run also contains seven archive tests requiring external C# wire exports and the unrelated `main_window::tests::download_context_menu_omits_actions_that_cannot_run_for_the_selection` failure. The suite was not rerun and is not reported as green.

All Cargo commands used one build job, disabled incremental compilation and `-C codegen-units=256`. Compilation still reports the three existing Desktop `private_interfaces` warnings. Workspace-wide strict Clippy/type-check and a clean full-suite result are not claimed.

## Acceptance limits and remaining work

- **First-build performance:** no authorized real NTFS baseline, before/after timing or Listary-native comparison exists. Per-record native timestamp hydration remains unchanged. Selecting and implementing a raw-MFT replacement still requires measured attribution and correctness fixtures. The inspected Listary reference exposes managed boundaries, not its native engine source.
- **Installed indexer:** packaged LocalSystem restart/base-plus-delta restoration, idle CPU/I/O regression and real service search are NOT RUN. The approved authenticated Desktop admission must remain active during these probes.
- **Native interaction:** actual dialogs, foreground/focus, physical keyboard/Escape, unrelated application interaction and supported DPI are NOT RUN. Controlled GPUI and self-owned/injected C# fixtures do not establish these results.
- **Shell and uninstall:** real Explorer Apply/Revert/creation and disposable-install removal/retain/service/process/output-preservation acceptance are NOT RUN. Unknown runtime payloads may remain when deleting them would risk user outputs.
- **External archive evidence:** the three AC repair tests require their C# production progress-wire export; four OPT03 tests require effects-audited output and `NXTOOLS_OPT03_CREATE_WIRE`. No export was fabricated and no native authorization was assumed.

No packaging, installation, UAC, real registry/SCM mutation, service restart/removal, real-volume scan, product/native picker launch or current-user-data deletion was performed. Unrelated worktree changes remain intact. No commit or task archival was made. The task remains `in_progress` for the outstanding performance and operational acceptance.
