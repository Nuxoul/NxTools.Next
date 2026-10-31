//! ZIP mutation through the original worker/native output rendezvous, never a final path write.
use crate::{
    ArchiveEngine, ArchiveError, OperationControl, Result,
    creation::{CreationOptions, CreationPlan, CreationSource},
    interaction::{ConflictDecision, InteractionPort, Prompt, PromptResponse},
    output::ArchiveOutput,
    worker::WorkerRequest,
    worker_conflict::{ConflictPolicy, ConflictPrompt},
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

fn additions<W: Write + Send>(
    request: &WorkerRequest,
    native: &mut PrivateOutput<'_, W>,
    control: &OperationControl,
    snapshot: &crate::ArchiveSnapshot,
    interaction: &InteractionPort,
) -> Result<(Option<File>, Vec<ZipEdit>, u64)> {
    let shared = RefCell::new(native);
    let mut source = Sources::new(&shared);
    let limits = request.limits.into();
    let plan = CreationPlan::build(
        &mut source,
        &CreationOptions {
            strip_single_root: false,
            exclude_patterns: Vec::new(),
        },
        &limits,
        control,
    )?;
    let prefix = request
        .job_modify
        .as_ref()
        .and_then(|value| value.value.as_deref())
        .unwrap_or("");
    let mut metadata = 0u64;
    for entry in snapshot.entries() {
        control.checkpoint()?;
        metadata = metadata
            .checked_add(256 + entry.display_path().len() as u64 * 8)
            .ok_or(ArchiveError::ResourceLimit)?;
        if metadata > limits.max_metadata_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
    }
    for entry in plan.entries() {
        control.checkpoint()?;
        metadata = metadata
            .checked_add(512 + (prefix.len() + entry.relative_path().len()) as u64 * 16)
            .ok_or(ArchiveError::ResourceLimit)?;
        if metadata > limits.max_metadata_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
    }
    let mut selected = Vec::new();
    let mut deleted = Vec::new();
    let mut skipped = 0u64;
    let mut remembered = None;
    let mut reserved = snapshot
        .entries()
        .iter()
        .map(|entry| {
            (
                entry.display_path().trim_end_matches('/').to_uppercase(),
                entry.kind() == crate::EntryKind::Directory,
            )
        })
        .collect::<Vec<_>>();
    // AutoRename must not consume a later source's original planned destination.
    reserved.extend(plan.entries().iter().map(|item| {
        (
            format!("{prefix}{}", item.relative_path().trim_end_matches('/')).to_uppercase(),
            item.is_directory(),
        )
    }));
    for item in plan.entries() {
        control.checkpoint()?;
        let mut name = format!(
            "{prefix}{}{}",
            item.relative_path().trim_end_matches('/'),
            if item.is_directory() { "/" } else { "" }
        );
        let key = name.trim_end_matches('/').to_uppercase();
        let existing = snapshot
            .entries()
            .iter()
            .find(|entry| entry.display_path().trim_end_matches('/').to_uppercase() == key);
        // A descendant already establishes this directory even without a ZIP directory record.
        // Merge only against the ORIGINAL snapshot, never the incoming/reserved source plan.
        if item.is_directory()
            && existing.is_none()
            && snapshot.entries().iter().any(|entry| {
                entry
                    .display_path()
                    .to_uppercase()
                    .starts_with(&format!("{key}/"))
            })
        {
            continue;
        }
        if let Some(existing) = existing {
            if existing.kind() == crate::EntryKind::Directory && item.is_directory() {
                continue;
            }
            if existing.kind() != crate::EntryKind::File || item.is_directory() {
                return Err(ArchiveError::OutputCollision);
            }
            let decision = if let Some(decision) = remembered {
                decision
            } else {
                match request.conflict_policy {
                    Some(ConflictPolicy::Overwrite) => ConflictDecision::Overwrite,
                    Some(ConflictPolicy::Skip) => ConflictDecision::Skip,
                    Some(ConflictPolicy::AutoRename) => ConflictDecision::AutoRename,
                    Some(ConflictPolicy::Ask) => {
                        let response = interaction.request_notified(
                            Prompt::Conflict {
                                existing_bytes: existing.size(),
                                incoming_bytes: Some(item.size()),
                            },
                            control,
                            |pending| {
                                shared.borrow_mut().conflict_prompt(&ConflictPrompt {
                                    ticket: pending.ticket,
                                    existing_bytes: existing.size(),
                                    incoming_bytes: Some(item.size()),
                                    display: crate::worker_conflict::edit_display(
                                        &name,
                                        existing.display_path(),
                                    ),
                                })
                            },
                        )?;
                        let PromptResponse::Conflict {
                            decision,
                            apply_to_all,
                        } = response
                        else {
                            return Err(ArchiveError::Cancelled);
                        };
                        if apply_to_all {
                            remembered = Some(decision);
                        }
                        decision
                    }
                    None => return Err(ArchiveError::OutputCollision),
                }
            };
            match decision {
                ConflictDecision::Skip => {
                    skipped += 1;
                    continue;
                }
                ConflictDecision::Overwrite => {
                    if deleted.len() >= 4096 {
                        return Err(ArchiveError::ResourceLimit);
                    }
                    deleted.push(existing.id());
                }
                ConflictDecision::AutoRename => {
                    let mut found = false;
                    for number in 2..=4097 {
                        control.checkpoint()?;
                        let candidate = crate::worker_conflict::numbered(&name, number)?;
                        let candidate_key = candidate.to_uppercase();
                        if !reserved.iter().any(|(old, directory)| {
                            old == &candidate_key
                                || old.starts_with(&format!("{candidate_key}/"))
                                || !directory && candidate_key.starts_with(&format!("{old}/"))
                        }) {
                            name = candidate;
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        return Err(ArchiveError::OutputCollision);
                    }
                }
            }
        } else if reserved.iter().any(|(old, directory)| {
            (!item.is_directory() && old.starts_with(&format!("{key}/")))
                || !directory && key.starts_with(&format!("{old}/"))
        }) {
            return Err(ArchiveError::OutputCollision);
        }
        reserved.push((
            name.trim_end_matches('/').to_uppercase(),
            item.is_directory(),
        ));
        selected.push((item, name));
    }
    let edits = if deleted.is_empty() {
        Vec::new()
    } else {
        vec![ZipEdit::Delete { entries: deleted }]
    };
    if selected.is_empty() {
        source.revalidate()?;
        return Ok((None, edits, skipped));
    }
    let failure = Cell::new(None);
    let mut encoded =
        crate::worker_tree_io::Spool::new(limits.max_output_bytes, control, &failure)?;
    {
        let mut zip = zip::ZipWriter::new(&mut encoded);
        for (item, name) in selected {
            control.checkpoint()?;
            let options = zip::write::SimpleFileOptions::default().compression_method(
                if item.is_directory() {
                    zip::CompressionMethod::Stored
                } else {
                    zip::CompressionMethod::Deflated
                },
            );
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
    Ok((Some(encoded.file), edits, skipped))
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
    snapshot
        .capabilities()
        .admit(intent.operation())
        .map_err(|reason| match reason {
            crate::ArchiveModificationRefusal::SourceChanged => ArchiveError::SourceChanged,
            crate::ArchiveModificationRefusal::Corrupt => ArchiveError::CorruptArchive,
            crate::ArchiveModificationRefusal::UnsupportedEncryption => {
                ArchiveError::UnsupportedEncryption
            }
            crate::ArchiveModificationRefusal::SourceUnavailable => ArchiveError::IoFailure,
            _ => ArchiveError::UnsupportedOperation,
        })?;
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
    let (mut added, addition_edits, skipped) = if intent.action == ArchiveJobModifyAction::Add {
        additions(request, &mut native, control, &snapshot, interaction)?
    } else {
        (None, Vec::new(), 0)
    };
    edits.extend(addition_edits);
    if intent.action == ArchiveJobModifyAction::Add && added.is_none() {
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
    if result.is_ok() && skipped > 0 {
        native.member_summary(skipped, false)?;
    }
    result
}
