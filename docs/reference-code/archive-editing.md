# Multi-format archive editing — local-source review

2026-10-28. User requested a second review against locally cloned source, not website feature lists. **Research only:** no reference/product build, test execution, archive modification, dependency/backend integration, package/install/system operation. Current NxTools ZIP editing and HZ1/WAC acceptance remain unchanged. This is not a new implementation phase or universal format-support promise.

## Immutable local inputs

Root `D:\Code\archive-edit-references`; all five worktrees were clean at review. Source file hashes/anchors are in [manifest](archive-editing-manifest.json). Recheck commit and hashes before reuse; line numbers are navigation, not immutable identity by themselves.

| Repository | Commit | Main value |
| --- | --- | --- |
| rikyoz/bit7z | a9e4c8a54b67672a89b8a934868abef87a880317 | Small explicit edit model, UpdateItems adapter and tests |
| ip7z/7zip | b88c6bc09339de32f7d18bbab900686b244f42ba | Actual format update/repack engine |
| M2Team/NanaZip | 555a3ff0f0e6c754dbf93c528253afeb0444e9e1 | Windows UI→Agent→format update, subtree rename, password/progress |
| KDE/ark | f7ea89e828925244d0a6b76cbfb4ac5b9fafe2e5 | TAR/filter streaming rewrite and per-backend capability policy |
| peazip/PeaZip | a6259b90031827887bae6a041db1f1c1787c6825 | User operation/backend orchestration; not a safe transaction implementation to copy |

Earlier [archive note](archive.md) covers Archiver/SupaZip. Correction to conversational overstatement: Archiver `core/src/lib.rs::delete`763–825 has a generic extract-kept/recreate branch for writable non-single-file formats, not ZIP-only. It is a delete reference, not evidence of complete add/rename APIs. SupaZip's inspected ArchiveFormat has list/extract/create/test, not a general edit API. [HatchZip](hatchzip.md) is extraction/planning/recovery research, not a general archive editor.

## Capability conclusions — actual code, not runtime acceptance

| Shape / operation | Verified source route | Important limitation |
| --- | --- | --- |
| Ordinary single-volume7z add/replace/delete/rename | bit7z Editor/Writer→IOutArchive::UpdateItems; NanaZip Agent→7zHandlerOut/7zUpdate | Solid changes can repack affected blocks; encrypted repacks need password; does not imply cheap in-place edits |
| TAR add/delete/move-as-rename | Ark ReadWriteLibarchivePlugin streaming rewrite; NanaZip TAR UpdateItems; PeaZip delegates7z a/d/rn | Header/data/metadata fidelity, duplicate names, links, sparse/PAX and empty-result policy need independent acceptance |
| Compressed TAR members | Ark writes TAR and reapplies original compression filter | Not an edit of a single gzip/xz member through bit7z; outer/inner layers must be preserved. NxTools existing writers cover only their implemented subset |
| Existing multi-volume archives | 7-Zip rejects actual input VolumePaths>1; NanaZip/Ark have equivalent restrictions | Creating volumes does not imply updating a volume family safely; keep separate scope |
| RAR write/update | PeaZip conditional external WinRAR commands | Not supplied by UnRAR/7-Zip writer. Requires separate tool/licensing/product decision, not silent downloaded/bundled executable |
| Standalone gz/bz2/xz | bit7z existing-archive update constructor refuses non-MultipleFiles formats | Writable encoder!=multi-entry edit support; do not promise “all writable formats editable” |

## 1. bit7z: explicit edit intent, useful test model

Paths relative to `bit7z`:

