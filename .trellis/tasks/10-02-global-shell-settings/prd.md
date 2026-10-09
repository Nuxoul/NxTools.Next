# Fix Global Windows Shell Settings Apply Errors

## Goal

Make the global settings save/apply result accurate and actionable when Windows startup registration or tray-icon configuration fails.

## Confirmed Context

- Rust persists the desired configuration before queuing `ConfigureDesktopShell`; `SystemSettings` is updated only after the correlated Windows success result.
- PlatformHost applies approved desktop executable lookup, current-user Run-key registration, and tray configuration; staged `WindowsCommandResult.Data` tags already exist in the uncommitted changes.
- The current worktree diff has two syntax/compile defects in this path: Rust's `global_shell_failure_message` lacks a match arm pattern for `desktop_executable_unavailable`, and C#'s `ApplyDesktopShellConfiguration` catch block computes an error code but omits construction/return of the failure result.
- Preserve the existing fail-closed behavior for live close-to-tray policy. Do not silently roll back persisted user intent or claim the native shell change succeeded.

## Requirements

1. Repair the Rust staged failure mapping and C# exception result construction without weakening protected executable validation.
2. Preserve one correlation ID and IPC command-result routing from the user save through the native result.
3. Return a distinct actionable user message for missing approved desktop binary, startup Run-key failure, tray creation/display failure, enqueue/disconnect/timeout and unknown native errors.
4. Keep logs payload-free and include failure stage, native error code and elapsed time at the owning boundary.
5. Cover normal enable/disable combinations for startup and tray, including the requirement that either feature needing a desktop executable fails safely when no approved executable exists.

## Out of Scope

- Changing protected installation, UAC, service, registry permissions, startup command target, or executable approval rules.
- Changing whether the desired settings remain persisted after the Windows side fails.
- Broad refactoring of global settings UI.

## Acceptance Criteria

 - [x] Rust desktop target and PlatformHost build/tests compile successfully.
 - [ ] C# tests cover approved executable absent, Run key unavailable/write failure, tray create/show/release failure, and generic native exception result.
 - [x] Rust tests verify stage-to-message mapping and stale/late command results do not close the window or update live `SystemSettings` as applied.
 - [ ] Successful matching result updates shared `SystemSettings`, closes the settings window and shows success; failed result keeps the window open, shows actionable feedback, and preserves fail-closed runtime policy.
 - [ ] `cargo fmt --check`, focused desktop tests, full PlatformHost test program and desktop clippy pass under the required Rust environment.

## Out of Scope Summary

No protected files, registry entries, ACLs, service state, or global UAC policy are changed by this task.