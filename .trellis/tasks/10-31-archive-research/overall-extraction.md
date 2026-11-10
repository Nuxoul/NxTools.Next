# Shared extraction efficiency

## Scope

This note covers overall extraction efficiency within the [whole-product reference](bandizip-product-reference.md). Browser open, ordinary Extract, entry materialization and drag are consumers of the shared engine/native-output pipeline. Optimize that common work before changing consumer-specific gesture or presentation mechanisms.

The Bandizip reference establishes an indexed selection plan passed once into its engine and a measured exact-member extraction with a small process read-transfer count. [Selected-engine evidence](bandizip-performance-picker.md) retains those observations. Native drag-specific transport is not the basis for the shared-engine optimization.

The [shared ZIP pipeline trace](bandizip-core-pipeline.md) reaches input-handle/buffer reuse, EOCD/central-directory loading, selected-member streaming CRC and native output. The [solid-format/output trace](bandizip-solid-output.md) connects normal RAR/RAR5 solid dispatch, ordered predecessor decoding without output creation,7z block grouping and selected-prefix decoding, coder reuse/reset, and buffered consumer/drain/close. Together they distinguish independent members, solid dependency chains and block-level work rather than treating every selected file as an independent job.

## Read-only core measurement

A scratch Rust probe links the existing workspace Debug `nxtools-archive` rlib and invokes its real public `ArchiveEngine::open` and `modification_eligibility`. It reads the original supplied ZIP, writes only a measurement report, extracts no payload, and uses no Desktop/native bridge. Product sources, budgets and validation logic are unchanged.

| Phase | Measured |
| --- | ---: |
| One sequential read of137,737,166bytes |280.2250ms |
| `ArchiveEngine::open` returning215 members |9,203.1564ms |
| Separate explicit ZIP modification eligibility |4,555.7176ms;`Ok(())` |
| Entire probe wall time |14,073.8211ms |
| Entire probe user/kernel CPU time |3,562.5ms /15.625ms |
| Entire probe process read-transfer bytes |965,675,946 |

The process counter covers all three phases, not only open. It is not per-file attribution or a mapped-page measurement. No payload extraction or output IPC occurred in these timings.

## Verified source work

`ArchiveEngine::open` first calls `open_impl`, then automatically asks `modification_eligibility` for ZIP capabilities. The ordinary ZIP `open_impl` computes a whole-source revision before metadata listing and recomputes it afterward. ZIP modification eligibility independently reopens the source and recomputes the same revision before and after its structural inspection.

Thus an eligible ZIP open takes four whole-source revision passes. The probe's extra explicit eligibility call takes two more; together with its plain read, the inspected path accounts for approximately seven archive-sized reads, consistent with the observed process transfer count. `revision` reads every byte and updates SHA-256; these calls are not cheap metadata identity checks.

Evidence:
- `repo:rust/crates/nxtools-archive/src/engine.rs#ArchiveEngine::open` (lines66–85)
- `repo:rust/crates/nxtools-archive/src/engine.rs` (ZIP open lines214–225; revision at725)
- `repo:rust/crates/nxtools-archive/src/zip_edit.rs#eligibility` (lines78–119)

Extraction adds further revision work: `engine.rs` checks at extraction entry and return; `formats/zip.rs` checks after structural preflight and after each selected file's decode before commit. The per-file check makes work scale with archive size times selected-file count, not only selected payload size.

The actual9.20second open measurement proves substantial cost before decode or native output. The4.56second explicit eligibility measurement establishes the extra preflight's runtime cost in this Debug environment. These values do not isolate each internal hash or establish the complete native task's remaining decode, IPC and commit durations.

## Optimization order

1. **Consolidate open/admission work.** Reuse the validated source/snapshot within one attempt instead of attaching independent whole-source revision passes to metadata and capability discovery. Preserve standalone/unleased caller source-change semantics and the actual native held-source authority.
2. **Remove archive-sized work from the member loop where the original source lease proves stability.** Retain ordinal/browser/source binding, structural checks, payload CRC, native commit/result validation and terminal cleanup. Do not accept an arbitrary caller's immutable-source claim or skip checks merely to improve a benchmark.
3. **Measure and optimize bulk output.** The existing64-KiB synchronous JSON/native reply path is a separate shared cost. Compare decode, transport, staging/commit and final validation before choosing a representation change. Ordinary Extract and entry/drag consumers must benefit through their original owners.

The first change should address the verified common open/revision work, not a drag-only fast path, a new extractor, a global timeout increase, or speculative codec concurrency.

## Local evidence and remaining measurements

Stable local archive: [evidence README](evidence/README.md), [index](evidence/index.json).

- `evidence/raw/archive_core_timing_probe.rs`
- `evidence/raw/core-open-timing.json`
- `evidence/raw/core-open-process-cost.json`

These are existing Debug-library measurements, not release-optimized acceptance or a fair end-to-end speed ratio against Bandizip's optimized build. A comparable build-mode baseline and real native phase durations remain required before claiming total performance improvement. The earlier approximately27second selected native extraction is still not repaired by this research.