- `include/bit7z/bitarchiveeditor.hpp:40–61`: editor inherits writer; add/new plus rename/update/delete, applied only on applyChanges.
- `src/bitarchiveeditor.cpp:66–104`: rename indexes old member with new property path; update supplies file/buffer/stream data. `:106–131,166–200`: delete policy distinguishes item-only from recursive descendants. `:222–230`: no-op check→compressTo original→reopen.
- `src/bitarchiveeditor.cpp:296–313`: renamed old member NewData=false/NewProperties=true; changed contents/new members request data. This API does **not itself expand a renamed directory subtree**; a product planner must map descendant paths, as NanaZip does. Do not assume single renameItem changes all children.
- `src/bitoutputarchive.cpp:225–248`: update mode removes old matching-name index, remaps remaining input indices and calls UpdateItems; non-S_OK becomes error. `src/bitinputarchive.cpp:379–381`: QueryInterface obtains IOutArchive from opened input, so read formats lacking output interface cannot update.
- `src/internal/updatecallback.cpp:44–65,125–153`: total/completed/ratio callbacks; completed callback false→E_ABORT; old-index/new-data/new-properties and password callbacks. It is cooperative cancellation, not forced interruption of all IO or codec allocation.
- `src/bitoutputarchive.cpp:81–87`: explicitly rejects existing single-file formats. `:252–279`: same-path update writes separate output, closes input, then commits; configured volumeSize>0 rejected. This particular volumeSize check alone is **not proof of detecting every existing volume family**; refer to 7-Zip/NanaZip input-volume guards.
- `tests/src/test_bitoutputarchive.cpp:117–166`: single-file update refusal and actual test logic for replacing/adding7z members. `tests/src/test_bitarchiveeditor.cpp:51–88,137–468` contains7z/TAR/WIM/ZIP delete cases; `:762–883`7z rename/update contents; `:485–717`temp-collision/symlink/staging/multivolume-output cases. These tests were READ, NOT RUN. TAR delete-last-item case is commented out with an old7-Zip22.01 Ubuntu Release problem (`:99–106`); do not advertise complete test matrix.

### Do not import its transaction unchanged

`src/internal/atomicfilereplacer.cpp:26–50` retries temp names but creates exclusively; default CFileOutStream CreateNew (`cfileoutstream.hpp:25`) maps Windows CREATE_NEW/POSIX O_EXCL (`filehandle.hpp:67–78`). This is stronger than the old vulnerable truncate-existing temp pattern. **Predictable names alone are not evidence of that old vulnerability in this commit.**

However, `atomicfilereplacer.cpp:63–93` closes/releases then path-renames; no expected-old native identity is compared at that boundary. A conditional MinGW+standard-filesystem branch deletes the old target before rename (`:73–82`). Its default destructor has no explicit temp-file cleanup (`atomicfilereplacer.hpp:34`); inspected FileHandle destructor closes only (`filehandle.cpp:96–101`). No explicit durable flush/verified-readback step was established in this boundary. These are static integration caveats, not reproduced loss/crash findings. NxTools must retain its stronger C# held-source/output/native commit/journal/cleanup ownership.

License: inspected sources MPL-2.0; backend7-Zip license separate. Adding bit7z would introduce C++/7z-library integration and distribution work, not a drop-in Rust crate or dependency-free improvement.

## 2. 7-Zip and NanaZip: complete update planning and solid-block behavior

7-Zip paths:
- `CPP/7zip/Archive/IArchive.h:509–531`: IOutArchive UpdateItems contract.
- `CPP/7zip/UI/Common/Update.cpp:477–528`: rename pairs preserve old data and change paths/properties. `:853`actual UpdateItems; `:1299–1303`reject actual multi-volume input; `:1316–1320`reject trailing-data update.
- `CPP/7zip/Archive/7z/7zUpdate.cpp:1994–2054`: counts retained streams in each old solid folder; whole retained block can copy, partial retained block needs repack; encrypted repack tracked. `:2180–2205`obtains decoder password. Progress complexity includes pack copy/repack work, not just “number of files deleted”.
- Additional edit-admission check2026-10-28: `CPP/7zip/Archive/7z/7zOut.cpp::SkipToAligned:462–479` writes repeated `kDummy` (0x19) FilesInfo padding with zero-only payload before aligned time/attribute values; `7zIn.cpp:1439–1445` consumes it, flags nonzero bytes as header error and explicitly excludes it from preserved property IDs. Zero-only alignment is not unknown member metadata; accept bounded repeated zero padding, reject nonzero padding and all other unknown semantic properties. Immutable files are rechecked against the pinned commit before use.
- `CPP/7zip/UI/Common/Update.cpp:1606–1682`: closes old archive, preserves temp on move failure, but path1634 deletes original before moving new file. Do not reuse this UI-command replacement boundary as identity-conditioned atomic replacement.

