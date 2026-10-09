# Implementation Plan

## Ordered Steps

1. Add red probe/layout tests for duplicate resize suppression, stale-generation rejection, and compact/confirmed/error footer geometry.
2. Add a red real GPUI handoff test that observes the task window registration and native command order: stash, hidden creation, placement, visibility, then source close.
3. Add a red motion/lifecycle regression proving the incoming task window uses a bounded generation-keyed content animation and no timer resend loop.
4. Implement the layout request guard and route every post-HWND new-download resize through the native-aware helper. Remove the Rust five-delay retry loop; retain C# native retry behavior.
5. Implement hidden task-window creation/reveal after `MoveWindowCenteredOverKey`, preserving activation and registration cleanup. Apply the same ordering to browser Blob import.
6. Add probe and task-surface GPUI transitions using existing motion/easing patterns; keep them short, non-looping, and generation-scoped.
7. Run focused tests and repair only concrete failures. Then run the relevant full desktop suite and PlatformHost tests.
8. Manually exercise browser probe success/failure, user drag before probe completion, submit/queue-only/scheduled paths, task retry, completion, and reveal/open actions.
9. Update the durable reference/spec note only if the final implementation establishes a new reusable window lifecycle invariant; remove temporary probes.

## Expected Files

- `rust/apps/nxtools-desktop/src/downloads/new_download_view.rs`
- `rust/apps/nxtools-desktop/src/downloads/browser_handoff_impl.rs`
- `rust/apps/nxtools-desktop/src/downloads/task_windows_impl.rs`
- `rust/apps/nxtools-desktop/src/downloads/task_window.rs`
- `rust/apps/nxtools-desktop/src/downloads/tests.rs`
- `rust/apps/nxtools-desktop/src/ui/motion.rs` only if a named token is needed
- `csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.NativeMethods.cs` only if a missing visibility/geometry seam is proven; prefer existing commands
- `csharp/NxTools.PlatformHost.Tests/Program.cs` only for a native geometry regression not covered by current tests

## Validation

```powershell
$env:CARGO_BUILD_JOBS = "1"
$env:CARGO_INCREMENTAL = "0"
$env:RUSTFLAGS = "-C codegen-units=256"
cargo test -p nxtools-desktop --bin nxtools-desktop <focused-download-window-tests>
dotnet run --project csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj --no-restore <focused-window-geometry-tests>
dotnet run --project csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj --no-restore
```

Long Rust/build checks must use the committed wrappers or the required memory/concurrency preflight. Do not start a duplicate Cargo process after a wrapper timeout. Record manual visual verification separately.

Automated checks completed: footer action vertical-baseline/edge-inset regression; result-driven completed-window success/failure/timeout/disconnect regression; clipboard panel pending-before-send, failed-result, disconnect, late-result and stale-preparation GPUI regression; caller-supplied IPC command-ID/result-routing and duplicate-ID tests; full desktop test binary (423 tests); full `nxtools-ipc` suite (21 tests); desktop clippy with `-D warnings`; `cargo check`; formatting; all five desktop/Quick Launch/clipboard architecture gates; and scoped diff checks.

Packaging: Release commit headroom preflight passed (22.89 GiB; 6 GiB required). The normal package wrapper's full workspace run had four timing failures in `nxtools-download` localhost/HLS fixtures; those four passed individually, and a serialized retry had one HLS bandwidth-wait timing test fail while that test passed individually. The package was therefore built with `-SkipVerification` after the separate Rust/C#/architecture gates above; Release Cargo build, PlatformHost publish, package PE/icon gates, and Inno Setup compilation succeeded. Installer: `artifacts/installer/NxTools.Next-setup-0.7.0.exe` (15,113,331 bytes; SHA-256 `57ECEF6C623F9AA4D5C6CF11F33ADA9F8FBEA20AECED0D6A12BBDC9BD2DCC42B`).

Still required before task completion: interactive Windows smoke for Explorer foreground activation, native drag/resize, browser probe/handoff, scheduled submission, and retry/completion; screenshot/manual visual acceptance for the supplied download references. The PlatformHost real-Explorer HWND test skipped because no interactive Explorer window was available.
## Rollback Points

