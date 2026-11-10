# Archive quality and executable contracts

Evidence: `repo:rust/crates/nxtools-archive/tests/engine_contract.rs`, `repo:rust/crates/nxtools-archive/tests/fixtures/generate.py`, `repo:rust/crates/nxtools-archive/src/formats/zip.rs`.

## Scenario: Independent bounded ZIP listing and stream test
### 1. Scope / Trigger
Opening/testing untrusted ZIP Store/Deflate through the public engine. ZIP extraction/native codecs have separate scoped fixtures below; no complete Phase A/creation/A–F claim.
### 2. Signatures
`ArchiveEngine::open(OpenRequest, &OperationControl) -> Result<ArchiveSnapshot>`; `execute(ArchiveRequest::test(snapshot), &OperationControl) -> Result<ArchiveOutcome>`; `with_limits(ResourceLimits)`; control checkpoint/cancel/set_paused/latest_progress.
### 3. Contracts
Bound EOCD/ZIP64/count/central bytes before allocator-heavy parsing. Preserve original central records; zip8.6.0 IndexMap collapses duplicate names. Test current immutable locations, validate local signature/flags/method/raw name/known sizes/CRC and bounded data extent; no second unbounded ZipArchive parse. Fixed-size streaming checks actual bytes/CRC and cancellation. Snapshot source revision verified before and after operation. Advertise only actual operations; native output seam is separate. Every Cargo command inherits CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, RUSTFLAGS=-C codegen-units=256.
### 4. Validation & Error Matrix
Independent valid ZIP -> truthful counts/Unicode/comment and all stream CRCs; duplicate raw names -> visible distinct IDs and OutputCollision execution reject; unsafe lexical names -> visible warnings, no writes; local/central mismatch or damaged CRC -> CorruptArchive; finite entry/metadata/actual bytes over budget -> ResourceLimit; replaced source -> SourceChanged; pause+cancel -> wakes and returns Cancelled. Unsupported operations/formats remain typed, never fake enabled.
### 5. Good/Base/Bad Cases
Good: Python fixture exact literal names/counts/CRC and malicious mutations tested through open/execute. Base: valid ZIP Store and Deflate both stream. Bad: self-roundtrip alone, import-string tests, unrestricted unpack, duplicate lost in backend map or native safety claimed from ../ check.
### 6. Tests Required
Keep independent ZIP, unsafe/collision, corruption, codepage, budgets, stale-source, cancel, local-header and explicit Deflate StreamEnd tests. Concrete review failures (commentEOCD, ShiftJIS trail0x5c and100k default admission) were red then fixed; current engine14/14, worker-wire3/3 pass. Collision projection checks cancellation per entry/component and charges nodes before allocation. Real C#→Rust fixture separately passes; RAR/7z codec tests/resources/native outputs and full A–F remain separately gated. No editing-policy blocker remains; earlier per-path count was corrected in task checkpoint.
### 7. Wrong vs Correct
Wrong: ZipArchive.len loses duplicates; by_index trusts a different local name; a timeout pretends native allocation stopped. Correct: independent budgeted central enumeration -> immutable session ordinals -> validated local bounded streams -> CRC/resource checks; unsupported unbounded codecs require an explicit reviewed worker/backend decision.

