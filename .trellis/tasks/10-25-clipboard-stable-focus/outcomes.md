# Audit #10 only — Clipboard stable row/action/focus identity

## Outcome and boundary

Canonical phase remains **Clipboard + Tools** (current-phase stabilization, no renumbering).
Implemented only original audit #10. Delivery is ready for parent review, **not accepted by this worker**.
No fourth item, recursive delegation, commit/push, package/install, product/PlatformHost/service launch,
real clipboard/native operation, user config/database or registry access was performed.
Existing dirty work was preserved through per-file current dirty baselines. Accepted #1/#9 and shared
`ui/components/action.rs` are unchanged; 13 protected manifest/helper hashes were checked.

## Reference and exact retained ancestry

Read AGENTS/applicable Desktop/shared specs, component selection, current Kit/Base/Component0.8.0,
pre0.3.9 API and clipboard note before production edits. Legacy inspected read-only:
`D:\Code\NxTools\Controls\ClipboardHistoryListView.cs` constructor79–99 and PointerReleased472–491
resolve the actual cached item; `Views\ClipboardHistoryPopupWindow.axaml.cs` constructor61–167,
Prepare206–239, Hide248–259 and keyboard321+ preserve popup/menu/preview/search lifecycle.
The inherited Avalonia native base is unavailable; no native show/hide proof is claimed.
Newly verified entries/invariants were appended to `docs/reference-code/clipboard.md`; prior prefix unchanged.

Exact pre0.3.9 List has `id=None`; render_item/layout_as_root/prepaint do not add an index namespace.
Window::use_keyed_state -> with_global_id uses the current full element stack, including enclosing
entity/window/slot IDs. Therefore just changing the innermost Button is insufficient.
Production changes:
- HistoryRow RenderOnce obtains its actual row FocusHandle at the row's real retained scope, keyed by
  `("history-row-focus", ClipboardItem.id)`; contains_focused includes real child controls.
- Actual slot, row action wrapper, ListItem, Toolbar, open-target Button, source and hover/feather group
  identities use item ID + action kind. Management Button ID is `clipboard-item-action-{item_id}-{kind}`.
- Removed panel-level index-based probing created different keyed handles from actual nested controls;
  deleted it and removed the now-invalid injected actions_focused parameter (affected fixtures adapted).
- Kept original focused_action click owner, border0/full-width/flex/content wrapper and keyboard overlay;
  the shared helper is byte-identical to baseline. Official ListItem, Toolbar and ghost/xsmall management
  Button variants remain. Layout, padding, theme, radius, density, shadow, Markdown height unchanged.
- Index remains only positions/debug selectors/paging/hints; native menu request IDs, IPC correlation
  and database IDs are untouched. Private management helper dropped unused theme parameter rather
  than adding a Clippy warning suppression for its extra immutable-ID argument.

Removal: Window::focused may retain a strong orphan FocusHandle after its node is unmounted.
Context::on_focus_lost is weak-view-bound and runs after rendered tree swap, outside panel rendering.
A non-Tab panel focus_scope identifies a still-mounted ancestor through focus_lost_restore_target.
Only that exact scope + original search_input_ready may restore original search. A replaced/unmounted
panel cannot steal focus; existing inputs/editors/menu/dialog readiness remains original. Focus change
increments GPUI's original focus_generation and invalidates armed Enter/Space releases. No global
keyboard dispatcher, row cache, alternate ListState or synchronous cross-owner update was added.
Existing action diagnostics/correlation and bounded search-focus event remain; no payload/per-frame logs.

Original 4x80=320 window, ListState.splice/measure, query/worker/generation, selected ID alignment,
full-by-ID payload, native menu/preview owner and Ctrl-digit/search/IME/type-anywhere/pause unchanged.
Original open_panel creates a new session and ClipboardPanel; tests reconstruct that actual constructor
in the same synthetic window, not an invented native lifecycle.

## Red/green evidence (commands from repository root)

All Rust commands inherited CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0,
RUSTFLAGS=-C codegen-units=256; serial with full-operation timeouts. No build was killed or duplicated.

1. Before removal fallback, `cargo test -p nxtools-desktop history_identity_ -- --test-threads=1`:
   **7 PASS / 1 FAIL**, deleted_armed_row fails `deleted row must retire actual focus`.
   The old release already did not paste, but the orphan handle did not retire to search.
2. After fallback, same eight tests: **8/8 PASS**.
3. Controlled final-test mutation (after fixture convergence): only actual row focus key changed
   from self.entry.id to self.index. `cargo test -p nxtools-desktop
   history_identity_prepend_keeps_focused_record_and_enter_full_by_id -- --test-threads=1`:
   **0 PASS / 1 FAIL**, `same record retains its FocusHandle after prepend` (search vs old row).
   Immediately restored entry.id through Edit; SHA256 matched pre-mutation whole file exactly.
   This is a final-test sensitivity proof, not a claim that the entire original tree was replayed.
4. Final source: `cargo test -p nxtools-desktop ui:: -- --test-threads=1`: **228/228 PASS**;
   complete final Desktop also contains all 228 UI and **11/11 new identity tests PASS**.
