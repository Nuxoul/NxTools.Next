//! RFC1952 streaming adapter. Header names/comments are untrusted and never retained.
use crate::{
    ArchiveEntry, ArchiveError, ArchiveOutcome, ArchiveProgress, ArchiveSnapshot, EntryId,
    EntryKind, NameProvenance, OperationControl, ResourceLimits, Result,
};
use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    path::Path,
};
use uuid::Uuid;

fn input_error(failure: std::io::Error) -> ArchiveError {
    if let Some(typed) = failure
        .get_ref()
        .and_then(|value| value.downcast_ref::<ArchiveError>())
    {
        return *typed;
    }
    match failure.kind() {
        std::io::ErrorKind::UnexpectedEof | std::io::ErrorKind::InvalidData => {
            ArchiveError::CorruptArchive
        }
        _ => ArchiveError::IoFailure,
    }
}
fn read_exact(input: &mut impl Read, bytes: &mut [u8]) -> Result<()> {
    input.read_exact(bytes).map_err(input_error)
}
fn header(
    input: &mut impl BufRead,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<Option<u32>> {
    control.checkpoint()?;
    if input.fill_buf().map_err(input_error)?.is_empty() {
        return Ok(None);
    }
    let mut fixed = [0u8; 10];
    read_exact(input, &mut fixed)?;
    if fixed[..3] != [0x1f, 0x8b, 8] || fixed[3] & 0xe0 != 0 {
        return Err(ArchiveError::CorruptArchive);
    }
    let budget = limits.max_metadata_bytes.min(65536);
    let mut count = 10u64;
    let mut checksum = crc32fast::Hasher::new();
    checksum.update(&fixed);
    let mut byte = || -> Result<u8> {
        count += 1;
        if count > budget {
            return Err(ArchiveError::ResourceLimit);
        }
        if count.is_multiple_of(4096) {
            control.checkpoint()?;
        }
        let mut value = [0];
        read_exact(input, &mut value)?;
        checksum.update(&value);
        Ok(value[0])
    };
    if fixed[3] & 4 != 0 {
        let length = u16::from_le_bytes([byte()?, byte()?]);
        for _ in 0..length {
            byte()?;
        }
    }
    for flag in [8, 16] {
        if fixed[3] & flag != 0 {
            while byte()? != 0 {}
        }
    }
    if fixed[3] & 2 != 0 {
        let expected = checksum.finalize() as u16;
        let mut crc = [0; 2];
        read_exact(input, &mut crc)?;
        if u16::from_le_bytes(crc) != expected {
            return Err(ArchiveError::CorruptArchive);
        }
    }
    if count > budget {
        return Err(ArchiveError::ResourceLimit);
    }
    Ok(Some(u32::from_le_bytes(
        fixed[4..8].try_into().expect("fixed MTIME"),
    )))
}
pub(crate) fn list(
    file: &mut File,
    path: &Path,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<Vec<ArchiveEntry>> {
    if limits.max_entries == 0 || limits.max_working_bytes < 256 * 1024 {
        return Err(ArchiveError::ResourceLimit);
    }
    let name = path
        .file_stem()
        .and_then(|name| name.to_str())
        .ok_or(ArchiveError::UnsafePath)?
        .to_owned();
    if (name.len() as u64).saturating_mul(8).saturating_add(1024) > limits.max_metadata_bytes {
        return Err(ArchiveError::ResourceLimit);
    }
    let packed = file.metadata()?.len();
    let modified =
        header(&mut BufReader::new(file), limits, control)?.ok_or(ArchiveError::CorruptArchive)?;
    let mut entries = vec![ArchiveEntry {
        id: EntryId {
            session: Uuid::new_v4(),
            ordinal: 0,
        },
        raw_name: name.as_bytes().to_vec(),
        display_path: name,
        provenance: NameProvenance::FormatUnicode,
        kind: EntryKind::File,
        size: None,
        compressed_size: Some(packed),
        method: 8,
        encrypted: false,
        crc32: None,
        dos_modified: None,
        metadata: crate::metadata::OutputMetadata {
            modified: (modified != 0).then_some(crate::metadata::ModifiedTime::UnixUtc {
                seconds: i64::from(modified),
                nanoseconds: 0,
                precision_ns: 1_000_000_000,
            }),
            attributes: None,
        },
        warnings: Vec::new(),
        zip: None,
    }];
    crate::path_policy::annotate(&mut entries, control, 1024, limits.max_metadata_bytes)?;
    Ok(entries)
}
pub(crate) fn stream(
    file: &mut impl Read,
    limits: &ResourceLimits,
    control: &OperationControl,
    chunk: impl FnMut(&[u8]) -> Result<()>,
) -> Result<ArchiveOutcome> {
    stream_members(file, limits, control, chunk).map(|(outcome, _)| outcome)
}
pub(crate) fn stream_members(
    file: &mut impl Read,
    limits: &ResourceLimits,
    control: &OperationControl,
    mut chunk: impl FnMut(&[u8]) -> Result<()>,
) -> Result<(ArchiveOutcome, u64)> {
    if limits.max_working_bytes < 256 * 1024 {
        return Err(ArchiveError::ResourceLimit);
    }
    let mut input = BufReader::with_capacity(8192, file);
    let mut total = 0u64;
    let mut members = 0u64;
    while header(&mut input, limits, control)?.is_some() {
        members += 1;
        if members > limits.max_entries {
            return Err(ArchiveError::ResourceLimit);
        }
        let mut decoder = flate2::Decompress::new(false);
        let mut crc = crc32fast::Hasher::new();
        let mut member_bytes = 0u64;
        let mut buffer = [0u8; 65536];
        loop {
            control.checkpoint()?;
            let compressed = input.fill_buf().map_err(input_error)?;
            let before_in = decoder.total_in();
            let before_out = decoder.total_out();
            let status = decoder
                .decompress(compressed, &mut buffer, flate2::FlushDecompress::None)
                .map_err(|_| ArchiveError::CorruptArchive)?;
            let consumed = (decoder.total_in() - before_in) as usize;
            let produced = (decoder.total_out() - before_out) as usize;
            input.consume(consumed);
            total = total
                .checked_add(produced as u64)
                .ok_or(ArchiveError::ResourceLimit)?;
            member_bytes = member_bytes
                .checked_add(produced as u64)
                .ok_or(ArchiveError::ResourceLimit)?;
            if total > limits.max_entry_bytes || total > limits.max_output_bytes {
                return Err(ArchiveError::ResourceLimit);
            }
            crc.update(&buffer[..produced]);
            if produced > 0 {
                chunk(&buffer[..produced])?;
            }
            control.publish(ArchiveProgress {
                bytes: total,
                files: 0,
                total_bytes: None,
            });
            if status == flate2::Status::StreamEnd {
                break;
            }
            if consumed == 0 && produced == 0 {
                return Err(ArchiveError::CorruptArchive);
            }
        }
        let mut trailer = [0u8; 8];
        read_exact(&mut input, &mut trailer)?;
        if u32::from_le_bytes(trailer[..4].try_into().expect("CRC field")) != crc.finalize()
            || u32::from_le_bytes(trailer[4..].try_into().expect("size field"))
                != member_bytes as u32
        {
            return Err(ArchiveError::CorruptArchive);
        }
    }
    if members == 0 {
        return Err(ArchiveError::CorruptArchive);
    }
    Ok((
        ArchiveOutcome {
            files_verified: 1,
            bytes_verified: total,
        },
        members,
    ))
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
    let planned = &plan.files()[0];
    if plan.files().len() != 1 || planned.id() != snapshot.entries[0].id {
        return Err(ArchiveError::SourceChanged);
    }
    if let Some(root) = plan.subfolder() {
        output.ensure_directory(root)?;
    }
    let relative = plan.subfolder().map_or_else(
        || planned.relative_path().to_owned(),
        |root| format!("{root}/{}", planned.relative_path()),
    );
    let token = output.begin_extracted_file(&relative, planned.size())?;
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
            output.set_file_metadata(token, snapshot.entries[0].metadata)?;
            output.commit_file(token)?;
        }
        Ok(verified)
    })();
    match result {
        Ok(verified) => Ok(crate::output::ExtractOutcome {
            files_committed: u64::from(token.is_some()),
            bytes_committed: if token.is_some() {
                verified.bytes_verified
            } else {
                0
            },
            skipped_special: 0,
        }),
        Err(failure) => {
            if let Some(token) = token {
                output.abort_file(token);
            }
            Err(failure)
        }
    }
}
