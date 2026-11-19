# PlatformHost UIA memory lifecycle repair

Status: source implementation delivered for user-owned verification. Compiled, native lifecycle and memory acceptance have not been run. B6 timing remains separate and unstarted.

## Reported observations

The supplied incident records ordinary PlatformHost and elevated-directory helper near20GiB private memory with a small managed heap; the ordinary instance was reported at about14.6MiB managed heap/29MiB GC committed. OOM stacks pass through InterfaceMarshaler.ConvertToNative → RawUiaRaiseStructureChangedEvent → WindowsListView.GroupSpecificEvents → WinEventTracker.WinEventProc. OneCommander was absent, but both startup roles reached global managed focus subscription through WarmUp.

The complete native allocation/CCW share is not established by allocation stacks. These are incident observations and an inferred allocation attribution, not a reproduced measurement or a new memory result.

## Reference authority

[System-shell](system-shell.md#sta-dispatcher-and-onecommander-focus-lifecycle) indexes the inspected SwiftList path collector, inline-search adapter, UiaPathAccessor and STA call chain. Preserve early process rejection, pre-overlay strong HWND-less WPF focus, nearest CurrentPathGet/CurrentPathSet pane and original activation/focus restoration. The legacy NxTools checkout has no OneCommander implementation; Listary evidence is its managed boundary, not native engine source.

## Implemented owners

- `WindowsPlatformAdapter` acquires `ThirdPartyFileManagerAdapter.BeginLifetime`; startup does not initialize OneCommander UIA. Original EventWindow foreground/destroy callbacks supply demand only for enabled directory-source features. Explicit summon capture still precedes panel publication.
- `ElevatedDirectoryNavigationHelper.Run` owns the same lifetime below existing rendezvous/elevation validation, plus a disposable public-to-local context registry. EOF, failure and normal return release captured contexts and stop the native owner. The original role log is passed through Program.
- Elevated OneCommander capture now uses the original authenticated180ms capture channel before QuickLaunchPanelRequested is published. It does not infer a pane from overlay focus. Other elevated hosts retain deferred capture.
- `OneCommanderUiaSession` owns native UIA resources on the existing OneCommander MTA. Named120ms capture/1500ms navigation STA calls hand over one operation and keep their physical admission until that native operation returns. A retiring MTA cannot be replaced before actual thread exit.
- `NativeUiAutomation` binds CUIAutomation8/IUIAutomation/IUIAutomation2 directly using installed SDK10.0.26100.0 ABI declarations. Element queries, raw parent traversal, conditions, comparisons, ValuePattern, cache requests and callbacks no longer use System.Windows.Automation in the affected OneCommander path.
- The Quick Launch source gate follows `OneCommanderFileManagerOperations.TryCaptureActivePane` / `Navigate` through `BoundedStaOperationRunner` to `OneCommanderUiaSession.CaptureOnOwner` / `Navigate` and the native UIA interface. Capture requires a validated focused element or retained anchor in the target window; global focus is queried only while that window owns foreground. Navigation uses the frozen pane, not the first window pane.
- Native AddFocusChangedEventHandler accepts cacheRequest/handler only; it is global while enabled, not per-process. Cached PID/HWND filter before retained references or traversal. Native callback references coalesce to one pending element on the existing MTA owner. The helper does not keep an idle global subscription.
- Focus cache holds at most64strong latest anchors; request panes are64owner-bound native leases with original10minute expiry. Cache replacement does not invalidate a frozen request. Existing60second native window/process sweep and original destroy observations retire unavailable targets without UIA idle traversal.
- Session consume, fatal result, capacity, expiry, target retirement, pipe disconnect and scope disposal release native leases. Helper outer-context removal also releases its original inner context. An abandoned capture handoff disposes the lease even when its bounded caller already timed out.
- Stop closes admission/generation, removes the exact event handler, drains entered callbacks, releases pending/cache/pane/local interfaces and native buffers, balances COM initialization and exits the thread. Unregister failure retains its HRESULT and closes the owner rather than reporting successful re-registration. A500ms wait can report retirement_pending, not completed cleanup.

Raw pointers own exactly one native interface reference; Clone owns an explicit AddRef. BSTR/VARIANT/cache/walker/pattern references have matching cleanup. The callback's native IUnknown lifetime owns its GCHandle/instance until final native Release; its dispatch vtable is one fixed process-wide table. No shared-RCW FinalRelease, forced GC, trimming, restart, memory cap, replacement scheduler or unbounded queue is used.

## Diagnostics

`onecommander.uia.lifecycle` records correlation, ordinary/helper role, fixed state/reason, pane-window count, elapsed and native code. Started/tracking/unsubscribed/retired/retirement_pending/failed are distinct observations. No path, focus text, window title, Header/Cookie/credential or per-event/per-byte stream is recorded.

## Acceptance boundary

Verification belongs to the user for this implementation. No C#/Rust build, test selection, native fixture, GUI/UAC launch, memory sampling or B6 timing was run. There is no behavioral RED/GREEN or demonstrated memory-repair result to report. Existing synthetic tracker/process/STA cases are policy coverage, not native ownership proof.

User verification should distinguish no-target startup, active/split/WPF pane interaction, native unregister/drain, cancelled or blocked calls, window/session/pipe retirement and sustained memory behavior. Physical OneCommander/elevation, human visual and installed/Release acceptance remain distinct. If growth remains, allocation ownership still requires investigation; source migration is not proof that the incident is resolved.

[Implementation record](../../.trellis/tasks/11-11-platformhost-uia-memory/implementation-outcomes.md). B1–B5 accepted behavior and unrelated Shell/clipboard/icon/STA owners remain outside this repair.
