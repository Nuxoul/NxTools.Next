# 7z metadata Open workspace admission

## Scope and owning boundary

Installed feedback correction within the existing archive phases; no phase is renamed or advanced. The supplied `D:\Temp\SarasaUi-TTF-1.0.43.7z` is 162,676,868 bytes, 60 files, no folders, 1,476,080,380 declared expanded bytes, largest file 25,717,924 bytes. Independent Bandizip 7.32.0.1 and 7-Zip 26.00 read-only listings report solid LZMA2:192m, ten blocks, no encryption. The encoded-header descriptor is 37 bytes, its dictionary 6,144 bytes, total headers 1,165 bytes.

The original worker Open loop fails with ResourceLimit and no entries at 64 MiB and 195 MiB; 196/256 MiB list all 60 entries without payload decode/output. `worker_codecs.rs::run_sevenz` admits the encoded header, constructs the catalog reader, then unconditionally charges payload blocks. `sevenz_budget::blocks` charges the 192 MiB dictionary plus 4 MiB LZMA2 state. This is a typed admission refusal, not observed exhaustion or a native failure.

## Verified reference and lifecycle

Reference evidence: `docs/reference-code/archive.md`, `bandizip-solid-output.md` lines 40–70 and retained `evidence/raw/ark-7z-code-0x18004c7a0.json` (Evidence ID `ev_c0506115b5d4668a581a31aaa831fccacf80a8d55a3aa6784ac68193c6bd6cb6`). Bandizip's constructor `0x18004c2f0` creates the retained handler through `0x1800ef080`; Open `0x18004c7a0` invokes `0x1800ebdd0` and catalogs through `0x180134660 → 0x180138050`. Later extraction `0x18004f0b0 → 0x18004e170 → 0x180139760` groups selected files by block and decodes each required prefix. These are shipped-binary/Ghidra reconstructions, not proprietary original source.

Locked sevenz-rust2 0.23.0 `reader.rs::ArchiveReader::new` calls `Archive::read` and builds an index; payload decoding begins at `for_each_entries`/`BlockDecoder`. Next retains this reader only inside `run_sevenz`; normalized readers are rebuilt from the admitted archive, and retained drag receives that same reader in `sevenz_drag::execute`. Fresh selected reads re-enter `run_sevenz`. Create/Edit verification has separate callers of the existing full `blocks` admission and is unchanged.

Original product owner remains ArchiveReadSession → ArchiveWorkerRunner → suspended Job admission, original pipes/source lease, and actual process/pump cleanup. No new window, layout, placement, focus, native API, timeout, cancellation or terminal owner is introduced. ResourceLimit remains failure; no entries are silently truncated. Rust owns admission, C# native Job/source/output, GPUI rendering. Existing correlated accepted/start/terminal/elapsed/native diagnostics remain; no new operation or payload/frame logging is needed.

## Repair contract

- Always preflight encoded-header metadata, coder properties and dictionary workspace before constructor/header decode.
- Metadata-only Open validates payload coder support/properties without reserving payload dictionaries that it never allocates.
- Test, extraction, selected reads and retained drag must pass the unchanged full payload-block working admission before decoder preparation or output. Retained drag is admitted before its reader/layout handoff, so metadata-only admission cannot enter that cache path.
- Preserve source/metadata/entry/output limits, native Job ceilings, single-thread decode, integrity/source checks and cleanup. No global budget increase or new resource option.
- Keep optional edit qualification separate: a read-only Open can succeed while modification eligibility refuses the structure under the original full edit limits.

## Regression and verification boundary

`rust/crates/nxtools-archive/tests/read_work_contract.rs::sevenz_metadata_workspace` has four worker regressions: full metadata at 64 MiB and zero output work; Test/extract/selected/retained refusal before decode/output; encoded-header dictionary refusal during Open; invalid/unsupported payload coder refusal. `worker_codecs/sevenz_budget.rs::tests` adds a source that rejects every read overlapping the packed payload, plus the unchanged 64/195 MiB refusal and 196 MiB admission threshold and coder-validation assertions.

