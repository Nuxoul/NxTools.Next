# Design: PlatformHost completion-resource ownership

## Boundary and confirmed fault path

```text
PlatformHost caller
  -> BoundedStaOperationRunner.TryRun
      -> dedicated STA worker runs COM/UIA provider
      -> worker publishes result/error and signals completion
  -> synchronous caller observes completion, timeout, or cancellation
```

The confirmed Event 1026 stack fails in `ManualResetEventSlim.Set` on the worker's completion path, after repeated PlatformHost use. `TryRun` also creates a linked `CancellationTokenSource` for each operation. The current source/test diff includes a best-effort signal exception guard but neither the crash event nor the available dump proves which managed/native resource quota was exhausted. Treat undisposed completion/worker resources as a concrete candidate to test, not as an already proven root cause.

## Intended contract

- The caller owns the right to wait until it leaves the wait operation.
- The worker owns its operation token until provider code has returned, including when the caller times out or cancels.
- A completion primitive may be disposed only after both parties can no longer access it. Timeout/cancellation must not cause a late worker to signal a disposed primitive; worker completion must not dispose a primitive while the caller is still waking from its wait.
- The worker remains STA, the caller's wait remains bounded/cancellable, provider concurrency stays capped by the existing `maxConcurrency` argument, and results/errors retain current meanings.
- Signal failures are captured at the worker boundary. They must not become unhandled thread exceptions; if the signal cannot wake the caller, that caller still exits through its existing bound.

## Design approach

1. Establish a failing regression at the real `TryRun` seam. Prefer deterministic ownership/disposal assertions using a small internal completion abstraction/factory if that is the narrowest reliable seam; supplement with a repeated-operation process-handle stress check only if it is stable on Windows CI/workstations.
2. Exercise both ordinary completion and a provider that deliberately outlives caller timeout/cancellation. Verify each resource has exactly one eventual owner/disposal and that neither party touches it afterward.
3. Implement the smallest lifecycle/ownership change in `BoundedStaOperationRunner.cs`. Keep any injected factory internal to tests. Avoid broad polling, unbounded thread growth, or weakening provider cancellation/concurrency contracts.
4. Preserve the existing uncommitted `TrySignalCompletion` protection and its test; revise it only where the failing real-boundary regression demonstrates necessity.

## Compatibility and operational safety

- No wire-protocol or UI behavior changes are intended.
- Do not use the currently running service as a test fixture and do not stop/restart it.
- The service binary currently differs from both the workspace package and source; passing source tests does not mean the installed instance is fixed.
- No package overwrite/deployment is part of this task. Any later authorized release must obey the repository's exact-path process/service ownership and memory-headroom rules.
- The separate `0xc0000005` dialog remains unresolved unless it is matched to this code path through a fresh event/dump or red-capable reproduction.

## Diagnostics

This is an in-process worker-lifecycle fix, not a new cross-process operation. Reuse existing bounded operation diagnostics if any are needed. Do not log operation inputs, paths, UI Automation values, or exception messages that can carry user data. Temporary probes require a unique `DEBUG-...` marker and removal before completion.

## Rollback

Revert only the runner lifecycle change and its regression test if the test demonstrates a compatibility regression. Preserve the existing working-tree guard/test changes and unrelated edits. Do not roll back or overwrite installed artifacts as part of source rollback.
