# Release archive test-only fields — bounded runtime outcomes

2026-10-24. **Actual Release worker runtime verification completed; awaiting independent acceptance.**
Only [the checkpoint](phase-a-checkpoint.md) owns current status. [Pre-execution contract](release-test-fields-contract.md)
fixes this bounded slice; [B2/B3 closeout acceptance](phase-a-b2-b3-safe04-closeout-acceptance.md) and
all four format safety acceptances remain unchanged/closed at their original evidence levels.
This is NOT full SAFE04/Phase A/F, first-release, installed product, C# Job or UI acceptance.

## Delivered change / exact entry

No product repair: original Release behavior already rejects the disabled field. Added only reusable
[raw-frame regression driver](../../../tests/integration/archive-release-test-fields.py) and
[driver unit tests](../../../tests/integration/archive-release-test-fields.tests.py), this contract/
outcomes and current checkpoint. No parser/threshold/cfg/production/C#/protocol/dependency/vendor
changes by this slice. Existing `worker_wire` in-process cfg tests were read, not broadly rerun.

Actual process entry: original worker_main `--archive-worker` -> worker::run -> bounded first
read_frame::<WorkerRequest> -> serde deny_unknown_fields. Release removes the cfg-only request field.
The original run returns InvalidData before validation/control thread/accepted/source_acquire;
worker_main exits1 with no diagnostic output. The observed rejection is therefore zero frames plus
exit1, NOT a business ResourceLimit/UnsupportedOperation response. The original protocol is unchanged.

Production Risk remains strictly >64MiB AND >1000x reliable positive packed bytes with u128 math,
exact immutable current ceiling and original tickets. Source hashes are unchanged; no real high-
expansion sample, lowering production thresholds or broader Risk retest occurred.

## Actual Release build identity

One normal source build, command:

```text
cargo build -p nxtools-archive --bin nxtools-archive-worker --release --features isolated-codecs --offline --locked --message-format=json-render-diagnostics
```

Shared lane inspected ONLY for cargo/rustc immediately before Cargo; idle. Required environment:
CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, RUSTFLAGS=-C codegen-units=256. No package/build wrapper,
signing, sequence reservation, generated approvals or artifacts delivery writes. One foreground
3600s timeout allowance; actual build241.801s/exit0, no timeout/duplicate build/other-task termination.

Actual Cargo compiler-artifact target `nxtools-archive-worker`, features `[isolated-codecs]`, profile
opt_level3/debug_assertionsfalse/debuginfo0/overflow_checksfalse/testfalse. This is build identity,
NOT by itself runtime proof. Original executable:
`D:\Code\NxTools.Next-main\target\release\nxtools-archive-worker.exe`,4688384bytes,
SHA256 **4C4E4C1DB14647195206C9F97FB3942C26A197C42609947646C7D54C07703785**.
The driver binds exact Cargo executable/profile/features/hash; before/after six cases and later
read-only audit match. No binary copy/patch, product launch, installer or installed identity claim.

## Exact six actual cases

One fresh CPython ZIP_STORED archive has alpha.txt=`release-private-field-control\n` (30bytes) and
empty.txt (0bytes). Original TEST actually checks full payload/integrity and reports2files/30bytes.
No extraction output capability exists in the driver. Requests share the same original basis and
operation ID within the batch, with only the specified field/operation changes. Separate processes
are intentional: original worker owns one start request, and IDs do not establish C# lifecycle.

| Case / expected | Actual exit | Actual stdout / frame sequence | Native/source facts / elapsed |
| --- | --- | --- | --- |
| control_test / completed no-output TEST |0|497bytes;accepted,native_call,complete;2files/30bytes|source_acquire once acknowledged by raw driver;output0;51.482ms|
| test_fixture_object / parser reject |1|0bytes/0frames|source_acquire0/output0;30.966ms|
| test_fixture_null / parser reject |1|0bytes/0frames|source_acquire0/output0;28.494ms|
| extract_fixture_object / parser reject |1|0bytes/0frames|source_acquire0/output0;28.284ms|
| extract_fixture_null / parser reject |1|0bytes/0frames|source_acquire0/output0;31.565ms|
| unknown_field / representative strict unknown reject |1|0bytes/0frames|source_acquire0/output0;30.855ms|

