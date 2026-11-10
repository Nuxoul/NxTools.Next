# Quick Launch search proportions and query-bound geometry

## Requested outcome

User rejects the shrink-delay effect and asks to follow Listary's handling and enlarge the search header to match52DIP result rows.

## Reference

Listary6.3.5.94 LauncherSearchWindow.xaml448 has34DIP input,26DIP font and12DIP top/bottom margins:58DIP header. SearchMode.cs701–758 retains committed Results until all providers finish then publishes once. Launcher SizeToContent follows that completed collection. Prior reference note: docs/reference-code/quick-launch-listary-results.md.

## Change boundary

- Existing native resize dispatcher retains the established nonempty-query viewport for all intermediate batches, not just contractions. Final completion commits current geometry immediately, with no180ms timer.
- Preserve early actionable results within that viewport. First current rows may reveal the initially collapsed viewport; no results yet keeps the search-only shell. Clear/hide/action/recent modes and explicit errors keep their direct sizing paths.
- Search header58DIP; official Input Large44DIP control,26DIP text/34DIP line,24DIP search icon. Original single input frame, InputState/IME/focus, horizontal4DIP inset and28DIP recent action remain. The44DIP official control includes its own padding/border; no substitute custom input or vendor patch.
- Preserve52DIP rows/32DIP result icons, ranking, current-query action guards and all other surfaces. Remove obsolete shrink timer/state, retain original correlated SetWindowBounds/SwpNoActivate and extension completion signal.

## Component selection

Kit0.8.0 Input::with_size(Size::Large) supplies the official44DIP input tier. Public Styled text size/line height and4DIP vertical padding refinements apply after defaults;34DIP text plus8DIP padding and2DIP border fits the control. Custom Size falls back to24DIP in this released input_h implementation. Input::h() is an inherent multiline-height API and is not used for single-line sizing. No Toolbar parent changes; no new stateful Command/List controller.

## Verification

Red-capable native resize tests reject intermediate growth and require immediate final contraction; production render test asserts58header/44input/24icon and original input identity/focus. Run all Quick Launch regressions, strict Desktop lint, workspace test check and applicable source/document gates. Capture a startup-isolated Debug synthetic layout if available. Installed/native/physical/user visual acceptance remains separate from compiled or synthetic evidence.
