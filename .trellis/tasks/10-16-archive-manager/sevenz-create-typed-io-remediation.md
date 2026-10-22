# 7z creation verifier — typed-I/O repair and executed evidence

Date: 2026-10-18. Canonical phase: **Phase A — Engine contracts and safety foundation**.
Scope: only the independent P2 finding in [acceptance](tree-create-acceptance-review.md).
That source-confirmed finding remains unchanged: no real failing-disk reproduction, unsafe
publication or data loss was established. This repair uses small synthetic injected failures,
not system faults. [Single current checkpoint](phase-a-checkpoint.md) owns current status.

## Root cause and bounded change

The signature read and decoded member callback discarded every read error as CorruptArchive.
The callback retained that projection, so later sevenz_error could not recover storage origin.
The creation path's encoded-header budget preflight could similarly discard a read error;
ArchiveReader construction and encrypted decoding may additionally wrap it.

`worker_sevenz_create.rs` now wraps only the owned verification spool in private
VerificationInput<R: Read + Seek>. It records the first underlying typed failure before any
7z parser/decoder wrapper, across signature, encoded-header admission, ArchiveReader construction,
seek and payload decoding. verify checks that retained failure after verify_inner on every return.
Raw storage failures (including Other and InvalidData) become IoFailure; actual EOF/truncation,
malformed data and integrity/hash/plan failures remain CorruptArchive. Existing ArchiveError is
preserved; OutOfMemory becomes ResourceLimit. Interrupted is left retryable, not latched as a
terminal failure. Untagged decoder I/O is not presumed to be storage failure. No error-text matching.

The production create entry passes its File unchanged through a private create_with_reader seam.
Tests adapt only that same verifier input after real encoding and complete held-partial readback
hash/length comparison. sevenz_budget::encoded_header now accepts Read+Seek and obtains length by
seek-to-end; its budget, CRC, extent and descriptor checks are unchanged. Its callers rewind before
ArchiveReader construction. Other codec/extraction error policies were not broadened or repaired.

Unchanged chain: bounded native-ID source plan -> finite anonymous encoded spool -> native partial
-> same held partial full readback/hash -> bounded complete structure/decode/exact plan/SHA256
-> source/cancel recheck -> existing C# flush/single atomic-path publication. No change to approved
overwrite/TOCTOU, source authority, native cleanup, wire, dependencies, upstream code or runtime.

## Real red -> green

Both invocations inherited CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0,
RUSTFLAGS=-C codegen-units=256 and checked for Cargo/rustc before starting:

```powershell
cargo test -p nxtools-archive --lib --features isolated-codecs --offline worker_sevenz_create::tests -- --test-threads=1
```

| Actual run | Result | Exit |
| --- | --- | --- |
| Red, 18:25:27 +08:00 | 7 selected/run: 2 passed / 5 failed / 0 ignored; 3 unrelated lib tests filtered | 101 |
| Green, 18:26:56 +08:00, same command | 7 selected/run: 7 passed / 0 failed / 0 ignored; 3 filtered | 0 |

Red failures were the intended category mismatch, not compilation: signature Other, budget-header
Other, parser-header InvalidData and payload Other returned CorruptArchive instead of IoFailure;
typed Cancelled at signature was also overwritten. Normal and corruption/truncation tests already
passed. The cleanup assertions run before category assertions and passed in the red cases too.
Logs: session scratch `typed-io-red.log`, `typed-io-green.log`, `typed-io-red-green-ledger.txt`.

Seven harness tests execute 31 small creation transactions in green (transactions are not extra
test counts): signature/payload storage faults with absent and preexisting targets; budget-header
and parser-header faults; signature corruption, truncation and damaged payload; normal publication;
15 typed Cancelled/ResourceLimit/SourceChanged cases across five read/seek points, five OOM cases,
and encrypted payload storage failure. The parser-header case uses AES/header encryption too.
Each injected fault asserts its boundary was actually reached.

All failure transactions assert zero commits, exactly one abort with the owned token, removed
owned partial, untouched foreign partial, source opened/closed exactly once, no active source or
partial handle, unchanged source bytes, and no final or unchanged preexisting target. The harness
uses real temporary source/partial/final files with the production PrivateOutput/ReplyPort protocol
and actual encoder/stage/readback/verifier/abort path. This is not a real hardware-fault test or a
substitute for C# protected Windows-handle acceptance; the retained C# integration below checks that
native boundary separately. Temporary directories/files are owned by tempfile RAII.

## One final integrated batch

Actual foreground batch: 18:34:32 -> 18:45:26 (+08:00), normal terminal BATCH_PASS, all steps exit0.
No abandoned/background invocation is counted. Every Cargo inherited the three memory settings
above; commands were serialized, checking the machine-wide Cargo/rustc lane before each step.
Exact commands, filters and count assertions are retained in session scratch `typed-io-verify.ps1`;
ledger `typed-io-final-ledger.txt`, complete logs `typed-io-final-*.log`.

