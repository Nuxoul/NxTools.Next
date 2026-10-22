use crate::{
    ArchiveEntry, ArchiveError, ArchiveOutcome, ArchiveProgress, ArchiveSnapshot, EntryId,
    EntryKind, EntryWarning, NameEncoding, NameProvenance, OperationControl, ResourceLimits,
    Result, encoding,
};
use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Seek, SeekFrom},
};
use uuid::Uuid;

struct Directory {
    offset: u64,
    size: u64,
    count: u64,
    comment: Vec<u8>,
    retained: u64,
}
type Listed = (Vec<ArchiveEntry>, String, bool);

fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}
fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("fixed header field"),
    )
}
fn u64_at(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(
        bytes[offset..offset + 8]
            .try_into()
            .expect("fixed ZIP64 field"),
    )
}
fn read_exact(file: &mut File, bytes: &mut [u8]) -> Result<()> {
    file.read_exact(bytes).map_err(|failure| {
        if failure.kind() == std::io::ErrorKind::UnexpectedEof {
            ArchiveError::CorruptArchive
        } else {
            ArchiveError::IoFailure
        }
    })
}

/// Never call the backend's allocator-heavy constructor before this independent preflight.
fn directory(file: &mut File, limits: &ResourceLimits) -> Result<Directory> {
    let length = file.metadata()?.len();
    let tail_size = usize::try_from(length.min(65_557)).map_err(|_| ArchiveError::ResourceLimit)?;
    if tail_size < 22 {
        return Err(ArchiveError::CorruptArchive);
    }
    if tail_size as u64 > limits.max_working_bytes {
        return Err(ArchiveError::ResourceLimit);
    }
    file.seek(SeekFrom::End(-(tail_size as i64)))?;
    let mut tail = vec![0u8; tail_size];
    read_exact(file, &mut tail)?;
    let mut last_failure = ArchiveError::CorruptArchive;
    for position in (0..=tail.len() - 22).rev() {
        if &tail[position..position + 4] != b"PK\x05\x06"
            || position + 22 + usize::from(u16_at(&tail, position + 20)) != tail.len()
        {
            continue;
        }
        let eocd_offset = length - tail_size as u64 + position as u64;
        match directory_candidate(file, limits, &tail[position..], eocd_offset) {
            Ok(directory) => return Ok(directory),
            Err(failure) => last_failure = failure,
        }
    }
    Err(last_failure)
}

