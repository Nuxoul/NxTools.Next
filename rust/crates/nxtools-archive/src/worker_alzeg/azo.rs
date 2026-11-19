//! EGG Appendix A: bounded arithmetic input, unfiltered history and streamed x86 output.
use crate::{ArchiveError, OperationControl, ResourceLimits, Result};
use std::io::{self, BufReader, Read};

const MASK: u64 = 0xffff_ffff;
const TOP: u64 = 0x8000_0000;
const SECOND: u64 = 0x4000_0000;
const FIXED_WORKING: u64 = 256 << 10;

struct Input<R> {
    reader: BufReader<R>,
    consumed: u64,
    control: OperationControl,
}
impl<R: Read> Input<R> {
    fn byte(&mut self) -> Result<u8> {
        if self.consumed.is_multiple_of(1024) {
            self.control.checkpoint()?;
        }
        let byte = super::exact::<1>(&mut self.reader)?[0];
        self.consumed += 1;
        Ok(byte)
    }
    fn word(&mut self) -> Result<u32> {
        let mut bytes = [0; 4];
        for byte in &mut bytes {
            *byte = self.byte()?;
        }
        Ok(u32::from_be_bytes(bytes))
    }
}

struct Arithmetic {
    low: u64,
    high: u64,
    tag: u64,
    remaining: u32,
    current: u8,
    bits: u8,
    lookahead: u8,
}
impl Arithmetic {
    fn new(remaining: u32) -> Self {
        Self {
            low: 0,
            high: MASK,
            tag: 0,
            remaining,
            current: 0,
            bits: 0,
            lookahead: 0,
        }
    }
    fn bit<R: Read>(&mut self, input: &mut Input<R>) -> Result<u64> {
        if self.bits == 0 {
            if self.remaining == 0 {
                self.lookahead += 1;
                if self.lookahead > 32 {
                    return Err(ArchiveError::CorruptArchive);
                }
                return Ok(0);
            }
            self.current = input.byte()?;
            self.remaining -= 1;
            self.bits = 8;
        }
        self.bits -= 1;
        Ok(u64::from((self.current >> self.bits) & 1))
    }
    fn seed<R: Read>(&mut self, input: &mut Input<R>) -> Result<()> {
        for _ in 0..32 {
            self.tag = (self.tag << 1) | self.bit(input)?;
        }
        Ok(())
    }
    fn unit(&self, bits: u8) -> Result<u64> {
        if self.tag < self.low || self.tag > self.high {
            return Err(ArchiveError::CorruptArchive);
        }
        let unit = (self.high - self.low + 1) >> bits;
        if unit == 0 {
            return Err(ArchiveError::CorruptArchive);
        }
        Ok(unit)
    }
    fn normalize<R: Read>(&mut self, input: &mut Input<R>) -> Result<()> {
        while self.low & TOP == self.high & TOP {
            self.tag = ((self.tag << 1) & MASK) | self.bit(input)?;
            self.low = (self.low << 1) & MASK;
            self.high = ((self.high << 1) & MASK) | 1;
        }
        while self.low & SECOND != 0 && self.high & SECOND == 0 {
            self.tag = (((self.tag << 1) | self.bit(input)?) ^ TOP) & MASK;
            self.low = (self.low << 1) & (MASK >> 1);
            self.high = ((self.high << 1) | TOP | 1) & MASK;
        }
        Ok(())
    }
    fn boolean<R: Read>(
        &mut self,
        input: &mut Input<R>,
        prob: &mut u16,
        bits: u8,
        shift: u8,
    ) -> Result<u32> {
        let unit = self.unit(bits)?;
        let split = self.low + unit * u64::from(*prob);
        let bit = u32::from(self.tag >= split);
        if bit == 0 {
            self.high = split - 1;
            *prob += ((1 << bits) - *prob) >> shift;
        } else {
            self.low = split;
            *prob -= *prob >> shift;
        }
        self.normalize(input)?;
        Ok(bit)
    }
    fn uniform<R: Read>(&mut self, input: &mut Input<R>, bits: u8) -> Result<u32> {
        if bits == 0 {
            return Ok(0);
        }
        let unit = self.unit(bits)?;
        let value = (self.tag - self.low) / unit;
        if value >= 1 << bits {
            return Err(ArchiveError::CorruptArchive);
        }
        self.high = self.low + unit * (value + 1) - 1;
        self.low += unit * value;
        self.normalize(input)?;
        Ok(value as u32)
    }
}

