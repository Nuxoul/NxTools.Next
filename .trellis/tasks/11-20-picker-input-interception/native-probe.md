# Owned picker input native probe

## Scope and effects

Two separately selected `NativeAcceptance` cases in
`csharp/NxTools.PlatformHost.Tests/ProcessWindowPickInputNativeTests.cs`:

- `process window pick input native marked left selects without delivery`
- `process window pick input native marked Escape cancels without delivery`

Each case creates fresh source, target and overlay Forms on one owned PMv2 STA.
Source and the small 240x100 target are visible; overlay is created hidden and is
NEVER shown, including after native full-virtual-desktop layered preparation.
No product, installed NxTools, external process, Registry, service, configuration,
clipboard or Shell operation is used. No foreign-window enumeration is performed.
Only the previous foreground handle and cursor position are retained for conditional
restoration; no titles, paths or input payloads are printed.

`SafetyApi.CurrentProcessId` returns zero to permit same-process source/overlay
ownership. Its getter also establishes PMv2 on the dedicated picker thread.
`TopLevelWindows` returns ONLY the owned source, deliberately excluding target and
overlay. Process pinning/liveness, HWND facts/root lookup, process-name resolution,
overlay styles/bounds/alpha, source hide/restore and native hook registration remain
production Win32 calls. This seam does not establish cross-process ownership or
GPUI/UI acceptance.

The only production addition is `ProcessWindowPickCoordinator(controller)`. It leaves
the fixture-enqueue seam unset and runs the original queue, STA, WinForms timer and
`Loop`. Test-only reflection reads its original `_thread` solely to join actual
cleanup even if Start fails; it does not replace or invoke Loop/controller methods.

## Input and assertions

`Install` wraps each supplied real HookProc and calls base `Install` for actual
WH_MOUSE_LL and WH_KEYBOARD_LL. Delegates, including the captured production
callbacks, remain rooted until cleanup. All physical/nonmatching input immediately
calls CallNext unchanged and never enters the production callback. Its occurrence
marks the probe interrupted and prevents cursor/focus restoration over user activity.

Generated input uses a fresh nonzero GUID-derived31-bit ExtraInfo marker and full
native-value equality. Its positive32-bit range preserves identity through the observed
native input transport; no lower-bit-only matching is used. A marked absolute
virtual-desktop move positions the cursor at the target's client center; its
packet destination must resolve to that owned target while foreground/focus/capture
remain owned. Left down/up or Escape down/up then enter SendInput in ONE atomic
pair. Every marked transition checks foreground, focused root, capture and pointer
root; mouse coordinates must also resolve to target. Unsafe marked input is swallowed
and the case becomes typed SKIP. The packets and Windows injected flags are never
rewritten. Managed callback failures are FAIL, not environmental SKIP.

Each case requires two real callback packets, one down and one up, both injected and
both consumed by the production callback. The real controller must publish its
session-qualified Selected (exact owned target PID/HWND) or Cancelled terminal.
Target-thread IMessageFilter counts actual left down/up and Escape down/up before
WinForms dialog-key preprocessing; all four counts must remain zero after a queue
drain. Terminal must restore source while overlay remains hidden. Cleanup prints
only fixed stage, thread-stop and input/terminal/message counts.

## Lifetime and outcomes

UI readiness, Start, placement, pair, terminal and UI dispatch have bounded waits. Timeout failures identify their fixed stage: ui_ready, picker_start, placement_prepare, placement_callback, input_arm, input_pair_callback, target_queue_drain or terminal.
Finally releases a partial pair while safety hooks are still installed, cancels and
disposes the coordinator, and joins its actual owner (5s). It then checks the exact
owned hook handles are already invalid, with exact-handle unhook fallback if the
owner failed cleanup. Wrappers are disarmed first; no unrelated hook is touched.
A still-live owner or retained native hook is FAIL, not successful logical disposal.
The UI owner exits its loop, disposes all three Forms and is joined (3s); all owned
HWNDs must be gone. Only owned threads/windows are stopped/destroyed.

Cursor restoration occurs only if no non-probe input intervened and the cursor
still equals the probe point. Foreground restoration occurs only if no such input
intervened, foreground is still an owned source/target, and the original HWND exists.
Requested restorations require native success/readback. Thread-local DPI changes
never mutate global display/font/configuration state; the UI context is restored
on exit and the dedicated picker thread terminates.

Unavailable interactive desktop, hook permission, owned foreground or SendInput
permission is typed SKIP, never PASS. Missing callbacks, leaked target messages,
wrong/missing terminal and incomplete cleanup are FAIL. User intervention/obstruction
is typed SKIP with probe input blocked. A passing isolated injected case is not
physical-input, cross-process, mixed-DPI, GPUI rendering or installed acceptance.

## Verification

The C# test-project build passed with0warnings/0errors. Native listing selected exactly2
NativeAcceptance entries with run0. Each exact case ran separately: selected1/run1/pass1/
skip0/fail0/not_run0; combined2pass/0skip/0fail. Both reported down1/up1/injected2/consumed2,
terminal1 and target left_down0/left_up0/escape_down0/escape_up0. Assertions confirmed actual
Selected/Cancelled session results, source restoration, hidden overlay, conditional
cursor/foreground restoration, hook-owner exit, native hook removal and owned-HWND destruction.
This accepts the owned-window injected-input hook boundary, not physical/installed acceptance.

```powershell
dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --native --list "process window pick input native marked left selects without delivery" "process window pick input native marked Escape cancels without delivery"
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --native --authorize-native "process window pick input native marked left selects without delivery" "process window pick input native marked Escape cancels without delivery"
```

LIST must contain exactly the two NativeAcceptance entries and RUN0; defaults and
`--fixtures` must not select either entry. Execution remains within the user's
explicit short self-owned global-hook/marked-left/Escape authorization. Inspect
actual RUN/PASS/SKIP/FAIL and cleanup counts; do not treat a zero exit with skips as
acceptance. The five `process window pick input fixture` registrations remain separate
owned-packet/injected-fact cases; they do not install real hooks.

## Native references

- [Microsoft LowLevelMouseProc](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelmouseproc): installing-thread message loop, nonzero suppression result and hook timeout constraints.
- [Microsoft MOUSEINPUT](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-mouseinput): SDK field types including ULONG_PTR dwExtraInfo.
- [dotnet/runtime1515](https://github.com/dotnet/runtime/issues/1515): pointer-width field and naturally aligned INPUT union declarations.
- [PowerToys638](https://github.com/microsoft/PowerToys/issues/638): low-level mouse hook callback cost and dedicated-owner discussion.
