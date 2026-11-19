//! ZIP14 has a four-byte SDK/properties-size prefix, not a .lzma file header.
use crate::{ArchiveError, OperationControl, ResourceLimits, Result};
use std::io::{Read, Take};

pub(crate) struct Lzma<R: Read> {
    input: Take<R>,
    decoder: lzma_rust2::LzmaStream,
    buffer: [u8; 8192],
    start: usize,
    end: usize,
    finished: bool,
    control: OperationControl,
}
impl<R: Read> Lzma<R> {
    pub(super) fn new(
        input: Take<R>,
        size: u64,
        flags: u16,
        limits: &ResourceLimits,
        control: OperationControl,
    ) -> Result<Self> {
        Self::framed(input, size, flags, limits, control, true)
    }
    #[cfg(feature = "isolated-codecs")]
    pub(crate) fn egg(
        input: Take<R>,
        size: u64,
        limits: &ResourceLimits,
        control: OperationControl,
    ) -> Result<Self> {
        Self::framed(input, size, 0, limits, control, false)
    }
    fn framed(
        mut input: Take<R>,
        size: u64,
        flags: u16,
        limits: &ResourceLimits,
        control: OperationControl,
        zip_properties_size: bool,
    ) -> Result<Self> {
        control.checkpoint()?;
        let mut header = [0; 9];
        input
            .read_exact(&mut header)
            .map_err(super::zip::payload_error)?;
        if zip_properties_size && u16::from_le_bytes([header[2], header[3]]) != 5 {
            return Err(ArchiveError::UnsupportedMethod);
        }
        let dictionary = u32::from_le_bytes(header[5..9].try_into().expect("fixed dictionary"));
        let memory = lzma_rust2::lzma_get_memory_usage_by_props(dictionary, header[4])
            .map_err(super::zip::payload_error)?;
        // Reserve the full dictionary/model twice, covering the bounded speculative tail clone,
        // plus all fixed buffers/state. This is conservative even for known-size dictionary shrink.
        if u64::from(memory)
            .saturating_mul(2048)
            .saturating_add(1 << 20)
            > limits.max_working_bytes
        {
            return Err(ArchiveError::ResourceLimit);
        }
        let decoder = lzma_rust2::LzmaStream::new_with_props(
            if flags & 2 != 0 { u64::MAX } else { size },
            header[4],
            dictionary,
            None,
        )
        .map_err(super::zip::payload_error)?;
        Ok(Self {
            input,
            decoder,
            buffer: [0; 8192],
            start: 0,
            end: 0,
            finished: false,
            control,
        })
    }
    pub(crate) fn consumed(&self) -> u64 {
        9 + self.decoder.total_in() - self.decoder.unused_input().len() as u64
    }
}
impl<R: Read> Read for Lzma<R> {
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
            let action = if self.input.limit() == 0 {
                lzma_rust2::Action::Finish
            } else {
                lzma_rust2::Action::Run
            };
            let result = self
                .decoder
                .process(&self.buffer[self.start..self.end], output, action)
                .map_err(|failure| {
                    std::io::Error::other(match failure.kind() {
                        std::io::ErrorKind::OutOfMemory => ArchiveError::ResourceLimit,
                        std::io::ErrorKind::Unsupported => ArchiveError::UnsupportedMethod,
                        _ => ArchiveError::CorruptArchive,
                    })
                })?;
            self.start += result.bytes_consumed;
            self.finished = result.status == lzma_rust2::Status::StreamEnd;
            if result.bytes_produced > 0 || self.finished {
                return Ok(result.bytes_produced);
            }
            if result.bytes_consumed == 0 {
                return Err(std::io::ErrorKind::InvalidData.into());
            }
        }
    }
}
