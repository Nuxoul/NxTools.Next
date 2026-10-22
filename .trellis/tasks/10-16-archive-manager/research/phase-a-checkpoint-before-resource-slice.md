# Phase A — Engine contracts and safety foundation: partial checkpoint

Date: 2026-10-19. Status: **Phase A in progress, not passed**. Only this file owns current state.

## Current state — product read P2 cancellation / overflow epoch repairs verified

[Original independent findings](product-read-acceptance.md) remain immutable;
[repair owner/reds/current gates/limits](product-read-p2-remediation.md) supersedes earlier
[integration evidence](product-read-evidence.md). Rust owner/typed pipe/shared C#session/original
runner/Job/actualworker remain open/test/ack/cancel only. Originalconnection router reserves2
read lifecycles and coalesces cancellation beyond Drop/localfailure, fair start-before-cancel
delivery; actualcleanup or originalconnection retirement alone frees it. Event8/wake1emit/
overflow/drain/reset is one synchronousepoch boundary; no lockawait/oldflag/newrequest poisoning.
Ordinaryreceipt saturation fails/closes THAT whole pipe; actualC#EOF drains originalJobs,not
local-onlydisconnect. Oneboundedreview found that order; controllingIPC+realchain reds repaired.

Actual2026-10-19:Archive138pass/0fail/0ignored(24nonemptygroups),Protocol50,IPC29;all0fail/ignored.
Cancellation10/consumer5/realchain11 INCLUDEDin138. Default+isolated Archive/Protocol/IPC strict
Clippytests,fmtall,workspace checktestsoffline,samefeatureworker exit0(check NOTworkspace tests).
Extensions7/7,clipboard5/5;C#build0warnings/errors. Exactoriginal126+new1 firstLIST127/RUN0 then
selected127/run127/pass127/skip0/fail0/not_run0,namesexact;actual4GiB+17ZIP64RUN1/PASS1ONCE.
AfterlateIPC-onlyreviewfix,affectedRust138/29/allgates and rebuiltworkerC#126(no large)rerun
0skip/fail/namesexact;127precededthatfix,large intentionallyNOTrepeated. NoDesktop/protocolenum
change;Desktop806/strictClippy NOT rerun thisrepair,historicalonly. Originalsource-deleteP1/P2
acceptance unchanged. Exacttimestamps/logs/cleanup/toolinginterruptions are in repairrecord.

RemainingA:real native recycle identity contract still unapproved/unproven,production null
native_protection_unproven/NotAttempted; directory/exclusionrefusal unchanged. Deployment
package/signed/protectedworker mapping unperformed,FIFO/conflicts,broadcodec/resources/
100k/millionentrytiming/RSS/longpath/UNC/nativeuser acceptance remain open. NoB/A/A-Fcompletion,
Release/package/install/signing/realproduct/userarchive/registry/service/mapping/commit/push.

## Historical accepted — two source-delete qualification defects repaired

[Independent P1/P2 findings](source-deletion-acceptance.md) remain unchanged. [Repair/rootcause/
red-green/final evidence](source-deletion-remediation.md) closes oldevidence/newlease/requestclone
authority and staleRustfinish only. C#private originalRun session captures sources/backend at
source_acquire,only real completionowner qualifies/executes,awaits actualSTAresult,then retires
before lease disposal. Observer cannot start/requalify/replace sources;oldfactory/weakkeys removed.
Rust everybegin creates private lifecycleidentity,only matching finish records;lateropen/test/
failedextract invalidates olderfinish,consumed remains operationwide. All factproducers updated.
No sourcelease sharing/native/outputpublication/TOCTOU/dependency/UI/productIPC/queue change.

