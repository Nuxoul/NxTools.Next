# Rust H4 / U3 outcomes

## Wire and behavior

Outer PlatformHost protocol remains version 1. HelloAck negotiates `quick_launch_keyboard_health_v1`; C# must publish `quick_launch_keyboard_health_changed` only after acceptance because an older Rust decoder rejects unknown PlatformEvent variants and the IPC reader terminates that session. Event envelope correlation ID and payload UUID agree; states are `install_failed`, `thread_exited`, `reinstalled` with native i32 error code. No silent hook-loss inference or heartbeat. C# owns the hook thread and post-discard health snapshot; Rust owns setting state. IPC records the original `quick_launch_panel_hotkey` Hello capability in the current primary connection before sending Connected, and never reuses an older generation's snapshot.

QuickLaunchApplication shows a warning for enabled double-Control outside recording, and defers one NotificationCenter notice per unavailable transition. Existing recording/reset/save owner and ConfigureQuickLaunch builder remain unchanged. Retry installs its ID before IPC enqueue, has a single pending operation and logs correlation, accepted/terminal result, native code and elapsed time without key payload. Each health event advances a local revision: a delayed successful or failed retry cannot overwrite later `ThreadExited`/`InstallFailed`; matching unchanged-revision success or `reinstalled` clears the warning. Successful ordinary automatic ConfigureQuickLaunch clears a stale generic Hello-unavailable warning, but never clears newer concrete native failure. Disconnect retires pending ID and health; reconnect reads fresh Hello snapshot.

UI shape: existing `Settings -> SettingPage -> SettingGroup -> SettingItem::render` summon row/HotkeyField, with a subordinate `Alert::warning(id,reason).title("双击 Ctrl 已失效").with_size(Size::Small)` and a direct outlined `Button::new(id).label("重新启用").outline().with_size(Size::Medium)` (official 32 DIP) outside Toolbar.child's ghost adaptation. Loading/disabled while pending, pointer callback defers cross-entity application update. Transparent layout nodes with ElementIds and TestSupport provide mounted-bounds observations; they do not repaint or replace either official control.

## Verification

All Cargo processes set `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`, `CARGO_NET_OFFLINE=true`; one owned Cargo invocation at a time, timeout 21600 seconds. Logs are in the shared `D:\.pi-desktop\data\scratch\6a21b011-4230-4aa5-a2ed-f2dc1b4153e6\` directory:

- Behavior red: `input-rust-health-order-red.log` — `cargo --config 'profile.test.package.nxtools-desktop.debug=0' test -p nxtools-desktop keyboard_health_ -- --test-threads=1` reached `keyboard_health_new_thread_exit_outlives_earlier_successful_retry` and failed at the assertion that a new ThreadExited must survive an earlier successful result. `input-rust-health-behavior-red.log` also records the intended assertion that an ordinary successful configuration must clear the old generic Hello warning. The final same-filter result is4passed/0failed.
- Final current-tree green: `input-rust-health-latest-source-green.log` — `cargo --config 'profile.test.package.nxtools-desktop.debug=0' test -p nxtools-desktop keyboard_health_ -- --test-threads=1`, 4 passed. `input-rust-health-ui-green-second.log` separately records the production settings render, Alert bounds, actual Button click, exactly one typed ConfigureQuickLaunch, pending duplicate rejection, wrong result, failed retry, matching recovery and disconnect/reconnect (1 passed).
- `input-rust-quick-launch-latest.log` — `quick_launch::` filtered, 170 passed; `input-rust-main-quick-launch-latest.log` — `main_window::tests::quick_launch` filtered, 10 passed; `input-rust-hotkey-reset-latest.log` — 3 passed. Desktop filters used `--test-threads=1` and the required test debug override.
- `input-rust-strict-clippy-latest.log` — `cargo --config 'profile.test.package.nxtools-desktop.debug=0' clippy -p nxtools-protocol -p nxtools-desktop --tests -- -D warnings` passed. Final `cargo test -p nxtools-ipc` passed 45 unit tests + 0 doc tests (`input-rust-ipc-final.log`). Earlier full `cargo test -p nxtools-protocol` passed 99 unit + 2 integration = 101; unchanged since then.
- Scoped `rustfmt --edition 2024 --config skip_children=true --check` on seven owned Rust files, `python tests/architecture/spec-contracts.py` (120 docs / 14 owners), and scoped `git diff --check` passed. Previous GPUI release and Markdown editor architecture source gates passed; no vendor/dependency changes.

## Limitations

Synthetic GPUI/TestApp does not establish physical keyboard, installed hook, DPI, screenshots or user acceptance. No package, native product launch, external network or commit was performed.

## Reproduction commands

```powershell
$env:CARGO_BUILD_JOBS='1'
$env:CARGO_INCREMENTAL='0'
$env:RUSTFLAGS='-C codegen-units=256'
$env:CARGO_NET_OFFLINE='true'
cargo test -p nxtools-protocol
cargo test -p nxtools-ipc
cargo --config 'profile.test.package.nxtools-desktop.debug=0' test -p nxtools-desktop keyboard_health_ -- --test-threads=1
cargo --config 'profile.test.package.nxtools-desktop.debug=0' test -p nxtools-desktop quick_launch:: -- --test-threads=1
cargo --config 'profile.test.package.nxtools-desktop.debug=0' test -p nxtools-desktop main_window::tests::quick_launch -- --test-threads=1
cargo --config 'profile.test.package.nxtools-desktop.debug=0' test -p nxtools-desktop hotkey_reset_tests -- --test-threads=1
cargo --config 'profile.test.package.nxtools-desktop.debug=0' clippy -p nxtools-protocol -p nxtools-desktop --tests -- -D warnings
$changedRust = @(
    'rust/crates/nxtools-protocol/src/lib.rs',
    'rust/crates/nxtools-ipc/src/lib.rs',
    'rust/apps/nxtools-desktop/src/clipboard.rs',
    'rust/apps/nxtools-desktop/src/quick_launch.rs',
    'rust/apps/nxtools-desktop/src/quick_launch/tests.rs',
    'rust/apps/nxtools-desktop/src/main_window/quick_launch_page.rs',
    'rust/apps/nxtools-desktop/src/main_window/tests.rs'
)
rustfmt --edition 2024 --config skip_children=true --check @changedRust
```

Run Cargo sequentially with a21600second command budget. Counts and results above distinguish overlapping health filters from the broader Desktop groups.
