# TAR / stream risk and finite output readmission — bounded implementation outcomes

2026-10-24. **TAR／流格式风险与有限预算再准入切片待独立验收**. Remaining work within the original
**Phase A — Engine contracts and safety foundation** only. No independent acceptance was performed
here; no full B3/SAFE04/Phase A/F/first-release PASS. Current status belongs only to
[phase-a-checkpoint.md](phase-a-checkpoint.md). [Pre-code contract](tar-stream-risk-readmission-contract.md),
[approved sequence](implement.md) and [seven-section owner contract](../../spec/nxtools-archive/backend/tar-stream-safety.md)
record scope. ZIP/7z/RAR and ordinary/bounded-lifecycle CLI acceptances remain closed/history.
UI/Explorer stays DEFERRED / NOT RUN, no user operation requested; package27 remains unchanged.

## 1. Verified original boundaries and actual product changes

Read AGENTS/current specs/PRD/design/implement/checkpoint, prior ZIP/7z/RAR contracts/acceptances and
TAR/stream references before implementation. A bounded read-only explorer followed recorded supplied
Archiver/SupaZip and exact locked tar/flate2/bzip2/lzma-rust2/zstd-safe source; no backend research,
dependency or vendor changes. Findings are indexed in [archive reference](../../../docs/reference-code/archive.md#tar--stream-declaration-follow-up-2026-10-24).
A separate bounded implementer added only two new C# fixture files; parent registered/built/executed
and corrected them. Neither task is independent acceptance; no optional review pass was needed.

The original worker already had output_readmission/PrivateOutput/plan/Risk and native output wiring.
Do not infer missing hooks from old ledgers. Grouped actual boundaries:
- **Plain TAR:** supported fixed/PAX regular member size is known; packed=None, no invented ratio.
  Size excludes headers/padding/extensions/tail. Metadata structure/extents/header checksum are not
  payload checksums. Original staged native transaction, actual copy counts/source checks remain.
- **TGZ/TBZ2/TXZ/TZST:** all use original verified outer anonymous disk spool, then the same TAR
  metadata. Outer stream length has no per-member packed attribution. Original worker spool credit
  is max(output,metadata); inner extraction retains original output limits. This path was already
  correct and was not rewritten.
- **GZ/BZ2:** complete output remains unknown. Gzip ISIZE is per member modulo2^32, not total; BZh
  block/work level is not output length. Actual counters/full end/CRC checks remain original.
- **XZ/ZST:** existing preventive structure scans can supply complete declarations without payload
  decoding. XZ sums all checked Index records and validates block/LZMA2 extents. Zstd requires FCS
  on ALL standard frames; skippable contributes zero, any unknown standard frame makes total unknown.
  Partial known sum is not a reliable whole-output declaration. Full actual decoder remains required.

Two candidate gaps were reproduced before corresponding product repairs:
1. **Complete stream declarations could not reach original finite preflight.** The actual worker
   rejected known XZ/single/all-known double Zstd under entry1/total1 before output_budget_shortfall.
   Minimal product changes are engine.rs, worker.rs,
   formats/xz.rs and formats/zstd.rs:
   - unchanged public open delegates to private open_impl(false); crate-private
     open_for_output_readmission selects complete stream declarations only for original extract+
     output_readmission. Public open/read/TEST still list size=None and keep original limit behavior.
   - original scans now return XZ total / Zstd window-log+optional complete total. Private admission
     defers only premature output-quota refusal until original complete plan/preflight. Original
     metadata/count/structure/dictionary/window checks still complete; no extra full decode/index.
   - XZ/Zstd extraction uses original confirm_zip_entry/Risk before current partial/decode. Only a
     complete logical stream declaration and its own encoded stream length can provide a ratio.
     Actual extract/TEST streaming still calls original limit-enforcing preflight and actual counters.
2. **Native source admission rejected low-output TAR before metadata.** Minimal native changes are
   ArchiveWorkerRunner.cs and ArchiveVolumeSources.cs. Original extract+OutputReadmission selects
   tarStreamOutputPreflight; AFTER original lease, a <=512byte signature probe recognizes supported
   stream magic or ustar/zero TAR header and uses max(original output, original metadata) for encoded
   input only. This is the original finite metadata-sized reservation, not RAR/7z100GiB. Existing
   RAR/7z/family behavior, open/TEST/non-opt-in/other-format permissions and actual output/Job stay.

Only these six production files changed this round. Original TAR/tgz/Gzip/BZip2 adapters, worker_job
spool, output_readmission, interaction, zip_risk, worker_output and isolated accepted-codec owners
match opening hashes. No new C# approval/retry policy, public protocol/CLI/window authority,
password store, unbounded source allowance, output merge or rollback.

## 2. Single evidence root, red -> green and retained failures

Evidence root (all log/JSON names below are relative to it):
`C:\Users\Administrator\.pi-desktop\scratch\7c2ebec3-f1a7-4623-9ed1-760f9adb94f5\tar-stream-safety-20261024-161945`.
Opening baseline is this round's snapshot, not an assumption that the previously accepted workspace
was frozen. All early outputs/failures remain retained.

| Real owner regression | Genuine RED | GREEN |
| --- | --- | --- |
| Actual original worker known XZ/all-known Zstd declaration |03-known-declaration-red.log:2tests/1PASS1FAIL; known.xz/known.zst/two-known.zst all absent from shortfall; unknown guard PASS|04-known-declaration-green.log2/2 after original-scan private admission; final20 repeats|
| Original native runner/Job/source/output, five TAR formats total1 |06-native-low-total-red.log:allfive collected; source_unavailable before metadata,zero output; per-format assertion/events retained|08-native-low-total-green.log1/1 after native finite encoded repair; final30 repeats|

Plain/compound TAR policy/decoder paths did not need product changes. Their later green evidence is
validation of existing behavior, not fabricated red or a rewritten safety framework.

Retained non-product/tool expectations:
- 02-independent-fixtures.log: system libarchive rejects --format in extraction mode, before product
  tests. The first partial fixture directory is retained as fixtures-first-producer-failure.02b uses
  hash-verified stock7Zip forced Zstd decode against literal bytes and passes.
- Mixed Zstd first-frame known declaration4096 can fail an entry3072 structural preflight, so that
  value is NOT counted as actual-decode evidence. The final mixed native/worker case uses6144 <8192
  actual total, above known first-frame declaration; no guessed total or shortfall is exposed.
- 16-second-actual-chain.log: an invalid false-FCS4096/actual65537 Zstd negative returned the original
  CorruptArchive BEFORE the expected ResourceLimit. This failed expectation is retained. The scenario
  is accurately renamed second_false_declaration and final31 asserts CorruptArchive/no third attempt.
  It is NOT native second-actual-overflow evidence; no product change to force the desired category.
- Shared Cargo refusals/waits started no duplicate command, never killed another task.13 briefly
  waited on Cargo package-cache locks after the external-lane check; it later passed. No timeout.
- Intermediate Edit stale-tag/unseen-line/range corrections changed no unrelated files; final source
  stability and per-round diffs are audited separately, not inferred from cumulative HEAD.

## 3. Tiny independent data and declared/unknown variants

prepare-tar-stream-safety-fixtures.py creates owned small CPython tarfile/gzip/bz2/lzma fixtures and
system libarchive Zstd data, never NxTools encoder. Fixed payloads:
- alpha.bin:4096bytes cyclic0..255x16, SHA256c8f5d0341d54d951a71b136e6e2afcb14d11ed8489a7ae126a8fee0df6ecf193.
- beta.bin:2048bytes B, SHA25690c4a574cd6699066e08ec614a847b977c61a5e764d9b0d2b2c4d06583fcf208.
- single GZ/BZ2/XZ/ZST:one4096byte cyclic payload. known.xz:4096zeros. known.zst:4096B.

Plain PAX TAR is10240bytes with two regular members; payload6144, headers/padding/tail are not output.
Compound sizes: TGZ431 / TBZ2 796 / TXZ412 / TZST360 bytes. Ordinary single streams:327/657/324/275bytes.
Exact archive sizes/hashes, producer and command records are in fixtures/manifest.json. CPython
independently decodes Gzip/BZip2/XZ and compares literal/TAR bytes. Windows System32/tar.exe
SHA25692fafed485af0fe4c291457dc3c00d0241f6a136731bab63ef18a3f0c949d37a produces raw Zstd;
stock7-Zip26.00 SHA2564bcc005758c5e604c0aaa257b6097846a4a708e62a9c0c7471fa360fad863f1f forced
Zstd decode independently compares the original bytes. No new package/tool installation.

System-produced Zstd frames omit FCS. known.zst is an explicitly labeled **correct-FCS structural
variant** over unchanged libarchive payload, independently re-decoded. two-known.zst concatenates
those complete declarations; mixed.zst combines known4096 plus unknown4096 and stays wholly unknown.
These are not fabricated per-TAR-member ratios. All logical stream encoded lengths include their
own framing; ordinary public metadata remains unknown size even for XZ/known Zstd.

Corruption/false declarations are separate **structural negative copies**, not valid interoperability:
- bad.gz/bad.tgz flip only original CRC byte[len-8]; bad-known.xz flips own payload byte34 with header/
  Index declaration unchanged. These cannot publish invalid output or authorize another attempt.
- false-size.zst uses a bounded65537byte cyclic payload from libarchive but inserts false FCS4096.
  Encoded278bytes, SHAdd215684ac1c29af19d00f959f7144d6d598a1d1074d040468320561a19806e2.
  It demonstrates that a declaration is not integrity, and decoder CorruptArchive is preserved.
  Actual data is <70KiB, not a large/bomb test. Do not call it a valid compressed-file producer oracle.

33 fixture reproduction under the same evidence root passed; final audit compares all generated
archive hashes with the tested originals. The original main manifest predates the two later negative
copies; their separate attribution JSON and reproduction manifest preserve them without overwriting
original evidence. The new preparation script reproduces both. No executable content is launched.

## 4. Final verification, exact selection and identity

Initial isolated C#host01 passed first. Every Rust command checked/waited for the shared cargo/rustc
lane and inherited CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256.
Other Desktop/network tasks were observed and left intact. final-commands.jsonl retains final Rust
commands/exits/elapsed; per-command raw logs and final-rust/native scripts preserve all invocations.

| Final command / target | Actual result |
| --- | --- |
| cargo test -p nxtools-archive --features isolated-codecs --test output_readmission_contract --test tar_stream_readmission_contract --test tar_stream_readmission_guard -- --test-threads=1 |20:3+2+1PASS,failed/ignored/filtered0|
| cargo test -p nxtools-archive --features isolated-codecs --test xz_read_contract --test zstd_read_contract --test zstd_safety_contract -- --test-threads=1 |21:5+1+4PASS,failed/ignored/filtered0|
| archive strict Clippy --tests with isolated-codecs, then default |22/23PASS,-Dwarnings|
| final archive feature tree then isolated-codecs worker build |24/25PASS; matchingworker rebuilt after strict/features|
| original write-release-binary-manifest.ps1 with explicit scratch outputs |26PASS,no signing/default source overwrite|
| final isolated host/tests builds with scratch identities |27/28PASS,0warnings/errors|
| exact --fixtures --list / same filter archive TAR stream safety fixture |29/30:LIST5=RUN5=PASS5,same names/order,skip0/fail0/not_run0|
| original coordinator ->actualnativechain |31:1test/17unfilteredscenariosPASS,failed/ignored/filtered0|
| task context validation |32PASS82implement/79checkentries; existing oversized-document injection warnings retained|
| independent sample reproduction / scoped rustfmt --check |33/34PASS,seven affected Rust files|
| evidence/name/order/cleanup/source/options/hash audit |35PASS|
| python tests/architecture/spec-contracts.py |36PASS114documents/14owners; docs checks only|

Exact final C# names in order (prefix `archive TAR stream safety fixture `):
1. low total preflight
2. metadata ordinary no risk
3. unknown corruption finite spool
4. postcommit cancellation
5. known stream risk tickets

These are five test cases, not five individual archive operations. Internal representative assertions
cover all nine ordinary metadata/extraction boundaries, three no-output TEST representatives, five
TAR lowtotal cases, unknown counters/compound spool/corruption and ten known-stream Risk interactions.
No repeated format×password×conflict matrix or whole Desktop/CLI collection is claimed.

Final matching scratch worker SHA256:
**2D349D8D8B5BDC0A1FD8EDEA826A617AB595E1B23E68F4F1D9BBBDC570A749CF**.
final-native-worker-hashes.json and final-chain-worker-hashes.json record identical before/after,
Exit0;35rehashes it. No worker rebuild after native evidence. This Debug identity is not installed27,
and must not be substituted alone into the protected release tuple.

## 5. Native lifecycle, original coordinator and evidence levels

All five native tests drive original runner/Job/source/helper/output, with business-frame-only
observation, real native write/publication probes and actual exited/DrainAsync/active0/exclusive source
and helper reopen. Risk uses original operation/prompt/generation/exact ceiling. XZ and all-known
Zstd at loweredDEBUG(1,1) approve/decline/cancel/stale/larger/wronggeneration/crossoperation replies;
production threshold remains >64MiB AND>1000x. No write while pending, exact literal payload only after
approval/full decoder checks. Existing/foreign bytes retained. No read/TEST/CLI silent approval.

Public nine-format listings preserve TAR sizes/packedNone and standalone sizeNone/own encodedlength.
Native ordinary outputs match independent literals; no false prompt. Representative TEST has zero
output. Compound finite2048spool rejects before complete inner declarations, no shortfall/root effects.
Gzip/BZip2/unknownZstd actual counters reject entry3072 without shortfall; mixedZstd uses6144 against
actual8192. CRC corruption retains CorruptArchive/zero publication, compound failure precedes any
inner target effects. TAR payload checksums are never inferred from structure success.

TarStreamReadmissionHost is only bounded fixture stdio transport; no approval/budget/retry owner.
At held-cleanup checkpoint:active1/source+helperprotected/noexited and original Rust port has no
OutputBudget pending. Explicit fixture release ->actual exited/drain/active0/exclusive reopen precedes
original AttemptResult and finite approval. Fresh IDs, original logical correlation/sharedcancel,
codepage936,auto_rename,MOTWfalse and source-bound selected ordinals are preserved. Native source
identity/version plus Rustdigest are carried/rechecked; Job remains work64MiB+metadata4MiB+overhead64MiB
=138412032bytes (spool-limit representative retains its own smaller metadata reservation).

Final31 scenario order and disposition (final-chain-index.json selects only final successful dirs):
- tar_entry,tar_total:2attempts,zero-effect known max4096/total6144 ->explicit finite approval ->2files6144.
- tgz_total,txz_total:2attempts over original verified outer spool, same reliable inner declaration.
- xz_total,zstd_total:2attempts,complete4096 declaration, one4096byte independently compared output.
- second_corrupt,second_false_declaration:2attempts,declared shortfall then original CorruptArchive;
  zero commits/no later shortfall/no third attempt. These are integrity failures, NOT actual overflow.
- cancel,shared_cancel:1native attempt with ResourceLimit shortfall, then coordinator Cancelled;
  no second dispatch. Do not confuse last native error in audit with coordinator cancellation.
- source_changed:2attempts; change only after real cleanup; new native identity rejects source_changed.
- selected:2attempts; original held context/ordinal0 and existing explicit target; one4096byte alpha.
- actual_gzip,actual_bzip,actual_zstd:each1attempt; unknown actual output ResourceLimit,no shortfall/commit.
- spool_limit:1attempt;incomplete outer discovery ResourceLimit,no target declaration/retry.
- committed:1attempt;cancel after actual native publication,onefile/2048bytes in final run,
  Committed/journal retained. TAR commit order is token order, not assumed archive order; fixture
  independently validates whichever2048/4096byte payload actually survives and cleans unpublished token.

**Second actual ResourceLimit limitation:** a complete accurate known stream declaration naturally
prevents ordinary later quota overflow; false FCS native path above returned CorruptArchive. The
separate tar_stream_readmission_guard test injects original first-shortfall/second-actual-ResourceLimit
AttemptResult facts into the unchanged coordinator and proves exactly2calls/no pending approval/no
third. This is policy guard evidence, NOT a native decoder second-overflow PASS. Real unknown first-
attempt counters and native second-integrity failures are separately verified. No product changes
or more speculative negative generators were added to force a desired error category.

## 6. Reading/decode cost, preservation, limits and stop

No new full predecode for budget estimates. Original source revision hashes full encoded input;
TAR metadata seeks ordinary payload while reading headers/extensions/tail. Test/extract reads data.
Compound open already fully decodes/checks entire outer stream into anonymous finite spool; extraction
repeats original spool decode before TAR copy/publication. Readmission adds a fresh source/header/hash
and normal native admission; it does not reuse/merge earlier outputs. XZ declaration uses existing
all-index/block/chunk scan; Zstd uses all frame/block structure, not first-frame guess. Actual extraction
retains its original preventive scan plus full decoder. Native opt-in adds <=512byte signature read
and finite metadata-sized encoded scanning, not output/work/dictionary/metadata/Job permission.
Existing password owners unchanged, not applicable/enabled for these unencrypted stream formats.

35opening source/report snapshots and saved bytes establish round ownership, not cumulativeHEADdiff.
Audit confirms original TAR/tgz/Gzip/BZip2/coordinator/ticket/native-output/accepted-codec owners,
dependencies and generated defaults unchanged; six production repair files only. Current Program/
TestEffects diffs contain this round's five exact registrations/entry PLUS the separately owned
FloatingAssistantCoordinatorTests.PickerWaitingForUserIsNotBackgroundTimeout registration/effect.
That concurrent floating-picker change is retained, not attributed or accepted here. Other simultaneous
Desktop/network/protocol work remains unrelated; no whole-workspace frozen/compiled/tested claim.
Historical ledger/matrix and existing ZIP/7z/RAR/CLI
acceptance reports match opening hashes. Original checkpoint history is retained below current state.

Saved package27 delivery80/80 and protected binaries6/6 remain unchanged; updater state matches opening
Sequence27. These are read-only file observations, not installed-image/service/search or UI acceptance.
No formal product startup, service/bootstrap/Registry/COM/UNC/recycle/P209 operation, package/sign/
sequence reservation/install/dependency/vendor/elevation/commit/push. Scratch inputs/failures remain.

Not run/not accepted: dedicated Release test-field runtime rejection (cfg/strict parser source proof
only), native second actual-overflow path, legacy extension-only TAR low-output source expansion,
all filter/method/concatenation/encryption/large/long-path/performance matrices, full Desktop/workspace
or already accepted CLI/codec collections, realUI/Explorer/HWND/capture/button/foreignCOM/DPI/UIA.
TAR remains without payload checksum; Zstd checksum exists only if supplied. Broad resource/native/
trustedUNC/strictsource-recycling/JOB07/SAFE08/P206/P207live and earlier A–F obligations remain;
ProductApplyAuthorized=false. No remaining implementation/build blocker in the bounded tested slice.

**Stop: TAR／流格式风险与有限预算再准入切片待独立验收。** No automatic packaging or next debt item.
