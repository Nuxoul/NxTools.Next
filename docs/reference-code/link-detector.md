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

## Source-aware extraction and background detection

`ClipboardApplication::handle_link_detector_capture` submits detection on the existing
clipboard owner. `clipboard/link_detection.rs` owns one running background detection
and one latest queued capture; `tools/link_detector.rs::detect_links` owns source-aware
extraction, target validation and deduplicated filesystem checks. Completion must match
the current capture generation, pause generation and enabled state before showing a panel.
New content, rejected captures, pause, blacklist updates and tool toggles retire pending
detection results; duplicate delivery retains the same accepted request.
An already-blocked filesystem query is not forcibly interrupted; it never runs on GPUI
and cannot cause replacement worker growth. Accepted/start/terminal diagnostics carry
one operation ID and bounded counts/elapsed/reason, never text, URLs or paths.

Candidate boundaries depend on their source: standalone HTTP(S), explicit quotes/angle
brackets, inline Markdown destinations, ordinary prose and complete local-path lines.
Standalone and explicitly wrapped HTTP(S) targets preserve punctuation inside the target;
prose excludes sentence separators and unmatched enclosing brackets. Markdown label text
is not a target. Internal apostrophes in ordinary names do not terminate single quotes.
Bare domains retain the known-TLD policy and support ports, queries and fragments.
Explicit HTTP(S) continues to use the existing `url` dependency for host/port validation.

A drive/UNC-leading unquoted line is one complete path, including spaces; a missing path
does not fall back to an existing prefix. Multiple paths use separate lines or explicit
quotes. Each distinct candidate is validated once, including negative path checks. Only
the first 64 lines, a bounded candidate set and first 8 distinct matches are considered;
the existing native clipboard byte limit remains authoritative. No network URL requests
or payload diagnostics are introduced.

The original one-panel reuse, focus, placement, timer, close-before-deferred-dispatch,
explicit Copy and echo suppression remain. `link_panel_shell_command` still maps Web to
HTTP(S) Shell Open, LocalFile to SelectInExplorer and LocalDirectory to Open. There is no
legacy Link equivalent. Source/worker boundary tests do not certify native or visual use.

## Compact link-row actions

The current `LinkPanelView::render` uses the original official ListItem as the ordinary
pointer/keyboard action surface, with direct XSmall ghost Copy and optional Download
Buttons. No separate Open button is needed. `ClipboardApplication::dispatch_link_panel_route`
retains the current ordinary-action settings: copy-only selects Copy, and Web without
automatic browser opening also selects Copy. An eligible open still maps Web to Shell
Open, LocalFile to SelectInExplorer and LocalDirectory to Open.

The unchanged chain is capture -> one keyed panel -> row/Copy/Download -> remove the
owning window -> deferred current-window identity check -> original Clipboard or Shell
owner. Preserve 420px width, 28px rows, expansion, passive native placement, timer
retirement and Copy echo suppression. Exact Kit0.8.0 ListItem public children/on_click
and Button XSmall ghost retain their own dispatch; no Toolbar parent adaptation or
replacement controller is introduced. Production regressions cover absence of duplicate
Open, ordinary routes, independent Copy and Download, and close-before-dispatch.

## Environment-variable local paths

`tools/link_detector.rs::detect_links_with_path_kind` owns whole-line and wrapped local
candidate boundaries; `is_candidate`, `scan_prose`, `closing_wrapper` and `classify`
share local-path recognition. `DetectedLink.display` feeds the existing Label, while
`copy_value` feeds explicit Copy and deferred typed Shell routing in `tools/link_panel.rs`.
There is no legacy Link detector equivalent.

Percent-delimited environment-variable roots, such as
`%LOCALAPPDATA%\NxTools\NxTools.Next\logs`, belong to the local-path branch. Expand
variables once from the Desktop process environment before metadata checks, require
an absolute drive/UNC result and retain the existing file/directory distinction.
Show the original target and use the resolved path for Copy/Open. Whole paths retain
spaces and never fall back to a shorter existing prefix; undefined variables and
missing targets produce no match. HTTP(S) percent escapes remain URL data.
Preserve the existing background detection, budgets, generation retirement, diagnostics,
one-panel identity, passive placement, focus, timer and deferred action boundaries.
