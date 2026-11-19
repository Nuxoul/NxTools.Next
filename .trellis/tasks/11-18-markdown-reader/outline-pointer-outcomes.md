# Outline pointer boundary

## Behavior

Floating and pinned directory containers consume their left-button down event. A heading click continues through the official Tree and the Root's mouse-up cleanup, while the reader applies navigation on the next turn. Pointer movement after release does not start or extend prose selection. Ordinary pressed text dragging and copying remain available.

## Event ownership

The reader window uses Base Root and its window-level TextSelectionLayer. That layer begins selection on bubbling left down, updates an active gesture on pointer movement, and ends it on bubbling mouse up. An outline row must not admit the press to this layer and then suppress its release. The boundary is owned by the reader outline, without a framework patch or a second selection controller.

Reference trace: [Winc and Kit interaction note](../../../docs/reference-code/markdown-reader.md#winc-mode-outline-and-code-color-behavior). Implementation: `outline.rs`; regression: `outline_pointer_tests.rs`.

## Verification

- Reader tests: **24 passed, 0 failed** using `cargo test -p nxtools-desktop --bin nxtools-desktop markdown_reader -- --test-threads=1`.
- The pointer regression creates the same Base Root as the production reader, proves normal paragraph selection, clicks/releases a directory row over prose, moves into prose without a pressed button and asserts unchanged selected text. It repeats in Preview and Split and verifies normal text dragging after navigation.
- Existing keyboard navigation, duplicate-heading jumps, hover/pin lifecycle, source scrolling, font controls, read-only Preview, tab selection, save/export/history tests pass.
- Debug build, scoped rustfmt, spec navigation, component catalog, GPUI release and pinned-editor provenance gates pass.
- The isolated Windows Debug reader was exercised with the current `artifacts/installer/INSTALL.md`: click/release an expanded outline row, then move the physical pointer into prose without pressing a button. The inspected capture `artifacts/markdown-reader-visual/outline-after-click-no-drag.png` shows no drag-selection highlight.

## Verification limits

Native visual evidence covers the isolated Preview window; Preview and Split selection state are asserted in the Root-backed regression. No installed executable, association, package, vendor source or other Markdown surface was changed. Mixed-DPI and full Desktop strict lint were not rerun.
