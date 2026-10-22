# ZIP remaining reads — exact capability and slice evidence

Phase A — Engine contracts and safety foundation. Overall current state belongs only to
[checkpoint](../phase-a-checkpoint.md);current remediation:[ZIP fixes](../zip-read-remediation.md).

## Capability table after this slice

| ZIP ID / encryption | Delivered and independently verified | Exact dependency / public API | Stream end / input consumption | Allocation and errors |
| --- | --- | --- | --- | --- |
| 0 Store, plain | Existing public/native regressions rerun | std::io::Take | exact bounded bytes + size/CRC | fixed buffers; corruption typed |
| 8 Deflate, plain | Existing public/native regressions rerun | flate2=1.1.9 Decompress/total_in/Status::StreamEnd | explicit end + exact input/size/CRC | fixed buffers/state |
| 12 BZip2, plain | Existing independent CPython/native regressions rerun | bzip2=0.6.1 Decompress::new(true)/decompress/total_in/StreamEnd | explicit end + exact input/size/CRC | conservative8MiB before allocation |
| 9 Deflate64, plain | NEW public/native normal,damage,budgets,cancel/source/cleanup | deflate64=0.1.12 InflaterManaged::new/inflate/finished/errored; InflateResult | true BFINAL/EOB,never declared-size stop; final32bytes held and one-byte-driven after pending drain; exact final consumed count | reserve1MiB before fixed131072window/tables; CorruptArchive/ResourceLimit/Cancelled/IoFailure retained |
| 14 ZIP-LZMA, plain | Public/native EOS and genuine stock7-Zip eos=off known-size sample verified | locked lzma-rust2=0.21.0 LzmaStream::new_with_props/process/total_in/unused_input/Status::StreamEnd | ZIP9byte prefix;bit1requires EOS,no bit1known size;explicit end and unused_input subtraction | 2xmemory estimate+1MiB admission;known-size/truncated/wrong-size/mismatched-EOS tests |
| 95 ZIP-XZ, plain | NEW public/native normal,safety,empty,Unicode | locked lzma-rust2=0.21.0 XzReader::new_mem_limit + existing project xz::preflight/stream over Read+Seek | bounded exact original payload; preventive footer/index/block/chunk/dictionary checks + full decoder + EOF/CRC | existing XZ memory limits before allocation,typed errors |
| 93 ZIP-Zstd, plain | NEW public/native normal,safety,empty,Unicode | locked zstd-safe=7.2.4 DCtx::try_create/set_parameter(WindowLogMax)/decompress_stream | frame return0 + exact bounded view EOF/CRC | existing preventive window/frame/block/metadata admission |
| 20 legacy Zstd | Explicitly rejected,not silently aliased | PKWARE6.3.10§4.4.5 and locked zip8.6.0 agree20 deprecated/93 current | n/a | UnsupportedMethod; no historical20 interoperability sample |
| Traditional ZipCrypto0/8/9/12/14/93/95 | CRC/time-header oracle and all7method combinations independent/public/native exact bytes | project fixed Zeroizing3key state + current method decoders |12byte header is probabilistic;full CRC/size/end/input mandatory | PasswordRequired;ambiguous AuthenticationOrDataError;IO/cancel/resource/source retained |
| AES AE1/AE2×128/192/256 Store0/Deflate8 | Independent Python cryptography/stdlib+7-Zip,public/native exact bytes and fault matrix verified | aes0.8.4+zeroize,hmac0.12.1,pbkdf20.12.2,sha10.10.6,subtle2.6.1 | full bounded ciphertextHMAC10verified before decoded output,including zero length;private actual method separate99;AE1CRC vs AE2exception | strict extra/vendor/version/strength/local equality/overhead;UnsupportedEncryption/Method vs auth/IO/budget/cancel/source |
| AES extra methods9/12/14/93/95 | AE2AES256 independently/public/native tested;other AESversion/strength cross-products NOT executed | same private bounded CTR view + current decoders | preventive method preflight cannot skip already completed authentication | broader matrix/interoperability/large resources unverified |

New method/combination fixtures contain3records,2regular files/empty file/directory,Unicode
path and1050body bytes;genuine noEOS fixture has1file. No AES/general creation claim.

## Verified sources and preserved boundary

- Legacy tracked search again found only BrowserExtensionPackageService/FfmpegInstallerService and
  image/video compression,not a general archive UI. Not an exhaustive untracked/generated search.
- Archiver core/src/lib.rs499/589–598/700–705 constructs ZipArchive/by_index_decrypt. Not copied:
  exact zip8.6.0 src/read/zip_archive.rs SharedBuilder inserts raw names into IndexMap and folds
  duplicates. Next preserves original central records/session/revision/ordinal and never reparses
  payload selection through ZipArchive. Local flags/method/name/date/time,size,CRC,ZIP64 and data
  descriptor are checked; descriptor extent participates in overlap detection.
- Official PKWARE APPNOTE6.3.10 FINAL2022-11-01 from
  https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT §§4.4.5/5.8.8/5.8.9; fetched to
  scratch only,not redistributed. ZIP-LZMA prefix is not a .lzma file header.
