# Link Detector Panel Reference Scope

`docs/reference-code/clipboard.md` records verified Next ownership at lines 179-205 (capture/detection/panel/settings chain), 232-266 (single-window preview, transparent shell, native bounds/show requirements), and 253-260 (Windows rounded-root and HWND caveat). Read those sections before implementation. The note confirms old NxTools has no equivalent link-detector popup; this is new Next behavior, so do not invent a legacy implementation.

Current production owners: `rust/apps/nxtools-desktop/src/clipboard.rs::handle_link_detector_capture/show_link_panel`, `tools/link_detector.rs`, `tools/link_panel.rs`, `csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.NativeMethods.cs::SetWindowTopmost/SetWindowBounds`, correlated IPC command/result routing. Rust owns state/layout; GPUI renders; C# owns HWND APIs.

Key invariants: one reusable panel window, final native bounds before reveal, no synchronous parent-entity re-entry, no payload logging, copy-only default/auto-open opt-in, configured auto-hide delay. The current Root paints opaque background; validate actual Windows alpha and hit-testing before claiming rounded transparent corners.
