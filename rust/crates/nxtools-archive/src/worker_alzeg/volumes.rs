//! Format framing over the original complete native-admitted source set.
use super::exact;
use crate::{
    ArchiveError, OperationControl, ResourceLimits, Result, worker_codecs::volumes::SourceSet,
};
use std::{
    collections::HashSet,
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
};

fn admission(set: &SourceSet, limits: &ResourceLimits) -> Result<u64> {
    let retained = set.parts().len() as u64 * 128;
    if retained > limits.max_metadata_bytes || limits.max_working_bytes < 256 << 10 {
        return Err(ArchiveError::ResourceLimit);
    }
    Ok(retained)
}
fn spool(set: &SourceSet, ranges: &[(u64, u64)], control: &OperationControl) -> Result<File> {
    if ranges.len() == 1 {
        return Ok(File::open(set.primary())?);
    }
    let mut output = tempfile::tempfile()?;
    let mut buffer = [0; 65536];
    for ((path, _), &(start, length)) in set.parts().zip(ranges) {
        control.checkpoint()?;
        let mut source = File::open(path)?;
        source.seek(SeekFrom::Start(start))?;
        let mut remaining = length;
        while remaining != 0 {
            control.checkpoint()?;
            let bound = remaining.min(buffer.len() as u64) as usize;
            let count = source.read(&mut buffer[..bound])?;
            if count == 0 {
                return Err(ArchiveError::SourceChanged);
            }
            output.write_all(&buffer[..count])?;
            remaining -= count as u64;
        }
    }
    set.verify(control)?;
    output.rewind()?;
    Ok(output)
}
pub(super) fn alz(
    set: &SourceSet,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<File> {
    admission(set, limits)?;
    let count = set.parts().len();
    if count > 1000 {
        return Err(ArchiveError::ResourceLimit);
    }
    let mut ranges = Vec::with_capacity(count);
    for (ordinal, (path, length)) in set.parts().enumerate() {
        control.checkpoint()?;
        if length < 24 {
            return Err(ArchiveError::CorruptArchive);
        }
        let mut source = File::open(path)?;
        let header = exact::<8>(&mut source)?;
        if &header[..4] != b"ALZ\x01" {
            return Err(ArchiveError::CorruptArchive);
        }
        if header[4..6] != [10, 0] {
            return Err(ArchiveError::UnsupportedFormat);
        }
        if usize::from(u16::from_le_bytes([header[6], header[7]])) != ordinal {
            return Err(ArchiveError::MissingVolume);
        }
        source.seek(SeekFrom::Start(length - 16))?;
        let tail = exact::<16>(&mut source)?;
        if &tail[..4] != b"CLZ\x01" || tail[8..12] != [0; 4] {
            return Err(ArchiveError::CorruptArchive);
        }
        let final_part = ordinal + 1 == count;
        match &tail[12..] {
            b"CLZ\x03" if tail[4..12] == [0; 8] => {
                if final_part {
                    return Err(ArchiveError::MissingVolume);
                }
            }
            b"CLZ\x02" => {
                if !final_part {
                    return Err(ArchiveError::CorruptArchive);
                }
            }
            _ => return Err(ArchiveError::CorruptArchive),
        }
        let start = if ordinal == 0 { 0 } else { 8 };
        ranges.push((start, length - start - if final_part { 0 } else { 16 }));
    }
    spool(set, &ranges, control)
}
const END: u32 = 0x08e28222;
struct Prefix {
    id: u32,
    previous: u32,
    next: u32,
    split: bool,
    solid: bool,
    end: u64,
}
fn egg_prefix(
    source: &mut File,
    length: u64,
    retained: &mut u64,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<Prefix> {
    let header = exact::<14>(source)?;
    if &header[..4] != b"EGGA" {
        return Err(ArchiveError::CorruptArchive);
    }
    if header[4..6] != [0, 1] || header[10..14] != [0; 4] {
        return Err(ArchiveError::UnsupportedFormat);
    }
    let id = u32::from_le_bytes(header[6..10].try_into().expect("header ID"));
    if id == 0 {
        return Err(ArchiveError::CorruptArchive);
    }
    let mut split = None;
    let mut solid = false;
    loop {
        control.checkpoint()?;
        let tag = u32::from_le_bytes(exact(source)?);
        match tag {
            END => {
                let (previous, next) = split.unwrap_or((0, 0));
                if solid && previous != 0 {
                    return Err(ArchiveError::CorruptArchive);
                }
                return Ok(Prefix {
                    id,
                    previous,
                    next,
                    split: split.is_some(),
                    solid,
                    end: source.stream_position()?,
                });
            }
            0x24f5a262 => {
                if split.is_some() {
                    return Err(ArchiveError::CorruptArchive);
                }
                let flags = exact::<1>(source)?[0];
                let size = if flags & 1 != 0 {
                    u64::from(u32::from_le_bytes(exact(source)?))
                } else {
                    u64::from(u16::from_le_bytes(exact(source)?))
                };
                if flags & !1 != 0
                    || size != 8
                    || source
                        .stream_position()?
                        .checked_add(size)
                        .is_none_or(|end| end > length)
                {
                    return Err(ArchiveError::CorruptArchive);
                }
                *retained = retained
                    .checked_add(96)
                    .ok_or(ArchiveError::ResourceLimit)?;
                if *retained > limits.max_metadata_bytes {
                    return Err(ArchiveError::ResourceLimit);
                }
                let data = exact::<8>(source)?;
                split = Some((
                    u32::from_le_bytes(data[..4].try_into().expect("previous")),
                    u32::from_le_bytes(data[4..].try_into().expect("next")),
                ));
            }
            0x24e5a060 => {
                if solid {
                    return Err(ArchiveError::CorruptArchive);
                }
                let flags = exact::<1>(source)?[0];
                let size = if flags & 1 != 0 {
                    u64::from(u32::from_le_bytes(exact(source)?))
                } else {
                    u64::from(u16::from_le_bytes(exact(source)?))
                };
                if flags & !1 != 0 || size != 0 {
                    return Err(ArchiveError::CorruptArchive);
                }
                *retained = retained
                    .checked_add(16)
                    .ok_or(ArchiveError::ResourceLimit)?;
                if *retained > limits.max_metadata_bytes {
                    return Err(ArchiveError::ResourceLimit);
                }
                solid = true;
            }
            0xffff0000 => return Err(ArchiveError::UnsupportedMethod),
            0x08d144a8 => return Err(ArchiveError::UnsupportedEncryption),
            _ => return Err(ArchiveError::CorruptArchive),
        }
    }
}
pub(super) struct EggInput {
    pub(super) file: File,
    pub(super) boundaries: Vec<u64>,
    pub(super) header_end: u64,
    pub(super) retained: u64,
    pub(super) solid: bool,
}
pub(super) fn egg(
    set: &SourceSet,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<EggInput> {
    let mut retained = admission(set, limits)?;
    let count = set.parts().len();
    let mut ranges = Vec::with_capacity(count);
    let mut boundaries = Vec::with_capacity(count.saturating_sub(1));
    let mut ids = HashSet::with_capacity(count);
    let mut previous_id = 0;
    let mut expected_id = 0;
    let mut header_end = 0;
    let mut logical_length = 0u64;
    let mut solid = false;
    for (ordinal, (path, length)) in set.parts().enumerate() {
        control.checkpoint()?;
        let prefix = egg_prefix(
            &mut File::open(path)?,
            length,
            &mut retained,
            limits,
            control,
        )?;
        if !ids.insert(prefix.id) {
            return Err(ArchiveError::CorruptArchive);
        }
        if prefix.previous != previous_id || ordinal != 0 && prefix.id != expected_id {
            return Err(ArchiveError::MissingVolume);
        }
        if ordinal != 0 && !prefix.split
            || ordinal + 1 < count && (!prefix.split || prefix.next == 0)
            || ordinal + 1 == count && prefix.next != 0
        {
            return Err(ArchiveError::MissingVolume);
        }
        let start = if ordinal == 0 {
            solid = prefix.solid;
            header_end = prefix.end;
            0
        } else {
            prefix.end
        };
        let size = length
            .checked_sub(start)
            .filter(|size| *size != 0)
            .ok_or(ArchiveError::CorruptArchive)?;
        ranges.push((start, size));
        logical_length = logical_length
            .checked_add(size)
            .ok_or(ArchiveError::ResourceLimit)?;
        if ordinal + 1 != count {
            boundaries.push(logical_length);
        }
        previous_id = prefix.id;
        expected_id = prefix.next;
    }
    Ok(EggInput {
        file: spool(set, &ranges, control)?,
        boundaries,
        header_end,
        retained,
        solid,
    })
}
pub(super) fn header_range(boundaries: &[u64], start: u64, end: u64) -> Result<()> {
    if boundaries
        .iter()
        .any(|boundary| start < *boundary && *boundary < end)
    {
        return Err(ArchiveError::CorruptArchive);
    }
    Ok(())
}