Exact selected6=run6=pass6,fail0/skip0/not_run0. Every case stderr0bytes, bounded natural exit,
forcedfalse/driver_errornull; deadline10s, no timed-out case credited. The object `{bytes:1,ratio:1}`
has a valid DEBUG shape; extract variants prevent misidentifying a Debug semantic TEST restriction
as Release parser rejection. Null is tested explicitly, not treated as an omitted field. One ordinary
unknown field is representative, not a generic field matrix. Negative cases receive NO native reply;
any protocol byte/accepted/source request is failure, never approval or execution fallback.

The positive source_acquire success is **transport fact injection only**, with no real native lease,
Job/helper/source protection or C# cleanup assertion. Full decoder TEST completion and owned process
exit are actual worker facts. Source path exists only in synthetic private test evidence, no secrets.
No raw user payload, executable sample or real output target was used. Worker stdout/stderr, exact
request JSON/bin, source replies, per-case results/frames/elapsed/exit and fixture/hash are retained.
No extra files appeared in case directories beyond these six evidence files. No current member
partial/decode/output authority occurred on rejected starts; zero stdout before original source
rendezvous plus the inspected entry ordering supports this exact worker boundary, not OS tracing.

## C# evidence — SOURCE ONLY

Read original `ArchiveWorkerRunner.cs` Pack/start path and `ArchiveWorkerRunner.Risk.cs`.
Base Pack branches do not serialize the field. AddRiskFixture is called on that original input;
its RiskThresholdFixture property and ALL JSON mutation/repacking are inside `#if DEBUG`. In Release
the method has an empty body. RespondRisk remains original bounded private answer forwarding with
no threshold injection/policy change. A scoped source search found the single production field-
serialization occurrence in that guarded method. No shadow serializer, C# compilation or serializer
runtime test was run, and no C# Job/source/helper/native lifecycle acceptance is implied.

## Verification / preserved failures and concurrency

Evidence root:
`C:\Users\Administrator\.pi-desktop\scratch\7c2ebec3-f1a7-4623-9ed1-760f9adb94f5\archive-release-fields-20261024-223228`.
`02-release-build.jsonl/.stderr.log`, `02-build-command.json`, `release-build-record.json`;
`09-final-runtime-command.json/.stdout.log/.stderr.log`; final successful child
`runtime-ab95debd-c548-45a0-b997-b56c4979e424`; prior05child/audits retained, not added to final6.
`10-final-evidence-audit.json` records final exact identities/counts/facts. The build artifact record
is an input identity guard; all runtime assertions are separately retained, not inferred from compilation.

- Tool-only FINAL unit invocation:7tests PASS,0failure/error/skip,Python `-W error`;08logs. It covers
  framing/bounds, exact TEST/extract variants, exit1-not-sufficient, completed control/literal30bytes/
  schema/identity, DEBUG/profile/hash/features refusal, non-optimized assertion helper and a mocked
  already-exited child observed after deadline. It is NOT7additional native cases. Earlier01/04unit6
  and05runtime6 remain distinct previous results; no aggregated test total.
- Two Python files compile/AST/indent/tab/whitespace/EOF checks PASS in the evidence audit. No Rust or
  C# source changed, so no unrelated Clippy/full-workspace/format matrix was started. No external
  Python linter/formatter is installed in this environment; no Ruff/Black/type-check PASS claimed.
- First runtime invocation03 failed ONLY in driver response field checking: WorkerFrame Rust field
  `version` serializes as `schema`, while ControlFrame correctly uses `version`. Driver read version,
  closed stdin and original worker returned Cancelled/exit0 after accepted/source_acquire. Its own
  process exited naturally,forcedfalse; NO negative case ran. Old root
  `runtime-f93fa167-6f67-49a9-b47c-b6007612dd3b` and logs remain. Corrected driver+unit schema checks;
  same exact unrebuilt worker then completed all six in05. No product red/fix or previous PASS rewrite.
