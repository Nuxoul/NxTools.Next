# Global fullscreen summon suppression

## Goal

Move the Quick Launch-only fullscreen restriction into Global Settings → Application Blacklist and apply it to both Quick Launch and Clipboard panel summons.

## Requirements

- One global setting, labeled `全屏时暂不呼出速启和剪贴板`, replaces the old Quick Launch control.
- When enabled, exclusive D3D and borderless foreground fullscreen both suppress panel summons before publication or activation.
- Ordinary framed/maximized windows, desktop, hidden/minimized/invalid windows and unavailable observations do not independently suppress a summon.
- Clipboard background capture/history and all other background work remain unaffected.
- Existing per-application Clipboard/Quick Launch exclusions remain independent.
- Preserve the previous saved choice when reading legacy files; explicit global values are authoritative.
- Keep the existing atomic global draft/Apply/Cancel transaction, deferred live consumers, panel placement/focus/cancellation and passive-preview lifecycle.
- Diagnostics carry correlation, meaningful decision/state, native error codes and elapsed time without titles, paths or clipboard payload.

## Acceptance

- [x] The Application Blacklist page owns the official setting and the Quick Launch page has no old switch.
- [x] Pointer/keyboard draft changes, Cancel, save failure and successful Apply retain the original transaction boundary.
- [x] Both configure commands carry the same global policy and consumers never resave the global section.
- [x] Native publication fixtures cover Quick Launch hotkey/double-control and Clipboard, both fullscreen modes, disabled policy, ordinary windows, foreground-monitor geometry and native query failure.
- [x] Clipboard background capture remains independent of the panel restriction.
- [x] Core legacy/default/explicit values and empty-rule updates survive load/save without unintended writes.
- [x] Relevant compiled, lint/format and lifecycle/document gates are run with any remaining failures recorded separately.

## Scope

Keep original hotkey grammar/recording, clipboard data, capture exclusions, panel/native lifecycles and assistant hiding policy. No packaging or installed-game interaction is included in this source slice.
