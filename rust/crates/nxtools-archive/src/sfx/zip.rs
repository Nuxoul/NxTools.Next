use super::input::{Parts, u16le, u32le, u64le};
use super::{Entry, Error, Kind, Limits, Result, add, require};

struct End {
    start: u64,
    size: u64,
    entries: u64,
    on_disk: u64,
    end_disk: u32,
    boundary: u64,
}
fn equal16(value: u16, actual: u64) -> Result<()> {
    require(value == u16::MAX || u64::from(value) == actual)
}
fn equal32(value: u32, actual: u64) -> Result<()> {
    require(value == u32::MAX || u64::from(value) == actual)
}
fn end(parts: &mut Parts, limits: Limits) -> Result<End> {
    let tail_len = (parts.total - parts.last_start()).min(65557) as usize;
    require(tail_len >= 22)?;
    let tail_start = parts.total - tail_len as u64;
    let tail = parts.bytes(tail_start, tail_len)?;
    let position = (0..=tail.len() - 22)
        .rev()
        .find(|&p| {
            &tail[p..p + 4] == b"PK\x05\x06"
                && u16le(&tail, p + 20).is_ok_and(|n| p + 22 + usize::from(n) == tail.len())
        })
        .ok_or_else(Error::corrupt)?;
    let e = &tail[position..];
    let epos = add(tail_start, position as u64)?;
    let disk = u16le(e, 4)?;
    let cdisk = u16le(e, 6)?;
    let on = u16le(e, 8)?;
    let count = u16le(e, 10)?;
    let size = u32le(e, 12)?;
    let offset = u32le(e, 16)?;
    equal16(disk, u64::from(parts.count() - 1))?;
    let mut locator = [0; 20];
    let has_locator = if epos >= 20 {
        parts.read(epos - 20, &mut locator)?;
        &locator[..4] == b"PK\x06\x07"
    } else {
        false
    };
    if has_locator {
        let zdisk = u32le(&locator, 4)?;
        require(u32le(&locator, 16)? == parts.count())?;
        let zpos = parts.coordinate(zdisk, u64le(&locator, 8)?)?;
        let mut z = [0; 56];
        parts.read(zpos, &mut z)?;
        require(&z[..4] == b"PK\x06\x06")?;
        let zsize = u64le(&z, 4)?;
        if zsize > limits.metadata.min(64 * 1024 * 1024) {
            return Err(Error::new(Kind::Limit));
        }
        require(zsize >= 44 && add(zpos, add(12, zsize)?)? == epos - 20)?;
        parts.intact(zpos, add(12, zsize)?)?;
        require(u32le(&z, 16)? == zdisk)?;
        let cstart_disk = u32le(&z, 20)?;
        let count64 = u64le(&z, 32)?;
        let size64 = u64le(&z, 40)?;
        let off64 = u64le(&z, 48)?;
        equal16(cdisk, u64::from(cstart_disk))?;
        equal16(count, count64)?;
        equal32(size, size64)?;
        equal32(offset, off64)?;
        if zdisk == parts.count() - 1 {
            equal16(on, u64le(&z, 24)?)?;
        } else {
            require(on != u16::MAX)?;
        }
        Ok(End {
            start: parts.coordinate(cstart_disk, off64)?,
            size: size64,
            entries: count64,
            on_disk: u64le(&z, 24)?,
            end_disk: zdisk,
            boundary: zpos,
        })
    } else {
        require(
            disk != u16::MAX
                && cdisk != u16::MAX
                && on != u16::MAX
                && count != u16::MAX
                && size != u32::MAX
                && offset != u32::MAX,
        )?;
        Ok(End {
            start: parts.coordinate(u32::from(cdisk), u64::from(offset))?,
            size: u64::from(size),
            entries: u64::from(count),
            on_disk: u64::from(on),
            end_disk: parts.count() - 1,
            boundary: epos,
        })
    }
}
fn extra64(extra: &[u8]) -> Result<Option<&[u8]>> {
    let mut position = 0;
    let mut result = None;
    while position < extra.len() {
        let tag = u16le(extra, position)?;
        let len = usize::from(u16le(extra, position + 2)?);
        let end = position.checked_add(4 + len).ok_or_else(Error::corrupt)?;
        let value = extra.get(position + 4..end).ok_or_else(Error::corrupt)?;
        if tag == 1 {
            require(result.is_none())?;
            result = Some(value);
        }
        if tag == 0x9901 {
            return Err(Error::new(Kind::Unsupported));
        }
        position = end;
    }
    Ok(result)
}
fn wide_value(extra: Option<&[u8]>, position: &mut usize) -> Result<u64> {
    let value = u64le(extra.ok_or_else(Error::corrupt)?, *position)?;
    *position += 8;
    Ok(value)
}
fn profile(flags: u16, method: u16, version: u16) -> Result<()> {
    if flags & !0x080e != 0
        || ![0, 8].contains(&method)
        || version > 45
        || (method == 0 && flags & 6 != 0)
    {
        return Err(Error::new(Kind::Unsupported));
    }
    Ok(())
}
pub(super) fn catalog(parts: &mut Parts, limits: Limits) -> Result<Vec<Entry>> {
    let end = end(parts, limits)?;
    if end.entries > limits.entries.min(65536) {
        return Err(Error::new(Kind::Limit));
    }
    let charge = end
        .size
        .checked_mul(8)
        .and_then(|n| n.checked_add(end.entries.checked_mul(512)?))
        .and_then(|n| n.checked_add(256 * 1024))
        .ok_or_else(|| Error::new(Kind::Limit))?;
    if charge > limits.metadata.min(64 * 1024 * 1024) {
        return Err(Error::new(Kind::Limit));
    }
    require(add(end.start, end.size)? == end.boundary && end.start >= parts.prefix)?;
    let bytes = parts.bytes(
        end.start,
        usize::try_from(end.size).map_err(|_| Error::new(Kind::Limit))?,
    )?;
    let mut entries = Vec::with_capacity(end.entries as usize);
    let mut position = 0;
    let mut on_disk = 0;
    for ordinal in 0..end.entries as usize {
        let b = bytes.get(position..).ok_or_else(Error::corrupt)?;
        require(u32le(b, 0)? == 0x02014b50)?;
        let needed = u16le(b, 6)?;
        let flags = u16le(b, 8)?;
        let method = u16le(b, 10)?;
        profile(flags, method, needed)?;
        let nlen = usize::from(u16le(b, 28)?);
        let xlen = usize::from(u16le(b, 30)?);
        let clen = usize::from(u16le(b, 32)?);
        if nlen > limits.name.min(4096) {
            return Err(Error::new(Kind::Limit));
        }
        let record_len = 46 + nlen + xlen + clen;
        let record = b.get(..record_len).ok_or_else(Error::corrupt)?;
        let at = add(end.start, position as u64)?;
        parts.intact(at, record_len as u64)?;
        if parts.disk_at(at)? == end.end_disk {
            on_disk += 1;
        }
        let raw = &record[46..46 + nlen];
        if flags & 0x800 == 0 && !raw.is_ascii() {
            return Err(Error::new(Kind::Unsupported));
        }
        let name = std::str::from_utf8(raw)
            .map_err(|_| Error::new(Kind::UnsafeName))?
            .to_owned();
        let directory = name.ends_with('/');
        let attr = u32le(b, 38)?;
        let host = u16le(b, 4)? >> 8;
        let mode = (attr >> 16) & 0xf000;
        if ![0, 0x4000, 0x8000].contains(&mode) || attr & 0x440 != 0 {
            return Err(Error::new(Kind::Unsupported));
        }
        require(
            (attr & 0x10 == 0 || directory)
                && (host != 3 || mode == 0 || (mode == 0x4000) == directory),
        )?;
        let size32 = u32le(b, 24)?;
        let packed32 = u32le(b, 20)?;
        let offset32 = u32le(b, 42)?;
        let disk16 = u16le(b, 34)?;
        let extra = extra64(&record[46 + nlen..46 + nlen + xlen])?;
        let mut xp = 0;
        let size = if size32 == u32::MAX {
            wide_value(extra, &mut xp)?
        } else {
            u64::from(size32)
        };
        let packed = if packed32 == u32::MAX {
            wide_value(extra, &mut xp)?
        } else {
            u64::from(packed32)
        };
        let offset = if offset32 == u32::MAX {
            wide_value(extra, &mut xp)?
        } else {
            u64::from(offset32)
        };
        let disk = if disk16 == u16::MAX {
            let n = u32le(extra.ok_or_else(Error::corrupt)?, xp)?;
            xp += 4;
            n
        } else {
            u32::from(disk16)
        };
        if xp > 0 {
            require(extra.is_some_and(|e| e.len() == xp))?;
            require(needed >= 45)?;
        }
        require(!directory || size == 0)?;
        entries.push(Entry {
            name,
            size,
            method,
            ordinal,
            disk,
            offset,
            directory,
            packed,
            crc: u32le(b, 16)?,
            flags,
            version: needed,
            time: u32le(b, 12)?,
            wide: size32 == u32::MAX || packed32 == u32::MAX,
            data: 0,
            record_end: 0,
        });
        position += record_len;
    }
    require(position == bytes.len() && on_disk == end.on_disk)?;
    super::names::validate(
        entries.iter().map(|e| (e.name.as_str(), e.directory)),
        limits.name.min(4096),
    )?;
    for entry in &mut entries {
        local(parts, entry, end.start)?;
    }
    let mut extents: Vec<_> = entries
        .iter()
        .map(|e| Ok((parts.coordinate(e.disk, e.offset)?, e.record_end)))
        .collect::<Result<_>>()?;
    extents.sort_unstable();
    let mut cursor = add(parts.prefix, if parts.count() > 1 { 4 } else { 0 })?;
    for (start, finish) in extents {
        require(start == cursor && finish > start)?;
        cursor = finish;
    }
    require(cursor == end.start)?;
    Ok(entries)
}
fn local(parts: &mut Parts, entry: &mut Entry, central: u64) -> Result<()> {
    let start = parts.coordinate(entry.disk, entry.offset)?;
    require(start >= parts.prefix && add(start, 30)? <= central)?;
    let mut header = [0; 30];
    parts.read(start, &mut header)?;
    require(
        u32le(&header, 0)? == 0x04034b50
            && u16le(&header, 4)? == entry.version
            && u16le(&header, 6)? == entry.flags
            && u16le(&header, 8)? == entry.method
            && u32le(&header, 10)? == entry.time,
    )?;
    let nlen = usize::from(u16le(&header, 26)?);
    let xlen = usize::from(u16le(&header, 28)?);
    require(nlen == entry.name.len())?;
    let data = add(start, (30 + nlen + xlen) as u64)?;
    parts.intact(start, (30 + nlen + xlen) as u64)?;
    require(data <= central)?;
    let variable = parts.bytes(add(start, 30)?, nlen + xlen)?;
    require(&variable[..nlen] == entry.name.as_bytes())?;
    let extra = extra64(&variable[nlen..])?;
    let s32 = u32le(&header, 22)?;
    let p32 = u32le(&header, 18)?;
    let crc = u32le(&header, 14)?;
    let mut xp = 0;
    let size = if s32 == u32::MAX {
        wide_value(extra, &mut xp)?
    } else {
        u64::from(s32)
    };
    let packed = if p32 == u32::MAX {
        wide_value(extra, &mut xp)?
    } else {
        u64::from(p32)
    };
    if xp > 0 {
        require(extra.is_some_and(|x| x.len() == xp) && entry.version >= 45)?;
    }
    let descriptor = entry.flags & 8 != 0;
    require(
        (size == entry.size || (descriptor && size == 0))
            && (packed == entry.packed || (descriptor && packed == 0))
            && (crc == entry.crc || (descriptor && crc == 0)),
    )?;
    let payload_end = add(data, entry.packed)?;
    require(payload_end <= central)?;
    let mut finish = payload_end;
    if descriptor {
        let mut first = [0; 4];
        parts.read(finish, &mut first)?;
        let signature = u32le(&first, 0)? == 0x08074b50;
        let wide = entry.wide || s32 == u32::MAX || p32 == u32::MAX;
        let len = if wide { 20 } else { 12 };
        let body = parts.bytes(add(finish, if signature { 4 } else { 0 })?, len)?;
        let dp = if wide {
            u64le(&body, 4)?
        } else {
            u64::from(u32le(&body, 4)?)
        };
        let ds = if wide {
            u64le(&body, 12)?
        } else {
            u64::from(u32le(&body, 8)?)
        };
        require(u32le(&body, 0)? == entry.crc && dp == entry.packed && ds == entry.size)?;
        finish = add(finish, (len + if signature { 4 } else { 0 }) as u64)?;
    }
    require(finish <= central)?;
    entry.data = data;
    entry.record_end = finish;
    Ok(())
}
