# Markdown outline heading interaction

## Behavior gap and original boundary
The reader's heading ListItem performs its own deferred source-anchor navigation, then the same click bubbles into the official Base Tree entry. Base0.8.0 tree.rs::on_entry_click selects and toggles folder expansion. Nested headings are TreeItem children, so a parent heading click collapses the following chapter rows. Winc app/preview/markdown/scroll.rs::press_toc152-173 handles Pin or Heading and maps a heading to scroll/active/flash only, without folding descendants.

## Component and retained owners
Keep released Component0.8.0 Tree/ListItem and Base TreeState, existing20DIP rows, source anchors, active selection and deferred Reader::jump_outline. Preserve Tree keyboard Left/Right expansion, Up/Down/Enter navigation, hover/pin/fade, Root's text-selection release and all document/tab/save state. No dependency/vendor changes, replacement tree or navigation controller.

## Files and regression
Extend outline_pointer_tests.rs at the production Root boundary: repeated parent pointer clicks retain child entries/visible bounds in Preview and Split, alongside existing no-stuck-selection and body-drag assertions. Change only the outline row's pointer boundary in outline.rs after observing the red test. Intercept at down rather than suppressing the release needed by original Root/Tree; keep selection through the existing reader navigation owner. Reference note records exact released call chain before production change.
