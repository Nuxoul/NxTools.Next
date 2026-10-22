# Independent acceptance — tree creation and output preparation

Date: 2026-10-18. Canonical phase: **Phase A — Engine contracts and safety foundation**.
Scope: independently review the pasted delivery of output preparation remediation and
7z/TAR/TGZ/TXZ creation. No product/test source fixes were made during acceptance.

## Verdict

- **PASS:** previous missing-inside-source output-directory side effect is repaired, including
  the retained independent probe and actual native source-binding regression fixtures.
- **PASS at bounded fixture level:** new format creation, independent interoperability,
  created-output open/test/extract, existing ZIP/native regressions and quality checks.
- **NOT an unconditional slice acceptance:** one P2 typed-I/O classification defect remains
  in the new 7z creation verifier. This finding is source-confirmed, not reproduced with a
  real failing disk. It does not establish unsafe publication or data loss.
- Phase A as a whole remains in progress; no Phase B/UI/A–F/release acceptance.

## P2 — real verification-spool read errors are reported as corrupt data

Locations: `rust/crates/nxtools-archive/src/worker_sevenz_create.rs:191–193` and `:260–262`.

The signature `read_exact` and decoded-entry `input.read` both use
`map_err(|_| ArchiveError::CorruptArchive)`. Consequently an actual OS/device/filesystem
read failure of the owned verification spool is indistinguishable from malformed archive
bytes. At the first location this is a direct `File` read, with no codec ambiguity. At the
second location an underlying file-read failure can propagate through the decoder but is
also discarded. The resulting failure is retained by the callback, so the later
`decoded.map_err(sevenz_error)` cannot restore its original category.

Expected: actual storage I/O -> `IoFailure`; malformed/truncated/integrity-invalid archive
-> `CorruptArchive`; already typed cancellation/resource failures must retain their category.
This is consistent with the shared typed-I/O contract and the existing provenance wrapper
in `formats/xz.rs:421–438`. Do not simply map every decoder error to `IoFailure`, match error
text, weaken integrity checking or change publication semantics.

Required repair: retain/tag the first underlying spool I/O failure before passing through
7z parsing/decoding, and add a red-capable regression at the real creation-verification seam.
Cover both signature/header reads and payload-decode reads with a valid encoded archive and
injected storage I/O failure. Assert typed `IoFailure`, zero commit, exact owned-partial abort
and source closure; retain corrupt/truncated `CorruptArchive` checks. No real disk failure or
system mutation is necessary. The existing worker fault test covers native source I/O,
native output write I/O and corrupt/truncated partial readback, not these spool OS failures.

Evidence: one bounded directed read-only review proposed this finding; the parent independently
read the exact call sites, callback first-error flow, `sevenz_error`, existing XZ provenance
handling and fault-test coverage. No repeated review or speculative redesign was performed.

## Previous failure independently rechecked

Read-only production chain verified:
`ArchiveWorkerOutputBridge` selects `Prepare` for all tree creation ->
`ArchiveCreationSources.Open` acquires source roots and checks collisions against the actual
held output session -> `CreatePreparedRoot` checks cancellation/root versions around relative
component creation -> the same held-session collision check is repeated.

Retained scratch `zip-create-review-probe/Program.cs` was rebuilt against the current
PlatformHost DLL and run against the actual same-feature worker. It creates two tiny owned
source files and leaves `input/new-output/nested` absent before Start. Actual result:

```json
{"mode":"nonexistent_output_inside_source","Category":"worker_failure","FilesCommitted":0,"CommitState":null,"CleanupComplete":true,"error":"OutputCollision","names":[],"body":null,"final_exists":false,"output_created":false,"source_members":["approved.txt","replacement.txt"]}
```

This closes the previously observed `output_created=true` symptom. C# source-binding cases
also passed missing single/multi/identity-alias internal output, unchanged existing internal
content, legal missing external output, safe parent/sibling overwrite and preparation failures.
Lawfully created external directories may remain after later failure by the explicit contract;
resource cleanup is not recursive filesystem rollback.

## Independently executed verification

Scratch prefix: `tree-acceptance-`; exact commands/filters in `tree-acceptance-verify.ps1`.
Final ledger: START18:00:35 -> BATCH_PASS18:10:32 (+08:00), all steps exit0.
Every Cargo inherited CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0,
RUSTFLAGS=-C codegen-units=256. Commands were serialized and checked for active Cargo/rustc
before each invocation. No Release packaging or build was requested or run by this acceptance.

