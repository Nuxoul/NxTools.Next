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
 dotnet run --project csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj --no-restore -- "directory navigation resolves library URIs" "localized known folder aliases" "real Directory Opus navigation"
 dotnet run --project csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj --no-restore
 cargo test -p nxtools-desktop --bin nxtools-desktop library_uri_is_a_navigable_directory_target_and_nested_paths_are_preserved
```

Use the repository's committed Windows wrappers for Rust/package checks if the change reaches Rust. Direct Cargo commands must inherit the repository-required single-job environment, but this slice is expected to remain C#-local.

## Rollback Points

- Before implementation: the new regression must be red.
- After retry helper change: focused tests must be green before diagnostics or doc updates.
- Before completion: full PlatformHost suite must pass and no temporary `DEBUG-...` instrumentation may remain.

## Existing DOpus tab return — implementation follow-up, 2026-10-04

- User repro: the bottom search-row directory action navigates, but switching to an already-existing DOpus tab and returning to the same open file dialog does not; navigating within the tab or creating a tab makes the old implementation work.
- Root cause confirmed in NxTools.Next: dialog-return refresh ran only while the one-shot `_sourceDirty` bit was still set; after a prior sync it was clear. Separately, `ProcessForeground` returned immediately when the visible file-dialog search host was reused, bypassing Auto Quick Switch processing.
- Fix: before a known dialog consumes a source, passively recapture the registered DOpus lister/session even when dirty is false. The visible-host reuse fast path still reprocesses only the same dialog when Auto Quick Switch is enabled and the retained source is a live DOpus session. Existing-tab selection is additionally coalesced through `EVENT_OBJECT_SELECTION` (with focus/name-change fallbacks), all constrained to the tracked lister HWND/PID; changed path yields one candidate, unchanged path yields none, and failed/superseded refresh discards pending state. No activation, DOpus navigation, Rust IPC or protocol change was added.
- Regression coverage uses the real `ThirdPartyFileManagerSessionRegistry` seam for existing-tab path changes, same-path no-op, one-shot consumption, non-DOpus/session mismatch and failure without stale navigation. A separate production-boundary predicate test covers the visible-search-host early return. The old dirty-only coordinator gate was run red; the corrected gate is green.
- Verification: `dotnet build csharp\NxTools.PlatformHost\NxTools.PlatformHost.csproj --no-restore` passed with 0 warnings/errors; five filtered DOpus/known-dialog/reuse tests passed; `tests/architecture/quick-launch-lifecycle.ps1` passed. The full PlatformHost suite is intentionally not being rerun because it includes a real DOpus C-drive navigation test.
- Manual DOpus tab-switch/file-dialog smoke remains required; keep this task `in_progress` until the user verifies the actual DOpus layout.
- Manual DOpus tab-switch/file-dialog smoke remains pending. Do not mark this task complete until user verifies the actual DOpus layout. One earlier full PlatformHost run inadvertently included the real `RealDirectoryOpusNavigationReachesCDrive` test; this was disclosed to the user. No further real-DOpus test or navigation was run for this follow-up.
## Fixed NxTools Validation package — 2026-10-04

- Rebuilt into the existing validation package directory `artifacts/win-x64-validation-20261004` and updated the existing installer `artifacts/installer-validation-20261004/NxTools.Next-validation-20261004-setup.exe` (14,281,434 bytes; SHA-256 `55A6DCFE4755C1C5BD5CB0C7EF752FDC442C5C7252785F5E6DF56E034D4698EF`). The original **NxTools Validation** AppId and `%LOCALAPPDATA%\NxTools.Next-Validation-20261004` install path are unchanged; no new DOpus-specific identity is used. The package also skips automatic protected Indexer service management for this Validation path. The BrowserHost entry was already hash-identical and was intentionally not recopied while an active Native Messaging client held it open.
- Fixed validation installer instructions: `artifacts/installer-validation-20261004/INSTALL.md`. The fixed Validation installer was successfully installed; the Validation desktop was not launched. Actual DOpus tab/path-switch and already-open file-dialog acceptance remains pending. Keep this task `in_progress` until manual acceptance.
