//! Codecs with no public allocation ceiling run only in the C#-admitted worker.
//! This module is private and never selected by ArchiveEngine in the desktop process.
use crate::{
    ArchiveEntry, ArchiveError, EntryId, EntryKind, EntryWarning, NameProvenance, OperationControl,
    ResourceLimits, Result,
    output::ArchiveOutput,
    worker::WorkerRequest,
    worker_output::{PrivateOutput, ReplyPort},
};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Seek, Write},
    path::Path,
};

#[cfg(windows)]
mod rar;
pub(crate) mod sevenz_budget;
mod volumes;

pub(crate) fn try_execute<W: Write + Send>(
    request: &WorkerRequest,
    control: &OperationControl,
    output: &mut W,
    replies: ReplyPort,
    metadata: &crate::worker_metadata::MetadataCredit,
    interaction: &crate::interaction::InteractionPort,
) -> Option<Result<(u64, u64)>> {
    let result = (|| {
        // Never sniff a later volume or pass an unleased sibling to a codec.
        let first = request
            .volume_paths
            .first()
            .map_or(request.path.as_str(), String::as_str);
        let mut file = File::open(first)?;
        let mut signature = [0u8; 8];
        let count = file.read(&mut signature)?;
        if count >= 6 && &signature[..6] == b"7z\xbc\xaf\x27\x1c" {
            if request.version != 2 {
                return Err(ArchiveError::UnsupportedOperation);
            }
            codec_budget(request)?;
            return Ok(Some(run_sevenz(
                request,
                control,
                output,
                replies,
                metadata,
                interaction,
            )?));
        }
        #[cfg(windows)]
        if count >= 7 && &signature[..7] == b"Rar!\x1a\x07\x00"
            || count == 8 && &signature[..8] == b"Rar!\x1a\x07\x01\0"
        {
            if request.version != 2 {
                return Err(ArchiveError::UnsupportedOperation);
            }
            codec_budget(request)?;
            let format = if signature[6] == 0 { "Rar4" } else { "Rar5" };
            return Ok(Some(run_rar(
                request,
                control,
                output,
                format,
                replies,
                metadata,
                interaction,
            )?));
        }
        Ok(None)
    })();
    match result {
        Ok(Some(outcome)) => Some(Ok(outcome)),
        Ok(None) => None,
        Err(error) => Some(Err(error)),
    }
}

trait CodecOutput: ArchiveOutput {
    fn source_checkpoint(&mut self, stage: &str) -> Result<()>;
}
impl<W: Write + Send> CodecOutput for PrivateOutput<'_, W> {
    fn source_checkpoint(&mut self, stage: &str) -> Result<()> {
        PrivateOutput::source_checkpoint(self, stage)
    }
}
fn codec_budget(request: &WorkerRequest) -> Result<()> {
    let limits = request.limits;
    if limits.max_working_bytes < 64 << 20
        || limits
            .max_working_bytes
            .checked_add(limits.max_metadata_bytes)
            .and_then(|value| value.checked_add(64 << 20))
            .is_none_or(|value| value > 1 << 30)
    {
        return Err(ArchiveError::ResourceLimit);
    }
    Ok(())
}

fn source_checkpoint<W: Write + Send>(
    request: &WorkerRequest,
    control: &OperationControl,
    output: &mut W,
    replies: ReplyPort,
    stage: &str,
) -> Result<()> {
    if request.operation == "extract" {
        PrivateOutput::new(output, control, replies).source_checkpoint(stage)?;
    }
    Ok(())
}

fn fingerprint(path: &Path, control: &OperationControl) -> Result<([u8; 32], u64)> {
    let mut file = File::open(path)?;
    let before = file.metadata()?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        control.checkpoint()?;
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    let after = file.metadata()?;
    if before.len() != after.len() || before.modified()? != after.modified()? {
        return Err(ArchiveError::SourceChanged);
    }
    Ok((digest.finalize().into(), after.len()))
}

