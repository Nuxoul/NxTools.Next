# Step 3 — TAR family editing route (before implementation)

2026-10-28. Scope is **Phase E — Format expansion and ZIP editing**, third item of the user's seven-step list. Implement plain TAR first, then TAR.GZ/TGZ and TAR.XZ/TXZ. Do not start the fourth unified-interaction item, HZ3/HZ4/HZ5, packaging, installation, system integration, source reclamation, commit or push. Existing 7z/common foundation and old failures remain accepted/open at their documented scope.

## Baseline and reference

Opening evidence root: `C:\Users\Administrator\.pi-desktop\scratch\0c0e5320-11c4-40dd-bb50-f6a437188fc1\tar-edit`. Opening HEAD/status/owner hashes and complete checkpoint bytes are saved there before changes. The checkpoint is 169655 bytes, SHA256 `CB635E63E79A8E16702586943AF697E05856C4F5C34EB4B1CFC7E61B37415DE7`; only a prepend is allowed and the entire old byte sequence must remain an exact suffix. Other dirty work is not this task's attribution.

Read current checkpoint, sevenz-edit acceptance/outcomes, feedback-hz2-repair acceptance, canonical A–F plan, owner/shared contracts, archive reference and archive-editing note/manifest before implementation. Ark at `D:\Code\archive-edit-references\ark` is clean and commit `f7ea89e828925244d0a6b76cbfb4ac5b9fafe2e5`; readwrite plugin SHA256 `16D219AC0955DB5712AAD8D7C25C8108CF5509062417B16407B7727CD1447B5C` matches manifest.

Actual reference chain inspected: `kerfuffle/jobs.cpp` AddJob/MoveJob/DeleteJob -> ReadWriteArchiveInterface -> `plugins/libarchive/readwritelibarchiveplugin.cpp` addFiles/moveFiles/deleteFiles -> initializeReader + initializeWriter/initializeWriterFilters -> processOldEntries -> writeEntry/copyData -> finish. Reader base is `plugins/libarchive/libarchiveplugin.cpp`. Inspected plugin/base/jobs headers are BSD-2-Clause; no Qt/UI/source transaction is imported. Reference finish ignores close/commit and processOldEntries does not distinguish EOF from read failure; NxTools must check both. This is streaming-design evidence, not Ark runtime acceptance. Legacy NxTools general archive UI remains absent per the bounded tracked-source search recorded in archive.md.

## Smallest behavior gap and owners

Current Modify/public worker/native routing admits ZIP and qualified7z only. Required change is a separate TAR candidate writer and actual source qualification, routed through the same immutable ordinal/revision/attempt, native lease and one-publication chain. Do not put TAR into ZIP/7z encoders or replace UI/controller/queue/commit ownership.

Expected files: new archive TAR structural/catalog/planning/streaming candidate module(s), worker TAR editor and boundary tests; minimal model/engine/worker dispatch; existing Protocol/C# Modify shape and worker kind/capability routing; existing Desktop browser/draft/form format checks and cost disclosure; focused C# owned native fixtures; owner/reference documentation. IPC transport, source lease/share mode, ordinary extraction/test/read, dependencies/vendor and native publication algorithm are not changes in scope.

## Minimum fidelity/refusal table

| Structure | Planned handling, verified before writing |
| --- | --- |
| ustar ordinary regular/directory headers | Preserve raw header fields, uid/gid/mode/uname/gname/mtime and empty directories; change only necessary path/size/checksum fields |
| Local PAX | Bounded record parsing; allow path/size/mtime/atime/ctime/uid/gid/uname/gname, preserve exact value/precision; rewrite path/size only when required |
| GNU long name | Bounded long-name extension; preserve or regenerate only its name representation, retain ordinary GNU header metadata |
| Implicit directories | No invented old records; immutable descendant ordinal + explicit selected prefix; component boundary checks |
| Duplicate/case/file-directory-prefix names | Reject before encoding rather than collapse/guess duplicate resolution |
| Global PAX/unknown semantic extensions | Initially explicit UnsupportedOperation, no silent normalization/discard; revise only on concrete preservation proof |
| Links/devices/sparse/unknown special types | Explicit refusal before candidate write, not path-tree extraction or dereference |
| TAR integrity | Header/extent/end/tail validation and exact candidate content comparison; SHA256 is transactional comparison, NOT a TAR payload checksum |

No full POSIX/NTFS/ACL/ADS fidelity claim. New sources use existing protected source-ID contract and disclosed canonical creation metadata, not fabricated UTC timestamps.

