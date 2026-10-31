//! Explicit v2 data accounting. Unknown transfers never receive a budget exemption.
use crate::{
    InvocationError,
    blobs::{BlobChunk, MAX_CHUNK_BYTES},
};
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;
pub(crate) const DATA_DECODED_BYTES: usize = 16 * 1024 * 1024;
pub(crate) const DATA_WIRE_BYTES: usize = 24 * 1024 * 1024;
const TRANSFERS: usize = 4;
const CHUNKS: usize = 512;
#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DataFrame {
    pub protocol_version: u16,
    pub op: String,
    pub session_id: Uuid,
    pub run_id: Uuid,
    pub transfer_id: Uuid,
    pub sequence: u64,
    pub offset: usize,
    pub data: String,
}
#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DataAck {
    pub protocol_version: u16,
    pub op: String,
    pub session_id: Uuid,
    pub run_id: Uuid,
    pub transfer_id: Uuid,
    pub sequence: u64,
    pub offset: usize,
}
#[derive(Default)]
pub(crate) struct DataPlane {
    transfers: usize,
    chunks: usize,
    input_wire: usize,
    output_wire: usize,
    input_decoded: usize,
    output_decoded: usize,
    uploads: HashMap<Uuid, Upload>,
    downloads: HashMap<Uuid, Download>,
}
struct Upload {
    length: usize,
    offset: usize,
    sequence: u64,
    pending: bool,
}
struct Download {
    offset: usize,
    end: usize,
    sequence: u64,
    pending: Option<(u64, usize)>,
}
fn decoded_length(data: &str) -> Result<usize, InvocationError> {
    if data.is_empty()
        || data.len() > MAX_CHUNK_BYTES.div_ceil(3) * 4
        || !data.len().is_multiple_of(4)
    {
        return Err(InvocationError::InvalidFrame);
    }
    let padding = data.bytes().rev().take_while(|byte| *byte == b'=').count();
    if padding > 2
        || !data.as_bytes()[..data.len() - padding]
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'+' | b'/'))
    {
        return Err(InvocationError::InvalidFrame);
    }
    let decoded = data.len() / 4 * 3 - padding;
    if decoded == 0 || decoded > MAX_CHUNK_BYTES {
        return Err(InvocationError::InvalidFrame);
    }
    Ok(decoded)
}
impl DataPlane {
    fn transfer(&mut self) -> Result<(), InvocationError> {
        if self.transfers >= TRANSFERS {
            return Err(InvocationError::InvalidFrame);
        }
        self.transfers += 1;
        Ok(())
    }
    pub(crate) fn upload(&mut self, id: Uuid, length: usize) -> Result<(), InvocationError> {
        if self.uploads.contains_key(&id) || self.downloads.contains_key(&id) {
            return Err(InvocationError::InvalidFrame);
        }
        self.transfer()?;
        self.uploads.insert(
            id,
            Upload {
                length,
                offset: 0,
                sequence: 0,
                pending: false,
            },
        );
        Ok(())
    }
    pub(crate) fn download(
        &mut self,
        id: Uuid,
        offset: usize,
        length: usize,
    ) -> Result<(), InvocationError> {
        if self.downloads.contains_key(&id) || self.uploads.contains_key(&id) {
            return Err(InvocationError::InvalidFrame);
        }
        self.transfer()?;
        let end = offset
            .checked_add(length)
            .ok_or(InvocationError::InvalidFrame)?;
        if length > 0 {
            self.downloads.insert(
                id,
                Download {
                    offset,
                    end,
                    sequence: 0,
                    pending: None,
                },
            );
        }
        Ok(())
    }
    pub(crate) fn accept_data(
        &mut self,
        frame: &DataFrame,
        wire: usize,
    ) -> Result<(), InvocationError> {
        let decoded = decoded_length(&frame.data)?;
        let upload = self
            .uploads
            .get(&frame.transfer_id)
            .ok_or(InvocationError::InvalidFrame)?;
        let end = frame
            .offset
            .checked_add(decoded)
            .ok_or(InvocationError::InvalidFrame)?;
        if upload.pending
            || frame.sequence != upload.sequence + 1
            || frame.offset != upload.offset
            || end > upload.length
        {
            return Err(InvocationError::InvalidFrame);
        }
        self.account(false, wire, decoded)?;
        let upload = self
            .uploads
            .get_mut(&frame.transfer_id)
            .ok_or(InvocationError::InvalidFrame)?;
        upload.offset = end;
        upload.sequence = frame.sequence;
        upload.pending = true;
        Ok(())
    }
    pub(crate) fn upload_ack(
        &mut self,
        id: Uuid,
        sequence: u64,
        offset: usize,
    ) -> Result<(), InvocationError> {
        let upload = self
            .uploads
            .get_mut(&id)
            .ok_or(InvocationError::InvalidFrame)?;
        if !upload.pending || upload.sequence != sequence || upload.offset != offset {
            return Err(InvocationError::InvalidFrame);
        }
        upload.pending = false;
        Ok(())
    }
    pub(crate) fn send_data(
        &mut self,
        chunk: &BlobChunk,
        wire: usize,
    ) -> Result<(), InvocationError> {
        let decoded = decoded_length(&chunk.data)?;
        let download = self
            .downloads
            .get(&chunk.transfer_id)
            .ok_or(InvocationError::InvalidFrame)?;
        let end = chunk
            .offset
            .checked_add(decoded)
            .ok_or(InvocationError::InvalidFrame)?;
        if download.pending.is_some()
            || chunk.sequence != download.sequence + 1
            || chunk.offset != download.offset
            || end > download.end
        {
            return Err(InvocationError::InvalidFrame);
        }
        self.account(true, wire, decoded)?;
        self.downloads
            .get_mut(&chunk.transfer_id)
            .ok_or(InvocationError::InvalidFrame)?
            .pending = Some((chunk.sequence, end));
        Ok(())
    }
    pub(crate) fn accept_ack(&mut self, ack: &DataAck) -> Result<(), InvocationError> {
        let download = self
            .downloads
            .get_mut(&ack.transfer_id)
            .ok_or(InvocationError::InvalidFrame)?;
        if download.pending != Some((ack.sequence, ack.offset)) {
            return Err(InvocationError::InvalidFrame);
        }
        download.offset = ack.offset;
        download.sequence = ack.sequence;
        download.pending = None;
        if ack.offset == download.end {
            self.downloads.remove(&ack.transfer_id);
        }
        Ok(())
    }
    fn account(&mut self, input: bool, wire: usize, decoded: usize) -> Result<(), InvocationError> {
        let (wire_total, decoded_total) = if input {
            (&mut self.input_wire, &mut self.input_decoded)
        } else {
            (&mut self.output_wire, &mut self.output_decoded)
        };
        let next_wire = wire_total
            .checked_add(wire)
            .ok_or(InvocationError::InvalidFrame)?;
        let next_decoded = decoded_total
            .checked_add(decoded)
            .ok_or(InvocationError::InvalidFrame)?;
        if next_wire > DATA_WIRE_BYTES || next_decoded > DATA_DECODED_BYTES || self.chunks >= CHUNKS
        {
            return Err(InvocationError::InvalidFrame);
        }
        *wire_total = next_wire;
        *decoded_total = next_decoded;
        self.chunks += 1;
        Ok(())
    }
    pub(crate) fn release_upload(&mut self, id: Uuid) -> Result<(), InvocationError> {
        let upload = self.uploads.get(&id).ok_or(InvocationError::InvalidFrame)?;
        if upload.pending {
            return Err(InvocationError::InvalidFrame);
        }
        self.uploads.remove(&id);
        Ok(())
    }
    pub(crate) fn quiescent(&self) -> bool {
        self.downloads.is_empty()
            && self
                .uploads
                .values()
                .all(|upload| upload.offset == upload.length && !upload.pending)
    }
}
