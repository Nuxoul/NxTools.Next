//! Low-level ZIP12 payload; explicit StreamEnd and actual compressed consumption are mandatory.
use crate::{ArchiveError, OperationControl};
use std::io::{Read, Take};

pub(crate) struct Bzip2<R: Read> {
    input: Take<R>,
    decoder: bzip2::Decompress,
    buffer: [u8; 8192],
    start: usize,
    end: usize,
    finished: bool,
    control: OperationControl,
}
impl<R: Read> Bzip2<R> {
    pub(crate) fn new(input: Take<R>, control: OperationControl) -> Self {
        Self {
            input,
            decoder: bzip2::Decompress::new(true),
            buffer: [0; 8192],
            start: 0,
            end: 0,
            finished: false,
            control,
        }
    }
    pub(crate) fn consumed(&self) -> u64 {
        self.decoder.total_in()
    }
}
impl<R: Read> Read for Bzip2<R> {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        if output.is_empty() || self.finished {
            return Ok(0);
        }
        loop {
            self.control.checkpoint().map_err(std::io::Error::other)?;
            if self.start == self.end {
                self.end = self.input.read(&mut self.buffer)?;
                self.start = 0;
            }
            let before_in = self.decoder.total_in();
            let before_out = self.decoder.total_out();
            let status = self
                .decoder
                .decompress(&self.buffer[self.start..self.end], output)
                .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))?;
            let consumed = (self.decoder.total_in() - before_in) as usize;
            let produced = (self.decoder.total_out() - before_out) as usize;
            self.start += consumed;
            if status == bzip2::Status::MemNeeded {
                return Err(std::io::Error::other(ArchiveError::ResourceLimit));
            }
            if status == bzip2::Status::StreamEnd {
                self.finished = true;
            }
            if produced > 0 || self.finished {
                return Ok(produced);
            }
            if consumed == 0 {
                return Err(std::io::ErrorKind::InvalidData.into());
            }
        }
    }
}
