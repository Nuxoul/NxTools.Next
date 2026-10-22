# Layout quality acceptance matrix

## Evidence levels
- **S** source/document inventory: entry/budget known; no aesthetic or interaction claim.
- **P** local browser prototype: synthetic DOM geometry/interaction +1:1 visual inspection. Not GPUI/native.
- **G** production GPUI fixture: real render/bounds/focus/state with synthetic owners. Not installed physical gestures.
- **N** authorized native/installed acceptance: actual HWND/pointer/focus/DPI and owning lifecycle.
- **U** user visual approval: explicitly accepted image/workflow, not inferred from silence or test success.

The approved deliverable is S+P. G/N/U remain future gates for the proposed whole-app layout. Prior485 desktop tests are a product baseline, not evidence that this browser replan is integrated. State coverage of a representative does not equal all entries in its family.

## Visual quality rubric
Each page is independently judged; no overall average hides a failed critical page.
| Dimension | Observable pass condition | Failure examples |
| --- | --- | --- |
| Reading order | First task and primary action clear; no duplicated title/description | Caption+page title+card title compete; configuration presented as fake dashboard |
| Typography | Ordinary Chinese readable at actual size; one intentional hierarchy | Tiny `.small()` default, arbitrary sizes/line heights, muted critical instructions |
| Alignment | Shared header/data/editor column lines; row/control baselines match | Centered names under left header; checkbox/header different x; thin input next to taller action |
| Density | Useful information fits current budget without shrinking labels/inflating window | Blank card wall, giant filler margin, footer displaced or hidden action |
| Action hierarchy | Related actions adjacent, one primary per decision; eligibility clear | Blue every row, danger default emphasized, disabled only opacity |
| State stability | Long/empty/loading/error/busy preserve geometry and last-good/draft | Long path pushes controls, fake empty error, layout jumps on count/change |
| Interaction | Pointer/key equivalent once; focus/return/escape logical | Inert Tab stop, double Enter dispatch, modal click-through or focus theft |

Color token compliance is a separate check, not sufficient to pass any row above.

## Per-family matrix (browser design delivery)
| IDs | Required prototype/blueprint states and actions | Required size checks | Current evidence | Deferred |
| --- | --- | --- | --- | --- |
| SH-01–02 | task normal/long/empty/loading/error/busy; filter/search/sort, row/detail, multiselect |1060×650 and920×480; shared data header and action rail | S verified; P DOM/state/action checks pass; download screenshot inspected | G/N/U |
| SH-03/QF-05 | General/Index/Context/Commands/Diagnostics; list select/editor/local setting; pending hotkey/index fault | Both workbench sizes; every category in normal/long/busy/error | S verified; P layout+command/draft checks pass; command screenshot inspected | All actual topics G/N/U |
| SH-04 | Trigger/action/retention/privacy/storage; immediate local simulation; error/migration status | Both workbench sizes; every category in normal/long/busy/error | S verified; P categories/immediate-setting checks pass; screenshot acquired | G/N/U; complete visual review |
| GS-01 | General/Network/rules; independent scopes, add/edit/remove, apply/cancel, retained failure/busy |720×480; many/long/empty rules, header/editor centers | S; P categories/actions/alignment/scroll pass; blacklist screenshot inspected | Proposed G/N/U |
| QF-01–04 | search/arrow/Enter, action mode/Escape, no match/loading/error; passive/host wireframe |620×44/410,42px search/40px rows; preview wireframe | S; P collapsed/input expansion and result/action/keyboard checks pass | Host/native preview G/N/U; actual passive400×410 render |
| CB-01–04 | category/search/select, edit/cancel/focus; long/empty/fault; passive preview wireframe |300×440; editor inert background/focus return | S; P edit failure/keyboard/geometry pass; popup screenshot inspected | G/N/U; full preview/native copy-paste lifecycle |
| DL-01–03 | simulated probe/confirmation/retry/paused-add; progress/completed,expanded/failed;media wireframes |560×176/278,440×184/280/196; separate353/418/456 | S; P applicable states/budgets/actions pass; captures acquired | Native handoff/state/DPI G/N/U; full visual fields |
| DL-04–09 | settings/category schedule/delete/shutdown/overlay/date blueprints and supporting wireframes | Recorded984×650/430×326/450×200; others measured by owner | S blueprints; P supporting board/category states pass, not full-size interactive coverage | Full P and G/N/U |
| SH-05/TL-01–02 | current Link Detection entry/config and panel blueprint | Main default/min; native460×320/420px-derived panel deferred | S; P tools representative/layout pass, panel blueprint | Dedicated panel full P/G/N/U |
| UI-01–02 | component/menu/submenu/notification/preview/schedule/delete/shutdown board | Board default/min; annotated owner budgets | S; P board states,disabled/reduced-motion checks pass | Native lifecycle/provider G/N/U |
| EX-01–02 | extension popup/media/drag wireframes; busy/retry/keyboard annotations | Supporting board default/min, actual browser sizes deferred | S blueprint; P companion board layout pass only | Full extension DOM/runtime/U |

