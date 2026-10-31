# Browser details / marquee / preview layout correction

## Current implementation status

Current continuation completes the earlier partial source: Tree/list24DIP PNG/filledfallback and
optional readonly preview are implemented, with original selection/materialization/lease owners.
The prior three-failed-edit stop was honored in its turn; the current turn re-grounded fresh files.
See [completion outcomes](browser-completion-outcomes.md). Prior partial report/history remains;
compiled/isolated evidence is not native/physical/user visual acceptance.

2026-10-27 implementation boundary within the existing archive task and canonical phases.
The new user request supersedes only the old name-list default and optional-preview scope;
all historical outcomes and the installation-code1 OPEN status remain unchanged.

## Source and component review before implementation

Read the current component-first rules, Kit0.8.0 delta, full component table/index,
Desktop owner specs, archive PRD/design/implement/ui-spec, Phase C contract and both
render/name-view outcomes. Current Kit/Base/Component/Assets0.8.0/pre0.3.9 unchanged.
The original reference note found no legacy general archive browser. Bandizip public
resources name packed/original/type/modified columns and an image preview but do not
prove its proprietary native lifecycle. Existing Next is the source for selection,
virtualization, native row drag, worker, passwords, source and lease compatibility.

Exact released files inspected: Component0.8.0 list/list_item.rs ListItem::render keeps
its public selected/secondary_selected/children/callback slots; tree.rs Tree::render
adapts original Base Tree state and styled ListItems; resizable.rs h_resizable wraps
Base group with official resize-handle appearance. Original browser uses these public
interfaces. No new TableState/ListState/selection controller is introduced.

| Surface | Decision / visible form | Parent adaptation |
| --- | --- | --- |
| Right data pane | Retain approved ListItem + public uniform_list exception,32DIP rows; default five detail columns in requested Name/Packed/Original/Type/Modified order | Original model owns immutable selection/focus/anchor; widths keep semantic indexes; shared header/row tracks and horizontal scroll |
| Tree/list folder icon | Shared Shell type PNG at24DIP in32DIP rows; official filled folder fallback, readable pending/failure tooltip | Original TypeIcons bounded cache/request owner only; no member paths/materialization |
| Rectangle selection | Existing body layout hosts transient gesture and theme-derived selection quad, not an ordinary control replacement | Starts only empty body, row down stops propagation; official header/splitter/scrollbar owners untouched |
| Preview split | Existing h_resizable/resizable_panel, optional right pane default320DIP/range200..600 | Official handle owns resize; list min160 and horizontal scroll preserve columns at720DIP |
| Preview controls/status | Actual Small outlined Button outside Toolbar.child; Empty/Alert/Spinner | No ghost adaptation; source/close use one official callback; unified passive MarkdownDocument and owned decoded image |

## Gesture invariants

One BrowserModel remains the selection/focus/anchor owner. Gesture stores only the
press baseline and content origin. Normal rectangle replaces, Ctrl XORs against the
fixed baseline (leaving a rectangle restores baseline, not repeated toggles). Parent
navigation row is excluded. Content coordinates include vertical/horizontal scroll;
all projected rows can be selected without mounting all rows. Empty-body start only:
file-row down retains original still-pressed>=4DIP native OLE callback/sourceHWND.
Header sort/column grips, scrollbar and tree splitter are not in body admission.
Escape/focus loss restores baseline and ends current gesture. Folder/filter/sort,
refresh/snapshot/generation/source/close retire without writing old IDs into new data.
Selection quad is passive, clipped to the actual body viewport, no timer or pointer logs.

## Preview invariants

Scope confirmed: images, text, code and Markdown, read-only. Single selection shares
one latest-generation path for keyboard/pointer/marquee. Double-click still opens via
original consent/Open. Preview never Shell-opens or executes content. IO/decode are
background, actual bytes and decode allocation are bounded independently of512MiB open.
Original materialize/password/cache/source/lease lifecycle remains authoritative;
stale/close/refresh/disconnect/source-change cancel and release. Markdown uses the
approved unified passive renderer, placeholders for images, no network/file resources.
Cross-owner callbacks are next-turn, including cached/immediate results.

## Verification levels

Real synthetic GPUI down/move/up, Ctrl, scroll, cancellation and row-drag exclusion
are required, not geometry-helper-only proof. Minimum/default Light/Dark long-name
production bounds are automated evidence only. No product launch, physical input,
current HWND screenshot, mixed DPI/IME/UIA or user visual acceptance is authorized
or implied. No package/install/service/Registry/COM/Explorer/commit/push.
