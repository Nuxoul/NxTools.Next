use super::{Error, Kind, Limits, Result, add, require};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

pub(super) fn u16le(b: &[u8], p: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(
        b.get(p..p + 2)
            .ok_or_else(Error::corrupt)?
            .try_into()
            .map_err(|_| Error::corrupt())?,
    ))
}
pub(super) fn u32le(b: &[u8], p: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        b.get(p..p + 4)
            .ok_or_else(Error::corrupt)?
            .try_into()
            .map_err(|_| Error::corrupt())?,
    ))
}
pub(super) fn u64le(b: &[u8], p: usize) -> Result<u64> {
    Ok(u64::from_le_bytes(
        b.get(p..p + 8)
            .ok_or_else(Error::corrupt)?
            .try_into()
            .map_err(|_| Error::corrupt())?,
    ))
}
fn open(path: &Path) -> Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(1);
    }
    let file = options.open(path)?;
    require(file.metadata()?.is_file())?;
    Ok(file)
}
fn at<const N: usize>(file: &mut File, offset: u64) -> Result<[u8; N]> {
    file.seek(SeekFrom::Start(offset))?;
    let mut bytes = [0; N];
    file.read_exact(&mut bytes)?;
    Ok(bytes)
}
fn descriptor(file: &mut File, limits: Limits) -> Result<(u64, u32)> {
    let length = file.metadata()?.len();
    let dos = at::<64>(file, 0)?;
    require(&dos[..2] == b"MZ")?;
    let pe = u64::from(u32le(&dos, 60)?);
    require((64..=1024 * 1024).contains(&pe))?;
    let coff = at::<24>(file, pe)?;
    require(&coff[..4] == b"PE\0\0" && u16le(&coff, 4)? == 0x8664)?;
    let count = u16le(&coff, 6)?;
    let optional_len = u16le(&coff, 20)?;
    require(count > 0 && count <= 96 && optional_len >= 240)?;
    let optional = at::<240>(file, add(pe, 24)?)?;
    require(u16le(&optional, 0)? == 0x20b)?;
    require(u32le(&optional, 108)? <= 16)?;
    require(u64le(&optional, 144)? == 0)?;
    let headers = u64::from(u32le(&optional, 60)?);
    let section_start = add(add(pe, 24)?, u64::from(optional_len))?;
    require(add(section_start, u64::from(count) * 40)? <= headers && headers <= length)?;
    let mut end = headers;
    for index in 0..count {
        let section = at::<40>(file, add(section_start, u64::from(index) * 40)?)?;
        let size = u64::from(u32le(&section, 16)?);
        let offset = u64::from(u32le(&section, 20)?);
        if size > 0 {
            require(offset >= headers)?;
            end = end.max(add(offset, size)?);
        }
    }
    require(add(end, 48)? <= length)?;
    let descriptor = at::<48>(file, end)?;
    require(&descriptor[..16] == b"NXTOOLS-ZIP-SFX1")?;
    require(
        u32le(&descriptor, 16)? == 1
            && u32le(&descriptor, 20)? == 48
            && u32le(&descriptor, 28)? == 0
            && u64le(&descriptor, 40)? == 0,
    )?;
    let disks = u32le(&descriptor, 24)?;
    require(disks > 0)?;
    if disks > limits.disks.min(1024) {
        return Err(Error::new(Kind::Limit));
    }
    let prefix = add(end, 48)?;
    require(u64le(&descriptor, 32)? == prefix)?;
    Ok((prefix, disks))
}
struct Disk {
    file: File,
    start: u64,
    len: u64,
}
pub(super) struct Parts {
    disks: Vec<Disk>,
    pub prefix: u64,
    pub total: u64,
}
impl Parts {
    pub fn open(path: &Path, limits: Limits) -> Result<Self> {
        let mut first = open(path)?;
        let (prefix, count) = descriptor(&mut first, limits)?;
        let first_len = first.metadata()?.len();
        let mut disks = vec![Disk {
            file: first,
            start: 0,
            len: first_len,
        }];
        let mut total = first_len;
        for index in 1..count {
            let extension = if index + 1 == count {
                "zip".to_owned()
            } else {
                format!("z{:02}", index + 1)
            };
            let file = open(&path.with_extension(extension))?;
            let len = file.metadata()?.len();
            require(len > 0)?;
            disks.push(Disk {
                file,
                start: total,
                len,
            });
            total = add(total, len)?;
        }
        let mut parts = Self {
            disks,
            prefix,
            total,
        };
        if count > 1 {
            let mut marker = [0; 4];
            parts.read(prefix, &mut marker)?;
            require(&marker == b"PK\x07\x08")?;
        }
        Ok(parts)
    }
    pub fn count(&self) -> u32 {
        self.disks.len() as u32
    }
    pub fn coordinate(&self, disk: u32, offset: u64) -> Result<u64> {
        let part = self.disks.get(disk as usize).ok_or_else(Error::corrupt)?;
        require(offset < part.len)?;
        add(part.start, offset)
    }
    pub fn disk_at(&self, offset: u64) -> Result<u32> {
        require(offset < self.total)?;
        Ok((self.disks.partition_point(|d| d.start <= offset) - 1) as u32)
    }
    pub fn last_start(&self) -> u64 {
        self.disks[self.disks.len() - 1].start
    }
    pub fn intact(&self, start: u64, len: u64) -> Result<()> {
        let disk = self.disk_at(start)?;
        let part = &self.disks[disk as usize];
        require(add(start, len)? <= add(part.start, part.len)?)
    }
    pub fn read(&mut self, mut offset: u64, mut output: &mut [u8]) -> Result<()> {
        require(add(offset, output.len() as u64)? <= self.total)?;
        while !output.is_empty() {
            let disk = self.disk_at(offset)? as usize;
            let part = &mut self.disks[disk];
            let local = offset - part.start;
            let count = output
                .len()
                .min((part.len - local).min(usize::MAX as u64) as usize);
            part.file.seek(SeekFrom::Start(local))?;
            part.file.read_exact(&mut output[..count])?;
            offset = add(offset, count as u64)?;
            output = &mut output[count..];
        }
        Ok(())
    }
    pub fn bytes(&mut self, offset: u64, len: usize) -> Result<Vec<u8>> {
        let mut bytes = vec![0; len];
        self.read(offset, &mut bytes)?;
        Ok(bytes)
    }
}
