# Implementation plan

Status: in_progress after user approval. The task owns the shared requirements and final integration acceptance; independently verifiable workstreams follow the concurrent-file/build boundary in [coordination](coordination.md).

## 1. Native picker ownership and lifetime (R7)

- Refresh indexed ABDM/legacy/archive reference notes with verified entry files, constructor/base/native presentation chain and lifecycle invariants from research.md before implementation.
- Add production-boundary regression tests before changing behavior:
  - Enqueue every human picker through the real CommandRouter, advance controlled time beyond 10 seconds, two minutes and the FFmpeg caller deadline, and assert it remains pending. Ordinary machine commands must still expire.
  - Delayed selection completes once, normal Cancel is quiet, original owner close/disconnect retires once, and a late result cannot update a reopened/replaced/edited draft or start package export.
  - C# self-owned HWND fixtures observe the actual source owner handed to the native presentation seam and Topmost suspend/restore state. Test native modal admission through actual cleanup and verify unrelated command dispatch while selection waits.
- Carry initiating HWND/peer through protocol models, callers, parsers and C# adapter. Use one existing bounded STA/modal owner, not a second thread/controller. Add the narrow matching cancellation path required for owner retirement.
- Remove human elapsed deadlines from router/native/feature owners. Split browser destination selection from bounded export execution, preserving the original pending operation and correlation.
- Update affected tests/gates for protocol roundtrips and real caller-ID/generation behavior. Do not globally increase command timeouts or remove native cleanup bounds.

Expected files: Rust protocol lib.rs and picker contracts, IPC lib.rs and targeted tests, Desktop downloads/clipboard/archive/extension picker call sites and lifecycle tests; C# parser/pipe routing/individual picker adapters/existing STA boundary and isolated fixtures. Each file is necessary to carry owner identity or correct lifetime across the existing call chain.

## 2. Settings scrolling, theme identity and effective paths (R2/R3/R5)

- Add real GPUI wheel/keyboard/bounds regressions at minimum supported window sizes. Wheel input over a theme preview or category form must move the page rather than an inner viewport; last option/field and Apply/Cancel remain reachable.
- Replace duplicated default/named theme nodes with a single visible palette choice and legacy alias selection mapping. Test opening and cancelling old saved aliases without writing config.
- Remove the theme inner viewport. Use released Settings page scrolling and the existing official Radio controls.
- Replace nested category master/form scroll regions with an official selection field and natural-height original editor. Test maximum supported categories, Add/Delete/Reorder/Enable, preserved drafts and once-only Save/Cancel.
- Display effective default/custom temporary paths using existing application/native policy. Test Rust display/C# native result parity, custom appended subtree, default reset, failed save, reload conflict and picker late-result rejection. No cache IO or migration during render.

Expected files: network_settings/theme_chooser.rs and kit_migration_tests.rs; downloads/settings_view.rs plus relevant settings/rem/polish regressions; main_window/archive_page.rs and archive_temp_settings.rs and existing archive settings tests; owning AppPaths/native policy seam only if needed for actual resolver parity. No dependency/vendor changes.

## 3. Archive Shell entry points and uninstall cleanup (R4/R6)

- Include Create in fresh recommended defaults and show its ordinary-file/folder purpose. Preserve deliberately saved custom selections; apply changes only through the original explicit registration transaction and grouped --archive-compress route.
- Add red-capable registration-plan/creation-dispatch fixtures for one file, one directory and mixed multiselection, then connect only the missing boundary. Keep explicit Apply/Revert and original archive creation owner.
- Add isolated uninstall fixtures with fake known folders, actual application-owned DB/cache files, app-owned process images and unrelated sentinel files/processes. Verify cleanup and preserve-data paths separately from upgrades.
- Expose clear default-remove/explicit-retain personal-data semantics. Remove app history/settings/caches and runtime state only from approved owned locations; never delete download outputs or a custom directory wholesale.
- Preserve exact process image and protected tuple/service ownership. Complete desktop/background/helper shutdown before destructive cleanup; revert owned Shell/native-messaging registration and remove installation-owned services/protected deployment through the existing privileged boundary. A lock, stop or cleanup failure is reported, not silently converted to success. Preserve download outputs even inside application data roots.

Expected files: core archive defaults, original archive settings/UI labels and registration-plan/dispatch tests; installer/nxtools-next.iss, installer shutdown/cleanup entry, existing privileged native owner and isolated cleanup fixtures. No new archiver backend or installer migration framework.

## 4. Indexer first-build latency (R1)

