# Icon layout / HZ1 independent acceptance — PARTIAL

2026-10-28. Acceptance only of the supplied icon-layout + HZ1 delivery. No product repair, package, installation, real product launch, system integration, dependency/vendor edits, commit or push. Original A–F phases, WAC acceptance and installer32 exit-race repair remain unchanged.

## Verdict

- **Icon layout: accepted at source / compiled synthetic interaction / resource boundaries only.** Actual eight icon tests pass within the independently rerun archive set; fixed Contain/nonshrinking Tab16DIP, Tree20DIP and caption20DIP use official public slots and existing32/40px assets. Titlebar8PASS. Generator check25resources and actual rebuilt testPE18decoded frames pass. No supplied screenshot or physical/user visual acceptance; do not call the rejected visual effect user-approved.
- **HZ1: NOT fully accepted.** Two disclosed problems independently reproduced; one additional directory-estimate omission independently confirmed and reproduced without product changes. Native observer and original execution confirmation pass12fixtures, but those do not close the Rust lifecycle/estimate failures or strict lint.

## Confirmed repair list

### HZ1-AC01 — P2: retired form retains UI observation bookkeeping

`repo:rust/apps/nxtools-desktop/src/archive/jobs.rs#retire_form` removes forms/pickers/edit ownership but not the matching `jobs.spaces` records. Successful submission also removes/closes its form without immediate observation retirement (`jobs.rs:193–205`). Records survive until response or7sdeadline; eight retired pending forms can temporarily fill UI max8. Late form writes are already rejected; this is not native max2-slot leakage.

Independent actual GPUI regression `hz1_closed_form_releases_only_its_advisory_pending_record` FAILS at hz1_lifecycle_tests.rs106. Completed-plan disconnect regression passes. Repair narrowly in original owner: retire only this form's advisory records on close and accepted submit, retain other forms, reject late results, and **do not release or replace native slots before native cleanup**. Keep failing close test enabled; add/confirm accepted-submit and cross-form isolation cases. Submit omission is call-chain evidence, not a separately executed new submit regression in this audit.

### HZ1-AC02 — P2 NEW: missing destination components omitted from demand

`repo:rust/apps/nxtools-desktop/src/archive/space.rs#estimate` counts archive-internal directories and Name/Smart wrapper (lines189–212), not the destination path's missing components. Native `ArchiveOutputSession.Prepare` knows `pendingRootParts`; `ObserveSpace` returns only missing=true plus opaque suffix identity, and `ArchiveSpaceObservation` has no missing-count/cost. Actual execution `CreatePreparedRoot` creates every missing component (ArchiveOutputSession.cs113–115).

Reproduction: existing validated parent + destination `missing\\child`, one known zero-byte regular member, Here strategy,4096 allocation unit,0available. The **unmodified production** space.rs returns allocation=Some(0),shortfall=Some(0),remaining=Some(0),destination_missing=true; forms_space.rs146–149 therefore chooses “当前观察可用（估算）”. This contradicts the estimator's own per-directory allowance and omits the two destination directories that execution creates. It is advisory underestimation, not output-budget expansion or execution-admission bypass; original destination confirmation still runs.

Independent scratch test directly imports current production space.rs and links the exact dependencies identified by the rebuilt Desktop test fingerprint. Build exit0; exact test RUN1/PASS0/FAIL1,exit101 (`probe-matched-build.log`, `probe-matched-run.log`). The expected8192B is consistency with the current4096-per-directory **allowance**, not a measured filesystem-allocation guarantee. Honest unknown is also permitted by the assertion. No real disk exhaustion or actual peak measurement was performed.

Repair using held-native observation of missing target components and checked Rust allowance, or make this case explicitly unknown rather than definite0. Do not infer layer count by parsing opaque target identity, assume every path component is missing, create target/probe directories, guess allocation granularity, introduce source-space credit, or change execution budgets/reserve. Add a real cross-layer/form regression for nested missing target + zero-length member + zero capacity, alongside existing/missing/changed target cases.

### HZ1-AC03 — strict gate: unnecessary owned path comparison

`repo:rust/apps/nxtools-desktop/src/archive/space_requests.rs` line60 constructs `PathBuf::from(path)` just for equality. Independent strict `--tests` Clippy exits101 with exactly one `clippy::cmp_owned`. Compare borrowed paths; no suppression or unrelated lint cleanup. This is new HZ1 strict failure, not the historical production-only SPACING_12/animated issues.

