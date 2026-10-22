# Phase A small interoperability closure — C1/C2/C3/C5

2026-10-20. **Original production implementation PASS; missing assertions/evidence added.**
No production repair/red→green is claimed. Scoped implementation checks PASS; independent acceptance
pending. Still **Phase A — Engine contracts and safety foundation**, NOT Phase A/A–F completion.
Current state only in [checkpoint](phase-a-checkpoint.md); original [exit ledger](phase-a-exit-ledger.md)
and all historical reports/results remain unchanged. Latest [ZIP preflight readmission independent
PASS](zip-output-readmission-acceptance.md) is retained, not rerun.

## Four exact closures

| Item / expected | Actual result / independent oracle |
| --- | --- |
| **C1 empty Gzip/XZ**: zero-byte ordinary source, representative normal level, native create then existing open/test/extract, empty bytes/counts/cleanup | `ArchiveStreamWorkerTests.EmptyGzipXzCreationInterop` PASS: 2create+6read/test/extract operations through original ForPackage runner/Job/actualworker/nativepublication. CPython3.13.2 stdlib gzip/lzma independently reads each complete stream and returns empty. Each read/test/extract reports files_verified1/bytes_verified0; open expanded size stays null (no inflate-to-size), extract publishes exactly one0byte file. Native create committed1, read-only committed0, extract committed1; actual exited/drain/cleanup and exclusive source reopen pass; no owned partial. |
| **C2 x86 BCJ+LZMA2**: actual independently encoded chain, test/extract exact bytes | `ArchiveCodecWorkerTests.X86BcjLzma2Interop` PASS: original runner/Job/actualworker test+extract report1file/4096bytes; single native x86.bin equals retained independent input, cleanup/source release pass. Existing stock7-Zip26.00 creates155byte archive, tests it and independently extracts exact4096bytes. Actual `-slt` archive Method=LZMA2:12 BCJ, member Method=BCJ LZMA2:12, one block, packed49; not inferred from name. |
| **C3 signature/ZIP aliases**: same independent ZIP .bin/.jar/.apk, typed random/corrupt/compound refusal | Original `engine_contract.independent_zip_aliases_use_signature_and_misleading_inputs_keep_typed_errors` PASS: unchanged CPython-generated stored.zip under3aliases -> Zip/4entries/test3files275bytes.7refusals: deterministic random.bin, damaged first ZIP signature and random.tar.gz/.tar.xz -> UnsupportedFormat; recognised PK signature with truncated structure, standalone Gzip disguised.tar.gz and XZ disguised.tar.xz -> CorruptArchive. Existing API only; no JAR/APK editing or signature-authentication authority. |
| **C5 solid optional metadata**: shared packed bytes stay unknown; non-solid only real member values | `ArchiveCodecWorkerTests.SolidPackedMetadata` PASS,2original runner/Job/worker opens of existing independent upstream fixtures. Both exact3entries; solid members3447/2654bytes each packed_size=null, never copied shared2240byte block. Non-solid member packed1446/1232 matches independent7-Zip26.00 `l -slt`; no extra decode to list. Original `worker_wire.optional_member_metadata_missing_or_null_remains_unknown_not_zero` PASS: absent/null packed and absent encrypted/provenance/codepage remain None; null roundtrip, no fabricated0. |

## Independent producer and reproducibility

Existing tool, no install/download/upgrade:
`D:\Tool\Directory Opus\Directory Opus\_Green\7z极限压缩\bin\x64\7z.exe`, version26.00x64
(2026-02-12), executable SHA256
`4bcc005758c5e604c0aaa257b6097846a4a708e62a9c0c7471fa360fad863f1f`.

`python rust/crates/nxtools-archive/tests/fixtures/generate-interop.py --sevenzip <above> --output
rust/crates/nxtools-archive/tests/fixtures/interop` uses stock7Zip, NOT a custom archive encoder.
Plain synthetic payload is 256 × 16-byte x86 call/jump patterns, not executed. Exact producer command:

```text
7z a -t7z -m0=BCJ -m1=LZMA2:d=64k -ms=off -mhc=off -mtc=off -mta=off -mtm=off x86-bcj-lzma2.7z x86.bin
7z l -slt -sccUTF-8 x86-bcj-lzma2.7z
7z t x86-bcj-lzma2.7z
7z x -y -ooracle x86-bcj-lzma2.7z
```

