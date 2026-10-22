# Layout quality design system

## Status
Initial design/prototype approval was followed by explicit local production migration approval after checkpoint42f60a8. First implementation uses opt-in13/20 workbench body and176px/40px named rail, while compact/task/summon typography and native budgets remain unchanged. These values have synthetic GPUI geometry/render evidence, not blanket user/mixed-DPI visual acceptance. Preserve [feature-first contracts](feature-preservation.md), [behavior/architecture](../../../.trellis/spec/shared/architecture.md) and fixed windows in the [inventory](surface-inventory.md). Prototype content is never the authoritative feature inventory.

## Point of view
NxTools is a Windows productivity workbench with fast summon surfaces. Borrow Zed's continuous planes, restrained chrome, disciplined start/end slots, subtle actions and clear focus. Do not borrow editor tabs, a project tree, decorative terminal motifs or a metrics dashboard. Quality means a coherent reading/action order and precise geometry under real data, not a familiar palette.

Rejected defaults: repeated title+subtitle+card title; icon-only primary navigation; equal card walls; centralizing every label; reduced Chinese text to create density; spreading controls to distant edges; giant filler sections; whole-view animations. Tighten duplicate copy and hierarchy before reducing type.

## Color roles (installed-feedback correction)
| Role | Existing direction | Use |
| --- | --- | --- |
| Canvas |#18181b | Continuous working plane |
| Raised surface |#1f1f23 | Distinct chrome/auxiliary surface, not every row |
| Recessed |#121215 | Input/content recess when semantically useful |
| Border/element |#27272a | Structural dividers and neutral interaction |
| Primary / secondary / muted text |#f4f4f5 / #a1a1aa / #71717a | Critical / explanatory / nonessential; muted is not ordinary body |
| Action / hover |#bfc0c4 / #d5d6da | Restrained neutral primary action; no cyan navigation fill |
| Selection / keyboard focus |#353539 / #9d9ea3 | Separate selected plane and keyboard-only ring; mouse click retains functional focus without persistent outline |

Semantic statuses retain existing green/amber/red. Use label/icon as well as color. No new theme, gradient or saturated status chips for every value. Contrast is inspected on rendering; hex values alone are not compliance evidence.

## Typography candidates
| Context / role | Candidate size/line-height | Weight / intent |
| --- | --- | --- |
| Workbench page title |16/24 | Semibold; once per content context |
| Group/dialog title |14/20 | Medium/semibold; no more than needed |
| Workbench body/control |13/20 | Regular; labels, task names, normal actions |
| Supporting copy |12/18 | Regular; readable path/description/state text |
| Genuine metadata |11/16 | Timestamps/counters only, never critical errors/settings |
| Fixed small-window body/control |12/18 | Compact tier with preserved native budget |
| Fixed small-window secondary |12/16–18 | Avoid arbitrary reduction; bound/wrap noncritical content |

Windows UI font stays Microsoft YaHei UI. Browser fallback differs only to make the prototype portable. Consolas is reserved for command/structured technical payload, not every counter or English label. English/CJK share visual baseline; names remain natural text. No tracking or all-caps status decoration. Define type+line-height+weight as one semantic role, not independent per-leaf adjustments.

Native comparison must decide13 vs12 workbench body and measure CJK readability at100/125/150%. A browser design can select an intended role, not certify DirectWrite rendering. If the fixed278 confirmation cannot fit a larger role, keep its compact role rather than inflate or silently omit fields.

## Layout classes
### Workbench
- Default1060×650, min920×480 retained.
- Proposed main rail176px (current208); single-line16px icon + name,36–40px action height, no repeated module descriptions. Do not introduce a collapsed icon-only mode without a separate product decision.
- Chrome36px and module header48px; body owns toolbar, content and optional detail. Header may contain title, essential state and one primary action. No second page title below it.
- Content gutters16–24px;20px candidate inset on ordinary settings. Reading columns have a reasonable max width; control panels do not stretch to fill arbitrarily wide windows.
- At minimum width, flexible name/description shrinks before fixed actions; secondary copy may wrap, but search/actions remain usable. No hidden core action/no tiny-text rescue. If optional detail reduces result area too far, use existing scrolling and bounded detail, not a new lifecycle/side-window owner.

