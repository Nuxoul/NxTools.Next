//! ZIP disk coordinates and raw central records. Payload decoding stays in formats::zip.
use crate::{ArchiveError, OperationControl, ResourceLimits, Result};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

const MAX_VOLUMES: usize = 1024;
fn word(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(bytes[at..at + 2].try_into().expect("checked field"))
}
fn dword(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().expect("checked field"))
}
fn qword(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(bytes[at..at + 8].try_into().expect("checked field"))
}
fn exact(file: &mut File, bytes: &mut [u8]) -> Result<()> {
    file.read_exact(bytes).map_err(|e| {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            ArchiveError::CorruptArchive
        } else {
            ArchiveError::IoFailure
        }
    })
}
fn missing(error: std::io::Error) -> ArchiveError {
    if error.kind() == std::io::ErrorKind::NotFound {
        ArchiveError::MissingVolume
    } else {
        ArchiveError::IoFailure
    }
}

pub(crate) struct DiskSet {
    pub paths: Vec<PathBuf>,
    pub files: Vec<File>,
    starts: Vec<u64>,
    pub length: u64,
}
pub(crate) fn candidate_set(files: &[File], limits: &ResourceLimits) -> Result<DiskSet> {
    if files.is_empty() || files.len() > MAX_VOLUMES {
        return Err(ArchiveError::ResourceLimit);
    }
    let mut copies = Vec::new();
    let mut starts = Vec::new();
    let mut length = 0u64;
    for file in files {
        starts.push(length);
        length = length
            .checked_add(file.metadata()?.len())
            .ok_or(ArchiveError::ResourceLimit)?;
        copies.push(file.try_clone()?);
    }
    if length > limits.max_output_bytes {
        return Err(ArchiveError::ResourceLimit);
    }
    Ok(DiskSet {
        paths: Vec::new(),
        files: copies,
        starts,
        length,
    })
}
impl DiskSet {
    fn new(
        paths: Vec<PathBuf>,
        limits: &ResourceLimits,
        control: &OperationControl,
    ) -> Result<Self> {
        if paths.is_empty() || paths.len() > MAX_VOLUMES {
            return Err(ArchiveError::ResourceLimit);
        }
        let mut files = Vec::new();
        let mut starts = Vec::new();
        let mut length = 0u64;
        for path in &paths {
            control.checkpoint()?;
            let file = File::open(path).map_err(missing)?;
            let size = file.metadata()?.len();
            if size == 0 {
                return Err(ArchiveError::CorruptArchive);
            }
            starts.push(length);
            length = length
                .checked_add(size)
                .ok_or(ArchiveError::ResourceLimit)?;
            if length > limits.max_output_bytes {
                return Err(ArchiveError::ResourceLimit);
            }
            files.push(file);
        }
        Ok(Self {
            paths,
            files,
            starts,
            length,
        })
    }
    fn global(&self, disk: u32, offset: u64) -> Result<u64> {
        let index = usize::try_from(disk).map_err(|_| ArchiveError::CorruptArchive)?;
        let start = *self.starts.get(index).ok_or(ArchiveError::MissingVolume)?;
        let end = self.starts.get(index + 1).copied().unwrap_or(self.length);
        if offset > end - start {
            return Err(ArchiveError::CorruptArchive);
        }
        start
            .checked_add(offset)
            .ok_or(ArchiveError::CorruptArchive)
    }
    fn spool(&mut self, control: &OperationControl) -> Result<File> {
        let mut out = tempfile::tempfile()?;
        let mut buffer = [0u8; 65536];
        let mut copied = 0u64;
        for file in &mut self.files {
            file.rewind()?;
            loop {
                control.checkpoint()?;
                let count = file.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                copied = copied
                    .checked_add(count as u64)
                    .ok_or(ArchiveError::ResourceLimit)?;
                if copied > self.length {
                    return Err(ArchiveError::SourceChanged);
                }
                out.write_all(&buffer[..count])?;
            }
        }
        if copied != self.length {
            return Err(ArchiveError::SourceChanged);
        }
        out.rewind()?;
        Ok(out)
    }
}
struct End {
    disk: u32,
    central_disk: u32,
    count: u64,
    on_disk: u64,
    size: u64,
    offset: u64,
    position: u64,
    comment: Vec<u8>,
}
fn end(file: &mut File, base: u64, length: u64) -> Result<End> {
    let size = length.min(65557) as usize;
    if size < 22 {
        return Err(ArchiveError::CorruptArchive);
    }
    file.seek(SeekFrom::Start(base + length - size as u64))?;
    let mut tail = vec![0; size];
    exact(file, &mut tail)?;
    let at = (0..=size - 22)
        .rev()
        .find(|&at| {
            tail[at..at + 4] == *b"PK\x05\x06"
                && at + 22 + usize::from(word(&tail, at + 20)) == size
        })
        .ok_or(ArchiveError::CorruptArchive)?;
    Ok(End {
        disk: u32::from(word(&tail, at + 4)),
        central_disk: u32::from(word(&tail, at + 6)),
        count: u64::from(word(&tail, at + 10)),
        on_disk: u64::from(word(&tail, at + 8)),
        size: u64::from(dword(&tail, at + 12)),
        offset: u64::from(dword(&tail, at + 16)),
        position: base + length - size as u64 + at as u64,
        comment: tail[at + 22..].to_vec(),
    })
}
pub(crate) fn discover(
    path: &Path,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<Option<DiskSet>> {
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let numbered = extension
        .as_bytes()
        .first()
        .is_some_and(|v| matches!(v, b'z' | b'Z'))
        && extension.len() >= 3
        && extension.as_bytes()[1..].iter().all(u8::is_ascii_digit);
    if !extension.eq_ignore_ascii_case("zip") && !numbered {
        return Ok(None);
    }
    let final_path = if numbered {
        path.with_extension("zip")
    } else {
        path.to_owned()
    };
    let mut last = File::open(&final_path).map_err(missing)?;
    let length = last.metadata()?.len();
    if length.min(65557) > limits.max_working_bytes || length.min(65557) > limits.max_metadata_bytes
    {
        return Err(ArchiveError::ResourceLimit);
    }
    let ending = end(&mut last, 0, length)?;
    let mut last_disk = ending.disk;
    if ending.disk == 65535
        || ending.count == 65535
        || ending.offset == u64::from(u32::MAX)
        || ending.size == u64::from(u32::MAX)
    {
        if ending.position < 20 {
            return Err(ArchiveError::CorruptArchive);
        }
        last.seek(SeekFrom::Start(ending.position - 20))?;
        let mut locator = [0u8; 20];
        exact(&mut last, &mut locator)?;
        if locator[..4] != *b"PK\x06\x07" {
            return Err(ArchiveError::CorruptArchive);
        }
        let count = dword(&locator, 16);
        last_disk = count.checked_sub(1).ok_or(ArchiveError::CorruptArchive)?;
    }
    if last_disk == 0 {
        return if numbered {
            Err(ArchiveError::CorruptArchive)
        } else {
            Ok(None)
        };
    }
    if last_disk as usize >= MAX_VOLUMES {
        return Err(ArchiveError::ResourceLimit);
    }
    let mut paths = Vec::new();
    for index in 1..=last_disk {
        paths.push(final_path.with_extension(format!("z{index:02}")));
    }
    paths.push(final_path);
    let set = DiskSet::new(paths, limits, control)?;
    if numbered {
        let ordinal = extension[1..]
            .parse::<usize>()
            .map_err(|_| ArchiveError::CorruptArchive)?;
        if ordinal == 0
            || ordinal > last_disk as usize
            || !extension.eq_ignore_ascii_case(&format!("z{ordinal:02}"))
        {
            return Err(ArchiveError::CorruptArchive);
        }
        let expected = set
            .paths
            .get(ordinal - 1)
            .ok_or(ArchiveError::CorruptArchive)?;
        // Resolve only the exact parsed member, never scan or ignore a differing actual path.
        // Windows canonicalization follows that directory's case policy; Unix stays case-sensitive.
        if std::fs::canonicalize(expected).map_err(missing)?
            != std::fs::canonicalize(path).map_err(missing)?
        {
            return Err(ArchiveError::CorruptArchive);
        }
    }
    Ok(Some(set))
}

pub(crate) struct Record {
    pub central: Vec<u8>,
    pub local: u64,
    pub packed: u64,
    pub size: u64,
    pub disk: u32,
}
fn extra_position(raw: &[u8], tag: u16) -> Result<Option<(usize, usize)>> {
    let name = usize::from(word(raw, 28));
    let extra = usize::from(word(raw, 30));
    let mut at = 46 + name;
    let stop = at + extra;
    let mut found = None;
    while at < stop {
        if at + 4 > stop {
            return Err(ArchiveError::CorruptArchive);
        }
        let size = usize::from(word(raw, at + 2));
        if at + 4 + size > stop {
            return Err(ArchiveError::CorruptArchive);
        }
        if word(raw, at) == tag {
            if found.is_some() {
                return Err(ArchiveError::CorruptArchive);
            }
            found = Some((at, size));
        }
        at += 4 + size;
    }
    Ok(found)
}
fn record(raw: Vec<u8>) -> Result<Record> {
    let mut size = u64::from(dword(&raw, 24));
    let mut packed = u64::from(dword(&raw, 20));
    let mut local = u64::from(dword(&raw, 42));
    let mut disk = u32::from(word(&raw, 34));
    if size == u64::from(u32::MAX)
        || packed == u64::from(u32::MAX)
        || local == u64::from(u32::MAX)
        || disk == 65535
    {
        let (at, len) = extra_position(&raw, 1)?.ok_or(ArchiveError::CorruptArchive)?;
        let mut cursor = at + 4;
        let stop = cursor + len;
        for value in [&mut size, &mut packed, &mut local] {
            if *value == u64::from(u32::MAX) {
                if cursor + 8 > stop {
                    return Err(ArchiveError::CorruptArchive);
                }
                *value = qword(&raw, cursor);
                cursor += 8;
            }
        }
        if disk == 65535 {
            if cursor + 4 > stop {
                return Err(ArchiveError::CorruptArchive);
            }
            disk = dword(&raw, cursor);
        }
    }
    Ok(Record {
        central: raw,
        local,
        packed,
        size,
        disk,
    })
}
pub(crate) fn relocate(record: &Record, disk: u32, offset: u64) -> Result<Vec<u8>> {
    let mut raw = record.central.clone();
    let old = extra_position(&raw, 1)?;
    let mut fields = Vec::new();
    if dword(&raw, 24) == u32::MAX {
        fields.extend_from_slice(&record.size.to_le_bytes());
    }
    if dword(&raw, 20) == u32::MAX {
        fields.extend_from_slice(&record.packed.to_le_bytes());
    }
    let wide_offset = offset >= u64::from(u32::MAX);
    if wide_offset {
        fields.extend_from_slice(&offset.to_le_bytes());
    }
    let wide_disk = disk >= 65535;
    if wide_disk {
        fields.extend_from_slice(&disk.to_le_bytes());
    }
    let start = 46 + usize::from(word(&raw, 28));
    let old_extra = usize::from(word(&raw, 30));
    let old_size = old.map_or(0, |(_, len)| len + 4);
    if let Some((at, len)) = old {
        raw.drain(at..at + 4 + len);
    }
    if !fields.is_empty() {
        let length = u16::try_from(fields.len()).map_err(|_| ArchiveError::ResourceLimit)?;
        let mut extra = vec![1, 0];
        extra.extend_from_slice(&length.to_le_bytes());
        extra.extend(fields);
        raw.splice(start..start, extra);
    }
    let new_extra = old_extra - old_size
        + if wide_offset || wide_disk || dword(&raw, 24) == u32::MAX || dword(&raw, 20) == u32::MAX
        {
            4 + usize::from(dword(&raw, 24) == u32::MAX) * 8
                + usize::from(dword(&raw, 20) == u32::MAX) * 8
                + usize::from(wide_offset) * 8
                + usize::from(wide_disk) * 4
        } else {
            0
        };
    let new_extra = u16::try_from(new_extra).map_err(|_| ArchiveError::ResourceLimit)?;
    raw[30..32].copy_from_slice(&new_extra.to_le_bytes());
    raw[34..36].copy_from_slice(&(if wide_disk { 65535 } else { disk as u16 }).to_le_bytes());
    raw[42..46]
        .copy_from_slice(&(if wide_offset { u32::MAX } else { offset as u32 }).to_le_bytes());
    Ok(raw)
}
pub(crate) struct Directory {
    pub records: Vec<Record>,
    pub offset: u64,
    pub comment: Vec<u8>,
}
pub(crate) fn directory(
    file: &mut File,
    set: &DiskSet,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<Directory> {
    let last = set.files.len() - 1;
    let base = set.starts[last];
    let mut ending = end(file, base, set.length - base)?;
    let mut end_position = ending.position;
    if ending.count == 65535
        || ending.offset == u64::from(u32::MAX)
        || ending.size == u64::from(u32::MAX)
        || ending.disk == 65535
    {
        if ending.position < 20 {
            return Err(ArchiveError::CorruptArchive);
        }
        file.seek(SeekFrom::Start(ending.position - 20))?;
        let mut locator = [0; 20];
        exact(file, &mut locator)?;
        if locator[..4] != *b"PK\x06\x07" || dword(&locator, 16) != set.files.len() as u32 {
            return Err(ArchiveError::CorruptArchive);
        }
        let pos = set.global(dword(&locator, 4), qword(&locator, 8))?;
        file.seek(SeekFrom::Start(pos))?;
        let mut header = [0; 56];
        exact(file, &mut header)?;
        if header[..4] != *b"PK\x06\x06"
            || qword(&header, 4) != 44
            || pos + 56 != ending.position - 20
        {
            return Err(ArchiveError::CorruptArchive);
        }
        ending.disk = dword(&header, 16);
        ending.central_disk = dword(&header, 20);
        ending.on_disk = qword(&header, 24);
        ending.count = qword(&header, 32);
        ending.size = qword(&header, 40);
        ending.offset = qword(&header, 48);
        end_position = pos;
    }
    if ending.disk as usize != last {
        return Err(ArchiveError::CorruptArchive);
    }
    let start = set.global(ending.central_disk, ending.offset)?;
    if start.checked_add(ending.size) != Some(end_position) {
        return Err(ArchiveError::CorruptArchive);
    }
    let reserved = ending
        .size
        .checked_mul(4)
        .and_then(|v| {
            ending
                .count
                .checked_mul(256)
                .and_then(|fixed| v.checked_add(fixed))
        })
        .ok_or(ArchiveError::ResourceLimit)?;
    if ending.count > limits.max_entries || reserved > limits.max_metadata_bytes {
        return Err(ArchiveError::ResourceLimit);
    }
    file.seek(SeekFrom::Start(start))?;
    let mut records = Vec::new();
    let mut on_disk = 0u64;
    for _ in 0..ending.count {
        control.checkpoint()?;
        if file.stream_position()? >= set.starts[last] {
            on_disk += 1;
        }
        let mut raw = vec![0; 46];
        exact(file, &mut raw)?;
        if raw[..4] != *b"PK\x01\x02" {
            return Err(ArchiveError::CorruptArchive);
        }
        let len =
            usize::from(word(&raw, 28)) + usize::from(word(&raw, 30)) + usize::from(word(&raw, 32));
        if file
            .stream_position()?
            .checked_add(len as u64)
            .is_none_or(|v| v > start + ending.size)
        {
            return Err(ArchiveError::CorruptArchive);
        }
        raw.resize(46 + len, 0);
        exact(file, &mut raw[46..])?;
        let mut row = record(raw)?;
        row.local = set.global(row.disk, row.local)?;
        if row.local >= start {
            return Err(ArchiveError::CorruptArchive);
        }
        records.push(row);
    }
    if file.stream_position()? != start + ending.size {
        return Err(ArchiveError::CorruptArchive);
    }
    if on_disk != ending.on_disk {
        return Err(ArchiveError::CorruptArchive);
    }
    Ok(Directory {
        records,
        offset: start,
        comment: ending.comment,
    })
}
fn footer(out: &mut File, ending: End) -> Result<()> {
    let End {
        disk,
        central_disk,
        on_disk,
        count,
        size,
        offset,
        comment,
        ..
    } = ending;
    let zip64 = count >= 65535
        || on_disk >= 65535
        || offset >= u64::from(u32::MAX)
        || size >= u64::from(u32::MAX)
        || disk >= 65535;
    if zip64 {
        let pos = out.stream_position()?;
        out.write_all(b"PK\x06\x06")?;
        out.write_all(&44u64.to_le_bytes())?;
        for v in [45u16, 45] {
            out.write_all(&v.to_le_bytes())?;
        }
        for v in [disk, central_disk] {
            out.write_all(&v.to_le_bytes())?;
        }
        for v in [on_disk, count, size, offset] {
            out.write_all(&v.to_le_bytes())?;
        }
        out.write_all(b"PK\x06\x07")?;
        out.write_all(&disk.to_le_bytes())?;
        out.write_all(&pos.to_le_bytes())?;
        out.write_all(&(disk + 1).to_le_bytes())?;
    }
    out.write_all(b"PK\x05\x06")?;
    for v in [
        disk.min(65535) as u16,
        central_disk.min(65535) as u16,
        on_disk.min(65535) as u16,
        count.min(65535) as u16,
    ] {
        out.write_all(&v.to_le_bytes())?;
    }
    for v in [
        size.min(u64::from(u32::MAX)) as u32,
        offset.min(u64::from(u32::MAX)) as u32,
    ] {
        out.write_all(&v.to_le_bytes())?;
    }
    out.write_all(
        &u16::try_from(comment.len())
            .map_err(|_| ArchiveError::ResourceLimit)?
            .to_le_bytes(),
    )?;
    out.write_all(&comment)?;
    Ok(())
}
fn copy_range(
    input: &mut File,
    out: &mut File,
    at: u64,
    length: u64,
    control: &OperationControl,
) -> Result<()> {
    input.seek(SeekFrom::Start(at))?;
    let mut left = length;
    let mut buffer = [0; 65536];
    while left > 0 {
        control.checkpoint()?;
        let n = left.min(buffer.len() as u64) as usize;
        exact(input, &mut buffer[..n])?;
        out.write_all(&buffer[..n])?;
        left -= n as u64;
    }
    Ok(())
}

pub(crate) fn single_directory(
    file: &mut File,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<Directory> {
    let length = file.metadata()?.len();
    let set = DiskSet {
        paths: Vec::new(),
        files: vec![file.try_clone()?],
        starts: vec![0],
        length,
    };
    directory(file, &set, limits, control)
}
pub(crate) fn finish_single(
    file: &mut File,
    records: &[Record],
    comment: Vec<u8>,
    control: &OperationControl,
) -> Result<()> {
    let offset = file.stream_position()?;
    for record in records {
        control.checkpoint()?;
        file.write_all(&relocate(record, 0, record.local)?)?;
    }
    let size = file.stream_position()? - offset;
    footer(
        file,
        End {
            disk: 0,
            central_disk: 0,
            count: records.len() as u64,
            on_disk: records.len() as u64,
            size,
            offset,
            position: 0,
            comment,
        },
    )
}
pub(crate) fn normalize(
    set: &mut DiskSet,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<File> {
    let mut joined = set.spool(control)?;
    let directory = directory(&mut joined, set, limits, control)?;
    let mut out = tempfile::tempfile()?;
    copy_range(&mut joined, &mut out, 0, directory.offset, control)?;
    let start = out.stream_position()?;
    for row in &directory.records {
        control.checkpoint()?;
        out.write_all(&relocate(row, 0, row.local)?)?;
    }
    let size = out.stream_position()? - start;
    footer(
        &mut out,
        End {
            disk: 0,
            central_disk: 0,
            on_disk: directory.records.len() as u64,
            count: directory.records.len() as u64,
            size,
            offset: start,
            position: 0,
            comment: directory.comment,
        },
    )?;
    if out.stream_position()? > limits.max_output_bytes {
        return Err(ArchiveError::ResourceLimit);
    }
    out.rewind()?;
    Ok(out)
}

pub(crate) fn split_encoded(
    input: &mut File,
    volume_bytes: u64,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<Vec<File>> {
    if volume_bytes < 128 || volume_bytes > limits.max_output_bytes {
        return Err(ArchiveError::ResourceLimit);
    }
    let (entries, _, safe) =
        crate::formats::zip::list(input, crate::NameEncoding::Auto, limits, control)?;
    if !safe {
        return Err(ArchiveError::OutputCollision);
    }
    crate::formats::zip::preflight_created(input, &entries, limits, control)?;
    let length = input.metadata()?.len();
    let set = DiskSet {
        paths: Vec::new(),
        files: vec![input.try_clone()?],
        starts: vec![0],
        length,
    };
    let directory = directory(input, &set, limits, control)?;
    let mut order: Vec<usize> = (0..directory.records.len()).collect();
    order.sort_by_key(|&index| directory.records[index].local);
    if order
        .first()
        .is_some_and(|&index| directory.records[index].local != 0)
    {
        return Err(ArchiveError::UnsupportedOperation);
    }
    let mut sink = SplitWriter::new(volume_bytes, limits.max_output_bytes)?;
    sink.bytes(b"PK\x07\x08", true, control)?;
    let mut locations = vec![(0u32, 0u64); order.len()];
    for (position, &index) in order.iter().enumerate() {
        control.checkpoint()?;
        let row = &directory.records[index];
        input.seek(SeekFrom::Start(row.local))?;
        let mut header = [0u8; 30];
        exact(input, &mut header)?;
        let header_size = 30 + u64::from(word(&header, 26)) + u64::from(word(&header, 28));
        let end = order
            .get(position + 1)
            .map_or(directory.offset, |&next| directory.records[next].local);
        if end < row.local + header_size {
            return Err(ArchiveError::CorruptArchive);
        }
        sink.fit(header_size)?;
        locations[index] = sink.coordinate();
        sink.range(input, row.local, header_size, control)?;
        sink.range(
            input,
            row.local + header_size,
            end - row.local - header_size,
            control,
        )?;
    }
    if sink.files.len() == 1 {
        sink.next()?;
    }
    let mut central_start = None;
    let mut central_size = 0u64;
    let mut central_disks = Vec::new();
    for (index, row) in directory.records.iter().enumerate() {
        let (disk, offset) = locations[index];
        let raw = relocate(row, disk, offset)?;
        sink.fit(raw.len() as u64)?;
        central_start.get_or_insert(sink.coordinate());
        central_disks.push(sink.coordinate().0);
        central_size = central_size
            .checked_add(raw.len() as u64)
            .ok_or(ArchiveError::ResourceLimit)?;
        sink.bytes(&raw, true, control)?;
    }
    let count = directory.records.len() as u64;
    let mut start = central_start.unwrap_or(sink.coordinate());
    let needs64 =
        count >= 65535 || central_size >= u64::from(u32::MAX) || start.1 >= u64::from(u32::MAX);
    sink.fit(22 + directory.comment.len() as u64 + if needs64 { 76 } else { 0 })?;
    if count == 0 {
        start = sink.coordinate();
    }
    let disk = sink.coordinate().0;
    let on_disk = central_disks.iter().filter(|&&value| value == disk).count() as u64;
    let mut ending = tempfile::tempfile()?;
    // ZIP64 locator points to the ZIP64 end record's actual disk-local offset.
    ending.seek(SeekFrom::Start(sink.coordinate().1))?;
    let initial = ending.stream_position()?;
    footer(
        &mut ending,
        End {
            disk,
            central_disk: start.0,
            on_disk,
            count,
            size: central_size,
            offset: start.1,
            position: 0,
            comment: directory.comment,
        },
    )?;
    let final_position = ending.stream_position()?;
    ending.seek(SeekFrom::Start(initial))?;
    let mut bytes = vec![
        0;
        usize::try_from(final_position - initial)
            .map_err(|_| ArchiveError::ResourceLimit)?
    ];
    exact(&mut ending, &mut bytes)?;
    sink.bytes(&bytes, true, control)?;
    for file in &mut sink.files {
        file.flush()?;
        file.rewind()?;
    }
    Ok(sink.files)
}

struct SplitWriter {
    files: Vec<File>,
    maximum: u64,
    budget: u64,
    total: u64,
    position: u64,
}
impl SplitWriter {
    fn new(maximum: u64, budget: u64) -> Result<Self> {
        Ok(Self {
            files: vec![tempfile::tempfile()?],
            maximum,
            budget,
            total: 0,
            position: 0,
        })
    }
    fn coordinate(&self) -> (u32, u64) {
        ((self.files.len() - 1) as u32, self.position)
    }
    fn next(&mut self) -> Result<()> {
        if self.files.len() >= MAX_VOLUMES {
            return Err(ArchiveError::ResourceLimit);
        }
        self.files.push(tempfile::tempfile()?);
        self.position = 0;
        Ok(())
    }
    fn fit(&mut self, length: u64) -> Result<()> {
        if length > self.maximum {
            return Err(ArchiveError::ResourceLimit);
        }
        if self.position + length > self.maximum {
            self.next()?;
        }
        Ok(())
    }
    fn bytes(&mut self, mut bytes: &[u8], intact: bool, control: &OperationControl) -> Result<()> {
        if intact {
            self.fit(bytes.len() as u64)?;
        }
        while !bytes.is_empty() {
            control.checkpoint()?;
            if self.position == self.maximum {
                self.next()?;
            }
            let count = bytes.len().min((self.maximum - self.position) as usize);
            self.total = self
                .total
                .checked_add(count as u64)
                .ok_or(ArchiveError::ResourceLimit)?;
            if self.total > self.budget {
                return Err(ArchiveError::ResourceLimit);
            }
            self.files
                .last_mut()
                .ok_or(ArchiveError::IoFailure)?
                .write_all(&bytes[..count])?;
            self.position += count as u64;
            bytes = &bytes[count..];
        }
        Ok(())
    }
    fn range(
        &mut self,
        input: &mut File,
        at: u64,
        mut length: u64,
        control: &OperationControl,
    ) -> Result<()> {
        input.seek(SeekFrom::Start(at))?;
        let mut buffer = [0u8; 65536];
        while length > 0 {
            let count = length.min(buffer.len() as u64) as usize;
            exact(input, &mut buffer[..count])?;
            self.bytes(&buffer[..count], false, control)?;
            length -= count as u64;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_split_writer_crosses_payload_disks_and_preserves_original_payloads() {
        let control = OperationControl::default();
        let mut encoded = tempfile::tempfile().expect("encoded source");
        let mut writer = zip::ZipWriter::new(&mut encoded);
        writer
            .start_file(
                "payload.bin",
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored),
            )
            .expect("member");
        let payload: Vec<u8> = (0..600).map(|n| (n % 251) as u8).collect();
        writer.write_all(&payload).expect("payload");
        writer.finish().expect("finish");
        encoded.rewind().expect("rewind");
        let mut volumes = crate::ArchiveEngine::default()
            .zip_volume_candidates(&mut encoded, 128, &control)
            .expect("genuine split output");
        assert!(volumes.len() >= 5);
        let root = tempfile::tempdir().expect("owned family");
        let count = volumes.len();
        for (index, file) in volumes.iter_mut().enumerate() {
            assert!(file.metadata().expect("size").len() <= 128);
            file.rewind().expect("volume rewind");
            let path = root.path().join(if index + 1 == count {
                "created.zip".to_owned()
            } else {
                format!("created.z{:02}", index + 1)
            });
            let mut target = File::create(&path).expect("fixture target");
            std::io::copy(file, &mut target).expect("save fixture");
            if let Some(artifacts) = std::env::var_os("NXTOOLS_ARCHIVE_VOLUME_ARTIFACT_DIR") {
                let destination =
                    PathBuf::from(artifacts).join(path.file_name().expect("member component"));
                std::fs::copy(&path, destination).expect("copy requested isolated artifact");
            }
        }
        let engine = crate::ArchiveEngine::default().with_zip_volumes();
        let snapshot = engine
            .open(
                crate::OpenRequest::new(root.path().join("created.zip")),
                &control,
            )
            .expect("read created volumes");
        let result = engine
            .execute(crate::ArchiveRequest::test(snapshot), &control)
            .expect("full integrity");
        assert_eq!(result.bytes_verified(), 600);
        assert_eq!(result.files_verified(), 1);
    }
}
