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
            clipboard_panel: ClipboardPanelSize::default(),
            clipboard: ClipboardSettings::default(),
            quick_launch: QuickLaunchSettings::default(),
            download: DownloadSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DownloadSettings {
    #[serde(default)]
    pub default_directory: String,
    #[serde(default = "default_download_connection_limit")]
    pub default_connection_limit: u16,
    #[serde(default = "default_true")]
    pub show_progress_window: bool,
    #[serde(default = "default_true")]
    pub show_completion_window: bool,
    #[serde(default)]
    pub taskbar_progress_overlay: DownloadTaskbarProgressOverlaySettings,
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
}

impl Default for DownloadTaskbarProgressOverlaySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            anchor: DownloadTaskbarOverlayAnchor::Right,
            offset_x: default_taskbar_overlay_offset_x(),
            offset_y: 0,
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

impl Default for DownloadSettings {
    fn default() -> Self {
        Self {
            default_directory: String::new(),
            default_connection_limit: default_download_connection_limit(),
            show_progress_window: true,
            show_completion_window: true,
            taskbar_progress_overlay: DownloadTaskbarProgressOverlaySettings::default(),
        }
    }
}

const fn default_download_connection_limit() -> u16 {
    8
}

const fn default_taskbar_overlay_offset_x() -> i32 {
    12
}

impl DownloadSettings {
    pub fn normalize(&mut self) {
        self.default_directory = self.default_directory.trim().to_owned();
        self.default_connection_limit = self.default_connection_limit.clamp(1, 64);
        self.taskbar_progress_overlay.offset_x =
            self.taskbar_progress_overlay.offset_x.clamp(-4096, 4096);
        self.taskbar_progress_overlay.offset_y =
            self.taskbar_progress_overlay.offset_y.clamp(-4096, 4096);
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
        assert_eq!(settings.default_connection_limit, 8);
        assert!(settings.show_progress_window);
        assert!(settings.show_completion_window);
        assert!(settings.taskbar_progress_overlay.enabled);
        assert_eq!(
            settings.taskbar_progress_overlay.anchor,
            DownloadTaskbarOverlayAnchor::Right
        );
        assert_eq!(settings.taskbar_progress_overlay.offset_x, 12);
        assert_eq!(settings.taskbar_progress_overlay.offset_y, 0);
    }

    #[test]
    fn legacy_download_settings_receive_new_defaults() {
        let config: AppConfig = serde_json::from_str(
            r#"{"theme":"dark","platform_pipe_name":"legacy-pipe","download":{"show_completion_window":false}}"#,
        )
        .expect("legacy download settings must remain readable");

        assert_eq!(config.download.default_connection_limit, 8);
        assert!(config.download.show_progress_window);
        assert!(!config.download.show_completion_window);
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
                default_connection_limit: 12,
                show_progress_window: false,
                show_completion_window: false,
                taskbar_progress_overlay: DownloadTaskbarProgressOverlaySettings::default(),
            },
        )
        .expect("update only download settings");

        assert!(!updated.quick_launch.enabled);
        assert_eq!(updated.clipboard.retention_days, 91);
        assert_eq!(updated.download.default_connection_limit, 12);
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
