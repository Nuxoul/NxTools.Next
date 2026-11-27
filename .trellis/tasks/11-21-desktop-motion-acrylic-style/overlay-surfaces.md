# In-window Dialog and Sheet surfaces

## Stage-four boundary

This slice of **阶段四：全软件表面与动效接入** maps the existing in-window overlays to the shared overlay role. Kit/Base/Component/Assets remain 0.10.0 with default pre0.3.12. Native material, Root projection, window readiness and motion retain their existing owners.

Acrylic projects ordinary `colors.background.a = 0`; the per-HWND Root plugin supplies qualified native tint. Dialog and Sheet must not inherit that transparent window-foundation token. Every production opening closure now starts with `let dialog = dialog.bg(crate::theme::overlay_background(cx));` or the corresponding Sheet binding. This also covers the existing early-return branches. The theme owner provides the original `tokens.background.background` for Solid and `tokens.popover.background` for Acrylic (popover alpha 0.98).

## Component selection and exact public slot

The requested shape is unchanged: existing official modal decisions/forms and one right-edge extension-detail Sheet. All current widths, margins, modal/overlay options, content/footer/action/close parts, visible Button variants and sizes remain. No Toolbar adaptation or replacement ordinary control is introduced.

- `vendor/gpui-component-0100/src/dialog/dialog.rs:607–610`: `impl Styled for Dialog` returns the component's `StyleRefinement`. The actual outer `surface` (`dialog-{layer_ix}` debug selector) applies `tokens.background` at line 782, then `.refine_style(&self.style)` at line 790.
- `vendor/gpui-component-0100/src/sheet.rs:144–147`: `impl Styled for Sheet` exposes the same public slot. The actual `sheet-content` surface defaults to `tokens.background` at line 195, then refines with the caller style at line 198.
- The public `gpui_kit::Styled::bg(impl Into<Background>)` is therefore the outer-background seam, not a body-content fill or a vendor modification. The existing `WindowExt::open_dialog` / `open_sheet` remain the only opening APIs.

Reference contracts reviewed: [archive](../../../docs/reference-code/archive.md), [Markdown reader](../../../docs/reference-code/markdown-reader.md), [downloads](../../../docs/reference-code/download.md), [extensions](../../../docs/reference-code/winshun.md), [current release](../../../docs/reference-code/gpui-kit-0100.md), and mandatory component rules/table/index. This is a paint-role adjustment, not an interaction migration.

## Production owner inventory

Paths below are relative to `rust/apps/nxtools-desktop/src/`. Lines identify the outer `.bg` binding. Total: **18 files, 24 Dialog closures, 1 Sheet closure**.

| Owner | Surface | Background lines | Count |
| --- | --- | --- | --- |
| `archive/application.rs` | Source-open failure, incoming-drop choice, picker failure | 537, 1294, 1682 | 3 Dialog |
| `archive/progress.rs` | Stop decision and job prompt (all prompt kinds) | 582, 669 | 2 Dialog |
| `archive/password.rs` | Browser password | 264 | 1 Dialog |
| `archive/forms.rs` | Reclamation consent, discard encryption on format change | 927, 1109 | 2 Dialog |
| `archive/entry.rs` | Active-content consent, operation notice, preview password | 119, 460, 1202 | 3 Dialog |
| `archive/browser_find.rs` | Snapshot-scoped Find | 88 | 1 Dialog |
| `archive/browser.rs` | Clear-recent failure, modification refusal | 1345, 1363 | 2 Dialog |
| `downloads/new_download_view/template.rs` | URL-template preview | 128 | 1 Dialog |
| `downloads/paused_url.rs` | Paused-download URL replacement | 64 | 1 Dialog |
| `main_window/extensions_page.rs` | Extension details | 235 | 1 Sheet |
| `main_window/extensions_page/import_confirmation.rs` | Import/replace confirmation | 62 | 1 Dialog |
| `main_window/extensions_page/trust_confirmation.rs` | Runtime trust confirmation | 123 | 1 Dialog |
| `main_window/extensions_page/storage.rs` | Restore/clear/retry storage decision | 321 | 1 Dialog |
| `main_window/extensions_page/removal.rs` | Removal review | 48 | 1 Dialog |
| `extensions/manual_actions.rs` | Manual-run feedback | 171 | 1 Dialog |
| `tools/raw_join.rs` | Numbered-file join consent | 299 | 1 Dialog |
| `tools/markdown_reader/history.rs` | Restore-version confirmation | 294 | 1 Dialog |
| `tools/markdown_reader/edit.rs` | Discard-draft confirmation | 263 | 1 Dialog |

## Preserved invariants

Rust keeps the same drafts, source/snapshot identities, generations, pending latches and error states. The original callbacks, deferred cross-entity work, cancellation, modal decisions, actions and footer reachability are unchanged. Existing official Root/Dialog/Sheet owners retain placement, focus trapping/restoration and show/close behavior. No new HWND, manager, WindowExt wrapper, native request, timer, event stream or diagnostic operation is added. Failure still preserves the existing draft/source/task and uses the same notice/confirmation owner.

## Regression evidence

- `python tests/architecture/overlay-surfaces.py`: three source tests cover all 25 production closure entries, the exact official outer default/refinement order and public Styled storage, plus missing/inner-background mutation cases. The inventory scans Desktop production sources and fails on a newly discovered owner. Three existing test-only dialogs remain outside the production inventory.
- `tests/architecture/overlay_surfaces.rs`: two GPUI TestPlatform regressions mount the official Root and original WindowExt overlays, locate the actual Dialog/Sheet outer bounds, and inspect matching paint-command geometry/color through Light/Dark and Solid → Acrylic → Solid changes while keeping the same open overlay. Removing the public `.bg` override leaves Acrylic's default transparent outer background and makes the outer-paint assertion fail.
- `archive/application.rs:44–46` registers that focused Rust test module under `cfg(test)`; it introduces no production path.
- Source coverage passes 3/3. The new Rust fixture parses and is rustfmt-formatted. Compiled execution is deferred to the parent; filter `overlay_surfaces_` selects the two boundary tests.

## Remaining acceptance

No production overlay owner was found outside the authorized inventory. This source slice does not complete stage four or stage five. Compiled fixture execution, actual GPUI pixels, native Acrylic/fallback, pointer/keyboard/focus lifecycle, DPI and user visual acceptance remain separate. No build, package, native/product launch, user-data operation or system-setting change is part of this result.
