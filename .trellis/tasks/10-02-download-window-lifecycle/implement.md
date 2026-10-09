# Implementation Plan

1. Re-read `docs/reference-code/download.md` and the current canonical task PRD/design/implement; preserve exact phase title/order.
2. Add a red test for user drag during probe and native resize, plus actual position through handoff and terminal transition.
3. Inspect the native geometry owner and fix only the proven stale-boundary or ordering defect; reuse existing native commands.
4. Add no duplicate thread/segment/detail UI. Strengthen only missing focused regressions for current dynamic count, dots, disclosure/resize and phase reset.
5. Run focused GPUI tests, relevant desktop test suite, `tests/architecture/download-ui-lifecycle.ps1`, PlatformHost geometry suite, fmt/clippy.
6. Perform interactive Windows smoke with drag-during-probe, confirm, submit, retry, completion and failure; record any unavailable case honestly.

Expected files only if red tests demonstrate need: `downloads/new_download_view.rs`, `downloads/task_windows_impl.rs`, `downloads/task_window.rs`, `downloads/tests.rs`, `csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.NativeMethods.cs`, and its tests.

## Verified work (2026-10-02)

- Re-read `docs/reference-code/download.md`, this task's `reference-scope.md`, and canonical phase/order in `.trellis/tasks/09-28-download-window-transition-polish/implement.md` (preserved; no phase reordered).
- Rechecked old NxTools `MainWindow.axaml.cs`, `DownloadProgressWindow.axaml.cs/.axaml`, and `DownloadCompletedWindow.axaml.cs`: one task-keyed progress window is reused; completion transforms the open progress window; centered resize uses its current native position; the fallback completion window remains separate and passive.
- Verified current call chain: generation-matched `NewDownloadView` probe result -> one `SetWindowBoundsCentered` command -> C# reads actual visible HWND bounds, converts DWM frame insets and applies `SetWindowPos`; a resize swallowed during the move/size modal loop is re-applied by `WM_EXITSIZEMOVE` or the bounded native timer. GPUI cached origin is not used.
- Added a C# regression that creates a temporary visible Win32 HWND, moves it to a non-default position, then resizes it through the production native method for Progress, Failed and Completed dimensions. All center and visible-size assertions passed. PlatformHost test-project Debug build passed with 0 warnings/errors; focused native HWND test passed.
- Focused Rust regressions passed: probe resize/generation (1), native handoff stash/place-before-reveal (2), retry collapse/height (1), expandable details (1), segment-dot ratios (1). The full desktop test binary passed 427/427; clippy `-D warnings`, `cargo fmt --check --all`, `tests/architecture/download-ui-lifecycle.ps1`, and `git diff --check` passed. Removed one unused `PILL_PADDING_X` import required for clippy.
- Existing dynamic active-connection count, segment-dot rendering, details disclosure, handoff ordering and keyed phase lifecycle were retained; no duplicate UI was introduced.
- Updated `docs/reference-code/download.md` with the newly verified native resize call chain and explicit limit of the automated HWND regression.

## Still required

- Manual interactive Windows smoke remains outstanding: drag the actual GPUI probe window while a live probe is pending, confirm/submit, then exercise retry, failure and completion while observing the real HWND location. The current HWND test proves the native moved-window resize seam, but does not simulate the user's GPUI mouse drag or network probe.
- No source geometry defect was proven beyond the missing real-HWND regression; no native behavior change was needed.
- The task remains `in_progress` until the manual smoke/visual acceptance is completed. No package, install, deployment, process stop/restart, or commit was performed.

## Follow-up verification (2026-10-02)

- Re-ran the moved-HWND native resize regression: passed. Focused GPUI tests passed for probe move/size recovery, hidden handoff placement-before-reveal, phase resize delegation, expandable details, segment-dot ratios, and terminal retry collapse.
- Full desktop binary: 427 passed; `cargo fmt --check --all`, desktop clippy (`-D warnings`), download lifecycle architecture gate and UI token gate passed. PlatformHost Debug test-project build passed with 0 warnings/errors.
- No live GPUI probe window was dragged, and no real probe/submit/retry/failure/completion interaction was performed. These Windows smoke and visual acceptance gates remain outstanding; no product geometry change was needed.
- Task remains `in_progress`; no package/install/deployment/process restart/commit/archive was performed.

## Fixed NxTools Validation package — 2026-10-06

