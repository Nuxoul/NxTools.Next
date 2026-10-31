# Parent review — audit #2 only

Reviewed 2026-10-25 by parent session `7faedce5-fc3c-4ab5-bf6d-95910fa4f600` after the fifth single-item delivery from `da6a2ca3-52f0-41f1-a333-3b5548154f0b` (report message `5343df4e-2d51-4580-87c0-e040462d7275`). The implementation conversation reports its user directly authorized this single item; this review does not grant another implementation task.

## Decision

**Accepted for this slice's source, compiled and synthetic keyboard/pointer/sort/persistence/layout-paint behavior only.** No blocking defect found in the one directed parent review. The extra Quick Launch architecture gate remains failing. No native, physical-input/IME, DPI, reader, visual, installed-product, Release or user acceptance is implied. No sixth item is dispatched by this review.

Parent did the review and reruns itself; no reviewer subagent or recursive delegation was used. Canonical phase remains Downloads stabilization.

## Reviewed scope and evidence

- Read PRD/outcomes, entire changed production header function and unchanged surrounding table/persistence interfaces, all 361 appended regression lines and their original MainWindow/DownloadApplication fixture, complete own-delta and saved valid red.
- Verified five delivered source/document hashes and all 32 evidence hashes against the manifest. Independently reconstructed all three task-start dirty baselines from live delivery plus exact operations, checked baseline SHA256, then replayed forward in memory to identical live bytes: 3/3 passed, no workspace writes.
- Independently checked all 15 protected whole files/regions. Production prefix before the header and suffix after it are byte-identical; original menu, sort algorithm, selection/anchor, search/filter, virtual rows/detail and persistence remain. Existing MainWindow/tests.rs is unchanged, and the entire old 924-line kit-test prefix and prior download reference are preserved. Accepted #1/#9/#10/#3 protected implementations/tests are unchanged.
- Actual header contains an official Component0.8.0 Button with original keyed focus/Tab/release activation. Ghost/XSmall/compact/full-slot/zero-extra-padding is an explicit public adaptation to existing dense header geometry; original text tier, arrows, columns and Name inset stay. The Button itself has an action-specific accessible name and readable active direction. No hand-rolled keyboard/focus owner, controller replacement, helper/vendor/native/engine change or other audit-item repair is included.
- Original callback is preserved: set_download_sort(key,true), scroll-to-top, persist_download_sort and notify. The original menu key,false default-direction contract remains unchanged. Same-key reversal/new-key defaults and view-only sorting keep authoritative task/queue order and selection intact.
- Read actual tests for all four sortable headers at default/minimum viewport, Tab from original search, focused semantic node/modality and theme-ring painted quad, Enter/held/release/duplicate release, Space and pointer equivalence, mounted row order, stable dual selection/anchor, temporary config persistence and fresh application/MainWindow reconstruction. Focus transfer cancels an armed release.
- Read saved pre-fix red: 0/3 at missing actual semantic header Button. This is a semantic-boundary red, not proof that later release assertions ran against the original div. Subsequent fixture corrections are recorded, not disguised as product fixes. Old pointer/persistence/menu/geometry assertions are retained.

## Parent-executed checks

Every Rust command inherited `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`, ran serially after natural-idle checks and used full-operation timeouts. No other process/session was paused, terminated or asked to reserve/reorder work.

| Check | Actual result |
| --- | --- |
| `cargo test -p nxtools-desktop sort_keyboard_ -- --test-threads=1` | PASS: 3 passed / 0 failed / 0 ignored; 14.55s |
| `cargo test -p nxtools-desktop main_window:: -- --test-threads=1` | PASS: 177 passed / 0 failed / 0 ignored; 63.15s |
| `cargo test -p nxtools-desktop downloads:: -- --test-threads=1` | PASS: 185 passed / 0 failed / 0 ignored; 43.52s |
| `cargo test -p nxtools-desktop ui:: -- --test-threads=1` | PASS: 231 passed / 0 failed / 0 ignored; 53.64s |
| `cargo clippy -p nxtools-desktop --tests -- -D warnings` | PASS |
| Download UI / Clipboard UI / Clipboard / Desktop shell / UI state / UI token PowerShell gates | PASS: six gates |
| Clipboard gate sensitivity tests / GPUI release / Markdown editor / spec Python gates | PASS: four gates; sensitivity tests 5/5 |
| Exact own-delta / protected checks | PASS: 3/3 and 15/15 |
| Scoped `git diff --check` | PASS |
| Extra `quick-launch-lifecycle.ps1` | FAILED exit1 at line1765; retained, not weakened or repaired |

Parent logs are in its own scratch: `item02-review-sort.log`, `item02-review-main-window.log`, `item02-review-downloads.log`, `item02-review-ui.log`, `item02-review-clippy.log`, `item02-review-quick-gate.log`.

## Integrated evidence and failure boundaries

Parent read the hash-verified implementer full-suite log and compiled exit record: the single full Desktop run is **1196 passed / 0 failed / 0 ignored / 0 filtered**, 225.28s, exit0. Parent additionally checked the log contains #1 keyboard8, #9 picker14, #10 identity11, #3 names7 and current sort3 all passing, and Archive Unlock passing. This is implementer-run integrated evidence, not a parent full-suite rerun. Workspace check/fmt also remain separately attributed implementer evidence.

Earlier Archive Unlock failures remain historical records. This item changes only the header function and appends tests/reference; there is no Archive fix in its own-delta. Unrelated shared-tree convergence/test-count changes are not attributed to sorting and do not certify installed behavior. Parent did not rerun the complete suite or hunt for a green result.

Parent independently reproduced the extra Quick Launch gate failure at the same elevated-navigation source compound. Prior bounded review identified the direct build-script manifest-writer string check versus delegation through publish-release-hosts.ps1. Neither this item nor parent edits build/release/security code or that gate. The failure remains a separate verification gap; no elevated/native/security/release boundary is accepted here.

## Limits and stop

No real download/network/native Shell/OLE/picker/clipboard/product/PlatformHost/service action, real user configuration/database/registry mutation, package/install, commit/push, physical keyboard/pointer/IME, real DPI/mixed-monitor, UIA/reader, screenshot/visual comparison or user acceptance was performed in parent review. Only temporary/in-memory/synthetic interactions and static evidence are accepted.

The fifth single item (original #2) is reviewed and accepted at those limited evidence levels. The implementation conversation remains stopped. A later item requires a new explicit single-item dispatch under the existing review-before-next-item process; this record does not authorize it.
