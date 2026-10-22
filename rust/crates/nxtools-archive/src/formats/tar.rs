//! Fixed header parsing only; never tar::Archive::entries/unpack (extension allocations/native IO).
use crate::{
    ArchiveEntry, ArchiveError, ArchiveOutcome, ArchiveSnapshot, EntryId, EntryKind, EntryWarning,
    NameEncoding, NameProvenance, OperationControl, ResourceLimits, Result,
};
use std::{
    collections::{BTreeMap, HashMap},
    fs::File,
    io::{Read, Seek, SeekFrom},
};
use uuid::Uuid;
struct Record {
    entry: ArchiveEntry,
    offset: u64,
}
#[derive(Clone, Copy, Default)]
enum PaxModified {
    #[default]
    Unspecified,
    Set(crate::metadata::ModifiedTime),
    Delete,
}
#[derive(Default)]
struct Extensions {
    path: Option<Vec<u8>>,
    size: Option<u64>,
    modified: PaxModified,
}
fn corrupt(_: std::io::Error) -> ArchiveError {
    ArchiveError::CorruptArchive
}
fn exact(file: &mut File, bytes: &mut [u8]) -> Result<()> {
    file.read_exact(bytes).map_err(|failure| {
        if failure.kind() == std::io::ErrorKind::UnexpectedEof {
            ArchiveError::CorruptArchive
        } else {
            ArchiveError::IoFailure
        }
    })
}
fn pax(bytes: &[u8], extension: &mut Extensions) -> Result<()> {
    let mut cursor = 0;
    while cursor < bytes.len() {
        let space = bytes[cursor..]
            .iter()
            .position(|value| *value == b' ')
            .ok_or(ArchiveError::CorruptArchive)?
            + cursor;
        let length = std::str::from_utf8(&bytes[cursor..space])
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .ok_or(ArchiveError::CorruptArchive)?;
        let end = cursor
            .checked_add(length)
            .filter(|end| *end <= bytes.len() && *end > space + 1)
            .ok_or(ArchiveError::CorruptArchive)?;
        let value = &bytes[space + 1..end];
        if value.last() != Some(&b'\n') {
            return Err(ArchiveError::CorruptArchive);
        }
        let equal = value
            .iter()
            .position(|value| *value == b'=')
            .ok_or(ArchiveError::CorruptArchive)?;
        let key = &value[..equal];
        let data = &value[equal + 1..value.len() - 1];
        if key.starts_with(b"GNU.sparse") || key == b"SCHILY.realsize" {
            return Err(ArchiveError::UnsupportedMethod);
        }
        match key {
            b"path" => {
                std::str::from_utf8(data).map_err(|_| ArchiveError::CorruptArchive)?;
                extension.path = Some(data.to_vec());
            }
            b"size" => {
                extension.size = Some(
                    std::str::from_utf8(data)
                        .ok()
                        .and_then(|value| value.parse().ok())
                        .ok_or(ArchiveError::CorruptArchive)?,
                )
            }
            b"mtime" => {
                extension.modified = if data.is_empty() {
                    PaxModified::Delete
                } else {
                    PaxModified::Set(crate::metadata::unix_decimal(data))
                };
            }
            _ => {}
        }
        cursor = end;
    }
    Ok(())
}
/// Decode only the fixed mtime field; size and other numeric authority stay unchanged.
fn header_mtime(bytes: &[u8; 12]) -> Option<crate::metadata::ModifiedTime> {
    use crate::metadata::ModifiedTime;
    if bytes.iter().all(|byte| matches!(byte, 0 | b' ')) {
        return None;
    }
    let seconds = (|| {
        if bytes[0] & 0x80 != 0 {
            // GNU's marker is exactly 0x80 or 0xff. Consume ALL remaining bytes with
            // sign extension, never reinterpret only the low64 bits of an overflow.
            let initial = match bytes[0] {
                0x80 => 0i128,
                0xff => -1i128,
                _ => return None,
            };
            let value = bytes[1..].iter().try_fold(initial, |value, byte| {
                value.checked_mul(256)?.checked_add(i128::from(*byte))
            })?;
            i64::try_from(value).ok()
        } else {
            let start = bytes.iter().position(|byte| *byte != b' ')?;
            let end = bytes.iter().rposition(|byte| !matches!(byte, 0 | b' '))? + 1;
            bytes[start..end].iter().try_fold(0i64, |value, byte| {
                if !(b'0'..=b'7').contains(byte) {
                    return None;
                }
                value.checked_mul(8)?.checked_add(i64::from(byte - b'0'))
            })
        }
    })();
    Some(
        seconds.map_or(ModifiedTime::Invalid, |seconds| ModifiedTime::UnixUtc {
            seconds,
            nanoseconds: 0,
            precision_ns: 1_000_000_000,
        }),
    )
}
fn records(
    file: &mut File,
    encoding: NameEncoding,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<Vec<Record>> {
    if limits.max_working_bytes < 256 * 1024 || limits.max_metadata_bytes < 512 {
        return Err(ArchiveError::ResourceLimit);
    }
    let length = file.metadata()?.len();
    let session = Uuid::new_v4();
    let mut offset = 0u64;
    let mut items = Vec::new();
    let mut global = Extensions::default();
    let mut local = Extensions::default();
    let mut longname = None;
    let mut pending_extension = false;
    let mut extension_count = 0u64;
    let mut metadata = 0u64;
    loop {
        control.checkpoint()?;
        if offset.checked_add(512).is_none_or(|end| end > length) {
            return Err(ArchiveError::CorruptArchive);
        }
        file.seek(SeekFrom::Start(offset))?;
        let mut header = tar::Header::new_old();
        exact(file, header.as_mut_bytes())?;
        let raw = header.as_bytes();
        if raw.iter().all(|value| *value == 0) {
            if offset.checked_add(1024).is_none_or(|end| end > length)
                || local.path.is_some()
                || local.size.is_some()
                || longname.is_some()
                || pending_extension
            {
                return Err(ArchiveError::CorruptArchive);
            }
            let mut buffer = [0u8; 8192];
            loop {
                control.checkpoint()?;
                let count = file.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                if buffer[..count].iter().any(|value| *value != 0) {
                    return Err(ArchiveError::CorruptArchive);
                }
            }
            return Ok(items);
        }
        let checksum: u32 = raw
            .iter()
            .enumerate()
            .map(|(index, value)| {
                if (148..156).contains(&index) {
                    32
                } else {
                    u32::from(*value)
                }
            })
            .sum();
        if header.cksum().map_err(corrupt)? != checksum {
            return Err(ArchiveError::CorruptArchive);
        }
        let kind = header.entry_type();
        let extension = kind.is_gnu_longname()
            || kind.is_gnu_longlink()
            || kind.is_pax_local_extensions()
            || kind.is_pax_global_extensions();
        if kind.is_gnu_sparse() {
            return Err(ArchiveError::UnsupportedMethod);
        }
        let size = if extension {
            header.entry_size().map_err(corrupt)?
        } else {
            local
                .size
                .or(global.size)
                .unwrap_or(header.entry_size().map_err(corrupt)?)
        };
        let payload = offset
            .checked_add(512)
            .ok_or(ArchiveError::CorruptArchive)?;
        let end = payload
            .checked_add(size)
            .filter(|end| *end <= length)
            .ok_or(ArchiveError::CorruptArchive)?;
        let next = end
            .checked_add((512 - size % 512) % 512)
            .filter(|end| *end <= length)
            .ok_or(ArchiveError::CorruptArchive)?;
        if extension {
            if !kind.is_pax_global_extensions() {
                pending_extension = true;
            }
            extension_count += 1;
            if extension_count > limits.max_entries
                || size > 65536
                || size.saturating_mul(4) > limits.max_metadata_bytes
                || size > limits.max_working_bytes
            {
                return Err(ArchiveError::ResourceLimit);
            }
            let mut bytes = vec![0; size as usize];
            exact(file, &mut bytes)?;
            if kind.is_gnu_longname() {
                if longname.is_some() {
                    return Err(ArchiveError::CorruptArchive);
                }
                if bytes.last() == Some(&0) {
                    bytes.pop();
                }
                longname = Some(bytes);
            } else if kind.is_pax_local_extensions() {
                pax(&bytes, &mut local)?;
            } else if kind.is_pax_global_extensions() {
                pax(&bytes, &mut global)?;
            }
            offset = next;
            continue;
        }
        if items.len() as u64 >= limits.max_entries {
            return Err(ArchiveError::ResourceLimit);
        }
        let authoritative = local.path.take().or_else(|| global.path.clone());
        let raw_name = authoritative
            .clone()
            .or_else(|| longname.take())
            .unwrap_or_else(|| header.path_bytes().into_owned());
        longname = None;
        let modified = match local.modified {
            PaxModified::Set(time) => Some(time),
            // A local empty record deletes even the raw header value for this member.
            PaxModified::Delete => None,
            PaxModified::Unspecified => match global.modified {
                PaxModified::Set(time) => Some(time),
                // Global deletion removes the persistent override, restoring member headers.
                PaxModified::Delete | PaxModified::Unspecified => {
                    header_mtime(&header.as_old().mtime)
                }
            },
        };
        local = Extensions::default();
        pending_extension = false;
        metadata = metadata
            .checked_add(1024 + (raw_name.len() as u64).saturating_mul(32))
            .ok_or(ArchiveError::ResourceLimit)?;
        if metadata > limits.max_metadata_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
        let (display_path, provenance) = if authoritative.is_some() {
            (
                String::from_utf8(raw_name.clone()).map_err(|_| ArchiveError::CorruptArchive)?,
                NameProvenance::FormatUnicode,
            )
        } else {
            crate::encoding::decode(
                &raw_name,
                std::str::from_utf8(&raw_name).is_ok(),
                &[],
                encoding,
            )?
        };
        let entry_kind = if kind.is_file() || kind.is_contiguous() {
            EntryKind::File
        } else if kind.is_dir() {
            EntryKind::Directory
        } else if kind.is_symlink() || kind.is_hard_link() {
            EntryKind::Link
        } else {
            EntryKind::Special
        };
        if entry_kind == EntryKind::Directory && size != 0 {
            return Err(ArchiveError::CorruptArchive);
        }
        let warnings = if matches!(entry_kind, EntryKind::Link | EntryKind::Special) {
            vec![EntryWarning::LinkNotExtracted]
        } else {
            Vec::new()
        };
        items.push(Record {
            offset: payload,
            entry: ArchiveEntry {
                id: EntryId {
                    session,
                    ordinal: items.len() as u64,
                },
                raw_name,
                display_path,
                provenance,
                kind: entry_kind,
                size: Some(size),
                compressed_size: None,
                method: 0,
                encrypted: false,
                crc32: None,
                dos_modified: None,
                metadata: crate::metadata::OutputMetadata {
                    modified,
                    attributes: None,
                },
                warnings,
                zip: None,
            },
        });
        offset = next;
    }
}
pub(crate) fn list(
    file: &mut File,
    encoding: NameEncoding,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<Vec<ArchiveEntry>> {
    let mut entries: Vec<_> = records(file, encoding, limits, control)?
        .into_iter()
        .map(|record| record.entry)
        .collect();
    let metadata = entries
        .iter()
        .map(|entry| 1024 + (entry.raw_name.len() as u64) * 32)
        .sum();
    crate::path_policy::annotate(&mut entries, control, metadata, limits.max_metadata_bytes)?;
    Ok(entries)
}
fn validate(
    file: &mut File,
    snapshot: &ArchiveSnapshot,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<Vec<Record>> {
    let records = records(file, NameEncoding::Auto, limits, control)?;
    if records.len() != snapshot.entries.len()
        || records
            .iter()
            .zip(snapshot.entries.iter())
            .any(|(current, old)| {
                current.entry.raw_name != old.raw_name
                    || current.entry.size != old.size
                    || current.entry.kind != old.kind
                    || current.entry.metadata != old.metadata
            })
    {
        return Err(ArchiveError::SourceChanged);
    }
    Ok(records)
}
fn copy(
    file: &mut File,
    record: &Record,
    total: &mut u64,
    limits: &ResourceLimits,
    control: &OperationControl,
    mut chunk: impl FnMut(&[u8]) -> Result<()>,
) -> Result<u64> {
    let size = record.entry.size.ok_or(ArchiveError::CorruptArchive)?;
    if size > limits.max_entry_bytes {
        return Err(ArchiveError::ResourceLimit);
    }
    file.seek(SeekFrom::Start(record.offset))?;
    let mut remaining = size;
    let mut buffer = [0u8; 65536];
    while remaining > 0 {
        control.checkpoint()?;
        let length = remaining.min(buffer.len() as u64) as usize;
        exact(file, &mut buffer[..length])?;
        *total = total
            .checked_add(length as u64)
            .ok_or(ArchiveError::ResourceLimit)?;
        if *total > limits.max_output_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
        chunk(&buffer[..length])?;
        remaining -= length as u64;
    }
    Ok(size)
}
pub(crate) fn test(
    mut file: File,
    snapshot: &ArchiveSnapshot,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<ArchiveOutcome> {
    let records = validate(&mut file, snapshot, limits, control)?;
    let mut bytes = 0;
    let mut files = 0;
    for record in records
        .iter()
        .filter(|record| record.entry.kind == EntryKind::File)
    {
        copy(&mut file, record, &mut bytes, limits, control, |_| Ok(()))?;
        files += 1;
    }
    Ok(ArchiveOutcome {
        files_verified: files,
        bytes_verified: bytes,
    })
}
pub(crate) fn extract(
    mut file: File,
    snapshot: &ArchiveSnapshot,
    plan: &crate::planning::ExtractionPlan,
    limits: &ResourceLimits,
    control: &OperationControl,
    source: &mut File,
    output: &mut dyn crate::output::ArchiveOutput,
) -> Result<crate::output::ExtractOutcome> {
    let records = validate(&mut file, snapshot, limits, control)?;
    let by_ordinal: HashMap<_, _> = records
        .iter()
        .map(|record| (record.entry.id.ordinal, record))
        .collect();
    let relative = |path: &str| {
        plan.subfolder()
            .map_or_else(|| path.to_owned(), |root| format!("{root}/{path}"))
    };
    if let Some(root) = plan.subfolder() {
        output.ensure_directory(root)?;
    }
    for directory in plan.directories() {
        control.checkpoint()?;
        output.ensure_directory(&relative(directory))?;
    }
    let mut pending = BTreeMap::new();
    let mut bytes = 0;
    let mut written_bytes = 0;
    let result = (|| {
        for planned in plan.files() {
            control.checkpoint()?;
            let record = by_ordinal
                .get(&planned.id().ordinal)
                .ok_or(ArchiveError::SourceChanged)?;
            let token = output
                .begin_extracted_file(&relative(planned.relative_path()), record.entry.size)?;
            if let Some(token) = token {
                pending.insert(token, record.entry.metadata);
            }
            let count = copy(&mut file, record, &mut bytes, limits, control, |chunk| {
                if let Some(token) = token {
                    output.write_file(token, chunk)
                } else {
                    Ok(())
                }
            })?;
            if token.is_some() {
                written_bytes += count;
            }
        }
        crate::engine::ensure_revision(source, &snapshot.revision, control)?;
        let mut files = 0;
        for token in pending.keys().copied().collect::<Vec<_>>() {
            control.checkpoint()?;
            output.set_file_metadata(token, pending[&token])?;
            output.commit_file(token)?;
            pending.remove(&token);
            files += 1;
        }
        Ok(crate::output::ExtractOutcome {
            files_committed: files,
            bytes_committed: written_bytes,
            skipped_special: plan.skipped_special(),
        })
    })();
    for token in pending.keys() {
        output.abort_file(*token);
    }
    result
}

/// Creation verification shares the fixed-header parser and bounded payload copier; no new parser.
pub(crate) fn verify_created(
    file: &mut File,
    plan: &crate::creation::CreationPlan,
    digests: &[Option<[u8; 32]>],
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<()> {
    use sha2::{Digest, Sha256};
    let records = records(file, NameEncoding::Utf8, limits, control)?;
    if records.len() != plan.entries().len() || digests.len() != records.len() {
        return Err(ArchiveError::CorruptArchive);
    }
    let mut total = 0;
    for ((record, item), expected) in records.iter().zip(plan.entries()).zip(digests) {
        if record.entry.display_path() != item.relative_path()
            || record.entry.size() != Some(item.size())
            || record.entry.kind()
                != if item.is_directory() {
                    EntryKind::Directory
                } else {
                    EntryKind::File
                }
        {
            return Err(ArchiveError::CorruptArchive);
        }
        let mut digest = Sha256::new();
        copy(file, record, &mut total, limits, control, |bytes| {
            digest.update(bytes);
            Ok(())
        })?;
        if *expected != Some(digest.finalize().into()) {
            return Err(ArchiveError::CorruptArchive);
        }
    }
    Ok(())
}
