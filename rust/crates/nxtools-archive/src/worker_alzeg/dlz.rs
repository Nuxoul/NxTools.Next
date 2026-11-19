//! ALZ method1: fixed-size BWT blocks with bit-aligned DLZ framing.
use crate::{ArchiveError, OperationControl, ResourceLimits, Result};
use std::io::{self, BufReader, Read};

const BLOCK_BYTES: usize = 900_000;
const SELECTORS: usize = 18_002;
const WORKING_BYTES: u64 = 5 << 20;

struct Bits<R: Read> {
    input: BufReader<R>,
    current: u8,
    left: u8,
    consumed: u64,
    control: OperationControl,
}
impl<R: Read> Bits<R> {
    fn read(&mut self, mut count: u8) -> Result<u32> {
        let mut value = 0;
        while count != 0 {
            if self.left == 0 {
                if self.consumed.is_multiple_of(1024) {
                    self.control.checkpoint()?;
                }
                self.current = super::exact::<1>(&mut self.input)?[0];
                self.left = 8;
                self.consumed += 1;
            }
            let take = count.min(self.left);
            self.left -= take;
            value = (value << take) | (u32::from(self.current >> self.left) & ((1 << take) - 1));
            count -= take;
        }
        Ok(value)
    }
}

struct Huffman {
    first: [u32; 21],
    count: [u32; 21],
    offset: [usize; 21],
    symbols: [u16; 258],
}
impl Huffman {
    fn read<R: Read>(bits: &mut Bits<R>, alphabet: usize) -> Result<Self> {
        let mut lengths = [0u8; 258];
        let mut length = bits.read(5)? as i32;
        for slot in &mut lengths[..alphabet] {
            if !(1..=20).contains(&length) {
                return Err(ArchiveError::CorruptArchive);
            }
            while bits.read(1)? != 0 {
                length += if bits.read(1)? == 0 { 1 } else { -1 };
                if !(1..=20).contains(&length) {
                    return Err(ArchiveError::CorruptArchive);
                }
            }
            *slot = length as u8;
        }
        let mut table = Self {
            first: [0; 21],
            count: [0; 21],
            offset: [0; 21],
            symbols: [0; 258],
        };
        for &length in &lengths[..alphabet] {
            table.count[usize::from(length)] += 1;
        }
        let mut code = 0;
        let mut index = 0;
        for width in 1..=20 {
            table.first[width] = code;
            table.offset[width] = index;
            if code + table.count[width] > 1 << width {
                return Err(ArchiveError::CorruptArchive);
            }
            for (symbol, &length) in lengths[..alphabet].iter().enumerate() {
                if usize::from(length) == width {
                    table.symbols[index] = symbol as u16;
                    index += 1;
                }
            }
            code = (code + table.count[width]) << 1;
        }
        Ok(table)
    }
    fn symbol<R: Read>(&self, bits: &mut Bits<R>) -> Result<usize> {
        let mut code = 0;
        for width in 1..=20 {
            code = (code << 1) | bits.read(1)?;
            if let Some(delta) = code.checked_sub(self.first[width])
                && delta < self.count[width]
            {
                return Ok(usize::from(
                    self.symbols[self.offset[width] + delta as usize],
                ));
            }
        }
        Err(ArchiveError::CorruptArchive)
    }
}

struct Symbols {
    selectors: [u8; SELECTORS],
    selector_count: usize,
    tables: [Option<Huffman>; 6],
    decoded: usize,
}
impl Symbols {
    fn read<R: Read>(bits: &mut Bits<R>, alphabet: usize) -> Result<Self> {
        let groups = bits.read(3)? as usize;
        let selector_count = bits.read(15)? as usize;
        if !(2..=6).contains(&groups) || !(1..=SELECTORS).contains(&selector_count) {
            return Err(ArchiveError::CorruptArchive);
        }
        let mut result = Self {
            selectors: [0; SELECTORS],
            selector_count,
            tables: std::array::from_fn(|_| None),
            decoded: 0,
        };
        let mut order = [0u8, 1, 2, 3, 4, 5];
        for slot in &mut result.selectors[..selector_count] {
            let mut index = 0;
            while bits.read(1)? != 0 {
                index += 1;
                if index >= groups {
                    return Err(ArchiveError::CorruptArchive);
                }
            }
            let chosen = order[index];
            order.copy_within(..index, 1);
            order[0] = chosen;
            *slot = chosen;
        }
        for table in &mut result.tables[..groups] {
            *table = Some(Huffman::read(bits, alphabet)?);
        }
        Ok(result)
    }
    fn next<R: Read>(&mut self, bits: &mut Bits<R>) -> Result<usize> {
        if self.decoded.is_multiple_of(4096) {
            bits.control.checkpoint()?;
        }
        let selector = self.decoded / 50;
        if selector >= self.selector_count {
            return Err(ArchiveError::CorruptArchive);
        }
        self.decoded += 1;
        self.tables[usize::from(self.selectors[selector])]
            .as_ref()
            .ok_or(ArchiveError::CorruptArchive)?
            .symbol(bits)
    }
}

