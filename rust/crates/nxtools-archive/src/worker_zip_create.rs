//! ZIP creation through the existing worker/native rendezvous. No filesystem source path reads.
use crate::{
    ArchiveError, ArchiveProgress, EntryKind, NameEncoding, OperationControl, Result,
    creation::{CreationOptions, CreationPlan, CreationSource},
    output::ArchiveOutput,
    worker::WorkerRequest,
    worker_output::{PrivateOutput, ReplyPort},
    worker_sources::{SharedNative, Sources},
};
use sha2::{Digest, Sha256};
use std::{
    cell::{Cell, RefCell},
    io::{Seek, Write},
};
use uuid::Uuid;

struct Sink<'b, 'a, 'n, W: Write> {
    native: &'n SharedNative<'b, 'a, W>,
    token: Uuid,
    failure: &'n Cell<Option<ArchiveError>>,
}
impl<W: Write + Send> Write for Sink<'_, '_, '_, W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.failure.get().is_none() {
            for chunk in bytes.chunks(65536) {
                if let Err(failure) = self.native.borrow_mut().write_file(self.token, chunk) {
                    self.failure.set(Some(failure));
                    break;
                }
            }
        }
        // Keep the backend finalizable after a native error; the retained failure is mandatory.
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn zip_error(failure: zip::result::ZipError) -> ArchiveError {
    match failure {
        zip::result::ZipError::Io(value) => crate::formats::zip::payload_error(value),
        _ => ArchiveError::CorruptArchive,
    }
}
pub(crate) fn create<W: Write + Send>(
    request: &WorkerRequest,
    control: &OperationControl,
    writer: &mut W,
    replies: ReplyPort,
) -> Result<(u64, u64)> {
    let lifecycle = control.source_delete_begin();
    let options = request
        .zip_options
        .as_ref()
        .ok_or(ArchiveError::UnsupportedOperation)?;
    let encrypted = options.encryption == "aes256";
    let limits = request.limits.into();
    let mut native = PrivateOutput::new(writer, control, replies);
    let shared = RefCell::new(&mut native);
    shared.borrow_mut().source_checkpoint("creation_planning")?;
    let mut sources = Sources::new(&shared);
    let plan = CreationPlan::build(
        &mut sources,
        &CreationOptions {
            strip_single_root: options.strip_single_root,
            exclude_patterns: options.exclude_patterns.clone(),
        },
        &limits,
        control,
    )?;
    shared.borrow_mut().source_checkpoint("source_validated")?;
    let token = shared.borrow_mut().begin_file(
        request
            .create_name
            .as_deref()
            .ok_or(ArchiveError::UnsupportedOperation)?,
    )?;
    let result = (|| {
        shared.borrow_mut().source_checkpoint("creation_encoding")?;
        let failure = Cell::new(None);
        let sink = Sink {
            native: &shared,
            token,
            failure: &failure,
        };
        let mut archive = zip::ZipWriter::new_stream(sink);
        let mut digests = Vec::<[u8; 32]>::with_capacity(plan.entries().len());
        let mut total = 0u64;
        let encoding = (|| {
            for item in plan.entries() {
                control.checkpoint()?;
                let stored = options.method == "store";
                let level = match request.compression_level.as_deref() {
                    Some("fast") => 1,
                    Some("normal") => 6,
                    Some("maximum") => 9,
                    Some("store") => 0,
                    _ => return Err(ArchiveError::UnsupportedOperation),
                };
                // Deflate worst-case expansion plus encryption/header allowance; never truncate.
                let large = item
                    .size()
                    .checked_add(item.size() / 1000)
                    .and_then(|value| value.checked_add(65536))
                    .is_none_or(|value| value >= u32::MAX as u64);
                let mut settings = zip::write::FileOptions::<()>::default()
                    .compression_method(if stored {
                        zip::CompressionMethod::Stored
                    } else {
                        zip::CompressionMethod::Deflated
                    })
                    .compression_level(if stored { None } else { Some(level) })
                    .large_file(large);
                if let Some((date, time)) = item.source.dos_modified {
                    settings = settings.last_modified_time(
                        zip::DateTime::try_from_msdos(date, time)
                            .map_err(|_| ArchiveError::SourceChanged)?,
                    );
                }
                if encrypted && !item.is_directory() {
                    settings = settings.with_aes_encryption(
                        zip::AesMode::Aes256,
                        request
                            .password
                            .as_deref()
                            .ok_or(ArchiveError::PasswordRequired)?,
                    );
                }
                if item.is_directory() {
                    archive
                        .start_file(
                            item.relative_path(),
                            settings
                                .compression_method(zip::CompressionMethod::Stored)
                                .compression_level(None),
                        )
                        .map_err(zip_error)?;
                    digests.push(Sha256::digest([]).into());
                } else {
                    sources.open_file(item.source.id)?;
                    let read_result = (|| {
                        archive
                            .start_file(item.relative_path(), settings)
                            .map_err(zip_error)?;
                        let mut offset = 0u64;
                        let mut digest = Sha256::new();
                        loop {
                            control.checkpoint()?;
                            let bytes = sources.read_file(item.source.id, offset, 65536)?;
                            if bytes.is_empty() {
                                break;
                            }
                            offset = offset
                                .checked_add(bytes.len() as u64)
                                .ok_or(ArchiveError::ResourceLimit)?;
                            total = total
                                .checked_add(bytes.len() as u64)
                                .ok_or(ArchiveError::ResourceLimit)?;
                            if offset > item.size() {
                                return Err(ArchiveError::SourceChanged);
                            }
                            if total > request.limits.max_output_bytes {
                                return Err(ArchiveError::ResourceLimit);
                            }
                            digest.update(&bytes);
                            archive
                                .write_all(&bytes)
                                .map_err(crate::formats::zip::payload_error)?;
                            if let Some(value) = failure.get() {
                                return Err(value);
                            }
                            control.publish(ArchiveProgress {
                                bytes: total,
                                files: 0,
                                total_bytes: Some(plan.total_bytes()),
                            });
                        }
                        if offset != item.size() {
                            return Err(ArchiveError::SourceChanged);
                        }
                        Ok(digest.finalize().into())
                    })();
                    let close = sources.close_file(item.source.id);
                    digests.push(read_result?);
                    close?;
                }
                if let Some(value) = failure.get() {
                    return Err(value);
                }
            }
            Ok(())
        })();
        // Explicit finish even after source/native failure: no implicit destructor result.
        let finished = archive.finish().map_err(zip_error);
        encoding?;
        finished?;
        if let Some(value) = failure.get() {
            return Err(value);
        }
        shared
            .borrow_mut()
            .source_checkpoint("creation_verifying")?;
        let mut spool = tempfile::tempfile()?;
        let mut offset = 0u64;
        loop {
            control.checkpoint()?;
            let bytes = shared.borrow_mut().read_partial(token, offset, 65536)?;
            if bytes.is_empty() {
                break;
            }
            offset = offset
                .checked_add(bytes.len() as u64)
                .ok_or(ArchiveError::ResourceLimit)?;
            if offset > request.limits.max_output_bytes {
                return Err(ArchiveError::ResourceLimit);
            }
            spool.write_all(&bytes)?;
        }
        spool.flush()?;
        spool.rewind()?;
        let (entries, comment, safe) =
            crate::formats::zip::list(&mut spool, NameEncoding::Auto, &limits, control)?;
        if !safe || !comment.is_empty() || entries.len() != plan.entries().len() {
            return Err(ArchiveError::CorruptArchive);
        }
        crate::formats::zip::preflight_created(&mut spool, &entries, &limits, control)?;
        let mut verified = 0u64;
        for ((entry, item), expected) in entries.iter().zip(plan.entries()).zip(digests) {
            if entry.display_path() != item.relative_path()
                || entry.size() != Some(item.size())
                || (entry.kind() == EntryKind::Directory) != item.is_directory()
                || entry.encrypted() != (encrypted && !item.is_directory())
                || entry.method()
                    != if item.is_directory() {
                        0
                    } else if encrypted {
                        99
                    } else if options.method == "store" {
                        0
                    } else {
                        8
                    }
                || encrypted
                    && !item.is_directory()
                    && entry
                        .zip
                        .as_ref()
                        .and_then(|location| location.aes)
                        .is_none_or(|aes| {
                            aes.version != 2
                                || aes.strength != 3
                                || aes.method != if options.method == "store" { 0 } else { 8 }
                        })
            {
                return Err(ArchiveError::CorruptArchive);
            }
            let mut actual = Sha256::new();
            crate::formats::zip::stream_entry(
                &mut spool,
                entry,
                &limits,
                control,
                request.password.as_deref(),
                &mut verified,
                |bytes| {
                    actual.update(bytes);
                    Ok(())
                },
            )?;
            if <[u8; 32]>::from(actual.finalize()) != expected {
                return Err(ArchiveError::CorruptArchive);
            }
        }
        shared.borrow_mut().source_checkpoint("source_precommit")?;
        sources.revalidate()?;
        control.checkpoint()?;
        if request.volume_bytes.is_some() {
            shared.borrow_mut().abort_file(token);
            spool.rewind()?;
            crate::worker_volume_output::publish(
                &mut spool,
                request,
                &mut shared.borrow_mut(),
                control,
            )?;
        } else {
            shared.borrow_mut().commit_file(token)?;
            plan.record_published_verification(control, &lifecycle);
        }
        control.finish_progress(ArchiveProgress {
            bytes: total,
            files: 1,
            total_bytes: Some(total),
        });
        Ok((
            plan.entries()
                .iter()
                .filter(|item| !item.is_directory())
                .count() as u64,
            total,
        ))
    })();
    if result.is_err() {
        shared.borrow_mut().abort_file(token);
    }
    result
}
