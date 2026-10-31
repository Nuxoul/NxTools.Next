//! Operation-private bounded crypto view. Seek replays traditional keys; no plaintext spool.
use crate::{ArchiveEntry, ArchiveError, OperationControl, Result};
use std::io::{self, Read, Seek, SeekFrom};
use zeroize::Zeroizing;

type Keys = Zeroizing<[u32; 3]>;
fn crc(mut value: u32, byte: u8) -> u32 {
    value ^= u32::from(byte);
    for _ in 0..8 {
        value = (value >> 1) ^ (0xedb8_8320 & 0u32.wrapping_sub(value & 1));
    }
    value
}
fn update(keys: &mut [u32; 3], byte: u8) {
    keys[0] = crc(keys[0], byte);
    keys[1] = keys[1]
        .wrapping_add(keys[0] & 255)
        .wrapping_mul(134775813)
        .wrapping_add(1);
    keys[2] = crc(keys[2], (keys[1] >> 24) as u8);
}
fn decrypt(keys: &mut [u32; 3], bytes: &mut [u8]) {
    for byte in bytes {
        let value = (keys[2] & 0xffff) | 2;
        *byte ^= (value.wrapping_mul(value ^ 1) >> 8) as u8;
        update(keys, *byte);
    }
}
pub(super) enum Reader<R> {
    Plain(super::zip_range::Range<R>, u64),
    Traditional(Classic<R>),
    Aes(Box<super::zip_aes::Aes<R>>),
}
pub(super) struct Classic<R> {
    input: super::zip_range::Range<R>,
    keys: Keys,
    initial: Keys,
    length: u64,
    position: u64,
    control: OperationControl,
}
impl<R: Read + Seek> Reader<R> {
    pub(super) fn new(
        input: R,
        offset: u64,
        packed: u64,
        entry: &ArchiveEntry,
        password: Option<&str>,
        control: &OperationControl,
    ) -> Result<Self> {
        let mut input = super::zip_range::Range::new(input, offset, packed)?;
        if !entry.encrypted {
            return Ok(Self::Plain(input, packed));
        }
        let flags = entry
            .zip
            .as_ref()
            .ok_or(ArchiveError::CorruptArchive)?
            .flags;
        if flags & 0x2040 != 0 {
            return Err(ArchiveError::UnsupportedEncryption);
        }
        let password = password.ok_or(ArchiveError::PasswordRequired)?;
        if let Some(descriptor) = entry.zip.as_ref().and_then(|location| location.aes) {
            return Ok(Self::Aes(Box::new(super::zip_aes::Aes::new(
                input, packed, descriptor, password, control,
            )?)));
        }
        let length = packed.checked_sub(12).ok_or(ArchiveError::CorruptArchive)?;
        let mut keys = Zeroizing::new([0x1234_5678, 0x2345_6789, 0x3456_7890]);
        for byte in password.bytes() {
            update(&mut keys, byte);
        }
        let mut header = Zeroizing::new([0; 12]);
        input
            .read_exact(&mut *header)
            .map_err(super::zip::payload_error)?;
        decrypt(&mut keys, &mut *header);
        let check = if flags & 8 != 0 {
            (entry.dos_modified.ok_or(ArchiveError::CorruptArchive)?.1 >> 8) as u8
        } else {
            (entry.crc32.ok_or(ArchiveError::CorruptArchive)? >> 24) as u8
        };
        if header[11] != check {
            return Err(ArchiveError::AuthenticationOrDataError);
        }
        let initial = Zeroizing::new(*keys);
        Ok(Self::Traditional(Classic {
            input,
            keys,
            initial,
            length,
            position: 0,
            control: control.clone(),
        }))
    }
    pub(super) fn length(&self) -> u64 {
        match self {
            Self::Plain(_, length) => *length,
            Self::Traditional(input) => input.length,
            Self::Aes(input) => input.length(),
        }
    }
}
impl<R: Read + Seek> Read for Reader<R> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Plain(input, _) => input.read(bytes),
            Self::Aes(input) => input.read(bytes),
            Self::Traditional(input) => {
                input.control.checkpoint().map_err(io::Error::other)?;
                let count = input.input.read(bytes)?;
                decrypt(&mut input.keys, &mut bytes[..count]);
                input.position += count as u64;
                Ok(count)
            }
        }
    }
}
impl<R: Read + Seek> Seek for Reader<R> {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let Self::Traditional(input) = self else {
            return match self {
                Self::Plain(input, _) => input.seek(from),
                Self::Aes(input) => input.seek(from),
                Self::Traditional(_) => unreachable!(),
            };
        };
        let target = match from {
            SeekFrom::Start(value) => i128::from(value),
            SeekFrom::End(value) => i128::from(input.length) + i128::from(value),
            SeekFrom::Current(value) => i128::from(input.position) + i128::from(value),
        };
        if target < 0 || target > i128::from(input.length) {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let target = target as u64;
        if target < input.position {
            input.input.seek(SeekFrom::Start(12))?;
            input.keys.copy_from_slice(&*input.initial);
            input.position = 0;
        }
        let mut buffer = Zeroizing::new([0; 8192]);
        while input.position < target {
            input.control.checkpoint().map_err(io::Error::other)?;
            let count = (target - input.position).min(8192) as usize;
            input.input.read_exact(&mut buffer[..count])?;
            decrypt(&mut input.keys, &mut buffer[..count]);
            input.position += count as u64;
        }
        Ok(input.position)
    }
}
