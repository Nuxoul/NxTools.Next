# Extraction metadata / MOTW — implemented slice evidence

Historical delivery evidence below predates the two independently reproduced TAR P2 findings.
Do not use its earlier no-blocker sentence as current acceptance. Findings remain in
[independent acceptance](extraction-metadata-acceptance.md); this repair and new-run evidence
are in [TAR time remediation](tar-time-remediation.md), current status only in the checkpoint.

Date: 2026-10-18. Canonical phase remains **Phase A — Engine contracts and safety foundation**;
this slice is implemented and isolated/native-fixture verified, not Phase A/A–F completion.
Current task state has one owner: [phase-a-checkpoint.md](phase-a-checkpoint.md).

## Scope, sources and exact support

Approved: restore archive mtime and supplied Windows readonly/hidden; optional original archive
Zone.Identifier copy, default enabled. No UI, source deletion/Recycle Bin, product IPC/queues,
packaging/install, registry/service/mapping, user archives, commits or pushes.

- [Executable contract](extraction-metadata-contract.md).
- [Exact source/API/time/attribute matrix and native limits](research/extraction-metadata-sources.md).
- [Independent synthetic RAR generator/oracle and preserved initial fixture history](research/rar-metadata-fixtures.md).
- Owner contracts: [Rust](../../spec/nxtools-archive/backend/extraction-metadata.md),
  [PlatformHost](../../spec/platformhost/archive-extraction-metadata.md).
- Actual references: legacy tracked NxTools archive/package consumers; Archiver/SupaZip public
  source; locked zip8.6.0,sevenz-rust2=0.23.0,unrar_sys=0.5.8/UnRAR7.1,tar0.4.46,
  flate2=1.1.9,bzip2=0.6.1,lzma-rust2=0.21.0,zstd-safe7.2.4; WindowsSDK10.0.26100.0.
  No Bandizip proprietary runtime source was available or claimed.

| Format | Delivered archive time | Delivered Windows attrs |
| --- | --- | --- |
| ZIP | Original central NTFS100ns UTC > UT u32Unix seconds > DOS local2s; centralUT length5/full flags handled | Windows-origin0/10/14 readonly/hidden only |
| 7z | presence-qualified NtTime100ns since1601 | presence-qualified Windows readonly/hidden only |
| RAR4/5 | DLL normalized UTC FILETIME,0 API-unavailable; source precision/encoding/zone ambiguity not recoverable | public Windows host2 only; Unix mode ignored |
| TAR/TAR.GZ/BZ2/XZ/ZST | localPAX > globalPAX > header Unix; bounded signed decimal fractions,earlier100ns native floor | absent; Unix mode not replayed |
| Gzip | genuine first member RFC1952MTIME UTC seconds;0 unavailable | absent |
| standalone BZ2/XZ/ZST | genuinely absent; never now/source-stat metadata | absent |

DOS is not relabeled UTC. Native valid/ambiguous/nonexistent local calendars are distinguished.
Supplied invalid/out-of-range data is explicit failure. ZIP FILETIME0 remains a valid archive
value but Windows SetFileTime returned success/no-op on this workstation; exact held-handle
observation detects mismatch and fails metadata_time_not_supported before publication, native
code null rather than an invented error. Unix1970zero restores normally. FAT/SMB precision and
remote/native-user acceptance are not inferred from NTFS fixtures.

## Lifecycle, authority and failure policy

Rust retains immutable bounded typed metadata; C# alone executes Windows APIs. File lifecycle:
existing decode/integrity/plan/source checks -> managed flush -> fixed MOTW -> time -> whitelist
attrs -> native flush/held observation -> original target recheck -> original single publication.
No final-path metadata mutation, extra publication/fallback/delete/truncate/backup-first/retry.
Native success immediately retains FilesCommitted/state before later close/hash/cancel. Target
close->publication accepted TOCTOU is unchanged, not ID-CAS.

