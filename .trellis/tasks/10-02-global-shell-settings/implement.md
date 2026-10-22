# Implementation Plan

1. Read the Rust desktop and IPC backend spec checklists; inspect the current exact diffs in `network_settings.rs`, `WindowsPlatformAdapter.EventWindow.cs`, `WindowsPlatformAdapter.cs`, `DesktopTrayController.cs` and existing platform command tests.
2. Add/adjust red tests for the missing desktop-executable stage, generic C# catch result, startup/tray stages, Rust message mapping and stale correlated result.
3. Correct the Rust match and C# return result; preserve safe user-facing text and native error codes. No unrelated formatting/refactor.
4. Run focused PlatformHost tests and the full C# test program; run Rust focused network-settings tests, desktop suite, fmt, clippy with the required environment.
5. Review exactly scoped diff and verify persisted-vs-applied semantics and fail-closed close-to-tray invariant.

Expected files: `rust/apps/nxtools-desktop/src/network_settings.rs`, `csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.EventWindow.cs`, possibly `csharp/NxTools.PlatformHost.Tests/Program.cs`; other files only if the production result seam requires them.

## Validation evidence (2026-10-02)

- Reference reviewed: `docs/reference-code/system-shell.md` and old NxTools `StartupLaunchService.cs` / `MainWindow.axaml.cs`; Run-key, close-to-tray fail-closed, and native tray lifecycle invariants retained.
- PlatformHost Debug test-project build passed with 0 warnings and 0 errors.
- Five focused PlatformHost shell/tray tests passed, including stage-code mapping, registry failure, tray cleanup, and correlation-safe result serialization.
- Ten `network_settings::tests` passed, including persisted intent after failure, stale-result rejection, and actionable stage messages.
- Full desktop test binary passed: 427 passed, 0 failed. `cargo fmt --check --all`, desktop clippy with `-D warnings`, UI token gate, and download UI lifecycle gate passed.
- Four focused native window geometry tests passed.
- The full PlatformHost harness launched through `dotnet run` completed with exit code 0; stderr included the non-fatal `Could not register Alt+C: Win32=1409.` diagnostic. A separate direct-DLL rerun did not complete cleanly: its real Directory Opus navigation test failed while restoring the originally captured lister directory. This is a pre-existing interactive integration boundary and must not be altered as part of shell settings. Do not claim the PlatformHost suite is consistently green.
- No package, install, deployment, or commit performed.

## Remaining gate

The settings apply implementation and scoped regressions are in place, and one full PlatformHost run passed. Keep task status `in_progress` until its full PlatformHost/DOpus integration result is reproducible and the work can be committed/archived through the repository workflow. The separate direct-DLL run entered the real Directory Opus navigation test: it moved the active pane to `C:\Windows`, then failed while reactivating the lister before the saved original-directory restore command could run. The original path is no longer available from the exited test process; I issued no guessed navigation and left the window/process running. The shell settings were not applied to the live user's registry/tray configuration. No package, install, deployment or commit was performed.

## Follow-up verification (2026-10-02)

- `network_settings::tests`: 10 passed, including staged actionable messages, stale-result rejection, persisted intent after failure, fail-closed close-to-tray and success confirmation.
- Filtered PlatformHost tests passed for shell/tray bounded intent, staged failure codes (missing executable, registry write failure, tray failure and generic safe failure), startup registration/approved path and tray action lifecycle. PlatformHost Debug test-project build passed with 0 warnings/errors.
- Full desktop binary: 427 passed; `cargo fmt --check --all`, desktop clippy (`-D warnings`), UI token gate, download lifecycle gate and scoped task validation passed.
- Not repeated: full PlatformHost/DOpus interactive harness, because the previous run navigated the active DOpus pane and failed before restoring its unknown original location. The user must restore that pane manually. No live user's startup/tray settings were changed.
- Task remains `in_progress`; manual apply evidence, reproducible full harness and repository-authorized closeout are still outstanding. No package/deployment/restart/commit/archive was performed.

## Isolated user-mode validation installer (2026-10-03)

- Created `artifacts/installer-validation-20261003/NxTools.Next-validation-20261003-setup.exe`, SHA-256 `503BC888ACF0868E2CD08073B86B11515776C5C81D7D829C20401A52BE565B6F` (15,118,096 bytes). It has a separate product identity, installs under `%LOCALAPPDATA%`, and omits startup registration, post-install launch and service-stop actions. `INSTALL.md` documents limits.
- Release filtered PlatformHost shell-stage, safe registry/tray boundaries, startup registration and HWND tests passed. The prior default 0.7.0 installer hash is unchanged; the generated dirty manifest source was restored byte-for-byte.
- Validation installer was not run; no registry, live tray configuration or ProgramData service was changed. Previous DOpus pane restore failure remains; cancel any unexpected UAC. Task stays `in_progress`.

## 2026-10-09 global per-application blacklist
User explicitly replaces Clipboard-local shared-list/global-switch UX with Global settings -> Application blacklist. A bounded continuous table provides per-row independent Clipboard/Quick Launch scopes, add/edit/remove, validated normalized executable matching and cancel/error-preserved drafts. One locked global settings transaction commits network/system/rules; deferred consumers publish the committed snapshot without saving again. Rules-only edits skip unchanged startup/tray policy. Missing canonical entries migrate legacy once; present-empty is authoritative and prevents removed rules returning. Core temporary-file and GPUI pointer/keyboard/cancel/failure/single-save/bound-consumer tests pass within Desktop484/Core42; Protocol41, strict lint/workspace/fmt and9 gates pass. Synthetic native100%720x480 screenshot is in `artifacts/ui-global-blacklist/`; no ordinary runtime/service/real Clipboard/DOpus/package/install operation. See `docs/audits/2026-10-09-completion-and-global-blacklist.md`; actual installation/DPI acceptance remains pending.

## Per-application rules Validation package — 19:30
Explicit user packaging request completed: same fixed Validation identity, installer15286988 bytes,SHA25680F1F3310C7219C3363A92A3C36E12B0FB70A98B4616995F39F90200A9AB14DA includes global independent Clipboard/Quick Launch rules. Release build/publish/Inno and package/hash/manifest/PE/icon/memory checks pass; service/production binaries unchanged and dirty manifest restored. No agent install/product launch/live shell settings; user-installed rule/function/DPI acceptance remains pending.

## Installed-upgrade Apply correction — 2026-10-10
User reports current package rejected after updating old installation plus unreadable light Apply text. Temporary-file actual resolver test red→green: ordinary HKCU startup/tray may use exact current PlatformHost sibling with stable current manifest desktop hash, independently of old protected service tuple; outside/tampered/wrong-name/invalid-hash/missing reject, protected fallback unchanged. Project primary_action_button explicitly owns label/disabled colors and retains Button eligibility/KeyUp. GPUI paint-time normal/hover/disabled+no-disabled-dispatch red→green. Final Desktop498/498, strict lint/workspace/fmt, exact safe C#3RUN/3PASS and build0warnings/errors. Error copy now saved≠Windows-applied. No package/install/real service/user Run-key operation this turn. Audit docs/audits/2026-10-10-upgrade-shell-apply-and-button-contrast.md.