# Phase F P206/P207 — F-AC01/F-AC02/F-AC03 repair outcomes

Date: 2026-10-23. Scope: only the three defects confirmed by [independent acceptance](phase-f-shell-acceptance.md).
Same **Phase F — Explorer integration and genuine native drag**; no new phase, deployment or redesign.
**Status: three repairs implemented, awaiting independent acceptance.** Not P206/P207 live, P208,
Phase F or first release. `phase-f-shell-acceptance.md`, `phase-f-shell-outcomes.md` and every earlier
report are unchanged.

## Repairs and ownership changes

| ID | Location | Change |
| --- | --- | --- |
| F-AC01 | `csharp/NxTools.ShellSelectionHelper/ShellSelectionCore.cs` `ArchiveSelectionClassFactory.CreateInstance`, new `SelectionCommandIdentity` | The built-in managed CCW gave no signal on final `IUnknown::Release`, so an unexecuted command kept its lifetime slot. Ownership of the COM identity moved (helper-only) from the runtime CCW to an explicit native identity: one block exposing IUnknown/IExecuteCommand (slot 0) and IObjectWithSelection (slot 1) in exact SDK order with an explicit reference count. The final Release frees the block and calls the existing idempotent `Retire()` synchronously on the caller's thread, which releases the retained selection and the slot. Executed commands already retired; their later final Release cannot decrement twice. Once-only Execute, selection freeze, 8-live-object bound and the creation-reference failure path are unchanged (QI failure drops the only reference and retires). No GC/finalizer, timer, larger limit or forced Execute. |
| F-AC02 | `ShellSelectionHost.cs` `LocalServer.PumpUntilIdle`; `ServerLifetime.ServerLocks` | Activation/idle decision now uses `LiveObjects > 0 || ServerLocks > 0`. A held `LockServer(true)` no longer takes the initial-idle exit; when objects and locks both reach zero the normal idle exit still happens. The explicit hard lifetime bound is retained unchanged. `Pump(lifetime)` keeps production 30s/2min values; the bounds became parameters of the same decision method so the regression is short and deterministic. |
| F-AC03 | `csharp/NxTools.PlatformHost/Windows/Archive/ArchiveIntegrationService.cs` Apply ledger-failure path | Each reverse `Restore` has its own try/catch (same convention as `ArchiveIntegrationTransaction`): every attempted value is restored even if one fails; any failure reports `ledger_failed_compensation_incomplete`, all succeed reports `ledger_failed_compensated`. No new recovery framework. |

Unchanged boundaries: `ProductApplyAuthorized=false`; helper `ExpectedDesktopSha256=""`; future
CLSID/DelegateExecute plan unapplied; whole-selection reject, ordered once-only forward, desired/applied
separation, exact owned revert and path-free diagnostics retained. Test project gains
`AllowUnsafeBlocks` only so the P207 fixtures call exact vtable slots as real unmanaged stdcall
function pointers (`delegate* unmanaged[Stdcall]`).

## New regressions (real boundaries)

- `archive Phase F P207 final release without execute readmits`: 8× CreateInstance → final
  `Marshal.Release`, no Execute/SetSelection → 9th CreateInstance must succeed without forced GC;
  repeat, then all retired and Idle set; an executed object's final Release does not double-decrement;
  held interfaces remain callable; 8-live bound still refuses a 9th live object.
- `archive Phase F P207 lock server keeps pump alive`: actual pump decision method, fresh lifetime +
  `Lock()`, 250ms initial idle; must stay until the lock is released at ~900ms; never-activated server
  still exits at initial idle; explicit hard bound still ends a held lock. No COM init/registration.
- `archive Phase F P206 ledger failure compensates all values`: 17-value plan, failing ledger Save,
  first reverse Restore fails ⇒ 17 attempts, only that value remains, truthful incomplete failure;
  plus all-succeed compensation keeps `ledger_failed_compensated`.

## Evidence

Root: `C:\Users\Administrator\.pi-desktop\scratch\46a534b7-c8c9-4656-a9de-76e0c8baac60\phase-f-shell-repair-20261023-080133`
(raw per-command logs, `commands.log`, `00-baseline-hashes.txt`). Each build used `--no-incremental`
and a NEW output directory (`host-red`, `host-red2`, `host-green`, `host-green2`); helper DLL hash
differs between red (`795AFB80…`) and green (`C216B037…`) outputs.

| Step | Result |
| --- | --- |
| Red, unfixed product + new tests (`host-red`, 02–04) | F-AC01 FAIL `live=8`; F-AC02 FAIL `elapsed 254ms`; F-AC03 FAIL `attempted (1)`; each SUMMARY selected1/run1/fail1 |
| Red, strengthened no-GC F-AC01 assertion (`host-red2`, 06) | FAIL `ninth creation readmitted … live=8` |
| First green attempt (`host-green`, 09) | 9 PASS then FAIL in the old P207 order test: test cast a native slot to a managed delegate type; fixed in the test by calling slots as unmanaged stdcall pointers (not a product change) |
| Final build (`host-green2`, 10) | 0 warnings / 0 errors |
| Final exact LIST (11) → RUN (12) | **LIST16 = RUN16 = PASS16**, selected16/run16/pass16/skip0/fail0/not_run0, names and order equal (13 prior + 3 new); exit 0 |
| Independent probe re-run (13–14), hash-equal `Program.cs` against `host-green2` | `live_after_final_release=0`, `ninth_hresult=0`, `compensation_attempts=17`, `written_values=1` (the deliberately failing one), `ledger_failed_compensation_incomplete` |
| Independent lock probe (15), lock never released | no longer returns at ~30s; still pumping at 40,016ms, then the harness stopped its own probe process |
| Gates | archive-phase-f-shell, archive-phase-f-spike, spec-contracts, platformhost-test-selection PASS |

No Rust was touched, so no Cargo, full Desktop, P208 worker, split/ZIP-edit/100k/performance runs
(previous independent full Desktop 1022/1022 retained as historical). No review loop.

## Not executed

Real Registry writes/deletes, product Apply/Revert, CLSID/DelegateExecute/LocalServer32 registration,
real COM local-server activation, Explorer/Shell/GUI, packaging/signing/install/deploy/service changes.
P208 real gesture/foreign target remain unaccepted; production row drag DISCONNECTED; no P209, source
recycling, commit or push.