- Read the owning current indexer plans using their exact canonical phase names; this task does not renumber or pull forward their phases.
- Adapt the existing performance/service probes to retain the authenticated approved Desktop required by the current service. Establish an authorized isolated-cache initial-build baseline; never delete the current service cache for a cold run.
- Measure discovery, scan/native metadata IO, construction, persistence, reopen and per-volume publication using one correlation and bounded stage events/item counts.
- Add real FILE-record/fixup/identity/standard-time parsing fixtures and a red-capable initial-build test for row-by-row metadata-open amplification. Once measured attribution confirms the cost, replace that hot path with sequential raw-MFT metadata extraction. Keep the incremental metadata reader, true file times, scan-start journal cursor and replay.
- Add a blocked-later-volume regression proving the first durable/exactly reopened volume is searchable while final status remains Building. Preserve failed-first-build and last-good publication behavior. Do not add multivolume concurrency in this change.
- Report before/after first-search/all-volume stage timings and counts. Run focused failed_first_build_without_a_snapshot_never_publishes_ready, a_slow_initial_snapshot_does_not_block_handshake_or_ping and mapped_publication_ filters alongside new synthetic cases; none replaces native measurement.

Expected files: indexer lib.rs, mft.rs/time_metadata.rs, existing cache/maintenance stage owners and synthetic tests; existing performance/service scripts and the narrow authenticated Desktop observation entry needed by those probes. No cache format rewrite, dependency upgrade, full scan on reconnect or persistence removal.

## 5. Verification and integration

Before each command, state the concrete failure it can detect and what the next change would be if it fails. Run only relevant checks; a green structural gate is not behavior acceptance.

### Safe compiled checks after implementation approval

Every Cargo invocation, including listing/build wrappers, inherits:

```powershell
$env:CARGO_BUILD_JOBS='1'
$env:CARGO_INCREMENTAL='0'
$env:RUSTFLAGS='-C codegen-units=256'
```

Coordinate one Cargo process with a full-operation timeout. Do not start Cargo if compiled verification has been declined because of memory pressure.

- `cargo test -p nxtools-protocol --lib` and `cargo test -p nxtools-ipc --lib`: detect shape/owner propagation, human/machine deadline confusion, pending-before-send, stale generation and disconnect regressions.
- `cargo test -p nxtools-desktop -- --test-threads=1`: exercise real GPUI layout/actions/draft/lifecycle; run focused newly added cases red before fixing, then integrate with the desktop suite. Record existing unrelated failures without changing their owners.
- Indexer: use exact isolated synthetic filters established from the inspected test inventory; do not run unrestricted real-volume tests. A regression must detect the measured expensive path while validating identical search data.
- C#: build the affected host/test projects, list the exact safe fixture selection, then run only reviewed owner/STA/protocol/export/uninstall-plan fixtures. No production signing keys, user registry, system picker or service effects.
- `cargo fmt --all -- --check`, affected-owner strict Clippy, workspace type check and relevant architecture lifecycle gates detect formatting/type/boundary drift. Scope runtime tests to the actual changes.
- When updating reference/spec contracts, run `python tests/architecture/spec-contracts.py`; UI contract changes also run gpui-component-catalog.py, gpui-release-contracts.py and markdown-editor-contracts.py as required by AGENTS.md. These are source/document checks, not native acceptance.

### Separately authorized live acceptance

- Real file/folder/save dialogs from primary/settings/auxiliary windows: delayed selection past old limits, Cancel/Escape, owner close, application Topmost windows, unrelated application focus, keyboard and supported DPI.
- Disposable install/uninstall: normal and retain-data paths, background download owner, configured clipboard/archive storage, ordinary/protected installs, download output preservation and no unrelated process/service changes. Never uninstall the user's current installation to run this gate.
- Explorer Shell creation: ordinary file, folder and mixed selection enter the existing create workflow once; Apply/Revert and uninstall remove only owned registration.
- Indexer package release: `tests/performance/indexer-idle-cpu.ps1 -RestartService -AssertRegression` against the packaged LocalSystem service, followed by the existing real service search smoke test. Restart restores base plus persisted delta; assert the atomic protected/runtime release identity. First-build performance and restart-idle/search are distinct measurements.

## Stop and rollback boundary

Finish the requested change when targeted regressions and relevant integration checks pass. Preserve task-owned diffs and disclose deferred native/performance acceptance. Do not package/install/restart/clear data without the separate operation approval. If any release/native gate is not run, report it as NOT RUN rather than substituting compilation or screenshots.
