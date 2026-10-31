# Audit #1 — scoped delivery and evidence

Local verification completed 2026-10-25 (task/audit requested 2026-10-24). Delivered for parent review, **not accepted**. Do not begin any other audit item without a new single-item task.

## Own changes, not the repository's complete dirty diff

The working tree was already extensively dirty. Baselines were copied before editing each existing file; no reset, commit, push or broad formatting was used.

| File | This session's exact change |
| --- | --- |
| `rust/apps/nxtools-desktop/src/downloads/new_download_view.rs` | Import existing EntityInputHandler trait; replace window-wide Enter subscription with local held state + activation-reset subscription; add `default_enter_key_down`; bind root capture down/up; mark category-confirm Enter already owned before it restores outer focus; register private keyboard test module. Relative to session dirty baseline: +68/-22 lines. |
| `rust/apps/nxtools-desktop/src/downloads/new_download_keyboard_tests.rs` | New isolated production-view keyboard fixture and eight tests, 417 lines. No direct submit/probe invocation used as keyboard evidence. |
| `rust/apps/nxtools-desktop/src/downloads/tests.rs` | Update old Enter source gate to the new handler. Existing schedule pointer lifecycle fixture now scrolls a clipped late-week date into the actual viewport before clicking, preserving all original draft/commit/clear/dismiss assertions. Relative baseline: +38/-11. No production calendar change. |
| `docs/reference-code/download.md` | Append-only Audit #1 prerequisite/source/invariant record (+29 initially); result clarification below. Preserve previous note byte-for-byte as prefix. |
| `.trellis/tasks/10-24-download-enter-popover/prd.md`, `outcomes.md` | New single-item scope and delivery record only. No thirteen-item implementation plan. |

Own-delta evidence is in this session scratch (`da6a2ca3-52f0-41f1-a333-3b5548154f0b`): `new_download_view.rs.own-delta.txt`, `tests.rs.own-delta.txt`, `download.md.own-delta.txt`, `download-enter-own-manifest.json`. These are review aids, not patch files to apply. Existing-file baseline SHA256:
- new_download_view.rs: `593a561126fc00b5fb007a84fb21e3abcb3590a8fce703c0866fb46fa0d69ea8`
- tests.rs: `c3b29567ec984756f4c8f47c67f28f4a0d807c387f4d8b98083b127f9cb7af5a`
- download.md: `80b2dd51fa97e6116f394b87f0b9c2cfa4d2541f0e0124044baa332213fc4837`

## Reference and implementation boundary

Read AGENTS, component selection/current UI amendments, current 0.8.0 release/index/table, Desktop/shared engineering/verification specs and download reference before implementation. Fetched both official design/coding guides directly to own scratch. ABDM AddDownloadPage/MainActionButtons and BaseOptionDialog were inspected first, legacy AddDownloadWindow AXAML/code-behind and WindowAnimationService second. Constructor -> Opened/Closed and guarded deferred close traced; Avalonia/AWT native base sources are unavailable, not claimed as inspected. Exact published GPUI pre0.3.9 dispatch and Base/Component0.8.0 Input/Popover/Select/Button sources inspected read-only. No dependency/vendor/registry source changed.

Default Enter is a fallback after unconsumed official actions, restricted to the original URL/directory/file-name focus handles. Schedule/category open state, modifiers, character preference, marked text, held/down-repeat, probing/submitting, handoff/native viewport gates reject it without swallowing the child control's gesture. A local press latch prevents synchronous validation failure from admitting a second down; key release/activation change retires it. Category's existing interceptor marks its handled Enter before focus return, preventing the same key from reaching the outer default path. No new command, business owner, window or polling.

Byte-equality extraction against the starting dirty baseline confirmed `request`, `reset_schedule_draft`, `begin_probe`, `begin_probe_with_mode`, `submit`, `schedule_fixed_confirmation`, `fixed_confirmation_draft` and `hydrate` unchanged. Schedule draft is committed only by existing Confirm, Clear removes it, dismissal preserves old committed state; fixed client/layout/theme/native/C# ownership and deferred cross-entity effects retained. No added diagnostics or temporary DEBUG markers; original correlated probe/submission diagnostics remain. No text, URL or path payload logging added.

## Red / green chronology (including failures)

All Rust commands inherited CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, RUSTFLAGS=-C codegen-units=256. Commands were serial in this session; when another Cargo/rustc existed, this session waited and did not start a duplicate or terminate it. No wrapper timed out and no own build remains running after completion.

