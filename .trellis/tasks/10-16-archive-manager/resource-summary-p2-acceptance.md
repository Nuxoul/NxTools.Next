# Independent acceptance — retained RESOURCE summary P2

Date:2026-10-19. Canonical **Phase A — Engine contracts and safety foundation**.
Scope: valid attempt JSON containing malformed/truncated RESOURCE stdout only.

## Verdict

**PASS; this remaining summary P2 can be closed.** No concrete new in-scope blocker found.
The original safe-stop/sample/timing fixes were independently accepted earlier;this report does
not rerun/restate them as fresh native evidence. No PhaseA/ENG08/ENG09/A–Fcompletion claim.
No product/project-test source was modified by acceptance;only this report is added.

## Source assessment

Read summarize.py,test_summary.py,repair evidence/current checkpoint. Actual collect now catches
per-line JSONDecodeError,retains valid events in original order and continues later attempts.
Independent summary_parse Complete/Incomplete exposes parse completeness without changing original
schema2stdout/failure/operation/measurement/stop/cleanup/resource_verdict,including original PASS.
Error details contain only one-based line/fixed category,max16perattempt,with total/truncation flag;
no payload/exception prose copy. Legacy conversion/dynamic count behavior unchanged. No general
invalid outer JSON/file recovery framework was introduced. No additional delegate review needed.

## Independently executed verification

All Python commands used PYTHONDONTWRITEBYTECODE=1 where process tests/probes execute.
- Full unittest discover test_*.py: **28tests,allPASS,0failure/error/skip**,exit0.
- Original unchanged independent scratch probe now reports SUMMARY_PRODUCED=true,ATTEMPTS=2,
  ORIGINAL_ATTEMPT_BYTES_PRESERVED=true. Prior failure log remains historical.
- New independent synthetic collect+CLI probe:bad attempt has valid-before,not-json,truncatedJSON,
  20additionalbadlines,valid-after;laterattemptvalid. Output reports2attempts,22parseerrors,
  exactly16details,truncatedtrue,badsummaryIncomplete/laterComplete. Valid before/after events
  retained. OriginalPASS/facts/stdout preserved. All13inputfiles bytes/SHA256 unchanged.
  Actual CLIexit0 and completeJSON exactlyequals directcollect output.
- AST/whitespace/conflict/temporary-marker checks for both Pythonfiles:PASS.
- spec-contracts.py:PASS106documents/14owners,exit0.
- taskvalidate:PASS65implement/62check,exit0. Existing unrelated native-contracts.md35497bytes
  above32768context warning remains;not silently reported warning-free or modified here.
- gitdiff--check:exit0. Existing unrelated CRLFnormalization warnings retained in scratchlog.

Scratch independent logs:summary-p2-independent-python.log,summary-p2-original-probe-current.log,
summary-p2-fixed-independent-probe.py/log,summary-p2-independent-diff.log. Syntheticprobe uses
TemporaryDirectory with automaticcleanup;no userfile/archiveinput. Project tests also verify
16/17/4096badline detail bounds and originalbytes/facts;their count is includedin28,not extra.

Original resource-tool-repair-acceptance SHA256 unchanged:
f553329a7cba08ece6b8166b9a66464cea67337ca237de25585d9680a40434c6.
Historical measurements/results remain unchanged. The previous independent132/132C#integration
is historical;its earlier unexplained process-exit record remains,not erased or newly diagnosed.

## Not run / remaining boundaries

No Cargo,C#build/fullcollection,actualworker/smoke,sampling,100k/1GiB/largeZIP64,performance repeat,
formalDesktop/installedhost,Release/package/sign/install,commit/push or systemconfiguration change.
Python suite's existing self-owned tiny process fixtures are not productworker/resource sampling.
No nativeRecycle/permanentsourcedelete,userarchives,registry/service/drivemapping operations.
Unrelateddirtychanges preserved. Close only this smallsummaryP2;fullPhaseA/native/deployment/
large-resource/product100kescalation/UI/FIFO and A–Ffirstrelease remain unfinished.
