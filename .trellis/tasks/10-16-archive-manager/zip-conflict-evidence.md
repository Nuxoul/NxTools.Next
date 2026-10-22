# B2 ZIP extraction conflicts — implementation evidence

2026-10-20. **ZIP conflict slice implemented, scoped checks PASS, pending independent acceptance. Not full B2/Phase A/A–F.** [Sources](research/zip-conflict-sources.md), [results](zip-conflict-results.json), [current checkpoint](phase-a-checkpoint.md). [Password-only independent PASS](password-operation-acceptance.md) and B1/delivered C4 are not reopened. Other formats/risk/UI remain unimplemented.

## Delivered boundary

Private schema2 opt-in `conflict_policy` accepts ask/overwrite/skip/auto_rename for ZIP extract only. Existing default/preconfirmed behavior is retained when absent. ArchiveRead versions/DTOs/write authority, budgets, dependencies and native publication primitives are unchanged. Original ExtractionPlan rejects unsafe/internal duplicate/case/file-prefix collisions BEFORE native target observation or prompt.

Rust owns decisions through existing InteractionPort, original control reader and one task-local apply-all value. No new job/process/transaction framework. C# original output bridge observes ordinary target via held-relative no-reparse handles and stores ONE token/path/full volume+128-bitID+size+mtime+hash snapshot. The target check handle closes before waiting, permitting external change. Overwrite consumes matching observation and invokes original BeginFile(expected,true); original begin recheck and full validated partial -> flush -> immediate recheck -> close target check -> single atomic path replacement remain. A changed target returns native target_changed, not silent approval/retry/CAS/delete/truncate/backup-first.

Skip creates no file partial and increments Rust skipped count. Completion summary records files_skipped/all_members_written; original engine source-delete evidence becomes members_skipped, never eligible. C# terminal retains exact FilesSkipped, actual FilesCommitted and succeeded_with_warnings, not complete-output success. AutoRename uses same-directory name (2).ext onward, finite2..4097 search and original path validation; shared immutable archive entries reserve original outputs/implicit parents. Final publication is always no-replace: a racing occupant is preserved and OutputCollision returned, no blind publication retry.

Original source/worker/Job/output session and cancellation remain owners. Prompt ticket installed before notification, stale/duplicate responses ignored by existing port, apply-all scoped to later same-task ordinary-file conflicts. Cancel prevents later members and retains already committed files/count; original session cleans only own unpublished partials. Safe correlated diagnostics record awaiting_conflict/summary/terminal counts/elapsed, never paths/signatures/content. Other formats do not opt in. No passwords, UI or source-recycle behavior added.

## Actual red and real verification

Independent tiny ZIP producer is .NET ZipArchive, not NxTools. Fixed literal incoming/original bytes are asserted through original C# runner/Job -> actual Rust isolated worker -> original native publication.

- First actual Ask red: LIST1/RUN1/FAIL1/skip0/exit1 (`conflict-red-run.log`): existing ordinary target never entered pending. Ask->Overwrite green1/1 after implementation, exact native bytes (`conflict-green-run.log`).
- New fixture16small scenarios: Ask Overwrite/Skip/AutoRename; explicit3policies; two conflicts with each of3apply-all choices (one prompt); next task asks again; stale ticket during second wait and retired/prior-job tickets; changed target while waiting; cancel before output and after one commit; externally raced numbered destination; internal case collision rejected with0prompts; original archive numbered name reserved. All passed. Final old-target/new-name/payload/counts/foreignpartial/owned cleanup and source-exclusive-reopen assertions are real, not preconfirmed dictionary/fake worker success.
- Skip source-delete exclusion additionally tested through public engine+output seam:3skips/0partials/0commits/members_skipped. No source deletion/recycle API is invoked.

All Cargo commands serial with CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256,offline. External Desktop/Core/Network Cargo jobs were observed; this slice waited for idle, never terminated them. One early check waited on an external build lock; subsequent commands checked/waited before dispatch. No duplicate own build/worker/relink overlap.

| Actual command/check | Result |
| --- | --- |
| `cargo test -p nxtools-archive --features isolated-codecs --offline --test interaction_contract --test worker_wire --test transaction_contract --test extraction_plan_contract --test zip_boundary_contract --test metadata_wire_contract -- --test-threads=1` |29pass/0fail/ignored,6targets (3+13+5+5+2+1),exit0; conflict-small.log |
| `cargo clippy -p nxtools-archive --features isolated-codecs --tests --examples --offline -- -D warnings`; same without feature |both exit0; conflict-clippy-isolated/default.log |
| `cargo fmt -p nxtools-archive -- --check` |exit0; conflict-fmt.log. No global/workspace check claimed |
| `dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore` |exit0,0warnings/errors; conflict-final-dotnet.log |
| `cargo build -p nxtools-archive --features isolated-codecs --offline --bin nxtools-archive-worker` |exit0 after apphost/before actual C#execution; conflict-configured-worker.log |
| Exact same C#LIST/RUN/PASS4 |selected4/run4/pass4/skip0/fail0/not_run0,exit0; conflict-configured-list/run.log |
| Spec/context/fixtureselector/diff |exit0;106docs/14owners,82/79context. Reference within32768byte threshold; existing native-contracts35497bytewarning only |

Exact C#filters: `archive conflict fixture actual ZIP decisions`; `archive output fixture rechecked atomic path overwrite`; `archive outcome fixture actual worker operation isolation`; `archive extraction actual Rust handle-session fixture`.

Initial SAME4collection had3PASS/1SKIP/0FAIL because NXTOOLS_ARCHIVE_CODEC_FIXTURES was not set; **not PASS acceptance** (`conflict-final-run.log`). Configured worker=<workspace>/target/debug/nxtools-archive-worker.exe, ZIPfixtures=<workspace>/rust/crates/nxtools-archive/tests/fixtures, CODECfixtures=<ZIPfixtures>/upstream, Pythonoracle=existing Python313/python.exe, then matching worker rebuilt and sameLIST/RUN4/4PASS. Original skipped attempt retained. No unrelated suite/format/password/longpath matrix rerun.

Logs/baseline reside in session scratch `C:\Users\Administrator\.pi-desktop\scratch\949da288-1ef9-46b3-a698-90f543613cf5`, prefix conflict-.107historical task files unchanged before currentcheckpoint update. Helper absent and no archive/test/build processes at close. No100k/largefiles/performance, formalprogram, package/install/sign,UNC/system/sourceRecycle/delete,dependencyupgrade,commit/push or next feature. Stop for independent scoped acceptance; completeB2/PhaseA still not passed.
