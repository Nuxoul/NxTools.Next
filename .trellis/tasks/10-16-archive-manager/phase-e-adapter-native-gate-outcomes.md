# Phase E — ZIP container partial implementation and native same-source gate

2026-10-22. **PARTIAL implementation; BLOCKED at a new actual native source/commit ownership decision.**
Phase E/P202/P203 NOT complete or independently accepted. The earlier locked-ZIP backend decision
is CLOSED by the user; this report does not ask to repeat it. Only [checkpoint](phase-a-checkpoint.md)
owns current status. [contract](phase-e-implementation-contract.md) and [plan](implement.md) record
approved route/order/new-target-only publication. Historical outcomes/acceptances are unchanged.

## Delivered source boundary (not production activation)

- New Rust zip_container parses original central records/disk+offset/EOCD/standard ZIP64 fields,
  maps exact .zip/.zNN family (<=1024), bounded64KiB encoded spool/copies and explicit byte/metadata
  budgets. It normalizes disk coordinates for existing ZIP metadata/crypto/codec/integrity checks;
  no new payload compression or encryption implementation, no whole archive Vec/plaintext spool.
- Snapshot revision retains each original volume's path/size/mtime/digest and combined digest;
  old snapshot test refuses replaced source. These portable hashes are NOT native file-ID leases.
- Explicit ArchiveEngine::with_zip_volumes opt-in only. Default/with_limits/production worker do
  NOT opt in until C# all-volume native admission is implemented. No fake production split UI.
- zip_volume_candidates returns anonymous unpublished candidate files, copies existing encoded
  local/payload records, recalculates member/central/end disk coordinates/counts; data crosses
  disks while structural headers fit intact. It does not select final paths,flush Windows output
  handles or publish a volume. Production worker/native publish is NOT connected.
- Changes: archive src/{zip_container,engine,model,lib,worker_job}.rs, tests/zip_volume_contract.rs.
  worker_job's only edit adds an empty per-volume list to its existing synthetic codec-plan revision.
- Added C# ArchiveZipEditCommitTests with exactly3fixture registrations in Program.cs. Uses original
  ArchiveSourceLease and ArchiveOutputSession only,actual GUID-owned native files and candidate
  readback. C# production source/output/share flags/rights unchanged. New failure proof preserves
  source SHA256/native state; tests still require successful edit publication,not expected32PASS.
- Reference note and source-backed ZIP/native-owner specs updated with partial status and failure.
  No Desktop/Protocol/IPC product changes, manifest/lock/vendor/dependency change or alternate engine.

## Real reds and small interoperability

1. Public production engine genuine2disk source fixture tests first: **0PASS/2FAIL**,UnsupportedFormat
   on.z01 and MissingVolume on.zip. Same tests then2PASS. Final explicit opt-in small collection4PASS
   covers first/last/missing/replaced/default-off/wrong order/directory per-disk count/encoded budget.
2. Candidate generator real-boundary test initially **0PASS/1FAIL UnsupportedOperation** before body;
   then1PASS. Current exact candidate test still1PASS after footer record refactor.
3. Existing locked ZIP encoder produces a600byte Store payload; container writer makes6real volumes
   with <=128bytes each, not byte-chopped single-disk metadata. Hash-verified independent7Zip26.00
   l -slt/t/x all exit0,Multivolume=+,Volumes=6,member disk0/offset4. Extracted payload600bytes SHA256
   `DB4F2AC25D140369324DBED60D7B8E314FDF1252C171F8513FB7DBF5CC92E88D`; original pattern is n%251.
   No self-reader-only claim. Small same-implementation test also verifies full original CRC streams.
4. Final small original engine14/ZIPboundary2/publicvolume4 = **20PASS**,0fail/ignore; existing100k
   case explicitly filtered1. Candidate1 is separate. No full Desktop/format×policy matrix.

## New concrete native blocker (not guessed from flags)

