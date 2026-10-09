# Implementation Plan: Download Dialog Visuals and Explorer Reveal

## Ordered steps

1. Before product edits, reread this task, `docs/reference-code/download.md`, the focused GPUI/UI specs, the Windows PlatformHost boundary and all existing dirty-state paths that will be edited. Keep this child independent from the older native geometry task; do not claim its pending drag/resize smoke has passed.
2. Add a red production-GPUI bounds regression for the schedule Popover `time row`, actual Clear/Confirm buttons, action row, shell and parent. Assert shared button tier/center baseline, both visible hit targets are inside the action row/shell, horizontal insets and gaps are stable, and parent bounds remain unchanged. First run the focused test to observe the current bounds/gap.
3. Add named debug selectors only at real visible GPUI elements where the current `download-new-schedule-clear-action` / `download-new-schedule-confirm` wrappers do not uniquely measure the button hit target. Do not replace the real Button callback or loosen the production test to match bad geometry.
4. Implement the schedule layout from the approved plan: calendar, a separate compact hour/minute row, and a stable `清除` / `确定` action footer. Keep draft, commit, clear, trigger/outside-dismiss focus and generation/lifecycle behavior. Tune only measured dimensions required to prevent actual clipping in the existing bounded Popover.
5. Build one small coherent presentation hierarchy across add/probe, progress, failed and completed content using existing Theme/type/action tokens. Retain the existing shared task-window shell, task identity, state ownership, progress/path/error semantics and each phase's correct actions. Add production `debug_bounds` assertions for real file identity, state/progress, path, error, action-row containment, primary/secondary hierarchy and one task window across transitions; verify any changed size uses the existing native center boundary, not cached GPUI origin.
6. Before shell changes, add a pure/injected C# regression seam for Explorer resolution, launching/selection and activation/readback. Red cases: already-visible matching HWND, matching minimized HWND, Explorer shell process reuse, delayed HWND publication, activation denied, the wrong window foreground, foreground child of the exact expected Explorer root, and target destroyed/mismatched. Assert an existing file uses `/select,"file"` even when its parent directory HWND already exists; an existing directory opens directly whether reused or newly launched; a missing final file falls back to the task save directory.
7. Keep all native USER32/process calls behind a small injectable adapter; make a testable helper accept target kind, desired Explorer/root HWND and observed foreground root. Bound retry count/time and return failure with stage/native error if the exact intended root never becomes foreground. Never treat process launch, `BringWindowToTop`, or the BOOL from `SetForegroundWindow` alone as success. Do not add unbounded retry, focus stealing/topmost pulses, a log/title/path, or a new IPC field unless a red integration test proves the visible caller HWND is required.
8. Fix Explorer selection/activation in the owning C# boundary; preserve current current-user path validation and dedicated STA shell command. Retain correlation, accepted/start, coarse shell stage, terminal result, elapsed time and native error diagnostics, without URL/path/name/title/header/cookie payloads.
9. Keep the Rust terminal action identity contract. Verify failed native result, timeout, disconnect and late result continue to keep or suppress close; only a correlated foreground-confirmed result for the original native HWND + generation may close. Add/update the GPUI artifact action test only if the C# result contract changes. Cover existing-file/ directory selection and deleted-final-file -> save-directory fallback at the Rust/C# owner seam.
10. Run focused red/green test groups, Rust formatter, desktop full test binary and scoped architecture gates, C# focused Explorer regression then the PlatformHost non-DOpus subset/build; comply with workstation Cargo memory flags. Never run the side-effecting DOpus navigation harness, build/installer/package, install/launch an alternate build, or touch/restart protected service processes in this slice.
11. If enabled and safe, manually verify in the real Windows interactive desktop at native scaling: the supplied download windows, schedule open/dismiss/click targets, failed/retry/completed state, reveal while matching Explorer exists and when it must be started/reused, file selection, and retry surface when foreground activation is unavailable. Capture visual/terminal evidence. If real Explorer/UI is not available, report it as an explicit pending manual gate.
12. Remove temporary debug probes; run final focused Rust/C# gates plus download UI lifecycle architecture and scoped diff review; only then update the task execution log. No packaging or release/deployment in this plan.

## Expected product/test files

- `rust/apps/nxtools-desktop/src/downloads/new_download_view.rs` — schedule and add/probe presentation only if measured.
- `rust/apps/nxtools-desktop/src/downloads/task_window.rs`, `window_support.rs` — shared content/action visual changes; preserve task-window lifecycle/native drag.
- `rust/apps/nxtools-desktop/src/downloads/tests.rs` — real production GPUI schedule, content bounds and current behavior gates.
- `rust/apps/nxtools-desktop/src/downloads.rs` — only if result routing needs a proven integration correction; preserve existing action correlation/generation.
- `csharp/NxTools.PlatformHost/Windows/ExplorerShellActivation.cs`, `ClipboardShellTarget.cs` — testable Windows shell target selection, Explorer focus/readback and error code ownership.
- `csharp/NxTools.PlatformHost.Tests/Program.cs` and, if needed, one small internal adapter/interface file — injected activation tests, not DOpus navigation.
- `docs/reference-code/download.md` — newly verified Windows Shell compatibility/evidence, if implementation confirms the seam.

