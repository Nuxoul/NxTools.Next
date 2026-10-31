# RAR4/RAR5 risk / finite output readmission — bounded implementation outcomes

2026-10-24. **RAR4/RAR5风险与有限预算再准入切片待独立验收**. This is only a remaining slice of
**Phase A — Engine contracts and safety foundation**; no new/reordered phase, full B3/SAFE04/Phase A/F
or first-release acceptance. No independent acceptance was performed here. Current status has one
owner: [phase-a-checkpoint.md](phase-a-checkpoint.md). [Pre-code contract](rar-risk-readmission-contract.md),
[owner code-spec](../../spec/nxtools-archive/backend/rar-safety.md) and the approved implement.md
amendment preserve scope. 7z/ZIP/CLI acceptances and UI/Explorer deferral remain unchanged.

## 1. Original chain, verified metadata and minimal change

Read current PRD/design/implement/checkpoint, scoped Archive/IPC/Protocol/PlatformHost rules, ZIP and
7z contracts/independent acceptances, and existing RAR password/conflict/volume/source evidence.
Reference-first exact API/source follow-up is appended to [archive reference](../../../docs/reference-code/archive.md#rar-riskreadmission-source-admission-follow-up-2026-10-24).
The read-only explorer checked locked unrar_sys0.5.8 / bundled UnRAR7.1, supplied Archiver and SupaZip
and existing attributed data. It performed no edits/build/Cargo; this is reference analysis, not
independent acceptance. A bounded C# test implementer wrote two new fixture files before interruption;
parent took over compilation, corrections, registration and verification. No optional review pass.

Current run_rar already entered SourceSet -> metadata-only rar::execute -> metadata/browser/selection/
codec plan -> extraction_preflight -> PrivateOutput declaration and original Risk/InteractionPort.
It was not missing generic safety wiring. Keep original callback RAR_TEST/null destination -> bounded
native token writes -> full integrity/source recheck -> staged reverse-order commit. No new backend,
protocol, extraction/retry framework, dependencies, vendor or RAR creation.

Exact metadata facts (original rar.rs unchanged):
- DLL pack(1) PackedHeader ABI and x64 size/offset assertions remain. PackSize/UnpSize combine high/
  low32. INT64NDF=0x7fffffff7fffffff becomes None, never zero/u64::MAX.
- Split PackSize is current part only, while UnpSize is member size; do not sum repeated unpacked
  declarations across volumes or use aggregated external-list packed as DLL evidence. LIST skips
  SplitBefore continuation headers; last-part CRC has separate DLL semantics.
- ROADF_SOLID is archive-wide, RHDF_SOLID is current member using earlier data. Even first member
  may have RHDF_SOLID=false in a solid archive. Reliable member packed requires neither archive nor
  member solid and no member split flags. Complete nonsolid/nonsplit members inside a volume family
  are not automatically split solely because ROADF_VOLUME is present.
- Unknown/shared/solid/split packed cannot finance a ratio. Known unpacked declarations can still
  provide a finite max/sum; unknown ratio never exempts actual decode/output limits.
- Archiver lists RAR compressed=None and uses native extract_with_base; SupaZip has no RAR backend.
  Neither justifies replacing the project's original callback/native budget path.

**One concrete product defect, reproduced at two owners:** encoded-source acquisition consumed the
low actual output allowance before original RAR declaration preflight. Minimal four-file repair:
1. worker_codecs.rs switches only run_rar's source entry to Windows-only SourceSet::open_rar.
2. worker_codecs/volumes.rs gives only RAR extract+output_readmission
   max(original output, ResourceLimits::default().max_output_bytes) as finite encoded limit.
3. ArchiveWorkerRunner.cs passes separate rarOutputPreflight only for extract+OutputReadmission.
4. ArchiveVolumeSources.cs probes actual RAR4/5 signature AFTER original source lease, then uses
   max(original, ArchiveWorkerBudget.Capture(default).max_output_bytes). Existing default is100GiB
   at both owners, verified independently, not blindly copied from the7z signature/constant.

The7z branch, non-opt-in/open/TEST/other-format permissions, ordered family acquisition/missing/wrong
volume/source checks and actual output/work/dictionary/metadata/Job limits remain. New behavior can
scan more encoded input under the existing finite default; it does not grant output or memory.
Original rar.rs/worker_password/worker/worker_job/worker_output/interaction/zip_risk/output_readmission/
product_job/ArchiveJobSession and generated defaults match opening hashes. Correct metadata/Risk/
password/conflict/cleanup/publication behavior required tests/evidence only, not product rewrites.

## 2. Evidence root and genuine red -> green

Single evidence root:
`C:\Users\Administrator\.pi-desktop\scratch\7c2ebec3-f1a7-4623-9ed1-760f9adb94f5\rar-risk-readmission-20261024-144756`.
All log/JSON names below are relative to it. Failures and early observations are retained.

| Boundary | Genuine red | Green |
| --- | --- | --- |
| Actual original worker RAR4/5, total1/entry1024 |02-rar-total-red.log: both lacked output_budget_shortfall and exited ResourceLimit before declaration; one test FAIL|03-rar-total-green.log PASS after Rust-only source repair; final28 repeats PASS|
| Original native runner/Job/source/output |07-native-total-red.log: both formats collected, no declaration; retained per-format failure/events JSON, zero publication|08 build +09-native-total-green.log PASS after native source repair; final37 repeats PASS|

These are two owner proofs of the same source/output coupling defect. No fabricated RAR bomb, full
pre-extraction estimation or manually enlarged second request was used.

Retained fixture/tool failures (not product red):11 C# test int/ulong comparison;16 password fixture
incorrectly demanded no prepared empty root (old accepted contract allows empty root, not output);
19 same-operation previous-member ticket was mistaken for cross-operation transport rejection;23
selected test used a missing destination, correctly yielding destination_confirmation_required after
readmission. Parent fixed only test typing/preconditions/assertions.25 passed remaining selected/
encrypted/actual/committed scenarios;27 passed added second_actual. Final38 is unfiltered13 scenarios,
not a sum of old successful directories. Shared Cargo lane refusals/waits started no duplicate build.
34 final-host first failed because scratch PowerShell property-array concatenation joined two paths;
34b passed with separate properties, then only unexecuted final steps continued. No product fix or
failure-log replacement. Intermediate Edit anchor/range mistakes did not authorize unrelated changes.

## 3. Attributed fixtures, independent oracle and negative copies

Original four small RAR inputs come from published unrar0.5.8/data; workspace manifest/README and
UnRAR restriction are copied into owned fixtures. Producer version is not asserted. RAR4 ordinary
VERSION11bytes=`unrar-0.4.0`, packed21; RAR5 ordinary Unicode file7bytes=`foobar\n`, packed7. Their
ratio<=1 cannot be a positive DEBUG Risk sample. EncryptedRAR4/password unrar and header-encrypted
RAR5/password password each contain .gitignore18bytes=`target\nCargo.lock\n`, packed32; original
independent password oracle hashes retained. Synthetic fixture passwords never enter argv/logs.

The prior genuine libarchive volume source at commit79c571f845dbdd250d69bff9d93ae034f674135e retains
original .uu, COPYING, manifest and independently extracted expected hashes. This round copies all
seven exact volumes, not chopped archives: RAR4three physical files total20432bytes/member20111;
RAR5solid four physical files total43216bytes/nine members117398. Existing historical20111/117398
figures are expanded totals, not physical archive length. Only new native metadata/missing-member
checks were run here, not a repeat of prior full volume extraction acceptance.

Fresh genuinely compressed samples were retrieved only from the same fixed libarchive commit's
already indexed .uu URLs, decoded as data (not a compressor), with BSD notice retained:

| Source test data | Archive bytes / SHA256 | Independent output |
| --- | --- | --- |
| test_read_format_rar_compress_normal.rar |14617 /7d32e2ef511d0ade372345440838165df828197eff4d7816880dafde690cce03|Two20111byte HTML files SHAee16390e...fa61ad; testdir/test.txt20bytes SHA5a5f16e0...8e6dd2; original link is skipped/warning|
| test_read_format_rar5_compressed.rar |436 /68f9d7e2662cd5bc5607ba9c6bf1edbf121cf3b08011a7d8643091791750a7c8|test.bin1200 /588870a2dade35c2650fbb7898c9a9c7f21fce7c281198604e8d0c9737f2c375|
| test_read_format_rar5_multiple_files_solid.rar |677 /0aa2c652c06304dedd93019663c1d52d5e85055d5fdb8045bfcc4345b90b4a0f|four4096byte files; full SHA hashes in compressed-manifest.json/native/Rust assertions|

Independent stock7-Zip26.00 SHA4bcc005758c5e604c0aaa257b6097846a4a708e62a9c0c7471fa360fad863f1f
only listed/tested/decoded, never generated RAR.04 ordinary samples:2list+2test+2decode PASS.05 new
compressed data:3list+3test+8individual member-decodes PASS. Raw uu/hash, list/test stdout/stderr and
payload hashes are retained in fixtures/compressed-manifest.json and fetch-compressed-fixtures.py.
RAR4 reliable positive member20111/7091, RAR5positive1200/361; DEBUG(1,1) only lowers original policy.
Native compares exact independent payload bytes/hashes, not self-roundtrip output.

Two **structural negative copies, NOT independently produced RAR** exercise actual callback limits:
- rar4-unknown-size.rar,87bytes SHA5b1ed7f9ca7a6250583c261a20cd1d5d9a00224e284c97c8345f9f6ff654de82:
  original rar4.rar LowUnpSize becomesffffffff and header CRC is repaired; original packed payload unchanged.
- rar5-solid-first-unknown.rar,677bytes SHAf46638f8b21f64e79dcdb729000e7794b964f201c3724ecf9019742fd6ae1542:
  first member FHFL_UNPUNKNOWN set, header CRC repaired; all packed data unchanged.
These prove original actual-byte failure/no retry, not general unknown-size compatibility or new
creation support. Corruption test changes own solid copy byte100 and observes original CorruptArchive.
No executable payload is launched. No RAR4solid sample or all-method/encryption/volume matrix claimed.

prepare-rar-safety-fixtures.py copies attributed sources and optional retained compressed manifests/
uu/payloads, recreates labeled negative copies.39 fresh reproduction under the same evidence root
passed;41 compared all16RAR archive hashes with originals, plus Python AST. A fresh preparation needs
--root <new-existing-owned-root>, --tool <hash-verified stockdecoder>, --volumes <retained volume tree>,
--compressed <this round fixtures>. It is not a product backend or permission to manufacture RAR.

## 4. Final commands and exact results

Initial isolated C# host01 passed before worker/native work. Every Cargo command checked/waited for
the shared cargo/rustc lane and inherited CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0,
RUSTFLAGS=-C codegen-units=256. Other-project builds were observed but never terminated/overlapped.
Final per-command ledger is commands.jsonl; early raw logs and wrapper invocations remain evidence.

| Final check / exact target | Result |
| --- | --- |
| cargo test -p nxtools-archive --features isolated-codecs --test output_readmission_contract --test rar_readmission_contract -- --test-threads=1 |28:3+1PASS,failed/ignored/filtered0|
| cargo clippy -p nxtools-archive --features isolated-codecs --tests -- -D warnings |29PASS|
| cargo clippy -p nxtools-archive --tests -- -D warnings |30PASS default features|
| cargo tree -p nxtools-archive --features isolated-codecs -e features |31final feature tree retained|
| cargo build -p nxtools-archive --features isolated-codecs --bin nxtools-archive-worker |32PASS after final strict/features|
| original write-release-binary-manifest.ps1 explicit scratch outputs |33PASS; checked-in defaults unchanged; no signing|
| final isolated host/tests builds with scratch identities |34b/35PASS,0warnings/errors|
| final-tests/NxTools.PlatformHost.Tests.exe --fixtures --list then same filter archive RAR safety fixture |36/37 exact8names/order,LIST8=RUN8=PASS8,skip0/fail0/not_run0|
| cargo test -p nxtools-archive --features isolated-codecs --test rar_readmission_chain -- --test-threads=1 --nocapture |38:1test/13scenariosPASS,failed/ignored/filtered0; NXTOOLS_RAR_CHAIN_CASES unset|
| rustfmt --check --edition2024 --config skip_children=true, four affected Rust files |40PASS|
| evidence/name/order/cleanup/options/source/hash audit |41PASS|
| python tests/architecture/spec-contracts.py |42PASS113documents/14owners; docs structure,not behavior/native acceptance|

Exact final C# names in order (all prefixed `archive RAR safety fixture `):
ordinary open TEST metadata; low total preflight; conflicts skip overwrite; password preparation;
postpublication cancellation; current budget risk tickets; solid skip integrity; volume packed unknown.
Earlier partial batches/fail-fast not_run rows remain historical, not relabeled eight-case PASS.

Final isolated worker SHA256 **922E699B65AEA3EC4BD45F0BA3881BE746A7A49C957AF6C21DD7C781CF69B65F**,
identical before/after final8 and final13, checked again in41. No rebuild after native evidence.
This scratch Debug identity is not installed27 and must not be copied alone into the protected tuple.

## 5. Actual lifecycle assertions and policy distinctions

Risk native fixtures use real compressed data, ordinary thresholds/no false prompt and loweredDEBUG
approve/decline/cancel/stale. RAR4two risky members/RAR5one have exact declared/packed facts. Before
approval original worker/slot/source/helper remain held with no native writes/publication. Cross-
operation/wrongID rejects; transported larger/wrong-generation/duplicate previous-member replies cannot
satisfy Rust current ticket. Risk confirms exact original ceiling, not an output/work/memory increase.
No-output open/TEST omit DEBUG Risk fields and have no extraction interaction/approval/output. New CLI
execution is neither required nor run; its original noninteractive authority remains unchanged.

Password native cases preserve original no-output TEST Required and RAR5explicit WrongPassword ->
corrected password; then one original-ID extraction. No new C# password state machine. Empty prepared
root is allowed by the original contract; no partial/write/publication before password success.
RAR4/5skip retains old/foreign bytes; overwrite publishes independent exactbytes through original
transaction/journal. Real RAR5solid skip decodes full dictionary chain and emits remaining3files12288;
corrupt solid publishes none. Volume metadata verifies correct full member totals/unknown packed,
and missing last member fails with actual lease cleanup. Original metadata gate needed no change.

Final38 uses original Rust coordinator; RarReadmissionHost is only bounded stdio transport, never
approves/retries/budgets. Cleanup-held asserts ActiveCount1/source+helperprotected/noexited and the
Rust port has no OutputBudget pending; explicit fixture release -> exited/DrainAsync true/active0/
exclusive source+helper reopen -> original AttemptResult. Only then explicit finite approval and new
normal native admission can occur. Original guard3 rejects incomplete cleanup/commits/wrongID/current
limits/hard ceilings/actual failures and invalid/stale approval, no new coordinator policy.

| Final scenario | Native attempts / disposition |
| --- | --- |
| rar4_entry,rar4_total |each2; first zero-effect declared11; explicit approval; exact1file11bytes|
| rar5_entry,rar5_total |each2; declared7; exact1file7bytes|
| cancel,shared_cancel |each1; original first attempt ResourceLimit shortfall, then coordinator Cancelled; no second dispatch (audit's last native error is not coordinator outcome)|
| source_changed |2; mutation after actual cleanup; new native source identity rejects source_changed,zero output|
| selected |2; original held browser binding/ordinal0 retained, existing empty explicit target,one7byte file|
| encrypted_total |2; supplied private original password retained; header-encrypted18 declaration,exact18bytes; not a combined interactive-password/readmission matrix|
| actual |1; labeled unknown RAR4copy reaches source_decoding/UCM_PROCESSDATA,entry10 fails actual11;ResourceLimit/no shortfall/zero commits|
| solid_total |2; genuine solid declared max4096/total16384; explicit finite approval;4files16384 independently hashed|
| second_actual |2; labeled solid-copy selection ordinal1 declared4096,first entry1 shortage; secondentry4096/total4096 decodes unknown preceding block then overflows actual total;ResourceLimit/no shortfall/no third prompt|
| committed |1; RAR4actual publication then cancel;cancelled_with_committed_output,one11byteVERSION,Committed/originaljournal retained|

Serialized request equality is checked in memory without dumping secrets: only operation/control ID,
source/readmission bookkeeping and two output limits differ; logical ID/shared cancellation, password
presence, codepage936, auto_rename, MOTWfalse and selection are preserved. Job memory remains work64MiB
+metadata4MiB+overhead64MiB=138412032bytes, and dictionaries/work/metadata are unchanged. Source binding
uses original native ID/version and Rust SHA, not path equality or ACK. Commit-bearing results never
request readmission, merge outputs, erase journals or rollback previously replaced user files.

## 6. Cost, preservation, remaining limits and stop

Metadata open is not full TEST. Existing RAR extraction has a metadata-only pass then full RAR_TEST
callback decode/CRC/source checks; skip still charges decoded bytes. Existing password interaction
adds whole-archive no-output TEST preparation before extraction, potentially more on wrong-password
attempts. No new full predecode is added for output estimation. This repair adds held-source RAR
signature probe and permits finite encoded scanning to original default100GiB-or-original rather than
prematurely failing at low output. One readmission repeats source hashing/header/metadata/native
admission and any original password preparation; report that extra I/O/decode cost honestly.

Opening baseline31 files/snapshots is round ownership, not cumulativeHEADdiff. Audit confirms original
policy/codec/password/native-session and both generated defaults unchanged, Cargo.toml/lock/archive
manifest unchanged; existing dirty/concurrent work is preserved. Four production repair files only;
three new C# fixture files, two new Rust tests, safe selector registrations, sample preparation and
scoped docs are this round. Original7z/ZIP/CLI acceptances, historical ledger/matrix match openinghash.
Saved delivery **80/80** unchanged including setup/manifest/signature; protected installed **6/6**
unchanged. Opening updater state alreadySequence27/release1295dec1-1429-4a1e-8df2-51ec19547a36/
manifestad5a72a03820678c74c63d0f41c4114359799952c69617bb9710b189952a8f1d; this is read-only file-state
observation, not an installation action, in-memory-image/service/search or liveUI acceptance.
Own executable processes under roundroot0 at final observation. Samples/logs/failures retained.

Dedicated Release runtime test-field rejection NOT RUN: cfg(debug_assertions)/strict unknown-field
and C#DEBUG source proof only; no new Release build/package. Not claimed: realRAR4solid, arbitrary
unknown-size compatibility, old-name genuinevolume runtime, allmethods/encryption/split cross-products,
wrong-volume new native matrix, expanded resource/performance/100k/1GiB/4GiB/long-path, full Desktop/
workspace or already accepted CLI collections. UI/Explorer/HWND/capture/button/foreignCOM/DPI/UIA
remains deferred; no user action requested. TAR/stream risk/readmission and prior A–F/UNC/strictsource
recycling/JOB07/SAFE08/P206/P207live obligations remain; ProductApplyAuthorized=false.

No product launch, package/install/sign/sequence reservation/service/bootstrap/Registry/COM/UNC/
source recycle/P209/dependency/vendor/elevation/commit/push. **Stop: RAR4/RAR5风险与有限预算再准入切片待独立验收。**
