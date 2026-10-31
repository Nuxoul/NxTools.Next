# Archive feedback / HZ2 independent acceptance — PARTIAL

2026-10-28. Acceptance only of pasted feedback delivery and feedback-hz2-outcomes.md. **Overall NOT fully accepted.** Decimal display, preview/capability and bounded async-native core pass the independently executed relevant regressions; HZ2 and result location have concrete defects below, scopedfmt fails, reported two-line alignment remains unresolved. No product repair/task replanning/package/install/product or Explorer launch/system mutation/commit/push in this audit.

## Independent findings and minimum repair list

### FB-AC01 — P2, reproduced: no-event stall retains obsolete positive speed/ETA

Production `archive/progress_metrics.rs:35–52` computes rate/ETA using only stored sample endpoints; it takes no current-time/freshness input. `progress.rs:154–159,548–553` observes only incoming snapshots and renders these values; no no-event expiry refresh exists. After two samples0B@0ms and1000000B@1000ms, an independent scratch test imports **unmodified production progress_metrics.rs**, waits6100ms without another sample, and still receives rate=Some(1000000.0),ETA=Some(9). Build exit0, exact RUN1/PASS0/FAIL1,exit101,6.10s. This reproduces the disclosed defect rather than merely noting incomplete coverage. It does not exercise a real product window or physical disk stall.

Repair using monotonic freshness/expiry and a bounded active-view refresh mechanism so speed/ETA and elapsed remain truthful without worker events; no perpetual idle timer or per-byte transport/logging. Cover actual production view with fake monotonic time and no new event, plus stop/background/terminal cleanup. Do not fix by fabricating byte events or changing actual job counters.

### FB-AC02 — P2 NEW, static call-chain confirmation: rejected pause/resume leaves controls pending

`progress.rs:201–216` sets pause_requested before deferring original app control. It clears only when a later paused state matches or terminal appears (`:151–153`), and blocks subsequent pause/resume (`:203,592`). `jobs.rs:437–442` ignores send_archive_job errors and generates an untracked control receipt. Original IPC `archive_job.rs:80–82,108–110` explicitly returns Busy when pending/queue is full, while active task can continue normally. The app command-result branch `jobs.rs:594–623` tracks start receipts, not these controls; native pause refusal likewise has no view recovery route. Trigger: active running job, full command/pending lane, user Pause (or paused job Resume) → send rejected → task never changes paused flag → view indefinitely says pausing/resuming and disables retry until task terminal. Cancel remains available while pausing, so this is not a claim that all stop control is lost.

No additional injected GPUI rejection test was run during this acceptance: **source-confirmed, not reproduced behavioral RED**. Existing progress tests cover successful pause-state projection, not enqueue/native-receipt/timeout failure. Repair in original correlated control owner: bounded pending ID/action/attempt, failed admission/refusal/timeout resolves UI state with visible retryable error, success acknowledgement!=worker actually paused, stale attempt result ignored. Preserve once-only control dispatch and actual worker cleanup/slot ownership; do not add a second manager or simply reenable repeated uncontrolled commands.

### FB-AC03 — P2 NEW, native boundary reproduced: directory-only output has no result location

`ArchiveOutputSession.cs:572–573` returns null when no **file transaction** committed before considering held outputDirectories. A successful archive containing only an empty folder has actual directory output but no file commits; `jobs.rs:694–698` then cannot auto-locate, and progress.rs582 also hides manual locate without result_location. This differs from a truly empty archive with no output.

Independent scratch C# probe loads the exact freshly checked current PlatformHost assembly and invokes production Prepare→CreatePreparedRoot→EnsureDirectory("empty")→ResultLocation with a GUID-owned root. Build exit0; run exit1 prints `actual_empty_directory_exists=True result_location=NULL`. Handles are disposed and exact probe-owned root cleaned. This is actual native output-boundary reproduction, **not a full worker archive extraction or Explorer run**. Readonly native reviewer independently found the same omission; main verified call chain and probe. Existing native shape fixtures contain files inside folders; empty-archive fixture intentionally has no output, so neither catches directory-only output.

Repair from finalized/actual held output facts including directories, not from requested path or invented file count. Keep no-output empty archive no-location. Add real worker→terminal→Desktop regressions for sole/multiple/nested empty folders, Here/Name/Smart and failure/cancel/cleanup-unknown. Do not count created preparation directories as successful result automatically.

### FB-AC04 — delivery quality: scopedfmt still fails18paths

Independent `rustfmt --check --edition2024 --config skip_children=true` on the exact reported18Rust paths exits1 and identifies all18; fmt-files.json/accounting.json retain list. Strict Clippy and type-check success do not waive this gate. No formatter/product edit was performed. Prior implementing turn's edit-protection stop is preserved evidence, not an external capability limitation or permission to bypass it.

### FB-AC05 — unresolved user visual requirement and evidence gaps

The two lines were not identified and no exact alignment fix was implemented. No annotated screenshot supplied in this audit; do not invent line coordinates or claim existing general geometry resolves the complaint. Only this exact visual correction needs that evidence; it is not a code defect diagnosis.

Full requested HZ2 fake-clock/control-once/long-error/terminal matrix remains incomplete; FB-AC01/02 are now specific missing behaviors. Real Explorer drag/target publication/focus and physicalDPI/IME/UIA/user visual acceptance remain NOTRUN. These are evidence limits, not independently proved new defects. Installed Release32 is not this source; no installed usability completion claim.

