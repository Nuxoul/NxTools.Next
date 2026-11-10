//! Retained 7z catalog; the public consuming decoder makes one full block pass per demand.
use super::*;
use crate::worker_drag::DragPort;
use std::sync::Arc;

#[allow(clippy::too_many_arguments)]
pub(super) fn execute<W: Write + Send>(
    request: &WorkerRequest,
    control: &OperationControl,
    output: &mut W,
    replies: ReplyPort,
    drag: &DragPort,
    set: &volumes::SourceSet,
    reader: sevenz_rust2::ArchiveReader<File>,
    entries: Arc<[ArchiveEntry]>,
    decoder_ordinals: Option<Vec<usize>>,
) -> Result<(u64, u64)> {
    let archive = reader.archive();
    let password = request
        .password
        .as_deref()
        .map(sevenz_rust2::Password::from)
        .unwrap_or_default();
    if request.password.is_none() && entries.iter().any(|entry| entry.encrypted) {
        return Err(ArchiveError::PasswordRequired);
    }
    let mut source = set.sevenz_input(control)?;
    set.verify(control)?;
    crate::worker_drag::run_codec(
        request,
        control,
        output,
        replies,
        drag,
        |current, demand, native| {
            let selected = crate::worker_browser::selected(current, &entries)?
                .ok_or(ArchiveError::SourceChanged)?;
            if selected.id().ordinal != demand.ordinal {
                return Err(ArchiveError::SourceChanged);
            }
            let index = usize::try_from(demand.ordinal).map_err(|_| ArchiveError::SourceChanged)?;
            let decoded_index = if let Some(ordinals) = &decoder_ordinals {
                ordinals
                    .iter()
                    .position(|&ordinal| ordinal == index)
                    .ok_or(ArchiveError::SourceChanged)?
            } else {
                index
            };
            let raw = archive
                .files
                .get(decoded_index)
                .ok_or(ArchiveError::SourceChanged)?;
            let block = archive
                .stream_map
                .file_block_index
                .get(decoded_index)
                .ok_or(ArchiveError::CorruptArchive)?
                .to_owned();
            let relative = selected.display_path.replace('\\', "/");
            native.source_checkpoint("source_decoding")?;
            let mut pending = None;
            let mut written = 0u64;
            let mut block_bytes = 0u64;
            let decode_limits = crate::worker_browser::decode_limits(request);
            let result = (|| -> Result<()> {
                if let Some(block_index) = block {
                    let start = *archive
                        .stream_map
                        .block_first_file_index
                        .get(block_index)
                        .ok_or(ArchiveError::CorruptArchive)?;
                    let decoder = sevenz_rust2::BlockDecoder::new(
                        1,
                        block_index,
                        archive,
                        &password,
                        &mut source,
                    );
                    let count = decoder.entry_count();
                    if decoded_index < start
                        || decoded_index
                            >= start
                                .checked_add(count)
                                .ok_or(ArchiveError::CorruptArchive)?
                    {
                        return Err(ArchiveError::CorruptArchive);
                    }
                    let mut cursor = start;
                    let mut failure = None;
                    control.record_work(|work| work.sevenz_block_decodes += 1);
                    let complete = decoder.for_each_entries(&mut |file, input| {
                        let outcome: Result<()> = (|| {
                            control.checkpoint()?;
                            let current_index = decoder_ordinals
                                .as_ref()
                                .map_or(cursor, |ordinals| ordinals[cursor]);
                            let item = entries
                                .get(current_index)
                                .ok_or(ArchiveError::CorruptArchive)?;
                            cursor += 1;
                            if file.name != item.display_path
                                || file.size != item.size.unwrap_or(0)
                                || file.is_directory != (item.kind == EntryKind::Directory)
                            {
                                return Err(ArchiveError::SourceChanged);
                            }
                            let mut bytes = 0u64;
                            let mut crc = crc32fast::Hasher::new();
                            let mut buffer = zeroize::Zeroizing::new([0u8; 65536]);
                            if current_index == index {
                                pending = native.begin_extracted_file(&relative, item.size)?;
                                if pending.is_none() {
                                    return Err(ArchiveError::OutputCollision);
                                }
                            }
                            loop {
                                control.checkpoint()?;
                                let read = input.read(&mut buffer[..]).map_err(|_| {
                                    if request.password.is_some() {
                                        ArchiveError::AuthenticationOrDataError
                                    } else {
                                        ArchiveError::CorruptArchive
                                    }
                                })?;
                                if read == 0 {
                                    break;
                                }
                                bytes = bytes
                                    .checked_add(read as u64)
                                    .ok_or(ArchiveError::ResourceLimit)?;
                                block_bytes = block_bytes
                                    .checked_add(read as u64)
                                    .ok_or(ArchiveError::ResourceLimit)?;
                                if bytes > decode_limits.max_entry_bytes
                                    || block_bytes > decode_limits.max_output_bytes
                                    || current_index == index
                                        && (bytes > current.limits.max_entry_bytes
                                            || bytes > current.limits.max_output_bytes)
                                {
                                    return Err(ArchiveError::ResourceLimit);
                                }
                                crc.update(&buffer[..read]);
                                if current_index == index {
                                    native.write_file(
                                        pending.ok_or(ArchiveError::IoFailure)?,
                                        &buffer[..read],
                                    )?;
                                }
                            }
                            if bytes != file.size
                                || item
                                    .crc32
                                    .is_some_and(|expected| crc.finalize() != expected)
                            {
                                return Err(if request.password.is_some() {
                                    ArchiveError::AuthenticationOrDataError
                                } else {
                                    ArchiveError::CorruptArchive
                                });
                            }
                            if current_index == index {
                                written = bytes;
                            }
                            Ok(())
                        })();
                        match outcome {
                            Ok(()) => Ok(true),
                            Err(error) => {
                                failure = Some(error);
                                Ok(false)
                            }
                        }
                    });
                    if let Some(error) = failure {
                        return Err(error);
                    }
                    if !complete.map_err(sevenz_error)? || cursor != start + count {
                        return Err(ArchiveError::CorruptArchive);
                    }
                } else {
                    // Files without a block are the reader's trailing zero-stream records.
                    if raw.has_stream || raw.size != 0 || selected.size != Some(0) {
                        return Err(ArchiveError::CorruptArchive);
                    }
                    pending = native.begin_extracted_file(&relative, selected.size)?;
                    if pending.is_none() {
                        return Err(ArchiveError::OutputCollision);
                    }
                }
                set.verify(control)?;
                native.source_checkpoint("source_precommit")?;
                let token = pending.ok_or(ArchiveError::CorruptArchive)?;
                selected.metadata.validate()?;
                native.set_file_metadata(token, selected.metadata)?;
                native.commit_file(token)?;
                pending = None;
                Ok(())
            })();
            if result.is_err()
                && let Some(token) = pending
            {
                native.abort_file(token);
            }
            result.map(|()| written)
        },
    )
}
