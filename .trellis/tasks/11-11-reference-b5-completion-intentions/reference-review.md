# B5 reference review

Behavioral review only; no reference code, comments, tests or assets are incorporated. Product scope is [REF-07/REF-11](prd.md). Reference applications were not launched and no native power operation was executed.

## AB Download Manager

Source root: `D:\Code\ab-download-manager-master`.

- `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/ondownloadcompletion/OnDownloadCompletionActionRunner.kt:22` subscribes only to OnJobCompleted and delegates through the provider. Its independent per-job callbacks do not establish batch coordination.
- `OnDownloadCompletionActionProvider.kt:5` defines the action provider. Desktop `actions/onevennts/getOnDownloadCompletionAction.kt:24` obtains extra task settings and produces PowerActionOnDownloadFinish plus settings cleanup; the action delegates to PowerActionManager rather than performing OS calls in the download engine.
- `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/AppComponent.kt:1197` owns one SlotNavigation/childSlot with serializer=null. The child receives dismissPowerAction and onBeforePowerAction → downloadSystem.stopAnything. `initiatePowerAction:1216` activates that one slot; `dismissPowerAction:1230` dismisses it. PowerActionManager is declared in the same file at1300, not a separate file.
- `pages/poweractionalert/PowerActionComponent.kt:42` starts its countdown; Config defaults to30000ms. Cancel delegates to close. Execution sets busy, catches error into visible state, runs onBeforePowerAction, then invokes DesktopUtils.powerAction().initiate. A native Boolean is not physical power acceptance.
- `PowerActionAlertWindow.kt:55` creates centered450×200DIP fixed always-on-top CustomWindow. Native close delegates to performCancel. Actual controls show remaining time/action/error, Cancel and immediate execution.
- `desktop/app-utils/.../window/custom/CustomWindow.kt:343` passes onCloseRequest/state/resizable/alwaysOnTop through to Compose Window at387. This is the observed show/close boundary; unavailable deeper framework internals are not claimed.
- `desktop/shared/src/main/kotlin/ir/amirab/util/desktop/poweraction/PowerActionWindows.kt:5` implements Shutdown via shutdown /s /t0; Sleep and Hibernate are TODO. They are not evidence of Windows support. B5 must implement support/failure truthfully at NxTools' original C# boundary, not copy shell commands or TODO behavior.

## Legacy NxTools

Source root: `D:\Code\NxTools`.

- `ViewModels/MainWindowViewModel.cs:4317` HandleDownloadCompleted owns completed-window feedback, optional folder opening and shutdown eligibility. `QueueDownloadPostAction:4338` clears one-shot post action before dispatch.
- `TryShutdownWhenAllRequestedDownloadsComplete:4350` requires armed tasks to be Completed and waits while any New/Probing/Scheduled/Queued/Downloading/Merging/Paused/Verifying task remains. It invokes shutdown.exe immediately; B5 retains successful-completion/global-idle compatibility but not immediate or non-cancellable OS effects.
- `Views/DownloadProgressWindow.axaml.cs:26` constructor initializes controls and attaches WindowAnimationService, then binds the original task. OnClosed removes subscriptions, not engine work. `ShowCompletionState:84` transforms that same progress window, preserves center, applies completed bounds and activates it; B5 must not replace the HWND merely to add intentions.
- `Services/WindowAnimationService.cs:13` attaches actual Opened/Closing handling. Closing defers one close animation and calls the original window.Close at43. UI close is presentation lifecycle, not download cancellation or a completion trigger.

## Mister comparison

Source root: `D:\Code\misterdownloadmanager-main\misterdownloadmanager-main`. GPL-3.0-or-later reference, behavior only.

- `crates/app/src/progress.rs:750` exposes completion presentation and when_done options. Other actions are enabled only when the complete dialog is off; Sleep/Hibernate/Exit and force-close are held by the progress object. B5 does not couple durable intention to presentation or import force termination/dial-up actions.
- `crates/app/src/app.rs:5101` collects genuine finished separately from failed. At5149 it reads progress-window completion options before dropping that object. `when_done:4620` gives power priority over Exit and installs a countdown; Exit is a direct requested flag. B5 instead uses one existing cancellation-aware coordinator and the normal shutdown/drain chain for both.
- `crates/app/src/queue.rs:14` persists queue Schedule action fields. `finished:183` accepts Complete or exhausted Failed; an empty member list also satisfies all(). Those are not successful queue completion under B5. Waiting, scheduled, failed, cancelled and empty work must not authorize a power/Exit action.
- `crates/app/src/dialogs.rs:1995` owns one30s TurnOffCountdown with named Shutdown/Sleep/Hibernate text. Enter/Escape cancel and immediate action is explicit. Native execute calls real OS commands/APIs and may force other processes; ordinary B5 tests must never run those effects.

## Migration invariants

Persist desired task/queue intention independently of presentation. Observe genuine durable success through original domain owners. Keep durable intention distinct from a current completion round and one-shot pending coordinator action. Initial terminal records are historical, not a replay trigger. Preserve original same-window completion, focus/IME, generation and close/cancel distinctions. One coordinator/countdown arbitrates concurrent sources; new accepted work and explicit Cancel retire pending work. Directory feedback uses the original bounded deduplication. Windows support/failure/native codes remain C# facts; normal Exit must await real original cleanup.
