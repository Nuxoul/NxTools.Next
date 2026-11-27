# NxTools notification extensions on Component 0.10.0

The approved 2026-11-20 upgrade carries notification duration and optional header-action APIs onto published gpui-component 0.10.0. Archive SHA256: 607c90f6985ac2b96e54102338994eaea143aff521375c6f9e7fc7ecb3ba3646. Apache-2.0 LICENSE-APACHE and original published file hashes in NXTOOLS-UPSTREAM.json are retained. Only src/notification.rs differs in implementation.

## Duration
Optional Notification::duration(Duration) snapshots at admission and forwards to original ToastOptions.timeout. Unconfigured callers retain five seconds; autohide(false) remains persistent; system delivery stays platform-owned. Product range 1–60/default 6 remains application policy. Official NotificationList/ToastManager owns the only clock, hover/focus pause, stack, close and animation lifecycle.

## Header action
Optional Notification::header_action defaults absent and places caller-owned compact content immediately before original Close in the same header. Hover or focus within the enabled header keeps controls visible through keyboard release; optional content is omitted during official Ending. Autohide, body actions and dismissal are unchanged. The 0.10.0 logical inset/spacing, Close accessible name and fully_visible_bounds correction remain. No clipboard transport, global shortcut or second toast controller is introduced.

## Sources and verification
The maintained https://github.com/Nuxoul/gpui-kit fork carries the same notification source on stable v0.10.0. NxTools still consumes published crates plus this path patch, not fork main. Historical Component vendors remain inactive and unchanged. The external Windows patch remains separate.

Run tests/architecture/gpui-release-contracts.py with independent --upstream and enabled --active-tree evidence, plus production duration/header copy/pause/exit regressions. Source and compiled tests are not native, physical, DPI, installed or user visual acceptance.
