# Status-read observation P2 — independent acceptance

Date: 2026-10-20. Canonical **Phase A — Engine contracts and safety foundation** remains incomplete.
Scope: [repair contract](status-read-p2-contract.md), [implementation evidence](status-read-p2-evidence.md),
Python reader/call-site repair and deterministic small regressions. No implementation/test changes
were made by this acceptance; only this additive report and scratch probes/logs were written.

## Verdict

**PASS for the bounded status-read P2 tool correction. No concrete in-scope defect remains from
this review and verification.** This closes the reader-recovery finding in
[original product100k acceptance](product-100k-acceptance.md), not the complete resource gate.

Unchanged facts:
- Independent primary100k correctness passed:100000manifest-exact members/391batches/original
  successful terminal and confirmed cleanup.
- The original complete observation remains **Failed /Stopped(driver_exception)**. Its attempt
  and sample files retain exactly their previously recorded SHA256values.
- Same-connection follow-up reuse was not independently verified in that observation. No new
  100k/resource/lifecycle PASS is claimed here. Future large observation needs separate approval.
- The original competing handle/cause remains unproved. The repair handles identified native32/33
  reader conflicts; it does not diagnose the historical failure or certify every writer race fixed.

## Source review

Reviewed control.py, driver_loop.py, run.py, test_status.py, repair contract/evidence, README and
current checkpoint, plus applicable shared/archive verification contracts. No delegate was needed.

- Windows read uses actual CreateFileW/ReadFile/CloseHandle, preserving nativecode and stage;
  GENERIC_READ/OPEN_EXISTING, SHARE_READ|WRITE|DELETE; bounded16385byte read for a16384byte limit.
- Only native32/33 at open/read retry, max4attempts with3x10ms delays. AccessDenied5, unknown
  nativecode/errno13alone, missingparent3, otherI/O and invalid data fail immediately. Native2 at
  open returns unavailable{}, not a cached state. Close failures are not a retry permission.
- Driver, sampler and finalread use the same StopController reader/diagnostics. Counters use a
  short mutex outside I/O/sleep; <=16fixed details preserve native/errno/stage/category/attempt,
  without status payload/path copies. Original resource PASS predicate remains unchanged.
- Main clears currentstatus before reading; final status starts empty and cannot fall back to
  earlier Cleaned on failure/missing. Earlier raw events are not current cleanup authority.
- Tests exercise actual reader/driver/sampler/final execute boundaries. Only scheduling or the
  controlled failure/counter/child variable is replaced; real tiny Windows file handles cover
  shared-open refusal and actual AccessDenied. No archive worker or large fixture is needed.
- Delete sharing is not a guarantee that Windows overwrites a still-open target. The contract
  explicitly preserves native5failure and original Rust/C#temp-close-replace publisher policy.

## Independent execution

All Python commands inherited PYTHONDONTWRITEBYTECODE=1; cwd D:\Code\NxTools.Next-main.

| Check | Actual independent result |
| --- | --- |
| `python -m unittest discover -s tests/performance/archive-resources -p 'test_status.py' -v` |14pass/0fail/error/skip,0.059s,exit0 |
| `python -m unittest discover -s tests/performance/archive-resources -p 'test_*.py' -v` |42pass/0fail/error/skip,0.395s,exit0 |
| Independent scratch native probe |PASS/exit0; original reader, four caller labels, real handles |
| AST control.py/driver_loop.py/run.py/test_status.py |4files PASS |
| spec-contracts / catalog / taskcontext / selection sourcegate / diff |all exit0 |
| Baseline preservation |11reviewed files unchanged through tests/probe; includes reader/callers/tests, original reports/results and Rust/C#publishers |
| Original independent100k attempt/sample hashes |both unchanged |

The independent scratch probe does NOT import the added StatusReadTests class. It creates its own
small valid schema2file and retains an exact Win32share0handle. For each reader/driver/sampler/final
label, only retry scheduling releases that handle: actual native32open refusal -> one10msrequested
retry -> success, diagnostics2attempts/1retry/1recovery. Input bytes remain equal. A continuously
held real handle gives native32/open/errno13 after exactly4attempts/3delays and1exhaustion. Actual
normal-file open of a directory yields native5with no sleep/retry; missingfile gives unavailable{},
missingparent fails native3. Probe files are retained as evidence, not product/test-source deletion.

The project14tests independently cover additional native33read injection, no-code PermissionError,
othernativeI/O, wrongrun/schema/type/oversize/corruptJSON/identitylimit, diagnostic16detailbound,
compatibleDELETEaccess, handleclose then replacement, sampler recovery/exhaustion, driverlastgood
invalidation, and finalexecute fresh-status failure/missing cannot produce stale-cleanup PASS.
Full42also retains previous stop/counter/summary regressions. These are not compiled codec tests.

Static gate results: specs106documents/14owners, catalog79pages/79rows/78locked,
context71implement/68checkentries, fixture/native selection sourcegate PASS. Existing
native-contracts.md35497bytes>32768injection warning remains; no unrelated cleanup was attempted.

## Evidence and preservation

Scratch root: C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df.
- `status-read-independent-directed.log` SHA256
  `660d6964014adb9af10944615656bec42870ca317e97ce3f1bf18accabbefd19`.
- `status-read-independent-full.log` SHA256
  `116ef4eb01f7c23cd99a05cac67b0a62cbad833875f8af2c6e9a20cbdea7d04f`.
- `status-read-independent-probe.py` and `status-read-independent-probe-result.json`; resultSHA256
  `6ae2f01e2727d7b55086604a3db157d50de1ff047d87eb3aab11aa0bce2af314`.
- Opening11file hashbaseline `status-read-independent-baseline.json`; gates/exits in
  `status-read-independent-gates.json`. Not a claim to have independently rehashed the implementer's
  entire79historicalfile baseline; the narrower actual preservation checks are listed above.
- Original failed100kattempt SHA256
  `ca165d886c837a78da137d0b387a4da0e44c946d37511155dbbd566812756f55`;
  sampleSHA256 `e8b291144acb16eb9041adc57cb73a06854eb6d20e25c65f4b9f91cc4078e8d3`.

Closing observation:0fixture probe/host/worker processes,0archiveproduct/resource pipes, no fixed
testhelper. No Cargo/C#build, archiveworker/collaboration smoke, sampling,100k/1GiB/largeZIP64,
formalDesktop/installedPlatformHost,Release/package/sign/install,commit/push,source recycle/delete,
registry/service/drive-map change. No product implementation changes or new review loop.

**Stop: this bounded tool repair is independently accepted. Complete100kresource/follow-upreuse,
PhaseA/ENG08/A–F/native-user/deployment acceptance remains separate and uncompleted.**
