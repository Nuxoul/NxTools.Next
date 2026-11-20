# Listary multi-monitor placement and window lifecycle

## Source
User requests Listary decompiled behavior as the primary multi-monitor reference. Inspected canonical persistent archive: `D:\Code\Listary.Decompiled\6.3.5.94\managed\Listary`. Its README identifies resource-enabled `.xaml.cs` exports and retained original assemblies. This is managed decompiled/native-call-boundary evidence, not Listary native engine source. Inspection was read-only; no Listary process or service was changed.

## Call chain
- `Core\Launcher\Class292.cs:132-168` (`ShowLauncher`, token `0x0600157A`): if no visible launcher exists, dispose the previous view model, create a new model and call the launcher factory; then Show(true,false) and focus the query. `method_3` means model exists and IsVisible, not HWND existence.
- `Main\Views\LauncherSearchWindowFactory.cs:18-20` constructs LauncherSearchWindow.
- `Main\Views\LauncherSearchWindow.xaml.cs:24-32` calls SearchWindowBase(viewModel), binds LocationChanged and SourceInitialized. Its `aq:91-96` calls `ah` on SourceInitialized unless SkipAutoWindowPositioning.
- `Main\Views\SearchWindowBase.cs:28-34,318-336` binds the model's ShowWindow/HideWindow/ActivateWindow/CloseWindow to its show method, base.Hide, native foreground plus base.Activate, and base.Close.
- `Core\ViewModels\SearchBarViewModel.cs:791-862` publishes show, then activation when requested; Hide publishes hide; Close resets search, hides, then closes.
- `Main\Views\SearchWindowBase.cs:373-399` reaches WindowUtils.ForceForegroundWindow and base.Activate; ShowWindow configures ShowActivated and reaches base.Show, then input focus. Collapsed initialization sets offscreen manual location and skips automatic positioning.
- `Core\Utils\WindowUtils.cs:188-225` validates HWND, restores a minimized target, attempts SetForegroundWindow and checks GetForegroundWindow. Its later aggressive fallback contains console/input maneuvers; Next keeps its existing bounded native activation/readback owner rather than copying these maneuvers.
- `Main\Views\SearchWindowBase.cs:524-554` ignores deactivation while closing/activating, otherwise updates active state and invokes the configured focus-loss reset; close clears visible state.

## Verified placement
`Main\Views\LauncherSearchWindow.xaml.cs:50-69`, token `0x06000145`:
- When both LauncherPositionX and LauncherPositionY are saved, apply those WPF Left/Top values and return.
- Otherwise obtain the real HWND and GetWindowRect.
- Select `Screen.FromPoint(Cursor.Position).WorkingArea`, not the foreground window's screen and not the primary display.
- x = work.X + (work.Width - actual native outer width) / 2.
- y = work.Y + work.Height / 4. The top edge, not the window's center, is at the quarter-height line.
- SetWindowPos changes only absolute position using NOACTIVATE | NOSIZE | NOZORDER. Positioning is separate from activation and does not resize the window.
- `:82-86` invalidates the existing PreviewPopup horizontal offset when the main window moves, retaining parent-relative preview ownership.

## Next contract
Use this cursor/work-area/device-coordinate behavior for ordinary default placement. Preserve Next's existing reusable window and independently owned passive preview; recompute target monitor and bounds at each summon rather than recreating the entire controller. Clamp to the selected work area for supported small/negative-origin/mixed-DPI configurations. Existing Next settings do not acquire Listary's additional saved absolute-position preference in this task.

One sampled monitor identity/work area/DPI must feed both GPUI display binding and final native placement. Tests must distinguish an enumeration index from HMONITOR, cursor monitor from foreground monitor, work area from full display, actual outer frame from client width, and work-height-quarter top edge from remaining-height-quarter positioning. Physical mixed-monitor behavior remains a separate native acceptance gate.
