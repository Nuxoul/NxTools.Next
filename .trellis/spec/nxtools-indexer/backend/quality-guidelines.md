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
- PID/session and registered protected desktop path/hash authorize all service commands; Hello payload is not authority. Reject unknown/zero session/unapproved identities before runtime registration. Elevated broker keeps separate checks.
- Four authenticated pending slots; decode Hello before coordinator registration. Initial read deadline is not extended by partial bytes; newline disarms it for healthy long-lived sessions.
- No snapshot after first failure means not Ready. Last-good snapshot remains Ready/searchable through failed maintenance. Unique USN candidates consume recursive budget, not duplicate raw entries.
- Unregistered development/Validation clients fail closed; do not weaken authorization to infer compatibility.

### 4. Validation & Error Matrix
- Unidentified/unapproved/session zero -> reject, no lease; capacity exhausted -> bounded reject.
- Silent/partial first frame -> TimedOut and released admission; non-Hello -> reject before state/ack.
- First build failure -> no false Ready; maintenance failure -> preserve last-good search. Genuine large subtree -> bounded RebuildRequired maintenance, not payload leakage.

### 5. Good/Base/Bad Cases
- Good: approved client Hello -> session; later idle reads remain alive.
- Base: duplicate-heavy direct changes remain incremental.
- Bad: trust client_name, register state before Hello, reset timeout per byte, or publish Ready with no snapshot.

### 6. Tests Required
- Admission denial/cap release; preadmitted non-Hello; first-failure/last-good and duplicate-heavy/large-subtree regressions.
- Temporary named pipe partial deadline and healthy-session tests; no unrestricted real-volume suite when scanning is prohibited.
- Release gate remains packaged LocalSystem idle CPU with RestartService/AssertRegression, persisted base+delta restart, real service search and approved/runtime hash consistency. Never infer release acceptance from synthetic tests.