| Check | Independently observed result |
| --- | --- |
| archive tests, isolated-codecs, offline, no-fail-fast | 86 passed / 0 failed / 0 ignored, summed across 20 result groups |
| same-feature actual worker build | exit0 |
| strict archive Clippy including tests, isolated feature | exit0 |
| strict archive Clippy including tests, default feature | exit0, includes final constructor fix |
| workspace fmt --all --check | exit0 |
| workspace check --tests --offline | exit0; NOT workspace tests |
| extensions archive_import | 7 passed / 0 failed / 0 ignored |
| clipboard --lib archive_ | 5 passed / 0 failed / 0 ignored |
| C# product and test build --no-restore | 0 warnings / 0 errors |
| exact checkpoint fixture union --fixtures --list | selected89 / run0 / pass0 / skip0 / fail0 / not_run89 |
| same exact fixture union execution | selected89 / run89 / pass89 / skip0 / fail0 / not_run0 |
| actual 4GiB+17 FastDeflate ZIP64 | RUN1 / PASS1 in this acceptance; native publication plus Python streaming size/SHA256 |
| retained independent missing-internal-output probe | OutputCollision, commits0, output_created=false, original source members only |
| spec document gate | PASS101 documents /14 owners |
| component catalog document gate | PASS79 pages /79 rows /78 locked entries |
| task context validation | PASS34 implement /31 check |
| PlatformHost selector source gate | PASS |
| git diff --check | exit0; unrelated existing CRLF normalization warnings are not whitespace errors |

All nine new tree harness entries actually ran and passed, including 18 option combinations
inside the 7z-options entry. Multiple operations within one fixture are not additional test
counts. Tests use actual Start/Job/Rust/native paths, exact literal synthetic manifests,
independent 7-Zip/Python names/kinds/sizes/SHA256, and created-output open/test/extract.
Bounded cases cover empty/long UTF8/multi-source/admission snapshots, budgets, cancellation,
source changes, no-replace/confirmed overwrite and low-noise correlated diagnostic secrecy.

7-Zip oracle: existing local26.00 executable at the explicit portable path in the verification
environment; SHA256 independently matched
`4bcc005758c5e604c0aaa257b6097846a4a708e62a9c0c7471fa360fad863f1f`.
Python oracle: `C:/Users/Administrator/AppData/Local/Programs/Python/Python313/python.exe`.
No download, installation or PATH changes; passwords passed through private stdin.

### Execution interruption retained, not counted as PASS

An initial background PowerShell wrapper disappeared without a terminal status or any archive
log content; no owned Cargo/rustc survived. Its partial ledger is preserved as
`tree-acceptance-abandoned-background-ledger.txt`. A separate wh-desktop Release build then
occupied the machine; it was not killed. After process inspection showed it had finished,
the full foreground batch above ran with a1800-second timeout and completed normally.
No completed test result is claimed for the abandoned wrapper. The retained probe also ran
once before the final batch; the large ZIP64 fixture ran only once in the final batch.

## Cleanup and evidence limits

Postbatch inspection: no Cargo/rustc/archive-worker/test-host/7z processes found; fixed test
helper absent; zero owned archive-tree-create/source-binding/independent-probe roots.
C30.1GiB/D57.8GiB free. No user archives, registry/default associations, services, drive mappings,
Release installation, commits or pushes were operated. Extensive unrelated dirty changes were
preserved. Acceptance adds this report only; scratch scripts/logs are outside the project.

Reference status: read the current source/API investigation covering legacy NxTools bounded
archive-term search and inspected Archiver/SupaZip adapter entry chains. This review does not
claim a new exhaustive legacy search, Bandizip proprietary source access or reference lifecycle
acceptance.

Remaining Phase A scope is unchanged: timestamps/attributes/MOTW, guarded recycle/source-delete
eligibility, product IPC/capability/helper-package mapping/FIFO/backpressure/conflicts and broad
resource/performance/large-count/long-path/UNC/native acceptance. Full workspace tests and full
unrestricted C# suite were not run. Passing documentation, compilation and these isolated
fixtures do not certify UI/DPI/UIA/Shell/user/release acceptance.
