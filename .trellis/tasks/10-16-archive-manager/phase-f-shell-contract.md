# Phase F — Explorer integration and genuine native drag: P206/P207 shell contract

Status: **P206 source/isolated spike implemented; P207 researched and stopped at authority gate.** This record covers only classic HKCU plan/compensation and grouped-selection research. P208 is independent. No real Registry, Explorer, association, COM, package or system mutation occurred.

## Scope and invariants

- P206 default is disabled. Rendering/saving desired settings does not apply the Shell plan.
- Saved desired revision/intent and observed applied facts are different types/lifecycles. A plan never sets its own applied result.
- Scope is pure `HKCU\Software\Classes` planning only. No HKLM, `UserChoice`, Windows 11 top-level menu, admin claim or default-association claim.
- P206 classic leaves are quoted `%1` commands into existing archive startup actions and therefore per-file. They do not satisfy P207 grouping.
- Before the first fake-store write, every planned value captures previous existence/data/kind plus owned missing key ancestry. Attempted index advances before `Write`, so mutation-then-throw rolls back.
- Compensation restores all attempted values in reverse and aggregates rollback errors.
- Uninstall is compare-before-restore. If the current value differs in kind/data from the exact applied fact, preserve it as user/foreign modification. Restore an exact prior value/kind or remove an unchanged newly-owned value; an eventual Registry store may remove only captured-created keys that are empty. Never delete a shared root because it has an NxTools-like name.
- No diagnostics payload may contain selected paths. A future real apply needs correlation, accepted/start/terminal/elapsed/native code and bounded counts only.

## Concrete P206 plan

Owner: `csharp/NxTools.PlatformHost/Windows/Archive/ArchiveIntegrationPlan.cs`.

Open With:

- owned ProgID `Software\Classes\NxTools.Archive`;
- owned `DefaultIcon` and `shell\open\command` using `--archive-open "%1"`;
- owned named value `NxTools.Archive` under each `Software\Classes\.<ext>\OpenWithProgids`;
- never sets `.<ext>` default value and never edits `UserChoice`.

Classic cascade per selected archive extension:

- `Software\Classes\SystemFileAssociations\.<ext>\shell\NxTools.Archive`;
- `MUIVerb=NxTools`, empty `SubCommands`, owned icon;
- ordered child leaves: open, smart extract, extract here, extract to name, test;
- each command uses exact current parser vocabulary `--archive-open`, `--archive-extract smart|here|name`, or `--archive-test`, followed by quoted `%1`.

The source seam has no `Microsoft.Win32`, `SHChangeNotify`, COM or Explorer call. Parent registration must later provide a reviewed Registry-backed store and command/protocol wiring only after authority.

## P207 decision and STOP

Evidence is recorded in `docs/reference-code/archive-shell.md`. Exact SDK interfaces:

- `IExecuteCommand` IID `7F9185B0-CB92-43C5-80A9-92277A4F7B54`;
- `IObjectWithSelection` IID `1C9CD5BB-98E9-4491-A60F-31AACC72B83C`;
- `IShellItemArray` IID `B63EA76D-1F85-456F-A19C-48159EFA858B` with `GetCount`/`GetItemAt`;
- `IShellItem::GetDisplayName(SIGDN_FILESYSPATH)` for path extraction.

Decision: reliable one-invocation grouped selection requires one native COM handler implementing both `IExecuteCommand` and `IObjectWithSelection`; classic `%1` and unproved `MultiSelectModel` are not accepted. No timer/debounce/coalescing may join launches.

Narrow proposed deployment is a signed x64 out-of-process native COM local server beside the protected product tuple, current-user `HKCU\Software\Classes\CLSID\{CLSID}\LocalServer32`, and owned verb `DelegateExecute={CLSID}` values. It needs packaged helper identity/hash/signing/lifecycle, exact current-user registration compensation/uninstall, and actual Windows 10/11 Shell acceptance.

**Authority gap to parent:** request user approval for a new packaged native COM helper/CLSID, HKCU CLSID + `DelegateExecute` registration, and live Explorer acceptance. Until granted, do not implement the COM server, registration, protocol handler or claim P207 PASS.

## Parent harness registration (exact fixtures)

Parent registered these five memory-only fixtures under the exact `archive Phase F integration` prefix:

1. `default disabled pure HKCU` → DefaultsAreDisabledAndPlanIsPureHkcu.
2. `classic cascade OpenWithProgids plan` → ClassicCascadeAndOpenWithProgidsHaveExactOwnedValues.
3. `precapture write throw compensation` → ApplyCapturesAllBeforeWriteAndCompensatesWriteThenThrow.
4. `exact owned uninstall mixed ownership` → UninstallRestoresOwnedValuesAndPreservesForeignOrModifiedEntries.
5. `saved intent separate applied facts` → SavedIntentAndAppliedFactsRemainSeparate.

Effects for all five: memory-only fake dictionaries; no Registry API, file, process, Explorer, COM, service or system mutation.

## Verification status

- Tests were authored before implementation. Red execution was not obtained because the parent explicitly owns `Program.cs` fixture registration and forbade project dotnet builds/runs in this lane.
- No Cargo or project dotnet build/test was run. A scratch-only `dotnet build --no-restore` probe first stopped at missing scratch `project.assets.json` (NETSDK1004); no restore/retry was performed. A direct scratch Roslyn compile against the installed .NET 8 reference pack then passed, with temporary files removed.
- Static source inspection confirms the production file contains no Registry/COM/Explorer API use and the test file uses only the fake store.
- P206 real apply/unapply, Shell refresh, installed executable path, live Explorer menus and Windows 10/11 evidence remain NOT RUN and unauthorized.

Parent convergence: isolated host build0warnings/errors, exactLIST5=RUN5=PASS5/skipfailnot_run0;
also included in final exactPhaseF10 controlled/native+pureplan set. Delegate did not run tests;
parent logs/actual final counts live in phase-f-spike-outcomes.md. No real Registry/Explorer evidence.