### 7. Wrong vs Correct
```text
Wrong: pipe ACL -> service mutex -> read unbounded Hello -> trust name.
Correct: actual PID/session/path/hash -> bounded admission -> deadline Hello -> service-issued runtime lease.
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

## Scenario: Journal correctness and service-lifetime maintenance (2026-11-01)
### 1. Scope / Trigger
USN deletion/replay, base publication/retirement, authenticated reconnect, per-volume maintenance and overlay-copy costs. Reference: Listary managed session/status boundary only and SwiftList journal/enumeration/delta/maintenance source in `docs/reference-code/quick-launch-research.md`; Listary native engine source is unavailable.
### 2. Signatures
`enumerate_initial_snapshot(mount,journal_id,start_usn,enumerate)` calls `enumerate(i64::MAX)` and keeps the saved pre-scan cursor. `apply_journal_batch` returns `JournalApplyError::RebuildRequired(JournalRebuild { reason: IndexRebuildReason, native_error_code: Option<u32> })`. `SharedIndexerRuntime` runs the original `run_coordinator` once per service lifetime. `IndexerService::{queue_snapshot_maintenance,start_snapshot_maintenance,finish_snapshot_maintenance,project_maintenance}` own per-volume operations and projection.
### 3. Contracts
- Direct leaf/invisible deletes consume no recursive expansion units; directory candidates exclude tombstones and moved-away/invisible rows before unique accounting. A truly large visible subtree still exceeds the4096bound transactionally.
- Unfiltered upper-USN enumeration includes preexisting files and directories modified during scanning. Replay starts at the pre-scan journal identity/cursor; repeated create/upsert is idempotent. Unknown metadata never broadly inserts/resurrects rows or adds native metadata reads.
- Build/compaction work and immutable search snapshots survive endpoint replacement/disconnect. Requests, reader terminals and search completions carry a service-issued lease; replacement resets search generation once. Stale endpoints cannot change or receive current work; the current authenticated endpoint supplies the native command handler.
- Per-volume maintenance is accepted/started/completed once per operation with the same correlation ID, fixed state/reason/outcome, elapsed time and owning native error code. Queue coalescing does not create duplicate operations. Initial build uses snapshot presence, not nonzero count; a valid empty snapshot is Ready. Rebuild retains last-good; genuine worker/poll/persistence/compaction faults remain errors.
- Scan base persistence and exact-file mapped reopen must succeed before build publication; either failure ends maintenance as failed and retains last-good. Native scan/open errors keep an io::Error source. Initial status is projected into the existing handshake status frame; pending next work is marked before a terminal status is sent.
- `MftOverlay.replacements` shares immutable `Arc<MftEntry>` rows; actual mutation uses copy-on-write. Maps/deletion sets remain transactional, owned single-row lookup remains owned, content equality handles distinct Arcs. Formats, durable checkpoint authority and ranking are unchanged. Completed rebuild/compaction base residency follows the mapped-publication contract below.
### 4. Validation & Error Matrix
Identity/range mismatch -> CursorInvalid; bounded visible expansion -> BulkChanges; native1181 -> JournalWrapped with native code; explicit request -> Requested. Recoverable rebuild -> maintenance only, no last_error. Failed initial build -> neither busy nor Ready; failed rebuild -> last-good Ready plus real error. Nonreading/full endpoint -> retire endpoint only; scan/compaction completion still publishes. Corrupt legacy counts -> reject before row allocation and recover base.
### 5. Good / Base / Bad Cases
Good:5000direct leaf deletes remain incremental; modified directory retains unchanged descendants; B receives HelloAck/status within500ms while A's scan/output are held. Base: unchanged native cursor returns before clone; cursor advancement remains separately measured. Bad: count every delete, filter enumeration at the saved cursor, use another session coordinator, or publish success after swallowed base-write failure.
### 6. Tests Required
`h2_`, `h3_`, `m3_`, `l11_`, `m4_`, `u4_` and `m8_snapshot_clone` at production apply/enumeration/cache/admitted-runtime/Settings boundaries. Retain all `mft/cache/journal/tests.rs` fault/identity/replay cases. Default-ignored million-row apply/classify and owned/mapped memory measurements exclude setup and distinguish allocator bytes, mapping size, working set and private commit. No performance inference from test count or idle-copy suspicion.
### 7. Wrong vs Correct
Wrong: session mutex owns scan lifetime; new desktop waits for old writer; successful scan implies durable publication. Correct: one service coordinator owns work, bounded lease-scoped endpoint I/O owns delivery, durable base success precedes memory replacement, and maintenance is an additive status projection. Synthetic/TestApp/temporary pipes do not certify packaged LocalSystem release or physical/user visual acceptance.

## Scenario: Launcher first-provider publication before the second kind
### 1. Scope / Trigger
Ordinary launcher search with retain_kinds_apart, both file/directory kinds and no directory scope.
### 2. Signatures
`search_launcher_snapshots_while` reuses production `search_snapshots_query_while` and emits through
original `RuntimeEvent::SearchCompleted`/framed `IndexSearchBatch`; `run_snapshot_search` owns routing.
### 3. Contracts
Send completed file-kind candidates before starting directory-kind work; append directory candidates
and exactly one terminal. Preserve original per-kind limits/scores and Finder's final merge/rank.
Only no normal results plus explicit permission enters bounded homophone fallback. Single-kind,
scoped and recent searches retain their original route. Every send checks original current task;
coordinator lease checks and bounded output backpressure still own actual endpoint delivery.
First nonempty/terminal diagnostics carry correlation/generation/elapsed/count only, never query/path.
### 4. Validation & Error Matrix
Superseded/cancelled after first batch -> no later candidates/terminal for that task. Empty results ->
one truthful empty terminal. Stale lease -> no endpoint delivery. Full sink -> original bounded wait.
### 5. Good/Base/Bad Cases
Good: files cross original framed transport while directory query remains outstanding. Base: final
candidate set/scores match the complete provider. Bad: partition only after all query work, or treat
an injected ready Desktop batch as measurement of production query speed.
### 6. Tests Required
`repo:rust/apps/nxtools-indexer/src/tests.rs` launcher_* uses actual snapshot query and normal worker,
framed correlation, final candidate comparison, cancellation and empty/homophone fallback. Desktop
arrival/paint/selection tests are a distinct consumer seam. Installed input-to-paint and packaged
LocalSystem acceptance remain separate; never run real scans/service changes merely for this fixture.
### 7. Wrong vs Correct
Wrong: faster batch projection proves the provider no longer waits. Correct: real production worker
emits a frame before provider completion; compare complete results and separately measure installed UI.
Reference: `repo:docs/reference-code/quick-launch-listary-results.md` preserves managed-only Listary
evidence; measured boundary and limits: `repo:docs/quick-launch-result-latency.md`.

## Scenario: Desktop-owned demand service lifetime
### 1. Scope / Trigger
User2026-11-06 rejects file tracking after product Exit. Supersedes only earlier resident-service
policy; native privilege/identity, close-to-tray and persisted snapshot recovery remain unchanged.
### 2. Signatures
`DesktopLifetime::{admit,poll,clear}` retains SYNCHRONIZE process handles; `start_admitted_runtime`
serializes first discovery/start in an optional shared runtime slot. `SharedIndexerRuntime::shutdown`
sets original coordinator stop and joins it before Windows SCM reports Stopped.
Evidence: `repo:rust/apps/nxtools-indexer/src/windows_service_host.rs`,
`repo:rust/apps/nxtools-indexer/src/desktop_lifetime.rs`,
`repo:rust/apps/nxtools-indexer/src/session_runtime.rs`.
### 3. Contracts
Only exact native desktop PID/session/approval plus bounded Hello admits a process lease. Pipe
disconnect/reconnect does not release a live desktop; duplicate Hello creates no extra owner.
An unclaimed service start expires after15s without discovery/scan/USN work. First approved owner
starts original cache restore and one coordinator; stop before/during discovery prevents later spawn.
Last desktop process exit signals the same stop boundary as SCM Stop/Shutdown, wakes accept,
retires native endpoint I/O and joins coordinator/sessions before Stopped. Multiple live owners
remain independent. C# registers Demand and starts only after original complete-tuple validation.
Original mmap base/delta and persisted USN cursor are restored; no rebuild merely for reopening.
Lifecycle diagnostics use fixed reasons/correlation/elapsed/native error, not per-poll or paths.
### 4. Validation & Error Matrix
Unauthorized/invalid/no Hello -> no owner/work; deadline -> normal stop. Live owner/pipe lost ->
original reconnect. Last process signals exit -> stop; another live owner -> retain. Configuration
migration failure/cancel -> original actionable result, no data reset or unowned process termination.
### 5. Good/Base/Bad Cases
Good: desktop Exit ends the process lease; next startup resumes persisted changes. Base: tray Hide
retains the same process. Bad: forever service, pipe EOF equated with exit, or mandatory full rescan.
### 6. Tests Required
`repo:rust/apps/nxtools-indexer/src/desktop_lifetime_tests.rs` uses actual owned process handles.
Windows service admission tests cover lazy initialization, concurrency and stop-during-discovery;
session_runtime tests prove actual native reader retirement/coordinator join. Temp cache reopen
tests preserve delete/restore/cursor replay. Release still requires approved packaged LocalSystem
restart/base+delta/idle CPU+I/O/search and final desktop-exit observation. Unapproved PowerShell
Hello probes cannot replace an authenticated product Desktop for this gate.
### 7. Wrong vs Correct
Wrong: independent registration means a process must remain alive after Exit. Correct: Demand
registration persists, while actual work and process lifetime belong to authenticated live desktops.
Reference: [shell](../../../../docs/reference-code/system-shell.md).

## Scenario: Request-scoped interactive search cancellation
### 1. Scope / Trigger
Launcher clear/hide or retirement without a replacement Indexer Search on the persistent pipe.
### 2. Signatures
`IndexerCommand::CancelSearch(IndexSearchCancel { request_id, generation })`;
`SearchExecutor::{submit,cancel,retire}` inside original `run_coordinator`.
Evidence: `repo:rust/apps/nxtools-indexer/src/lib.rs`,
`repo:rust/apps/nxtools-indexer/src/search_retirement_tests.rs`.
### 3. Contracts
Existing endpoint lease check precedes command handling. Cancellation must match that lease and
the original Search envelope ID AND generation; it never advances the accepted search generation.
Under the original worker-state gate, a match invalidates the atomic active task and drops pending
work/current identity. Active computation and backpressured completion observe the same task token;
its actual return releases the job-owned snapshots, request and result buffers. Service-owned
immutable snapshots and bounded reusable candidate pools remain available for subsequent queries.
Ordinary Search supersession keeps one original worker and one latest pending job. Attach/current
disconnect/refresh retire pending ownership as well as the task token; old endpoint events cannot
retire a newer lease. Index building, USN, cache persistence and maintenance are independent.
### 4. Validation & Error Matrix
Wrong lease/request/generation or repeated cancellation -> no effect on current work. Matching
pending -> immediate release without execution; matching active -> cooperative exit, no current
result delivery. Full completion queue -> cancelled worker exits without waiting for output drain;
the same worker can serve a new query. Cancellation is quiet, not empty success or search failure.
### 5. Good/Base/Bad Cases
Good: clear retires its provider while the last-good index remains searchable. Base: new Search
supersedes the old one. Bad: only reject UI batches, cancel whichever job is current, or retain a
cancelled pending snapshot until another query arrives.
### 6. Tests Required
search_retirement_ sends the cancellation wire contract through the original frame reader and
coordinator, observes Weak snapshot counts for active/pending and exercises real completion
backpressure plus lease/ID/generation mismatch and newer-query survival. Test-only dequeue probes
control scheduling; no production hook, volume discovery or real service operation is used. Keep
m4_, launcher_, existing cooperative-cancel and executor-shutdown regressions. Packaged LocalSystem
idle/restart/base+delta/search acceptance requires separate authorization and remains unexecuted here.
### 7. Wrong vs Correct
Wrong: hide -> UI generation only -> service query continues. Correct: local generation retirement
-> bounded framed request-specific cancel -> current lease/identity match -> pending drop/active
cooperative return -> real job release. Fixed correlated cancel-handled and worker-retired events
contain IDs/generation/lease/match/elapsed only, never query/path payloads.

## Scenario: Exact mapped publication after rebuild/compaction
### 1. Scope / Trigger
Representation optimization for completed owned scan/compaction bases, not attribution of search leakage.
### 2. Signatures
`mft::{scan_volume_and_persist_with,compact_system_snapshot_with}` share production persistence seams;
`cache::reopen_persisted_snapshot(MftSnapshot) -> Result<MftSnapshot>` consumes only the new build.
Evidence: `repo:rust/apps/nxtools-indexer/src/mft/cache/publication_tests.rs`.
### 3. Contracts
Durable base success -> release newly built owned rows/auxiliaries -> exact read-only mapped reopen
-> rebuild bounded search data in persisted row order -> existing service publication. Preserve journal,
cursor, count, row metadata, base+delta continuation, stale-compaction checks and old immutable leases.
No startup candidate fallback/cleanup at reopen, owned-success fallback, cache format/search change or trim.
### 4. Validation & Error Matrix
Write failure -> original failure. New file missing/truncated -> reopen failure, no successful build or
compaction terminal; last-good remains Ready/searchable, first failure never fabricates Ready.
Stale compacted cursor -> discard without replacing newer state. Cleanup waits for the original
publication/worker-quiescence boundary; leased mappings remain readable until actual owner release.
### 5. Good / Base / Bad Cases
Good: mapped service snapshot has no owned rows/masks/reference/child tables; new delta replays on it.
Base: search auxiliary memory remains bounded by its existing budgets. Bad: transfer owned row IDs to
file-reference-sorted mapped rows, select an older cache as a successful build, or trim working set as proof.
### 6. Tests Required
`mapped_publication_` writes small real temporary bases, runs scan/compaction publication and the actual
compaction worker terminal handoff, checks duplicate-name groups/postings, owned Weak release, old search
lease/retirement, base+delta replay, cursor/metadata/count, reopen failure and stale-compaction rejection.
Retain cache/journal faults, `m3_`, `u4_`, `persistence_fault_` and frozen search-result regressions.
These prove representation/lifecycle, not private-commit savings. No million-row or real-volume/service
tests without authorization; packaged LocalSystem restart/base+delta/search/idle CPU+I/O remains separate.
### 7. Wrong vs Correct
Wrong: write owned -> publish owned, or write -> startup cache selection -> report build success.
Correct: write -> exact mapped reopen -> original publication; failure -> original last-good/error path.

## Scenario: Global single-worker compaction
### 1. Scope / Trigger
Multiple volumes reach the existing overlay threshold together; prevent overlapping complete owned
compaction builds without changing threshold, format, search or native service policy.
### 2. Signatures
`run_coordinator` owns active/pending compaction state; `schedule_snapshot_compactions(&mut Vec<String>, ...)`
admits one source through the original `spawn_snapshot_compaction_with` handoff. Test-only
`IndexerService::compaction_source` injects temporary persistence/failure, not alternative scheduling.
### 3. Contracts
Maximum one executing complete compaction globally. Pending entries are volume names only, validated
against current enabled/oversized snapshots before launch. Hold active ownership until real terminal
processing and worker-quiescent retirement; no replacement worker on delay/timeout. Success, stale
completion or failure advances the waiting batch. Retain each volume's overlay, fault and retry deadline.
Last-good SearchExecutor output must complete while compaction remains held. Original maintenance
projection, build concurrency, delta durability and mapped-publication contracts remain unchanged.
### 4. Validation & Error Matrix
Held first worker -> no second admission. Success -> publish, retire released old mapping/pair, then next.
Failure -> retain searchable last-good/error/backoff, release worker input, then another ready volume.
Disabled/removed/rebuilt waiting volume -> discard stale pending name. Standalone EOF -> finish the
discovered eligible batch, not future poll/retry work. Persistent disconnect -> preserve service work.
### 5. Good / Base / Bad Cases
Good: both oversized volumes eventually compact with observed maximum concurrency1. Base: leased
mapping deletion failure still defers retirement under the original policy. Bad: clone every pending
snapshot, let another volume clear a failure, exit after only the first EOF terminal, or replace a held worker.
### 6. Tests Required
`serial_compaction_` uses the actual coordinator/scheduler/worker, two small temporary mapped bases
and original-threshold synthetic tombstones. Observe concurrent execution, held-worker search,
success/failure advancement, old mapping Weak ownership, obsolete-pair cleanup before next start,
independent retry/error retention and EOF batch drain for both terminal outcomes. Retain `m3_`,
`mapped_publication_`, journal fault/replay, compaction retry, maintenance, admitted reconnect and search
retirement regressions. Synthetic concurrency/lifetime evidence is not a private-commit measurement
or packaged LocalSystem acceptance. Real service/volume/stress/package operations require authorization.
### 7. Wrong vs Correct
Wrong: for every oversized volume -> spawn a full compaction. Correct: retain names -> admit one ->
real worker terminal -> original result/error and retirement -> revalidate and admit the next volume.