## Scenario: Isolated codec extraction through native final-name capabilities
### 1. Scope / Trigger
RAR4/5 and7z lack public preventive memory/cancel limits; only approved C#-Job worker may select them. Ordinary output follows the shared final-name contract; virtual cache and archive creation/editing remain separate. Evidence: `repo:rust/crates/nxtools-archive/src/worker_codecs.rs`, `repo:rust/crates/nxtools-archive/src/worker_codecs/rar.rs`, `repo:csharp/NxTools.PlatformHost.Tests/ArchiveExtractionWorkerTests.cs`.
### 2. Signatures
Private worker request operation=extract; ArchiveOutput directory/begin/write/commit/abort. Ordinary begin is final-name CREATE_NEW/direct decided overwrite,write records are <=32KiB without ACK,and commit carries metadata then closes. worker::run remains Write+Send because serialized UnRAR callback may borrow output across native threads. C# OutputRoot never crosses to Rust.
### 3. Contracts
All paths/internal collisions validate before begin; current4096file native capacity preflight remains explicit. RAR lists then rechecks source/second-pass ordinal/name/type/size/encryption/CRC and uses only RAR_TEST/null destination. UCM_PROCESSDATA read-only slices live only through the bounded original-pipe send under Mutex; no unwind across ABI.7z retains first error because the upstream outer block loop ignores false; no later callback output after failure. Keep final-name handles incomplete through required solid/block integrity and source recheck,then metadata+drain+close; failure deletes only incomplete owned handles. Existing targets enter Rust conflict policy and never clobber without chosen direct overwrite. No whole-entry Vec/unrestricted unpack.
### 4. Validation & Error Matrix
Missing password -> PasswordRequired; explicit UnRAR bad-password code -> WrongPassword; ambiguous -> AuthenticationOrDataError. Corruption -> delete incomplete output,no invalid close. Declared/actual budgets -> ResourceLimit. Target exists -> conflict prompt/user bytes intact until chosen overwrite. Cancellation cleanup remains C# owned even if private abort cannot run after transport loss; earlier closed outputs remain.
### 5. Good/Base/Bad Cases
Good: independent bsdtar literal/hash matches actual closed native output. Base: small plain/encrypted/solid archives. Bad: infer extract from test,close before solid integrity,use callback buffer after return or silently widen target sharing.
### 6. Tests Required
RAR/7z actual Job/native fixtures retain plain exact bytes/solid hashes,password/budget/corrupt/existing cases,source lease and cleanup. Add final-name/no-partial/rename/flush/fingerprint,two-call/zero-ACK/sticky-write-error and prior-closed retention assertions. Historical counts remain in the task checkpoint; they do not accept this new contract.
### 7. Wrong vs Correct
Wrong: callback false means later blocks stop,or close every selected file before required solid integrity. Correct: retain first error across callbacks,consume required integrity,close valid files,and delete only still-incomplete owned handles.

## Acceptance-remediation historical boundary and current supersession
Earlier remediation evidence remains historical: Rust31/31,C# existing16/16 plus new6/6,no skips;
strict identity-conditioned overwrite then remained FAIL32. The later rechecked atomic-path overwrite
passed its recorded29/29 combined fixtures and superseded strict target-ID CAS for that implementation.
Do not rewrite those outcomes as current ordinary-output acceptance.

The historical worker/native path used an exclusive create-new partial,FlushFileBuffers,target
fullID/hash/version recheck,close-only target check,SetFileInformationByHandle rename,post-ID/content
verification and idempotent publication before close/hash. Its exact failure codes,TOCTOU decision and
late-cancel journal remain evidence for archive candidate publication and prior extraction behavior.

Current ordinary known-destination extraction supersedes that output shape only. It opens the final
held-relative name with CREATE_NEW or directly truncates the Rust-decided overwrite path,streams
<=32KiB records on the original anonymous pipe without ACK,and completes by metadata+drain+close.
There is no target signature/CAS,partial rename,durability flush,output fingerprint or published reopen.
Cancel/failure deletes only incomplete owned final-name output(s); earlier closed files/directories remain.
Archive Create/Edit candidates,HZ5 and ArchiveEntryCache keep their verification/publication contracts.
See [ordinary extraction output](../../../tasks/11-02-archive-workflow-optimization/ordinary-extraction-output.md).

