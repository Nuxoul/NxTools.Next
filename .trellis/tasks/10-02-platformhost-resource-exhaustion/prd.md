# Prevent PlatformHost long-running resource exhaustion

## Goal

Prevent the recurring long-running PlatformHost failure while preserving bounded Windows Shell/UI Automation operations and keeping the platform-command host responsive. Do not claim that this also fixes the separate `0xc0000005` dialog unless a matching crash record or reproduction proves the connection.

## Confirmed evidence

- Windows Application Error / .NET Runtime records identify repeated `NxTools.PlatformHost.exe` failures on 2026-09-26, 09-27, 09-29 and 09-30. The matching Event 1026 exception is `System.Runtime.InteropServices.COMException (0x80070486)`; Windows describes it as exhaustion of the process allowance for Window Manager object handles.
- The recorded stack is `System.Threading.Monitor.ReliableEnter` -> `System.Threading.ManualResetEventSlim.Set` -> `NxTools.PlatformHost.Windows.BoundedStaOperationRunner.TryRun` worker completion.
- A 97 MB PlatformHost crash dump exists near the 2026-09-29 failure. Its inspected managed stacks do not independently identify the originating resource owner.
- The running PlatformHost image is under `C:\ProgramData\NxTools\NxTools.Next\Service` and its hash/product version differ from the workspace Release package. Do not replace or restart it as part of this task.
- Event 1023 on 2026-10-02 reports a .NET Runtime internal error (`0x80131506`). No matching PlatformHost Application Error `0xc0000005` record was found in the last 48 hours; the exact dialog remains uncorrelated.
- The working tree already contains uncommitted `BoundedStaOperationRunner.TrySignalCompletion` protection and its test. Preserve and verify those changes; the installed service image does not match the current source/package identity.

## Requirements

- Identify and close the concrete completion/wait/cancellation resource-lifetime gap exercised by `BoundedStaOperationRunner`, including successful completion, worker exceptions, timeout, cancellation, and a worker that returns after its caller has timed out.
- Keep each provider operation on its dedicated STA thread, preserve existing timeout and maximum-concurrency semantics, and prevent completion signalling failures from escaping as unhandled worker-thread exceptions.
- Add a regression test at the runner boundary that exercises repeated operations and deterministically verifies bounded resource growth / completion-object disposal. Cover late worker completion after timeout or cancellation so disposal never races a waiter or a still-running worker.
- Keep the command protocol and user-visible operation results compatible. Any inability to complete an operation remains a bounded failure, not an unbounded wait or a PlatformHost process crash.
- Keep diagnostics bounded and free of user paths, target values, or other operation payloads. Do not add temporary `DEBUG-...` probes to production code.
- Preserve unrelated working-tree changes and do not stop, restart, replace, or package the currently installed/running PlatformHost as part of this task.

## Acceptance criteria

 - [x] **Regression:** The real `TryRun` regression failed before the fix after 96 bounded STA completions (`219 -> 543` process handles) and passes after the fix; the final version alternates successful and throwing providers across 192 operations with a bounded allowance for CLR thread-wrapper collection noise.
 - [x] **Lifecycle:** Success, provider exception, timeout, caller cancellation and late worker completion remain bounded. Completion signal and cancellation source are released only after caller and worker owners finish.
 - [x] **Signal failure:** The existing injected completion-signal failure test passes and proves the exception cannot escape as an unhandled STA-thread exception.
 - [x] **Suite:** The complete PlatformHost harness passes, 170/170, including timeout/concurrency, detached operations and Explorer/Files/OneCommander coverage.
 - [x] **Scope / rollout:** Product changes are limited to runner lifetime code and tests; no IPC payload or download/UI code changed. The installed service was not stopped, restarted, replaced or packaged; rollout remains separate.
 - [x] **Dialog:** The `0xc0000005` dialog remains uncorrelated with the recorded `0x80070486` runner failures; no claim is made that this fix addresses it.

## Out of scope

- Changing the IPC protocol, command semantics, or the number/architecture of STA workers.
- Reworking OneCommander/UI Automation behavior unrelated to the confirmed completion stack.
- Packaging, deploying, restarting, or terminating any desktop/PlatformHost/indexer/service process.
- Treating the dialog text `0xc0000005` as proof that it is the same failure as the recorded `0x80070486` CLR exception.

## Open questions

- The exact `0xc0000005` dialog has not yet been correlated with a Windows crash record. Retain this as an explicit validation limitation unless a matching artifact becomes available during implementation.
