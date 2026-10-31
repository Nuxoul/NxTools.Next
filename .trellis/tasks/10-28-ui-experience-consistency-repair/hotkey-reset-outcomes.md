# Summon-hotkey reset follow-up — 2026-10-28

## User request and source behavior
Clipboard and Quick Launch custom global summon shortcuts now show official outlined `重置` immediately left of the displayed shortcut, within their original Settings row. Core defaults stay Clipboard `Alt+C` and Quick Launch double Ctrl (empty modifiers + `Control`). Default bindings have no reset element; localized display text/key case alone does not count as a custom binding.

The original Clipboard recorder is retained; the shared Quick Launch hotkey field has an optional official Button slot, keeping its original bounded width and status line. Both Buttons use released Medium/default32DIP; no Toolbar adaptation, fake control, vendor or palette change.

Each original application exposes fixed-default reset admission, checking connected state, no configuration in flight and no recording (including pending pause/cancel). The UI defers to that original owner, which rechecks. Arbitrary recorded shortcuts still require the original acknowledged recording gate. Only the shared private configuration-submission fragment was extracted; no second transaction/pending controller. Reset reuses ConfigureClipboard/ConfigureQuickLaunch, original pending candidate, matching CommandResult, section persistence and failure presentation. No optimistic memory/disk default publication. Failed native registration retains custom binding and retry; original applied-but-save-failed message remains unchanged. Submitted/completed events record request IDs/result/native code, never shortcut text.

## References and real red/green
Reference notes were read first, then legacy MainWindow Configure/capture/Escape -> ClipboardNativeService hidden-window/configure -> ClipboardHotKeyService RegisterHotKey/dispose, and SwiftList HotkeyPageSettings/default/control-to-native ShowWindow. Popup/focus/native registration policies unchanged. Notes contain newly verified boundaries.

New tests mount actual MainWindow Settings with temporary config/SQLite and fake IPC, never real global hotkey registration:
- red1: actual missing nondefault reset Button; second default-absence fixture assertion exposed unrelated fake bootstrap messages, drained only before gesture measurement.
- green1:7PASS/1FAIL; old hotkey tests and default absence passed. Keyboard retry used simulate_keystrokes, which lacks official Button KeyUp; corrected to actual KeyDown/KeyUp and asserted zero effect before release.
- green2:7PASS/2FAIL; enabled semantic snapshots were incorrectly expected Some(false), while exact Button uses None when enabled/Some(true) when disabled. Corrected only enabled assertions; kept actual click/key/repeat/ACK/disk/recording/disconnect checks.
- green3: **9PASS/0FAIL/0ignored**,4.25s, Desktop test compile2m56s. Three new cases cover both pages: semantic default absence; actual left/same-row32DIP bounds; pointer and focused Enter release; duplicate pending no effect; wrong/rejected/success ACK; original memory/disk save; recording pause-pending/active/cancel and disconnect UI+owner rejection. Original recorder/cancel/registration cases remain green.

## Verification
- `cargo test -p nxtools-desktop --bin nxtools-desktop hotkey -- --test-threads=1`:9PASS.
- `cargo check --workspace --tests`:PASS25.97s.
- Original shared-editor ordinary-size geometry regression:1PASS/0FAIL/0ignored,0.12s; actual original hotkey width/fixed viewport preserved.
- Strict Desktop bin/tests Clippy:FAIL101 on preexisting unused SPACING_12 at main_window.rs75 and unread floating_assistant/media.rs62 animated field. No suppression or unrelated owner correction; not a strict-lint pass.
- Specs114documents/14owners, catalog79historicalpages/78entries, coherent release/exact carried deltas, editor57file provenance:PASS.
- Clipboard UI/command-result/reference lifecycle, shared UI-state and UI-token source gates:PASS.
- Quick Launch lifecycle source gate:stillFAIL at unrelated QL11custom-command-editor old token contract458. Gate unchanged.
- Owned Rust format, scoped diff and task context4+2:PASS. Per-command scratch logs/results `hotkey-reset-*`; source SHA256 snapshot retained.

## Scope/acceptance limits
No product launch, retained live-window operations, service/Registry change, real user configuration/history/Clipboard/credentials/network, reference execution, package/install, commit/push/archive or delegation. No Windows hotkey native code or IPC/schema/dependency/vendor change. Pointer/keyboard here are TestApp events; physical input/real registration-conflict/Windows IME/DPI/screenshots/user visual/provider/installed acceptance remain unperformed. Earlier six-item UI counts predate this follow-up and are not current full-worktree acceptance. HTTP and packaging remain paused.

## Final disposition
Hotkey reset source implementation and affected tests completed. Status remains
`source_completed_verification_limited` because strict whole-bin lint and QL11source gate are not
green; native/user acceptance unperformed. Final owned source snapshot unchanged at18:28+08.
No further source change, build, package, HTTP continuation, commit or archive is required here.
