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

mod data {
    include!("pinyin_data/syllables.rs");
}

const READINGS: &[u8; 83_968] = include_bytes!("pinyin_data/readings.bin");
const MAX_TERM: usize = 63;

fn readings(character: char) -> impl Iterator<Item = &'static str> {
    let mut packed = if ('\u{4e00}'..='\u{9fff}').contains(&character) {
        let offset = (character as usize - 0x4e00) * 4;
        u32::from_le_bytes([
            READINGS[offset],
            READINGS[offset + 1],
            READINGS[offset + 2],
            READINGS[offset + 3],
        ])
    } else {
        0
    };
    std::iter::from_fn(move || {
        let index = (packed & 1023) as usize;
        packed >>= 10;
        index
            .checked_sub(1)
            .and_then(|index| data::SYLLABLES.get(index).copied())
    })
}

/// Query-owned spellings only. At most eight variants, bounded before concatenation.
pub(crate) fn homophone_spellings(text: &str) -> Vec<String> {
    if text.chars().count() > 32 {
        return Vec::new();
    }
    let mut changed = false;
    let mut spellings = vec![String::new()];
    for character in text.chars() {
        let choices = readings(character).collect::<Vec<_>>();
        let literal = character.to_string();
        let choices = if choices.is_empty() {
            vec![literal.as_str()]
        } else {
            changed = true;
            choices
        };
        let mut next = Vec::new();
        for prefix in &spellings {
            for choice in &choices {
                if prefix.len() + choice.len() <= MAX_TERM {
                    next.push(format!("{prefix}{choice}"));
                }
                if next.len() == 8 {
                    break;
                }
            }
            if next.len() == 8 {
                break;
            }
        }
        spellings = next;
        if spellings.is_empty() {
            break;
        }
    }
    if changed { spellings } else { Vec::new() }
}

/// Immutable query-offset automaton. A Han character consumes a nonempty
/// pronunciation prefix, not a product of whole-name reading combinations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PinyinTerm {
    positions: [u64; 36],
    length: usize,
}

fn letter_index(letter: u8) -> Option<usize> {
    match letter {
        b'a'..=b'z' => Some(usize::from(letter - b'a')),
        b'0'..=b'9' => Some(26 + usize::from(letter - b'0')),
        _ => None,
    }
}

impl PinyinTerm {
    pub(crate) fn new(text: &str) -> Option<Self> {
        if !(2..=MAX_TERM).contains(&text.len())
            || !text.bytes().any(|byte| byte.is_ascii_lowercase())
        {
            return None;
        }
        let mut positions = [0; 36];
        for (offset, letter) in text.bytes().enumerate() {
            positions[letter_index(letter)?] |= 1_u64 << offset;
        }
        Some(Self {
            positions,
            length: text.len(),
        })
    }

    fn advance(&self, character: char, states: u64) -> u64 {
        if character.is_ascii() {
            return letter_index((character as u8).to_ascii_lowercase())
                .map_or(0, |index| (states & self.positions[index]) << 1);
        }
        let mut next = 0;
        for reading in readings(character) {
            let mut reached = states;
            for letter in reading.bytes() {
                let index = usize::from(letter - b'a');
                let positions = self.positions[index]
                    | if letter == b'v' {
                        self.positions[usize::from(b'u' - b'a')]
                    } else {
                        0
                    };
                reached = (reached & positions) << 1;
                next |= reached;
                if reached == 0 {
                    break;
                }
            }
        }
        next
    }

    pub(crate) fn find(&self, text: &str) -> Option<std::ops::Range<usize>> {
        let done = 1_u64 << self.length;
        for (start, first) in text.char_indices() {
            let mut states = self.advance(first, 1);
            if states == 0 {
                continue;
            }
            let mut end = start + first.len_utf8();
            for character in text[end..].chars().take(self.length - 1) {
                if states & done != 0 {
                    break;
                }
                states = self.advance(character, states);
                end += character.len_utf8();
                if states == 0 {
                    break;
                }
            }
            if states & done != 0 {
                return Some(start..end);
            }
        }
        None
    }
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

    #[test]
    fn w1_table_and_term_work_are_bounded_without_reading_products() {
        for code in 0x4e00..=0x9fff {
            let character = char::from_u32(code).expect("Han scalar");
            let values = readings(character).collect::<Vec<_>>();
            assert!(values.len() <= 3);
            assert!(values.iter().all(
                |value| value.len() <= 6 && value.bytes().all(|byte| byte.is_ascii_lowercase())
            ));
        }
        assert_eq!(
            PinyinTerm::new(&"b".repeat(63))
                .expect("maximum term")
                .find(&"报".repeat(63)),
            Some(0..189)
        );
        assert!(PinyinTerm::new(&"b".repeat(64)).is_none());
        for term in ["", "b", "123", "b g", "b-g", "报告", "BAOGAO"] {
            assert!(PinyinTerm::new(term).is_none());
        }
        assert_eq!(
            PinyinTerm::new("baogao").expect("term").find("🙂年度报告"),
            Some(10..16)
        );
    }
}
