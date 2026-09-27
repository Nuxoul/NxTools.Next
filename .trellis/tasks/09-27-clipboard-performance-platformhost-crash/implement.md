# Clipboard Performance and PlatformHost Crash Implementation Plan

1. Complete planning evidence: preserve the EcoPaste review and NxTools evidence artifacts; resolve the performance acceptance target with the user using boundedness/relative regression as the recommended initial contract.
2. Repair `tests/performance/clipboard-panel-cpu.ps1` for the current PowerShell C# compiler, then run it once against the packaged desktop to establish a real baseline.
3. Add an unattended Rust clipboard fixture with 10k/100k rows, first/deep pages, content/extended search, delete/refresh cycles, and SQLite query-plan/timing counters. Add a worker stress seam that can block a query and count obsolete work executed.
4. Add a red-capable UI/worker stress path for rapid search/delete generations. Assert latest-result authority, bounded queue/retained rows, and no duplicate page append. Keep clipboard payloads out of logs and fixtures where unnecessary.
5. Review and measure the bounded-window/list-summary options. Implement only the smallest change proven by the loop: query coalescing/cancellation, bounded range cache, SQL count/search optimization, or payload/render deferral.
6. Add PlatformHost capture-tail/async boundary diagnostics and an unattended clipboard soak. Capture process liveness, private memory, event drops, exception type/HResult/stack context, and WER/dump correlation without payload secrets.
7. Add regression tests for the confirmed root cause and rerun the original scroll/search/delete loop. If crash attribution remains impossible without user WER/dump evidence, document the boundary hardening and evidence gap instead of claiming a module.
8. Run targeted Rust/C# tests, desktop tests, `cargo fmt`, clippy/check gates, and scoped diff checks; review unrelated dirty files before any commit.

Risky files/boundaries:
- `rust/apps/nxtools-desktop/src/ui/mod.rs`
- `rust/apps/nxtools-desktop/src/clipboard_worker.rs`
- `rust/crates/nxtools-clipboard/src/lib.rs`
- `csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.EventWindow.cs`
- `csharp/NxTools.PlatformHost/Protocol/PlatformPipeClient.cs`
- `tests/performance/clipboard-panel-cpu.ps1`

Required stop conditions:
- Do not start product implementation before a final planning summary is approved and `task.py start` is run.
- Do not attribute `0xe0434352` without faulting-module, stack, dump, or WER evidence.
- Do not copy EcoPaste/Ditto code or add an unmeasured replacement architecture.

## Implementation Outcome

- Completed the deterministic 10k/100k SQLite benchmark, known-total optimization, worker query supersession, cursor forward pagination, and bounded 320-row bidirectional history window.
- Added regression coverage for cursor/offset equivalence, worker pre-execution supersession, shared-entry append behavior, and the bounded window request/eviction contract.
- Added PlatformHost process-level exception diagnostics, async clipboard publication containment, payload-free logging resilience, and the repaired PowerShell harness compiler boundary.
- Verification is green for the 39 clipboard tests, 414 desktop tests, PlatformHost build/test suite, formatting, and scoped diff checks.
- Remaining evidence work is the packaged interactive UI/clipboard-storm soak and user-provided WER/dump/stack correlation for `0xe0434352`; neither is claimed complete.

## Review Fix Outcome

- Review findings were fixed directly in the current task scope; the final Rust desktop suite passed 414 tests, clipboard tests passed 39 tests, download core/contract tests passed when run with serialized fixtures, and PlatformHost build/tests passed.
- The first combined download run exposed loopback/FFmpeg fixture contention; affected tests were rerun individually and the full engine suite was rerun with serialized test threads.
- No commit or unrelated worktree cleanup has been performed yet.
