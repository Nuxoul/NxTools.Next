# Indexer quality and release boundaries

Evidence: `repo:rust/apps/nxtools-indexer/src/tests.rs`, `repo:rust/apps/nxtools-indexer/src/windows_service_host.rs`, `repo:rust/apps/nxtools-indexer/tests/process_lifecycle.rs`.

Synthetic scanner/poller/cache tests prove last-good publication, cursor/version and unique bounds. Admission tests precede shared state. Some suites scan real volumes; never unrestricted under scan/service prohibition. Native release requires authorized packaged LocalSystem idle/restart/base+delta/search/hash, not Ready/source gate. [Verification](../../shared/verification.md); retain admission contract below.
Idle release evidence now includes process I/O, not only CPU: tests/performance/indexer-idle-cpu.ps1 retains atomic protected tuple checks and explicit RestartService, then reports read/write/other; AssertRegression defaults read+write<64KiB/s in controlled idle. Process counters include file/IPC, not physical-disk attribution. Source regression must measure real temporary writer bytes after small updates to a multi-MBoverlay and prove mapped base+latestcomplete delta replay; see [durable delta contract](database-guidelines.md). Never run or certify a new-source service gate against the unchanged old installed binary.


## Scenario: Authenticated bounded session and truthful Ready

### 1. Scope / Trigger
- Changes to Windows service admission, initial Hello, snapshot state or recursive USN work budgeting.

### 2. Signatures
- `windows_service_host` obtains LocalPipeConnection actual client_process_id/client_session_id and calls authorized_packaged_desktop before shared state.
- `LocalPipeReader::set_initial_frame_timeout(Duration)`; service uses three seconds; MAX_PENDING_SESSIONS = 4.

### 3. Contracts
- PID/session and registered protected desktop path/hash authorize all service commands; Hello payload is not authority. Reject unknown/zero session/unapproved identities before shared mutex. Elevated broker keeps separate checks.
- Four authenticated pending slots; decode Hello before shared mutex. Initial read deadline is not extended by partial bytes; newline disarms it for healthy long-lived sessions.
- No snapshot after first failure means not Ready. Last-good snapshot remains Ready/searchable through failed maintenance. Unique USN candidates consume recursive budget, not duplicate raw entries.
- Unregistered development/Validation clients fail closed; do not weaken authorization to infer compatibility.

### 4. Validation & Error Matrix
- Unidentified/unapproved/session zero -> reject, no lease; capacity exhausted -> bounded reject.
- Silent/partial first frame -> TimedOut and released admission; non-Hello -> reject before state/ack.
- First build failure -> no false Ready; maintenance failure -> preserve last-good search. Genuine large subtree -> bounded RebuildRequired maintenance, not payload leakage.

### 5. Good/Base/Bad Cases
- Good: approved client Hello -> session; later idle reads remain alive.
- Base: duplicate-heavy direct changes remain incremental.
- Bad: trust client_name, lock state before Hello, reset timeout per byte, or publish Ready with no snapshot.

### 6. Tests Required
- Admission denial/cap release; preadmitted non-Hello; first-failure/last-good and duplicate-heavy/large-subtree regressions.
- Temporary named pipe partial deadline and healthy-session tests; no unrestricted real-volume suite when scanning is prohibited.
- Release gate remains packaged LocalSystem idle CPU with RestartService/AssertRegression, persisted base+delta restart, real service search and approved/runtime hash consistency. Never infer release acceptance from synthetic tests.

### 7. Wrong vs Correct
```text
Wrong: pipe ACL -> service mutex -> read unbounded Hello -> trust name.
Correct: actual PID/session/path/hash -> bounded admission -> deadline Hello -> service mutex.
```

Reference: SwiftList UsnServicePipeServer and Listary managed boundary only; Listary native source unavailable. Evidence: `docs/audits/2026-10-08-sol-review-remediation.md`.

