# Phase F P206/P207 — independent acceptance of F-AC01–03 repairs

Date: 2026-10-23. **F-AC01, F-AC02 and F-AC03 independently CLOSED at source/isolated boundaries.**
The bounded P206/P207 source/controlled delivery is accepted after these repairs. This is NOT live
Registry/COM/Explorer acceptance, P208, whole Phase F or first-release completion. Current status:
[sole checkpoint](phase-a-checkpoint.md). Prior [negative acceptance](phase-f-shell-acceptance.md)
and [repair implementation outcomes](phase-f-shell-repair-outcomes.md) remain unchanged.

## Inspected repair and actual closure

Read current helper core/host, service compensation, regression tests/project, Shell reference and
implementation evidence. No additional optional delegate review or broad review cycle was opened.

| ID | Source repair | Independent boundary result |
| --- | --- | --- |
| F-AC01 | Helper-only SelectionCommandIdentity owns two SDK-order interface vtables, one controlling IUnknown, rooted delegates, shared explicit refcount; final Release frees native identity and invokes original idempotent Retire. Class factory returns QI reference and drops creation reference. | Original probe source unchanged: after8create/finalRelease-without-Execute, live0/ninthHRESULT0. Exact regression also passes without requiring GC for readmission, preserves8-live bound and no double decrement after Execute. Extra direct ABI probe confirms both interfaces return the same IUnknown, unsupported QI returns null/error, extra AddRef/Release balances, remaining selection interface stays callable, final secondary-interface Release retires to live0/Idle. |
| F-AC02 | PumpUntilIdle considers objects OR server locks; production Pump retains30s initial/2min hard bounds; short tests call the same decision method. | Existing short real-pump regression PASS (lock/unlock, never-activated idle, explicit hard bound). Independent actual private production Pump held a lock past30s, then returned normally after intentional unlock at32008ms with locks0. No process kill or COM registration used. |
| F-AC03 | Ledger-save failure attempts each reverse Restore inside its own catch; partial failure remains explicit, complete compensation behavior retained. | Original pure-memory probe source unchanged:17attempts,only1deliberately failing value remains, ledger_failed_compensation_incomplete. Exact regression also verifies all-succeed path leaves0values and ledger_failed_compensated. |

The helper implementation remains managed C# with an explicit native COM identity, not a new codec/
IPC framework. Test-only AllowUnsafeBlocks permits exact unmanaged stdcall slot calls rather than
managed-delegate casts; existing ordered/once-only/whole-selection tests continue to pass. Inspected
QI identity, shared refcount, final-release and idempotent retirement paths; no new confirmed in-scope
defect found. Cross-process COM marshaling/server lifecycle remains a separate unperformed live gate.

## Independent verification

Evidence root:
`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\phase-f-repair-independent-20261023-082542`.
Runner: parent scratch `phase-f-repair-independent.ps1`; per-command raw logs and `commands.jsonl`.
Scratch projects `phase-f-repair-probe` (links the unchanged original `phase-f-shell-probe/Program.cs`)
and `phase-f-repair-boundary` (COM identity + productionPump unlock) reference fresh isolated binaries.

- Fresh no-incremental Tests/PlatformHost/helper build: PASS,0warnings/errors.
- **LIST16=RUN16=PASS16**, selected16/run16/pass16/skip0/fail0/not_run0;
  raw names and order equal, checked in `independent-results.json`.
- Composition: prior13fixtures +3defect regressions; no P208worker/real-target fixture selected.
- Original probe and additional boundary probe both build/run exit0; checked numeric results above.
- Shell/spike ownership, spec, PlatformHost test-selection, task validation and diff gates PASS.
  Specs109documents/14owners; task82implement/79check entries. Existing context-size warnings remain
  (native-contracts36632/archive49080bytes); no unrelated document restructuring.
- Independent helper DLL SHA256:
  `C216B037BAAB9A6FE69A6CCEFE8B176B967190252067E6CD953D4C72CF8F58FD`.

Read preserved implementation reds:02/03/04 logs each selected1/run1/pass0/fail1/not_run0,
respectively live8 / premature254ms / attempted1. Strengthened06red refuses ninthcreation atlive8.
These are observed historical implementation reds, not fresh parent executions against reverted code.
Parent ran final code only and reproduced each original failure boundary as green; no product-source
mutation, stale-binary reuse or overwriting of prior raw logs.

## Preservation and stop

460acceptance-baseline files all unchanged before report/checkpoint writes, including source/tests,
Rust files and historical task reports. No Rust change in this repair slice; no Cargo, fullDesktop,
P208worker, split/ZIP-edit/100k/performance rerun. Prior independent fullDesktop1022/1022,128.48s remains
historical integration evidence, not a new full run. Historical1005/7 and negative F-AC report retained.
This acceptance adds only this report, updates current checkpoint and writes scratch evidence.

ProductApplyAuthorized=false, ExpectedDesktopSha256 empty and future registration plan unapplied.
No real Registry writes/deletes/Apply/Revert, CLSID/DelegateExecute/LocalServer32, real COM local-server
activation, Explorer/Shell/GUI, package/sign/install/deploy/service change, source recycle, commit/push.
P208 real pressed-HWND/physicalgesture/foreign target/fullDoDragDrop remains unaccepted; production
archive-row drag stays disconnected. EarlierA–E/JOB07/SAFE08/native/user debts remain.

Stop: the three confirmed repair blockers are closed; no further cleanup/review loop is required.
Any next helper packaging/release-tuple/registration/liveShell task needs separately explicit authority.
