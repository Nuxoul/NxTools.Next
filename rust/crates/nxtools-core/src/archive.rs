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
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveDoubleClickAction {
    #[default]
    Open,
    Smart,
    Name,
}
impl ArchiveDoubleClickAction {
    pub fn wire_name(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Smart => "smart",
            Self::Name => "name",
        }
    }
}
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveMenuLayout {
    #[default]
    Cascade,
    Flat,
}
impl ArchiveMenuLayout {
    pub fn wire_name(self) -> &'static str {
        match self {
            Self::Cascade => "cascade",
            Self::Flat => "flat",
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveMenuCommand {
    Open,
    ExtractSmart,
    ExtractHere,
    ExtractName,
    ExtractDialog,
    Test,
    Create,
}
impl ArchiveMenuCommand {
    pub const ALL: [Self; 7] = [
        Self::Open,
        Self::ExtractSmart,
        Self::ExtractHere,
        Self::ExtractName,
        Self::ExtractDialog,
        Self::Test,
        Self::Create,
    ];
    pub fn wire_name(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::ExtractSmart => "extract_smart",
            Self::ExtractHere => "extract_here",
            Self::ExtractName => "extract_name",
            Self::ExtractDialog => "extract_dialog",
            Self::Test => "test",
            Self::Create => "create",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Open => "在 NxTools 中打开",
            Self::ExtractSmart => "智能解压",
            Self::ExtractHere => "解压到当前文件夹",
            Self::ExtractName => "解压到同名文件夹",
            Self::ExtractDialog => "选择解压路径…",
            Self::Test => "测试压缩包",
            Self::Create => "创建压缩包…",
        }
    }
}
fn legacy_menu_commands() -> Vec<ArchiveMenuCommand> {
    vec![
        ArchiveMenuCommand::Open,
        ArchiveMenuCommand::ExtractSmart,
        ArchiveMenuCommand::ExtractHere,
        ArchiveMenuCommand::ExtractName,
        ArchiveMenuCommand::Test,
    ]
}
/// Saved Phase F Explorer integration *desired* intent. It is never an applied state: only the
/// native owner's explicit Apply result reports observed facts, which are stored natively.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ArchiveShellIntegrationSettings {
    pub classic_cascade: bool,
    pub open_with: bool,
    pub extensions: Vec<String>,
    /// Monotonic desired revision; every saved change increments it.
    pub desired_revision: i64,
    #[serde(default)]
    pub double_click_action: ArchiveDoubleClickAction,
    #[serde(default)]
    pub menu_layout: ArchiveMenuLayout,
    #[serde(default = "legacy_menu_commands")]
    pub menu_commands: Vec<ArchiveMenuCommand>,
}

/// Extensions offered for current-user registration; mirrors recognized launch archives.
pub const ARCHIVE_SHELL_EXTENSIONS: [&str; 13] = [
    ".zip", ".7z", ".rar", ".tar", ".tgz", ".txz", ".gz", ".xz", ".bz2", ".tbz", ".tbz2", ".zst",
    ".tzst",
];
impl Default for ArchiveShellIntegrationSettings {
    fn default() -> Self {
        Self {
            classic_cascade: false,
            open_with: false,
            extensions: Vec::new(),
            desired_revision: 0,
            double_click_action: Default::default(),
            menu_layout: Default::default(),
            menu_commands: legacy_menu_commands(),
        }
    }
}

