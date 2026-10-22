# Phase D — End-to-end operations and desktop launch intents: implementation outcomes

## Disposition and authority

Source/isolated implementation delivered; **independent acceptance pending, NOT overall Phase D
PASS**. The one final full Desktop run is **973 passed / 6 failed** (979 registered/run, zero
ignored). Archive/startup/Quick Find rows in that run passed, but those successes do not erase
six other fixture failures. No second full Desktop run was made. Source recycling and live
GUI/Shell/DPI/mixed-monitor/UIA/user acceptance remain unaccepted/NOT RUN.

The user's dependency amendment is retained in [implement.md](implement.md). Only
[phase-a-checkpoint.md](phase-a-checkpoint.md) owns current status. Phase A/B gaps and Phase C
live acceptance are not passed or waived. Cache fairness/font acceptance were not reopened.
No E/F, dependency/vendor upgrade, package/install/signing/system/registry/service mutation,
GUI launch workaround, source deletion, commit or push occurred in this slice.

Evidence directory (local session scratch):
`C:\Users\Administrator\.pi-desktop\scratch\163f435b-1ee5-4c07-8102-31a2c7441b9b\phase-d-operations-20261021-233711-4b58dee8`.
Names below are relative to that directory. Earlier failures/timeouts/skips remain evidence,
not rewritten as passes. One evidence-accounting exception is explicitly documented below.

## Delivered production chain

- Official locked Kit0.7.1 Form/Field/Input/Select/Checkbox/Dialog/Button/Progress; create680,
  extract560 starting widths, bounded scrolling body, fixed Medium32px footer, stable entities,
  masked private passwords and explicit discard confirmation on incompatible format switch.
  Public slots compose controls; no custom substitute, palette or vendor edit. Physical paint,
  keyboard/IME/DPI and user approval are NOT inferred from TestWindow.
- Real source/destination/save pickers, immutable capability-validated drafts seeded from actual
  ArchiveSettings. Cancel/close retires draft work/secrets without config/output mutation. Accepted
  submission alone may persist explicitly remembered nonsecret defaults; save failure does not
  rewrite job outcome. GZ/XZ reject directories/multiple sources unless separate independent
  single-file jobs are explicitly chosen; whole batches validate/admit together, never truncate.
- Typed ArchiveJob/ArchivePicker DTOs and original platform pipe. C# ArchiveJobSession translates
  to original ArchiveWorkerRunner, immutable source/helper/output admission and Windows Job.
  Public ArchiveRead remains read-only/password-free. No second codec/process executor.
- Strict FIFO max2running/max32queued; aggregate2GiB/per-worker1GiB, original default832MiB
  calculation. Cleanup-held slots and conflicting source/output serialization. Pending state
  precedes send; logical job/native attempt/receipt/session/generation/ticket stay distinct.
- All/selected extraction binds browser source digest/native identity before ordinal mapping.
  Original Here/Name/Smart plan remains; Rust chooses <=4096 unique Name/Smart subfolders and
  C# exclusively reserves them relative to held handles, never merges existing named folders.
  Empty archives produce no spurious destination.
- Original finite password/conflict/risk/readmission mechanisms. Known ZIP/7z/RAR/TAR/compound
  plans report shortage before directory/partial/publication, with actual native source binding.
  Original coordinator permits at most one fresh output-only admission after matching zero
  commits and exited cleanup. Work/metadata/native memory limits never increase. All extraction
  requests enter this coordinator; extension alone does not certify capability.
- Meaningful per-member ratios only: original >64MiB AND >1000x positive known packed bytes;
  independent7z blocks and nonsplit/nonsolidRAR use real packed metadata. Shared solid/split or
  unknown ratios remain unknown, TAR has no payload checksum claim. Standalone streams retain
  actual-byte enforcement, not guessed preflight escalation. Compressed-TAR discovery uses the
  existing verified anonymous spool bounded by max(output,metadata); overflow before known
  inner sizes is terminal ResourceLimit, not a shortage. This is an explicit finite limitation.
- Native destination writability/space observations use the held directory/volume, no probe file,
  elevation or guessed free space. Rust sends known plan bytes before unique/output effects;
  creation encoded size remains unknown. Missing directory/known low space asks current ticket;
  cancellation has zero output effects, approval does not raise budgets. Actual writes remain
  authoritative. Official Destination Dialog returns only current approval/cancel, no apply-all.
