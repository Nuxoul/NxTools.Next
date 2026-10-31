//! Shared native ID-source adapter. Authority and protected object handles remain in C#.
use crate::{
    ArchiveError, OperationControl, Result,
    creation::{CreationSource, PlannedSource, SourceEntry},
    worker_output::PrivateOutput,
};
use sha2::{Digest, Sha256};
use std::{
    cell::{Cell, RefCell},
    io::{self, Read, Write},
};
use uuid::Uuid;

pub(crate) type SharedNative<'b, 'a, W> = RefCell<&'b mut PrivateOutput<'a, W>>;
pub(crate) struct Sources<'b, 'a, 'n, W: Write> {
    native: &'n SharedNative<'b, 'a, W>,
    observed: Vec<Uuid>,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceDto {
    id: Uuid,
    name: String,
    directory: bool,
    size: u64,
    dos_modified: Option<(u16, u16)>,
}
impl From<SourceDto> for SourceEntry {
    fn from(value: SourceDto) -> Self {
        Self {
            id: value.id,
            name: value.name,
            directory: value.directory,
            size: value.size,
            dos_modified: value.dos_modified,
        }
    }
}
impl<'b, 'a, 'n, W: Write + Send> Sources<'b, 'a, 'n, W> {
    pub(crate) fn new(native: &'n SharedNative<'b, 'a, W>) -> Self {
        Self {
            native,
            observed: Vec::new(),
        }
    }
    fn metadata(&self, action: &str, token: Option<Uuid>, offset: Option<u64>) -> Result<String> {
        let reply = self.native.borrow_mut().call_at(
            action,
            None,
            token,
            &[],
            offset.map(|value| (value, 1)),
        )?;
        let value = reply.read_data.ok_or(ArchiveError::IoFailure)?;
        if value.len() > 262144 {
            return Err(ArchiveError::ResourceLimit);
        }
        Ok(value)
    }
}
impl<W: Write + Send> CreationSource for Sources<'_, '_, '_, W> {
    fn roots(&mut self) -> Result<Vec<SourceEntry>> {
        let values: Vec<SourceDto> =
            serde_json::from_str(&self.metadata("source_roots", None, None)?)
                .map_err(|_| ArchiveError::IoFailure)?;
        if values.len() > 32 {
            return Err(ArchiveError::ResourceLimit);
        }
        self.observed.extend(values.iter().map(|value| value.id));
        Ok(values.into_iter().map(Into::into).collect())
    }
    fn next_child(&mut self, directory: Uuid, offset: u64) -> Result<Option<SourceEntry>> {
        let value: Option<SourceDto> =
            serde_json::from_str(&self.metadata("source_next", Some(directory), Some(offset))?)
                .map_err(|_| ArchiveError::IoFailure)?;
        if let Some(value) = &value {
            self.observed.push(value.id);
        }
        Ok(value.map(Into::into))
    }
    fn open_file(&mut self, id: Uuid) -> Result<()> {
        self.native
            .borrow_mut()
            .call_at("source_open", None, Some(id), &[], None)
            .map(|_| ())
    }
    fn read_file(&mut self, id: Uuid, offset: u64, length: u32) -> Result<Vec<u8>> {
        let reply = self.native.borrow_mut().call_at(
            "source_read",
            None,
            Some(id),
            &[],
            Some((offset, length)),
        )?;
        use base64::Engine as _;
        let value = reply.read_data.ok_or(ArchiveError::IoFailure)?;
        if value.len() > 87384 {
            return Err(ArchiveError::ResourceLimit);
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(value)
            .map_err(|_| ArchiveError::IoFailure)?;
        if bytes.len() > length as usize {
            return Err(ArchiveError::ResourceLimit);
        }
        Ok(bytes)
    }
    fn close_file(&mut self, id: Uuid) -> Result<()> {
        self.native
            .borrow_mut()
            .call_at("source_close", None, Some(id), &[], None)
            .map(|_| ())
    }
    fn revalidate(&mut self) -> Result<()> {
        for id in &self.observed {
            self.native
                .borrow_mut()
                .call_at("source_check", None, Some(*id), &[], None)?;
        }
        Ok(())
    }
}

/// Lazy, sequential protected reader. Solid codecs can retain readers without retaining handles.
/// EOF closes the current native file before the next reader opens; errors survive backend wrappers.
pub(crate) struct Reader<'s, S: CreationSource> {
    sources: &'s RefCell<S>,
    item: &'s PlannedSource,
    control: &'s OperationControl,
    failure: &'s Cell<Option<ArchiveError>>,
    digests: &'s RefCell<Vec<Option<[u8; 32]>>>,
    index: usize,
    offset: u64,
    open: bool,
    finished: bool,
    digest: Sha256,
}
impl<'s, S: CreationSource> Reader<'s, S> {
    pub(crate) fn new(
        sources: &'s RefCell<S>,
        item: &'s PlannedSource,
        control: &'s OperationControl,
        failure: &'s Cell<Option<ArchiveError>>,
        digests: &'s RefCell<Vec<Option<[u8; 32]>>>,
        index: usize,
    ) -> Self {
        Self {
            sources,
            item,
            control,
            failure,
            digests,
            index,
            offset: 0,
            open: false,
            finished: false,
            digest: Sha256::new(),
        }
    }
    fn next(&mut self, buffer: &mut [u8]) -> Result<usize> {
        self.control.checkpoint()?;
        if let Some(failure) = self.failure.get() {
            return Err(failure);
        }
        if self.finished || buffer.is_empty() {
            return Ok(0);
        }
        if !self.open {
            self.sources.borrow_mut().open_file(self.item.source.id)?;
            self.open = true;
        }
        let bytes = self.sources.borrow_mut().read_file(
            self.item.source.id,
            self.offset,
            buffer.len().min(65536) as u32,
        )?;
        if bytes.is_empty() {
            if self.offset != self.item.size() {
                return Err(ArchiveError::SourceChanged);
            }
            self.open = false;
            self.sources.borrow_mut().close_file(self.item.source.id)?;
            self.digests.borrow_mut()[self.index] = Some(self.digest.clone().finalize().into());
            self.finished = true;
        } else {
            self.offset = self
                .offset
                .checked_add(bytes.len() as u64)
                .ok_or(ArchiveError::ResourceLimit)?;
            if self.offset > self.item.size() {
                return Err(ArchiveError::SourceChanged);
            }
            self.digest.update(&bytes);
            buffer[..bytes.len()].copy_from_slice(&bytes);
        }
        Ok(bytes.len())
    }
}
impl<S: CreationSource> Read for Reader<'_, S> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.next(buffer).map_err(|failure| {
            if self.failure.get().is_none() {
                self.failure.set(Some(failure));
            }
            io::Error::other(failure)
        })
    }
}
impl<S: CreationSource> Drop for Reader<'_, S> {
    fn drop(&mut self) {
        if self.open
            && let Err(failure) = self.sources.borrow_mut().close_file(self.item.source.id)
            && self.failure.get().is_none()
        {
            self.failure.set(Some(failure));
        }
    }
}
