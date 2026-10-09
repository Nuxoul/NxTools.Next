# Implementation Plan

## Ordered Steps

1. Review the child PRDs and reference notes again immediately before implementation; keep the DOpus fix outside this task.
2. Add red link-routing regressions for an existing file, directory, web target, bare domain, copy-only mode, echo suppression, and ignored nonexistent path. Assert that the local file reaches `SelectInExplorer` rather than browser URL construction.
3. Add red download regressions at the real GPUI boundary for one resize request per probe generation, stale-generation rejection, stable error/footer geometry, hidden replacement placement, and one task-window identity across progress/terminal transitions.
4. Implement the typed detector result and pure route decision in Rust. Preserve close-before-copy and generation-scoped panel lifecycle. Keep C# shell validation unchanged.
5. Implement the generation-scoped new-download layout guard and route all real-HWND size changes through the native-aware helper. Remove redundant Rust timer resends; rely on the existing C# native retry boundary.
6. Implement the probe-to-task handoff: stash before source retirement, create/register the replacement hidden, queue placement then visibility, and preserve the existing activation/close recovery paths.
7. Add restrained GPUI content transitions using shared motion tokens. Keep animation keys tied to a window/probe generation and do not add high-frequency native animation commands.
8. Add the bounded lifecycle diagnostics and remove any raw target/path/URL fields from the link-panel click logs.
9. Run focused tests, fix only concrete in-scope failures, then run the broader desktop and PlatformHost checks.
10. Perform a manual Windows smoke test with the reported `Throne.exe` path, a directory path, an HTTPS URL, a browser-captured probe, a dragged probe window, a submit handoff, retry, completion, and reveal/open actions.
11. Update reference/spec notes only if implementation confirms a durable lifecycle invariant; remove temporary `DEBUG-...` probes before finish.

## Expected Files

- `rust/apps/nxtools-desktop/src/tools/link_detector.rs`: target kind and detection tests.
- `rust/apps/nxtools-desktop/src/tools/link_panel.rs`: pure route mapping, click behavior, and interaction tests.
- `rust/apps/nxtools-desktop/src/clipboard.rs`: typed link-panel command dispatch/diagnostic boundary if needed.
- `rust/apps/nxtools-desktop/src/downloads/new_download_view.rs`: layout request coalescing, native resize calls, and probe transition motion.
- `rust/apps/nxtools-desktop/src/downloads/browser_handoff_impl.rs`: hidden replacement handoff ordering if the shared open path needs a source-specific hook.
- `rust/apps/nxtools-desktop/src/downloads/task_windows_impl.rs`: hidden task-window creation, native placement/reveal, and handoff diagnostics.
- `rust/apps/nxtools-desktop/src/downloads/task_window.rs`: bounded entering/phase content animation.
- `rust/apps/nxtools-desktop/src/ui/motion.rs`: only if existing duration tokens need a named transition token.
- `rust/apps/nxtools-desktop/src/quick_launch/tests.rs` or `rust/apps/nxtools-desktop/src/downloads/tests.rs`: focused regressions where ownership makes the test most direct.
- `csharp/NxTools.PlatformHost.Tests/Program.cs`: only if the existing shell/native geometry contract needs a missing focused assertion; do not alter `ClipboardShellTarget` semantics.

## Validation Commands

Focused Rust checks should use the repository-required environment on this workstation:

```powershell
$env:CARGO_BUILD_JOBS = "1"
$env:CARGO_INCREMENTAL = "0"
$env:RUSTFLAGS = "-C codegen-units=256"
cargo test -p nxtools-desktop --bin nxtools-desktop <focused-link-and-download-tests>
```

PlatformHost checks:

```powershell
dotnet run --project csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj --no-restore <focused-shell-and-window-tests>
dotnet run --project csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj --no-restore
```

Before any long Rust verification, use the committed wrapper/preflight rules from the repository and do not start a duplicate Cargo build after a timeout. Run the relevant desktop test target and static checks after the focused tests. Record the manual Windows smoke result separately from automated tests.

## Rollback Points

- Before implementation: each new red boundary regression must fail for the current behavior.
- After detector routing: focused link tests must pass before touching download lifecycle code.
- After layout guard: probe geometry tests must pass before changing handoff ordering.
- After hidden replacement handoff: task-window lifecycle and native command-order tests must pass before adding motion.
- Before completion: full relevant suites pass, no temporary probes remain, and the existing DOpus behavior is unchanged.
