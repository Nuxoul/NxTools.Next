# Phase F P206/P207 formal source — independent acceptance

Date: 2026-10-23. **NOT ACCEPTED: three confirmed source/lifecycle defects remain.**
Existing isolated regression and workspace integration checks independently PASS; those results do
not cover the additional failing boundaries below. Not P206/P207 live, P208, Phase F or first release.
Current status is owned only by [checkpoint](phase-a-checkpoint.md).

## Authority / inspected scope

Read the [approved amendment](implement.md#approved-phase-f-p206p207-formal-source-amendment--2026-10-22),
[delivery outcomes](phase-f-shell-outcomes.md), [Shell reference](../../../docs/reference-code/archive-shell.md),
current component-selection/release/table/index rules and applicable owner guidance. Traced:

- P206 desired settings → deferred ArchiveApplication request/reply → typed wire → parser/adapter →
  ArchiveIntegrationService → existing pure plan/transaction → Registry adapter and applied-facts ledger.
- Official Settings/Switch/Checkbox and standalone outlined Buttons (not Toolbar.child adaptation),
  revision-only setting saves, pending state and observed response display. Compiled-view evidence is
  not visual/keyboard/DPI/UIA/user acceptance.
- P207 dedicated x64 helper, selection COM interfaces/class factory, lifetime/pump, identity/forwarder
  and pure future registration plan. One bounded optional read-only review was performed; its two
  concrete lifecycle findings were checked by the parent and independently reproduced as below.

ProductApplyAuthorized remains false; tests use fake stores. Helper ExpectedDesktopSha256 remains
empty; real forwarding is refused. No helper packaging/release-tuple change or product registration
was performed. These authorization boundaries are correctly preserved, not defects to bypass.

## Confirmed defects / minimal next repair

### F-AC01 — P207 final COM Release does not retire an unexecuted object

Location: `csharp/NxTools.ShellSelectionHelper/ShellSelectionCore.cs:305-321,369-386,400-427`.
CreateInstance increments LiveObjects, but the normal decrement is called from Execute's finally.
There is no corresponding retirement on final COM Release for a command that is created but never
executed (a cancelled/discarded Shell invocation). The two-minute hard server lifetime does not make
that a correct per-object lifetime.

Independent owned-STA, in-process CCW probe: create one command, release its returned interface,
repeat8 times without Execute, force GC/finalizers, then create a ninth command. Actual:
`live_after_final_release=8`, `ninth_hresult=-2147467259` (E_FAIL). No COM server/registration/Explorer
was started. No external objects remain, yet new valid requests are refused until server reset.

Repair at the real COM ownership boundary, preserving once-only Execute and the8-live-object bound.
Add a red-capable create→finalRelease-without-Execute→readmission regression; do not merely raise the
limit, shorten/reset a timer, or call Execute artificially from the cancellation test.

### F-AC02 — P207 LockServer-only lifetime is ignored by initial-idle pump exit

Location: `ShellSelectionHost.cs:239-258`; `ShellSelectionCore.cs:390-393,430-439`.
LockServer(true) increments the server-lock count and resets Idle, but Pump marks activation only
when LiveObjects>0. A client holding only the class-factory server lock is therefore treated as never
activated and exits when the30s initial deadline expires, despite the lock remaining held.

Independent probe calls the actual private Pump with a fresh ServerLifetime and Lock(), without
LocalServer.Run, CoInitializeEx, CoRegisterClassObject or any product activation. Actual:
`pump_returned_while_locked=true`, `elapsed_ms=30009`, `live_objects=0`, `com_registration=false`.

Make ordinary idle/activation decisions account for both objects and server locks. Keep the separately
specified hard lifetime policy explicit; the initial-idle path must not ignore a valid lock. Add a
bounded deterministic regression at this decision boundary, not another real registration test.

### F-AC03 — P206 ledger-save compensation stops after the first Restore exception

Location: `csharp/NxTools.PlatformHost/Windows/Archive/ArchiveIntegrationService.cs:202-213`.
After successful registry writes but failed ledger Save, all Restore calls share one try/catch.
One restoration failure exits the loop, so other independent attempted values are not restored.
The service reports incomplete compensation, but needlessly leaves every other write in place with
no successfully persisted applied-facts ledger. This violates the existing all-attempted reverse
compensation contract; the original ArchiveIntegrationTransaction already continues after failures.

Independent pure-memory probe: one17-value plan successfully writes, ledger Save throws, first
Restore throws, later Restore calls would succeed. Actual:
`compensation_attempts=1`, `written_values=17`, state`failed`,
failure`ledger_failed_compensation_incomplete`, `system_mutations=0`.

Attempt every reverse restoration even if one fails, retain truthful incomplete outcome and enough
failure facts under the existing ownership contract. Add a regression proving all17 attempts occur
and unaffected values restore. Reuse the existing transaction conventions; no new recovery framework.

## Independent checks actually executed

Evidence root:
`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\phase-f-shell-independent-20261023-040531`.
Runner: parent scratch `phase-f-shell-independent.ps1`; raw logs and append-only `commands.jsonl`.
Probe source/project/build/run logs: parent scratch `phase-f-shell-probe` (`run.log`, `lock-run.log`).
The probes report current defective behavior; their exit0 is successful reproduction, NOT product PASS.

| Check | Actual result |
| --- | --- |
| Fresh isolated Tests/PlatformHost/helper Rebuild | PASS,0warnings/errors; no stale mutant binary reused |
| Exact C# selection | LIST13=RUN13=PASS13,skip0/fail0/not_run0; raw names/order equal |
| Composition | 5retained pure-plan +4P206 service/ledger/wire/target +4P207 controlled COM/selection/identity/lifetime/future-plan |
| Core lib | 76PASS,0fail/ignored/filtered |
| Protocol lib | 69PASS,0fail/ignored/filtered |
| Desktop integration/settings/launch | 1/6/8PASS; actual named matches,not zero selection |
| Strict Desktop/Core/Protocol Clippy,full dependencies | PASS,`--tests -- -D warnings` |
| Workspace `--tests` | Compilation PASS,not workspace test execution |
| Globalfmt | PASS |
| ONE fullDesktop | **1022/1022PASS**,0fail/ignored/filtered,128.48s test time |
| Shell/spike ownership,GPUIrelease,editor provenance,catalog,spec/selection/task/diff | PASS; spec109documents/14owners; task82implement/79check |

All Rust commands inherited jobs1/incremental0/codegen-units256 and checked the shared lane. No
P208worker build/tests, split/edit/100k/performance matrices, real registry/COM registration/Shell/GUI
were run. Helper DLL SHA256:
`DD40AFB3B0CAE101F4019ED21840C73D18535EE6DDE7BB26EA4932862527FB76`.
`identity-results.json` independently checks raw LIST/RUN/PASS and fullDesktop counts.

Existing integration failures from historical1005/7 are absent in this current independent full run;
that closes the current regression-suite observation, not these three source defects or native/live
acceptance. Historical1005/7, implementation1022/1022 and earlier reports remain unchanged.
The implementation disclosed tests-after-code plus3mutation reds and stale-mutant rebuild history;
this acceptance does not relabel that as strict test-first development or repeat workspace mutations.

## Preservation / stop

Opening475files:473unchanged; only2helper obj build outputs changed (`assets.cache` and
`csproj.FileListAbsolute.txt`). All baselined source/test/task/history files stayed hash-identical
through independent verification. This acceptance subsequently adds this report and updates only
current checkpoint; no product/test source was repaired here. No further review loop/fullsuite rerun.
Task validation retains2preexisting context-size warnings (native-contracts36632,archive49080bytes).

**Stop for the three scoped repairs above, then targeted retest. Do not proceed to registration or
deployment from green test counts.** Real HKCU/HKLM Apply/Revert, CLSID/DelegateExecute/LocalServer32,
real COM activation/Explorer/Windows10–11 menus, packaging/signing/install/release-tuple decision and
live settings GUI/DPI/UIA/user acceptance remain NOT RUN/unauthorized. P208 real gesture/foreign target
and production-row hookup remain unaccepted/disconnected. EarlierA–E/JOB07/SAFE08/recycle debts retained.
No commit/push or first-release completion claim.
