# Download engine responsibility outcomes

## Scope and result

Completed the behavior-preserving registration/removal of the already prepared private browser, queue and projection implementations. `DownloadEngine` remains the same public API and shared `Arc<DownloadEngineInner>` owner; no manager, trait, forwarding layer, dependency, state/schema or diagnostic event was added. The original inline tests remain in the same module, preserving their fully qualified names; only their private imports changed.

### Source owners

| Source | Responsibility and entry evidence |
| --- | --- |
| `rust/crates/nxtools-download/src/engine.rs:42–74` | Unchanged shared engine, store, task map, scheduler gate, limiter and generation ownership. |
| `rust/crates/nxtools-download/src/engine.rs:140–153` | Private module registration; same public `daily_schedule_window_contains` re-export; narrow internal request/projection imports. |
| `rust/crates/nxtools-download/src/engine/browser.rs:22–721` | Browser handoff replay/admission, bounded confirmation channel, Blob import/discard and persistence compensation. Restored the original public receiver API documentation at lines32–34. |
| `rust/crates/nxtools-download/src/engine/queue.rs:22–756` | Queue order, admission, daily/delayed schedules and generation-scoped wakes. `process_daily_schedules_at_correlated` stays module-private at line229; cross-module methods/helpers use only necessary `pub(super)`. |
| `rust/crates/nxtools-download/src/engine/projection.rs:16–368` | Task/summary conversion, full/compact snapshots, selected-task detail, snapshot projection and one private search predicate. Prepared implementation adopted unchanged. |
| `rust/crates/nxtools-download/src/engine.rs:3163–4994` | Existing inline test module and subsequent existing root tests; same bodies/names, with browser/queue/chrono/model imports explicitly private to tests. |

Removed the duplicate originals of 42 functions, `BrowserBlobImportRequest`, `impl DownloadTaskSummary`, `impl DownloadScheduleMoment` and `BrowserBlobPersistenceError`. Removal was based on live reviewed source, not the prepared helper's ranges alone. In particular, original lines386–430 contained the whole summary impl and its trailing blank; the next schedule type's derive at original line431 was preserved. The prepared range list omitted the original Blob persistence-error enum at lines4684–4691; that duplicate was also removed.

## Reference and unchanged invariants

Reviewed the owning PRD/design, download backend specification topics and `docs/reference-code/download.md` before edits. External references inspected in the required order:

1. `D:\Code\ab-download-manager-master\downloader\core\src\commonMain\kotlin\ir\amirab\downloader\queue\DownloadQueue.kt:20–180`: persisted queue identity, boot-once listener/save/schedule lifecycle, bounded active-item admission and refill.
2. `D:\Code\NxTools\Services\DownloadManagerService.cs:344–390`: synchronized pending dequeue and active-count ownership, worker completion refill. Existing reference-index entries were sufficient; no new call chain or behavior was introduced.

Preserved task/request/handoff identities, request-fingerprint replay, scheduler/task locks, SQLite-before-memory-before-event ordering, Blob artifact publication and typed compensation, browser confirmation timeout and4096-entry suppression bound, queue ordering/concurrency, user-pause eligibility, daily start-inclusive/stop-exclusive and cross-midnight rules, delayed/daily wake generations, projection status/order/limits and final-path search, manual/runtime/per-task speed-limit ownership, errors and diagnostic names. No layout, focus, placement, Windows boundary or cancellation policy was redesigned.

The only functional-code simplification is replacing two identical search expressions with `matches_task_search`: lowercase source URL OR optional lowercase filename OR optional lowercase category OR lossy lowercase final path. Full projection still searches the request filename and projected final path; compact projection still searches its derived summary filename and stored final path. `as_ref` versus `as_deref` supplies the same borrowed path, optional absent values still fail that OR term, and the surrounding trim/lowercase normalization, status filters, sorting and truncation are unchanged.

## Function-preservation evidence

Source-only checks compared against the parent-captured pre-refactor `download-engine-before.rs`, not Git HEAD (which would ignore accumulated dirty work).