fn directory_candidate(
    file: &mut File,
    limits: &ResourceLimits,
    eocd: &[u8],
    eocd_offset: u64,
) -> Result<Directory> {
    if u16_at(eocd, 4) != 0 || u16_at(eocd, 6) != 0 || u16_at(eocd, 8) != u16_at(eocd, 10) {
        return Err(ArchiveError::MissingVolume);
    }
    let mut count = u64::from(u16_at(eocd, 10));
    let mut size = u64::from(u32_at(eocd, 12));
    let mut offset = u64::from(u32_at(eocd, 16));
    let mut directory_end = eocd_offset;
    if count == 0xffff || size == 0xffff_ffff || offset == 0xffff_ffff {
        if eocd_offset < 20 {
            return Err(ArchiveError::CorruptArchive);
        }
        file.seek(SeekFrom::Start(eocd_offset - 20))?;
        let mut locator = [0u8; 20];
        read_exact(file, &mut locator)?;
        if &locator[..4] != b"PK\x06\x07" {
            return Err(ArchiveError::CorruptArchive);
        }
        if u32_at(&locator, 4) != 0 || u32_at(&locator, 16) != 1 {
            return Err(ArchiveError::MissingVolume);
        }
        let zip64_offset = u64_at(&locator, 8);
        if zip64_offset
            .checked_add(56)
            .is_none_or(|end| end > eocd_offset - 20)
        {
            return Err(ArchiveError::CorruptArchive);
        }
        file.seek(SeekFrom::Start(zip64_offset))?;
        let mut header = [0u8; 56];
        read_exact(file, &mut header)?;
        if &header[..4] != b"PK\x06\x06" || u64_at(&header, 4) < 44 {
            return Err(ArchiveError::CorruptArchive);
        }
        if u64_at(&header, 4) > limits.max_metadata_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
        if zip64_offset
            .checked_add(12)
            .and_then(|start| start.checked_add(u64_at(&header, 4)))
            != Some(eocd_offset - 20)
        {
            return Err(ArchiveError::CorruptArchive);
        }
        if u32_at(&header, 16) != 0
            || u32_at(&header, 20) != 0
            || u64_at(&header, 24) != u64_at(&header, 32)
        {
            return Err(ArchiveError::MissingVolume);
        }
        count = u64_at(&header, 32);
        size = u64_at(&header, 40);
        offset = u64_at(&header, 48);
        directory_end = zip64_offset;
    }
    if offset.checked_add(size) != Some(directory_end) || count > size / 46 {
        return Err(ArchiveError::CorruptArchive);
    }
    // Retained entry/map capacities and names; projection nodes are charged before allocation.
    let estimate = count
        .checked_mul(384)
        .and_then(|fixed| {
            size.checked_mul(8)
                .and_then(|variable| fixed.checked_add(variable))
        })
        .and_then(|estimate| estimate.checked_add((eocd.len() as u64 - 22).saturating_mul(4)))
        .and_then(|estimate| estimate.checked_add(if eocd.len() > 22 { 64 } else { 0 }))
        .ok_or(ArchiveError::ResourceLimit)?;
    if count > limits.max_entries || estimate > limits.max_metadata_bytes {
        return Err(ArchiveError::ResourceLimit);
    }
    Ok(Directory {
        offset,
        size,
        count,
        comment: eocd[22..].to_vec(),
        retained: estimate,
    })
}

