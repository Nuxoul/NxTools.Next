# Phase F — Explorer integration and genuine native drag: spike outcomes

Date2026-10-22. **Source/controlled spike delivered; NOT fullP208/PhaseF/first-release acceptance.**
Only [checkpoint](phase-a-checkpoint.md) owns current status. User dependency amendment was recorded
in [plan](implement.md) before code. EarlierA–E/JOB07/SAFE08/live acceptance debts remain. Independent
PhaseE integration1002/1002/fullstrict evidence stays historical and unchanged, not rerun as research.

Contracts: [P208](phase-f-spike-contract.md), [P206/P207](phase-f-shell-contract.md). References:
[virtual OLE](../../../docs/reference-code/archive-virtual-drag.md),
[Shell interfaces](../../../docs/reference-code/archive-shell.md).

## 1. P208 source and controlled capability

- Isolated cfg(test) GPUI MouseDown seam directly enqueues spike start before MouseUp/deferredturn;
  frozen sourceHWND/snapshot/source/ordinal and once-only submission tested. Production ArchiveBrowser
  rows are NOT connected. This proves callback timing with syntheticHWND42, not physical/nativeHWND.
- New bounded explicit archive_virtual_drag_spike_v1 contract:1..32known-size regular files,
  <=512MiB total, safe relativeUTF16name<260, original ArchiveEntryCommand intents from Rust, same
  source/snapshot/generation/codepage and unique operations/ordinals/names. Exact cancel has no output
  authority. Directory/unknown-size/password-prompt support is NOT advertised by this minimal spike.
- C# one active admitted drag/dedicated backgroundSTA. Production native path reuses existing source
  inputqueue/capture WM_CANCELMODE helpers, verifies connected Desktop peerHWND and left-button
  highbit, then final current button/cancel/source check before ONE COPY DoDragDrop call. Serial pipe
  reader and shared Windows EventWindow do not run/wait on OLE. No GPUI on_drag/clipboard/SendInput/
  CF_HDROP/Extract-then-drag substitute. Existing download real-file drag behavior unchanged.
- Controlled consumer requests FileGroupDescriptorW(HGLOBAL), indexedFileContents(ISTREAM),
  PreferredDropEffectCOPY; descriptor performs zero materialization. SDK10.0.26100.0 layout592bytes/
  UTF16filenameoffset72, real IDataObject slot3ABI, format/index/medium rejection and enumNext/Skip/
  Reset/Clone verified. No IDataObjectAsyncCapability advertised; blockingGetData is OLE-STA-only.
- Actual content request enters Task.Run -> original ArchiveEntrySession -> original Rust worker/
  WindowsJob/source binding/native output verification/Cache.Seal. No second codec, output or cleanup
  framework. Exact sealed lease outputID/version/hash/size checked under held parents/FILE; native
  read-only SHCreateStreamOnFileEx gets stable exactFILE path. NativeIStream+1 transferred in
  STGMEDIUM; consumer ReleaseStgMedium owns release. Repeated requests reuse one materialization,
  independent native cursors; finite256contentrequests. Native streams already returned survive
  dataobject/drag retirement; original identity-checked cache cleanup retains busy owned output.
- Actual tinyZIP worker test requests index1 twice thenindex0, checks exact bytes, target-held stream
  read after session retirement and stale source refusal. Slowwrite boundary verifies cancel/Escape/
  sourceclose/disconnect stop waits yet retain session slot until actual original worker cleanup.
  Source monitoring/Escape uses bounded50ms lifetime timer, no logs per tick. No source recycling.
- Diagnostic events accepted/start/firstrequest+readyperindex/terminal carry same dragcorrelation,
  count/index/elapsed/nativecode/success/cleanup only. No names/paths/passwords/content/frame logs.

**Not proved:** real pressed GPUI HWND/capture release and physicalbutton across processes,
foreign target/apartment-marshaled content behavior, real Explorer/materialized+virtual multidrop,
full actualDoDragDrop interaction lifecycle. No GUI/Shell/physical/DPI/UIA/user acceptance performed.
Do not infer Windows unsupported from missing permitted live path. Production hookup waits for
independent spike acceptance; this is NOT permission to connect archive rows next automatically.

## 2. P206 pure foundation / P207 authority gate

P206 source has no Registry store/API/SHChangeNotify/system operation. Defaultintent disabled;
pure HKCU Software\\Classes classic cascade and owned NxTools.Archive ProgID/OpenWithProgids plan.
Does not set extensiondefault/UserChoice/HKLM or claim Windows11top-level menu. Savedintent/revision
and returned appliedfacts are separate. Fake transaction pre-captures every value/kind/missingowned
ancestry, marksattempt beforeWrite, compensates mutation-then-throw at every step in reverse.
Uninstall compare-checks exact appliedkind/data; restores prior values or deletes unchanged owned
values only, preserves foreign/user-modified values/children.5memory-only fixturesPASS. No real
settings/protocol Apply or uninstall registration was enabled.

P207 SDK research: IExecuteCommand(7F9185B0-CB92-43C5-80A9-92277A4F7B54) has no patharray argument;
IObjectWithSelection(1C9CD5BB-98E9-4491-A60F-31AACC72B83C) supplies
IShellItemArray(B63EA76D-1F85-456F-A19C-48159EFA858B); bounded GetCount/GetItemAt and
IShellItem.GetDisplayName(SIGDN_FILESYSPATH) provide real selection. Existing classic "%1" commands
remain perfile, NOT P207PASS. No timer/debounce/neighboring-launch union.

