# GPUI Component re-audit after rejected Extensions visuals

## User correction and scope
User rejected the extension-feedback gallery: plugin entries should use recognizable cells/tiles like Other Tools; standalone header actions need visible button boundaries. Existing AI screenshot inspection and747passing Desktop tests are not user visual approval. This audit changes documentation, AI selection rules and a local documentation gate only. It does NOT fix production Extensions layout, alter behavioral source, upgrade dependencies, build/package/install or execute user plugins.

## Live source review
- Live official entry: https://gpui-kit.com/zh-CN/component/ . Browser read the actual version selector **v0.7.3**, enumerated79unique component-page paths (landing page excluded).
- Fetched79/79pages with HTTP200; read per-page purpose, composition, API headings and cautions, then authored per-entry selection guidance. New path relative to old78snapshot is `/zh-CN/component/diff`. No full demo execution, all-method compilation or full interaction review claim.
- Product `Cargo.toml` pins `gpui-kit = "=0.7.1"`. Existing78entry mappings were compared against exact published source-file presence/digests. New Diff module/export is absent in locked0.7.1; marked unavailable, not an instruction to upgrade.
- Page and source SHA256 plus final official URLs and reviewed sections are stored in `docs/reference-code/gpui-kit-component-catalog.json`. Raw fetched HTML/text stays in session scratch `gpui-component-reaudit/`, not the project. Historical Base47/Shell15 inventories were NOT refreshed by this Component-only audit.
- Documentation summaries credit GPUI Kit / Longbridge, CC BY4.0; software/code examples retain Apache-2.0 and third-party rights.

## Verified concrete root causes
1. `rust/apps/nxtools-desktop/src/main_window/extensions_page/workspace.rs::render_workspace` places header Buttons via released Toolbar.child. gpui-component0.7.1 `toolbar.rs::ToolbarItem::sized` calls prepare_for_toolbar then with_size; `button/button.rs::impl Sizable for Button::prepare_for_toolbar` is **self.ghost().compact()**. Thus Medium changes height but does not prevent the parent making buttons quiet/ghost. Earlier height-only review missed the final variant.
2. `main_window/tools_page.rs::render_tools` already composes a180x160cell from ListItem, Icon, Label and outlined Configure Button. The Extensions long text row was a layout choice, not a Kit limitation. User's cell reference does not imply DataTable, nor an imaginary Card/Grid export. Cell dimensions must accommodate real content, not blindly copy180x160.
3. Prior mandatory index was long and mostly availability-oriented; it did not enforce visible variant/parent transformation or compare the requested catalog shape. Preserve detailed source map, add a short mandatory selection contract and a compact full current table instead of another vague promise.

## Delivered local entry points
- `.trellis/spec/shared/ui-component-selection.md`: short mandatory scene-to-compound/variant table, public outlined-action composition candidate, exact Toolbar pitfall, tile target and visual/interaction acceptance boundaries. Under default32768byte injection cap; no context-limit edits.
- `docs/reference-code/gpui-kit-component-table.md`: all79official families, purpose, locked import entry, version status and per-component pitfall. Under32768bytes for a bounded full read.
- `docs/reference-code/gpui-kit-components.html`: standalone offline searchable table, Chinese purpose/category, version filters, source entry/sections and official links. Not a GPUI mock/prototype; no external assets/fetch.
- `docs/reference-code/gpui-kit-component-catalog.json`: same machine-readable dataset and provenance, not intended for unconditional full context injection.
- `docs/reference-code/gpui-kit-component-index.md`: retained locked0.7.1detailed map, clearly labelled historical78count; current audit/rules linked at top.
- `AGENTS.md`, spec root, Desktop owner index, UI authority and reference README link the mandatory short rules/current table. Later tile/outlined-action target explicitly supersedes earlier list-first rule. Prior task notes/outcomes mark user visual rejection; tested runtime/result/trust/cancel behavior is preserved.
- `tests/architecture/gpui-component-catalog.py`: read-only local consistency check; no network/product/native operations.

## Verification and limits
- Live website enumeration versus local catalog:79unique paths, exact set equal; each fetched pageHTTP200and SHA256recorded.
- Local catalog gate PASS; all79Markdownrows equal JSON/viewer data,78locked entry maps, version/Toolbar cautions and mandatory links verified.
- Five scratch negative fixtures rejected: missing entry, viewer JSON drift, disconnected AGENTS link, removed Toolbar hook caution, conflated website/locked versions. Original fixture restored and positive check passed.
- Independent owned headless Chrome, temporary profile:9checks PASS—79rows, Toolbar search, Diff-only unavailable filter, Forms includes Settings, no-match state, clear/focus recovery,79official links, no network resource requests,520pxpage containment with table-only horizontal scroll. Own browser process closed. Repeated after final import-entry update.
- Built-in BrowserPreview was requested, but interaction tool returned session-not-visible; no screenshot/visual inspection claim for the offline viewer. Headless DOM/interaction checks are not visual/GPUI/native acceptance.
- `python tests/architecture/spec-contracts.py`:94documents/13owners PASS. Scoped diff and explicitUTF8/trailing-whitespace checks pass; existing dirty tree preserved.
- No Cargo, product source change, normal application launch, live clipboard/history/credentials/service mutation, real plugin execution, packaging/install, commit/push or global reset.

## Remaining product work
Extensions tile/cell presentation and visibly outlined header actions are still unimplemented. Apply these rules in that bounded visual correction while retaining existing usage/state/result/cancellation/trust functions. Do not describe this audit/table as the fixed product UI or all components visually certified. No docs rule can guarantee future AI compliance; actual reference comparison remains an acceptance gate.