Actual chain: ArchiveSourceLease.Open(original ZIP) -> held read-only file/no-reparse parents ->
CaptureTargetIdentity -> BeginFile(expected,overwrite:true) -> complete staged candidate+readback ->
ArchiveOutputSession.Commit -> final full target recheck -> original FileRenameInfoEx
REPLACE_IF_EXISTS|POSIX_SEMANTICS. With original source file lease live, native replacement returns:

**stage=replace_file, native_code=32 (sharing violation), state=FailedBeforeCommit.**

No sharing/right changes,early source close,delete/truncate/backup/retry was attempted. Success is
not inferred from POSIX_SEMANTICS; failure is actual production native boundary behavior.
Failure-proof fixture confirms:
- original before/after SHA256 both
  `579FFEC4A8A21CD5CC2612BD52E382A35FB8B313804B54BBCF04D445981BF48D`;
- held native identity unchanged; journal FailedBeforeCommit,replaced=false;
- candidate disposed/owned partial cleaned by original resource lifetime; no final publish.

### Exact C# selection/execution accounting

Initial same-set LIST selects3,RUN0; one combinedRUN then fail-fast selected3/run1/pass0/fail1/
skip0/not_run2. This historical partial run is retained,not combined as full3coverage.
After adding read-only failure-proof assertions, rebuilt isolated host,final isolated feature check
then matching worker build. Each exact name had its own LIST followed by SAME exact-name RUN:

| Exact name | Final selected/run/pass/skip/fail/not_run | Result |
| --- | --- | --- |
| archive ZIP edit native held source same path commit |1/1/0/0/1/0|replace_file/native32/FailedBeforeCommit; unchanged source proof retained |
| archive ZIP edit native same path cancellation truth |1/1/0/0/1/0|precommit cancel assertions completed; late-cancel subcase cannot reach native_published because same native32; whole case FAIL,not partial PASS |
| archive ZIP edit native changed revision refusal |1/1/1/0/0/0|after deliberate lease retirement,identical bytes/different native fileID -> target_changed/FailedBeforeCommit,foreign/original bytes retained |

Final unique set **LIST3=RUN3; PASS1/SKIP0/FAIL2/NOT_RUN0**. This is not an all-green native slice.
All failed GUID fixture roots retained. Matching worker SHA256 before/after final native exactly
`2C15CC9142B4E3BD704AC8694CC4735840692A3A84E9AC5F5B1C4CA313CBDDCD`.
These three tests call native boundary directly,not actual worker jobs; building worker does not
turn them into worker-job evidence. No product UI/GUI/Shell/source recycle is exercised.

## Compiled/static checks and retained failures

Unique run directory:
`C:\Users\Administrator\.pi-desktop\scratch\7c2ebec3-f1a7-4623-9ed1-760f9adb94f5\phase-e-implementation-20261022-102842`.
Per-command independent logs/append-only commands.jsonl; opening481-file baseline and native/oracle
artifacts retained. No log overwrites. No duplicate concurrent Cargo; every Cargo command checked
lane and inherited jobs1/incremental0/RUSTFLAGS=-C codegen-units=256/full-operation timeout.

| Check | Actual result |
| --- | --- |
| Final candidate container test |1PASS/0fail |
| Final small engine/boundary/volume |14+2+4PASS,0fail;100k filtered1 |
| Isolated Archive strict Clippy --lib --bins --tests |PASS after local footer argument-object correction; initial too_many_arguments exit101 retained,no allow suppression |
| Final isolated feature check --tests |PASS |
| Full workspace check --tests |PASS,compilation only |
| Full fmt --check |PASS |
| Isolated C# host build |PASS,0warnings/0errors |
| Matching isolated-codecs worker after feature check |PASS,hash stable across native |
| Native exact three |1PASS/2FAIL/0skip/0not_run,not PASS |
| spec-contracts / component-catalog / task validate |PASS,108docs/14owners,79pages/78locked entries,82implement/79check; existing context-length warnings retained |
| Final scoped diff/new report links/preservation/native proof |PASS;473/481opening files unchanged,8expected existing differences only;new files/spec edits separately listed above;worker hash/payload/native unchanged-source proof checked |
| Full Desktop |NOT RUN: source not converged/new explicit native decision needed; prior979/979 remains Phase D history |
| Real GUI/Shell/DPI/UIA/user/release |NOT RUN,no safety bypass |