- Final client-height correction build: added an adapter-owned client mismatch check using a frame-changed Win32 resize after the initial native request and after move-end replay. This directly addresses the observed Validation trace where requested client height was 278 but actual client height remained 176.
- Rebuilt into `artifacts/win-x64-validation-20261004` and compiled the fixed installer `artifacts/installer-validation-20261004/NxTools.Next-validation-20261004-setup.exe`, 14,282,969 bytes, SHA-256 `58B938EA78FF6C46D1C51B6A55FA139D99A404C24DE4E3C69F54A18CFAEE8B14`. The existing Validation AppId and install directory are unchanged.
- Post-fix verification passed: Rust desktop 430/430, cargo check/fmt, C# build/full suite, native HWND client-height/center/replay tests, and download/Quick Launch/UI-token architecture gates. The fixed installer was reinstalled; package and installed hashes match. No NxTools/PlatformHost/Indexer process is running; `NxToolsIndexerV1` remains stopped.
- Still required: manual Validation smoke—drag the live GPUI probe window before completion, confirm that the client height visibly expands and the directory/inspector/footer are not masked, then exercise retry/failure/completion and real DOpus tab/path behavior. Keep tasks `in_progress` until user acceptance.
## Follow-up verification — 2026-10-06 WM_SIZE synchronization

- The remaining cross-layer gap was the idempotent native resize path: when `GetClientRect` already matched the requested GPUI client size, PlatformHost returned without publishing `WM_SIZE`; GPUI could therefore retain its old logical viewport while the native client was already enlarged. `WindowsPlatformAdapter.ClientResize.cs` now posts one bounded same-size `WM_SIZE` in that path, without adding a retry loop or changing window-center ownership.
- Rebuilt the fixed Validation package at `artifacts/win-x64-validation-20261004`; compiled the existing installer `artifacts/installer-validation-20261004/NxTools.Next-validation-20261004-setup.exe`, 14,284,190 bytes, SHA-256 `7ADEBA13808C8426C1C94BD49D81FCB353FDC667604679FBB61F346DB1CA5B21`. The existing AppId and install directory are unchanged.
- Verification: desktop 430/430 with `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`; cargo fmt; PlatformHost/Tests Release-relevant builds with 0 warnings/errors; safe PlatformHost set 180/180 excluding real DOpus lister/navigation tests; native HWND resize/replay tests; download, Quick Launch and UI-token architecture gates; package-memory gate; and `git diff --check` exit 0. The installer was re-run with `Start-Process -Wait`; package and installed hashes match for NxTools, PlatformHost, Indexer and CLI. No NxTools/PlatformHost/Indexer process is running and no Indexer service operation was performed.
- Still required: user must launch the installed Validation package and visually repeat drag-during-probe at the actual DPI. Retry/failure/completion and real DOpus multi-pane/tab behavior remain manual gates; tasks stay `in_progress`.
## Follow-up verification — 2026-10-07 unified blacklist and probe transition

- Probe completion now submits the immediate GPUI/native resize before notifying the confirmed-state render, so the first confirmed frame no longer intentionally paints into the compact viewport. The existing PlatformHost client/frame/WM_SIZE ownership remains unchanged.
- Added shared Rust `AppConfig.process_blacklist` with per-feature flags for Clipboard and Quick Launch. Legacy `clipboard.excluded_processes` entries are merged during normalization; process names are normalized by basename, optional `.exe` removal and case-folding. Clipboard capture and Quick Launch summon both consult this shared policy, while Direct3D exclusive-fullscreen suppression remains an independent system-level guard.
- Added Clipboard settings rows for the shared process list plus independent “禁用剪贴板” and “禁用速启” scope toggles. Updates synchronize both Rust application entities and persist the shared config.
- Verification: core 34/34, desktop 430/430, focused probe resize/layout tests, cargo fmt/check, download/Quick Launch/UI-token architecture gates, Release package/build and fixed Validation installer installation. Package/installed hashes match; no product processes are running and `NxToolsIndexerV1` remains stopped.
- Still required: launch the installed Validation package and visually verify the faster probe transition at the user's DPI, then exercise retry/failure/completion and real DOpus multi-pane/tab behavior.
## Follow-up verification — 2026-10-07 GPUI blacklist synchronization boundary

