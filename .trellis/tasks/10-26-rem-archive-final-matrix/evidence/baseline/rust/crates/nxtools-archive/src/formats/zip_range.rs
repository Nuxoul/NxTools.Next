//! Seekable view of exactly one independently validated ZIP payload, never adjacent records.
use std::io::{self, Read, Seek, SeekFrom};

pub(super) struct Range<R> {
    input: R,
    start: u64,
    length: u64,
    position: u64,
}
impl<R: Read + Seek> Range<R> {
    pub(super) fn new(mut input: R, start: u64, length: u64) -> io::Result<Self> {
        start
            .checked_add(length)
            .ok_or(io::ErrorKind::InvalidData)?;
        input.seek(SeekFrom::Start(start))?;
        Ok(Self {
            input,
            start,
            length,
            position: 0,
        })
    }
}
impl<R: Read + Seek> Read for Range<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let count = buffer
            .len()
            .min((self.length - self.position).min(usize::MAX as u64) as usize);
        let count = self.input.read(&mut buffer[..count])?;
        self.position += count as u64;
        Ok(count)
    }
}
impl<R: Read + Seek> Seek for Range<R> {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let position = match from {
            SeekFrom::Start(value) => i128::from(value),
            SeekFrom::Current(value) => i128::from(self.position) + i128::from(value),
            SeekFrom::End(value) => i128::from(self.length) + i128::from(value),
        };
        if position < 0 || position > i128::from(self.length) {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        self.input
            .seek(SeekFrom::Start(self.start + position as u64))?;
        self.position = position as u64;
        Ok(self.position)
    }
}