1. Initial test fixture compilation failed on borrowed VisualTestContext/Select borrow, then corrected in the new test file. This is not a behavioral red.
2. **Before production fix:** `cargo test -p nxtools-desktop new_download_keyboard_ -- --test-threads=1`: RUN6, 1 passed/5 failed. Actual production keyboard events entered outer validation behind active schedule/IME/held guards. Initial button fixtures lacked final focus-snapshot proof; final versions use real Tab + mounted Button focused snapshots, rather than overstating the initial red's focus certainty. Log: `download-enter-red.log`.
3. First post-fix RUN6: 5 passed/1 failed due fixture using target999 below actual maximized test viewport. Changed fixture to actual viewport+100 and asserted native gate is pending; no production native-gate change.
4. Added category regression exposed a real same-key fallthrough after category focus return; fixed only this default-Enter boundary. Positive submit fixture compile fields and closed-window second key were corrected. Compact snapshot intentionally omits schedule request data; full accepted task is read from the same in-memory engine, not mistaken for a product loss.
5. **Focused green:** same filter, RUN8/8 PASS, no warnings. Tests cover schedule active with retained outer focus, official Select draft confirmation, real focused Clear Space/Confirm Enter/Escape, ordinary Cancel Enter, all three outer inputs, modifiers/held/repeated down/character preference/IME, busy/native gates, category same-key fallthrough, and exactly one full accepted future task using committed (not dirty draft) schedule. `download-enter-green.log`.
6. First existing Downloads run: 166 passed/2 failed. One stale source-string gate expected removed Enter listener; synchronized. Other fixture clicked an offscreen date: measured day y130.5..149.5 vs time row starting140.5; added visibility assertion first (red), then bounded actual scroll-before-click. All original lifecycle assertions retained, production calendar unchanged.
7. **Downloads final:** `cargo test -p nxtools-desktop downloads:: -- --test-threads=1`: **168 passed, 0 failed, 0 ignored**. `download-enter-downloads-final.log`.
8. Strict Clippy initially flagged only new nonminimal bool; algebraically simplified with existing handoff_pending disjunct. **Final `cargo clippy -p nxtools-desktop --tests -- -D warnings`: PASS**. `download-enter-clippy-final.log`.
9. Formatting initially reported only new closure/helper formatting; applied exact local formatting. **`cargo fmt --all -- --check`: PASS**; **`cargo check --workspace --tests`: PASS**. `download-enter-fmt-final.log`, `download-enter-workspace-check.log`.
10. **Single final integrated Desktop run:** `cargo test -p nxtools-desktop -- --test-threads=1`: **1158 passed / 2 failed / 0 ignored** (187.52s). This is FAILED, not a green full suite. No whole-suite green hunting. `download-enter-desktop-full.log`.

### Full-suite failures outside this slice (not repaired)
- `archive::application::tests::production_password_application_original_dialog_ticket_and_deferred_grant_never_start_ole`: `archive/application_tests.rs:954`, expected official unlock Button not found.
- `main_window::extensions_page::extensions_settings::tests::remediation_usage_settings_opens_editable_settings_directly`: existing ExtensionManagerView already-updating panic during settings focus; log includes another surface's DEBUG-extensions-focus marker, not introduced by this session.

Neither failure is in a file/owner changed by #1. They are recorded as integration blockers, not declared proven pre-existing from this run alone. This task does not modify those owners or grant them acceptance.

## Static checks

Passed after production repair: GPUI component catalog, GPUI release contracts, Markdown editor contracts, spec contracts, Download UI lifecycle, UI token boundaries and scoped `git diff --check`. Final docs/source rerun recorded at delivery. These check structure/provenance only, not physical/native/visual acceptance.

## Limits and stop condition

Source + compiled + synthetic GPUI keyboard/render and in-memory accepted task evidence only. No product/service/native host launch, actual Windows IME/physical keyboard/mouse, real HWND/native handoff, DPI matrix, accessibility reader, screenshots, installed or user visual acceptance. No Release/package/install, real user data/config, commit/push or other audit item. Full Desktop has the two recorded unrelated-owner failures; parent review remains pending.

During execution this session improperly requested other conversations reserve/defer Cargo time. The user rejected that request; it was explicitly withdrawn to all recipients. No conversation or process was actually paused, cancelled or terminated. Subsequent occupied-build handling was wait-only. Never repeat that coordination request.

Deliver #1 now and stop. Further work is limited to parent revisions on #1; starting another item requires a new single-item task.
