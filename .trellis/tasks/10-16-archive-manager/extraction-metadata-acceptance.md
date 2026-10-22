# Independent acceptance — extraction metadata / MOTW

Date: 2026-10-19. Canonical phase: **Phase A — Engine contracts and safety foundation**.
Reviewed delivery: extraction-metadata-contract/evidence and current source. No product or project
test code was modified during acceptance. Extensive unrelated dirty changes were preserved.

## Verdict

**Not fully accepted: two P2 TAR timestamp compatibility defects were independently reproduced.**
The delivered normal metadata/MOTW/native lifecycle fixtures and original regressions all pass.
No unsafe publication, data loss or false committed-count result was established by these probes.
Phase A remains in progress; no Phase B/UI/A–F/release acceptance.

## P2-1 — valid negative GNU base-256 header mtime is rejected

Location: `rust/crates/nxtools-archive/src/formats/tar.rs:213–228`, especially219–222.
The new metadata adapter calls unsigned `tar::Header::mtime()` and then converts to i64.
A valid signed GNU base-256 mtime=-1 therefore becomes ModifiedTime::Invalid rather than
UnixUtc(-1). This is within the native supported time range and needs no PAX override.

Independent fixture: CPython tarfile GNU_FORMAT, single payload.txt byte'x',mtime=-1.
CPython rereads mtime=-1/content'x'. Stock7-Zip26 test/extract exit0 and restores
1969-12-31T23:59:59Z; Windows bsdtar3.7.7/libarchive3.7.7 lists it successfully.
Actual C# Start/Job/Rust/native extraction, explicit copy_motw=false:

```json
{"fixture":"gnu-negative","Category":"worker_failure","NativeErrorCode":null,"FilesCommitted":0,"CommitState":null,"CleanupComplete":true,"error":"InvalidMetadata","files":[]}
```

Required repair: bounded signed TAR numeric decoding for mtime, distinguishing valid negative
GNU base-256 from malformed/out-of-range input. Do not reinterpret every high-bit unsigned value
as an allowed time, weaken checksum/extent validation, fall back to current time or add unrestricted
TAR unpack. Add public/parser and actual worker/native regression restoring the exact pre1970time,
including supported TAR compounds through the same parser. Retain invalid/range tests.

## P2-2 — empty global PAX mtime removal is treated as invalid metadata

Location: `rust/crates/nxtools-archive/src/formats/tar.rs:76`, with merge at213.
PAX global empty-value records remove the previously effective key; `9 mtime=\n` is not an
invalid decimal timestamp. Current pax() stores Some(unix_decimal(empty)) == Some(Invalid),
so later members fail instead of using their header mtime (or another applicable override).

Independent synthetic USTAR/PAX fixture sequence:
1. global mtime=1500000000;
2. first.txt, header mtime=1600000000;
3. global `9 mtime=\n` removal;
4. second.txt, header mtime=1600000001; two zero end blocks.
Expected effective times: first1500000000,second1600000001. Header checksums and payload sizes are
valid. Stock7-Zip test/extract exit0 and Windows bsdtar lists exit0. These tools restore raw header
times for both entries in this fixture, so their output does NOT independently certify PAX global
precedence; the expected merge/removal semantics come from the PAX contract, not that tool result.
Actual product first applies the global override correctly, then fails on the removal:

```json
{"fixture":"pax-clear","Category":"worker_failure","NativeErrorCode":null,"FilesCommitted":1,"CommitState":"Committed","CleanupComplete":true,"error":"InvalidMetadata","files":[{"name":"first.txt","mtime":"2017-07-14T02:40:00.0000000Z"}]}
```

Required repair: represent PAX key absence/set/removal as needed rather than feeding removal into
invalid-number handling. Preserve correct global/local/header precedence and local override scope;
check local empty-value semantics explicitly. Add set->clear->subsequent-member regression with
exact times, successful terminal and native cleanup. Do not silently ignore genuinely malformed
nonempty times. Do not broaden this fix into unrelated PAX keys without verified need.

Both candidates originated in one bounded Rust read-only review. Parent independently read the
call sites, constructed fixtures and reproduced both through actual C# Job/worker/native output.
No second review loop was launched. Neither failure is a merely hypothetical source finding.