All exited0; independent extracted bytes match input.7Zip reduces dictionary for this tiny input;
actual12/4KiB proof, not requested16/64KiB, is recorded. First producer preparation assertion expected
64KiB text and failed; corrected to observed exact coder proof before native execution. This was
fixture-preparation only, not a product defect or product red. Retained
[manifest](../../../rust/crates/nxtools-archive/tests/fixtures/interop/manifest.json) and
[coder listing](../../../rust/crates/nxtools-archive/tests/fixtures/interop/producer-list.txt):
archive SHA256 `9524f23142ec77b4a2ce94779dd44f6be6cf7cbc06efae072a9d09dec3814d07`;
4096byte input SHA256 `b7b46b694042e3b4270ca0928fee293974fc1b9fcddf802e39f51da7cc0d91e5`.
C1 oracle command uses CPython `-I -c` stdlib gzip.open/lzma.open.read(1)==b'' (forces stream end for
these empty payloads), exit0 and exact `empty:0`. C5 `7z l -slt` existing solid.7z/non-solid.7z lists
shared block0 versus two blocks0/1; original fixture attribution/hashes stay in upstream README/manifest.

## Exact executed collection

C#Debug apphost built FIRST,0warnings/errors, then matchingisolatedworker after Rust tests/lint.
All Cargo inherited jobs1/incremental0/RUSTFLAGS=-C codegen-units=256, offline, one machine lane;
worker execution/relink serial. No broad suite/filter or100k/large/longpath matrix was executed.

| Command / selection | Actual result |
| --- | --- |
| `dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore` |exit0,0warnings/errors |
| `cargo test -p nxtools-archive --features isolated-codecs --offline --test engine_contract independent_zip_aliases_use_signature_and_misleading_inputs_keep_typed_errors -- --exact --test-threads=1` |1PASS,14filtered,exit0;100k test NOT run |
| Same Cargo flags, `--test worker_wire optional_member_metadata_missing_or_null_remains_unknown_not_zero -- --exact --test-threads=1` |1PASS,15filtered,exit0 |
| Same Cargo flags, `--test creation_worker_contract -- --test-threads=1` |1PASS,4normal/corrupt Gzip/XZ readback scenarios,exit0; existing coverage reused |
| Strict Clippy isolated/default `--tests --examples --offline -- -D warnings` |both exit0 |
| `cargo fmt -p nxtools-archive -- --check`; matchingisolatedworker build |both exit0 |
| C# `--fixtures --list` then same exact5filters `--fixtures` |selected5/run5/pass5/skip0/fail0/not_run0,exits0 |

C#exact names (LIST=RUN=PASS, same catalog order):
1. `archive stream fixture empty Gzip XZ creation interop`
2. `archive codec fixture x86 BCJ LZMA2 interop`
3. `archive codec fixture solid packed optional metadata`
4. `archive remediation actual 7z working budget fixture`
5. `archive remediation actual 7z header classification fixture`

Last2are EXISTING bounded regression, not duplicated BCJ matrices: work1byte test is explicit direct
private-worker admission (NOT original Job proof); header classification9small cases use original
runner/Job for malformed extents/truncation/metadata/dictionaries. Existing creation_readback normal/
damage regression proves no publication before full verification. Prior accepted codec/native/stream
integrity remains referenced; no reimplementation or full password/risk/readmission rerun.
Explicit NXTOOLS_ARCHIVE_WORKER_FIXTURE/ZIP_FIXTURES/CODEC_FIXTURES/PYTHON_ORACLE/7ZIP_ORACLE set.

## Source and preservation boundary

Read indexed archive reference first: Archiver core/src/lib.rs230–275 streaming encoder/decoder and
ParGzWriter.finish_box empty dispatch; SupaZip nested formats/mod.rs detect_format20–78 is extension-only,
not Next signature authority. Legacy tracked ZIP search still finds unrelated package ZIP only.
Next original ArchiveEngine.open signature→bounded parser and worker_codecs::run_sevenz metadata /
sevenz_budget BCJ handling remain unchanged. No UI/layout/window lifecycle or production API change.
Only original tests/registration + tiny producer/retained fixture and this evidence/checkpoint changed.

Scratch `C:\Users\Administrator\.pi-desktop\scratch\949da288-1ef9-46b3-a698-90f543613cf5` retains
interop-opening-hashes/status, interop-producer/7zip-version/solid/non-solid-list logs,
interop-rust-verify.ps1/results and per-step logs, interop-native-verify.ps1/list/run/results.
Final spec-contracts/task-context/selector/gitdiff--check, fixture hashes, prior-checkpoint snapshot,
historical-file equality, production unchanged and zero archiveprocess/helper/pipes are verified in
interop-closing-assertions.json. Existing native-contracts35497byte context warning is unchanged.

Stop for independent scoped acceptance. No PhaseB/UI/FIFO/other-format interaction/resume,100k,
1GiB/4GiB+17ZIP64/performance,new framework/dependency/system/sourceRecycle/delete,formalapp,
package/install/sign/commit/push. No PhaseA/A–F completion or broad review claim.
