# Quick Launch shared query work — 2026-11-17

## Outcome

Two computation paths now avoid work that does not contribute to a different visible result:

1. Retained file/directory queries share the existing matching walk. Normal results keep independent per-kind quotas, the original score/path comparator, exclusions, delta suppression and preferred-directory score. Cold-underfill recovery searches only missing kinds, preserving the other kind's selected candidates. Homophone fallback retains its original permission and combined result cap.
2. Desktop uses `FinderCatalog::has_local_normal_hit` instead of a complete local search whose result collection was discarded after an emptiness check. The probe and complete searches share candidate visitors; the probe stops at the first valid ordinary candidate. Final merged ranking remains unchanged.

No dependency, protocol, cache format, cross-query result cache, ranking policy or service-lifetime owner was added. User history and index settings were not changed.

## Reference and boundaries

Reverified inspectable SwiftList sources under `D:\Code\SwiftList-main\Core\IndexV2\Search\`:

- `SearchMatcher.cs::{BuildContext,MatchUniques}` prepares one query context, matches unique names and reuses bounded worker resources.
- `SearchMatcherRow.cs::{MatchRow,TryMatchNameOrAliases}` preserves row/delta matching and existing cancellation ownership.
- `NameSearch.cs::{SearchStreaming,CollectRanks,RefineWithWeight}` separates matching/fanout from bounded refinement and final result construction.

The fixed Listary managed reference remains `D:\Code\Listary.Decompiled\6.3.5.94`. Its ordinary provider/completion boundary informs submission and cancellation ownership; its native engine implementation is unavailable. Detailed entry chains and invariants are recorded in `repo:docs/reference-code/quick-launch-research.md`.

The original Kit0.8.0 InputState/InputGroup, list, selection and action owners remain. No visible variant, size, layout or parent adaptation changed. Primary popup activation, passive preview, placement, native show/hide, IME and deferred cross-entity callbacks are unchanged. Attached file-dialog search retains its own65ms/150ms cadence and native interaction; its shared service query semantics remain covered by scoped/preferred-directory equivalence tests. Its local caller was not changed to the new probe.

Diagnostics reuse the original correlated search start/first/terminal/cancellation lifecycle and bounded local-duration projection. No per-row/frame events, payload logs or production measurement hooks were added.

## Implementation

| Owner | Entry |
| --- | --- |
| Shared index matching and per-kind retention | `repo:rust/apps/nxtools-indexer/src/lib.rs#search_snapshots_query_while` |
| Bounded launcher delivery | `repo:rust/apps/nxtools-indexer/src/lib.rs#search_launcher_snapshots_while` |
| Shared ordinary candidate visitor | `repo:rust/crates/nxtools-finder/src/lib.rs#visit_search_candidates` |
| Declarative package visitor and public existence probe | `repo:rust/crates/nxtools-finder/src/extensions.rs#has_local_normal_hit` |
| Actual request permission decision | `repo:rust/apps/nxtools-desktop/src/quick_launch.rs#search_and_request_icons` |

## Measured synthetic work

| Fixture | Before | After | Meaning |
| --- | ---: | ---: | --- |
|64mixed-kind rows; calls at the existing current-query boundary |393|199|Same64results; repeated matching work removed |
|512synthetic shortcuts; bytes requested by complete local ranking versus existence probe |1,838,199|2,053|Allocation requests within public calls; setup excluded |

The allocation fixture retains built-in system entries. Requested allocation bytes are not retained memory, working set or private commit. Query-liveness checks are a bounded work observation, not CPU time. Neither fixture measures installed input-to-visible latency.

## Verification

Evidence: `artifacts/performance/quick-launch-search-work-20261117/verification.json` and adjacent UTF-8 logs. The manifest records commands, exit codes, selected test names, baseline failure-set comparison and raw fixture values.

| Check | Result |
| --- | --- |
| Indexer selected synthetic search contracts |38unique tests passed,0failed |
| Finder full unit/integration suite |98passed,0failed |
| Public local probe contracts |3passed,0failed |
| Quick Launch suite |213passed,0failed |
| Runtime subset in full Desktop run |14passed,0failed |
| Full Desktop unit suite, one final run |1959passed,21failed,1ignored;803.55s test execution |
| Desktop HTTP integration |10passed,0failed |
| Strict Clippy: Indexer, Finder, Desktop test targets |Passed |
| `cargo check --workspace --tests` |Passed |
|8changed Rust files: rustfmt; scoped diff whitespace |Passed |
| Spec navigation/evidence/contracts |Passed:130documents,14workspace owners |
| GPUI release and Markdown editor contracts |Passed |
| Indexer lifecycle source gate |Passed |
| Full Quick Launch lifecycle source gate |Existing OneCommander assertion fails |

`repo:rust/apps/nxtools-indexer/src/launcher_single_walk_tests.rs` exercises shared work, independent single-kind equivalence, dominant kinds, cold recovery, exclusions, delta replacement/delete, scopes, preferred-directory score, zero/small limits, cancellation before publication and the original combined homophone cap. Existing framed correlation/terminal, R8, pinyin, permission, recent failure and cancellation/backpressure retirement tests remain green.

`repo:rust/crates/nxtools-finder/tests/normal_hit.rs` checks literal/pinyin admission, misses, calculator, command preparation/limits, enabled/disabled/aliased package actions and list filtering, unchanged complete rank, and isolated child-process environment values. The allocator wrapper is confined to that integration-test executable and forwards unchanged to System. Desktop w2_ verifies the real typed request's fallback flag for custom commands, package/full-prefix hits, disabled declarations, misses and recent mode.

All Cargo commands were serialized with `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0` and `RUSTFLAGS=-C codegen-units=256`.

## Known issues and remaining acceptance

Full Desktop's21failed names exactly match the [ordinary-search baseline](2026-11-17-quick-launch-normal-search.md): Archive9, Downloads10, Main window2. The new-failure set and missing-baseline-failure set are both empty. Full assertions remain in `desktop-full.log`.

The Quick Launch lifecycle gate still expects OneCommander calls in the old adapter file and the earlier UIA API. Current `CurrentPathGet`/`CurrentPathSet` live in `OneCommanderUiaSession.cs` behind the independent UIA boundary. This task does not weaken that unrelated assertion; ordinary search's deferred/complete-provider source check runs before it and passes.

No packaging, deployment, protected-tuple replacement, real-volume scan, service restart or installed A/B was performed. Before release acceptance, the matching packaged LocalSystem service must pass `tests/performance/indexer-idle-cpu.ps1 -RestartService -AssertRegression`, persisted base+delta restart and the real service search/approved-runtime identity gates. Testing the unchanged installed binary cannot certify this source. Physical input, DPI and user acceptance remain separate.