struct Flag {
    probs: [u16; 256],
    state: usize,
}
impl Flag {
    fn new() -> Self {
        Self {
            probs: [2048; 256],
            state: 0,
        }
    }
    fn decode<R: Read>(&mut self, code: &mut Arithmetic, input: &mut Input<R>) -> Result<u32> {
        let bit = code.boolean(input, &mut self.probs[self.state], 12, 6)?;
        self.state = (self.state * 2 + bit as usize) & 255;
        Ok(bit)
    }
}
fn tree<R: Read>(
    code: &mut Arithmetic,
    input: &mut Input<R>,
    probs: &mut [u16],
    width: u8,
) -> Result<u32> {
    let mut index = 1;
    for _ in 0..width {
        index = index * 2 + code.boolean(input, &mut probs[index], 10, 4)? as usize;
    }
    Ok((index - (1 << width)) as u32)
}
fn update(probs: &mut [u16], width: u8, value: u32) {
    let mut index = 1;
    for shift in (0..width).rev() {
        let bit = (value >> shift) & 1;
        if bit == 0 {
            probs[index] += (1024 - probs[index]) >> 4;
        } else {
            probs[index] -= probs[index] >> 4;
        }
        index = index * 2 + bit as usize;
    }
}
fn compare(fine: &[u16], coarse: &[u16], width: u8, value: u32) -> std::cmp::Ordering {
    let (mut a, mut b, mut index) = (1u32, 1u32, 1usize);
    for shift in (0..width).rev() {
        if (a | b) & 0xffc0_0000 != 0 {
            a >>= 10;
            b >>= 10;
        }
        let bit = (value >> shift) & 1;
        a = a.wrapping_mul(u32::from(if bit == 0 {
            fine[index]
        } else {
            1024 - fine[index]
        }));
        b = b.wrapping_mul(u32::from(if bit == 0 {
            coarse[index]
        } else {
            1024 - coarse[index]
        }));
        index = index * 2 + bit as usize;
    }
    a.cmp(&b)
}
struct Predictor {
    fine: Vec<u16>,
    coarse: Vec<u16>,
    lucky: Vec<i64>,
    width: u8,
    shift: u8,
}
impl Predictor {
    fn new(contexts: usize, width: u8, shift: u8) -> Self {
        Self {
            fine: vec![512; contexts << width],
            coarse: vec![512; (contexts >> shift) << width],
            lucky: vec![0; contexts],
            width,
            shift,
        }
    }
    fn reset(&mut self) {
        self.fine.fill(512);
        self.coarse.fill(512);
        self.lucky.fill(0);
    }
    fn decode<R: Read>(
        &mut self,
        code: &mut Arithmetic,
        input: &mut Input<R>,
        context: usize,
    ) -> Result<u32> {
        let size = 1 << self.width;
        let start = context * size;
        let group = (context >> self.shift) * size;
        let fine = &mut self.fine[start..start + size];
        let coarse = &mut self.coarse[group..group + size];
        let value = if self.lucky[context] >= 0 {
            let value = tree(code, input, fine, self.width)?;
            update(coarse, self.width, value);
            value
        } else {
            let value = tree(code, input, coarse, self.width)?;
            update(fine, self.width, value);
            value
        };
        self.lucky[context] += match compare(fine, coarse, self.width, value) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        };
        Ok(value)
    }
}
struct Codes {
    base: [u32; 128],
    extra: [u8; 128],
}
const fn codes(first: u32, threshold: usize, stride: usize) -> Codes {
    let mut table = Codes {
        base: [0; 128],
        extra: [0; 128],
    };
    table.base[0] = first;
    let mut index = 0;
    while index < 128 {
        if index >= threshold {
            table.extra[index] = ((index - threshold) / stride) as u8;
        }
        if index > 0 {
            table.base[index] = table.base[index - 1] + (1 << table.extra[index - 1]);
        }
        index += 1;
    }
    table
}
const DISTANCE: Codes = codes(1, 16, 4);
const LENGTH: Codes = codes(2, 32, 8);
fn promote<const N: usize>(values: &mut [u32; N], index: usize) -> u32 {
    let value = values[index];
    values.copy_within(0..index, 1);
    values[0] = value;
    value
}
fn push<const N: usize>(values: &mut [u32; N], value: u32) {
    values.copy_within(0..N - 1, 1);
    values[0] = value;
}
struct Models {
    alpha: Predictor,
    length: Predictor,
    flags: [Flag; 4],
    distance: [u16; 128],
    index: [u16; 128],
    short_distance: [u16; 2],
    short_index: [u16; 2],
    distances: [u32; 2],
    indices: [u32; 2],
    lengths: [u32; 128],
    positions: [u32; 128],
}
impl Models {
    fn new() -> Self {
        Self {
            alpha: Predictor::new(256, 8, 5),
            length: Predictor::new(128, 7, 4),
            flags: std::array::from_fn(|_| Flag::new()),
            distance: [512; 128],
            index: [512; 128],
            short_distance: [512; 2],
            short_index: [512; 2],
            distances: [1, 2],
            indices: [0, 1],
            lengths: std::array::from_fn(|i| i as u32 + 2),
            positions: [0; 128],
        }
    }
    fn reset(&mut self) {
        self.alpha.reset();
        self.length.reset();
        self.flags = std::array::from_fn(|_| Flag::new());
        self.distance.fill(512);
        self.index.fill(512);
        self.short_distance.fill(512);
        self.short_index.fill(512);
        self.distances = [1, 2];
        self.indices = [0, 1];
        self.lengths = std::array::from_fn(|i| i as u32 + 2);
        self.positions.fill(0);
    }
    fn pair<R: Read>(
        &mut self,
        code: &mut Arithmetic,
        input: &mut Input<R>,
        position: u32,
    ) -> Result<(u32, u32)> {
        if self.flags[1].decode(code, input)? != 0 {
            let index = if self.flags[3].decode(code, input)? != 0 {
                let slot = tree(code, input, &mut self.short_index, 1)? as usize;
                promote(&mut self.indices, slot)
            } else {
                let index = tree(code, input, &mut self.index, 7)?;
                push(&mut self.indices, index);
                index
            } as usize;
            let length = promote(&mut self.lengths, index);
            let start = promote(&mut self.positions, index);
            return Ok((
                position
                    .checked_sub(start)
                    .ok_or(ArchiveError::CorruptArchive)?,
                length,
            ));
        }
        let distance = if self.flags[2].decode(code, input)? != 0 {
            let slot = tree(code, input, &mut self.short_distance, 1)? as usize;
            promote(&mut self.distances, slot)
        } else {
            let symbol = tree(code, input, &mut self.distance, 7)? as usize;
            let distance = DISTANCE.base[symbol] + code.uniform(input, DISTANCE.extra[symbol])?;
            push(&mut self.distances, distance);
            distance
        };
        let context = DISTANCE.base.partition_point(|base| *base <= distance) - 1;
        let symbol = self.length.decode(code, input, context)? as usize;
        let length = LENGTH.base[symbol] + code.uniform(input, LENGTH.extra[symbol])?;
        push(&mut self.lengths, length);
        push(&mut self.positions, position);
        Ok((distance, length))
    }
}

