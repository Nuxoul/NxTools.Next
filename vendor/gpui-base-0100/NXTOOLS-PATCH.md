# NxTools Base 0.10.0 editor-host compatibility

Source: published `gpui-base` 0.10.0 (Apache-2.0). `NXTOOLS-UPSTREAM.json` records the original archive and per-file identities. Ordinary TextView rendering and whitespace normalization retain the published behavior.

The approved editor-host boundary permits exactly five implementation files, in this modified_files order:
- `src/text/editor_host.rs`: editable Markdown imports use a dedicated NodeContext with preserve_editor_whitespace=true. LeafGeometry exposes frame-local `table_cell_bounds()` for actual column slack/dividers and row frames, and `table_cell_content_bounds()` for content regions resolved from final Div style padding/border. Text position/index already reflect actual aligned positions. Includes whitespace isolation coverage.
- `src/text/editor_seam.rs`: internal cell-part and row reports use the existing scoped sink.
- `src/text/format/markdown.rs`: the editor import context bypasses ordinary whitespace folding/boundary trimming; soft-break handling and normal TextView paths remain unchanged.
- `src/text/node.rs`: private editable-import context and transparent table bounds observers. Host-keyed cells report existing lead/content/trail layout and row boxes; non-host tables keep their original elements. No layout, style, selection or input handler is replaced.
- `src/text/state.rs`: only the single mechanical preserve_editor_whitespace:false field in its complete NodeContext initializer; ordinary TextView retains published behavior.

Original published-file metadata and hashes remain fixed for all files; independent --upstream comparison is retained. Component/Windows/Base are confined to the1/1/5-file scope. The fixed editor consumes renderer-owned grid/content boxes rather than reconstructing them from aligned text widths; its text-box vertical bounds remain. Its original round-trip, cell-hit, grip, resize and scroll fixtures/assertions remain the behavioral acceptance boundary. The original d20 and full editor regressions pass; Base editor-host tests include the whitespace-isolation boundary. Native/physical/visual acceptance remains separate. No new diagnostic operation, native policy or runtime backend is introduced.