Actual final13:21:50->13:33:14(+08:00): archive112pass/0fail/0ignored across23groups,exit0;
samefeatureworker,default+isolated strictClippy--tests,fmt--all--check,workspace check--tests
--offline all exit0(check NOT workspace tests). Extensions7/7,clipboard5/5,0ignored;
C#build0warnings/0errors. Original117preserved+new2 first listed119/run0,then selected119/
run119/pass119/skip0/fail0/not_run0;LIST/RUN/PASSnames exact. Actual4GiB+17ZIP64RUN1/PASS1
only final119 thisrepair. New4Rustpublictests includedin112,not extra counts. Focused9Rust+3
creationworker and9CSharpPASS. Oneboundedreadonlyreview:no concretefindings,not independentrun.

Original realP1red1fail/backendcalls2->originalsession retiredreplay0extra;normalrecordingrun1
stays1afterreplacement/newlease/oldJSON/clone. P2callbacktestredeligible->incompleteafteroldfinish;
lateropen(success/fail),failedextract(plan/identity),newervalidextract,consumed/cancelmatrixgreen.
Historical108/117green did NOTcover these defects;originalsource-deletion-evidence.md retained.

Nativecontract **stillpending**: public IFileOperation/RecycleItem useIShellItem,notHANDLE+
expectedfull128ID/version;strict lease blocks move,close->Shellrace unapproved. Production
native_protection_unproven/NotAttempted retains source. No actualrecycle/permanentfallback run.
Directoryroot/exclusion conservative refusal preserved,no directorycoverageproof. RemainingA:
nativecontract/acceptance,productIPC/capabilities/helpermapping/FIFO/conflicts,broadresources/
performance/count/longpath/UNC/nativeexits. NoB/UI/fullA/A-F/releaseclaim. Finalstatic/hygiene
inrepairrecord. HistoricalTARacceptance andoriginalfailure records remain unchanged.

## Historical accepted — 7z creation verifier typed-I/O repair

[Repair and actual evidence](sevenz-create-typed-io-remediation.md) closes the P2 finding in
[independent acceptance](tree-create-acceptance-review.md), whose original source-confirmed
finding and historical results remain unchanged. No real failing disk or data loss was shown.
Only the owned verifier input now retains first typed Read/Seek failure before parser/decoder
wrappers: storage -> IoFailure, corruption/truncation/integrity -> CorruptArchive, existing
typed cancel/resource/source errors preserved. No dependency/wire/runtime/publication change.

Real create/encode/full-held-readback/verifier/abort regressions: same command red2pass/5fail,
exit101 -> green7pass/0fail/0ignored,exit0. Seven tests execute31small transactions with exact
owned abort/source-close/zero-commit/no-final/old-target/foreign-partial assertions.
Final integration18:34:32-18:45:26(+08:00): archive93pass/0fail/0ignored across20groups,exit0;
same-feature worker build, default+isolated strict Clippy--tests, fmt--all--check and workspace
check--tests--offline all exit0 (check is NOT workspace tests). Extensions7/7,clipboard5/5,
39filtered,0ignored,exit0. C#build0warnings/0errors; original exact89listed first then selected89/
run89/pass89/skip0/fail0/not_run0,exit0. Actual4GiB+17ZIP64RUN1/PASS1only in this final89.
Equivalent real missing-internal-output fixture: OutputCollision/commits0/output_created=False,
source names/hash unchanged. Explicit7-Zip26.00/Python3.13.2versions/hashes rechecked this run.
Exact commands/count assertions/logs: session scratch typed-io-verify.ps1,typed-io-final-ledger.txt,
typed-io-final-*.log; red/green logs retained. Optional reviewer terminated without report,not a
passed review; no repeat review. Postbatch0Cargo/rustc/worker/test-host/7z,0fixed helpers,0inspected
archive fixture roots. No packaging/install/user archives/registry/service/mapping/commit/push.

Remaining Phase A stays timestamps/attributes/MOTW,guarded recycle/source-delete eligibility,
product IPC/capabilities/helper package mapping/FIFO/backpressure/conflicts and broad resources/
performance/large-count/long-path/UNC/native acceptance. Stop this repair here; no B/UI/A-F claim.
Final document gates: spec101documents/14owners,catalog79pages/79rows/78locked,context36implement/
33check(no truncation),PlatformHost fixture selector source gate all exit0. Diff/conflict/DEBUG/
cleanup checks and complete evidence live in the repair record;these do not certify Phase A.

