# SOL review remediation — implementation and verification

Date: 2026-10-08. Scope: the approved four-child remediation task in the current working tree on `refactor/ui-gpui-kit`, based at `70707a6`. Existing unrelated changes were preserved. This report supersedes the outstanding-status ledger, not the historical evidence, in [the initial review](2026-10-08-download-transition-and-feature-review.md).

## Outcome and acceptance boundary

Confirmed review defects have code repairs and automated regressions. Strict Clippy, workspace test compilation, formatting, eleven architecture gates and the post-lint targeted suites pass. The initial two serial download `engine_contract` runs each returned 102/103. **2026-10-09 closeout:** both symptoms were reproduced and traced to the shared Winsock HTTP fixture, repaired without product-code changes, and one post-fix serial full run passed **103/103**. See the dated closeout below; the earlier failed evidence remains historical.

The parent and all four children remain `in_progress`. This is not release acceptance: no package/install, product launch, real Directory Opus navigation, real clipboard inspection, or Indexer service mutation was performed during this remediation. No commit or archive was made. Previously built Validation artifacts do not contain the current working-tree fixes merely because they exist.

## References and ownership

- Download: AB Download Manager first, including `PartSplitSupport.kt`, `HttpDownloadJob` and single-download window/component flow; then legacy `D:\Code\NxTools\Services\DownloadManagerService.cs`, `Views\AddDownloadWindow.axaml.cs`, `DownloadProgressWindow.axaml.cs`, and `WindowAnimationService.cs`. Legacy constructors/Open/Closed and animation/native-visible transitions were traced. Avalonia's native/base implementation is absent from that checkout; it was not claimed as inspected.
- Clipboard: legacy `ClipboardRepository.Cleanup`, `ClipboardHistoryService.ImportJson`/`Configure`, `ClipboardPasteService.CopyItemAsync`, and the main-window dispatch/storage controls. Portable ZIP import and actual storage migration are Next-specific contracts, not legacy ZIP behavior.
- Launcher/indexer: SwiftList `Plugins\CustomCommands\ArgQuoting.cs` and `Core\Services\UsnServicePipeServer.cs`; Listary evidence is only its managed search/session/index-status boundary. **Listary's native index engine source was unavailable.**
- Native system: legacy `WindowsFileDragService.cs`, ABDM completed-download presentation, and the actual Next HWND/client-readback, OLE, registration and protected-release ownership seams.
- Rust owns state/layout/storage scheduling; C# owns Windows APIs; GPUI renders and learns actual native client size through `WM_SIZE`. A native acceptance acknowledgment is not proof that pixels were applied.

## Initial-review finding ledger

