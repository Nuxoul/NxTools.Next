# ZIP creation — bounded slice contract

Status: bounded ZIP preparation repair integrated green; see zip-create-output-preparation.md and unique phase-a-checkpoint.md. Historical zip-create-remediation.md and independent zip-create-remediation-acceptance.md/zip-create-acceptance-review.md remain retained. Phase A — Engine contracts and safety foundation remains current; no UI/product IPC/ZIP editing/split/source deletion/release scope.

## Boundary and first tracer

Existing worker_create only creates one regular Gzip/XZ source. Extend the same private worker/Job/native-output rendezvous, not a second controller. Files expected to change: archive creation policy and worker adapters/wire, Archive native source/output/runner adapters, their public/worker/native tests, exact test registration, archive manifest only for proved public backend features, and task/spec/reference evidence. Existing ZIP read behavior and approved publication are retained.

Tests use the user-approved seams: public creation policy, actual worker protocol, actual C# Job→Rust→protected native source/partial→publication. Independent Python plain ZIP and independent AES reader expectations originate from synthetic inputs, not produced archives. Each tracer is run red before implementation; no extra format reds before ZIP green.

## Minimal interface

- Input: ordered, host-authorized local file/directory roots (max32); worker has no authority to name another source or destination. Each native source object has an operation-private opaque ID plus observed name/kind/size/time; native session retains full volume/128-bit file identity and change metadata.
- Plan: Rust owns bounded traversal, relative archive names, single-root strip, validated exclusion globs, collision/entry/depth/name/metadata/input budgets. The public source capability is one cohesive business seam, not a public trait per helper. Metadata only is retained, never file contents.
- Native source: C# opens roots/ancestors without reparse/delete sharing; bounded one-directory enumeration and one active read handle. Enumeration derives a volume-GUID namespace from the held directory, reopens that actual directory and verifies its retained identity/version before enumeration; no reconstructed DOS enumeration/attributes. Root/child kind comes from relative no-reparse opens and handle basic info. Descendants are reopened relative to held roots through a bounded transient ancestor chain, comparing planned native identity before use. Bytes come from the same actual protected handle, not a normal path read followed by a hash.
- Resource strategy: max32 root chains, depth64, one directory enumerator, one active regular file and one transient chain; metadata/count finite on both sides. Plan does not retain one file handle per entry. No parallel file reads. Excluded members remain observed for directory stability checks but are not decoded.
- Mutation policy: fail closed on observed added/deleted/replaced/inaccessible members or changed identity/version at plan, open, read-close and final metadata/member revalidation. Reparse/link/special input is rejected, never silently skipped. This is optimistic identity/version validation at explicit boundaries, not a filesystem point-in-time snapshot or atomic source-set CAS. No once-only full-tree content hash scan.
- Self-inclusion: reject any output directory located at/below an input directory, plus source/final equality and actual identity aliases/reparse. Do not reject a source directory merely because it is below outputRoot: a final sibling ZIP is outside that input tree. This conservative dangerous-direction admission also excludes owned partial/readback artifacts; no source deletion.
- Options: ZIP Store or Deflate, semantic Store/Fast/Normal/Maximum (Deflate1/6/9); AES256 only with explicit private password. Root stripping only applies to exactly one directory. UTF8 names, empty files/directories preserved. No added ZipCrypto/AES128/192/extra-method writing.
- Output: exclusive same-directory partial token owned by current native session. Narrow create-only bounded seek/write/read adaptation if locked public writer needs it. Never write final paths in Rust or retain whole ZIP in memory.
- Verification: finish writer and propagate underlying errors; read same held partial into bounded anonymous delete-on-close spool if required by current File-based ZIP validator; validate original central/local structure and exact ordered plan names/kinds/sizes; fully stream every entry with CRC/AES authentication and compare size/SHA256 formed while reading source. Final source/member revalidation and cancel checkpoint precede existing flush/native single publication.
- Result/cancel: existing committed/unknown/native-state projection, cancellation and owned-partial cleanup; original target/source preserved on precommit failure, no automatic retry/delete after committed/unknown. Count archive publication separately from verified source entries.
- Negotiation: schema2 create_zip only, explicit source-tree/seek/read/ZIP-create capabilities checked by host. Additive fields omitted for old requests; reject irrelevant/invalid new fields symmetrically. No public PlatformCommand changes in this slice.
- Diagnostics: existing accepted/started/creation_planning/creation_encoding/creation_verifying/creation_committing and once-only terminal with same operation correlation/elapsed/native/state/count. No source paths/names/password/content/tokens or per-entry/chunk logs.
- Admission: ArchiveWorkerAdmission.Capture owns bounded independent SourcePaths, ZipOptions.ExcludePatterns, ConfirmedOverwrites and five immutable Limits scalars before create validation/wire/Run/accepted. The same snapshot authorizes native sources/output and Job/wire/native budgets for ZIP,extract,Gzip/XZ; caller add/remove/replace cannot change accepted authority. Observable inconsistent capture rejects before accepted; no unbounded ToArray/arbitrary JSON-first copy or caller-backed read-only wrappers. Legacy missing options/defaults and CreateName-only authority remain.

