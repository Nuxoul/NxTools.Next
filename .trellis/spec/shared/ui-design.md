# UI design and interaction contract

## Authority and scope
**Approved target:** [staged desktop design](../../tasks/10-02-staged-desktop-ui/design.md). **Observed implementation:** `repo:rust/apps/nxtools-desktop/src/theme.rs#Theme`, `repo:rust/apps/nxtools-desktop/src/ui/metrics.rs`, `repo:rust/apps/nxtools-desktop/src/ui/typography.rs`, `repo:rust/apps/nxtools-desktop/src/ui/motion.rs`. [Earlier remediation](../../../docs/ui-remediation-plan.md) and [GPUI reference](../../../docs/reference-code/gpui.md) remain historical source/behavior evidence, not numeric authority over the later approved brief. No product token consumer was changed by this standards task.

## Approved target / current / exception matrix
| Role | Approved target | Current evidence | Resolution / exception status |
| --- | --- | --- | --- |
| Base/surfaces | #18181b; #1f1f23/#121215/#27272a | Foundation source now maps these exact roles; selection/pressed tints derive from accent/base | Product mapping implemented2026-10-09; real screenshot/DPI visual approval pending |
| Border/focus/accent | subtle#27272a; user2026-10-09 amendment brighter focus/accent#38bdf8; hover#7dd3fc | Product/component mapping follows brighter accepted direction; old#0ea5e9 action rejected in real Validation feedback | Supersedes earlier action-blue token, not base palette; native visual approval pending |
| Text | primary#f4f4f5, secondary#a1a1aa, muted#71717a | Foundation uses exact approved values | Actual CJK readability at supported DPI still requires visual review |
| Status colors | semantic green/amber/red | #6fbf8f/#d6a85f/#ff6b78 | Approved brief fixes roles, not exact values; reuse semantic values pending visual review, no invented replacement palette |
| Controls | 24/28/32 logical px by role | Foundation compact/default24, action28/input32; settings-toggle boundary tests use actual24 | Shared target implemented; fixed-client regressions retained; no oversized-window workaround |
| Layout spacing | strict 4px grid | Shared settings rows40/header28, menu/hotkey/field/state/diagnostic gaps4/8; module-specific measured roles remain | Consolidated migration retains ordinary176/278, media budget now HLS418/error456 and one-row DASH353 with real footer gates; no blind inflation |
| Radius | 3/6/8 or semantic pill | Control3/floating6 plus component large-dialog8 mapped | Shared roles implemented; actual native alpha corners/hit testing still independently accepted |
| Type/fonts | Centralized existing family unless demonstrated issue; compact legible text | YaHei UI/Consolas; 16/14/12/11/10, line22/20/18/16/14 | Retain roles/family absent measured issue; Body/Label for normal Chinese information, Meta10 for genuine metadata only; status glyph28 is not ordinary text |
| Focus | Visible 2px or equivalent semantic ring; keyboard equivalent actions | Caption/toggle/state/nav/weekday/history/category/radio/command/volume/hotkey/result/menu/disclosure actions use existing GPUI focus + KeyUp click | Final474 desktop tests cover changed actual action/budget paths; provider/native whole-app acceptance remains open |
| Motion | Short user-triggered only; honor reduced preference | C# SPI_GETCLIENTAREAANIMATION -> dedup typed event/per-session snapshot -> Rust global; all current animation sites branch | Unknown/disconnect/failure reduce; local view observers defer repaint; synthetic state/render/close gates pass, live Windows toggle/reconnect pending |

Exceptions require: named role/surface, exact reason (content/native/hit target), measured regression, approved owner decision and verification evidence. Existing tests or comments alone cannot authorize a new exception. Native client/content budgets (ordinary download176/278, media-aware sizes), actual pixel readback, passive preview and activation/focus contracts are behavioral constraints; styling must not defeat them.

## Composition and semantic roles
A compact Windows productivity tool uses continuous work surfaces, clear module rail/search/list hierarchy, restrained separators and one primary action per decision area. No equal card walls, decorative gradient/glass/oversized radius or idle animation. Accent means action/focus/selection, not a large decorative background. Reuse components in `repo:rust/apps/nxtools-desktop/src/ui/components/action.rs`, field/list/settings/titlebar/menu/popover/state. Shell owns layout/chrome, not persistence or native policy. Real brand/SVG icons are registered through assets; font_family is one installed family, not CSS fallback string.

## Component state matrix (required on every changed actionable surface)
| State | Required behavior / assertion |
| --- | --- |
| Default | Stable label/baseline, semantic role, actual hit bounds; icon-only tooltip/name |
| Hover | Subtle semantic feedback, no layout shift or domain side effect |
| Pressed | Distinct active feedback; pointer capture/native gesture preserved; exactly one dispatch |
| Focus-visible | Visible semantic outline/equivalent; no clipping; Tab/arrow order and return-to-trigger |
| Selected/checked | Persistent independent state, not only hover; one selected segmented option, stable count cell width |
| Disabled | No pointer/keyboard dispatch; reason discoverable when useful; not merely low-opacity clickable text |
| Loading/busy | Stable geometry, bounded operation/ID; avoid duplicate submit; cancellation/retry policy follows owner |
| Empty | Genuine no-data vs filtered no-match; useful next action; never database/network error disguised as empty |
| Error | Local actionable typed failure, relevant native stage; keep draft/last-good state and retry reachability |
| Success | Observable result without focus theft; copy-only stays open where policy says; persisted != applied |