- Before code: the new geometry/handoff tests must be red.
- After layout guard: focused probe tests pass before changing window ordering.
- After hidden handoff: command order and task lifecycle tests pass before adding motion.
- Before finish: full relevant suites pass, no `DEBUG-...` instrumentation remains, and the existing DOpus fix is untouched.

## User-reported DOpus/tab and in-flight probe follow-up — 2026-10-04

- Refreshed the same tracked Directory Opus HWND/session's active pane path before a known file dialog consumes a pending Auto Quick Switch; same-window selection, title/name and focus events coalesce passive refreshes of the captured pane. Path changes create one sync, unchanged paths do not; refresh failure leaves the file dialog untouched rather than navigating a stale directory.
- Replaced the cross-process-HWND resize timer attempt with target HWND + requested logical dimensions retained at the native centered-resize boundary. The global `EVENT_SYSTEM_MOVESIZEEND` hook now re-applies that requested size around the target window's live center after the user finishes dragging. Rust still sends one final-size request per generation; no frame-by-frame HWND animation was added.
- Added real GPUI confirmed-probe field/inspector/footer containment assertions and a generation-keyed 120 ms opacity-only probe-inspector entrance. Motion is content-only; native bounds remain atomic and owned by C#.
- Verification: full `nxtools-desktop` test binary passed 429/429 with required serialized Cargo settings; focused probe-stage GPUI bounds test and inspector-motion test passed; `cargo fmt --all -- --check` passed. PlatformHost Debug test-project build passed 0 warnings/errors; five selected DOpus/resize tests passed; download lifecycle and UI token architecture gates passed. No full DOpus navigation harness or service/process operation was run.
- Still required: user smoke on the actual DOpus multi-pane/tab layout to confirm the named path event matches the DOpus version in use, and native probe-drag while a live probe resolves to confirm the confirmed form remains fully visible at the user's DPI. Tasks stay `in_progress` until that manual interaction gate and authorized repository closeout.
## Fixed NxTools Validation package — 2026-10-06

- Final client-height correction build: an adapter-owned client mismatch check now applies a frame-changed Win32 resize after the initial native request and after move-end replay. This addresses the observed Validation trace where requested client height was 278 but actual client height remained 176.
- Rebuilt into the existing validation package directory `artifacts/win-x64-validation-20261004` and compiled the existing installer `artifacts/installer-validation-20261004/NxTools.Next-validation-20261004-setup.exe` (14,282,969 bytes; SHA-256 `58B938EA78FF6C46D1C51B6A55FA139D99A404C24DE4E3C69F54A18CFAEE8B14`). The original **NxTools Validation** AppId and `%LOCALAPPDATA%\NxTools.Next-Validation-20261004` install path are unchanged.
- Post-fix verification passed: Rust desktop 430/430, cargo check/fmt, C# build/full suite, native HWND client-height/center/replay tests, and download/Quick Launch/UI-token architecture gates. The fixed installer was reinstalled; package and installed hashes match. No NxTools/PlatformHost/Indexer process is running; `NxToolsIndexerV1` remains stopped.
- Still required: manual smoke on the installed Validation package—drag the live GPUI probe window before completion, confirm the client height visibly expands and the directory/inspector/footer are not masked, then exercise retry/failure/completion and real DOpus tab/path behavior. Keep tasks `in_progress` until user acceptance.
- Still required: manual smoke on the installed Validation package—drag the actual GPUI probe window before completion, confirm the form at the user's DPI, exercise retry/failure/completion, and verify real DOpus multi-pane/tab behavior. Keep tasks `in_progress` until user acceptance.
## Follow-up verification — 2026-10-06 WM_SIZE synchronization

