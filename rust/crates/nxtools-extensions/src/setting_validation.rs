//! Bounded host-owned text validation. No filesystem, network or script execution.
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextValidation {
    #[serde(default)]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_length: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
}
impl TextValidation {
    fn compiled_pattern(&self) -> Result<Option<Regex>, ()> {
        self.pattern
            .as_ref()
            .map(|pattern| {
                if pattern.len() > 512 {
                    return Err(());
                }
                RegexBuilder::new(&format!(r"\A(?:{pattern})\z"))
                    .size_limit(1024 * 1024)
                    .dfa_size_limit(1024 * 1024)
                    .build()
                    .map_err(|_| ())
            })
            .transpose()
    }
    fn valid_lengths(&self) -> bool {
        self.min_length.unwrap_or(0) <= self.max_length.unwrap_or(32_768)
            && self.max_length.unwrap_or(0) <= 32_768
            && self.min_length.unwrap_or(0) <= 32_768
    }
    pub(crate) fn valid(&self) -> bool {
        self.valid_lengths() && self.compiled_pattern().is_ok()
    }
    pub(crate) fn accepts(&self, value: &str) -> bool {
        if !self.valid_lengths() || (self.required && value.trim().is_empty()) {
            return false;
        }
        let count = value.chars().count();
        count >= self.min_length.unwrap_or(0) as usize
            && count <= self.max_length.unwrap_or(32_768) as usize
            && self
                .compiled_pattern()
                .is_ok_and(|pattern| pattern.is_none_or(|pattern| pattern.is_match(value)))
    }
}

/// Untagged serialization deliberately retains old string arrays byte-for-byte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChoiceOption {
    Legacy(String),
    Labeled(ChoiceLabel),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChoiceLabel {
    pub value: String,
    pub label: String,
}
impl ChoiceOption {
    pub fn value(&self) -> &str {
        match self {
            Self::Legacy(value) => value,
            Self::Labeled(option) => &option.value,
        }
    }
    pub fn label(&self) -> &str {
        match self {
            Self::Legacy(value) => value,
            Self::Labeled(option) => &option.label,
        }
    }
    pub(crate) fn labeled(&self) -> bool {
        matches!(self, Self::Labeled(_))
    }
}
