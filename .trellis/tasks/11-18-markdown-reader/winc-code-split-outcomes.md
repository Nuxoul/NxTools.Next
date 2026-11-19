# Code, Split and official file tabs

## Result

- Code uses the existing Kit Editor without a border or rounded input frame, with 12-DIP source text and 18-DIP line height. Kit retains line numbers, selection, highlighting and input ownership.
- Split starts with equal panes. Dragging changes a reader-local proportion; resizing and mode changes retain it. Double-clicking the official ResizeHandle resets the panes. Both panes have a 240-DIP minimum, excluding the one-DIP divider; narrower workspaces show only source without changing the selected mode or losing its draft.
- The rendered pane receives its actual width and remains read-only without a source gutter. Standalone Preview typography and article layout are unchanged.
- Official Folder TabBar/Tab owns file-tab shape, hover, selection, truncation and overflow. The public suffix contains the named official close Button; the original deferred confirmation and deduplication owner remain. Unsaved tabs carry a label marker.

Reference: `D:\Code\Winc-dev\crates\winc-app\src\app\lifecycle\window.rs::place_split`, `app/input/mouse/{click,drag}.rs`, `view/editor/{area,picture}.rs` and `theme.rs`. Exact entry points and public component seams are recorded in [the reference note](../../../docs/reference-code/markdown-reader.md#winc-source-and-split-layout).

## Verification

| Check | Result |
| --- | --- |
| `cargo test -p nxtools-desktop --bin nxtools-desktop markdown_reader -- --test-threads=1` | 20 passed, 0 failed |
| `cargo build -p nxtools-desktop --bin nxtools-desktop` | Debug build passed |
| Scoped rustfmt check | Passed |
| `spec-contracts.py`, `gpui-component-catalog.py`, `gpui-release-contracts.py`, `markdown-editor-contracts.py` | Passed |
| Actual reader-window regression | Equal initial split, press/redraw/drag, proportion on resize and mode return, double-click reset, both minimums, narrow fallback, unchanged source entity/selection/undo and no implicit save passed |
| Actual tab-row regression | Two files switch through the official tabs; closing the clean file retains the other file's draft |
| Existing reader regressions | Three modes, preview read-only, bidirectional source-based scrolling, outline lifecycle, Light/Dark syntax paint, save conflict, HTML and history passed |
| Windows Debug captures | Same `artifacts/installer/INSTALL.md`, Code and Split in Light and Dark; captured 2576×1416 window images and inspected 2560×770 content/header crops |
| Windows divider interaction | In the Light Debug window, native drag moved the captured seam from column 1288 to 1527 for 239 pixels of input. A double-click returned it to column 1288 |

Local captures: `artifacts/markdown-reader-visual/INSTALL-{dark,light}-{code,split}.png`; the additional Light drag capture records the resized pane. These are inspected local UI evidence, not approval of Winc parser/pixel equivalence.

Rust invocations used one build job, disabled incremental compilation and 256 codegen units. No vendor or installed executable was changed.

## Known verification limits

Strict Desktop Clippy remains blocked by five existing diagnostics: `main_window/about/emblem.rs` lazy `then`, `main_window/extensions_page/activity.rs` enum size, and pre-existing reader `history.rs`, `settings.rs` and `mod.rs` expressions. No diagnostic was reported for the new split module. No full Desktop suite, mixed-DPI, physical-device input, installed release, packaging or Explorer association acceptance was performed in this slice.