pub(super) struct Azo<R> {
    input: Input<R>,
    code: Arithmetic,
    models: Models,
    history: Vec<u8>,
    dictionary_credit: u64,
    remaining: u64,
    block_size: u32,
    position: u32,
    match_distance: u32,
    match_left: u32,
    raw: bool,
    filter: bool,
    ended: bool,
    output: [u8; 5],
    output_at: usize,
    output_len: usize,
}
impl<R: Read> Azo<R> {
    pub(super) fn new(
        reader: R,
        expanded: u64,
        limits: &ResourceLimits,
        control: OperationControl,
    ) -> Result<Self> {
        let dictionary_credit = limits
            .max_working_bytes
            .checked_sub(FIXED_WORKING)
            .ok_or(ArchiveError::ResourceLimit)?;
        let mut input = Input {
            reader: BufReader::with_capacity(8192, reader),
            consumed: 0,
            control,
        };
        if input.byte()? != b'1' {
            return Err(ArchiveError::UnsupportedMethod);
        }
        let flags = input.byte()?;
        if flags & !1 != 0 {
            return Err(ArchiveError::UnsupportedMethod);
        }
        Ok(Self {
            input,
            code: Arithmetic::new(0),
            models: Models::new(),
            history: Vec::new(),
            dictionary_credit,
            remaining: expanded,
            block_size: 0,
            position: 0,
            match_distance: 0,
            match_left: 0,
            raw: false,
            filter: flags != 0,
            ended: false,
            output: [0; 5],
            output_at: 0,
            output_len: 0,
        })
    }
    pub(super) fn consumed(&self) -> u64 {
        self.input.consumed
    }
    fn next_block(&mut self) -> Result<bool> {
        self.input.control.checkpoint()?;
        if self.code.remaining != 0 {
            return Err(ArchiveError::CorruptArchive);
        }
        let expanded = self.input.word()?;
        let packed = self.input.word()?;
        if self.input.word()? != expanded ^ packed {
            return Err(ArchiveError::CorruptArchive);
        }
        if expanded == 0 && packed == 0 {
            if self.remaining != 0 {
                return Err(ArchiveError::CorruptArchive);
            }
            self.ended = true;
            return Ok(false);
        }
        if expanded == 0 || packed == 0 {
            return Err(ArchiveError::CorruptArchive);
        }
        self.remaining = self
            .remaining
            .checked_sub(u64::from(expanded))
            .ok_or(ArchiveError::CorruptArchive)?;
        self.block_size = expanded;
        self.position = 0;
        self.match_left = 0;
        self.raw = expanded == packed;
        self.code = Arithmetic::new(packed);
        if !self.raw {
            let capacity = usize::try_from(self.dictionary_credit.min(u64::from(expanded)))
                .map_err(|_| ArchiveError::ResourceLimit)?;
            if capacity == 0 {
                return Err(ArchiveError::ResourceLimit);
            }
            if capacity > self.history.len() {
                self.history
                    .try_reserve_exact(capacity - self.history.len())
                    .map_err(|_| ArchiveError::ResourceLimit)?;
            }
            self.history.resize(capacity, 0);
            self.models.reset();
            self.code.seed(&mut self.input)?;
        }
        Ok(true)
    }
    fn unfiltered(&mut self) -> Result<u8> {
        if self.position.is_multiple_of(4096) {
            self.input.control.checkpoint()?;
        }
        let byte = if self.raw {
            self.code.remaining -= 1;
            self.input.byte()?
        } else {
            if self.match_left == 0 {
                if self.position == 0
                    || self.models.flags[0].decode(&mut self.code, &mut self.input)? == 0
                {
                    let context = if self.position == 0 {
                        0
                    } else {
                        usize::from(self.history[(self.position as usize - 1) % self.history.len()])
                    };
                    let byte = self
                        .models
                        .alpha
                        .decode(&mut self.code, &mut self.input, context)?
                        as u8;
                    let index = self.position as usize % self.history.len();
                    self.history[index] = byte;
                    self.position += 1;
                    return Ok(byte);
                }
                let (distance, length) =
                    self.models
                        .pair(&mut self.code, &mut self.input, self.position)?;
                if distance == 0
                    || distance > self.position
                    || length > self.block_size - self.position
                {
                    return Err(ArchiveError::CorruptArchive);
                }
                if distance as usize > self.history.len() {
                    return Err(ArchiveError::ResourceLimit);
                }
                self.match_distance = distance;
                self.match_left = length;
            }
            let byte =
                self.history[(self.position - self.match_distance) as usize % self.history.len()];
            let index = self.position as usize % self.history.len();
            self.history[index] = byte;
            self.match_left -= 1;
            byte
        };
        self.position += 1;
        Ok(byte)
    }
    fn next_output(&mut self) -> Result<bool> {
        if self.ended {
            return Ok(false);
        }
        if self.position == self.block_size && !self.next_block()? {
            return Ok(false);
        }
        let start = self.position;
        self.output[0] = self.unfiltered()?;
        self.output_at = 0;
        self.output_len = 1;
        if self.filter && matches!(self.output[0], 0xe8 | 0xe9) {
            while self.output_len < 5 && self.position < self.block_size {
                self.output[self.output_len] = self.unfiltered()?;
                self.output_len += 1;
            }
            if self.output_len == 5 && matches!(self.output[4], 0 | 0xff) {
                let mut address =
                    u32::from_le_bytes(self.output[1..5].try_into().expect("address"))
                        .wrapping_sub(start);
                address = if address & 0x0100_0000 != 0 {
                    address | 0xff00_0000
                } else {
                    address & 0x00ff_ffff
                };
                self.output[1..5].copy_from_slice(&address.to_le_bytes());
            }
        }
        Ok(true)
    }
}
impl<R: Read> Read for Azo<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.input.control.checkpoint().map_err(io::Error::other)?;
        let mut written = 0;
        while written < buffer.len() {
            if self.output_at == self.output_len && !self.next_output().map_err(io::Error::other)? {
                break;
            }
            let count = (buffer.len() - written).min(self.output_len - self.output_at);
            buffer[written..written + count]
                .copy_from_slice(&self.output[self.output_at..self.output_at + count]);
            written += count;
            self.output_at += count;
        }
        Ok(written)
    }
}
