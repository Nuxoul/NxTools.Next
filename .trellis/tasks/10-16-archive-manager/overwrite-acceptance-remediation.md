# Independent overwrite acceptance findings — remediation

Status: both confirmed outcome-propagation defects repaired and isolated-fixture verified. **Phase A — Engine contracts and safety foundation remains in progress, not passed.** A–F scope/order and approved rechecked atomic-path overwrite are unchanged. Historical [independent FAIL evidence](overwrite-acceptance-review.md) remains intact. No strict target-ID CAS, fallback/native flag experiment, Release/install/registry/service operation, commit or push.

## Boundary and implementation

Changed `ArchiveWorkerOutputBridge.cs`, `ArchiveWorkerRunner.cs`, new `ArchiveWorkerOutcomeTests.cs` and exact Program/TestEffects registrations. Native publication/codec/source lease/budget code and Rust private protocol are unchanged.

`Execute(..., out ArchiveNativeOutputFailure?)` returns a typed result belonging to the exact operation ID, call ID, action and token (Begin has no allocated token). At the actual failed call, runner fixes its first failure using CompareExchange; it does not retrieve a mutable last-file category at terminal time. Six target categories (changed/missing/directory/reparse/busy/denied) and existing overwrite special outcomes are fail-stop causes. Other native/domain mappings, including target_exists→OutputCollision, remain worker-owned and compatible. A prior cancel/timeout/disconnect is not overwritten. Publication/unknown/postcommit facts outrank cancellation; native success count stays immediate and idempotent. No IoFailure blanket remapping or error-text matching.

Runner resolves one immutable `ArchiveWorkerEvent` terminal after process/pump/resource cleanup. Both `DiagnosticTerminal` and publish consume it: correlation/category/native_error_code/commit_state/files_committed/elapsed_ms/cleanup_complete are identical. Terminal phase is completed/cancelled/failed from that same category. Logging exceptions remain observational. No raw frames, paths/names/content/passwords/call tokens were added to diagnostics. No schema/version/capability or Rust changes are needed for this C#-authoritative repair.

## Red before production changes

Same-feature worker build exit0; C# test-host build exit0,0warnings/0errors. Exact new selector `--fixtures --list 'archive outcome fixture'`: selected2/run0,exit0.

- `... --fixtures 'archive outcome fixture actual worker target failures'`: selected1/run1/pass0/skip0/fail1,exit1. All12 actual Job→Rust7z→native cases reproduce lost category; Begin also loses state/native. Commit has FailedBeforeCommit but worker_failure. Native missing2/busy32/denied5 observed.
- `... --fixtures 'archive outcome fixture actual worker terminal diagnostics'`: selected1/run1/pass0/skip0/fail1,exit1. Unsupported50,success,cancel-with-output,unknown64 and postcommit results disagree with log; cleanup field absent.

Logs in session scratch534a5bed-05be-4587-9498-89ae907bc183: `outcome-red-{worker-build,build,list,target-failures,terminal-diagnostics}.log`.

## Regression scenarios and intermediate results

`ActualTargetFailures`: Begin and Commit × target_changed/missing/directory/reparse/busy/denied,12 actual worker operations. Uses real changed/missing/type/junction/exclusive occupation; denied uses a self-owned file DACL and restores its captured DACL. Asserts exact category/state/native/count/cleanup, final content/type, outside sentinel, user .nxpart ownership and own-partial cleanup.

`ActualTerminalDiagnostics`:8actual worker operations: success,cancel before publish,cancel with native publication,unsupported50,unknown64,post-verification5,external replacement after commit and throwing log sink. Captures structured log and compares fields to terminal, checks bytes/evidence retention, privacy and diagnostic failure independence.

`ActualOperationIsolation`:4actual ZIP worker operations using independent .NET-generated two-file ZIP. First file committed then second Begin/Commit failure; cancellation with one published output; same runner's next operation succeeds with two outputs. Exact category/state/native/count/cleanup/bytes and correlated logs prove no old-file/job category pollution.

