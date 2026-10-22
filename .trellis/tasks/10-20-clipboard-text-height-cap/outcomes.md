# Clipboard text-card maximum outcome

## Implemented boundary
- User authorizes text card maximum210DIP or a setting. Implemented the direct210DIP alternative, without adding settings/config/schema scope. `TEXT_CARD_MAX_HEIGHT` applies to the whole official Kit0.7.1 ListItem, including source/header/border/padding and any inline extension feedback. External8DIPgap is separate, so the text slot is at most218DIP.
- Short/under-budget text retains its natural official Markdown height and compact62DIPminimum. Ordinary body allowance is172DIP after existing10verticalchrome+24metadata+4gap. Existing76DIPfeedback allowance is reserved before sizing body, preserving feedback buttons inside210. Header and body are nonshrinking; only over-budget list summary clips. No new nested scroll layer, fade, payload truncation or resource load.
- Original full passive preview/native ACK/measurement/scroll/footer, complete-by-ID paste, original actions/menu/keyboard/paging/source-icon hue remain unchanged. Ordinary image media cap144DIP/card190DIP and file card62DIP remain unchanged, excluding8DIPgap/optional feedback.
- Product code change is confined to `ui/history.rs`; tests in `ui/efficiency_tests.rs` and the existing `extensions/manual_actions.rs` feedback fixture cover actual boundaries. No dependency/vendor/C#/protocol/native/data/config changes. Existing operation diagnostics remain; no new callback/operation or per-frame logging. No delegation/contact, package/install/product/service/live-user-data operations, commit or task archive.

## Evidence actually run
Scratch logs use prefix `clipboard-text-cap-` in the current session scratch directory.
- `red.log`: valid production bounds red,1testfailed; long Markdown actual official ListItem was790DIP at280DIPwide/Dark, exceeding210. Test was added before product change.
- `green.log`: original cap test1/1passed. Actual official ListItem/slot/header checked at280/460DIPwidth and Light/Dark: long card210, short card<120, slot<=218, source header24, externalgap8.
- `panel.log`: **30/30passed**, including whole-card cap, retained image/file heights, clipped-summary click -> exact original full text with tail, under-budget natural growth/reflow, typed action/target/lifecycle, source-icon hue, gap/noaction, pause and shortcuts.
- `history.log`: **16/16passed**; existing row action/management/focus/hover/menu/image/naturalheight owners retained.
- `preview.log`: **17/17passed**; original full Markdown reaches last block inside real scroll viewport, footer remains visible, source bytes retained and resource request count0. No preview code was changed for the card cap.
- `feedback-final.log`: **1/1passed**. Existing actual feedback source fixture now first uses long text; Running/Cancelling/Succeeded cards210 and all Cancel/ViewResult/Next/Details/Dismiss controls inside source card. Then restores original short three-row geometry and runs original reorder/filter/cancel/dismiss/motion/window lifecycle.
- `clippy.log`: strict Desktop `cargo clippy -p nxtools-desktop --tests --locked --offline -- -D warnings` PASS.
- `workspace.log`: `cargo check --workspace --tests --locked --offline` PASS.
- `desktop.log`: one final full Desktop run **933passed/0failed/0ignored**,104.34seconds.
- `static.log`: Clipboard lifecycle/UI lifecycle/UIstate gates PASS using explicit UTF8 source execution to preserve Chinese literals/PSScriptRoot; spec106documents/14owners and catalog79pages/78lockedentries PASS. Scoped rustfmt/whitespace and Cargo.lock equality against saved dirty baseline PASS.
- Reviewed task-owned diffs against saved baseline; no unrelated formatting/refactor/product changes. Manual-actions changes are test-only.

## Test adjustments and limits
- Initial panel27pass/1fail exposed the previous requirement that all long links fit unbounded in a list card. Updated only that assertion to the explicit new contract: complete within budget, clipped only when actual slot reaches218; rendered text and real width reflow still asserted.
- First long-feedback fixture passed210/button containment but failed a later direct dismiss click after moving the now-tall source to the third row, outside viewport. Restored original short fixture/order after cap-specific checks; original lifecycle then passed, with no extension product repair or weakened assertion.
- Source/compiled/static work is complete and no remaining verification blocker was observed. Tests are not a running-product screenshot or user visual approval. No package/install/product launch, real HWND/physical pointer/keyboard/mixed-DPI/reader/installed/native/release acceptance was performed. The installed package is not claimed to include these source changes.