## Scenario: Standalone Gzip and verified single-source creation
### 1. Scope / Trigger
Required Phase A standalone Gzip, not compound TAR.GZ or general tree creation. Evidence: `repo:rust/crates/nxtools-archive/src/formats/gzip.rs`, `repo:rust/crates/nxtools-archive/src/worker_create.rs`, `repo:csharp/NxTools.PlatformHost.Tests/ArchiveStreamWorkerTests.cs`.
### 2. Signatures
ArchiveEngine.open/execute/extract supports Gzip; size remains None. WorkerRequest operation=create_gzip,schema2,create_name(single component),compression_level fast/normal/maximum. NativeCall read{token,offset:u64,length:1..65536}→native_result read_data(base64<=87384chars). C# ArchiveOutputSession.ReadPartial accepts current uncommitted token only.
### 3. Contracts
Bound RFC1952 optional header<=64KiB and metadata budget without retaining FNAME/COMMENT; require Deflate StreamEnd/every member CRC+ISIZE, actual-byte budgets and cancellation. Never inflate to supply listed size. Creation is worker-only under native source lease; stream one ordinary source through flate2=1.1.9, levels1/6/9. Full exclusive partial read-back decode plus SHA256 comparison/source recheck precedes existing native flush+publish. No whole-archive Vec. New partial_read_v1 negotiation required for create; absent new request fields omitted, legacy schema1/2 open/test/extract unchanged. Read payload remains private/not business frames/logs; only create can use it. Source/output same path and unsupported Store/password/unsafe name rejected before launch; no source delete.
### 4. Validation & Error Matrix
Malformed/truncated/CRC/ISIZE/unfinished/extra garbage→CorruptArchive; finite header/work/actual output/member limits→ResourceLimit; cancellation→Cancelled; changed source→SourceChanged; bad read-back→abort/no publication. Exact current native call owns target faults. Partial unknown/published token/negative or beyond-end offset/oversized read refused; EOF empty is valid. Parent/temp/source protections remain.
### 5. Good/Base/Bad Cases
Good: CPython multi-member payload equals exact expected bytes; independently .NET-decoded created output equals original. Base: unknown expanded size until successful stream. Bad: trust gzip ISIZE as64-bit size, retain hostile header string or publish immediately after encoder.finish without full read-back test.
### 6. Tests Required
stream_contract independent single/multi and bounded CRC/truncation/header/actual-budget cases; creation_worker_contract corrupt read-back asserts abort/no commit; worker_wire legacy omission/new roundtrip/schema1 rejection. ActualGzipExtraction/ActualGzipCreation/CreationSafetyAndPartialRead prove real Job/native bytes,cancel/source lease/partial limits/conflict; broad resource/native-user acceptance remains separate.
### 7. Wrong vs Correct
Wrong: memory buffer whole output→rename, or read reopens partial by mutable path. Correct: leased source→stream acknowledged token writes→read same held partial in<=64KiB→full decode/hash/source validation→native flush/single publication.

## Scenario: Bounded TAR headers and native extraction
### 1. Scope / Trigger
Plain TAR regular files/directories/PAX/GNU longnames; links/special entries listed/skipped, sparse typed unsupported. Evidence: `repo:rust/crates/nxtools-archive/src/formats/tar.rs`, `repo:rust/crates/nxtools-archive/tests/stream_contract.rs`, `repo:csharp/NxTools.PlatformHost.Tests/ArchiveStreamWorkerTests.cs`.
### 2. Signatures
ArchiveEngine.open/execute/extract handles ArchiveFormat::Tar through fixed512-byte tar=0.4.46 Header only; no Archive.entries/unpack. Detection checks bounded header signature (extension only routes to structural validation). Immutable raw-name/ordinal/size/kind is compared before payload operations.
### 3. Contracts
Checked header/payload/padding extents,header checksum,two zero end blocks plus zero tail; no payload-checksum claim. PAX records/GNU longnames remain bounded before allocation. Preserve unsafe/duplicate names and run ordinary path/collision projection before output. Open final-name regular files,retain them until required TAR/source integrity,then metadata+drain+close; abort deletes only incomplete owned handles. No links/special materialization or unrestricted unpack. Conservative retained-record reservation and finite cancellation/actual bytes remain.
### 4. Validation & Error Matrix
Checksum/truncation/overflow/invalid PAX/orphan local extension/nonzero tail→CorruptArchive. Oversized/count/metadata/work/actual output→ResourceLimit. Sparse GNU/PAX semantics→UnsupportedMethod. Traversal/collision→visible metadata+UnsafePath/OutputCollision plan reject before writes; symlink→LinkNotExtracted warning+skipped count/source-delete ineligible. Native source/output protections remain separate C# ownership.
### 5. Good/Base/Bad Cases
Good: independently CPython-generated PAX Unicode/empty directory/bytes/symlink plus GNU longname. Base: TAR validates structure/readability only. Bad: use entries() that read_all's extensions before caller budget, silently treat sparse as contiguous, infer pending extension from only path/size fields or claim TAR data CRC.
### 6. Tests Required
Public red before adapter2fails UnsupportedFormat; independent normal/checksum/truncate/metadata and extension/path scenarios. Actual worker native fixture checks exact bytes/empty dirs/link absent, unsafe/collision/huge header no output and cleanup. Orphan comment-only PAX red2pass/1fail, then pending-state fix/retest3pass. Preserve Gzip/ZIP12 and original codec/terminal regressions. Later bounded TAR.GZ/BZ2 contract below; TAR create, timestamps/full resource acceptance remain separate required work.
### 7. Wrong vs Correct
Wrong: local extension.path/size are None, so accept EOF. Correct: explicit pending local-extension state survives any accepted extension payload until the real described entry.