Intermediate issues are not concealed: an incorrectly escaped shell command selected101/run12/pass11/fail1/not_run89,exit1; fixed exact PowerShell argument invocation. Fixture ACL restore initially resubmitted an unmodified FileSecurity descriptor and did not remove its deny ACE; explicit captured-DACL restoration fixed it, with no leftover fixture tree/worker. A later32-selection run selected32/run23/pass22/fail1/not_run9,exit1 exposed an overbroad category promotion (target_exists). Narrowed promotion to the requested target categories/existing overwrite special states, preserving old OutputCollision mapping; existing test expectations were not weakened. One editing placement mistake was corrected before compiling and is not a behavior test result.

## Executed verification

All Rust commands serial with CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256 in the same execution environment. No timeout or duplicate build.

| Check | Actual result |
| --- | --- |
| Same-feature worker build --features isolated-codecs --offline |exit0 |
| C# test-host build --no-restore |exit0,0warnings/0errors |
| Exact original worker overwrite +2new regressions |selected3/run3/pass3/skip0/fail0,exit0 |
| Existing safe16+remediation6+overwrite7+new3, --list first |selected32/run0,exit0 |
| Same integrated32 after compatibility correction |selected32/run32/pass32/skip0/fail0/not_run0,exit0 |
| cargo test -p nxtools-archive --features isolated-codecs --offline |31passed/0failed/0ignored,exit0 |
| cargo clippy -p nxtools-archive --tests --features isolated-codecs --offline -- -D warnings |exit0 |
| cargo fmt --all -- --check |exit0 |
| cargo check --workspace --tests --offline |exit0; not workspace tests |
| cargo test -p nxtools-extensions --test archive_import --offline |7passed/0failed/0ignored,exit0 |
| cargo test -p nxtools-clipboard --lib archive_ --offline |5passed/0failed/0ignored,39filtered,exit0 |

Logs: `outcome-final32-{build,list,tests}.log`, `outcome-check-{archive,clippy,fmt,workspace,extensions,clipboard}.log`. This is a scoped32fixture set, not full/default C# or native/release acceptance. Final document gates and continuing format work are recorded separately as executed. FAT/SMB/old Windows, power loss, user archives, live Shell/default associations and release actions were not exercised.

## Continuation

Proceed directly within Phase A: exact required ZIP methods/encryption, TAR/compound/streams and bounded streaming creation/traversal, then remaining native attributes/source-delete, worker contracts/backpressure and resource acceptance. Do not infer Phase A exit from these32fixtures or rewrite the independent historical findings as past PASS.

Continuation was implemented, not just planned: Gzip single/multi read/test/extract, single-file
streamed Gzip creation with full protected partial read-back test, ZIP12 and plain TAR/PAX/GNU
metadata/native extraction. Latest integrated40Rust/37C# pass,0skips; exact commands and remaining
scope in phase-a-capability-matrix.md. Repair itself required no private-wire change; later
creation adds optional schema2 fields and required partial_read_v1, separately documented in
.trellis/spec/platformhost/archive-creation.md with both-end compatibility tests. No Phase A exit.

Final continuation integration after TAR.GZ/BZip2/TAR.BZ2: Rust43/43,C#39/39,0skips,all
build/Clippy/fmt/workspace-check/shared ZIP gates exit0. Earlier32/31 and40/37 counts above
are historical checkpoints, not rewritten. Final commands/details in phase-a-capability-matrix.md.
No Phase A completion; next XZ/Zstd+remaining ZIP methods/encryption and general creation/traversal.

Historical checkpoint after independent XZ creation: original two defects repaired; C#40/40,
Rust45pass/1fail,exit101 public XZ read UnsupportedFormat after that turn's grounded-edit stop.
The red was retained, no deletion/skip/bypass. This turn public XZ adapters/compound spool/engine
integration and actual native reads pass. [Single current state](phase-a-checkpoint.md) and
[XZ read evidence](xz-read-evidence.md) supersede that execution boundary without rewriting its
historical failure. Phase A still incomplete; Zstd/remaining ZIP/general creation/native work remains.