Evidence: `repo:rust/apps/nxtools-desktop/src/ui/components/state.rs#empty_state`, `repo:rust/apps/nxtools-desktop/src/ui/components/action.rs#action_button`, `repo:rust/apps/nxtools-desktop/src/quick_launch/actions.rs`, `repo:rust/apps/nxtools-desktop/src/network_settings.rs`. Custom clickable divs do not automatically inherit Button keyboard/role/disabled behavior; audit in Foundation, do not pretend every current component meets this matrix.

## Keyboard, focus and accessibility
- Enter/Space activate the same eligible action as pointer; Escape dismisses the owning transient level and returns focus according to lifecycle, not unconditional app close. Tab/Shift+Tab order is logical; arrows navigate list/menu/segmented structures; no trapping unless a real modal owns it.
- Primary search popup remains activating; auxiliary preview stays non-activating. After first preview creation restore native primary HWND and GPUI search-input focus. Pointer and keyboard selection share one generation-scoped preview controller.
- Icon action has tooltip and semantic name/role when the pinned API can expose it; descriptions/errors need association with fields. Do not claim screen-reader support from tooltips or screenshots. Verified GPUI0.2.2 source lacks accessibility/accesskit/rendered role builder/Windows WM_GETOBJECT provider; actual reader names/roles are blocked on a dependency/native-provider seam. No dependency upgrade or cross-process provider injection is implied by ordinary UI styling authorization.
- Severity and selection use text/icon/shape as well as color. Inspect contrast and disabled legibility on actual rendering; do not invent measured compliance from hex alone. Long paths/errors wrap/clip within content, never push fixed footer out of bounds.

## DPI, surfaces and motion
Logical layout -> final device-pixel client bounds -> native apply/readback -> WM_SIZE viewport. Set final auxiliary bounds before reveal; default hidden HWND origin is not trustworthy. One native resize owner after real HWND creation; no competing GPUI resize. Use production render/hit-target bounds tests plus 100/125/150% visual/keyboard/pointer matrix when authorized, including move during probe and close/reopen generations. Transparent child clipping cannot prove transparent HWND corners/hit testing.

Motion responds to a user transition once and stops at rest. Reduced-motion reads the system client-area animation preference through C# and typed IPC; no fake app flag. Rust unknown/failure/disconnect defaults reduce and skips animation elements/timers while preserving immediate feedback. Global changes notify weak local views on the next turn; native bounds never animate. Per-frame logs/work are forbidden; timeouts don't mean UI-ready.

## Stage and acceptance handoff
Canonical sequence: **Foundation → Downloads → Quick Find → Clipboard + Tools → Global settings + supporting surfaces**. Read the [owning plan](../../tasks/10-02-staged-desktop-ui/implement.md) before each stage. User amendment2026-10-09 authorizes implementing all remaining surfaces/common capabilities first and consolidating visual/interaction acceptance at final closeout; no separate user pause between modules. Preserve scoped baseline/state/regression and native evidence, not blind mass replacement. Link Detection stays default copy-only, six-second configurable generation-scoped hide, optional browser route and one popup; its [existing task](../../tasks/10-02-link-detector-panel/prd.md) owns behavior. Native/release/service operations still need their own authorization.

Consolidated source/check evidence: [2026-10-09 audit](../../../docs/audits/2026-10-09-consolidated-ui-polish.md). Final production/test lint and474 desktop tests pass, with11 source gates and native100% screenshot subset. Retired2/6 layout spacing tokens and Downloads raw off-grid gaps are gated; exact retained calendar/titlebar/footer/detail/meta/preview chrome roles are named migration debt in the audit, not blanket target compliance. Actual reader provider is unimplemented/blocked; full surface/state/native DPI/live motion/download recovery acceptance remains open. No package/install/service authority is implied.

## Whole-app layout-quality replan (design-only)
After user rejection of installed typography/alignment and palette-only polish, the approved next deliverable is [layout documentation and prototype](../../../docs/ui/layout-quality/design-system.md), with [complete inventory](../../../docs/ui/layout-quality/surface-inventory.md) and [per-family acceptance](../../../docs/ui/layout-quality/acceptance-matrix.md). Proposed workbench13/20 body,176px named module rail and related values remain candidates until user visual approval plus native comparison; current production tokens above are not automatically superseded. Preserve existing palette and compact/native budgets. Test actual text/columns/control centers and all states; token gates/no overflow are necessary but not aesthetic certification. Browser DOM/ARIA/screenshots do not prove GPUI, passive HWND/provider or installed-DPI behavior. Current scope excludes product migration/build/package/service/user data; future migration retains the canonical order and reference/lifecycle gates.
