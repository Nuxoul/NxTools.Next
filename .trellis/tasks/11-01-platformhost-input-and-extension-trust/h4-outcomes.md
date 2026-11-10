# H4/U3 native keyboard ownership

## Root cause and implementation

WH_KEYBOARD_LL was installed by EventWindow's constructor on the clipboard/paste STA. Delayed clipboard rendering, image capture/encoding, clipboard retries and paste sleeps therefore delayed the hook's installing message loop. A nonzero HHOOK only recorded installation, not current Windows delivery.

`LowLevelKeyboardMonitor` owns a dedicated background `NxTools.KeyboardHook` thread. `WindowsKeyboardHookPlatform.InitializeThread` creates its message queue before exposing the thread ID; installation, blocking `GetMessage`/dispatch and final uninstallation all run on that owner. Dispose requests `WM_QUIT` with `PostThreadMessage` and joins within two seconds. A prior thread must actually exit before a retry starts another. The callback delegate remains rooted through native cleanup.

`QuickLaunchKeyboardGesture` owns both original 60–800 ms detectors. Only hook callbacks mutate them. EventWindow publishes immutable enabled/recording/trigger/reset-generation configuration through Volatile; each down/up callback consumes one snapshot and applies a changed reset generation itself. The original injected flags remain filtered. The callback only translates input, updates detectors and posts the existing gesture to the captured EventWindow HWND. Foreground capture, fullscreen gate, panel placement, IPC and focus remain under their existing owners.

`ApplyQuickLaunchConfiguration` passes its command correlation to `TryInstall` before rejecting unavailable double Ctrl. Configuration/recording branches publish their final snapshot. An already installed monitor is idempotent.

## Observable lifecycle and protocol

Typed C# health state is `QuickLaunchKeyboardHealthChangedEvent`, mapped by `PlatformEventEnvelopeFactory` to `quick_launch_keyboard_health_changed` with `correlation_id`, `state` and `native_error_code`. States are `install_failed`, `thread_exited` and `reinstalled`. Initial installation success is recorded in diagnostics and Hello availability; intentional shutdown is not an unexpected exit.

Outer protocol remains 1. The new `quick_launch_keyboard_health_v1` capability is explicitly accepted in HelloAck before `ForwardEventsAsync` may forward the event. Old Rust receivers reject unknown platform-event variants and end that session, so unnegotiated health events are not sent. Existing event forwarding is unchanged. `quick_launch_panel_hotkey` continues to describe current availability in a reconnect Hello.

Diagnostics use the installation/configuration correlation, elapsed time and native error code only: install_started/installed/install_failed/thread_exited/reinstalled, plus stop_deferred when the bounded join cannot establish physical exit. No callback logs, key contents, heartbeat, idle probe or silent-removal inference is added.

## ReleaseActivationModifiers finding

`EventWindow.DrainPasteRequests` invokes `WindowsPasteCoordinator.Execute`. Its `IWindowsPasteOperations.ReleaseActivationModifiers` implementation returns to `WindowsPlatformAdapter.NativeMethods.ReleaseActivationModifiers`: SendInput key-up followed by 3 ms sleeps, up to 12 attempts, without pumping EventWindow messages. Injected-event filtering occurs only after hook delivery, so it did not remove the old shared-thread dependency. The dedicated loop removes that self-wait dependency. This structural result does not establish physical modifier release or real paste acceptance.

## Reference evidence

Listary managed `LauncherSearchWindow` constructor/base and `SearchWindowBase.method_7`/ShowWindow/ActivateWindow/Hide binding were inspected first, through the actual WPF/native foreground boundary. Listary native hook source is unavailable. SwiftList `GlobalHotkeyDetector`, `KeyboardHookService`, first-party hotkey documentation and `QuickSearchWindowController` constructor/ShowWindow/HideWindow/ActivateAndFocus were then inspected. Verified call chains and retained lifecycle invariants are recorded in `docs/reference-code/quick-launch-research.md` under the 2026-11-01 ownership section.

## Behavior regression

Before dedicated-thread implementation:

```powershell
dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures 'keyboard monitor fixture dedicated ownership and disposal'
```

Build passed with zero warnings/errors. Fixture result: run 1, pass 0, fail 1, at installation-thread ownership. The injected native seam retained the old caller-thread installation; no real global hook was installed.

After implementation, the same ownership regression and three additional registered/reviewed fixtures passed: selected/run/pass 4, skip/fail 0. The list command reported four reviewed Fixture entries and run 0. They cover:

- Different install thread, two-second caller stall, same-thread callback/loop/uninstall, WM_QUIT, bounded joined exit, idempotent Dispose and no surviving owned thread.
- Reset on next callback, full-release gesture, injected flags, recording cancellation, custom hotkey suppression, independent left/right Ctrl.
- Install failure with native code/correlation, explicit successful retry, healthy idempotence, unexpected loop error, recovery, intentional stop and metadata-only/no-key diagnostics.
- All three wire states, envelope/data correlation/native facts, Hello support versus availability, explicit peer acceptance and old-peer forwarding exclusion.

Logs: session scratch `input-h4-build-red.log`, `input-h4-red.log`, `input-h4-build-green.log`, `input-h4-list.log`, `input-h4-green.log`.

## Affected native-boundary fixtures

Final affected selection used `--fixtures` with the keyboard group, detached-reader fixture, Quick Launch contracts/diagnostics/fullscreen/WndProc, both double-control detector tests and six paste/coordinator regressions. List: selected 17/run 0. Execution: selected/run/pass 17, skip/fail 0.

Logs: session scratch `input-csharp-affected-list.log` and `input-csharp-affected.log`.

## Handshake state preservation

The monitor retains its current immutable health fact independently of the event queue. After HelloAck and queue discard, the pipe sends the latest snapshot to an accepted health peer before normal forwarding. A thread exit between the original Hello availability sample and the acknowledgement therefore remains visible with its original correlation/native error. Existing events and old-peer behavior remain unchanged.

## Final C# commands

```powershell
$testProject = 'csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj'
$filters = @('keyboard monitor fixture', 'extension runtime fixture detached reader responsiveness',
    'quick-launch configuration', 'Quick Launch ', 'double-control gesture',
    'paste retries transient', 'paste diagnostics preserve', 'pressed modifiers keep',
    'incomplete input keeps', 'paste result correlates', 'unverified fallback')
dotnet build $testProject --no-restore
dotnet run --project $testProject --no-build --no-restore -- --fixtures --list @filters
dotnet run --project $testProject --no-build --no-restore -- --fixtures @filters
```

Final results: build0warnings/0errors; list17/run0; execution17passed/0skipped/0failed.

The extended lifecycle fixture consumed the thread-exit event and checked the post-ack snapshot projection. With snapshot publication absent, build passed and the fixture failed at the missing thread_exited snapshot (run1/pass0/fail1). With publication enabled, final build passed with zero warnings/errors and all17affected fixtures passed, with zero skips/failures. Logs: `input-h4-snapshot-build-red.log`, `input-h4-snapshot-red.log`, `input-h4-snapshot-build-green.log`, `input-csharp-final.log`.

## Limits

The queue/delegate fixture substitutes native hook installation and message delivery. It proves real managed thread ownership and lifecycle while the caller does not pump; it is not SendInput, physical keyboard, real clipboard delayed rendering, UAC, installed service, product launch, HWND/DPI, accessibility-provider or user visual acceptance. Windows silent hook removal is not observable here and is not claimed to be detected.