- Creation overwrite confirmation observes original native target identity; cancel preserves it,
  same-byte replacement refuses target_changed, original verified atomic-path publication retained.
  Post-create Test is a real new original attempt inside the same logical slot; failure/cancel
  preserves created-output facts. Old create receipts cannot cancel current Test.
- Coalesced <=10Hz progress, original cooperative pause/resume, running-time timeout excludes
  real paused/prompt waiting. Background/close does not cancel job; restore/terminal remains
  readable. One actual terminal/notification, native commits survive late cancel/error, taskbar
  cleared. Retired/terminal prompts close; duplicate/late replies cannot revive work. Cross-entity
  callbacks defer to the next application turn, including immediate completions.
- args_os launch intent parsing precedes ordinary single-instance return; bounded Rust same-user
  SID/session/PID/executable-authenticated route, max32pending/max256paths, finite readiness/receipt
  waits and bounded replay. Preserve protected/silent/background/browser modes and cold hidden/
  unfocused main policy. Handler receipt follows actual browser reuse/creation or draft/job
  admission, not scheduling. Quick Find intercepts ordinary recognized IndexedFile Open only,
  preserving existing usage/accepted-close/failure owner; Reveal/OpenWith/elevated/custom unchanged.

Reference sources: [archive note](../../../docs/reference-code/archive.md) records actual legacy
NxTools tracked search, Archiver args/main/view/core and SupaZip controller/native boundary;
locked sevenz/UnRAR metadata inspection is identified there. Bandizip evidence is public resources,
not proprietary source. No unsafe reference path-write/global-password/recursive-temp pattern copied.

## Acceptance rows — evidence level is part of the result

“Scoped PASS” means the named source/isolated automation only; it is not live/user or whole-phase
acceptance. Filtered/subset counts overlap the full run and must not be added as new tests.

| Case | Actual result and evidence | Remaining acceptance |
| --- | --- | --- |
| JOB01 | Scoped PASS: all/selected source binding, original plans, compound stem, empty output and native unique Name/Smart folders; Archive plan5/job2, production Archive tests and native selected/unique/empty fixtures. Target preflight four extract branches plus two create branches pass. | Real picker/selection/window interaction NOT RUN. Not every format x shape combination. |
| JOB02 | Scoped PASS: saved defaults -> real inputs -> request; hidden field omission, semantic levels, source metadata/exclusions, GZ/XZ rejection/separate jobs, actual ZIP/7z/TAR/TGZ/TXZ/GZ/XZ creation plus independent bytes/test in tiny native fixtures. | Real native form/IME/keyboard/layout/user acceptance NOT RUN; split/edit remains E. |
| JOB03 | Scoped PASS: original password/conflict/risk/output-budget and current Destination/CreateOverwrite tickets; cancel/stale/duplicate, source-bound finite retry, unknown overflow no retry. Production official Dialog regression reproduces terminal-retirement defect then passes. | Physical Escape/window/prompt/focus/native UI NOT RUN; no full format/prompt cross-product. |
| JOB04 | Scoped PASS: bounded FIFO/resource/conflict/duplicate/cleanup admission; real read/job shared global max2 and cancelled detached-preflight slot retained. | Application live shutdown/load interaction NOT RUN; no large/performance run. |
| JOB05 | Scoped PASS: original progress1000 sample/coalescing, pause/resume/prompt timeout native fixture; background/restore/terminal taskbar commands tested through production owner. | Visible taskbar/physical pause/focus/close NOT RUN; cooperative pause is not instantaneous codec interruption. |
| JOB06 | Scoped PASS: actual commit journal/warnings/cancel/failure, once-only terminal and existing NotificationCenter action owner; terminal taskbar cleanup. | Real notification interaction/Shell open/user acceptance NOT RUN. |
| JOB07 | PARTIAL, NOT full PASS: real native publication/change-target/cancel facts and production post-create Test failure preserve created archive/source. Deletion remains visibly off/unavailable. | Strict original-object Recycle Bin is UNACCEPTED/NOT RUN; SAFE08 not passed; no path/permanent-delete fallback. |
| IPC01 | Scoped PASS: strict symmetric bounded jobs/pickers/intents, versions/enums/options, redacted secret handling; final Protocol63, IPC38 and native strict-wire fixture. | No installed incompatible-host/release claim. |
| IPC02 | Scoped PASS: pending-before-send, original-generation cancellation under pressure, once-only receipt/terminal, disconnect unknown cleanup retains slot, old attempt/receipt suppression; native cleanup fixtures. | Real desktop/host disconnect lifecycle NOT RUN; not replaced by TestWindow. |
| IPC03 | Scoped PASS: picker DTO/cancel validation, real bounded STA adapter source, production old-operation/session/generation/once-only application regression. | Actual interactive accept/cancel/fail picker UI NOT RUN. |
| IPC04 | Scoped PASS: startup parser8, authenticated real local listener tests (within final IPC38), owner handoff/Quick Find production test; preserved modes/source hidden-main policy. | Cold/protected/silent/browser real process-window/flash verification NOT RUN. |
| IPC05 | Scoped PASS: bounded duplicate/replay/overflow/readiness failures; explicitly grouped path vector preserved, unrelated requests never timer merged, no password argv. | Actual Shell grouping/association/menu delivery belongs to F, NOT RUN/not claimed. |
| DIAG01 | Scoped PASS: original worker/native synthetic trace privacy fixture, accepted/started/state/one terminal, elapsed/native2 preservation and throwing sink unchanged result. IDs link logical job/native attempt/receipt; progress frames are not logged as byte ticks. | Real installed GUI trace/user/native acceptance NOT RUN. |

