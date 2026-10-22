# GPUI Kit Windows acceptance fixtures

Build Debug `nxtools-desktop` and `NxTools.PlatformHost.Tests` using the workstation's required Cargo memory settings. The script does not build/package/install or connect product services. Set `PI_SCRATCH_DIR` to an explicit session scratch directory.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tests/native/gpui-kit-acceptance.ps1 -Mode quick -AuthorizeNative
powershell -NoProfile -ExecutionPolicy Bypass -File tests/native/gpui-kit-acceptance.ps1 -Mode clipboard -AuthorizeNative
```

Run these only after separate explicit native-acceptance approval. Missing -AuthorizeNative rejects before starting the preview process. The wrapper passes --native --authorize-native plus its exact existing filters into the C# harness; ordinary build verification never invokes it.

Default `-Check All` requires both the passive mouse activation gate and client/UIA gate. Both modes now pass with the explicitly user-approved repository-local Windows0.3.8 patch: live `WS_EX_NOACTIVATE` yields `MA_NOACTIVATE=3`; primary/unflagged windows still yield `MA_ACTIVATE=1`, including policy toggles. The original released backend returned1 despite the flag; retain that historical red evidence, never remove/ignore the gate. Kit stays exact0.7.1, the Cargo registry is unchanged, and the patch is limited to `vendor/gpui-pre-windows/src/events.rs`.

`-Check ClientAndUia` explicitly runs a narrower gate: actual current-DPI client size, no foreground change while applying/showing/hiding, same-HWND reapply/re-show and native UIA synthetic input/action names/roles/focus capability. At96DPI both Quick400×410 and Clipboard460×406 pass after C# client/outer-size compensation. All's bounded WM_MOUSEACTIVATE query is actual HWND message-policy evidence, not physical pointer-click acceptance. Neither subset certifies physical gestures, screen readers, whole product accessibility or mixed-DPI behavior.

The Debug preview is intercepted before product startup. Synthetic content reuses production Quick Launch/Clipboard preview rendering with invalid application handles; it does not exercise whole generation controllers or IPC sessions. C# invokes the production native no-activate/bounds/visibility adapter through scoped test seams. Tests require exact fixture PID, executable path and synthetic window titles; no ordinary product targets. The script checks actual exit code plus exact RUN/PASS, bounds waits, cleans only the processes it created, restores environment and writes evidence under session scratch. No unrestricted native/DOpus harness, registry/service mutation, system DPI change or user-history access.
