# Download probe-to-confirmation repair and whole-product feature review

Date: 2026-10-08. Scope: the **current working tree**, including existing uncommitted and untracked source, not merely a branch diff. This is a source-backed functional review, not a certification that every Windows interaction works.

> Historical review snapshot. The approved SOL remediation and current R01–R13 status are recorded in [SOL review remediation](2026-10-08-sol-review-remediation.md). Do not read the outstanding ledger below as the current implementation state. Download engine-contract failures and live/service acceptance remain open; this original evidence is retained unchanged.

## Executive summary

The reported download-window defect has two concrete native failures, reproduced at the production Win32 boundary, plus two presentation hazards:

1. Four prefixed C# P/Invoke declarations did not specify their real Win32 exports. The actual client-correction branch threw `EntryPointNotFoundException` instead of correcting the window.
2. Visible-center correction could change the client after the resize/replay had already been classified as successful. Recovery could retire with the final client still at the compact height.
3. Rust also scheduled GPUI `window.resize` for the same real HWND, despite the documented single native resize owner.
4. The presentation gate released after ten seconds even when the viewport was still too small.

The first two defects were reproduced as consecutive failures of the same real HWND regression before correction. They are repaired; the custom-client native test then passed, including 20 consecutive repetitions. The production GPUI view test now holds confirmation metadata at compact size and asserts the inspector is withheld until the viewport is expanded. The ten-second gate no longer makes clipped content visible.

**Acceptance status:** native boundary and GPUI presentation regressions are green. A fresh full-product, real-pointer drag/probe/confirmation interaction at the user's DPI has **not** been run in this session. No installer/package was built or installed, no product was launched, and no Indexer service was stopped, restarted, installed or repaired. No real Directory Opus navigation harness was run.

## 1. Authority, invariants and repair boundary

### References actually inspected

- `docs/reference-code/download.md`, especially “Phase 1A new-download presentation invariants”.
- `D:\Code\NxTools\Views\AddDownloadWindow.axaml.cs`: constructor, `OnOpened`/`OnClosed`, view-model subscriptions, `QueueContentSizeRefresh`, `RefreshContentSize`, `AddDownloadWindow_SizeChanged`, `BeginMoveDrag`, close/disposal.
- AB Download Manager's `AddDownloadPage.kt` and the checked-in download reference index. The duplicate-options dialog's `rememberDialogState` is **not** evidence for the main add-window lifecycle; the product flow is referenced separately in the index.
- GPUI 0.2.2 Windows `events.rs`: `WM_SIZE`, `WM_NCCALCSIZE`, the move/size loop and renderer/viewport update. GPUI `window.rs::resize` schedules native `SetWindowPos`; it is not a purely logical viewport setter.
- `.trellis/tasks/10-02-download-window-lifecycle/{prd,design}.md` and the canonical `.trellis/tasks/09-28-download-window-transition-polish/implement.md`.
- Desktop quality guidelines and shared cross-layer guide. There is no PlatformHost-specific spec index in this checkout; `AGENTS.md` and the reference/task contracts govern the C# boundary. Some package spec indexes remain templates, so their existence was not treated as substantive compliance evidence.

### Preserved contracts

- Compact and confirmed layout remain 560×176 and 560×278 logical pixels; no oversized-window workaround or visual redesign.
- Rust owns layout/state; C# owns native Windows calls; GPUI renders and learns native client size from `WM_SIZE`.
- After a real HWND exists there is one native resize owner, not competing GPUI/C# `SetWindowPos` requests.
- A dragged window retains its live user-moved center. Native recovery is bounded and generation/correlation scoped.
- Confirmation controls, inspector and footer are not rendered into an insufficient viewport.
- Enter, Download and Queue cannot submit while confirmation is withheld.
- Window close remains presentation-only; closed/stale generations must not receive late work.
- Timeout surfaces a retryable presentation error rather than silently declaring success or showing a clipped form.
- Diagnostics record IDs, HWND, dimensions, state, elapsed time and native failure metadata, never download payloads.

### Fresh runtime evidence

