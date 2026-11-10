# Stable Quick Launch height while typing

## Scope

Prevent a populated launcher from shrinking for early local/partial results and expanding again when later results arrive. Preserve the approved content-sized52DIP row layout, current results remaining immediately actionable, ranking, default-first/explicit selection, input focus, passive preview and the original native resize owner.

## Behavior

- Ordinary nonempty search may grow immediately when needed.
- Do not contract while input is coalescing or the current index/script providers are unfinished.
- After completion, require the same query/session and target height for180ms before contraction. Re-read live state when the timer fires; later input, a different target, closing or disconnect retires the old intent.
- Clear input, hide and action/recent-directory modes retain immediate geometry changes.
- Completion of a script provider only updates generation-scoped completion accounting and reevaluates existing sizing; result admission and execution remain unchanged.

## Implementation boundary

`quick_launch.rs` owns the contraction intent at `resize_popup_content`, before original SetWindowBounds dispatch. `extension_providers.rs` tracks current running providers and closes accounting at the original invocation terminal. `refresh_tests.rs` and the provider tests exercise the actual typed native resize boundary. No component/layout/palette/vendor/protocol/C# changes. Existing Label/InputGroup/focusable-row APIs,52/32geometry and parent adaptations remain unchanged. Retain accepted/completed resize diagnostics; do not log deferred keystrokes or intermediate sizes.

## Reference

Listary6.3.5.94 SearchMode.cs701–758 waits for the merged query before replacing Results; LauncherSearchWindow uses content sizing. NxTools intentionally publishes current results earlier. Keep that publication contract and defer contraction separately, rather than disabling useful results. Reference index: docs/reference-code/quick-launch-listary-results.md. Constructor/base show/hide/focus and deferred native resize boundaries remain those recorded there.

## Acceptance

Native command regression detects partial1row/old tall viewport contraction; final large results produce no shrink/grow pair. Completed small results contract only after settling. New input cancels the earlier timer; growth remains immediate. Script completion is generation-scoped. Run focused red/green then all Quick Launch tests, strict Desktop lint and applicable lifecycle/provenance/document gates. Installed mixed-DPI and user visual acceptance remain separate.