## Final commands and results

Every Cargo command used jobs1/incremental0/codegen-units256 in its own process environment and
checked the shared Cargo/rustc lane first. No unrelated builds were terminated. Build/check commands
are Debug verification, not packaging. All C# operations used reviewed `--fixtures` filters only.

| Evidence | Command / result |
| --- | --- |
| `98-final-protocol-ipc-core.log` | `cargo test --offline --locked -p nxtools-core -p nxtools-protocol -p nxtools-ipc --lib -- --test-threads=1`: exit0, Core70/IPC38/Protocol63 PASS. |
| `99-final-strict-touched.log` | touched Desktop/Core/Protocol/IPC `clippy --bins --tests -- -D warnings`: exit0. |
| `100-final-strict-isolated.log` | Archive `clippy --all-targets --features isolated-codecs -- -D warnings`: exit0. |
| `101-final-desktop-full.log` | ONE final `cargo test -p nxtools-desktop -- --test-threads=1` with offline/locked: exit101, **973 PASS/6 FAIL**,0ignored. Within that run Archive68/startup8/QuickFind1 PASS (overlapping subsets, not additional totals). |
| `106-unrelated-fixture-isolated-failure.log` | Exact first clipboard resize case re-run alone with backtrace: exit101,0/1. Establishes direct ClipboardPanel TestWindow missing WindowState/Root at paint, not an archive call chain; no unrelated fixture/vendor fix made. |
| `107-final-archive-representative.log` (second execution) | isolated-codecs Archive lib20, plan5, interaction4, job2, output-readmission3, worker-extraction9, worker-wire17: **60 PASS**,0fail/ignore. Not full archive integration/codec/performance matrix. |
| `108-final-feature-workspace-check.log` | final `cargo check --workspace --tests --offline --locked`: exit0. |
| `109-final-matching-isolated-worker.log` | after feature checks, `cargo build -p nxtools-archive --bin nxtools-archive-worker --features isolated-codecs --offline --locked`: exit0. |
| `115-final-native-build.log` | `dotnet build ...Tests.csproj --no-restore -v:q`: exit0,0warnings/errors. |
| `116-final-native-list.log`, `117-final-native-run.log/results.json` | Exact same three filters `archive job fixture`, `archive create overwrite fixture`, `archive destination fixture`: LIST=RUN=PASS13,skip/fail/not_run0; identities compared exactly. Explicit worker env; SHA256 before/after `6552BAFBF393448718A52A60B506444F2253BC998CF9ECBEA071580D7E5D34FC`. |
| `96-final-fmt-check.log`, `118-source-doc-results.json` | fmt/source/document gates exit0: desktop-shell-lifecycle, font regression16/16, UI consistency/token boundaries, PlatformHost selection, spec-contracts, Kit catalog and task context. Known oversized context injection warnings remain warnings; relevant source was explicitly read. |

