# Global fullscreen summon policy

## Ownership and flow

Core owns `ProcessBlacklistSettings.suppress_in_fullscreen`. Its optional read form distinguishes old files and rule-only updates from explicit false. `AppConfig::normalize` resolves a missing value from the old Quick Launch choice; load never writes. Global Settings owns the draft and the existing locked network/system/blacklist save. Rule-only setters preserve the current policy, including when deleting every process entry. Explicit global values take precedence over the old field.

After durable success, `publish_live_blacklist` defers the same snapshot to Clipboard and Quick Launch. Both original configure commands carry the effective boolean, including reconnect and subsequent feature-specific configuration. Clipboard adds `suppress_in_fullscreen`; Quick Launch retains its existing compatible wire name. Correlated sends preserve the global operation ID and independent command receipt IDs. No feature consumer writes the global section.

C# owns the independent per-feature native policy snapshots and the shared pre-publication gate. Exclusive D3D is recognized by the existing Shell API. Borderless classification uses a visible valid non-desktop foreground root without ordinary frame styles, covering its own monitor's complete device-pixel bounds. The pure existing assistant predicate is reused without changing assistant lifecycle or monitor choice. A failed observation does not independently block a summon; a successful independent observation can still identify fullscreen. Detection stays outside the low-level keyboard hook.

## UI composition

Keep the existing 720×480 global Settings window, original Apply/Cancel footer and Application Blacklist page. Remove the old Quick Launch control/setter. The shared blacklist renderer mounts the official Switch in a compact 24 DIP row with an official Label and Tooltip, then retains the original table/scroll owner and 120 DIP row viewport; this keeps the last rule and footer reachable at the approved minimum. No vendor changes.

## Lifecycle invariants

- Suppressed summons publish no panel event and therefore perform no create/show/activate/focus/preview operation.
- Allowed summons retain the original target snapshot, debounce, recording, panel reuse and hide/restore sequence.
- Clipboard capture/history continue independently of panel suppression and existing per-app exclusions remain unchanged.
- Cancel and persistence failure do not publish the draft. Consumer effects run on a later GPUI turn, never synchronously back into a leased entity.
- Quick Launch enablement is not a condition for Clipboard suppression.

## Diagnostics and regression seams

Existing bounded diagnostics carry correlation, trigger, fullscreen kind/reason, publication result, native error/HRESULT and elapsed time; repeated observation failures are rate-limited. Titles, paths and clipboard content are excluded. Tests exercise actual native message publication, both configure paths and policy independence, Core temporary-file load/save/rule changes, actual GPUI pointer/keyboard/Cancel/Apply, minimum-window table/footer geometry and deferred consumer updates.

## Acceptance limits

Fixture and TestWindow evidence is separate from physical hotkey behavior in a real exclusive/borderless game, mixed-DPI interaction and user visual acceptance. No package, service change or product launch belongs to this slice.