pub(super) struct Dlz<R: Read> {
    bits: Bits<R>,
    // Low byte is the BWT symbol; high bits hold the inverse permutation.
    table: Vec<u32>,
    cursor: usize,
    remaining: usize,
    last: u8,
    repeated: u8,
    pending: usize,
    finished: bool,
}
impl<R: Read> Dlz<R> {
    pub(super) fn new(
        input: R,
        limits: &ResourceLimits,
        control: OperationControl,
    ) -> Result<Self> {
        if limits.max_working_bytes < WORKING_BYTES {
            return Err(ArchiveError::ResourceLimit);
        }
        let mut table = Vec::new();
        table
            .try_reserve_exact(BLOCK_BYTES)
            .map_err(|_| ArchiveError::ResourceLimit)?;
        Ok(Self {
            bits: Bits {
                input: BufReader::with_capacity(8192, input),
                current: 0,
                left: 0,
                consumed: 0,
                control,
            },
            table,
            cursor: 0,
            remaining: 0,
            last: 0,
            repeated: 0,
            pending: 0,
            finished: false,
        })
    }
    pub(super) fn consumed(&self) -> u64 {
        self.bits.consumed
    }
    fn block(&mut self) -> Result<bool> {
        self.bits.control.checkpoint()?;
        match self.bits.read(32)? {
            0x444c5a02 => {
                self.finished = true;
                return Ok(false);
            }
            0x444c5a01 => {}
            _ => return Err(ArchiveError::CorruptArchive),
        }
        let origin = self.bits.read(24)? as usize;
        let used = self.bits.read(16)?;
        let mut mtf = [0u8; 256];
        let mut alphabet = 0;
        for group in 0..16 {
            if used & (1 << (15 - group)) != 0 {
                let members = self.bits.read(16)?;
                for value in 0..16 {
                    if members & (1 << (15 - value)) != 0 {
                        mtf[alphabet] = (group * 16 + value) as u8;
                        alphabet += 1;
                    }
                }
            }
        }
        if alphabet == 0 {
            return Err(ArchiveError::CorruptArchive);
        }
        let mut symbols = Symbols::read(&mut self.bits, alphabet + 2)?;
        self.table.clear();
        let end = alphabet + 1;
        let mut symbol = symbols.next(&mut self.bits)?;
        while symbol != end {
            if symbol < 2 {
                let mut run = 0;
                let mut weight = 1;
                while symbol < 2 {
                    run += (symbol + 1) * weight;
                    if self.table.len() + run > BLOCK_BYTES {
                        return Err(ArchiveError::CorruptArchive);
                    }
                    weight <<= 1;
                    symbol = symbols.next(&mut self.bits)?;
                }
                self.table.resize(self.table.len() + run, u32::from(mtf[0]));
            } else {
                let index = symbol - 1;
                if index >= alphabet || self.table.len() == BLOCK_BYTES {
                    return Err(ArchiveError::CorruptArchive);
                }
                let value = mtf[index];
                mtf.copy_within(..index, 1);
                mtf[0] = value;
                self.table.push(u32::from(value));
                symbol = symbols.next(&mut self.bits)?;
            }
        }
        if origin >= self.table.len() {
            return Err(ArchiveError::CorruptArchive);
        }
        let mut positions = [0usize; 257];
        for (index, &value) in self.table.iter().enumerate() {
            if index % 4096 == 0 {
                self.bits.control.checkpoint()?;
            }
            positions[value as usize + 1] += 1;
        }
        for value in 0..256 {
            positions[value + 1] += positions[value];
        }
        for index in 0..self.table.len() {
            if index % 4096 == 0 {
                self.bits.control.checkpoint()?;
            }
            let value = (self.table[index] & 255) as usize;
            let target = positions[value];
            positions[value] += 1;
            self.table[target] |= (index as u32) << 8;
        }
        self.cursor = (self.table[origin] >> 8) as usize;
        self.remaining = self.table.len();
        self.repeated = 0;
        Ok(true)
    }
    fn next_bwt(&mut self) -> u8 {
        let cell = self.table[self.cursor];
        self.cursor = (cell >> 8) as usize;
        self.remaining -= 1;
        cell as u8
    }
    fn read_checked(&mut self, output: &mut [u8]) -> Result<usize> {
        let mut count = 0;
        while count < output.len() && !self.finished {
            if count % 4096 == 0 {
                self.bits.control.checkpoint()?;
            }
            if self.pending != 0 {
                let take = self.pending.min(output.len() - count);
                output[count..count + take].fill(self.last);
                self.pending -= take;
                count += take;
                continue;
            }
            if self.repeated == 4 {
                if self.remaining == 0 {
                    return Err(ArchiveError::CorruptArchive);
                }
                self.pending = usize::from(self.next_bwt());
                self.repeated = 0;
                continue;
            }
            if self.remaining == 0 {
                if !self.block()? {
                    break;
                }
                continue;
            }
            let value = self.next_bwt();
            self.repeated = if self.repeated != 0 && value == self.last {
                self.repeated + 1
            } else {
                1
            };
            self.last = value;
            output[count] = value;
            count += 1;
        }
        Ok(count)
    }
}
impl<R: Read> Read for Dlz<R> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        self.read_checked(output).map_err(io::Error::other)
    }
}
