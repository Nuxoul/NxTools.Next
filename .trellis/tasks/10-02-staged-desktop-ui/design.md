# Staged Desktop UI Design

## Tokens

The user's supplied base tokens remain canonical: base `#18181b`; surfaces `#1f1f23`, `#121215`, `#27272a`; subtle border `#27272a`; primary/secondary/muted text `#f4f4f5` / `#a1a1aa` / `#71717a`; semantic status colors. User feedback2026-10-09 after native Validation acceptance explicitly rejects the dark action blue: focus/action accent is now brighter `#38bdf8`, hover `#7dd3fc`, with distinct pressed feedback. This supersedes the earlier `#0ea5e9` action/focus requirement; define roles once in theme.rs and map product/component consumers together.

Spacing is a strict 4px grid; controls 24/28/32px; radii 3/6/8px/pill according to role. Keep existing UI font family unless visual review demonstrates a concrete readability issue; centralize type roles. Focus states use a 2px or equivalent visible semantic accent ring. Enter/Space trigger the same actions as pointer. Motion is short and triggered by user action only.

## Hierarchy and composition

Keep the existing custom-framed shell and modules. Use the active module rail and precise content hierarchy as the signature device. Keep settings dense and structured, prefer continuous surfaces and meaningful separators to equal nested cards. Do not add decorative gradients, excessive shadows or oversized whitespace. Each popup/secondary window keeps a purpose-built lifecycle, not just a shared visual skin.

## Migration stages

1. Foundation/tokens/shared shell with one representative settings/workspace surface; existing source delivery and screenshots remain baseline evidence.
2. Downloads surfaces and settings.
3. Quick Find surfaces and preview/menu.
4. Clipboard and Tools including passive preview and link panel.
5. Global settings, notifications and supporting dialogs; final cross-app consistency/accessibility pass.

User-approved execution amendment (2026-10-09): implement the remaining stages in canonical order without pausing for user visual acceptance after each stage. Retain per-surface scoped changes and automatic regression checks; consolidate visual/interaction acceptance and release closeout after all requested surfaces/common capabilities are implemented. Behavior contracts are read before style migration. Rust owns layout/state; GPUI owns rendering; C# keeps Windows APIs. Don't refactor domain code to make styling convenient.

## Validation

`tests/architecture/ui-token-boundaries.ps1`, affected package GPUI tests, module interaction/lifecycle tests, accessibility/keyboard checks, and native screenshot matrix. Focused checks during implementation, full desktop tests/clippy/fmt at integration. User visual acceptance is now one consolidated closeout, not an implementation gate between stages. Packaging/install/service operations remain separately authorized.

## Global settings typography/alignment acceptance correction
User rejects the19:30 installed blacklist layout: tiny inconsistent component typography, centered executable labels under left-aligned headers, misaligned checkboxes and thin input beside taller actions. Palette-only work is not visual polish. Retain palette/font family/native720x480 budget and domain behavior; use a restrained Zed-like information hierarchy,128px left rail,20px content inset,14/20 title and12/18 ordinary text,40px table rows and32px editing/footer actions. Headers, rows and editor share fixed feature/action columns; application labels align left, checkbox marks align to feature-column centers. Keep one clear title/description, bounded scroll and continuous surface, not decorative cards. Actual native render inspection plus production center/baseline/long-name/many/empty/busy/keyboard/save tests is required; installed/DPI acceptance remains separate.

## Whole-app layout replan — design candidate authority
The user approved a design/prototype delivery before further production migration. [Candidate design system](../../../docs/ui/layout-quality/design-system.md), [surface inventory](../../../docs/ui/layout-quality/surface-inventory.md), [blueprints](../../../docs/ui/layout-quality/page-blueprints.md), [acceptance](../../../docs/ui/layout-quality/acceptance-matrix.md) and [migration map](../../../docs/ui/layout-quality/migration-map.md) now define the proposal to review. Candidate workbench13/20 body,12/18 support,16/24 title,176px named module rail and32px work-form/actions are not silently applied to current tokens; compact fixed windows retain measured12px density and protected budgets. Palette unchanged. One continuous workbench/settings/fast-summon grammar serves distinct real tasks, not fake dashboards or identical card templates. Full original stage order preserved; future code requires reference+red regression and actual GPUI render, not prototype screenshots as native acceptance.