NanaZip inspected **Classic** call chain (Modern has matching Agent logic; not an exhaustive duplicate-source audit):
- `NanaZip.UI.Classic/SevenZip/CPP/7zip/UI/FileManager/PanelOperations.cpp:41–98` worker IFolderOperations rename/delete and progress; `PanelCopy.cpp:351–421` CopyFrom add.
- `.../UI/Agent/ArchiveFolderOut.cpp:97–163`: temp output→DeleteItems/RenameItem/DoOperation2→close→MoveToOriginal→reopen.
- `.../UI/Agent/AgentOut.cpp:306–413` update pairs→IOutArchive::UpdateItems; `:435–478`omit deleted entries; `:537–603`expands folder real indices and rewrites each descendant prefix. This is the better directory-rename semantics reference.
- `.../UI/Agent/Agent.cpp:1586–1597`CanUpdate limits layer/device/tail; `NanaZip.Core/SevenZip/CPP/7zip/Archive/7z/7zHandlerOut.cpp:392–413`rejects unsupported input/volumes under applicable build; `:721–819`password/header-encryption/solid options. TAR UpdateItems is real (`Archive/Tar/TarHandlerOut.cpp:184–285`). Solid WIM is separately non-updatable (`Archive/Wim/WimHandler.h:75–92`); not target scope.
- `.../UI/Common/WorkDir.cpp:85–92`returns replacement native failure, but `.../CPP/Windows/FileDir.cpp:832–845`with deleteDestBefore=true **deletes destination first** then moves. Preserve this as a caution, not preferred transaction code. Reopen can also fail; no full end-to-end recovery acceptance here.

License:7-Zip mostly LGPL-2.1-or-later with per-file BSD/public-domain and restricted UnRAR decompressor portions (`DOC/License.txt`). NanaZip-authored code MIT, inherited SevenZip code NOT MIT (`License.md:7–22,70–165`); icons have separate terms, are not reuse candidates.

## 3. Ark: streaming TAR rewrite, backend-specific capability policy

- `part/part.cpp:1433–1451`rename becomes MoveJob; `kerfuffle/jobs.cpp:701–874`Add/Move/Delete jobs call original ReadWriteArchiveInterface.
- `plugins/libarchive/readwritelibarchiveplugin.cpp:33–199`add/move/copy/delete; `:422–501`iterate old headers, change selected paths or skip deletes/replaced entries, stream survivors. No whole extracted directory tree is necessary for this route.
- `:202–210`7z writer only for7z, otherwise pax-restricted TAR. `:247–298`preserves recognized old compression filters. This is useful for TAR family, not proof all original records/filters/metadata are byte-preserved.
- `plugins/cli7zplugin/cliplugin.cpp:69–101`7z a/d/rn; CLI7z higher priority for7z/ZIP. Libarchive7z path has no encrypted7z capability; CLI path provides encryption. Do not conflate plugin declarations.
- `kerfuffle/archive_kerfuffle.cpp:256–259`nonempty multivolume read-only; `part/part.cpp:453–480`encrypted add disabled when password unknown to prevent mixed plaintext/encrypted content.
- Progress/cancel uses jobs/backend callbacks and thread interruption; CLI route may terminate child. This does not certify transactional cancellation for a new integration.

**Concrete source caveats:** `finish(:407–419)`does not inspect archive_write_close or QSaveFile::commit results. `processOldEntries(:440–501)`loop exits on any non-ARCHIVE_OK header but returns only !interrupted, so copied design must distinguish normal EOF from read failure. These are directly observed unchecked-return/control-flow patterns, not locally reproduced archive corruption. NxTools should explicitly check read→encode finish→flush/verify→native commit and publish precise terminal facts.

License is file-scoped: libarchive read/write plugin and inspected jobs BSD-2-Clause; UI part and CLI7z plugin GPL-2.0-or-later. Do not label entire Ark BSD or copy GPL UI code as permissive code.

