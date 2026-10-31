# R5 first-show repair — approved continuation

User requested repair after read-only live evidence. Same approved R5 scope, no new UI/framework/product deployment. Reference invariants: old App.axaml.cs29–35 hides silent startup, old MainWindow.axaml.cs2696–2751 reuses same HWND/window across tray hide/show; Avalonia native internals unavailable. Current exact hiddenGPUI placement and live1904x1021DIP firstshow evidence are in r5-live-boot-evidence.md.

## Smallest behavior gap and boundary
Hidden main creation does not apply cachedGPUI initialplacement. Original C# tray/secondarylaunch/floating routes show nativeCW_USEDEFAULT bounds. Prepare the hidden main once BEFORE publishing its HWND/enabling dependent entrypoints. Preserve1060x650logical default and current nativefocus/render/state owners. No per-show resize, no GPUI.activate workaround, no vendor patch.

## Source changes
- Protocol additive PrepareMainWindow(width,heightDIP,window,peerPID) and boundedreadback DTO, tests.
- C# strict parser+authenticatedpipepeer validation, detached boundedSTA prepare. Hidden normal root owned by peer/session, primarymonitor staging then actualtargetDPI, existing NativeMethods.SetWindowBounds compensation/readback or equivalent exact client apply. Revalidate hidden/identity after staging, noSHOW/activation/TOPMOST. Readback returns actualdeviceclient+DPI. Structured accepted/start/terminal/correlation/nativecode, no payloads. Own HWND/fake native fixture registrations explicitly audited.
- Rust existing PlatformEventDispatcher owns startup barrier (no second notificationreceiver/controller). Hidden startup buffers boundedordinarynotifications while exact request is pending, releases Connected/notifications/entrypointpublication only after validACK. Queuefail/disconnect/nativefailure/deadline terminate hiddenstartup; staleACK ignored. Startup callback installs notification/floating attachment and records original HWND hint once. Normal shownstartup unchanged.
- main.rs uses barrier for hidden normal/SilentStart/BrowserDialog/archiveorigin, passing existinglogicalsize. Entryowners unchanged afterready.

## Tests
First run original ownedhiddenCW_USEDEFAULT -> existingnativeShowNoActivate -> actualclientassert1060x650 red. Then same native sequence with exactproductionprepare -> intendedclient verified whilehidden -> separateoriginalshow green; userresize/hide/show retains dimensions. FakeDPI96/120/144/192/negativeorigin/foreignpeer/visible/cancel/mismatch/nativeerror; Rust pendingbeforequeue/ACK/data/stale/disconnect/deadline/notificationbudget/once/callbackdefer. Fulloperationbuildtimeouts, singlejobs/memoryenv, no realuserwindowoperations. Run focused tests/lint/workspace/nativefixtures/static gates; one finalDesktop run, preserveunrelatedfailure. No delegation/package/install/service/commit/push.