The latest available PlatformHost trace for one operation requested a 278px client, read back 176px after application, then emitted `download.window_resize_recovered` with `success=true` and `final_client_height=176`. This demonstrates that a recovery success event was not sufficient proof of success. No URL, file name, headers or cookie values were read for this investigation.

## 2. Repair, reproduction and verification

### Red-capable tests actually run before the fix

Command:

```powershell
dotnet run --project .\csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj -c Debug -- 'centered resize checks client after position correction'
```

First result:

```text
FAIL ... Unable to find an entry point named 'ClientResizeGetWindowRect' in DLL 'user32.dll'.
```

After fixing only the Win32 export bindings, the same test failed again:

```text
FAIL ... resize reported success after center correction restored compact client: 176
```

The test creates a real Win32 custom-client-chrome HWND, handles `WM_NCCALCSIZE`, and injects compact-height restoration at the position-only correction boundary. It invokes the production centered resize and recovery functions, then reads the real final client rectangle. This is stronger than a source-string test or an ordinary `#32770` window. It models the observed boundary race; it is **not** a claim that the user's physical pointer sequence was executed.

Rust regression was also run red before its fix:

```text
confirmed_form_is_withheld_until_the_native_viewport_reaches_its_height
assertion failed: confirmed_window_pending(target, true, px(176.0))
```

### Files changed for this repair

- `WindowsPlatformAdapter.ClientResize.cs`: real `EntryPoint` names for `GetWindowRect`, `GetClientRect`, `ClientToScreen`, `SetWindowPos`.
- `WindowsPlatformAdapter.cs`: center correction precedes final client correction; mismatch cannot be reported as successful completion. Replay passes final correction to the recovery owner before terminal readback.
- `WindowsPlatformAdapter.NativeMethods.cs`: final client correction runs before stability/pending retirement; snapshot carries its own requested dimensions.
- `CenteredResizeRecovery.cs`: dimensions added to native pending snapshot, without changing IPC.
- `WindowsPlatformAdapter.EventWindow.cs`: successful recovery reason is `client_size_settled`, not `replay_not_applied`.
- `new_download_view.rs`: real HWND no longer also schedules GPUI resize; timeout checks the current generation/viewport, clears unpresentable confirmation and reserves the existing error row for retry.
- C# and Rust download tests: custom-frame/native readback and compact-viewport confirmation withholding.
- Download reference index: API/export and final-readback invariants recorded.

### Verification actually completed

| Check | Result / limit |
|---|---|
| Custom-client native correction/recovery | Green after both fixes; 20/20 consecutive runs passed |
| Existing moved-center and move-end native tests | Passed |
| Production GPUI probe/form/inspector/footer test | Passed; now stages metadata while viewport remains compact before simulating native size |
| Rust desktop suite | 434 passed, 0 failed |
| Workspace `cargo check --workspace --tests` | Passed, no warnings observed |
| `cargo fmt --all -- --check` | Passed |
| C# Debug test-project build | 0 warnings, 0 errors |
| Safe C# suite | 181 PASS labels, 0 FAIL; two of those report explicit SKIP for absent interactive Files/Explorer windows. The two real DOpus tests were excluded entirely |
| Download, clipboard command/UI, Quick Launch, desktop-shell, file-dialog-search, UI-token, package-memory architecture gates | Passed |
| `git diff --check` | Passed |
| Fresh packaged GUI at real DPI, held pointer drag, release and probe completion | Not executed; remains required for live acceptance |

Every Cargo invocation used `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`. No Release package or service regression gate is claimed.

## 3. Review method and confidence

Three non-overlapping read-only review slices inspected current feature implementations and reference/spec contracts. The parent then checked disputed findings against current callers, state guards and tests, and separately inspected Finder query, custom-command, calculator and environment-variable providers. This was one review pass, not recursive agent debate.

Evidence labels:

- **Reproduced:** a command exercised the production boundary and failed, then passed after repair.
- **Static-confirmed:** a concrete reachable branch or violated contract follows from source. No interactive reproduction is implied.
- **Validation gap:** more evidence is required; not a proven failure.
- **Rejected/reframed:** reviewer claim was contradicted by guards/callers or overstated.

