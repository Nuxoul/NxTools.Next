# Finder usage storage

Evidence: `repo:rust/crates/nxtools-finder/src/lib.rs#FinderCatalog`, `repo:rust/crates/nxtools-finder/src/lib.rs#record_activation`, `repo:rust/crates/nxtools-finder/src/lib.rs#clear_history`.

rusqlite connection owned by catalog; tables quick_launch_usage, quick_launch_query_usage and quick_launch_directory_history store activation/query/directory ranking metadata, not raw file index. Bind values with params and preserve entry/query keys and Windows path normalization. Clear history commits transaction before clearing caches; writes update in-memory projection only after success. No imagined numbered schema/WAL guarantee: current tables initialized idempotently in open. Last-good refresh retains catalog on scan errors via apply_refresh_snapshot. Recent directory count bounded64. Test durable reopen/clear/ranking with temp DB; never user history. [Engineering persistence](../../shared/engineering.md).

## Scenario: Shared path history with keyword recency
### 1. Scope / Trigger
Application/indexed results share a path or an existing query is shortened.
Evidence: `repo:rust/crates/nxtools-finder/src/ranking.rs#canonical_history_id`,
`repo:rust/crates/nxtools-finder/src/lib.rs#load_history`, `repo:rust/crates/nxtools-finder/src/lib.rs#record_activation`.
### 2. Signatures
`canonical_history_id(&str) -> String`; original `FinderCatalog::record_activation(id,query,time)`;
unchanged `quick_launch_usage` and `quick_launch_query_usage` tables and public result IDs.
### 3. Contracts
History keys beginning app:/indexed: normalize separators and Unicode case to one app:path key.
Load sums counts and takes latest timestamps across existing aliases without rewriting rows. Writes
increment only the canonical persisted row and the aggregated in-memory value once after commit;
never persist an aggregate count over one alias, which would count old rows twice on reopen.
Commands/extensions retain their distinct IDs. Exact keyword history outranks prefix history;
within either category newest selection wins before counts. Prefix lookup scans existing associations
once per merged query, not per candidate or comparison. Clear uses the original transaction/caches.
### 4. Validation & Error Matrix
Old app/indexed mixed-case/slash aliases -> shared count/time; new activation plus reopen -> exactly
one additional count; failed transaction -> no cache mutation; unrelated keyword -> general history
only; shorter keyword -> prefix association; malformed SQL rows -> existing error propagation.
### 5. Good/Base/Bad Cases
Good: selecting an indexed shortcut teaches its application row. Base: command identity unchanged.
Bad: frequency beats a newer exact-query choice, or clearing user data to unify source keys.
### 6. Tests Required
`repo:rust/crates/nxtools-finder/tests/listary_ranking.rs` covers exact/prefix/reopen, legacy SQLite
aliases, cross-source dedup and clear. `repo:rust/crates/nxtools-finder/tests/listary_merge.rs` covers
prefix learning on grouped shortcuts when enabled packages participate.
### 7. Wrong vs Correct
Wrong: overwrite canonical SQL count with sum(all aliases)+1. Correct: canonical SQL count+=1;
in-memory aggregate+=1; reopen folds persisted rows once.