- xz/zstd only generalized their existing File-specific read/preflight helpers to Read+Seek.
  zip_range::Range confines seek/read to the original validated record. Full codec integrity,
  actual output and original-source revision precede current-file publication. Native source
  lease,held parents,owned partials,accepted rechecked-path overwrite/TOCTOU and postcommit
  ownership remain unchanged. No UI/focus/window or native production code changes.
- New encrypted path borrows `ArchiveEngine::with_password(&str)` operation-scoped password;
  snapshot never retains it. Worker uses its existing private framed stdin/zeroized request.
  C# still has its pre-existing immutable managed password lifetime; no new argv/log/config/disk
  password storage. Diagnostics reuse existing result projection,not raw errors/secret frames.

## Exact dependencies and independent fixture provenance

Prior method slice added deflate64=0.1.12,MIT (.NET Foundation and Contributors;anatawa12 2023),
pure Rust/no runtime deps/default features off. AES remediation newly resolves sha1=0.10.6,
reuses locked AES/HMAC/PBKDF2/subtle;Windows build and exact licenses in remediation/notices.
Its ordinary bytes_consumed includes bit-reader prefetch; checkpoint cannot return drained final
state and is not enabled. Final32byte withheld/empty drain/single-byte tail strategy is protected
by nonoverlapping tails00/ff/50 and unfinished stored-block regressions. Broader bit alignment,
large-data performance/cancellation remains unverified. No zip8.6.0/GPUI0.7.1 upgrade or feature change.

Synthetic generator files live in tests/fixtures and never invoke NxTools writers. No global
software install/PATH modification/user archives. 7-Zip is an independent local test oracle,not
product backend/bundled code. Empty Deflate640300 is a specified fixed-Huffman EOB verified by7-Zip.

| Fixture | Independent producer/reader | SHA256 |
| --- | --- | --- |
| independent-zstd.zip | bsdtar3.7.7/libzstd1.5.4 + Python framing;libarchive ZIP read | bdfc5d2824d97db352efa0aa3e86d73ec625ba70b14ba91dc2501ad62adea213 |
| independent-xz.zip | CPython3.13.2 liblzma preset1 + framing;libarchive ZIP read | 3314ccba53f3753a6c4965c2400cf4f9698dcda41e0add2437400d5464f5fada |
| independent-lzma.zip | CPython ZIP_LZMA;CPython+libarchive read | 29648880cb441be7628feac99444915a447f46678dde433e05c4d2b9dd4c21ba |
| independent-deflate64.zip | installed7-Zip26.00 payload + framing;7-Zip t/x -so | 185b1586c1068b6397feac0123d551f2ccf56e7432a370ce555b935ab5f51852 |
| independent-zipcrypto.zip | Python PKWARE framing/encryption;CPython zipfile decrypt | 4891c26b024c7bf3d30b725fda83a9152b740cdc11973e00f39b98054749672f |
| independent-zipcrypto-dd.zip | same,bit3DOS-time password header+real descriptor | 0f7b8d1d4f7bcccf9d07020e5a366b43d1c727a770b41f0e22c02ffd03225feb |

Plain-method body SHA256 e886ea86544c0dba2061c9369fea8cf668b61423f6ce0c2c15e012d2bb7c53af;
crypto body SHA256 e047f4623a80710bce0458a2198b9dc5e34c617f48f2012e3dd94ea4f1a8a7b5.
libarchive3.7.7 explicitly cannot decode9; it was not counted as a passing Deflate64 oracle.

AES generator uses existing Python cryptography/OpenSSL AES ECB for ZIP LE-counter blocks and
stdlib hashlib.pbkdf2_hmac/hmac. Independent7-Zip t and x -so verify six AE1/AE2×128/192/256
archives; AE2 Store full t verifies empty ciphertext. Password stdin only: omit-p to prompt,
bare-p means empty password in this tool. Expected body1050bytes/hash above.

| AES fixture | SHA256 |
| --- | --- |
| independent-ae1-128.zip | 81c12346d810289c89440db3d9094ed2f42b144a7a425088daa66c4e2653e73b |
| independent-ae1-192.zip | 43ceab1f172eb28a76a29b2d582927e7c7199716d38867f4054a1ce64dea339d |
| independent-ae1-256.zip | 676d6822499bae768bb18b576fa7176b17d31168eb7500e832ce401be3b84d5a |
| independent-ae2-128.zip | 556e4b7f1d4626c07663eb94546e99a7ca42e90e1f10547d17d6703838269aac |
| independent-ae2-192.zip | 97a13803141a4d04b2315ab055eb0dd5b1eb006a9bd1a1fe1206ee2a7a872566 |
| independent-ae2-256.zip | 7884f120c15de0f36e0676fe1a2eba01e6045104ec7255d15cad09140be99959 |
| independent-ae2-store.zip | c1d2137b4b369fc3a886b155426e21c8a3f73e8a2277296fba5e792b92109ab2 |

## Red/green history (not competing current checkpoints)

