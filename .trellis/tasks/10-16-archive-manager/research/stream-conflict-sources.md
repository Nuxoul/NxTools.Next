# Standalone stream conflicts — verified narrow seam

2026-10-21. Phase A — Engine contracts and safety foundation only. Reference-first read
archive.md341–384, existing format/ZIP conflict independent acceptances and XZ/Zstd capability
notes. Legacy tracked ZipArchive/SevenZip/SharpCompress search still finds unrelated package ZIP
only (Services/BrowserExtensionPackageService.cs227). D:\Code\Archiver-main\core\src\lib.rs
230–239 selects stream decoders;665–670 single-file extract uses File::create(final)+copy_prog.
Do not copy truncating publication, approximate totals or extension-only recognition.

Original worker execute_attempt -> ArchiveEngine.open/signature+bounded metadata -> original
ExtractionPlan -> PrivateOutput.conflicts(shared immutable entry) -> format extract. Gzip236,
Bzip146,XZ79,Zstd319 currently call begin_file. Each fixed-buffer stream validates its own end/
applicable integrity and ALL actual bytes, then ensure_revision and checkpoint before commit;
error aborts exact owned token. Reuse begin_extracted_file(relative,None) and preserve same stream
call with sink if token=None, then source revision/checkpoint. Skip returns0files/0publishedbytes,
not verified-archive fiction: corruption/actual-budget failure still fails. Engine already derives
members_skipped from skipped_files; original worker summary counts skip/warnings. No new policy,
StreamConflict, protocol or codec refactor. Worker format restriction may admit these four known
ArchiveFormat variants; schema2extract-only and ArchiveRead read-only validation stay.

Original C#runner/Job/source/helper/slot/timeout and output session remain owners. Existing typed
InteractionPort ticket, target observation token/full signature, BeginFile/Commit recheck and one
approved atomic-path publication, numbered no-replace and cancellation are unchanged. No native
show/hide/layout/focus surface or lifecycle added. Production diagnostics reuse fixed correlated
awaiting_conflict/conflict_finished/terminal counts/elapsed/native codes; no payload/path logs.

Independent small existing fixtures: CPython gzip/lzma/bz2 literal UTF8+0..255 bytes, existing
checked Zstd bsdtar/libzstd+independent checksum fixture. Actual native regression distributes
Gzip AskOverwrite, XZAutoRename, BZip2Skip, ZstdOverwrite; waitCancel, skipped-Gzip CRC failure
and skipped-BZip2 actual-budget failure. Existing format/native recheck/isolation gates reuse
numbered race/oldticket/target change boundaries, not four duplicated failure matrices. Source
qualification uses existing never-called backend observer only; no Shell/recycle/delete.

Prior verified shared native/codec chain is preserved in [format conflict sources](format-conflict-sources.md)
and [independent format acceptance](../format-conflict-acceptance.md). Stream admission retains the
existing extraction_conflict_v1/ZIP compatibility markers; no new capability or wire shape needed.
Unknown incoming size,task-local ticket/cancel,postdecode source recheck and original native
publication remain unchanged. This slice changes only four output admissions and known-format
restriction; original C#production output/runner and stream decoders are byte-unchanged.
