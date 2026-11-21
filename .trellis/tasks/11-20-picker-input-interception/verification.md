# Picker input verification

## Implementation

The registered mouse/keyboard HookProc accepts physical-style and Windows-injected left/Escape packets. Mouse eligibility/composition/rectangle/class checks moved out of the callback to the existing owner timer. A separate pending-click bit retains root0 and the first target; invalid and pending selection clicks own both transitions instead of entering the target application. Initial held-left recovery uses physical-up observation. Terminal lost-release recovery requires an observed pressed-to-released transition; a false async state alone cannot retire an intercepted release. Fixed lifecycle diagnostics include aggregate received/swallowed counts and pending-release bits. Original protocol, source hide/restore, deferred draft refill and Add/Apply ownership remain unchanged.

## Checks

- C# test project build:0warnings/0errors.
- Fixture listing:selected17/run0. Execution:selected17/run17/pass17/skip0/fail0/not_run0. Includes original12picker lifecycle/visibility/pipe fixtures and5registered-HookProc input regressions for mouse, Escape, invalid/pending targets, callback cost and held-release ownership. Each new regression was observed failing before its production repair.
- Native listing:selected2/run0, exact NativeAcceptance labels. Each authorized exact case ran separately with selected1/run1/pass1/skip0/fail0/not_run0; combined2pass/0skip/0fail. Both observed down1/up1/injected2/consumed2/terminal1 and all four target-message counts0. Actual controller results were Selected for left and Cancelled for Escape. Source restoration, hidden overlay, conditional cursor/foreground restoration, hook-owner exit, native hook removal and owned-window destruction checks passed.
- `python tests/architecture/spec-contracts.py`:PASS132documents/14workspace owners; document structure/links only.
- Scoped `git diff --check`:PASS.
- Native input uses a nonzero31-bit random ExtraInfo marker with full-value equality; system injected flags and callback packets remain unmodified. Timeout errors identify the fixed waiting stage.

Commands:

```powershell
dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures --list "process window pick fixture" "process window pick input fixture"
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures "process window pick fixture" "process window pick input fixture"
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --native --list "process window pick input native marked left selects without delivery" "process window pick input native marked Escape cancels without delivery"
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --native --authorize-native "process window pick input native marked left selects without delivery" "process window pick input native marked Escape cancels without delivery"
python tests/architecture/spec-contracts.py
```

## Limitations

The real Win32 hook/message-loop/controller boundary passed using marked injected input and self-owned windows. Physical-input, cross-process target, GPUI/mixed-DPI and installed-version acceptance remain separate. No installed-program replacement was performed, so these checks do not establish the running installed build's behavior. No Cargo, package, installer, Registry/service mutation or commit was performed.