| Finding | Implemented correction | Automated evidence / limit |
| --- | --- | --- |
| R01 ordinary indexer authorization | Authenticate actual pipe PID/session and registered protected desktop path/hash before the service mutex; four pending slots; first Hello read has a three-second non-resetting deadline | Admission regressions and temporary pipe tests pass; protected LocalSystem service not exercised; unauthorized development/Validation clients fail closed |
| R02 unreachable adaptive shrink | Preserve shrink worker identity through join, sync and Waiting checkpoint; merge adjacent Waiting ranges only afterward; append one speed observation per tick; evaluate stable shrink before low-benefit/growth decisions; request-derived floor and restored-plan baseline | Production localhost runner proves 2→3→2 and at-ceiling 2→1, durable pause/reopen/resume and exact 5 MiB output; fifteen consecutive passes before mechanical lint edits and final download-lib rerun pass |
| R03 DASH completed fragment mismatch | Completed checkpoint bytes must match the nonempty fragment length; mismatch resets/re-fetches instead of adopting corrupt state; valid uncheckpointed crash-window adoption remains | Real reopen/corrupt-fragment regression passes; this is length reconciliation, not cryptographic validation of equal-length bytes |
| R04 premature copy-action hide | Acceptance-time hide obeys `ActionSuccessPolicy`; CopyPath/CopyCommand KeepOpen actions retain the current panel | Integrated GPUI action→IPC→still-open assertions pass, not merely policy-helper tests |
| R05 false Ready after first build failure | No snapshot means not Ready; failed maintenance with last-good snapshot remains Ready/searchable | Both synthetic index-state regressions pass; no service CPU/restart/search release gate |
| R06 incorrect purge minimum | Permanent deleted-row purge uses `max(7, retention_days)`; favorites/pinned soft-delete behavior preserved | 1/7/14/30-day retention matrix passes |
| R07 ZIP orphan assets and ambiguous partial import | Preflight importable rows only; cleanup tracks newly owned assets/temp files and never deletes reused shared assets; late DB failure reports partial counts and removes only proven unreferenced newly owned resources | Invalid/missing asset fixtures, skipped row, and real SQLite trigger with two icons pass; committed row and its asset remain |
| R08 migration switch before durable config | Worker serializes prepare/replacement open/config commit before switching live history; ordinary settings save preserves latest disk storage root; completion reloads latest disk config | Worker/config failure and delayed-completion settings-save regressions pass; no real user database migration |
| R09 name-only uninstall service stop | Stop is guarded by exact quoted ImagePath/`--service` and owned installed/protected tuple/hash | Source and C# ownership tests; installer was not compiled/run |
| R10 cancelled drag indistinguishable from drop | Normal Escape cancellation remains non-error, with distinct cancelled/dropped terminal status in result data/diagnostics | Native cancellation/result regressions pass; no real Shell drop |
| R11 duplicate USN budget inflation | Deduplicate recursive candidates before applying the bounded budget; genuine large subtree remains bounded | Duplicate-heavy and large-subtree regressions pass; no real journal soak |
| R12 payload diagnostics | Remove query/source-directory/icon-key payload fields; reqwest errors strip URL; production lifecycle metadata remains bounded | Source inspection and architecture checks; no payload-secret inspection |
| R13 stale architecture scans | Gates follow actual extracted Rust modules/C# partials; PowerShell 5-compatible SHA checks preserve prior assertions | Both repaired gates and eleven-gate integration rerun pass |

## Additional independently confirmed SOL defects

### Window request ownership and responsiveness

`CenteredResizeCoordinator.cs` has a fixed budget of two background native workers and 64 HWND slots, one in-flight operation plus one coalesced next operation per HWND. Native execution does not hold the global scheduling lock. The command reader and EventWindow enqueue rather than waiting inside cross-process `SetWindowPos`; initial result distinguishes `status=accepted, applied=false`.

Pending resize/recovery uses immutable generation transactions, compare-and-update and compare-and-remove. Even repeated `Guid.Empty` requests have distinct generations. Replay carries the same complete snapshot; old completion cannot overwrite, retire, or report terminal ownership for a newer request. Final client readback follows center correction and any final client correction. Low-noise `download.window_resize.readback` can prove settlement even when an initial worker retires without a separate recovery event.

Native calls themselves are **not interruptible**. A permanently stuck call retains a worker; timeout does not create an unbounded replacement thread. If both workers are stuck, subsequent work is bounded/coalesced/rejected, not magically recovered.

### Probe/error/media presentation and GPUI leases

- `begin_probe` retires the old generation and confirmed target before URL validation; URL-change submit failure also retires stale state. Invalid input cannot leave a previous confirmation/watchdog authoritative.
- In a truly compact viewport, the error replaces the existing guidance line. It does not append a clipped row while waiting for native reserve.
- Ordinary compact/confirmed heights remain **176/278 logical pixels**. Media height is content-budgeted: five-track HLS with three visible rows uses 424 (462 with error); the tested single-row DASH case uses 355. No ordinary oversized-window workaround was introduced.
- Submission error reserve is based on the same media-aware layout and actual viewport. Ten-second presentation timeout produces retryable error instead of releasing clipped confirmation.
- Task detail expansion uses the same centered native boundary, carrying a read-only bridge snapshot. Real HWNDs do not also receive GPUI `window.resize`.
- Settings export and shared blacklist synchronization defer cross-entity side effects beyond the active GPUI lease, including fast/cache-hit paths.