fn metadata_entry(
    id: EntryId,
    name: String,
    kind: EntryKind,
    size: u64,
    packed: Option<u64>,
    encrypted: bool,
    crc: Option<u32>,
) -> ArchiveEntry {
    ArchiveEntry {
        id,
        raw_name: name.as_bytes().to_vec(),
        display_path: name,
        provenance: NameProvenance::FormatUnicode,
        kind,
        size: Some(size),
        compressed_size: packed,
        method: 0,
        encrypted,
        crc32: crc,
        dos_modified: None,
        metadata: Default::default(),
        warnings: if matches!(kind, EntryKind::Link | EntryKind::Special) {
            vec![EntryWarning::LinkNotExtracted]
        } else {
            Vec::new()
        },
        zip: None,
    }
}
fn validate_metadata(
    entries: &mut [ArchiveEntry],
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<()> {
    if entries.len() as u64 > limits.max_entries {
        return Err(ArchiveError::ResourceLimit);
    }
    let retained = entries.iter().try_fold(0u64, |total, entry| {
        total
            .checked_add(512 + entry.raw_name.len() as u64 * 8)
            .ok_or(ArchiveError::ResourceLimit)
    })?;
    if retained > limits.max_metadata_bytes {
        return Err(ArchiveError::ResourceLimit);
    }
    crate::path_policy::annotate(entries, control, retained, limits.max_metadata_bytes)
}
fn declared_preflight(
    entries: &[ArchiveEntry],
    limits: &ResourceLimits,
    selected: Option<u64>,
) -> Result<()> {
    let mut total = 0u64;
    for entry in entries {
        if selected.is_some_and(|ordinal| entry.id().ordinal != ordinal) {
            continue;
        }
        let Some(size) = entry.size else {
            continue;
        };
        if size > limits.max_entry_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
        total = total.checked_add(size).ok_or(ArchiveError::ResourceLimit)?;
        if total > limits.max_output_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn extraction_preflight<W: Write + Send>(
    request: &WorkerRequest,
    entries: &[ArchiveEntry],
    plan: Option<&crate::planning::ExtractionPlan>,
    selected: Option<u64>,
    digest: [u8; 32],
    control: &OperationControl,
    output: &mut W,
    replies: ReplyPort,
    interaction: &crate::interaction::InteractionPort,
) -> Result<()> {
    if plan.is_some_and(|plan| {
        plan.files().len() > 4096
            || (plan.files().len() as u64).saturating_mul(64) > request.limits.max_metadata_bytes
    }) {
        return Err(ArchiveError::ResourceLimit);
    }
    let limits: ResourceLimits = request.limits.into();
    let selected_files = plan.map(|plan| {
        plan.files()
            .iter()
            .map(|file| file.id())
            .collect::<std::collections::HashSet<_>>()
    });
    let wanted = |entry: &ArchiveEntry| {
        selected.is_none_or(|ordinal| ordinal == entry.id().ordinal)
            && selected_files
                .as_ref()
                .is_none_or(|ids| ids.contains(&entry.id()))
    };
    // The original immutable selection/plan has already rejected unsafe/internal collisions.
    let required = if let Some(plan) = plan {
        crate::worker_job::declared_output(plan)
    } else {
        entries
            .iter()
            .filter(|entry| entry.kind == EntryKind::File && wanted(entry))
            .try_fold(
                crate::output_readmission::OutputBudget {
                    entry_bytes: 1,
                    output_bytes: 0,
                },
                |mut required, entry| {
                    let size = entry.size?;
                    required.entry_bytes = required.entry_bytes.max(size);
                    required.output_bytes = required.output_bytes.checked_add(size)?;
                    Some(required)
                },
            )
            .map(|mut required| {
                required.output_bytes = required.output_bytes.max(1);
                required
            })
    };
    let mut native = PrivateOutput::new(output, control, replies);
    native.preflight_output(request, digest, required)?;
    if required.is_some_and(|required| {
        required.entry_bytes > limits.max_entry_bytes
            || required.output_bytes > limits.max_output_bytes
    }) {
        return Err(ArchiveError::ResourceLimit);
    }
    if entries.iter().any(&wanted) {
        native.destination_preflight(request, required.map(|budget| budget.output_bytes))?;
    }
    drop(native);
    if request.browser_entry.is_none() {
        let risk = crate::zip_risk::ZipRisk::new(interaction);
        #[cfg(debug_assertions)]
        let risk = risk.fixture(request.zip_risk_fixture);
        // SAME finite Risk tickets and exact admitted ceiling as ZIP. A shared solid block
        // has no per-member packed size; metadata already leaves it unknown.
        for entry in entries
            .iter()
            .filter(|entry| entry.kind == EntryKind::File && wanted(entry))
        {
            risk.confirm(entry, &limits, control, output)?;
        }
    }
    Ok(())
}

fn sevenz_member_packed(
    archive: &sevenz_rust2::Archive,
    index: usize,
    block_members: &[u64],
) -> Option<u64> {
    let entry = archive.files.get(index)?;
    if !entry.has_stream {
        return None;
    }
    let block = archive
        .stream_map
        .file_block_index
        .get(index)
        .copied()
        .flatten()?;
    if block_members.get(block).copied() != Some(1) {
        return None;
    }
    let first = *archive
        .stream_map
        .block_first_pack_stream_index()
        .get(block)?;
    let end = archive
        .stream_map
        .block_first_pack_stream_index()
        .get(block + 1)
        .copied()
        .unwrap_or(archive.pack_sizes().len());
    // Every currently admitted coder chain has one packed input. The locked parser
    // permits orphan trailing pack sizes; an ambiguous span is UNKNOWN, not a larger
    // denominator. Future unsupported multi-input graphs likewise have no risk ratio.
    if end.checked_sub(first) != Some(1) {
        return None;
    }
    // Never divide a shared solid block by member or trust entry.compressed_size.
    archive
        .pack_sizes()
        .get(first..end)?
        .iter()
        .try_fold(0u64, |sum, size| sum.checked_add(*size))
}

fn run_sevenz<W: Write + Send>(
    request: &WorkerRequest,
    control: &OperationControl,
    output: &mut W,
    replies: ReplyPort,
    metadata: &crate::worker_metadata::MetadataCredit,
    interaction: &crate::interaction::InteractionPort,
) -> Result<(u64, u64)> {
    let lifecycle = control.source_delete_begin();
    let set = volumes::SourceSet::open_sevenz(request, control)?;
    let path = set.primary();
    let before = (set.digest(), set.length());
    source_checkpoint(
        request,
        control,
        output,
        replies.clone(),
        "source_validated",
    )?;
    let limits: ResourceLimits = request.limits.into();
    let mut source = set.sevenz_input(control)?;
    let mut header = [0u8; 32];
    source
        .read_exact(&mut header)
        .map_err(|_| ArchiveError::CorruptArchive)?;
    let next_size = u64::from_le_bytes(header[20..28].try_into().expect("fixed signature header"));
    let next_offset =
        u64::from_le_bytes(header[12..20].try_into().expect("fixed signature header"));
    if crc32fast::hash(&header[12..32])
        != u32::from_le_bytes(header[8..12].try_into().expect("fixed CRC"))
        || next_offset
            .checked_add(next_size)
            .and_then(|value| value.checked_add(32))
            .is_none_or(|value| value > before.1)
    {
        return Err(ArchiveError::CorruptArchive);
    }
    if next_size > limits.max_metadata_bytes {
        return Err(ArchiveError::ResourceLimit);
    }
    sevenz_budget::encoded_header(&mut source, &header, &limits)?;
    source.rewind()?;
    let password = request
        .password
        .as_deref()
        .map(sevenz_rust2::Password::from)
        .unwrap_or_default();
    let mut reader = sevenz_rust2::ArchiveReader::new(source, password).map_err(sevenz_error)?;
    source_checkpoint(request, control, output, replies.clone(), "codec_opened")?;
    reader.set_thread_count(1);
    let archive = reader.archive();
    sevenz_budget::blocks(archive, &limits)?;
    if archive.files.len() as u64 > limits.max_entries {
        return Err(ArchiveError::ResourceLimit);
    }
    if (archive.blocks.len() as u64).saturating_mul(8) > limits.max_metadata_bytes {
        return Err(ArchiveError::ResourceLimit);
    }
    let mut block_members = vec![0u64; archive.blocks.len()];
    for (index, _) in archive
        .files
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.has_stream)
    {
        control.checkpoint()?;
        let block = archive
            .stream_map
            .file_block_index
            .get(index)
            .copied()
            .flatten()
            .ok_or(ArchiveError::CorruptArchive)?;
        let members = block_members
            .get_mut(block)
            .ok_or(ArchiveError::CorruptArchive)?;
        *members = members.checked_add(1).ok_or(ArchiveError::ResourceLimit)?;
    }
    let mut entries = Vec::new();
    let mut retained = (block_members.len() as u64).saturating_mul(8);
    for (index, entry) in archive.files.iter().enumerate() {
        control.checkpoint()?;
        retained = retained
            .checked_add(512 + entry.name.len() as u64 * 8)
            .ok_or(ArchiveError::ResourceLimit)?;
        if retained > limits.max_metadata_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
        let block = archive
            .stream_map
            .file_block_index
            .get(index)
            .and_then(|value| *value)
            .and_then(|index| archive.blocks.get(index));
        let encrypted = block.is_some_and(|block| {
            block.coders.iter().any(|coder| {
                coder.encoder_method_id() == sevenz_rust2::EncoderMethod::ID_AES256_SHA256
            })
        });
        let kind = if entry.is_anti_item {
            EntryKind::Special
        } else if entry.has_windows_attributes && entry.windows_attributes & 0x400 != 0 {
            EntryKind::Link
        } else if entry.is_directory {
            EntryKind::Directory
        } else {
            EntryKind::File
        };
        let packed = sevenz_member_packed(archive, index, &block_members);
        let mut item = metadata_entry(
            EntryId {
                session: request.operation_id,
                ordinal: index as u64,
            },
            entry.name.clone(),
            kind,
            entry.size,
            packed,
            encrypted,
            entry.has_crc.then_some(entry.crc as u32),
        );
        item.metadata = crate::metadata::OutputMetadata {
            modified: entry.has_last_modified_date.then(|| {
                crate::metadata::ModifiedTime::FiletimeUtc {
                    ticks: entry.last_modified_date.into(),
                }
            }),
            attributes: entry.has_windows_attributes.then_some(
                crate::metadata::WindowsAttributes {
                    read_only: entry.windows_attributes & 1 != 0,
                    hidden: entry.windows_attributes & 2 != 0,
                },
            ),
        };
        entries.push(item);
    }
    let mut skipped = 0;
    validate_metadata(&mut entries, &limits, control)?;
    let entries: std::sync::Arc<[ArchiveEntry]> = entries.into();
    let edit_refusal = if request.operation == "open" {
        let strict = (|| {
            let password = request
                .password
                .as_deref()
                .map(sevenz_rust2::Password::from)
                .unwrap_or_default();
            let mut source = set.sevenz_input(control)?;
            let facts =
                crate::sevenz_edit_header::inspect(&mut source, &limits, control, &password)?;
            let encrypted = crate::worker_sevenz_edit::qualify(archive, &limits, control)?;
            if facts.encrypted && !encrypted && archive.files.iter().any(|entry| entry.has_stream) {
                return Err(ArchiveError::UnsupportedEncryption);
            }
            Ok(())
        })();
        strict.err().map(|error| match error {
            ArchiveError::SourceChanged => crate::ArchiveModificationRefusal::SourceChanged,
            ArchiveError::CorruptArchive => crate::ArchiveModificationRefusal::Corrupt,
            ArchiveError::UnsupportedEncryption | ArchiveError::PasswordRequired => {
                crate::ArchiveModificationRefusal::UnsupportedEncryption
            }
            ArchiveError::IoFailure => crate::ArchiveModificationRefusal::SourceUnavailable,
            _ => crate::ArchiveModificationRefusal::UnsupportedStructure,
        })
    } else {
        None
    };
    crate::worker_browser::bind_with_edit(
        request,
        request.operation_id,
        "SevenZ",
        "",
        before.0,
        before.1,
        std::fs::metadata(path)?.modified()?,
        output,
        control,
        replies.clone(),
        Some(nxtools_protocol::archive_browser::SevenzEditEligibility {
            refusal: edit_refusal,
        }),
        None,
    )?;
    let selected =
        crate::worker_browser::selected(request, &entries)?.map(|entry| entry.id().ordinal);
    let mut job_plan = crate::worker_job::codec_plan(request, entries.clone())?;
    let result = if request.operation == "open" {
        crate::worker_metadata::emit_entries(
            request,
            &entries,
            request.operation_id,
            "SevenZ",
            output,
            control,
            metadata,
        )?;
        (entries.len() as u64, 0)
    } else {
        // Extraction paths/collisions must be rejected before any budget/risk prompt.
        if request.operation != "extract" {
            declared_preflight(&entries, &limits, selected)?;
        }
        if request.password.is_none() && entries.iter().any(|entry| entry.encrypted) {
            return Err(ArchiveError::PasswordRequired);
        }
        let extracting = request.operation == "extract";
        if extracting && selected.is_none() {
            validate_extraction(&entries)?;
        }
        if extracting {
            set.verify(control)?;
            extraction_preflight(
                request,
                &entries,
                job_plan.as_ref(),
                selected,
                before.0,
                control,
                output,
                replies.clone(),
                interaction,
            )?;
        }
        let mut native = extracting
            .then(|| -> Result<_> {
                Ok(PrivateOutput::new(output, control, replies)
                    .planned_unique(job_plan.as_mut())?
                    .extraction_metadata(request.extract_options.is_some())
                    .conflicts(request.conflict_policy, interaction, entries.clone()))
            })
            .transpose()?;
        let mut pending = Vec::new();
        let mut bytes = 0u64;
        let mut files = 0u64;
        let mut written_bytes = 0u64;
        let encrypted = entries.iter().any(|value| value.encrypted);
        // The locked reader visits block spans first, then all files without a block.
        // Names cannot identify a callback: distinct 7z ordinals may share one name.
        let callback_bytes = (entries.len() as u64)
            .checked_mul(std::mem::size_of::<usize>() as u64)
            .ok_or(ArchiveError::ResourceLimit)?;
        if retained
            .checked_add(callback_bytes)
            .is_none_or(|bytes| bytes > limits.max_metadata_bytes)
        {
            return Err(ArchiveError::ResourceLimit);
        }
        let mut callback_ordinals = Vec::with_capacity(entries.len());
        {
            let archive = reader.archive();
            for (block_index, &member_count) in block_members.iter().enumerate() {
                let start = *archive
                    .stream_map
                    .block_first_file_index
                    .get(block_index)
                    .ok_or(ArchiveError::CorruptArchive)?;
                let count =
                    usize::try_from(member_count).map_err(|_| ArchiveError::ResourceLimit)?;
                let end = start
                    .checked_add(count)
                    .filter(|end| *end <= entries.len())
                    .ok_or(ArchiveError::CorruptArchive)?;
                callback_ordinals.extend(start..end);
            }
            for (index, block) in archive.stream_map.file_block_index.iter().enumerate() {
                if block.is_none() {
                    callback_ordinals.push(index);
                }
            }
        }
        if callback_ordinals.len() != entries.len() {
            return Err(ArchiveError::CorruptArchive);
        }
        let mut callback_ordinals = callback_ordinals.into_iter();
        let mut domain_error = None;
        let mut decoding_checkpoint = false;
        let result = (|| {
            let decoded = reader.for_each_entries(|entry, input| {
                // Upstream continues with later blocks after false; never overwrite first error
                // or create more output after a previous block callback has failed.
                if domain_error.is_some() {
                    return Ok(false);
                }
                let result: Result<()> = (|| {
                    control.checkpoint()?;
                    if !decoding_checkpoint {
                        decoding_checkpoint = true;
                        if let Some(native) = &mut native {
                            native.source_checkpoint("source_decoding")?;
                        }
                    }
                    let ordinal = callback_ordinals
                        .next()
                        .ok_or(ArchiveError::CorruptArchive)?;
                    let metadata = entries.get(ordinal).ok_or(ArchiveError::CorruptArchive)?;
                    if entry.name != metadata.display_path {
                        return Err(ArchiveError::SourceChanged);
                    }
                    let relative = metadata.display_path.replace('\\', "/");
                    let relative = relative.trim_end_matches('/');
                    let wanted = selected.is_none_or(|ordinal| ordinal == metadata.id().ordinal);
                    let token = match (native.as_mut().filter(|_| wanted), metadata.kind) {
                        (Some(native), EntryKind::Directory) => {
                            native.ensure_directory(relative)?;
                            None
                        }
                        (Some(native), EntryKind::File) => {
                            let token = native.begin_extracted_file(relative, metadata.size)?;
                            if let Some(token) = token {
                                pending.push((token, metadata.metadata));
                            }
                            token
                        }
                        _ => None,
                    };
                    let mut count = 0u64;
                    let mut crc = crc32fast::Hasher::new();
                    let mut buffer = zeroize::Zeroizing::new([0u8; 64 * 1024]);
                    loop {
                        control.checkpoint()?;
                        let read = input.read(&mut buffer[..]).map_err(|_| {
                            if encrypted {
                                ArchiveError::AuthenticationOrDataError
                            } else {
                                ArchiveError::CorruptArchive
                            }
                        })?;
                        if read == 0 {
                            break;
                        }
                        count = count
                            .checked_add(read as u64)
                            .ok_or(ArchiveError::ResourceLimit)?;
                        bytes = bytes
                            .checked_add(read as u64)
                            .ok_or(ArchiveError::ResourceLimit)?;
                        let decode_limits = crate::worker_browser::decode_limits(request);
                        if count > decode_limits.max_entry_bytes
                            || bytes > decode_limits.max_output_bytes
                            || wanted
                                && (count > limits.max_entry_bytes
                                    || written_bytes.saturating_add(count)
                                        > limits.max_output_bytes)
                        {
                            return Err(ArchiveError::ResourceLimit);
                        }
                        crc.update(&buffer[..read]);
                        if let (Some(native), Some(token)) = (&mut native, token) {
                            native.write_file(token, &buffer[..read])?;
                        }
                    }
                    if count != entry.size
                        || metadata
                            .crc32
                            .is_some_and(|expected| crc.finalize() != expected)
                    {
                        return Err(if encrypted {
                            ArchiveError::AuthenticationOrDataError
                        } else {
                            ArchiveError::CorruptArchive
                        });
                    }
                    if metadata.kind == EntryKind::File && (!extracting || token.is_some()) {
                        files += 1;
                        written_bytes += count;
                    }
                    Ok(())
                })();
                match result {
                    Ok(()) => Ok(true),
                    Err(error) => {
                        domain_error = Some(error);
                        Ok(false)
                    }
                }
            });
            if let Some(error) = domain_error {
                return Err(error);
            }
            decoded.map_err(sevenz_error)?;
            if callback_ordinals.next().is_some() {
                return Err(ArchiveError::CorruptArchive);
            }
            set.verify(control)?;
            // Solid-block integrity may be checked only at its end. No final file is
            // published until every decoder and the source recheck have succeeded.
            if let Some(native) = &mut native {
                native.source_checkpoint("source_precommit")?;
                while let Some(&(token, metadata)) = pending.last() {
                    control.checkpoint()?;
                    native.set_file_metadata(token, metadata)?;
                    native.commit_file(token)?;
                    pending.pop();
                }
                if selected.is_none() {
                    finalize_directories(&entries, native, control)?;
                }
            }
            Ok((files, if extracting { written_bytes } else { bytes }))
        })();
        if let Some(native) = &mut native {
            for (token, _) in pending {
                native.abort_file(token);
            }
            native.extraction_summary(
                result.is_ok()
                    && entries
                        .iter()
                        .all(|entry| !matches!(entry.kind, EntryKind::Link | EntryKind::Special)),
                job_plan.as_ref().map_or_else(
                    || {
                        entries
                            .iter()
                            .filter(|entry| {
                                matches!(entry.kind, EntryKind::Link | EntryKind::Special)
                            })
                            .count() as u64
                    },
                    |plan| plan.skipped_special(),
                ),
            )?;
            skipped = native.skipped_files();
        }
        result?
    };
    set.verify(control)?;
    record_extraction(request, &entries, result.0, skipped, control, &lifecycle);
    Ok(result)
}
pub(crate) fn sevenz_error(error: sevenz_rust2::Error) -> ArchiveError {
    use sevenz_rust2::Error;
    match error {
        Error::PasswordRequired => ArchiveError::PasswordRequired,
        Error::MaybeBadPassword(_) => ArchiveError::AuthenticationOrDataError,
        Error::MaxMemLimited { .. } => ArchiveError::ResourceLimit,
        Error::UnsupportedCompressionMethod(_)
        | Error::ExternalUnsupported
        | Error::Unsupported(_) => ArchiveError::UnsupportedMethod,
        Error::Io(ref error, _)
            if error.kind() == std::io::ErrorKind::InvalidData
                || error.kind() == std::io::ErrorKind::UnexpectedEof =>
        {
            ArchiveError::CorruptArchive
        }
        Error::Io(_, _) | Error::FileOpen(_, _) => ArchiveError::IoFailure,
        _ => ArchiveError::CorruptArchive,
    }
}

fn finalize_directories(
    entries: &[ArchiveEntry],
    output: &mut dyn ArchiveOutput,
    control: &OperationControl,
) -> Result<()> {
    let mut directories: Vec<_> = entries
        .iter()
        .filter(|entry| entry.kind == EntryKind::Directory)
        .collect();
    directories.sort_by_key(|entry| {
        std::cmp::Reverse(
            entry
                .display_path
                .trim_end_matches(['/', '\\'])
                .matches(['/', '\\'])
                .count(),
        )
    });
    for entry in directories {
        control.checkpoint()?;
        let relative = entry.display_path.replace('\\', "/");
        output.finalize_directory_metadata(relative.trim_end_matches('/'), entry.metadata)?;
    }
    Ok(())
}

fn validate_extraction(entries: &[ArchiveEntry]) -> Result<()> {
    let mut files = 0;
    for entry in entries {
        if matches!(entry.kind, EntryKind::File | EntryKind::Directory) {
            entry.metadata.validate()?;
        }
        if entry.warnings.contains(&EntryWarning::UnsafePath) {
            return Err(ArchiveError::UnsafePath);
        }
        if entry.warnings.contains(&EntryWarning::OutputCollision) {
            return Err(ArchiveError::OutputCollision);
        }
        if entry.kind == EntryKind::File {
            files += 1;
            if files > 4096 {
                return Err(ArchiveError::ResourceLimit);
            }
        }
    }
    Ok(())
}

#[cfg(windows)]
fn run_rar<W: Write + Send>(
    request: &WorkerRequest,
    control: &OperationControl,
    output: &mut W,
    format: &str,
    replies: ReplyPort,
    metadata: &crate::worker_metadata::MetadataCredit,
    interaction: &crate::interaction::InteractionPort,
) -> Result<(u64, u64)> {
    let lifecycle = control.source_delete_begin();
    let path = Path::new(&request.path);
    let set = volumes::SourceSet::open_rar(request, control)?;
    let before = (set.digest(), set.length());
    source_checkpoint(
        request,
        control,
        output,
        replies.clone(),
        "source_validated",
    )?;
    let (mut entries, mut files, mut bytes, _) = rar::execute(request, control, None, None)?;
    validate_metadata(&mut entries, &request.limits.into(), control)?;
    let entries: std::sync::Arc<[ArchiveEntry]> = entries.into();
    crate::worker_browser::bind(
        request,
        request.operation_id,
        format,
        "",
        before.0,
        before.1,
        std::fs::metadata(path)?.modified()?,
        output,
        control,
        replies.clone(),
    )?;
    let selected =
        crate::worker_browser::selected(request, &entries)?.map(|entry| entry.id().ordinal);
    let mut job_plan = crate::worker_job::codec_plan(request, entries.clone())?;
    let mut skipped = 0;
    if request.operation == "extract" {
        if selected.is_none() {
            validate_extraction(&entries)?;
        }
        set.verify(control)?;
        extraction_preflight(
            request,
            &entries,
            job_plan.as_ref(),
            selected,
            before.0,
            control,
            output,
            replies.clone(),
            interaction,
        )?;
        let mut native = PrivateOutput::new(output, control, replies)
            .planned_unique(job_plan.as_mut())?
            .extraction_metadata(request.extract_options.is_some())
            .conflicts(request.conflict_policy, interaction, entries.clone());
        let result: Result<()> = (|| {
            let (_, count, decoded, mut pending) =
                rar::execute(request, control, Some(&mut native), Some(&entries))?;
            let result: Result<()> = (|| {
                set.verify(control)?;
                native.source_checkpoint("source_precommit")?;
                while let Some(&(token, metadata)) = pending.last() {
                    control.checkpoint()?;
                    native.set_file_metadata(token, metadata)?;
                    native.commit_file(token)?;
                    pending.pop();
                }
                if selected.is_none() {
                    finalize_directories(&entries, &mut native, control)?;
                }
                Ok(())
            })();
            for (token, _) in pending {
                native.abort_file(token);
            }
            result?;
            files = count;
            bytes = decoded;
            Ok(())
        })();
        native.extraction_summary(
            result.is_ok()
                && entries
                    .iter()
                    .all(|entry| !matches!(entry.kind, EntryKind::Link | EntryKind::Special)),
            job_plan.as_ref().map_or_else(
                || {
                    entries
                        .iter()
                        .filter(|entry| matches!(entry.kind, EntryKind::Link | EntryKind::Special))
                        .count() as u64
                },
                |plan| plan.skipped_special(),
            ),
        )?;
        skipped = native.skipped_files();
        result?;
    }
    if request.operation == "open" {
        crate::worker_metadata::emit_entries(
            request,
            &entries,
            request.operation_id,
            format,
            output,
            control,
            metadata,
        )?;
    }
    set.verify(control)?;
    record_extraction(request, &entries, files, skipped, control, &lifecycle);
    Ok((
        if request.operation == "open" {
            entries.len() as u64
        } else {
            files
        },
        bytes,
    ))
}

fn record_extraction(
    request: &WorkerRequest,
    entries: &[ArchiveEntry],
    files: u64,
    skipped: u64,
    control: &OperationControl,
    lifecycle: &crate::source_deletion::Lifecycle,
) {
    if request.operation != "extract" {
        return;
    }
    let expected = entries
        .iter()
        .filter(|entry| entry.kind == EntryKind::File)
        .count() as u64;
    let reason = if skipped != 0
        || entries
            .iter()
            .any(|entry| matches!(entry.kind, EntryKind::Link | EntryKind::Special))
    {
        "members_skipped"
    } else if expected == 0 {
        "empty_source_set"
    } else if files != expected {
        "coverage_incomplete"
    } else {
        "eligible"
    };
    control.source_delete_finish(lifecycle, reason, vec![request.operation_id]);
}

#[cfg(test)]
mod browser_budget_tests {
    use super::*;
    #[test]
    fn selected_preflight_does_not_charge_unselected_declared_output() {
        let session = uuid::Uuid::new_v4();
        let entries = vec![
            metadata_entry(
                EntryId {
                    session,
                    ordinal: 0,
                },
                "large.bin".into(),
                EntryKind::File,
                600 << 20,
                None,
                false,
                None,
            ),
            metadata_entry(
                EntryId {
                    session,
                    ordinal: 1,
                },
                "small.txt".into(),
                EntryKind::File,
                8,
                None,
                false,
                None,
            ),
        ];
        let limits = ResourceLimits {
            max_entry_bytes: 512 << 20,
            max_output_bytes: 512 << 20,
            ..Default::default()
        };
        assert!(declared_preflight(&entries, &limits, None).is_err());
        assert!(declared_preflight(&entries, &limits, Some(1)).is_ok());
        assert!(declared_preflight(&entries, &limits, Some(0)).is_err());
    }
}

#[cfg(test)]
mod packed_risk_tests {
    use super::*;
    #[test]
    fn orphan_packed_stream_is_not_an_independent_member_denominator() {
        // Plain 7z Header/MainStreamsInfo: one Copy block, one1byte file, but two
        // declared packed streams. Locked reader accepts the unused trailing stream.
        let next = vec![
            1, 4, 6, 0, 2, 9, 1, 1, 0, 7, 11, 1, 0, 1, 1, 0, 12, 1, 0, 8, 0, 0, 5, 1, 17, 5, 0,
            b'a', 0, 0, 0, 0, 0,
        ];
        let mut signature = [0u8; 32];
        signature[..6].copy_from_slice(b"7z\xbc\xaf\x27\x1c");
        signature[6] = 0;
        signature[7] = 4;
        signature[20..28].copy_from_slice(&(next.len() as u64).to_le_bytes());
        signature[28..32].copy_from_slice(&crc32fast::hash(&next).to_le_bytes());
        let crc = crc32fast::hash(&signature[12..32]);
        signature[8..12].copy_from_slice(&crc.to_le_bytes());
        let mut bytes = signature.to_vec();
        bytes.extend(next);
        let archive = sevenz_rust2::Archive::read(
            &mut std::io::Cursor::new(bytes),
            &sevenz_rust2::Password::default(),
        )
        .expect("locked metadata parser");
        assert_eq!(archive.pack_sizes(), &[1, 1]);
        assert_eq!(archive.files.len(), 1);
        assert_eq!(
            sevenz_member_packed(&archive, 0, &[1]),
            None,
            "unreferenced stream cannot dilute risk ratio"
        );
    }
}