P1 means high-impact data/security/availability risk; P2 means concrete user behavior, reliability or correctness defect; P3 means narrower diagnostics/tooling/maintenance issue. This report does not use P0 for ordinary behavior defects.

## 4. Confirmed outstanding findings

These are review results, **not additional repairs made in this task**.

### R01 — P1: Indexer ordinary sessions are not authorized beyond the pipe ACL

Evidence: `rust/crates/nxtools-ipc/src/local_pipe.rs:38-40` grants read/write/synchronize to Authenticated Users. `rust/apps/nxtools-indexer/src/windows_service_host.rs:81-125` records PID/session, but does not gate ordinary sessions. `elevated_helper_broker.rs:49-81` authenticates the special elevated-helper command only. `lib.rs:834-855,1736-1772` accepts Search/UpdateSettings through the ordinary handler.

Trigger: another permitted local authenticated-user process connects and holds a session or issues ordinary commands. Effect: cross-user service metadata/search exposure and settings mutation risk; sessions also hold the shared service mutex until disconnect, so a stalled client can block legitimate users. This is not a claim that an elevated-helper launch is unauthenticated; that command has a separate packaged-client gate.

Recommendation: authorize read and write commands explicitly against the intended user/session/product policy before acquiring a long-lived service session; add unauthorized-client and held-session tests. No service manipulation or cross-user exploitation was performed.

### R02 — P2: Adaptive segment shrinking branch is unreachable

Evidence: `rust/crates/nxtools-download/src/engine/segmented_runner.rs:1022-1036`. When `shrink_requested` is true, `pending_shrink` is cleared at 1024. `skip_shrink` then starts with `pending_shrink.is_none()`, which is necessarily true. Thus `if shrink_requested && !skip_shrink` never executes for the requested worker.

Effect: the controller may request a shrink, but the waiting-range merge/decrement path cannot occur; the generic rebalanced result path may instead split again. Recommendation: preserve the requested-operation identity until classification and add an actual slow-transfer→shrink→persisted-plan test, not merely a controller helper test.

### R03 — P2: DASH resume adopts a changed nonempty fragment solely by length

Evidence: `engine/dash_runner.rs:227-248` accepts any nonempty `.fragment`, even if its length differs from the completed checkpoint; it overwrites the checkpoint to the new filesystem length. HLS adoption is more conservative for completed checkpoints (`hls_runner.rs:286-307` checks the recorded length), but neither path stores a per-fragment byte digest.

Trigger: a task-owned DASH fragment is truncated/replaced yet remains nonempty, or resource identity changes behind compatible manifest metadata. Effect: resumed output can use incorrect bytes without re-downloading that fragment. Static-confirmed adoption rule; no corrupted-media execution was run. Recommendation: reject a mismatch with a completed checkpoint, and define integrity/identity evidence for crash-window adoption. Signed-URL refresh alone is not a byte-identity proof.

### R04 — P2: Copy-path/copy-command actions are hidden before KeepOpen policy runs

Evidence: `quick_launch.rs:3122-3143` calls `hide_popup` on every accepted platform action. `actions.rs:135-136` classifies CopyPath/CopyCommand as `KeepOpen`. `panel.rs:221-230` marks the panel closed, clears action mode and collapses it. The later `KeepOpen` handler (`quick_launch.rs:1804-1813`) sets status/focus but does not undo the earlier hide.

Trigger: copy path/command from the actual action-mode dispatch. Effect: the supposedly non-terminal action dismisses the launcher. Existing `copy_path_success_keeps_the_launcher_open_without_recording_a_launch` checks the policy helper, not this end-to-end dispatch. Recommendation: choose hide behavior from action policy before dispatch and test native result plus visible session state.

### R05 — P2: Initial index build failure advertises Ready without any published snapshot

Evidence: `nxtools-indexer/src/lib.rs:529-537` and `build_snapshots:1470-1487` use Ready even if all first-build volumes fail. Desktop `quick_launch/search.rs:340-352` treats Ready as searchable and retries based on a transition to Ready.

