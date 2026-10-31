# Archive feedback / HZ2 repair and non-ZIP common foundation — PARTIAL acceptance

2026-10-28. This is the user-authorized bounded repair round, not a new A–F phase, continuation replan, release or installed-product acceptance. Current status belongs only to the new top of `phase-a-checkpoint.md`. Original independent PARTIAL reports and raw failures are unchanged.

## Itemized outcome

| Item | Implementation / verified outcome | Remaining limit |
|---|---|---|
| **FB-AC01** | **CLOSED at production presentation / automated boundary.** Original JobProgressView uses monotonic executor time; original RateSamples expires old samples without new worker events. Visible active window refresh is bounded100ms; hidden/occluded/background/terminal/retired stops it. Original Progress-event observation time is retained separately so UI-only refresh cannot renew stale samples. No fabricated bytes, events or heartbeat/global idle timer. | Physical/native-window visibility, real disk stall and user visual acceptance NOTRUN. |
| **FB-AC02** | **CLOSED at original application→IPC / presentation boundary.** Original JobRecord correlates receipt/action/logical job/attempt/generation/connection before dispatch. Not-submitted/refused/accepted-awaiting-state/real State/receipt-timeout/state-timeout/disconnect/terminal are distinct. Failed waits clear and show explicit retry; no automatic replay or native-slot release. Pause pending still allows Cancel; original reserved cancellation and returned deduplicated receipt retained. | Controlled PlatformBridge tests are not a real named-pipe saturation or installed worker-control acceptance claim. |
| **FB-AC03** | **CLOSED at real worker→native terminal and Desktop consumer boundary.** Actual completed held/verified directory output contributes result location even with zero file commits. Preparation root/wrapper/unfinished file ancestry alone does not.13 Here/Name/Smart sole/multiple/nested/collision cases, six zero-output cases, real failed/cancelled/unknown guards.22 real terminal transcripts consumed by original Desktop event/location policy; success-only/cleanup/pref/duplicate/manual behavior retained. | Replay crosses the exact typed terminal DTO, not a concurrently launched real Desktop pipe/Explorer. Actual Explorer/focus NOTRUN. |
| **FB-AC04** | **CLOSED.** Exact18 reported paths formatted; final scoped `rustfmt --check --edition2024 --config skip_children=true` exit0 and scoped diff check exit0. No lint exemption or unrelated network/download formatting. | Whole-repository formatting was not run or claimed. |
| **FB-AC05** | **OPEN.** No current full-window/annotated image exists in this turn's supplied scratch attachments. Historical task assets are the old Bandizip layout and icon proofs, not an identification of the reported two lines. Full-window marked screenshot requested. No guessed offsets/borders. | Target lines/common parent and requested preview/scrollbar/min/default/Light/Dark/scale geometry matrix cannot be accepted until supplied. |
| **Related HZ2 boundaries** | Production no-event expiry/elapsed, hidden→visible retirement, UI-only refresh, wait/pause/attempt/stage resets, unknown/zero totals, zero speed, terminal processed≠committed/partial failure/cancel/unknown cleanup, actual control dispatch/retry/old result/cancel competition and long error/member minimum/footer regressions pass in the final310 archive collection. | This does not accept physical DPI/IME/UIA/user visuals or every external-process/native timing permutation. |
| **Non-ZIP editing common foundation** | **IMPLEMENTED and targeted regressions PASS.** Existing ArchiveCapabilities has distinct Browse/Extract/Preview/Add/Delete/ReplaceContent/RenameFile/RenameDirectorySubtree plus original directory/comment operations; typed refusal and existing request semantic intent are shared. Opened source/backend/revision drive capability, not suffix or create support. Original browser/application/worker gates and ZIP candidate/native commit remain. | **7z/TAR/other-format write backends and ReplaceContent remain unsupported.** No new wire action/writer/ABI/codec dependency or write permission. No future format acceptance implied. |

**Overall remains PARTIAL**, not all-green native acceptance: the broadened56-case C# selection has **55 distinct PASS /1 FAIL /0 SKIP /0 not_run**. The retained failure is `archive job fixture actual readmission shortfall no output`, `source_unavailable` under its existing1-byte output budget. It reproduces with the frozen matching isolated worker; do not attribute it merely to earlier overwritten worker output. This is outside the repaired progress/control/directory-output slices; no source/encoded/output budget was raised to mask it.

## Minimal route, owner and preservation

