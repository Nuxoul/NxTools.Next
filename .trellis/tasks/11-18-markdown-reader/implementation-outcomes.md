# Markdown Reader verification

The Windows desktop feature is reachable from Other Tools. The reader owns its document tabs, document-scoped outline and find state, title-bar mode buttons, drafts, export and local versions. Only the chooser and PDF renderer cross PlatformHost. The reader does not modify system file associations or existing passive preview surfaces.

## Verified

- Desktop targeted `markdown_reader`: **8 passed**. Production Tools pointer action exercises open/reuse/reopen. Reader fixtures cover file intake, duplicate tabs, committed-text find, nested/duplicate headings and local image paths, minimum-width edit actions, F2/F3 no-op, real source input and Ctrl+S, exact UTF-8 BOM/CRLF handling, protected external conflicts, offline HTML, failed/superseded export publication, and SQLite reopen/name/restore with the previous committed bytes retained.
- Rust protocol `markdown_picker`: **4 passed**; IPC `human_pickers_outlive_machine_deadlines_and_obey_generation`: **1 passed**.
- PlatformHost: scoped open-picker and export fixture groups passed. Export build and single-file bundle input review verified the pinned managed Core assembly and x64 WebView2 loader. The controlled fixtures did not start a production chooser or generate a PDF through the installed WebView2 Runtime.
- Scoped Rust formatting and diff whitespace checks passed. Trellis context validation and the spec, GPUI catalog, GPUI release and pinned editor source gates passed.
- Full Desktop suite did not produce a green result. Archive tests reported failures; the tree-row layout assertion and missing AC repair wire fixture also reproduced in the pre-reader Desktop test executable. The full suite remains separate from the focused Markdown fixtures.

## Acceptance remaining

- Split preview follows source-editor scroll by mapped Markdown line; reverse preview-to-source scrolling is not implemented.
- Real Windows file dialog, physical file drag/drop and keyboard/IME, mixed DPI, and native PDF text/image inspection need a live acceptance run. No package, installation or installed-product launch was performed.