Full Desktop failures retained, all at `vendor/gpui-component-071/src/root.rs:126` with missing
component WindowState/Root during direct ClipboardPanel fixture paint (global init alone is present):
- `clipboard::resize_tests::capture_pause_ack_failure_reconnect_and_inflight_capture_cleanup`
- `clipboard::resize_tests::clipboard_resize_ignores_hidden_startup_and_retired_panel_sessions`
- `clipboard::resize_tests::clipboard_resize_observer_persists_latest_only_and_reopens_logically_at_125_150`
- `clipboard::resize_tests::clipboard_resize_save_failure_keeps_session_size_and_visible_error`
- `ui::extension_feedback::tests::explicit_result_location_uses_real_worker_page_and_ignores_closed_origin`
- `ui::tests::round2_active_clipboard_preview_does_not_close_on_conflicting_foreground`

They are a genuine full-suite acceptance blocker, not a product regression proven by this record.
No claim of baseline PASS/causality beyond the observed call boundary is made. Preserve their
unrelated work; separate owner repair/verification is needed before a full-suite PASS claim.

## Red/green and evidence caveats

- Real defaults/config10, missing create action, FIFO events, original-budget blocking watchdog,
  post-create Test and native unique-directory red/green remain in earlier numbered evidence.
  `21-budget-red.*` records owned-test watchdog termination, not a passed cancellation test.
- Native destination red58–69 includes protocol/native5/native123 failures;70–72 passes missing
  cancel after stable native-path fix.73–75 then reintroduces known-size assertion red. Matched
  worker83–84 passes four extraction branches;93–94 adds and passes two creation branches.
- Official Dialog76 first failed compilation (not behavior evidence);77 is real red for retained
  terminal prompt;78 same production regression PASS. New native readmission/risk fixtures pass
  ZIP plus representative non-ZIP sources; lowered Debug thresholds test transport, not huge data.
- Archive79 fixture compile inference error ->80 contract14PASS;88 redundant closure ->89 strictPASS.
  Final Archive lib first run was19PASS/1FAIL: encrypted browser test used general default budgets
  above existing512MiB browser caps. Only fixture limits were corrected; validator unchanged.
- **Evidence-recording mistake:** attempted scratch-script filename renumber Edit failed with
  EDIT_RANGE_INVALID. Rerun consequently overwrote `107-final-archive-representative.log` and its
  results JSON. These now describe the SECOND successful run. Original complete first log is not
  retained; observed exact failure-tail transcript is preserved in
  `107-initial-library-failure-preserved-transcript.txt` and conversation tool output. This is an
  evidence limitation, not a pretend untouched original. All other listed historical red/full-suite
  failures remain. An initial PowerShell metadata-heavy counts file110 is superseded by clean111.
- Earlier native11 selection with2SKIP is retained; explicit worker later11/11, then preflight12/12,
  final13/13 are separate states. No zero-selection/skip/wrapper0 is presented as behavior PASS.
- Optional bounded reviewer was stopped without a report; **no review PASS** and no new review loop.

## Preservation and stop

Opening task baseline154 files: before final result/checkpoint edits,152 byte-identical; only
user-authorized implement.md and sole checkpoint changed. New contract/outcomes are additive.
Historical Phase A/B/C reports and font/cache independent acceptances are not rewritten.
Workspace remains heavily dirty; no reset/staging/commit and no unrelated repair. Final scoped
whitespace, historical hash and doc gates are recorded in the final evidence addendum.

Stop at **independent Phase D acceptance** with full-suite failure, source-recycle and live/native/
user gaps explicit. Do not proceed to E/F or infer first-release completion from compiled fixtures.

### Final evidence addendum
`119-final-format-check.log` and `120-final-fixture-strict.log`: exit0 after the fixture-only
browser-cap correction. `121-outcome-doc-results.json`: spec/catalog/task-context all exit0.
After that last isolated feature lint, `122-last-feature-matching-worker.log` rebuilt the same
worker; `123-last-native-build.log` again0warnings/errors. `124-last-native-list.log` ->
`125-last-native-run.log/results.json`: exact same13names/PASS13,skip/fail/not_run0,unchanged
worker SHA256 as above. `126-final-history-preservation.json`:152/154opening files unchanged;
only implement.md/current checkpoint differ. Workspace git diff --check exit0 (line-ending
warnings only). No product behavior changed after the full Desktop run; only the Archive
unit fixture's explicit existing browser ceilings changed. The full Desktop973/6 remains final.
