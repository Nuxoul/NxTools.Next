//! Edit-only strict 7z metadata admission. The ordinary read/TEST parser remains unchanged.
use crate::{ArchiveError, OperationControl, ResourceLimits, Result};
use std::io::{Cursor, Read, Seek, SeekFrom};

pub(crate) struct HeaderFacts {
    pub encrypted: bool,
}
fn corrupt<T>() -> Result<T> {
    Err(ArchiveError::CorruptArchive)
}
struct Header<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Header<'a> {
    fn byte(&mut self) -> Result<u8> {
        let v = *self
            .bytes
            .get(self.at)
            .ok_or(ArchiveError::CorruptArchive)?;
        self.at += 1;
        Ok(v)
    }
    fn number(&mut self) -> Result<u64> {
        let first = self.byte()?;
        let mut mask = 0x80;
        let mut value = 0;
        for i in 0..8 {
            if first & mask == 0 {
                return Ok(value | u64::from(first & (mask - 1)) << (8 * i));
            }
            value |= u64::from(self.byte()?) << (8 * i);
            mask >>= 1;
        }
        Ok(value)
    }
    fn count(&mut self) -> Result<usize> {
        let n = usize::try_from(self.number()?).map_err(|_| ArchiveError::ResourceLimit)?;
        if n > self.bytes.len() {
            return corrupt();
        }
        Ok(n)
    }
    fn skip(&mut self, n: usize) -> Result<()> {
        self.at = self
            .at
            .checked_add(n)
            .filter(|end| *end <= self.bytes.len())
            .ok_or(ArchiveError::CorruptArchive)?;
        Ok(())
    }
    fn bits(&mut self, n: usize) -> Result<usize> {
        let mut defined = 0;
        for i in 0..n.div_ceil(8) {
            let value = self.byte()?;
            let count = (n - i * 8).min(8);
            for bit in 0..count {
                defined += usize::from(value & (0x80 >> bit) != 0);
            }
        }
        Ok(defined)
    }
    fn crcs(&mut self, n: usize) -> Result<()> {
        let defined = if self.byte()? != 0 { n } else { self.bits(n)? };
        self.skip(defined.checked_mul(4).ok_or(ArchiveError::ResourceLimit)?)
    }
    fn streams(&mut self) -> Result<bool> {
        let mut nid = self.byte()?;
        let mut folders = 0;
        let mut substreams = 0usize;
        let mut folder_crc = Vec::new();
        if nid == 6 {
            self.number()?;
            let packs = self.count()?;
            if self.byte()? != 9 {
                return corrupt();
            }
            for _ in 0..packs {
                self.number()?;
            }
            nid = self.byte()?;
            if nid == 10 {
                self.crcs(packs)?;
                nid = self.byte()?;
            }
            if nid != 0 {
                return corrupt();
            }
            nid = self.byte()?;
        }
        if nid == 7 {
            if self.byte()? != 11 {
                return corrupt();
            }
            folders = self.count()?;
            if self.byte()? != 0 {
                return Err(ArchiveError::UnsupportedOperation);
            }
            for _ in 0..folders {
                let coders = self.count()?;
                if coders == 0 || coders > 32 {
                    return Err(ArchiveError::UnsupportedOperation);
                }
                let mut inputs = 0usize;
                let mut outputs = 0usize;
                for _ in 0..coders {
                    let flags = self.byte()?;
                    if flags & 0xc0 != 0 || flags & 15 == 0 {
                        return Err(ArchiveError::UnsupportedOperation);
                    }
                    self.skip(usize::from(flags & 15))?;
                    let (a, b) = if flags & 16 != 0 {
                        (self.count()?, self.count()?)
                    } else {
                        (1, 1)
                    };
                    if a != 1 || b != 1 {
                        return Err(ArchiveError::UnsupportedOperation);
                    }
                    inputs = inputs.checked_add(a).ok_or(ArchiveError::ResourceLimit)?;
                    outputs += b;
                    if flags & 32 != 0 {
                        let n = self.count()?;
                        self.skip(n)?;
                    }
                }
                if outputs == 0 {
                    return corrupt();
                }
                for _ in 0..outputs - 1 {
                    self.number()?;
                    self.number()?;
                }
                if inputs.checked_sub(outputs - 1) != Some(1) {
                    return Err(ArchiveError::UnsupportedOperation);
                }
                folder_crc.push((outputs, false));
            }
            if self.byte()? != 12 {
                return corrupt();
            }
            for (outputs, _) in &folder_crc {
                for _ in 0..*outputs {
                    self.number()?;
                }
            }
            nid = self.byte()?;
            if nid == 10 {
                let all = self.byte()?;
                let mut defined = vec![true; folders];
                if all == 0 {
                    for group in defined.chunks_mut(8) {
                        let b = self.byte()?;
                        for (i, v) in group.iter_mut().enumerate() {
                            *v = b & (0x80 >> i) != 0;
                        }
                    }
                }
                for (i, yes) in defined.iter().enumerate() {
                    folder_crc[i].1 = *yes;
                    if *yes {
                        self.skip(4)?;
                    }
                }
                nid = self.byte()?;
            }
            if nid != 0 {
                return corrupt();
            }
            nid = self.byte()?;
        }
        let explicit_substreams = nid == 8;
        if nid == 8 {
            let mut counts = vec![1usize; folders];
            nid = self.byte()?;
            if nid == 13 {
                for n in &mut counts {
                    *n = self.count()?;
                }
                nid = self.byte()?;
            }
            for n in &counts {
                substreams = substreams
                    .checked_add(*n)
                    .ok_or(ArchiveError::ResourceLimit)?;
            }
            if nid == 9 {
                for n in &counts {
                    for _ in 1..*n {
                        self.number()?;
                    }
                }
                nid = self.byte()?;
            }
            if nid == 10 {
                let crcs =
                    counts
                        .iter()
                        .zip(&folder_crc)
                        .try_fold(0usize, |sum, (n, (_, crc))| {
                            sum.checked_add(if *n == 1 && *crc { 0 } else { *n })
                                .ok_or(ArchiveError::ResourceLimit)
                        })?;
                self.crcs(crcs)?;
                nid = self.byte()?;
            }
            if nid != 0 {
                return corrupt();
            }
            nid = self.byte()?;
        }
        if nid != 0 {
            return Err(ArchiveError::UnsupportedOperation);
        }
        Ok(explicit_substreams)
    }
    fn file_property(&mut self, property: u8, files: usize, empty: Option<usize>) -> Result<usize> {
        match property {
            25 => {
                while self.at < self.bytes.len() {
                    if self.byte()? != 0 {
                        return Err(ArchiveError::UnsupportedOperation);
                    }
                }
                Ok(0)
            }
            14 => {
                if empty.is_some() {
                    return corrupt();
                }
                self.bits(files)
            }
            15 => {
                let count = empty.ok_or(ArchiveError::CorruptArchive)?;
                self.bits(count)?;
                Ok(0)
            }
            17 => {
                if self.byte()? != 0 {
                    return Err(ArchiveError::UnsupportedOperation);
                }
                let mut names = 0usize;
                while self.at < self.bytes.len() {
                    let lo = self.byte()?;
                    let hi = self.byte()?;
                    if lo == 0 && hi == 0 {
                        names += 1;
                    }
                }
                if names != files {
                    return corrupt();
                }
                Ok(0)
            }
            18..=21 => {
                let defined = if self.byte()? != 0 {
                    files
                } else {
                    self.bits(files)?
                };
                if self.byte()? != 0 {
                    return Err(ArchiveError::UnsupportedOperation);
                }
                self.skip(
                    defined
                        .checked_mul(if property == 21 { 4 } else { 8 })
                        .ok_or(ArchiveError::ResourceLimit)?,
                )?;
                Ok(0)
            }
            _ => Err(ArchiveError::UnsupportedOperation),
        }
    }
    fn strict(&mut self) -> Result<()> {
        if self.byte()? != 1 {
            return corrupt();
        }
        let mut nid = self.byte()?;
        if nid == 2 || nid == 3 {
            return Err(ArchiveError::UnsupportedOperation);
        }
        if nid == 4 {
            self.streams()?;
            nid = self.byte()?;
        }
        if nid == 5 {
            let files = self.count()?;
            let mut seen = std::collections::HashSet::new();
            let mut empty = None;
            loop {
                let property = self.byte()?;
                if property == 0 {
                    break;
                }
                if !matches!(property, 14 | 15 | 17 | 18 | 19 | 20 | 21 | 25) {
                    return Err(ArchiveError::UnsupportedOperation);
                }
                if property != 25 && !seen.insert(property) {
                    return corrupt();
                }
                let n = self.count()?;
                let end = self
                    .at
                    .checked_add(n)
                    .filter(|v| *v <= self.bytes.len())
                    .ok_or(ArchiveError::CorruptArchive)?;
                let mut payload = Header {
                    bytes: &self.bytes[self.at..end],
                    at: 0,
                };
                let count = payload.file_property(property, files, empty)?;
                if payload.at != n {
                    return corrupt();
                }
                if property == 14 {
                    empty = Some(count);
                }
                self.at = end;
            }
            nid = self.byte()?;
        }
        if nid != 0 || self.at != self.bytes.len() {
            return Err(ArchiveError::UnsupportedOperation);
        }
        Ok(())
    }
}

