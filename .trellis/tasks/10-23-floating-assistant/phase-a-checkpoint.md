# Phase A — Native receiving feasibility checkpoint

2026-10-23. Implementation approved; current phase remains Phase A. Feature is NOT integrated and Phase B/C have not started.

## Implemented native boundary
- `csharp/NxTools.PlatformHost/Windows/FloatingAssistantInputSurface.cs`: self-owned STA input-only WS_EX_NOREDIRECTIONBITMAP popup, source PID/session validation, foreign renderer owner (not SetParent), hidden creation, NOACTIVATE/TOOLWINDOW, native final bounds/readback, circular region, reversible receiver hide/pass-through, OLE initialize/register/revoke/dispose, native pointer capture and click/drag suppression. GPUI/vendor sources unchanged.
- `FloatingAssistantDropTarget.cs`: native IDropTarget ABI, query-only offer, one admitted drop, original Unicode text/URL/HTML/PNG/DIB/GDI/file representations, finite payload budgets, STGMEDIUM release, cancellation/hidden rejection and payload-free terminal diagnostics. Routing remains Rust's future responsibility, no clipboard/network/file reads.
- These classes are not connected to production adapter/IPC/runtime. No Other Tools UI, settings section, GIF playback or action/speed integration exists yet.

## Red-capable observations
1. Transparent hit/pass-through fixture first failed with the unimplemented native surface, then passed after implementation.
2. OLE capture-once fixture failed on supported-offer admission, then passed with actual bounded capture.
3. Foreign-renderer fixture failed on invisible rectangular corners, then passed after actual circular native region and OLE registration.
4. Native gesture fixture failed on first click, then passed on real own-HWND messages with one left/right, drag-no-click, lock-click and capture-loss cancellation.
Compile-only type errors were corrected before behavioral evidence; not counted as red regressions.

## Verification actually performed
- `dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore -v minimal`: PASS,0warnings/0errors.
- Exact5`floating input fixture` entries LIST selected5/run0; final execution union6`test selection` cases selected11/run11/pass11/skip0/fail0/not_run0.
- Owned tests: pixel equality under input layer, actual WindowFromPoint and WM_MOUSEACTIVATE, same/foreign process source identity, one real OLE HWND registration, bounds resync and receiver disposal, original representations, native COM CCW slot3/slot6 ABI, oversized text rejection, leave cancellation, local path without file read, real GDI bitmap without clipboard.
- `python tests/architecture/gpui-release-contracts.py`: PASS, current coherent0.8.0/pre0.3.9/two original narrow vendor deltas unchanged.
- Scoped `git diff --check`: exit0; Git emits existing Program.cs line-ending warning (CRLF->LF on later Git write), not a whitespace failure.
- No Cargo, package/install/service, normal product runtime, global hooks, actual clipboard, network, user files, commit or push.

## Authorized actual OLE prerequisite — passed
The user explicitly authorized the exact owned native route test and necessary same-scope repair/reverification. `FloatingAssistantOleAcceptance.ActualOleRoutesAcrossOwnedProcesses`, exact name `floating input native actual owned OLE routing`, executed with `--native --authorize-native`: selected1/run1/pass1/skip0/fail0/not_run0. It was rerun after coordinator work and again passed1/1.

The test temporarily moves/restores the real pointer, creates only its own child renderer/input receiver and uses Windows DoDragDrop with in-memory Unicode text, GDI bitmap and synthetic file-path data. No button/key injection, user files, network or actual clipboard was involved. Input source cancels outside the exact owned receiver and has a bounded deadline; cleanup restored the pointer and stopped only its own child.

Final safe union after coordinator fixes:7floating native/coordinator fixtures plus6selection cases = selected13/run13/pass13/skip0/fail0/not_run0. One earlier broader subagent run failed the baseline pixel-color assertion before receiver creation; single reproduction and final integrated13case run passed. This intermittent owned-pixel setup observation remains disclosed, not erased by rerun.

The native receiving feasibility prerequisite passed and Phase B began. It does not accept actual GPUI rendering/opacity, physical browser/Shell drop, mixed-DPI or user visuals. Current Phase B compilation blocker and precise remaining work: [implementation checkpoint](implementation-checkpoint.md).
