# Unresolved CreateOverwrite keyboard diagnostic — not a default regression

Status: **UNRESOLVED / NOT ACCEPTED**. Moving this newly introduced diagnostic out of the default Desktop suite does not fix or accept CreateOverwrite keyboard activation. Parent review R1 explicitly requests this placement change and prohibits a product keyboard repair in audit #4.

## Preserved source and failure

`create-keyboard-unresolved-probe.rs.txt` is the complete original `#[gpui_kit::test]` function, copied byte-for-byte from the R1/R2 start snapshot of `archive/job_tests.rs`. It retains its real Tab traversal, focused Button assertion, explicit Space KeyDown/KeyUp, expected reply count **1**, duplicate-release expectation, and the entire later fallback/cancel section. No assertion is weakened, ignored or conditionally bypassed.

`job_tests.before-r1-r2.rs.txt` preserves the complete original compilation context. Its SHA256 is `1723f2428a980d14a06871069c41ae5a92c5746f308e7e37c52f89347a5c639e`. The probe originally occupied lines 1070–1145; the actual focused Button assertion at 1091 passed before the reply assertion at 1102 failed with actual **0**, expected **1**. That later fallback/cancel section was not executed in the failing diagnostic. A separate standard pointer/fallback regression verifies that behavior.

The following original evidence remains immutable in the parent evidence directory:
- `desktop-object-final.log`: seven selected tests, six passed and this diagnostic failed.
- `desktop-full-once.log`: the sole complete Desktop run, **1207 passed / 1 failed / 0 ignored**. This failed historical run is not replaced by a new complete suite.
- `desktop-object-input-green.log` and `desktop-object-expanded.log`: earlier input-probe failures; their names are historical, not assertions that those runs passed.
- `delivery-manifest.json`, `own-deltas.json` and `late-instruction/`: original hashes, deltas, full diagnostic snapshot, footer comparison and attribution limits.

## Reproduction context

The saved function is not standalone and is deliberately not included by any Rust module or Cargo target. It needs the original private `archive::application::jobs::tests` module context and these unchanged test helpers: `request`, `completion`, `object_prompt`, `show_object_prompt`, `object_replies`, `object_node`, `object_draw`, and `object_labels`; the module's production `ArchiveApplication`, protocol imports, `Uuid`, and `super::super::tests::setup` are also required. The complete saved source provides those exact definitions.

Historical commands, run before R1 removal:

```text
cargo test -p nxtools-desktop audit04_ -- --test-threads=1
cargo test -p nxtools-desktop -- --test-threads=1
```

Required environment was `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`. A narrow selection of the original function would use `cargo test -p nxtools-desktop audit04_create_keyboard_release_and_missing_object_cancel_are_once_only -- --test-threads=1` only in an explicitly authorized isolated reproduction retaining the original private module context. That narrow command is explanatory; it was **not** run during R1/R2. After relocation the current default product target no longer registers this diagnostic; a zero-match command is not reproduction or acceptance. Do not restore it into shared production sources or change dispatch ownership merely to obtain a green result.

## Scope and remaining limitation

Source comparison preserves the original `DialogAction.child(Button)`, `on_ok`, `on_cancel` and Conflict decision callback lines. Exact Component0.8.0 source supports the documented callback-owner limitation, but no complete old-UI runtime comparison was performed. No no-op `on_click`, new dispatcher, vendor patch, #5/default-Enter repair, keyboard-success claim, physical-input/IME/DPI/UIA/reader or visual acceptance is introduced.

The six standard audit04 tests remain active: object display, two jobs/consecutive tickets/once-only decisions, CreateOverwrite pointer/missing-object cancellation, Conflict keyboard activation, and long-identifier scrolling/fixed-footer reachability. Their results concern audit #4 only, not this outstanding keyboard diagnostic.