Exact locked API/reference findings, red→green counts, independent verification and remaining limits will be recorded in the creation capability/evidence ledger before updating the unique current checkpoint.

## Exact references inspected before implementation

- Re-ran legacy tracked source search; only unrelated package/FFmpeg ZIP and image/video compression, no general archive interaction identified (not exhaustive untracked/generated search).
- Archiver core/src/lib.rs859–983: collect→zip_write_items→create_zip. Unbounded path metadata, ordinary File::open/create and predictable parallel temporaries cannot supply native source/output safety. Local chrono conversion is honest DOS wall time, not UTC.
- SupaZip supazip/supazip-core/src/formats/zip.rs449–539: create uses supplied paths as names, only top-level loop, optional whole-file Brotli Vec. Do not reuse path/naming/resource behavior or fake Brotli ZIP.
- Locked zip8.6.0 write.rs2002–2018 new_stream supports sequential Write with data descriptors; no native Seek extension needed. large_file584 prevents u32 overflow; writer central count/offset emits ZIP64 at real thresholds. Source-known sizes choose ZIP64 before header if conservative compressed bound can exceed u32.
- write.rs621 public with_aes_encryption(Aes256,&str) borrows operation password; aes-crypto feature uses standard WinZip PBKDF2/HMAC/AES/random salt. aes.rs334–446 uses Zeroizing key/buffer, random salt getrandom::fill, full final MAC. Do not call fixed-salt API. ZipCrypto write.rs589 is crate-private, not used.
- write.rs1747 add_directory disables encryption/uses Store but new_stream omits its descriptor; actual full-test red proved this. Creation uses public start_file(trailing slash,Store),which closes an empty data stream with descriptor. Directory names remain visible,no name-encryption claim. types.rs457 sets UTF8 bit11 for nonASCII;ASCII stays UTF8-compatible. No low-level name patch.
- finish1795 propagates finalize errors; Drop2022 can write backend error text to stderr. Native sink retains first typed failure and stops physical writes while permitting explicit backend closure, then necessarily returns original failure and aborts, never publishes. No upstream patch.
- globset0.4.19 already locked, Unlicense OR MIT, no build.rs/native build; exact public GlobBuilder literal_separator/case_insensitive/build rejects syntax errors. New direct use disables default log feature. Pattern count/length and conservative memory reservation are finite.
- Final source/metadata limits,descriptor fix/red→green,dependency feature/license details and independent exact oracle are in zip-create-evidence.md. No broad large/ZIP64count acceptance from header-only tests.

Output collision identity checks in Runner use the already-held ArchiveOutputSession ancestors/root and relative CreateName final handle, not a fresh outputRoot DOS mapping. An output-namespace synthetic substitution regression retains real hardlink/fullID checks; no system mapping mutation is implied. This binds safety admission to actual publication authority without changing accepted target-recheck-close→publish TOCTOU.

ZIP preparation order: ArchiveOutputSession.Prepare holds only existing actual no-reparse
ancestors and bounded missing suffix; it cannot create directories. Original source lease /
eligibility and RejectOutputCollisions run against that session before CreatePreparedRoot.
Relative suffix creation checks cancel/source root versions before/after each component,then
the SAME collision logic rechecks the completed actual output session/final identity. Lexical
checks are early refusal,not alias proof. Invalid missing internal roots leave source members
unchanged; legal external missing roots remain supported. On later failure created directories
are retained,not recursively rolled back; cleanup closes leases/removes exact unpublished own
partials only,and never removes preexisting/raced/external directory content. CleanupComplete
does not promise global rollback. Exact real-worker red/green and80fixture integration are in
zip-create-output-preparation.md; original75results/failure remain historical.
