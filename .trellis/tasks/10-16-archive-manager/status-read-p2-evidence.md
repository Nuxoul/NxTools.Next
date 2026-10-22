# Status-read observation P2 — Python-only repair evidence

Date: 2026-10-20. Canonical **Phase A — Engine contracts and safety foundation** remains in progress.
[Repair contract](status-read-p2-contract.md); immutable [independent acceptance](product-100k-acceptance.md).
This is implementation evidence for independent acceptance, not a new full resource/lifecycle PASS.

## Unchanged acceptance conclusions

- Actual product primary100k read correctness independently PASSED:100000members compared against
  the streamed manifest,391batches, same original success terminal and actualcleanup confirmed.
- That ONE independent whole observation remains **Failed**, wrapper **Stopped/driver_exception**:
  PermissionError reading status.json during follow-up. Complete PID samples/original successful
  terminal do NOT override this verdict. **Same-connection follow-up reuse was not verified.**
- Tiny Win32exclusive-handle probe reproduces errno13reader failure and recovery after close;
  the precise competinghandle/cause of the original attempt remains unproved.
- Historical acceptance, implementation evidence/results, attempts and samples are not rewritten.
  No100k/1GiB/4GiB+17ZIP64measurement was run in this repair. PhaseA/ENG08/A–F not complete.

## Actual traced reader/publisher boundary

`driver_loop.drive -> controller.read_status('driver')`; existing Sampler.discover (50msrequested
schedule or main fallback) -> same wrapper('sampler'); final execute -> samewrapper('final').
All call control.read_status. Status failure previously immediately reached driver_exception or
sampler stop. Main assignment could leave last-good dict after exception; final read used
`read_status(...) or old_status`, so a missing/failedfreshread could retain oldcleanup evidence.

Rust `ProbeStatus.update` (resource_probe_support/control.rs112–153) serializes under its own
state mutex, bounds16384bytes, writes/truncates status.tmp, closes it, then std::fs::rename to
status.json; failedpublication records raw_os_error and accepts ownedstop. C#testresourceprobe
Control.Publish144–153 serializes bounded status, writesFileStream ShareNone, flushes/closes,
then File.Move(overwrite). ExistingC#cap65536differs from Python16384; not changed here. No writer,
engine/protocol/Job/source/helper/slot or producttimeout/ACK/cancel code changed.

Actual tinyfile mapping probe on this workstation:

| Boundary / code | Actual Python exception / errno / winerror |
| --- | --- |
| pathlib.open('rb') while exact CreateFileW share0handle held |PermissionError /13 /None |
| native CreateFileW samefile while handle held |PermissionError /13 /32 |
| ctypes.WinError5 AccessDenied |PermissionError /13 /5 |
| ctypes.WinError33 LockViolation |PermissionError /13 /33 |
| native2 FileNotFound /3 PathNotFound |FileNotFoundError /2 /2or3 |

Therefore PermissionError/errno13 alone is NOT evidence of a recoverable sharing race. No secondary
reopen/stat guesses a nativecode for a failedCRTcall. Newreader uses actualCreateFileW/ReadFile/
CloseHandle to retain the error at its owningboundary. Read at most16385bytes, validate original
run_id/schema2/dicttype/identityarraylimit, always close exacthandle; corruption/bounds failclosed.

## Minimal implementation

- Only native32SharingViolation or33LockViolation at open/read is retryable: **max4attempts per
  read,10msdelay between attempts, max3delays**. One function is shared by main/sampler/final.
  No extra thread/task/process/runtime or productbudget/deadline change. No indefinitely pending
  nextpoll policy: a continuously refusedread exhausts and raises the originalexception.
- Native5,otherI/O,PermissionError/errno13withoutnativecode,missingparent,invalidrun/schema/JSON/
  oversize fail immediately. Missingfile native2 returns unavailable{}, never a last-good dict.
- Per-run StatusReadDiagnostics (shortmutex never over I/O/sleep) keeps separate caller reads/
  attempts/retries/recoveries/exhausted/failures/missing totals, actualerrno/nativecode/type/stage
  and fixedcategory/attempt in at most16detailrows plus total/truncationflag. No path/rawJSON/
  payload/exceptionprose copies or perpoll/samplelogs. Newledger field status_reads retains facts.
- Mainclears currentstatus before eachread. Finalread starts empty and does NOTfallback to old
  observedCleaned. Earlier events remain originalevidence, not currentstatus/terminal/cleanupproof.
  Resourceverdict function and requiredPID/measurement/stop/cleanup PASSconditions unchanged.
- Reader grants SHARE_READ|WRITE|DELETE, so it does not deny DELETEaccess, and closes before
  returning. This does NOTguarantee overwriting a still-open target. The actual smallboundary
  probe returned MoveFileEx(overwrite) native5while targetopen despite share7; DeleteFile succeeded;
  overwrite afterclose succeeded. Never retry/ignore5 on that basis. Originalpublisher remains
  failclosed; possiblewriter conflicts beyond identified32/33readerrecovery are not certifiedfixed.

## Actual red -> green (real reader/driver, before implementation)

