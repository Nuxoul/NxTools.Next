# Indexer correctness design

## Boundary

The behavior gaps live in USN recursive expansion, native enumeration input, cache publication/restore, the service coordinator lifetime and status projection. Change those owners, not search ranking or popup interaction. Preserve base VERSION3/new durable-journal compatibility, original persistence-before-memory publication and true fault owners. Reference review and current source chains are recorded in docs/reference-code/quick-launch-research.md, section2026-11-01 journal correctness and maintenance status.

## H2 — visible recursive work

Direct invisible/leaf deletion performs no descendant expansion. Visible directory deletion bounds unique currently visible child candidates, excluding tombstones and moved-away replacements before counting. Only actual descendant expansion consumes the existing4096unit budget. Keep the same transactional cloned-snapshot failure behavior and genuinely-large-subtree RebuildRequired contract. Tests apply5000direct visible leaf deletes, a mostly-tombstoned wide directory, invisible deletes and the existing true-large case through apply_journal_batch.

## H3 — complete scan and idempotent catch-up

Use FSCTL_ENUM_USN_DATA HighUsn=i64::MAX while retaining the journal ID/NextUsn read before scanning as the snapshot's replay cursor. This includes preexisting files and directories whose latest USN moves during enumeration. A narrow injected enumeration seam must be used by the native path and synthetic fixture: the fixture filters records according to the production requested upper bound, then replays metadata-only changes. The same fixture verifies created-and-modified rows are not duplicated. Do not insert arbitrary unknown Metadata rows or spend additional MAX_METADATA_READS_PER_POLL budget; existing metadata does not resurrect split rename/deleted state.

## M3 / L11 — publication and cache bounds

Expose the existing obsolete-file cleanup through the current persisted snapshot owner, using its actual cache path and mount-derived prefix. Invoke after successful build/compaction publication has released the service's previous snapshot references. Keep current base/delta excluded; outstanding search mappings may make deletion fail and remain for the next cleanup. Restore continues to select a complete current journal-bound pair before cleanup. No new cache format, hash, retry manager or snapshot-residency policy.

Bound replacement/delete declared counts against the actual legacy delta payload length, with checked minimum record/string/delete-table arithmetic, before HashMap/HashSet capacity allocation. Rejection follows existing base+USN catch-up recovery. Use a safe small corrupt-count allocation regression before fixing; the maximal corrupt-header case is executed only after the preallocation bound exists, not as an OOM experiment. Test-only allocation measurement may observe the real System allocator; no product instrumentation or dependencies.

## M4 — one coordinator, replaceable endpoints

Move the original runtime event loop's lifetime from one pipe session to the Windows service. The same coordinator owns IndexerService, active/pending build generations, compaction set, poll schedule and SearchExecutor. Build/compaction completion channels survive client disconnect/takeover; a replacement connection never creates a replacement coordinator or duplicate scan. Completed last-good snapshots apply even without an attached desktop.

Keep actual PID/session/path/hash authentication, four admission leases,1MiB framing and3second initial Hello before registration. A service-generated session lease distinguishes every admitted endpoint. Registration retires prior connection-owned search work and resets the new client's search generation once. Old-lease request/closed/result records cannot mutate or send to the current client. Scanner/compaction completion belongs to the service, not the originating desktop lease.

Pipe writes stay outside the coordinator and any state mutex. Each admitted endpoint has bounded response delivery; a full queue or expired native write retires only that endpoint. Reuse LocalPipeReader's existing idle-peek loop for retirement checks and the existing deadline-limited PIPE_NOWAIT write approach for service output. Do not wait for A's writer or its running scan before B receives HelloAck/current status. Keep standalone fixture/stdin composition on the same runtime mechanics rather than adding a competing implementation.

This is the minimum ownership correction that covers a nonreading A plus an in-flight scan: merely unlocking commands retains session-local worker completion and can lose a finished scan; waiting for A to exit still waits for its worker. The requested bound covers asynchronous scans/rebuilds and blocked client I/O, not an arbitrarily stalled native poll/persistence call. No new polling-I/O architecture is included.

## U4 — additive maintenance projection

Add IndexDriveStatus.maintenance as an optional serde-default field omitted when absent. The typed projection is IndexDriveMaintenance::InitialBuilding / Rebuilding { reason: IndexRebuildReason } / Ready; fixed reasons are JournalWrapped/BulkChanges/CursorInvalid/Requested. Missing legacy data retains existing count/unavailable copy; new state does not grant search or service authority. Old desktop DTOs ignore the new field. Keep outer protocol version1.

The coordinator projects initial build while an enabled volume has no usable snapshot, rebuilding while work retains last-good, and ready after matching successful publication. Rebuild decision/queue/start/terminal transitions are bounded, correlated maintenance events with fixed state/reason/native/elapsed metadata. No per-record/poll-idle stream or raw paths/query/error payload. Genuine poll/persist/compaction/worker errors remain in original last_error owners.

UI shape: retain Settings -> Index SettingPage -> existing volume selection rows and their scroll/selection callbacks. Use released Component0.8.0 Tag::info/Tag::warning/Tag::success, with_size(Size::Small), official foreground/background and a semantic Label child; plain readable reason accompanies rebuilding text `正在重建，搜索结果可能不完整`. A layout wrapper may arrange the Tag and description but may not repaint a substitute. Tag has no Toolbar adaptation. Preserve checkboxes, current save/deferred application, minimum-width containment and original row hit semantics. No added popup status or620x410geometry change, window/native focus or show/hide changes.

## M8 / L11 measurements

Default-ignore a synthetic1million-row measurement, with1k and100k overlay cases. Record single apply and classification time plus allocation count/requested bytes; separate setup from measured work. Measure cursor-only/no-change behavior as distinct inputs. Decide optimization against measured coordinator occupancy/search scheduling, not source suspicion or idle heartbeat logs. Preserve transactional failure and immutable search snapshot ownership if an optimization is justified. Separately compare heap-owned and mapped persisted synthetic snapshots, reporting allocator/file-mapping and actual process counters with clear units; keep production residency unchanged.

## Verification scope

Use production batch, enumeration, cache, service-runtime and Settings/TestApp boundaries with synthetic records/temp files/temporary pipes. Add behavior-red regressions before production fixes. One Cargo process at a time, offline, jobs1/incremental0/codegen-units256; Desktop debug0/one test thread. List and exclude native/system-data tests before running indexer suites. Final scoped tests/lint/fmt/spec/lifecycle and optional one bounded review are separate from unauthorized packaged LocalSystem and physical/user acceptance.
