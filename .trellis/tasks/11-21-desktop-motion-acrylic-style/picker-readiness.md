# Process window picker material readiness

## Stage three boundary

This slice belongs to **阶段三：原生材质与展示生命周期**. The existing passive desktop outline and executable/dimensions label remain unchanged. The settings editor retains the official Kit0.10.0 Small outlined Button beside its original32DIP Input and Add action; it is not a Toolbar child. The transparent GPUI Root is a read-only native adornment, not an ordinary control replacement. Kit0.10.0/defaultpre0.3.12 and all vendor sources remain fixed.

## Verified reference chain

- `docs/reference-code/system-shell.md:389-410,439-474` records the picker, hidden-source and raw-input contracts. Legacy `D:\Code\NxTools\Views\SettingsWindow.axaml.cs:15-24,56-77,185-194` retains constructor -> `WindowAnimationService.Attach` -> scoped OnOpened -> Apply/Close or unhandled cancellation. `Services\WindowAnimationService.cs:13-44` attaches Opened/Closing and ultimately calls Avalonia `Window.Close`; Avalonia's native base is not available. There is no indexed legacy window-picker implementation.
- `network_settings/window_picker.rs::begin_process_window_pick` installs the original session/source/view/connection, hidden overlay HWND and125second deadline before Start. Hover updates the existing overlay and schedules its first frame; only that frame calls `show_process_picker_overlay`.
- `PlatformPipeClient.cs:951-952` parses `process_window_pick` and forwards the authenticated Desktop PID. `ProcessWindowPickCoordinator.cs:16-17,25-75` constructs the native/controller pair and executes commands and ticks on one bounded dedicated STA owner.
- `ProcessWindowPickController.cs:61-74,85-109` owns Show and Start admission. Start validates owners, prepares while hidden, installs hooks, then hides captured Desktop windows. Show revalidates the same session/owners and native120second deadline before `native.ShowOverlay`.
- `ProcessWindowPickNative.cs:23-46` validates root HWND/PID/source lifetime, applies passive/click-through/layered styles and alpha255, and verifies final virtual-screen outer/client bounds while hidden. `ShowOverlay` at48 reaches `ProcessPickApi.Show` at217-219: SetWindowPos TOPMOST with NOSIZE/NOMOVE/NOACTIVATE/SHOWWINDOW/NOOWNERZORDER. Hide/restore at221-231 preserve placement and activation. Controller terminal paths hide and restore before publication.

## Preserved invariants and admission

- Start remains hidden; Hover and the original GPUI frame remain prerequisites. A current shared-runtime Transparent receipt for the exact overlay HWND must additionally precede assigning `show_command` or sending ShowOverlay. No material command targets the candidate's external HWND.
- Rust keeps the same pending session/source/view/connection/overlay identity and125second timer. Native placement, passive focus, hooks, source hiding/restoration, Show ACK and deferred input refill remain with their original owners. Selection never adds or persists a rule.
- Register material readiness on a deferred application turn outside the settings application/window leases. Revalidate original identity before registration and completion, including cached receipt paths. `is_waiting` retains the first callback and its deadline across repeated admission attempts.
- Readiness failure uses original `cancel_process_window_pick(id, "native_failed")`. Close, page cancellation, timeout, retired view/source, disconnect and late ACK cannot reveal an old or replacement overlay.
- Keep existing picker accepted/completed/native-failed diagnostics and shared material readiness/request/receipt diagnostics. No candidate names, titles, pointer/frame streams or new visibility owner.

## Regression boundary

Existing native/frame tests explicitly seed qualified facts, source HWND101/confirmed Backdrop before the settings defer and overlay HWND102/confirmed Transparent. This fixture does not alter production HWND discovery; the actual picker Button still rejects TestWindow's absent source HWND. New material tests remove the overlay preparation, require the real ApplyWindowMaterial wire, and cover current-receipt success, repeated wait, cancellation/close/disconnect, stale receipts, failure and late ACK without reveal. Cargo and native execution are deferred to the parent integration run; this note records source evidence, not native or user acceptance.