## Scenario: Bounded runtime unique-name and selective candidates
### 1. Scope / Trigger
Runtime name grouping/3gram acceleration; no persisted MFT/base/delta format change.
### 2. Signatures
`BaseSearchData` immutable Arc auxiliary groups/postings; Finder prepared name materials built on catalog construction/replacement. Query narrowing requires a single positive ASCII contiguous exact/word/prefix/suffix term.
### 3. Contracts
Exact-name group score fans out to original ascending row IDs before unchanged headroom/cold filters/rank; independent delta and underfill full scan remain. Unsupported fuzzy/short/CJK/OR/exclusion/path/compound/wide queries fully fall back. Unclosed quote absorbs `|`, `!`, whitespace as literal text: do not reinterpret parser; conservatively reject narrowing. Include every non-ASCII name group to preserve Unicode folding (Kelvin sign).32MiB conservative group budget/1Mrows;2Mpostings/65536terms; candidates<=32768 and<=quarter unique groups; budgets fall back rather than omit results. Reuse pooled worker buffers and same score/snapshot/row cutoff; no per-group Vec/sort. Scale Rayon group leaf size to roughly8192represented rows.
### 4. Validation & Error Matrix
Overflow/budget/unsupported query→complete legacy scan; cancel→discard abandoned result; stale delta suppression/per-row filters unchanged. Timing regression→measure actual caller, not infer improvement from fewer comparisons.
### 5. Good/Base/Bad Cases
Good: eligible exact query narrows verified safe superset plus delta. Base: broad/fuzzy complete group scoring. Bad: ASCII postings omit Unicode names; unclosed quote treated as OR; group units retain row chunk8192 and serialize duplicate-heavy work.
### 6. Tests Required
Finder r8_3, Indexer r8_5 incl frozen row oracle/interleaved fanout/ties/cold-underfill/cancel/base+delta/Kelvin/caps, mapped constructor and existing headroom regressions. Debug synthetic20krows/2knames9samples passed; build28.424ms,payload365824bytes is not resident memory or Release/live service speed. Production Release latency/map-open CPU/memory and packaged LocalSystem gates require authorized follow-up.
### 7. Wrong vs Correct
Wrong: Python model speedup proves product faster; grouped matches change row tie policy. Correct: production caller frozen-result comparison plus measured allocation/time tradeoff; unchanged row headroom comparator/fallback.
Evidence: `repo:rust/apps/nxtools-indexer/src/search_data.rs`, `repo:rust/apps/nxtools-indexer/src/tests.rs`, `repo:rust/crates/nxtools-finder/src/query.rs`. Steward and SwiftList inspectable source informed ownership only; Listary native source unavailable.

## Scenario: Explicit search extensions and true recent metadata
### 1. Scope / Trigger
Approved W2–W5 search extensions; WinShun source reference is recorded in docs/reference-code/winshun.md. Listary native engine remains unavailable.
### 2. Signatures
IndexSearchContext default-off retain_kinds_apart/allow_homophone_fallback/recent_modified; search_snapshots_while/run_snapshot_search; recent::search_recent_modified returns typed classifier-budget failure. Evidence: `repo:rust/apps/nxtools-indexer/src/lib.rs`, `repo:rust/apps/nxtools-indexer/src/recent.rs`, `repo:rust/apps/nxtools-indexer/src/mft/time_metadata.rs`.
### 3. Contracts
Ordinary results precede <=8 bounded homophone variants; caller permission is required. Separate kinds retain <=2limit candidates without a new final rank. Recent mode requires explicit opt-in AND prefix at snapshot/runtime entries; default/file-dialog queries retain ordinary semantics. Effective base+delta true last-write only; view-only noise/diversity, <=200 results, folder2/project3, classifier roots4096/string1MiB/path32768/depth128/extra overflow work1M. Budget failure is correlated terminal SearchFailed, not empty success or maintenance fault. MFT uses full-ID FileBasicInfo true last-write; USN reads<=256uniqueIDs/poll, retains deferred cursor, permits backwards times and unknown failure. Old unchanged caches need a legitimate rebuild; no query-time IO/reconnect backfill.
### 4. Validation & Error Matrix
Normal hit or no permission -> no approximate phase; default recent prefix -> ordinary terminal batch; stale/cancel -> discard; classifier budget -> fixed current-generation failure; timestamp denied -> unknown; native-read cap -> deferred cursor retained.
### 5. Good/Base/Bad Cases
Good: local ordinary hit disables indexed approximation while normal files still search. Base: mapped/delta reload retains times. Bad: prefix hijacks file-dialog caller or journal event time fabricates last-write.
### 6. Tests Required
W1/W2/W3/W4/R8 synthetic filters, real mapped base/delta reopen, USN structure/cursor/budget and actual Desktop request/action regressions. Final28isolatedIndexer contracts passed; no real scans. Initial metadata cost, native cancellation/NTFS races and packaged LocalSystem idle/restart/search/hash remain separate authorized gates. Full contracts/evidence: `.trellis/tasks/10-14-winshun-w2-w5/code-spec.md` and outcomes.md.
### 7. Wrong vs Correct
Wrong: empty indexed rows authorize approximation or raw prefix selects every caller. Correct: local ordinary decision -> typed permission -> indexed normal phase; explicit recent mode plus prefix with truthful correlated terminal.