## Scenario: BZip2 streams and compound TAR spooling
### 1. Scope / Trigger
Standalone BZip2(single/multiple members), TAR.GZ/TAR.BZ2 listing/test/extract. Evidence: `repo:rust/crates/nxtools-archive/src/formats/bzip.rs`, `repo:rust/crates/nxtools-archive/src/formats/tgz.rs`, `repo:csharp/NxTools.PlatformHost.Tests/ArchiveStreamWorkerTests.cs`.
### 2. Signatures
ArchiveEngine accepts BZh signature; compound suffix selects TarBz2 only after outer signature and actual TAR validation. tgz::decode/decode_bzip returns a bounded disk File, not Vec. tar::extract receives both decoded-file and original-source handles for precommit revision check. Standalone BZip2 size stays None.
### 3. Contracts
BZip2 low-level small-mode Decompress requires conservative8MiB working budget before allocation; explicit StreamEnd/CRC for each member,count/actual expanded bytes/cancellation checked. Invalid trailing data/truncated stream cannot be EOF success. Complete outer integrity precedes ANY compound target directory/file begin. Spool uses exact already-locked tempfile3.27.0 anonymous file (Windows create_new/share0/delete-on-close),never persist/MoveFileEx; actual spool bytes<=max_output_bytes,individual TAR files<=max_entry_bytes. Source revision/lease remains compressed original,rechecked before ordinary closes; auto-clean spool on all returns. Nonencrypted formats only,no password or secret persistence/new Rust Windows FFI. No BZip2 creation capability.
### 4. Validation & Error Matrix
Outer CRC/truncated/trailing garbage -> CorruptArchive; work/member/spool/output budget -> ResourceLimit; cancel -> Cancelled; malformed inner TAR -> typed TAR failure. Outer failure produces zero native outputs; standalone failure deletes only its incomplete final-name output. Source changed before close -> SourceChanged,no invalid completion. Compound suffix is routing only,never validity proof.
### 5. Good/Base/Bad Cases
Good: independent CPython bz2 multi-member bytes and tarfile+gzip/bz2 compound oracle. Base: disk spool bounded with fixed-memory streaming. Bad: infer standalone capability from ZIP12, load decoded TAR Vec, test only inner TAR while ignoring outer CRC, or compare spool metadata to compressed source revision.
### 6. Tests Required
stream_contract normal single/multi/TAR.BZ2; corruption/truncate/garbage/work/output rejects and no publish; TAR.GZ outer CRC/spool budget. ActualBzip2Streams6worker operations and ActualTarGzipExtraction3 operations verify exact native bytes/full cleanup/zero outer-failure output; preserve previous archive fixtures. No large100k/RSS/UNC/full safety acceptance inferred.
### 7. Wrong vs Correct
Wrong: decode outer on demand while closing early inner files. Correct: bounded disk spool through full outer integrity -> bounded TAR plan -> final-name files -> original compressed source recheck -> metadata+drain+close.