Route recorded before production implementation: `feedback-hz2-repair-route.md`. Opening checkpoint159549bytes was copied and verified identical immediately before prepend. Original checkpoint bytes are retained as exact suffix; final preservation evidence is in session scratch. Existing dirty state was not reset and cumulative HEAD was not used as this round's scope.

- Progress changes stay in original `progress.rs`, `progress_metrics.rs`, `progress_feedback_tests.rs`, `jobs.rs`/`job_tests.rs`. Original one titlebar/body-scroll/nonshrinking-footer, official Kit0.8.0 Progress/DescriptionList/Alert/outlined Buttons, Form/Dialog and deferred entity/application callbacks remain. No queue/selection/config/preview/OLE/commit owner rebuilt.
- Native output fix stays in original `ArchiveOutputSession`; `OutputDirectory.Completed` is marked after successful EnsureDirectory and original native handle verification. Original worker Runner/JobSession forwards optional result_location unchanged. File commit counts remain zero for directory-only output. Original native no-reparse/held identity/cleanup/publication budgets unchanged.
- Capability changes extend existing `model.rs`/`engine.rs`/`zip_edit.rs`/`worker.rs`/`worker_zip_edit.rs`, existing protocol Modify type, original browser eligibility and application modify gate. Typed reasons include backend missing, not-ready/changed/unavailable source, volumes, application container, unsupported structure/encryption, corrupt/unverified metadata. Unknown/SFX facts are not invented; generic structure remains generic when that is all the source establishes.
- Preserve ZIP record fidelity and original per-kept-record policy: added regression caught a capability downgrade where unknown metadata blocked deleting the unsupported record. Delete can still enter the original complete candidate validator; retaining unknown records still fails. Pending/unbound metadata does not inherit the deletion exception. Existing AES/ZipCrypto/ZIP64/metadata/source/handoff/native failure/cancel/unknown tests pass.
- Existing HZ1/password/space/space_requests/archive_space/installer helper/Cargo.lock/rootCargo.toml protected hashes remain unchanged. HZ118/icon-layout8/WAC12 pass. Preview/drag/ZIP old regressions remain enabled and pass. No dependency/vendor/installer logic edits.

## Reference evidence

Read current checkpoint and all requested acceptance/outcome/reference notes before implementation. Actual HatchZip sources read at `D:\Code\hatchzip-main`: `runner.rs::launch/tick/on_update`, `task.rs::tick`, `text.rs::Rate`; reference separates processed counters and monotonic presentation sampling and stops at terminal. No Slint singleton/controller transplanted.

Native lane read Archiver `core/src/lib.rs:583` empty-directory output, GUI completion and Bandizip public language settings; no proprietary engine inspected. Legacy NxTools has no indexed general archive owner, as recorded by the existing archive note.

Capability lane rechecked all five fixed repositories against `archive-editing-manifest.json`: clean pinned commits and all recorded source hashes match (`capabilities/reference-recheck.json`). Reviewed source routes are bit7z explicit intent/UpdateItems,7-Zip/NanaZip update and subtree mapping, Ark streaming rewrite/per-backend capability and PeaZip orchestration caveats. No reference engine code or transaction is copied; delete-original/move-new/unchecked commit/password argv patterns remain prohibited. The pinned reference note is historical research, not a statement that these engines were integrated or run.

## Genuine RED→GREEN evidence

All logs and exact commands/exits are retained under:
`C:\Users\Administrator\.pi-desktop\scratch\a03c841f-63f5-4955-8c7b-3f30a8aac0cf\archive-feedback-repair`

