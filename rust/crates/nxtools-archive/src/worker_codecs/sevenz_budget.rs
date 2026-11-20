//! Allocation preflight for the locked public 7z API, before encoded-header/data decode.
use super::*;
use std::io::{Cursor, SeekFrom};

pub(crate) fn blocks(archive: &sevenz_rust2::Archive, limits: &ResourceLimits) -> Result<()> {
    for block in &archive.blocks {
        if block_work(block)? > limits.max_working_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
    }
    Ok(())
}

/// Catalog admission checks coder declarations without reserving payload decoder workspace.
pub(crate) fn validate_blocks(archive: &sevenz_rust2::Archive) -> Result<()> {
    for block in &archive.blocks {
        block_work(block)?;
    }
    Ok(())
}

fn block_work(block: &sevenz_rust2::Block) -> Result<u64> {
    use sevenz_rust2::EncoderMethod as Method;
    let mut work = 0u64;
    for coder in &block.coders {
        let properties = coder.properties();
        let cost = match coder.encoder_method_id() {
            Method::ID_LZMA => {
                if properties.len() != 5 || properties[0] >= 225 {
                    return Err(ArchiveError::CorruptArchive);
                }
                let dict =
                    u32::from_le_bytes(properties[1..5].try_into().expect("checked properties"));
                let lc = u32::from(properties[0] % 9);
                let lp = u32::from(properties[0] / 9 % 5);
                // Locked lzma-rust2 literal probability table: 0x300 u16 per lc+lp.
                u64::from(dict.max(4096)) + (1536u64 << (lc + lp)) + (64 << 10)
            }
            Method::ID_LZMA2 => {
                let prop = *properties.first().ok_or(ArchiveError::CorruptArchive)?;
                if properties.len() != 1 || prop > 40 {
                    return Err(ArchiveError::CorruptArchive);
                }
                let dict = if prop == 40 {
                    u64::from(u32::MAX)
                } else {
                    (2u64 | u64::from(prop & 1)) << (u32::from(prop) / 2 + 11)
                };
                dict + (4 << 20) // single-thread LZMA2 range/literal state and input buffers
            }
            Method::ID_PPMD => {
                if properties.len() != 5 {
                    return Err(ArchiveError::CorruptArchive);
                }
                let memory = u32::from_le_bytes(
                    properties[1..5]
                        .try_into()
                        .expect("checked PPMd properties"),
                );
                u64::from(memory) + (1 << 20)
            }
            Method::ID_BZIP2 => 8 << 20,
            Method::ID_DEFLATE | Method::ID_BCJ2 => 1 << 20,
            Method::ID_COPY
            | Method::ID_AES256_SHA256
            | Method::ID_DELTA
            | Method::ID_BCJ_X86
            | Method::ID_BCJ_ARM
            | Method::ID_BCJ_ARM64
            | Method::ID_BCJ_ARM_THUMB
            | Method::ID_BCJ_PPC
            | Method::ID_BCJ_SPARC
            | Method::ID_BCJ_IA64 => 1 << 20,
            _ => return Err(ArchiveError::UnsupportedMethod),
        };
        work = work.checked_add(cost).ok_or(ArchiveError::ResourceLimit)?;
    }
    Ok(work)
}