Actual pre-fix GPUI failures included generation remaining 17 after invalid probe, compact error creating an extra clipped row, and media error bottom 406 overlapping a footer beginning at 396. Earlier custom-client HWND tests first failed on the prefixed P/Invoke export, then on final height 176 after center correction. Those boundaries now pass; none is a substitute for the user's live pointer/DPI reproduction.

### Blob receiving lease and browser registration

Active Blob `Receiving` is not idle-reaped; only the connection owner releases it. Each body has a two-minute total deadline with per-read timeout capped at 30 seconds or remaining time. Duplicate Receiving claims remain rejected; abandoned/failed/terminal states clean up and release capacity. Real temporary socket regressions pass.

`BrowserRegistrationTransaction.Apply` captures all prior file/registry state before mutation and records an attempted write **before** invoking it, since a write may mutate then throw. Every attempted manifest/browser/legacy-key step is compensated in reverse order; rollback failures surface as an aggregate instead of silently claiming success. Synthetic stores exercise every failure step; real registry failure injection was not run.

### Windows arguments and configuration isolation

Windows argument splitting preserves ordinary backslashes and implements backslash-run/quote parity, including trailing quoted separators and escaped literal quotes. Finder tests cover prepared commands, not just strings passed to a helper.

App configuration updates lock, reread disk, change only the owned section and commit via a same-directory temporary file. Missing/corrupt config returns failure rather than silently rebuilding it. Process names are normalized to lowercase before stripping `.exe`. Clipboard and Quick Launch share the process list with separate scopes and preserve legacy compatibility.

## One final SOL review and the concrete follow-up

Exactly one optional final directed SOL review was performed. It found an interleaving after the migration worker durably committed destination D but before the GPUI completion replaced its cached source S: an ordinary settings save could write S back to disk.

`settings_save_during_migration_result_delay_preserves_committed_destination` exercised that real worker/config seam and **failed before correction (S != D)**. `save_clipboard_section` now preserves the latest disk `storage_directory` inside the locked merge; only the migration transaction owns that field. `refresh_committed_migration_config` rereads current disk config instead of publishing an early merged snapshot. The regression and the final 440-test desktop suite pass. No second broad review was started.

## Fixture diagnosis, not production policy relaxation

The new grow/shrink fixture initially timed out after genuine client retry disabled adaptation. Bounded temporary probes showed Winsock accepted sockets inherited the listener's nonblocking state: an early read returned `WSAEWOULDBLOCK` (10035), dropped the connection and correctly triggered `DisabledAfterRetry`.

The fixture now explicitly makes accepted streams blocking and uses bounded buffered header reads and `write_all`. A separate timeline assertion was corrected for the store's newest-first ordering. No production retry policy, deadline or sample-count assertion was weakened. All `DEBUG-SOL-SHRINK` probes were removed. The final test still asserts four slow samples cannot shrink and five can, real writer synchronization/checkpoints, prefix identity, active cap, durable timeline decrease and exact resumed bytes. Injected samples do not validate real one-second speed measurement under external network jitter.

## Verification actually performed

All Rust commands use `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`, with one Cargo command at a time.

### Final post-lint rerun

| Check | Result |
| --- | --- |
| Clippy for desktop/download/browser-host/clipboard/finder/indexer/ipc, `--tests -- -D warnings` | Pass; no added warning suppression |
| Desktop full suite, serial test threads | 440 passed |
| Clipboard library | 43 passed |
| Finder library | 30 passed |
| Download library, including full grow/shrink fixture | 110 passed |
| Download domain contract | 12 passed |
| BrowserHost library | 29 passed |
| `cargo check --workspace --tests` | Pass |
| `cargo fmt --all -- --check` | Pass |
| `git diff --check` | Pass |
| Eleven architecture gates | Pass |