## Validation commands (execute after task receives implementation approval)

From PowerShell / repository root:

```powershell
$env:CARGO_BUILD_JOBS = "1"
$env:CARGO_INCREMENTAL = "0"
$env:RUSTFLAGS = "-C codegen-units=256"
cargo test -p nxtools-desktop --bin nxtools-desktop production_delayed_start_uses_shared_popover_and_preserves_draft_lifecycle -- --exact
cargo test -p nxtools-desktop --bin nxtools-desktop production_task_window_reuses_one_skeleton_and_generation_across_phases -- --exact
cargo test -p nxtools-desktop --bin nxtools-desktop
cargo fmt --check --all
& .\tests\architecture\download-ui-lifecycle.ps1

dotnet run --project csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj --no-restore "Explorer activation"  # Replace with the added regression's exact test-name substring; verify an actual RUN/PASS line.
dotnet run --project csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj --no-restore "shell target launch does not block later platform commands"
dotnet build csharp\NxTools.PlatformHost\NxTools.PlatformHost.csproj --no-restore
```

Resolve any remaining unknown exact C# test name after adding the regression and require a visible RUN/PASS line. The second C# command is a real existing non-DOpus responsiveness test; suite/build selection must exclude the DOpus `C:\Windows` navigation/restoration harness. No Release packaging command is authorized by this plan.

## Rollback points

- Revert only the proposed Popover/content layout if production bounds, draft lifecycle or visual assessment regresses; retain the newly added meaningful failure test if it records the user symptom.
- Revert Explorer helper changes if the injected C# test cannot prove correct matching/root foreground or if a failed activation is still reported as success. Keep the existing Rust result-driven retry/close contract.
- Do not revert, reformat wholesale, stage or commit unrelated pre-existing dirty paths.

## Execution results — 2026-10-03

Completed the approved implementation slice:

- Added production GPUI bounds assertions for the schedule time row, action row, and actual Clear/Confirm button hit targets; moved the controls into distinct aligned rows and compacted the calendar cells to stay within the existing bounded Popover. Draft/commit/clear/outside-dismiss coverage remains passing.
- Kept one keyed task-window shell and its phase lifecycle. The compact progress/failed presentation now uses destination metadata beneath file identity/status; completion keeps its compact identity and separated action footer.
- Added injected Explorer activation tests and native result readback. Existing-file Reveal always requests `/select`, directory targets open directly, stale/destroyed HWNDs and unrelated foreground windows cannot report success, and the retry loop is bounded. Missing-file Reveal resolves to the task save directory; Open/drag do not fall back to a directory.
- Reveal only closes the original window generation after correlated foreground-confirmed success. Failure, timeout, disconnect, duplicate requests and late success retain/suppress close. Diagnostics carry the command correlation, fixed stage, elapsed time and native error only; no path/title/file-name payload is recorded.
 - Verification passed: `cargo fmt --all -- --check`; all 428 `nxtools-desktop` tests; `tests/architecture/download-ui-lifecycle.ps1`; five PlatformHost Explorer activation cases; the existing non-DOpus `shell target launch does not block later platform commands` test; and PlatformHost Debug build (0 warnings/errors). Rust runs used the required single-job memory settings.

## Pending manual acceptance

- The supplied screenshots could not be previewed in this environment. At native Windows DPI, manually inspect the add/probe, progress, failure, completion and schedule surfaces; exercise schedule hit targets; and confirm Explorer foreground/file-selection behavior with both existing and newly opened Explorer windows. Automated GPUI/Win32 seams do not replace this check.
- Do not run the DOpus navigation harness, package/install/deploy, or touch protected services as part of this follow-up.

## Isolated Release validation package — 2026-10-04

- On explicit user request, built current Release binaries with `scripts/package.ps1 -Configuration Release -OutputPath artifacts\win-x64-validation-20261004 -SkipVerification -SkipInstaller -MinimumCommitHeadroomGiB 6.0`; the committed-memory preflight passed at 18.43 GiB. The package wrapper set the repository's serialized Rust build environment.
- Rust Release build, PlatformHost Release publish, desktop PE-subsystem gate, product-icon gate and package-memory-policy gate passed. All five packaged executable SHA-256 values match their Release build/publish outputs. The generated `ReleaseBinaryManifest.g.cs` was restored to its pre-package working-tree bytes.
- Compiled `artifacts/installer-validation-20261004/NxTools.Next-validation-20261004-setup.exe` (15,121,494 bytes; SHA-256 `FE8743EBB79C750F0B67EA061546F48A1310C787D4C1A8BC30AEAECF3EBE8DE3`) using a separate NxTools Validation AppId and `%LOCALAPPDATA%` destination, with no startup registration, auto-launch or service-stop actions. See the adjacent `INSTALL.md`.
- The formal 0.7.0 installer hash remains `57ECEF6C623F9AA4D5C6CF11F33ADA9F8FBEA20AECED0D6A12BBDC9BD2DCC42B`; the 2026-10-03 validation installer is unchanged. The installer was not run and no service/process was stopped/restarted. Full build verification/harness was skipped to avoid the known DOpus navigation side effect. Real DPI, Explorer and window-interaction acceptance remains pending.