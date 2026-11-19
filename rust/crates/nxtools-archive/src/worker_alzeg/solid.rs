//! Solid Deflate uses independent file spans and decoded block-integrity spans.
use crate::{
    ArchiveError, OperationControl, ResourceLimits, Result,
    formats::zip::{Deflated, payload_error},
};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    sync::Arc,
};

pub(super) struct Span {
    pub(super) offset: u64,
    pub(super) packed: u64,
    pub(super) expanded: u64,
    pub(super) crc: u32,
}
struct Ranges {
    input: File,
    spans: Arc<[Span]>,
    index: usize,
    position: u64,
    control: OperationControl,
}
impl Read for Ranges {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        if output.is_empty() {
            return Ok(0);
        }
        while let Some(span) = self.spans.get(self.index) {
            self.control.checkpoint().map_err(std::io::Error::other)?;
            if self.position == span.packed {
                self.index += 1;
                self.position = 0;
                continue;
            }
            let count = (span.packed - self.position).min(output.len() as u64) as usize;
            self.input
                .seek(SeekFrom::Start(span.offset + self.position))?;
            self.input.read_exact(&mut output[..count])?;
            self.position += count as u64;
            return Ok(count);
        }
        Ok(0)
    }
}

pub(super) struct Solid {
    input: File,
    spans: Arc<[Span]>,
    packed: u64,
    expanded: u64,
    verified: bool,
    output: Option<Deflated<Ranges>>,
    position: u64,
    control: OperationControl,
}
impl Solid {
    pub(super) fn new(
        input: File,
        spans: Vec<Span>,
        expanded: u64,
        control: OperationControl,
    ) -> Result<Self> {
        let mut packed = 0u64;
        let mut decoded = 0u64;
        for span in &spans {
            packed = packed
                .checked_add(span.packed)
                .ok_or(ArchiveError::CorruptArchive)?;
            decoded = decoded
                .checked_add(span.expanded)
                .ok_or(ArchiveError::CorruptArchive)?;
        }
        if decoded != expanded || packed == 0 && expanded != 0 {
            return Err(ArchiveError::CorruptArchive);
        }
        Ok(Self {
            input,
            spans: spans.into(),
            packed,
            expanded,
            verified: false,
            output: None,
            position: 0,
            control,
        })
    }
    fn decoder(&self) -> Result<Deflated<Ranges>> {
        let input = Ranges {
            input: self.input.try_clone()?,
            spans: self.spans.clone(),
            index: 0,
            position: 0,
            control: self.control.clone(),
        };
        Ok(Deflated::new(
            input.take(self.packed),
            self.control.clone(),
            false,
        ))
    }
    fn finish(decoder: &mut Deflated<Ranges>, packed: u64) -> Result<()> {
        if decoder.read(&mut [0]).map_err(payload_error)? != 0 || decoder.consumed() != packed {
            return Err(ArchiveError::CorruptArchive);
        }
        Ok(())
    }
    fn verify(&mut self) -> Result<()> {
        let mut decoder = self.decoder()?;
        let mut buffer = [0; 65536];
        for span in self.spans.iter() {
            let mut remaining = span.expanded;
            let mut crc = crc32fast::Hasher::new();
            while remaining != 0 {
                self.control.checkpoint()?;
                let count = remaining.min(buffer.len() as u64) as usize;
                decoder
                    .read_exact(&mut buffer[..count])
                    .map_err(payload_error)?;
                crc.update(&buffer[..count]);
                remaining -= count as u64;
            }
            if crc.finalize() != span.crc {
                return Err(ArchiveError::CorruptArchive);
            }
        }
        Self::finish(&mut decoder, self.packed)?;
        self.verified = true;
        Ok(())
    }
    pub(super) fn extract(
        &mut self,
        start: u64,
        size: u64,
        limits: &ResourceLimits,
        writer: &mut impl Write,
    ) -> Result<u64> {
        self.control.checkpoint()?;
        if limits.max_working_bytes < 256 << 10
            || self.expanded
                > limits
                    .max_output_bytes
                    .max(ResourceLimits::default().max_output_bytes)
        {
            return Err(ArchiveError::ResourceLimit);
        }
        if start
            .checked_add(size)
            .is_none_or(|end| end > self.expanded)
        {
            return Err(ArchiveError::CorruptArchive);
        }
        if self.spans.is_empty() {
            return Ok(0);
        }
        if !self.verified {
            self.verify()?;
        }
        if self.output.is_none() || self.position > start {
            self.output = Some(self.decoder()?);
            self.position = 0;
        }
        let decoder = self.output.as_mut().expect("solid replay initialized");
        let mut buffer = [0; 65536];
        while self.position < start {
            self.control.checkpoint()?;
            let count = (start - self.position).min(buffer.len() as u64) as usize;
            decoder
                .read_exact(&mut buffer[..count])
                .map_err(payload_error)?;
            self.position += count as u64;
        }
        let mut remaining = size;
        while remaining != 0 {
            self.control.checkpoint()?;
            let count = remaining.min(buffer.len() as u64) as usize;
            decoder
                .read_exact(&mut buffer[..count])
                .map_err(payload_error)?;
            writer.write_all(&buffer[..count])?;
            self.position += count as u64;
            remaining -= count as u64;
        }
        if self.position == self.expanded {
            Self::finish(decoder, self.packed)?;
        }
        Ok(size)
    }
}