All Python tests used `$env:PYTHONDONTWRITEBYTECODE='1'`, cwd `D:\Code\NxTools.Next-main`.
The firsttwo projectregressions use a real tinyvalidstatusfile and retained Win32share0handle;
only retry scheduling is patched to deterministically release the competitor. Actualread_status
and drive execute. Noarchive/productprocess; driver child/counters use existing tooltestfakes.

```powershell
python -m unittest discover -s tests/performance/archive-resources -p test_status.py -v
```

- BEFOREcodechange: **Ran2, failures1/errors1, exit1**. Realreader raised PermissionError;
  realdriver recorded PermissionError and requested driver_exception. Logs: status-read-p2-red.log.
- AFTERminimalreader/callsitefix: SAMEtwo **2pass/exit0**, no stop or termination, inputbytes equal.
  Log status-read-p2-green.log. AST4files pass in samecommand.
- Finaldirected: **14pass/0fail/error/skip, exit0** (0.080s), status-read-p2-directed-final.log.

Regression coverage: realnative32transientrecovery/heldexhaustion(4attempts/3delays/native32errno13),
realAccessDenied5via directorynormalfileopen(noACLmutation),native33readfailurecontrolledinjection,
unknowncode/errno13andotherI/O immediatefailure,sharedsamplerrecoverystop/exhaustionbehavior,
driverclearslastgoodoncurrentfailure,finalexecute missing/deniedstatus cannot use oldCleaned even
with successfuloperation/completefakecounters,invalidrun/schema/type/oversize/JSON/identitylimit,
100invalidreaderrors produce only16details/no payloadcopy,compatibleDELETEaccess and successful
replacement/newstatusafterreaderclose. Error/recoverydiagnostics retain originalcodes and counts.

One replacement-while-open test initially asserted too much: native5was observed. Firstassumption
that only a repeatedreplace caused it was disproved on rerun and explicitsmallWin32probe. Test
now checks the actual share/access/handleclose+replacementcontract, not a falseuniversalrename
promise. Those intermediate directedruns (11pass1error, then13pass1error) remain logged; no product
or permissionscope expansion. They are testexpectationcorrections, not hidden resourcePASS.

## Final verification actually executed

| Gate / command | Actual result |
| --- | --- |
| `python -m unittest discover -s tests/performance/archive-resources -p 'test_*.py' -v` |**42pass/0fail/error/skip, exit0**,0.479s; status-read-p2-python-full.log |
| AST control.py/driver_loop.py/run.py/test_status.py |**4files PASS**,exit0 |
| Existing Pythonstop/counters/summary/ownedprocessfixtures |included in42,not an archiveworker/resourcesmoke |
| Rust/C#source and statepublishers |not modified; SHA256against openingbaseline checked at closeout |
| Native tinyfile probes |actualhandle/share/error mapping and replacementboundaries, noarchive/processsmoke |

No collaboration smoke was necessary: RUN0. No Cargo/dotnetbuild/compiledsuites,100k/1GiB/largeZIP64,
resource sampling/performance remeasurement, formalDesktop/installedPlatformHost,Release/package/
sign/install/commit/push, userfile/source delete/recycle/registry/service/drive/pagefile/cachechange.
No delegate/broadreview was launched. Toolfixgreen does NOTprove full resourceacceptance or follow-up
samereuse; any largeobservation requires separateauthorization. NativeRecycle remains unproven.

## Preservation and final document gates

| Closing gate | Actual result |
| --- | --- |
| `python tests/architecture/spec-contracts.py` |PASS106documents/14owners,exit0 |
| `python .trellis/scripts/task.py validate .trellis/tasks/10-16-archive-manager` |PASS71implement/68checkentries,exit0 |
| `tests/architecture/platformhost-test-selection.ps1` |PASSfixture/native-selection sourcegate |
| `git diff --check` /scopedwhitespace/conflict/temporary-marker scan |exit0/PASS |
| Openingbaseline comparison |94files:8authorizedchanges/86unchanged;79historicaltaskfiles unchanged |
| Rust/C#statepublisher sources |exactopeningSHA256unchanged |
| Wholetrackeddirtydiff |byte-for-byte unchanged against openingbaseline |
| Currentownedresources |0probe/host/worker,0archiveproduct/resourcepipes,0fixedtesthelpers |

Three newfiles: test_status.py and thisrepair's contract/evidence. Originalindependentacceptance,
product100kcontract/evidence/results and all historicalreports/results remain openinghash-identical.
Rawcloseout: session scratch status-read-p2-final-hygiene.json; originaldiff/status baseline and
red/green/directed/fullPythonlogs retained. Independentattempt/samples in another sessionroot were
not opened/modified/cleaned by this repair; their Failedreport remains immutable, no retrospective
claim of rehashedrawinputs outside authorizedroots. Testtemporaryfiles used automaticownedcleanup;
no unknownfilecleanup. Existing native-contracts.md35497byte>32768contextwarning remains unchanged;
unrelatedCRLFnormalizationwarnings from gitdiff were not fixed or hidden.

Toolimplementation and relevantchecks complete; prepared for independentacceptance. **No full
resource/lifecycleacceptance or sameconnectionfollow-upreuse success is claimed.** The original
independent Failed/Stopped and unknowncontender remain as observed. Largeobservationneeds separate
authorization; no newreview or nextfeature. Stopthiscorrection here.