## Historical saved green — verified 7z / TAR / TGZ / TXZ tree creation

[Tree creation evidence](tree-create-evidence.md) records per-format actual Start/Job/worker/native
red→green and complete independent7Zip/Python exact manifest/kind/size/SHA256 verification.
7z LZMA2fast1/normal6/maximum7,solid/nonsolid,AES256,explicit optional name encryption;
TAR(no level/password),TGZ1/6/9,TXZ1/6/7 reuse the same immutable source plan/held native IDs,
read-only output preparation/collision checks and unchanged atomic-path publication/TOCTOU.
Bounded anonymous encoded spool→native partial→complete held readback/hash→full structure/
decode/exact plan+source hash→source/cancel recheck→native flush/publish. No plaintext-source
encryption spool,whole archive Vec,multithread API or ordinary source/final path authority.

Actual final integration: archive86passed/0failed/0ignored,exit0; same-feature worker,strict
archive Clippy--tests,fmt--all--check,workspace check--tests all exit0(check is not tests).
C#build0warnings/0errors;original80+new9 listed selected89/run0,then selected89/run89/pass89/
skip0/fail0/not_run0,exit0. Actual4GiB+17FastDeflateZIP64 ran and passed ONCE in this continuation's
final89,real Job/native publication+Python streaming size/SHA256. Extensions7/7,clipboard5/5
(39filtered),0ignored,exit0. Large preflight/logs/exact commands:tree-create-verification-ledger.txt
and tree-final-* in session scratch. Default-feature strictClippy also passed before final small
constructor error fix; final isolated lint includes it. No repeated review loop was started.

Real long-CJK/multi-root red found native directory physical Size4096was mistaken for archive
payload;public plan red0pass/1fail→4/4green. PlannedSource.size now0for directories;C#original
size/fullID/ChangeTime source-version checks stay intact. Empty7z budget16red caught constructor
seek wrapping ResourceLimit;first typed failure is retained. New9native fixtures pass complete
created-output open/test/extract,levels/solid/AES/header,empty/longname/multi/admission/budget/
cancel/source/no-replace/confirmed overwrite/diagnostics. Final all89retains original ZIP/native
regressions. Temporary DEBUG probes removed.0Cargo/rustc/worker/test-host/7z,fixed helper absent,
0owned tree fixture roots after batch;partial/old-target/source assertions pass in fixtures.
Static gates:spec101documents/14owners,catalog79pages/79rows/78locked,task context34implement/
31check(no truncation),fixture-selector source gate,21scoped files whitespace/conflict/DEBUG
hygiene and git diff--check all pass. These do not certify native/product/resource acceptance.

Next approved work remains Phase A native timestamps/attributes/MOTW,guarded recycle/source-delete
eligibility,product IPC/helper package mapping/FIFO/backpressure/conflict integration and broad
resources/performance/large/long-path acceptance. No B/UI/release/install/registry/service/mapping/
user archives/commit/push. The saved ZIP checkpoint and independent failures below stay historical.

## Historical saved green — invalid internal output rejected before directory creation

[Output preparation repair/evidence](zip-create-output-preparation.md) closes the independently
reproduced [directory-side-effect failure](zip-create-remediation-acceptance.md). Preserve that
failure and historical75/75 below; passing75 did not detect missing internal output roots.
Actual current integration: archive82passed/0failed/0ignored,exit0; same-feature worker,
strict archive Clippy--tests,fmt--all--check,workspace check--tests all exit0 (check is not tests).
C#product/test build0warnings/0errors; list selected80/run0,execution selected80/run80/pass80/
skip0/fail0/not_run0,exit0. Original75 retained plus5new. Actual4GiB+17FastDeflateZIP64 passed
once this repair with real Job/worker/native publication and Python streaming size/SHA256.
Extensions archive_import7/7 and clipboard archive_5/5(39filtered),0ignored,exit0.