## Scenario: Public XZ and TAR.XZ source-preserving reads
### 1. Scope / Trigger
Standalone XZ(single/multiple streams) and TAR.XZ/.txz open/test/extract through the public engine and unchanged worker path. Evidence: `repo:rust/crates/nxtools-archive/src/formats/xz.rs`, `repo:rust/crates/nxtools-archive/tests/xz_read_contract.rs`, `repo:csharp/NxTools.PlatformHost.Tests/ArchiveXzWorkerTests.cs`.
### 2. Signatures
xz::list/extract/stream; tgz::matches_xz/decode_xz; ArchiveEngine open/execute/extract. Six-byte signature gates extension routing. One standalone session/ordinal entry has size=None and actual compressed length.0x21 is the XZ terminal LZMA2 filter ID, not ZIP's method field semantics.
### 3. Contracts
Before decoder allocations,validate bounded footer/index/header CRCs,canonical VLIs/record counts/extents,block/filter/dictionary admission,encoded LZMA2 chunk boundaries and stream padding/member counts. Do not decompress just to list size or trust index sizes as verified output. Full decoder verifies every stream/payload check; <=32KiB ordinary output records enforce actual budgets. Before close recheck original compressed-source revision and cancellation; failure deletes only incomplete owned output. TAR.XZ outer complete integrity uses the exclusive delete-on-close disk spool before target creation; source lease/recheck stays original. Typed controlled-reader errors remain by provenance; cancellation includes padding scans.
### 4. Validation & Error Matrix
Header/index/footer/payload/truncation/garbage/invalid padding→CorruptArchive; metadata/member/dictionary/declared-or-actual-output/spool budget→ResourceLimit; cancel→Cancelled; source change→SourceChanged; actual source/output I/O→IoFailure. Payload-only damage may list standalone unknown metadata but test/extract fail; TAR may list oversized entries but test/extract enforce entry limit. Outer corruption creates no native target output.
### 5. Good/Base/Bad Cases
Good: independent CPython fixtures plus literal expected bytes, actual Job→Rust→native outputs. Base: valid padding is multiple4, including between streams. Bad: infer public read from XZ creation, add only engine arms without adapters, use ZIP method numbers, or compare spool revision with compressed-source snapshot.
### 6. Tests Required
Keep original public UnsupportedFormat red unchanged,then verify green; xz_read_contract public normal/failure/budgets/source/cancel/token tests and decoder typed-I/O regression. Real worker coverage retains open/test/extract,compounds,batch metadata/exact bytes,corruption/padding/work/metadata/output/spool,source lease/native cancel/existing target and incomplete-output cleanup. Historical task counts do not close current ordinary-output acceptance.
### 7. Wrong vs Correct
Wrong: decode on demand while closing early inner files or collapse all io::Error into corruption. Correct: preventive admission -> bounded full-outer spool when compound -> existing bounded TAR -> original-source check -> final-name metadata+drain+close; retain typed errors by provenance.

## Scenario: Bounded Zstandard frames and compound TAR.ZST
### 1. Scope / Trigger
Standalone/concatenated Zstandard and TAR.ZST/.tar.zstd/.tzst public reads, not creation or external dictionaries. Evidence: `repo:rust/crates/nxtools-archive/src/formats/zstd.rs`, `repo:rust/crates/nxtools-archive/tests/zstd_safety_contract.rs`, `repo:csharp/NxTools.PlatformHost.Tests/ArchiveStreamWorkerTests.cs`.
### 2. Signatures
zstd::preflight/stream/list/extract and tgz::matches_zstd/decode_zstd; unchanged ArchiveEngine/ArchiveOutput and private worker wire. Exact zstd-safe7.2.4 std,default-features=false, zstd-sys2.0.16/Zstandard1.5.7; no bindgen/legacy/mt/experimental. DCtx::try_create, set_parameter(WindowLogMax), InBuffer/OutBuffer and decompress_stream; return0 means actual frame end.
### 3. Contracts
Signature gates content routing,including skippable prefix but at least one real frame required. Preflight descriptors/dictionary/content size/window/block headers/extents/checksum and finite frame/block metadata before allocation. Reserve admitted work and set native WindowLogMax before streaming; no one-shot whole-frame allocation. Unknown listed expanded size stays None. Actual <=32KiB ordinary records and total/entry budgets/cancel apply; EOF succeeds only after complete last frame. Compound full-outer decode/checks into bounded anonymous spool before any target output; original compressed-source revision/lease remains preclose authority. Preserve typed output I/O/cancel/source errors and delete only incomplete owned final-name output. Checksums are verified when supplied; checksumless formats make no checksum promise.
### 4. Validation & Error Matrix
Reserved header/invalid block/extent/truncation/garbage/checksum/content-size mismatch→CorruptArchive. Work/window/metadata/frame/block/declared-or-actual output/spool→ResourceLimit before oversized allocation/publication. External dictionary→UnsupportedMethod. Skippable-only/truncated skippable→CorruptArchive for this product. Cancel/source change/I/O remain typed. No native target files on compound outer failure; native source lease denies writers and releases after actual drain.
### 5. Good/Base/Bad Cases
Good: independent bsdtar/libzstd input plus Python XXH64/published vectors and independent libarchive validation. Base: standard frames may omit checksum and listed size remains unknown. Bad: infer Zstd from suffix in unregistered reference backend, call DCtx::create panic constructor, treat decoder read0/timeout as verified frame end, or claim checksum on checksumless data.
### 6. Tests Required
Retain public UnsupportedFormat red→green; zstd_read_contract1 and zstd_safety_contract4 cover exact bytes/aliases/skippable/empty/single-segment/declaration/damage/budgets/cancel/I/O/original-source/token cleanup. ActualZstdReads50Job/native operations cover open/test/extract/count/state/checksum/truncate/garbage/window/metadata/output/source lease/native cancel/cleanup. Full affected archive + previous XZ/native/shared ZIP regressions rerun, not inherited. Actual counts in task checkpoint/research; broad performance/native-user/Phase A exit remains separate.
### 7. Wrong vs Correct
Wrong: hostile window allocation and generic io::Error strings. Correct: checked bounded headers -> admitted native limit -> fixed-step decode -> original-source check -> ordinary metadata+drain+close.

