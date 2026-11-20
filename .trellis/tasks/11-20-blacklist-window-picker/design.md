# Window selection boundary

## Reference and components
User reference: thin desktop window outline with executable name and dimensions at its upper edge, hover then left-click confirmation. Legacy SettingsWindow constructor attaches animation; OnOpened configures scope; Save invokes Apply then Close; unhandled close cancels. Legacy blacklist only has textual exclusions and no window-picker path was found by window-selection/hook/point-lookup searches.

Existing Settings/SettingPage/SettingGroup/SettingItem rendering and Small Form/Field editor remain. Add official released Kit0.8.0 Button::new(...).small().outline().label(...), with semantic label and no Toolbar.child transformation. Existing Input remains32DIP; new Small outlined action sits beside it, before scope checkboxes and Add. During selection it becomes Cancel selection. Ordinary controls remain Kit-owned. Native highlight is a GPUI transparent, nonactivating, click-through auxiliary surface, not a replacement ordinary control; frame/label are read-only adornment.

## Wire
PlatformCommand::ProcessWindowPick uses command process_window_pick and action-specific data:
- start: {action:"start",session_id:UUID,source_window_handle:i64,overlay_window_handle:i64}
- show_overlay: {action:"show_overlay",session_id:UUID}
- cancel: {action:"cancel",session_id:UUID}
Original command results acknowledge admission/failure; start data is optional. Native event process_window_pick_changed carries {session_id,state:"hover"|"selected"|"cancelled"|"failed",viewport?:{left:i32,top:i32,width:u32,height:u32},candidate?:{window_handle:i64,process_id:u32,process_name:string,left:i32,top:i32,width:u32,height:u32},native_error_code?:i32}. Hover always supplies viewport; absent candidate means no eligible target. Terminal selected supplies candidate. Session UUID is correlation across layers. Outer protocol remains1; events are emitted only for this opt-in command.

## Ownership and lifecycle
Rust registers pending request before deferred Start dispatch, holds original source window/view and connection, creates one hidden GPUI overlay and updates it only from current session facts. Native Start validates source/overlay ownership, configures overlay hidden with WS_EX_NOACTIVATE/WS_EX_LAYERED/WS_EX_TRANSPARENT, initializes SetLayeredWindowAttributes with alpha255/LWA_ALPHA, and verifies exact virtual-screen outer/client bounds. Rust computes overlay-local layout from native viewport origin and actual window scale, then requests session-qualified show_overlay only after a rendered frame. Native Show refuses a retired session. C# never paints the frame or label.
The rendered outline intersects the native target rectangle with the virtual desktop before scale conversion, keeping maximized/off-edge frame and caption visible. Caption dimensions remain the original native window dimensions; target bounds are never changed.

Native session uses existing adapter lifetime and one bounded message-loop owner, never a modal loop on the serial reader. Hover facts coalesce to target/rect changes; native keyboard/mouse callbacks perform no process IO/logging/IPC. Left confirmation resolves the actual clicked target and swallows down/up; the source settings window remains pass-through for Cancel/Close. Escape cancels without forwarding to another app. Release hooks, hide overlay and emit terminal once; preserve release suppression through an intercepted button/key up. Cancel/disconnect/owner disappearance/native120s expiry tears down. Rust125s deadline also retires and sends native Cancel. Selection only fills original InputState in a deferred source-window update and restores its input focus; it never adds or persists rules.

## Diagnostics
Rust global.blacklist.window_pick.accepted/completed, native blacklist.window_pick.accepted/started/completed or failed; session UUID, state/fixed failure kind, elapsed and native code only. No hover/pointer logs or process/window payload logging.
