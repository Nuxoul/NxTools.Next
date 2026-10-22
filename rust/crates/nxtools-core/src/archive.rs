//! Saveable operation presets, not active jobs or archive browser state.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveDestination {
    #[default]
    Ask,
    Smart,
    Here,
    Name,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveConflictPolicy {
    #[default]
    Ask,
    Overwrite,
    Skip,
    AutoRename,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveFilenameEncoding {
    #[default]
    Auto,
    Utf8,
    Gbk,
    Big5,
    ShiftJis,
    Korean,
    Cp437,
    Windows1250,
    Windows1251,
    Windows1252,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveCreateFormat {
    #[default]
    Zip,
    SevenZ,
    Tar,
    Tgz,
    Txz,
    Gz,
    Xz,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveCompressionLevel {
    Store,
    Fast,
    #[default]
    Normal,
    Maximum,
}

/// Original fields remain required in present sections; additive fields name legacy defaults.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ArchiveSettings {
    #[serde(default = "internal_open_default")]
    pub open_in_nxtools: bool,
    pub default_destination: ArchiveDestination,
    pub conflict_policy: ArchiveConflictPolicy,
    pub filename_encoding: ArchiveFilenameEncoding,
    pub create_format: ArchiveCreateFormat,
    /// A format-independent preference; TAR ignores but retains it.
    pub compression_level: ArchiveCompressionLevel,
    pub test_after_create: bool,
    pub strip_single_root: bool,
    pub exclude_patterns: Vec<String>,
    pub copy_motw: bool,
}

impl Default for ArchiveSettings {
    fn default() -> Self {
        Self {
            open_in_nxtools: true,
            default_destination: ArchiveDestination::Ask,
            conflict_policy: ArchiveConflictPolicy::Ask,
            filename_encoding: ArchiveFilenameEncoding::Auto,
            create_format: ArchiveCreateFormat::Zip,
            compression_level: ArchiveCompressionLevel::Normal,
            test_after_create: true,
            strip_single_root: false,
            exclude_patterns: vec!["Thumbs.db".into(), "desktop.ini".into()],
            copy_motw: true,
        }
    }
}
fn internal_open_default() -> bool {
    true
}

impl ArchiveSettings {
    /// Checks finite preset limits and the existing archive engine's glob grammar.
    pub fn validate(&self) -> Result<(), String> {
        if self.exclude_patterns.len() > 32
            || self.exclude_patterns.iter().map(String::len).sum::<usize>() > 8192
        {
            return Err("排除规则不能超过 32 条或合计 8 KiB".into());
        }
        if self
            .exclude_patterns
            .iter()
            .any(|pattern| pattern.chars().count() > 256 || pattern.contains('\0'))
        {
            return Err("每条排除规则不能超过 256 个字符，且不能包含空字符".into());
        }
        for pattern in &self.exclude_patterns {
            if pattern.is_empty()
                || pattern.contains('\\')
                || pattern.starts_with(['/', '!'])
                || pattern.split('/').any(|part| matches!(part, "." | ".."))
                || globset::GlobBuilder::new(pattern)
                    .literal_separator(true)
                    .case_insensitive(true)
                    .build()
                    .is_err()
            {
                return Err("排除规则无效；请使用文件名或以 / 分隔的相对路径 glob".into());
            }
        }
        if matches!(
            self.create_format,
            ArchiveCreateFormat::Gz | ArchiveCreateFormat::Xz
        ) && self.compression_level == ArchiveCompressionLevel::Store
        {
            return Err("GZ 和 XZ 格式不支持仅存储压缩等级".into());
        }
        Ok(())
    }
}
