//! WinZip AE-1/AE-2 framing and authenticated, seekable, bounded payload view.
use crate::{ArchiveError, OperationControl, Result, model::ZipAes};
use aes::cipher::{BlockEncrypt, KeyInit};
use hmac::Mac;
use std::io::{self, Read, Seek, SeekFrom};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

pub(super) fn descriptor(
    extra: &[u8],
    flags: u16,
    method: u16,
    packed: u64,
) -> Result<Option<ZipAes>> {
    // PKWARE strong encryption and masked/encrypted headers are not WinZip AES.
    if flags & 0x2040 != 0 {
        return Err(ArchiveError::UnsupportedEncryption);
    }
    let mut cursor = 0;
    let mut found = None;
    while cursor < extra.len() {
        let header = extra
            .get(cursor..cursor + 4)
            .ok_or(ArchiveError::CorruptArchive)?;
        let tag = u16::from_le_bytes([header[0], header[1]]);
        let size = usize::from(u16::from_le_bytes([header[2], header[3]]));
        cursor += 4;
        let field = extra
            .get(cursor..cursor + size)
            .ok_or(ArchiveError::CorruptArchive)?;
        cursor += size;
        if tag == 0x9901 && found.replace(field).is_some() {
            return Err(ArchiveError::CorruptArchive);
        }
    }
    if method != 99 {
        return if found.is_some() {
            Err(ArchiveError::CorruptArchive)
        } else {
            Ok(None)
        };
    }
    let field = found.ok_or(ArchiveError::CorruptArchive)?;
    if flags & 1 == 0 || field.len() != 7 || &field[2..4] != b"AE" {
        return Err(ArchiveError::CorruptArchive);
    }
    let version = u16::from_le_bytes([field[0], field[1]]);
    let strength = field[4];
    if !matches!(version, 1 | 2) || !matches!(strength, 1..=3) {
        return Err(ArchiveError::UnsupportedEncryption);
    }
    let method = u16::from_le_bytes([field[5], field[6]]);
    if method == 99 {
        return Err(ArchiveError::UnsupportedMethod);
    }
    if packed < u64::from((strength + 1) * 4) + 12 {
        return Err(ArchiveError::CorruptArchive);
    }
    Ok(Some(ZipAes {
        version,
        strength,
        method,
    }))
}

enum Cipher {
    A128(aes::Aes128),
    A192(aes::Aes192),
    A256(aes::Aes256),
}
impl Cipher {
    fn stream_block(&self, counter: u128) -> [u8; 16] {
        let mut block = counter.to_le_bytes().into();
        match self {
            Self::A128(cipher) => cipher.encrypt_block(&mut block),
            Self::A192(cipher) => cipher.encrypt_block(&mut block),
            Self::A256(cipher) => cipher.encrypt_block(&mut block),
        }
        block.into()
    }
}
pub(super) struct Aes<R> {
    input: super::zip_range::Range<R>,
    cipher: Cipher,
    start: u64,
    length: u64,
    position: u64,
    control: OperationControl,
}
impl<R: Read + Seek> Aes<R> {
    pub(super) fn new(
        mut input: super::zip_range::Range<R>,
        packed: u64,
        descriptor: ZipAes,
        password: &str,
        control: &OperationControl,
    ) -> Result<Self> {
        let key_length = match descriptor.strength {
            1 => 16,
            2 => 24,
            3 => 32,
            _ => return Err(ArchiveError::UnsupportedEncryption),
        };
        let salt_length = key_length / 2;
        let start = (salt_length + 2) as u64;
        let length = packed
            .checked_sub(start + 10)
            .ok_or(ArchiveError::CorruptArchive)?;
        let mut salt = [0; 16];
        input
            .read_exact(&mut salt[..salt_length])
            .map_err(super::zip::payload_error)?;
        let mut verifier = Zeroizing::new([0; 2]);
        input
            .read_exact(&mut *verifier)
            .map_err(super::zip::payload_error)?;
        let mut derived = Zeroizing::new([0; 66]);
        control.checkpoint()?;
        pbkdf2::pbkdf2_hmac::<sha1::Sha1>(
            password.as_bytes(),
            &salt[..salt_length],
            1000,
            &mut derived[..2 * key_length + 2],
        );
        control.checkpoint()?;
        if !bool::from(verifier[..].ct_eq(&derived[2 * key_length..2 * key_length + 2])) {
            return Err(ArchiveError::AuthenticationOrDataError);
        }
        let mut mac =
            <hmac::Hmac<sha1::Sha1> as Mac>::new_from_slice(&derived[key_length..2 * key_length])
                .map_err(|_| ArchiveError::UnsupportedEncryption)?;
        let mut remaining = length;
        let mut buffer = [0; 65536];
        // Authenticate even zero ciphertext. Preflight seeks/decoder EOF cannot bypass this.
        while remaining > 0 {
            control.checkpoint()?;
            let count = remaining.min(buffer.len() as u64) as usize;
            input
                .read_exact(&mut buffer[..count])
                .map_err(super::zip::payload_error)?;
            mac.update(&buffer[..count]);
            remaining -= count as u64;
        }
        let mut tag = [0; 10];
        input
            .read_exact(&mut tag)
            .map_err(super::zip::payload_error)?;
        mac.verify_truncated_left(&tag)
            .map_err(|_| ArchiveError::AuthenticationOrDataError)?;
        control.checkpoint()?;
        let cipher = match descriptor.strength {
            1 => Cipher::A128(
                aes::Aes128::new_from_slice(&derived[..key_length])
                    .map_err(|_| ArchiveError::UnsupportedEncryption)?,
            ),
            2 => Cipher::A192(
                aes::Aes192::new_from_slice(&derived[..key_length])
                    .map_err(|_| ArchiveError::UnsupportedEncryption)?,
            ),
            3 => Cipher::A256(
                aes::Aes256::new_from_slice(&derived[..key_length])
                    .map_err(|_| ArchiveError::UnsupportedEncryption)?,
            ),
            _ => return Err(ArchiveError::UnsupportedEncryption),
        };
        input.seek(SeekFrom::Start(start))?;
        Ok(Self {
            input,
            cipher,
            start,
            length,
            position: 0,
            control: control.clone(),
        })
    }
    pub(super) fn length(&self) -> u64 {
        self.length
    }
}
impl<R: Read + Seek> Read for Aes<R> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.control.checkpoint().map_err(io::Error::other)?;
        let count = bytes
            .len()
            .min((self.length - self.position).min(usize::MAX as u64) as usize);
        let count = self.input.read(&mut bytes[..count])?;
        let mut done = 0;
        while done < count {
            let offset = self.position + done as u64;
            let key_stream = Zeroizing::new(self.cipher.stream_block(u128::from(offset / 16) + 1));
            let within = (offset % 16) as usize;
            let length = (16 - within).min(count - done);
            for index in 0..length {
                bytes[done + index] ^= key_stream[within + index];
            }
            done += length;
        }
        self.position += count as u64;
        Ok(count)
    }
}
impl<R: Read + Seek> Seek for Aes<R> {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let target = match from {
            SeekFrom::Start(value) => i128::from(value),
            SeekFrom::Current(value) => i128::from(self.position) + i128::from(value),
            SeekFrom::End(value) => i128::from(self.length) + i128::from(value),
        };
        if target < 0 || target > i128::from(self.length) {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        self.input
            .seek(SeekFrom::Start(self.start + target as u64))?;
        self.position = target as u64;
        Ok(self.position)
    }
}
