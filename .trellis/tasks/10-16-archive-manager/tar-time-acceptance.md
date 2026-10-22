# Independent acceptance — two TAR mtime P2 repairs

Date: 2026-10-19. Canonical phase: **Phase A — Engine contracts and safety foundation**.
Scope: GNU signed base-256 header mtime and PAX empty mtime semantics only.
Original [independent failures](extraction-metadata-acceptance.md) remain unchanged;
[repair evidence](tar-time-remediation.md) records the implementer's history.

## Verdict

**PASS for both requested P2 repairs.** Both original independent failing inputs were recreated
with the retained independent generator and rerun through actual C# Start/Job/Rust/native output.
They now succeed with exact expected Windows UTC times and truthful committed counts/cleanup.
No concrete new blocker was found in this bounded review. No product/test source was changed
by acceptance. No additional subagent review was used. This closes these findings, not Phase A
as a whole, Phase B prerequisites or A–F/release/user acceptance.

## Source review

Read the current TAR parser, six new public tests, four native fixtures, Python generator,
source/semantics evidence and applicable owner contract. The mtime-only change:
- consumes all12bytes using checked i128 accumulation and checked i64 conversion for GNU
  markers0x80/0xff; valid negative values remain typed signed Unix times;
- preserves missing all-NUL/space, strict octal digits/terminators and invalid marker/full-width
  overflow distinction; leading NUL followed by digits is not silently accepted;
- leaves native time-range validation intact, not a low64 reinterpretation or current-time fallback;
- represents PAX mtime Unspecified/Set/Delete. Global clear restores per-member header fallback;
  local empty suppresses global/header for that member only, then resets on a real member;
- retains last-record precedence, malformed nonempty values, negative fractional flooring and
  pending-local-extension checks. Other numeric fields and PAX keys are not generalized.

The local-empty rule and explicitly approved global-clear compatibility contract are documented
separately from differences in CPython/GNU/libarchive tool behavior. External acceptance of a TAR
is not presented as proof of PAX precedence. No dependency/native publication/cleanup/MOTW authority
change was needed. The original integrity/path/source/budget pipeline remains in place.

## Original independent probes — now green

Retained scratch metadata-review-probe/generate.py and Program.cs were used unchanged, rebuilt
against current PlatformHost and the newly built isolated-codecs worker. Same raw fixture hashes
as the earlier acceptance:
- GNU:8801c6c5fb0210ec140acb6f5bb70556dfe19e75867a583b6b7f17563ee9c71e.
- PAX:3e0a88ae26f64020eabbf87fa5101012c8648b5b82b088e6042a436f07cd35f9.

```json
{"fixture":"gnu-negative","Category":"success","NativeErrorCode":null,"FilesCommitted":1,"CommitState":"Committed","CleanupComplete":true,"error":null,"files":[{"name":"payload.txt","mtime":"1969-12-31T23:59:59.0000000Z"}]}
{"fixture":"pax-clear","Category":"success","NativeErrorCode":null,"FilesCommitted":2,"CommitState":"Committed","CleanupComplete":true,"error":null,"files":[{"name":"first.txt","mtime":"2017-07-14T02:40:00.0000000Z"},{"name":"second.txt","mtime":"2020-09-13T12:26:41.0000000Z"}]}
```

The probe also repeats the previous nonblocking ADS observation: writer allowed after captured
ADS handle closes; capture with active ADS writer denied native32. This is not a new finding or
proof of lifetime ADS locking. The output-inside-source independent probe still returns
OutputCollision/0commits/output_created=false and unchanged original two source members.

## Independently executed verification

Final foreground serial batch10:09:48->10:20:41(+08:00), all steps exit0, no interruption/timeout.
Exact commands/filters/ledger/logs: session scratch tar-time-acceptance-verify.ps1,
tar-time-acceptance-ledger.txt,tar-time-acceptance-*.log. All Cargo inherited
CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256 and checked machine-wide
Cargo/rustc lane before starting. No repeated ZIP64 or duplicate build was launched.

| Check | Actual independent result |
| --- | --- |
| focused public tar_time_contract |6passed/0failed/0ignored |
| complete archive isolated-codecs,offline,no-fail-fast,serial |102passed/0failed/0ignored across22result groups |
| actual same-feature worker build |exit0 |
| isolated/default strict archive Clippy including tests |both exit0 |
| workspace fmt --all --check |exit0 |
| workspace check --tests --offline |exit0; NOT workspace tests |
| extensions --test archive_import |7passed/0failed/0ignored |
| clipboard --lib archive_ |5passed/0failed/0ignored |
| C# product/test build |0warnings/0errors |
| original106+new4 exact fixture list |selected110/run0/pass0/skip0/fail0/not_run110 |
| same exact execution |selected110/run110/pass110/skip0/fail0/not_run0 |
| LIST/RUN/PASS names |0differences |
| actual4GiB+17FastDeflateZIP64 |RUN1/PASS1,once only in this acceptance |
| two original TAR probes |success,exact UTCtimes,commits1/2,cleanuptrue |
| original missing-inside-output probe |PASSunchanged rejection/no directory side effect |
| spec gate |PASS103documents/14owners |
| catalog gate |PASS79pages/79rows/78locked |
| task context |PASS43implement/40check |
| PlatformHost selector source gate |PASS |
| git diff --check |exit0; unrelated existing CRLF normalization warnings only |
| changed Rust/test/generator whitespace/conflict/DEBUG hygiene |PASS |

The focused6 are included in102; do not add them as extra unique tests. Four new C# entries contain
multiple small scenarios, not extra harness counts. Actual native assertions cover GNU-1 and global
clear through plainTAR/GZ/BZ2/XZ/ZST, local override/delete scope, absence/invalid/native-range values,
exact bytes, old target/foreign partial preservation and deterministic failure after one file
commit. Existing signed-PAX fractional/native metadata/MOTW/typed-I/O/source-binding suites remain.

The current green was independently run; historical red revisions were not restored/reexecuted.
Prior red evidence remains the original acceptance and implementation record, not rewritten.
Explicit local7-Zip26.00 oracle SHA256 matched
4bcc005758c5e604c0aaa257b6097846a4a708e62a9c0c7471fa360fad863f1f.
Python original GNU encoder/raw PAX bytes and independent Windows UTC file-info supply expectations.
No download/install/PATH change or user archive operation.

## Cleanup and remaining boundaries

Postbatch0Cargo/rustc/archiveworker/testhost/7z;product-test/probe fixed helpers absent;0inspected
archive-tar-time/metadata/tree/source-binding/independent-output temporary roots. Scratch owned-data
TARs and extracted files are removed after hash/results capture;generator/probe sources/logs retained.
Free disk observed C26.5GiB/D49.5GiB before removing the small probe data. Unrelated dirty state kept.

No packaging/install/commit/push,registry/association/service/drive-map change,source deletion or UI.
Full workspace tests,unrestricted C#,FAT/SMB/UNC/DST/large-count/RSS/performance/native-user/release
acceptance were not performed. Remaining Phase A source-delete/recycle eligibility,product IPC/
capabilities/helper mapping/FIFO/backpressure/conflicts and broad resource/native exits stay open.
The repaired metadata/MOTW slice may now retain its bounded accepted checkpoint without claiming
those remaining gates are complete.
