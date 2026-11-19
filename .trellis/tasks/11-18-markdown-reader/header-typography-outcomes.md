# Reader typography and header

## Result

Source text and preview body use the same reader font request. The default remains 20 DIP and both use 1.5 leading; GPUI rounds final line boxes. A−/A+ affects Code, both panes of Split, and Preview without rebuilding the source entity or saving the document. Headings retain their relative scale and each renderer keeps its existing font family.

Preview is read-only. Editing uses Code or Split. The reader retains the official Folder/Small TabBar and public close slot, with built-in tab scrolling and no additional file-list dropdown beside the filename.

## Verification

- Reader regressions: **23 passed, 0 failed** with `cargo test -p nxtools-desktop --bin nxtools-desktop markdown_reader -- --test-threads=1`.
- Actual header clicks verify source line geometry in Code and Split, Preview paragraph enlargement, source entity identity and unchanged file bytes. Preview typing remains non-editing; its edit action is absent. A single official tab has no redundant dropdown space.
- Existing mode/draft/undo, divider, source-based scrolling, outline, syntax-color, save, export and history regressions pass.
- Debug build and scoped rustfmt pass. Spec navigation, component catalog, GPUI release and pinned-editor source gates pass.
- Same `artifacts/installer/INSTALL.md` in a Windows Debug reader: inspected the equal-size Split capture, enlarged Split capture and read-only Preview capture. Native A+ input increased the source active-row height from 30 to 33 pixels; A− restored 30. In Code, one A+ activation produced a 32-pixel line box.

Local captures are `artifacts/markdown-reader-visual/INSTALL-unified-font.png`, `INSTALL-unified-font-larger.png` and `INSTALL-read-only-preview.png`. The supplied typography and tab-menu screenshots are retained as local reference images in that directory.

## Verification limits

No package, installed executable, Windows association or other Markdown surface was changed. Full Desktop strict Clippy and mixed-DPI/installed acceptance were not rerun in this slice; the previous package-wide lint diagnostics remain recorded separately.
