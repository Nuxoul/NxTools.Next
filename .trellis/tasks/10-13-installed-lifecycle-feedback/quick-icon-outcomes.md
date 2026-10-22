# Quick Find recent-directory icon clipping

## Reference/component review before correction
Quick Launch note recent-directory invariants require a separate right-side history action using same list/controller, only with navigation context/history. Listary managed menu grouping and SwiftList memory history/menu source were already indexed in quick-launch-research.md730-760; no old NxTools launcher surface exists. Complete official Kit index plus gpui-component0.7.1 input/group.rs and button/button.rs were inspected. InputGroup addons are relevant compound; current user-approved standalone Input prefix remains, no input/entity/controller redesign. Released Button::render treats child elements as normal content with custom-size20% horizontal padding; dedicated Button::icon with no label/children uses actual icon-only bounds. Decision: retain official ghost Button and use public icon slot, not padding suppression/custom painted button.

## Executed red
Production collapsed620x44 header with history present:28px action,15px glyph. A temporary cfg(test) canvas measured actual content-mask intersection, yielding8px x15px instead of15px x15px; test1failed1 in0.02s. Earlier exact center failed on0.5px rounding and was corrected to1px tolerance; that was not clipping evidence. First compile constant-name typo and parallel worker intermediate-state compile failure were not product red evidence.

## Intended correction
Use .icon(Icon::new(History).size15) with no child content, preserve28px action/tooltip/accessibility/original callback and input entity/focus/IME. Remove temporary test-only fields/canvas; no dependency patch. Final tests validate public icon slot/hit bounds/header containment/no overlap and pointer action; passing layout tests do not claim actual screenshot or installed DPI acceptance.
