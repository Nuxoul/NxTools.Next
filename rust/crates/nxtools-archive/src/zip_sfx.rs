//! Product-owned ZIP-SFX framing; payloads remain owned by the original ZIP reader.
use crate::{ArchiveError, ResourceLimits, Result, zip_container::Directory};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
};

pub(crate) const STUB: &[u8] = include_bytes!("../assets/zip-sfx-x64.bin");
pub(crate) const PREFIX_BYTES: u64 = STUB.len() as u64 + 48;
pub(crate) const MAX_EXE_BYTES: u64 = u32::MAX as u64;
pub(crate) struct Descriptor {
    pub prefix: u64,
    pub disks: u32,
}
fn word(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(bytes[at..at + 2].try_into().expect("bounded field"))
}
fn dword(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().expect("bounded field"))
}
fn qword(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(bytes[at..at + 8].try_into().expect("bounded field"))
}
fn at<const N: usize>(file: &mut File, offset: u64) -> Result<[u8; N]> {
    file.seek(SeekFrom::Start(offset))?;
    let mut bytes = [0; N];
    file.read_exact(&mut bytes).map_err(|failure| {
        if failure.kind() == std::io::ErrorKind::UnexpectedEof {
            ArchiveError::CorruptArchive
        } else {
            ArchiveError::IoFailure
        }
    })?;
    Ok(bytes)
}
fn require(valid: bool) -> Result<()> {
    if valid {
        Ok(())
    } else {
        Err(ArchiveError::CorruptArchive)
    }
}

pub(crate) fn descriptor(file: &mut File, limits: &ResourceLimits) -> Result<Option<Descriptor>> {
    let length = file.metadata()?.len();
    if length < 2 || at::<2>(file, 0)? != *b"MZ" {
        return Ok(None);
    }
    if length > MAX_EXE_BYTES {
        return Err(ArchiveError::ResourceLimit);
    }
    if limits.max_metadata_bytes < 64 {
        return Err(ArchiveError::ResourceLimit);
    }
    let dos = at::<64>(file, 0)?;
    let pe = u64::from(dword(&dos, 60));
    require((64..=1024 * 1024).contains(&pe))?;
    let coff = at::<24>(file, pe)?;
    require(coff[..4] == *b"PE\0\0" && word(&coff, 4) == 0x8664)?;
    let sections = word(&coff, 6);
    let optional_bytes = word(&coff, 20);
    require((1..=96).contains(&sections) && optional_bytes >= 240)?;
    if 64 + 24 + 240 + u64::from(sections) * 40 + 48 > limits.max_metadata_bytes {
        return Err(ArchiveError::ResourceLimit);
    }
    let optional = at::<240>(file, pe + 24)?;
    require(
        word(&optional, 0) == 0x20b && dword(&optional, 108) <= 16 && qword(&optional, 144) == 0,
    )?;
    let headers = u64::from(dword(&optional, 60));
    let section_start = pe + 24 + u64::from(optional_bytes);
    require(section_start + u64::from(sections) * 40 <= headers && headers <= length)?;
    let mut end = headers;
    for index in 0..sections {
        let section = at::<40>(file, section_start + u64::from(index) * 40)?;
        let size = u64::from(dword(&section, 16));
        let offset = u64::from(dword(&section, 20));
        if size != 0 {
            require(offset >= headers)?;
            end = end.max(offset + size);
        }
    }
    require(end <= length)?;
    if end + 48 > length {
        return Ok(None);
    }
    let bytes = at::<48>(file, end)?;
    if bytes[..16] != *b"NXTOOLS-ZIP-SFX1" {
        return Ok(None);
    }
    require(
        dword(&bytes, 16) == 1
            && dword(&bytes, 20) == 48
            && dword(&bytes, 28) == 0
            && qword(&bytes, 32) == end + 48
            && qword(&bytes, 40) == 0,
    )?;
    let disks = dword(&bytes, 24);
    require(disks > 0)?;
    if disks > 1024 {
        return Err(ArchiveError::ResourceLimit);
    }
    Ok(Some(Descriptor {
        prefix: end + 48,
        disks,
    }))
}

pub(crate) fn descriptor_bytes(disks: u32) -> [u8; 48] {
    let mut bytes = [0; 48];
    bytes[..16].copy_from_slice(b"NXTOOLS-ZIP-SFX1");
    bytes[16..20].copy_from_slice(&1u32.to_le_bytes());
    bytes[20..24].copy_from_slice(&48u32.to_le_bytes());
    bytes[24..28].copy_from_slice(&disks.to_le_bytes());
    bytes[32..40].copy_from_slice(&PREFIX_BYTES.to_le_bytes());
    bytes
}

pub(crate) fn metadata_admission(central_bytes: u64, count: usize) -> Result<()> {
    if count > 65536
        || central_bytes
            .checked_mul(8)
            .and_then(|bytes| bytes.checked_add(count as u64 * 512))
            .and_then(|bytes| bytes.checked_add(256 << 10))
            .is_none_or(|bytes| bytes > 64 << 20)
    {
        return Err(ArchiveError::ResourceLimit);
    }
    Ok(())
}
pub(crate) fn profile(directory: &Directory) -> Result<()> {
    let mut central_bytes = 0u64;
    for row in &directory.records {
        let raw = &row.central;
        let name_bytes = usize::from(word(raw, 28));
        if name_bytes > 4096 {
            return Err(ArchiveError::ResourceLimit);
        }
        let name = &raw[46..46 + name_bytes];
        let flags = word(raw, 8);
        let method = word(raw, 10);
        let mode = (dword(raw, 38) >> 16) & 0xf000;
        if flags & !0x080e != 0
            || !matches!(method, 0 | 8)
            || word(raw, 6) > 45
            || method == 0 && flags & 6 != 0
            || flags & 0x800 == 0 && !name.is_ascii()
            || std::str::from_utf8(name).is_err()
            || !matches!(mode, 0 | 0x4000 | 0x8000)
            || dword(raw, 38) & 0x440 != 0
        {
            return Err(ArchiveError::UnsupportedOperation);
        }
        central_bytes = central_bytes
            .checked_add(raw.len() as u64)
            .ok_or(ArchiveError::ResourceLimit)?;
    }
    metadata_admission(central_bytes, directory.records.len())
}
