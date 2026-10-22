# Tree creation continuation — exact API investigation

Date: 2026-10-18. Phase A — Engine contracts and safety foundation. ZIP preparation repair
is saved green first in ../zip-create-output-preparation.md (82Rust/80C#,actual large ZIP64once).
This note records approved7z/TAR/TGZ/TXZ continuation APIs; executed evidence is in ../tree-create-evidence.md.

## References actually re-read

- Legacy D:/Code/NxTools tracked archive-term search still identifies unrelated package/FFmpeg
  ZIP/image/video owners only. No new general archive interaction/native lifecycle identified;
  not exhaustive generated/untracked inspection.
- D:/Code/Archiver-main/core/src/lib.rs987–1075: collect -> create dispatch.7z998 uses ordinary
  ArchiveWriter::create(output),AES options+header encryption1000–1002,LZMA2single or MT1004–1012,
  solid push_archive_entries1030,finish1034. TAR1050–1065 uses follow_symlinks(false),append_dir
  and append_data from ordinary files then compressor finish. Reuse format API knowledge only,
  not path authority,parallel source handling or overwrite behavior.
- D:/Code/supazip-master/supazip/supazip-core/src/formats/sevenz.rs463–535: old sevenz-rust0.6.1
  writer,new path metadata/file opens,per-entry nonsolid push,finish;does not supply Next safety.
  formats/tar.rs85–135 similarly ordinary path metadata/append_dir/append_file and shallow
  source list. Both are unpacked source directories,not Git repos;git grep failed before direct
  file reads. No proprietary Bandizip decompilation/backend call.

## Locked public backend facts

Actual registry C:/Users/Administrator/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/:
- sevenz-rust2-0.23.0/src/writer.rs113–143: ArchiveWriter::new<W:Write+Seek> seeks to32;
  set_content_methods(Vec<EncoderConfiguration>);set_encrypt_header(bool),default true,so set
  explicitly from authorized options.166push_archive_entry<R:Read> nonsolid;282push_archive_entries
  (Vec<ArchiveEntry>,Vec<SourceReader<R>>) solid,asserts equal lengths. SeqReader advances one
  reader at EOF; wrappers can lazily open/close exactly one real native source handle at a time.
- writer.rs391–419finish consumes writer,builds metadata Vec,writes encoded header,rewrites
  signature at offset0 and flushes. Not sequential-only. Do not use auto_finish,File::create,
  prepare_block/compressed Vec or uncontrolled MT. Bound seek spool length and metadata BEFORE
  backend construction; preserve explicit finalization/native error/cancel results.
- archive.rs88–160 public ArchiveEntry::new_file/new_directory and metadata fields permit
  construction from native plan without from_path/File metadata. writer/source_reader.rs
  records count+CRC at EOF. worker/readback must compare original plan names/kinds/sizes/hashes,
  not merely trust backend finish or CRC.
- encoder_options.rs56–136:Lzma2Options::from_level is single-thread;from_level_mt is separate,
  not selected. Existing lzma-rust2/XZ reservation16×preset dictionary+32MiB is conservative;
  fixed semantic presets need preventive budget admission and actual Job bound. No new deps.
- Continuation verified writer.rs431–482: finish builds bounded-by-admitted-plan raw/encoded
  header Vecs; header always uses default single-thread LZMA, optionally AES. Reserve default
  preset6 encoder memory even when data selects fast. SeqReader reads each source through EOF
  before advancing; native reader closes on EOF. No source plaintext spool is required.
- tar-0.4.46/src/builder.rs169append(header,Read),213append_data(header,path,Read)may emit GNU
  longname;header checksum updated.561finish writes1024zero bytes;126into_inner propagates
  finish and returns underlying writer. Use explicit native plan/header/data,never append_dir/
  append_file/from_path/unpack paths. Windows path encoding must be verified against independent
  Unicode fixtures,not assumed from successful builder import.
- Existing flate2=1.1.9 GzEncoder and lzma-rust2=0.21.0 XzWriter own approved single-source
  levels1/6/9 and1/6/7 plus full decoder verification;reuse contracts for outer TAR.GZ/TAR.XZ.

## Minimal change boundary for implementation

Reuse one CreationPlan/CreationSource and existing protected token/Job runtime. Extract only the
private native ID-source adapter from worker_zip_create if needed to share it;ZIP encoding,
crypto and read behavior must not be rewritten. New per-format worker adapter owns encoding,
full held-partial readback and exact plan/hash verification.7z Write+Seek may use finite anonymous
exclusive delete-on-close output spool;never plaintext source pre-spool for encryption,whole-
archive Vec or native final-path authority. Retain first typed source/output error across
backend wrappers. Before publication revalidate all sources and cancellation,then unchanged
native flush/recheck/single atomic-path commit. The new read-only prepare/create ordering must
apply to every new tree creation,not eager output-root creation.

Add only bounded optional private tree/sevenz options and negotiated capabilities,with old
ZIP/extract/Gzip/XZ fields/serialization unchanged and unsupported options rejected symmetrically.
Common tree root-strip/exclusions apply to all formats;7z LZMA2solid/nonsolid,AES256,optional
name encryption,finite semantic levels;TAR has no compression level/password. No RAR creation,
UI/product IPC/split editing/service/mapping/package/install/commit/push. Independent Python
tarfile and existing hash-verified7-Zip26.00 plus actual C#Job/native fixtures are required.

## Native directory size invariant discovered by real long-name fixture

C# CaptureSnapshot retains OutputNative.Identity.Size for directory membership/version checking;
ToDto sends that physical value. An NTFS directory may have nonzero physical size as its index
grows. CreationPlan's PlannedSource.size is archive payload size and must return0for directories,
while original SourceEntry.size and C# held snapshot stay unchanged for revalidation. Actual
long-CJK/multiple-root creation red rejected at7z exact size comparison; independent protocol
probe with synthetic0directory size did not reproduce. Public plan4096directory red proves the
semantic mismatch,not encoding corruption. No format data/identity check should be relaxed.

## Executed continuation

Per-format actual Start/Job/worker/native reds then green, directory payload-size and empty7z
constructor-budget regressions, exact independent7Zip/Python oracles and final integration are
recorded in [tree creation evidence](../tree-create-evidence.md). Only the single current
checkpoint owns current counts/remaining Phase A gates; this API note is not phase acceptance.

## Creation-verifier typed-I/O follow-up

Exact locked sevenz-rust2 0.23.0 reader.rs: ArchiveReader::new calls Archive::read on generic
Read+Seek; BlockDecoder::for_each_entries builds bounded optional CRC readers and may wrap
callback errors for encryption. error.rs Io/MaybeBadPassword wrapping cannot restore provenance
already discarded by the caller. Next's encoded_header budget preflight also had a blanket
read_exact -> CorruptArchive projection. Creation now retains first underlying spool Read/Seek
typed failure outside the entire verifier before those wrappers; untagged decode/data errors
remain corruption. Private generic reader seam does not grant path or wire authority.
The public API version/dependencies,preventive budgets and publication contract are unchanged.
This repair re-read the recorded Archiver create dispatcher,SupaZip7zcreate and bounded legacy
NxTools archive-term search;no new reference entry point or interaction lifecycle was found.
Actual red/green and final results: [typed-I/O repair](../sevenz-create-typed-io-remediation.md).
