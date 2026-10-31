# Archive module structure

Evidence: `repo:rust/crates/nxtools-archive/src/lib.rs`, `repo:rust/crates/nxtools-archive/src/model.rs`, `repo:rust/crates/nxtools-archive/src/formats/zip.rs`.

The public seam is ArchiveEngine.open/execute/extract with immutable entries/revision and an ArchiveOutput capability. model/engine own identity/dispatch; formats/zip shares one CRC/StreamEnd/budget path for test and extraction; encoding/path_policy own provenance/collisions; planning/interaction own immutable output rules and bounded generation-scoped prompts; control owns cancel/progress. worker/worker_output provide private acknowledged frames; worker_codecs and its narrow UnRAR FFI exist only behind isolated-codecs, never selected by desktop ArchiveEngine. Tests use public production APIs and independent fixtures.

Rust owns format policy/state, GPUI rendering, C# Windows process/Job/output handles/reparse/commit/Shell. ZIP and isolated RAR4/5+7z extract only through ArchiveOutput. RAR validates names and second-pass headers; RAR_TEST callback borrows<=64KiB chunks, no native destination.7z retains first error; both stage through full integrity/source check. C# owns confirmed target snapshots and approved rechecked atomic-path overwrite(accepted TOCTOU,no strictID-CAS);committed/uncertain states prevent partial deletion. Bounded ZIP/7z/TAR/TGZ/TXZ and single-source Gzip/XZ creation exist;edit/volume/UI/full resources remain incomplete. Clipboard/extension intake policies stay separate.

Continuation: formats/gzip owns bounded standalone RFC1952; formats/tar owns fixed-header/plain
TAR/PAX/GNU metadata and capability extraction; formats/zip_bzip2 adds explicit-ended ZIP12
payload only. formats/xz owns structural/allocation admission and public list/test/extract;
formats/tgz owns full-integrity Gzip/BZip2/XZ disk spools before TAR metadata/native output,
keeping original compressed-source revision. worker_create owns verified single-source Gzip/XZ
creation only;worker_zip_create adds bounded tree ZIP;subsequent tree codecs are described below.
Native partial_read_v1 is create-only, bounded and private; see [quality](quality-guidelines.md)
and [native creation owner](../../platformhost/archive-creation.md).

formats/zstd owns preventive standard/skippable-frame/window/block admission and explicitly
ended zstd-safe streaming, unknown standalone size and token extraction. tgz also owns Zstd
compound spool; original source remains separate. No external dictionary or Zstd create API.

creation owns one public bounded source/plan seam; worker_zip_create composes it with the same
private rendezvous/native output for ZIP Store/Deflate/AES256. Source identities/Windows handles
remain C# ArchiveCreationSources. Existing ZIP original-record preflight/stream_entry are reused
crate-internally for complete output readback; reader behavior is unchanged. Native source limit
65536observed records/32roots is not million-entry acceptance; no generic UI create advertisement.

Continuation: worker_sources extracts only the private ID-source adapter from worker_zip_create;
its lazy Reader supplies sequential protected reads/EOF/hash/close for tree codecs.
worker_sevenz_create owns single-thread public LZMA2/AES encoding plus full held-partial7z
verification; worker_tar_create owns explicit bounded TAR headers and optional Gzip/XZ outer
encoder plus shared strict TAR verification. worker_tree_io owns finite anonymous encoded spool
and full encoded-hash/length held-partial readback. No ordinary source/final-path authority,
dependency source patch,whole archive Vec or plaintext-source encryption spool is added.
PlannedSource.size means archive payload0for directories; original SourceEntry/native size stays
available for actual source-version checks. See native creation owner for exact wire/contracts.
