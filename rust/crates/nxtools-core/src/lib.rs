use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use anyhow::{Context, Result, bail};
use directories::ProjectDirs;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub log_dir: PathBuf,
}

impl AppPaths {
    pub fn discover() -> Result<Self> {
        let dirs = ProjectDirs::from("dev", "NxTools", "NxTools.Next")
            .context("Windows did not provide application data directories")?;
        let paths = Self {
            config_dir: dirs.config_dir().to_path_buf(),
            data_dir: dirs.data_dir().to_path_buf(),
            cache_dir: dirs.cache_dir().to_path_buf(),
            log_dir: dirs.data_local_dir().join("logs"),
        };
        paths.ensure_created()?;
        Ok(paths)
    }

    fn ensure_created(&self) -> Result<()> {
        for path in [
            &self.config_dir,
            &self.data_dir,
            &self.cache_dir,
            &self.log_dir,
        ] {
            fs::create_dir_all(path)
                .with_context(|| format!("failed to create {}", path.display()))?;
        }
        Ok(())
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
    pub download: DownloadSettings,
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
            download: DownloadSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemSettings {
    #[serde(default)]
    pub start_on_login_silently: bool,
    #[serde(default = "default_close_to_tray")]
    pub close_to_tray: bool,
}

impl Default for SystemSettings {
    fn default() -> Self {
        Self {
            start_on_login_silently: false,
            close_to_tray: default_close_to_tray(),
        }
    }
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
    #[serde(default)]
    pub automatic_categories_enabled: bool,
    #[serde(default = "default_download_categories")]
    pub categories: Vec<DownloadCategorySettings>,
    #[serde(default)]
    pub duplicate_strategy: DownloadDuplicateStrategySettings,
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
    #[serde(default)]
    pub taskbar_progress_overlay: DownloadTaskbarProgressOverlaySettings,
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
            automatic_categories_enabled: false,
            categories: default_download_categories(),
            duplicate_strategy: DownloadDuplicateStrategySettings::AddNumbered,
            double_click_action: DownloadRowDoubleClickActionSettings::OpenFile,
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
            taskbar_progress_overlay: DownloadTaskbarProgressOverlaySettings::default(),
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuickLaunchSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub auto_quick_switch: bool,
    #[serde(default = "default_true")]
    pub file_dialog_search: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClipboardSettings {
    pub enabled: bool,
    pub capture_images: bool,
    pub capture_files: bool,
    #[serde(default)]
    pub hotkey: ClipboardHotKey,
    #[serde(default = "default_clipboard_max_items")]
    pub max_items: usize,
    #[serde(default = "default_clipboard_retention_days")]
    pub retention_days: u32,
    #[serde(default = "default_clipboard_excluded_processes")]
    pub excluded_processes: Vec<String>,
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
            hotkey: ClipboardHotKey::default(),
            max_items: default_clipboard_max_items(),
            retention_days: default_clipboard_retention_days(),
            excluded_processes: default_clipboard_excluded_processes(),
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
        .trim_end_matches(".exe")
        .to_ascii_lowercase();
    if matches!(
        process_name.as_str(),
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

fn default_clipboard_max_items() -> usize {
    1000
}

fn default_clipboard_retention_days() -> u32 {
    30
}

fn default_clipboard_excluded_processes() -> Vec<String> {
    vec![
        "KeePass.exe".into(),
        "1Password.exe".into(),
        "Bitwarden.exe".into(),
    ]
}

impl ClipboardSettings {
    pub fn normalize(&mut self) {
        // Capture is a single product capability now. Keep these legacy fields on the wire so
        // older configs remain readable, but do not leave users stuck with hidden disabled flags.
        self.enabled = true;
        self.capture_images = true;
        self.capture_files = true;
        self.max_items = self.max_items.clamp(100, 100_000);
        self.retention_days = self.retention_days.clamp(1, 3650);
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
        Ok(config)
    }

    pub fn load_or_create(paths: &AppPaths) -> Result<Self> {
        let path = paths.config_file();
        if path.exists() {
            Self::load_from(&path)
        } else {
            let config = Self::default();
            fs::write(&path, serde_json::to_string_pretty(&config)?)
                .with_context(|| format!("failed to write {}", path.display()))?;
            Ok(config)
        }
    }

    fn normalize(&mut self) {
        self.network.normalize();
        self.clipboard_panel =
            ClipboardPanelSize::clamped(self.clipboard_panel.width, self.clipboard_panel.height);
        self.clipboard.normalize();
        self.quick_launch.normalize();
        self.download.normalize();
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        fs::write(path, serde_json::to_string_pretty(self)?)
            .with_context(|| format!("failed to write {}", path.display()))
    }

    pub fn update_download_settings(path: &Path, mut settings: DownloadSettings) -> Result<Self> {
        settings.normalize();
        let mut config = Self::load_from(path)?;
        config.download = settings;
        config.save_to(path)?;
        Ok(config)
    }

    pub fn update_network_settings(path: &Path, mut settings: NetworkSettings) -> Result<Self> {
        settings.normalize();
        settings.validate().map_err(anyhow::Error::msg)?;
        let mut config = Self::load_from(path)?;
        config.network = settings;
        config.save_to(path)?;
        Ok(config)
    }

    pub fn update_system_settings(path: &Path, settings: SystemSettings) -> Result<Self> {
        let mut config = Self::load_from(path)?;
        config.system = settings;
        config.save_to(path)?;
        Ok(config)
    }

    pub fn update_global_settings(
        path: &Path,
        mut network: NetworkSettings,
        system: SystemSettings,
    ) -> Result<Self> {
        network.normalize();
        network.validate().map_err(anyhow::Error::msg)?;
        let mut config = Self::load_from(path)?;
        config.network = network;
        config.system = system;
        config.save_to(path)?;
        Ok(config)
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

impl AppPaths {
    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join("config.json")
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreference {
    Dark,
    Light,
    System,
}

pub struct SingleInstanceGuard {
    _file: File,
}

impl SingleInstanceGuard {
    pub fn acquire(paths: &AppPaths, process_name: &str) -> Result<Self> {
        Self::acquire_with_timeout(paths, process_name, Duration::ZERO)
    }

    /// Persists a fresh hint file for this instance. Callers update the
    /// window handle after the main window exists; a second launch reads it
    /// to summon the running instance instead of failing.
    pub fn capture_hints(paths: &AppPaths, process_name: &str) {
        let hint_path = paths.cache_dir.join(format!("{process_name}.hints"));
        let _ = std::fs::write(hint_path, std::process::id().to_string());
    }

    /// Reads the running instance's main-window handle recorded by the
    /// desktop at startup. Returns 0 when no valid hint exists.
    pub fn read_main_window_handle(paths: &AppPaths, process_name: &str) -> i64 {
        let hint_path = paths.cache_dir.join(format!("{process_name}.hints"));
        std::fs::read_to_string(hint_path)
            .ok()
            .and_then(|text| text.trim().parse::<i64>().ok())
            .unwrap_or(0)
    }

    /// Records the main window handle into the hint file.
    pub fn record_main_window_handle(paths: &AppPaths, process_name: &str, handle: i64) {
        let hint_path = paths.cache_dir.join(format!("{process_name}.hints"));
        let _ = std::fs::write(hint_path, handle.to_string());
    }

    pub fn acquire_with_timeout(
        paths: &AppPaths,
        process_name: &str,
        timeout: Duration,
    ) -> Result<Self> {
        let path = paths.cache_dir.join(format!("{process_name}.lock"));
        let deadline = Instant::now() + timeout;
        loop {
            let file = OpenOptions::new()
                .create(true)
                .truncate(false)
                .read(true)
                .write(true)
                .open(&path)
                .with_context(|| format!("failed to open {}", path.display()))?;

            if file.try_lock_exclusive().is_ok() {
                return Ok(Self { _file: file });
            }
            if Instant::now() >= deadline {
                bail!("another {process_name} instance is already running");
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

pub type LogGuard = tracing_appender::non_blocking::WorkerGuard;

pub fn init_file_logging(paths: &AppPaths, process_name: &str) -> Result<LogGuard> {
    delete_expired_logs(
        &paths.log_dir,
        process_name,
        Duration::from_secs(14 * 24 * 60 * 60),
    );
    let appender = tracing_appender::rolling::daily(&paths.log_dir, format!("{process_name}.log"));
    let (writer, guard) = tracing_appender::non_blocking(appender);
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_ansi(false)
        .with_writer(writer)
        .try_init()
        .map_err(|error| anyhow::anyhow!("failed to initialize logging: {error}"))?;
    Ok(guard)
}

fn delete_expired_logs(log_dir: &Path, process_name: &str, retention: Duration) {
    let prefix = format!("{process_name}.log.");
    let Some(cutoff) = SystemTime::now().checked_sub(retention) else {
        return;
    };
    let Ok(entries) = fs::read_dir(log_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_process_log = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(&prefix));
        if !is_process_log {
            continue;
        }
        let is_expired = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .is_ok_and(|modified| modified < cutoff);
        if is_expired {
            let _ = fs::remove_file(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(config.quick_launch.hotkey.display, "双击 Ctrl");
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
                automatic_categories_enabled: true,
                categories: default_download_categories(),
                duplicate_strategy: DownloadDuplicateStrategySettings::AddNumbered,
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
                taskbar_progress_overlay: DownloadTaskbarProgressOverlaySettings::default(),
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
