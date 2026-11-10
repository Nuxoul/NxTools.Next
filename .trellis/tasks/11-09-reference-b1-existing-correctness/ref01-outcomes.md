# REF-01 — Link panel routing and Copy lifecycle

## Production result

Production ListItem activation closes its owning panel and defers dispatch to the current ClipboardApplication. `link_panel_copy_only` copies every target kind; otherwise Web opens only when `link_panel_auto_open_browser` is enabled. Local files select in Explorer and directories open. Settings are read at activation, not captured during render. Named Copy and Open remain independent; named Open uses the original typed Shell boundary without copying.

Every deferred callback validates its original window before retiring or dispatching. Restaging retains one panel, placement, dimensions, focus and the existing auto-hide task owner. Explicit Copy keeps ownerless behavior through `Option::is_none_or`.

Explicit Copy emits `copy_started` before local close. The original deferred callback writes once and emits `copy_completed` for a current or ownerless panel; an upgraded owner's stale-window guard emits `copy_cancelled` with fixed `reason=panel_replaced`. Each terminal carries the same correlation ID and numeric elapsed_ms. A stale Copy cannot change the replacement, clipboard or platform commands. No links/paths/clipboard text, extra application-turn delay, retry, native API or replacement controller is added.

## Released composition

The existing420-DIP panel keeps28-DIP ListItem rows, Label/Icon, independent XSmall ghost Copy/Open Buttons and StatusBar. Exact Component0.8.0 `list/list_item.rs` and `button/button.rs` public callback/render APIs were reviewed. Buttons are direct children of layout within ListItem, not transformed by Toolbar. Layout, variants, accessible names, input/focus and timer owners are unchanged. Native/visual acceptance is separate from compiled fixtures.

## Lifecycle regressions

`clipboard::asset_tests::ref01_row_uses_saved_settings_at_activation_and_stale_close_preserves_reopened_panel` changes and reloads saved settings after staging, then drives the real row. The window-closed observer establishes a replacement before deferred row copying and asserts the sentinel at that boundary.

`clipboard::link_copy_tests::ref01_explicit_copy_has_one_correlated_terminal_for_current_stale_and_ownerless_windows` clicks the actual `link-panel-copy-0` Button. The same real close observer establishes a replacement before the old callback. It checks the replacement/application handle, sentinel clipboard, zero platform commands, three captured starts and exactly one same-ID terminal for each normal/stale/ownerless case. Terminals require numeric elapsed, the fixed stale reason and no synthetic payload. The test does not assume `simulate_click` returns before deferred work drains.

## Executed verification

All commands use one Cargo lane and same-execution `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`, scratch TEMP/TMP. Counts overlap and are not summed.

| Current command | Result | Scratch evidence |
| --- | --- | --- |
| `cargo test -p nxtools-desktop --bin nxtools-desktop ref01_ -- --nocapture --test-threads=1` | **4 passed / 0 failed**, including explicit Copy diagnostics | `b1-repair-ref01-green.log` |
| Same command with `tools::polish_tests::` filter | **11 passed / 0 failed** | `b1-repair-polish.log` |
| Same command with `link_panel` filter | **8 passed / 0 failed** | `b1-repair-link-panel.log` |

The explicit Copy regression observed RED before the terminal fix: one test failed because stale case1 had zero same-ID terminal instead of one (`b1-repair-copy-red.log`). Clipboard/window/command assertions already passed at that boundary. Current4-test GREEN proves normal and ownerless completion and stale cancellation together. Original routing RED is preserved in `ref01-red.log`.

Current lint, format and source gates are recorded in `outcomes.md`; targeted behavior does not certify whole-workspace green. The prior full Desktop suite was not rerun for this correction.

## Limitations

No native Windows clipboard/Shell/browser, physical pointer/keyboard, IME/mixed-DPI, accessibility-provider, human visual, packaged or installed acceptance was performed. Tests use the compiled GPUI TestPlatform and original typed command boundary, not the product executable.
