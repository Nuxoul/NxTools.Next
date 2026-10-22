# Indexer persistence fault visibility — outcome (2026-10-13)

## Scope and authority
User requested this concrete bug fix after whole-project review. Existing dirty tree preserved. Change boundary: IndexerService maintenance/settings error state and narrow injectable persistence seams, synthetic/temp-file tests, owner/reference evidence. No MFT/USN algorithms, cache VERSION3/base+delta wire layout, search ranking/cancellation/generations, service admission/authorization, installer/deployment or native UI changes. No new dependency/framework/worker. mft/cache.rs production implementation is unchanged; its only new code is cfg(test) regression. settings.rs only widens the existing path-specific writer to parent-private access for isolated real-IO tests.

References: read Indexer/shared specs and quick-launch research first. Listary managed/session boundary establishes last-good availability; proprietary native engine source was unavailable. Inspected SwiftList Core/DriveMonitoring/DriveRecovery.cs::RestoreOrRebuild/TrySaveDriveCache and current Core/DriveMonitoring/UsnMonitor.cs: restore/catch-up per drive, save failure does not discard usable runtime, journal recovery remains bounded. Those sources are not claimed to supply Next last_error policy. Verified current call chain and correction invariants recorded in docs/reference-code/quick-launch-research.md before production fix.

## Red evidence and root cause
Added parent-private injected write-function seams inside actual apply_settings/poll_journals implementations first, leaving behavior unchanged. Four initial persistence_fault_ regressions compiled and ran:0pass/4fail at missing visible cursor/delta/settings persistence errors. Scratch indexer-persistence-red.log preserves red evidence. All fixtures use synthetic descriptors/scanners/pollers, never real volume discovery or scanning; injected failures use native5/112.

Causes:
- CursorAdvanced/Updated only warned on failed write, then installed usable memory state and removed journal error.
- refresh_last_error had no persistence owner, and NoChange/RebuildRequired success erased unrelated poll state.
- apply_settings only warned; equal desired settings returned before retry, so failure could not recover through the same draft.
- Rediscovery reconstructed service state without a persistence error owner.

## Fix and unchanged order
- Separate per-volume persistence_errors and settings_persistence_error join existing build/journal/compaction owners in refresh_last_error.
- Save attempt still precedes current cursor/delta in-memory publication, exactly as original behavior; write failure remains Ready/searchable, retaining last-good immutable base plus usable overlay. It no longer claims durable health.
- Failed current snapshot is retried on later NoChange maintenance using the existing1..8second idle cadence, not a new polling thread/scan. Only corresponding successful save removes that volume fault. Other volume work, poll success, RebuildRequired and ordinary scans do not acknowledge it.
- Settings same-value updates retry if prior write failed; only successful settings commit clears its own error. Original live desired settings-before-write policy retained. Settings success does not erase delta failure and journal success does not erase settings failure.
- Rediscovery retains settings error and errors for still-present volumes. Disabled volumes retain unsaved fault until meaningful save recovery or disappearance, rather than silently presenting durable success.
- Failure/recovery diagnostics are low-frequency transitions, deduplicate identical failures, contain local operation correlation/elapsed/native code, not raw backing paths/error text. Existing protocol Envelope ID generator reused without adding uuid dependency or sending extra frames. No DEBUG probes.
- StatusChanged covers persistence state transitions and existing Status replies expose errors/recovery with original request identity. RebuildRequired reason remains solely maintenance and last-good search stays available.

## Tests and concrete checks
New9 persistence_fault_ tests cover cursor failure/idle retry/current cursor save; delta failure/RebuildRequired/other-volume/rediscovery; settings same-value recovery/unrelated journal/build; repeated same fault quiet/per-volume recovery; settings success vs delta failure; real blocked settings temporary-file preservation/recovery; real delta atomic-replace failure/base search/cursor reload; correlated Status reply.

Eight existing synthetic maintenance fixtures now inject successful persistence explicitly because their synthetic snapshots have no persisted base. All original poll/rebuild/query/status assertions retained. Prior implementation hid that fixture's actual missing-base write failure; production write rules were not weakened. New real-IO tests execute original file writers in temporary directories and require visible failures and actual restored JSON/cursor after successful retry.

Executed final exact filters:
- persistence_fault_:9/9.
- scheduled_journal_rebuild_is_maintenance_not_a_user_facing_error; journal_recovery_scopes_rebuild_to_the_affected_volume; one_volume_poll_failure_does_not_starve_a_healthy_volume; healthy_journal_polling_does_not_hide_an_initial_volume_build_failure; a_published_usn_overlay_is_visible_to_subsequent_searches; compaction_failures_are_visible_and_exponentially_backed_off_per_volume; index_settings_limit_volumes_and_filter_excluded_paths_at_the_service_boundary; failed_maintenance_build_keeps_last_good_snapshot_ready_and_searchable:8/8.
- settings::tests:::3/3; persisted_delta_restores_cursor_replacements_and_deletions:1/1; r8_search_matches_frozen_row_scan_with_delta_filters_and_cancellation:1/1.
- Total22 filtered tests passed,0failed/ignored. No unrestricted Indexer suite.
- cargo clippy -p nxtools-indexer --tests -- -D warnings, cargo check --workspace --tests, cargo fmt --all -- --check:exit0.
- indexer-lifecycle.ps1:passed; spec-contracts.py:87documents/12owners passed; git diff --check:exit0. These source checks do not execute native/release acceptance.
- Required jobs1/incremental0/codegen-units256 and locked1.96.0 command-local PATH applied on every Rust command; no parallel/surviving Cargo build.

Recoverable fixture/compile corrections recorded separately: no direct uuid dependency (reuse protocol-generated local ID); rediscovery fixture needed original Ready transition via synthetic rebuild; normalized mount key strips trailing slash; delta-reader test used actual existing signature; search fixture explicitly enabled files instead of disabled default. No tests ignored, assertion weakening or production scope/default/permission changes.

Logs in session scratch: indexer-persistence-red.log, indexer-persistence-regressions.log, indexer-persistence-safe-final.log, indexer-persistence-real-io.log, indexer-persistence-quality-2.log, indexer-persistence-final.log. Pre-task lib snapshot: indexer-persistence-before.rs.

## Remaining release boundary
No source/compiled blocker remains for this fix. No actual service/registry/installer/package operation, real disk scan or service search performed. Packaged LocalSystem restart with persisted versioned mmap base+delta, real service search, approved/runtime tuple hashes and indexer-idle-cpu.ps1 -RestartService -AssertRegression remain mandatory release gates pending separate authorization. Synthetic Ready/search and temporary files do not certify release acceptance. No updated Release delivered or running service behavior claimed.