## Independent verification

Evidence directory:
`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\hz1-independent-20261028-123956`

`results.json`, `extra-results.json`, `probe-matched-results.json` retain exact arguments/exits; individual .log files retain stdout/stderr. Every Cargo operation checks idle lane and inherits jobs1/incremental0/RUSTFLAGS codegen-units256; no duplicate Cargo or timeout. Direct scratch rustc probe uses the same environment/codegen policy after Cargo completion.

| Executed check | Independent result |
| --- | --- |
| Desktop original archive filter, serial | **273PASS/1FAIL/0ignored**,1157filtered,96.62s test execution; command320.75s including rebuild. Sole failure AC01. |
| Icon layout / WAC within that run | **8/8 icon;12/12 WAC PASS**; not extra runs added to totals. |
| Desktop titlebar filter | **8PASS/0FAIL**,exit0 |
| Protocol library | **85PASS/0FAIL**,exit0 |
| Desktop+Protocol Clippy --tests -- -D warnings | **FAIL1 cmp_owned**,exit101,41.10s |
| cargo check --workspace --tests --locked --offline | PASS,exit0,54.88s |
| C# tests project dotnet build --no-restore -m:1 -nr:false -p:UseSharedCompilation=false | PASS,0warnings/0errors. Normal output directory/current incremental MSBuild, **not isolated clean rebuild**. |
| C# exact selected --fixtures --list then same filters --fixtures | **LIST12/RUN12/PASS12/SKIP0/FAIL0**,exact ordered names independently compared.11observer+1original destination confirmation. |
| New missing-target scratch probe | matched build0; behaviorFAIL1/exit101,production allocation/shortfall0 |
| approved-icons.py | PASS9 |
| generate-approved-icons.py --check | PASS25resources |
| approved-icons.py --desktop-pe target/debug/deps/nxtools_desktop-da7215c5e26ccf8d.exe | PASS18decoded frames,group1blue/group2orange; **testPE, not packaged product** |
| component catalog / release contracts / Markdown editor contracts / spec contracts | PASS; spec114documents/14owners |
| platformhost-test-selection.ps1 | PASS,static classification only |
| Scoped rustfmt --check and git diff --check | PASS; no globalfmt or fullDesktop/fullC#/engine suite claim |

C# selected fixture effects were inspected before running: GUID-owned tiny files/directories/symlinks, held native identities, controlled byte overrides, cancellation/max2STA, original tiny worker publication/confirmation. No real Registry/service/ACL/UserChoice/Explorer operation. Original confirmation fixture uses the existing target/debug archive worker; it is not a newly rebuilt/reproducibly approved Release worker.

Probe infrastructure failures remain in raw evidence and are **not behavioral REDs**: first standalone rustc linked newest same-name but incompatible-feature uuid/serde artifacts (compileFAIL); first fingerprint retry failed discovery before compilation because third-party directories retain underscores. Corrected scratch-only fingerprint lookup then compiled successfully and ran the failing assertion. No product edit/disabled test/loosened assertion was used.

## Preservation and limits

Opening baseline hashes cover relevant Desktop/protocol/IPC/native/test source, Cargo files, checkpoint and current installer. `source-drift.json` is empty after verification; final bookkeeping separately records only the intended checkpoint prepend. Existing dirty work is preserved; cumulative HEAD diff is not round ownership. Before-prepend checkpoint149553bytes is retained as an exact suffix and snapshot. No product code was changed in acceptance; new repository file is this report, plus additive checkpoint status.

One bounded readonly native review converged and found AC02; main verified call chain and obtained direct compiled behavior. No repeated broad review. In inspected native chain, source/target identity/reparse/no-create observation, cancellation publish suppression and actual-cleanup max2slot ownership remain intact; this is bounded evidence, not proof of every Windows configuration.

Physical/native product UI,100/125/150/200/mixed-monitor DPI, IME/UIA/real focus/Explorer OLE, fresh-process preferences, named-pipe end-to-end, actual ACL/restricted user/mount swap/quotas/FAT/remote/disk exhaustion/huge archives, measured capacity peak and user visual acceptance remain NOTRUN. No HZ2–HZ5 or source truncation/punch/recycle acceptance. Reserve remains0/current rule, not reserved space; native runtime rechecks remain authoritative. No packaging/install authority is implied; installer32 remains unchanged and does not contain this un-packaged HZ1 work.