Source MOTW: held original source object, fixed ADS and same volume/fullID; bounded65536bytes,
exact read/extra-byte/recheck; source read share-read denies concurrent ADS writers during capture.
Target/cleanup ADS share-delete only coexists with held base DELETE access; base protections do
not widen. Bytes remain C#-owned and zeroized, never business wire/logs. Compound source is original
compressed archive, not spool. Held-volume named-stream capability distinguishes unsupported
from missing. Empty ADS stays empty, absent stays absent; disabled does no source read or target
write. Native/budget/unsupported errors are explicit, not full success.

Only actually CREATE_NEW directories are metadata-owned. Existing/raced-existing directories are
layout-only; explicit new directories finalize deepest-first after files. Failure retains created
directories; no recursive rollback. Directory failure/cancel after files keeps real committed count
and Committed state. Owned partial readonly is cleared only on its held object. Optional guarded
new-output deletion requires fullID,size,mtime,content hash,actual attrs and fixed ADS digest/absence
for **every output**, even one with no SetMetadata/BindZoneIdentifier call. External changes retain
output; replacements/unknown/published-failed remain nonrollback.

Schema2 predates this slice: it alone is not metadata negotiation. New C# caller omission defaults
on and explicitly packs extract_options; new host requires both capabilities. Omitted raw private
wire is old-host authority and emits no unknown metadata actions. Explicit copy_motw=false still
restores archive metadata. Actual worker compatibility test runs12 ZIP/7z/RAR4/RAR5 transactions
for old-omitted/newfalse/newtrue; schema1 illegal options remain rejected. This clarifies an
ambiguous sentence in the initial contract, not a product-default change or authority expansion.

## Actual red -> green and diagnostic evidence

Logs are in session scratch `metadata-*`; no historical baseline is counted as this run.

- ZIP DOS1fail ->1pass; NTFS UTC1fail ->2pass includingDOS; attributes1fail ->3pass.
- Default MOTW initially missing -> native32 due ADS/base DELETE sharing -> fixed stream-only
  sharing ->4pass. Options/capability initially unknown wire/absent-as-empty nullable failure ->
  fixed explicit negotiation/nullable absence ->6pass; worker_wire9/9 at that stage.
- Directory parent time overwritten ->7pass after deepest-first owned-directory finalization.
- TAR initial .NET8 fixture was structurally invalid to the existing parser(all-NUL directory
  size); not counted as a metadata red and parser safety was not relaxed. Independent CPython
  TAR member time red -> green. Gzip MTIME red -> green. TAR5variants includingZST now pass.
- External7z source/oracle exact time/attrs passed while product FILETIME red -> green.
- RAR4 normalized time red -> exposed actual pack1 ABI mismatch: vendor dll.hpp4/78-117 versus
  upstream repr(C). Private exact packed header(offset6188/6268,size10244win64) fixed it.
  Initial WindowsRAR5 backslash accepted by7z but normalized to underscore byUnRAR was fixture
  error,not a product path regression;canonical '/' fixture reverified externally and all3 pass.
- Legal ZIP centralUT flags7+mtime-only red InvalidMetadata -> fixed. Signed negative PAX
  sub-ns red toward-zero -> earlier100ns floor fixed. Zero/absence/invalid/out-of-range covered.
- Native lifecycle11modes: time/attrs/ADS denied,ADS unsupported,cancel after readonly metadata,
  directory postpublish failure,postcommit observation failure,throwing log,overwrite,no-replace.
  Directory failure state red FailedBeforeCommit -> correct Committed/count1/code5.
- Guarded cleanup red discarded changed attrs -> attrs/ADS credentials fixed;unique read-only
  review found no-metadata variant bypass;new plain_ads_added red removed externally changed
  output -> unconditional fixed-ADS credential -> green5cleanup scenarios.
- ADS source/output unsupported flags red success -> explicit motw_not_supported,code null.
  Explicit disabled0capability probes;source base write denied32 at before_metadata;source ADS
  unchanged. Logs assert same correlation,exact code/category/count and no synthetic URL/ADS/
  path/member payload. Throwing sink success remains success.

One optional bounded read-only review only. Its concrete P1 cleanup omission was fixed red->green.
Its P2 proposed schema2-as-authority would break old-host action compatibility;retained explicit
negotiation,clarified C# versus raw-wire defaults and added actual12transaction test instead.
No second broad review. Production low-noise lifecycle diagnostics remain;temporary DEBUG probe
was removed. Per-file/pointer/frame/byte logging was not added.

