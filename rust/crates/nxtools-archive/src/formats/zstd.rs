//! Standard Zstandard frames: preventive window admission and explicitly ended bounded streams.
use crate::{
    ArchiveEntry, ArchiveError, ArchiveOutcome, ArchiveSnapshot, EntryId, EntryKind,
    NameProvenance, OperationControl, ResourceLimits, Result,
};
use std::{
    fs::File,
    io::{BufRead, BufReader, Read, Seek, SeekFrom},
    path::Path,
};

const MAGIC: u32 = 0xfd2fb528;
const FIXED_WORK: u64 = 8 << 20;

fn exact(file: &mut impl Read, bytes: &mut [u8]) -> Result<()> {
    file.read_exact(bytes)
        .map_err(|failure| match failure.kind() {
            std::io::ErrorKind::UnexpectedEof => ArchiveError::CorruptArchive,
            _ => ArchiveError::IoFailure,
        })
}
fn integer(file: &mut impl Read, length: usize) -> Result<u64> {
    let mut bytes = [0; 8];
    exact(file, &mut bytes[..length])?;
    Ok(u64::from_le_bytes(bytes))
}
fn skip(file: &mut impl Seek, bytes: u64, length: u64) -> Result<()> {
    let end = file
        .stream_position()?
        .checked_add(bytes)
        .filter(|end| *end <= length)
        .ok_or(ArchiveError::CorruptArchive)?;
    file.seek(SeekFrom::Start(end))?;
    Ok(())
}
fn window_log(limits: &ResourceLimits) -> Result<u32> {
    let available = limits
        .max_working_bytes
        .checked_sub(FIXED_WORK)
        .ok_or(ArchiveError::ResourceLimit)?
        / 2;
    if available < 1024 {
        return Err(ArchiveError::ResourceLimit);
    }
    Ok(available.ilog2().min(30))
}
fn charge(metadata: &mut u64, bytes: u64, limits: &ResourceLimits) -> Result<()> {
    *metadata = metadata
        .checked_add(bytes)
        .ok_or(ArchiveError::ResourceLimit)?;
    if *metadata > limits.max_metadata_bytes {
        return Err(ArchiveError::ResourceLimit);
    }
    Ok(())
}
pub(crate) fn preflight(
    file: &mut (impl Read + Seek),
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<u32> {
    let log = window_log(limits)?;
    let length = file.seek(SeekFrom::End(0))?;
    file.rewind()?;
    let mut frames = 0u64;
    let mut blocks = 0u64;
    let mut metadata = 0u64;
    let mut declared = 0u64;
    let mut standard = false;
    while file.stream_position()? < length {
        control.checkpoint()?;
        frames += 1;
        if frames > limits.max_entries {
            return Err(ArchiveError::ResourceLimit);
        }
        charge(&mut metadata, 64, limits)?;
        let magic = integer(file, 4)? as u32;
        if magic & 0xfffffff0 == 0x184d2a50 {
            let size = integer(file, 4)?;
            skip(file, size, length)?;
            continue;
        }
        if magic != MAGIC {
            return Err(ArchiveError::CorruptArchive);
        }
        standard = true;
        let descriptor = integer(file, 1)? as u8;
        if descriptor & 8 != 0 {
            return Err(ArchiveError::CorruptArchive);
        }
        let single = descriptor & 0x20 != 0;
        let mut window = if single {
            0
        } else {
            let descriptor = integer(file, 1)?;
            let base = 1u64 << (10 + (descriptor >> 3));
            base + (base / 8) * (descriptor & 7)
        };
        let dictionary_size = match descriptor & 3 {
            0 => 0,
            1 => 1,
            2 => 2,
            _ => 4,
        };
        if integer(file, dictionary_size)? != 0 {
            return Err(ArchiveError::UnsupportedMethod);
        }
        let size_length = match descriptor >> 6 {
            0 => usize::from(single),
            1 => 2,
            2 => 4,
            _ => 8,
        };
        let content_size = if size_length == 0 {
            None
        } else {
            let size = integer(file, size_length)? + if size_length == 2 { 256 } else { 0 };
            if single {
                window = size;
            }
            declared = declared
                .checked_add(size)
                .ok_or(ArchiveError::ResourceLimit)?;
            if declared > limits.max_entry_bytes || declared > limits.max_output_bytes {
                return Err(ArchiveError::ResourceLimit);
            }
            Some(size)
        };
        if window > (1u64 << log) {
            return Err(ArchiveError::ResourceLimit);
        }
        let mut raw_expanded = 0u64;
        loop {
            control.checkpoint()?;
            blocks += 1;
            if blocks > limits.max_entries {
                return Err(ArchiveError::ResourceLimit);
            }
            charge(&mut metadata, 32, limits)?;
            let header = integer(file, 3)?;
            let kind = (header >> 1) & 3;
            let size = header >> 3;
            if kind == 3 || size > window.min(128 << 10) {
                return Err(ArchiveError::CorruptArchive);
            }
            if kind < 2 {
                raw_expanded = raw_expanded
                    .checked_add(size)
                    .ok_or(ArchiveError::ResourceLimit)?;
            }
            skip(file, if kind == 1 { 1 } else { size }, length)?;
            if header & 1 != 0 {
                break;
            }
        }
        if content_size.is_some_and(|size| raw_expanded > size) {
            return Err(ArchiveError::CorruptArchive);
        }
        if descriptor & 4 != 0 {
            skip(file, 4, length)?;
        }
    }
    if !standard {
        return Err(ArchiveError::CorruptArchive);
    }
    file.rewind()?;
    Ok(log)
}

fn decode_error(code: usize) -> ArchiveError {
    use zstd_safe::zstd_sys::ZSTD_ErrorCode::*;
    // The exact native error_private.h uses size_t-negative stable enum values.
    let kind = code.wrapping_neg();
    if [
        ZSTD_error_memory_allocation,
        ZSTD_error_workSpace_tooSmall,
        ZSTD_error_frameParameter_windowTooLarge,
    ]
    .iter()
    .any(|value| kind == *value as usize)
    {
        ArchiveError::ResourceLimit
    } else if [
        ZSTD_error_dictionary_wrong,
        ZSTD_error_dictionary_corrupted,
        ZSTD_error_version_unsupported,
        ZSTD_error_frameParameter_unsupported,
        ZSTD_error_parameter_unsupported,
    ]
    .iter()
    .any(|value| kind == *value as usize)
    {
        ArchiveError::UnsupportedMethod
    } else {
        ArchiveError::CorruptArchive
    }
}
pub(crate) fn stream(
    file: &mut (impl Read + Seek),
    limits: &ResourceLimits,
    control: &OperationControl,
    mut chunk: impl FnMut(&[u8]) -> Result<()>,
) -> Result<ArchiveOutcome> {
    let log = preflight(file, limits, control)?;
    let mut decoder = zstd_safe::DCtx::try_create().ok_or(ArchiveError::ResourceLimit)?;
    decoder
        .set_parameter(zstd_safe::DParameter::WindowLogMax(log))
        .map_err(decode_error)?;
    let mut input = BufReader::with_capacity(65536, file);
    let mut buffer = [0u8; 65536];
    let mut total = 0u64;
    let mut frame_end = false;
    loop {
        control.checkpoint()?;
        let available = input.fill_buf()?;
        let eof = available.is_empty();
        if eof && frame_end {
            break;
        }
        let mut compressed = zstd_safe::InBuffer::around(available);
        let mut expanded = zstd_safe::OutBuffer::around(&mut buffer[..]);
        let hint = decoder
            .decompress_stream(&mut expanded, &mut compressed)
            .map_err(decode_error)?;
        let consumed = compressed.pos();
        let produced = expanded.pos();
        input.consume(consumed);
        control.checkpoint()?;
        frame_end = hint == 0;
        total = total
            .checked_add(produced as u64)
            .ok_or(ArchiveError::ResourceLimit)?;
        if total > limits.max_entry_bytes || total > limits.max_output_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
        if produced > 0 {
            chunk(&buffer[..produced])?;
        }
        let stalled = if eof { !frame_end } else { consumed == 0 };
        if produced == 0 && stalled {
            return Err(ArchiveError::CorruptArchive);
        }
        control.publish(crate::ArchiveProgress {
            bytes: total,
            files: 0,
            total_bytes: None,
        });
    }
    Ok(ArchiveOutcome {
        files_verified: 1,
        bytes_verified: total,
    })
}

pub(crate) fn list(
    file: &mut File,
    path: &Path,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<Vec<ArchiveEntry>> {
    preflight(file, limits, control)?;
    let name = path
        .file_stem()
        .and_then(|name| name.to_str())
        .ok_or(ArchiveError::UnsafePath)?
        .to_owned();
    if (name.len() as u64).saturating_mul(8).saturating_add(1024) > limits.max_metadata_bytes {
        return Err(ArchiveError::ResourceLimit);
    }
    let mut entries = vec![ArchiveEntry {
        id: EntryId {
            session: uuid::Uuid::new_v4(),
            ordinal: 0,
        },
        raw_name: name.as_bytes().to_vec(),
        display_path: name,
        provenance: NameProvenance::FormatUnicode,
        kind: EntryKind::File,
        size: None,
        compressed_size: Some(file.metadata()?.len()),
        method: 0,
        encrypted: false,
        crc32: None,
        dos_modified: None,
        metadata: Default::default(),
        warnings: Vec::new(),
        zip: None,
    }];
    crate::path_policy::annotate(&mut entries, control, 1024, limits.max_metadata_bytes)?;
    Ok(entries)
}
pub(crate) fn extract(
    mut file: File,
    snapshot: &ArchiveSnapshot,
    plan: &crate::planning::ExtractionPlan,
    limits: &ResourceLimits,
    control: &OperationControl,
    output: &mut dyn crate::output::ArchiveOutput,
) -> Result<crate::output::ExtractOutcome> {
    if plan.files().is_empty() {
        return Ok(crate::output::ExtractOutcome {
            files_committed: 0,
            bytes_committed: 0,
            skipped_special: 0,
        });
    }
    if plan.files().len() != 1
        || snapshot.entries.len() != 1
        || plan.files()[0].id() != snapshot.entries[0].id
    {
        return Err(ArchiveError::SourceChanged);
    }
    if let Some(root) = plan.subfolder() {
        output.ensure_directory(root)?;
    }
    let name = plan.subfolder().map_or_else(
        || plan.files()[0].relative_path().to_owned(),
        |root| format!("{root}/{}", plan.files()[0].relative_path()),
    );
    let token = output.begin_extracted_file(&name, plan.files()[0].size())?;
    let result = (|| {
        let verified = stream(&mut file, limits, control, |bytes| {
            if let Some(token) = token {
                output.write_file(token, bytes)
            } else {
                Ok(())
            }
        })?;
        crate::engine::ensure_revision(&mut file, &snapshot.revision, control)?;
        control.checkpoint()?;
        if let Some(token) = token {
            output.commit_file(token)?;
        }
        Ok(crate::output::ExtractOutcome {
            files_committed: u64::from(token.is_some()),
            bytes_committed: if token.is_some() {
                verified.bytes_verified()
            } else {
                0
            },
            skipped_special: 0,
        })
    })();
    if result.is_err()
        && let Some(token) = token
    {
        output.abort_file(token);
    }
    result
}
