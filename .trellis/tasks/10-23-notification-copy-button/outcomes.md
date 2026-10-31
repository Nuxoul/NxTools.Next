# Notification header copy and GPUI Kit fork preservation — 2026-10-23

## Delivered locally
- Actual official XSmall ghost Copy icon immediately left of original Close, equal height/no overlap; Copy/Check accessible label and Tooltip. Long title reserves room while original natural body/scroll/actions stay intact.
- `Notification::header_action(builder -> AnyElement)` is optional/defaultNone. Same official card/list/clock/hoverpause/exit/native passive lifecycle; caller owns business copy state. Optional action omitted during Ending. Only existing notification.rs vendor file changes; no new backend/GPUI patch or dependency/version switch.
- Header focus scope remains visible during keyboard modality. Actual regression went red because GPUI KeyDown suppresses group_hover and hidden toolbar drops keyboard listeners; hover OR headerfocus fixes Space release through the original official Button. No duplicate manual key dispatcher, globalCtrlC or new hook.
- CopyButton defers original NotificationCenter ID/generation admission. One pending caller-ID-correlated PasteText(copy-only,targets0), <=3retained status records. Snapshot title/message/first8displayeddetails; <=256KiBUTF8 preflight before allocation, reject oversize without truncating. Existing C# WindowsPasteCoordinator returns after write before activation/paste branch.
- Matching native success shows Check until replacement/retirement. Fixed errorTooltip and retry on enqueue/nativefail/15sdeadline/disconnect. No new success notification, display clock, notification repush/close/action claim/timer reset. Late/old result cannot mark replacement. Already admitted Windows write cannot be retracted and is not claimed cancellable.

## Verification actually performed
All Rust commands serial with CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256. Other task Cargo activity was observed before starting operations; no foreign process terminated.

| Check | Result |
| --- | --- |
| Final notification-related focused suite | **46 passed,0failed** |
| Full `cargo test -p nxtools-desktop -- --test-threads=1` | **1102 passed,0failed,0ignored** |
| `cargo clippy -p nxtools-desktop --tests -- -D warnings` | **PASS** |
| `cargo check --workspace --tests` | **PASS** |
| `cargo fmt --all -- --check` | **PASS** |
| Fork/local notification rustfmt check after final pure field wrap | **PASS** |
| Desktop shell/UI state/UI motion/UI consistency/Clipboard UI lifecycle source gates | **PASS** |
| GPUI release/catalog, Markdown editor provenance, spec navigation and task JSONL validation | **PASS** |
| Scoped git diff whitespace check | **PASS** |
| Temporary DEBUG-NOTIFICATION-COPY/eprintln/dbg/todo probes | **NONE remaining** |

Red evidence: original actual button test failed because no copy button existed. Added keyboard test failed until focus-within toolbar rule; long-title test initially assumed Label semantic node but now asserts actual rendered title text container and passes. Pending/result/copy-only/fullpayload/failure/retry/stale/timeout/disconnect/closed/oversize/originalexpiry regressions all pass. Temporary test probes and ineffective wrapper/forced-refresh experiments removed.

One optional bounded read-only review was attempted, then cancelled after prolonged no-report; it is **not a passed review**. No second review. Parent source checks and concrete regression loop are the recorded verification.

The final source change after full compiled verification was one rustfmt-only line wrap in the header_action_builder field; local and fork file formatting/parity checked afterward. No application behavior changed after the full test run.

## Fork publication — completed and verified
Repository: https://github.com/Nuxoul/gpui-kit ; branch `main`.
Original remote main `627ba35afc08ffb49c34affd0af587b2ccf8d836` remains ancestor; no force-push/reset/history replacement. Published Component0.8.0 provenance is upstream `1676826d9d49b90f4d7b6808b69924f58796f6f1`; port focused hunks, retain newer main `fully_visible_bounds().size` correction.

1. [`58dc3881c2c486201a1336a66edcbc27a9fbc2ce`](https://github.com/Nuxoul/gpui-kit/commit/58dc3881c2c486201a1336a66edcbc27a9fbc2ce) — configurable notification duration, upstream5sdefault/official clock retained.
2. [`a5b185c7a7a153f92e5ba699077b1135773a116c`](https://github.com/Nuxoul/gpui-kit/commit/a5b185c7a7a153f92e5ba699077b1135773a116c) — optional compact header action with keyboard focus visibility and additive builder regression.
3. [`c00db47e214840168ede499cd3d8fe57408fd7f1`](https://github.com/Nuxoul/gpui-kit/commit/c00db47e214840168ede499cd3d8fe57408fd7f1) — DOWNSTREAM.md public APIs/provenance/limits and external Windows archival JSON.
4. [`edffc53bf292487a26039a3db435c3d1482770de`](https://github.com/Nuxoul/gpui-kit/commit/edffc53bf292487a26039a3db435c3d1482770de) — rustfmt-only field wrap. **Verified final remote HEAD**.

Only three fork paths differ from initialmain: `crates/component/src/notification.rs`, `DOWNSTREAM.md`, `downstream/external/gpui-pre-windows-0.3.9-noactivate.json`. No NxTools application/config/data, theme/editor source, lock/version/backend dependency changes uploaded.

External gpui-pre-windows is NOT source-owned by Kit; no foreign package copied or activated. Its archival JSON records immutable crates.io0.3.9 archive SHA256, exact match-once replacement for WM_MOUSEACTIVATE/liveWS_EX_NOACTIVATE and source origin. Downloaded archive checksum verified and replacement replay matches active native code modulo comments/whitespace. Apache license stays with original package. This preserves the user's special modification without pretending it is upstream Kit source.

Initial anonymous-URL push failed noninteractive credential lookup. Retried existing account-qualified URL `https://Nuxoul@github.com/Nuxoul/gpui-kit.git`; existing Git credential manager authenticated normally. No credential printed/changed/uploaded. Normal push succeeded; `ls-remote`, then fetch `origin/main`/HEAD/diff confirmed final hash and exact files; fork status clean. Local and fetched fork notification source match except the intentionally retained upstream bounds line. Standalone fork full build was **not run**; source-equivalent component is exercised through current NxTools integration, not a claimed independent fork build.

## Not performed / acceptance limits
No normal product launch/package/install/services/registry mutation, actual Windows clipboard read/write, physical keyboard/pointer/mixedDPI/accessibility-reader or user visual acceptance. These remain separate from synthetic production GPUI input/native-command tests. NxTools worktree remains dirty with all unrelated user work preserved; no NxTools commit/push was made. Fork publication only was explicitly authorized.

Logs under session scratch: `notification-copy-red.log`, `notification-copy-focused-final.log`, `notification-copy-desktop-full.log`, `notification-copy-clippy-final.log`, `notification-copy-workspace-final.log`, `notification-copy-fmt-final.log`, `notification-copy-static-results.json`, plus bounded red/diagnostic logs. Old failed hypotheses are historical, not current blockers. Current implementation and requested fork push have no unresolved technical blocker.
