# NxTools.Next surface inventory

## Status and authority
The original design-only inventory is the historical pre-migration baseline, not new feature authority. After remote checkpoint42f60a8, the user authorized local feature-first production migration and continuation of remaining surfaces. Entries below retain original observed budgets; superseding implementation status is recorded at the end. Existing functionality/native budgets remain authoritative; source/check completion is not installed or user visual acceptance.

[Design system](design-system.md) · [Page blueprints](page-blueprints.md) · [Acceptance](acceptance-matrix.md) · [Migration map](migration-map.md)

Paths below are relative to `rust/apps/nxtools-desktop/src/` unless explicitly qualified. States: N normal; L long/many; E empty/no match; P pending/loading; F failure; D disabled/busy; K keyboard/focus. Every interactive surface also requires hover/pressed/selected where applicable. A listed source is an entry, not proof that constructor/base/native lifecycle tracing is complete.

## Main workbench and settings
| ID | Existing surface / entry | Current budget or owner | Design responsibility / required states |
| --- | --- | --- | --- |
| SH-01 | `main_window/shell.rs` titlebar, module rail; `main.rs` creation | Default1060×650, minimum920×480;36px titlebar; current208px rail | Single-line named modules, stable utility/drag regions; N/L/D/K |
| SH-02 | `main_window/download_page.rs` task workspace | Main content; current46px rows, bottom176px detail | Shared header/data grid, filter/search/sort, multi-selection toolbar, on-demand detail; N/L/E/P/F/D/K |
| SH-03 | `main_window/quick_launch_page.rs` configuration | General, Index, Context, Commands, Diagnostics | Settings, not an invented search dashboard; navigation/reading hierarchy; N/L/P/F/D/K |
| SH-04 | `main_window/clipboard_page.rs` configuration | Main content; current two-column workspace | Trigger/actions, retention/privacy/storage hierarchy; preserve per-control persistence; N/L/P/F/D/K |
| SH-05 | `main_window/tools_page.rs` registered tools | Currently Link Detection; one keyed settings window per tool | Continuous entry with clear feature state; no invented feature cards; N/D/K |
| SH-06 | `main_window/notifications.rs`, `notifications.rs` | Max3 notifications; bounded336px surface, max112px | Bottom-right non-obstructing status/actions; N/L/P/F/D/K |
| GS-01 | `network_settings.rs`, `network_settings/blacklist.rs` |720×480; General/Network/Application blacklist; draft Apply/Cancel | Common settings grammar, independent rule scopes, correct saved/applied feedback; N/L/E/P/F/D/K |

## Download surfaces
| ID | Surface / entry | Current budget or owner | Design responsibility / required states |
| --- | --- | --- | --- |
| DL-01 | `downloads/new_download_view.rs`, `browser_handoff_impl.rs` | Ordinary560×176 probe → new560×278 confirmation | Keep accepted fixed-window handoff; URL/result/name/destination/category/schedule and footer fit; N/L/P/F/D/K |
| DL-02 | Same new view, media selection | Ordinary confirmation not a media budget; measured DASH353/HLS418/error456 heights recorded in prior audits/tests | Track selection and context have their own bounded composition; N/L/P/F/D/K |
| DL-03 | `downloads/task_window.rs`, `task_windows_impl.rs` | Progress/completed440×184; expanded440×280; failed440×196 | Ordinary content-only completion; actionable retry, progress/detail metrics, stable footer; N/L/P/F/D/K |
| DL-04 | `downloads/settings_view.rs`, `window_support.rs` |984×650; General/Request/Transfer/Window/Diagnostics | Continuous navigation/forms, categories list-editor, request context, FFmpeg/status and overlay options; N/L/P/F/D/K |
| DL-05 | `downloads/schedule_view.rs`, `browser_handoff_impl.rs` |430×326 | Start/stop/day plan; draft/cancel and valid scheduling, not generic form enlargement; N/F/D/K |
| DL-06 | `downloads/delete_confirmation.rs` | Content-defined budget; remeasure before migration | Explicit task/file consequences, one danger action; N/L/F/D/K |
| DL-07 | `downloads/shutdown_window.rs`, `completion_shutdown_impl.rs` |450×200 creation | Countdown/cancel without shifting primary action; N/P/D/K; reduced motion |
| DL-08 | `download_taskbar_overlay.rs` | User-configured Rust placement/size; own lane budget | Bounded task metrics and edit state; no new native owner; N/L/P/D/K where actionable |
| DL-09 | New-download category/date/media popovers, `ui/components/path_list.rs` | Trigger-anchored and content-bounded | Label/icon/action slots, selected/current/disabled dates, outside-dismiss/focus; N/L/E/P/F/D/K |

