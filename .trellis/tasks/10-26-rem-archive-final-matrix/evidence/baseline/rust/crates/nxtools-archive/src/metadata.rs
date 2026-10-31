//! Immutable archive-supplied values. Native application stays behind ArchiveOutput.
use crate::{ArchiveError, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ModifiedTime {
    DosLocal {
        date: u16,
        time: u16,
    },
    FiletimeUtc {
        ticks: u64,
    },
    UnixUtc {
        seconds: i64,
        nanoseconds: u32,
        precision_ns: u32,
    },
    Invalid,
}
impl ModifiedTime {
    pub fn validate(self) -> Result<()> {
        let valid = match self {
            Self::DosLocal { date, time } => {
                let year = 1980 + (date >> 9);
                let month = (date >> 5) & 15;
                let day = date & 31;
                let days = match month {
                    1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
                    4 | 6 | 9 | 11 => 30,
                    2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
                    2 => 28,
                    _ => 0,
                };
                day > 0 && day <= days && time >> 11 < 24 && (time >> 5) & 63 < 60 && time & 31 < 30
            }
            Self::FiletimeUtc { ticks } => ticks <= 2_650_467_743_999_999_999,
            Self::UnixUtc {
                seconds,
                nanoseconds,
                precision_ns,
            } => {
                (-11_644_473_600..=253_402_300_799).contains(&seconds)
                    && nanoseconds < 1_000_000_000
                    && matches!(
                        precision_ns,
                        1 | 10
                            | 100
                            | 1_000
                            | 10_000
                            | 100_000
                            | 1_000_000
                            | 10_000_000
                            | 100_000_000
                            | 1_000_000_000
                    )
            }
            Self::Invalid => false,
        };
        if valid {
            Ok(())
        } else {
            Err(ArchiveError::InvalidMetadata)
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindowsAttributes {
    pub read_only: bool,
    pub hidden: bool,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputMetadata {
    pub modified: Option<ModifiedTime>,
    pub attributes: Option<WindowsAttributes>,
}
impl OutputMetadata {
    pub fn validate(self) -> Result<()> {
        if let Some(time) = self.modified {
            time.validate()?;
        }
        Ok(())
    }
}

/// PAX decimal seconds, no floating point or exponent. At most64 input bytes retained nowhere.
pub(crate) fn unix_decimal(bytes: &[u8]) -> ModifiedTime {
    let parsed = (|| {
        if bytes.is_empty() || bytes.len() > 64 {
            return None;
        }
        let negative = bytes[0] == b'-';
        let unsigned = if negative { &bytes[1..] } else { bytes };
        let mut pieces = unsigned.split(|byte| *byte == b'.');
        let whole = pieces.next()?;
        if whole.is_empty() || !whole.iter().all(u8::is_ascii_digit) {
            return None;
        }
        let seconds = whole.iter().try_fold(0i64, |value, byte| {
            value.checked_mul(10)?.checked_add(i64::from(byte - b'0'))
        })?;
        let fraction = pieces.next();
        if pieces.next().is_some() {
            return None;
        }
        let mut nanos = 0u32;
        let precision_ns = if let Some(fraction) = fraction {
            if fraction.is_empty() || !fraction.iter().all(u8::is_ascii_digit) {
                return None;
            }
            for digit in fraction.iter().take(9) {
                nanos = nanos * 10 + u32::from(digit - b'0');
            }
            nanos *= 10u32.pow(9 - fraction.len().min(9) as u32);
            // For a negative value, discarded nonzero sub-ns digits must round the absolute
            // fraction up, so native floor-to100ns never moves the instant toward zero.
            if negative && fraction.iter().skip(9).any(|digit| *digit != b'0') {
                nanos += 1;
            }
            10u32.pow(9 - fraction.len().min(9) as u32)
        } else {
            1_000_000_000
        };
        let (seconds, nanoseconds) = if negative && nanos != 0 {
            (
                seconds.checked_neg()?.checked_sub(1)?,
                1_000_000_000 - nanos,
            )
        } else {
            (
                if negative {
                    seconds.checked_neg()?
                } else {
                    seconds
                },
                nanos,
            )
        };
        Some(ModifiedTime::UnixUtc {
            seconds,
            nanoseconds,
            precision_ns,
        })
    })();
    parsed.unwrap_or(ModifiedTime::Invalid)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_pax_decimal_keeps_earlier_instant_with_bounded_precision() {
        assert_eq!(
            unix_decimal(b"-0.0000000001"),
            ModifiedTime::UnixUtc {
                seconds: -1,
                nanoseconds: 999_999_999,
                precision_ns: 1
            }
        );
        assert_eq!(
            unix_decimal(b"-1.1234567899"),
            ModifiedTime::UnixUtc {
                seconds: -2,
                nanoseconds: 876_543_210,
                precision_ns: 1
            }
        );
        assert_eq!(
            unix_decimal(b"-0.9999999999"),
            ModifiedTime::UnixUtc {
                seconds: -1,
                nanoseconds: 0,
                precision_ns: 1
            }
        );
        assert_eq!(
            unix_decimal(b"0"),
            ModifiedTime::UnixUtc {
                seconds: 0,
                nanoseconds: 0,
                precision_ns: 1_000_000_000
            }
        );
        for bytes in [
            b"".as_slice(),
            b"1e2",
            b"--1",
            b"1.",
            b"+1",
            b"99999999999999999999999999999999999999999999999999999999999999999",
        ] {
            assert_eq!(unix_decimal(bytes), ModifiedTime::Invalid);
        }
    }
}
