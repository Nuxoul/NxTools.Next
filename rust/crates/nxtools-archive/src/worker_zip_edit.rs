//! ZIP mutation through the original worker/native output rendezvous, never a final path write.
use crate::{
    ArchiveEngine, ArchiveError, OperationControl, Result,
    creation::{CreationSource, IncomingPlan},
    interaction::InteractionPort,
    output::ArchiveOutput,
    worker::WorkerRequest,
    worker_conflict::add::{self, ExistingMember},
    worker_output::{PrivateOutput, ReplyPort},
    worker_sources::Sources,
    zip_edit::ZipEdit,
};
use nxtools_protocol::archive_job::ArchiveJobModifyAction;
use sha2::{Digest, Sha256};
use std::{
    cell::{Cell, RefCell},
    fs::File,
    io::{Read, Seek, Write},
    time::UNIX_EPOCH,
};

fn addition_options(directory: bool, size: u64) -> zip::write::SimpleFileOptions {
    let large = size
        .checked_add(size / 1000)
        .and_then(|value| value.checked_add(65536))
        .is_none_or(|value| value >= u32::MAX as u64);
    zip::write::SimpleFileOptions::default()
        .compression_method(if directory {
            zip::CompressionMethod::Stored
        } else {
            zip::CompressionMethod::Deflated
        })
        .large_file(large)
}

type AdditionPlan = (
    Option<File>,
    Vec<ZipEdit>,
    u64,
    Option<nxtools_protocol::archive_job::ArchiveJobAddResult>,
);

