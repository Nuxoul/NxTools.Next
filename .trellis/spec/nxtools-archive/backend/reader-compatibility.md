# Reader compatibility and independent source budgets

## 1. Scope / Trigger
User-authorized compatibility expansion for CAB, ISO/Joliet/Rock Ridge, UDF, WIM, AR, CPIO and standalone LZMA/LZ4/Brotli; 7z methods/layout and continuous ZIP volumes. Evidence: `repo:rust/crates/nxtools-archive/src/worker_formats.rs`, `repo:rust/crates/nxtools-archive/src/worker_codecs/sevenz_layout.rs`, `repo:docs/reference-code/archive-formats.md`. Existing ZIP/TAR/RAR/7z writers and native publication owners remain unchanged.

## 2. Signatures
`worker_formats::try_execute` is isolated-codecs-only and returns through the original worker. `Reader::extract(index,path,&mut MemberWriter)` uses ordinary final-name tokens and <=32KiB original-pipe records without per-write ACK. `ArchiveEngine::with_held_zip_volumes(&[String])` accepts original native-acquired paths,not an input JSON authority flag. Existing read/browser allowlists admit the listed formats without a new public command/version.

## 3. Contracts
- Original C# source acquisition,source identity,Job/process ceilings,timeout,correlation and actual cleanup remain authoritative. New libraries never unpack to filesystem paths. Fixed-size output/cancellation and retained first typed output failure precede ordinary metadata+drain+close; Create/Edit/cache owners remain separate.
- Encoded source and intermediate ZIP/TAR spools are independent of the selected text64KiB/image8MiB output limits. 7z/RAR source admission and isolated split-ZIP discovery/reopen use the existing finite general source allowance. Actual selected bytes, dictionary/working memory, metadata and native output retain their original limits.
- Continuous ZIP `.001/.002` input retains global ZIP offsets; genuine `.zNN/.zip` retains disk-coordinate normalization. Production continuous discovery uses only the exact ordered native-held list. Snapshot reopen uses its original external volumes, not a new directory scan. Public explicit volume discovery remains opt-in. Native MissingVolume survives worker/read/job projection.
- Interspersed 7z empty records use an operation-local, pre-budgeted decoded catalog layout with an inverse original-ordinal map. Group only stream-bearing records by their original block; trailing no-stream iteration covers all others. Do not change packed streams/substream CRC order, browser identity or callback completeness. Ordinary and retained-drag consumers share that mapping.
- 7z PPMd memory comes from its validated properties plus bounded state; BZip2/Deflate/BCJ2 state is charged with all other coders. Size-prefixed LZ4 block admission charges encoded bytes plus declared decoded bytes plus fixed scratch against working memory before allocating either full buffer; selected output is independently bounded.
- Multi-image WIM maps global ordinals to image-local IDs and prefixes member paths with the one-based image index; a single image keeps its original names. CPIO recognizes newc/crc/odc and both binary byte orders, preserves raw names and reuses existing encoding selection/provenance. Link/hardlink/special metadata stays visible but is not published as regular content. Their presence does not prevent read-only verification of ordinary files.
- Test success reports verified ordinary-file counts. Do not promise a payload checksum for ISO/AR/odc or other formats lacking one. Actual supplied-format integrity and source/native checks remain required.

## 4. Validation & Error Matrix
| Boundary | Required result |
| --- | --- |
| Small member in larger source/split source | Exact selected output within unchanged preview limit |
| Selected bytes or working/metadata/source allowance exceeded | ResourceLimit, no invalid publication |
| Missing native-held part | MissingVolume, actual cleanup; no target publication |
| Replaced source or wrong original ordinal | SourceChanged |
| Invalid decoded Windows name | Visible metadata; UnsafePath for whole extraction, no silent rewrite |
| Link/special in Test | Verify ordinary members; no special-file materialization |
| Native write/cancel failure wrapped by decoder | Original typed failure wins; incomplete owned final-name output is deleted,prior closed output remains |
| Distinct WIM images with same member name | Distinct prefixed paths and exact original image bytes |

## 5. Good/Base/Bad Cases
Good: an 80-byte text in a larger 7z/split ZIP previews; a py7zr solid catalog visits all original records; image 1 and image 2 yield different `same.txt` bytes. Base: ordinary archives use their existing paths. Bad: charge encoded source to text output, map callbacks by filename, treat iterator success as complete coverage, rescan unleased siblings, or silently sanitize irreversibly damaged names.

## 6. Tests Required
`repo:rust/crates/nxtools-archive/tests/worker_formats_compat.rs` drives the actual worker with controlled output replies, independent corpus payloads, binary byte orders, WIM ordinal mapping, corrupt stream and native write refusal. This is not real C# output evidence. `repo:csharp/NxTools.PlatformHost.Tests/ArchiveCompatibilityTests.cs` separately drives original Read/Entry/Job/native source/output/cache, exact bytes/commit counts, missing-volume category, preview refusal and exclusive source reopen. Run the exact `archive compatibility` fixture selection with `NXTOOLS_ARCHIVE_WORKER_FIXTURE` and `NXTOOLS_ARCHIVE_COMPAT_CORPUS`; TEMP/TMP point at PI_SCRATCH_DIR. Keep sevenz_layout, continuous_lease_tests, zip_volume_contract, public codec integrity and native source/output regressions. No physical GUI, deployment or broad resource acceptance is implied.

## 7. Wrong vs Correct
Wrong: decoder returns success or a file exists,so report extraction complete. Correct: bind original source/ordinal,charge each stage,consume complete selected integrity,observe ordinary metadata+drain+close,then report actual cleanup and completed counts. The supplied bsdtar odc names containing `?` remain an explicit Windows-path refusal; the native4096-active-file ceiling is not removed by adding readers.