Architecture gates: download-ui-lifecycle, clipboard-lifecycle, clipboard-ui-lifecycle, quick-launch-lifecycle, desktop-shell-lifecycle, file-dialog-window-follow, file-dialog-search-lifecycle, browser-native-messaging, ui-token-boundaries, package-memory-policy, indexer-lifecycle. Running package-memory-policy only checks source policy; it does not package anything.

### Earlier integration in the same remediation

- Core 39, Network 2, IPC 23, Protocol 39, Download IPC 6, CLI 10: pass.
- Download HTTP client 22, HTTP fixture 6, storage 7, artifact 4: pass.
- Browser extension Node suite: 21 passed.
- Indexer: four focused library regressions plus two admission tests pass; unrestricted real-volume suite deliberately not run. Two temporary named-pipe deadline regressions pass.
- Safe C# harness: **187 PASS labels, 0 FAIL, 2 explicit environment SKIP** (Files and Explorer unavailable). PASS labels include the two skip-returning cases; they are not 187 live native assertions. The two real DOpus cases were excluded.
- Five native concurrency/compensation cases: twelve consecutive rounds pass. Custom-client size/center/retirement regression also passed twenty repetitions in the earlier window repair.

Session evidence lives in scratch, notably `sol-clippy-final.log`, `sol-post-lint-validation.log`, `sol-architecture-final.log`, `sol-native-safe.log`, `sol-native-repeat.log`, `sol-shrink-final-repeat.log`, `sol-full-validation.log`, `sol-final-integration.log`, `sol-extra-contracts.log`, `sol-index-focused.log` and `sol-extension.log`. Scratch is session-local, not a checked-in release artifact.

## Remaining failures and deferred acceptance

1. **Download engine-contract failures closed on 2026-10-09 at the fixture boundary.** Historical serial runs each passed 102/103:
   - `cancelling_during_ffmpeg_mux_removes_dash_staging_and_output`: `FFmpeg mux process did not start`; reproduced as earlier terminal Network/error-sending-request before mux after an empty socket read returned 10035.
   - `crash_window_bytes_ahead_of_sqlite_checkpoint_are_safely_overwritten_on_resume`: Protocol `HTTP probe returned status 409`; reproduced after lost HEAD/Range caused ordinary GET fallback to consume the fixture's one-time zero-offset eligibility, and the next probe fallback was rejected.
   The shared accepted socket now explicitly restores blocking mode with bounded IO; delayed-first-byte regression is red/green, one serial full run is 103/103 and ten fixed focused rounds pass. No repeated full-suite green hunting. HEAD source already has the fixture defect; no compiled HEAD baseline was run.
2. Real pressed-pointer move during probe, completion, release and DPI/client/viewport/form/inspector/footer lifecycle remains untested. Native temporary HWND and GPUI tests do not claim this acceptance.
3. Indexer release gates remain deferred: packaged LocalSystem `indexer-idle-cpu.ps1 -RestartService -AssertRegression`, persisted base+delta restart, actual service search and approved/runtime hash checks. Service operations were prohibited.
4. Protected-service admission now fails closed for unregistered development/Validation clients. Live compatibility has not been established; an explicit approved development contract would be needed rather than weakening authorization.
5. Installer exact-owner stop is source/ownership-tested only; no new installer compilation/execution. Registration rollback is synthetic-store tested only; no real registry injection. Native blocking calls remain subject to the fixed-worker limitation above.
6. External-CDN throughput sampling, real Shell drop/extension behavior and real user archive/storage migration are not verified.

Out-of-scope/unproven claims were not converted into speculative fixes: MFT volume-serial collision, HWND reuse, arbitrary blocking Shell extensions, CLI requested output as diagnostic leakage, and HLS Finalizing cancellation as already accepted. Earlier disproved review claims remain documented in the initial audit.

## Task and workflow state

The approved implementation and bounded automated verification/documentation pass are recorded. The engine-suite failure gate was closed by the 2026-10-09 fixture-only correction; overall quality/release acceptance remains incomplete because of deferred live/native/service gates. Phase 3.3 executable-contract updates were recorded; Phase 3.4 commit and archiving were explicitly not authorized. Keep the parent/children `in_progress`, do not reset unrelated changes, and do not treat a historical Validation installer as a new verified release.