## Compression and finite resources

Outer compression is separate from inner TAR. Use existing flate2/lzma-rust2 encoders and complete Gzip/XZ decoders; no external product backend/new ABI/dependency changes. Actual signature+validated TAR decides format, suffix alone never grants editing. Compressed input uses the existing finite anonymous decoded spool, not a path tree/whole archive Vec. Metadata/count, decoded bytes, encoded candidates/readback, memory, control and original Job time limits remain finite. Preserve compatible outer meaningful metadata where representable; refuse unpreservable fields or disclose any method/parameter change before submission. Multimember/tail policy must be explicit and tested, not decoder first-stream success. TBZ2/TZST/standalone streams/RAR/volumes/SFX remain unavailable for edits.

All edits rewrite the complete archive; compressed edits decode/reencode the complete outer stream. Form must disclose original+candidate/spool coexistence, unknown estimate honestly, no source deletion credit. Reuse official Kit0.8.0 Form/Field, Small nonclosable warning Alert, original outlined actions (outside Toolbar.child ghost adaptation), Root/titlebar/progress. Original picker/form/focus/next-turn cross-entity and no-event/control/result lifecycle remain. Fourth-step interaction unification is deliberately not implemented.

## Transaction, cancellation and diagnostics invariants

Native source/parents held -> revision/native binding + edit-only structural admission -> immutable plan -> stream old payloads (including removed/replaced members for bounded complete reading) / transform or skip / new protected sources -> one complete finite candidate -> checked TAR/outer finish and flush -> full local candidate verification -> held native readback/full outer+TAR+exact metadata/content verification -> truly close ALL worker old readers -> original acknowledged edit_reads_complete -> C# still-held full source/target/candidate recheck -> final cancellation -> release ONLY source FILE -> ONE original native publication. Keep protected parents/candidate, accepted final-check/close/path-publish TOCTOU (not ID-CAS), commit5/64/latecancel truth, unknown no automatic retry. No delete-old/backup/truncate/global sharing/recursive path cleanup.

Original correlation accepted/start/fixed planning/reading/reencoding/verifying/committing/terminal/elapsed/native codes; no path/name/content/password/byte-tick logs. Processed is not committed. Original FIFO/same-archive serialization/pause/cancel and active preview/drag/nested/read/edit lease policy remain.

## Pre-agreed regression seams and execution

User task explicitly requires real format/worker/native seams. First vertical RED is a compiled actual worker Modify TAR rename with full readback and real exclusive-reopen at handoff, plus corrupted readback no handoff. Then plain operation/metadata/unsafe-structure cases, compressed TAR routes, edit-specific encoder finish/flush injected storage errors, source/cancel/budget/write/readback/native outcome cases. Independent CPython tarfile and bsdtar/stock7Zip are isolated oracles, not product backends; record versions/hashes/availability and all skips.

Every Cargo call naturally checks live Cargo/rustc before starting, same-process jobs1/incremental0/RUSTFLAGS=-C codegen-units=256, --locked --offline, serial tests, full timeout; no duplicate or name kill. Preserve raw failures and complete test names/counts. Effects audit -> exact C# LIST -> identical RUN. Build final isolated-codecs worker from this round's source, copy/freeze in this session scratch, use that exact file/hash for native checks. One integrated convergence; strict affected-owner Clippy --tests -Dwarnings, workspace --tests, scopedfmt/diff and applicable spec/release/editor/component gates.

Known unrelated readmission source_unavailable, historical CLI helper_untrusted and FB-AC05/real Explorer/DPI/IME/UIA/user/installed acceptance remain open. Final outcomes must report exact supported/refused shapes, cost/fidelity, commands/counts/skips/failures, frozen worker, source drift and unexecuted items. Stop after Step3 report for coordinator independent acceptance, not PhaseE/A–F completion.

## Recorded integration refinement
The initial ZIP-style Desktop reinspection experiment was replaced before delivery by optional typed `ArchiveBrowserContext.tar_edit` completed-worker facts (Rust/C# parity). Browser/ModifyDraft missing facts fail closed; TAR remains fully decoded/admitted in the original isolated worker, not Desktop. No new native ABI/backend/controller. Exact current public composition remains Form::vertical/Field and Alert::warning.with_size(Small), default nonclosable, no Toolbar adaptation or new palette. Post-implementation boundary findings and all test/compile/OOM histories belong to tar-edit-outcomes.md, not retroactive changes to the plan above.