## Scenario: Original-record ZIP methods,private crypto and metadata/directory validation
### 1. Scope / Trigger
Phase A ZIP0/8/9/12/14/93/95 plain/ZipCrypto,AES reading;not editing/creation/full resources.
Evidence:`repo:rust/crates/nxtools-archive/src/formats/zip.rs`,
`repo:rust/crates/nxtools-archive/src/formats/zip_aes.rs`,
`repo:rust/crates/nxtools-archive/src/planning.rs`,
`repo:rust/crates/nxtools-archive/tests/zip_boundary_contract.rs`,
`repo:rust/crates/nxtools-archive/tests/zip_crypto_contract.rs`,
`repo:csharp/NxTools.PlatformHost.Tests/ArchiveZipReadRemediationTests.cs`.
### 2. Signatures
ArchiveEngine<'password>::with_password(self,&'password str) borrows operation secret only;
WorkerRequest.password stays existing private stdin,no snapshot/argv/log/config secret.
ZipLocation retains private Option<ZipAes{version,strength,method}>,while public method99 stays
original. ExtractionPlan.explicit_directories retains selected original EntryIds;directories()
also includes implicit parents,which have no separate payload.
### 3. Contracts
Independent original central ordinals/session/revision never reconstruct via filename-folding
ZipArchive. Local name/flags/method/date/time/size/CRC/ZIP64 and descriptor width/CRC/sizes/extents
must match. Execution re-admits current central+comment budget even for prior snapshots.
Metadata retained estimate central records/bytes plus raw EOCDcomment+worst3xlossyUTF8+fixed
container reserve is checked BEFORE comment copy/decode;the same cumulative value enters path
projection admission. Never let comment/central/projection independently spend full budget.
Selected explicit-directory payloads(plain/encrypted)stream once through full applicable checks
BEFORE directory effects;unselected unrelated records are not decoded. Legitimate compressed
empty data is valid,not restricted to packed_size0. Implicit parents are layout only.
Bounded Range Read+Seek cannot access adjacent records.95/93 reuse existing preventive XZ/Zstd
framing/memory/full decoders.14 ZIP9byte prefix requires2xpublic memory estimate+1MiB before
raw stream;bit1requires EOS,otherwise known size;subtract unused_input. Genuine7-Zip eos=off
positive is independent encoder output,not simply cleared flag.9 reserves1MiB and requires
true BFINAL/EOB without declared-size termination;final32bytes withheld/pending drained before
single-byte tail feeding prevent complete-byte read-ahead acceptance.
Traditional12byte header CRC-high-byte or bit3DOS-time-high-byte is probabilistic;fixed keys
zeroize. AES extra0x9901 must occur exactly once,length7,vendorAE,version1|2,strength1|2|3,
method99+encrypted flag and checked salt/verifier/ciphertext/MAC overhead;local equals central.
PKWARE strong/masked-header encryption is typed unsupported. Public RustCrypto PBKDF2-HMAC-SHA1
1000rounds,salt8/12/16,key16/24/32,LE128counter starting1. Authenticate complete bounded
ciphertext via Mac::verify_truncated_left(10) before decoding,even zero ciphertext;verifier
ct_eq is not authentication. Then seekable CTR/fixed streaming decode with full end/input/size;
AE1 additionally CRC,AE2 ignores content CRC. Source revision/cancel precede each ordinary close.
Incomplete final-name cleanup/native source lease/direct decided overwrite/completed-journal truth stay; archive candidate atomic-path publication remains separate.
### 4. Validation & Error Matrix
Unknown method/deprecated20→UnsupportedMethod;unknown AESversion/strength/strong encryption→
UnsupportedEncryption;malformed/missing/duplicate/truncated/contradictory extras→CorruptArchive.
No password→PasswordRequired;wrong/verifier/MAC/ambiguous encrypted codec/CRC→
AuthenticationOrDataError. Real source/outputIO,cancel,resource,SourceChanged stay typed.
Pure LZMA slice Other→CorruptArchive,not text-matchedIO. Invalid directory creates no associated
target;combinedcomment/central/projection budget→ResourceLimit,not corrupt/unbounded claim.
### 5. Good/Base/Bad Cases
Good: independent7-Zip/CPython/bsdtar oracles→public engine→actual Job/native bytes.
Base: selected explicit/implicit directories,CRC/time traditional headers,AE1/AE2normal/empty.
Bad: verifier equals authentication,decoderEOF skipsMAC,AE2CRC hardcoded,unselected directory
decoded,or comment excluded from reservation. No whole-archive/entryVec or plaintext spool.
### 6. Tests Required
Public directory test/extract consistency(normal/corrupt/truncated/unsupported+selected-only),
comment empty/max/lossy/combined/lower-execution-budget,original local/collision/StreamEnd gates.
Preserve unchanged AESpublic red→green;all6AEversion/strength Store/Deflate,AE2AES256extra methods,
ZipCryptoall7methods;exact1050bytes/empty/Unicode,extra/ZIP64/descriptors and validMACtrailing
compressed input. Native source lease/cancel/IO/target retention/owned cleanup/diagnostic secrecy.
Actual counts/current globalfmt limits live in task checkpoint,not inferred from spec.
### 7. Wrong vs Correct
Wrong: folded metadata→header verifier→publish;plain directory skipped;comment free allocation.
Correct: originalrecord+combinedadmission→selecteddirectory validation→fullcrypto/codec→source
recheck→native publish. Other AESextra-method version/strength cross-products,large/performance/
full Phase A remain unverified;this read scenario alone does not certify encrypted creation.

