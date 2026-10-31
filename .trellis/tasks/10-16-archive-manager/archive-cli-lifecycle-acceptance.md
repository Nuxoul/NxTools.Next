# Archive CLI lifecycle — independent bounded acceptance

Date: 2026-10-24. **PASS for the executed controlled-process/native-signal, original-runner short
Drain and scoped diagnostics evidence.** No confirmed product defect or product fix. This does not
accept physical keyboard input, uninstrumented-host timing, the independently winning parent monitor,
public prolonged cleanup timeout, Explorer/foreign COM or full Phase F/first release.
Current state: [sole checkpoint](phase-a-checkpoint.md). [Implementation outcomes](archive-cli-lifecycle-outcomes.md),
[contract](archive-cli-contract.md#bounded-lifecycle-evidence-contract--2026-10-24), ordinary CLI
[acceptance](archive-cli-acceptance.md) and all historical failures remain unchanged.

## Inspected test boundary

Parent directly read prepare-archive-cli-lifecycle.ps1, archive-cli-lifecycle.py,
ArchiveCliLifecycleFixture.cs and ArchiveCliCleanupLifecycleTests.cs. No optional delegate review or
additional broad product review was needed. Test-only observer uses original output-write,
native_published, control-input and cleanup probes, bounded8second named-event barriers; no second
codec/Job/output owner or product test-switch was added.

Fresh scratch host copies were independently compared to repository sources: only Program.cs and
Protocol/ArchiveCliHost.cs differ, at log factory/session observation/Publish hooks. Original
Read/JobSession/runner/log implementations in the copy match source. ArchiveCliLifecycleFixture.cs
is an added scratch-only file, not compiled by product projects. The unchanged real CLI receives the
actual Windows event; host instrumentation controls timing, not CLI cancellation or terminal results.

Driver creates a NEW Windows console and checks GetConsoleProcessList immediately before sending
CTRL_C_EVENT: exactly driver + Popen-owned CLI required; no AttachConsole/FreeConsole/current-terminal
operation. Native group0 event is therefore confined to that new console. Driver handler suppresses
its own event; original CLI converts its event into original protocol cancel. Process image/parent
relationships are checked for owned host/worker. Missing OpenProcess with error87 means absent;
access denial is not treated as exited. Only exact owned processes can be terminated by fixture
failure cleanup. No unrelated process/window/service/user directory was targeted.

## Independent build and execution

Evidence root:
`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\archive-cli-lifecycle-independent-20261024-111616`.
Unique original-build/per-case logs, fixture-identities.json, independent-results.json and final
preservation records retain identities/results. Used checked-in preparation/driver directly; no
implementation/test edits in acceptance. C# initial/controlled/owner-test builds0warnings/errors.
Host first, feature check then matching isolated worker rebuild; Cargo lane checked and
CARGO_BUILD_JOBS=1/CARGO_INCREMENTAL=0/RUSTFLAGS=-C codegen-units=256 inherited. No duplicate/timeout.

Fresh identity SHA256:
- unchanged CLI `DF24070DED3AA927703F614CAEBCEC46CD881A3DBD1762D852BAFA2DD9BBDA3E`
- matching worker `7C81BF94489D878C41F18B46857EF7851DE24ED693CF9E5388DE5442CF9BB8D1`
- controlled host DLL `28E036E7FF96DB9612938E9F9F541C301D008C0AC214D6241516467387FD1514`

All six fresh driver invocations passed once, no retry/green-hunting:

| Boundary | Independent observation |
| --- | --- |
| Native precommit Ctrl+C | signal-15f8391e-c965-481f-8a69-30b01e88491e. Console exactly31152+32416; one event; actual cancel_written; CLI still alive/no JSON after cancel/while cleanup held. Final exit7,cancelled,0committed,cleanuptrue; original unpublished journal has partial_removed. Source/helper exclusive-denied before/at cleanup, reopen after. |
| Native Ctrl+C after publication | partial_signal-4472e3aa-d420-49bd-a69a-7998d6f62189. Console exactly32388+32640; one event; original cancel/cleanup ordering. Exit7,cancelled_with_committed_output,1file6bytes,Committed,publishedjournal; first.txt literal retained,second absent,no partial. |
| Owned parent terminated after publication | parent_exit-3a846cd6-1817-4d42-b9cb-4c6dd1037cc6. Exact owned CLI exit1/noJSON; cancel_written observed before work release; children exit/source-helper reopen; first6bytes retained,second absent. This proves effective EOF/Stop retirement, not independently winning MonitorAsync. |
| Private pipe interruption | eof-3687bc7d-1255-46be-a1a9-74903c0e1fe8. Only own pipe disposed; cancel_written before work release; public CLI exit7/disconnected and cleanup_complete=false,not invented remote cleanup. Host/worker later exit,source/helper exclusiveopen,no partial. No requirement for terminal observation after transport is retired. |
| Isolated sink | diagnostics-78623d57-4675-44d4-a190-2d9fc59867af:20ownvalidrecords,start/connected/stopped under session12c4c7d8-0a63-458c-971b-4ae622eeb923,originaljob terminal cleanuptrue,elapsed/native fields; matching-malformed0. |
| Default sink | diagnostics-ff5d5f56-9f8f-4481-925a-41ff41635931:20ownvalidrecords,start/connected/stopped under sessione3a490a0-c4ef-470e-8696-6ba91a3ebdd7,originaljob terminal cleanuptrue,elapsed/native fields; matching-malformed0. Reads only involved dates/ownPID,not general log cleanup. |

Sources/helpers hash before/after equal in each case; each measured abnormal-process case confirms
recorded exact host/worker exit. No case partial remained. Diagnostics normal cases confirm original
successful terminal/CLIexit0 and source/helper reopening; no foreign child-exit branch is inferred
from an empty abnormal-process ID list. Final independent process scan found no executable under
this evidence root. Returned nativecode null is retained as unavailable/no native error,not fabricated0.

## Independent original-runner short Drain fixture

Fresh isolated owner-tests executable exact name:
`archive cli lifecycle fixture actual cleanup timeout retains ownership`.
**LIST1=RUN1=PASS1**,selected1/run1/pass1/skip0/fail0/not_run0; exact names/order checked.
09-cleanup-list.log,10-cleanup-run.log and
archive-cli-cleanup-d2b5472d-6a2f-4481-9c01-81a88b2d0167/evidence.json retain details.

Actual two original worker TESTs reached cleanup after process/Job/pump drainage but BEFORE original
source/helper/resources/slots retire. DrainAsync(150) returnedfalse at151ms; twoactive slots remained,
no exited/cleanupcredit,exclusiveaccess refused. Third original request refused concurrency_limit,
not admitted. Explicit checkpoint release led to draintrue,twoactualsuccess/cleanuptrue terminals,
active0,source/helper reopen and unchanged hashes,0failedassertions/0checkpointtimeouts. Only the
fixture's unchanged staged helper copy was removed AFTER confirmed drain; sample/evidence retained.
This is NOT a public CLI timeout/session DisposeAsync Drain(false) false-success reproduction.

## Gates / scope / preservation

Python AST, preparation-script syntax through execution, source gate16assertions, spec111documents/
14owners, task and diff checks PASS. Existing context-injection size warnings retained. No ordinary
15-command rerun,CLI/Download test matrix,full Desktop/workspace run,large/100k/codec/password/conflict/
performance test or optional review. Previous ordinary CLI acceptance remains at its own evidence.

No product bug was confirmed; all results were obtained without product/test source edits. The
implementation's early default-tail0 observations and later successful bounded captures remain
unchanged: no deterministic sink failure proved, no logging repair justified. Both own default
captures passing does NOT establish general concurrent-sink reliability/throwing/full/blocked sink.

Before report/checkpoint writes, **3170/3170 baseline files unchanged**,missing0,including source,
Generated,tests,history,dependency/vendor,installer and approval artifacts. **84/84 monitored sequence26
delivery files unchanged** (broader than implementer's explicitly counted80package+setup subset).
Six protected binaries match packaged26 hashes. Setup SHA256 remains
`308F02D08B22FF66018CAD963A66605C9E3E0F9B2BA8334E1963C55645DA2DD6`.
Only this additive acceptance report/current checkpoint and scratch evidence written. No installed
launch/install/package/signing/service/bootstrap/Registry/COM/GUI/globalconsole/userdatacleanup,
commit/push or version change. No recurring cleanup/review work needed for this finite acceptance.

Remaining NOT RUN/unaccepted: physical keypress; exact controlled timing under uninstrumented host;
parent monitor independently winning; public long cancel-drain timeout/irrecoverable OS non-drain;
full default-sink reliability/failure injection. Real Explorer/OLE/foreignCOM/DPI/UIA and prior partial
drag confirmation/P206/P207live/A-E/JOB07/SAFE08 remain separate. Installed26 still lacks newCLI;
use whole matching isolated directory only. **Stop: bounded CLI lifecycle evidence independently
accepted, not fullCLIeveryfailurepath/P208/PhaseF/firstrelease.**
