# Link detector panel reference index

Verified 2026-10-09 for Clipboard + Tools polish. No prior matching note existed; the old NxTools tree has no link detector popup or equivalent three-position setting (see [clipboard index](clipboard.md#link-detector-floating-panel)). Do not attribute Next's link detector behavior to the legacy product.

## Source chain and invariants

- `D:\Code\NxTools\Views\ClipboardHistoryPopupWindow.axaml.cs:61-167,206-259,749-905` constructs a reusable popup, restores search focus, cancels superseded hover preview, checks visibility and hovered item before showing, and hides/reuses a single preview window. `D:\Code\NxTools\Views\ClipboardPreviewWindow.cs:29-95,200-237` constructs the passive topmost/no-activate preview and places it within the screen. This is a lifecycle precedent, **not** a legacy link panel. The old `MainWindow.axaml.cs:971-1067` distinguishes copy from paste/hide-before-focus; its action policy does not override Next's link route.
- Zed `D:\Code\zed-main\crates\theme\src\default_colors.rs:205-225` has separate neutral surface/hover/active/selected and blue focus border; `crates\acp_tools\src\acp_tools.rs:708,738-752` uses tracked focus and click. These are presentation/GPUI interaction references, not link detection behavior. Next's `rust/apps/nxtools-desktop/src/ui/components/action.rs:12-69` generates one activation on GPUI KeyUp via `on_click`; avoid a duplicate key handler.
- Next entry: `rust/apps/nxtools-desktop/src/clipboard.rs:869-1088` detects and routes captures, opens one popup or restages its old window, applies selected display bounds/size and topmost through native commands; `rust/apps/nxtools-desktop/src/tools/link_detector.rs` owns typed extraction; `tools/link_panel.rs:28-103,118-334` owns copy-only-first typed routes, collapsed/expanded exact height, rendered rows, deferred cross-owner dispatch, and a window timer. `main_window/tools_page.rs:41-176,179-465` owns the Tools row and per-tool settings window/draft.
- Preserve default copy-only, opt-in browser route, local file/directory shell distinction, generation-scoped hide policy, topmost/alpha/native position contracts and one popup. Native show/hide and focus are not owned by the GPUI row. Row click closes its window **before** deferring copy/route to the owning application. Settings callbacks modify local draft first and defer application updates; dropdown selection is already doubly deferred. Preserve a single visual pointer/keyboard dispatch, bounded text and exact fixed panel height. No fabricated metadata or newly claimed provider support.

## Keyboard follow-up verification (2026-10-09)

- Rechecked old `D:\Code\NxTools\Views\ClipboardHistoryPopupWindow.axaml.cs:61-167,845-905` and `ClipboardPreviewWindow.cs:29-95,200-237`: the constructor's focus/show boundary is popup-owned, preview remains hidden/reused/no-activate; the old tree still contains no link popup. In Next, `tools/link_panel.rs:189-321` has a keyed focus handle on each link row and a Button for the expander; KeyUp-generated clicks share the exact pointer route, close before the deferred clipboard dispatch, and expander resizes only the owning window. `main_window/tools_page.rs:41-465` contains the only registered Tools workspace feature (Link Detection) and all its four settings. Polish these actual surfaces rather than inventing additional tools. Do not attach a focus stop to passive popup chrome or change the native topmost/window timer/alpha policy in `clipboard.rs:934-1088`.

## Queue08 component prerequisite
Reverified Next ClipboardApplication::show_link_panel -> original reused view.swap_links/
collapsed420px-derived bounds -> C# SetWindowPosition/SetWindowBounds; fresh popup options
-> topmost/shadow commands -> oneRoot/LinkPanelView and original window timer. Old popup/
PreviewWindow constructor/show-hide/passive native source is lifecycle precedent only;
no legacy Link panel equivalent. Link row original remove_window precedes cx.defer,
note_panel_copied/forget_link_panel/dispatch_link_panel_route/write_to_clipboard.
Preserve copy-only-first/Web opt-in/local shell distinction, row28/expander24/footer22,
60single/84collapsed3/116expanded3 heights and timer/controller unchanged. Candidate
releasedListItem publicchildren/no action controller, StatusBar left/right public slots,
Label literal text and styledSmall expander; keep original focus/one click owner.
Tools workspace same registeredLinkDetection/keyedsettings owner; completed460x320
Form/NumberInput/Select/Switch settings excluded from this slice. No payload diagnostics,
business startup/native action/real Clipboard access authorized.

Queue08 result: actualstatelessListItem/Label rows,Smallstyledexpander,StatusBar left/
right slots/literalLabels retain original420px/28row/24expander/22footer/derived60/84/
116px. Originalfocus/Tab/oneclick boundary and close-before-deferredroute/copy callback
whitespace-normalizedequalbaseline;route/height/swap/timer functions unchanged.
Toolsworkspace actualListItem/Label/SmallsemanticTag with originalfocused_action sole
dispatcher and Configuretabstopfalse;completedsettingsFormNumberInputSelect tail exact
baselineequal. Productionpointer+focusedKeyDownUp TestPlatformcopy/windowcount/timer,
longcollapsedexpandedfooter/Toolskeyedreuse/minimum/centers pass;full598/598/strictchecks/
8sourcegates0. No actualWindowsClipboard/browser/Shell/nativebusiness action/PNG/buildstart/
Release/service/realuserdata. No new08decision;finalvisual/nativeacceptance12pending.

## Approved action-policy replacement (2026-10-12)
The later installed-feedback task R6 explicitly supersedes historical copy-only-first/Web-opt-in requirements in this note. Current `ClipboardApplication::show_link_panel` only identifies/presents; row-open maps Web→C# Shell Open URL, LocalFile→SelectInExplorer, LocalDirectory→Open, without Clipboard writes. Released XSmall Copy button stops propagation, closes then defers echo mark/copy through the original owner. Legacy copy_only/browser config fields remain readable but do not gate routing. Keep one keyed panel,420px/28pxrows/24pxexpander/22pxfooter, generation-scoped restaging and6second default hide. No legacy Link equivalent is newly claimed.
Verified source/native references and exact component APIs: [link outcomes](../../.trellis/tasks/10-12-installed-feedback-capabilities/research/link-outcomes.md). Current production regressions execute route/Copy/timeout/identity under synthetic platform; real Windows Clipboard/browser/Shell and visual approval remain separate.

## Approved round2 component correction prerequisite (2026-10-13)
User reports popup still looks custom; approval permits revisiting rendered components, not replacing native controller. Rechecked original Next show_link_panel -> hidden Kit Root/LinkPanelView -> native noactivate/bounds/show and one generation/timer; no old Link equivalent. Reviewed full official index, gpui-component0.7.1 ListState/ListDelegate internal query/selection/scroll, stateless ListItem::render public children/suffix/on_click/InteractiveElement, Alert fixed message (no arbitrary action/list slot), StatusBar public left/right and default py1/px2/text_xs. Full List would introduce another search/selection owner for a bounded capture action list; Alert does not own those actions, internal Popover/Dialog are not independent HWND replacements. Retain existing bounded list controller, adopt unwrapped actual ListItem as focus/click surface, retain public Button/Label and StatusBar default grammar. Remove outer 2px custom focus box/inner row shrink/padding override; use public keyed track_focus and existing single click boundary. Preserve row28/copyXSmall20/expanderSmall24, derive footer from actual official text/padding budget rather than compressing22px. Exact red actual bounds and pointer/keyboard/copy/expand/timer regressions govern compatibility, not imports.

## Installed tool-card action correction (2026-10-14)

User explicitly supersedes the earlier whole-card-settings behavior. Card body pointer,
Enter and Space toggle the registered Link Detection feature; Configure is an independent
styled Button/focus stop that opens/reuses the same keyed settings window and stops
propagation so it cannot toggle. `main_window/tools_page.rs::render_tools` schedules the
cross-entity setter for the next app turn and reads the latest enabled value then; the
existing ClipboardApplication::set_link_detector_enabled retains section persistence/error
ownership. No legacy Link tool equivalent or new native lifecycle is claimed.
Real production red reproduced body-click opening settings without enabling; pointer,
keyboard and temporary durable-save/settings compatibility regression then passed.
Evidence: .trellis/tasks/10-14-installed-feedback-actions/ui-outcomes.md.
