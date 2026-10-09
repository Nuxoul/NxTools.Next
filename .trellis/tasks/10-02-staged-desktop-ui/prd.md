# Redesign NxTools.Next Desktop UI in Stages

## Goal

Redesign the entire NxTools.Next desktop UI using the user's supplied GPUI desktop design specification while preserving all established workflows and data behavior.

## Scope

Included surfaces: shared titlebar/sidebar and working shell; Downloads list, settings and task/secondary windows; Quick Find search/results/settings/preview; Clipboard history/settings/preview; Tools and link panel; global network settings; notifications, dialogs, popups and auxiliary windows.

## Design Requirements

- Canonical base palette: base `#18181b`, surfaces `#1f1f23/#121215/#27272a`, subtle border `#27272a`, primary/secondary/muted text `#f4f4f5/#a1a1aa/#71717a`, semantic status roles. User2026-10-09 real Validation feedback supersedes earlier dark action blue: focus/action `#38bdf8`, hover `#7dd3fc`, distinct pressed response.
- Strict 4px spacing grid, compact controls (24/28/32 px), radius 3/6/8 px/pill by semantic role.
- Dense desktop-first alignment and clear hierarchy; avoid decorative gradients, excessive shadows, oversized cards and repeated one-size-fits-all card surfaces.
- Reusable typed theme tokens and GPUI components; no scattered hard-coded color/spacing values.
- Keyboard equivalents for actions, visible focus states, consistent naming/feedback, and restrained short user-triggered motion only.
- Preserve Rust state/layout ownership, GPUI rendering, C# Windows API boundary and all interaction lifecycles.

## Stages

1. **Foundation and representative shell:** existing source delivery supplies tokens/focus/typography and shared shell baseline; finish common capability gaps alongside remaining module work.
2. **Downloads:** main list, filters/detail, settings, schedule/delete/task/terminal windows.
3. **Quick Find:** search/results, settings, preview and contextual actions.
4. **Clipboard and Tools:** history, settings, categories, preview and link detector panel.
5. **Global settings and supporting surfaces:** network settings, notifications, popups/dialogs and final cross-app consistency/accessibility pass.

Execution amendment approved by user2026-10-09: implement all listed stages and common UI capabilities first, then consolidate visual/interaction closeout. Do not stop for separate user acceptance at stage boundaries; keep focused automated gates and preserve sibling lifecycle contracts throughout. Common scope explicitly includes reduced-motion preference propagation, old titlebar/custom action keyboard/focus parity, actual screen-reader names/roles where the pinned native/provider boundary supports them, and measured spacing/radius/fixed-content-budget migration. An unavailable provider capability must be reported precisely, never substituted with tooltips or invented compliance.

## Out of Scope

- Re-platforming the application or introducing a second GPUI runtime.
- Changing domain state, persistence, clipboard semantics, search scoring, download behavior, IPC, or Windows lifecycle solely for aesthetics.
- A single unreviewable mass rewrite; no stage is complete on compilation alone.

## Acceptance Criteria

- [ ] User tokens are centralized and `tests/architecture/ui-token-boundaries.ps1` enforces their use.
- [ ] Each stage names exact included files/modules, visual acceptance reference and behavior regression suite before implementation.
- [ ] Every major surface has keyboard/pointer parity, visible focus, readable density, and correct empty/loading/error/disabled states.
- [ ] Existing GPUI lifecycle gates remain green, including no synchronous cross-entity re-entry, passive preview reuse and native window bounds/show order.
- [ ] Each stage has before/after screenshots and a reviewed interaction checklist where the environment supports capture; unavailable manual evidence is reported explicitly.
- [ ] All four product modules and global/secondary windows follow one coherent design system without losing existing behavior.

## Whole-app layout quality replan — approved design-only scope
Following the installed typography/alignment rejection, the user approved producing a complete surface inventory, layout system, page blueprints, acceptance matrix, migration map and interactive browser prototype first. Deliverables live in `docs/ui/layout-quality/` and `artifacts/ui-layout-quality/prototype/`. Existing palette and established workflows/native budgets remain fixed. Workbench13/20 type,176px named rail and related density values are candidates for visual review, not approved production-token changes. No product code migration, Cargo/package/runtime/service/user-data operation belongs to this round. After prototype review obtain explicit approval for scoped production migration; retain canonical stage names/order and historical failures. Source tests/token presence alone do not satisfy layout quality or native/user acceptance.

Design acceptance: all current interface families mapped to real entry/budget/state; at least7 interactive representatives plus supporting wireframes/component board; default1060×650/min920×480 and fixed popup budgets inspectable at1:1; normal/long/empty/pending/error/busy and keyboard/actions documented and checked; prototype vs GPUI/native/user evidence separated. User visual approval remains outstanding until the resulting prototype is reviewed.