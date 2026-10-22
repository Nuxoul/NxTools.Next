# 7z / TAR / TGZ / TXZ creation — bounded Phase A evidence

Date: 2026-10-18. Phase: **Phase A — Engine contracts and safety foundation**.
Scope delivered: private worker/native tree creation and isolated interoperability/lifecycle
verification. This is NOT Phase A completion, Desktop/public product integration, broad resource,
user/native/release acceptance. [Single current status](phase-a-checkpoint.md).

## Later independent finding and repair

The integration below is historical, not the typed-I/O repair's execution. Independent
[acceptance](tree-create-acceptance-review.md) found real verifier spool read errors collapsed
to CorruptArchive; it did not establish failing-disk data loss or invalid publication.
[Repair evidence](sevenz-create-typed-io-remediation.md) retains actual red2pass/5fail -> green7/7,
then this repair's final93Rust/89C#and ZIP64once, with first underlying verifier I/O provenance,
exact owned cleanup/source closure and unchanged corruption/publication contracts.

## References and change boundary

[Exact source/API investigation](research/tree-create-capability.md) records actually inspected
legacy NxTools, Archiver create dispatcher and SupaZip7z/TAR adapters, locked public APIs,
licenses and resource limitations. No general legacy archive UI/native lifecycle was found by
that bounded search. No proprietary Bandizip decompilation, dependency upgrade/source patch,
new runtime, registry/service/mapping/user archive operation, UI, package/install, commit or push.

Reuse CreationPlan/CreationSource, immutable host admission, held no-reparse source IDs,
Prepare→source collision admission→create→same actual held collision recheck and unchanged
native partial/read/flush/recheck/single atomic path publication. Approved overwrite still
accepts target-recheck-close→publish TOCTOU; no strict identity-CAS, delete/truncate/backup-first
or blind retry. Created legal directories remain on failure; cleanup owns exact unpublished
partials and resources, not a recursive filesystem rollback.

worker_sources extracts only the existing ZIP private native-ID adapter. Its new lazy reader
opens/closes one actual native source at a time even when solid readers are retained; exact
length/EOF/SHA256 and first typed I/O/cancel failures survive codec wrappers. ZIP encoding,
crypto and read behavior remain unchanged. A shared payload-size correction below is tested.

## Executable options / finite runtime

- Schema2 create_7z requires tree_options(strip_single_root,exclude_patterns) and
  sevenz_options(solid,encryption=none|aes256,encrypt_header). C# optional values get explicit
  defaults; names default unencrypted. Password/encryption/header choice must agree. LZMA2
  fast1/normal6/maximum7, single thread only; no Store or caller thread count.
- create_tar has tree_options and no level/password. create_tgz maps1/6/9 and create_txz1/6/7;
  both accept fast/normal/maximum only and reject encryption/7z/ZIP options.
- Old ZIP/extract/Gzip/XZ request serialization omits new fields; unsupported combinations reject
  symmetrically. source_tree_v1 plus matching sevenz_create_v1/tar_create_v1/tgz_create_v1/
  txz_create_v1, lease/budget/partial_read are required.7z only negotiated with isolated-codecs.
- Tree exclusions and source lists are captured into bounded private storage before accepted.
 32roots/native65536observed records/64depth and existing source/count/metadata/output limits
  remain. No all-source handles, ordinary source-path reads or final-path writes.
-7z uses public ArchiveWriter::new on a finite anonymous exclusive delete-on-close encoded
  Write+Seek spool. No plaintext source pre-spool for encryption, whole archive Vec, MT writer,
  auto_finish, prepare_block or unbounded path helpers. Work admission uses max(data dictionary,
  default preset6 LZMA header dictionary)×16+32MiB before codec construction. Thus fast also
  reserves header encoding memory;default512MiB permits all chosen presets. This is a
  conservative preventive bound plus Job containment, not measured peak RSS/performance.