Effect: empty/unpublished index can be called searchable; later real readiness may lack the expected retry edge. This differs from legitimate maintenance with a last-good snapshot, which should remain searchable. Recommendation: distinguish no-snapshot failure from last-good maintenance and assert both cases. This is not a service-release acceptance claim.

### R06 — P2: Clipboard purge minimum differs from the verified reference

Evidence: `nxtools-clipboard/src/lib.rs:1619-1623` uses `retention_days.max(30)`. `docs/reference-code/clipboard.md:89-90` records `max(7, cleanupDays)` and explicitly says thirty is only the default, not a hard minimum.

Trigger: retention configured below thirty days. Effect: deleted rows/assets remain longer than specified. Recommendation: reference-driven seven-day floor with tests at 1,7,14,30 days, preserving favorite/pinned semantics. No deletion of real history was performed.

### R07 — P2: Archive import can retain orphan assets and partially imported state on error

Evidence: `nxtools-clipboard/src/archive.rs:272-343` extracts assets before rejecting/skipping an invalid kind. `extract_asset:404-428` has no scope cleanup on interrupted extraction; items are captured individually before a later item can fail.

Effect: malformed input can leave unreferenced assets or `.import-*.tmp`, and a late error can leave partial database imports without a terminal report that accurately describes them. Traversal and size guards exist and were not ignored. Recommendation: prevalidate manifest references/types, stage with cleanup guards, and define transaction/partial-import semantics with malformed-archive tests.

### R08 — P2: Storage migration can switch the worker before new config is durable

Evidence: `clipboard_worker.rs:629-649` replaces live history/asset root. `clipboard.rs` persists the destination only after worker migration; failure reports that the worker has already switched and leaves old config unchanged.

Trigger: migration succeeds but config update fails (corrupt/missing/denied config). Effect: session uses the new store while restart opens the old store; later captures split across roots. Recommendation: explicit prepare/commit/rollback or durable migration receipt/recovery. No real storage migration was run.

### R09 — P2: Uninstaller stops a service by name without checking ImagePath

Evidence: `installer/nxtools-next.iss:70-72` unconditionally runs `sc stop NxToolsIndexerV1`; unlike `scripts/package.ps1`, no exact registered-executable comparison is made.

Effect: an unrelated service using the same name could be stopped; uninstall behavior is not limited to the installation's approved indexer. Recommendation: exact registered path/ownership validation at uninstall, with script tests. Installer/uninstaller was not run.

### R10 — P3: Native drag cancellation is indistinguishable from completed drag in the result

Evidence: `NativeFileDragService.cs:166-168` maps both `DRAGDROP_DROP` and `DRAGDROP_CANCEL` to true. `NativeFileDragCoordinator.cs:225-235` propagates that as success. Escape is correctly detected by QueryContinueDrag, but its terminal meaning is lost.

Effect: telemetry/consumer cannot distinguish user cancellation from an actual drop (drop effect zero gives only indirect evidence). Recommendation: model cancellation explicitly rather than turning a normal user cancel into an error toast. No live OLE drop-target interaction was performed.

### R11 — P3: USN duplicate candidates inflate the bounded rebuild budget

Evidence: `usn.rs:171-174` appends each upsert reference without deduplicating. `bounded_visible_child_candidates:391-407` checks raw vector length before `refresh_descendant_paths:281-283` deduplicates via visited.

Trigger: many duplicate upserts under a directory in one batch. Effect: avoidable bounded RebuildRequired despite a small unique subtree. It is maintenance/performance, not a user-visible failure or proven data-loss path. Recommendation: bound/count unique candidates and add duplicate-heavy batch tests.

### R12 — P3: Payload-bearing fields remain in some diagnostics

Examples verified in current source: `quick_launch.rs:2052` logs the full query on an error; Auto Quick Switch enqueue failure logs `source_directory` around 1319-1325; icon failure includes a resource key that can contain a target path. Source contains the unsafe log fields, but no actual secret-bearing log line was generated for this review.

