# Picker input interception

## Report and authorization

User reports local physical mouse/keyboard: some target windows receive left-click interaction instead of being selected, and Escape does not cancel. User explicitly authorizes one isolated real Win32 hook test with self-owned test windows and marked left/Escape input, no clicks into user applications/settings, and final hook/cursor/focus cleanup. This does not authorize product launch/replacement, arbitrary global-input capture, Registry/service mutation or packaging.

## Boundary

Repair only the existing C# ProcessWindowPickNative raw HookProc -> ProcessWindowPickController input/terminal -> original coordinator message-loop path. Establish red-capable raw callback regressions, not only HandleMouse/HandleEscape controller tests. Verify real SetWindowsHookEx/message-loop/interception with a separately classified native probe that limits all generated input to its owned HWNDs. Preserve source hide/restore, original correlated protocol, paired release suppression, cancellation/deadline/disconnect and original Rust refill/Add/Apply owners.

Existing callback flags discard all injected input despite picker not injecting/replaying input. Input eligibility currently causes invalid-target clicks to pass through; window facts are queried synchronously inside the mouse callback. These are candidate defects to prove at their actual boundary, not a declaration that all reported physical-input failure has one cause. No fallback poll pretending to consume an event that already reached another app, no parallel interaction framework or broad keyboard lock. Define low-noise terminal input counts to distinguish delivered/swallowed input from an installed handle with no delivered events.

All work stays C#/tests/docs unless concrete evidence requires a narrower cross-layer change. No Cargo required. Preserve existing changes and untracked gpui-kit task. No commit/package/install. Native validation must use exact reviewed native selection and require the current explicit authorization; injected regressions remain separately classified.