| Boundary / raw evidence | Actual RED | Final GREEN |
|---|---|---|
| FB-AC01 original rendered surface, `parent-red-{list,run}.log` | Fresh compiled shared test binary LIST1/RUN1/PASS0/FAIL1,exit101;6.1s no-event fake time still rendered1.0MB/s + ETA00:09. This is not merely RateSamples/source-string inspection. | Named no-event/visibility/UI-only regressions pass in final310. |
| FB-AC02 actual buttons→ArchiveApplication→PlatformBridge, `controls-red-{list,run}.log` | LIST2/RUN2/PASS0/FAIL2,exit101; unsent Busy and native refusal leave real button disabled. | Four named control regressions in final310 PASS; real dispatch counts, receipt/state deadlines, accepted≠Paused, refusal retry, old receipt/attempt, cancel/disconnect/terminal and slot retention asserted. |
| FB-AC03 real worker→native output→typed terminal, `native/red-{build,list,run}.log` | Build0/warnings0/errors0, LIST1/RUN1/PASS0/FAIL1,exit1; successful real empty-directory output exists, file commits0, result locationNULL. Production hash equals opening baseline. | Frozen worker exact directory fixtures PASS,13 shape/collision + six zero-output + guarded terminal cases; fresh exported22 + default checked-in replay pass original Desktop policy. |
| Opened ZIP actual capability, `capabilities/01-archive-red.log` | Compile0/RUN1FAIL1,exit101; implemented ZIP extract capability falsely absent. | Matrix/current-source/original ZIP tests pass. |
| Actual unsupported source worker dispatch, `capabilities/03-worker-red.log` | Compile0/RUN1FAIL1,exit101; returned SourceChanged instead of unsupported-operation; no output frames. | Same worker-wire case PASS with actual format before ZIP mutation. |
| Original ZIP delete compatibility, `zip-delete-compat-red.log` | Compile0/RUN1FAIL1,exit101; capability Err(UnverifiedMetadata) disabled existing legal deletion. | Same case PASS plus original ZIP candidate integrity/source retention. |

Initial shared binary execution was used only for exact RED tests present in its --list; final source was explicitly rebuilt by Cargo. Native compilation failures, wrong API enum, zero selection, pending channel contention and source-string gates are not counted as behavioral RED.

Intermediate failures retained: first fb probe5PASS5FAIL (enabled semantic disabled=None versus obsoleteSome(false) assertions; Alert lacks the expected semantic label); actual click/dispatch assertions retained after correcting observation. First final archive308PASS2FAIL exposed native transcript test channel accumulation and premature non-ZIP metadata reason; corrected with per-case original application and backend-first refusal. Final310PASS. No old failure report rewritten and no assertion disabled to hide a product failure.

## Executed final checks and exact accounting

Every Cargo invocation checks project/global shared channels and active cargo/rustc before launch; same process sets jobs1/incremental0/RUSTFLAGS `-C codegen-units=256`, --locked --offline, serial tests and full-operation budgets. Other UI sessions repeatedly occupied the lane; rejected/waiting commands are not passing tests. No unrelated process was killed. `commands-final.json` and each per-command JSON/log record actual start/end/exit. PowerShell NativeCommandError wrappers around Cargo stderr are retained; native exit is separately captured.

| Check | Final result |
|---|---|
| Desktop archive `cargo test -p nxtools-desktop --bin nxtools-desktop archive -- --test-threads=1` | **310PASS/0FAIL/0ignored/1164filtered**,109.99s,exit0. Default real-transcript consumer executed, not env-absent return. |
| Protected subsets in that collection | HZ118/icon-layout8/WAC12 PASS; no extra-run double counting. |
| Desktop new feedback filter |10PASS; first env-absent transcript result was not accepted as cross-layer evidence; final full310 uses default actual transcript. |
| Desktop capability filter |2PASS/0FAIL/1472filtered,exit0. |
| Archive with isolated-codecs: lib + job_execution + worker_wire + zip_edit + zip_edit_worker | **22 +2 +19 +10 +1 =54PASS**,0FAIL/ignored,exit0; original ZIP encrypted-record and native-handoff worker regression included. |
| Core / IPC / Protocol --lib |**93 /43 /88 PASS**,0FAIL/ignored,exit0. |
| Strict Desktop+Archive+Core+IPC+Protocol Clippy --tests -- -D warnings, isolated-codecs |PASS exit0,152.59s; no lint exemption. |
| `cargo check --workspace --tests` |PASS exit0,2.56s final command; not a full workspace runtime test suite. |
| C# current build |PASS exit0,0warnings/0errors; normal incremental serial build, not clean rebuild. |
| Frozen worker feedback/HZ1/OLE/ZIP selection |Exact LIST=RUN=PASS47,SKIP0/FAIL0/not_run0. |
| Broader native original attempt |LIST56/RUN5/PASS4/FAIL1/SKIP0/not_run51,exit1; readmission failure retained. |
| Previously unrun four job fixtures |Exact LIST=RUN=PASS4,0SKIP/FAIL; password diagnostics/native failure, cancel after commit, create formats and risk/pause timeout. |
| Distinct union of original56 native cases |**55PASS/1FAIL/0SKIP/0not_run**; `native-final-accounting.json` retains names, not misleading total-green. |
| Scoped fmt exact18 / diff18 |PASS exit0 /0. Additional touched capability paths separately rustfmt scoped. |
| spec/release/editor/catalog/approved-icons/CLI/tree-root-drag/Phase-F-spike/fixture-selection source gates |PASS; documents/source/resource only, not physical acceptance. |
| Protected hashes |No drift. Frozen worker hash unchanged through native verification. |

