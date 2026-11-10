# H4/U3 Rust acceptance follow-up

## Result

- Quick Launch recovery button uses the original `QuickLaunchApplication::keyboard_health_warning` projection. Its busy value now includes any pending `ConfigureQuickLaunch`, including the automatic command sent after an unavailable Hello. The original `reenable_keyboard_hotkey` admission and `send_configuration_for_with_id` remain the only retry owner. The existing Settings -> SettingPage -> SettingGroup -> SettingItem::render summon row retains the released Kit 0.8.0 Small `Alert::warning(...).title("双击 Ctrl 已失效")` and standalone `.outline().with_size(Size::Medium)` Button (32 DIP). The row is outside `Toolbar.child`, so its outlined boundary is not converted to ghost/compact. Existing `.loading(busy).disabled(busy)` and deferred click receive the corrected state; no component/vendor/layout edit.
- `keyboard_health_notified` belongs to the original Quick Launch application. An unavailable event during recording retains the fault but does not consume notification eligibility; native stop/cancel ACK rechecks the original double-Control setting and defers one NotificationCenter notice when the Alert becomes relevant. Repeated events cannot restart the same notice; `reinstalled`, successful matching recovery, successful generic Hello reconfiguration, disconnect, and the next Hello reset eligibility for a later episode. Existing hotkey recording/reset/config/save state stays owned by the same application.
- On `PlatformNotification::Disconnected`, a pending keyboard retry is taken before clearing the configuration/session state. It emits exactly one `quick_launch.keyboard_reenable.completed` with the original `correlation_id`, `success=false`, `failure_kind="disconnected"` and `elapsed_ms` from the original start. There is no native code on a disconnected boundary. A previously consumed matching result cannot emit a second terminal. No key contents or exception payload are logged.

## Test evidence

All Cargo commands ran offline, one at a time, with `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`, `CARGO_NET_OFFLINE=true`, timeout 21600 seconds; Desktop tests used `--config 'profile.test.package.nxtools-desktop.debug=0'` and `-- --test-threads=1`. Logs below are in `D:\.pi-desktop\data\scratch\6a21b011-4230-4aa5-a2ed-f2dc1b4153e6\`.

| Command / evidence | Result |
| --- | --- |
| `cargo --config 'profile.test.package.nxtools-desktop.debug=0' test -p nxtools-desktop u3_followup_ -- --test-threads=1`; `u3-followup-rust-red-owner.log` on original behavior | **0/2**: after native recording-stop ACK the notification count was 0 instead of 1; after disconnect no correlated retry terminal was captured. Both tests reached their behavioral assertions. |
| `cargo --config 'profile.test.package.nxtools-desktop.debug=0' test -p nxtools-desktop quick_launch_keyboard_health_auto_configuration_busy_matches_retry_button -- --test-threads=1`; `u3-followup-rust-red-ui.log` on original behavior | **0/1**: actual official Button snapshot had disabled `None` instead of `Some(true)` while automatic configuration was pending. |
| Same `u3_followup_` filtered command; `u3-followup-rust-final-owner.log` | **3/3**: recording notification episode, disconnected terminal ID/outcome/elapsed, completed-before-disconnect no duplicate. |
| Same real Settings button filtered command; `u3-followup-rust-green-ui-final.log` | **1/1**: auto pending disables the official 32 DIP button, busy click has no duplicate command, failed ACK enables one manual retry; the same production TestApp also covers recorded fault -> stop ACK -> mounted Alert and one new notice generation. |
| `cargo --config 'profile.test.package.nxtools-desktop.debug=0' test -p nxtools-desktop quick_launch:: -- --test-threads=1`; `u3-followup-rust-quick-launch.log` | **173/173** passed. Includes the 3 follow-up owner tests; not additive to the preceding 3. |
| Same Desktop command filtered `main_window::tests::quick_launch`; `u3-followup-rust-main-window.log` | **11/11** passed, including the new UI test. |
| Same Desktop command filtered `hotkey_reset`; `u3-followup-rust-hotkey-reset.log` | **3/3** passed. |
| `cargo --config 'profile.test.package.nxtools-desktop.debug=0' clippy -p nxtools-protocol -p nxtools-desktop --tests -- -D warnings`; `u3-followup-rust-clippy-green.log` | Passed, no warnings. |

Scoped `rustfmt --edition 2024 --config skip_children=true --check` on `quick_launch.rs`, `quick_launch/tests.rs`, and `main_window/tests.rs`; `git diff --check --` on the three edited Rust files plus this scenario's spec; `python tests/architecture/spec-contracts.py` (120 documents / 14 owners), `python tests/architecture/gpui-release-contracts.py`, and `python tests/architecture/markdown-editor-contracts.py`: all passed. Protocol/IPC and browser extension sources were unchanged; their full suites were not repeated.

## Acceptance boundary

The PlatformHost bounded-queue health delivery change belongs to the C# owner and is not implemented or verified here. The GPUI/TestApp tests exercise production event/notification/Settings/button/command/diagnostic boundaries, not a physical keyboard, installed global hook, real HWND/DPI, screenshot or user visual acceptance. No product/service launch, packaging, network access or commit was performed.