## Independently rerun integration

Scratch exact commands/filter script: metadata-acceptance-verify.ps1; logs metadata-acceptance-*.
Final ledger08:31:47->08:42:16(+08:00), all executed steps exit0. Every Cargo inherited
CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256,serial lane checks.
An earlier guard stopped before ANY test because the build lane was busy. That ledger is retained
as metadata-acceptance-blocked-ledger.txt. Reinspection showed no build; ownership was not
established, nothing killed, no competing build or timeout retry. ZIP64 had not run in that guard.

| Check | Actual independent result |
| --- | --- |
| complete archive isolated-codecs,offline,no-fail-fast,serial |96passed/0failed/0ignored across21groups |
| same-feature worker build |exit0 |
| isolated/default strict archive Clippy including tests |both exit0 |
| workspace fmt --all --check |exit0 |
| workspace check --tests --offline |exit0; NOT workspace tests |
| extensions --test archive_import |7passed/0failed/0ignored |
| clipboard --lib archive_ |5passed/0failed/0ignored |
| C# product/test build |0warnings/0errors |
| exact original89+new17 fixture list |selected106/run0/pass0/skip0/fail0/not_run106 |
| same exact execution on final source |selected106/run106/pass106/skip0/fail0/not_run0 |
| LIST versus RUN names |0differences |
| actual4GiB+17FastDeflateZIP64 |RUN1/PASS1,once only this acceptance |
| retained independent missing-inside-output probe |OutputCollision,0commits,output_created=false,source members unchanged |
| spec gate |PASS103documents/14owners |
| catalog gate |PASS79pages/79rows/78locked |
| context validation |PASS40implement/37check |
| fixture-selector source gate |PASS |
| git diff --check |exit0; existing unrelated CRLF normalization warnings only |

Independent tools: explicit existing7-Zip26.00 SHA256
4bcc005758c5e604c0aaa257b6097846a4a708e62a9c0c7471fa360fad863f1f;
CPython3.13 local interpreter; Windows system bsdtar3.7.7/libarchive3.7.7.
No download/install/PATH change. New17tests cover normal per-format metadata, original-source MOTW,
options/budgets/absence, directory ownership/finalization, native failures/cancel/committed facts,
metadata-aware cleanup and diagnostic secrecy. Passing them did not cover the two inputs above.

## Additional native observation — not a finding

Scratch held-source ADS probe observed:
- after CaptureZoneIdentifier returns and closes its ADS handle, an ADS writer can open while
  the base source lease remains held;
- capture attempted with an already-active ADS writer fails motw_read_failed/native32.
This demonstrates the source mark is a captured snapshot, not an ADS read handle held until
publication. It does not invalidate the documented read-once capture by itself, and is NOT raised
as a third blocker. Existing base-stream write-denial tests alone do not prove lifetime ADS locking.

## Probe artifacts, cleanup and evidence limits

Scratch metadata-review-probe/{generate.py,Program.cs,Probe.csproj,probe.log} records reproduction.
Probe build initially lacked System.IO; that compile failure is not counted as a product red.
After fixing scratch only, both product failures and ADS observations above executed normally.
Synthetic archive hashes:
- GNU negative:8801c6c5fb0210ec140acb6f5bb70556dfe19e75867a583b6b7f17563ee9c71e.
- PAX clear:3e0a88ae26f64020eabbf87fa5101012c8648b5b82b088e6042a436f07cd35f9.
Generated archives/extracted probe trees are removed after evidence capture; scripts/logs retained.
Postbatch0Cargo/rustc/archive-worker/test-host/7z;fixed product-test/probe helpers absent;0inspected
archive-metadata/tree/source-binding/independent-output fixture roots. No product code fixed here.

No user archives,registry/associations/services/drive maps,Release package/install,commit or push.
No actual hardware fault,full workspace tests,unrestricted C#,FAT/SMB/UNC/DST-transition/UI/DPI/UIA/
Shell/native-user/release acceptance. Remaining Phase A native source-delete/recycle eligibility,
product IPC/helper mapping/FIFO/backpressure/conflicts and broad resource/performance exits remain
open in addition to the two concrete corrections required before this slice can be fully accepted.
