# Notification copy button and fork preservation

User approved2026-10-23: add a copy icon immediately left of the existing official notification close button, and upload relevant local GPUI Kit changes to https://github.com/Nuxoul/gpui-kit preserving modification history.

## Requirements / acceptance
- Keep official Notification/NotificationList/ToastStack/card/close/hover/remaining timeout/animation and passive native HWND owners. No global Ctrl+C implementation.
- Optional header action slot in existing Component0.8.0 notification.rs only; actual official XSmall ghost Copy/Check button directly left of original XSmall ghost Close, same toolbar visibility. Long title must not collide; body/actions remain scrollable/reachable at existing native budgets.
- Copy current title + message + up to8 displayed detail label/value rows as plain text. Exclude action labels/internal IDs; preserve full displayed strings. Bounded text <=256KiBUTF8, reject oversized rather than silently truncate. No implicit paste, activation, dismiss, action claim, timer reset or recursive notification.
- Reuse PlatformBridge -> PasteText(paste_after_write=false) -> existing C# WindowsPasteCoordinator. Current notification ID/generation and hidden/closed state checked before deferred dispatch; bounded pending, matching result/timeout/disconnect cleanup. Accepted Windows write cannot be retracted. Button shows busy, success Check (until card replacement/retirement), or retryable error Tooltip; no second notification/lifetime timer.
- Synthetic production pointer/bounds/result/generation/timeout/close/normal expiry tests and source/provenance/strict/workspace/fullDesktop checks. No actual clipboard write/product launch/package/service operation authorized by testing.
- Preserve all unrelated NxTools dirty changes. Do not commit/push NxTools repository. Fork only: retain remote main history, apply minimal relevant component patch(es), document provenance and external backend patch if separate repository, normal fast-forward push (no force), verify remote commit/content. User authorizes this fork publication.

## Scope boundary
Change vendor Component notification.rs for optional generic header action slot; Desktop notifications.rs/new copy module/presentation.rs/main binding/platform_events ingress; new focused tests and reference/provenance docs. No download/archive/floating/keyboard/clipboard-engine rewrite. Fork main starts627ba35afc08ffb49c34affd0af587b2ccf8d836; inspect version before porting, never replace newer whole files with old vendor source.