| Command / boundary | This repair's observed result |
| --- | --- |
| cargo test -p nxtools-archive --features isolated-codecs --offline --no-fail-fast -- --test-threads=1 | 93 passed / 0 failed / 0 ignored across 20 result groups; exit0 |
| cargo build -p nxtools-archive --bin nxtools-archive-worker --features isolated-codecs --offline | exit0 |
| cargo clippy -p nxtools-archive --features isolated-codecs --tests --offline -- -D warnings | exit0 |
| cargo clippy -p nxtools-archive --tests --offline -- -D warnings | exit0 |
| cargo fmt --all -- --check | exit0 |
| cargo check --workspace --tests --offline | exit0; NOT workspace tests |
| cargo test -p nxtools-extensions --test archive_import --offline | 7 passed / 0 failed / 0 ignored; exit0 |
| cargo test -p nxtools-clipboard --lib archive_ --offline | 5 passed / 0 failed / 0 ignored, 39 filtered; exit0 |
| dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore | product and tests: 0 warnings / 0 errors; exit0 |
| Original exact 22-filter checkpoint union, --fixtures --list | selected89 / run0 / pass0 / skip0 / fail0 / not_run89; exit0 |
| Same exact union, --fixtures | selected89 / run89 / pass89 / skip0 / fail0 / not_run0; exit0 |
| Actual 4GiB+17 FastDeflate ZIP64 | RUN1 / PASS1, once only in final89, actual Job/native publish + Python streaming size/SHA256 |
| Equivalent real missing-internal-output regression | OutputCollision, commits0, CleanupComplete=True, output_created=False; exact source names/hash unchanged |

No C# fixtures were added/removed. The script compared all 89 LIST names against the original
checkpoint filter union/catalog, then the RUN names against that exact list. It additionally
asserted the large ZIP64 RUN and PASS each occurred once. The missing-internal-output fixture
(ArchiveSourceBindingTests.MissingInternalOutput) printed identical before/after empty directory
and keep.txt SHA256, asserted no new-output ancestor/final/partial and original source content.
This is the requested equivalent real regression, not a claim to have rerun the unavailable prior
scratch probe. Original multi/held-identity/missing-external/preparation regressions also remain
in the same89. Prior acceptance86/89 results are historical, not counted as this repair's runs.

### Independent tools and resource preflight

Explicit existing tools were rechecked this run, never downloaded/installed or added to PATH:
- 7-Zip26.00: `D:\Tool\Directory Opus\Directory Opus\_Green\7z极限压缩\bin\x64\7z.exe`,
  SHA256 `4bcc005758c5e604c0aaa257b6097846a4a708e62a9c0c7471fa360fad863f1f`.
- Its existing 7z.dll SHA256 `bbd705e3b58ca7677c1e9e67473f166a6712da034dcb567d571fbb67507a443f`.
- Python3.13.2: `C:\Users\Administrator\AppData\Local\Programs\Python\Python313\python.exe`,
  SHA256 `7d96a4ed35d6e596cd9dd8933feafc66349cc21f75be3b15c89fd336e50140c1`.

Independent exact manifest/kind/size/hash tests and created-output open/test/extract in original89
all actually ran. Passwords stayed on private stdin. Final preflight: free physical16606616KiB,
free commit16687012KiB, temp disk32188243968bytes. This is bounded verification, not RSS/performance
or general memory-pressure acceptance.

## Review, cleanup and limits

The one optional bounded read-only reviewer terminated without a report; it is NOT a passed
review and was not restarted. Parent source inspection and executable gates above completed.
Post-integration inspection found zero Cargo/rustc/archive-worker/test-host/7z processes, zero
fixed NxTools.ArchiveWorker.exe test helper copies and zero inspected archive fixture roots.
Owned partial/source/final/foreign-content assertions passed. No temporary product DEBUG probes
were introduced; only scratch scripts/logs/baselines remain outside the project.

Inspected references this repair: recorded legacy NxTools tracked archive-term search (only
unrelated package/FFmpeg ZIP/image/video owners, not an exhaustive absence claim); Archiver
core/src/lib.rs987-1075 create dispatcher; SupaZip nested formats/sevenz.rs463-535; exact locked
sevenz-rust2 0.23.0 reader/error API and existing formats/xz.rs provenance pattern. No Bandizip
proprietary source or new reference interaction/native lifecycle acceptance is claimed.

Preserved unrelated dirty changes. No package/install/commit/push, user archives, real registry,
associations, services or drive mappings were operated. No Phase B/UI work. Remaining Phase A:
timestamps/attributes/MOTW; guarded recycle/source-delete eligibility; product IPC/capabilities,
helper package mapping/FIFO/backpressure/conflicts; broad resources/performance/large-count/
long-path/UNC/native acceptance. Full workspace tests and unrestricted C# tests were not run.
No Phase A/A-F/native-user/UI/release completion is claimed.

## Final document and hygiene gates

All four requested gates executed after evidence/context edits, each exit0:
- python tests/architecture/spec-contracts.py: PASS101documents/14owners.
- python tests/architecture/gpui-component-catalog.py: PASS79pages/79rows/78locked entries.
- python .trellis/scripts/task.py validate .trellis/tasks/10-16-archive-manager:
  PASS36implement/33check entries, no truncation warnings.
- tests/architecture/platformhost-test-selection.ps1: PASS,source only.

Logs: typed-io-final-{spec,catalog,context,selector}.log in the session scratch.
git diff --check separately returned exit0; unrelated existing CRLF normalization warnings
are retained in typed-io-diff-check.log and are not whitespace failures. The first combined
postbatch reporting command returned nonzero without a product/test failure; a separate
literal-count/diff invocation confirmed93/0/0and diff exit0. No tests or ZIP64 were rerun.
Scoped12file conflict/whitespace/DEBUG and final process/helper/fixture cleanup results are
recorded in typed-io-final-hygiene.log. An initial dirty-path comparison falsely differed on
Git quoted versus unquoted nonASCII display; matching quotePath settings resolved that
scratch-check issue without touching the unrelated plan file.
