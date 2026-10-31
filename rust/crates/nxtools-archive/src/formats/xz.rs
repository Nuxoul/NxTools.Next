//! Preventive XZ structural/allocation admission before the locked public streaming decoder.
use crate::{
    ArchiveEntry, ArchiveError, ArchiveOutcome, ArchiveSnapshot, EntryId, EntryKind,
    NameProvenance, OperationControl, ResourceLimits, Result,
};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

pub(crate) fn list(
    file: &mut File,
    path: &Path,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<Vec<ArchiveEntry>> {
    list_for_output_preflight(file, path, limits, control, false)
}
pub(crate) fn list_for_output_preflight(
    file: &mut File,
    path: &Path,
    limits: &ResourceLimits,
    control: &OperationControl,
    declared_output: bool,
) -> Result<Vec<ArchiveEntry>> {
    let declared = scan(file, limits, control, !declared_output, false)?;
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
        // Index declarations are admission evidence, not verified payload sizes.
        size: declared_output.then_some(declared),
        compressed_size: Some(file.metadata()?.len()),
        method: 0x21, // XZ terminal LZMA2 filter ID, not a ZIP method number.
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
    output.confirm_zip_entry(&snapshot.entries[0], limits)?;
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

fn exact(input: &mut impl Read, bytes: &mut [u8]) -> Result<()> {
    input.read_exact(bytes).map_err(|failure| {
        if failure.kind() == std::io::ErrorKind::UnexpectedEof {
            ArchiveError::CorruptArchive
        } else {
            ArchiveError::IoFailure
        }
    })
}
fn at(file: &mut (impl Read + Seek), offset: u64, bytes: &mut [u8]) -> Result<()> {
    file.seek(SeekFrom::Start(offset))?;
    exact(file, bytes)
}
fn u32le(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes.try_into().expect("fixed u32 field"))
}
fn vli(bytes: &[u8], cursor: &mut usize) -> Result<u64> {
    let mut value = 0u64;
    for index in 0..9 {
        let byte = *bytes.get(*cursor).ok_or(ArchiveError::CorruptArchive)?;
        *cursor += 1;
        value |= u64::from(byte & 0x7f) << (index * 7);
        if byte & 0x80 == 0 {
            if index > 0 && byte == 0 {
                return Err(ArchiveError::CorruptArchive);
            }
            return Ok(value);
        }
    }
    Err(ArchiveError::CorruptArchive)
}
fn padded(value: u64) -> Result<u64> {
    value
        .checked_add(3)
        .map(|value| value & !3)
        .ok_or(ArchiveError::CorruptArchive)
}
fn block(
    file: &mut (impl Read + Seek),
    start: u64,
    unpadded: u64,
    expanded: u64,
    check: u8,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<()> {
    let mut first = [0];
    at(file, start, &mut first)?;
    let length = (usize::from(first[0]) + 1) * 4;
    if first[0] == 0 || length < 8 {
        return Err(ArchiveError::CorruptArchive);
    }
    let check_size = match check {
        0 => 0,
        1 => 4,
        4 => 8,
        10 => 32,
        _ => return Err(ArchiveError::UnsupportedMethod),
    };
    if unpadded < length as u64 + check_size + 1 {
        return Err(ArchiveError::CorruptArchive);
    }
    let mut header = [0u8; 1024];
    at(file, start, &mut header[..length])?;
    let body = &header[..length - 4];
    if u32le(&header[length - 4..length]) != crc32fast::hash(body) {
        return Err(ArchiveError::CorruptArchive);
    }
    let flags = body[1];
    if flags & 0x3c != 0 {
        return Err(ArchiveError::UnsupportedMethod);
    }
    let compressed = unpadded - length as u64 - check_size;
    let mut cursor = 2;
    if flags & 0x40 != 0 && vli(body, &mut cursor)? != compressed {
        return Err(ArchiveError::CorruptArchive);
    }
    if flags & 0x80 != 0 && vli(body, &mut cursor)? != expanded {
        return Err(ArchiveError::CorruptArchive);
    }
    let filters = (flags & 3) + 1;
    for index in 0..filters {
        let id = vli(body, &mut cursor)?;
        let size =
            usize::try_from(vli(body, &mut cursor)?).map_err(|_| ArchiveError::CorruptArchive)?;
        let end = cursor
            .checked_add(size)
            .filter(|end| *end <= body.len())
            .ok_or(ArchiveError::CorruptArchive)?;
        let props = &body[cursor..end];
        cursor = end;
        if index + 1 == filters {
            if id != 0x21 {
                return Err(ArchiveError::UnsupportedMethod);
            }
            if props.len() != 1 || props[0] > 40 {
                return Err(ArchiveError::CorruptArchive);
            }
            let dictionary = if props[0] == 40 {
                u32::MAX
            } else {
                (2 | u32::from(props[0] & 1)) << (props[0] / 2 + 11)
            };
            let memory =
                u64::from(lzma_rust2::lzma2_get_memory_usage(dictionary)) * 1024 + 256 * 1024;
            if memory > limits.max_working_bytes {
                return Err(ArchiveError::ResourceLimit);
            }
        } else {
            match id {
                3 if props.len() == 1 => {}
                4..=11 if props.is_empty() || props.len() == 4 => {}
                0x21 => return Err(ArchiveError::CorruptArchive),
                _ => return Err(ArchiveError::UnsupportedMethod),
            }
        }
    }
    if body[cursor..].iter().any(|byte| *byte != 0) {
        return Err(ArchiveError::CorruptArchive);
    }
    // LZMA2 control chunks delimit encoded bytes without allocating/decompressing dictionaries.
    let data_start = start + length as u64;
    let data_end = data_start + compressed;
    let mut position = data_start;
    let mut actual = 0u64;
    loop {
        control.checkpoint()?;
        if position >= data_end {
            return Err(ArchiveError::CorruptArchive);
        }
        let mut code = [0];
        at(file, position, &mut code)?;
        position += 1;
        if code[0] == 0 {
            if position != data_end || actual != expanded {
                return Err(ArchiveError::CorruptArchive);
            }
            break;
        }
        let (unpacked, packed) = if code[0] == 1 || code[0] == 2 {
            let mut size = [0; 2];
            at(file, position, &mut size)?;
            position += 2;
            let size = u64::from(u16::from_be_bytes(size)) + 1;
            (size, size)
        } else if code[0] >= 0x80 {
            let mut sizes = [0; 4];
            at(file, position, &mut sizes)?;
            position += 4;
            let unpacked = (u64::from(code[0] & 0x1f) << 16)
                + u64::from(u16::from_be_bytes([sizes[0], sizes[1]]))
                + 1;
            let packed = u64::from(u16::from_be_bytes([sizes[2], sizes[3]])) + 1;
            if code[0] >= 0xc0 {
                position += 1;
            }
            (unpacked, packed)
        } else {
            return Err(ArchiveError::CorruptArchive);
        };
        position = position
            .checked_add(packed)
            .filter(|end| *end < data_end)
            .ok_or(ArchiveError::CorruptArchive)?;
        actual = actual
            .checked_add(unpacked)
            .ok_or(ArchiveError::CorruptArchive)?;
        if actual > expanded {
            return Err(ArchiveError::CorruptArchive);
        }
    }
    let padding = (4 - unpadded % 4) % 4;
    if padding > 0 {
        let mut bytes = [0; 3];
        at(file, data_end, &mut bytes[..padding as usize])?;
        if bytes[..padding as usize].iter().any(|byte| *byte != 0) {
            return Err(ArchiveError::CorruptArchive);
        }
    }
    Ok(())
}
pub(crate) fn preflight(
    file: &mut (impl Read + Seek),
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<()> {
    scan(file, limits, control, true, false).map(|_| ())
}
pub(crate) fn edit_preflight(
    file: &mut (impl Read + Seek),
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<()> {
    let outer_limits = ResourceLimits {
        max_entry_bytes: limits.max_output_bytes,
        ..*limits
    };
    scan(file, &outer_limits, control, true, true).map(|_| ())
}
fn scan(
    file: &mut (impl Read + Seek),
    limits: &ResourceLimits,
    control: &OperationControl,
    enforce_output: bool,
    edit: bool,
) -> Result<u64> {
    if limits.max_working_bytes < 256 * 1024 {
        return Err(ArchiveError::ResourceLimit);
    }
    let mut end = file.seek(SeekFrom::End(0))?;
    let mut streams = 0u64;
    let mut records_total = 0u64;
    let mut expanded_total = 0u64;
    while end > 0 {
        control.checkpoint()?;
        let mut padding = 0u64;
        let mut byte = [0];
        while end > 0 {
            control.checkpoint()?;
            at(file, end - 1, &mut byte)?;
            if byte[0] != 0 {
                break;
            }
            end -= 1;
            padding += 1;
        }
        if !padding.is_multiple_of(4) || end < 24 {
            return Err(ArchiveError::CorruptArchive);
        }
        streams += 1;
        if edit && streams != 1 {
            return Err(ArchiveError::UnsupportedOperation);
        }
        if streams > limits.max_entries {
            return Err(ArchiveError::ResourceLimit);
        }
        let mut footer = [0; 12];
        at(file, end - 12, &mut footer)?;
        if &footer[10..] != b"YZ" || u32le(&footer[..4]) != crc32fast::hash(&footer[4..10]) {
            return Err(ArchiveError::CorruptArchive);
        }
        let index_size = (u64::from(u32le(&footer[4..8])) + 1) * 4;
        let index_start = end
            .checked_sub(12 + index_size)
            .ok_or(ArchiveError::CorruptArchive)?;
        if index_size < 8 {
            return Err(ArchiveError::CorruptArchive);
        }
        if index_size.saturating_mul(4) > limits.max_metadata_bytes
            || index_size > limits.max_working_bytes
        {
            return Err(ArchiveError::ResourceLimit);
        }
        let mut index = vec![0; index_size as usize];
        at(file, index_start, &mut index)?;
        let body = &index[..index.len() - 4];
        if body[0] != 0 || u32le(&index[index.len() - 4..]) != crc32fast::hash(body) {
            return Err(ArchiveError::CorruptArchive);
        }
        let mut cursor = 1;
        let count = vli(body, &mut cursor)?;
        if count > (body.len() - cursor) as u64 / 2 {
            return Err(ArchiveError::CorruptArchive);
        }
        records_total = records_total
            .checked_add(count)
            .ok_or(ArchiveError::ResourceLimit)?;
        if records_total > limits.max_entries
            || count.saturating_mul(64).saturating_add(index_size * 4) > limits.max_metadata_bytes
        {
            return Err(ArchiveError::ResourceLimit);
        }
        let mut records = Vec::new();
        let mut block_bytes = 0u64;
        for _ in 0..count {
            control.checkpoint()?;
            let packed = vli(body, &mut cursor)?;
            let size = vli(body, &mut cursor)?;
            if packed < 5 {
                return Err(ArchiveError::CorruptArchive);
            }
            block_bytes = block_bytes
                .checked_add(padded(packed)?)
                .ok_or(ArchiveError::CorruptArchive)?;
            expanded_total = expanded_total
                .checked_add(size)
                .ok_or(ArchiveError::ResourceLimit)?;
            records.push((packed, size));
        }
        if body[cursor..].iter().any(|byte| *byte != 0) {
            return Err(ArchiveError::CorruptArchive);
        }
        if enforce_output
            && (expanded_total > limits.max_entry_bytes || expanded_total > limits.max_output_bytes)
        {
            return Err(ArchiveError::ResourceLimit);
        }
        let start = index_start
            .checked_sub(
                block_bytes
                    .checked_add(12)
                    .ok_or(ArchiveError::CorruptArchive)?,
            )
            .ok_or(ArchiveError::CorruptArchive)?;
        let mut header = [0; 12];
        at(file, start, &mut header)?;
        if &header[..6] != b"\xfd7zXZ\0"
            || u32le(&header[8..]) != crc32fast::hash(&header[6..8])
            || header[6..8] != footer[8..10]
        {
            return Err(ArchiveError::CorruptArchive);
        }
        if header[6] != 0 || header[7] & 0xf0 != 0 {
            return Err(ArchiveError::UnsupportedMethod);
        }
        let mut offset = start + 12;
        for (packed, size) in records {
            if edit {
                let mut descriptor = [0; 2];
                at(file, offset, &mut descriptor)?;
                if descriptor[1] & 3 != 0 {
                    return Err(ArchiveError::UnsupportedOperation);
                }
            }
            block(file, offset, packed, size, header[7], limits, control)?;
            offset += padded(packed)?;
        }
        if offset != index_start {
            return Err(ArchiveError::CorruptArchive);
        }
        end = start;
    }
    if streams == 0 {
        return Err(ArchiveError::CorruptArchive);
    }
    file.rewind()?;
    Ok(expanded_total)
}
fn decode_error(failure: std::io::Error) -> ArchiveError {
    if let Some(typed) = failure
        .get_ref()
        .and_then(|value| value.downcast_ref::<ArchiveError>())
    {
        return *typed;
    }
    match failure.kind() {
        std::io::ErrorKind::OutOfMemory => ArchiveError::ResourceLimit,
        std::io::ErrorKind::Unsupported => ArchiveError::UnsupportedMethod,
        std::io::ErrorKind::InvalidData
        | std::io::ErrorKind::UnexpectedEof
        | std::io::ErrorKind::InvalidInput
        | std::io::ErrorKind::Other => ArchiveError::CorruptArchive,
        _ => ArchiveError::IoFailure,
    }
}
struct Controlled<'a, R: Read>(&'a mut R, &'a OperationControl);
impl<R: Read> Read for Controlled<'_, R> {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        self.1.checkpoint().map_err(std::io::Error::other)?;
        // Tag source I/O before it enters the codec, whose data errors may use Other.
        self.0.read(bytes).map_err(|failure| {
            if failure
                .get_ref()
                .is_some_and(|value| value.is::<ArchiveError>())
            {
                return failure;
            }
            std::io::Error::other(match failure.kind() {
                std::io::ErrorKind::OutOfMemory => ArchiveError::ResourceLimit,
                _ => ArchiveError::IoFailure,
            })
        })
    }
}
pub(crate) fn stream(
    file: &mut (impl Read + Seek),
    limits: &ResourceLimits,
    control: &OperationControl,
    mut chunk: impl FnMut(&[u8]) -> Result<()>,
) -> Result<ArchiveOutcome> {
    preflight(file, limits, control)?;
    let memory = ((limits.max_working_bytes - 256 * 1024) / 1024).min(u64::from(u32::MAX)) as u32;
    let mut reader = lzma_rust2::XzReader::new_mem_limit(Controlled(file, control), true, memory);
    let mut total = 0u64;
    let mut buffer = [0; 65536];
    loop {
        control.checkpoint()?;
        let count = reader.read(&mut buffer).map_err(decode_error)?;
        control.checkpoint()?;
        if count == 0 {
            break;
        }
        total = total
            .checked_add(count as u64)
            .ok_or(ArchiveError::ResourceLimit)?;
        if total > limits.max_entry_bytes || total > limits.max_output_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
        chunk(&buffer[..count])?;
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decoder_errors_keep_input_io_resource_and_cancellation_types() {
        struct Failing(std::io::ErrorKind);
        impl Read for Failing {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(self.0.into())
            }
        }
        let control = OperationControl::default();
        for (kind, expected) in [
            (std::io::ErrorKind::Other, ArchiveError::IoFailure),
            (
                std::io::ErrorKind::PermissionDenied,
                ArchiveError::IoFailure,
            ),
            (std::io::ErrorKind::OutOfMemory, ArchiveError::ResourceLimit),
        ] {
            let failure = Controlled(&mut Failing(kind), &control)
                .read(&mut [0])
                .expect_err("input failure");
            assert_eq!(decode_error(failure), expected);
        }
        struct Typed(ArchiveError);
        impl Read for Typed {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other(self.0))
            }
        }
        for expected in [
            ArchiveError::Cancelled,
            ArchiveError::ResourceLimit,
            ArchiveError::IoFailure,
        ] {
            assert_eq!(
                decode_error(
                    Controlled(&mut Typed(expected), &control)
                        .read(&mut [0])
                        .expect_err("typed input error")
                ),
                expected
            );
        }
        control.cancel();
        assert_eq!(
            decode_error(
                Controlled(&mut Failing(std::io::ErrorKind::Other), &control)
                    .read(&mut [0])
                    .expect_err("cancelled input")
            ),
            ArchiveError::Cancelled
        );
        assert_eq!(
            decode_error(std::io::Error::other("codec data error")),
            ArchiveError::CorruptArchive
        );
    }
    #[test]
    fn independent_fixture_passes_preflight_and_full_decoder() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        for name in ["independent.xz", "multi.xz", "independent.tar.xz"] {
            let mut file = File::open(root.join(name)).expect("independent XZ fixture");
            let limits = ResourceLimits::default();
            let control = OperationControl::default();
            assert_eq!(
                preflight(&mut file, &limits, &control),
                Ok(()),
                "preflight {name}"
            );
            let verified =
                stream(&mut file, &limits, &control, |_| Ok(())).expect("full independent decode");
            assert!(verified.bytes_verified() > 0);
        }
    }
    #[test]
    fn malformed_indexes_dictionaries_and_payloads_are_typed_and_bounded() {
        let root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/independent.xz");
        let original = std::fs::read(root).expect("independent XZ");
        let owned = tempfile::tempdir().expect("bounded mutations");
        for mode in [
            "footer",
            "backward",
            "index",
            "payload",
            "dictionary",
            "output",
            "padding",
            "overflow",
        ] {
            let mut bytes = original.clone();
            let footer = bytes.len() - 12;
            let index_size = (u32le(&bytes[footer + 4..footer + 8]) as usize + 1) * 4;
            let index = footer - index_size;
            if mode == "footer" {
                bytes[footer] ^= 1;
            }
            if mode == "backward" {
                bytes[footer + 4..footer + 8].copy_from_slice(&u32::MAX.to_le_bytes());
                let crc = crc32fast::hash(&bytes[footer + 4..footer + 10]);
                bytes[footer..footer + 4].copy_from_slice(&crc.to_le_bytes());
            }
            if mode == "index" {
                bytes[index + 1] = 0x7f;
                let crc = crc32fast::hash(&bytes[index..footer - 4]);
                bytes[footer - 4..footer].copy_from_slice(&crc.to_le_bytes());
            }
            if mode == "payload" {
                bytes[34] ^= 0x40;
            }
            if mode == "padding" {
                bytes.push(0);
            }
            if mode == "overflow" {
                let mut data = vec![0, 1];
                let mut value = u64::MAX >> 1;
                loop {
                    let mut byte = (value & 0x7f) as u8;
                    value >>= 7;
                    if value > 0 {
                        byte |= 0x80;
                    }
                    data.push(byte);
                    if value == 0 {
                        break;
                    }
                }
                data.push(1);
                while !data.len().is_multiple_of(4) {
                    data.push(0);
                }
                let crc = crc32fast::hash(&data);
                data.extend_from_slice(&crc.to_le_bytes());
                let mut footer_bytes = bytes[footer..].to_vec();
                footer_bytes[4..8].copy_from_slice(&((data.len() / 4 - 1) as u32).to_le_bytes());
                let crc = crc32fast::hash(&footer_bytes[4..10]);
                footer_bytes[..4].copy_from_slice(&crc.to_le_bytes());
                bytes.truncate(index);
                bytes.extend(data);
                bytes.extend(footer_bytes);
            }
            let path = owned.path().join(mode);
            std::fs::write(&path, bytes).expect("owned mutation");
            let mut file = File::open(path).expect("mutated input");
            let limits = ResourceLimits {
                max_working_bytes: if mode == "dictionary" {
                    1 << 20
                } else {
                    512 << 20
                },
                max_output_bytes: if mode == "output" { 1 } else { 1 << 20 },
                ..Default::default()
            };
            let failure = stream(&mut file, &limits, &OperationControl::default(), |_| Ok(()))
                .expect_err("must reject");
            assert_eq!(
                failure,
                if mode == "dictionary" || mode == "output" {
                    ArchiveError::ResourceLimit
                } else {
                    ArchiveError::CorruptArchive
                },
                "{mode}"
            );
        }
    }
}