- Initial Edit syntax rejection (`101<` rather than `<101`) wrote nothing, corrected before execution.
  Initial optional `.cargo` directory lookup found no directory. Initial unittest stderr was decorated
  by PowerShell NativeCommandError despite Pythonexit0/6PASS;04rawsubprocess logs avoid that wrapper.
  A progress message said29bytes; actual payload/terminal is30bytes and assertions always used its
  real length. The byte count here comes from the retained terminal, not that message.
- Opening436scoped source/document hashes exclude artifacts/protected/runtime state. Before checkpoint
  writes432unchanged,4concurrentchanges,missing0. The strict initial no-drift audit stopped on those
  changes (not a product failure); subsequent audit records exact hashes and classification. Concurrent:
  `csharp/NxTools.PlatformHost/Protocol/PlatformCommandParser.cs`, `PlatformCommandSequenceDispatcher.cs`,
  `PlatformPipeClient.cs`, `rust/crates/nxtools-protocol/src/lib.rs`. Read-only cumulative diff is context,
  NOT exact round attribution; the before/after hashes prove these paths changed in the interval.
  Their main-window/other pipe/protocol work was not edited, reverted, compiled wholesale or accepted.
  Archive/Risk/source owners and Cargo manifests/lock stayed unchanged. Tested result binds the actual
  worker binary and stable archive owners, not final dependency source or a frozen entire workspace.

One optional bounded read-only review completed; `review-resolution.md` retains all three observations.
Accepted two test-tool issues: recheck deadline after poll reports exit, and assert extract request
shape before unit normalization. Fixed at original driver/unit seams, then final7unit/6runtimePASS on
SAME unrebuilt worker. Reviewer suggested changing payload to29based on the parent's mistaken brief;
not accepted as a contract/product defect (user specified no bytecount). Keep original30byte sample,
assert literal30and payload length explicitly; no fixture rewrite to fit the typo. No second review.
Documentation/source gates executed (`initial-*` and final post-record logs under the same root):

| Check | Actual result |
| --- | --- |
| spec-contracts |PASS,114documents/14workspace owners;navigation/source-only|
| task validate |PASS,82implement/79check entries;8existingcontext-size warnings retained|
| scoped git diff --check / five-file whitespace/conflict/EOF |PASS;new Python compile/AST/indent2filesPASS|
| local links / result rows / checkpoint ownership |13validlocal links,6exactcase rows,1current heading|
| opening436hashes / history preservation |431unchanged,1authorizedcheckpoint change,4identifiedconcurrentpaths,missing0;4newownedfiles;historicalcheckpoint bodybyte-identical aftertitle relabel|

`10-final-evidence-audit.json` verifies finalrawrequest lengths/JSON,6case filesets/nooutput,exact
terminal30bytes,zero negative-source/native-output calls/forced kills and stableworker/sourcehashes.
`initial-closeout.json`/`final-closeout.json` retain finalownedhashes and concurrentclassification.
These gates are not C#native/UI/Releaseperformance acceptance. No further code changes or testmatrix.

## Remaining boundaries / stop

Only actual worker Release rejection evidence is delivered, pending independent acceptance. No C#
serializer runtime/Job/native lifecycle, formal product/installed build, live UI/Explorer, Release
large-resource/timing/first-paint acceptance. No new format/password/conflict/readmission/CLI matrix,
100k/large/long-path/performance run. TAR/stream second actual ResourceLimit remains coordinator fact
injection only, not native proof; historical source-only Release wording in old reports stays intact.
Full SAFE04/Phase A/F/first release, UNC/strict recycling/JOB07/SAFE08/cross-stage/P206-P207live/P209
remain unaccepted/separate decisions; ProductApplyAuthorized=false. No product/process-list audit,
service/updater/protected-directory/installation/release-ledger observation, package/sign/install,
Registry/COM/UNC/recycling/elevation/dependency/vendor/commit/push.

**Stop: Release测试专用字段运行时拒绝验证完成，等待独立验收。**
No automatic packaging, UI resumption or next debt.