Recommended reliable grouped handler needs NEW authority: dedicated signedx64 out-of-process COM
localserver, classfactory/CoRegisterClassObject lifecycle, packaged/protected helper identity, current
user CLSID\\{id}\\LocalServer32 and owned DelegateExecute registration, plus separately authorized
Windows10/11classicShell acceptance. HKCU normally needs no admin, but remains real system mutation.
**STOP at this gate:** no COM selection server/registration/helper deployment implemented; proposal
is not approved by this round. Actual grouped Shell delivery remains NOT RUN/unaccepted.

## 3. Actual verification and preserved failures

Unique logs/append-only commands.jsonl:
`C:\Users\Administrator\.pi-desktop\scratch\7c2ebec3-f1a7-4623-9ed1-760f9adb94f5\phase-f-spike-20261022-200102`.
All Cargo checked shared lane and inherited jobs1/incremental0/codegen-units256; waited on unrelated
NxMonitor/Clipboard builds, never killed another process or launched duplicateCargo.

| Check | Actual observation |
| --- | --- |
| Initial controlled descriptor red |Build0warnings/errors;LIST1/RUN1/FAIL1(descriptor query must succeed), native-abi unrelated later |
| Isolated GPUI pressed callback |Exact1PASS,1002filtered(pointer-corrected-test.log); initial trait/borrow compileerrors retained |
| Spike protocol only |2PASS,65filtered(protocol-final.log); no whole Protocol/IPC rerun |
| Final C#host |BuildPASS0warnings/errors(escape-final-host-build.log) |
| Final exactPhaseF10 |LIST10=RUN10=PASS10,SKIP0/FAIL0/NOT_RUN0; final-phase-f10-{list,run}.log and exact names compared |
| Composition of10 |5OLE/COM/lifecycle fixtures:2directcontrolled,2actualworker,1COMABI+cancelledboundary;5purefakeplan. Do not call all10worker/native-target tests |
| Matching final isolated-codecs worker |Rebuilt after featurechecks;before/after SHA256922C36AF1A19A73C076F936B8344E0D6C615B51E9B9D6DC112160B95A51268ED;stable |
| Strict Desktop/Protocol Clippy |PASS final-desktop-strict.log,-Dwarnings/full dependencies; earlier pass then concurrentClipboard missingconfig compilefailure retained |
| Workspace --tests compilation |PASS phase-f-workspace-check.log |
| One completeDesktop |**1003PASS/2FAIL**,0ignored/filtered,total1005,115.10s,exit101;final-full-desktop-once.log;NOT PASS |
| Globalfmt |FAIL only concurrentui/efficiency_tests.rs1827/1843 in recordedrun;no unrelated format edit. Scoped touchedRustfmtPASS |
| Source ownership/diff/provenance/docs |PhaseFownership/touchedDiff/scopedfmt and unchangedGPUI/editor gatesPASS; final document gate result logged separately |

FullDesktop exact failures in concurrentClipboard height slice:
ui::efficiency_tests::height_feedback_history_multiline_rows_grow_with_rendered_content(line1023,
rendered final paragraph fits its row) and
ui::efficiency_tests::height_feedback_long_links_reflow_when_history_width_changes(line1057,168vs218px).
No repeated fullsuite or unrelated fix. Source state may continue changing in other tasks; this
observation remains fixed, not a permanent causal attribution. Historical1002/1002 remains history.

InitialPhaseF9run3PASS/1FAIL/5NOT_RUN was a slowfixture waiting on generic sourcecheckpoints absent
from this tinyZIP path. Both source_precommit/source_validated attempts retained. Corrected fixture
uses original outputwrite probe; thenPASS for4retirement variants. No worker change/probe weakening.
Other compile/nullability/stale-line edits, intermediateDLL filelockCS2012 and test-before-parallelP206
implementation compilefailures remain in logs. These are not behavioral reds for every new test.

Attempted realOle32 pre-cancel/no-input fixture **TIMED OUT** after its10s assertion; foregroundSTA
kept test process alive until wrapper600stimeout. Logs native-abi-run/build/list preserved. No
surviving ownedtest/build process found afterward. It proved no successful OLEinteraction; never
fakePASS/unsupported. Native wrapper now final-checks cancellation/currentbutton before Ole32.
Renamed finalfixture validates actualCOMABI and cancelledadmission without callingDoDragDrop.
No claim realOle32 loopPASS from this repairedfixture. Sole optional read-only reviewer aborted
without finalreport: reviewINCOMPLETE, no secondreview/debate cycle or independentacceptanceclaim.

## 4. Preservation / stop

Opening baseline2883tracked+preexistinguntracked files captured after recording authorization;
opening-status preserves prior dirtywork. Preserve concurrent browserextension/Clipboard/theme/
network changes, no restore/reset/dependency/vendor changes. Perfile final audit distinguishes
explicit touchedfiles from concurrentchanges; not cumulativeHEAD ownership. Previous phase outcome/
acceptance reports remain hash-identical; only sole checkpoint/currenttask/navigation updated.

No P204/P205/P209, source recycle, actualRegistry/service/UserChoice/association/admin, install/
package/signing/commit/push or GUI bypass. EarlierA–E/native/user debts unchanged. **Stop at Phase F
spike independent acceptance, with real gesture/target evidence and P207 authority gate still open.**

Final preservation audit:2883openingfiles/2837unchanged/46changed(including explicit concurrent
work),15PhaseFnewfiles;24historicaloutcome/acceptance/convergence reports hash-identical. Exact
classification/hashes in final-file-audit.json. Scoped gitdiffcheckexit0; only existingCRLF→LFwarnings
on sharedProgram/PlatformPipeClient,not hidden as behavior. NoDEBUGprobes or productionrowhookup.
