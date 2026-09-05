//! Pinyin initial matching for Chinese application names, ported from the
//! legacy NxTools file-rename service: a CJK character is encoded to GB2312,
//! its signed two-byte area code falls into one of the 23 level-1 pinyin
//! ranges, and the range's initial letter becomes the matchable character.
//! ASCII passes through; characters outside GB2312 stay unchanged, matching
//! the legacy fallback contract.

use encoding_rs::GBK;

const AREA_CODE_BOUNDS: [i16; 24] = [
    -20319, -20284, -19776, -19218, -18710, -18526, -18239, -17922, -17417, -16474, -16212, -15640,
    -15165, -14922, -14914, -14630, -14149, -14090, -13318, -12838, -12556, -11847, -11055, -10247,
];

const INITIALS: [char; 23] = [
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T',
    'W', 'X', 'Y', 'Z',
];

/// Maps every character of `name` to its pinyin initial (ASCII characters and
/// out-of-table characters pass through), so a query like "jsq" can match a
/// candidate named "计算器".
pub(crate) fn pinyin_initials(name: &str) -> String {
    let mut initials = String::with_capacity(name.len());
    for character in name.chars() {
        if character.is_ascii() {
            // Match queries case-insensitively against the folded initial.
            initials.extend(character.to_lowercase());
            continue;
        }
        initials.extend(pinyin_initial(character).to_lowercase());
    }
    initials
}

fn pinyin_initial(character: char) -> char {
    let mut encoded_input = [0_u8; 4];
    let encoded_input = character.encode_utf8(&mut encoded_input);
    // `encode` returns a Cow that borrows the input; keep the buffer alive in
    // this scope so the borrow outlives the area-code computation.
    let (encoded, _, had_errors) = GBK.encode(encoded_input);
    if had_errors || encoded.len() != 2 {
        return character;
    }
    let area_code = i16::from_be_bytes([encoded[0], encoded[1]]);
    for (index, bounds) in AREA_CODE_BOUNDS.windows(2).enumerate() {
        if area_code >= bounds[0] && area_code < bounds[1] {
            return INITIALS[index];
        }
    }
    character
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cjk_names_map_to_pinyin_initials() {
        assert_eq!(pinyin_initials("计算器"), "jsq");
        assert_eq!(pinyin_initials("资源管理器"), "zyglq");
        assert_eq!(pinyin_initials("任务管理器"), "rwglq");
    }

    #[test]
    fn ascii_and_out_of_table_characters_pass_through() {
        assert_eq!(pinyin_initials("VS Code"), "vs code");
        assert_eq!(pinyin_initials("NxTools"), "nxtools");
        // Characters without a GBK level-1 pinyin range stay unchanged.
        assert_eq!(pinyin_initials("\u{3007}"), "\u{3007}");
    }
}