Update P cells from actual test/inspection report, not anticipated implementation. Keep failures and fixes in a dated evidence report. Screenshot evidence records page/state, frame dimensions, scale/zoom, source artifact and inspected status.

Final browser harness:305 passed,0 failed;125 applicable page/state/size cases +148 subsection/state/size cases +32 action/keyboard/alignment checks. Seventeen initial page choices plus the added collapsed-search choice are18 views, not18 fully migrated product surfaces. [Delivery evidence](../../../artifacts/ui-layout-quality/verification.md) records exact scope/failures/limits. Captured screenshots are browser CSS pixels and do not certify native font or provider quality.

## Prototype checks to execute
1. Parse JavaScript without dependencies and reject external requests/fonts/CDNs, user-data APIs, persistent storage or product endpoints.
2. Inspect representative pages at1:1 CSS pixels. Fit mode must visibly report scale and is not typography evidence; browser viewport may scroll around the prototype workbench.
3. Programmatically loop every available page×applicable state×default/min or fixed size. Audit inner horizontal overflow, footer/control bounds, nonzero interactive targets, fixed frame dimensions and deliberate scroll containers. Document unverified states.
4. Exercise actual DOM events for downloads filter/search/select/multiselect; rules normalization/scopes/add/edit/remove/cancel/error/busy; command editing; summon query/arrow/action/Escape; clipboard edit/cancel.
5. Verify Tab/Space/Enter/Shift+Tab and logical close/return on owning overlays. Hidden/disabled controls excluded; no duplicate handler paths.
6. Inspect normal+long+error screenshots for typography/columns/copy. Don't claim browser screenshots certify a11y provider, GPUI or native DPI.
7. Run spec-contracts and diff check; confirm production files weren't changed by this round. No Cargo/package/service/native test required or performed for P evidence.

## Future native matrix
For every implemented surface use100/125/150% DPI (and mixed displays for placed auxiliary HWNDs), long CJK/Latin names, values/count changes, keyboard/pointer and minimum viewport. Collect real GPUI screenshots before/after, source/binary identity and interaction traces only when explicitly authorized. Fixed windows need actual WM_SIZE/client readback plus content/footer. Passive previews need one retained HWND/noactivate/show-order and primary focus restoration; screenshots alone cannot prove these. C# tests use exact safe filters, not unrestricted DOpus harness.

## User approval questions after viewing
Confirm typography/readability and main rail/page hierarchy first, then list/table/editor and popup density. Proposed values remain candidates until these decisions. Record explicit accepted/rejected pages and state, not a global 'all UI complete'. Packaging/installation acceptance is a subsequent authorization and gate.

[Inventory](surface-inventory.md) · [Design system](design-system.md) · [Migration](migration-map.md)

## First production slice evidence (supersedes design-only execution scope)
User authorized local feature-first migration after pushed42f60a8. Integrated Desktop488 passed/0 failed; production/tests strict Clippy,workspace tests,fmt,Debug build and11 source gates passed. All original functional assertions retained; intended geometry expectations synchronized to measured layouts. Static13-view owner-call/settings-member comparison found no missing entry and9 critical controllers/fixed-window files unchanged; not semantic/native certification. Owned Debug100% main1060x650/min920x480 captures, pointer Downloads->Tab/Enter Quick Find->Clipboard and Clipboard wheel reaching archive/storage were inspected. Global720x480 and direct560x278 synthetic confirmation captures acquired; direct confirmation is not end-to-end handoff evidence. Exact files, failure fixes and limits: [audit](../../audits/2026-10-09-feature-first-ui-migration.md).

G evidence now exists for the bounded changed leaf/settings/shell slice; N/U still pending for installed/mixed-DPI/whole-state matrix. Full download-settings/command/index/storage operation screenshots, attached-search/native preview/OLE/provider and browser-extension refinement are not declared complete. No packaging/service/real Clipboard/DOpus/user URL/commit operation occurred.

## Remaining local source slice (supersedes first-slice omissions, not native acceptance)
Downloads auxiliary dialogs/tasks, Quick Find advanced command/volume/path/diagnostic components, Clipboard category/text/scope overlays, long notification/actions and browser companion presentation are now implemented. Final Desktop494 passed/0 failed/0 ignored; strict production/tests lint,workspace/fmt/Debug build pass. Production G evidence covers actual fixed/minimum/long/empty/disabled/focus/action boundaries. Browser companion uses product HTML/CSS without controller scripts:21 existing tests and38 DOM checks pass; this is not installed extension behavior. Static24-surface/11-callback/10-owner omission check passes.

Five owned Debug100% PNGs with measured clients are in `artifacts/ui-layout-quality/remaining-native/`; four browser PNGs in `remaining-browser/`. All new images remain uninspected: screenshot tooling returned another PI client surface twice, and those captures are excluded. Actual advanced editor/Clipboard overlay native screenshots, full states/DPI/physical gestures/live motion/native provider and U remain open. Source and automated slice completion is not whole-app N/U certification. [Exact evidence/failures/limits](../../audits/2026-10-09-remaining-ui-layout-migration.md).
