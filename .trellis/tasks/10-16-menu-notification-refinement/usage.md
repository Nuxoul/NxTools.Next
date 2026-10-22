# Current source usage

## Official notification integration (2026-10-17, not packaged)

- Notification cards now use released GPUI Kit0.7.1 **Notification / NotificationList / ToastStack**, including the official card, stack, close and enter/exit behavior; no custom GroupBox/Alert replacement or second expiry timer.
- Ordinary notifications use the official **5 seconds**. Hovering/focusing the stack pauses the group's remaining time; leaving/blur resumes it. Close follows official hover-only presentation. Progress remains persistent until updated/dismissed.
- Global Settings -> General shows **5 秒（官方通知默认）** as read-only information. The legacy duration field remains compatible and unchanged by unrelated saves, but does not control official timing.
- Different operation invocations produce separate notices, up to3retained business records. Only the same invocation's progress/result updates replace its own ID. Extensions enable/disable/save, Clipboard operations and Downloads/Quick Find outcomes use invocation-scoped IDs.
- Global Settings -> **通知位置** selects six positions relative to the source monitor's screen work area. Apply uses the original atomic transaction; Cancel/failed save does not change position. Changing position does not restart an existing notice's countdown.
- One small reusable passive popup grows for official pointer/keyboard stack expansion and shrinks after collapse; no fullscreen catcher or permanent expanded transparent input region. Updates resize in place without hide/reveal flicker. Original-source, current-generation actions dispatch once after leases; no automatic copy/paste/replay/reopen/trust/enable.
- Extensions busy keeps catalog/search mounted. Transient feedback uses the shared screen center; actual validation/trust/durable task/query state and explicit user-opened reports stay contextual. Clipboard processing remains inside its immutable source item, with explicit host-confirmed result navigation.
- Clipboard right-click uses the official PopupMenu in an independent correctly sized popup; hidden scene is explicitly drawn before deferred reveal/activation. Downloads selected summary/actions remain centered with clear at right. **打开插件目录** opens the managed config-adjacent extensions directory through the existing Shell boundary, without plugin execution or state changes.

[Implementation and exact evidence](official-notification-integration-outcomes.md) distinguish compiled, owned HWND, generated capture and user-installed acceptance. Physical pointer/keyboard, mixed-DPI, reader and visual acceptance remain outstanding.

## Package boundary

The latest delivered installer is still **sequence15** ([historical package](release-package-sequence15.md)); it contains the prior custom notification implementation, not this official integration. Its historical configurable1..60/default6 behavior is superseded in current source only. This task did not package, install, start the normal product, change services/user data/plugins, commit or push.