## 2026-10-09 bounded engine-contract closeout

### Evidence and cause
- Initial isolated baselines and six fixed rounds passed; they were not treated as proof of repair. A new raw socket regression delays the first HTTP bytes after connect for both serial and concurrent server modes. Before the fix it failed in 0.06 seconds: client ConnectionAborted/10053, server WouldBlock/10035 with zero request bytes.
- Three ranked hypotheses were checked: inherited nonblocking stream, probe/one-time resume fixture state, and terminal task failure before FFmpeg marker. Temporary `DEBUG-ENGINE-CLOSEOUT` probes recorded only native/error state and synthetic request method/shape, never user payloads.
- FFmpeg original regression failed during bounded probing: accepted stream read 10035, task reached Failed/Network/error-sending-request before the marker; no FFmpeg mux was started. Added terminal-state assertion exposes this actual stage instead of waiting fifteen seconds. Existing cancel-responsive and no-staging/no-output assertions remain.
- Crash-window original regression reproduced on the third bounded run: two empty reads returned 10035; acceptance probe fallback GET without Range consumed one-time transfer eligibility, next worker probe fallback GET without Range saw started=true and received HTTP409. Production submit/probe/recovery policy was correct for these transport failures; no policy was weakened.
- `git show HEAD` confirms the same nonblocking listener and accepted stream lacking reset, plus the original tests and concurrent HEAD/Range probe already existed. This is source baseline evidence, not a compiled HEAD differential run or proof of every historical failure instance.

### Minimal change and validation
- Only test code changed: `tests/support/mod.rs::serve_connection` sets accepted TcpStream blocking before the existing two-second read timeout and a two-second write timeout; `http_fixture_contract.rs` adds delayed-first-byte response/body/request-count assertions for both modes; `engine_contract.rs` fails promptly if mux marker is absent after a terminal task state. Previous unrelated additions in these files were preserved. No product, URL/probe, retry, timeout, cancel or resume semantics changed.
- Reference rechecked: ABDM `HttpDownloadJob.fetchDownloadInfoAndValidate`, pause/stopAllParts; then old NxTools `DownloadManagerService.MergeDashAndVerifyAsync:3595-3682`. Existing product validation/stop/join/cancellation/publication boundaries remain unchanged.
- Green: fixture7; one serial engine_contract103; download-lib110; HTTP-client22; storage7; domain12; artifact4. Strict download Clippy --tests -D warnings, download check --tests, fmt and diff check pass. Ten fixed rounds each of delayed socket, FFmpeg cancellation and crash-window resume pass (30 invocations); no second full engine suite was run.
- All temporary DEBUG-ENGINE-CLOSEOUT probes removed. Logs: `engine-closeout-initial.log`, `engine-closeout-bounded-baseline.log`, `engine-closeout-delayed-socket-red.log`, `engine-closeout-socket-probe.log`, `engine-closeout-resume-probe.log`, `engine-closeout-focused-green.log`, `engine-closeout-validation.log`, `engine-closeout-stability.log`, `engine-closeout-additional-contracts.log` in session scratch.
- No package/install, product launch, real DOpus navigation, user clipboard/payload inspection, service operation, commit or archive. Real pointer/DPI and LocalSystem release acceptance stay pending. This closes the two approved test blockers only; no later UI/standards task was started.

## 2026-10-09 installed download recurrence supersedes earlier acceptance impressions

The installed fixed Validation package still clips confirmation/footer despite matching packaged hashes. Source-native corrections and the one directed review's delayed WINDOWPOS lifetime red/green are recorded in [the recurrence report](2026-10-09-download-window-recurrence.md). The follow-up is not packaged or live-accepted; standards delivery is not UI polish. That report also records an erroneous quoted test filter which actually ran unrelated real DOpus navigation, the disclosure and limited continued-repair authorization. Earlier no-package/no-DOpus statements above are dated historical evidence, not a blanket description of later turns.
