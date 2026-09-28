# Implementation Plan

## Ordered Steps

1. Add a red PlatformHost regression for the exact event order: known file dialog, pending DOpus recognition, foreground moves to the file dialog, passive retry reads the pending DOpus HWND, and one candidate is produced.
2. Add a regression for pending-target destruction/supersession and bounded exhaustion; verify no source remains dirty for a later dialog.
3. Implement an explicit pending-target/background recognition operation in the existing C# retry boundary. Keep file-dialog foreground recognition semantics unchanged.
4. Add bounded start/success/terminal diagnostics without payload secrets or high-frequency timer logs.
5. Run the focused tests and fix only concrete failures.
6. Run the full PlatformHost test executable and relevant project build/static checks.
7. Run the interactive DOpus/file-dialog soak if the environment provides DOpus; otherwise record the deferred environment gate.
8. Review the reference note and update `docs/reference-code/system-shell.md` or the relevant spec only if the fix establishes a new durable lifecycle invariant.

## Expected Files

- `csharp/NxTools.PlatformHost/Windows/FileDialogRecognitionRetry.cs`: explicit foreground-gated versus pending-target retry contract, if the existing helper remains the right seam.
- `csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.EventWindow.cs`: source retry target capture, terminal diagnostics, and no changes to navigation IPC.
- `csharp/NxTools.PlatformHost.Tests/Program.cs`: red regression and terminal/supersession coverage.
- `.trellis/tasks/09-28-dopus-dialog-auto-switch-race/*`: planning and validation records.
- `docs/reference-code/system-shell.md` or `.trellis/spec/` only when the implementation confirms a reusable contract.

## Validation Commands

```powershell
 dotnet run --project csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj --no-restore -- "Directory source recognition" "auto quick switch" "Directory Opus" "file-dialog"
 dotnet run --project csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj --no-restore -- "localized known folder aliases" "real Directory Opus navigation"
 dotnet run --project csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj --no-restore
```

Use the repository's committed Windows wrappers for Rust/package checks if the change reaches Rust. Direct Cargo commands must inherit the repository-required single-job environment, but this slice is expected to remain C#-local.

## Rollback Points

- Before implementation: the new regression must be red.
- After retry helper change: focused tests must be green before diagnostics or doc updates.
- Before completion: full PlatformHost suite must pass and no temporary `DEBUG-...` instrumentation may remain.
