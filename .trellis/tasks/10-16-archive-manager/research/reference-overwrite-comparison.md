# Reference overwrite semantics — source comparison

## Scope and conclusion

Read-only source inspection of the supplied Archiver and SupaZip snapshots, following two user-pasted remediation/overwrite reports. **No reference executable or native overwrite probe was run, no new code fix or change to the approved contract is made here.** This comparison does not re-accept the remediation report's31/22 test counts; those are a separate verification task.

The references do not solve the exact current requirement. Separate three properties:
1. Preserve an old file until a fully written/verified new file is ready.
2. Publish a whole new file at the destination name in a single replacement operation.
3. Replace only the exact previously confirmed destination object, refusing an intervening name/object replacement (identity-conditioned publish).

SupaZip CLI addresses1/2 through sibling temp+persist. Neither inspected reference implements3. Their successful rename strategy therefore is not evidence that the current protected-target handle combination can work. Conversely, current replace_file:32 does not prove Windows lacks ordinary atomic name replacement or that all native primitives are impossible.

## Archiver — direct extract and path-based ZIP rewrite

Root `D:\Code\Archiver-main`:
- `core/src/lib.rs:583-611` ZIP extraction creates parent dirs, then `File::create(&out)` and writes decoded bytes. Existing destination is truncated before decode finishes; no partial commit or old-target identity condition.
- `core/src/lib.rs:614-640`7z likewise `File::create(&out)` at630. RAR uses UnRAR `extract_with_base(dest)` at655; no project-owned protected-target transaction is visible there.
- `core/src/lib.rs:765-824` ZIP delete writes predictable `<archive>.rewrite.tmp` via `File::create`, raw-copies retained compressed entries, finishes/flushes writer, exits closure (reader/writer drop), then `fs::rename(&tmp, archive)` at818. No expected destination file ID/hash and no retained non-share-delete target handle at publication. Rename errors propagate; this code does not establish the project's stronger crash/identity guarantees.
- `gui/src/main.rs:485-507` create prompts if `output.exists()` at492, then later calls core::create from background work. No binding from confirmation to target identity.
- `core/src/lib.rs:987-1073` create writes directly to final output (ZIP helper at927,7z ArchiveWriter::create at998,TAR/stream File::create at1042/1048); on error removes output at1071. Do not copy this for replacing an existing user archive: it can destroy the prior output before failure.

**Lesson:** raw-copy ZIP and prepare-before-rename are useful. Direct truncate and predictable temp paths are not the safe Next implementation. This reference does not demonstrate safe concurrent-target overwrite.

## SupaZip — direct extraction, better CLI create publication

Root `D:\Code\supazip-master`:
- `supazip/supazip-core/src/formats/zip.rs:298-342` extracts to `File::create(&outpath)` at320 (selected extraction repeats at419), then reads/checks/writes chunks. A cancelled/failed decode can leave a truncated destination; no final verified publication seam.
- `supazip/supazip-core/src/formats/sevenz.rs:406-449` uses File::create at413 before decoding. `formats/tar.rs` likewise creates target at237.
- `supazip/supazip-cli/src/main.rs:355-399` documents atomic create, collects the complete encoded archive in an owned Vec (not suitable for large archives), then:
  1. NamedTempFile::new_in(parent) at394;
  2. write_all at395;
  3. sync_all at396;
  4. tmp.persist(archive) at397.
  There is no earlier expected-target identity argument/check, nor a protected old-target handle.
- SupaZip Cargo.lock pins tempfile3.27.0. Inspected matching local published source:
  - `tempfile-3.27.0/src/file/mod.rs:732-745`: persist documentation says existing target is atomically replaced, notes same-filesystem requirement and no implicit file/directory synchronization, and warns about path-based security.
  - `src/file/imp/windows.rs:92-117`: persist uses SetFileAttributesW on temp, then `MoveFileExW(old_path,new_path,MOVEFILE_REPLACE_EXISTING)` when overwrite=true. No expected old file object/identity parameter.

**Lesson:** unique sibling temp→finish/verify→flush/close→single replacement is the useful pattern. It provides ordinary path-name replacement, not compare-and-swap of a previously confirmed file object. Do not inherit the whole-archive Vec.

## Why Next's current operation fails differently

Next holds old destination without delete sharing to prevent object/name replacement after validation, then calls NtSetInformationFile(FileRenameInformationEx) to replace that very destination directory entry. Earlier independent fixture/probe reproduced32 under this protected-handle combination. The references do not hold that target lease at publication, so their approach does not reproduce this self-conflict; they also do not enforce the associated identity condition.

Source-file read leases, dictionary/Job budgets and encrypted-output hashes repaired in the pasted report concern other invariants. They do not resolve destination replacement semantics.

## Product/engineering decision, not an API-name substitution

The strict identity-conditioned atomic-overwrite property was an added engineering safety contract, **not a behavior established from Bandizip or these two references**. That distinction should have been explicit earlier. No evidence here says it must be a baseline capability of an ordinary archiver; it also must not now be silently dropped to pass a test.

Options to present to the user:
- **Keep strict contract:** bounded investigation for a proved atomic+identity-conditioned primitive; keep genuine red test. Do not claim impossible based on current probe matrix.
- **Ordinary atomic path replacement (reference-like):** unique same-directory temp, full decode/test/flush then native atomic replacement at commit. Recheck immediately beforehand and protect parent/reparse policy, but explicitly acknowledge that a separate recheck does not eliminate target-name TOCTOU. This needs approval to narrow the strict identity guarantee; not equivalent to safe no-replace.
- **Protected handle backup→no-replace publish→restore:** retains the verified old object and refuses competitor overwrite, but has a name-visibility gap and crash/recovery complexity. Also needs approval; not atomic replacement.
- **Skip/auto-rename on conflicts:** already legitimate conflict policies using no-replace publication. Useful interim behavior but not completion of required overwrite support.

Do not replace this decision with endless API flag combinations or permit one gate to block unrelated required Phase A work. No option is approved by this investigation itself. Before claiming Bandizip uses any of them, obtain actual permitted evidence; proprietary implementation was not inspected.

## Subsequent user decision2026-10-17

The user has now approved the ordinary atomic path replacement option above: complete validated
sibling temporary, immediate identity/version recheck, close target check handle, single atomic
path publication. The close→publish TOCTOU is explicitly accepted; strict target-ID CAS is no
longer an exit requirement. Backup-first remains rejected. This update does not rewrite the
earlier source comparison/probe or imply reference/native behavior not actually inspected.
Follow design§4.3 and the current native gate for implementation/state/cancel requirements.
