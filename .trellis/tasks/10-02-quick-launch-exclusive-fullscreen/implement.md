# Implementation plan

1. Re-read this PRD/design, `docs/reference-code/quick-launch-research.md`, the desktop and IPC spec indexes/quality guides, and the current dirty diffs. Preserve all user changes and keep download-secondary UI out of this slice.
2. Add red-capable pure tests first for serde default/round-trip, typed command compatibility, and the policy truth table: enabled+state 3 suppresses; disabled, state 2/borderless/windowed, and failed query allow.
3. Add a C# message-loop seam test proving both `WM_HOTKEY` and posted `WmQuickLaunchGesture` emit no summon request only in the suppressing case; query failures remain fail-open.
4. Implement the persisted boolean/default and typed protocol field. Keep missing JSON fields defaulting on. Wire the existing settings row with `compact_toggle`, `cx.defer` for cross-entity changes, save error notification and existing configuration synchronization.
5. Implement the smallest injectable `SHQueryUserNotificationState` wrapper/policy. Call from the Window Procedure just before the Quick Launch publication, never from the low-level hook. Leave clipboard and other hotkeys untouched.
6. Add bounded decision/configuration diagnostics with correlation, trigger/state/outcome/HRESULT/elapsed time; rate-limit repeated failures, never log titles/paths/key streams, and contain logger failures.
7. Validate with Rust focused desktop/core/protocol tests; `cargo fmt --check`; `tests/architecture/ui-token-boundaries.ps1`; C# PlatformHost test project build and full harness; `git diff --check` and source review. Every Cargo command must use `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`, and only run when memory headroom permits.
8. Do not package or overwrite `artifacts/win-x64`; do not stop/restart PlatformHost or desktop processes. State clearly that source/tests are not an installed-service rollout.

## Worktree preservation

Before every edit, inspect the current exact diff for that target. Expected Rust dirty work already exists in the Quick Launch settings, theme/components and main-window test files; merge only the lines required by this feature. Keep unrelated downloads, clipboard, settings and diagnostic fixes unchanged.

## 验证记录

实现与本任务范围内的验证已完成（以下结果来自本任务已有的验证记录）：

- C# PlatformHost Debug 构建：通过，0 warnings / 0 errors。
- 四个聚焦的 Quick Launch C# harness 测试：通过。
- Rust 格式检查、core 与 protocol 定向测试、Quick Launch desktop 107 项测试：通过。
- UI token architecture gate 与 `git diff --check`：通过。
- `python ./.trellis/scripts/task.py validate 10-02-quick-launch-exclusive-fullscreen`：本轮复核通过，implement/check 上下文清单均有效。
- 未过滤的完整 C# harness 因环境错误 `Could not register Alt+C: Win32=1409.` 中止；该错误不代表 Quick Launch 聚焦测试失败，完整 harness 仍未通过验证。
- 未执行打包、安装、部署，也未停止或重启 PlatformHost。

## 收尾状态

实现及已批准范围内的定向验证已完成。任务仍为 `in_progress`：完整 C# harness 受 Alt+C 注册冲突阻断；并且本工作区保留大量未提交改动。按照仓库流程，提交/归档需要另行确认，归档会移动任务目录并可能自动提交；在获得授权前不得把状态改为 `completed` 或归档。本任务不包括下载辅助窗口 UI 或转场动画。

## Follow-up verification (2026-10-02)

- Re-ran the full desktop binary with the required serialized Rust environment: 427 passed, 0 failed; `cargo fmt --check --all` and desktop clippy (`-D warnings`) passed.
- Re-ran all 107 `quick_launch::tests`: 107 passed, 0 failed. C# filtered tests passed for configuration/event contracts, correlated/rate-limited diagnostics, exclusive-fullscreen policy, and both WndProc summon routes. PlatformHost Debug test-project build passed with 0 warnings/errors.
- UI token and download lifecycle architecture gates passed. Task context validation passed.
- No full PlatformHost harness was run in this follow-up; the prior DOpus restore side effect remains a reason not to repeat it. No installation/deployment/restart or commit/archive was performed.

## Isolated user-mode Release installer (2026-10-03)

- Installer: `artifacts/installer-validation-20261003/NxTools.Next-validation-20261003-setup.exe`, SHA-256 `503BC888ACF0868E2CD08073B86B11515776C5C81D7D829C20401A52BE565B6F` (15,118,096 bytes). It uses a separate app name/AppId and current-user install directory; `INSTALL.md` is beside it. The erroneous portable ZIP/package from the prior response was removed.
- Release filtered C# tests passed for Quick Launch configuration, diagnostics, both summon sources and fullscreen gating. Package PE/icon, memory-policy, UI-token and download-lifecycle gates passed; the five input binaries matched Release outputs.
- Protected Desktop/PlatformHost/Indexer hashes differ from the installer payload; the installed Indexer reported Running. If Quick Launch requests UAC or protected-service repair, cancel and skip index/fullscreen smoke. Installer was not run; no service/process was stopped or restarted.
- Existing 0.7.0 installer and dirty `ReleaseBinaryManifest.g.cs` source remain unchanged. Task stays `in_progress` pending manual verification and repository closeout.
## Follow-up verification — 2026-10-07 unified process blacklist

- Added shared `AppConfig.process_blacklist` entries with independent Clipboard/Quick Launch scope flags. Quick Launch summon now rejects a matching foreground process before opening the panel; Clipboard capture reads the same normalized process policy. The existing Direct3D exclusive-fullscreen suppression remains separate and continues to cover both summon triggers.
- Clipboard settings now expose the shared process list plus independent “禁用剪贴板” and “禁用速启” toggles. Rust entities synchronize and persist the same config file; legacy Clipboard excluded-process values migrate into the shared list.
- Verification: core 34/34, desktop 430/430, cargo fmt/check, Quick Launch/download/UI-token architecture gates, Release package and fixed Validation installer installation. No product processes were started and the Indexer service was not touched.
- Still required: manual Validation smoke for the blacklist toggles and actual foreground-process behavior, plus the existing user DPI/DOpus acceptance gates. Keep this task `in_progress`.
## Validation package refresh — 2026-10-07 deferred blacklist synchronization

- Rebuilt the fixed Validation Release package and installer after the GPUI blacklist synchronization boundary fix. The package-memory gate passed with 24.78 GiB release commit headroom; the existing fixed AppId/install directory were preserved.
- Installer: `artifacts/installer-validation-20261004/NxTools.Next-validation-20261004-setup.exe`; 14,289,579 bytes; SHA-256 `B1D06215D49A539AF352352B571FCB36386DAF07B7176E78D1079601FC505212`.
- The installer was not run and the Validation desktop was not launched. No product process was started and `NxToolsIndexerV1` remains `Stopped`; manual blacklist/foreground, real-DPI, and DOpus acceptance remain pending.
## Follow-up fix — 2026-10-08 confirmed download form height

- Raised the confirmed new-download client-height floor from 278px to 380px after the installed Validation screenshot still showed the full destination/file-name/category/footer content clipped at the compact probe height.
- Desktop 431/431, cargo fmt/check, download lifecycle gate and diff check passed. Rebuilt and silently installed the fixed Validation package using the existing AppId/install directory; package and installed core hashes match.
- Installer SHA-256: `7966CFED61423A347CF0648372F76C959274265AA3CED0F187211A0298182F37` (14,292,576 bytes). No product process started; `NxToolsIndexerV1` remains `Stopped`. Manual real-DPI and foreground-process acceptance remains pending.
