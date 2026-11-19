# A native COM and demand lifetime

Status: source implementation ready for user-owned verification. No compiled/native/memory acceptance result is claimed.

## Native boundary

NativeUiAutomation binds CUIAutomation8/IUIAutomation/IUIAutomation2 using the installed Windows SDK10.0.26100.0 interface identities and slots. The OneCommander path uses native element/cache/condition/raw-walker/ValuePattern/comparison/event APIs throughout, without System.Windows.Automation or shared RCWs. Native interfaces, BSTR and VARIANT buffers have explicit ownership. The callback instance and its GCHandle survive actual native references until final Release; one fixed process-lifetime vtable is shared.

OneCommanderUiaSession composes the existing OneCommanderFocusTracker MTA. It is the single origin apartment for the client, cache and strong request panes. Native callbacks first read cached PID/HWND, reject unrelated senders and coalesce one retained focus element. Accepted traversal runs on the originating MTA. A HWND-less WPF child remains valid.

## Admission and cancellation

Existing named120ms capture/1500ms navigation STA admission remains. Each caller hands one operation to the MTA and waits for physical completion. Logical timeout/cancellation does not release the original STA name while a native call is still running. At most one request and one pending focus element are retained; no replacement thread or unbounded work queue.

A retiring MTA cannot be replaced until its actual Thread exit/join. Existing bounded-runner Dispatcher cleanup remains unchanged. Native timeouts use IUIAutomation2; every cooperative tree/readback step also observes request generation and cancellation. A500ms shutdown wait can report retirement_pending, never completed native cleanup.

CaptureHandoff owns a result until the bounded caller accepts it. A timed-out or abandoned caller disposes an already-published or late result's pane lease, rather than leave an orphan native anchor.

## Demand

Startup in both roles creates a directory lifetime only: no UIA client, subscription or worker. Ordinary foreground/destroy events reuse EventWindow; automatic OneCommander observation/capture runs only for enabled file-dialog search or auto quick-switch. Turning both off or leaving the target removes focus demand. Explicit Quick Launch capture remains available without automatic observation.

Native AddFocusChangedEventHandler is global while enabled; there is no per-process/root focus overload. Its lifetime is target demand, not process startup. After demand ends, remove the handler and focus cache. Frozen request panes retain only their required COM origin/resources, not desktop-wide tracking. The original60second sweep performs native HWND/PID checks, never idle UIA traversal.

The elevated helper uses explicit native queries. For elevated OneCommander only, the original authenticated180ms capture request executes before QuickLaunchPanelRequested publication, while the original focused WPF child is still current. Other elevated hosts retain deferred capture. Native activation/focus restoration, main-window/process gates, nearest CurrentPathGet/CurrentPathSet selection and original layout stay intact.

## Pane and session ownership

The focus cache remains64latest strong anchors with unchanged recency semantics. Native request panes are64owner/identity/HWND/PID leases, expire with the original10minute lifetime and remain independent of later cache replacement. Queries cannot substitute a first pane when the captured pane is unavailable.

Registry consume, fatal failure, expiry, capacity, explicit release and scope cleanup mark native leases retired. Window destroy observations and the native sweep retire unavailable targets. The MTA owns actual COM release; registry locks only signal retirement. An executing navigation holds a separate native reference until its operation exits.

The helper's disposable outer-context registry retires the original inner context, including EOF/failure/normal completion. Ordinary PlatformPipe disconnect closes directory admission/contexts; reconnect resumes the dormant lifetime. Adapter disposal and constructor startup failure also stop it. Scope generation rejects late requests/callback mutation.

## Cleanup and diagnostics

Stop order is admission/generation close → exact native handler removal → entered-callback drain → pending/focus cache/pane/operation-local release → client/COM teardown → actual thread exit. A removal failure retains its native code, refuses successful re-registration over that failure and tears down the owner. Native registration setup failure also removes the exact handler before releasing local ownership.

onecommander.uia.lifecycle records correlation, role, fixed state/reason, pane-window count, elapsed and native code. It does not log path/focus/window text or payload secrets, and does not emit per-event/per-byte production records. Sink errors cannot alter native results.

## Acceptance ownership

The user owns validation. No build, test, native fixture, GUI/UAC launch, memory sampling or B6 timing was executed for this source implementation. There is no behavioral RED/GREEN or observed memory improvement. Physical OneCommander/elevation, actual native resource retirement, sustained memory, human visual and installed/Release acceptance remain unconfirmed. The incident's complete CCW allocation share remains inference without allocation stacks.

[Implementation outcome and precise paths](implementation-outcomes.md). B6 remains unstarted; accepted B1–B5 and unrelated native owners are preserved as the baseline.
