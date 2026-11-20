# Native picker visibility lifetime

## Boundary and invariants

Reference: `docs/reference-code/system-shell.md` blacklist selection section and the original picker design. Legacy `D:\Code\NxTools\Views\SettingsWindow.axaml.cs` constructor attaches animation; OnOpened configures scope; Save applies then closes; unhandled OnClosed cancels through the original command. There is no legacy window picker. This task intentionally changes source visibility, not settings publication or focus ownership.

Current native call chain: original pipe route -> ProcessWindowPickCoordinator dedicated STA/timer -> ProcessWindowPickController.Execute/Tick/Stop -> ProcessWindowPickNative -> injected ProcessPickApi/Win32. Coordinator finally invokes Stop; connection cancellation is observed by the original controller. Overlay preparation remains hidden, layered, transparent and nonactivating; show remains frame/session-qualified. Hover/process resolution stays off hook callbacks; intercepted left/Escape releases remain owned after terminal publication. Native deadline stays120seconds.

After source/overlay validation, hidden overlay preparation and hook admission, capture only currently visible top-level HWNDs of the verified Desktop peer, excluding the overlay. Pin that process through a retained native handle so process exit/PID reuse cannot authorize another process. Hide with visibility-only SetWindowPos flags; never modify placement, normal/minimized/maximized state or Z order. Restore captured surviving same-process HWNDs with visibility-only SWP_SHOWWINDOW/NOACTIVATE/NOZORDER, not overlay's topmost Show. Already-hidden and foreign HWNDs stay untouched. Revalidate existence/root/PID before each restore; source destruction still retires the session even while intentionally hidden source visibility is accepted.

Every terminal/startup rollback restores before publishing completion or returning failed admission. Capture the full visible set before any hide so owned-window side effects and mutation-then-failure can unwind. Attempt all restores even if one fails; report the first native failure rather than claim successful completion. Rust's deferred source activation/input focus remains authoritative; C# does not activate any restored HWND.

## Outcome and verification

Controller startup hides only after both hooks are admitted. Common Finish and startup rollback restore the captured set before publication/failed acknowledgement. Native ValidateOwners accepts intentionally captured hidden source windows but still requires the original live retained process, source/root/PID and overlay identity. The process handle closes on terminal/rollback. No protocol, Coordinator, Program or TestEffects registration changes are needed.

The production-controller regression requires both settings and main Desktop HWNDs hidden after Start. The reviewed12picker fixtures pass, including19native lifecycle variants,8startup rollback variants and an isolated restore-rejection case. Coverage includes selected/Escape, external cancel with pending releases, disconnect, Coordinator disposal, timer/Show deadlines, source/overlay destruction, retained-process exit, changed HWND ownership, display change, event backpressure, Show/overlay-hide/target failure, partial hook install and mutation-then-hide failure. Flags passed to the injected SetWindowPos leaf require NOSIZE/NOMOVE/NOZORDER/NOACTIVATE/NOOWNERZORDER; bounds and normal/maximized/minimized fixtures remain unchanged. Already-hidden, foreign, child and overlay HWNDs are excluded from source capture. Terminal publication follows restore; paired releases, coalesced hover and late-Show retirement remain covered.

Commands (workspace root):

```powershell
dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures --list "process window pick fixture"
$pickerFixtures = @(
    'process window pick fixture strict wire',
    'process window pick fixture ownership startup failure',
    'process window pick fixture viewport hover names',
    'process window pick fixture click release suppression',
    'process window pick fixture escape cancellation',
    'process window pick fixture stale cancel bounded admission',
    'process window pick fixture owner disconnect dispose deadline',
    'process window pick fixture diagnostics event envelope',
    'process window pick fixture actual pipe reader route',
    'process window pick fixture native policy hook boundary',
    'process window pick fixture partial hooks lost release',
    'process window pick fixture deferred cancel failure facts'
)
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures @pickerFixtures
git diff --check -- csharp/NxTools.PlatformHost/Windows/ProcessWindowPickController.cs csharp/NxTools.PlatformHost/Windows/ProcessWindowPickNative.cs csharp/NxTools.PlatformHost.Tests/ProcessWindowPickTests.cs
```

Build:0warnings/0errors. Listing:selected12/run0. Execution:selected12/run12/pass12/skip0/fail0/not_run0. Scoped whitespace check passes. Effects: in-memory injected Windows/process/hook/clock facts, original adapter/router/event framing and one GUID-owned pipe; no real HWND enumeration/hide/show, global hooks, product launch, Registry, service, Cargo, packaging or install operations executed.

## Limitations

The SetWindowPos P/Invoke, actual EnumWindows/process-handle facts and physical Windows placement/focus behavior were not executed. Visibility-only hide/show preserves existing native placement and minimized/maximized bits without replaying activating show commands; injected fixtures validate the flags and unchanged modeled state, not real Windows acceptance. An OS-rejected restore reports restore_failed/native code and still attempts every other captured HWND; the rejected HWND can remain hidden. Actual product interaction and mixed-DPI/native focus acceptance remain separate gates.
