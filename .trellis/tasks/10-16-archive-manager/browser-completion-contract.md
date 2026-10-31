# Browser completion and explicit integration repair — 2026-10-27

The user requests finishing the preceding partial implementation and repairing the reported
integration denial. Source implementation and isolated verification only; no real workstation
Registry/COM/Shell/service mutation, product/GUI launch, package/install/signing or commit/push.
Canonical phases and earlier failures/installation-error1 OPEN history remain unchanged.

## Smallest change boundary and reviewed public components

Read original PRD/design/implement/ui-spec/PhaseC, prior correction request/outcome, archive and
Markdown references, Desktop/shared guidelines, current Kit0.8.0 delta/table/index. Exact released
Component list/list_item.rs::render retains public styled selection/content/callback; Tree::render
uses original Base Tree state and supplied ListItem; resizable.rs::h_resizable/resizable_panel
retain official handles, ranges and resize callbacks. No new selection/menu/cache/native runtime.

- Tree/list directory PNG becomes24DIP in original32DIP rows. Original shared TypeIcons/cache8pending/
  128types/15seconds and stale guards are preserved. A local archive filled Material folder asset
  derives from the existing Apache-2.0 Material folder path, does not repaint a control. Tooltip
  distinguishes pending/failure/offline fallback, never probes member paths or extracts for icons.
- Original details columns/semantic indexes/persistence remain. Name space and horizontal scrolling
  survive minimum720x480 and default960x640, both themes and longnames. No unrelated layout reset.
- Optional browser right preview uses existing h_resizable, default280DIP/range200..600, original
  content minimum160DIP. Small outline source/retry/close Buttons outside Toolbar.child, semantic
  Empty/Alert/Spinner. One local presentation snapshot/generation, no alternative preview controller.
- Selection mutations notify preview before deferred cross-owner work; stable snapshot+row+local
  generation+enabled gate protects ABA/cache-hit and late results. Render never performs IO/decode.
  Original BrowserModel remains the only selected/focused/anchor owner; doubleclick opens unchanged.
- Original EntrySession/worker/source/password/cache/lease handle readonly preview purpose, bounded
  actual output and background decode. Images are owned decoded pixels, so file lease is released
  only after actual read/decode and cannot be referenced by path-backed rendering. Unified passive
  MarkdownDocument uses placeholders/no HTML execution/network/local-resource loading. Exact finite
  resource and cancellation contracts are recorded by the backend implementation.
- Integration repair is at original ArchiveIntegrationService production composition. Explicit user
  Apply/Revert may reach original HKCU transaction after existing package/helper identity checks;
  saved intent != observed applied. No UserChoice/HKLM, unsafe identity bypass, new registration
  framework or actual Registry mutation during this development. Separate integration report owns
  real-boundary red/green and source amendment.

## Regression/acceptance

Before UI changes, actual production render tests assert recognizable Tree/list folder size and
real View menu preview entry. Resource/cancel/latest/lease tests exercise actual entry/result owner
and original native worker seam. Real GPUI pointer/keyboard/splitter/scroll and Light/Dark minimum
bounds are required. Tests/captures do not certify physical/DPI/provider/user visual acceptance.
Original native row OLE path, header/grips/bar/splitter gestures, parent row and transient cancellation
stay covered. No current scope grants packaging or installed runtime acceptance.