ZIP output now Prepare(opens/holds existing real ancestors,no create) -> original source
lease/eligibility/lexical+actual held-identity collision checks -> CreatePreparedRoot(relative
missing suffix,cancel/source-version checks before/after components) -> SAME collision check
against completed actual output session -> existing Rust plan/verified partial/native commit.
Single/multi-source missing internal roots leave source names/hashes unchanged; actual ancestor
identity still rejects when synthetic check-path is harmless. Existing internal content stays
unchanged; legal missing external output and parent/sibling confirmed overwrite succeed.
Precreation cancel/denial/source change creates no directories; after lawful creation starts,
created directories are intentionally retained,including external additions. Only exact owned
unpublished partials are cleaned. CleanupComplete is not global filesystem rollback.

One bounded directed review found no concrete regression. Earlier integrated80selection failed
at AES due scratch PowerShell's misdecoded Chinese oracle path:run19/pass18/fail1/not_run61,
exit1;ZIP64not run then. Correct inherited environment AES1/1 then FULL batch above passed.
Failure logs retained,no product/test weakening. Final logs prepare-order-final-*,ledger in
repair evidence;0Cargo/rustc/worker/test-host/7z,fixed helper absent,0owned source-binding roots.
No writer/crypto/reader/dependency/UI/package/install/registry/service/mapping/user-archive/
commit/push changes. Next approved work remains7z/TAR/TGZ/TXZ creation within exact
**Phase A — Engine contracts and safety foundation**,not B. Broad resources/native/product
and full Phase A exits remain open.

## Historical state — ZIP creation admission / namespace remediation green

Independent creation rejection remains unchanged in [acceptance review](zip-create-acceptance-review.md).
This-turn repair/evidence is [zip-create-remediation.md](zip-create-remediation.md); original
82/62 passing results did not disprove those defects. Actual post-repair integration:

| Gate | Actual current result |
| --- | --- |
| archive isolated-codecs --offline --no-fail-fast |82passed/0failed/0ignored,exit0 |
| worker same-feature build / strict archive Clippy --tests |both exit0 |
| archive fmt / workspace fmt --all --check / workspace check --tests |all exit0;check is NOT workspace tests |
| C# build |test project and referenced product0warnings/0errors,exit0 |
| Exact original62+new13 |listed selected75/run0;executed selected75/run75/pass75/skip0/fail0/not_run0,exit0 |
| Actual4GiB+17FastDeflate ZIP64 |included in final75;real Job/worker/native publication plus Python streaming size/SHA256 PASS |
| Extensions archive_import / clipboard --lib archive_ |7/7 and5/5PASS,39clipboard filtered,exit0 |

Admission now privately fixes SourcePaths, nested exclusions, confirmed path/identity authorization
and immutable five-scalar Limits before validation/wire/Run/accepted. Caller mutation cannot
change approved source/overwrite/budgets; observed inconsistent capture rejects before acceptance.
One owner covers ZIP and old extract/Gzip/XZ. Field-specific errors/defaults/CreateName-only
authority remain. Parent-root sibling ZIP works; dangerous self-inclusion/real identity/reparse
protections remain. Enumeration uses held-directory-derived volume-GUID namespace with real
reopened identity validation; relative no-reparse handles determine root/child attributes.
Collision checks use the **actual held output session's** ancestors/root/final, not a fresh DOS
mapping. Synthetic root/nested enumeration and output-namespace seams went red then green; no
real SUBST/DefineDosDevice/system mapping operation was performed or claimed.

One bounded directed review found held-output namespace and2error-category defects; concrete
regressions/fixes/retests completed, no repeat review. External oracle is CPython3.13.2 plus
existing7-Zip26.00(hash in remediation),not historical24.09. Password stdin only. Initial
background Rust batch ended without status and is not PASS;no surviving build,full foreground
batch rerun succeeded. Final75 logs:remediation-final-native-{list,tests}.log.