**Scope deviation retained:** earlier `cargo test --test engine_contract ...` unintentionally included
existing `default_budget_lists_one_hundred_thousand_independent_entries` despite user's no100k rule.
That one existing100k test actually ran/passed in first15+2+2 collection,3.64s engine suite; this was
an incorrect test selection,not authorized performance evidence or a needed gate. It was not run
again; final14engine tests explicitly skip it. No other large-file/performance test was run.
Initial new-function dead-code warnings existed before a narrow public candidate method used it;
final strict Clippy is clean. A misplaced method insertion was corrected before compilation; one
unseen-line Edit was rejected and re-grounded. Failed attempts remain in tool/log history.
No optional broad review; one bounded C# fixture implementation delegate only,no recursive work.

## Requirement disposition and remaining work

| Requirement | Current status |
| --- | --- |
| P202 ZIP real volume read/write |PARTIAL container implementation/small interop only. Native all-volume identity leases,protected worker admission,ZIP64/encrypted representative evidence,surplus-family conflict/no-replace race,flush/set validation/per-volume publication and cleanup journal remain NOT delivered. Production default remains off. |
| P2027z read/create, RAR4/5 actual families |NOT implemented/rerun this round; exact approved order stopped at ZIP native gate. No inference from ZIP;no RAR create. |
| P203 add/delete/rename/new-folder/comment |NOT implemented. Container preservation groundwork only; same-source native commit success unproved/actualFAIL32. No blanket editable-ZIP rejection marketed as completion. |
| P203 metadata/encryption rules |Approved operation-specific preserve/rewrite/refuse contract recorded; actual editing allowlist/Unicode-CRC/ZipCrypto/AES metadata preservation not yet implemented/verified. Prior raw_copy negatives remain. |
| P203 queue/UI/drafts/lifecycle |NOT implemented; no fake enabled action,old ID mutation,second framework or unverified success refresh. |
| P204/P205 |Unimplemented by scope;no watcher/image/service. |
| Phase F |Not entered. |

A/B unresolved obligations,JOB07/SAFE08 original-object recycling,A–D real GUI/native/user acceptance
unchanged. No source recycling/deletion,registry/service/system mutation,package/install/signing,
commit/push or A–F/first-release completion claim. Fixture cleanup is only owned synthetic trees.

## Decision requested: edit-only source lease handoff (proposal, NOT implemented)

The already-approved ZIP backend remains unchanged. New evidence requires an explicit edit-only
source/commit ownership amendment before P203 can publish:

**Recommended narrow proposal:** after complete candidate verification and worker closure of original
ZIP readers, C# performs final source fullID/version/hash and target recheck under original parent
handles. Transition the exact edit-source FILE lease only at the existing commit linearization
boundary; retain protected parent/candidate handles,then one original atomic path replacement.
State explicitly the new final-check-to-publication TOCTOU interval; failed/unknown native outcomes
retain truthful facts with no automatic retry. Do not globally allow source writers/delete sharing,
release source protection while generating candidates,move/delete/truncate original,or add backups.

This changes the current continuously-held-source invariant for same-source edit ONLY; existing
ordinary extraction/creation remain untouched. No Windows elevation/new API/backend is proposed.
Precise ordering,which worker source handles are closed and barrier/failure/cancel/changed-source
regressions must be specified before code; no success is promised until actual native tests pass.
**Wait for explicit user confirmation** rather than silently implement that lease transition or
change rights. This is a genuine new native gate,not the Phase E independent completion checkpoint.
