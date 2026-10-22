//! Standalone BZip2, with explicit member ends and no whole-stream in-memory buffer.
use crate::{
    ArchiveEntry, ArchiveError, ArchiveOutcome, ArchiveSnapshot, EntryId, EntryKind,
    NameProvenance, OperationControl, ResourceLimits, Result,
};
use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    path::Path,
};
use uuid::Uuid;
pub(crate) fn list(
    file: &mut File,
    path: &Path,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<Vec<ArchiveEntry>> {
    if limits.max_entries == 0 || limits.max_working_bytes < 8 << 20 {
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
    let mut signature = [0; 4];
    file.read_exact(&mut signature)
        .map_err(|_| ArchiveError::CorruptArchive)?;
    if &signature[..3] != b"BZh" || !(b'1'..=b'9').contains(&signature[3]) {
        return Err(ArchiveError::CorruptArchive);
    }
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
        compressed_size: Some(file.metadata()?.len()),
        method: 12,
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
pub(crate) fn stream(
    source: &mut File,
    limits: &ResourceLimits,
    control: &OperationControl,
    mut chunk: impl FnMut(&[u8]) -> Result<()>,
) -> Result<ArchiveOutcome> {
    if limits.max_working_bytes < 8 << 20 {
        return Err(ArchiveError::ResourceLimit);
    }
    let mut input = BufReader::with_capacity(8192, source);
    let mut total = 0u64;
    let mut members = 0u64;
    let mut buffer = [0u8; 65536];
    while !input.fill_buf()?.is_empty() {
        control.checkpoint()?;
        members += 1;
        if members > limits.max_entries {
            return Err(ArchiveError::ResourceLimit);
        }
        let mut decoder = bzip2::Decompress::new(true);
        loop {
            control.checkpoint()?;
            let compressed = input.fill_buf()?;
            let before_in = decoder.total_in();
            let before_out = decoder.total_out();
            let status = decoder
                .decompress(compressed, &mut buffer)
                .map_err(|_| ArchiveError::CorruptArchive)?;
            let consumed = (decoder.total_in() - before_in) as usize;
            let produced = (decoder.total_out() - before_out) as usize;
            input.consume(consumed);
            if status == bzip2::Status::MemNeeded {
                return Err(ArchiveError::ResourceLimit);
            }
            total = total
                .checked_add(produced as u64)
                .ok_or(ArchiveError::ResourceLimit)?;
            if total > limits.max_output_bytes || total > limits.max_entry_bytes {
                return Err(ArchiveError::ResourceLimit);
            }
            if produced > 0 {
                chunk(&buffer[..produced])?;
            }
            control.publish(crate::ArchiveProgress {
                bytes: total,
                files: 0,
                total_bytes: None,
            });
            if status == bzip2::Status::StreamEnd {
                break;
            }
            if consumed == 0 && produced == 0 {
                return Err(ArchiveError::CorruptArchive);
            }
        }
    }
    if members == 0 {
        return Err(ArchiveError::CorruptArchive);
    }
    Ok(ArchiveOutcome {
        files_verified: 1,
        bytes_verified: total,
    })
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
    if plan.files().len() != 1 || plan.files()[0].id() != snapshot.entries[0].id {
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
