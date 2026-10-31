# NxTools notification extensions on Component0.8.0

User-authorized2026-10-21 upgrade carries the original2026-10-20duration patch to published gpui-component0.8.0. User2026-10-23 additionally authorizes a compact header copy-button slot. Archive SHA2564097e161cc81e86323b439664be0cdd4f7c56148e867ed6f67fd72b966b6c2d1; Apache-2.0 LICENSE-APACHE retained. NXTOOLS-UPSTREAM.json preserves original published file hashes. Only src/notification.rs differs; no registry edits/version upgrade/backend switch.

## Duration
Optional duration field/public duration(Duration) builder snapshots at admission and forwards to original ToastOptions.timeout. Raw unconfigured callers retain5seconds; autohidefalse remains persistent and system delivery stays platform-owned. Product1–60/default6 policy is not part of the generic component. Official NotificationList/ToastManager retains sole countdown/hover/focus/pause/stack/close/animation ownership.

## Optional header action
`Notification::header_action(builder -> AnyElement)` defaults absent and adds caller-owned compact control immediately left of existing close in the same top-right toolbar. Hover or focus within the enabled header keeps it visible; this prevents GPUI keyboard-modality hover suppression removing controls before Enter/Space release. Optional content is omitted during official Ending; it does not change autohide/body actions/dismiss. Caller owns business/current-generation/result feedback and content reservation. No clipboard/global-key hook, extra clock/card/palette or private lifecycle controller.

## Preservation
The user-authorized fork is https://github.com/Nuxoul/gpui-kit. Generic duration/header patches are ported as focused commits onto its current main, preserving upstream fully_visible_bounds correction; local locked0.8.0 source remains based on published release. External gpui-pre-windows patch is only separately documented/archived, not activated by the fork. Task `.trellis/tasks/10-23-notification-copy-button/` records final remote commits and verification.

Run python tests/architecture/gpui-release-contracts.py (optional --upstream for verified published sources), notification production regression suite, strict/workspace/fmt and relevant source gates. Source/compiled tests are not physical/native/installed/user visual acceptance.
