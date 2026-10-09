# Page blueprints and reading order

[Inventory](surface-inventory.md) · [Design system](design-system.md) · [Acceptance](acceptance-matrix.md)

## Status
Design proposal, not production implementation. All examples use synthetic content. One browser representative cannot certify an entire surface family. Budget numbers marked current derive from source; candidate rails/spacing/type require visual approval. Drawing omits state-specific content only for legibility, not permission to remove existing features.

## Shared workbench (SH-01)
```text
+---------------------------------------------------------------+
| NxTools                                      settings   _ □ × | 36
+----------+----------------------------------------------------+
| Downloads| Page title / necessary live state    Primary action| 48
| QuickFind+----------------------------------------------------+
| Clipboard| Page-specific toolbar/navigation/content            |
| Tools    |                                                    |
|          |                                                    |
+----------+----------------------------------------------------+
```
Proposed176px named module rail instead of current208px two-line description blocks. Essential module identity is in the page header; redundant status slogans removed. Existing global-settings entry stays discoverable. Native drag/control regions are unchanged in function. Default1060×650 and min920×480: pages share content origin, not an arbitrary different top band. No icon-only navigation or newly invented application status bar.

## Downloads workbench (SH-02)
```text
| Downloads       3 active                         + New task    |
| All / Active / Waiting / Paused / Complete / Failed  Search Sort|
| Name / path         Progress        Speed     State  Actions   |
| design-assets.zip   248MB / 420MB    8.4MB/s   Active pause ... |
| Windows image ...   1.8GB / 6GB     12MB/s    Active pause ... |
|---------------------------------------------------------------|
| Selected task: Overview / Segments / Headers / Check / Timeline|
| destination / plan / error / metadata                        × |
```
Task names remain dominant. One fixed grid defines header/value/actions; progress values don't change column widths. Filter counts remain within their segment; sort beside filter/search, not a far-away text target. Search is a real filter, no fake universal search. Multi-select action band appears only during selection and owns pause/resume/remove; normal selection retains row actions. Bottom detail dock stays on demand (existing owner), contains long information and internal scroll. At minimum width prioritize name, state and available actions; optional description/path truncates rather than shrinking main text. Proposed48px two-line workspace rows need native density comparison; current46px is not blindly replaced in compact popups.

## Quick Find configuration (SH-03/QF-05)
```text
| Quick Find                           necessary adapter status |
| General | Trigger/hotkey / behavior                            |
| Index   | Label + description                  fixed control   |
| Context |                                                    |
| Commands| command list        selected command editor          |
| Diagnose| Name + keyword       name / keyword / target / args  |
```
Keep current General/Index/Context/Commands/Diagnostics topics. Main module rail and narrower topic rail are different levels; don't duplicate both with equal visual emphasis. Common actions adjacent to their object. Commands use a list+editor because that is the current workflow, not executions on selection. Scope paths/volumes preserve their existing list actions. Index unavailable/maintenance states remain truthful; no cosmetic Ready from synthetic success. Diagnostics are readable values and actionable failures, not decorative counters. Preserve immediate-setting behavior and per-operation feedback; no invented giant Apply button.

## Clipboard configuration (SH-04)
```text
| Clipboard                                current capture state|
| Trigger and actions                                           |
| Call shortcut / default action / modifier actions   controls  |
|---------------------------------------------------------------|
| Retention and privacy                                         |
| Item limit / age / capture limit                  controls     |
| Global app blacklist                             existing link|
|---------------------------------------------------------------|
| Storage                                                        |
| location / export/import / migration status       actions      |
```
Replace competing fixed-width two-column policy blocks with a readable ordered settings composition (candidate subsections may navigate, no new behavior). Each explanation belongs to its label. Immediate edits don't pretend to be unpublished drafts. Retention and migration errors preserve last-good owner; storage choices and import/archive remain separate actions. Do not move history into the main window simply to imitate another application's UI.

## Global settings (GS-01)
Current720×480:128px topic rail; title+short explanation; bounded content; footer48px. General, Network and Application blacklist remain distinct. Common grammar with download settings, not common persistence ownership.
```text
| topic rail | Application blacklist                             |
|            | app name          disable Clipboard disable Quick |
|            | editor.exe              [x]             [ ] remove|
|            | game.exe                [ ]             [x] remove|
|            | editor / add row shares exact column rails        |
+------------+---------------------------------------------------+
| local error/saved≠applied state                 Cancel    Apply|
```
Name/header left edge shared; boolean markers centered under header; editor uses the same grid. Name/path/.EXE duplicates normalize; editing scopes is independent. Cancel restores committed memory in prototype; actual atomic save/native confirmation remains production gate. General uses existing startup/tray intents; Network has mode/host/port and local validation. Navigation/editing disabled during apply; failure retains draft, not a successful-close toast.

