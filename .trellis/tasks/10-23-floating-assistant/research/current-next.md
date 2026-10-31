# Current Next integration research

Read-only source inspection 2026-10-23. No build/product/service/native user operation. Legacy reference is separately recorded in `docs/reference-code/assistant.md`.

## Reusable owners and exact seams
- `rust/apps/nxtools-desktop/src/main_window/tools_page.rs:99-233,497-560`:180x160 ListItem-based catalog cells, keyed reused460x320 settings windows; whole-tile enable and Small outlined Configure have separate dispatchers. `tools_page/currency_settings.rs:14-88,127-210` shows deferred settings publication/observers.
- `ui/components/settings.rs:110-270`:48DIP title/close header, first body activation row, one outer scrolling body. Use its existing compact-tool `Form/Field` pattern for this single-surface tool rather than adding an independent settings navigation system.
- `rust/crates/nxtools-core/src/lib.rs#AppConfig/update_section`: no assistant section. Add a serde-default dedicated section with a locked section update; do not attach an independent desktop window to ClipboardSettings just because previous clipboard-driven tools live there.
- `downloads.rs:921-958,3591-3688`: live `tasks/latest_progress` are generation/sequence-guarded and feed existing taskbar projections. `download_taskbar_overlay.rs:245-288` provides active-state/aggregate precedent. Full/compact engine snapshots in `nxtools-download/src/engine/projection.rs:150-158,258-266` fill `aggregate_metrics` with defaults: do not consume those defaults as speed or fix the download engine in this task.
- `downloads/browser_handoff_impl.rs:404-540#open_new_download_window_with_request`: existing optional typed request prefill/auto-probe; reuse ordinary normal confirmation/window lifecycle. No existing downloader accepts local image bytes/path as a network request. Current `DownloadRequest` accepts HTTP(S), not legacy FTP; preserve current protocol eligibility.
- `quick_launch.rs:269-314#extension_set_query`: safe deferred prefill only for already-open popup. `quick_launch/panel.rs:233-288#prepare_for_show` clears normal summon query; new internal summon-with-initial-query must feed this original owner, not set input before it gets cleared. Preserve existing query generation/IME/target/activation/preview/Escape rules; ordinary hotkey summon remains empty.
- `platform_events.rs:477-531#enqueue_tray_restore/advance_tray_restore`: main visibility+activation acknowledgement precedent. Expose a narrow normal restore entry using the existing lifecycle; register phase pending before dispatch rather than copy an unsafe send-before-register pattern.
- `csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.NotificationWindow.cs:77-94`: pipe-peer PID, HWND ownership and session checks. Apply to the assistant native controller.
- `TaskbarOverlayPlacement.cs:88-109,147-183`: existing layered/tool/noactivate/transparent style and visibility readback precedent. Its interactive mode removes NOACTIVATE; assistant non-click-through mode MUST remain passive, so reuse only the narrow style concept, not that policy.
- Native clipboard image path: `WindowsPlatformAdapter.cs:350-366`, `WindowsPasteCoordinator.cs#Submit`, `WindowsPlatformAdapter.EventWindow.cs:3082-3096`. Reuse copy-only image write and correlated result, never synthesize Paste or mutate history directly.

## Real gaps
1. `vendor/gpui-pre-windows-039/src/window.rs:1144-1285,1516-1527` registers one built-in IDropTarget on every GPUI HWND and supports CF_HDROP only; text/URL/bitmap are not exposed. PlatformHost currently has no incoming IDropTarget implementation. Do not patch the locked framework or install two registrations on one HWND.
2. `Cargo.toml:36` pins image0.25.10 with PNG feature only. Explicit desktop-local GIF/JPEG/BMP/WebP feature enablement on the same version is needed for bounded custom assets. No framework upgrade/new decoder version is required; verify lock delta and codec licenses.
3. No existing assistant animation scheduler, skins, URL/image/text classifier or native incoming-drop broker.
4. New programmatic Quick Launch summon/prefill and source-safe main restoration entry are required, not a second search controller.

## Windows official source check
Fetched and read Microsoft Learn on2026-10-23; scratch HTML only, no product code downloaded:
- https://learn.microsoft.com/en-us/windows/win32/api/ole2/nf-ole2-registerdragdrop : use OleInitialize, one registered target per HWND, calling thread must pump messages including OLE-created windows; missing pump can hang the source application. Already-registered and invalid HWND are typed failures.
- https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setparent : cross-process SetParent with differing DPI-awareness modes can forcibly reset child-process DPI state. Do not casually reparent a foreign GPUI HWND.
Neither document nor existing source proves that a new cross-process drop/input layer works in this product. A bounded self-owned multi-process fixture is an explicit implementation prerequisite, not a claimed passed test.

## Native receiving feasibility contract
Preferred experiment: one C#-owned passive input receiver tied to the renderer's HWND/session, with one C# STA/OLE pump and no second drawing/UI framework. It must not replace the GPUI WndProc, marshal executable callback pointers into another process, modify framework sources, install global mouse hooks or create a screen-sized catcher. Avoid cross-process SetParent. Receiver is exactly scoped to the floating surface, with matched shape/bounds/topmost/passive/visibility and coordinated disposal. If a layered input shim requires a nonzero-alpha hit surface, measure the actual rendering/hit impact and record it; do not describe source flags as proof of transparent input behavior.

Gate before feature integration: self-owned foreign-process renderer+receiver fixture proves real OLE text/URL/HTML-image/bitmap/local-file delivery, no-activate click/drag, exact DPI/bounds sync, drop cancellation, pointer pass-through to a third owned target, alpha/empty-region behavior, hide/disconnect/replacement cleanup and later-command responsiveness. Prototype has no real clipboard/network/config. If the architecture cannot pass without an unapproved vendor/native architecture change, stop and report that exact blocker for a decision; do not ship controls backed by no-op routes or broaden patches.