- 42 relocated functions: browser11, queue24, projection7.
- 40 function bodies exactly identical after removing only `pub(super)` from signatures.
- Both changed snapshot functions exactly match their original bodies after expanding the shared search-helper call into the original expression and normalizing whitespace.
- All147 retained function bodies unchanged; includes54 inline test/helper functions.
- All41 existing engine-file test names, in order, preserved. The entire test section is byte-identical after removing import statements only.
- Public function signatures unchanged; six shared structs (`DownloadEngine`, `DownloadEngineInner`, `EngineTaskState`, `DownloadEngineTask`, `DownloadScheduleMoment`, `GlobalSpeedLimits`) byte-identical.
- Both relocated Blob types identical except private visibility; no duplicate originals remain; all three private modules registered; no new lint suppression.

Full per-function original/destination line ranges and SHA-256 hashes, final file hashes and preserved test-name list are recorded in session evidence:
`C:\Users\Administrator\.pi-desktop\scratch\f32f826f-f318-49e4-a4bf-509b7d001dcd\download-relocation-verified.json`.

Representative original function SHA-256 hashes (exact relocated body comparison passed):

| Function | SHA-256 |
| --- | --- |
| `submit_browser_correlated` | `d0ff8ce0d2338203863859f0d0e0968403c2335660087ded4e1c7915c39b22fd` |
| `import_browser_blob_correlated` | `cb0a502c59db46cf3480d38f6464d2b03276e95094dd0bdaa30cdcba2af73130` |
| `persist_browser_blob_with_compensation` | `f6b0ff93fe63110b9ca1a19587e0d10b8ad23d011f56607393a5644c4cfa4a88` |
| `schedule_queue_locked` | `f134a107fa2cb2e2f7b18b9ffdc8f117b810cff827d7fd27e5af6b4c3404998d` |
| `refresh_delayed_schedule_wake` | `8276fbf6de2e917c0de243d966b208fd516ca6b57b0cddb0dd8ca22b09c19e24` |
| `snapshot_from_state` | `79ce65f66b4c94767764976989a44a2c04a596613f85a8c675a66be444ed98d3` |

Final source SHA-256 hashes before parent-coordinated formatting:

| Source | SHA-256 |
| --- | --- |
| `engine.rs` | `ec8363579721e6df17ab7a118f51467c783724359ba0ffb39a4e17873ba2f958` |
| `engine/browser.rs` | `bac293365654ff77b8d09c6be01ddefd1c3fe2236892028cf46bb40133168448` |
| `engine/queue.rs` | `4a5a533ccb3443913ee37ff898f323f70397ef2e95527dcb1525cde80ff14e83` |
| `engine/projection.rs` | `72f7c52f96d646ef0fd90a34aeafd66a3a1845ece5d1b38d832a364927693b1a` |

## Actual validation and parent follow-up

Passed deterministic Python source comparisons described above and scoped `git diff --check`. Passed a source assertion that all six existing Browser Native Messaging engine tokens occur in the actual `engine/browser.rs` owner. These are source evidence only, not compiled or interactive acceptance.

**Location-dependent gate requiring parent update:** `tests/architecture/browser-native-messaging.ps1:152–165` reads only `engine.rs`; read `engine/browser.rs` for its browser assertions (or compose both files). Three tokens moved out of that old input: `import_blob_artifact`, `download.browser_blob.discard_failed`, `download.browser_blob.rollback_failed`. All six assertions remain valid and must not be removed. This gate is outside this subagent's write scope and was not edited or represented as passing.

Cargo, rustfmt, builds and real operations were explicitly prohibited for this subtask. Parent must coordinate formatting, strict scoped lint/check, `cargo test -p nxtools-download --lib`, relevant engine/browser/queue/runtime/storage contracts (engine_contract serial), the corrected Browser Native Messaging architecture gate and final workspace/source-contract checks. Inline regression locations and fully qualified test names did not change. No packaging, service, browser, clipboard, Shell or real download operation was performed.