## Download settings (DL-04)
Current984×650:General/Request/Transfer/Window/Diagnostics topic rail, one scrolling content and fixed footer. Category settings contain a stable master list and selected category form; name/location/extensions/URL patterns group by responsibility. Transfer limits are readable related fields, not arbitrary dense grids. Request headers/cookies/invalid-cert policy remain explicit sensitive-data settings; diagnostic logs never inherit these values. FFmpeg discovery/install feedback stays meaningful state transitions. Overlay options use measured lanes; avoid a fake animated preview that promises native behavior.

## Fast summon: Quick Find (QF-01–04)
```text
+-----------------------------------------------+ 620×44/410
| Search input                        shortcut  |
| app icon   name                       hint    |
|            path/type                          |
| selected row                                  |
| ...                                           |
| compact action/help strip if currently owned  |
+-----------------------------------------------+
                 passive preview 400×410 → (no focus/action)
```
Search first. Result rows use consistent icon/title/path/end-slot. Arrow selection and pointer selection feed the same preview proposal; Enter dispatches once. In-window action mode replaces results contextually, Escape returns to the selected result/search. Action-menu lazy submenus have loading/error rows and return focus. Collapsed search-only size remains distinct from empty-results state. Attached file-dialog search blueprint uses the same visual row grammar but existing host-following placement; it is not another standalone summon window. Preview content/type/header/metadata is passive and contains no hidden interactive focus target.

## Fast summon: Clipboard (CB-01–04)
```text
+-------------------------+ 300×440 default
| category strip     add  |
| Search history    scope |
| text preview/name       |
| source · time           |
| image/files summary     |
| selected row            |
| ...                 bar |
| current action hints    |
+-------------------------+
```
Panel widths are genuinely compact:12px compact type, bounded path/source/time; no small decorative cards. Category overflow is scrollable rather than tiny labels. Selected≠hover. Copy-only success remains open; confirmed paste follows existing terminal behavior. Edit/category overlay occludes underlying rows, focuses its input and returns focus on cancel. Text/image/files passive preview remains one retained window; no invented click-to-open controls there. Long history pagination doesn't become unbounded DOM/native payload in production.

## Fixed download windows (DL-01–03)
- Probe560×176: one URL field/paste and short stage/failure guidance; primary probe and cancel stay contained. A prototype button simulates the result, never accesses a URL.
- Confirmation560×278: compact12px fields, destination/name, category/schedule, inspector metadata, stable submit/cancel. It is a new fixed frame in production, not an animated same-HWND expansion. Pending URL/paste locked; native presentation failure preserves successful probe/draft.
- Media selection: separate recorded budgets for DASH353/HLS418/error456; render tracks/options with proper footer reserve. Don't show ordinary278 with extra tracks as if native-safe.
- Task440×184 progress/completed: identical frame/chrome/footer. Progress metrics become final-artifact summary/actions without size command. Expanded440×280 and failed440×196 are genuine geometry transitions; no main-workbench type inflation.

## Tools (SH-05/TL-01–02)
Only current Link Detection is shown. Entry is a continuous feature row with title, clear short behavior description, state and discoverable configuration.460×320 settings keep enabled/position/auto-hide/copy-only/browser behavior.420px link panel exposes bounded detected link/path rows, copy first, optional expansion and existing hide policy; no new scanner or parallel browser-opening flow.

## Supporting surface wireframes (DL-05–09/UI-01–02)
| Surface | Structure and observable intent |
| --- | --- |
| Schedule430×326 | Named start/stop fields, weekdays with checked/disabled states, bounded error, fixed Cancel/Apply; no auto-submit on selecting a day |
| Delete confirmation | Action subject + task/file consequences; separate delete-file option if currently present; neutral Cancel and one danger action; counts/long names don't displace footer |
| Shutdown450×200 | Countdown/message with stable cancel; motion reduced if system preference; no decorative ticking frames |
| Date/category/path popover | Anchored to trigger; one icon/title/value grid; selected and keyboard focus separate; bounded list; outside/Escape returns focus |
| Context menu/submenu | Normal, focused, checked, disabled, pending native enumeration and retry states; one submenu owner; no floating action-card wall |
| Notification | Title then bounded message then related actions; close aligned; bottom-right, max3 and no titlebar obstruction; generation-scoped expiry remains |
| Tooltip | One short label/help phrase, not a hidden essential instruction; no interaction/provider claim |
| Taskbar overlay | Existing configured anchor/size with metric rails; edit affordance constrained; no independent native position loop |
| Native tray/pickers/startup | Keep Windows native UI; product icon/copy coherent, no self-drawn tray substitution |

## Browser extension companion (EX-01–02)
Popup keeps actual task/capture/tools/settings views and lifetime/CSP; use same clear headers, continuous rows and related actions within browser width. Media selection row labels/size/type and one selection action, busy/failure state local. Shadow drag target communicates drop eligibility without imitating a desktop window. Prototype labels these as wireframes; no actual extension injection/download handoff or privacy payload transfer occurs.

## Prototype vs implementation
All visible actions are in-memory simulations; no user clipboard, filesystem picker, Shell action, indexer or network. Native show/focus/provider differences are explicitly outside the prototype. Every production migration requires reading matching reference notes and real constructor/native boundaries, adding behavior regression before changing code, then comparing actual rendering under the same data/budget.
