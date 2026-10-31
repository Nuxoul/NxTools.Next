//! ZIP9: true BFINAL/EOB required; never stop merely at declared output size.
use crate::{ArchiveError, OperationControl};
use std::io::{Read, Take};

pub(super) struct Deflate64<R: Read> {
    input: Take<R>,
    decoder: Box<deflate64::InflaterManaged>,
    buffer: [u8; 8192],
    start: usize,
    end: usize,
    consumed: u64,
    control: OperationControl,
}
impl<R: Read> Deflate64<R> {
    pub(super) fn new(input: Take<R>, control: OperationControl) -> Self {
        Self {
            input,
            decoder: Box::new(deflate64::InflaterManaged::new()),
            buffer: [0; 8192],
            start: 0,
            end: 0,
            consumed: 0,
            control,
        }
    }
    pub(super) fn consumed(&self) -> u64 {
        self.consumed
    }
}
impl<R: Read> Read for Deflate64<R> {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        if output.is_empty() {
            return Ok(0);
        }
        loop {
            self.control.checkpoint().map_err(std::io::Error::other)?;
            let result = self
                .decoder
                .inflate(&self.buffer[self.start..self.end], output);
            self.start += result.bytes_consumed;
            self.consumed += result.bytes_consumed as u64;
            if result.data_error || self.decoder.errored() {
                return Err(std::io::Error::other(ArchiveError::CorruptArchive));
            }
            if result.bytes_written > 0 || self.decoder.finished() {
                return Ok(result.bytes_written);
            }
            if result.bytes_consumed > 0 {
                continue;
            }
            if self.start != self.end {
                return Err(std::io::ErrorKind::InvalidData.into());
            }
            // inflate(empty) above first drains pending output/bits. Withhold the final32
            // bytes (bit reader holds <=32bits), then supply one byte per need-input turn.
            // Thus an appended full byte cannot disappear into EOB lookahead accounting.
            let remaining = self.input.limit();
            let count = if remaining > 32 {
                (remaining - 32).min(8192) as usize
            } else {
                1
            };
            self.end = self.input.read(&mut self.buffer[..count])?;
            self.start = 0;
            if self.end == 0 {
                return Err(std::io::ErrorKind::UnexpectedEof.into());
            }
        }
    }
}