No default full native suite was run. New safe fixture effects were inspected and registered; LIST occurred before identical RUN filters. Native async slow-consumer test remains controlled COM/STA/worker evidence, never proof actual Explorer does not stall.

### Worker provenance and retained native failures

Final command: `cargo build --locked --offline -p nxtools-archive --bin nxtools-archive-worker --features isolated-codecs`,exit0. Immediately copied outside shared target to `frozen-worker/nxtools-archive-worker.exe` and used explicitly for native tests. SHA256:
`771EEBDB507500E7E8F5E44B233F880798743EA597F195440CD4F1409150072D`.
It matches the final source feature build, not a signed/packaged Release approval. Native initial directory RED/GREEN used independently checked prior isolated worker `EED2…6770`; intermediate shared output was overwritten with features=[] by another lane. Those old codec/protocol/overwrite compatibility failures remain in `native/fb-ac03-native-results.md`. Final frozen worker resolves later codec-related cases but does **not** waive the earlier helper_untrusted result or the still reproduced readmission failure.

`archive job fixture actual readmission shortfall no output` uses existing MaxEntryBytes=MaxOutputBytes1 and returns `source_unavailable`; exact native code/stage was not established in this bounded repair. All referenced fixture paths exist. Source inspection confirms original native source-family encoded admission also sees output-derived sourceLimit and typed resource-limit mapping, but that alone is not a proven full cause. **OPEN**, no budget increase/native policy rewrite or blind rerun into success. The original CLI cleanup fixture was not included in the final56; its earlier helper_untrusted failure is preserved separately, not claimed PASS.

## Exact18 formatting scope

```
rust/apps/nxtools-desktop/src/archive/progress.rs
rust/apps/nxtools-desktop/src/archive/progress_metrics.rs
rust/apps/nxtools-desktop/src/archive/progress_feedback_tests.rs
rust/apps/nxtools-desktop/src/archive/drag_progress.rs
rust/apps/nxtools-desktop/src/archive/job_location.rs
rust/apps/nxtools-desktop/src/archive/jobs.rs
rust/apps/nxtools-desktop/src/archive/job_tests.rs
rust/apps/nxtools-desktop/src/archive/application.rs
rust/apps/nxtools-desktop/src/archive/application_tests.rs
rust/apps/nxtools-desktop/src/archive/entry.rs
rust/apps/nxtools-desktop/src/archive/browser.rs
rust/apps/nxtools-desktop/src/archive/browser_drag.rs
rust/apps/nxtools-desktop/src/archive/browser_preview.rs
rust/apps/nxtools-desktop/src/archive/browser_preview_tests.rs
rust/crates/nxtools-archive/src/control.rs
rust/crates/nxtools-ipc/src/lib.rs
rust/crates/nxtools-protocol/src/archive_job.rs
rust/crates/nxtools-protocol/src/archive_virtual_drag.rs
```

Original list copied from the independent acceptance fmt-files.json; baseline check exit1 retained. This is the newly authorized repair round for progress.rs. Edit protection rejections were refreshed before retry; an accidental plus-prefixed operation-header insertion was precisely removed through Edit before compilation. Two per-path rejections remained below the three-failure stop; no shell/whole-file/delegate bypass was used. Scope formatter is not a bypass of a stopped path.

## Remaining OPEN / NOTRUN

1. **FB-AC05**: user-marked complete screenshot and actual requested geometry matrix absent; no line alignment fix claimed.
2. **Broader native readmission regression**: the56-case set is not all green; source_unavailable failure retained above. CLI cleanup helper_untrusted historical round failure remains unaccepted, not silently dropped.
3. **Real user acceptance**: actual Explorer responsiveness, target writes/reuse/focus, physicalDPI/IME/UIA/user visuals, real named-pipe saturation and installed release are NOTRUN and require separate authorization.
4. No7z/TAR writer, ReplaceContent engine, HZ3–HZ5, source reclamation or destructive/crash recovery work started.

No packaging/signing/install/product or Explorer launch/Registry/service/ACL/UserChoice/source recycle/commit/push. No claim current Release32 includes this code. Overall source/automated repairs are delivered with the explicit unresolved native and visual rows, not a release-complete statement.