2026-10-28 Step3 actual checkout recheck remains clean at f7ea89e828925244d0a6b76cbfb4ac5b9fafe2e5. Additional inspected chain entries: `kerfuffle/jobs.cpp` AddJob/MoveJob/DeleteJob -> ReadWriteArchiveInterface -> `readwritelibarchiveplugin.h/.cpp` addFiles/moveFiles/deleteFiles -> base `plugins/libarchive/libarchiveplugin.cpp` reader -> initializeWriterFilters/processOldEntries/copyData/finish. All4file hashes are retained in the manifest. NxTools retains its own immutable ordinal/source/controller/native publication owners, checks true EOF/finish/flush, and refuses unsupported metadata before rewriting. [Step3 route and invariants](../../.trellis/tasks/10-16-archive-manager/tar-edit-route.md), [exact outcomes](../../.trellis/tasks/10-16-archive-manager/tar-edit-outcomes.md); reference inspection is not Ark runtime acceptance.

## 4. PeaZip: orchestration reference, not edit engine

Paths relative to `PeaZip/peazip-sources/dev`:
- `peach.pas:59176–59335`existing add eligibility; `:58717–58894`precheck→archive_mainsequence/goarchive→`compose_archive_cl`/launch. `:50777–50803`7z a/u/update-existing/synchronize modes; `:53286–53323,55258–55405`backend launch.
- `:35511–35587`delete dispatch; `:51594–51600`7z d. `:61571–61807`rename uses backend rn **after deleting colliding destination**; `:61863–61953`per-item UI loop. Return results of execute_cl_simple are not consumed in renamefileinarchive.
- `:50583–50613`editable allowlist7z/TAR/ZIP/etc.; RAR only havewinrar && userar. Forced unsupported editing switch exists, must NOT copy into NxTools safety policy.
- Compressed TAR is not directly allowlisted as outer editable file: nested TAR auto-open (`:36900–36927`), temporary inner TAR editing and generic changed-preview parent sync (`:56504–56540,79527–79558`). This is staged nested edit/reinsert, NOT proven complete end-to-end compressed-TAR support in this review.
- `unitgwrap.pas:1496–1520`parses stderr percentages; `:1903–1931`password pipe then child kill on cancel. Delete/rename do not share equally rich progress/cancel. Do not copy command-line password fallback or console command rendering into secret-safe diagnostics.
- Collision staging/restore (`peach.pas:58358–58476`)uses intermediate internal names and multiple backend commands, not a single rollback-safe edit transaction. Process termination/crash between stages cannot be assumed recoverable.

License: peach.pas424–440 LGPL-3.0-or-later; unitgwrap.pas160–176 LGPL-2.1-or-later. ppp.pas permissive password utility is unrelated to archive editing; not a suitable edit-owner source. RAR path requires separately configured proprietary WinRAR; not evidence that open-source UnRAR can encode RAR.

2026-10-29 Step4 rechecked the same clean PeaZip commit and hashes before implementation. Additional reviewed boundaries: `peach.pas:58358–58476` precheck/collision temporary-name staging and ignored-return risk, `:59176–59335` actual add eligibility/form route; `unitgwrap.pas:1881–1945` process constructor/start/password/wait/pause/cancel ownership. Existing file provenance/license stays in the manifest. [Step4 route](../../.trellis/tasks/10-16-archive-manager/edit-interaction-route.md) records actual entrypoints and invariants; [Step4 outcomes](../../.trellis/tasks/10-16-archive-manager/edit-interaction-outcomes.md) records only NxTools tests. No Pascal code/backend or unsafe multi-command/delete-first transaction was imported.

## NxTools recommendation — proposal, not implementation authorization

