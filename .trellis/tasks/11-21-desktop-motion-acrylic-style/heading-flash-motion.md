# Markdown heading flash motion

## Stage and decision

This bounded slice belongs to **阶段四：全软件表面与动效接入** in
[the implementation plan](implement.md). The [approved motion grammar](design.md#motion-grammar)
and [reader reference](../../../docs/reference-code/markdown-reader.md) retain immediate navigation
and active-heading feedback. Reduced or unknown motion omits the decorative heading flash,
showing the final navigated result directly. Normal motion retains the existing 1200 ms linear
accent fade from 0.14 to zero. There is no static-highlight timeout, zero-duration animation,
new clock, native operation or presentation manager.

## Verified source chain

- The reader reference records that no independent reader was found in legacy `D:\Code\NxTools`;
  mdview's proprietary native reading internals are unavailable. The source-backed heading
  behavior reference is Winc, not reconstructed mdview code.
- Read-only Winc source: `D:\Code\Winc-dev\crates\winc-app\src\app\preview\markdown\scroll.rs`
  `press_toc` lines 152–173 keeps Pin separate, glides to the heading, sets `toc_picked`, then
  assigns `toc_flash` only when `App::clock(TOC_FLASH)` returns a clock.
- Winc `app/animation.rs::clock` lines 325–344 returns None when animation is disabled;
  `settle_animations` lines 347–380 retires finished heading flash. `TOC_FLASH` is 1200 ms.
  `view/preview/markdown.rs` lines 904–916 paints the measured heading band at
  `0.14 * (1.0 - clock.linear())`.
- NxTools `tools/markdown_reader/mod.rs::show_window` creates Reader through `Reader::new`,
  wraps it in the original Base Root and retains the original native close callback.
  `material::prepare/present` remains the existing show barrier; this slice does not change
  window creation, material, show/hide, HWND placement or focus.
- `outline.rs::outline_tree` pointer and keyboard paths defer `Reader::jump_outline` with the
  original tab ID and anchor. `viewport.rs::jump_outline` validates the active tab and committed
  projection, updates `picked`/`active_heading`, maps the source anchor to a block and scrolls
  Preview and, in Split, Source. `mod.rs::render_document` calls `heading_flash` above the same
  TextView; its canvas uses committed revision, list item bounds and viewport clipping.
- `ui/motion.rs::{motion_allowed, reduced_motion, UiMotionPreference}` is the policy owner:
  absent/unknown/native-error/disconnect facts reduce motion. Each tab's Navigation retains one
  Reader-local subscription after its first valid jump. Reduction retires that tab's flash even
  while inactive or in Code mode, then defers its weak-view repaint. Closing the tab drops the
  subscription. The paint boundary reads current policy before painting or requesting a frame.

## Preserved invariants

The original Reader, tabs, document/source entities, rendered revision, committed projection,
source anchors, scroll owners and outline selection stay intact. Navigation is never delayed
by feedback. Pointer release, Tree keyboard navigation, passive text selection, hover dwell,
outline cancellation and native material/visibility/focus owners are unchanged. Reduced motion
clears an older flash and cannot resume it when animation is enabled again; only a fresh valid
jump can arm another normal flash. Expired or revision-stale paint cleanup is deferred and
qualified by tab ID and the exact flash tuple so it cannot retire a newer navigation.

## Component/API boundary

No layout/control composition, variant, size or parent adaptation changes. The existing
Base0.10.0 `TextView`/`TextViewState`, official Tree/ListItem and XSmall ghost pin Button remain.
The existing canvas is the source-geometry decoration, not a substitute ordinary control.
Kit/default GPUI remains 0.10.0/pre0.3.12; no vendor edits. Public GPUI APIs used are
`Context::observe_global`, `App::defer`, `Window::defer`,
`Window::paint_quad` and the original bounded `Window::request_animation_frame`.
This local presentation-policy correction adds no cross-process operation or per-frame logging;
existing reader and system-preference diagnostics remain the owning event boundaries.

## Regression boundary and verification

The cfg(test)-only `heading_flash_motion_tests.rs` contains four real Reader/Base Root
regressions. They use synthetic temporary documents and the test clock, inspect the actual
canvas quad via `Window::painted_quads`, and inspect frame demand
via `Window::simulate_next_frame`. Official component motion stays reduced in the fixture;
the independently injected shared UiMotionPreference controls only the reader's flash:

- Reduced and unknown preference: Preview/Split navigate with the same view/revision and active
  heading, without flash or decorative frames; enabling motion cannot replay that jump.
- Normal: alpha 0.14 at admission, 0.07 at 600 ms, absent at 1200 ms, then no flash frame demand;
  ordinary repaint does not rearm.
- Mid-flash reduction: clear paint/frame demand immediately, no resume within the old lifetime,
  and a fresh jump can flash after motion is enabled again.
- A background tab's flash is retired on reduction and cannot resume when that tab is restored.

Scoped rustfmt and diff whitespace checks pass. The GPUI release source gate passes with the
approved Component/Base/core/Windows 1/5/2/4 union; the pinned-editor source gate passes, and
spec-contracts passes with 132 documents / 14 owners. These are source checks, not compiled
heading-flash behavior evidence.

Compiled execution is reserved for the parent after concurrent source owners are stable.
Native/product launches, real system-setting changes, user data and visual/user acceptance are
outside this slice's verification. Stage four remains in progress.
