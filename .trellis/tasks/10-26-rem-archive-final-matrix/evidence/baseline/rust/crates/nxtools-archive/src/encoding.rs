use crate::{ArchiveError, NameEncoding, NameProvenance, Result};

/// ZIP-only heuristic. At most three strict decodes; existing central-directory admission
/// bounds raw bytes and decoded strings before this runs. Never inferred Unicode authority.
pub(crate) fn decode_zip(
    raw: &[u8],
    utf8: bool,
    extra: &[u8],
    selected: NameEncoding,
    candidate_budget: u64,
) -> Result<(String, NameProvenance)> {
    let decoded = decode(raw, utf8, extra, selected)?;
    if decoded.1 != NameProvenance::UncertainCp437 || raw.is_ascii() {
        return Ok(decoded);
    }
    // The fallback was already admitted by the original central-directory estimate.
    // Only two additional decoded strings can coexist, each at most 3x raw bytes.
    if (raw.len() as u64) * 6 > candidate_budget {
        return Err(ArchiveError::ResourceLimit);
    }
    let mut candidate = None;
    for (page, decoder) in [
        (NameEncoding::Utf8, encoding_rs::UTF_8),
        (NameEncoding::Gbk, encoding_rs::GBK),
        (NameEncoding::ShiftJis, encoding_rs::SHIFT_JIS),
    ] {
        let Some(name) = decoder.decode_without_bom_handling_and_without_replacement(raw) else {
            continue;
        };
        let mut letters = 0;
        let mut kana = 0;
        let plausible = name.chars().filter(|c| !c.is_ascii()).all(|c| {
            letters += 1;
            let han = matches!(c, '\u{3400}'..='\u{4dbf}' | '\u{4e00}'..='\u{9fff}');
            let is_kana = matches!(c, '\u{3041}'..='\u{3096}' | '\u{30a1}'..='\u{30fa}' | '\u{ff66}'..='\u{ff9d}');
            kana += usize::from(matches!(c, '\u{3041}'..='\u{3096}' | '\u{30a1}'..='\u{30fa}'));
            match page {
                NameEncoding::Utf8 => c.is_alphabetic(),
                NameEncoding::Gbk => han,
                NameEncoding::ShiftJis => han || is_kana,
                _ => unreachable!("three fixed candidates"),
            }
        });
        // Decode success alone is insufficient. Short names and Japanese all-Han names
        // have no distinguishing evidence; competing plausible scripts are not ranked.
        if !plausible || letters < 4 {
            continue;
        }
        if candidate.is_some() {
            return Ok(decoded);
        }
        candidate = Some((
            name.into_owned(),
            page,
            page != NameEncoding::ShiftJis || kana >= 2,
        ));
    }
    Ok(candidate
        .filter(|value| value.2)
        .map(|(name, page, _)| (name, NameProvenance::Heuristic(page)))
        .unwrap_or(decoded))
}

pub(crate) fn decode(
    raw: &[u8],
    utf8: bool,
    extra: &[u8],
    selected: NameEncoding,
) -> Result<(String, NameProvenance)> {
    if utf8 {
        return Ok((
            std::str::from_utf8(raw)
                .map_err(|_| ArchiveError::CorruptArchive)?
                .to_owned(),
            NameProvenance::Utf8Flag,
        ));
    }
    let mut offset = 0;
    while offset + 4 <= extra.len() {
        let tag = u16::from_le_bytes([extra[offset], extra[offset + 1]]);
        let size = usize::from(u16::from_le_bytes([extra[offset + 2], extra[offset + 3]]));
        offset += 4;
        let Some(value) = extra.get(offset..offset + size) else {
            return Err(ArchiveError::CorruptArchive);
        };
        if tag == 0x7075 && value.len() >= 5 && value[0] == 1 {
            let checksum = u32::from_le_bytes(
                value[1..5]
                    .try_into()
                    .map_err(|_| ArchiveError::CorruptArchive)?,
            );
            if checksum == crc32(raw)
                && let Ok(name) = std::str::from_utf8(&value[5..])
            {
                return Ok((name.to_owned(), NameProvenance::UnicodeExtra));
            }
        }
        offset += size;
    }
    let decoder = match selected {
        NameEncoding::Auto | NameEncoding::Cp437 => None,
        NameEncoding::Utf8 => Some(encoding_rs::UTF_8),
        NameEncoding::Gbk => Some(encoding_rs::GBK),
        NameEncoding::Big5 => Some(encoding_rs::BIG5),
        NameEncoding::ShiftJis => Some(encoding_rs::SHIFT_JIS),
        NameEncoding::Korean => Some(encoding_rs::EUC_KR),
        NameEncoding::Windows1250 => Some(encoding_rs::WINDOWS_1250),
        NameEncoding::Windows1251 => Some(encoding_rs::WINDOWS_1251),
        NameEncoding::Windows1252 => Some(encoding_rs::WINDOWS_1252),
    };
    let name = if let Some(decoder) = decoder {
        decoder
            .decode_without_bom_handling_and_without_replacement(raw)
            .ok_or(ArchiveError::CorruptArchive)?
            .into_owned()
    } else {
        // Unicode CP437 mapping; ASCII control bytes remain controls for path policy to reject.
        const HIGH: &str = "ÇüéâäàåçêëèïîìÄÅÉæÆôöòûùÿÖÜ¢£¥₧ƒáíóúñÑªº¿⌐¬½¼¡«»░▒▓│┤╡╢╖╕╣║╗╝╜╛┐└┴┬├─┼╞╟╚╔╩╦╠═╬╧╨╤╥╙╘╒╓╫╪┘┌█▄▌▐▀αßΓπΣσµτΦΘΩδ∞φε∩≡±≥≤⌠⌡÷≈°∙·√ⁿ²■ ";
        let high: Vec<char> = HIGH.chars().collect();
        raw.iter()
            .map(|&byte| {
                if byte < 128 {
                    char::from(byte)
                } else {
                    high[usize::from(byte - 128)]
                }
            })
            .collect()
    };
    let provenance = if selected == NameEncoding::Auto {
        NameProvenance::UncertainCp437
    } else {
        NameProvenance::Explicit(selected)
    };
    Ok((name, provenance))
}

pub(crate) fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & (0u32.wrapping_sub(crc & 1)));
        }
    }
    !crc
}
