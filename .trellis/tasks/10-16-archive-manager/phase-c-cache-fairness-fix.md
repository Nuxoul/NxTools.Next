# Phase C — Archive browser and entry lifecycle: cache fairness fix

2026-10-21. Fix only the local-retirement starvation confirmed in
[closeout independent acceptance](phase-c-closeout-acceptance.md). No broad review, new cleanup
framework, root-rename finding or Phase D/E/F work. Current status remains in
[phase-a-checkpoint.md](phase-a-checkpoint.md). Historical reports are unchanged.

## Minimal change

`ArchiveEntryCache` adds one local-retirement ID queue. First retirement enqueues once; each Sweep
snapshots at most8queued candidates, removes successful/absent IDs and moves incomplete leases to
the tail. A candidate is attempted at most once per batch. The existing lock serializes collection
changes. This replaces fixed-first8dictionary enumeration; filesystem sweep remains unchanged.
No threads/timers/unbounded retry, expiry change, recursive deletion or weakened validation.
24h eligibility, active locks, output identity/hash/metadata/MOTW, retirement record and restart
recovery retain their original owners and semantics.

## Original-cache regression: observed red -> green

New Fixture `ArchiveCacheRetirementTests.LocallyRetiredLeasesRotateWithoutStarvation`:
- Creates9tiny real leases and seals each output, then adds one owned unknown obstruction each.
- All initial releases remain incomplete; only the ninth obstruction is removed.
- Two normal Sweeps must retire the ninth without an explicit ninth-ID release; first8unknown
  files and their recovery records must remain intact.
- After that success, admits/releases another obstructed lease; removes only test-created
  obstructions and verifies a single batch cannot retire all9, while subsequent finite batches
  finish all remaining roots/records. Covers removal/admission during rotation.

Before production edit: C#build0warnings/errors; LIST selected1/run0; RUN selected1/run1/pass0/
fail1/skip0/not_run0,exit1: `ninth locally retired lease starved behind eight obstructed leases`.
The same test/registration sources were hash-identical between red and green; no assertion relaxed.

After fix: C#test-host build0warnings/0errors,exit0. Exact same-set LIST -> RUN filter
`--fixtures "archive cache fixture"` selected these4identities in catalog order:
1. archive cache fixture obstructed retirement restart
2. archive cache fixture expiry startup retained owners
3. archive cache fixture ancestry malformed recovery
4. archive cache fixture fair local retirement rotation

LIST selected4/run0/pass0/skip0/fail0/not_run4; RUN **selected4/run4/pass4/skip0/fail0/not_run0**,
exit0. LIST/RUN identities were compared exactly, not inferred from counts. Confirmed local cache
fairness defect is closed by this scoped implementation/regression; independent acceptance pending.

## Checks, preservation and stop

Test-selection architecture gate PASS. Documentation gates/scoped diff and final historical hash
results are recorded in this turn's evidence directory. No Cargo, Desktop/Protocol/IPC/codec test,
worker rebuild, GUI/Shell or full native10 rerun: this change touches cache/C#fixtures only.
Prior independently accepted Desktop922/922,archive40,Protocol59,IPC31,native10 and strict lint/fmt
remain prior evidence, not freshly executed results. Font gate and real GUI/native interaction
acceptance retain their existing FAIL/NOT RUN status; they were not rerun or modified here.
No dependency/vendor/system/config/source-recycle/package/install/sign/commit/push or D/E/F.

Unique session evidence directory:
`phase-c-cache-fairness-20261021-180515` under scratch163f435b-1ee5-4c07-8102-31a2c7441b9b.
Contains append-only commands.jsonl,01-red-build/02-red-list/03-red-run,04-green-build/05-green-list/
06-green-run,07-test-selection-gate logs, exact test-identities.json and red-test-hashes.json.
Failure log is preserved. Opening149historical task-file hashes exclude the current checkpoint;
all matched before result/checkpoint edits. Stop at the Phase C independent acceptance point.

Final gates: platformhost-test-selection PASS; spec-contracts PASS106documents/14owners;
task validate PASS82implement/79check with existing native-contracts/archive-reference size warnings.
Scoped git diff --check PASS; no-index whitespace checks for all6changed/new scoped paths report
zero diagnostics. The first extra no-index wrapper mistook normal difference exit1 for failure;
its raw log remains, and13-scoped-whitespace-verified.log records correct interpretation without
source changes. Final historical preservation149/149matches. No additional product tests/builds.