- Corrected the shared blacklist settings callback so `MainWindow` updates only its local `ProcessBlacklistSettings` snapshot while leased; Clipboard and Quick Launch persistence/synchronization now runs from a deferred application turn instead of synchronously re-entering sibling entities.
- Added `main_window::tests::process_blacklist_sync_defers_cross_entity_updates` as a lifecycle regression gate. Re-ran `cargo fmt --all -- --check`, `cargo check -p nxtools-desktop`, the focused regression, Quick Launch lifecycle architecture, and UI-token architecture; all passed with zero compile warnings/errors.
- The full Rust verification remains green: `nxtools-core` 34/34 and `nxtools-desktop` 430/430 with the required serialized Cargo environment. No product process was started and `NxToolsIndexerV1` remains stopped.
- Manual Validation DPI drag/probe, retry/failure/completion, blacklist foreground-process behavior, and real DOpus multi-pane/tab acceptance remain pending; task stays `in_progress`.
## Validation package refresh — 2026-10-07 deferred blacklist synchronization

- Rebuilt the fixed Validation Release package at `artifacts/win-x64-validation-20261004` from the latest source with `scripts/package.ps1 -Configuration Release -OutputPath artifacts\win-x64-validation-20261004 -SkipVerification -SkipInstaller -MinimumCommitHeadroomGiB 6.0`. The release commit-headroom preflight reported 24.78 GiB; desktop PE/icon and package-memory gates passed.
- Recompiled `artifacts/installer-validation-20261004/NxTools.Next-validation-20261004-setup.exe` with the existing AppId and install directory. Installer size is 14,289,579 bytes; SHA-256 is `B1D06215D49A539AF352352B571FCB36386DAF07B7176E78D1079601FC505212`.
- Current package hashes: `NxTools.exe` `AEF11B9F2F0F2D57BBBB18D7613298B6F9A6A03A0D542354DE05ED5B1A13576D`; `NxTools.PlatformHost.exe` `9E2536A57E07C842981CD42F9F6BE3F0776FD94692E7EFB6AB1AE3BACB346AC8`; `NxTools.Indexer.exe` `E5A5B6E4F3F588716F74294B8838BF3066B6DCA2BC6A7C661B96004E32C074AB`; `NxTools.Cli.exe` `AE637B4DF2A3AAE09CEFE496E6150A20C7A299ECB7FC2D335493A5B4B5E157D1`.
- The installer was not run, the existing validation installation was not modified, no product process was started, and `NxToolsIndexerV1` remains `Stopped`. Manual DPI/drag, retry/failure/completion, blacklist behavior, and DOpus acceptance remain pending.
## Follow-up fix — 2026-10-08 confirmed form still clipped after drag

- The user's installed Validation binary matched the previous package, so this was not an old-installation mismatch. The confirmed add-download client height was still only 278px, which left the destination/file-name/category rows and footer at the clipped edge after the compact probe surface was retained through a native drag.
- Raised the confirmed form's explicit client-height floor to `NEW_DOWNLOAD_CONFIRMED_HEIGHT = 380.0` while retaining the compact probe height at 176px. Updated the production GPUI bounds regression to require the 380px confirmed height and keep the form/footer/inspector inside the window.
- Verification passed: cargo fmt/check, desktop 431/431, download UI lifecycle architecture gate, and git diff check. Release package rebuilt with 17.93 GiB commit headroom; package-memory gate passed.
- Recompiled and silently ran the fixed Validation installer with the same AppId/install directory. Installer size: 14,292,576 bytes; SHA-256: `7966CFED61423A347CF0648372F76C959274265AA3CED0F187211A0298182F37`. Package and installed hashes match for NxTools, PlatformHost, Indexer and CLI.
- No product process is running and `NxToolsIndexerV1` remains `Stopped`. User must launch the installed Validation copy and repeat drag-during-probe; retry/failure/completion, blacklist behavior, real DPI and DOpus acceptance remain manual gates.
## Follow-up adjustment — 2026-10-08 compact confirmed form