- Historical pre-remediation93/95/14/9 each independently generated fixture then public red1run/0pass/
1fail/0ignored,exit101 before minimal adaptation. Then plain ZIP group12/12,including
four damage matrices,lower execution budget/forged descriptor,EOS properties/dictionary,
unfinished Deflate64 and nonoverlapping full-byte tails. Each method native fixture20operations.
-93 safety2pass/1fail exposed admitted snapshot bypass of lower max_entries; fixed current central
budget admission. Local/central bit3 with absent descriptor is rejected; descriptor extents checked.
-LZMA safety7pass/1fail showed codec Other incorrectly IoFailure; mapped at pure slice codec
boundary,not by text. Real source/output errors remain independent. No-EOS was pending then;now verified.
-Deflate64 initial10pass/1fail was a test assumption(first7-Zip block final),not product decode;
replaced with specified BFINAL0 stored block preserving output/CRC but lacking final termination.
-Native93 initial1fail was nonexistent test bytes key; fixed to existing bytes_verified,no protocol
change. Later exact method4/4native and ZipCrypto2/2native all pass,0skips.
-ZipCrypto correct-password red PasswordRequired; after propagation1/1,then safety1pass/1fail
exposed local/central DOS-time mismatch; fixed exact date/time compare,unchanged2/2green.
-AES historical public red1fail UnsupportedMethod vs PasswordRequired;unchanged test now green.

## Historical pre-remediation integration and execution boundary

All Rust commands same environment CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256,
serial,no timeout/duplicate build. Logs session scratch370bbe8f-756d-402b-aa5c-34da1adf8814:
zip-slice-{archive,worker,clippy,fmt,workspace,clipboard}.log,zip-slice-extensions-exact.log,
zip-slice50-{list,tests}.log. Earlier directed logs zip{93,95,14,9}-*,zipcrypto-* retained.

| Check | Actual result |
| --- | --- |
| archive --features isolated-codecs --offline --no-fail-fast |71pass/1fail/0ignored,exit101;only retained AES public red |
| Same-feature actual worker build |exit0 |
| strict archive Clippy --tests --features isolated-codecs --offline -- -D warnings |exit0 |
| cargo fmt --all -- --check |exit1;formatting differences only in stopped zip.rs |
| cargo check --workspace --tests --offline |exit0,NOT workspace tests |
| C# test-host build --no-restore |exit0,0warnings/0errors |
| original44+new4plain+2ZipCrypto exact union |list selected50/run0;execute selected50/run50/pass50/skip0/fail0/not_run0,exit0 |
| extensions --test archive_import |7pass/0fail/0ignored,exit0;initial name-filter zero-match was rejected as evidence |
| clipboard --lib archive_ |5pass/0fail/0ignored,39filtered,exit0 |

Six new native harness fixtures execute115operations (4×20 + ZipCrypto32 + descriptor3),not115tests.
No AES native output,all encrypted method combinations,no-EOS positive,large/RSS/DPI/UI/Shell/
release/install/service/registry/workspace-tests or source-deletion acceptance claimed.

**Historical previous-turn edit stop,resolved this remediation turn:** zip.rs got3failed Edits:
EDIT_PARSE_FAILED(empty colon PUT),EDIT_LINES_UNSEEN(line494,complete reveal+successful retry),
and EDIT_RANGE_INVALID(two ops at insert gap414) during AES integration;then tag7949/705lines.
no further Edit/Write/shell/formatter/delegate mutation of zip.rs. AES draft copied to scratch
zip_aes-unintegrated-draft.rs only;unconnected module/types/dependencies were removed from product.
The proposed aes0.8.4+zeroize/hmac0.12.1/pbkdf20.12.2/subtle2.6.1 and new sha10.10.6 were source/
license reviewed(pure Rust,no asm/native/rayon;subtleBSD-3-Clause correction in remediation);then NOT retained
runtime deps. No claim that AES draft is correct or delivered.

The permitted continuation described above was executed in the remediation turn:live full Read,
small grounded edits,strict AES integration,formatting and77/77archive+54/54C# now pass. The
previous raw errors remain historical,not a permanent file/architecture blocker. Current
limitations and unrelated whole-workspace fmt failure are owned by the linked checkpoint.
No general creation/UI work was started in the remediation turn.

Historical document/selector gates passed:spec-contracts101docs/14owners,component catalog79pages/
79rows/78locked,task context25implement/22check,no truncated referenced file;fixture-selector
source gate,scoped git diff--check exit0(existing Program.cs CRLF advisory only). Reference
note32712bytes,archive quality28459bytes. Process inspection0Cargo/rustc/archive worker/test-host/
7-Zip,fixed fixture helper absent,0archive-zip-method temporary roots. Scratch draft/oracle spec/
download and test logs intentionally retained outside workspace;no third-party cache/source edits.
Final read-only hygiene:30text/Python-syntax/JSON files,13fixture hashes/not-ignored checks,
all context files<=32768 passed;task context25/22 revalidated,scoped diff exit0. No DEBUG probes/
ignored tests. Final zip.rs SHA256B5D226793C76FB51DD298D1E4761E5F70035AC24102F840FB64EB840EE027949;
lock confirms only deflate64 added to archive dependencies,no AES draft dependencies retained.
No broad reviewer pass was started because execution stopped with explicit unresolved work.
