# Parent review — audit #3 only

Reviewed 2026-10-25 by parent session `7faedce5-fc3c-4ab5-bf6d-95910fa4f600` after the fourth single-item delivery from `da6a2ca3-52f0-41f1-a333-3b5548154f0b`.

## Decision

**Accepted for this slice's source, compiled and synthetic Button role/name/state/interaction behavior only.** No blocking defect found in the one directed parent review. Full Desktop and the additional Quick Launch architecture gate remain failing as recorded below. This is not Windows UIA/provider/reader, physical input/IME, native effects, DPI, visual, installed-product or user acceptance.

The implementation conversation stopped; parent reviewed and reran checks before any new single-item dispatch. No reviewer subagent or recursive delegation was used.

## Reviewed change boundary

- Read PRD/outcomes, exact current released Button naming logic, all 197 lines of shared helper semantic tests, production Global/Downloads/path-delete tests and every existing-file own-delta.
- Verified all 15 source/document hashes and all 30 evidence hashes match the delivery manifest. Independently reconstructed all 12 dirty baselines from live delivery and exact operations, checked baseline SHA256, then replayed forward in memory to identical delivery bytes: 12/12. No product-file mutation during this check.
- Independently verified six protected whole files and three exact prefixes: 9/9. Shared focused_action/focusable_content, NewDownloadView constructor/picker/keyboard/probe/submit prefix, Settings constructor/picker/save prefix remain byte-identical. Accepted Clipboard history/panel/identity tests and picker/key regressions remain unchanged. Whole changed files are not falsely described as unchanged.
- Shared primary helper adds one required semantic name separate from existing IntoElement content. Shared icon helper likewise requires a name; icon_action forwards its existing name once, retaining one original Button and Tooltip. Actual Button::accessibility_label is the released public seam. No new semantic parent, focus, activation or dispatch owner was introduced.
- Reviewed all 12 production primary and six external icon-helper callsites, including parameterized diagnostic objects. Names are action/object-specific; dynamic busy text remains visible and names remain stable by action. Technical IDs do not become names. Path delete identifies its own excluded path without changing callback/index/frozen-path behavior or logging it.
- Visible labels/content, original variants/tiers, parent adaptation, disabled readable paint, picker/request identity, single release click and deferred native/feature owners remain. The existing contrast test keeps its assertions; local formatting around named helper calls does not change logic.
- Existing Download UI source assertion changes only the named drag helper signature, not its original behavioral requirement. Appended reference records exact API/reference limits; no vendor/native/protocol/dependency/build/security source change is part of this own-delta.
- Read the saved valid red: all seven tests compiled and failed at actual Button name None versus expected, after role identification. Initial fixture compilation and later focus/natural-width fixture failures are not misrepresented as product defects or valid reds.

## Parent-executed verification

Rust commands inherited `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`, executed serially after natural-idle checks. No other process/session was paused, killed or asked to reserve/reorder work.

| Check | Actual result |
| --- | --- |
| `cargo test -p nxtools-desktop accessible_name_ -- --test-threads=1` | PASS: 7 passed / 0 failed / 0 ignored; 0.88s |
| `cargo test -p nxtools-desktop downloads:: -- --test-threads=1` | PASS: 185 passed / 0 failed / 0 ignored; 43.61s; includes accepted picker/key regressions |
| `cargo test -p nxtools-desktop network_settings:: -- --test-threads=1` | PASS: 35 passed / 0 failed / 0 ignored; 7.20s |
| `cargo test -p nxtools-desktop ui:: -- --test-threads=1` | PASS: 231 passed / 0 failed / 0 ignored; 52.95s; includes accepted Clipboard identity and contrast/interaction regressions |
| `cargo clippy -p nxtools-desktop --tests -- -D warnings` | PASS |
| Download UI / Clipboard UI / Clipboard / Desktop shell / UI state / UI token PowerShell gates | PASS: six gates |
| Clipboard gate sensitivity tests / GPUI release / Markdown editor / spec Python gates | PASS: four gates; sensitivity tests 5/5 |
| Exact own-delta replay / protected logic | PASS: 12/12 and 9/9 |
| Scoped `git diff --check` | PASS; existing CRLF normalization warning only, not whitespace error |

Parent logs in its scratch: `item03-review-names.log`, `item03-review-downloads.log`, `item03-review-global.log`, `item03-review-ui.log`, `item03-review-clippy.log`, `item03-review-quick-gate.log`. Workspace check/fmt and the single integrated Desktop run remain separately attributed implementer evidence, not parent reruns. A first parent gate aggregator mixed native output into its result array and returned a false aggregate failure; corrected aggregation captured each native exit explicitly. Final ten related gates all exited zero; no product/gate change was made for that harness correction.

## Failures retained, not repaired in this slice

1. Implementer's one full Desktop run remains **FAILED: 1191 passed / 1 failed**, 0 ignored/filtered. Sole failure is Archive Unlock at `archive/application_tests.rs:954`, the same Archive-only fixture previously reproduced by parent. No Archive own-delta is present. Parent did not rerun the full suite or repair another owner.
2. Parent independently ran the additional `quick-launch-lifecycle.ps1`: **FAILED**, exit1 at line1765. Read its failing compound condition and bounded delivery diagnosis. Its direct `$buildScriptSource` check requires `write-release-binary-manifest.ps1` text. Current `scripts/build.ps1` delegates to `publish-release-hosts.ps1`; that script calls the manifest writer at line22. This explains the source-string mismatch, but does not certify the complete elevated-navigation/security/release boundary. This task changes neither script nor this gate; failure is retained, not weakened or declared accepted.

These are integration/release follow-ups, not evidence that this names-only slice changed those owners. No complete-product green claim is made.

## Limits and sequential handoff

Actual UIA/provider/reader, physical keyboard/pointer/IME, real native picker/clipboard/Shell/HWND behavior, DPI, screenshots/visual comparison, installed or user acceptance remain unexecuted/unaccepted. No product/PlatformHost/service, real user config/database/registry, package/install, commit/push operation was performed by parent review.

This accepts only original audit #3 at the evidence levels above. The next work requires a separate single-item delivery for original audit #2 (download sort keyboard equivalence). It must stop for parent review before any sixth item; no bundled backlog execution is authorized.
