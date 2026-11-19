# Design

## Owners
- ElevatedDirectoryNavigationClient retains serialized private requests, caches the latest input configuration, replays after authentication and observes the connected helper's actual process exit. A bounded coalesced configuration worker does not block ordinary settings or hotkeys.
- ElevatedInputHost owns a hidden message window and the existing QuickLaunchKeyboardGesture/LowLevelKeyboardMonitor. Only its message thread queries foreground elevation and forwards released gestures to the authenticated ordinary HWND. Destination ownership is verified against the pipe-server PID and session. Generation and helper PID qualify forwarded messages and hook failure notifications.
- WindowsPasteCoordinator separates clipboard writing from ExecuteInput. IElevatedPasteInput attempts only elevated targets after local write; null means unavailable before dispatch and permits the existing ordinary path. A dispatched request with an unknown result returns failure, never replay.
- WindowsPlatformAdapter supplies the same native input implementation to ordinary and elevated coordinators. Revalidate foreground after settling and modifier release before SendInput. Helper validates live same-session elevated target and focus ancestry before input; no payload enters the helper channel.

## Lifecycle and diagnostics
Existing bounded rendezvous/retry and directory context ownership remain. Configure, paste and directory requests share the serialized channel. Process-exit observation invalidates idle ownership without heartbeat polling. EOF/disposal stops helper input and native hooks. Hook callbacks perform no IO, logging, privilege queries or locks.
Input diagnostics use operation correlation, role, state, elapsed time and native error metadata. No clipboard data, key stream, credentials, window titles or paths. Ordinary keyboard-health protocol remains untouched.

## Reference invariants
Listary LauncherSearchWindowFactory.CreateView → LauncherSearchWindow constructor → SearchWindowBase constructor/method_7 → ShowWindow/base.Show, ActivateWindow/native ForceForegroundWindow, HideWindow/base.Hide confirms the UI remains an ordinary active search window; no rendering/layout change. Listary native hook source is unavailable. SwiftList's released modifier taps and existing NxTools clipboard pre-popup target/focus behavior remain authoritative for the migrated interactions.
