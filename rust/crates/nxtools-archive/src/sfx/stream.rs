use super::input::Parts;
use super::{Entry, Error, Result, add, require};
use flate2::{Decompress, FlushDecompress, Status};
use std::io::Write;

pub(super) fn decode(parts: &mut Parts, entry: &Entry, output: &mut impl Write) -> Result<()> {
    let mut input = [0u8; 65536];
    let mut plain = [0u8; 65536];
    let mut crc = crc32fast::Hasher::new();
    let mut decoded = 0u64;
    let mut loaded = 0u64;
    if entry.method == 0 {
        require(entry.packed == entry.size)?;
        while loaded < entry.packed {
            let count = (entry.packed - loaded).min(input.len() as u64) as usize;
            parts.read(add(entry.data, loaded)?, &mut input[..count])?;
            output.write_all(&input[..count])?;
            crc.update(&input[..count]);
            loaded = add(loaded, count as u64)?;
        }
        decoded = loaded;
    } else {
        let mut decoder = Decompress::new(false);
        let mut available = 0usize;
        let mut position = 0usize;
        loop {
            if position == available && loaded < entry.packed {
                available = (entry.packed - loaded).min(input.len() as u64) as usize;
                parts.read(add(entry.data, loaded)?, &mut input[..available])?;
                loaded = add(loaded, available as u64)?;
                position = 0;
            }
            let before_in = decoder.total_in();
            let before_out = decoder.total_out();
            let status = decoder
                .decompress(
                    &input[position..available],
                    &mut plain,
                    FlushDecompress::None,
                )
                .map_err(|_| Error::corrupt())?;
            let used = (decoder.total_in() - before_in) as usize;
            let made = (decoder.total_out() - before_out) as usize;
            position += used;
            decoded = add(decoded, made as u64)?;
            require(decoded <= entry.size)?;
            crc.update(&plain[..made]);
            output.write_all(&plain[..made])?;
            if status == Status::StreamEnd {
                require(decoder.total_in() == entry.packed)?;
                break;
            }
            require(used != 0 || made != 0)?;
        }
    }
    require(decoded == entry.size && crc.finalize() == entry.crc)
}
