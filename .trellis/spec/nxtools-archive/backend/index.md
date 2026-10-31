# Archive backend

Owner: host-neutral archive metadata/codecs/path/encoding/budgets/control. Partial Phase A includes ZIP0/8/9/12/14/93/95 plain/ZipCrypto,AES AE1/AE2all strengths Store/Deflate and AE2AES256extra-method reads,standalone streams/TAR compounds,isolated RAR4/5+7z and single-source Gzip/XZ creation. Genuine no-EOS LZMA known-size public/native evidence exists. Other AESextra-method cross-products and broad resources remain unverified;not a complete archive manager. Current evidence: `repo:rust/crates/nxtools-archive/src/formats/zip.rs`, `repo:rust/crates/nxtools-archive/src/formats/zip_aes.rs`, `repo:rust/crates/nxtools-archive/src/planning.rs`, `repo:.trellis/tasks/10-16-archive-manager/phase-a-checkpoint.md`.
Bounded general source planning and worker ZIP Store/Deflate/AES256 creation are implemented;ZIP/source/Job/native evidence is in `repo:.trellis/tasks/10-16-archive-manager/zip-create-evidence.md`. Subsequent7z(LZMA2solid/nonsolid/AES/header),TAR/TGZ/TXZ creation reuse that native source/output runtime; exact APIs and bounded red/green are in `repo:.trellis/tasks/10-16-archive-manager/research/tree-create-capability.md`. Full current gate results belong to the single checkpoint;ZIP64large-count/full resource/native/product acceptance remains incomplete.
Extraction metadata/MOTW changes also read [typed extraction metadata](extraction-metadata.md) and the [native owner](../../platformhost/archive-extraction-metadata.md); exact results stay in the single checkpoint.
Source-delete qualification changes also read [lifecycle facts](source-deletion.md) and its native-owner link; production recycle stays fail-closed pending the documented original-object API decision.
Read-only product IPC/helper/capability/credit work also reads [product read lifecycle](product-read.md); callable headless open/test does not add UI/FIFO/write/recycle authority.
Phase D general job/readmission changes also read [product job lifecycle](product-jobs.md); ArchiveRead remains unchanged/read-only and source recycling remains unavailable.
7z extraction risk/readmission changes also read [7z safety boundary](sevenz-safety.md): conservative
single-stream packed attribution and finite encoded admission do not enlarge actual output or Job budgets.
RAR4/RAR5 extraction risk/readmission changes also read [RAR safety boundary](rar-safety.md): exact
UnRAR split/solid/unknown metadata, RAR-only encoded opt-in and original cleanup-bound coordinator.
TAR/compound/standalone safety changes also read [TAR and stream boundary](tar-stream-safety.md):
private complete XZ/Zstd declarations, unknown totals and finite metadata-sized encoded/spool credit.
Phase E container/source-set work also reads [volume and ZIP-edit seam](zip-volumes.md). Production
worker activates volume routing only after native all-member acquisition; ZIP/7z new-family creation
and ZIP edit actions reuse original Job/output owners. Delivery evidence and pending acceptance live
in the sole task checkpoint; helper tests never substitute for native/product evidence.
Ordinary single-volume7z editing also reads [7z edit contract](sevenz-edit.md): strict original-header
admission, ordinal/subtree plan, disclosed bounded non-solidLZMA2 rewrite and original native handoff.
This separately authorized slice supersedes the earlier non-ZIP-backend-missing status only for
qualified7z operations. Step3 [TAR-family edit contract](tar-edit.md) separately admits qualified TAR/TGZ/TXZ with strict raw metadata and full outer/inner verification; RAR/volumes/TBZ2/TZST and ZIP ReplaceContent remain unavailable.
Step4 entry/conflict/terminal integration reads [edit interactions](edit-interactions.md); it reuses the original owners, optional private display and single candidate/native publication without expanding format capability.
Step7 HZ5 destructive extraction reads [Stored ZIP reclamation](reclamation.md): separate default-off intent, durable native authority, whole-file sparse units and fresh-process continuation; ordinary extraction/readmission remains source-preserving.

## Pre-Development Checklist
Read [shared architecture](../../shared/architecture.md), [engineering](../../shared/engineering.md), [verification](../../shared/verification.md), [reference note](../../../../docs/reference-code/archive.md), and the canonical [A–F task](../../../tasks/10-16-archive-manager/implement.md). Read [structure](directory-structure.md), [persistence](database-guidelines.md), [errors](error-handling.md), [logging](logging-guidelines.md) and [quality](quality-guidelines.md). No Windows APIs/GPUI or unrelated clipboard/extension intake policy in this crate. Additional codec allocation/cancellation gaps need the owning task's explicit decision, not silent dependency patches.

## Quality Check
`cargo test -p nxtools-archive --features isolated-codecs`; strict Clippy with the same feature; workspace check/fmt under required memory settings. Run actual C# Job/Rust codec/output fixtures plus existing clipboard/extension ZIP regressions. User-approved rechecked atomic-path overwrite replaces strict target-ID CAS; historical native32 red remains evidence, not a current CAS exit requirement. Verify pre/postcommit/cancel/cleanup states; no blanket Phase A/UI/A–F acceptance from these tests.
