# Desktop UI Redesign Reference Scope

The user's provided GPUI specification in the parent PRD is canonical for colors, spacing, radius, typography, focus, keyboard parity and motion. Do not substitute the current theme palette for it.

Before visually reshaping existing product interactions, read their matching `docs/reference-code/` notes first and inspect the recorded old NxTools files down through base classes and native show/hide/action boundaries. In particular:
- downloads: `docs/reference-code/download.md` (download child reference scope lists legacy window files);
- clipboard/popup: `docs/reference-code/clipboard.md` (popup ownership, no re-entry, passive preview/native bounds);
- GPUI lifecycle/motion: `docs/reference-code/gpui.md`.

Current desktop modules are Downloads, Quick Find, Clipboard and Tools plus shared titlebar/sidebar, global network settings, notifications, tool settings windows, task dialogs, preview and popup surfaces. Preserve each stage's existing behavior. Rust owns state/layout, GPUI renders, C# retains Windows APIs; design changes do not authorize behavior or protocol changes.