-7z backend metadata reservation is checked against admitted metadata before construction,
  covering retained entries/blocks/names/header buffers. Explicit finish rewrites only the
  anonymous output spool. TAR uses explicit plan/Header/Read and follow_symlinks(false), never
  append_file/append_dir/unpack. Raw TAR and encoded bytes each<=max_output_bytes, including
  header/padding/end blocks; an empty archive still needs budget. Fixed64KiB transfer/readback.
- Encoded spool→native partial→full held-partial length/SHA256 comparison→bounded structure and
  complete decode→exact original names/kinds/sizes/SHA256/CRC where supplied→source/cancel
  recheck→unchanged native publication. TAR.GZ/XZ outer complete integrity precedes inner TAR
  verification; existing strict fixed-header parser/copy is reused. TAR has no format payload
  checksum: source hashes are create-time verification, not a new TAR integrity guarantee.
- Diagnostics reuse accepted/started/creation_planning/encoding/verifying/committing and one
  terminal under operation correlation; private reads/data never become business/log payload.

## Red → green evidence retained

1. Each format got one actual C# Start/Job/Rust-worker/native test BEFORE enabling it:
   tree-7z-red-test.log, tree-tar-red-test.log, tree-tgz-red-test.log, tree-txz-red-test.log.
   Each listed1/run0 first, then selected1/run1/pass0/skip0/fail1/not_run0,exit1,
   invalid_request at unsupported operation admission. After minimal implementation, same
   fixtures passed; no hypothetical bulk format suite preceded those slices.
2. Independent7-Zip26.00 -slt has no Folder field and may omit Windows Attributes. Oracle now
   strictly compares exact list members/sizes then materializes to an owned temp folder to
   inspect actual kinds/bytes/hash. Localized password prompt bytes are separated by ASCII
   delimiter; member metadata remains strictUTF8. No silent name replacement/ignored decoding.
   Early oracle failures retained in tree-7z-green-test.log, tree-7z-green-retest.log,
   tree-7z-oracle-detail.log, tree-7z-options-test.log. Product checks were not weakened.
3. Real long-CJK/multi-root fixture failed CorruptArchive before commit. Short-name minimized
   variant passed; synthetic zero-directory-size protocol probes did not reproduce. Fixed
   DEBUG-7ZVERIFY stage probes localized exact size comparison, then ALL probes removed.
   C# stores the actual directory object's physical Size for source-version checks; it can
   be4096when NTFS index grows. PlannedSource.size incorrectly treated that as archive data.
   Public plan4096directory red:0pass/1fail/0ignored,3filtered,exit101,expected0actual4096
   (tree-directory-size-red.log). Fix makes archive directory payload0while preserving original
   SourceEntry and C# fullID/size/ChangeTime revalidation. Public4/4 then original real longname/
   multi-root/immutable admission/empty trees all pass. ZIP writer/reader was not rewritten.
4. Empty stripped7z tree with max_output_bytes16 red:1run/0pass/1fail,exit1,wrong generic
   worker_failure (tree-empty-budget-red-test.log). Writer constructor seeks32and wrapped the
   retained ResourceLimit. Check first retained failure immediately after constructor result,
   just as later codec stages do. Exact regression now resource_limit/0commits/cleanup/no final
   for all4formats. No budget/default relaxation.
5. Another session ran cargo build --release in a different session scratch/wh-probe target.
   Our preflight stopped before any integration command (exit2,RUN0); retained
   tree-create-blocked-ledger.txt. Never killed it or launched concurrent Cargo. After it ended,
   actual final batch below ran once, serially. No wrapper timeout/duplicate owned build.

## Final integrated verification

All direct Cargo commands inherited CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,
RUSTFLAGS=-C codegen-units=256. Scratch tree-create-verify.ps1 records exact commands/filters;
ledger tree-create-verification-ledger.txt START17:35:07→BATCH_PASS17:47:12(+08:00).
Final logs tree-final-*.log. Commands ran in order; all exit0:

| Gate | Actual result |
| --- | --- |
| cargo test -p nxtools-archive --features isolated-codecs --offline --no-fail-fast |86passed/0failed/0ignored |
| same-feature worker build |exit0 |
| cargo clippy -p nxtools-archive --features isolated-codecs --tests --offline -- -D warnings |exit0 |
| cargo fmt --all -- --check |exit0 |
| cargo check --workspace --tests --offline |exit0;check is NOT workspace tests |
| extensions --test archive_import --offline |7passed/0failed/0ignored |
| clipboard --lib archive_ --offline |5passed/0failed/0ignored,39filtered |
| C# test project + referenced product build --no-restore |0warnings/0errors,exit0 |
| Exact original80+new9 --fixtures --list |selected89/run0/pass0/skip0/fail0/not_run89,exit0 |
| Same exact89 execution |selected89/run89/pass89/skip0/fail0/not_run0,exit0 |
| Actual4GiB+17FastDeflateZIP64 |RUN/PASS once in this continuation's final89;Job/native publication and Python streaming length/SHA256 |

Default-feature strict archive Clippy also passed (tree-default-clippy.log) before the last
small constructor error-preservation fix; final isolated strict lint includes that fix.
Directed worker fault/wire:1+8passed (tree-worker-faults.log); final suite includes them.
Directed tree fixture final list9/run0 then9run/9pass/0skip/0fail,exit0 before integration;
logs tree-final-focused-{worker,build,list,tests}.log.

New9fixture entries execute101bounded real worker operations within the final89 harness cases:
7z19creates;TAR/TGZ/TXZ7creates;29safety(7z7,TAR7,TGZ7,TXZ8);layout/immutable/empty/budget12;
diagnostic4;produced-archive open/test/extract30. These scenario operations are NOT extra
harness test counts. Unsupported Start rejects are not worker operations.

Independent expectations come from original synthetic literal names/kinds/bytes, not backend
self-round-trip. Stock7-Zip26.00 SHA256
4bcc005758c5e604c0aaa257b6097846a4a708e62a9c0c7471fa360fad863f1f;
CPython3.13.2 SHA2567d96a4ed35d6e596cd9dd8933feafc66349cc21f75be3b15c89fd336e50140c1.
Passwords stdin only; no install/download/PATH changes. Product reads additionally exercise
created plain/encrypted/solid/header7z and TAR compounds through open/test/extract with exact
native names/kinds/file counts/bytes. Diagnostic sentinels absent; fixed phase counts bounded.

Large preflight: available16508MiB,commit headroom16169504768bytes,temp disk32331214848bytes.
Postbatch inspection:0Cargo/rustc/worker/test-host/7z processes,fixed helper absent,0owned
archive-tree-create roots; partial/no-final/previous-target assertions passed inside fixtures.
Temporary DEBUG production probes and scratch synthetic probe archives were removed; only
scratch diagnostic scripts/logs remain. Static/document/context gates are recorded below.

## Remaining / no phase completion

Full Phase A ENG/SAFE/IPC/P201 exits, broad encoder/decode RSS/cancellation latency/large7z/TAR,
100k/million-entry/ZIP64large-count,long-path/UNC/storage/native-user resources remain open.
Timestamps/attributes/MOTW,guarded recycle/source-delete eligibility,product IPC/package helper
mapping/FIFO/consumer backpressure/conflict integration remain. No B/UI or A–F first-release
completion,Release/install/native Shell/service/user acceptance is claimed.

## Final static gates

spec-contracts.py:PASS101documents/14owners;gpui-component-catalog.py:PASS79pages/79rows/
78locked;task.py validate:PASS34implement/31check,zero truncation warnings;
platformhost-test-selection.ps1:PASS(source only). Scoped21code/evidence files:0trailing-
whitespace/conflict issues;production/test DEBUG searches:0;git diff--check:exit0.
Logs tree-final-{spec,catalog,context,selector}.log. No documentation gate certifies product,
native-user,performance or release acceptance.