- Closed the remaining native/GPUI boundary gap in `WindowsPlatformAdapter.ClientResize.cs`: when the requested client dimensions already matched `GetClientRect`, PlatformHost now posts one same-size `WM_SIZE` so GPUI refreshes its logical viewport and renderer/layout state. This is bounded and idempotent; no additional timer or center correction was added.
- Rebuilt the existing Validation package and installer: `artifacts/installer-validation-20261004/NxTools.Next-validation-20261004-setup.exe`, 14,284,190 bytes, SHA-256 `7ADEBA13808C8426C1C94BD49D81FCB353FDC667604679FBB61F346DB1CA5B21`; AppId `B6C4B521-8BB5-4F84-A684-66900AD931FE` and `%LOCALAPPDATA%\\NxTools.Next-Validation-20261004` remain unchanged.
- Verification passed: desktop 430/430 with serialized Cargo settings, cargo fmt, PlatformHost/Tests builds with 0 warnings/errors, safe PlatformHost set 180/180 excluding real DOpus lister/navigation tests, native HWND resize/replay tests, download/Quick Launch/UI-token architecture gates, package-memory gate and diff check. `Start-Process -Wait` installation completed; package and installed hashes match; no NxTools/PlatformHost/Indexer process is running and the Indexer service was not touched.
- Still required: installed Validation visual smoke at the user's DPI, including drag-during-probe, retry/failure/completion and real DOpus multi-pane/tab behavior. Keep this task `in_progress` until user acceptance.
## Follow-up verification — 2026-10-07 unified blacklist and probe transition

- Probe completion now submits the GPUI/native resize before notifying the confirmed-state render, reducing the visible compact-to-confirmed transition window without adding animation or retry loops.
- Added the shared Rust process blacklist with independent Clipboard/Quick Launch scope flags and Clipboard settings controls; the existing Direct3D exclusive-fullscreen suppression remains an independent Quick Launch guard.
- Verification: core 34/34, desktop 430/430, focused native HWND resize/replay tests, cargo fmt/check, download/Quick Launch/UI-token gates, Release package and fixed Validation installer. Installed package hashes match; no product processes are running and `NxToolsIndexerV1` is Stopped.
- Still required: manual installed Validation smoke at the user's DPI, blacklist behavior confirmation, retry/failure/completion, and real DOpus multi-pane/tab behavior.

## Fresh probe-drag investigation and feature review — 2026-10-08

- Latest native log still reported requested height 278, final client 176 and recovery success. Old ordinary `#32770` regressions did not exercise the custom-client mismatch branch.
- Added a real custom-client-chrome HWND regression at production resize/replay boundaries. It first failed with `EntryPointNotFoundException` for `ClientResizeGetWindowRect`; after export-name repair, the same regression failed on reported success with final height 176.
- Corrected four prefixed Win32 P/Invoke export bindings. Visible-center correction now precedes final client correction/readback; recovery retires only after this full chain. Pending snapshot owns requested dimensions, and successful recovery logs `client_size_settled`.
- Removed the competing GPUI native resize for real HWNDs. Native WM_SIZE updates the viewport. The bounded presentation gate never releases a clipped confirmation; timeout clears unpresentable confirmation, reserves the existing error row, and permits explicit retry.
- Rust timeout regression was run red before correction. Production GPUI test now holds confirmed metadata in a compact viewport, rejects inspector rendering, then checks form/footer geometry after expansion.
- Verification: desktop 434/434; custom-client native regression 20/20 repetitions; safe PlatformHost suite 181 PASS labels with two environment SKIPs (Files/Explorer), real DOpus tests excluded; workspace check/fmt/scoped diff checks passed. Download and relevant UI/clipboard/Quick Launch/shell architecture gates passed. File-dialog-follow and browser-native-messaging gates have stale source paths; browser gate also requires an API unavailable in PowerShell 5.1. These failures are recorded, not hidden.
- Feature review document: `docs/audits/2026-10-08-download-transition-and-feature-review.md`. It separates parent-verified defects from validation gaps and rejected reviewer claims. No unrelated functional fixes were made.
- No package/install/product launch/service mutation or real DOpus navigation was performed. Fresh real-pointer probe-drag at user DPI, retry/failure/completion and manager interactions remain required. Keep this task in progress until that acceptance gate.
