# ZIP preflight output-budget readmission — scoped evidence

2026-10-20. Phase A — Engine contracts and safety foundation. **Implemented and scoped checks
PASS; independent acceptance pending. Only preflight member/total output-budget readmission,
NOT full B3/SAFE04/Phase A/A–F completion.** Current state belongs only to
[phase-a-checkpoint.md](phase-a-checkpoint.md). [Actual results](zip-output-readmission-results.json),
[verified source/lifecycle boundary](research/zip-output-readmission-sources.md).

## Small implementation

Rust `output_readmission::execute` retains the original private extraction request and uses a
caller-supplied existing runner adapter. It dispatches once, requires actual native exited cleanup,
matching ID/current limits, ResourceLimit and zero native commits before a separate OutputBudget
prompt. Explicit finite approval covers current and required quotas and existing32GiB/file +2^50total
native bounds. Cancel/invalid/stale/duplicate answers cannot dispatch. Approval changes only two
output fields plus fresh operation/source-binding identity; existing attempt control shares cancel.
One new attempt maximum; it re-enters original admission/runner/Job/source acquisition. No codec
retry loop, worker Job mutation or new generic runtime. Existing Risk exact-original-limit semantics
remain independently accepted and unchanged.

Opt-in schema2 worker negotiates zip_output_readmission_v1, builds the original All/Here plan and
checks ZIP local records before reporting current/required quotas. Original C# output Prepare holds
existing namespace without creating missing destination directories. Only output_ready creates root.
New attempt validates original native volume/full128bitID/size/mtime and Rust SHA256 snapshot before
output_ready. Actual entry/total counters, CRC/size/source/path/collision and original native atomic
publication remain authoritative. Host-owned destination and supported codepage/password/conflict/
MOTW/private options are retained; no selected-extraction capability is invented. No output merge,
committed-output upgrade or rollback, work/dictionary/metadata/Job increase, ArchiveRead write or UI.
Low-frequency correlated state/terminal/elapsed diagnostics exclude paths/payload/source fingerprints.

## Real red → green and independent tiny inputs

`ArchiveReadmissionTests.PreflightHasNoOutputEffects` first ran against the prior worker and failed
`declared output shortage must expose current and required finite quotas`: exact LIST1/RUN1/FAIL1,
SKIP0,exit1 (`readmit-red-run.log`). Same original runner/Job/actual worker preflight is now green;
missing target ancestor remains nonexistent and actual exited cleanup is true. Not a mock-only red.

`output_readmission_chain.rs` invokes the production Rust coordinator through a fixture-only stdio
adapter calling original C# ForPackage runner/Job/actual Rust worker/native publication. C# adapter
has no approval policy. CPython stdlib independently writes4096byte Deflate member; total case adds
2048bytes. Low ordinary limits1024member or4096total trigger genuine declared shortage. Scenarios:

- Member shortage -> actual original exit/cleaned source/helper/active0 -> finite4096approval -> fresh
  operation -> exact native4096bytes.
- Total shortage -> same cleanup -> finite6144total -> both native files exactly match independent data.
- Cancel -> one actual operation only, no target directory/partial/publication.
- Stale ticket and duplicate answer -> ignored; exactly one new operation, never third.
- Source overwritten while waiting -> SourceChanged on new acquisition, no output-directory effects.
- Forged local+central2048declaration with actual4096payload -> approved2048still fails ResourceLimit,
  zero native commits, no current file or owned partial, no new approval/third attempt.

Every attempt asserts new control/request ID agreement; unchanged path/codepage936/conflict auto_rename/
MOTWfalse/entries/metadata/work; real started1 and constant872415232byte native Job budget; actual
exited then DrainAsync and exclusive source/helper reopen before result. Initial missing output
ancestor is checked before approval. Small contracts additionally reject cleanup-incomplete,
commit-bearing/wrong ID/current/category reports, hard-cap shortage and invalid/unlimited/under-required
approvals; shared cancellation reaches new attempt. Strict private wire keeps schema1/read/delete out.

First chain preparation failed because test used slash-containing Windows destination; no worker
started. Fixed only fixture path and graceful adapter cleanup, then chain passed. This is not counted
as feature red. Temporary diagnostic script inspected only a synthetic fixture and left no probe code.

## Executed verification

All Cargo commands used CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256,
offline and serial tests. Waited repeatedly for unrelated Desktop Cargo; never killed it. C#Debug
apphost built first (0warnings/errors), matchingisolatedworker built last before native fixture RUN.

| Check | Actual result / scratch log |
| --- | --- |
| C#apphost build, no restore |exit0,0warnings/errors; readmit-chain-dotnet.log |
| Eight affected Rust targets |36PASS,0fail/ignored,exit0; readmit-small.log |
| Existing zip_risk::tests |2PASS,14filtered,exit0; readmit-domain.log |
| Strict Clippy isolated/default tests/examples, -D warnings |both exit0; readmit-clippy-isolated/default.log |
| Archive scoped fmt --check |exit0; readmit-fmt.log |
| Matchingisolatedworker build |exit0; readmit-final-worker.log |
| Exact C#LIST→same-setRUN |7selected/7run/7pass/0skip/0fail/0not_run,exits0; readmit-final-list/run.log |
| Spec/context/fixtureselector/diff |PASS; readmit-spec/context/selector.log and closing assertions |

Rust targets: output_readmission_chain1,output_readmission_contract3,interaction_contract4,worker_wire15,
transaction_contract5,extraction_plan_contract5,zip_boundary_contract2,metadata_wire_contract1.
C#exact names are retained in results JSON: preflight, original ZIP Risk/conflict, actual read-only
product privacy, suspended Job admission, stuck-slot retention and actual ZIP native publication.
Explicit worker/ZIPfixtures/codecfixtures/Pythonoracle configured; no missing input counted as PASS.
No full workspace suite or other-format matrix was run. Existing native-contracts35497byte context
warning remains; archive reference navigation is32764bytes (below32768).

Scratch root: `C:\Users\Administrator\.pi-desktop\scratch\949da288-1ef9-46b3-a698-90f543613cf5`.
`readmit-static-verify.ps1`, `readmit-native-verify.ps1`, baseline hashes and per-step logs retain exact
commands/results. Closing checks compare116historical taskfiles against opening hashes, validate
byte-identical prior checkpoint snapshot and JSON, and inspect no owned archiveprocess/helper/pipes.
No optional agent review or independent acceptance is claimed.

No100k/longpathmatrix/1GiB/4GiBZIP64/performance,sourceRecycle/delete/systemconfiguration,
formalapp/UI/FIFO/PhaseB,dependencyupgrade,package/install/sign/commit/push. Stop for independent
scoped acceptance; do not start next feature.
