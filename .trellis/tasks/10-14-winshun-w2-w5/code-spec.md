# W2–W5 executable search contract

## 1. Scope / Trigger
Approved canonical order: W2 no-result homophone fallback; W3 separate file/folder candidate retention; W4 useful recent-modified list plus approved accurate MFT/USN timestamp ingestion; W5 fast-input execution guard. W6–W10, plugin runtime, packaging and native service operations are excluded.
Reference: read-only WinShun Query.cpp/SearchEngine.cpp/Launcher.cpp, NameSearch.cpp, Ntfs.cpp and NtfsIndexer.cpp, indexed in docs/reference-code/winshun.md. Existing Listary/SwiftList Quick Launch interaction remains authoritative; Listary native engine source was unavailable.

## 2. Signatures
- Finder: `FileQuery::homophone_variants`, `homophone_match_ranges`, `FinderCatalog::search_with_sources_normal` and `search_with_sources` share parsed matching/ranking.
- Protocol: `IndexSearchContext { retain_kinds_apart, allow_homophone_fallback, recent_modified, .. }`; all three additions deserialize missing fields as false and omit false fields on serialization.
- Indexer: `search_snapshots_while` / `search_snapshots_retained_while`; `recent_modified_query` requires explicit mode plus `recent:` prefix. `recent::search_recent_modified` returns `Result<Option<Vec<IndexSearchItem>>, ClassifierBudgetExceeded>`; runtime `run_snapshot_search` carries correlated terminal `SearchFailed`.
- Timestamp ingestion: `hydrate_record_times_while`, `read_last_write_by_id`, `hydrate_journal_times_while`, `metadata_batch_prefix`; existing VERSION3 base/delta persists optional Unix milliseconds.
- Desktop: `search_and_request_icons`, `IndexSearchSession::begin`, `QuickLaunchPanel::activate_selected` / `activate_selected_action`, existing file-dialog request/batch owner.

## 3. Contracts
W2 ordinary local applications/commands finish before indexed approximation permission is decided. Local normal hits disable indexed fallback, not ordinary indexed search. Empty indexed normal results alone are insufficient permission. <=8 prepared variants, <=32 query characters/63 ASCII spelling bytes; only ordinary positive fuzzy Han name terms change. Explicit, excluded, case-sensitive, path, parent and drive constraints survive. Normal merged results win unchanged; approximate subtitle and highlights are explicit/shared. Existing generation/cancellation owns work.

W3 launcher opts in to existing disjoint file/directory scans, bounded candidate union <=2*request limit; final Finder score/rank remains unchanged. No folder bonus/forced visible quota. File dialogs remain default-off. Extra scans are a correctness/cost tradeoff, not a speedup claim.

W4 only explicitly opted-in launcher `recent:` requests enter recent mode. Ordinary/default/file-dialog requests retain query syntax and terminal ordinary batches. Effective mapped base+delta, valid positive nonfuture true last-write, files-only/scope/exclusions; view-only narrow noise filters and folder2/project3 diversity before global retention<=200. No query filesystem reads or global newest window starving older projects. Marker roots<=4096/string payload<=1MiB, path<=32768/depth<=128, extra overflow classification work<=1M; budget exhaustion is truthful correlated terminal failure, not empty success/maintenance fault.

MFT metadata uses full sequence-bearing OpenFileById identity and FileBasicInfo.LastWriteTime, never journal event/current time. Failure is unknown. USN data/basic-info/close metadata reads deduplicate<=256 unique IDs per poll; first deferred record remains the cursor. Metadata-only records do not resurrect or move rows. Backwards true time is allowed; failed read clears stale time. Unchanged legacy caches are not backfilled on reconnect; a legitimate rebuild is needed. Initial per-row metadata cost and production scan cancellation remain unmeasured/limited.

W5 pending or closed panels reject activation; no automatic held execution. Deferred primary activation rechecks open/generation/input/selected target. Action-mode activation also rechecks mode ID, menu-level ID, parent query, target and selected typed action including Shell session/command. Input changes/close-reopen/action replacement cannot dispatch stale work. Existing InputState, IME gate, selection, reused HWND/preview/focus and native success-only close owners remain intact. Cross-owner work remains deferred to the next GPUI turn.

Diagnostics carry request correlation, generation, mode/options, count, fixed failure reason and elapsed time; no query/name/path/native payload logging or per-file/idle spam. Native errors remain at their owning boundary.

## 4. Validation & Error Matrix
| Condition | Required outcome |
| --- | --- |
| Local or indexed normal hit | Original normal result set; no indexed approximate phase after local hit |
| Genuine empty ordinary set, permitted bounded variants | Labeled approximate matches or terminal empty result |
| Default context + `recent:` | Ordinary search, never classifier mode |
| Recent classifier budget exhausted | Matching generation SearchFailed, launcher clears busy/old execution and offers retry |
| Superseded query/cancel | No stale publication or action; quiet cancellation |
| Native timestamp inaccessible/deleted | Optional timestamp unknown, no fabricated recency |
| USN read budget reached | Retain deferred cursor, no skipped records |
| Deferred Shell mode/input/level/action changed | No InvokeShellAction and no pending action entry |
| Current unchanged action | Original typed command dispatched once |

## 5. Good / Base / Bad Cases
Good: 年度报高 finds 年度报告 only after all ordinary sources are empty; a local 年度报高 command prevents indexed approximation. `recent: report` lists cached accurate times; file-dialog `recent:` finishes ordinary search. Further action-filter input prevents old Shell command dispatch.
Base: ordinary literal/exact/excluded/OR/path searches and final rank stay unchanged; directory history remains separate. Snapshot restart preserves timestamps and delta visibility.
Bad: eight homophone variants scanned despite local normal hit; raw prefix hijacks every caller; cached unknown time replaced by USN event time; deferred action executes after close/reopen.

## 6. Tests Required / Evidence
2026-10-14 final: Desktop684/684, Finder80/80 across7 suites, Protocol44/44. Isolated Indexer28/28: W1=1,W2=1,W3=2,W4=14,R8=5,mapped base/delta reopen=2,USN structure/cursor/duplicate budget=3. Strict scoped Clippy `--tests -- -D warnings`, workspace `--tests` compilation, fmt check and four affected architecture gates passed.
Real red/green at deferred action entry, indexed fallback permission, launcher request coordination and default recent routing. File-dialog actual batch owner retires pending only for current generation. One cache fixture initially failed after mode defaults changed; explicit recent opt-in corrected the fixture without altering persistence or weakening assertions, then both reopen tests passed.
No unrestricted Indexer suite/real scan/service mutation/package/install/commit. Native ingestion cost, LocalSystem idle/restart/base+delta/search/hash, installed physical input/IME/DPI/visual acceptance remain separately authorized gates. No new end-to-end benchmark claim.

## 7. Wrong vs Correct
```text
Wrong: queue index search -> local result -> discard expensive indexed homophones.
Correct: finish local ordinary sources -> explicit approximation permission -> normal index scan -> permitted fallback only if empty.

Wrong: starts_with("recent:") means recent mode for every caller.
Correct: typed default-off recent mode AND prefix; file dialog stays ordinary and consumes a terminal batch.

Wrong: capture Shell action -> defer -> execute without current ownership.
Correct: capture -> defer -> check current panel/query/mode/level/target/action -> existing execution owner.
```