## Per-slice verdict

| Slice | Verdict and evidence level |
| --- | --- |
| Native async drag lifecycle / demand progress | PASS at source + controlled native COM/STA/stream/worker consumer and synthetic application boundaries; NOT actual Explorer acceptance. One original loop/slots/budgets retained. Slow600msmaterialization gate has UI-return1ms; cannot generalize to Explorer. |
| Actual result reveal/location | PARTIAL: normal file/directory-with-files/Name/Smart collision and policy tests pass; directory-only FB-AC03 remains. Actual Explorer reuse/foreground NOTRUN. |
| Decimal units | PASS at source/tested display conversion1000 B/KB/MB/GB/TB; raw byte budgets and persisted MiB inputs unchanged. |
| Built-in MD/TXT/image preview | PASS at relevant source/TestWindow/selection/connection/lease/preferences tests; visible official toggle, missing-field defaulttrue, explicitfalse persistence, no Shell fallback. No installed/physical/user visual claim. |
| Non-ZIP read vs modification message | PASS at source/TestWindow: no global false read error, attempted modification reason; original write eligibility not broadened. This audit does not implement non-ZIP editing. |
| HZ2 | PARTIAL: base sampling/member/terminal/minimum footer tests green, FB-AC01/02 and missing matrix remain. |
| Two-line alignment | UNRESOLVED: target lines not identified; no guessed fix. |

## Independently executed verification

Evidence directory:
`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\feedback-acceptance-20261028-153545`

Exact args/exits in results.json; ordered count/hash audit in accounting.json. Fourteen primary commands:13exit0,1fmtFAIL; the two scratch behavior probes are separate and not added to product passing counts. All Cargo checks idle lane and same-process jobs1/incremental0/RUSTFLAGS codegen-units256,serial --locked --offline and full operation budget; no build timeout/duplicate process kill. Normal current incremental Cargo/MSBuild outputs reused; not clean rebuild claims.

| Check | Independent result |
| --- | --- |
| Desktop archive serial filter |298PASS/0FAIL/0ignored/1159filtered;122.12s tests,123.74s command |
| Protected subsets within archive |HZ118/icon8/WAC12 PASS; exact names counted, not extra runs |
| Core / IPC / Protocol lib |93/43/87PASS,0FAIL/ignored |
| Clippy Desktop+Archive+IPC+Protocol --tests -- -D warnings |PASS exit0 |
| cargo check --workspace --tests |PASS exit0 |
| Current C# tests project build --no-restore -m:1 -nr:false UseSharedCompilation=false |PASS0warnings/errors,incremental |
| C# exact filters --fixtures --list then same RUN |Ordered LIST=RUN=PASS39,SKIP0/FAIL0/not_run0 |
| Native controlled slow consumer |600msoriginal writer gate,async negotiationtrue,target UI1ms; actual native marshal/GetData/End; not Explorer |
| Scoped rustfmt18paths |FAIL all18,exit1 |
| Scoped tracked diff whitespace |PASS |
| spec / release / editor / catalog / fixture selection gates |PASS; source/document/classification only |
| Independent production RateSamples no-event probe |build0;RUN1FAIL1/exit101,obsolete rate/ETA after6100ms |
| Independent actual OutputSession directory-only probe |build0;run1,real directoryexists/locationNULL |

C# exact filters: `archive Phase F virtual`, `Explorer reveal feedback fixture`, `archive job fixture actual unique destination folders`, `archive space fixture`, `archive destination fixture missing low space confirmation`. Effects were read before selection: original20drag/6Shell-readback/1result-shapes/12HZ1-destination fixtures; no real Explorer GUI or Registry/native external-target suite. Explicit encryptedZIP/RAR fixture paths prevented skipped cases. Used worker SHA256 `EED2F89901670AA9BE4EFA4D03BE5BC65BE37885CFA854B565D0CB4991666770`, equal to delivery final-built isolated-codecs worker and unchanged before/after. **This audit did not rebuild that worker or rerun full engine/worker-specific2+2 tests.** Native consumer/confirmation tests exercised it; not a packaged/Release-approved worker claim.

Read original RED logs for native synchronous drag and native collision-location, with compiled successful setup then actual behaviorFAIL; source report's intermediate260/5 and253/12 failures remain unchanged. New acceptance did not rerun pre-fix product states. One bounded native readonly review completed; no broad repeated reviewer cycle.

## Preservation / authorization

Opening691file baseline covers relevant Desktop/Archive/Core/IPC/Protocol/native/tests,Cargo files,delivery report,checkpoint,installer and worker; verification drift.json empty. Original checkpoint157771bytes is snapshot and retained as exact suffix after acceptance prepend; final preservation evidence is separate. Previous PARTIAL reports/negative logs remain. Only this new acceptance report and sole current checkpoint status are repository edits in acceptance; no product source/fixture/formatter/other task-plan changes.

No product launch, actual Explorer operation, source recycling/destructive fixture, package/sign/install/system integration/Registry/service/ACL/UserChoice/commit/push. Prior HZ1/WAC/icon/installer acceptance is not reopened, and new HZ3–HZ5/non-ZIP tasks are not started. Overall feedback/HZ2 stays PARTIAL until concrete fixes plus applicable gates pass; actual physical/user/install acceptance remains separately bounded.
