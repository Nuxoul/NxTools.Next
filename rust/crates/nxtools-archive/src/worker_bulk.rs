//! Versioned raw records on the original private worker pipes. No native authority lives here.
use std::io::{self, Read, Write};
use uuid::Uuid;
use zeroize::Zeroizing;

pub(crate) const FLAG: u32 = 0x8000_0000;
pub(crate) const HEADER: usize = 76;
pub(crate) const MAX_PAYLOAD: usize = 65536;
pub(crate) const OUTPUT: u8 = 1;
pub(crate) const SOURCE: u8 = 2;
pub(crate) const PARTIAL: u8 = 3;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Binding {
    pub kind: u8,
    pub operation: Uuid,
    pub call: Uuid,
    pub token: Uuid,
    pub sequence: u64,
    pub offset: u64,
    pub length: u32,
}
impl Binding {
    fn valid(self, actual: usize) -> bool {
        matches!(self.kind, OUTPUT | SOURCE | PARTIAL)
            && !self.operation.is_nil()
            && !self.call.is_nil()
            && !self.token.is_nil()
            && self.sequence != 0
            && (1..=MAX_PAYLOAD as u32).contains(&self.length)
            && actual <= self.length as usize
            && (self.kind != OUTPUT || actual == self.length as usize)
    }
    pub fn write(self, output: &mut impl Write, payload: &[u8]) -> io::Result<()> {
        if !self.valid(payload.len()) {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let mut header = [0u8; HEADER];
        header[0] = 1;
        header[1] = self.kind;
        header[4..20].copy_from_slice(self.operation.as_bytes());
        header[20..36].copy_from_slice(self.call.as_bytes());
        header[36..52].copy_from_slice(self.token.as_bytes());
        header[52..60].copy_from_slice(&self.sequence.to_le_bytes());
        header[60..68].copy_from_slice(&self.offset.to_le_bytes());
        header[68..72].copy_from_slice(&self.length.to_le_bytes());
        header[72..76].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        output.write_all(&(FLAG | (HEADER + payload.len()) as u32).to_le_bytes())?;
        output.write_all(&header)?;
        output.write_all(payload)?;
        output.flush()
    }
}

pub(crate) enum Packet {
    Json(Zeroizing<Vec<u8>>),
    Binary(Binding, Zeroizing<Vec<u8>>),
}

pub(crate) fn read(input: &mut impl Read) -> io::Result<Option<Packet>> {
    let mut prefix = [0u8; 4];
    if input.read(&mut prefix[..1])? == 0 {
        return Ok(None);
    }
    input.read_exact(&mut prefix[1..])?;
    let encoded = u32::from_le_bytes(prefix);
    let size = (encoded & !FLAG) as usize;
    if encoded & FLAG == 0 {
        if !(1..=super::worker::MAX_FRAME_BYTES).contains(&size) {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let mut body = Zeroizing::new(vec![0u8; size]);
        input.read_exact(&mut body)?;
        return Ok(Some(Packet::Json(body)));
    }
    if !(HEADER..=HEADER + MAX_PAYLOAD).contains(&size) {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let mut header = [0u8; HEADER];
    input.read_exact(&mut header)?;
    if header[0] != 1 || header[2..4] != [0, 0] {
        return Err(io::ErrorKind::InvalidData.into());
    }
    fn uuid(bytes: &[u8]) -> io::Result<Uuid> {
        Uuid::from_slice(bytes).map_err(|_| io::ErrorKind::InvalidData.into())
    }
    fn u64_at(bytes: &[u8]) -> io::Result<u64> {
        bytes
            .try_into()
            .map(u64::from_le_bytes)
            .map_err(|_| io::ErrorKind::InvalidData.into())
    }
    fn u32_at(bytes: &[u8]) -> io::Result<u32> {
        bytes
            .try_into()
            .map(u32::from_le_bytes)
            .map_err(|_| io::ErrorKind::InvalidData.into())
    }
    let binding = Binding {
        kind: header[1],
        operation: uuid(&header[4..20])?,
        call: uuid(&header[20..36])?,
        token: uuid(&header[36..52])?,
        sequence: u64_at(&header[52..60])?,
        offset: u64_at(&header[60..68])?,
        length: u32_at(&header[68..72])?,
    };
    let actual = u32_at(&header[72..76])? as usize;
    if size != HEADER + actual || !binding.valid(actual) {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let mut payload = Zeroizing::new(vec![0u8; actual]);
    input.read_exact(&mut payload)?;
    Ok(Some(Packet::Binary(binding, payload)))
}

#[cfg(test)]
#[path = "worker_bulk_tests.rs"]
mod tests;
