# Phase A capability matrix — current bounded evidence

**Phase A — Engine contracts and safety foundation: NOT passed.** Canonical A–F scope/order
unchanged. [Current checkpoint](phase-a-checkpoint.md) owns integration state;
[exit ledger](phase-a-exit-ledger.md) maps original exit IDs to exact evidence/gaps/decisions and
ONE recommended next slice. This update ran no product/build/performance checks.

| Capability | Applicable delivered / independent evidence | Specific remainder / original later owner |
| --- | --- | --- |
| ZIP plain0/8/9/12/14/93/95,ZipCrypto,AES reads | [Independent PASS](zip-aes-acceptance-review.md),including directory/comment repairs and genuine noEOS LZMA | No automatic allcrypto×method×size matrix; no current known decoder defect from these accepted slices. |
|7z LZMA/LZMA2/solid/AES/header;RAR4/5 | Actualworker/native plain/encrypted byte oracles/source/Job [repair](acceptance-remediation.md),later [independent unions](tar-time-acceptance.md) | C2:explicit7zBCJ coder/native bytes;C5:solidpackedunknown assertions. RARvolumes E/P202;RARcreate excluded. |
| Gzip/BZip2/XZ/Zstd/TAR and compounds | [XZ/Zstd PASS](xz-zstd-acceptance-review.md),retained earlier stream/TAR fixtures;outer integrity and boundedspool | No new externaldictionary/sparseTAR/BZip2orZstdcreate obligation. |
| ZIP/7z/TAR/TGZ/TXZ/Gzip/XZ creation | [Tree acceptance](tree-create-acceptance-review.md) plus [typed-I/O PASS](sevenz-create-typed-io-acceptance.md),independent readback/list/test/extract | C1:empty singleGzip/XZ independent roundtrip. Publiccreateform D;no percodec multiGiBmatrix. |
| ZIP64 singlelarge input | Actual4GiB+17FastDeflate/independenthash [PASS](tar-time-acceptance.md#independently-executed-verification) | No unrequired repeated large tests or allmethod×large proof. |
| Finite100k product/resource/reuse | [Independent PASS](product-100k-post-status-acceptance.md),100000exact/391batches/DebugPID samples/quotas/originalconnectioncleanup+reuse | Firstbatch≠firstpaint;sampledpeak≠absolute;noRelease1.5sclaim. LedgerD1 and layer-specific ceiling clarification,not wholly unimplemented. |
| Local Windows ZIP longpaths | [Independent PASS](windows-long-path-acceptance.md),realproduct/nativeextract/create/publication/unsafe cases | TrustedUNC distinctD3;not allformat longpathmatrix. |
| Realworker cancellation | [Independent PASS](worker-cancel-acceptance.md),receive/observe/reply/credit/pause/cleanup-heldleases+slots | B2actualoperationpromptwait;D1measurement/stage conflict. Not universalcodec/Win32200ms. |
| Encoding / safe plans | AuthoritativeUnicode/manualGBK/ShiftJIS/CP437/uncertainfallback [tests](../../../rust/crates/nxtools-archive/tests/engine_contract.rs) | B1Autocandidates;C4othermanualpages/reprojection. ONE nextslice;codepageUI C. |
| Native output/source/overwrite | [Approved review](overwrite-acceptance-review.md),[outcome repair](overwrite-acceptance-remediation.md),later independentLP/Q | B2perjobconflictpolicy;publicdialog D. OutputTOCTOU not sourceRecycle authority. |
| Metadata/time/attrs/MOTW | [Metadata acceptance](extraction-metadata-acceptance.md) plus [TARtime repair PASS](tar-time-acceptance.md) | Delivered,notunimplemented. SourceADS capturedsnapshot,not lifetimeADSlock guarantee. |
| Resource admission /risk | Finite count/model/work/output/Job tests and100kPASS | B3ratio+finiteoperationriskapproval. Layer-specific ceilings remain explicit;millionbenchmark not required. |
| Source-delete qualification | [Independent P1/P2 PASS](source-deletion-remediation-acceptance.md),originalRun/session/lifecycle/onceonly;recordingbackend | D2realstrictRecycle/directorycoverage unresolved;native_protection_unproven/NotAttempted. Integration D. |
| Currentworker and typedread IPC | [Product P2 PASS](product-read-p2-acceptance.md),finitev2,Q;legacy/capability/identity/bounds/cleanup | CurrenttouchedIPC01 boundedPASS;futurecorepromptpayloads B2/B3;FIFO/publicwrite/pickers/launch B–D;deployment Rel. |

Open actions:ledger B1–B3(function),C1–C5(named necessary evidence),D1–D3(decisions). No generic
“broader formats/resources/safety” backlog. Typedreject does not waive an approved capability.
UI/FIFO/split/ZIPedit/Shell/OLE/Release stay in their original stages. Sections below are
**preserved historical execution records**, not current remaining work or next-slice instructions.

## Historical Gzip red/green evidence

- Rust `stream_contract` before implementation:2selected/2run/0pass/2fail/0ignored,exit101, UnsupportedFormat. After implementation2/2,exit0. Independent single/multi-member exact bytes, CRC/truncation/budget/oversized optional-header rejects and zero invalid publication.
- C# `archive stream fixture` first listed2/run0; execution2selected/2run/1pass/1fail/0skip,exit1: extraction passed, create admission failed before implementation. Same two after implementation2/2,exit0; Fast/Normal/Maximum independently decoded by .NET.
- Added CreationSafetyAndPartialRead: three actual worker create(cancel/existing/budget) operations plus exact token/offset/size/published read refusals and unsupported option/self-output admission. Latest exact3fixtures: selected3/run3/pass3/skip0/fail0,exit0.
- `creation_worker_contract`: actual Rust child + bounded temp-file spool, normal vs corrupted native read-back.1test/2operations passes,exit0; damaged read returns CorruptArchive, abort and no commit. This tests private protocol, not Windows source-lease enforcement; real C# worker tests prove the latter.
- New dependency use only existing locked base64=0.22.1 (MIT OR Apache-2.0,pure Rust,no native build); already locked flate2=1.1.9/crc32fast/SHA256. No ZIP/GPUI/dependency-line upgrade. Same-feature Windows worker and C# builds pass,0C#warnings/errors. Strict Clippy initially rejected one manual modulo idiom; corrected without suppression,then exit0.

Independent fixture generator: `rust/crates/nxtools-archive/tests/fixtures/generate-streams.py` (CPython standard library). SHA256 independent.gz48FE0D149EE67F9F8A0628A0E065B652F8E748711584172809CE1A1956498554; multi.gz715D3BA66ACE15E47AE90460D58C96F81532FCAA8B1918485A4119F99DED1D2C. Expected single payload is UTF8 `独立 gzip fixture\n` then bytes0..255; multi=`first\nsecond\n`. No NxTools writer supplies the read oracle.

The full post-continuation check/count table is appended after actual execution. Two earlier confirmed outcome repairs are recorded separately in overwrite-acceptance-remediation.md; original independent FAIL records remain historical.

## Historical post-continuation integration

All commands serial with the3required Rust memory environment values, no timeout/duplicate.
Logs session scratch `phase-a-final-*.log` and `phase-a-final36-{list,tests}.log`.
- archive --features isolated-codecs --offline:37passed/0failed/0ignored,exit0.
- strict archive Clippy --tests/same features/-Dwarnings, same-feature worker build, fmt--all--check,
  workspace check--tests--offline:each exit0; workspace check is not workspace tests.
- C# build--no-restore:exit0,0warnings/0errors. Exact safe16+remediation6+overwrite7+outcome3+stream4
  list selected36/run0,exit0; execution selected36/run36/pass36/skip0/fail0/not_run0,exit0.
- extensions archive_import7/7; clipboard --lib archive_5/5,39filtered;exit0each.
- New ZIP12 red1run/0pass/1fail/2filtered,exit101 UnsupportedMethod before decoder; current stream
  group4/4exit0 includes independent exact data and work/corrupt/unfinished rejects, no publication.
- New dependency bzip2=0.6.1 wrapper MIT/Apache-2.0; locked backend libbz2-rs-sys0.2.5 is
  bzip2-1.0.6 license (NOT MIT/Apache); complete notice retained. Pure Rust,Windows build passed.
  Conservative8MiB work admission before small-mode constructor, no unbounded/multithread decode.
  ZIP8.6.0/its features/GPUI0.7.1 unchanged. Independent ZIP12 SHA256
  2A1A78694B606B080EA42B24D198F3DC4CB2270613BB861CB04F57AB883206BA.

One bounded read-only terminal-fix review found no concrete issue. No second review loop.
Initial wide ZIP research was stopped after excessive time; it is not evidence or a blocker.
Next concrete work: remaining ZIP required decoder/encryption fixtures and compound TAR streams,
then general creation/traversal. No phase completion claim.

## Historical plain-TAR continuation

After the preceding checkpoint, TAR public normal/malformed reds selected2/run2/pass0/fail2,
exit101 UnsupportedFormat. Fixed-header adapter then passes2/2. Additional extension safety red
selected3/run3/pass2/fail1,exit101: comment-only local PAX without a following entry was accepted.
Fixed explicit pending local-extension state, did not weaken fixture/expectation; same3/3pass.
During preparation one rejected unseen-anchor edit left engine dispatch old; its subsequent
UnsupportedFormat run is not parser failure/success evidence. One fixture line-range mistake
was corrected before execution; original valid15-byte comment-only PAX retained.

tar=0.4.46 exact crate SHA2563F6221D9A6003C78398E3B239969F352578258DF48C8EB051CAADAE0015BC840;
MIT/Apache-2.0,no build.rs,default xattr disabled. New transitive filetime0.2.29 MIT/Apache-2.0,
no build.rs; Windows build passed. Only fixed Header used; ordinary entries() pre-budget read_all
not used. <=64KiB extensions/count/metadata and checked offsets, GNU/PAX raw names/size/header
checksum/zero end validation; links/special skipped, sparse explicit unsupported. TAR does NOT
provide a payload checksum. Independent TAR SHA256
49A94456AD1B698F9339E75DEFCA849556C291934173FD564C0C4D355C38EC43.

Latest actual commands (same3Rust memory settings,serial,no timeout), logs `phase-a-latest-*.log`:
| Command | Result |
| --- | --- |
| cargo test -p nxtools-archive --features isolated-codecs --offline |40passed/0failed/0ignored,exit0 |
| strict archive Clippy --tests/same feature/--offline/-Dwarnings |exit0 |
| same-feature worker build |exit0 |
| cargo fmt --all -- --check |exit0 |
| cargo check --workspace --tests --offline |exit0,NOT workspace tests |
| C# test-host build --no-restore |0warnings/0errors,exit0 |
| safe16+remediation6+overwrite7+outcome3+stream5 --fixtures --list |selected37/run0,exit0 |
| same37 execution |selected37/run37/pass37/skip0/fail0/not_run0,exit0 |
| extensions archive_import / clipboard --lib archive_ |7/7 and5/5(39filtered),exit0each |

TAR actual native fixture4operations checks normal exact bytes/real empty directory/link absent,
unsafe/collision/oversized PAX no output and own-partial cleanup. Public stream group7tests
covers independent Gzip,ZIP12,TAR and corruption/resource/path/extension scenarios. Earlier37/36
table remains historical checkpoint, not overwritten with later numbers. Full/default C#,
workspace tests,UI,DPI,Shell/registry/service/release and broad resource/performance acceptance
remain unexecuted. No new user decision blocker; current next slice is compound TAR and remaining
required ZIP methods/encryption, followed by full source traversal/creation. Phase A not passed.

## Historical compound/standalone stream continuation

TAR.GZ red1run/0pass/1fail/7filtered,exit101: identified as ordinary Gzip. After bounded disk
spool and original-source precommit check, same test passes (normal/outerCRC/spool budget3cases),
strict Clippy exit0. ActualTarGzipExtraction3real worker/native operations passed in integrated
41Rust/38C# checkpoint,0skips,all owner/shared/build/lint/fmt/workspace gates exit0.

Continued BZip2 standalone(single/multiple members)/TAR.BZ2 red1run/0pass/1fail/8filtered,
exit101 UnsupportedFormat. After adapter exact bytes pass; added corruption/truncation/garbage/
work/output limits test. Latest stream_contract10/10exit0; actual stream fixtures7selected/
7run/7pass/0skip/0fail,exit0,including6BZip2 worker operations; strict Clippy exit0. Final whole
archive/existing fixture integration is recorded below after execution. No BZip2 creation added.

Promoted already-locked tempfile3.27.0 MIT/Apache-2.0 from dev to runtime, no version upgrade;
Windows implementation source inspected create_new/share0/delete-on-close, no persist fallback.
Spool maximum=max_output_bytes, fixed-memory decoder plus full outer integrity before inner
metadata/native output; preserves original compressed-source lease/revision, no secret spooling.
Hashes: TAR.GZ154F63B9D5158E458DED102BC1BD06B8F7D70B5C12F92C5929EF4DC4F818956E;
BZ2 2C808041D731E959D983B45A008DAB353A93C1EB64FBEA8FC21D1B4740AA0104;
multiBZ2 45A63CEA7BA54AEFA134F204D452279E291D2C006C4AE7D51B285308407D299F;
TAR.BZ2 E46A391BFE9FA76FFAEDB9E005C64E45D014C37C60C6FD178D38266BAFEBC437.
Still required: XZ/Zstd+compound, ZIP remaining methods/encryption, creation/traversal and all
native/contracts/resource exits. No current decision blocker, no Phase A exit or B start.

## Historical pre-XZ-creation integrated checkpoint

Logs `archive-final-{archive,clippy,worker,fmt,workspace,extensions,clipboard,csharp-build}.log`
and `archive-final39-{list,tests}.log` in session scratch534a5bed-05be-4587-9498-89ae907bc183.
All serial,required3memory variables on every Rust command,no timeout/duplicate build.
| Check | Actual final result |
| --- | --- |
| cargo test -p nxtools-archive --features isolated-codecs --offline |43passed/0failed/0ignored,exit0 |
| strict archive Clippy --tests/isolated-codecs/--offline/-Dwarnings |exit0 |
| same-feature worker build |exit0 |
| cargo fmt --all -- --check |exit0 |
| cargo check --workspace --tests --offline |exit0,not workspace tests |
| C# test-host build --no-restore |0warnings/0errors,exit0 |
| Original29+outcome3+stream/create7 --fixtures --list |selected39/run0,exit0 |
| Same39execution |selected39/run39/pass39/skip0/fail0/not_run0,exit0 |
| Extensions archive_import |7passed/0failed/0ignored,exit0 |
| Clipboard --lib archive_ |5passed/0failed/0ignored,39filtered,exit0 |

No new approval/technical blocker; remaining approved A work is incomplete, not waived. Next
implementation slice: XZ/Zstd standalone/compound and remaining ZIP Deflate64/LZMA/XZ/Zstd,
ZipCrypto/AES; then general ZIP/7z/TAR/TGZ/TXZ/XZ creation/source traversal. Native attrs/MOTW/
time/guarded recycle/source-delete, product IPC/capabilities/consumer backpressure and large
resource/performance/long-path/cancel acceptance remain. No full/default C# or workspace test,
native-user/UI/release acceptance; no Release/install/registry/service/commit/push occurred.

Final document/source gates: spec-contracts101docs/14owners; component catalog79pages/79rows/
78locked; task context20implement/17check,no truncation; fixture-selector gate; scoped diff
all exit0 (existing Program.cs CRLF advisory only). No DEBUG probes/LastCommitCategory recovery.
Final process inspection0Cargo/rustc/archive worker/test-host; fixed fixture helper absent,
0outcome/stream fixture temp roots. Newly generated fixture paths are not git-ignored. Source
reference note31902bytes,design32671,native owner32522 stay below32768injection limit. Text
hygiene found only missing task.json final newline; corrected and revalidated below.

## Historical XZ creation continuation — retained red

Exact source/decision/test record: research/xz-capability.md. Added direct locked lzma-rust2
0.21.0,std/xz/encoder (Apache-2.0,pure Rust,no version/ZIP/GPUI upgrade). Public reader's index
preallocation requires preventive project footer/index/header/record/block/chunk/dictionary
validation, not error-text matching. XZ full read-back decode checks payload CRC and SHA vs source.
Fixed preset1/6/7 dictionary1/8/16MiB; reserve16×dict+32MiB before encoder,Job hard cap remains.
Initial native create red admission exit1; next red ResourceLimit because upstream documented
KiB estimate mixes byte buffer and KiB finder terms. Source-traced conservative reserve fixed it;
real Fast/Normal/Maximum creation now passes independent CPython3.13.2 lzma length/hash oracle.

**Previous-turn execution boundary, now resolved:** engine.rs received3failed edits, latest
EDIT_LINES_UNSEEN unseen150–153. That turn stopped the path without shell/delegate/formatter
bypass. Public branch was not applied; registered red was retained, not ignored. This turn
fresh reads and grounded edits supplied all missing adapter/engine paths; actual public/native
reading is green. The following45pass/1fail table remains historical evidence, not current state.

| Historical check | Actual result at that checkpoint / exit |
| --- | --- |
| cargo test -p nxtools-archive --features isolated-codecs --offline --no-fail-fast |45passed/1failed/0ignored,exit101 |
| Only failing test |independent_xz_single_multi_and_tar_streams_require_complete_outer_integrity:UnsupportedFormat |
| XZ codec --lib |2/2pass (independent3fixtures+8bounded malformed/resource cases),exit0 |
| Actual worker creation readback test |1test/4Gzip+XZoperations pass,corrupt→abort/no commit,exit0 |
| worker_wire |5/5pass,including schema2Gzip/XZ/legacy/schema1 refusal,exit0 |
| Strict archive Clippy --tests/isolated-codecs/offline/-Dwarnings |exit0 |
| Same-feature worker build / C# build |exit0each,C#0warnings/0errors |
| cargo fmt --all -- --check / workspace check--tests |exit0each;not workspace tests |
| Exact C# original29+outcome3+stream/create8,list first |list selected40/run0;selected40/run40/pass40/skip0/fail0,exit0 |
| Extensions archive_import / Clipboard archive_ |7/7 and5/5(39filtered),exit0each |

Logs:archive-current-full-tests.log,archive-current40-{list,tests}.log,archive-current-
{fmt,workspace,extensions,clipboard}.log,xz-final-{codec,readback,wire,clippy,worker,native}.log.
XZ independent SHA256:single7BC385917C6865D3EE5BB80A33A96E74D79B94965379AFED2A4FF6C70A493DA0;
multiA6387305AB8728E52DC528E83212135902E19FA9DC6846B4E4B54B95CA801008;
TAR.XZ217F76BC10862ED9DF46EEE066ABCB4DE35034011F5AE076FDFFFC0C194784E5.
Explicit CPython3.13.2 executable SHA2567D96A4ED35D6E596CD9DD8933FEAFC66349CC21F75BE3B15C89FD336E50140C1.
No Phase A exit,B,release/native-user/full-suite-green claim. No further broad review was run.

Historical hygiene at that checkpoint: spec-contracts101docs/14owners,component catalog79/79/78,
task context21implement/18check(no truncation),fixture-selector,scoped diff/text hygiene exit0.
Those gates did not conceal the then-retained1Rust public-XZ failure. Current evidence lives
only in the linked current checkpoint and XZ read evidence.