impl ArchiveShellIntegrationSettings {
    /// Draft validation; an enabled draft without extensions is rejected only at Apply.
    pub fn validate(&self) -> Result<(), String> {
        if self.desired_revision < 0 {
            return Err("集成设置版本无效".into());
        }
        if self.menu_commands.len() > ArchiveMenuCommand::ALL.len()
            || self
                .menu_commands
                .iter()
                .enumerate()
                .any(|(index, command)| self.menu_commands[..index].contains(command))
        {
            return Err("菜单命令不能重复或超过支持的数量".into());
        }
        let mut seen = std::collections::BTreeSet::new();
        for extension in &self.extensions {
            if !ARCHIVE_SHELL_EXTENSIONS.contains(&extension.as_str()) || !seen.insert(extension) {
                return Err("集成扩展名只能从受支持的压缩格式中选择且不能重复".into());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ArchiveTempSettings {
    pub base_directory: String,
    pub retention_hours: u32,
    pub cleanup_on_child_exit: bool,
}
impl Default for ArchiveTempSettings {
    fn default() -> Self {
        Self {
            base_directory: String::new(),
            retention_hours: 24,
            cleanup_on_child_exit: false,
        }
    }
}
impl ArchiveTempSettings {
    pub fn validate(&self) -> Result<(), String> {
        if !matches!(self.retention_hours, 1 | 6 | 24 | 72 | 168) {
            return Err("临时保留期只能选择 1、6、24、72 或 168 小时".into());
        }
        let path = &self.base_directory;
        if path.is_empty() {
            return Ok(());
        }
        let bytes = path.as_bytes();
        if bytes.len() > 4096
            || bytes.len() < 3
            || !bytes[0].is_ascii_alphabetic()
            || &bytes[1..3] != b":\\"
            || path.chars().any(char::is_control)
            || path[3..].contains(['/', ':', '*', '?', '"', '<', '>', '|'])
            || (path.len() > 3 && path[3..].trim_end_matches('\\').split('\\').count() > 59)
            || path.ends_with("\\\\")
            || (path.len() > 3
                && path[3..].trim_end_matches('\\').split('\\').any(|part| {
                    let stem = part
                        .split('.')
                        .next()
                        .unwrap_or_default()
                        .trim_end_matches(' ')
                        .to_ascii_uppercase();
                    part.is_empty()
                        || part.encode_utf16().count() > 255
                        || matches!(part, "." | "..")
                        || part.ends_with([' ', '.'])
                        || matches!(
                            stem.as_str(),
                            "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
                        )
                        || stem
                            .strip_prefix("COM")
                            .or_else(|| stem.strip_prefix("LPT"))
                            .is_some_and(|suffix| {
                                matches!(
                                    suffix,
                                    "1" | "2"
                                        | "3"
                                        | "4"
                                        | "5"
                                        | "6"
                                        | "7"
                                        | "8"
                                        | "9"
                                        | "¹"
                                        | "²"
                                        | "³"
                                )
                            })
                }))
        {
            return Err("请选择本地盘的绝对临时目录；不支持网络、设备路径或相对路径".into());
        }
        Ok(())
    }
}

/// Original fields remain required in present sections; additive fields name legacy defaults.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ArchiveSettings {
    #[serde(default = "internal_open_default")]
    pub open_in_nxtools: bool,
    #[serde(default = "internal_open_default")]
    pub remember_recent: bool,
    #[serde(default = "recent_limit_default")]
    pub recent_limit: u8,
    #[serde(default)]
    pub temp: ArchiveTempSettings,
    #[serde(default = "internal_open_default")]
    pub auto_enter_single_root: bool,
    #[serde(default = "internal_open_default")]
    pub show_parent_row: bool,
    #[serde(default)]
    pub show_grid_lines: bool,
    #[serde(default = "internal_open_default")]
    pub full_row_select: bool,
    #[serde(default)]
    pub single_click_open: bool,
    #[serde(default = "internal_open_default")]
    pub show_tree: bool,
    #[serde(default)]
    pub show_full_path: bool,
    #[serde(default = "internal_open_default")]
    pub auto_expand_tree: bool,
    #[serde(default)]
    pub escape_closes_window: bool,
    #[serde(default = "internal_open_default")]
    pub open_after_extract: bool,
    #[serde(default)]
    pub keep_extract_progress_open: bool,
    #[serde(default)]
    pub smart_single_file_folder: bool,
    #[serde(default)]
    pub keep_create_progress_open: bool,
    /// Explicit opt-in from the extract form. Never inferred from a source/session path.
    #[serde(default)]
    pub extract_custom_destination: Option<String>,
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
    /// Phase F desired intent, default off; absent in legacy sections.
    #[serde(default)]
    pub shell_integration: ArchiveShellIntegrationSettings,
}

impl Default for ArchiveSettings {
    fn default() -> Self {
        Self {
            open_in_nxtools: true,
            remember_recent: true,
            recent_limit: recent_limit_default(),
            temp: ArchiveTempSettings::default(),
            auto_enter_single_root: true,
            show_parent_row: true,
            show_grid_lines: false,
            full_row_select: true,
            single_click_open: false,
            show_tree: true,
            show_full_path: false,
            auto_expand_tree: true,
            escape_closes_window: false,
            open_after_extract: true,
            keep_extract_progress_open: false,
            smart_single_file_folder: false,
            keep_create_progress_open: false,
            extract_custom_destination: None,
            default_destination: ArchiveDestination::Ask,
            conflict_policy: ArchiveConflictPolicy::Ask,
            filename_encoding: ArchiveFilenameEncoding::Auto,
            create_format: ArchiveCreateFormat::Zip,
            compression_level: ArchiveCompressionLevel::Normal,
            test_after_create: true,
            strip_single_root: false,
            exclude_patterns: vec!["Thumbs.db".into(), "desktop.ini".into()],
            copy_motw: true,
            shell_integration: ArchiveShellIntegrationSettings::default(),
        }
    }
}
fn internal_open_default() -> bool {
    true
}
fn recent_limit_default() -> u8 {
    10
}

impl ArchiveSettings {
    /// Called inside the original AppConfig section lock after rereading disk. A stale dialog
    /// only changes fields the user actually changed; other fields keep their latest disk values.
    pub fn merge_extract_preferences(&mut self, baseline: &Self, desired: &Self) {
        macro_rules! changed {
            ($($field:ident),+ $(,)?) => { $(
                if desired.$field != baseline.$field { self.$field.clone_from(&desired.$field); }
            )+ };
        }
        changed!(
            default_destination,
            conflict_policy,
            copy_motw,
            open_after_extract,
            keep_extract_progress_open,
            smart_single_file_folder,
            extract_custom_destination
        );
    }
    /// Checks finite preset limits and the existing archive engine's glob grammar.
    pub fn validate(&self) -> Result<(), String> {
        if let Some(path) = &self.extract_custom_destination {
            // Same bounded local absolute-path grammar as the native temp picker; no I/O here.
            if path.is_empty() {
                return Err("记住的解压目标不能为空".into());
            }
            ArchiveTempSettings {
                base_directory: path.clone(),
                ..Default::default()
            }
            .validate()?;
        }
        self.temp.validate()?;
        if self.recent_limit > 30 {
            return Err("最近压缩包数量必须在 0–30 之间".into());
        }
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
        self.shell_integration.validate()?;
        Ok(())
    }
}
