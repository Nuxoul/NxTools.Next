# Parent review — audit #10 only

Reviewed 2026-10-25 by parent session `7faedce5-fc3c-4ab5-bf6d-95910fa4f600` after the third single-item delivery from `da6a2ca3-52f0-41f1-a333-3b5548154f0b`.

## Decision

**Accepted for this slice's source, compiled and synthetic GPUI identity/focus/action behavior only.** No blocking defect found in the one directed parent review. This is not complete Desktop, native clipboard/paste/popup/HWND, physical input/IME, DPI, reader, visual, installed-product or user acceptance.

The implementation conversation stopped after #10; parent reviewed and reran the checks below before authorizing any next single item. No reviewer subagent or recursive delegation was used. Canonical phase remains Clipboard + Tools stabilization.

## Reviewed changes and evidence integrity

- Read PRD/outcomes, full history identity implementation and 824-line identity fixture/tests, panel constructor/render focus scope, every existing-file own-delta, affected old fixture changes and updated lifecycle gates.
- Verified all 12 delivery source/document hashes and all 25 evidence file hashes against the delivered manifest. Independently reconstructed all nine existing dirty baselines from the live delivery plus exact operations, checked baseline hashes, then applied the operations in memory and checked exact live delivery bytes: 9/9 passed, no workspace mutation.
- Independently checked protected accepted #1/#9 plus shared action helper: 13/13 unchanged. Existing unrelated dirty state is not attributed to this item. Shared action helper SHA256 remains `55bea700ba6460dc5e4ee35d61c2f3e8f4d345ef041849463404ff9166b9e865`.
- Production retained row focus key, slot, action wrapper, ListItem, Toolbar, open/source and management action IDs use immutable record ID plus kind within original panel/window ancestry. Positional debug selectors and original paging indices remain positional intentionally.
- The new RenderOnce obtains the actual FocusHandle in the row's real scope and uses contains_focused for child action visibility. The old panel projection created different keyed focus handles from the actual nested controls; it is removed, not replaced by another dispatcher/cache/controller.
- Reviewed wrapper equivalence against unchanged focused_action/focusable_content behavior: original one release-click owner, disabled handling, border0/full-width/flex/content and keyboard outline are retained. Existing official controls, geometry, gradients/shadows/Markdown, row handlers, full-by-ID and cross-owner deferred effects remain.
- Reviewed the exact independently hash-verified gpui-pre0.3.9 List/render/layout, use_keyed_state/global ID and focus lifecycle APIs in parent scratch. List adds no per-index element namespace. on_focus_lost is weak-view-bound and dispatched after the new rendered tree replaces the old one. focus_lost_restore_target resolves only a still-mounted ancestor; original Window::focus increments focus_generation.
- The non-Tab panel scope qualifies orphan-row fallback to the original ready search input. An unmounted panel cannot restore its old search through this callback. Actual removal/trim/armed release/new session/two-panel regressions exercise those contracts.
- Old fixture adjustments remove synthetic actions_focused injection and use real hover/Tab instead; original feature assertions are retained. No unrelated owner is changed to obtain a green run.

## Parent-executed verification

Every Rust command inherited `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`, executed serially after natural-idle checks. No other session/process was paused, terminated or asked to reserve build time.

| Check | Actual result |
| --- | --- |
| `cargo test -p nxtools-desktop history_identity_ -- --test-threads=1` | PASS: 11 passed, 0 failed, 0 ignored; 18.12s |
| `cargo test -p nxtools-desktop ui:: -- --test-threads=1` | PASS: 228 passed, 0 failed, 0 ignored; 52.98s |
| `cargo clippy -p nxtools-desktop --tests -- -D warnings` | PASS |
| Clipboard UI / Clipboard / shared UI-state / UI token PowerShell gates | PASS |
| `python tests/architecture/clipboard-ui-lifecycle.tests.py` | PASS: 5/5 gate sensitivity tests; temporary fixture copies, not product mutation |
| GPUI release / Markdown editor / spec Python gates | PASS |
| Exact bidirectional own-delta / protected hashes | PASS: 9/9 and 13/13 |
| Scoped `git diff --check` | PASS |

Parent logs are in its scratch: `item10-review-identity.log`, `item10-review-ui.log`, `item10-review-clippy.log`. Implementer's workspace check/fmt and one full Desktop run remain separately attributed delivery evidence, not parent reruns.

Read the saved valid deletion-fallback red and final index-key mutation red. The latter fails the same-record FocusHandle assertion after Prepend; it is sensitivity evidence for the final converged test, not a replay of the entire original tree. Early fixture failures are not counted as behavioral red proof. Parent did not repeat production mutations or run another broad review.

## Integration and acceptance limits

Implementer's single complete Desktop run remains **FAILED: 1184 passed / 1 failed**, 0 ignored/filtered. Remaining Archive Unlock failure at `archive/application_tests.rs:954` is the same independent Archive-only fixture already reproduced by parent in #1 review and recorded before this item. This own-delta does not touch Archive. No additional full-suite run or green hunting was performed; this is not whole-product acceptance.

Actual native popup close/reopen/show/hide, clipboard/paste/menu/auxiliary HWND behavior, physical keyboard/IME/pointer, real DPI matrices, accessibility reader, screenshot/visual comparison and user acceptance remain unexecuted/unaccepted. No product/PlatformHost/service launch, real user data/config/registry changes, Release/package/install, commit or push occurred in this parent review.

## Next dispatch

This closes only original audit #10 at the evidence levels above. A separate single-item task may now address original audit #3 (accessible names of actual buttons). That item must stop for a new parent review; no bundled backlog or automatic continuation is authorized.
