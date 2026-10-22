//! Allocation preflight for the locked public 7z API, before encoded-header/data decode.
use super::*;
use std::io::{Cursor, SeekFrom};

pub(crate) fn blocks(archive: &sevenz_rust2::Archive, limits: &ResourceLimits) -> Result<()> {
    use sevenz_rust2::EncoderMethod as Method;
    for block in &archive.blocks {
        let mut work = 0u64;
        for coder in &block.coders {
            let properties = coder.properties();
            let cost = match coder.encoder_method_id() {
                Method::ID_LZMA => {
                    if properties.len() != 5 || properties[0] >= 225 {
                        return Err(ArchiveError::CorruptArchive);
                    }
                    let dict = u32::from_le_bytes(
                        properties[1..5].try_into().expect("checked properties"),
                    );
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
        if work > limits.max_working_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
    }
    Ok(())
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