pub(crate) fn list(
    file: &mut File,
    selected: NameEncoding,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<Listed> {
    let central = directory(file, limits)?;
    file.seek(SeekFrom::Start(central.offset))?;
    let session = Uuid::new_v4();
    let mut entries: Vec<ArchiveEntry> = Vec::new();
    let mut names: HashMap<Vec<u8>, usize> = HashMap::new();
    let mut backend_safe = true;
    for ordinal in 0..central.count {
        control.checkpoint()?;
        let mut header = [0u8; 46];
        read_exact(file, &mut header)?;
        if &header[..4] != b"PK\x01\x02" {
            return Err(ArchiveError::CorruptArchive);
        }
        let name_length = usize::from(u16_at(&header, 28));
        let extra_length = usize::from(u16_at(&header, 30));
        let comment_length = usize::from(u16_at(&header, 32));
        let end = file
            .stream_position()?
            .checked_add((name_length + extra_length + comment_length) as u64)
            .ok_or(ArchiveError::CorruptArchive)?;
        if end > central.offset + central.size {
            return Err(ArchiveError::CorruptArchive);
        }
        let mut raw_name = vec![0; name_length];
        let mut extra = vec![0; extra_length];
        read_exact(file, &mut raw_name)?;
        read_exact(file, &mut extra)?;
        file.seek(SeekFrom::Current(comment_length as i64))?;
        let disk = u16_at(&header, 34);
        let (display_path, provenance) = encoding::decode_zip(
            &raw_name,
            u16_at(&header, 8) & 0x800 != 0,
            &extra,
            selected,
            limits
                .max_working_bytes
                .min(limits.max_metadata_bytes - central.retained),
        )?;
        let kind = match u32_at(&header, 38) >> 28 {
            0xa => EntryKind::Link,
            0x4 => EntryKind::Directory,
            0 | 0x8 => {
                if display_path.ends_with(['/', '\\']) {
                    EntryKind::Directory
                } else {
                    EntryKind::File
                }
            }
            _ => EntryKind::Special,
        };
        let mut warnings = Vec::new();
        if matches!(
            provenance,
            NameProvenance::UncertainCp437 | NameProvenance::Heuristic(_)
        ) && !raw_name.is_ascii()
        {
            warnings.push(EntryWarning::UncertainEncoding);
        }
        if matches!(kind, EntryKind::Link | EntryKind::Special) {
            warnings.push(EntryWarning::LinkNotExtracted);
        }
        if let Some(&previous) = names.get(&raw_name) {
            entries[previous]
                .warnings
                .push(EntryWarning::OutputCollision);
            warnings.push(EntryWarning::OutputCollision);
            backend_safe = false;
        } else {
            names.insert(raw_name.clone(), entries.len());
        }
        let mut size = u64::from(u32_at(&header, 24));
        let mut packed = u64::from(u32_at(&header, 20));
        let mut header_offset = u64::from(u32_at(&header, 42));
        if size == 0xffff_ffff
            || packed == 0xffff_ffff
            || header_offset == 0xffff_ffff
            || disk == 0xffff
        {
            let field = extra_field(&extra, 1)?.ok_or(ArchiveError::CorruptArchive)?;
            let mut cursor = 0;
            for value in [&mut size, &mut packed, &mut header_offset] {
                if *value == 0xffff_ffff {
                    let bytes = field
                        .get(cursor..cursor + 8)
                        .ok_or(ArchiveError::CorruptArchive)?;
                    *value = u64_at(bytes, 0);
                    cursor += 8;
                }
            }
            if disk == 0xffff {
                let bytes = field
                    .get(cursor..cursor + 4)
                    .ok_or(ArchiveError::CorruptArchive)?;
                if u32_at(bytes, 0) != 0 {
                    return Err(ArchiveError::MissingVolume);
                }
            }
        }
        if disk != 0 && disk != 0xffff {
            return Err(ArchiveError::MissingVolume);
        }
        if header_offset >= central.offset {
            return Err(ArchiveError::CorruptArchive);
        }
        entries.push(ArchiveEntry {
            id: EntryId { session, ordinal },
            raw_name,
            display_path,
            provenance,
            kind,
            size: Some(size),
            compressed_size: Some(packed),
            method: u16_at(&header, 10),
            encrypted: u16_at(&header, 8) & 1 != 0,
            crc32: Some(u32_at(&header, 16)),
            dos_modified: Some((u16_at(&header, 14), u16_at(&header, 12))),
            metadata: zip_metadata(
                &extra,
                u16_at(&header, 14),
                u16_at(&header, 12),
                header[5],
                u32_at(&header, 38),
            )?,
            zip: Some(crate::model::ZipLocation {
                header_offset,
                directory_offset: central.offset,
                flags: u16_at(&header, 8),
                zip64_sizes: u32_at(&header, 20) == u32::MAX || u32_at(&header, 24) == u32::MAX,
                aes: super::zip_aes::descriptor(
                    &extra,
                    u16_at(&header, 8),
                    u16_at(&header, 10),
                    packed,
                )?,
            }),
            warnings,
        });
    }
    if file.stream_position()? != central.offset + central.size {
        return Err(ArchiveError::CorruptArchive);
    }
    let retained = central.retained;
    crate::path_policy::annotate(&mut entries, control, retained, limits.max_metadata_bytes)?;
    Ok((
        entries,
        String::from_utf8_lossy(&central.comment).into_owned(),
        backend_safe,
    ))
}

fn zip_metadata(
    extra: &[u8],
    date: u16,
    time: u16,
    host: u8,
    attributes: u32,
) -> Result<crate::metadata::OutputMetadata> {
    use crate::metadata::{ModifiedTime, OutputMetadata};
    let ntfs = extra_field(extra, 10)?;
    let unix = extra_field(extra, 0x5455)?;
    let modified = if let Some(bytes) = ntfs {
        if bytes.len() != 32 || u16_at(bytes, 4) != 1 || u16_at(bytes, 6) != 24 {
            Some(ModifiedTime::Invalid)
        } else {
            Some(ModifiedTime::FiletimeUtc {
                ticks: u64_at(bytes, 8),
            })
        }
    } else if let Some(bytes) = unix {
        // Central UT may carry only mtime while flags describe the local header's other times.
        if bytes.is_empty()
            || bytes[0] & !7 != 0
            || !(bytes.len() == 5 && bytes[0] & 1 != 0
                || bytes.len() == 1 + (bytes[0] & 7).count_ones() as usize * 4)
        {
            Some(ModifiedTime::Invalid)
        } else if bytes[0] & 1 != 0 {
            Some(ModifiedTime::UnixUtc {
                seconds: i64::from(u32_at(bytes, 1)),
                nanoseconds: 0,
                precision_ns: 1_000_000_000,
            })
        } else {
            (date != 0 || time != 0).then_some(ModifiedTime::DosLocal { date, time })
        }
    } else {
        (date != 0 || time != 0).then_some(ModifiedTime::DosLocal { date, time })
    };
    Ok(OutputMetadata {
        modified,
        attributes: matches!(host, 0 | 10 | 14).then_some(crate::metadata::WindowsAttributes {
            read_only: attributes & 1 != 0,
            hidden: attributes & 2 != 0,
        }),
    })
}

fn extra_field(extra: &[u8], tag: u16) -> Result<Option<&[u8]>> {
    let mut cursor = 0;
    let mut found = None;
    while cursor < extra.len() {
        let header = extra
            .get(cursor..cursor + 4)
            .ok_or(ArchiveError::CorruptArchive)?;
        let size = usize::from(u16_at(header, 2));
        let field = extra
            .get(cursor + 4..cursor + 4 + size)
            .ok_or(ArchiveError::CorruptArchive)?;
        if u16_at(header, 0) == tag && found.replace(field).is_some() {
            return Err(ArchiveError::CorruptArchive);
        }
        cursor += 4 + size;
    }
    Ok(found)
}

fn compression_method(entry: &ArchiveEntry) -> u16 {
    entry
        .zip
        .as_ref()
        .and_then(|location| location.aes)
        .map_or(entry.method, |aes| aes.method)
}

pub(crate) fn test(
    mut file: File,
    snapshot: &ArchiveSnapshot,
    limits: &ResourceLimits,
    control: &OperationControl,
    password: Option<&str>,
) -> Result<ArchiveOutcome> {
    preflight_payloads(&mut file, snapshot, limits, control)?;
    let mut outcome = ArchiveOutcome {
        files_verified: 0,
        bytes_verified: 0,
    };
    for entry in snapshot.entries.iter() {
        let count = stream_entry(
            &mut file,
            entry,
            limits,
            control,
            password,
            &mut outcome.bytes_verified,
            |_| Ok(()),
        )?;
        if entry.kind != EntryKind::Directory {
            outcome.files_verified += 1;
        }
        let _ = count;
    }
    Ok(outcome)
}

fn preflight_payloads(
    file: &mut File,
    snapshot: &ArchiveSnapshot,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<()> {
    preflight_created(file, &snapshot.entries, limits, control)
}

pub(crate) fn preflight_created(
    file: &mut File,
    entries: &[ArchiveEntry],
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<()> {
    let central = directory(file, limits)?;
    if central.count != entries.len() as u64 {
        return Err(ArchiveError::SourceChanged);
    }
    let mut extents = std::collections::BTreeMap::new();
    for entry in entries {
        control.checkpoint()?;
        let offset = validate_local(file, entry)?;
        let location = entry.zip.as_ref().ok_or(ArchiveError::CorruptArchive)?;
        let end = validate_descriptor(file, entry, offset)?;
        if extents
            .range(..=location.header_offset)
            .next_back()
            .is_some_and(|(_, &end)| end > location.header_offset)
            || extents
                .range(location.header_offset..)
                .next()
                .is_some_and(|(&start, _)| start < end)
        {
            return Err(ArchiveError::CorruptArchive);
        }
        extents.insert(location.header_offset, end);
        if entry.kind == EntryKind::Directory && entry.size != Some(0) {
            return Err(ArchiveError::CorruptArchive);
        }
    }
    Ok(())
}

pub(crate) fn validate_descriptor(
    file: &mut File,
    entry: &ArchiveEntry,
    offset: u64,
) -> Result<u64> {
    let location = entry.zip.as_ref().ok_or(ArchiveError::CorruptArchive)?;
    let end = offset
        .checked_add(entry.compressed_size.ok_or(ArchiveError::CorruptArchive)?)
        .ok_or(ArchiveError::CorruptArchive)?;
    if location.flags & 8 == 0 {
        return Ok(end);
    }
    let mut bytes = [0; 24];
    let available = location
        .directory_offset
        .checked_sub(end)
        .ok_or(ArchiveError::CorruptArchive)?
        .min(24) as usize;
    file.seek(SeekFrom::Start(end))?;
    read_exact(file, &mut bytes[..available])?;
    let size64 = location.zip64_sizes
        || entry.size.is_some_and(|size| size > u64::from(u32::MAX))
        || entry
            .compressed_size
            .is_some_and(|size| size > u64::from(u32::MAX));
    for prefix in [4, 0] {
        let length = prefix + if size64 { 20 } else { 12 };
        if available < length || prefix == 4 && &bytes[..4] != b"PK\x07\x08" {
            continue;
        }
        let crc = u32_at(&bytes, prefix);
        let packed = if size64 {
            u64_at(&bytes, prefix + 4)
        } else {
            u64::from(u32_at(&bytes, prefix + 4))
        };
        let size = if size64 {
            u64_at(&bytes, prefix + 12)
        } else {
            u64::from(u32_at(&bytes, prefix + 8))
        };
        if Some(crc) == entry.crc32
            && Some(packed) == entry.compressed_size
            && Some(size) == entry.size
        {
            return Ok(end + length as u64);
        }
    }
    Err(ArchiveError::CorruptArchive)
}

pub(crate) fn stream_entry(
    file: &mut File,
    entry: &ArchiveEntry,
    limits: &ResourceLimits,
    control: &OperationControl,
    password: Option<&str>,
    total: &mut u64,
    mut on_chunk: impl FnMut(&[u8]) -> Result<()>,
) -> Result<u64> {
    control.checkpoint()?;
    let method = compression_method(entry);
    if !matches!(method, 0 | 8 | 9 | 12 | 14 | 93 | 95) {
        return Err(ArchiveError::UnsupportedMethod);
    }
    if method == 12 && limits.max_working_bytes < 8 * 1024 * 1024 {
        return Err(ArchiveError::ResourceLimit);
    }
    if entry.encrypted && limits.max_working_bytes < 1 << 20 {
        return Err(ArchiveError::ResourceLimit);
    }
    let offset = validate_local(file, entry)?;
    let packed = entry.compressed_size.ok_or(ArchiveError::CorruptArchive)?;
    let mut input =
        super::zip_crypto::Reader::new(&mut *file, offset, packed, entry, password, control)?;
    let payload_length = input.length();
    let authentication = |failure| {
        if entry.encrypted && failure == ArchiveError::CorruptArchive {
            ArchiveError::AuthenticationOrDataError
        } else {
            failure
        }
    };
    let mut checksum = crc32fast::Hasher::new();
    let mut bytes = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    let mut consume = |chunk: &[u8]| {
        bytes = bytes
            .checked_add(chunk.len() as u64)
            .ok_or(ArchiveError::ResourceLimit)?;
        *total = total
            .checked_add(chunk.len() as u64)
            .ok_or(ArchiveError::ResourceLimit)?;
        if bytes > limits.max_entry_bytes || *total > limits.max_output_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
        checksum.update(chunk);
        on_chunk(chunk)?;
        control.publish(ArchiveProgress {
            bytes: *total,
            files: 0,
            total_bytes: None,
        });
        Ok(())
    };
    let consumed = if matches!(method, 93 | 95) {
        if method == 95 {
            super::xz::stream(&mut input, limits, control, &mut consume).map_err(authentication)?;
        } else {
            super::zstd::stream(&mut input, limits, control, &mut consume)
                .map_err(authentication)?;
        }
        // The adapter requires complete frames AND EOF of this exact bounded view.
        input.stream_position()?
    } else {
        let mut stream = Payload::new(
            (&mut input).take(payload_length),
            entry,
            limits,
            control.clone(),
        )
        .map_err(authentication)?;
        loop {
            control.checkpoint()?;
            let count = stream
                .read(&mut buffer)
                .map_err(payload_error)
                .map_err(authentication)?;
            control.checkpoint()?;
            if count == 0 {
                break;
            }
            consume(&buffer[..count])?;
        }
        stream.compressed_consumed(payload_length)
    };
    if Some(bytes) != entry.size
        || (entry
            .zip
            .as_ref()
            .and_then(|location| location.aes)
            .is_none_or(|aes| aes.version != 2)
            && Some(checksum.finalize()) != entry.crc32)
        || consumed != payload_length
    {
        return Err(authentication(ArchiveError::CorruptArchive));
    }
    Ok(bytes)
}

pub(crate) fn payload_error(failure: std::io::Error) -> ArchiveError {
    if let Some(typed) = failure
        .get_ref()
        .and_then(|value| value.downcast_ref::<ArchiveError>())
    {
        return *typed;
    }
    match failure.kind() {
        std::io::ErrorKind::OutOfMemory => ArchiveError::ResourceLimit,
        std::io::ErrorKind::InvalidData
        | std::io::ErrorKind::UnexpectedEof
        | std::io::ErrorKind::InvalidInput => ArchiveError::CorruptArchive,
        _ => ArchiveError::IoFailure,
    }
}

pub(crate) fn extract(
    mut file: File,
    snapshot: &ArchiveSnapshot,
    plan: &crate::planning::ExtractionPlan,
    limits: &ResourceLimits,
    control: &OperationControl,
    password: Option<&str>,
    output: &mut dyn crate::output::ArchiveOutput,
) -> Result<crate::output::ExtractOutcome> {
    preflight_payloads(&mut file, snapshot, limits, control)?;
    let mut directory_bytes = 0;
    for id in &plan.explicit_directories {
        control.checkpoint()?;
        let entry = snapshot
            .entries
            .get(usize::try_from(id.ordinal).map_err(|_| ArchiveError::ResourceLimit)?)
            .filter(|entry| entry.id == *id && entry.kind == EntryKind::Directory)
            .ok_or(ArchiveError::SourceChanged)?;
        stream_entry(
            &mut file,
            entry,
            limits,
            control,
            password,
            &mut directory_bytes,
            |_| Ok(()),
        )?;
    }
    crate::engine::ensure_revision(&mut file, &snapshot.revision, control)?;
    let relative = |name: &str| {
        plan.subfolder()
            .map_or_else(|| name.to_owned(), |root| format!("{root}/{name}"))
    };
    if let Some(root) = plan.subfolder() {
        control.checkpoint()?;
        output.ensure_directory(root)?;
    }
    for directory in plan.directories() {
        control.checkpoint()?;
        output.ensure_directory(&relative(directory))?;
    }
    let mut result = crate::output::ExtractOutcome {
        files_committed: 0,
        bytes_committed: 0,
        skipped_special: plan.skipped_special(),
    };
    let mut decoded = 0u64;
    for planned in plan.files() {
        control.checkpoint()?;
        let entry = snapshot
            .entries
            .get(usize::try_from(planned.id().ordinal).map_err(|_| ArchiveError::ResourceLimit)?)
            .filter(|entry| entry.id == planned.id())
            .ok_or(ArchiveError::SourceChanged)?;
        if entry.kind != EntryKind::File {
            return Err(ArchiveError::UnsafePath);
        }
        output.confirm_zip_entry(entry, limits)?;
        let Some(token) =
            output.begin_extracted_file(&relative(planned.relative_path()), entry.size)?
        else {
            continue;
        };
        let operation = (|| {
            let bytes = stream_entry(
                &mut file,
                entry,
                limits,
                control,
                password,
                &mut decoded,
                |chunk| output.write_file(token, chunk),
            )?;
            crate::engine::ensure_revision(&mut file, &snapshot.revision, control)?;
            control.checkpoint()?;
            entry.metadata.validate()?;
            output.set_file_metadata(token, entry.metadata)?;
            output.commit_file(token)?;
            Ok(bytes)
        })();
        match operation {
            Ok(bytes) => {
                result.files_committed += 1;
                result.bytes_committed += bytes;
            }
            Err(error) => {
                output.abort_file(token);
                return Err(error);
            }
        }
    }
    control.finish_progress(ArchiveProgress {
        bytes: result.bytes_committed,
        files: result.files_committed,
        total_bytes: plan.total_bytes(),
    });
    Ok(result)
}

enum Payload<R: Read> {
    Stored(std::io::Take<R>),
    Deflated(Box<Deflated<R>>),
    Bzip2(Box<super::zip_bzip2::Bzip2<R>>),
    Deflate64(Box<super::zip_deflate64::Deflate64<R>>),
    Lzma(Box<super::zip_lzma::Lzma<R>>),
}
impl<R: Read> Payload<R> {
    fn new(
        input: std::io::Take<R>,
        entry: &ArchiveEntry,
        limits: &ResourceLimits,
        control: OperationControl,
    ) -> Result<Self> {
        let method = compression_method(entry);
        Ok(if method == 0 {
            Self::Stored(input)
        } else if method == 9 {
            if limits.max_working_bytes < 1 << 20 {
                return Err(ArchiveError::ResourceLimit);
            }
            Self::Deflate64(Box::new(super::zip_deflate64::Deflate64::new(
                input, control,
            )))
        } else if method == 12 {
            Self::Bzip2(Box::new(super::zip_bzip2::Bzip2::new(input, control)))
        } else if method == 14 {
            Self::Lzma(Box::new(super::zip_lzma::Lzma::new(
                input,
                entry.size.ok_or(ArchiveError::CorruptArchive)?,
                entry
                    .zip
                    .as_ref()
                    .ok_or(ArchiveError::CorruptArchive)?
                    .flags,
                limits,
                control,
            )?))
        } else {
            Self::Deflated(Box::new(Deflated {
                input,
                decoder: flate2::Decompress::new(false),
                buffer: [0; 8192],
                start: 0,
                end: 0,
                finished: false,
                control,
            }))
        })
    }
    fn compressed_consumed(&self, packed: u64) -> u64 {
        match self {
            Self::Stored(input) => packed - input.limit(),
            Self::Deflated(input) => input.decoder.total_in(),
            Self::Bzip2(input) => input.consumed(),
            Self::Deflate64(input) => input.consumed(),
            Self::Lzma(input) => input.consumed(),
        }
    }
}
impl<R: Read> Read for Payload<R> {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Stored(input) => input.read(output),
            Self::Deflated(input) => input.read(output),
            Self::Bzip2(input) => input.read(output),
            Self::Deflate64(input) => input.read(output),
            Self::Lzma(input) => input.read(output),
        }
    }
}

struct Deflated<R: Read> {
    input: std::io::Take<R>,
    decoder: flate2::Decompress,
    buffer: [u8; 8192],
    start: usize,
    end: usize,
    finished: bool,
    control: OperationControl,
}
impl<R: Read> Read for Deflated<R> {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        if output.is_empty() || self.finished {
            return Ok(0);
        }
        loop {
            self.control.checkpoint().map_err(std::io::Error::other)?;
            if self.start == self.end {
                self.end = self.input.read(&mut self.buffer)?;
                self.start = 0;
            }
            let before_in = self.decoder.total_in();
            let before_out = self.decoder.total_out();
            let flush = if self.start == self.end {
                flate2::FlushDecompress::Finish
            } else {
                flate2::FlushDecompress::None
            };
            let status = self
                .decoder
                .decompress(&self.buffer[self.start..self.end], output, flush)
                .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))?;
            let consumed = (self.decoder.total_in() - before_in) as usize;
            let produced = (self.decoder.total_out() - before_out) as usize;
            self.start += consumed;
            if status == flate2::Status::StreamEnd {
                self.finished = true;
            }
            if produced > 0 || self.finished {
                return Ok(produced);
            }
            if consumed == 0 {
                return Err(std::io::ErrorKind::InvalidData.into());
            }
        }
    }
}