pub(crate) fn encoded_header(
    source: &mut (impl Read + Seek),
    header: &[u8; 32],
    limits: &ResourceLimits,
) -> Result<()> {
    let offset = u64::from_le_bytes(header[12..20].try_into().expect("fixed header"));
    let size = u64::from_le_bytes(header[20..28].try_into().expect("fixed header"));
    // The caller has already bounded this extent and size. Only metadata is buffered.
    if size
        .checked_mul(16)
        .is_none_or(|cost| cost > limits.max_metadata_bytes)
    {
        return Err(ArchiveError::ResourceLimit);
    }
    source.seek(SeekFrom::Start(32 + offset))?;
    let mut next = vec![0u8; usize::try_from(size).map_err(|_| ArchiveError::ResourceLimit)?];
    source
        .read_exact(&mut next)
        .map_err(|_| ArchiveError::CorruptArchive)?;
    if crc32fast::hash(&next) != u32::from_le_bytes(header[28..32].try_into().expect("fixed CRC")) {
        return Err(ArchiveError::CorruptArchive);
    }
    if next.first() != Some(&0x17) {
        return Ok(());
    }
    // Public Archive::read can parse streams without decoding: wrap the encoded-header
    // StreamsInfo as a normal Header/MainStreamsInfo, with no FilesInfo or packed payload.
    let mut metadata = Vec::with_capacity(next.len() + 2);
    metadata.extend_from_slice(&[0x01, 0x04]);
    metadata.extend_from_slice(&next[1..]);
    metadata.push(0);
    let mut signature = *header;
    signature[12..20].copy_from_slice(&0u64.to_le_bytes());
    signature[20..28].copy_from_slice(&(metadata.len() as u64).to_le_bytes());
    signature[28..32].copy_from_slice(&crc32fast::hash(&metadata).to_le_bytes());
    let crc = crc32fast::hash(&signature[12..32]);
    signature[8..12].copy_from_slice(&crc.to_le_bytes());
    let mut synthetic = Cursor::new(signature.to_vec());
    synthetic.get_mut().extend_from_slice(&metadata);
    let archive = sevenz_rust2::Archive::read(&mut synthetic, &sevenz_rust2::Password::default())
        .map_err(sevenz_error)?;
    blocks(&archive, limits)?;
    let mut packed = 32u64
        .checked_add(archive.pack_pos())
        .ok_or(ArchiveError::CorruptArchive)?;
    let source_size = source.seek(SeekFrom::End(0))?;
    for size in archive.pack_sizes() {
        packed = packed
            .checked_add(*size)
            .ok_or(ArchiveError::CorruptArchive)?;
        if packed > source_size {
            return Err(ArchiveError::CorruptArchive);
        }
    }
    for block in &archive.blocks {
        if block
            .get_unpack_size()
            .checked_mul(16)
            .is_none_or(|cost| cost > limits.max_metadata_bytes)
        {
            return Err(ArchiveError::ResourceLimit);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &[u8] = include_bytes!("../../tests/fixtures/sevenz-workspace.7z");

    struct HeaderOnly(Cursor<&'static [u8]>);
    impl Read for HeaderOnly {
        fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
            // This independent fixture's packed payload occupies [32, 47).
            let start = self.0.position();
            let end = start + bytes.len() as u64;
            assert!(
                bytes.is_empty() || end <= 32 || start >= 47,
                "payload read during metadata admission"
            );
            self.0.read(bytes)
        }
    }
    impl Seek for HeaderOnly {
        fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
            self.0.seek(position)
        }
    }

    #[test]
    fn metadata_admission_reads_only_headers_and_preserves_payload_workspace_threshold() {
        let limits = ResourceLimits {
            max_working_bytes: 64 << 20,
            ..Default::default()
        };
        let mut source = HeaderOnly(Cursor::new(FIXTURE));
        let mut header = [0; 32];
        source.read_exact(&mut header).expect("signature");
        encoded_header(&mut source, &header, &limits).expect("small encoded-header workspace");
        source.rewind().expect("catalog rewind");
        let reader = sevenz_rust2::ArchiveReader::new(source, sevenz_rust2::Password::default())
            .expect("metadata-only reader");
        let archive = reader.archive();
        assert_eq!(archive.files.len(), 2);
        validate_blocks(archive).expect("valid payload properties without payload allocation");
        for (mib, expected) in [
            (64, Err(ArchiveError::ResourceLimit)),
            (195, Err(ArchiveError::ResourceLimit)),
            (196, Ok(())),
        ] {
            assert_eq!(
                blocks(
                    archive,
                    &ResourceLimits {
                        max_working_bytes: mib << 20,
                        ..limits
                    }
                ),
                expected
            );
        }
    }

    #[test]
    fn metadata_validation_rejects_invalid_and_unsupported_payload_coders() {
        for (bytes, expected) in [
            (
                include_bytes!("../../tests/fixtures/sevenz-invalid-properties.7z").as_slice(),
                ArchiveError::CorruptArchive,
            ),
            (
                include_bytes!("../../tests/fixtures/sevenz-unsupported-coder.7z").as_slice(),
                ArchiveError::UnsupportedMethod,
            ),
        ] {
            let reader = sevenz_rust2::ArchiveReader::new(
                Cursor::new(bytes),
                sevenz_rust2::Password::default(),
            )
            .expect("catalog before payload validation");
            assert_eq!(validate_blocks(reader.archive()), Err(expected));
        }
    }
}