## Final integration and exact results

Every Cargo inherited CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256;
checked lane,no duplicate Cargo or timeout-survivor build. Exact commands in scratch
`metadata-final-rust.ps1`, `metadata-final-csharp.ps1` and recorded tool commands/logs.

| Check | This-run result |
| --- | --- |
| full archive isolated-codecs --offline --no-fail-fast | initial95pass;after compatibility regression final96pass/0fail/0ignored,exit0 |
| isolated actual worker build | exit0,including postreview |
| default+isolated archive strict Clippy --tests | exit0 after private return-type alias fixed initial isolated type_complexity;no warning suppression |
| fmt --all --check | exit0 |
| workspace check --tests --offline | exit0;explicitly NOT workspace tests |
| extensions --test archive_import |7pass/0fail/0ignored,exit0;earlier name-filter0tests not counted |
| clipboard archive_ |5pass/0fail/0ignored,39filtered,exit0 |
| C# build |0warnings/0errors,exit0,including postreview |
| original89 union new17 list |selected106/run0;all exact names retained |
| final C# integrated run |selected106/run106/pass106/skip0/fail0/not_run0,exit0 |
| actual4GiB+17ZIP64 |RUN1/PASS1 only in final106;real Job/native publication+independent Python size/SHA256 |
| postreview C# repair integration |list105/run0,then selected105/run105/pass105/skip0/fail0/not_run0,exit0;sole excluded name is already-passed ZIP64,not rerun |
| final additive combination tests |17/17newfixtures,skip0/fail0/not_run0,exit0;adds multi-file exact MOTW and directory-stage cancellation with count1/Committed |

Final106 occurred before last guarded-cleanup correction;postreview105 proves changed native
cleanup code across all other original/new cases. Do not label this as a second106run or second
ZIP64 execution. Initial final106 attempt failed external oracle launch due UTF8-no-BOM scratch
PowerShell decoding a Chinese path: selected106/run34/pass32/skip1/fail1/not_run72,exit1,
ZIP64RUN0. Preserved environment-failure log;ASCII glob with exactSHA256 verified oracle fixed
harness environment,not product code. Corrected same106run above has zero skips.

Oracles reverified: Python3.13.2 SHA256
`7d96a4ed35d6e596cd9dd8933feafc66349cc21f75be3b15c89fd336e50140c1`;
7-Zip26.00 x64 SHA256 `4bcc005758c5e604c0aaa257b6097846a4a708e62a9c0c7471fa360fad863f1f`.
CanonicalRAR3list+3test+3extract all0;exact payload/UTC/readonly+hidden orUnix absence checked.
Source leases,old target/no-replace,source namespace/output preparation,typed-I/O verifier and
original cleanup/terminal suites are retained in the exact original89selection.

## Limits and stop

Known native FILETIME0 refusal is explicit and tested,not silently cancelled functionality.
No genuine in-scope blocker remains for delivered slice. No broad FAT/SMB/UNC/long-path/DST
transition/large-count/RSS/performance/native-user acceptance is claimed. Full Phase A exits,
source-delete eligibility/guarded recycle,product integration/capability/package mapping/FIFO/
conflicts and later canonical phases remain unfinished and were not pulled forward.
Final gates actually passed: spec103documents/14owners,catalog79pages/79rows/78locked,
task context40implement/37check,PlatformHost fixture selector source gate,all exit0. Final
global/scoped git diff--check exit0;no conflict/temporary DEBUG-METADATA markers in scoped
sources. Reference32369bytes/checkpoint31495bytes before this final short record,both below
32KiB;source detail moved with links,not discarded. Final observed Cargo/rustc/worker/test-host/
7z0,fixed helper0,inspected temp fixture roots0. Removed exact owned native probe and generated/
extractedRARfiles;kept scratch logs/results. A transient other build observed during hygiene
exited on its own;never killed it or launched a competing Cargo. No unrelated dirty cleanup.