1. **Keep accepted ZIP record-preserving editor unchanged.** Reference raw copy/update does not justify degrading record/encoding/encryption/metadata fidelity.
2. Add a typed **per-format + archive-state + operation capability** policy, not a hardcoded format==Zip global banner or “can create therefore can edit”. Separate browsing/extraction/preview/add/delete/rename. Scope initial candidates to ordinary single-volume7z and TAR/current compressed-TAR writer subset; explicitly reject unproven volume/encryption/unsupported metadata shapes rather than silently normalize away data.
3. **TAR family:** prefer Ark's stream-old→transform/skip→stream-new pattern within current Rust worker and current compression encoders. Do not extract all survivors to a normal temporary directory; preserve safe entry identity/path/metadata policy and disclose recompression cost. PAX/sparse/links/unknown entries/duplicate names need explicit retention or fail-closed policy.
4. **7z:** choose separately between (A) bounded rewrite using current Rust reader/writer, accepting recompression and proving metadata/encryption preservation, or (B) separately approved isolated7-Zip UpdateItems backend for packed-block reuse. bit7z is a useful model/test reference, not automatically the best dependency. Need actual locked-Rust API/codec feasibility gate before selecting; do not promise zero-copy or cheap solid edits. Adding C++ ABI/helper/DLL/licensing/payload changes needs explicit approval and existing worker/Job isolation.
5. Rust owns edit intent/member IDs/source revision/capability/budgets/progress; C# owns held native source/output namespace and verified replacement, never delegated path-based delete-old/move-new. New output must finish, readback/verify as required, preserve old on precommit failure, and retain accurate commit/journal/cleanup state. Do not hold a foreign writeback callback over a GPUI lease.
6. Reuse original logical job/attempt/generation, password intent, original queue and cancellation. Model stages planning/copy/repack/verify/commit truthfully; emitted codec progress != committed archive. Existing HZ1 advisory free space must consider encoded replacement coexistence plus bounded temp needs, never source-delete credit.
7. Regression gate before feature edits: add/delete/file+directory rename, conflict/duplicate/case/unicode, encrypted/header-encrypted/solid, zero/empty-result, wrong password/cancel/low space/write/finish/readback/commit failure, old-source replacement, output validation and no-loss expected identities. Independent tools parse/test and compare member content+metadata; upstream test code presence alone is not acceptance. Crash/restart needs separately real durable seams; no fake power-loss claim.

## Verification and limits

Five commits and clean worktrees checked before and after; critical reviewed-source hashes recorded. Original source was read, not modified. Only reference documentation/index/manifest are added in NxTools. Run spec-contracts.py for document links/evidence and scoped diff; those checks do not compile APIs or prove behavior. No reference/Cargo/native/archive-write tests, GUI, real user archive, package/install or system action was performed. No online status/maintenance/version claims beyond these pinned local snapshots.

## 2026-10-29 HZ4 bounded repair — encoded header source map

Before product repair, both pinned worktrees were rechecked clean at the commits above. `7zip/CPP/7zip/Archive/7z/7zIn.cpp:1098–1155,1211–1274` and matching `NanaZip.Core/SevenZip/CPP/7zip/Archive/7z/7zIn.cpp` define omitted SubStreamsInfo as one unpack stream per folder, preserve PackPos relative to signature base, and check defined folder CRC after decoding metadata. Their actual file identities are recorded in the manifest; this is format-source evidence, not execution of the reference products.

Locked `sevenz-rust2=0.23.0` actual registry source was read, not edited: `src/reader.rs:432–512` directly decodes encoded-header StreamsInfo, while `:661–691` requires populated SubStreamsInfo for a synthetic streamed FilesInfo entry; `:1354–1419,1839–1905` BlockDecoder uses original PackPos/pack offsets, folder/substream CRC and real stream-count mapping. NxTools `sevenz_edit_header::inspect` currently builds a streamed synthetic file around that descriptor; absent default stream facts must not be mistaken for corrupt original header bytes. Strict original descriptor consumption, metadata/budget/CRC/encryption checks and actual source/native ownership remain required.

Writer-side investigation separately follows locked `src/writer.rs:422–500` -> `writer/pack_info.rs` / `writer/unpack_info.rs`: packed-header CRC, folder versus substream digest placement and encoded metadata are inspected against pinned `7zOut.cpp:405–457`. Normal original-Job output and one-variable private header copies are the regression evidence; no secondary oracle removal, vendor edit, dependency upgrade or new backend is authorized. Exact findings and current acceptance belong to the HZ4 repair report/checkpoint, not this reference map.

### HZ4-7Z02 — partial inline WinAttributes reader witness

