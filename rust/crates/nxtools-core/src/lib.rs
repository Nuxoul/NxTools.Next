use std::fs;
use std::path::Path;
use std::time::SystemTime;
#[cfg(test)]
use std::{path::PathBuf, time::Duration};

use anyhow::{Context, Result};
use fs2::FileExt;
use serde::{Deserialize, Serialize};

pub mod archive;
#[cfg(test)]
mod archive_tests;
#[cfg(test)]
mod bundled_extension_tests;
mod bundled_extensions;
mod extensions;
pub use extensions::ConfigExtensionStorage;
pub mod floating_assistant;
#[cfg(test)]
mod floating_assistant_tests;
mod instance;
mod logging;
mod paths;
pub use archive::{
    ArchiveCompressionLevel, ArchiveConflictPolicy, ArchiveCreateFormat, ArchiveDestination,
    ArchiveFilenameEncoding, ArchiveSettings, ArchiveShellIntegrationSettings,
};
pub use floating_assistant::{
    ClickAction, FloatingAssistantPosition, FloatingAssistantSettings, ImageDropAction,
    TextDropAction,
};
pub use instance::SingleInstanceGuard;
#[cfg(test)]
use logging::delete_expired_logs;
pub use logging::{LogGuard, init_file_logging};
pub use paths::AppPaths;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessBlacklistFeature {
    Clipboard,
    QuickLaunch,
}

/// One application, with independently selectable feature scopes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessBlacklistRule {
    pub process: String,
    #[serde(default)]
    pub disable_clipboard: bool,
    #[serde(default)]
    pub disable_quick_launch: bool,
}

