//! Single-thread 7z encoding from native source IDs; no final-path or plaintext source spool.
use crate::{
    ArchiveError, ArchiveProgress, OperationControl, ResourceLimits, Result,
    creation::{CreationPlan, CreationSource},
    output::ArchiveOutput,
    worker::WorkerRequest,
    worker_codecs::{sevenz_budget, sevenz_error},
    worker_output::{PrivateOutput, ReplyPort},
    worker_sources::{Reader, Sources},
    worker_tree_io::{Spool, io_error, stage_and_readback},
};
use sha2::{Digest, Sha256};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    fs::File,
    io::{Read, Seek, Write},
};

pub(crate) fn create<W: Write + Send>(
    request: &WorkerRequest,
    control: &OperationControl,
    writer: &mut W,
    replies: ReplyPort,
) -> Result<(u64, u64)> {
    create_with_reader(request, control, writer, replies, |source| source)
}

// Private seam: production retains the same File; tests can fault that exact verifier input.
fn create_with_reader<W: Write + Send, R: Read + Seek>(
    request: &WorkerRequest,
    control: &OperationControl,
    writer: &mut W,
    replies: ReplyPort,
    reader: impl FnOnce(File) -> R,
) -> Result<(u64, u64)> {
    let lifecycle = control.source_delete_begin();
    let options = request
        .sevenz_options
        .as_ref()
        .ok_or(ArchiveError::UnsupportedOperation)?;
    let tree = request
        .tree_options
        .as_ref()
        .ok_or(ArchiveError::UnsupportedOperation)?;
    let preset = match request.compression_level.as_deref() {
        Some("fast") => 1,
        Some("normal") => 6,
        Some("maximum") => 7,
        _ => return Err(ArchiveError::UnsupportedOperation),
    };
    // Header encoding always uses default LZMA preset6. Both phases are sequential, never MT.
    let dictionary = lzma_rust2::LzmaOptions::with_preset(preset)
        .dict_size
        .max(lzma_rust2::LzmaOptions::with_preset(6).dict_size);
    if u64::from(dictionary) * 16 + (32 << 20) > request.limits.max_working_bytes {
        return Err(ArchiveError::ResourceLimit);
    }
    let limits: ResourceLimits = request.limits.into();
    let mut native = PrivateOutput::new(writer, control, replies);
    let shared = RefCell::new(&mut native);
    shared.borrow_mut().source_checkpoint("creation_planning")?;
    let mut sources = Sources::new(&shared);
    let plan = CreationPlan::build(&mut sources, tree, &limits, control)?;
    // Preventive reservation covers backend entries/blocks, UTF16 names, both header buffers and readback metadata.
    let metadata = plan.entries().iter().try_fold(4u64 << 20, |sum, item| {
        sum.checked_add(8192 + item.relative_path().len() as u64 * 128)
            .ok_or(ArchiveError::ResourceLimit)
    })?;
    if metadata > limits.max_metadata_bytes {
        return Err(ArchiveError::ResourceLimit);
    }
    let sources = RefCell::new(sources);
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
        let digests = RefCell::new(vec![None; plan.entries().len()]);
        let spool = Spool::new(limits.max_output_bytes, control, &failure)?;
        let initialized = sevenz_rust2::ArchiveWriter::new(spool).map_err(sevenz_error);
        if let Some(error) = failure.get() {
            return Err(error);
        }
        let mut archive = initialized?;
        archive.set_encrypt_header(options.encrypt_header);
        let mut methods: Vec<sevenz_rust2::EncoderConfiguration> = Vec::new();
        if options.encryption == "aes256" {
            methods.push(
                sevenz_rust2::encoder_options::AesEncoderOptions::new(
                    request
                        .password
                        .as_deref()
                        .ok_or(ArchiveError::PasswordRequired)?
                        .into(),
                )
                .into(),
            );
        }
        methods.push(sevenz_rust2::encoder_options::Lzma2Options::from_level(preset).into());
        archive.set_content_methods(methods);
        let mut entries = Vec::new();
        let mut readers = Vec::new();
        for (index, item) in plan.entries().iter().enumerate() {
            control.checkpoint()?;
            let mut entry = if item.is_directory() {
                sevenz_rust2::ArchiveEntry::new_directory(
                    item.relative_path().trim_end_matches('/'),
                )
            } else {
                sevenz_rust2::ArchiveEntry::new_file(item.relative_path())
            };
            if item.is_directory() || item.size() == 0 {
                if !item.is_directory() {
                    // Even empty sources must be opened, checked for real EOF, and closed.
                    let mut source =
                        Reader::new(&sources, item, control, &failure, &digests, index);
                    let mut byte = [0u8; 1];
                    if source.read(&mut byte).map_err(io_error)? != 0 {
                        return Err(ArchiveError::SourceChanged);
                    }
                } else {
                    digests.borrow_mut()[index] = Some(Sha256::digest([]).into());
                }
                entry.has_stream = false;
                archive
                    .push_archive_entry::<std::io::Empty>(entry, None)
                    .map_err(sevenz_error)?;
            } else {
                let source = Reader::new(&sources, item, control, &failure, &digests, index);
                if options.solid {
                    entries.push(entry);
                    readers.push(sevenz_rust2::SourceReader::new(source));
                } else {
                    let encoded = archive
                        .push_archive_entry(entry, Some(source))
                        .map_err(sevenz_error);
                    if let Some(error) = failure.get() {
                        return Err(error);
                    }
                    encoded?;
                }
            }
        }
        if !entries.is_empty() {
            let encoded = archive
                .push_archive_entries(entries, readers)
                .map_err(sevenz_error);
            if let Some(error) = failure.get() {
                return Err(error);
            }
            encoded?;
        }
        let finished = archive.finish().map_err(io_error);
        if let Some(error) = failure.get() {
            return Err(error);
        }
        let mut encoded = finished?;
        let spool = stage_and_readback(
            &mut encoded.file,
            &mut shared.borrow_mut(),
            token,
            limits.max_output_bytes,
            control,
        )?;
        verify(reader(spool), &plan, &digests.borrow(), request, control)?;
        shared.borrow_mut().source_checkpoint("source_precommit")?;
        sources.borrow_mut().revalidate()?;
        control.checkpoint()?;
        if request.volume_bytes.is_some() {
            shared.borrow_mut().abort_file(token);
            crate::worker_volume_output::publish(
                &mut encoded.file,
                request,
                &mut shared.borrow_mut(),
                control,
            )?;
        } else {
            shared.borrow_mut().commit_file(token)?;
            plan.record_published_verification(control, &lifecycle);
        }
        control.finish_progress(ArchiveProgress {
            bytes: plan.total_bytes(),
            files: 1,
            total_bytes: Some(plan.total_bytes()),
        });
        Ok((
            plan.entries()
                .iter()
                .filter(|item| !item.is_directory())
                .count() as u64,
            plan.total_bytes(),
        ))
    })();
    if result.is_err() {
        shared.borrow_mut().abort_file(token);
    }
    result
}
// Record provenance before 7z parsing/decoding can wrap or discard the original I/O error.
// This is scoped to the owned creation-verification spool, not a codec-wide error policy.
struct VerificationInput<'a, R> {
    source: R,
    failure: &'a Cell<Option<ArchiveError>>,
    control: &'a OperationControl,
}
impl<R> VerificationInput<'_, R> {
    fn retain<T>(&self, result: std::io::Result<T>) -> std::io::Result<T> {
        result.map_err(|error| {
            // Read::read_exact retries Interrupted; it is not a terminal storage failure.
            if error.kind() == std::io::ErrorKind::Interrupted {
                return error;
            }
            let typed = error
                .get_ref()
                .and_then(|value| value.downcast_ref::<ArchiveError>())
                .copied()
                .unwrap_or(match error.kind() {
                    std::io::ErrorKind::UnexpectedEof => ArchiveError::CorruptArchive,
                    std::io::ErrorKind::OutOfMemory => ArchiveError::ResourceLimit,
                    _ => ArchiveError::IoFailure,
                });
            if self.failure.get().is_none() {
                self.failure.set(Some(typed));
            }
            std::io::Error::other(typed)
        })
    }
}
impl<R: Read> Read for VerificationInput<'_, R> {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        let result = self
            .control
            .checkpoint()
            .map_err(std::io::Error::other)
            .and_then(|()| self.source.read(bytes));
        self.retain(result)
    }
}
impl<R: Seek> Seek for VerificationInput<'_, R> {
    fn seek(&mut self, from: std::io::SeekFrom) -> std::io::Result<u64> {
        let result = self
            .control
            .checkpoint()
            .map_err(std::io::Error::other)
            .and_then(|()| self.source.seek(from));
        self.retain(result)
    }
}
fn verification_error(error: std::io::Error) -> ArchiveError {
    error
        .get_ref()
        .and_then(|value| value.downcast_ref::<ArchiveError>())
        .copied()
        .unwrap_or_else(|| {
            if error.kind() == std::io::ErrorKind::OutOfMemory {
                ArchiveError::ResourceLimit
            } else {
                ArchiveError::CorruptArchive
            }
        })
}
fn verification_codec_error(error: sevenz_rust2::Error) -> ArchiveError {
    match error {
        sevenz_rust2::Error::Io(error, _) => verification_error(error),
        other => sevenz_error(other),
    }
}
fn verify(
    source: impl Read + Seek,
    plan: &CreationPlan,
    digests: &[Option<[u8; 32]>],
    request: &WorkerRequest,
    control: &OperationControl,
) -> Result<()> {
    let failure = Cell::new(None);
    let source = VerificationInput {
        source,
        failure: &failure,
        control,
    };
    let result = verify_inner(source, plan, digests, request, control);
    // Includes header preflight, ArchiveReader construction, and callback/late block errors.
    // A callback's corruption projection must never erase a real underlying spool failure.
    failure.get().map_or(result, Err)
}
fn verify_inner(
    mut source: impl Read + Seek,
    plan: &CreationPlan,
    digests: &[Option<[u8; 32]>],
    request: &WorkerRequest,
    control: &OperationControl,
) -> Result<()> {
    let limits: ResourceLimits = request.limits.into();
    let source_length = source.seek(std::io::SeekFrom::End(0))?;
    source.rewind()?;
    let mut header = [0u8; 32];
    source.read_exact(&mut header).map_err(verification_error)?;
    let offset = u64::from_le_bytes(header[12..20].try_into().expect("fixed header"));
    let size = u64::from_le_bytes(header[20..28].try_into().expect("fixed header"));
    if &header[..6] != b"7z\xbc\xaf\x27\x1c"
        || crc32fast::hash(&header[12..32])
            != u32::from_le_bytes(header[8..12].try_into().expect("fixed CRC"))
        || offset
            .checked_add(size)
            .and_then(|value| value.checked_add(32))
            != Some(source_length)
    {
        return Err(ArchiveError::CorruptArchive);
    }
    sevenz_budget::encoded_header(&mut source, &header, &limits)?;
    source.rewind()?;
    let password = request
        .password
        .as_deref()
        .map(sevenz_rust2::Password::from)
        .unwrap_or_default();
    let mut reader =
        sevenz_rust2::ArchiveReader::new(source, password).map_err(verification_codec_error)?;
    reader.set_thread_count(1);
    let archive = reader.archive();
    sevenz_budget::blocks(archive, &limits)?;
    if archive.files.len() != plan.entries().len() {
        return Err(ArchiveError::CorruptArchive);
    }
    let expected: HashMap<_, _> = plan
        .entries()
        .iter()
        .enumerate()
        .map(|(index, item)| (item.relative_path().trim_end_matches('/'), (index, item)))
        .collect();
    let mut names = std::collections::HashSet::new();
    for entry in &archive.files {
        control.checkpoint()?;
        let (_, item) = expected
            .get(entry.name.as_str())
            .ok_or(ArchiveError::CorruptArchive)?;
        if !names.insert(entry.name.as_str())
            || entry.is_anti_item
            || entry.is_directory != item.is_directory()
            || entry.size != item.size()
        {
            return Err(ArchiveError::CorruptArchive);
        }
    }
    let mut seen = vec![false; plan.entries().len()];
    let mut first = None;
    let decoded = reader.for_each_entries(|entry, input| {
        if first.is_some() {
            return Ok(false);
        }
        let result = (|| {
            let (index, item) = expected
                .get(entry.name.as_str())
                .ok_or(ArchiveError::CorruptArchive)?;
            if seen[*index] {
                return Err(ArchiveError::CorruptArchive);
            }
            seen[*index] = true;
            let mut digest = Sha256::new();
            let mut crc = crc32fast::Hasher::new();
            let mut count = 0u64;
            let mut buffer = [0u8; 65536];
            loop {
                control.checkpoint()?;
                let read = input.read(&mut buffer).map_err(verification_error)?;
                if read == 0 {
                    break;
                }
                count = count
                    .checked_add(read as u64)
                    .ok_or(ArchiveError::ResourceLimit)?;
                if count > item.size() {
                    return Err(ArchiveError::CorruptArchive);
                }
                digest.update(&buffer[..read]);
                crc.update(&buffer[..read]);
            }
            if count != item.size()
                || digests[*index] != Some(digest.finalize().into())
                || entry.has_crc && u64::from(crc.finalize()) != entry.crc
            {
                return Err(ArchiveError::CorruptArchive);
            }
            Ok(())
        })();
        if let Err(error) = result {
            first = Some(error);
            return Ok(false);
        }
        Ok(true)
    });
    if let Some(error) = first {
        return Err(error);
    }
    decoded.map_err(verification_codec_error)?;
    if seen.iter().any(|value| !value) {
        return Err(ArchiveError::CorruptArchive);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