Pinned `7zIn.cpp::ReadBoolVector2:1158–1176` / WinAttrib `:1386–1391` and `7zOut.cpp::WriteAlignedBools:482–491` / attributes `:808–809` establish all-defined, optional MSB definition bitmap, external flag, then defined DWORDs. The real four-entry output preserves attributes32/32/16 and leaves the added directory undefined: `15 0f 00 e0 00 20 00 00 00 20 00 00 00 10 00 00 00`.

Read-only upstream libarchive v3.7.7 `archive_read_support_format_7zip.c:2698–2724` consumes two bytes before the partial-definition bitmap. Its snapshot URL/hash is in the manifest, not asserted to be byte-identical source provenance of Microsoft's executable. Actual pinned binaries independently distinguish canonical partial attributes(stock7Zip0/bsdtar1), only bitmap/external swapped(nonconforming2/0), and private all-defined witness(0/0), with identical packed data and valid CRCs. The all-defined witness invents a fourth attribute only for diagnosis; production must not acquire it. Normal original Job output reproduces the issue without process interruption.

[Parent's bounded reader decision](../../.trellis/tasks/10-16-archive-manager/hz4-7z02-reader-decision.md) permits test-only `reader-unsupported`, never reader PASS, solely for the witnessed structure and pinned identities after complete primary/native/process/refusal gates. Unknown structure/CRC/identity remains FAIL; no metadata stripping, writer/profile/backend or tool substitution. Original FAIL evidence remains immutable.

## OPT-05 reusable plan source boundary

Observed before OPT-05: `formats/zip::preflight_created` validated local headers/descriptors/overlap, streaming reread local headers, and `zip_edit::structure` repeated admission while retaining raw headers. Current private `formats/zip/plan.rs::ValidatedZip` exclusively borrows its actual reader and immutable original entries, retaining admitted ordinal/data/packed/end/header facts through Test/extract/edit. `structure` reserves raw Directory records before constructing that plan. Candidate planning source-verifies removed/replaced-away payloads, structurally admits additions, then completely decodes retained/new final payloads. Raw local/central names/extras/encoding/crypto, separate old-source/final actual-byte quotas, exact held partial and native single-publication ownership remain.

`ArchiveSourceLease::CaptureEditSource/VerifyEditSource` brackets each fresh source hash with full `ArchiveRecycleVersion` capture and compares the concurrently held target's complete version under uninterrupted no-write/no-delete protection. Equivalent observations share only that same call's read. EditOpen/EditVerification/EditReadsComplete/EditHandoff/Prepublication remain fresh; chunk cancellation, single-use read-completion and FILE-only handoff remain. `ArchiveOutputSession::CommitCore` retains flush/fingerprint, final target checks, actual rename, postpublication verification and committed/unknown precedence over late cancellation. TargetAdmission/TargetBegin and output reads remain separate.

`ArchiveWorkerRunner::ExecuteAsync` retains its source/output references after actual resource disposal, resolves one terminal, calls `DiagnosticTerminal`, then publishes that same event. `DiagnosticTerminal` aggregates existing source and primary-output `ReadObservation.Snapshot()` by Source/Target/Output, exporting `native_{role}_{attempts,full_reads,read_bytes,completed_bytes}` and separate `partial_read_calls/partial_read_bytes`. `Run.Source` includes an established lease whose bridge verification was interrupted before `BindEditSource` assigned its field. No extra source read or native authority is introduced; actual-byte versus EOF/digest-completed credit, original correlation/final result and throwing-sink isolation remain.

Ordinary `worker::execute_attempt(modify_zip)` uses private `ArchiveEngine::open_catalog` before `worker_zip_edit::execute`, avoiding optional browse qualification followed by duplicate candidate admission. Actual intent-aware `zip_edit::candidate` checks source revision, format/volume/application-container boundaries, backend safety, metadata, local/descriptor/extents, kept extras and full final payloads. The worker binds original browser/native source and refuses unsupported semantic operations before publication. A no-change Add independently calls fresh modification eligibility because no candidate executes. Public browse qualification remains fresh; unqualified catalog capabilities never become edit authority.

Directory Rename resolves the selected original ordinal and directory kind. An attribute-defined directory need not end in a slash; descendant matching uses a normalized slash-ended component prefix, never raw `starts_with` on the directory spelling. Only selected/subtree record names are rewritten; unrelated raw records remain byte-preserved.
