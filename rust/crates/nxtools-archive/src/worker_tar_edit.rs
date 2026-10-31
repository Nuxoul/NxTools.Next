//! TAR edits retain the original source/output/native-publication rendezvous.
use crate::{
    ArchiveError, ArchiveProgress, OperationControl, ResourceLimits, Result,
    creation::{CreationOptions, CreationPlan, CreationSource},
    interaction::{ConflictDecision, InteractionPort, Prompt, PromptResponse},
    output::ArchiveOutput,
    tar_edit::{self, Member},
    worker::WorkerRequest,
    worker_conflict::{ConflictPolicy, ConflictPrompt},
    worker_output::{PrivateOutput, ReplyPort},
    worker_sources::{Reader, Sources},
    worker_tree_io::{Spool, io_error, stage_and_readback},
};
use nxtools_protocol::archive_job::ArchiveJobModifyAction as Action;
use sha2::{Digest, Sha256};
use std::{
    cell::{Cell, RefCell},
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    time::UNIX_EPOCH,
};

fn stamp(path: &str, control: &OperationControl) -> Result<([u8; 32], u64, u64)> {
    let mut file = File::open(path)?;
    let before = file.metadata()?;
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        control.checkpoint()?;
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    let after = file.metadata()?;
    if before.len() != after.len() || before.modified()? != after.modified()? {
        return Err(ArchiveError::SourceChanged);
    }
    Ok((
        hash.finalize().into(),
        after.len(),
        u64::try_from(
            after
                .modified()?
                .duration_since(UNIX_EPOCH)
                .map_err(|_| ArchiveError::SourceChanged)?
                .as_nanos(),
        )
        .map_err(|_| ArchiveError::SourceChanged)?,
    ))
}
fn payload(
    input: &mut impl Read,
    size: u64,
    output: &mut impl Write,
    total: &Cell<u64>,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<[u8; 32]> {
    let mut remaining = size;
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    while remaining > 0 {
        control.checkpoint()?;
        let wanted = remaining.min(buffer.len() as u64) as usize;
        let n = input.read(&mut buffer[..wanted]).map_err(io_error)?;
        if n == 0 {
            return Err(ArchiveError::CorruptArchive);
        }
        let count = total
            .get()
            .checked_add(n as u64)
            .ok_or(ArchiveError::ResourceLimit)?;
        if size > limits.max_entry_bytes || count > limits.max_output_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
        total.set(count);
        output.write_all(&buffer[..n]).map_err(io_error)?;
        hash.update(&buffer[..n]);
        remaining -= n as u64;
        control.publish(ArchiveProgress {
            bytes: count,
            files: 0,
            total_bytes: None,
        });
    }
    Ok(hash.finalize().into())
}
fn verify(
    file: &mut File,
    plan: &[Member],
    hashes: &[[u8; 32]],
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<()> {
    let actual = tar_edit::inspect(file, limits, control)?;
    if actual.len() != plan.len() || hashes.len() != plan.len() {
        return Err(ArchiveError::CorruptArchive);
    }
    let total = Cell::new(0);
    for ((a, b), digest) in actual.iter().zip(plan).zip(hashes) {
        control.checkpoint()?;
        if a.name != b.name
            || a.size != b.size
            || a.directory != b.directory
            || a.header != b.header
            || a.extensions != b.extensions
        {
            return Err(ArchiveError::CorruptArchive);
        }
        file.seek(SeekFrom::Start(a.offset))?;
        if payload(file, a.size, &mut std::io::sink(), &total, limits, control)? != *digest {
            return Err(ArchiveError::CorruptArchive);
        }
    }
    Ok(())
}
pub(crate) fn execute<W: Write + Send>(
    request: &WorkerRequest,
    control: &OperationControl,
    output: &mut W,
    replies: ReplyPort,
    interaction: &InteractionPort,
) -> Result<(u64, u64)> {
    let intent = request
        .job_modify
        .as_ref()
        .ok_or(ArchiveError::UnsupportedOperation)?;
    if !matches!(intent.browser.format.as_str(), "Tar" | "TarGz" | "TarXz")
        || request.volume_paths.len() > 1
        || request.password.is_some()
    {
        return Err(ArchiveError::UnsupportedOperation);
    }
    let limits: ResourceLimits = request.limits.into();
    let original = stamp(&request.path, control)?;
    if original
        != (
            intent.browser.source.digest,
            intent.browser.source.size,
            intent.browser.source.modified_unix_ns,
        )
    {
        return Err(ArchiveError::SourceChanged);
    }
    let mut native = PrivateOutput::new(output, control, replies);
    native.source_checkpoint("edit_planning")?;
    native.edit_stage("edit_planning")?;
    if native.source_identity()? != intent.browser.source.native {
        return Err(ArchiveError::SourceChanged);
    }
    native.edit_stage("edit_reading")?;
    let (outer, mut source) = crate::tar_edit_outer::Outer::open(
        File::open(&request.path)?,
        &intent.browser.format,
        &limits,
        control,
    )?;
    let old = tar_edit::inspect(&mut source, &limits, control)?;
    let mut plan = tar_edit::plan(&old, intent, &limits, control)?;
    let shared = RefCell::new(&mut native);
    let mut sources = Sources::new(&shared);
    let additions = if matches!(intent.action, Action::Add | Action::ReplaceContent) {
        Some(CreationPlan::build(
            &mut sources,
            &CreationOptions {
                strip_single_root: false,
                exclude_patterns: vec![],
            },
            &limits,
            control,
        )?)
    } else {
        None
    };
    let mut skipped = 0;
    let mut remembered = None;
    if let Some(additions) = &additions {
        if intent.action == Action::ReplaceContent {
            if additions.entries().len() != 1 || additions.entries()[0].is_directory() {
                return Err(ArchiveError::UnsupportedOperation);
            }
            for item in &mut plan {
                if item.added == Some(0) {
                    item.change(&item.name.clone(), additions.entries()[0].size())?;
                }
            }
        } else {
            for (index, item) in additions.entries().iter().enumerate() {
                control.checkpoint()?;
                let name = format!(
                    "{}{}{}",
                    intent.value.as_deref().unwrap_or(""),
                    item.relative_path().trim_end_matches('/'),
                    if item.is_directory() { "/" } else { "" }
                );
                let mut added = Member::new(name, item.is_directory(), item.size())?;
                added.added = Some(index);
                if let Some(at) = plan.iter().position(|e| {
                    e.name.trim_end_matches('/').to_uppercase()
                        == added.name.trim_end_matches('/').to_uppercase()
                }) {
                    if plan[at].directory && added.directory {
                        continue;
                    }
                    if plan[at].directory != added.directory {
                        return Err(ArchiveError::OutputCollision);
                    }
                    let decision = if let Some(value) = remembered {
                        value
                    } else {
                        match request.conflict_policy.unwrap_or(ConflictPolicy::Ask) {
                            ConflictPolicy::Overwrite => ConflictDecision::Overwrite,
                            ConflictPolicy::Skip => ConflictDecision::Skip,
                            ConflictPolicy::AutoRename => ConflictDecision::AutoRename,
                            ConflictPolicy::Ask => {
                                let response = interaction.request_notified(
                                    Prompt::Conflict {
                                        existing_bytes: Some(plan[at].size),
                                        incoming_bytes: Some(added.size),
                                    },
                                    control,
                                    |pending| {
                                        shared.borrow_mut().conflict_prompt(&ConflictPrompt {
                                            ticket: pending.ticket,
                                            existing_bytes: Some(plan[at].size),
                                            incoming_bytes: Some(added.size),
                                            display: crate::worker_conflict::edit_display(
                                                &added.name,
                                                &plan[at].name,
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
                        }
                    };
                    match decision {
                        ConflictDecision::Overwrite => {
                            let name = plan[at].name.clone();
                            plan[at].change(&name, added.size)?;
                            plan[at].added = Some(index);
                            continue;
                        }
                        ConflictDecision::Skip => {
                            skipped += 1;
                            continue;
                        }
                        ConflictDecision::AutoRename => {
                            let base = added.name.clone();
                            let mut found = false;
                            for n in 2..=4097 {
                                control.checkpoint()?;
                                let name = crate::worker_conflict::numbered(&base, n)?;
                                let key = name.trim_end_matches('/').to_uppercase();
                                if !plan.iter().any(|e| {
                                    let existing = e.name.trim_end_matches('/').to_uppercase();
                                    existing == key || existing.starts_with(&format!("{key}/"))
                                }) {
                                    added.change(&name, added.size)?;
                                    found = true;
                                    break;
                                }
                            }
                            if !found {
                                return Err(ArchiveError::OutputCollision);
                            }
                        }
                    }
                }
                plan.push(added);
            }
        }
    }
    tar_edit::validate(&plan, &limits, control)?;
    if skipped > 0 && plan.iter().all(|e| e.added.is_none()) {
        sources.revalidate()?;
        if stamp(&request.path, control)? != original {
            return Err(ArchiveError::SourceChanged);
        }
        shared.borrow_mut().member_summary(skipped, false)?;
        return Ok((0, 0));
    }
    let failure = Cell::new(None);
    let sources = RefCell::new(sources);
    let digests = RefCell::new(vec![
        None;
        additions.as_ref().map_or(0, |p| p.entries().len())
    ]);
    shared.borrow_mut().source_checkpoint("edit_reencoding")?;
    shared.borrow_mut().edit_stage("edit_reencoding")?;
    let mut encoded = Spool::new(limits.max_output_bytes, control, &failure)?;
    let mut hashes = vec![[0; 32]; plan.len()];
    let total = Cell::new(0);
    // Decode every old payload even when deleted/replaced. No original reader survives handoff.
    for (index, item) in old.iter().enumerate() {
        control.checkpoint()?;
        source.seek(SeekFrom::Start(item.offset))?;
        if let Some((position, target)) = plan
            .iter()
            .enumerate()
            .find(|(_, e)| e.origin == Some(index) && e.added.is_none())
        {
            target.write_header(&mut encoded)?;
            hashes[position] = payload(
                &mut source,
                item.size,
                &mut encoded,
                &total,
                &limits,
                control,
            )?;
            tar_edit::padding(&mut encoded, item.size)?;
        } else {
            payload(
                &mut source,
                item.size,
                &mut std::io::sink(),
                &total,
                &limits,
                control,
            )?;
        }
    }
    // Retained originals stay in original order; replacement/new records follow deterministically.
    let mut ordered = Vec::with_capacity(plan.len());
    let mut ordered_hashes = Vec::with_capacity(plan.len());
    for (i, item) in plan.iter().enumerate() {
        if item.origin.is_some() && item.added.is_none() {
            ordered.push(item.clone());
            ordered_hashes.push(hashes[i]);
        }
    }
    for item in &plan {
        if item.origin.is_some() && item.added.is_none() {
            continue;
        }
        item.write_header(&mut encoded)?;
        let digest = if let Some(index) = item.added {
            if item.directory {
                Sha256::digest([]).into()
            } else {
                let additions = additions.as_ref().ok_or(ArchiveError::SourceChanged)?;
                let mut reader = Reader::new(
                    &sources,
                    &additions.entries()[index],
                    control,
                    &failure,
                    &digests,
                    index,
                );
                let result = payload(
                    &mut reader,
                    item.size,
                    &mut encoded,
                    &total,
                    &limits,
                    control,
                );
                if let Some(error) = failure.get() {
                    return Err(error);
                }
                let digest = result?;
                let mut eof = [0];
                if reader.read(&mut eof).map_err(io_error)? != 0 {
                    return Err(ArchiveError::SourceChanged);
                }
                digest
            }
        } else {
            Sha256::digest([]).into()
        };
        tar_edit::padding(&mut encoded, item.size)?;
        ordered.push(item.clone());
        ordered_hashes.push(digest);
    }
    tar_edit::finish(&mut encoded)?;
    if let Some(error) = failure.get() {
        return Err(error);
    }
    drop(source);
    shared.borrow_mut().source_checkpoint("edit_verifying")?;
    shared.borrow_mut().edit_stage("edit_verifying")?;
    verify(
        &mut encoded.file,
        &ordered,
        &ordered_hashes,
        &limits,
        control,
    )?;
    let mut encoded = outer.encode(encoded, &limits, control, &failure)?;
    if stamp(&request.path, control)? != original {
        return Err(ArchiveError::SourceChanged);
    }
    sources.borrow_mut().revalidate()?;
    let name = std::path::Path::new(&request.path)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or(ArchiveError::UnsafePath)?;
    let token = shared.borrow_mut().begin_file(name)?;
    let result = (|| {
        let readback = stage_and_readback(
            &mut encoded.file,
            &mut shared.borrow_mut(),
            token,
            limits.max_output_bytes,
            control,
        )?;
        let (actual_outer, mut readback) =
            crate::tar_edit_outer::Outer::open(readback, outer.format(), &limits, control)?;
        if actual_outer != outer {
            return Err(ArchiveError::CorruptArchive);
        }
        verify(&mut readback, &ordered, &ordered_hashes, &limits, control)?;
        drop(readback);
        let bytes = encoded.file.metadata()?.len();
        drop(encoded);
        shared.borrow_mut().source_checkpoint("source_precommit")?;
        if stamp(&request.path, control)? != original {
            return Err(ArchiveError::SourceChanged);
        }
        sources.borrow_mut().revalidate()?;
        control.checkpoint()?;
        shared.borrow_mut().edit_stage("edit_committing")?;
        shared
            .borrow_mut()
            .call("edit_reads_complete", None, Some(token), &[])?;
        shared.borrow_mut().commit_file(token)?;
        Ok((1, bytes))
    })();
    if result.is_err() {
        shared.borrow_mut().abort_file(token);
    } else {
        shared.borrow_mut().member_summary(skipped, skipped == 0)?;
    }
    result
}
