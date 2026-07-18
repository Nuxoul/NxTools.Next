# Learnings

Corrections, insights, and knowledge gaps captured during development.

**Categories**: correction | insight | knowledge_gap | best_practice

---

## [LRN-20260718-002] correction

**Logged**: 2026-07-18T12:10:00+08:00
**Priority**: high
**Status**: resolved
**Area**: frontend

### Summary

NxTools.Next clipboard controls must use Google Material Symbols, and every GPUI icon asset must be embedded through an application `AssetSource`.

### Details

The UI used `gpui-component::IconName` without configuring `Application::with_assets`. The resulting SVG paths resolved against GPUI's empty default asset source, leaving category creation and management controls visually blank. The user also explicitly selected the Google Fonts Material Symbols library as the product icon source.

### Suggested Action

Use a typed Material Symbol map, embed selected official SVG files in the executable, and keep a test that loads every clipboard icon path through the real application asset source.

### Metadata

- Source: user_feedback
- Related Files: rust/apps/nxtools-desktop/src/assets.rs, rust/apps/nxtools-desktop/src/icons.rs, rust/apps/nxtools-desktop/src/ui
- Tags: gpui, icons, material-symbols, assets
- Pattern-Key: frontend.verify_runtime_asset_source
- Recurrence-Count: 1
- First-Seen: 2026-07-18
- Last-Seen: 2026-07-18

---

## [LRN-20260718-001] correction

**Logged**: 2026-07-18T09:35:00+08:00
**Priority**: high
**Status**: in_progress
**Area**: frontend

### Summary

Clipboard UI should use supplied projects for proven interaction rules and measurements while retaining NxTools.Next's own GPUI/Zed visual direction.

### Details

The first GPUI implementation exposed metadata as stacked chips, forced image content into the text-row geometry, and lacked a deliberate density/alignment system. The user rejected that result, then clarified that the old project is a reference rather than a pixel-copy target. Reuse behavior and layout logic, not the old Avalonia appearance.

### Suggested Action

Before editing GPUI layout, inspect the exact old NxTools view and supplied screenshots, extract interaction/layout invariants, then reinterpret them through the new GPUI/Zed design system and require screenshot-based design QA.

### Metadata

- Source: user_feedback
- Related Files: rust/apps/nxtools-desktop/src/ui/history.rs, rust/apps/nxtools-desktop/src/ui/mod.rs
- Tags: gpui, clipboard, visual-fidelity
- Pattern-Key: frontend.reference_fidelity_before_implementation
- Recurrence-Count: 1
- First-Seen: 2026-07-18
- Last-Seen: 2026-07-18

---