pub(crate) fn validate_local(file: &mut File, entry: &ArchiveEntry) -> Result<u64> {
    let location = entry.zip.as_ref().ok_or(ArchiveError::CorruptArchive)?;
    file.seek(SeekFrom::Start(location.header_offset))?;
    let mut local = [0u8; 30];
    read_exact(file, &mut local)?;
    if &local[..4] != b"PK\x03\x04"
        || u16_at(&local, 6) != location.flags
        || u16_at(&local, 8) != entry.method
        || entry.dos_modified != Some((u16_at(&local, 12), u16_at(&local, 10)))
    {
        return Err(ArchiveError::CorruptArchive);
    }
    let name_size = usize::from(u16_at(&local, 26));
    let extra_size = usize::from(u16_at(&local, 28));
    let data_offset = location
        .header_offset
        .checked_add(30 + name_size as u64 + extra_size as u64)
        .ok_or(ArchiveError::CorruptArchive)?;
    if data_offset
        .checked_add(entry.compressed_size.ok_or(ArchiveError::CorruptArchive)?)
        .is_none_or(|end| end > location.directory_offset)
    {
        return Err(ArchiveError::CorruptArchive);
    }
    let mut name = vec![0u8; name_size];
    let mut extra = vec![0u8; extra_size];
    read_exact(file, &mut name)?;
    read_exact(file, &mut extra)?;
    if name != entry.raw_name
        || super::zip_aes::descriptor(
            &extra,
            location.flags,
            entry.method,
            entry.compressed_size.ok_or(ArchiveError::CorruptArchive)?,
        )? != location.aes
    {
        return Err(ArchiveError::CorruptArchive);
    }
    let mut size = u64::from(u32_at(&local, 22));
    let mut packed = u64::from(u32_at(&local, 18));
    if size == 0xffff_ffff || packed == 0xffff_ffff {
        let value = extra_field(&extra, 1)?.ok_or(ArchiveError::CorruptArchive)?;
        let mut cursor = 0;
        for field in [&mut size, &mut packed] {
            if *field == 0xffff_ffff {
                *field = u64_at(
                    value
                        .get(cursor..cursor + 8)
                        .ok_or(ArchiveError::CorruptArchive)?,
                    0,
                );
                cursor += 8;
            }
        }
    }
    let descriptor = location.flags & 8 != 0;
    if (!descriptor || size != 0) && Some(size) != entry.size
        || (!descriptor || packed != 0) && Some(packed) != entry.compressed_size
        || (!descriptor || u32_at(&local, 14) != 0) && Some(u32_at(&local, 14)) != entry.crc32
    {
        return Err(ArchiveError::CorruptArchive);
    }
    Ok(data_offset)
}
