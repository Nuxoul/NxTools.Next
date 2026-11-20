# Window presentation boundaries

## About caption

Released Component0.8.0 TitleBar's private Windows controls use window.window_controls() and WindowControlArea native hit regions; its application on_close callback is not a public Windows client-button override. Existing fixed options alone do not remove Minimize/Maximize. Reuse the project's already approved caption_action backed by the released styled Button:34DIP close hitbox, Small icon, transparent idle/danger hover, no Toolbar transformation and no WindowControlArea::Close. Compose one native drag-only area beside it in the original transparent absolute caption slot; preserve420x480 and body/footer/artwork. about_window_options uses modeless WindowKind::Normal, matching existing global Settings, plus caption_window_options. Read-only About has no parent-blocking decision transaction. Unlike framework Dialog, Normal does not disable the active parent in WindowsWindow::new or re-enable/force-activate it in handle_destroy_msg. Client close retires update ownership then requests GPUI removal once; native should-close retains its retirement guard. No framework source change.

## Blacklist editor

Remove only global-blacklist-new-clipboard and global-blacklist-new-quick controls. Keep original Input, Small Outline picker and Add/Update, row Checkbox columns, default Clipboard=true/QuickLaunch=false, local draft and durable Apply/Cancel. Row scope edits also synchronize the currently edited row's copied flags so Update cannot replace a scope just changed in the now-authoritative row controls. Picker selection still refills only the process input.

## Native selection

Original C# picker captures/hides visible top-level windows from the verified source Desktop process after preparation/hook admission, excludes overlay/foreign/child/already-hidden windows, and restores surviving captured windows on every terminal/rollback path without activation, bounds or window-state changes. No new protocol; native facts stay behind the existing injectable boundary. See native-picker.md for exact lifecycle and controlled evidence.

## Main-window flash diagnosis

User identifies the background main window as the flashing surface after About close. The original About Dialog enters the locked framework's real parent EnableWindow(false) and destroy-time EnableWindow(true)/SetForegroundWindow chain. Modeless About removes that parent-state transition without a renderer patch or blanket animation disable. Production regressions cover constructor policy, actual caption click/removal/update retirement and editor shape; source policy gate binds the mode to the inspected native branch. The settled-animation diagnostic checks ordinary refresh after the actual animation clock elapsed. Existing logs have INFO Direct3D startup records and no device-loss evidence. Physical frame stability and the broader reported page/data-update flashes remain native/visual acceptance, not a claim made from TestWindow/source checks.
