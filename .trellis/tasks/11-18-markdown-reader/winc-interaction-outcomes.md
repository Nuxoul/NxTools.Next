# Winc reader interactions

The reader header has three mutually exclusive views: **代码**, **分栏**, and **预览**. Code and Split share the source editor and undo history. Ordinary Preview is read-only; **就地编辑** enters the existing rich editor without adding a view choice. Navigation keeps the draft and does not write the file. A new document admitted through Explorer starts in Preview.

With at least two nonempty headings, the left outline rail follows the visible section, including the final visible heading at the end. Hover opens the tree after 200 ms without moving focus; the same tree handles explicit open, keyboard navigation and heading jumps. Duplicate labels use separate source locations. Pinning reserves a column where it fits and falls back to the rail in a narrow pane.

Split scroll aligns the source editor's laid-out line to the preview's committed Markdown block and back. Reflected scroll is not sent back to its origin, and content changes retire stale mappings. Alignment is at top-level block boundaries, not at each visual line within a tall table, paragraph or fence.

## Verification

- Desktop reader filter: **16 passed**. Covers actual title-bar mode selection, read-only Preview, explicit rich editing, draft and undo preservation, explicit save, outline hover/focus, pin and narrow fallback, keyboard and pointer jumps to duplicate headings, the end-of-document indicator, and two-way split scroll.
- The Windows DirectWrite TestWindow records rendered glyph colors in all three modes under Light and Dark. It verifies Markdown source colors, Rust keyword/string/comment colors in both rendered views, and readable monochrome fallback for unknown or unlabelled fences. The reader copy-action fixture verifies the original code text.
- Scoped Rust formatting and the spec, GPUI component catalog, locked release and pinned editor source gates passed. No GPUI dependency or vendor change was required.
- Full Desktop unit sweep: **1,986 passed, 26 failed, 1 ignored**. The failures are outside the Markdown reader in Archive, Downloads, Extensions and main-window tests.
- `cargo check --workspace --tests` passed. Strict Desktop Clippy remains blocked by existing code and test lints; it reports no diagnostics in the new viewport or color fixtures, or in the edited mode and presentation modules.

## Limitations

The comparison uses the Winc 0.1.796 source snapshot and NxTools' compiled interactions and paint scenes. It does not certify matching pixels, Windows input/DPI behavior, or Explorer integration on an installed build. No release package or machine default association was changed.