Recommendation: bounded lengths/kinds, IDs and native codes instead of query/path/resource text; test diagnostic schemas. Do not reinterpret CLI's intentionally user-requested task paths as diagnostic leakage without a separate output contract.

### R13 — P3: Two architecture gates cannot currently validate their intended implementation

- `file-dialog-window-follow.ps1:7-23` scans only `WindowsPlatformAdapter.cs`; geometry message and Follow call now reside in `WindowsPlatformAdapter.EventWindow.cs:30,463,2039,2120`. The gate fails despite the expected implementation being present.
- `browser-native-messaging.ps1` fails under Windows PowerShell 5.1 at `SHA256.HashData`. PowerShell 7 gets further, then fails at line 168 because its searched desktop open-window method has moved to another included file (negative startIndex).

Recommendation: read the owning partial/included files and fail with explicit missing-entry messages rather than unsupported APIs or negative indexes. These are validation failures, not proof the two product features are broken.

## 5. Validation gaps and reframed review claims

- **Blob same-capability simultaneous writes — rejected:** `claim_upload:430-455` only permits Pending/Uploading and atomically changes to Receiving under the registry mutex. A second claim in Receiving is rejected. A dedicated duplicate-in-flight regression would still be useful, but the reported write race is not established.
- **Clipboard preparation nested entity lease — rejected for cited path:** `ui/mod.rs:1585-1608` completes panel.update, then calls application.update separately. The report incorrectly treated them as nested.
- **Quick Launch cache-hit reentry — not confirmed:** panel.run_search does synchronously call the application, but `request_icons:2933-2940` defers cached icon writes back to the panel. No concrete reentrant callback chain was demonstrated. Keep the architecture risk/test gap, not a claimed crash.
- **HLS cancellation accepted during merge — rejected as stated:** ordinary HLS Finalizing is not an accepted cancel state; only DASH Finalizing has a cancel arm (`engine.rs:3476-3487`). A cancellation-at-transition race needs a specific interleaving test before it is reported as accepted cancellation lost during merge.
- **All failed Quick Launch actions permanently lose retry — overstated:** the native-failure path starts activation and focuses the panel. Action-mode restoration after hide remains a validation gap; R04 is the narrower verified unconditional-copy-hide defect.
- **Windows fs::rename cannot overwrite config — rejected as blanket claim:** the existing-file save test ran on this Windows workstation and passed. Concurrent readers/AV, directory durability and bounded locking remain useful fault-injection gaps, not evidence that ordinary saves always fail.
- **Extension URLs already logged by desktop — rejected as stated:** BrowserHost deserializes only event/correlation and logs those fields (`lib.rs:410-425`). Extra URL details are not logged by that boundary. Removing needless detail transmission is defense-in-depth, not a proven desktop log leak.
- **CLI safe JSON paths/URL output — not listed as defect:** CLI explicitly shows task identity/targets to the requesting user while omitting credentials/header material. A stricter output/privacy policy must be defined before calling user-requested paths an implementation bug.
- **Protected tuple absent — reframed:** `main.rs:602-620` checks Desktop/PlatformHost/Indexer equality before redirecting to protected copy. However `runtime.rs::platform_host_path` selects an unverified sibling or repository Debug fallback when not redirected. This is a trust-hardening/explicit-dev-mode gap, not a demonstrated elevation exploit.
- **MFT cache volume identity — validation gap:** mount/journal identities are checked; no stored volume serial. Replacement-drive-letter rejection should be tested, but a journal-ID collision was not demonstrated.
- **HWND reuse/navigation — validation gap:** third-party registry checks host-context ID, HWND and CanNavigate; captured PID is not consistently rechecked. File-dialog navigation has similar lifetime identity concerns. No real handle reuse was induced.
- **Detached navigation/COM cancellation — limitation:** session cancellation stops result acceptance and caches, but cannot abort arbitrary in-flight third-party COM calls. Need bounded admission/resource tests and adapter-level late-mutation checks; no hostile Shell extension was exercised.
- **Download IPC admission — hardening gap:** same-user per-client threads/read waits lack a demonstrated connection flood cap in the reviewed path. No local denial-of-service was run.