- The 380px confirmed form was visibly oversized in manual review, with excessive empty space above the footer. Reduced `NEW_DOWNLOAD_CONFIRMED_HEIGHT` to 278px, matching the intended compact confirmed layout while retaining the probe height at 176px.
- Focused confirmed-layout tests, `cargo fmt --check`, and `cargo check -p nxtools-desktop` passed. Rebuilt the fixed Validation package with 17.96 GiB commit headroom and passed the package-memory gate.
- Recompiled and silently installed the fixed Validation installer. Installer size: 14,292,247 bytes; SHA-256: `C33EA5B30655886515C7261949CFC459C38DFD4BBF47A571287CC1F4845BDA1E`. Package and installed hashes match for all four core binaries.
- No product process is running and `NxToolsIndexerV1` remains `Stopped`. Manual launch and final DPI/drag/probe visual acceptance remain pending.
## Root-cause fix — 2026-10-08 native resize lost during user move

- The Validation PlatformHost log proved the actual failure: a confirmed resize requested client height 278, returned success, but observed `after_client_height=176` with `pending_recovery=false`. `EnsureRequestedClientSize` was returning early while the HWND was in `GUI_INMOVESIZE`, without registering the client correction for move-end recovery.
- Fixed `WindowsPlatformAdapter.ClientResize.cs` to park a correlation-scoped pending centered resize during a native move; replay now re-applies the client/frame correction and uses the same correlation before publishing `WM_SIZE`.
- Added the PlatformHost regression `centered resize parks client correction during a native move`; PlatformHost Debug build and existing moved-window/replay/retry tests passed. Rust focused download checks, fmt/check and download lifecycle gate passed.
- Rebuilt and silently installed the fixed Validation package. Release commit headroom was 17.23 GiB; package-memory gate passed. Installer size: 14,291,820 bytes; SHA-256: `85D9A60362CA2B2F54B545571E6044940F8CC265C7C99E16E8E400FBDDEDA52F`. Package and installed hashes match for all four core binaries.
- No product process is running and `NxToolsIndexerV1` remains `Stopped`. Manual launch and the original drag-during-probe scenario remain the final acceptance gate.
## Final native race fix — 2026-10-08 after Sonnet review

- Independent Sonnet review and the existing Validation log isolated the remaining race: one resize request recorded `requested=278`, `before=278`, `after=176`, `success=true`, `pending=false`; the window was not yet reported as moving when `SetWindowPos` ran, but Windows moved it back to the compact client size immediately afterward.
- Strengthened the native boundary so a client/frame resize parks a correlation-scoped pending recovery whenever the final real client rectangle does not match the requested size, not only when `GUI_INMOVESIZE` is observed at the first check. Move-end/timer replay then re-applies the correction and republishes `WM_SIZE`.
- Added/kept the regression requiring the drag-time client correction to be parked. PlatformHost build, parking regression, moved-window resize, move-end replay, retry recovery, Rust download checks, lifecycle architecture gate and package-memory gate passed.
- Rebuilt and silently installed the final Validation package. Release commit headroom: 28.18 GiB. Installer size: 14,292,057 bytes; SHA-256: `EEAC05919973C279B9A5D2DE1150F6E1148B75A57511A7CC1387B89CF92154EC`. Package and installed hashes match for all four core binaries.
- No product process is running and `NxToolsIndexerV1` remains `Stopped`. The user must now repeat the original drag-during-probe scenario; this is the remaining real-desktop acceptance gate.
## Final fixed-center transition repair — 2026-10-08

- The user clarified the failure occurs specifically when the probe window has been moved and then transitions from probing to the confirmed/download stage. The native recovery path was updated to retain the real visible center captured after the move and reuse that fixed center for every bounded replay; it no longer recalculates the center from an intermediate resized rectangle.
- Recovery now requires two stable real-client observations before clearing pending state. A final client mismatch still re-parks the correlation-scoped recovery. This covers both the post-`SetWindowPos` asynchronous compact-height writeback and the stage-transition center drift.
- Verification passed: PlatformHost Debug build, centered resize/replay/parking regressions, Rust fmt/check, confirmed download bounds test, download lifecycle gate, package-memory gate and diff check.
- Rebuilt and silently installed the final Validation package. Release commit headroom: 19.24 GiB. Installer size: 14,290,892 bytes; SHA-256: `18933F9C9FCC1157E9C6CDE34209925B35BA03B6019AD460A0829E1B44E38489`. Package and installed hashes match for all four core binaries.
- No product process is running; `NxToolsIndexerV1` remains `Stopped`. Shutdown was explicitly cancelled by the user, so no shutdown command was issued. Manual launch and the moved probe-to-confirmed scenario remain the final acceptance gate.
## Final fixed-center package/install — 2026-10-08

