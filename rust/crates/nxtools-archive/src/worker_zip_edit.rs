//! ZIP mutation through the original worker/native output rendezvous, never a final path write.
use crate::{
    ArchiveEngine, ArchiveError, OpenRequest, OperationControl, Result,
    creation::{CreationOptions, CreationPlan, CreationSource},
    output::ArchiveOutput,
    worker::WorkerRequest,
    worker_output::{PrivateOutput, ReplyPort},
    worker_sources::Sources,
    zip_edit::ZipEdit,
};
use nxtools_protocol::archive_job::ArchiveJobModifyAction;
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    fs::File,
    io::{Read, Seek, Write},
    time::UNIX_EPOCH,
};

fn additions<W: Write + Send>(
    request: &WorkerRequest,
    native: &mut PrivateOutput<'_, W>,
    control: &OperationControl,
) -> Result<File> {
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
    let mut encoded = tempfile::tempfile()?;
    {
        let mut zip = zip::ZipWriter::new(&mut encoded);
        for item in plan.entries() {
            control.checkpoint()?;
            let name = format!("{prefix}{}", item.relative_path());
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
    source.revalidate()?;
    encoded.rewind()?;
    Ok(encoded)
}
pub(crate) fn execute<W: Write + Send>(
    request: &WorkerRequest,
    control: &OperationControl,
    writer: &mut W,
    replies: ReplyPort,
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
    let snapshot = engine.open(
        OpenRequest {
            path: request.path.clone().into(),
            encoding: request
                .encoding()
                .map_err(|_| ArchiveError::UnsupportedOperation)?,
        },
        control,
    )?;
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
    let edits = match intent.action {
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
    let mut added = if intent.action == ArchiveJobModifyAction::Add {
        Some(additions(request, &mut native, control)?)
    } else {
        None
    };
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
    result
}
