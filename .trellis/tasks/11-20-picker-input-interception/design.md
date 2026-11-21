# Picker input boundary

## Reference and owner

`docs/reference-code/system-shell.md` records the original settings Apply/Cancel behavior and the absence of a legacy window picker. This is the explicitly requested addition. The existing path remains original pipe -> ProcessWindowPickCoordinator dedicated STA Application.Run/timer -> ProcessWindowPickNative registered WH_MOUSE_LL/WH_KEYBOARD_LL callbacks -> ProcessWindowPickController -> deferred correlated event. Rust still owns draft refill, source focus, Add/Update and Apply. No protocol or GPUI change.

## Input and lifecycle

The raw mouse callback performs only packet decoding, WindowFromPoint/GetAncestor root lookup and fixed-size input state changes. Eligibility/composition/class/rectangle facts, process-name resolution, publication and logging run outside the hook callback. Relevant physical and Windows-injected packets use the same decision; production does not replay or inject its own input.

The initiating held left button and source HWND retain passthrough. A later timer observation of physical release clears a stale initiating-button latch. This poll never claims an already-delivered event was swallowed. Every fresh selection down otherwise owns its matching up, including root0, overlay, unavailable target and additional input before deferred selection. The first pending clicked root is retained independently of its numeric value. Unavailable targets produce failed rather than passing the click to another application. Escape down/repeats/up are swallowed and cancellation is processed on the next owner turn. A terminal session keeps hooks while an intercepted release remains pending. Its lost-release recovery requires observing pressed then released after interception; a false GetAsyncKeyState sample alone is insufficient because a suppressed down need not update that state. Without reliable pressed observation, the real release callback owns cleanup. The existing deadline, disconnect and disposal retire selection without discarding release ownership.

Source windows are captured/hidden only after both hooks are installed and are restored before terminal publication or failed-start acknowledgement. Original passive overlay placement, nonactivation, frame-qualified Show and late-session rejection are unchanged. Cancellation wins over deferred selection. Only the original draft editor receives a selected executable; no rule is published by native selection.

## Diagnostics

Original accepted/started/completed/failed events retain session correlation, fixed state/reason, elapsed_ms and native_error_code. Fixed aggregate fields add mouse_down_received, mouse_up_received, escape_down_received, escape_up_received, mouse_events_swallowed, escape_events_swallowed, pending_mouse_release and pending_escape_release. Counts describe the event's observation time; pending releases can be drained after terminal publication. No per-input, pointer, title, executable or payload logging.

## Verification boundary

Five registered-HookProc fixtures cover actual unmanaged packet decoding with injected flags intact and physical-style flags0, invalid/pending click consumption, stale initial-left recovery, absence of heavyweight callback reads and held-left/Escape release ownership. Original picker fixtures retain paired release, lifecycle, visibility rollback, native errors and original pipe framing. A separately classified and explicitly authorized native probe exercises real hook installation and the real coordinator loop using owned windows and uniquely marked SendInput packets. Its safety wrapper passes unrelated user input unchanged and never forwards probe packets into foreign windows. Owned-window injection is not physical-input or installed-product acceptance.

The native probe carries a fresh nonzero31-bit GUID-derived ExtraInfo marker through its actual SendInput packets and compares the full native value. Positive32-bit identity is sufficient for the observed transport; no truncated matching or injected-flag rewriting is used. Timeout exceptions identify the fixed waiting stage. Both owned-window native cases require actual down/up callbacks, production suppression, matching controller terminal and zero target input messages before cleanup.