fn additions<W: Write + Send>(
    request: &WorkerRequest,
    native: &mut PrivateOutput<'_, W>,
    control: &OperationControl,
    snapshot: &crate::ArchiveSnapshot,
    interaction: &InteractionPort,
) -> Result<AdditionPlan> {
    let shared = RefCell::new(native);
    let mut source = Sources::new(&shared);
    let limits = request.limits.into();
    let plan = IncomingPlan::build(&mut source, &limits, control)?;
    let prefix = request
        .job_modify
        .as_ref()
        .and_then(|value| value.value.as_deref())
        .unwrap_or("");
    let (selected, skipped) = add::resolve(
        &plan.entries,
        snapshot.entries().iter().map(|entry| ExistingMember {
            name: entry.display_path(),
            directory: entry.kind() == crate::EntryKind::Directory,
            size: entry.size(),
        }),
        prefix,
        request.conflict_policy,
        &mut shared.borrow_mut(),
        interaction,
        &limits,
    )?;
    let deleted: Vec<_> = selected
        .iter()
        .filter_map(|item| item.replaces)
        .map(|index| snapshot.entries()[index].id())
        .collect();
    if deleted.len() > 4096 {
        return Err(ArchiveError::ResourceLimit);
    }
    let edits = if deleted.is_empty() {
        Vec::new()
    } else {
        vec![ZipEdit::Delete { entries: deleted }]
    };
    if selected.is_empty() {
        source.revalidate()?;
        return Ok((None, edits, skipped, None));
    }
    let add_result = add::display_result(&plan.entries, prefix, &selected);
    let failure = Cell::new(None);
    let mut encoded =
        crate::worker_tree_io::Spool::new(limits.max_output_bytes, control, &failure)?;
    {
        let mut zip = zip::ZipWriter::new(&mut encoded);
        for selected in selected {
            let item = &plan.entries[selected.input];
            let name = selected.name;
            control.checkpoint()?;
            let options = addition_options(item.is_directory(), item.size());
            zip.start_file(&name, options).map_err(ArchiveError::from)?;
            if !item.is_directory() {
                source.open_file(item.source.id)?;
                let result = (|| {
                    let mut offset = 0u64;
                    loop {
                        control.checkpoint()?;
                        let bytes = source.read_file(item.source.id, offset, 65536)?;
                        if bytes.is_empty() {
                            break;
                        }
                        offset = offset
                            .checked_add(bytes.len() as u64)
                            .ok_or(ArchiveError::ResourceLimit)?;
                        if offset > item.size() {
                            return Err(ArchiveError::SourceChanged);
                        }
                        zip.write_all(&bytes)?;
                    }
                    if offset != item.size() {
                        return Err(ArchiveError::SourceChanged);
                    }
                    Ok(())
                })();
                let close = source.close_file(item.source.id);
                result?;
                close?;
            }
        }
        zip.finish().map_err(ArchiveError::from)?;
    }
    encoded.flush().map_err(crate::worker_tree_io::io_error)?;
    if let Some(error) = failure.get() {
        return Err(error);
    }
    source.revalidate()?;
    encoded.file.rewind()?;
    Ok((Some(encoded.file), edits, skipped, Some(add_result)))
}
pub(crate) fn execute<W: Write + Send>(
    request: &WorkerRequest,
    snapshot: crate::ArchiveSnapshot,
    control: &OperationControl,
    writer: &mut W,
    replies: ReplyPort,
    interaction: &InteractionPort,
) -> Result<(u64, u64)> {
    let intent = request
        .job_modify
        .as_ref()
        .ok_or(ArchiveError::UnsupportedOperation)?;
    let limits = request.limits.into();
    let mut native = PrivateOutput::new(writer, control, replies);
    native.source_checkpoint("edit_generating")?;
    let engine = ArchiveEngine::with_limits(limits);
    let engine = if let Some(password) = request.password.as_deref() {
        engine.with_password(password)
    } else {
        engine
    };
    let expected = &intent.browser.source;
    let modified = u64::try_from(
        snapshot
            .revision
            .modified
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ArchiveError::SourceChanged)?
            .as_nanos(),
    )
    .map_err(|_| ArchiveError::SourceChanged)?;
    if intent.browser.format != "Zip"
        || snapshot.format() != crate::ArchiveFormat::Zip
        || expected.digest != snapshot.revision.digest
        || expected.size != snapshot.revision.size
        || expected.modified_unix_ns != modified
        || native.source_identity()? != expected.native
    {
        return Err(ArchiveError::SourceChanged);
    }
    // The intent-aware candidate owns structural edit admission, not browse capabilities.
    if !snapshot.revision.volumes.is_empty() || crate::zip_edit::blocked_container(&snapshot) {
        return Err(ArchiveError::UnsupportedOperation);
    }
    if !snapshot.backend_safe {
        return Err(ArchiveError::OutputCollision);
    }
    let ids = intent
        .ordinals
        .iter()
        .map(|ordinal| {
            snapshot
                .entries()
                .get(*ordinal as usize)
                .filter(|entry| entry.id().ordinal == *ordinal)
                .map(|entry| entry.id())
                .ok_or(ArchiveError::SourceChanged)
        })
        .collect::<Result<Vec<_>>>()?;
    // Resolve semantic intent against immutable member kind before additions or output.
    // ReplaceContent has no write backend and cannot be disguised as Add/rename.
    use nxtools_protocol::archive_job::ArchiveOperation;
    if matches!(
        intent.operation(),
        ArchiveOperation::RenameFile | ArchiveOperation::RenameDirectorySubtree
    ) {
        let entry = ids
            .first()
            .and_then(|id| snapshot.entries().get(id.ordinal as usize))
            .ok_or(ArchiveError::SourceChanged)?;
        if (intent.operation() == ArchiveOperation::RenameDirectorySubtree)
            != (entry.kind() == crate::EntryKind::Directory)
        {
            return Err(ArchiveError::UnsupportedOperation);
        }
    }
    let mut edits = match intent.action {
        ArchiveJobModifyAction::ReplaceContent => return Err(ArchiveError::UnsupportedOperation),
        ArchiveJobModifyAction::Add => Vec::new(),
        ArchiveJobModifyAction::Delete => vec![ZipEdit::Delete { entries: ids }],
        ArchiveJobModifyAction::Rename => vec![ZipEdit::Rename {
            entry: *ids.first().ok_or(ArchiveError::SourceChanged)?,
            name: intent
                .value
                .clone()
                .ok_or(ArchiveError::UnsupportedOperation)?,
        }],
        ArchiveJobModifyAction::NewDirectory => vec![ZipEdit::NewDirectory {
            name: intent
                .value
                .clone()
                .ok_or(ArchiveError::UnsupportedOperation)?,
        }],
        ArchiveJobModifyAction::Comment => vec![ZipEdit::Comment {
            text: intent
                .value
                .clone()
                .ok_or(ArchiveError::UnsupportedOperation)?,
        }],
    };
    let (mut added, addition_edits, skipped, add_result) =
        if intent.action == ArchiveJobModifyAction::Add {
            additions(request, &mut native, control, &snapshot, interaction)?
        } else {
            (None, Vec::new(), 0, None)
        };
    edits.extend(addition_edits);
    if intent.action == ArchiveJobModifyAction::Add && added.is_none() {
        engine
            .modification_eligibility(&snapshot, control)
            .map_err(|reason| match reason {
                crate::ArchiveModificationRefusal::SourceChanged => ArchiveError::SourceChanged,
                crate::ArchiveModificationRefusal::Corrupt => ArchiveError::CorruptArchive,
                crate::ArchiveModificationRefusal::UnsupportedEncryption => {
                    ArchiveError::UnsupportedEncryption
                }
                crate::ArchiveModificationRefusal::SourceUnavailable => ArchiveError::IoFailure,
                _ => ArchiveError::UnsupportedOperation,
            })?;
        let mut source = File::open(&request.path)?;
        crate::engine::ensure_revision(&mut source, &snapshot.revision, control)?;
        drop(source);
        native.member_summary(skipped, false)?;
        return Ok((0, 0));
    }
    let mut candidate = engine.zip_edit_candidate(&snapshot, &edits, added.as_mut(), control)?;
    // zip_edit_candidate's original File/Zip readers are closed on return; added is encoded only.
    drop(added);
    native.source_checkpoint("edit_verifying")?;
    let name = std::path::Path::new(&request.path)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(ArchiveError::UnsafePath)?;
    let token = native.begin_file(name)?;
    let result = (|| {
        let mut bytes = [0u8; 65536];
        let mut length = 0u64;
        let mut digest = Sha256::new();
        loop {
            control.checkpoint()?;
            let count = candidate.read(&mut bytes)?;
            if count == 0 {
                break;
            }
            length = length
                .checked_add(count as u64)
                .ok_or(ArchiveError::ResourceLimit)?;
            if length > request.limits.max_output_bytes {
                return Err(ArchiveError::ResourceLimit);
            }
            digest.update(&bytes[..count]);
            native.write_file(token, &bytes[..count])?;
        }
        candidate.rewind()?;
        let expected_hash: [u8; 32] = digest.finalize().into();
        let mut readback = tempfile::tempfile()?;
        let mut offset = 0u64;
        let mut actual = Sha256::new();
        loop {
            control.checkpoint()?;
            let bytes = native.read_partial(token, offset, 65536)?;
            if bytes.is_empty() {
                break;
            }
            offset = offset
                .checked_add(bytes.len() as u64)
                .ok_or(ArchiveError::ResourceLimit)?;
            if offset > length {
                return Err(ArchiveError::CorruptArchive);
            }
            actual.update(&bytes);
            readback.write_all(&bytes)?;
        }
        if offset != length || <[u8; 32]>::from(actual.finalize()) != expected_hash {
            return Err(ArchiveError::CorruptArchive);
        }
        // Full candidate validation was performed by the same original codec before token staging;
        // byte-exact held-token readback proves that complete verified encoded candidate survived.
        drop(readback);
        drop(candidate);
        drop(snapshot);
        control.checkpoint()?;
        native.call("edit_reads_complete", None, Some(token), &[])?;
        native.commit_file(token)?;
        control.finish_progress(crate::ArchiveProgress {
            bytes: length,
            files: 1,
            total_bytes: Some(length),
        });
        Ok((1, length))
    })();
    if result.is_err() {
        native.abort_file(token);
    }
    if result.is_ok() {
        if skipped > 0 {
            native.member_summary(skipped, false)?;
        }
        if let Some(add_result) = add_result {
            native.add_result(add_result)?;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::addition_options;
    use std::io::{Cursor, Write};

    #[test]
    fn add_zip64_reserves_local_header_before_payload() {
        for (size, zip64) in [
            (0, false),
            (1024, false),
            (u32::MAX as u64 - 1, true),
            (u32::MAX as u64, true),
            (32 << 30, true),
            (u64::MAX, true),
        ] {
            let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
            writer
                .start_file("incoming.txt", addition_options(false, size))
                .expect("start incoming ZIP record");
            writer.write_all(b"x").expect("write tiny boundary payload");
            let bytes = writer
                .finish()
                .expect("finish boundary archive")
                .into_inner();
            let name_len = u16::from_le_bytes([bytes[26], bytes[27]]) as usize;
            let extra_len = u16::from_le_bytes([bytes[28], bytes[29]]) as usize;
            let extra = &bytes[30 + name_len..30 + name_len + extra_len];
            assert_eq!(
                extra.starts_with(&[1, 0, 16, 0]),
                zip64,
                "ZIP64 local extra reservation for declared size {size}"
            );
            let mut archive =
                zip::ZipArchive::new(Cursor::new(bytes)).expect("read boundary archive");
            assert_eq!(
                archive.by_index(0).expect("incoming member").compression(),
                zip::CompressionMethod::Deflated
            );
        }
    }
}
