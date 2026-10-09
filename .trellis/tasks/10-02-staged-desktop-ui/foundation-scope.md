# Foundation — approved implementation start, 2026-10-09

User explicitly requested UI polish after the download-window recurrence. Start the existing **Foundation** stage, not a new task or a jump to Downloads. No automatic package/install/product launch, real DOpus or service operation. Existing native download fixes stay untouched. Keep in_progress until visual/interaction acceptance.

## Smallest concrete delivery

Replace the old global Zed color mapping with the approved NxTools palette; map compact/default controls to24/24, action28/input32, control radius3 and sidebar row48. Existing fixed-client settings row42 remains staged debt: a global48 attempt changed actual media budgets and clipped network footer, so it was withdrawn. Update the production shell sidebar to four-grid geometry, stable selected accent rail, visible focus and pointer/Enter/Space parity. Refine Tools as one representative continuous workspace with a shared module header and full-width tool entry; preserve its one-per-tool settings-window ownership and existing persistence/cancellation behavior. Centralize the component-theme mapping so production and GPUI tests use the same palette/fonts/focus/default buttons.

Expected files: theme.rs (semantic palette + component mapping); ui/metrics.rs (shared roles); ui/components/action.rs (focused keyboard-action primitive and primary contrast); main.rs (use centralized component mapping); main_window.rs/shell.rs (local focus + existing module selection); main_window/tools_page.rs (representative presentation only); existing GPUI tests; token/source gates and this task/reference/quality records. No dependency/vendor change, domain/protocol/storage/Windows API changes, broad module redesign or visual-only native bounds changes.

## Visual intent

Base #18181b, work surface #1f1f23, recessed well #121215, hover/raised #27272a; action/focus #0ea5e9 / hover #38bdf8; primary text #f4f4f5, secondary #a1a1aa, muted #71717a. Semantic green/amber/red values remain. Derive selected tint from accent/base, not an unrelated palette. Cyan primary controls use a dark foreground for readable contrast. Existing Microsoft YaHei UI/Consolas type roles retained. Continuous left-aligned work surfaces, one quiet active-module rail; no decorative gradients, large whitespace or equal card grid.

```text
36px caption: brand | active module                         settings | native controls
208px rail                    | 48px module header: title / metadata
  workspace                   | representative content, 16px inset
  [icon title           ]     | tool icon / title + purpose / status / configure
  [     description     ]     | continuous surface; no nested card wall
```

## Preserved invariants

Old MainWindow constructor/Open/Closed/tray paths and WindowAnimationService were read through actual show/close attachment; Avalonia native/base source is absent. Current MainWindow render -> titlebar typed C# move/minimize/maximize/close remains unchanged. Module switch retains controller entities and clears an outgoing orphaned menu focus; keyboard selection restores focus to the selected navigation entry. Tools opens/reuses its registered tool settings window, not a new app flow. Callbacks update local view state; cross-owner settings updates stay deferred. No native bounds animation or focus theft from passive previews. Theme change must preserve download176/278 content budgets.

## Baseline and regression seams

Before edits, production theme uses old #1e1f22/#242529/#4791bf; metrics18/22 and radius4; sidebar42px,10px padding/gaps and1px text gap; custom sidebar/tool clicks have no keyboard parity. Existing production geometry/lifecycle tests are the behavioral baseline. Snapshot relevant source files in session scratch before implementation; native screenshots are unavailable without product launch authorization, do not invent before/after images.

Add red-capable exact approved-palette and actual MainWindow keyboard-switch tests first. Verify shared control height/radius, continuous Tools geometry at920x480 and1060x500, Tab/Shift+Tab, Enter/Space one activation, held/modifier suppression, disabled/loading eligibility in the shared action seam, and existing tool settings persistence/window reuse. Capture actual geometry/state evidence; a schematic or browser mock is not a GPUI screenshot.

Run one Cargo at a time with required memory settings; focused GPUI checks while implementing; desktop full suite once at end, strict Clippy/check/fmt plus UI token/consistency/state/shell/download/preview gates. No C# suite needed because native boundary unchanged. One optional directed SOL review only, no broad review loop. No commit/archive.

## Explicit remaining Foundation gaps

Module-local2/6px layout roles are not approved exceptions and remain staged migration debt; do not mass-remap them into fixed-client layouts. This delivery adds no fake reduced-motion setting or unsupported accessibility labels; global OS preference/provider integration remains unimplemented. Caption controls and older custom actions are not declared keyboard/provider-compliant from changing palette. Foundation stage acceptance still requires real screenshots and keyboard/pointer/DPI review, then user visual approval before Downloads.

## Native preview authorization — follow-up
User said to start showing/checking the actual interface after explanation of visual acceptance. Authorized a bounded isolated Debug GPUI preview, not service/clipboard/DOpus/runtime initialization or installer packaging. Add main_window/visual_preview.rs and explicit Debug argument intercept before runtime/protected redirect; use actual MainWindow/Root/theme with invalid business refs. Capture only its owned HWND, never desktop screenshots. Main/Tools preview is actual native rendering but no end-to-end business runtime evidence. Download confirmation/native recovery requires a separately isolated fixture and must not be inferred from this main preview.
The separate download Debug fixture now renders synthetic confirmed metadata at560x278 using an in-memory engine/dropped platform endpoint; it never invokes probe/submission. Native100% screenshots and message-level navigation captured; actual product runtime/C# resize recovery, physical drag and125/150% DPI remain out of this fixture's evidence. Browser guest viewing failed; PNG delivery is not agent visual approval.
