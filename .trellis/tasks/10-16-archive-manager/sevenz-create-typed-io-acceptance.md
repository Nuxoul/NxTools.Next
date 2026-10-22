# Independent acceptance — 7z creation verifier typed-I/O repair

Date: 2026-10-18. Canonical phase: **Phase A — Engine contracts and safety foundation**.
Scope: accept the bounded repair described in [repair evidence](sevenz-create-typed-io-remediation.md).
Original [P2 finding](tree-create-acceptance-review.md) remains historical and unchanged.

## Verdict

**PASS for this repair; the original P2 can be closed.** No concrete new blocker was found in
this bounded review. This is not Phase A completion, Phase B permission or A–F/release acceptance.
Acceptance modified no product/test source; it adds this report only. No additional subagent
review was used in this acceptance.

## Source and regression assessment

Read the live worker_sevenz_create.rs, its new tests.rs, sevenz_budget.rs and both header
preflight callers, ArchiveError conversion and applicable owner/error/verification contracts.

- Production create passes its existing File unchanged through a private create_with_reader seam;
  no wire injection flag, dependency update, new runtime or ordinary final-path authority.
- VerificationInput retains the first underlying Read/Seek category before header preflight,
  ArchiveReader construction or decoder wrapping. verify checks this retained category on every
  return, so parser/callback corruption projections cannot erase storage origin.
- Raw storage Other/InvalidData -> IoFailure, OutOfMemory -> ResourceLimit, existing ArchiveError
  preserved; untagged codec data errors remain CorruptArchive. Interrupted is not latched.
- Real EOF/truncation, malformed structure, CRC/hash/plan mismatches remain rejection paths.
- Shared encoded_header accepts Read+Seek and obtains length by seek; both callers rewind before
  constructing ArchiveReader. Budget/CRC/extent checks and native publication order remain intact.
- The new tests call the real production creation function through its private input seam, after
  actual encoding and complete held-partial length/hash readback. They inject failures at signature,
  budget header, parser header, payload and seek points and assert requested points are reached.
- Assertions cover zero commits, exactly the owned abort token, closed source/partial handles,
  removed owned partial, unchanged foreign partial/source/old target and absent new final.
  Normal publication and corruption/truncation controls prevent a blanket IoFailure workaround.
- Seven tests exercise31 small transactions, including encrypted parser/payload paths and typed
  Cancelled/ResourceLimit/SourceChanged/OOM cases. Transaction counts are not harness-test counts.

The prior implementation's red2pass/5fail record was reported in the repair evidence; this
acceptance did not revert product source or independently rerun the historical red revision.
The current green and complete integration below were independently executed.

## Independently executed checks

Scratch logs/script prefix: `typed-io-acceptance-`, plus `typed-io-acceptance-focused.log`.
Every Cargo inherited CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0,
RUSTFLAGS=-C codegen-units=256. Machine-wide Cargo/rustc checked before each build step.

| Check | This acceptance's actual result |
| --- | --- |
| focused --lib worker_sevenz_create::tests, isolated-codecs, serial | 7 passed /0 failed /0 ignored /3 filtered, exit0 |
| archive complete isolated-codecs --offline --no-fail-fast, serial | 93 passed /0 failed /0 ignored,20 result groups, exit0 |
| same-feature actual worker build | exit0 |
| isolated and default strict archive Clippy including tests | both exit0 |
| workspace fmt --all --check | exit0 |
| workspace check --tests --offline | exit0; NOT workspace tests |
| extensions archive_import | 7 passed /0 failed /0 ignored |
| clipboard --lib archive_ | 5 passed /0 failed /0 ignored |
| C# product/test build --no-restore | 0 warnings /0 errors |
| exact original fixture union --fixtures --list | selected89/run0/pass0/skip0/fail0/not_run89 |
| same exact fixture union execution | selected89/run89/pass89/skip0/fail0/not_run0 |
| LIST vs RUN name comparison | 0 differences |
| actual4GiB+17 FastDeflate ZIP64 | RUN1/PASS1, once only in this acceptance |
| retained independent missing-internal-output probe | OutputCollision,0 commits,output_created=false,original two source members only |
| specs | PASS101 documents/14 owners |
| component catalog | PASS79 pages/79 rows/78 locked entries |
| task context | PASS36 implement/33 check |
| PlatformHost selector source gate | PASS |
| git diff --check | exit0; unrelated existing CRLF normalization warnings only |
| scoped changed Rust production/test whitespace/conflict/DEBUG hygiene | PASS |

All compiled/check steps completed normally without wrapper timeout. Original89 retains actual
Start/Job/native creation/read lifecycle, independent 7-Zip/Python manifests/types/sizes/SHA256,
overwrite/cancel/source/budget/source-binding regressions. It is not unrestricted C# acceptance.

7-Zip existing26.00 explicit portable path SHA256 independently matched
`4bcc005758c5e604c0aaa257b6097846a4a708e62a9c0c7471fa360fad863f1f`.
No oracle installation/download/PATH change; passwords remain private stdin.

### Build-channel interruption, retained honestly

Ledger starts20:39:12(+08:00); archive finished20:39:27 with93/0/0. The next worker-step guard
observed a busy Cargo/rustc lane and stopped before launching worker. Reinspection found the lane
empty; process ownership was therefore not established, and no external-project attribution is
claimed. Nothing was killed. Resume20:40:35 continued only unexecuted worker-and-later steps;
BATCH_PASS20:50:11. The guard interruption remains in the same ledger, not rewritten as an
uninterrupted batch or a test failure. Archive was not rerun; ZIP64 ran only once.

## Cleanup and limits

Postbatch process inspection found0 Cargo/rustc/archive-worker/test-host/7z; fixed helper absent;
0 inspected archive-tree-create/source-binding/independent-probe roots. Free disk C29.2GiB/D56.9GiB.
The retained independent probe was rebuilt against current PlatformHost and returned:

```json
{"mode":"nonexistent_output_inside_source","Category":"worker_failure","FilesCommitted":0,"CommitState":null,"CleanupComplete":true,"error":"OutputCollision","names":[],"body":null,"final_exists":false,"output_created":false,"source_members":["approved.txt","replacement.txt"]}
```

No real hardware failure was manufactured; injected failures prove their bounded code boundary,
not universal disk-fault behavior. No actual data loss or invalid publication is claimed.
No release/package/install/commit/push, user archives, registry/default association/service/drive
mapping operations. Unrelated dirty changes were preserved. No new broad reference investigation
or proprietary source access is claimed; current owner/reference evidence supplied context.

Remaining Phase A is unchanged: timestamps/attributes/MOTW,guarded recycle/source-delete eligibility,
product IPC/capabilities/helper-package mapping/FIFO/backpressure/conflicts and broad resources,
performance/large-count/long-path/UNC/native acceptance. Full workspace tests, unrestricted C#,
UI/DPI/UIA/Shell/user and release acceptance were not performed. This P2 is closed without claiming
those unfinished gates have passed.