The three independent fixtures are 134 bytes each. `tests/fixtures/sevenz-workspace.py` uses CPython/liblzma, not the product writer; the valid solid archive contains `a.txt` = `alpha\n` (6 bytes), `b.txt` = `beta\n` (5 bytes), 192 MiB declared payload dictionary and 4 KiB encoded-header dictionary. The negative fixtures change only the payload declaration to property 41 or unknown method 0x7f. Stock 7-Zip 26.00 `l -slt -bd -mmt=1` reports both files/LZMA2:192m/one solid block; `t -bd -mmt=1` passes two files/11 bytes. No user-archive extraction was performed.

### Parent verification handoff

All commands run from the workspace root. Parent owns Cargo, worker builds and combined verification. Before each Rust command:

```powershell
$env:CARGO_BUILD_JOBS='1'
$env:CARGO_INCREMENTAL='0'
$env:RUSTFLAGS='-C codegen-units=256'
```

Focused compiled regressions (not run here):

```powershell
cargo test --locked -p nxtools-archive --features isolated-codecs --test read_work_contract sevenz_ -- --test-threads=1
cargo test --locked -p nxtools-archive --features isolated-codecs --lib worker_codecs::sevenz_budget::tests -- --test-threads=1
```

Controlled standalone worker build recipe (not run here):

```powershell
cargo build --locked -p nxtools-archive --features isolated-codecs --bin nxtools-archive-worker --target x86_64-pc-windows-msvc --target-dir target
```

That recipe produces `D:\Code\NxTools.Next-main\target\x86_64-pc-windows-msvc\debug\nxtools-archive-worker.exe`. It does not package or overwrite the installed helper. Preserve the parent's existing serial Cargo lane and use its matching feature-complete worker for later fixtures.

The inspected `$env:PI_SCRATCH_DIR\sarasa-worker-open.py` is Open-only, has a 30-second bounded loop, only ACKs `source_acquire`, rejects every other native call, and asserts 60 entries/1,476,080,380 declared bytes/zero payload-counter and output-write bytes under `--expect-browse`. Its line 3 hardcodes `artifacts\win-x64\NxTools.ArchiveWorker.exe`; retarget a scratch copy to the new compiled worker before `python <retargeted-script> --expect-browse`, without replacing package files. The original red evidence is `$env:PI_SCRATCH_DIR\sarasa-open-red.txt`; other retained diagnosis files are `sarasa-{7z-list.txt,bandizip-list.txt,metadata-summary.json,worker-64.json,worker-195.json,worker-196.json,worker-256.json}`. Its native ACK is a diagnostic surrogate, not evidence of the C# lease/Job boundary.

Native fixture expectation review: `csharp/NxTools.PlatformHost.Tests/ArchiveRemediationTests.cs::SevenzHeaderClassification` lines 195–226 tests payload `dictionary`, `encoded_dictionary` and `encoded_metadata` using operation **test** (line 222), so all ResourceLimit expectations remain valid. The direct one-byte-work guard at lines 126–138 remains below the unchanged 64 MiB codec minimum. No C# change is required for these cases. Relevant exact harness selectors after the parent's effects review/listing are `archive remediation actual 7z header classification fixture`, `archive codec actual Rust RAR4 RAR5 and 7z fixture`, `archive codec fixture solid packed optional metadata`, `archive codec actual Rust password budget corruption cancel fixture`, and `archive extraction actual Rust 7z handle-session fixture`. They require the matching `NXTOOLS_ARCHIVE_WORKER_FIXTURE` and `NXTOOLS_ARCHIVE_CODEC_FIXTURES`; TEMP/TMP stay in session scratch. Native fixtures copy a temporary helper into their own harness output and must refuse an existing helper.

### Verification and limitations

- Passed: independent fixture generation; stock 7-Zip 26.00 list and integrity test on the tiny valid fixture; scoped rustfmt parsing/format check and `git diff --check`.
- Not run: Cargo tests/check/Clippy, new worker build, repaired actual-user-archive Open, C# fixtures, shared document/combined gates. These remain parent-owned; no compiled/native/product acceptance is claimed.
- The ordinary worker's `sevenz_block_decodes` field is not an independent decoder oracle: its increment is in retained drag, not the ordinary `for_each_entries` loop. The helper regression therefore forbids actual packed-payload reads as well as checking worker work/output observations.
- Actual source/metadata and native Job/output ceilings remain in force; a metadata-admitted archive can still correctly return ResourceLimit when a later payload operation requires more workspace.