impl ProcessBlacklistRule {
    pub fn new(
        process: &str,
        disable_clipboard: bool,
        disable_quick_launch: bool,
    ) -> std::result::Result<Self, String> {
        let process = normalize_process_name(process);
        if process.is_empty()
            || process
                .chars()
                .any(|c| c.is_control() || "<>:\"|?*;,".contains(c))
        {
            return Err("请输入有效的应用进程名，例如 notepad.exe".into());
        }
        Ok(Self {
            process,
            disable_clipboard,
            disable_quick_launch,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessBlacklistSettings {
    /// None is legacy; Some(empty) is authoritative and must never remigrate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entries: Option<Vec<ProcessBlacklistRule>>,
    #[serde(default)]
    pub processes: Vec<String>,
    #[serde(default = "default_true")]
    pub disable_clipboard: bool,
    #[serde(default)]
    pub disable_quick_launch: bool,
}

impl Default for ProcessBlacklistSettings {
    fn default() -> Self {
        Self {
            entries: Some(
                default_clipboard_excluded_processes()
                    .into_iter()
                    .map(|process| ProcessBlacklistRule {
                        process: normalize_process_name(&process),
                        disable_clipboard: true,
                        disable_quick_launch: false,
                    })
                    .collect(),
            ),
            processes: default_clipboard_excluded_processes(),
            disable_clipboard: true,
            disable_quick_launch: false,
        }
    }
}

fn legacy_process_blacklist() -> ProcessBlacklistSettings {
    ProcessBlacklistSettings {
        entries: None,
        processes: Vec::new(),
        disable_clipboard: true,
        disable_quick_launch: false,
    }
}

fn normalize_process_name(value: &str) -> String {
    // Lowercase before stripping the extension: Windows file names are case
    // insensitive, so `Notepad.EXE` must match a `notepad` entry.
    let value = value.trim().replace('/', "\\").to_ascii_lowercase();
    let value = value.rsplit('\\').next().unwrap_or(&value);
    value.strip_suffix(".exe").unwrap_or(value).to_owned()
}

impl ProcessBlacklistSettings {
    pub fn from_entries(entries: Vec<ProcessBlacklistRule>) -> Self {
        let mut settings = Self {
            entries: Some(entries),
            processes: Vec::new(),
            disable_clipboard: false,
            disable_quick_launch: false,
        };
        settings.normalize();
        settings
    }

    pub fn entries(&self) -> Vec<ProcessBlacklistRule> {
        self.entries.clone().unwrap_or_else(|| {
            self.processes
                .iter()
                .filter_map(|process| {
                    ProcessBlacklistRule::new(
                        process,
                        self.disable_clipboard,
                        self.disable_quick_launch,
                    )
                    .ok()
                })
                .collect()
        })
    }

    pub fn normalize(&mut self) {
        let mut merged: Vec<ProcessBlacklistRule> = Vec::new();
        for entry in self.entries() {
            let Ok(entry) = ProcessBlacklistRule::new(
                &entry.process,
                entry.disable_clipboard,
                entry.disable_quick_launch,
            ) else {
                continue;
            };
            if let Some(existing) = merged
                .iter_mut()
                .find(|existing| existing.process == entry.process)
            {
                existing.disable_clipboard |= entry.disable_clipboard;
                existing.disable_quick_launch |= entry.disable_quick_launch;
            } else {
                merged.push(entry);
            }
        }
        self.processes = merged.iter().map(|entry| entry.process.clone()).collect();
        self.entries = Some(merged);
    }

    pub fn clipboard_processes(&self) -> Vec<String> {
        self.entries()
            .into_iter()
            .filter(|entry| entry.disable_clipboard)
            .map(|entry| entry.process)
            .collect()
    }

    pub fn contains(&self, process_name: &str, feature: ProcessBlacklistFeature) -> bool {
        let process_name = normalize_process_name(process_name);
        let matches = |entry: &ProcessBlacklistRule| {
            normalize_process_name(&entry.process) == process_name
                && match feature {
                    ProcessBlacklistFeature::Clipboard => entry.disable_clipboard,
                    ProcessBlacklistFeature::QuickLaunch => entry.disable_quick_launch,
                }
        };
        match &self.entries {
            Some(entries) => entries.iter().any(matches),
            None => self.entries().iter().any(matches),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    pub theme: ThemePreference,
    pub platform_pipe_name: String,
    #[serde(default)]
    pub network: NetworkSettings,
    #[serde(default)]
    pub system: SystemSettings,
    #[serde(default)]
    pub clipboard_panel: ClipboardPanelSize,
    #[serde(default)]
    pub clipboard: ClipboardSettings,
    #[serde(default)]
    pub quick_launch: QuickLaunchSettings,
    #[serde(default)]
    pub floating_assistant: FloatingAssistantSettings,
    #[serde(default = "legacy_process_blacklist")]
    pub process_blacklist: ProcessBlacklistSettings,
    #[serde(default)]
    pub download: DownloadSettings,
    #[serde(default)]
    pub archive: ArchiveSettings,
    #[serde(default)]
    pub extensions: nxtools_extensions::PackageRegistry,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: ThemePreference::Dark,
            platform_pipe_name: "nxtools-next-platform-v1".into(),
            network: NetworkSettings::default(),
            system: SystemSettings::default(),
            clipboard_panel: ClipboardPanelSize::default(),
            clipboard: ClipboardSettings::default(),
            quick_launch: QuickLaunchSettings::default(),
            floating_assistant: FloatingAssistantSettings::default(),
            process_blacklist: ProcessBlacklistSettings::default(),
            download: DownloadSettings::default(),
            archive: ArchiveSettings::default(),
            extensions: Default::default(),
        }
    }
}
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreference {
    System,
    Light,
    #[default]
    Dark,
    CatppuccinLatte,
    CatppuccinFrappe,
    CatppuccinMacchiato,
    CatppuccinMocha,
    GruvboxLight,
    GruvboxDark,
    SolarizedLight,
    SolarizedDark,
}

impl ThemePreference {
    pub const ALL: [Self; 11] = [
        Self::System,
        Self::Light,
        Self::Dark,
        Self::CatppuccinLatte,
        Self::CatppuccinFrappe,
        Self::CatppuccinMacchiato,
        Self::CatppuccinMocha,
        Self::GruvboxLight,
        Self::GruvboxDark,
        Self::SolarizedLight,
        Self::SolarizedDark,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::System => "跟随系统",
            Self::Light => "默认浅色",
            Self::Dark => "默认深色",
            Self::CatppuccinLatte => "Catppuccin Latte",
            Self::CatppuccinFrappe => "Catppuccin Frappe",
            Self::CatppuccinMacchiato => "Catppuccin Macchiato",
            Self::CatppuccinMocha => "Catppuccin Mocha",
            Self::GruvboxLight => "Gruvbox Light",
            Self::GruvboxDark => "Gruvbox Dark",
            Self::SolarizedLight => "Solarized Light",
            Self::SolarizedDark => "Solarized Dark",
        }
    }

    pub const fn is_dark(self, system_dark: bool) -> bool {
        match self {
            Self::System => system_dark,
            Self::Light | Self::CatppuccinLatte | Self::GruvboxLight | Self::SolarizedLight => {
                false
            }
            _ => true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackgroundDownloadLimitSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_background_download_rate")]
    pub bytes_per_second: u64,
}
const fn default_background_download_rate() -> u64 {
    1024 * 1024
}
impl Default for BackgroundDownloadLimitSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            bytes_per_second: default_background_download_rate(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum NotificationPosition {
    TopLeft,
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
    #[default]
    BottomRight,
}
impl NotificationPosition {
    pub const ALL: [Self; 6] = [
        Self::TopLeft,
        Self::TopCenter,
        Self::TopRight,
        Self::BottomLeft,
        Self::BottomCenter,
        Self::BottomRight,
    ];
    pub const fn label(self) -> &'static str {
        match self {
            Self::TopLeft => "左上",
            Self::TopCenter => "上中",
            Self::TopRight => "右上",
            Self::BottomLeft => "左下",
            Self::BottomCenter => "下中",
            Self::BottomRight => "右下",
        }
    }
    pub const fn is_top(self) -> bool {
        matches!(self, Self::TopLeft | Self::TopCenter | Self::TopRight)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemSettings {
    #[serde(default)]
    pub start_on_login_silently: bool,
    #[serde(default = "default_close_to_tray")]
    pub close_to_tray: bool,
    #[serde(default)]
    pub theme: ThemePreference,
    #[serde(default)]
    pub notification_position: NotificationPosition,
    #[serde(
        default = "default_notification_duration",
        deserialize_with = "deserialize_notification_duration"
    )]
    pub notification_duration_seconds: u32,
}

impl Default for SystemSettings {
    fn default() -> Self {
        Self {
            start_on_login_silently: false,
            close_to_tray: default_close_to_tray(),
            theme: ThemePreference::default(),
            notification_position: NotificationPosition::default(),
            notification_duration_seconds: default_notification_duration(),
        }
    }
}

pub const fn default_notification_duration() -> u32 {
    6
}
fn deserialize_notification_duration<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<u32, D::Error> {
    u32::deserialize(deserializer).map(|seconds| seconds.clamp(1, 60))
}

const fn default_close_to_tray() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct NetworkSettings {
    #[serde(default)]
    pub proxy: NetworkProxySettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NetworkProxySettings {
    #[serde(default)]
    pub mode: NetworkProxyMode,
    #[serde(default)]
    pub host: String,
    #[serde(default = "default_network_proxy_port")]
    pub port: u16,
}

impl Default for NetworkProxySettings {
    fn default() -> Self {
        Self {
            mode: NetworkProxyMode::Direct,
            host: String::new(),
            port: default_network_proxy_port(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum NetworkProxyMode {
    #[default]
    Direct,
    System,
    ManualHttp,
}

const fn default_network_proxy_port() -> u16 {
    8080
}

impl NetworkSettings {
    pub fn normalize(&mut self) {
        self.proxy.host = self.proxy.host.trim().to_owned();
    }

    pub fn validate(&self) -> std::result::Result<(), String> {
        if self.proxy.mode != NetworkProxyMode::ManualHttp {
            return Ok(());
        }
        let host = self.proxy.host.as_str();
        if host.is_empty() {
            return Err("手动代理缺少服务器地址".into());
        }
        if !valid_proxy_host(host) {
            return Err("手动代理服务器地址无效".into());
        }
        if self.proxy.port == 0 {
            return Err("手动代理端口必须在 1–65535 之间".into());
        }
        Ok(())
    }
}

fn valid_proxy_host(host: &str) -> bool {
    if host.parse::<std::net::IpAddr>().is_ok() {
        return true;
    }
    !host.is_empty()
        && host.len() <= 253
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DownloadSettings {
    #[serde(default)]
    pub default_directory: String,
    /// Optional exact Chrome Web Store ID; blank keeps local-install compatibility.
    #[serde(default)]
    pub chrome_web_store_extension_id: String,
    #[serde(default)]
    pub automatic_categories_enabled: bool,
    #[serde(default = "default_download_categories")]
    pub categories: Vec<DownloadCategorySettings>,
    #[serde(default)]
    pub duplicate_strategy: DownloadDuplicateStrategySettings,
    #[serde(default)]
    pub task_list_sort: Option<String>,
    #[serde(default)]
    pub task_list_sort_descending: Option<bool>,
    #[serde(default)]
    pub double_click_action: DownloadRowDoubleClickActionSettings,
    #[serde(default)]
    pub ffmpeg_executable_path: String,
    #[serde(default)]
    pub default_request_headers: String,
    #[serde(default)]
    pub default_cookie: String,
    #[serde(default)]
    pub accept_invalid_certificates: bool,
    #[serde(default = "default_download_connection_limit")]
    pub default_connection_limit: u16,
    #[serde(default = "default_true")]
    pub adaptive_segment_scaling_enabled: bool,
    #[serde(default = "default_download_adaptive_max_segments")]
    pub adaptive_segment_max_segments: u16,
    #[serde(default = "default_download_adaptive_min_segment_mib")]
    pub adaptive_segment_min_mib: u16,
    #[serde(default = "default_true")]
    pub show_progress_window: bool,
    #[serde(default = "default_true")]
    pub show_completion_window: bool,
    #[serde(default = "default_true")]
    pub completion_sound_enabled: bool,
    #[serde(default)]
    pub open_directory_on_completion: bool,
    #[serde(default = "default_true")]
    pub task_removal_deletes_file: bool,
    #[serde(default)]
    pub taskbar_progress_overlay: DownloadTaskbarProgressOverlaySettings,
    #[serde(default)]
    pub background_speed_limit: BackgroundDownloadLimitSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DownloadCategorySettings {
    pub id: String,
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub save_directory: String,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub url_patterns: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DownloadDuplicateStrategySettings {
    #[default]
    AddNumbered,
    Reject,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DownloadRowDoubleClickActionSettings {
    /// Opens the finished artifact with the associated Windows program.
    #[default]
    OpenFile,
    /// Reveals the finished artifact in Explorer.
    OpenDirectory,
    /// Opens the task window with progress, segments and diagnostics.
    OpenTaskWindow,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DownloadTaskbarProgressOverlaySettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub anchor: DownloadTaskbarOverlayAnchor,
    #[serde(default = "default_taskbar_overlay_offset_x")]
    pub offset_x: i32,
    #[serde(default)]
    pub offset_y: i32,
    #[serde(default = "default_taskbar_overlay_width")]
    pub width: u32,
    #[serde(default = "default_taskbar_overlay_height")]
    pub height: u32,
    #[serde(default)]
    pub progress_mode: DownloadTaskbarProgressMode,
    #[serde(default = "default_true")]
    pub show_progress: bool,
    #[serde(default)]
    pub show_speed: bool,
    #[serde(default)]
    pub show_remaining_time: bool,
    #[serde(default)]
    pub show_active_tasks: bool,
    #[serde(default)]
    pub show_active_connections: bool,
    #[serde(default)]
    pub show_transferred_bytes: bool,
}

impl Default for DownloadTaskbarProgressOverlaySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            anchor: DownloadTaskbarOverlayAnchor::Right,
            offset_x: default_taskbar_overlay_offset_x(),
            offset_y: 0,
            width: default_taskbar_overlay_width(),
            height: default_taskbar_overlay_height(),
            progress_mode: DownloadTaskbarProgressMode::PerTask,
            show_progress: true,
            show_speed: false,
            show_remaining_time: false,
            show_active_tasks: false,
            show_active_connections: false,
            show_transferred_bytes: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DownloadTaskbarOverlayAnchor {
    Left,
    #[default]
    Right,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DownloadTaskbarProgressMode {
    Aggregate,
    #[default]
    PerTask,
    PerSegment,
}

impl Default for DownloadSettings {
    fn default() -> Self {
        Self {
            default_directory: String::new(),
            chrome_web_store_extension_id: String::new(),
            automatic_categories_enabled: false,
            categories: default_download_categories(),
            duplicate_strategy: DownloadDuplicateStrategySettings::AddNumbered,
            double_click_action: DownloadRowDoubleClickActionSettings::OpenFile,
            task_list_sort: None,
            task_list_sort_descending: None,
            ffmpeg_executable_path: String::new(),
            default_request_headers: String::new(),
            default_cookie: String::new(),
            accept_invalid_certificates: false,
            default_connection_limit: default_download_connection_limit(),
            adaptive_segment_scaling_enabled: true,
            adaptive_segment_max_segments: default_download_adaptive_max_segments(),
            adaptive_segment_min_mib: default_download_adaptive_min_segment_mib(),
            show_progress_window: true,
            show_completion_window: true,
            completion_sound_enabled: true,
            open_directory_on_completion: false,
            task_removal_deletes_file: true,
            taskbar_progress_overlay: DownloadTaskbarProgressOverlaySettings::default(),
            background_speed_limit: BackgroundDownloadLimitSettings::default(),
        }
    }
}

const fn default_download_connection_limit() -> u16 {
    8
}

const fn default_download_adaptive_max_segments() -> u16 {
    32
}

const fn default_download_adaptive_min_segment_mib() -> u16 {
    3
}

const fn default_taskbar_overlay_offset_x() -> i32 {
    12
}

const fn default_taskbar_overlay_width() -> u32 {
    132
}

const fn default_taskbar_overlay_height() -> u32 {
    8
}

impl DownloadSettings {
    pub fn normalize(&mut self) {
        self.default_directory = self.default_directory.trim().to_owned();
        self.chrome_web_store_extension_id = self.chrome_web_store_extension_id.trim().to_owned();
        if !is_valid_chrome_web_store_extension_id(&self.chrome_web_store_extension_id) {
            self.chrome_web_store_extension_id.clear();
        }
        self.background_speed_limit.bytes_per_second =
            self.background_speed_limit.bytes_per_second.max(1);
        for category in &mut self.categories {
            category.id = category.id.trim().to_owned();
            category.name = category.name.trim().to_owned();
            category.save_directory = category.save_directory.trim().to_owned();
            category.extensions = normalize_download_category_values(&category.extensions, true);
            category.url_patterns =
                normalize_download_category_values(&category.url_patterns, false);
        }
        self.ffmpeg_executable_path = self.ffmpeg_executable_path.trim().to_owned();
        self.default_request_headers = self.default_request_headers.trim().to_owned();
        let cookie = self.default_cookie.trim();
        self.default_cookie = if cookie
            .get(.."Cookie:".len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("Cookie:"))
        {
            cookie["Cookie:".len()..].trim().to_owned()
        } else {
            cookie.to_owned()
        };
        self.default_connection_limit = self.default_connection_limit.clamp(1, 64);
        self.adaptive_segment_max_segments = self
            .adaptive_segment_max_segments
            .clamp(self.default_connection_limit, 256);
        self.adaptive_segment_min_mib = self.adaptive_segment_min_mib.clamp(1, 100);
        self.taskbar_progress_overlay.offset_x =
            self.taskbar_progress_overlay.offset_x.clamp(-4096, 4096);
        self.taskbar_progress_overlay.offset_y =
            self.taskbar_progress_overlay.offset_y.clamp(-4096, 4096);
        self.taskbar_progress_overlay.width = self.taskbar_progress_overlay.width.clamp(72, 640);
        self.taskbar_progress_overlay.height = self.taskbar_progress_overlay.height.clamp(6, 72);
        if !self.taskbar_progress_overlay.show_progress
            && !self.taskbar_progress_overlay.show_speed
            && !self.taskbar_progress_overlay.show_remaining_time
            && !self.taskbar_progress_overlay.show_active_tasks
            && !self.taskbar_progress_overlay.show_active_connections
            && !self.taskbar_progress_overlay.show_transferred_bytes
        {
            self.taskbar_progress_overlay.show_progress = true;
        }
        if self.taskbar_progress_overlay.show_speed
            || self.taskbar_progress_overlay.show_remaining_time
            || self.taskbar_progress_overlay.show_active_tasks
            || self.taskbar_progress_overlay.show_active_connections
            || self.taskbar_progress_overlay.show_transferred_bytes
        {
            self.taskbar_progress_overlay.height = self.taskbar_progress_overlay.height.max(20);
        }
    }
}

/// Empty before first store upload, otherwise one exact Chromium extension identity.
pub fn is_valid_chrome_web_store_extension_id(value: &str) -> bool {
    value.is_empty()
        || (value.len() == 32 && value.bytes().all(|byte| (b'a'..=b'p').contains(&byte)))
}

fn normalize_download_category_values(values: &[String], trim_dot: bool) -> Vec<String> {
    let mut normalized = Vec::new();
    for value in values {
        let value = value.trim();
        let value = if trim_dot {
            value.trim_start_matches('.')
        } else {
            value
        };
        if value.is_empty()
            || normalized
                .iter()
                .any(|existing: &String| existing.eq_ignore_ascii_case(value))
        {
            continue;
        }
        normalized.push(if trim_dot {
            value.to_ascii_lowercase()
        } else {
            value.to_owned()
        });
    }
    normalized
}

fn default_download_categories() -> Vec<DownloadCategorySettings> {
    fn category(
        id: &str,
        name: &str,
        directory: &str,
        extensions: &[&str],
    ) -> DownloadCategorySettings {
        DownloadCategorySettings {
            id: id.into(),
            name: name.into(),
            enabled: true,
            save_directory: directory.into(),
            extensions: extensions.iter().map(|value| (*value).into()).collect(),
            url_patterns: Vec::new(),
        }
    }
    vec![
        category(
            "compressed",
            "压缩包",
            "Compressed",
            &[
                "zip", "rar", "7z", "tar", "gz", "bz2", "xz", "iso", "dmg", "tgz",
            ],
        ),
        category(
            "programs",
            "程序",
            "Programs",
            &[
                "apk", "exe", "msi", "bat", "cmd", "sh", "jar", "app", "deb", "rpm", "bin",
            ],
        ),
        category(
            "videos",
            "视频",
            "Videos",
            &[
                "mp4", "avi", "mkv", "mov", "wmv", "flv", "webm", "m4v", "3gp", "mpeg", "mpg",
                "ts", "m3u8",
            ],
        ),
        category(
            "music",
            "音乐",
            "Music",
            &[
                "mp3", "wav", "aac", "flac", "ogg", "opus", "aiff", "wma", "m4a",
            ],
        ),
        category(
            "pictures",
            "图片",
            "Pictures",
            &[
                "jpg", "jpeg", "png", "gif", "bmp", "tiff", "tif", "svg", "webp", "heic", "ico",
                "raw", "psd",
            ],
        ),
        category(
            "documents",
            "文档",
            "Documents",
            &[
                "doc", "docx", "pdf", "txt", "rtf", "odt", "xls", "xlsx", "ppt", "pptx", "csv",
                "epub", "pages",
            ],
        ),
    ]
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum QuickLaunchRowStyle {
    Continuous,
    #[default]
    Separated,
}
impl QuickLaunchRowStyle {
    pub const ALL: [Self; 2] = [Self::Continuous, Self::Separated];
    pub const fn gap(self) -> f32 {
        match self {
            Self::Continuous => 0.0,
            Self::Separated => 4.0,
        }
    }
    pub const fn label(self) -> &'static str {
        match self {
            Self::Continuous => "连续列表",
            Self::Separated => "分隔列表",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuickLaunchSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub auto_quick_switch: bool,
    #[serde(default = "default_true")]
    pub file_dialog_search: bool,
    #[serde(default = "default_true")]
    pub suppress_in_exclusive_fullscreen: bool,
    #[serde(default)]
    pub row_style: QuickLaunchRowStyle,
    #[serde(default = "default_quick_launch_hotkey")]
    pub hotkey: ClipboardHotKey,
    #[serde(default)]
    pub indexed_volumes: Vec<String>,
    #[serde(default)]
    pub excluded_paths: Vec<String>,
    #[serde(default)]
    pub custom_commands: Vec<CustomCommandSettings>,
}

impl Default for QuickLaunchSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_quick_switch: true,
            file_dialog_search: true,
            suppress_in_exclusive_fullscreen: true,
            row_style: QuickLaunchRowStyle::default(),
            hotkey: default_quick_launch_hotkey(),
            indexed_volumes: Vec::new(),
            excluded_paths: Vec::new(),
            custom_commands: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CustomCommandSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub keyword: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub program: String,
    #[serde(default)]
    pub argument_template: String,
    #[serde(default)]
    pub working_directory: String,
    #[serde(default)]
    pub run_as_admin: bool,
    #[serde(default)]
    pub run_silently: bool,
}

impl Default for CustomCommandSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            keyword: String::new(),
            title: String::new(),
            program: String::new(),
            argument_template: String::new(),
            working_directory: String::new(),
            run_as_admin: false,
            run_silently: false,
        }
    }
}

fn default_quick_launch_hotkey() -> ClipboardHotKey {
    ClipboardHotKey::new(Vec::new(), "Control", "双击 Ctrl")
}

const fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum LinkPanelPosition {
    TopLeft,
    #[default]
    Center,
    TopRight,
}

impl LinkPanelPosition {
    pub const ALL: [Self; 3] = [Self::TopLeft, Self::Center, Self::TopRight];
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct CurrencyConversionSettings {
    pub enabled: bool,
    pub target: String,
}
impl Default for CurrencyConversionSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            target: "CNY".into(),
        }
    }
}
impl CurrencyConversionSettings {
    /// Common localized display names; live currency availability belongs to Desktop's provider catalog.
    pub const COMMON_CURRENCIES: &'static [(&'static str, &'static str)] = &[
        ("CNY", "人民币"),
        ("USD", "美元"),
        ("EUR", "欧元"),
        ("CAD", "加元"),
        ("GBP", "英镑"),
        ("JPY", "日元"),
        ("HKD", "港币"),
        ("AUD", "澳元"),
        ("NZD", "新西兰元"),
        ("SGD", "新加坡元"),
        ("CHF", "瑞士法郎"),
        ("KRW", "韩元"),
        ("TWD", "新台币"),
        ("INR", "印度卢比"),
        ("THB", "泰铢"),
        ("MYR", "林吉特"),
        ("IDR", "印尼盾"),
        ("PHP", "菲律宾比索"),
        ("VND", "越南盾"),
        ("SEK", "瑞典克朗"),
        ("NOK", "挪威克朗"),
        ("DKK", "丹麦克朗"),
        ("PLN", "波兰兹罗提"),
        ("CZK", "捷克克朗"),
        ("HUF", "匈牙利福林"),
        ("TRY", "土耳其里拉"),
        ("BRL", "巴西雷亚尔"),
        ("MXN", "墨西哥比索"),
        ("ZAR", "南非兰特"),
        ("AED", "阿联酋迪拉姆"),
        ("SAR", "沙特里亚尔"),
    ];
    pub fn valid_code(code: &str) -> bool {
        (2..=16).contains(&code.len())
            && code.bytes().all(|byte| byte.is_ascii_alphanumeric())
            && code.bytes().any(|byte| byte.is_ascii_alphabetic())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ClipboardImageViewMode {
    #[default]
    Auto,
    Fit,
    Panorama,
    Light,
    Mini,
    Compact,
}
impl ClipboardImageViewMode {
    pub const ALL: [Self; 6] = [
        Self::Auto,
        Self::Fit,
        Self::Panorama,
        Self::Light,
        Self::Mini,
        Self::Compact,
    ];
    pub const fn label(self) -> &'static str {
        match self {
            Self::Auto => "自动",
            Self::Fit => "适应",
            Self::Panorama => "全景",
            Self::Light => "轻量",
            Self::Mini => "迷你",
            Self::Compact => "小巧",
        }
    }
    pub const fn description(self) -> &'static str {
        match self {
            Self::Auto => "根据相邻可用位置最大化显示；超宽或超高图片可滚动查看。",
            Self::Fit => "始终完整显示整张图片，不应用滚动或导航。",
            Self::Panorama => "按原始像素 1:1 显示，超出浮窗的部分可滚动查看。",
            Self::Light => "自动显示，浮窗短边不超过 800 屏幕像素。",
            Self::Mini => "自动显示，浮窗短边不超过 500 屏幕像素。",
            Self::Compact => "自动显示，浮窗短边不超过 300 屏幕像素。",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClipboardSettings {
    pub enabled: bool,
    pub capture_images: bool,
    pub capture_files: bool,
    #[serde(default)]
    pub image_view_mode: ClipboardImageViewMode,
    #[serde(default = "default_clipboard_text_card_max_height")]
    pub text_card_max_height: u32,
    #[serde(default = "default_true")]
    pub link_detector_enabled: bool,
    #[serde(default)]
    pub currency_conversion: CurrencyConversionSettings,
    #[serde(default)]
    pub link_panel_position: LinkPanelPosition,
    #[serde(default = "default_link_panel_auto_hide_ms")]
    pub link_panel_auto_hide_ms: u64,
    #[serde(default)]
    pub link_panel_auto_open_browser: bool,
    #[serde(default = "default_true")]
    pub link_panel_copy_only: bool,
    #[serde(default)]
    pub hotkey: ClipboardHotKey,
    #[serde(default = "default_clipboard_max_items")]
    pub max_items: usize,
    #[serde(default = "default_clipboard_retention_days")]
    pub retention_days: u32,
    #[serde(default = "default_clipboard_excluded_processes")]
    pub excluded_processes: Vec<String>,
    #[serde(default = "default_clipboard_max_text_bytes")]
    pub max_text_bytes: u64,
    #[serde(default = "default_clipboard_max_image_bytes")]
    pub max_image_bytes: u64,
    #[serde(default = "default_clipboard_max_image_pixels")]
    pub max_image_pixels: u64,
    #[serde(default)]
    pub modifier_actions: ClipboardModifierActions,
    #[serde(default)]
    pub storage_directory: String,
}

impl Default for ClipboardSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            capture_images: true,
            capture_files: true,
            image_view_mode: ClipboardImageViewMode::default(),
            text_card_max_height: default_clipboard_text_card_max_height(),
            link_detector_enabled: true,
            currency_conversion: CurrencyConversionSettings::default(),
            link_panel_position: LinkPanelPosition::default(),
            link_panel_auto_hide_ms: 6_000,
            link_panel_auto_open_browser: false,
            link_panel_copy_only: true,
            hotkey: ClipboardHotKey::default(),
            max_items: default_clipboard_max_items(),
            retention_days: default_clipboard_retention_days(),
            excluded_processes: default_clipboard_excluded_processes(),
            max_text_bytes: default_clipboard_max_text_bytes(),
            max_image_bytes: default_clipboard_max_image_bytes(),
            max_image_pixels: default_clipboard_max_image_pixels(),
            modifier_actions: ClipboardModifierActions::default(),
            storage_directory: String::new(),
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClipboardClickAction {
    OriginalPaste,
    CopyOnly,
    PlainText,
    WithoutLineBreaks,
    Disabled,
}

impl ClipboardClickAction {
    pub const ALL: [Self; 5] = [
        Self::OriginalPaste,
        Self::CopyOnly,
        Self::PlainText,
        Self::WithoutLineBreaks,
        Self::Disabled,
    ];
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClipboardModifierActions {
    pub control: ClipboardClickAction,
    pub alt: ClipboardClickAction,
    pub shift: ClipboardClickAction,
}

impl Default for ClipboardModifierActions {
    fn default() -> Self {
        Self {
            control: ClipboardClickAction::CopyOnly,
            alt: ClipboardClickAction::PlainText,
            shift: ClipboardClickAction::WithoutLineBreaks,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardClickModifier {
    Control,
    Alt,
    Shift,
}

impl ClipboardModifierActions {
    pub fn action(self, modifier: ClipboardClickModifier) -> ClipboardClickAction {
        match modifier {
            ClipboardClickModifier::Control => self.control,
            ClipboardClickModifier::Alt => self.alt,
            ClipboardClickModifier::Shift => self.shift,
        }
    }

    pub fn set(&mut self, modifier: ClipboardClickModifier, action: ClipboardClickAction) {
        match modifier {
            ClipboardClickModifier::Control => self.control = action,
            ClipboardClickModifier::Alt => self.alt = action,
            ClipboardClickModifier::Shift => self.shift = action,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardImagePasteProfile {
    NativeImage,
    NativeImageWithFileDrop,
}

pub fn clipboard_image_paste_profile(process_name: &str) -> ClipboardImagePasteProfile {
    let process_name = Path::new(process_name.trim())
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(process_name)
        .to_ascii_lowercase();
    let process_name = process_name.strip_suffix(".exe").unwrap_or(&process_name);
    if matches!(
        process_name,
        "explorer"
            | "dopus"
            | "dopusrt"
            | "totalcmd"
            | "totalcmd64"
            | "onecommander"
            | "files"
            | "freecommander"
            | "freecommanderxe"
            | "doublecmd"
            | "xyplorer"
            | "q-dir"
            | "windowsterminal"
            | "openconsole"
            | "conhost"
            | "cmd"
            | "powershell"
            | "pwsh"
    ) {
        ClipboardImagePasteProfile::NativeImageWithFileDrop
    } else {
        ClipboardImagePasteProfile::NativeImage
    }
}

fn default_link_panel_auto_hide_ms() -> u64 {
    6_000
}

fn default_clipboard_max_items() -> usize {
    1000
}

fn default_clipboard_retention_days() -> u32 {
    30
}

fn default_clipboard_max_text_bytes() -> u64 {
    8 * 1024 * 1024
}

fn default_clipboard_max_image_bytes() -> u64 {
    64 * 1024 * 1024
}

fn default_clipboard_max_image_pixels() -> u64 {
    100_000_000
}

fn default_clipboard_excluded_processes() -> Vec<String> {
    vec![
        "KeePass.exe".into(),
        "1Password.exe".into(),
        "Bitwarden.exe".into(),
    ]
}

pub const CLIPBOARD_TEXT_CARD_MIN_HEIGHT: u32 = 120;
pub const CLIPBOARD_TEXT_CARD_MAX_HEIGHT: u32 = 320;

pub const fn default_clipboard_text_card_max_height() -> u32 {
    160
}

impl ClipboardSettings {
    pub fn normalize(&mut self) {
        self.text_card_max_height = self.text_card_max_height.clamp(
            CLIPBOARD_TEXT_CARD_MIN_HEIGHT,
            CLIPBOARD_TEXT_CARD_MAX_HEIGHT,
        );
        self.currency_conversion.target =
            self.currency_conversion.target.trim().to_ascii_uppercase();
        if !CurrencyConversionSettings::valid_code(&self.currency_conversion.target) {
            self.currency_conversion.target = "CNY".into();
        }
        // Capture is a single product capability now. Keep these legacy fields on the wire so
        // older configs remain readable, but do not leave users stuck with hidden disabled flags.
        self.enabled = true;
        self.capture_images = true;
        self.capture_files = true;
        self.max_items = self.max_items.clamp(100, 100_000);
        self.retention_days = self.retention_days.clamp(1, 3650);
        self.max_text_bytes = self.max_text_bytes.clamp(64 * 1024, 64 * 1024 * 1024);
        self.max_image_bytes = self.max_image_bytes.clamp(1024 * 1024, 256 * 1024 * 1024);
        self.max_image_pixels = self.max_image_pixels.clamp(1_000_000, 500_000_000);
        self.storage_directory = self.storage_directory.trim().to_owned();
        let mut seen = std::collections::HashSet::new();
        self.excluded_processes.retain_mut(|process| {
            *process = process.trim().to_owned();
            !process.is_empty() && seen.insert(process.to_ascii_lowercase())
        });
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClipboardHotKeyModifier {
    Control,
    Alt,
    Shift,
    Win,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ClipboardHotKey {
    pub modifiers: Vec<ClipboardHotKeyModifier>,
    pub key: String,
    pub display: String,
}

impl ClipboardHotKey {
    pub fn new(
        modifiers: Vec<ClipboardHotKeyModifier>,
        key: impl Into<String>,
        display: impl Into<String>,
    ) -> Self {
        Self {
            modifiers,
            key: key.into(),
            display: display.into(),
        }
    }
}

impl Default for ClipboardHotKey {
    fn default() -> Self {
        Self::new(vec![ClipboardHotKeyModifier::Alt], "C", "Alt+C")
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ClipboardHotKeyModifiersWire {
    Semantic(Vec<ClipboardHotKeyModifier>),
    LegacyWin32(u32),
}

#[derive(Deserialize)]
struct ClipboardHotKeyWire {
    modifiers: Option<ClipboardHotKeyModifiersWire>,
    key: Option<String>,
    virtual_key: Option<u32>,
    display: Option<String>,
}

impl<'de> Deserialize<'de> for ClipboardHotKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = ClipboardHotKeyWire::deserialize(deserializer)?;
        let modifiers = match wire.modifiers {
            Some(ClipboardHotKeyModifiersWire::Semantic(modifiers)) => modifiers,
            Some(ClipboardHotKeyModifiersWire::LegacyWin32(bits)) => {
                let mut modifiers = Vec::new();
                if bits & 0x0002 != 0 {
                    modifiers.push(ClipboardHotKeyModifier::Control);
                }
                if bits & 0x0001 != 0 {
                    modifiers.push(ClipboardHotKeyModifier::Alt);
                }
                if bits & 0x0004 != 0 {
                    modifiers.push(ClipboardHotKeyModifier::Shift);
                }
                if bits & 0x0008 != 0 {
                    modifiers.push(ClipboardHotKeyModifier::Win);
                }
                modifiers
            }
            None => ClipboardHotKey::default().modifiers,
        };
        let key = wire
            .key
            .or_else(|| wire.virtual_key.and_then(legacy_virtual_key_name))
            .unwrap_or_else(|| ClipboardHotKey::default().key);
        let display = wire.display.unwrap_or_else(|| key.clone());
        Ok(Self::new(modifiers, key, display))
    }
}

fn legacy_virtual_key_name(virtual_key: u32) -> Option<String> {
    match virtual_key {
        value @ 0x30..=0x39 | value @ 0x41..=0x5A => char::from_u32(value).map(String::from),
        value @ 0x70..=0x87 => Some(format!("F{}", value - 0x70 + 1)),
        0x20 => Some("Space".into()),
        0x09 => Some("Tab".into()),
        0x2D => Some("Insert".into()),
        0x2E => Some("Delete".into()),
        0x24 => Some("Home".into()),
        0x23 => Some("End".into()),
        0x21 => Some("PageUp".into()),
        0x22 => Some("PageDown".into()),
        0x26 => Some("Up".into()),
        0x28 => Some("Down".into()),
        0x25 => Some("Left".into()),
        0x27 => Some("Right".into()),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClipboardPanelSize {
    pub width: u32,
    pub height: u32,
}

impl Default for ClipboardPanelSize {
    fn default() -> Self {
        Self {
            width: 300,
            height: 440,
        }
    }
}

impl ClipboardPanelSize {
    pub fn clamped(width: u32, height: u32) -> Self {
        Self {
            width: width.clamp(280, 900),
            height: height.clamp(280, 900),
        }
    }
}

impl AppConfig {
    pub fn load_from(path: &Path) -> Result<Self> {
        let json = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let mut config: Self = serde_json::from_str(&json)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        config.normalize();
        config.extensions.validate().map_err(anyhow::Error::new)?;
        config.archive.validate().map_err(anyhow::Error::msg)?;
        Ok(config)
    }

    pub fn load_or_create(paths: &AppPaths) -> Result<Self> {
        bundled_extensions::load_or_create(paths)
    }

    fn normalize(&mut self) {
        self.network.normalize();
        self.clipboard_panel =
            ClipboardPanelSize::clamped(self.clipboard_panel.width, self.clipboard_panel.height);
        self.clipboard.normalize();
        if self.process_blacklist.entries.is_none() {
            let mut entries = self.process_blacklist.entries();
            for process in &self.clipboard.excluded_processes {
                if let Ok(rule) = ProcessBlacklistRule::new(process, true, false)
                    && !entries.iter().any(|entry| entry.process == rule.process)
                {
                    entries.push(rule);
                }
            }
            self.process_blacklist = ProcessBlacklistSettings::from_entries(entries);
        }
        self.process_blacklist.normalize();
        self.clipboard.excluded_processes = self.process_blacklist.clipboard_processes();
        self.quick_launch.normalize();
        self.floating_assistant.normalize();
        self.download.normalize();
    }

    pub fn process_is_blacklisted(
        &self,
        process_name: &str,
        feature: ProcessBlacklistFeature,
    ) -> bool {
        self.process_blacklist.contains(process_name, feature)
    }

    pub fn set_process_blacklist(
        &mut self,
        processes: Vec<String>,
        disable_clipboard: bool,
        disable_quick_launch: bool,
    ) {
        self.process_blacklist = ProcessBlacklistSettings {
            entries: None,
            processes,
            disable_clipboard,
            disable_quick_launch,
        };
        self.process_blacklist.normalize();
        self.clipboard.excluded_processes = self.process_blacklist.clipboard_processes();
    }

    pub fn process_blacklist_processes(&self) -> Vec<String> {
        self.process_blacklist.processes.clone()
    }

    /// Atomically replaces the config file: the JSON is written to a sibling
    /// temporary file, flushed to disk, then renamed over the target. A crash
    /// or power loss leaves either the old or the new file, never a torn one.
    pub fn save_to(&self, path: &Path) -> Result<()> {
        self.extensions.validate().map_err(anyhow::Error::new)?;
        self.archive.validate().map_err(anyhow::Error::msg)?;
        let json = serde_json::to_string_pretty(self)?;
        let file_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "config.json".into());
        let temporary = path.with_file_name(format!(
            ".{file_name}.{}.{:x}.tmp",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map_or(0, |elapsed| elapsed.as_nanos())
        ));
        let write = (|| -> std::io::Result<()> {
            use std::io::Write;
            let mut file = fs::File::create(&temporary)?;
            file.write_all(json.as_bytes())?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temporary, path)
        })();
        if let Err(error) = write {
            let _ = fs::remove_file(&temporary);
            return Err(error).with_context(|| format!("failed to write {}", path.display()));
        }
        Ok(())
    }

    /// Read-modify-write one logical section of the config under an
    /// inter-process lock. Long-lived modules keep their own in-memory
    /// `AppConfig`, so saving that copy whole would overwrite sections other
    /// modules changed since startup. Here the file is re-read, only the
    /// caller's fields are changed, and the merged result is returned so the
    /// caller can refresh its cache.
    ///
    /// The file must already exist and parse. A missing or corrupt target
    /// must never recreate unrelated settings from defaults.
    pub fn update_section(path: &Path, mutate: impl FnOnce(&mut Self)) -> Result<Self> {
        Self::try_update_section(path, |config| {
            mutate(config);
            Ok(())
        })
    }

    pub(crate) fn try_update_section(
        path: &Path,
        mutate: impl FnOnce(&mut Self) -> Result<()>,
    ) -> Result<Self> {
        Self::with_config_lock(path, || {
            let mut config = Self::load_from(path)?;
            mutate(&mut config)?;
            config.normalize();
            config.save_to(path)?;
            Ok(config)
        })
    }

    /// One original lock boundary shared by ordinary section updates and multi-store
    /// extension data lifecycle commits. The callback must never reacquire this lock.
    pub(crate) fn with_config_lock<T>(
        path: &Path,
        operation: impl FnOnce() -> Result<T>,
    ) -> Result<T> {
        let lock_path = path.with_extension("json.lock");
        let lock = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&lock_path)
            .with_context(|| format!("failed to open {}", lock_path.display()))?;
        lock.lock_exclusive()
            .with_context(|| format!("failed to lock {}", lock_path.display()))?;
        let result = operation();
        let _ = FileExt::unlock(&lock);
        result
    }

    /// Validates operation presets and commits only the archive section.
    pub fn update_archive_settings(path: &Path, settings: ArchiveSettings) -> Result<Self> {
        settings.validate().map_err(anyhow::Error::msg)?;
        Self::update_section(path, |config| config.archive = settings)
    }

    /// Commits only the floating assistant section against the latest on-disk config.
    pub fn update_floating_assistant_settings(
        path: &Path,
        mut settings: FloatingAssistantSettings,
    ) -> Result<Self> {
        settings.normalize();
        Self::update_section(path, |config| config.floating_assistant = settings)
    }

    pub fn update_download_settings(path: &Path, mut settings: DownloadSettings) -> Result<Self> {
        settings.normalize();
        Self::update_section(path, |config| config.download = settings)
    }

    pub fn update_network_settings(path: &Path, mut settings: NetworkSettings) -> Result<Self> {
        settings.normalize();
        settings.validate().map_err(anyhow::Error::msg)?;
        Self::update_section(path, |config| config.network = settings)
    }

    pub fn update_system_settings(path: &Path, settings: SystemSettings) -> Result<Self> {
        Self::update_section(path, |config| config.system = settings)
    }

    pub fn update_global_settings(
        path: &Path,
        mut network: NetworkSettings,
        system: SystemSettings,
    ) -> Result<Self> {
        network.normalize();
        network.validate().map_err(anyhow::Error::msg)?;
        Self::update_section(path, |config| {
            config.network = network;
            config.system = system;
        })
    }

    pub fn update_global_settings_with_blacklist(
        path: &Path,
        mut network: NetworkSettings,
        system: SystemSettings,
        blacklist: ProcessBlacklistSettings,
    ) -> Result<Self> {
        network.normalize();
        network.validate().map_err(anyhow::Error::msg)?;
        Self::update_section(path, |config| {
            config.network = network;
            config.system = system;
            config.set_process_blacklist_settings(blacklist);
        })
    }

    pub fn update_quick_launch_settings(
        path: &Path,
        settings: QuickLaunchSettings,
    ) -> Result<Self> {
        Self::try_update_section(path, |config| {
            let manual = settings
                .custom_commands
                .iter()
                .filter(|command| command.enabled)
                .map(|command| command.keyword.clone())
                .collect::<Vec<_>>();
            config.extensions.validate_manual_keywords(&manual)?;
            config.quick_launch = settings;
            Ok(())
        })
    }

    pub fn update_clipboard_settings(path: &Path, settings: ClipboardSettings) -> Result<Self> {
        Self::update_section(path, |config| config.clipboard = settings)
    }

    pub fn update_process_blacklist(
        path: &Path,
        settings: ProcessBlacklistSettings,
    ) -> Result<Self> {
        Self::update_section(path, |config| {
            config.set_process_blacklist_settings(settings)
        })
    }

    pub fn set_process_blacklist_settings(&mut self, mut settings: ProcessBlacklistSettings) {
        settings.normalize();
        self.clipboard.excluded_processes = settings.clipboard_processes();
        self.process_blacklist = settings;
    }
}

impl QuickLaunchSettings {
    pub fn normalize(&mut self) {
        let key = self.hotkey.key.trim();
        let is_double_control = self.hotkey.modifiers.is_empty()
            && (key.eq_ignore_ascii_case("control") || key.eq_ignore_ascii_case("ctrl"));
        if key.is_empty() || (self.hotkey.modifiers.is_empty() && !is_double_control) {
            self.hotkey = Self::default().hotkey;
        }
        self.hotkey.key = self.hotkey.key.trim().to_owned();
        self.hotkey.display = self.hotkey.display.trim().to_owned();
        if self.hotkey.display.is_empty() {
            self.hotkey.display = self.hotkey.key.clone();
        }
        normalize_unique_paths(&mut self.indexed_volumes);
        normalize_unique_paths(&mut self.excluded_paths);
        for command in &mut self.custom_commands {
            command.keyword = command.keyword.trim().to_owned();
            command.title = command.title.trim().to_owned();
            command.program = command.program.trim().trim_matches('"').to_owned();
            command.working_directory = command
                .working_directory
                .trim()
                .trim_matches('"')
                .to_owned();
        }
    }

    pub fn validate_custom_commands(&self) -> std::result::Result<(), String> {
        let mut keywords = std::collections::HashSet::new();
        for (index, command) in self.custom_commands.iter().enumerate() {
            if !command.enabled {
                continue;
            }
            let position = index + 1;
            if command.keyword.is_empty() {
                return Err(format!("第 {position} 条命令缺少关键词"));
            }
            if command.keyword.chars().any(char::is_whitespace) {
                return Err(format!("命令“{}”的关键词不能包含空白", command.keyword));
            }
            if command.program.is_empty() {
                return Err(format!("命令“{}”缺少程序路径", command.keyword));
            }
            if !keywords.insert(command.keyword.to_lowercase()) {
                return Err(format!("命令关键词“{}”重复", command.keyword));
            }
        }
        Ok(())
    }
}

fn normalize_unique_paths(paths: &mut Vec<String>) {
    let mut seen = std::collections::HashSet::new();
    paths.retain_mut(|path| {
        *path = path.trim().trim_matches('"').replace('/', "\\");
        while path.len() > 3 && path.ends_with('\\') {
            path.pop();
        }
        !path.is_empty() && seen.insert(path.to_ascii_lowercase())
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_lock_state_distinguishes_stale_hints_from_a_live_owner() {
        let root = std::env::temp_dir().join(format!(
            "nxtools-core-instance-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("system clock")
                .as_nanos()
        ));
        let paths = AppPaths {
            config_dir: root.join("config"),
            data_dir: root.join("data"),
            cache_dir: root.join("cache"),
            log_dir: root.join("logs"),
        };
        fs::create_dir_all(&paths.cache_dir).expect("create instance cache");

        SingleInstanceGuard::record_main_window_handle(&paths, "desktop", 12345);
        assert_eq!(
            SingleInstanceGuard::read_main_window_handle(&paths, "desktop"),
            12345
        );
        assert!(!SingleInstanceGuard::is_locked(&paths, "desktop"));
        let guard = SingleInstanceGuard::acquire(&paths, "desktop").expect("acquire instance");
        assert!(SingleInstanceGuard::is_locked(&paths, "desktop"));
        drop(guard);
        assert!(!SingleInstanceGuard::is_locked(&paths, "desktop"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rich_image_editors_do_not_receive_a_duplicate_file_drop_representation() {
        assert_eq!(
            clipboard_image_paste_profile("zed.exe"),
            ClipboardImagePasteProfile::NativeImage
        );
        assert_eq!(
            clipboard_image_paste_profile("ZED"),
            ClipboardImagePasteProfile::NativeImage
        );
        assert_eq!(
            clipboard_image_paste_profile("explorer.exe"),
            ClipboardImagePasteProfile::NativeImageWithFileDrop
        );
        assert_eq!(
            clipboard_image_paste_profile("C:/Windows/EXPLORER.EXE"),
            ClipboardImagePasteProfile::NativeImageWithFileDrop
        );
        assert_eq!(
            clipboard_image_paste_profile("WindowsTerminal.exe"),
            ClipboardImagePasteProfile::NativeImageWithFileDrop
        );
    }

    #[test]
    fn default_config_is_stable_and_serializable() {
        let config = AppConfig::default();
        assert_eq!(config.clipboard_panel.width, 300);
        assert_eq!(config.clipboard_panel.height, 440);
        let json = serde_json::to_string(&config).expect("serialize");
        let decoded = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(config, decoded);
        assert!(config.quick_launch.enabled);
        assert!(config.quick_launch.auto_quick_switch);
        assert!(config.quick_launch.file_dialog_search);
        assert!(config.quick_launch.suppress_in_exclusive_fullscreen);
        assert!(config.quick_launch.hotkey.modifiers.is_empty());
    }

    #[test]
    fn download_settings_have_reference_aligned_safe_defaults() {
        let settings = DownloadSettings::default();

        assert!(settings.default_directory.is_empty());
        assert!(settings.ffmpeg_executable_path.is_empty());
        assert_eq!(settings.default_connection_limit, 8);
        assert!(settings.adaptive_segment_scaling_enabled);
        assert_eq!(settings.adaptive_segment_max_segments, 32);
        assert_eq!(settings.adaptive_segment_min_mib, 3);
        assert!(settings.show_progress_window);
        assert!(settings.show_completion_window);
        assert!(settings.completion_sound_enabled);
        assert!(!settings.open_directory_on_completion);
        assert!(settings.taskbar_progress_overlay.enabled);
        assert_eq!(
            settings.taskbar_progress_overlay.anchor,
            DownloadTaskbarOverlayAnchor::Right
        );
        assert_eq!(settings.taskbar_progress_overlay.offset_x, 12);
        assert_eq!(settings.taskbar_progress_overlay.offset_y, 0);
        assert_eq!(settings.taskbar_progress_overlay.width, 132);
        assert_eq!(settings.taskbar_progress_overlay.height, 8);
        assert_eq!(
            settings.taskbar_progress_overlay.progress_mode,
            DownloadTaskbarProgressMode::PerTask
        );
        assert!(settings.taskbar_progress_overlay.show_progress);
        assert!(!settings.taskbar_progress_overlay.show_speed);
        assert!(!settings.taskbar_progress_overlay.show_remaining_time);
        assert!(!settings.taskbar_progress_overlay.show_active_tasks);
        assert!(!settings.taskbar_progress_overlay.show_active_connections);
        assert!(!settings.taskbar_progress_overlay.show_transferred_bytes);
    }

    #[test]
    fn taskbar_overlay_layout_is_normalized_to_safe_taskbar_bounds() {
        let mut settings = DownloadSettings::default();
        settings.taskbar_progress_overlay.width = 4;
        settings.taskbar_progress_overlay.height = 500;

        settings.normalize();

        assert_eq!(settings.taskbar_progress_overlay.width, 72);
        assert_eq!(settings.taskbar_progress_overlay.height, 72);
    }

    #[test]
    fn adaptive_segment_settings_are_normalized_against_the_connection_limit() {
        let mut settings = DownloadSettings {
            default_connection_limit: 48,
            adaptive_segment_max_segments: 4,
            adaptive_segment_min_mib: 0,
            ..DownloadSettings::default()
        };

        settings.normalize();

        assert_eq!(settings.adaptive_segment_max_segments, 48);
        assert_eq!(settings.adaptive_segment_min_mib, 1);
    }

    #[test]
    fn taskbar_overlay_text_information_keeps_a_readable_minimum_height() {
        let mut settings = DownloadSettings::default();
        settings.taskbar_progress_overlay.height = 8;
        settings.taskbar_progress_overlay.show_speed = true;

        settings.normalize();

        assert_eq!(settings.taskbar_progress_overlay.height, 20);
    }

    #[test]
    fn taskbar_overlay_normalization_never_leaves_an_empty_surface() {
        let mut settings = DownloadSettings::default();
        settings.taskbar_progress_overlay.show_progress = false;

        settings.normalize();

        assert!(settings.taskbar_progress_overlay.show_progress);
    }

    #[test]
    fn legacy_download_settings_receive_new_defaults() {
        let config: AppConfig = serde_json::from_str(
            r#"{"theme":"dark","platform_pipe_name":"legacy-pipe","download":{"show_completion_window":false}}"#,
        )
        .expect("legacy download settings must remain readable");

        assert_eq!(config.download.default_connection_limit, 8);
        assert!(config.download.ffmpeg_executable_path.is_empty());
        assert!(config.download.show_progress_window);
        assert!(!config.download.show_completion_window);
        assert!(config.download.completion_sound_enabled);
        assert!(!config.download.open_directory_on_completion);
    }

    #[test]
    fn legacy_config_receives_safe_direct_network_defaults() {
        let config: AppConfig =
            serde_json::from_str(r#"{"theme":"dark","platform_pipe_name":"legacy-pipe"}"#)
                .expect("legacy config must remain readable");

        assert_eq!(config.network.proxy.mode, NetworkProxyMode::Direct);
        assert!(config.network.proxy.host.is_empty());
        assert_eq!(config.network.proxy.port, 8080);
        assert!(!config.system.start_on_login_silently);
        assert!(config.system.close_to_tray);
    }

    #[test]
    fn global_settings_update_persists_shell_and_network_without_losing_modules() {
        let directory = std::env::temp_dir().join(format!(
            "nxtools-global-settings-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        fs::create_dir_all(&directory).expect("create global settings directory");
        let path = directory.join("config.json");
        let mut original = AppConfig::default();
        original.quick_launch.enabled = false;
        original.save_to(&path).expect("save original config");

        let updated = AppConfig::update_global_settings(
            &path,
            NetworkSettings::default(),
            SystemSettings {
                start_on_login_silently: true,
                close_to_tray: false,
                theme: ThemePreference::Dark,
                notification_position: NotificationPosition::TopCenter,
                notification_duration_seconds: default_notification_duration(),
            },
        )
        .expect("update global settings");

        assert!(updated.system.start_on_login_silently);
        assert!(!updated.system.close_to_tray);
        assert!(!updated.quick_launch.enabled);
        assert_eq!(
            AppConfig::load_from(&path).expect("reload").system,
            updated.system
        );
        fs::remove_dir_all(directory).expect("remove settings directory");
    }

    #[test]
    fn manual_proxy_validation_is_bounded_and_normalized() {
        let mut settings = NetworkSettings {
            proxy: NetworkProxySettings {
                mode: NetworkProxyMode::ManualHttp,
                host: "  proxy.example.test  ".into(),
                port: 3128,
            },
        };

        settings.normalize();

        assert_eq!(settings.proxy.host, "proxy.example.test");
        assert!(settings.validate().is_ok());

        settings.proxy.host = "bad host".into();
        assert!(settings.validate().is_err());
        settings.proxy.host = "http://proxy.example.test".into();
        assert!(settings.validate().is_err());
        settings.proxy.host = "proxy.example.test".into();
        settings.proxy.port = 0;
        assert!(settings.validate().is_err());
    }

    #[test]
    fn updating_network_settings_preserves_module_configuration() {
        let directory = std::env::temp_dir().join(format!(
            "nxtools-network-settings-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        fs::create_dir_all(&directory).expect("create settings directory");
        let path = directory.join("config.json");
        let mut original = AppConfig::default();
        original.quick_launch.enabled = false;
        original.download.default_connection_limit = 12;
        original.save_to(&path).expect("save original config");

        let updated = AppConfig::update_network_settings(
            &path,
            NetworkSettings {
                proxy: NetworkProxySettings {
                    mode: NetworkProxyMode::ManualHttp,
                    host: "proxy.example.test".into(),
                    port: 8081,
                },
            },
        )
        .expect("update only network settings");

        assert_eq!(updated.network.proxy.mode, NetworkProxyMode::ManualHttp);
        assert_eq!(updated.network.proxy.host, "proxy.example.test");
        assert_eq!(updated.network.proxy.port, 8081);
        assert!(!updated.quick_launch.enabled);
        assert_eq!(updated.download.default_connection_limit, 12);
        fs::remove_dir_all(directory).expect("remove settings directory");
    }

    #[test]
    fn updating_download_settings_preserves_other_module_configuration() {
        let directory = std::env::temp_dir().join(format!(
            "nxtools-download-settings-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        fs::create_dir_all(&directory).expect("create settings directory");
        let path = directory.join("config.json");
        let mut original = AppConfig::default();
        original.quick_launch.enabled = false;
        original.clipboard.retention_days = 91;
        original.save_to(&path).expect("save original config");

        let updated = AppConfig::update_download_settings(
            &path,
            DownloadSettings {
                default_directory: r"D:\Downloads".into(),
                chrome_web_store_extension_id: String::new(),
                automatic_categories_enabled: true,
                categories: default_download_categories(),
                duplicate_strategy: DownloadDuplicateStrategySettings::AddNumbered,
                task_list_sort: None,
                task_list_sort_descending: None,
                double_click_action: DownloadRowDoubleClickActionSettings::OpenFile,
                ffmpeg_executable_path: r"D:\Tools\ffmpeg.exe".into(),
                default_request_headers: "X-Test: value".into(),
                default_cookie: "session=test".into(),
                accept_invalid_certificates: true,
                default_connection_limit: 12,
                adaptive_segment_scaling_enabled: true,
                adaptive_segment_max_segments: 32,
                adaptive_segment_min_mib: 3,
                show_progress_window: false,
                show_completion_window: false,
                completion_sound_enabled: false,
                open_directory_on_completion: true,
                task_removal_deletes_file: true,
                taskbar_progress_overlay: DownloadTaskbarProgressOverlaySettings::default(),
                background_speed_limit: Default::default(),
            },
        )
        .expect("update only download settings");

        assert_eq!(
            updated.download.ffmpeg_executable_path,
            r"D:\Tools\ffmpeg.exe"
        );

        assert!(!updated.quick_launch.enabled);
        assert_eq!(updated.clipboard.retention_days, 91);
        assert_eq!(updated.download.default_connection_limit, 12);
        assert_eq!(updated.download.default_request_headers, "X-Test: value");
        assert_eq!(updated.download.default_cookie, "session=test");
        assert!(updated.download.accept_invalid_certificates);
        assert!(!updated.download.completion_sound_enabled);
        assert!(updated.download.open_directory_on_completion);
        fs::remove_dir_all(directory).expect("remove settings directory");
    }

    #[test]
    fn legacy_config_receives_quick_launch_defaults() {
        let config: AppConfig =
            serde_json::from_str(r#"{"theme":"dark","platform_pipe_name":"legacy-pipe"}"#)
                .expect("legacy config must remain readable");

        assert!(config.quick_launch.enabled);
        assert!(config.quick_launch.auto_quick_switch);
        assert!(config.quick_launch.file_dialog_search);
        assert!(config.quick_launch.suppress_in_exclusive_fullscreen);
        assert_eq!(config.quick_launch.hotkey.key, "Control");
    }

    #[test]
    fn partial_quick_launch_config_does_not_inherit_the_clipboard_hotkey() {
        let config: AppConfig = serde_json::from_str(
            r#"{"theme":"dark","platform_pipe_name":"legacy-pipe","quick_launch":{"enabled":true}}"#,
        )
        .expect("partial quick-launch config must remain readable");

        assert_eq!(config.quick_launch.hotkey.display, "双击 Ctrl");
    }
    #[test]
    fn exclusive_fullscreen_suppression_setting_defaults_on_and_round_trips() {
        let legacy: QuickLaunchSettings =
            serde_json::from_str(r#"{"enabled":true}"#).expect("legacy quick launch settings");
        assert!(legacy.suppress_in_exclusive_fullscreen);

        let settings = QuickLaunchSettings {
            suppress_in_exclusive_fullscreen: false,
            ..QuickLaunchSettings::default()
        };
        let encoded = serde_json::to_string(&settings).expect("serialize quick launch settings");
        let decoded: QuickLaunchSettings =
            serde_json::from_str(&encoded).expect("deserialize quick launch settings");
        assert!(!decoded.suppress_in_exclusive_fullscreen);
        assert_eq!(decoded.hotkey, settings.hotkey);
        assert_eq!(decoded.indexed_volumes, settings.indexed_volumes);
        assert_eq!(decoded.excluded_paths, settings.excluded_paths);
    }

    #[test]
    fn quick_launch_index_scope_and_exclusions_survive_config_reload() {
        let config = AppConfig {
            quick_launch: QuickLaunchSettings {
                indexed_volumes: vec![r"D:\".into()],
                excluded_paths: vec![r"D:\Build".into(), r"D:\Cache".into()],
                ..QuickLaunchSettings::default()
            },
            ..AppConfig::default()
        };

        let json = serde_json::to_string(&config).expect("serialize quick-launch settings");
        let restored: AppConfig =
            serde_json::from_str(&json).expect("deserialize quick-launch settings");

        assert_eq!(restored.quick_launch, config.quick_launch);
    }

    #[test]
    fn custom_commands_survive_config_reload_and_are_normalized() {
        let mut config = AppConfig::default();
        config
            .quick_launch
            .custom_commands
            .push(CustomCommandSettings {
                keyword: "  Git  ".into(),
                title: "  Git status  ".into(),
                program: r#""C:\Program Files\Git\bin\git.exe""#.into(),
                argument_template: "status %s1".into(),
                working_directory: r#""D:\Work""#.into(),
                run_as_admin: true,
                run_silently: true,
                ..CustomCommandSettings::default()
            });
        let json = serde_json::to_string(&config).expect("serialize config");
        let mut restored: AppConfig = serde_json::from_str(&json).expect("deserialize config");
        restored.quick_launch.normalize();
        let command = &restored.quick_launch.custom_commands[0];
        assert_eq!(command.keyword, "Git");
        assert_eq!(command.title, "Git status");
        assert_eq!(command.program, r"C:\Program Files\Git\bin\git.exe");
        assert_eq!(command.working_directory, r"D:\Work");
        assert!(command.run_as_admin);
        assert!(command.run_silently);
    }

    #[test]
    fn enabled_custom_commands_require_unique_keywords_and_programs() {
        let mut settings = QuickLaunchSettings {
            custom_commands: vec![
                CustomCommandSettings {
                    keyword: "open".into(),
                    title: "One".into(),
                    program: "one.exe".into(),
                    ..CustomCommandSettings::default()
                },
                CustomCommandSettings {
                    keyword: "OPEN".into(),
                    title: "Two".into(),
                    program: "two.exe".into(),
                    ..CustomCommandSettings::default()
                },
            ],
            ..QuickLaunchSettings::default()
        };
        assert_eq!(
            settings.validate_custom_commands(),
            Err("命令关键词“OPEN”重复".into())
        );
        settings.custom_commands[1].enabled = false;
        assert!(settings.validate_custom_commands().is_ok());
        settings.custom_commands[0].program.clear();
        assert_eq!(
            settings.validate_custom_commands(),
            Err("命令“open”缺少程序路径".into())
        );
    }

    #[test]
    fn legacy_config_without_panel_size_uses_the_current_default() {
        let config: AppConfig =
            serde_json::from_str(r#"{"theme":"dark","platform_pipe_name":"legacy-pipe"}"#)
                .expect("legacy config must remain readable");

        assert_eq!(config.clipboard_panel.width, 300);
        assert_eq!(config.clipboard_panel.height, 440);
        assert!(config.clipboard.enabled);
        assert!(config.clipboard.capture_images);
        assert!(config.clipboard.capture_files);
    }

    #[test]
    fn clipboard_capture_settings_are_serializable() {
        let config = AppConfig {
            clipboard: ClipboardSettings {
                enabled: false,
                capture_images: false,
                capture_files: true,
                hotkey: ClipboardHotKey::new(
                    vec![
                        ClipboardHotKeyModifier::Control,
                        ClipboardHotKeyModifier::Shift,
                    ],
                    "V",
                    "Ctrl+Shift+V",
                ),
                ..ClipboardSettings::default()
            },
            ..AppConfig::default()
        };
        let json = serde_json::to_string(&config).expect("serialize settings");
        let restored: AppConfig = serde_json::from_str(&json).expect("deserialize settings");
        assert_eq!(restored.clipboard, config.clipboard);
    }
    #[test]
    fn link_panel_position_round_trips_and_legacy_configs_default_to_center() {
        let settings = ClipboardSettings {
            link_panel_position: LinkPanelPosition::TopRight,
            ..ClipboardSettings::default()
        };
        let json = serde_json::to_string(&settings).expect("serialize link panel position");
        let restored: ClipboardSettings =
            serde_json::from_str(&json).expect("deserialize link panel position");
        assert_eq!(restored.link_panel_position, LinkPanelPosition::TopRight);

        let legacy: ClipboardSettings = serde_json::from_str(
            r#"{"enabled":true,"capture_images":true,"capture_files":true,"max_items":1000,"retention_days":30,"excluded_processes":[]}"#,
        )
        .expect("legacy clipboard settings must remain readable");
        assert_eq!(legacy.link_panel_position, LinkPanelPosition::Center);
    }

    #[test]
    fn clipboard_storage_directory_survives_config_reload() {
        let config = AppConfig {
            clipboard: ClipboardSettings {
                storage_directory: r"D:\NxToolsData\Clipboard".into(),
                ..ClipboardSettings::default()
            },
            ..AppConfig::default()
        };

        let json = serde_json::to_string(&config).expect("serialize storage directory");
        let restored: AppConfig =
            serde_json::from_str(&json).expect("deserialize storage directory");

        assert_eq!(
            restored.clipboard.storage_directory,
            r"D:\NxToolsData\Clipboard"
        );
    }

    #[test]
    fn canonical_blacklist_roundtrip_remove_empty_and_sibling_merge() {
        let directory = unique_config_dir("canonical-rules");
        let path = directory.join("config.json");
        AppConfig::default().save_to(&path).expect("seed config");
        let rules = ProcessBlacklistSettings::from_entries(vec![
            ProcessBlacklistRule::new("C:\\Apps\\Editor.EXE", true, false).expect("editor"),
            ProcessBlacklistRule::new("editor.exe", false, true).expect("duplicate editor"),
            ProcessBlacklistRule::new("Game.exe", false, true).expect("game"),
        ]);
        let merged = AppConfig::update_global_settings_with_blacklist(
            &path,
            NetworkSettings::default(),
            SystemSettings::default(),
            rules,
        )
        .expect("atomic global save");
        assert_eq!(merged.process_blacklist.entries().len(), 2);
        assert!(merged.process_is_blacklisted("EDITOR.EXE", ProcessBlacklistFeature::Clipboard));
        assert!(merged.process_is_blacklisted("editor", ProcessBlacklistFeature::QuickLaunch));
        assert!(!merged.process_is_blacklisted("game", ProcessBlacklistFeature::Clipboard));
        assert!(merged.process_is_blacklisted("game", ProcessBlacklistFeature::QuickLaunch));
        assert_eq!(merged.clipboard.excluded_processes, ["editor"]);
        let mut clipboard = merged.clipboard.clone();
        clipboard.max_items = 2048;
        AppConfig::update_clipboard_settings(&path, clipboard).expect("sibling save");
        let only_game = ProcessBlacklistSettings::from_entries(vec![
            ProcessBlacklistRule::new("Game.exe", false, true).expect("game"),
        ]);
        AppConfig::update_process_blacklist(&path, only_game).expect("remove editor");
        let reloaded = AppConfig::load_from(&path).expect("reload removed entry");
        assert!(!reloaded.process_is_blacklisted("editor", ProcessBlacklistFeature::Clipboard));
        assert_eq!(reloaded.clipboard.max_items, 2048);
        AppConfig::update_process_blacklist(&path, ProcessBlacklistSettings::from_entries(vec![]))
            .expect("explicit empty save");
        let empty = AppConfig::load_from(&path).expect("reload empty");
        assert!(empty.process_blacklist.entries().is_empty());
        assert!(empty.clipboard.excluded_processes.is_empty());
        fs::remove_dir_all(directory).expect("remove fixture");
    }

    #[test]
    fn canonical_rules_ignore_stale_legacy_and_keep_unchecked_rows() {
        let mut config = AppConfig {
            process_blacklist: ProcessBlacklistSettings::from_entries(vec![
                ProcessBlacklistRule::new("Keepass.exe", false, false).expect("unchecked row"),
            ]),
            ..Default::default()
        };
        config.clipboard.excluded_processes = vec!["KeePass.exe".into(), "Old.exe".into()];
        config.normalize();
        assert_eq!(config.process_blacklist.entries().len(), 1);
        assert!(!config.process_is_blacklisted("keepass", ProcessBlacklistFeature::Clipboard));
        assert!(config.clipboard.excluded_processes.is_empty());
        assert!(ProcessBlacklistRule::new("  ", true, false).is_err());
        assert!(ProcessBlacklistRule::new("bad;name.exe", true, false).is_err());
    }

    #[test]
    fn legacy_blacklist_migrates_from_disk_once_and_never_reimports_projection() {
        let directory = unique_config_dir("legacy-rule-migration");
        let path = directory.join("config.json");
        let mut json = serde_json::to_value(AppConfig::default()).expect("legacy seed");
        json["process_blacklist"] = serde_json::json!({
            "processes": ["C:\\Apps\\Game.EXE"], "disable_clipboard": false, "disable_quick_launch": true
        });
        json["clipboard"]["excluded_processes"] = serde_json::json!(["Game.exe", "Editor.exe"]);
        fs::write(&path, serde_json::to_vec(&json).expect("encode legacy")).expect("legacy disk");
        let migrated = AppConfig::load_from(&path).expect("migrate legacy");
        assert!(!migrated.process_is_blacklisted("game", ProcessBlacklistFeature::Clipboard));
        assert!(migrated.process_is_blacklisted("game", ProcessBlacklistFeature::QuickLaunch));
        assert!(migrated.process_is_blacklisted("editor", ProcessBlacklistFeature::Clipboard));
        assert!(!migrated.process_is_blacklisted("editor", ProcessBlacklistFeature::QuickLaunch));
        AppConfig::update_process_blacklist(&path, ProcessBlacklistSettings::from_entries(vec![]))
            .expect("remove all");
        assert!(
            AppConfig::load_from(&path)
                .expect("no resurrection")
                .process_blacklist
                .entries()
                .is_empty()
        );
        json.as_object_mut()
            .expect("object")
            .remove("process_blacklist");
        json["clipboard"]["excluded_processes"] = serde_json::json!([]);
        fs::write(&path, serde_json::to_vec(&json).expect("encode absent")).expect("absent legacy");
        assert!(
            AppConfig::load_from(&path)
                .expect("explicit legacy empty")
                .process_blacklist
                .entries()
                .is_empty()
        );
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn process_blacklist_normalizes_legacy_clipboard_entries_and_merges_features() {
        let mut config = AppConfig::default();
        config.process_blacklist.entries = None;
        config.clipboard.excluded_processes = vec!["Game.exe".into(), "Editor.exe".into()];
        config.process_blacklist.disable_quick_launch = true;
        config.normalize();

        assert!(config.process_is_blacklisted("GAME", ProcessBlacklistFeature::Clipboard));
        assert!(!config.process_is_blacklisted("game.exe", ProcessBlacklistFeature::QuickLaunch));
        assert!(config.process_is_blacklisted("Editor.exe", ProcessBlacklistFeature::Clipboard));
    }

    #[test]
    fn process_blacklist_round_trips_per_feature_flags() {
        let mut config = AppConfig::default();
        config.process_blacklist.entries = None;
        config.process_blacklist.processes = vec!["Editor.exe".into(), "Game.exe".into()];
        config.process_blacklist.disable_clipboard = false;
        config.process_blacklist.disable_quick_launch = true;
        config.process_blacklist.normalize();

        let encoded = serde_json::to_string(&config).expect("serialize process blacklist");
        let restored: AppConfig =
            serde_json::from_str(&encoded).expect("deserialize process blacklist");
        assert!(!restored.process_is_blacklisted("editor.exe", ProcessBlacklistFeature::Clipboard));
        assert!(restored.process_is_blacklisted("game.exe", ProcessBlacklistFeature::QuickLaunch));
    }

    #[test]
    fn process_blacklist_matches_executable_names_case_insensitively() {
        let mut settings = ProcessBlacklistSettings {
            entries: None,
            processes: vec![
                "KeePass.exe".into(),
                "notepad".into(),
                "C:\\Tools\\App.EXE".into(),
                "Foo.Exe".into(),
            ],
            disable_clipboard: true,
            disable_quick_launch: true,
        };
        settings.normalize();

        assert_eq!(
            settings.processes,
            ["keepass", "notepad", "app", "foo"],
            "entries are stored without extension, directory or case"
        );
        for probe in [
            "KEEPASS.EXE",
            "keepass.exe",
            "KeePass",
            "Notepad.EXE",
            "c:\\x\\APP.EXE",
            "Foo.EXE",
        ] {
            assert!(
                settings.contains(probe, ProcessBlacklistFeature::Clipboard),
                "{probe} must be treated as blacklisted"
            );
        }
        assert!(!settings.contains("keepass2.exe", ProcessBlacklistFeature::Clipboard));
    }

    fn unique_config_dir(label: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "nxtools-core-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        fs::create_dir_all(&directory).expect("create config directory");
        directory
    }

    #[test]
    fn text_card_height_legacy_integer_bounds_and_roundtrip() {
        let mut legacy = serde_json::to_value(ClipboardSettings::default()).expect("settings");
        legacy
            .as_object_mut()
            .expect("object")
            .remove("text_card_max_height");
        assert_eq!(
            serde_json::from_value::<ClipboardSettings>(legacy.clone())
                .expect("legacy")
                .text_card_max_height,
            160
        );
        for (requested, expected) in [
            (0, 120),
            (119, 120),
            (120, 120),
            (160, 160),
            (210, 210),
            (320, 320),
            (u32::MAX, 320),
        ] {
            let mut settings = ClipboardSettings {
                text_card_max_height: requested,
                ..Default::default()
            };
            settings.normalize();
            assert_eq!(settings.text_card_max_height, expected);
            settings.normalize();
            assert_eq!(settings.text_card_max_height, expected);
            assert_eq!(
                serde_json::from_str::<ClipboardSettings>(
                    &serde_json::to_string(&settings).expect("encode")
                )
                .expect("decode"),
                settings
            );
        }
        for invalid in [
            serde_json::json!(160.5),
            serde_json::json!(-1),
            serde_json::json!("160"),
        ] {
            legacy["text_card_max_height"] = invalid;
            assert!(serde_json::from_value::<ClipboardSettings>(legacy.clone()).is_err());
        }
    }

    #[test]
    fn text_card_height_field_merge_and_corrupt_refusal() {
        let directory = unique_config_dir("text-card-height");
        let path = directory.join("config.json");
        AppConfig::default().save_to(&path).expect("seed");
        AppConfig::update_section(&path, |config| {
            config.clipboard.storage_directory = "synthetic-new-root".into();
            config.clipboard.image_view_mode = ClipboardImageViewMode::Mini;
            config.download.default_connection_limit = 24;
        })
        .expect("other owners");
        AppConfig::update_section(&path, |config| config.clipboard.text_card_max_height = 210)
            .expect("height only");
        let saved = AppConfig::load_from(&path).expect("reload");
        assert_eq!(saved.clipboard.text_card_max_height, 210);
        assert_eq!(saved.clipboard.storage_directory, "synthetic-new-root");
        assert_eq!(
            saved.clipboard.image_view_mode,
            ClipboardImageViewMode::Mini
        );
        assert_eq!(saved.download.default_connection_limit, 24);
        fs::write(&path, b"{broken").expect("corrupt");
        assert!(
            AppConfig::update_section(&path, |config| config.clipboard.text_card_max_height = 320)
                .is_err()
        );
        assert_eq!(fs::read(&path).expect("unchanged"), b"{broken");
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn image_modes_default_legacy_and_all_choices_roundtrip() {
        let mut legacy = serde_json::to_value(ClipboardSettings::default()).expect("settings JSON");
        legacy
            .as_object_mut()
            .expect("object")
            .remove("image_view_mode");
        assert_eq!(
            serde_json::from_value::<ClipboardSettings>(legacy)
                .expect("legacy")
                .image_view_mode,
            ClipboardImageViewMode::Auto
        );
        for mode in ClipboardImageViewMode::ALL {
            let settings = ClipboardSettings {
                image_view_mode: mode,
                ..Default::default()
            };
            assert_eq!(
                serde_json::from_str::<ClipboardSettings>(
                    &serde_json::to_string(&settings).expect("encode")
                )
                .expect("decode")
                .image_view_mode,
                mode
            );
        }
    }
    #[test]
    fn image_modes_field_merge_preserves_other_sections_and_latest_storage() {
        let directory = unique_config_dir("image-mode");
        let path = directory.join("config.json");
        AppConfig::default().save_to(&path).expect("seed");
        AppConfig::update_section(&path, |config| {
            config.clipboard.storage_directory = "synthetic-latest-root".into();
            config.download.default_connection_limit = 24;
        })
        .expect("other owners");
        AppConfig::update_section(&path, |config| {
            config.clipboard.image_view_mode = ClipboardImageViewMode::Mini
        })
        .expect("view mode only");
        let config = AppConfig::load_from(&path).expect("reload");
        assert_eq!(
            config.clipboard.image_view_mode,
            ClipboardImageViewMode::Mini
        );
        assert_eq!(config.clipboard.storage_directory, "synthetic-latest-root");
        assert_eq!(config.download.default_connection_limit, 24);
        fs::write(&path, b"{broken").expect("corrupt fixture");
        assert!(
            AppConfig::update_section(&path, |config| config.clipboard.image_view_mode =
                ClipboardImageViewMode::Fit)
            .is_err()
        );
        assert_eq!(fs::read(&path).expect("unchanged"), b"{broken");
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn saving_a_stale_cached_copy_through_update_section_keeps_other_modules_changes() {
        let directory = unique_config_dir("merge");
        let path = directory.join("config.json");
        AppConfig::default().save_to(&path).expect("seed config");

        // Quick Launch caches a full copy at startup.
        let stale_quick_launch_copy = AppConfig::load_from(&path).expect("cache at startup");

        // The user later changes download and network settings.
        let download = DownloadSettings {
            default_connection_limit: 24,
            ..Default::default()
        };
        AppConfig::update_download_settings(&path, download).expect("save download settings");
        AppConfig::update_network_settings(
            &path,
            NetworkSettings {
                proxy: NetworkProxySettings {
                    mode: NetworkProxyMode::ManualHttp,
                    host: "127.0.0.1".into(),
                    port: 7890,
                },
            },
        )
        .expect("save network settings");

        // Quick Launch now saves only its own section.
        let quick_launch = stale_quick_launch_copy.quick_launch.clone();
        AppConfig::update_quick_launch_settings(&path, quick_launch).expect("save quick launch");

        let merged = AppConfig::load_from(&path).expect("reload merged config");
        assert_eq!(merged.download.default_connection_limit, 24);
        assert_eq!(merged.network.proxy.mode, NetworkProxyMode::ManualHttp);
        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn save_to_leaves_no_temporary_file_and_never_exposes_a_torn_file() {
        let directory = unique_config_dir("atomic");
        let path = directory.join("config.json");
        AppConfig::default().save_to(&path).expect("first save");
        AppConfig::default().save_to(&path).expect("second save");

        let leftovers = fs::read_dir(&directory)
            .expect("list config directory")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(leftovers, 0, "temporary files must be renamed or removed");
        AppConfig::load_from(&path).expect("saved config parses");
        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn update_section_refuses_to_overwrite_a_corrupt_config() {
        let directory = unique_config_dir("corrupt");
        let path = directory.join("config.json");
        fs::write(&path, b"{ \"theme\": ").expect("write torn config");

        let result = AppConfig::update_system_settings(&path, SystemSettings::default());

        assert!(result.is_err(), "a corrupt file must surface as an error");
        assert_eq!(
            fs::read(&path).expect("read corrupt config"),
            b"{ \"theme\": ",
            "the corrupt file must be left untouched for recovery"
        );
        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn update_section_does_not_resurrect_a_missing_config_from_defaults() {
        let directory = unique_config_dir("missing");
        let path = directory.join("config.json");

        let result = AppConfig::update_system_settings(&path, SystemSettings::default());

        assert!(
            result.is_err(),
            "a vanished config must not reset every section"
        );
        assert!(!path.exists(), "no default config may be written");
        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn clipboard_management_defaults_match_the_reference_behavior() {
        let settings = ClipboardSettings::default();
        assert_eq!(
            settings.hotkey.modifiers,
            vec![ClipboardHotKeyModifier::Alt]
        );
        assert_eq!(settings.hotkey.key, "C");
        assert_eq!(settings.hotkey.display, "Alt+C");
        assert_eq!(settings.max_items, 1000);
        assert_eq!(settings.retention_days, 30);
        assert_eq!(
            settings.excluded_processes,
            ["KeePass.exe", "1Password.exe", "Bitwarden.exe"]
        );
        assert_eq!(
            settings.modifier_actions,
            ClipboardModifierActions {
                control: ClipboardClickAction::CopyOnly,
                alt: ClipboardClickAction::PlainText,
                shift: ClipboardClickAction::WithoutLineBreaks,
            }
        );
    }

    #[test]
    fn clipboard_capture_limits_default_and_legacy_values_are_compatible() {
        let settings = ClipboardSettings::default();
        assert_eq!(settings.max_text_bytes, 8 * 1024 * 1024);
        assert_eq!(settings.max_image_bytes, 64 * 1024 * 1024);
        assert_eq!(settings.max_image_pixels, 100_000_000);

        let restored: ClipboardSettings = serde_json::from_str(
            r#"{"enabled":true,"capture_images":true,"capture_files":true,"max_items":1000,"retention_days":30,"excluded_processes":[]}"#,
        )
        .expect("legacy clipboard settings must remain readable");
        assert_eq!(restored.max_text_bytes, settings.max_text_bytes);
        assert_eq!(restored.max_image_bytes, settings.max_image_bytes);
        assert_eq!(restored.max_image_pixels, settings.max_image_pixels);
    }

    #[test]
    fn clipboard_capture_limits_normalize_to_supported_ranges_and_round_trip() {
        let mut settings = ClipboardSettings {
            max_text_bytes: 1,
            max_image_bytes: u64::MAX,
            max_image_pixels: 0,
            ..ClipboardSettings::default()
        };
        settings.normalize();
        assert_eq!(settings.max_text_bytes, 64 * 1024);
        assert_eq!(settings.max_image_bytes, 256 * 1024 * 1024);
        assert_eq!(settings.max_image_pixels, 1_000_000);

        let json = serde_json::to_string(&settings).expect("serialize capture limits");
        let restored: ClipboardSettings =
            serde_json::from_str(&json).expect("deserialize capture limits");
        assert_eq!(restored, settings);
    }

    #[test]
    fn legacy_clipboard_settings_receive_modifier_action_defaults() {
        let settings: ClipboardSettings = serde_json::from_str(
            r#"{"enabled":true,"capture_images":true,"capture_files":true,"hotkey":{"modifiers":["alt"],"key":"C","display":"Alt+C"},"max_items":1000,"retention_days":30,"excluded_processes":[]}"#,
        )
        .expect("legacy clipboard settings must remain readable");

        assert_eq!(
            settings.modifier_actions,
            ClipboardModifierActions::default()
        );
    }

    #[test]
    fn obsolete_capture_switches_are_migrated_to_always_on() {
        let mut settings = ClipboardSettings {
            enabled: false,
            capture_images: false,
            capture_files: false,
            ..ClipboardSettings::default()
        };

        settings.normalize();

        assert!(settings.enabled);
        assert!(settings.capture_images);
        assert!(settings.capture_files);
    }

    #[test]
    fn legacy_win32_hotkey_config_migrates_to_semantic_values() {
        let hotkey: ClipboardHotKey =
            serde_json::from_str(r#"{"modifiers":6,"virtual_key":86,"display":"Ctrl+Shift+V"}"#)
                .expect("deserialize legacy hotkey");

        assert_eq!(
            hotkey.modifiers,
            vec![
                ClipboardHotKeyModifier::Control,
                ClipboardHotKeyModifier::Shift
            ]
        );
        assert_eq!(hotkey.key, "V");
        assert_eq!(hotkey.display, "Ctrl+Shift+V");
    }

    #[test]
    fn resized_clipboard_panel_size_survives_config_reload() {
        let directory = std::env::temp_dir().join(format!(
            "nxtools-panel-size-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        fs::create_dir_all(&directory).expect("create config directory");
        let paths = AppPaths {
            config_dir: directory.clone(),
            data_dir: directory.clone(),
            cache_dir: directory.clone(),
            log_dir: directory.clone(),
        };
        let config = AppConfig {
            clipboard_panel: ClipboardPanelSize::clamped(512, 688),
            ..AppConfig::default()
        };
        config
            .save_to(&paths.config_file())
            .expect("persist resized panel");

        let restored = AppConfig::load_or_create(&paths).expect("reload config");
        assert_eq!(restored.clipboard_panel.width, 512);
        assert_eq!(restored.clipboard_panel.height, 688);

        fs::remove_dir_all(directory).expect("remove config directory");
    }

    #[test]
    fn log_retention_only_deletes_matching_process_logs() {
        let directory = std::env::temp_dir().join(format!(
            "nxtools-log-retention-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        fs::create_dir_all(&directory).expect("create test directory");
        let desktop_log = directory.join("desktop.log.2000-01-01");
        let other_log = directory.join("indexer.log.2000-01-01");
        fs::write(&desktop_log, b"old").expect("write desktop log");
        fs::write(&other_log, b"old").expect("write other log");

        delete_expired_logs(&directory, "desktop", Duration::ZERO);

        assert!(!desktop_log.exists());
        assert!(other_log.exists());
        fs::remove_dir_all(directory).expect("remove test directory");
    }
}

#[cfg(test)]
mod installed_feedback_config_tests {
    use super::*;

    #[test]
    fn legacy_theme_and_background_limit_default_and_normalize() {
        let mut value = serde_json::to_value(AppConfig::default()).expect("config JSON");
        value["system"]
            .as_object_mut()
            .expect("system object")
            .remove("theme");
        value["download"]
            .as_object_mut()
            .expect("download object")
            .remove("background_speed_limit");
        let mut config: AppConfig = serde_json::from_value(value).expect("legacy configuration");
        assert_eq!(config.system.theme, ThemePreference::Dark);
        assert!(!config.download.background_speed_limit.enabled);
        assert_eq!(
            config.download.background_speed_limit.bytes_per_second,
            1024 * 1024
        );
        config.download.background_speed_limit.bytes_per_second = 0;
        config.download.normalize();
        assert_eq!(config.download.background_speed_limit.bytes_per_second, 1);
    }

    #[test]
    fn theme_and_background_policy_section_updates_preserve_each_other() {
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("test clock")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "nxtools-feedback-config-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).expect("isolated directory");
        let path = directory.join("config.json");
        AppConfig::default()
            .save_to(&path)
            .expect("initial configuration");
        let stale = AppConfig::load_from(&path).expect("stale cache");
        let mut system = stale.system.clone();
        system.theme = ThemePreference::Light;
        AppConfig::update_system_settings(&path, system).expect("persist theme");
        let mut download = stale.download;
        download.background_speed_limit = BackgroundDownloadLimitSettings {
            enabled: true,
            bytes_per_second: 2048,
        };
        let merged =
            AppConfig::update_download_settings(&path, download).expect("persist download policy");
        assert_eq!(merged.system.theme, ThemePreference::Light);
        assert!(merged.download.background_speed_limit.enabled);
        let mut system = merged.system;
        system.theme = ThemePreference::System;
        let merged = AppConfig::update_system_settings(&path, system).expect("persist System mode");
        assert_eq!(
            merged.download.background_speed_limit.bytes_per_second,
            2048
        );
        assert_eq!(
            AppConfig::load_from(&path).expect("reopen").system.theme,
            ThemePreference::System
        );
        std::fs::remove_dir_all(directory).expect("clean isolated directory");
    }
}

#[cfg(test)]
mod notification_position_tests {
    use super::*;
    #[test]
    fn six_notification_positions_round_trip_and_legacy_defaults_to_bottom_right() {
        let legacy: SystemSettings = serde_json::from_str("{}").expect("legacy system settings");
        assert_eq!(
            legacy.notification_position,
            NotificationPosition::BottomRight
        );
        for position in NotificationPosition::ALL {
            let settings = SystemSettings {
                notification_position: position,
                ..Default::default()
            };
            let encoded = serde_json::to_vec(&settings).expect("encode position");
            assert_eq!(
                serde_json::from_slice::<SystemSettings>(&encoded)
                    .expect("decode position")
                    .notification_position,
                position
            );
        }
    }
}

#[cfg(test)]
mod notification_duration_tests {
    use super::*;
    #[test]
    fn dwell_duration_legacy_default_and_bounded_roundtrip() {
        assert_eq!(
            serde_json::from_str::<SystemSettings>("{}")
                .expect("legacy")
                .notification_duration_seconds,
            6
        );
        for (raw, expected) in [(0, 1), (1, 1), (15, 15), (60, 60), (999, 60)] {
            let settings: SystemSettings =
                serde_json::from_str(&format!("{{\"notification_duration_seconds\":{raw}}}"))
                    .expect("duration");
            assert_eq!(settings.notification_duration_seconds, expected);
            assert_eq!(
                serde_json::from_slice::<SystemSettings>(
                    &serde_json::to_vec(&settings).expect("encode")
                )
                .expect("decode"),
                settings
            );
        }
    }
}

#[cfg(test)]
mod currency_settings_tests {
    use super::*;
    #[test]
    fn currency_tool_settings_legacy_default_and_section_merge_keep_other_fields() {
        let mut value = serde_json::to_value(AppConfig::default()).expect("fixture");
        value["clipboard"]
            .as_object_mut()
            .expect("section")
            .remove("currency_conversion");
        let legacy: AppConfig = serde_json::from_value(value).expect("legacy");
        assert_eq!(
            legacy.clipboard.currency_conversion,
            CurrencyConversionSettings::default()
        );
        let dir = std::env::temp_dir().join(format!(
            "nxtools-currency-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("fixture directory");
        let path = dir.join("config.json");
        legacy.save_to(&path).expect("seed");
        AppConfig::update_section(&path, |config| {
            config.system.close_to_tray = false;
            config.clipboard.storage_directory = "owned-root".into();
        })
        .expect("other owner");
        AppConfig::update_section(&path, |config| {
            config.clipboard.currency_conversion = CurrencyConversionSettings {
                enabled: true,
                target: " cad ".into(),
            };
        })
        .expect("currency save");
        let saved = AppConfig::load_from(&path).expect("reopen");
        assert_eq!(saved.clipboard.currency_conversion.target, "CAD");
        assert!(saved.clipboard.currency_conversion.enabled);
        assert!(!saved.system.close_to_tray);
        assert_eq!(saved.clipboard.storage_directory, "owned-root");
        fs::remove_dir_all(dir).expect("cleanup currency fixture");
    }
}

#[cfg(test)]
mod currency_catalog_settings_tests {
    use super::*;
    #[test]
    fn currency_catalog_dynamic_codes_survive_normalization_and_roundtrip() {
        for code in [" usdt ", "1inch", "XAU", "newcoin", "KRW"] {
            let mut config = AppConfig::default();
            config.clipboard.currency_conversion.target = code.into();
            config.normalize();
            assert_eq!(
                config.clipboard.currency_conversion.target,
                code.trim().to_ascii_uppercase()
            );
            let encoded = serde_json::to_vec(&config).expect("encode");
            let mut saved: AppConfig = serde_json::from_slice(&encoded).expect("roundtrip");
            saved.normalize();
            assert_eq!(
                saved.clipboard.currency_conversion.target,
                config.clipboard.currency_conversion.target
            );
        }
        for code in ["x", "123", "bad code", "../USD", "💵"] {
            let mut config = AppConfig::default();
            config.clipboard.currency_conversion.target = code.into();
            config.normalize();
            assert_eq!(config.clipboard.currency_conversion.target, "CNY");
        }
    }
}

#[cfg(test)]
mod presentation_settings_tests {
    use super::*;
    #[test]
    fn row_spacing_default_legacy_and_both_styles_roundtrip() {
        let mut legacy = serde_json::to_value(QuickLaunchSettings::default()).expect("settings");
        legacy.as_object_mut().expect("object").remove("row_style");
        assert_eq!(
            serde_json::from_value::<QuickLaunchSettings>(legacy)
                .expect("legacy")
                .row_style,
            QuickLaunchRowStyle::Separated
        );
        for style in QuickLaunchRowStyle::ALL {
            let settings = QuickLaunchSettings {
                row_style: style,
                ..Default::default()
            };
            assert_eq!(
                serde_json::from_str::<QuickLaunchSettings>(
                    &serde_json::to_string(&settings).expect("encode")
                )
                .expect("decode")
                .row_style,
                style
            );
        }
    }
}

#[cfg(test)]
mod chrome_store_identity_tests {
    use super::*;

    #[test]
    fn chrome_store_optional_identity_legacy_validation_and_roundtrip() {
        let mut legacy = serde_json::to_value(DownloadSettings::default()).expect("settings");
        legacy
            .as_object_mut()
            .expect("object")
            .remove("chrome_web_store_extension_id");
        assert!(
            serde_json::from_value::<DownloadSettings>(legacy)
                .expect("legacy settings")
                .chrome_web_store_extension_id
                .is_empty()
        );
        for invalid in [
            "*",
            "chrome-extension://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/",
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaq",
            "abc",
        ] {
            assert!(!is_valid_chrome_web_store_extension_id(invalid));
        }
        let mut settings = DownloadSettings {
            chrome_web_store_extension_id: " aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa ".into(),
            ..Default::default()
        };
        settings.normalize();
        assert_eq!(
            settings.chrome_web_store_extension_id,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        );
        assert_eq!(
            serde_json::from_str::<DownloadSettings>(
                &serde_json::to_string(&settings).expect("encode")
            )
            .expect("decode"),
            settings
        );
        settings.chrome_web_store_extension_id = "*".into();
        settings.normalize();
        assert!(settings.chrome_web_store_extension_id.is_empty());
    }
}