No worker/test/7z/Cargo/rustc,owned fixture roots or fixed helper remained at final inspection.
Document/context/selector gates are recorded in remediation. No new dependencies/writer/crypto/
reader changes,UI,release/package/install,registry/service/drive-map/user archives,commit/push.
Next approved **Phase A — Engine contracts and safety foundation** work is7z/TAR/TGZ/TXZ
creation,then remaining native/product/resources. This remediation checkpoint is green;full
Phase A exit,ZIP64large-count/RSS/performance/long-path/UNC/storage/native-user remain open.
No new format tracer was mixed with these defects;do not enter Phase B.

## Historical pre-independent-review state — bounded ZIP creation and AES-256

**Current repaired-source integration: archive82passed/0failed/0ignored,exit0**, isolated-codecs
and --no-fail-fast. C# original54+new8 first listed selected62/run0; executed selected62/run62/
pass62/skip0/fail0/not_run0,exit0. Public plan3,worker ZIP1,wire6 include new creation behavior;
one test may execute multiple bounded scenarios,not extra harness cases.

ZIP Store/Deflate(create semantic store/fast1/normal6/maximum9) and AES256 now stream file,
directory and multiple authorized roots through the existing Job/worker/partial/native runtime.
Rust owns plans/globs/root/Unicode/empty/collision/budgets; C# owns held no-reparse identities,
ChangeTime/member versions and actual protected reads. Finite32root chains,64absolute source
depth,65536native observed-record ceiling,one enumerator OR active read file/transient chain.
No all-source handles/content Vec/path-read-then-hash or source deletion. Output in any source
directory is conservatively refused; exact CreateName-only native output/overwrite authorization.

Public ZIP8.6.0 new_stream needs no new native Seek ability. AES uses public borrowed-password
writer+random salt; directories are plain Store,names not encrypted. Full held-partial readback
spool is bounded/exclusive/delete-on-close. Original-record central/local/descriptor/extents
preflight and every applicable CRC/AES/SHA256/size/list/method check precede source/cancel
recheck and unchanged approved flush/single atomic path publication. Accepted target TOCTOU,
committed/unknown projection and owned-unpublished cleanup unchanged.

Real red→green repairs: metadata budget category,exact EOF entry limit,and stream-writer
directory descriptor omission. External readers tolerated the latter but created-archive test
returned CorruptArchive. Creation now uses public start_file(trailing slash,Store) and complete
existing structure preflight; no read validation weakened. Final actual7creation fixtures pass
normal/source mutation/reparse/budget/overwrite/cancel/owned cleanup/privacy/capability options.
Independent CPython zipfile and hash-verified official7-Zip24.09 verify exact names/sizes/SHA256,
UTF8 and AES authentication at actual native output. No password argv/config/logs.

Same-feature worker,strict archive Clippy including tests,archive fmt,workspace fmt--all--check,
workspace check--tests(each exit0;check is NOT workspace tests),C#build0warnings/0errors,
extensions archive_import7/7 and clipboard --lib archive_5/5(39filtered) all rerun after repair.
No current Desktopfmt blocker. Previous fmtFAIL and77/54read evidence remain historical;
[independent accepted baseline](zip-aes-acceptance-review.md),[creation capability/evidence](zip-create-evidence.md),
[minimal source/API contract](zip-create-contract.md). Scratch logs:zip-create-verified-*.

ZIP/GPUI versions remain8.6.0/0.7.1. Archive-only aes-crypto/direct lockedglobset added; six
new crypto transitive lock items,none removed/upgraded; exact licenses/notices/Windows build
recorded. Actual4GiB+17FastDeflate source→native ZIP64→Python streaming size/SHA256 passes.
Large fixture exposed120s cap;create_zip-only explicit max600000ms(default unchanged) fixed it.
ZIP64large-count/million-entry/performance/RSS/UNC/long-path/full resources remain incomplete.
One optional directed review stopped without report;not a passed review,no repeat review.