- Opus 5.5 was requested for an independent high-intensity review but the provider returned a 429 cooldown before delivering a report. I continued from the actual PlatformHost log and completed the stage-transition fix locally.
- The final native path retains the real visible center captured after the user's move, uses that fixed center for every bounded replay, requires two stable real-client observations, and re-parks any final client mismatch. This directly targets the moved probe -> confirmed/download transition rather than changing only the height.
- PlatformHost build and centered resize/replay/parking regressions passed; Rust fmt/check, confirmed layout test, download lifecycle gate, package-memory gate and diff check passed.
- Final package was rebuilt and silently installed with 24.61 GiB release commit headroom. Installer size: 14,290,892 bytes; SHA-256: `19C511FD368A7280B6BF3311B5731FE2AECF3376C91ED9D535B16F770559EF7A`. Package and installed hashes match for all four core binaries.
- No product process is running; `NxToolsIndexerV1` remains `Stopped`. Shutdown was cancelled by the user and was not executed. Final manual validation remains the moved probe-to-confirmed scenario at the real desktop DPI.

## User-approved fixed-stage replacement — 2026-10-09
- User13:40 candidate screenshot still shows compact source and explicit resize-timeout failure. Prior native-only/GPUI-fallback/direct278 evidence did not certify actual conversion. Do not blame old code or stale package; exact old intermittent timing is unproven.
- User requested fixed stages. Successful probe now creates a new fixed confirmation HWND (ordinary176→new278; media budgets preserved), one typed native apply/anchor placement/show, result-guarded registration/old-window retirement. No success-path resize watchdog or probe-data clearing; failure/disconnect/one IPC deadline retains data for presentation-only retry.
- Actual localhost production probe replacement regression ran red before fix; final478 desktop/41 protocol pass, strict production/tests lint/workspace/fmt/Debug build. Native owned GPUI hidden fixed placement and localhost→production handoff→C#apply→result/old HWND destroyed checks pass at100%. One review found DPI/default-monitor and editable-URL findings; source display/native monitor preparation and pending input freeze implemented/retested.
- Bright action/focus blue38bdf8/hover7dd3fc implements user's visual amendment. Evidence: docs/audits/2026-10-09-fixed-download-stage-handoff.md and artifacts/ui-fixed-transition. Physical held-pointer/mixed-DPI/installed runtime remain pending;13:40 installer unchanged, no new package/install/service/Clipboard/DOpus/commit/archive.

## Fixed-stage Validation package — 15:38
User explicitly requested packaging for manual validation. Fixed AppId/install directory retained; new installer15253349 bytes,SHA25699142F03043089BB114303F837F1499558AF990C9C71DCB615E910ACDF07538D contains fixed-stage and bright-blue changes. Release preflight9.21GiB≥6,build/publish/Inno and PE/icon/memory gates pass;five hashes/trusted manifest match,generated manifest restored,service state/path unchanged. No installation/launch/service operation; task remains in_progress pending user held-pointer/DPI acceptance.

## 2026-10-09 accepted fixed handoff and completion geometry refinement
User accepted the15:38 Validation new-download flow; retain it. Ordinary task progress/completed now both440x184 and transform content on the same keyed HWND without a native resize. The real task/min184 fixture failed the old180 target (actual184), and two production regressions went red before correction. Changed-size expanded/failed phases retain the native centered owner; min184 permits a taller initial failed window to collapse. Production lifecycle/containment regressions pass; exact owned GPUI native test15 observations reports zero center drift. Latest Desktop484/Core42/Protocol41 pass with strict lint/workspace/fmt and9 source gates. See `docs/audits/2026-10-09-completion-and-global-blacklist.md`. This follow-up is not packaged; installed completion and mixed-DPI acceptance remain separate.

## Completion/global rules Validation package — 19:30
Subsequent explicit user packaging request completed. Same Validation AppId/directory; installer15286988 bytes,SHA25680F1F3310C7219C3363A92A3C36E12B0FB70A98B4616995F39F90200A9AB14DA includes ordinary same-size completion and retains accepted fixed handoff. Release preflights8.41/7.87GiB≥6,build/publish/Inno, five hashes/trusted manifest and PE/icon/memory gates pass. Dirty generated manifest restored; production binaries/service unchanged; no agent installation/launch. Installed completion/DPI acceptance remains pending.