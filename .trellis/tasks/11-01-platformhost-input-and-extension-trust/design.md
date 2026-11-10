# Design

## Ownership and minimal gaps
The serial PlatformHost reader must remain independent of slow Shell/service work. Existing detached task tracking/drain and the install gate own H5; no new scheduler. Process disposal closes the returned handle, not the launched application.

The hook must not depend on the EventWindow STA that captures/writes the clipboard. LowLevelKeyboardMonitor owns its background message-loop installation and cleanup. The hook thread exclusively owns both modifier double-tap detectors. EventWindow publishes an immutable enabled/recording/trigger/reset-generation snapshot; hook callbacks consume it and post the original gesture message. The native API/message loop requires an isolated test seam without a live global hook by default.

Observable hook health remains a native fact and crosses one typed event into the existing Rust QuickLaunchApplication. Hello capabilities restore state on reconnect. ConfigureQuickLaunch remains the only apply/re-enable command. Record compatibility after inspecting real unknown-event decoding; no event-name guessing or blanket version change. Health diagnostics carry correlation, fixed reason, elapsed/native code only, off the hook callback.

The settings retain their current layout, official Kit0.8.0 controls, draft/save ownership and focus/native lifecycle. Add an official Alert and outlined re-enable action at the existing summon-method row. Cross-entity callbacks defer; pending identities precede dispatch. No new panel/banner/notification manager.

Content-script trusted-input checks are early no-op exits before any download side effect. Browser replay intentionally uses synthetic anchor.click and must still take the existing pass-through/native path. Page-context Blob hooks and drop-target on/off product policy are unchanged. Tests preserve current poster/thumbnail behavior.

## Change boundary
Expected owners: PlatformCommandSequenceDispatcher, WindowsPlatformAdapter/LowLevelKeyboardMonitor/EventWindow and narrow native DTO/event support; protocol/IPC and Quick Launch settings/application; content-script.js and its existing tests/README. Extend only owning specs/reference notes and this task record. Preserve unrelated changes and avoid broad refactors.

## References and invariants
Inspect Quick Launch reference index, Listary managed boundary/SwiftList and legacy NxTools before migrated interactions. Record newly verified call chains in the reference note before implementation. Existing panel generation, foreground capture, focused show/hide, recording/reset semantics and original WmQuickLaunchGesture handoff remain. Listary native hook implementation is unavailable and is not claimed as evidence.