## Quick Find surfaces
| ID | Surface / entry | Current budget or owner | Design responsibility / required states |
| --- | --- | --- | --- |
| QF-01 | `quick_launch/panel.rs`, `quick_launch.rs` |620×410 results; collapsed620×44 (42px search+boundary) | Search-first; app/file/command/calculator/URL results; coherent name/path/shortcut slots; N/L/E/P/F/D/K |
| QF-02 | `quick_launch/actions.rs`, `panel.rs`, `ui/components/menu.rs` | In-window action mode, lazy native Shell submenus | One hierarchy, real focus/return; no second gesture framework; N/L/E/P/F/D/K |
| QF-03 | `quick_launch/preview.rs` |400×410; permanent passive noactivate | Type/title/content/metadata; never put action buttons in a passive preview; N/L/E/P/F |
| QF-04 | `quick_launch/file_dialog_search.rs` | Host-following Rust/native layout | Attached collapsed/expanded search and result/action modes; no invented host activation; N/L/E/P/F/D/K |
| QF-05 | `ui/components/custom_command_editor.rs`, `volume_scope.rs`, `path_list.rs`, `hotkey.rs`, `quick_launch_diagnostics.rs` | Main settings-contained editors | List-editor layout; scope normalization; recording/waiting/applied distinct; N/L/E/P/F/D/K |

## Clipboard and tools
| ID | Surface / entry | Current budget or owner | Design responsibility / required states |
| --- | --- | --- | --- |
| CB-01 | `ui/mod.rs`, `ui/history.rs`, `clipboard.rs` | Default300×440, user-resize policy retained; bounded history window | Category/search/history/footer; selected≠hover, meaningful copy/paste hints; N/L/E/P/F/D/K |
| CB-02 | `ui/mod.rs` category/editor/source controls | Panel-contained occluding editor | Input focus, Save/Cancel, source selection, no click-through; N/L/E/F/D/K |
| CB-03 | `ui/history.rs`, `ui/components/menu.rs` entry/source/context actions | Panel-local menu and selected item | Pin/delete/edit/format actions without moving row slots; N/L/P/F/D/K |
| CB-04 | `ui/preview.rs`, `clipboard.rs` | One retained passive preview, Rust content sizing/C# visibility | Text/image/files plus bounded metadata; lifetime/generation/position constraints; N/L/E/P/F |
| TL-01 | `main_window/tools_page.rs` Link Detection configuration | Current460×320 | Enable/placement/copy-only/auto-hide/browser route; existing setting semantics; N/P/F/D/K |
| TL-02 | `tools/link_panel.rs`, `link_detector.rs` | Width420; height derived from rows/disclosure/footer | Copy-first rows, clear expand and hide semantics; N/L/E/P/F/D/K |

## Common supporting/browser surfaces
| ID | Surface / entry | Current boundary | Design responsibility / required states |
| --- | --- | --- | --- |
| UI-01 | `ui/components/{dialog,popover,menu,tooltip,state}.rs` | Owner-specific sizes, viewport clamping | Semantic roles, label/icon/value slots, action groups, focus-visible and empty/error copy; complete component state board |
| UI-02 | `startup_failure.rs`; Windows tray/native picker | Native platform surfaces remain Windows-owned | Consistent product copy/icon only, no custom tray redraw; failure/retry where currently available |
| EX-01 | `browser_extension/nxtools-interceptor/popup.{html,css,js}` | Browser popup lifecycle/CSP; current views | Same hierarchy within browser constraints, busy/refresh feedback; N/L/E/P/F/D/K |
| EX-02 | Same extension `content-script.js`, `shadow-styles.js` | Page-owned shadow media/drag overlays | Resource selection and drag target, no user request-context disclosure; N/L/E/P/F/D/K |

## Evidence and exclusions
Source inventory verified by repository entries and constants; not every state was run natively. Current native screenshot subset and rejected installed screenshots establish local evidence, not blanket acceptance. Reference roots: [GPUI/Zed](../../reference-code/gpui.md), [Downloads](../../reference-code/download.md), [Quick Find](../../reference-code/quick-launch-research.md), [Clipboard](../../reference-code/clipboard.md), [System Shell](../../reference-code/system-shell.md).

No service scans/repair, new providers/tools, dashboards, theme chooser, unified mega-settings transaction, new persistence semantics or provider injection is implied. Omitted native measurements are explicit migration gates, never guessed values.

## Current local source status (supersedes historical layout measurements above)
The first slice migrated SH-01 to176px/40px named rail, SH-02 to48px task rows, and SH-04 to continuous grouped configuration. The remaining slice now implements DL-01–07 auxiliary presentation, QF-05 advanced command/scope/path/diagnostic layout, CB-02 compact editors, SH-06 bounded notification copy/actions, and EX-01–02 product HTML/CSS/shadow hierarchy. Existing date/media/popover/overlay/shared/native owners are preserved, not deleted or newly redesigned. All original controls remain authoritative. Desktop494/494 and strict checks pass; browser21 tests/38 DOM checks pass. Five owned100% auxiliary/notification captures have actual budget readback but are not newly eye-inspected. Full installed/DPI/state/U/provider acceptance remains open. [Remaining evidence](../../audits/2026-10-09-remaining-ui-layout-migration.md).