5. `cargo test -p nxtools-desktop ui::history:: -- --test-threads=1`: **16/16 PASS** after adapting
   old fixture focus/hover preconditions. Earlier 226/228 then 227/228 UI runs are recorded failures,
   caused by removed synthetic visibility injection; no feature assertion was removed.
6. Early identity-red/diagnostic attempts had seed-reset/timestamp/paint/scroll-anchor fixture defects.
   They are **not** claimed as valid final-test red/green evidence. Only the explicit records above count.

## Acceptance mapping — actual production panel/ListState/action boundary

New `ui/history_identity_tests.rs`, temporary SQLite/config/images, real worker query callbacks,
real Root/ClipboardPanel/Tab/KeyDown/KeyUp, connected in-memory typed bridge; no native paste.

| Test suffix (all history_identity_) | Verified |
| --- | --- |
| prepend_keeps_focused_record_and_enter_full_by_id | A at index0 -> original Prepend -> index80; same actual FocusHandle and observed owner A; Enter release dispatches original pending ID and full text ending FULL-TAIL, not summary |
| prepend_keeps_favorite_button_and_space_does_not_paste | Real favorite Button retains focus; Space changes only A, not old index occupant; no paste command |
| real_append_front_trim_keeps_row_and_space_on_domain_id | Original Append/worker/front-splice320 cap; A index100 ->20 retains focus; Space targets A |
| real_front_trim_keeps_management_button_without_paste | Same real trim with favorite Button; Enter mutates A, no row paste |
| armed_row_prepend_release_still_activates_original_item_once | Space down before Prepend; release targets A once; duplicate release does not replace pending operation |
| deleted_armed_row_does_not_activate_old_index_occupant | Real DB soft-delete + original refresh Replace; Enter release no paste, orphan retires to existing search |
| removed_armed_management_release_does_not_mutate_or_paste | Favorite Space down then original refresh removes A; no neighbor mutation/paste, old focus retires to search |
| reorder_same_mounted_records_preserves_action_and_search_entity | Actual panel records reordered and ListState remeasure; edit Button same handle, full-by-ID editor A, search InputState unchanged |
| real_front_trim_removed_armed_row_retires_to_search | Original Append/front-trim actually removes armed A; explicit search fallback and no Space release paste |
| equal_domain_ids_in_distinct_panels_have_distinct_focus | Two actual panels/windows with same domain IDs retain distinct real handles; second Prepend does not alter first focus |
| unmount_and_new_session_do_not_reuse_focus_or_armed_release | Original retained panel subscription cannot focus orphan search after root unmount; same window/new actual panel session gets distinct row handle; old Enter up cancels, new press works |

Existing UI regressions cover query/generation/selection/Ctrl-digit replacement and repeats, IME synthetic
composition/type-anywhere, original editor/search entity, native-menu source policy, preview admission and
lease lifecycle; all passed. Physical keyboard/IME and native effects remain unverified.

## Final checks

- strict `cargo clippy -p nxtools-desktop --tests -- -D warnings`: **PASS**. Initial too-many-arguments
  error on the extra item_id was fixed by removing unused private theme parameter, then rerun PASS.
- `cargo check --workspace --tests`: **PASS**.
- `cargo fmt --all -- --check`: **PASS**; only own edited files were formatted (skip_children).
- Exactly one complete `cargo test -p nxtools-desktop -- --test-threads=1`:
  **FAILED — 1184 PASS / 1 FAIL**, 0 ignored/filtered. Sole failure remains
  `archive::application::tests::production_password_application_original_dialog_ticket_and_deferred_grant_never_start_ole`
  at `archive/application_tests.rs:954`, `official unlock Button`. Not modified, not rerun or hidden.
- Source gates PASS: clipboard-ui-lifecycle.ps1, clipboard-lifecycle.ps1, ui-state-lifecycle.ps1,
  ui-token-boundaries.ps1; clipboard-ui-lifecycle.tests.py **5/5 PASS** (including index-key and
  missing panel-scope mutation rejection); gpui-release-contracts.py, markdown-editor-contracts.py,
  spec-contracts.py (**114 documents/14 owners**). These are source/provenance evidence, not visuals.
- Scoped git diff --check: **PASS**.
- Protected #9 nine-file manifest + #1 three remaining unique new files + shared action helper:
  **13/13 hashes unchanged**. #1 files subsequently intentionally changed by accepted #9 are
  compared against #9's accepted current hashes, not obsolete #1 hashes.

## Delivery and remaining limits

`evidence/own-delta.json` contains exact baseline/delivery replacement text and line positions for all
nine existing files. Baselines are the task's current dirty files, NOT git HEAD; unrelated existing
changes are not claimed. New files are directly readable at their workspace paths.
`evidence/manifest.json` records each existing baseline/final hash and new-file hashes; evidence logs
and their hashes are included. Parent can read these workspace files without cross-scratch assumptions.

No task-specific source/compiled/synthetic blocker remains. Complete Desktop remains FAILED as above.
Not executed/not accepted: actual native popup close/reopen/show/hide, real clipboard/paste, native menu
or auxiliary HWND lifecycle, physical IME/keyboard/pointer, real DPI matrices, accessibility reader,
visual comparison or user acceptance, Release/package/install/service. Compilation/tests/screenshots
cannot certify those. Await parent independent review; worker stops after delivery.