### Settings
- Narrow section navigation128–160px based on labels; keep a stable active item and keyboard order.
- Each group has one heading; labels/descriptions occupy a flexible text area, controls a deliberate fixed/minimum rail. Form inputs32px; inline actions in that form32px. Compact toolbars28px; glyph-only24px hit targets only when separately measured.
- Short related fields may share one row; explanations wrap under their owning field, not inside unrelated columns.
- Global settings720×480 and download settings984×650 remain separate. Main-workbench Quick/Clipboard settings keep existing owners and immediate/draft semantics. A common appearance does not create a common transaction.
- Only draft owners get Apply/Cancel. Immediate-setting pages clearly identify the simulated immediate behavior in the prototype; production feedback stays correlated with durable/native completion.

### Fast summon/task windows
- Search or active task is the first reading anchor. No main rail, repeated brand hero or long explanatory introduction.
- Keep existing dimensions: Quick Find620×44/410, Clipboard300×440 default, Downloads560×176/278 and task440×184/280/196. Separate media budgets and previews remain separate.
- Preserve search focus, generation-scoped result selection, in-window action mode, passive auxiliary previews and one native placement owner. No unrequested resize animation.

## Alignment grammar
1. Text/name/path labels left-align with their owning header; vertical centers align across a single action row.
2. Numeric tabular columns right-align; use fixed digit/value rails when rapidly changing numbers would jitter other cells.
3. Boolean feature columns center-align header and check marker; header/data/editor share one grid specification.
4. Row actions are a stable end slot; text overflow cannot push them. Selected rows are continuous neutral planes, not individual framed cards.
5. Input, adjacent chooser and submit share height, baseline and focus treatment. Do not rely on `.small()` defaults to choose fonts or dimensions.
6. Icon glyph is centered inside its hit rectangle. Glyph size and hit-target size are distinct roles, not two competing layout boxes.
7. Alignment gates use actual bounds (within1 logical pixel) plus screenshots; source flex directives alone are insufficient.

## Rhythm and visual structure
Use4px grid:4 intra-label,8 related controls,12 compact internal padding,16 related sections,20/24 content/group spacing,32 major separation only when justified. Define role tokens after approval. No cumulative nested margins. Rows40px for compact one-line lists;48px for two-line task/results where content needs it. Existing fixed40px result rows are a protected compact case, not a mandate to inflate popup height.

Radii retain3px controls,6px floating surfaces,8px supporting dialogs and square continuous roots as appropriate. Shadows only distinguish actual elevated layers; no double elevation. One outer border or divider is usually enough. Scroll the owning list/content; never body+inner-list+editor all without a clear reason. Headers and primary action bands remain stable.

## State and interaction grammar
| State | Required quality |
| --- | --- |
| Default | Clear eligible action and label; no placeholder as the only accessible label |
| Hover/pressed | Subtle local feedback; no layout movement, duplicate dispatch or domain side effect on hover |
| Selected | Stable continuous fill/small marker; separate from keyboard focus and hover |
| Focus | Visible2px ring/equivalent; logical Tab/Shift+Tab; same eligible pointer/keyboard boundary |
| Disabled/busy | Real disabled input/action; reason when useful; existing data/draft intact; no extra hidden tab stops |
| Empty/no match | Distinguish missing data from filtered empty; one useful next action |
| Pending/loading | Stable geometry and quiet text; no decorative perpetual motion |
| Failure | Local actionable feedback, relevant retry; no fake empty; draft/last-good retained |
| Success | Result and persistence/native status truthful; no focus theft; no blanket confirmation toast |

Escape dismisses the owning transient level and returns focus; it does not universally quit. Result list arrows preserve search focus policies; Enter has one action path. Native drag remains a still-pressed pointer/OLE gesture, not a click simulation. Passive previews contain no actionable controls.

## Budget and exception process
A budget exception names surface, content reason, measurement, regression, native consequence and approval. Never increase a window only to hide clipping. Blueprint dimensions marked illustrative are not verified production values. A new text tier requires long/error/keyboard/full-state testing before shared migration. A visually attractive normal-state screenshot cannot excuse missing footer/error states.

## Approval checkpoints
1. Inspect1:1 prototype pages at default/min sizes and all significant states.
2. Select typography/navigation proportions; record resolved values and remaining constraints.
3. Implement a production GPUI Foundation representative and compare native rendering with prototype.
4. Only then migrate in canonical order with preserved reference lifecycle tests. Packaging/runtime/DPI/service actions require their own authorization.

[Page blueprints](page-blueprints.md) · [Acceptance matrix](acceptance-matrix.md) · [Migration map](migration-map.md)

2026-10-10 installed user feedback explicitly rejects the earlier cyan action/focus treatment. The neutral semantic roles above supersede#38bdf8/#7dd3fc for desktop product chrome; prototype/browser historical palette remains separate evidence, not current installed approval. Preserve original field/action/keyboard and fixed-client budgets. No 'visual accepted' claim from neutral token tests.
