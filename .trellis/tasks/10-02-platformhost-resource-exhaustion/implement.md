# Implementation plan

1. Re-read this task's PRD/design, `.trellis/spec/guides/cross-layer-thinking-guide.md`, and the applicable `nxtools-ipc` backend quality/logging guidance. Re-read the current runner and tests from the live worktree; preserve all dirty changes.
2. Build the tight regression first at `BoundedStaOperationRunner.TryRun`: prove repeated bounded operations do not accumulate completion/wait/cancellation resources, and exercise success, provider exception, timeout/cancel, late worker completion, and signal failure. Make the test red against the selected existing behavior before fixing it.
3. Implement the minimal ownership/disposal protocol in `csharp/NxTools.PlatformHost/Windows/BoundedStaOperationRunner.cs`. Keep the STA boundary, existing caller signatures, result/error contract, timeout and concurrency limits. Do not alter `WindowsPlatformAdapter` or the IPC protocol unless the regression proves the failure crosses that boundary.
4. Run the focused runner regression first, then the complete `NxTools.PlatformHost.Tests` harness:
   `dotnet run --project csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj --no-restore`
5. Run C# build/test with the repository's normal configuration; review a scoped diff and verify no unexpected line-ending churn or unrelated edits. No Cargo verification is relevant unless Rust is changed.
6. Recheck Windows Application/WER events and dump evidence only if a new occurrence is safely available. Do not deliberately exhaust OS handles or destabilize the user's running host to force a crash.
7. Do not stop/restart the current process, overwrite `artifacts\win-x64`, or install/package the new binary. Record deployment as a separate acceptance step requiring explicit release approval and exact registered-binary identity verification.
8. Report the confirmed `0x80070486` path separately from the currently uncorrelated `0xc0000005` dialog. Do not claim the latter is fixed without its own red-capable signal.

## Expected product files

- `csharp/NxTools.PlatformHost/Windows/BoundedStaOperationRunner.cs`
- `csharp/NxTools.PlatformHost.Tests/Program.cs`

Only add another product file if the failing boundary test proves it is required.

## Verified results (2026-10-02)

- Red-capable before fix: the real `TryRun` handle-growth regression failed after 96 bounded STA completions (`baseline=219`, `final=543`).
- `dotnet build csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj --no-restore -v:minimal` — passed, 0 warnings, 0 errors.
- Focused `BoundedStaOperationsReleaseCompletionWaitHandles` — passed with 192 alternating successful/provider-exception operations.
- Focused `BoundedStaOperationsCancelLateWork` — passed for timeout and caller cancellation while the provider continued running; same-name work remained blocked until worker exit.
- `dotnet run --project csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj --no-build --no-restore` — passed, 170/170.
- `dotnet format ... --verify-no-changes ... --include BoundedStaOperationRunner.cs` — passed. `Program.cs` was not rewritten for formatter-only CRLF diagnostics in unrelated existing end-of-file changes.
- `git diff --check` on the two product paths — passed.
- Installed service hash remained unchanged. No process was stopped/restarted and no package/deployment was performed. `0xc0000005` remains uncorrelated with the historical runner `0x80070486` failures.