## 6. Per-feature coverage ledger

“Reviewed” below means entry→state→worker/native→terminal/cleanup source tracing and test inspection. It does not mean the interactive endpoint was run.

| Feature | Owners / call chain examined | Validation and remaining live boundary |
|---|---|---|
| Add URL, probe, confirm, Enter/Queue/Download | new_download_view → engine probe → native centered resize → WM_SIZE → GPUI view | Repaired native and presentation regressions; real pointer/DPI acceptance outstanding |
| Browser prefilled / staged Blob add form | browser_handoff_impl → add-window creation → view → submit/handoff | Desktop tests pass; actual browser foreground launch not run |
| Progress/failure/completed windows, retry/close/handoff | task_windows_impl, task_window, download_window_lifecycle, native placement/visibility | Desktop/native suites pass; live retry/completion smoke outstanding |
| Task list filters/sort/multiselect/detail/icons | main_window/download_page, downloads.rs, icon boundary | Desktop suite; real dataset/large-list performance not run |
| HTTP request context/probe/Range/resume | request_policy → HttpDownloadClient → segmented_runner → artifact/storage | Download lib suite; localhost integration contracts not rerun here |
| Adaptive segments/retry/rate-limit | segmented_runner, controller, rate_limit | Source-confirmed R02; controller tests alone do not exercise shrink branch |
| HLS/AES-128/manifest refresh/staging/merge | hls parser → hls_runner → http/decrypt → durable checkpoint → merge | Lib tests; identity/tamper/transition races remain gaps |
| DASH tracks/fragments/FFmpeg mux | dash parser → dash_runner → media → ffmpeg | Lib tests; R03 fragment adoption; no external FFmpeg process run |
| Queue/manual/delayed/daily schedule/idempotency | engine admission/scheduler → SQLite transactions/receipts → wake controls | Download lib tests; real clock/restart/network soak not run |
| Download deletion/artifact ownership/cleanup | engine remove/retry → ownership checks → storage transactions | Tests inspected/run in lib; no real user file deletion |
| Taskbar overlay, completion sound, shutdown countdown | completion_actions / taskbar overlay → typed C# adapter | Desktop and C# tests; no real shutdown or completion action executed |
| FFmpeg automatic install/cancel/path | ffmpeg_install → checksum/staging/archive → media validation → settings | Download lib; no installation/network fetch |
| Browser interception/capture/request authority | extension policies/background/page/content → native host → download IPC | Node 21/21; actual browsers not launched |
| Batch/Blob handoff/replay | handoff policy → browser-host/blob_upload → durable engine receipts | Browser lib27 + download IPC6; duplicate Receiving guard verified; no real upload |
| Extension popup/media overlay/export package | popup CSS/JS/semantic contract; C# CRX/XPI package service | Node and safe C# tests; visual/accessibility actual browser proof outstanding |
| CLI add/list/show/action/watch | CLI parser → DownloadIpcClient → engine projections | CLI lib10; no persistent command session launched |
| Clipboard text/image/files/native formats/limits/exclusions | EventWindow capture → worker → history/assets → panel | Clipboard39 and desktop/C# suites; no real clipboard payload read/paste |
| Clipboard four action routes/confirmation/suppression | panel action preparation → PlatformHost paste coordinator → result → usage | Desktop/C# tests; no real target-app paste |
| Clipboard popup/passive preview/focus/close | clipboard.rs/ui preview → one HWND → staging/bounds/topmost/hide | Desktop tests; real hover/focus at DPI not run |
| Search/categories/virtual paging/selection/delete/restore/pin | UI generations/cursor window → worker queue → SQLite | Tests; retained-memory/CPU soak not run |
| Clipboard retention/import/export/compact/migrate | history cleanup + archive + worker migration/config | R06/R07/R08; no real data maintenance operations |
| Link detector/panel/local file/web routing | detector → link_panel → copy/Shell → suppression/auto-hide | Desktop tests; live URL/browser interaction not run |
| Shared blacklist / individual feature toggles | AppConfig normalize → clipboard/quick-launch checks → deferred settings sync | Core39/desktop tests; no actual excluded process exercised |
| Quick Launch hotkey/double Ctrl/fullscreen | C# trigger binding/gate → foreground snapshot → Rust popup | C# tests; actual exclusive D3D/hotkey hook not exercised |
| Finder catalog/fuzzy/pinyin/query/path/exclusion/ranking/history | FinderCatalog.search_with_sources / FileQuery / pinyin / SQLite usage | Parent inspected provider code; Finder28; real million-row scale not run |
| Calculator/base/Chinese-number conversions | calculator bounded parser → output representations → copy action | Parent inspected bounds/nonfinite checks; Finder tests; no UI numerical corpus beyond tests |
| Environment/custom commands | variable parser/target cap; keyword/template/argument tokenizer → typed launch | Parent reviewed providers; quoted Windows backslash handling deserves additional realistic-path tests |
| Launcher action mode/copy/Shell/recent directories | panel→actions→execute→native result/session→restore | R04; desktop suite does not lock down pre-result visibility policy |
| Launcher icons/passive preview/generation/cached results | request_icons deferred cache path → native icon STA/cache → panel; preview staging | Cache reentry claim not proven; real HWND keyboard/hover proof outstanding |
| Index service/search/latest generation/admission | pipe ACL→service session→search worker→batched results/settings | Indexer63 (1 ignored); R01/R05; no LocalSystem service smoke |
| MFT base/cache/delta/restart | raw enumeration→mapped base/delta→snapshot publication/compaction | Source/test review; no real NTFS scan or idle CPU gate |
| USN create/rename/move/delete/rebuild budgeting | journal bounds→overlay→descendant refresh→persisted delta | R11; library tests; no actual journal churn soak |
| Explorer/Desktop navigation/Shell open/reveal | capture HWND/context→native navigation→foreground/readback | Safe C# fixtures; real Explorer test skipped |
| DOpus active pane/passive refresh/dialog return | capture session→source dirty refresh→known dialog→WM_COPYDATA/readback | Synthetic tests; real DOpus tests excluded, multi-pane live acceptance outstanding |
| Files/OneCommander/Total Commander | adapter registry→captured pane→UIA/message navigation→readback | Source/synthetic tests; Files interactive skipped; no live manager navigation |
| File-dialog recognition/search hosting/follow | native recognizer→coalesced geometry message→GPUI bar→navigate | Source/synthetic tests; stale follow architecture gate documented in R13 |
| Shell context menus/OLE drag/STA cancellation | lazy Shell sessions→bounded STA workers→result/drain | C# tests; R10; no real OLE drag/drop or blocking third-party extension |
| Protocol/IPC/bounded queue/result/disconnect | typed Envelope→frame limit→command dispatcher→correlated result→drain | IPC21/protocol39/C# tests; held-client admission gap remains |
| Global/network/config persistence | locked section update→atomic save→runtime policy generation→native shell ack | Core39/network2/desktop tests; no real user config modified |
| Tray/startup/UAC/protected release/install/package | main protected tuple→supervisor→trusted C# paths; package exact-path checks; installer | Source/safe C# tests; R09; no install/UAC/service/package operation |

## 7. Recommended follow-up order

1. Verify this window patch in a fresh authorized build with the user's exact held-drag/release sequence and 100/125/150% DPI. Require `client_size_settled` with actual target dimensions; failure must remain actionable, never clipped.
2. Fix R01 before broad service deployment; validate under LocalSystem without broadening access. Any indexer change requires the documented idle-CPU/restart/search release gates, which were not run here.
3. Add integrated regressions and fix R02/R03 (adaptive plan and media integrity).
4. Fix R04 and clipboard retention/import/migration state contracts (R06-R08).
5. Correct uninstall ownership checks (R09), drag cancellation status (R10), diagnostics (R12) and stale validation scripts (R13).
6. Treat trust hardening, handle reuse and unabortable COM cases as explicit design/test work, not speculative emergency blockers.

No commit, task archival, installer execution or service mutation is implied by this document. Existing migration tasks remain in progress until live interaction acceptance.
