# Independent acceptance — XZ / TAR.XZ and Zstd / TAR.ZST reads

## Verdict

**PASS for the delivered bounded read slices on the tested source/fixture/native environment.** The previously retained public XZ UnsupportedFormat red is resolved. No new concrete blocking defect was confirmed in this acceptance. **Phase A and first-release A–F are NOT complete/accepted.** No product code changed during this review.

Acceptance is scoped to actual delivered capabilities and bounded cases, not every possible filter/method/filesystem/resource size. External Zstd dictionaries remain explicitly unsupported. Sources without format checksum do not gain a fabricated payload-integrity guarantee. Existing approved rechecked atomic-path overwrite/accepted TOCTOU is unchanged.

## Independently executed checks

All Cargo commands inherited CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, RUSTFLAGS=-C codegen-units=256 in their executing environment, serially; no timeout/duplicate builds.

| Check | Actual independent result |
| --- | --- |
| cargo test -p nxtools-archive --features isolated-codecs --offline --no-fail-fast |57passed/0failed/0ignored,exit0; full log counts independently summed |
| Original public XZ single/multi/TAR test |PASS within stream_contract11/11; not removed or skipped |
| xz_read_contract |5/5PASS |
| zstd_read_contract / zstd_safety_contract |1/1 +4/4PASS |
| Same-feature actual worker build |exit0 |
| C# test-host build --no-restore |exit0,0warnings/0errors |
| Exact original40+XZ3+Zstd1 C# union, explicit fixture/oracle environment |listed44/run0;selected44/run44/pass44/skip0/fail0/not_run0,exit0 |
| Strict archive Clippy including tests |exit0 |
| cargo fmt --all -- --check |exit0 |
| cargo check --workspace --tests --offline |exit0,not workspace tests |
| Extensions archive_import |7/7PASS |
| Clipboard --lib archive_ |5/5PASS,39filtered |
| spec-contracts / component catalog / task context / fixture selector |PASS101docs/14owners;79pages/79rows/78locked;24implement/21check,no truncation;source selectorPASS |
| Additional independently generated Python XZ cases |11/11 actual worker testPASS; exact verified byte count2096 each |

C# union reused the current checkpoint's exact15 substring filters and explicit NXTOOLS_ARCHIVE_WORKER_FIXTURE/CODEC_FIXTURES/ZIP_FIXTURES/PYTHON_ORACLE. These44 are harness cases, not44 archive operations; internal98XZ/50Zstd counts come from test loops, not an additional execution tally.

No unrestricted/default C# suite, full workspace tests, UI/native user Shell/DPI/UIA, FAT/SMB/UNC/old Windows, Release/install/service/registry, crash/power-loss or large-memory stress was run. No leftover Cargo/rustc/archive worker/test-host or fixed helper observed after tests and extra probe.

## Source and contract inspection

- `engine.rs`: XZ signature recognition, Xz/TarXz snapshots/capabilities, standalone unknown-size admission and actual test/extract dispatch all present. Zstd standard/skippable signature and standalone/compound dispatch present. Reads reuse ArchiveEngine in the worker, not a duplicate codec dispatcher.
- `formats/xz.rs`: list preserves size=None, preventive footer/index/block/dictionary bounds, explicit stream/padding validation, public memory-limited decoder; controlled I/O preserves typed failure provenance. Extract stages one owned token and commits only after successful full decode, original source revision recheck and cancellation checkpoint.
- `formats/tgz.rs`: case-insensitive .tar.xz/.txz and .tar.zst/.tar.zstd/.tzst; bounded anonymous disk spool completes outer decoding before TAR list/output; spool uses total budget, per-TAR-entry limits remain separate; extraction rechecks original compressed source.
- `formats/zstd.rs`: preventive frame/window/block/count/metadata bounds, set WindowLogMax before decompress_stream; explicit complete-frame/end-of-input and actual-byte checks, supported checksum handling and fixed code classification; no whole-frame buffer.
- Actual native fixtures cover open/test/extract, independent bytes/counts, damage/budget failure, owned partial cleanup, source lease and commit cancellation while retaining prior target protections. Prior output/outcome/diagnostic regressions are included in44PASS.

Additional scratch-only Python probe independently compressed+decompressed eleven2096-byte inputs then sent bounded test requests to the actual worker: XZ Check None/CRC32/CRC64/SHA256; Delta(distance3); x86/ARM/ARMThumb/PowerPC/SPARC/IA64 with explicit start offset. All returned complete with exact expected verified length. These small compatibility checks are not exhaustive filter interoperability or native output proof (native output is covered separately by fixtures). Generated sources were removed by the probe's TemporaryDirectory.

## One directed review and evidence filtering

One independent read-only Zstd review proposed two concerns. Neither was accepted as a new blocking finding:
1. Unused descriptor bit4 rejection: inspected exact bundled Zstandard1.5.7 `zstd_decompress.c:511` also rejects reserved bit3 (mask0x08), not bit4. Decoder compatibility requires distinguishing unused vs reserved. No confirmed malformed-input acceptance/security defect was established merely from ignoring unused bit4; do not add a stricter reject based only on that suggestion.
2. Source modified to corrupt an unread block may fail decode before the post-success SourceChanged check: this yields failure with no commit, and real production workers retain the approved source lease. Existing source-change tests target successful-decode/precommit revision detection. This review did not reproduce a lease bypass or invalid publication, so does not invent a new source-safety blocker or demand all decoder failures be reclassified SourceChanged.

## Remaining work and next action

Close the old public XZ dispatch/edit-stop item. Current delivered XZ/Zstd reads can be the green baseline for the next approved Phase A slice. Continue remaining ZIP methods/encryption, general creation/source traversal, native attributes/MOTW/time/guarded source deletion, product IPC/capability/package mapping/backpressure/diagnostics and broad resource/performance gates. No user decision or technical blocker was confirmed for these delivered read slices; remaining features are incomplete scope, not proof of failure or permission to cancel them.

Independent scratch artifacts: `xz-zstd-acceptance-{archive,clippy,fmt,workspace,extensions,clipboard}.log`, `xz_read_acceptance_probe.py`. Test-host/worker build and44fixture results were captured in tool output. No source edits, packaging, installation, user archive mutation, commit or push performed.