pub(crate) fn inspect(
    source: &mut (impl Read + Seek),
    limits: &ResourceLimits,
    control: &OperationControl,
    password: &sevenz_rust2::Password,
) -> Result<HeaderFacts> {
    control.checkpoint()?;
    let length = source.seek(SeekFrom::End(0))?;
    source.rewind()?;
    let mut signature = [0u8; 32];
    source
        .read_exact(&mut signature)
        .map_err(|_| ArchiveError::CorruptArchive)?;
    if signature[..6] != [0x37, 0x7a, 0xbc, 0xaf, 0x27, 0x1c] || signature[6] != 0 {
        return Err(ArchiveError::UnsupportedOperation);
    }
    if crc32fast::hash(&signature[12..])
        != u32::from_le_bytes(signature[8..12].try_into().expect("fixed crc"))
    {
        return corrupt();
    }
    let offset = u64::from_le_bytes(signature[12..20].try_into().expect("fixed offset"));
    let size = u64::from_le_bytes(signature[20..28].try_into().expect("fixed size"));
    if offset.checked_add(size).and_then(|v| v.checked_add(32)) != Some(length) {
        return Err(ArchiveError::UnsupportedOperation);
    }
    if size
        .checked_mul(32)
        .is_none_or(|v| v > limits.max_metadata_bytes)
    {
        return Err(ArchiveError::ResourceLimit);
    }
    crate::worker_codecs::sevenz_budget::encoded_header(source, &signature, limits)?;
    source.seek(SeekFrom::Start(32 + offset))?;
    let mut next = vec![0; usize::try_from(size).map_err(|_| ArchiveError::ResourceLimit)?];
    source.read_exact(&mut next)?;
    if crc32fast::hash(&next)
        != u32::from_le_bytes(signature[28..32].try_into().expect("fixed crc"))
    {
        return corrupt();
    }
    let mut encrypted = false;
    let raw = if next.first() == Some(&0x17) {
        let mut descriptor = Header {
            bytes: &next,
            at: 1,
        };
        let explicit_substreams = descriptor.streams()?;
        if descriptor.at != next.len() {
            return Err(ArchiveError::UnsupportedOperation);
        }
        let mut description = vec![1, 4];
        description.extend_from_slice(&next[1..next.len() - 1]);
        if !explicit_substreams {
            // Omitted SubStreamsInfo means one stream per folder (7-Zip/NanaZip).
            // Only the private synthetic archive needs it materialized for FilesInfo.
            description.extend_from_slice(&[8, 0]);
        }
        description.extend_from_slice(&[0, 5, 1, 0, 0]);
        signature[12..20].copy_from_slice(&0u64.to_le_bytes());
        signature[20..28].copy_from_slice(&(description.len() as u64).to_le_bytes());
        signature[28..32].copy_from_slice(&crc32fast::hash(&description).to_le_bytes());
        let crc = crc32fast::hash(&signature[12..]);
        signature[8..12].copy_from_slice(&crc.to_le_bytes());
        let mut wrapped = Cursor::new(signature.to_vec());
        wrapped.get_mut().extend(description);
        let mut archive =
            sevenz_rust2::Archive::read(&mut wrapped, &sevenz_rust2::Password::empty())
                .map_err(crate::worker_codecs::sevenz_error)?;
        if archive.blocks.len() != 1
            || sevenz_rust2::BlockDecoder::new(1, 0, &archive, password, source).entry_count() != 1
            || archive.stream_map.block_first_pack_stream_index().len() != 1
            || archive.stream_map.pack_stream_offsets().is_empty()
        {
            return Err(ArchiveError::UnsupportedOperation);
        }
        let size = archive.blocks[0].get_unpack_size();
        if size
            .checked_mul(32)
            .is_none_or(|v| v > limits.max_metadata_bytes)
        {
            return Err(ArchiveError::ResourceLimit);
        }
        encrypted = archive.blocks[0].coders.iter().any(|coder| {
            coder.encoder_method_id() == sevenz_rust2::EncoderMethod::ID_AES256_SHA256
        });
        if encrypted && password.is_empty() {
            return Err(ArchiveError::PasswordRequired);
        }
        let mut entry = sevenz_rust2::ArchiveEntry::new_file("metadata");
        entry.size = size;
        archive.files = vec![entry];
        archive.stream_map.block_first_file_index = vec![0];
        archive.stream_map.file_block_index = vec![Some(0)];
        let mut raw = Vec::new();
        let failure = std::cell::Cell::new(None);
        sevenz_rust2::BlockDecoder::new(1, 0, &archive, password, source)
            .for_each_entries(&mut |_, reader| {
                let mut buffer = [0; 65536];
                loop {
                    control.checkpoint().map_err(|error| {
                        if failure.get().is_none() {
                            failure.set(Some(error));
                        }
                        sevenz_rust2::Error::from(std::io::Error::other(error))
                    })?;
                    let n = reader.read(&mut buffer)?;
                    if n == 0 {
                        break;
                    }
                    if (raw.len() as u64)
                        .checked_add(n as u64)
                        .is_none_or(|v| v > size)
                    {
                        if failure.get().is_none() {
                            failure.set(Some(ArchiveError::ResourceLimit));
                        }
                        return Err(sevenz_rust2::Error::from(std::io::Error::other(
                            ArchiveError::ResourceLimit,
                        )));
                    }
                    raw.extend_from_slice(&buffer[..n]);
                }
                Ok(true)
            })
            .map_err(|error| {
                failure
                    .get()
                    .unwrap_or_else(|| crate::worker_codecs::sevenz_error(error))
            })?;
        if raw.len() as u64 != size {
            return corrupt();
        }
        raw
    } else {
        next
    };
    Header { bytes: &raw, at: 0 }.strict()?;
    source.rewind()?;
    Ok(HeaderFacts { encrypted })
}

#[cfg(test)]
#[path = "sevenz_edit_header_tests.rs"]
mod sevenz_edit_header_tests;
