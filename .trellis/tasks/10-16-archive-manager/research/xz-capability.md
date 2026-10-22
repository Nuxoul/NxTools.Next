# Exact XZ adapter capability check

Required Phase A work, not a dependency upgrade/scope decision. Inspected locked lzma-rust2=0.21.0 (Apache-2.0,pure Rust,no build.rs,Rust1.85), src/xz.rs and xz/{reader,writer}.rs,enc/lzma2_writer.rs. Add direct exact dependency with std/xz/encoder, no lzip/optimization or multi-thread API required. Existing isolated-codecs already builds this release on Windows; direct XZ features still need actual build/tests.

Public XzReader::new_mem_limit(inner,multiple,mem_limit_kb) checks block dictionary+overhead before allocation. But Index::parse try_reserve_exact(number_of_records), so allocation preflight is mandatory. Ordinary reader also doesn't compare footer backward_size to actual index size and doesn't compare every index record to observed block lengths. Project must independently validate bounded footer/header/index CRC/flags, canonical variable integers, record count and bounded index allocation, block extents, block header CRC/filter properties, LZMA2 encoded chunk boundaries/end and compressed/uncompressed counts. No third-party patch or error-text matching. OutOfMemory→ResourceLimit,Unsupported→UnsupportedMethod,structural invalid/EOF→CorruptArchive; real IO stays IoFailure. Full decoder checks payload CRC. Per-stream trailing padding must be multiple4; concatenated members bounded.

XzStream sans-I/O verifies record lengths but retains per-block records with no configurable count and supports only one prefilter; ordinary reader plus independent preventive structural preflight retains full supported filter chains. Decoder uses one thread and fixed IO chunks; cancellation reader wraps reads. No thread timeout as allocation proof.

XzOptions::with_preset, lzma_options.dict_size and XzWriter::new/finish are public. Semantic Fast→preset1,Normal→6,Maximum→7 (not universal9); dictionary1/8/16MiB. Exact get_memory_usage() implementation mixes get_buf_size byte terms with Hc4/Bt4 KiB terms despite KiB documentation; neither multiplying the total by1024 nor treating total as bytes is a reliable preventive estimate. Project reserves conservative16×dictionary+32MiB based on one-thread buffer+hash+chain/tree allocations, before construction, and retains C#Job cap. Fast previously red ResourceLimit from mistaken total×1024; corrected reserve must pass actual three-level fixture. Full held-partial read-back preflight/decode+SHA256 vs source and native lease/publication remain required. No whole output Vec, passwords/source deletion.

Independent CPython lzma.compress single/multi/TAR.XZ fixtures drive engine red UnsupportedFormat (1run/0pass/1fail,exit101), then actual worker create red admission fast (1run/0pass/1fail,exit1), both before implementation. Independent Python decoder supplies created-output byte/hash oracle, not NxTools roundtrip alone. Actual method corruption/index/dictionary/cancel/native tests required before capability claim.

Historical previous-turn boundary: engine.rs reached the per-turn3failed-edit stop, public read
remained UnsupportedFormat and Rust45pass/1fail was accurately retained. That stop is not a
permanent file lock. This turn fresh grounded reads restored list/extract, compound spool and
engine open/test/extract; unchanged original red is now green. See
[public-read evidence](../xz-read-evidence.md) and the single current checkpoint for actual results.

Additional exact upstream observation: src/lib.rs::error_other emits io::ErrorKind::Other;
range_dec.rs uses it for invalid decoder state/data. Do not match strings or classify all source
I/O as corruption. Project Controlled reader tags source I/O/OutOfMemory, retains already typed
ArchiveError and cancellation, so untagged codec Other is CorruptArchive. No third-party edit.
Independent single/multi/TAR.XZ public/native tests include footer/index/payload, legal stream
padding, work/metadata/actual-output/spool budgets, source protection/cancel/partial cleanup.