## ZIP creation verification seam
Evidence:`repo:rust/crates/nxtools-archive/src/creation.rs`,
`repo:rust/crates/nxtools-archive/src/worker_zip_create.rs`,
`repo:rust/crates/nxtools-archive/tests/zip_creation_worker_contract.rs`.
Use bounded native ID-source plans/reads and the existing output token. Store/Deflate1/6/9,
AES256 only through locked public writer;no private ZipCrypto API. Sequential directory records
must finish descriptors (new_stream/add_directory omitted them);use public start_file trailing
slash/Store. Full central/local/descriptor/extents preflight precedes per-entry CRC/AES/SHA256,
source revalidation/cancel and native publish. Native error retained even if backend closure
drains to discard;finish alone is not success. Independent Python/stock7Zip exact manifests,
actual Job/native source changes/reparse/budgets/EOF/cancel/overwrite/ownership/privacy and
schema/capability compatibility are required. See
`repo:.trellis/tasks/10-16-archive-manager/zip-create-evidence.md` for exact results/limits;
header-only ZIP64 test is not large/count evidence;actual4GiB+17FastDeflate native→Python
streaming hash now passes separately. ZIP64large-count/full-resource acceptance remains open.

## Tree creation verification seam
7z/TAR/TGZ/TXZ reuse bounded CreationPlan and protected native ID/partial runtime; exact wire,
first typed error,metadata/work/output/spool/full-readback/publication contracts live in
[native creation owner](../../platformhost/archive-creation.md). Sources:
`repo:rust/crates/nxtools-archive/src/worker_sevenz_create.rs`,
`repo:rust/crates/nxtools-archive/src/worker_tar_create.rs`.
Require per-format real Start/Job/native red→green,independent stock7Zip/Python exact manifests,
created-output open/test/extract and original ZIP regressions. Never compare physical directory
allocation with archive payload; PlannedSource.size0for directories while native snapshot stays
unchanged. Constructor seek/finish can wrap ResourceLimit: check first retained typed failure.
`repo:rust/crates/nxtools-archive/tests/tree_creation_worker_contract.rs` protects full corrupted/
truncated readback/no commit/source close/abort;public creation plan4096directory regression and
`repo:csharp/NxTools.PlatformHost.Tests/ArchiveTreeCreationTests.cs` cover real lifecycle/options/
source/cancel/budgets/overwrite/privacy. Exact current counts in single checkpoint,not inferred
from imports/registration. Full Phase A/native/product/resource acceptance remains separate.