Next approved Phase A remains7z/TAR/TGZ/TXZ creation reusing sources/output,then native attrs/
MOTW/time/source-delete eligibility,product IPC/package mapping/backpressure and broad resource
acceptance. No Phase B/UI,Release/install/registry/service/user archives/commit/push.

## Historical checkpoints — not current results

Earlier source lease/budget repairs, strict-CAS failure, approved atomic-path overwrite and
outcome/log repairs remain recorded below and in their linked evidence. The accepted overwrite
contract is complete verified temporary→flush→target recheck→close only target check→single
atomic path publication, with accepted TOCTOU, no strict target-ID CAS or backup-first fallback.
Prior Rust45pass/1fail/C#40pass checkpoint and exact previous edit-stop are preserved in the
[historical matrix](phase-a-capability-matrix.md#historical-xz-creation-continuation--retained-red).
Do not reuse older tables as current execution evidence.

## Implemented product sources

- `rust/crates/nxtools-archive`: narrow ArchiveEngine.open/execute/extract, immutable session+ordinal entries/source revision, bounded metadata, fixed errors, encoding provenance and output capabilities. ZIP Store/Deflate retains original central records, ZIP64 metadata, duplicate visibility, authoritative Unicode/manual code pages and lexical collision warnings.
- ZIP shares one test/extract decoder: local/central consistency, bounded nonoverlapping extents, actual bytes, CRC and explicit Deflate StreamEnd. `planning.rs` covers full/selected structure and smart destination; empty extraction creates no destination. `interaction.rs` has bounded operation/prompt/generation-scoped password/conflict/risk exchanges; cancel wakes waits.
- `src/worker_main.rs` is the tracked binary entry, not ignored src/bin. Private u32LE length1..1MiB + strict UTF8 JSON, one control reader, one outstanding acknowledged native call. Rust ordinary binary is `nxtools-archive-worker.exe`; C# package contract is `NxTools.ArchiveWorker.exe`. Product package mapping/routing is not integrated.
- Optional `isolated-codecs`: exact sevenz-rust2=0.23.0 (compress,aes256), unrar_sys=0.5.8 (bundled UnRAR7.1). Serial Windows C++ build and narrow advapi32 linkage passed without upstream edits. ZIP lock8.6.0 and Kit=0.7.1 unchanged. Full UnRAR license, fixture hashes/provenance and notices retained.
- `ArchiveWorkerRunner.cs`: fixed sibling helper, suspended create→Job assignment→resume; max2 Jobs/aggregate2GiB/one process. Current process reservation=work+metadata+64MiB overhead, default832MiB (512+256+64), max1GiB; explicit MemoryBytes must match. Work below64MiB is rejected, not rounded. Cooperative200ms cancel grace then own-Job termination; slot and source handles retained until actual process/pumps/Job cleanup. ExtensionNative.Spawn remains unchanged.
- `ArchiveOutputSession.cs`: held non-share-delete directories, relative single-component NT opens, no-reparse checks, GUID-exclusive sibling partials, no-replace commit, exact-handle partial deletion and identity/size/mtime/SHA256 guarded cleanup of unchanged newly created outputs. No implicit NFC conversion. Local drive roots only; current depth64,4096file records and finite directory handles are not full-scale acceptance.
- `ArchiveWorkerOutputBridge.cs`: private native directory/begin/write/commit/abort calls; output root remains in C#, chunks≤65536bytes, actual-byte limits, no file content published as business events.
- ZIP real C#→Rust→native extraction commits only verified files. Bad CRC never publishes the corrupt current file; earlier valid files remain.
- RAR4/5 and7z now also extract through ArchiveOutput in the isolated worker. RAR first enumerates/validates every output, rechecks source, then compares second-pass header ordinal/name/type/size/encryption/CRC. RAR_TEST+UCM_PROCESSDATA (null native destination) streams borrowed read-only callback data in≤64KiB acknowledged writes; Mutex state owns a Send output and never unwinds across C ABI.7z uses bounded readers, retains first callback error across upstream block iteration and independently checks per-file CRC where supplied. Both stage partials until full decode/integrity/source recheck, then no-replace commit; commit failure preserves existing targets and cleans remaining partials. No unrestricted unpack or RAR creation.
- Schema2 negotiates source_lease_v1/budget_reservations_v1. C# ArchiveSourceLease holds no-reparse drive/parent chain without delete sharing and source with only read sharing before Rust source reads.7z and both RAR path opens share this protected namespace/file object; active write/delete handles and the owned writable-mapping fixture are rejected. No snapshot/password disk persistence. Schema1 ZIP remains compatible; schema1 isolated codecs return UnsupportedOperation explicitly.
-7z preflight validates start-header CRC/checked extent/EOF as CorruptArchive before metadata admission; valid over-budget metadata/work returns ResourceLimit. Encoded-header stream descriptors are parsed through the locked public API without decode before dictionary/expanded-header checks; data blocks are checked before decode. No third-party patch. See remediation for conservative reservation semantics, actual Job allocation rejection and bounded-test limits.

## Approved architecture and historical strict-overwrite blocker

User approved the C#-managed OS-memory-capped Rust worker after exact public codec allocation/cancellation gaps were demonstrated. C# owns Windows process/Job/output/Shell APIs; Rust owns codecs/domain. No third-party patch, secret argv/disk, process-name kill or scope reduction. [Exact codec evidence](research/codec-capability-gates.md).

**Historical strict-CAS failure (superseded, not a current blocker).** Exact fixture `archive output fixture identity bound atomic overwrite` was independently rerun: selected1/run1/pass0/skip0/fail1, exit1, `replace_file:32`. On this workstation's NTFS, minimal FileRenameInformationEx flags3 probe succeeded without an old-target handle or with FILE_SHARE_DELETE, but failed STATUS_SHARING_VIOLATION0xC0000043/Win32=32 while the old target was protected without delete sharing. Changing target access/source sharing/parent access did not fix it. No protection was weakened. [Historical evidence](research/native-overwrite-gate.md).

The preceding paragraph is the historical strict-CAS result, not current status. The same
fixture was renamed to the approved rechecked atomic-path contract and is green; no test was
deleted. [Current implementation/limits](overwrite-remediation.md). The rejected backup proposal
below is still unapproved/not implemented.

Proposed **not approved / not implemented**: move the identity-checked old target by its held handle into a unique owned backup, publish the new file no-replace, restore backup no-replace on failure; retain/report backup if external competition prevents restore. This preserves data but has a directory-name visibility gap and is **not atomic replacement**. Requires explicit approval before changing design§4.3. This is one Phase A decision, not another approval request for already-authorized scope/worker.

## Actual verification

The following table records the earlier checkpoint. The later remediation command/count table
is authoritative for the current repaired slice; neither table closes Phase A.

All Cargo commands inherited CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, RUSTFLAGS=-C codegen-units=256 and ran serially. Latest logs: session scratch `archive-continuation-{tests,workspace,fmt}.log`; earlier `archive-final-*.log` are retained.

| Check / real seam | Actual result |
| --- | --- |
| Engine initial red and optional one review | missing API/safety/local-header/Deflate end reds; review14tests initially10pass/4fail; concrete commentEOCD/ShiftJIS/declared-preflight/100k defects fixed; no second broad review |
| `cargo test -p nxtools-archive --features isolated-codecs --offline` | **30passed/0failed/0ignored**, exit0: engine14,plan5,prompt3,transaction4,worker-extraction1,wire3 |
| strict archive Clippy with --tests --features isolated-codecs --offline -- -D warnings | PASS exit0 after RAR/7z extraction changes |
| `cargo check --workspace --tests --offline` | PASS exit0 |
| `cargo fmt --all -- --check` | PASS exit0 |
| C# test project build --no-restore | PASS0warnings/0errors |
| New7z extraction red→green | UnsupportedFormat red before dispatch; actual C# Job→Rust→native output passed after implementation |
| NewRAR extraction red→green | UnsupportedOperation red before callback output; actual RAR4 payload green after implementation |
| Latest precise C# safe subset (listed first:16selected/RUN0) | **16selected/16run/16pass/0skip/0fail**, exit0; deliberately excludes the separately failing atomic-overwrite fixture |
| C# subset composition |4native-output+5worker+2codec+3extraction+2existing extension-process regressions |
| Independent codec positive fixture |8archives×open/test=16operations: RAR4/5, encryptedRAR4/headerRAR5,7z LZMA/solid/non-solid/AES |
| New7z extraction fixture |9operations: LZMA,solid,non-solid,AES,missing/wrong password,budget,existing-target,damaged payload; independent bsdtar payload literal/hashes compared; no failed-output/partial leaks |
| NewRAR extraction fixture |9operations: RAR4,RAR5,encryptedRAR4/headerRAR5,missing/wrong password,budget,existing-target,damaged payload; plain payload literals compared; no failed-output/partial leaks |
| Existing ZIP extraction fixture |valid and badCRC actual native output, byte comparison, no content in business events, real cleanup |
| Atomic overwrite fixture |**FAIL** exit1 `replace_file:32`, retained; not excluded from default suite |
| Existing extension ZIP archive_import |earlier7/7 PASS; no dependency features/policies changed by this continuation |
| Existing clipboard --lib archive_ |earlier5/5 PASS,39filtered; no policy changes |

The small encrypted extraction cases demonstrate successful decode/native output, not a universal encrypted-format interoperability claim. No new dependencies or feature unification were introduced in this continuation. Process checks found no owned Cargo/rustc/archive worker left; fixed test helper copy was removed. Fixture directories were deleted. Scratch native probe sources/log are retained outside the project, with no product DEBUG probes.

Earlier editing-stop report was an agent counting error: zip.rs had two failed edits, not three; a different file's failure was mistakenly included. Full refreshed reads/successful grounded edits fixed regressions. No current per-path editing blocker or bypass.

## Still required before Phase A exit

- Approved rechecked atomic-path overwrite slice is verified locally; broad storage/native/release acceptance remains outside these fixtures. Strict target-ID CAS is superseded, backup-first remains unapproved.
- Required7z/TAR/TGZ/TXZ bounded creation is delivered in current section; broad source/resource acceptance remains. RAR/7z full method/link/malicious/large/cancel coverage is not established by small fixtures.
- Metadata/MOTW/time slice and source-delete qualification/isolation are locally verified in current evidence; strict original-object native Recycle Bin execution remains blocked by documented API decision. Conflict integration/full diagnostics and broad native acceptance remain open.
- Product PlatformHost IPC orchestration/capability/typed lifecycle, bounded business consumer/backpressure and large-metadata streaming. Current stdout8MiB cap is not million-entry acceptance; private output acknowledgement is not general metadata backpressure.
- Resource/performance evidence including actual dictionary-memory pressure,100k listing timings/RSS/first paint,≥4GiB/long paths/UNC and bounded1GiB cooperative cancellation.
- ENG01–ENG09,SAFE01–SAFE08,P201 and touched IPC01 remain partial/not passed as whole scenarios. Core/IPC/Protocol/Desktop/full workspace test suite, screenshots/physical/native Shell/DPI/UIA/user visual and Release/install acceptance have not been run here.
- Do not advance to B before A exit. B–F remain required product scope, not cancelled because this execution hits a gate.

## Reproduction / historical navigation
Current exact commands/counts/cleanup: [product read evidence](product-read-evidence.md).
Old fixture commands retained in [historical reproduction](research/historical-fixture-reproduction.md).
Historical sections above do not replace the current state at the top or certify full Phase A.
