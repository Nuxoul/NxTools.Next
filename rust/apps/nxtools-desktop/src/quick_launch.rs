use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::File;
use std::io::Read;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use chrono::{DateTime, Local};
use gpui::{
    Animation, AnimationExt, AnyElement, AnyWindowHandle, App, Bounds, Context, Div, Entity,
    EntityInputHandler, EventEmitter, Focusable, HighlightStyle, Image, ImageFormat, ImageSource,
    IntoElement, ObjectFit, Pixels, Render, ScrollHandle, ScrollStrategy, Size, StyledText,
    Subscription, UniformListScrollHandle, WeakEntity, Window, WindowBackgroundAppearance,
    WindowBounds, WindowDecorations, WindowKind, WindowOptions, div, ease_out_quint, img, point,
    prelude::*, px, size, uniform_list,
};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::scroll::ScrollableElement;
use gpui_component::{PixelsExt, Root};
use nxtools_core::{AppConfig, ClipboardHotKey, ClipboardHotKeyModifier, QuickLaunchSettings};
#[cfg(test)]
use nxtools_finder::query_requests_file_index;
use nxtools_finder::{
    ApplicationCatalogSnapshot, ApplicationSource, CatalogStatus, ExternalSearchEntry,
    FinderCatalog, QuickLaunchHistorySummary, RecentDirectory, SearchResult, SearchResultAction,
    query_requests_file_index_with_commands, scan_application_roots,
};
use nxtools_ipc::{PlatformBridge, PlatformBridgeError, PlatformNotification};
use nxtools_protocol::{
    ActivateWindowCommand, ConfigureQuickLaunchCommand, DirectoryNavigationHost,
    ElevatedDirectoryHelperRequest, ExtractShellIconCommand, IndexSearchBatch, IndexSearchContext,
    IndexSearchItem, IndexSearchRequest, IndexSettings, IndexStatus, IndexerCommand, IndexerEvent,
    InvokeShellActionCommand, LaunchProcessCommand, NavigateDirectoryCommand,
    OpenShellTargetCommand, PasteTextCommand, PickDirectoryCommand, PickProgramCommand,
    PlatformCommand, PlatformCommandResult, PlatformEvent, QueryShellActionsCommand,
    QueryShellSubmenuCommand, QuickLaunchPanelRequest, SetQuickLaunchHotKeyRecordingCommand,
    SetWindowBoundsCommand, SetWindowNoActivateCommand, SetWindowPositionCommand,
    SetWindowTopmostCommand, SetWindowVisibilityCommand, ShellActionItem, ShellActionMenu,
    ShellOpenAction,
};
use tracing::{error, info, warn};

use crate::icons::{MaterialIcon, shell_icon_path_is_renderable};
use crate::notifications::NotificationCenter;
use crate::runtime::{IndexerBridge, IndexerNotification};
use crate::theme::{
    COMPACT_RADIUS, META_TEXT_SIZE, ROW_HORIZONTAL_PADDING, SURFACE_INSET, Theme, UI_FONT_FAMILY,
    UI_MONO_FONT_FAMILY,
};
use crate::ui::components::state::{empty_state, progress_state};
use crate::ui::metrics::{
    COMPACT_RESULT_ICON_RAIL_WIDTH, COMPACT_RESULT_ROW_HEIGHT, COMPACT_RESULT_TRAILING_RAIL_WIDTH,
    SPACING_2, SPACING_8, SPACING_12,
};
use crate::ui::typography::{LINE_HEIGHT_BODY, LINE_HEIGHT_LABEL, TEXT_BODY, TEXT_LABEL};
use crate::ui::{
    PREVIEW_APPEAR_DURATION, native_window_handle, preview_media_surface, preview_shell,
};

mod file_dialog_search;
use file_dialog_search::FileDialogSearchSession;

const PANEL_WIDTH: f32 = 620.0;
const PANEL_HEIGHT: f32 = 410.0;
const SEARCH_HEADER_HEIGHT: f32 = 42.0;
const COLLAPSED_PANEL_HEIGHT: f32 = SEARCH_HEADER_HEIGHT + 2.0;
const QUICK_LAUNCH_SEARCH_TEXT_SIZE: f32 = TEXT_BODY;
const RESULT_LIMIT: usize = 200;
const RECENT_DIRECTORY_LIMIT: usize = 64;
const INDEX_CANDIDATE_LIMIT: usize = 200;
const ICON_PREFETCH_LIMIT: usize = 10;
const ICON_VISIBLE_OVERSCAN: usize = 2;
const ICON_RESOURCE_CACHE_LIMIT: usize = 1_024;
const ICON_RETRY_DELAY: Duration = Duration::from_millis(500);
const REQUEST_MAX_AGE_MS: i64 = 1_000;
const AUTO_QUICK_SWITCH_REQUEST_MAX_AGE_MS: i64 = 3_000;
const OPEN_GRACE: Duration = Duration::from_millis(180);
const FAILED_NAVIGATION_FOREGROUND_GUARD: Duration = Duration::from_millis(750);
const RESULTS_APPEAR_DURATION: Duration = Duration::from_millis(80);
const SEARCH_INPUT_DEBOUNCE: Duration = Duration::from_millis(65);
const QUICK_LAUNCH_PREVIEW_HOVER_DELAY: Duration = Duration::from_millis(420);
const QUICK_LAUNCH_PREVIEW_SWITCH_DELAY: Duration = Duration::from_millis(110);
const QUICK_LAUNCH_PREVIEW_LEAVE_DELAY: Duration = Duration::from_millis(140);
const QUICK_LAUNCH_PREVIEW_DISAPPEAR_DURATION: Duration = Duration::from_millis(90);
const QUICK_LAUNCH_EMPTY_RESULTS_LABEL: &str = "没有匹配项";
const QUICK_LAUNCH_ACTION_CONTEXT_HEIGHT: f32 = 24.0;
const QUICK_LAUNCH_RESULT_SHORTCUT_HINTS: [&str; 9] = [
    "Ctrl+1", "Ctrl+2", "Ctrl+3", "Ctrl+4", "Ctrl+5", "Ctrl+6", "Ctrl+7", "Ctrl+8", "Ctrl+9",
];
const QUICK_LAUNCH_PREVIEW_WIDTH: f32 = 400.0;
const QUICK_LAUNCH_PREVIEW_HEIGHT: f32 = 410.0;
const QUICK_LAUNCH_PREVIEW_HEADER_HEIGHT: f32 = 72.0;
const QUICK_LAUNCH_PREVIEW_FOOTER_HEIGHT: f32 = 30.0;
const QUICK_LAUNCH_PREVIEW_GAP: f32 = 10.0;
const QUICK_LAUNCH_TEXT_PREVIEW_LIMIT: usize = 96 * 1024;
const QUICK_LAUNCH_IMAGE_FILE_LIMIT: u64 = 24 * 1024 * 1024;
const QUICK_LAUNCH_IMAGE_PIXEL_LIMIT: u64 = 16_000_000;
const QUICK_LAUNCH_PREVIEW_STAGING_COORDINATE: i32 = -32_000;

#[derive(Default)]
struct IconResourceCache {
    capacity: usize,
    entries: HashMap<String, PathBuf>,
    recency: VecDeque<String>,
}

impl IconResourceCache {
    fn bounded(capacity: usize) -> Self {
        Self {
            capacity,
            ..Self::default()
        }
    }

    fn get(&mut self, key: &str) -> Option<PathBuf> {
        let path = self.entries.get(key)?.clone();
        self.touch(key);
        Some(path)
    }

    fn insert(&mut self, key: String, path: PathBuf) {
        if self.capacity == 0 {
            return;
        }
        self.entries.insert(key.clone(), path);
        self.touch(&key);
        while self.entries.len() > self.capacity {
            let Some(stale) = self.recency.pop_front() else {
                break;
            };
            self.entries.remove(&stale);
        }
    }

    fn touch(&mut self, key: &str) {
        if let Some(position) = self.recency.iter().position(|candidate| candidate == key) {
            self.recency.remove(position);
        }
        self.recency.push_back(key.to_owned());
    }
}

struct PendingIconResource {
    resource_key: String,
    entry_ids: HashSet<String>,
}

fn icon_retry_pending(failed_at: Instant, now: Instant) -> bool {
    now.saturating_duration_since(failed_at) < ICON_RETRY_DELAY
}

fn icon_resource_key(result: &SearchResult, size: u32) -> String {
    match result.entry.source {
        ApplicationSource::IndexedDirectory => format!("{size}:type:folder"),
        ApplicationSource::IndexedFile => {
            let extension = PathBuf::from(&result.entry.target)
                .extension()
                .and_then(|extension| extension.to_str())
                .map(str::to_ascii_lowercase)
                .unwrap_or_default();
            if !extension.is_empty() && !matches!(extension.as_str(), "exe" | "lnk" | "ico") {
                return format!("{size}:type:.{extension}");
            }
            format!(
                "{size}:target:{}",
                result.entry.target.trim().replace('/', "\\").to_lowercase()
            )
        }
        _ => format!(
            "{size}:target:{}",
            result.entry.target.trim().replace('/', "\\").to_lowercase()
        ),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QuickLaunchPreviewKind {
    Image,
    Text,
    Metadata,
}

#[derive(Clone)]
enum QuickLaunchPreviewBody {
    Image(Arc<Image>),
    Text { text: String, truncated: bool },
    Metadata { icon: MaterialIcon },
}

#[derive(Clone)]
struct QuickLaunchPreviewPayload {
    entry_id: String,
    name: String,
    target: String,
    detail: String,
    created_at: Option<String>,
    modified_at: Option<String>,
    body: QuickLaunchPreviewBody,
}

fn quick_launch_preview_image_asset(payload: &QuickLaunchPreviewPayload) -> Option<Arc<Image>> {
    match &payload.body {
        QuickLaunchPreviewBody::Image(image) => Some(Arc::clone(image)),
        QuickLaunchPreviewBody::Text { .. } | QuickLaunchPreviewBody::Metadata { .. } => None,
    }
}

fn quick_launch_preview_height(_payload: &QuickLaunchPreviewPayload) -> f32 {
    QUICK_LAUNCH_PREVIEW_HEIGHT
}

fn quick_launch_preview_kind(path: &Path) -> Option<QuickLaunchPreviewKind> {
    if !path.exists() {
        return None;
    }
    if path.is_dir() {
        return Some(QuickLaunchPreviewKind::Metadata);
    }
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    if matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "bmp") {
        return Some(QuickLaunchPreviewKind::Image);
    }
    if matches!(
        extension.as_str(),
        "txt"
            | "md"
            | "markdown"
            | "json"
            | "jsonc"
            | "xml"
            | "yaml"
            | "yml"
            | "toml"
            | "ini"
            | "cfg"
            | "conf"
            | "log"
            | "csv"
            | "rs"
            | "cs"
            | "c"
            | "h"
            | "cpp"
            | "hpp"
            | "py"
            | "js"
            | "jsx"
            | "ts"
            | "tsx"
            | "html"
            | "htm"
            | "css"
            | "scss"
            | "sql"
            | "ps1"
            | "bat"
            | "cmd"
            | "sh"
    ) {
        return Some(QuickLaunchPreviewKind::Text);
    }
    Some(QuickLaunchPreviewKind::Metadata)
}

fn load_quick_launch_preview(result: &SearchResult) -> Option<QuickLaunchPreviewPayload> {
    let path = PathBuf::from(result.entry.target.trim());
    let metadata = match std::fs::metadata(&path) {
        Ok(metadata) => metadata,
        Err(_) => {
            return Some(QuickLaunchPreviewPayload {
                entry_id: result.entry.id.clone(),
                name: result.entry.name.clone(),
                target: result.entry.target.clone(),
                detail: "目标当前不可访问，仅显示索引信息".into(),
                created_at: None,
                modified_at: None,
                body: QuickLaunchPreviewBody::Metadata {
                    icon: if result.entry.source == ApplicationSource::IndexedDirectory {
                        MaterialIcon::Folder
                    } else {
                        MaterialIcon::Info
                    },
                },
            });
        }
    };
    let kind = quick_launch_preview_kind(&path).unwrap_or(QuickLaunchPreviewKind::Metadata);
    let created_at = metadata.created().ok().map(format_preview_timestamp);
    let modified_at = metadata.modified().ok().map(format_preview_timestamp);
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_uppercase)
        .filter(|extension| !extension.is_empty());
    let mut detail = if metadata.is_dir() {
        "文件夹".to_owned()
    } else {
        let kind_label = extension
            .map(|extension| format!("{extension} 文件"))
            .unwrap_or_else(|| "文件".into());
        format!("{kind_label}  ·  {}", format_preview_bytes(metadata.len()))
    };
    let body = match kind {
        QuickLaunchPreviewKind::Image => {
            if metadata.len() > QUICK_LAUNCH_IMAGE_FILE_LIMIT {
                detail.push_str("  ·  图片过大，仅显示信息");
                QuickLaunchPreviewBody::Metadata {
                    icon: MaterialIcon::Image,
                }
            } else {
                let mut bytes = Vec::with_capacity(
                    metadata
                        .len()
                        .min(1024 * 1024)
                        .try_into()
                        .unwrap_or_default(),
                );
                let read = File::open(&path).and_then(|file| {
                    file.take(QUICK_LAUNCH_IMAGE_FILE_LIMIT + 1)
                        .read_to_end(&mut bytes)
                });
                let format = quick_launch_image_format(&path);
                match (read, format, imagesize::blob_size(&bytes)) {
                    (Ok(_), Some(format), Ok(size))
                        if quick_launch_image_within_budget(
                            bytes.len() as u64,
                            size.width,
                            size.height,
                        ) =>
                    {
                        detail.push_str(&format!("  ·  {} × {}", size.width, size.height));
                        QuickLaunchPreviewBody::Image(Arc::new(Image::from_bytes(format, bytes)))
                    }
                    (Ok(_), Some(_), Ok(_))
                        if bytes.len() as u64 > QUICK_LAUNCH_IMAGE_FILE_LIMIT =>
                    {
                        detail.push_str("  ·  图片过大，仅显示信息");
                        QuickLaunchPreviewBody::Metadata {
                            icon: MaterialIcon::Image,
                        }
                    }
                    (Ok(_), Some(_), Ok(_)) => {
                        detail.push_str("  ·  图片尺寸过大，仅显示信息");
                        QuickLaunchPreviewBody::Metadata {
                            icon: MaterialIcon::Image,
                        }
                    }
                    _ => {
                        detail.push_str("  ·  无法读取图片内容");
                        QuickLaunchPreviewBody::Metadata {
                            icon: MaterialIcon::Image,
                        }
                    }
                }
            }
        }
        QuickLaunchPreviewKind::Text => {
            let mut bytes = Vec::with_capacity(QUICK_LAUNCH_TEXT_PREVIEW_LIMIT + 1);
            let read = File::open(&path).and_then(|mut file| {
                file.by_ref()
                    .take((QUICK_LAUNCH_TEXT_PREVIEW_LIMIT + 1) as u64)
                    .read_to_end(&mut bytes)
            });
            match read {
                Ok(_) => {
                    let truncated = metadata.len() > QUICK_LAUNCH_TEXT_PREVIEW_LIMIT as u64
                        || bytes.len() > QUICK_LAUNCH_TEXT_PREVIEW_LIMIT;
                    bytes.truncate(QUICK_LAUNCH_TEXT_PREVIEW_LIMIT);
                    if let Some(text) = decode_preview_text(&bytes) {
                        QuickLaunchPreviewBody::Text { text, truncated }
                    } else {
                        QuickLaunchPreviewBody::Metadata {
                            icon: MaterialIcon::Info,
                        }
                    }
                }
                Err(_) => {
                    detail.push_str("  ·  内容当前不可读取");
                    QuickLaunchPreviewBody::Metadata {
                        icon: MaterialIcon::Info,
                    }
                }
            }
        }
        QuickLaunchPreviewKind::Metadata => QuickLaunchPreviewBody::Metadata {
            icon: if metadata.is_dir() {
                MaterialIcon::Folder
            } else {
                MaterialIcon::Info
            },
        },
    };
    Some(QuickLaunchPreviewPayload {
        entry_id: result.entry.id.clone(),
        name: result.entry.name.clone(),
        target: result.entry.target.clone(),
        detail,
        created_at,
        modified_at,
        body,
    })
}

fn format_preview_timestamp(timestamp: SystemTime) -> String {
    DateTime::<Local>::from(timestamp)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

fn quick_launch_image_format(path: &Path) -> Option<ImageFormat> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())?
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => Some(ImageFormat::Png),
        "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
        "bmp" => Some(ImageFormat::Bmp),
        _ => None,
    }
}

fn quick_launch_image_within_budget(file_len: u64, width: usize, height: usize) -> bool {
    if file_len > QUICK_LAUNCH_IMAGE_FILE_LIMIT || width == 0 || height == 0 {
        return false;
    }
    let Ok(width) = u64::try_from(width) else {
        return false;
    };
    let Ok(height) = u64::try_from(height) else {
        return false;
    };
    width
        .checked_mul(height)
        .is_some_and(|pixels| pixels <= QUICK_LAUNCH_IMAGE_PIXEL_LIMIT)
}

fn decode_preview_text(bytes: &[u8]) -> Option<String> {
    let mut text = if bytes.starts_with(&[0xff, 0xfe]) {
        let units = bytes[2..]
            .chunks_exact(2)
            .map(|unit| u16::from_le_bytes([unit[0], unit[1]]))
            .collect::<Vec<_>>();
        String::from_utf16_lossy(&units)
    } else if bytes.starts_with(&[0xfe, 0xff]) {
        let units = bytes[2..]
            .chunks_exact(2)
            .map(|unit| u16::from_be_bytes([unit[0], unit[1]]))
            .collect::<Vec<_>>();
        String::from_utf16_lossy(&units)
    } else {
        if bytes.contains(&0) {
            return None;
        }
        String::from_utf8_lossy(bytes).into_owned()
    };
    while text.len() > QUICK_LAUNCH_TEXT_PREVIEW_LIMIT {
        text.pop();
    }
    Some(text)
}

fn format_preview_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn quick_launch_preview_bounds(
    panel_bounds: Bounds<Pixels>,
    preview_size: Size<Pixels>,
    screen: Bounds<Pixels>,
) -> Bounds<Pixels> {
    let gap = px(QUICK_LAUNCH_PREVIEW_GAP);
    let right = panel_bounds.right() + gap;
    let x = if right + preview_size.width <= screen.right() {
        right
    } else {
        (panel_bounds.origin.x - gap - preview_size.width).max(screen.origin.x)
    };
    let max_x = (screen.right() - preview_size.width).max(screen.origin.x);
    let max_y = (screen.bottom() - preview_size.height).max(screen.origin.y);
    Bounds::new(
        point(
            x.clamp(screen.origin.x, max_x),
            panel_bounds.origin.y.clamp(screen.origin.y, max_y),
        ),
        preview_size,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QuickLaunchPreviewOpenAction {
    Create,
    Reuse,
    Ignore,
}

fn quick_launch_preview_open_action(
    window_exists: bool,
    visible: bool,
    displayed_entry_id: Option<&str>,
    requested_entry_id: &str,
) -> QuickLaunchPreviewOpenAction {
    if !window_exists {
        QuickLaunchPreviewOpenAction::Create
    } else if visible && displayed_entry_id == Some(requested_entry_id) {
        QuickLaunchPreviewOpenAction::Ignore
    } else {
        QuickLaunchPreviewOpenAction::Reuse
    }
}

fn quick_launch_preview_target_changed(
    visible: bool,
    displayed_entry_id: Option<&str>,
    requested_entry_id: &str,
) -> bool {
    visible && displayed_entry_id.is_some() && displayed_entry_id != Some(requested_entry_id)
}

fn index_settings_from_quick_launch(settings: &QuickLaunchSettings) -> IndexSettings {
    IndexSettings {
        indexed_volumes: settings.indexed_volumes.clone(),
        excluded_paths: settings.excluded_paths.clone(),
    }
}

fn quick_launch_results_visible(
    query: &str,
    action_mode_active: bool,
    recent_directory_mode_active: bool,
) -> bool {
    action_mode_active || recent_directory_mode_active || !query.trim().is_empty()
}

fn quick_launch_catalog_refresh_query(
    is_open: bool,
    action_mode_active: bool,
    query: String,
) -> Option<String> {
    (is_open && !action_mode_active).then_some(query)
}

fn quick_launch_panel_height(
    query: &str,
    action_mode_active: bool,
    recent_directory_mode_active: bool,
) -> f32 {
    if quick_launch_results_visible(query, action_mode_active, recent_directory_mode_active) {
        PANEL_HEIGHT
    } else {
        COLLAPSED_PANEL_HEIGHT
    }
}

fn recent_directory_results(recent: &[RecentDirectory], filter: &str) -> Vec<SearchResult> {
    let filter = filter.trim().to_lowercase();
    recent
        .iter()
        .filter(|directory| filter.is_empty() || directory.path.to_lowercase().contains(&filter))
        .map(|directory| {
            let name = Path::new(&directory.path)
                .file_name()
                .and_then(|name| name.to_str())
                .filter(|name| !name.is_empty())
                .unwrap_or(&directory.path)
                .to_owned();
            SearchResult {
                entry: nxtools_finder::ApplicationEntry {
                    id: format!(
                        "recent-directory:{}",
                        directory.path.replace('/', "\\").to_lowercase()
                    ),
                    name,
                    target: directory.path.clone(),
                    source: ApplicationSource::IndexedDirectory,
                },
                action: SearchResultAction::OpenTarget,
                subtitle: None,
                score: 0,
                activation_count: directory.visit_count,
                last_activated_at_unix_ms: Some(directory.last_visited_at_unix_ms),
                query_activation_count: 0,
                last_query_activation_at_unix_ms: None,
            }
        })
        .collect()
}

fn recent_directory_navigation_action(
    result: &SearchResult,
    navigation_context: Option<&nxtools_protocol::DirectoryNavigationContext>,
) -> Option<QuickLaunchAction> {
    let context = navigation_context.filter(|context| context.window_handle != 0)?;
    (result.entry.source == ApplicationSource::IndexedDirectory
        && Path::new(&result.entry.target).is_absolute())
    .then(|| QuickLaunchAction::SwitchDirectory {
        host: context.host,
        window_handle: context.window_handle,
        target_directory: result.entry.target.clone(),
        host_context_id: context.host_context_id.clone(),
    })
}

fn elevated_directory_helper_command(
    request: Option<&ElevatedDirectoryHelperRequest>,
    request_pending: bool,
    acknowledged_pipe_name: Option<&str>,
    indexer_connected: bool,
) -> Option<IndexerCommand> {
    (!request_pending
        && indexer_connected
        && request
            .is_some_and(|request| acknowledged_pipe_name != Some(request.pipe_name.as_str())))
    .then(|| request.cloned())
    .flatten()
    .map(IndexerCommand::EnsureElevatedDirectoryHelper)
}

fn elevated_helper_status_matches_request(
    request: Option<&ElevatedDirectoryHelperRequest>,
    status: &nxtools_protocol::ElevatedDirectoryHelperStatus,
) -> bool {
    request.is_some_and(|request| request.pipe_name == status.pipe_name)
}

fn directory_context_resolution_matches(
    active_summon: Option<(i64, i64)>,
    resolution: &nxtools_protocol::QuickLaunchDirectoryContextResolution,
    now_unix_ms: i64,
) -> bool {
    let age = now_unix_ms.saturating_sub(resolution.requested_at_unix_ms);
    age <= REQUEST_MAX_AGE_MS
        && active_summon
            == Some((
                resolution.target_window_handle,
                resolution.requested_at_unix_ms,
            ))
}

fn auto_quick_switch_command(
    settings: &QuickLaunchSettings,
    request: &nxtools_protocol::QuickLaunchAutoSwitchRequest,
    now_unix_ms: i64,
) -> Option<NavigateDirectoryCommand> {
    let age = now_unix_ms.saturating_sub(request.requested_at_unix_ms);
    let source_directory = request.source_directory.trim();
    (settings.enabled
        && settings.auto_quick_switch
        && age <= AUTO_QUICK_SWITCH_REQUEST_MAX_AGE_MS
        && request.target.host == DirectoryNavigationHost::FileDialog
        && request.target.window_handle != 0
        && !source_directory.is_empty())
    .then(|| NavigateDirectoryCommand {
        host: DirectoryNavigationHost::FileDialog,
        window_handle: request.target.window_handle,
        target_directory: source_directory.to_owned(),
        host_context_id: request.target.host_context_id.clone(),
    })
}

fn visible_icon_prefetch_range(
    top_visible: usize,
    bottom_visible: usize,
    result_count: usize,
) -> std::ops::Range<usize> {
    let start = top_visible
        .saturating_sub(ICON_VISIBLE_OVERSCAN)
        .min(result_count);
    let end = bottom_visible
        .saturating_add(1)
        .saturating_add(ICON_VISIBLE_OVERSCAN)
        .min(result_count)
        .max(start);
    start..end
}

fn quick_launch_shortcut_hint(visible_position: usize) -> Option<&'static str> {
    QUICK_LAUNCH_RESULT_SHORTCUT_HINTS
        .get(visible_position)
        .copied()
}

fn quick_launch_shortcut_target(
    key: &str,
    modifiers: gpui::Modifiers,
    visible_range: Range<usize>,
    result_count: usize,
) -> Option<usize> {
    if !modifiers.control
        || modifiers.alt
        || modifiers.shift
        || modifiers.platform
        || modifiers.function
    {
        return None;
    }
    let shortcut_position = key.parse::<usize>().ok()?.checked_sub(1)?;
    QUICK_LAUNCH_RESULT_SHORTCUT_HINTS.get(shortcut_position)?;
    let target = visible_range.start.checked_add(shortcut_position)?;
    (target < visible_range.end && target < result_count).then_some(target)
}

fn quick_launch_match_ranges(text: &str, query: &str) -> Vec<std::ops::Range<usize>> {
    let mut ranges = query
        .trim_start()
        .strip_prefix('*')
        .unwrap_or(query.trim_start())
        .split_whitespace()
        .filter_map(|raw| {
            if raw == "|" || raw.starts_with('!') || raw.starts_with("::") {
                return None;
            }
            let pattern = raw
                .trim_matches('"')
                .trim_matches('\'')
                .trim_start_matches('^')
                .trim_end_matches('$');
            (!pattern.is_empty()).then_some(pattern)
        })
        .flat_map(|pattern| match_ranges_for_pattern(text, pattern))
        .collect::<Vec<_>>();
    ranges.sort_by_key(|range| (range.start, range.end));
    let mut merged: Vec<std::ops::Range<usize>> = Vec::with_capacity(ranges.len());
    for range in ranges {
        if let Some(previous) = merged.last_mut()
            && range.start <= previous.end
        {
            previous.end = previous.end.max(range.end);
        } else {
            merged.push(range);
        }
    }
    merged
}

fn match_ranges_for_pattern(text: &str, pattern: &str) -> Vec<std::ops::Range<usize>> {
    let text_chars = text.char_indices().collect::<Vec<_>>();
    let pattern_chars = pattern.chars().collect::<Vec<_>>();
    if pattern_chars.is_empty() || pattern_chars.len() > text_chars.len() {
        return Vec::new();
    }
    let case_sensitive = pattern_chars
        .iter()
        .any(|character| character.is_uppercase());
    let matches = |left: char, right: char| {
        if case_sensitive {
            left == right
        } else {
            left.to_lowercase().eq(right.to_lowercase())
        }
    };
    for start in 0..=text_chars.len() - pattern_chars.len() {
        if pattern_chars
            .iter()
            .enumerate()
            .all(|(offset, pattern)| matches(text_chars[start + offset].1, *pattern))
        {
            let end_index = start + pattern_chars.len();
            let end = text_chars
                .get(end_index)
                .map(|(index, _)| *index)
                .unwrap_or(text.len());
            return std::iter::once(text_chars[start].0..end).collect();
        }
    }
    let mut ranges = Vec::with_capacity(pattern_chars.len());
    let mut text_offset = 0;
    for pattern in pattern_chars {
        let Some((relative, (byte_index, character))) = text_chars[text_offset..]
            .iter()
            .enumerate()
            .find(|(_, (_, character))| matches(*character, pattern))
        else {
            return Vec::new();
        };
        ranges.push(*byte_index..*byte_index + character.len_utf8());
        text_offset += relative + 1;
    }
    ranges
}

fn should_publish_immediate_results(indexer_connected: bool, search_queued: bool) -> bool {
    !indexer_connected || !search_queued
}

fn elapsed_microseconds(started: Instant) -> u64 {
    started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64
}

fn should_search_indexer(indexer_connected: bool, index_ready: bool) -> bool {
    indexer_connected && index_ready
}

fn index_status_has_searchable_snapshot(status: Option<&IndexStatus>) -> bool {
    status.is_some_and(|status| {
        status.phase == nxtools_protocol::IndexPhase::Ready || status.indexed_items > 0
    })
}

fn should_retry_index_search_after_status(
    previous: Option<&IndexStatus>,
    current: &IndexStatus,
) -> bool {
    let was_ready =
        previous.is_some_and(|status| status.phase == nxtools_protocol::IndexPhase::Ready);
    !was_ready && current.phase == nxtools_protocol::IndexPhase::Ready
}

fn index_status_affects_presentation(
    previous: Option<&IndexStatus>,
    current: &IndexStatus,
) -> bool {
    let Some(previous) = previous else {
        return true;
    };
    previous.phase != current.phase
        || previous.indexed_items != current.indexed_items
        || previous.last_error != current.last_error
        || previous.drives.len() != current.drives.len()
        || previous
            .drives
            .iter()
            .zip(&current.drives)
            .any(|(previous, current)| {
                previous.mount_point != current.mount_point
                    || previous.file_system != current.file_system
                    || previous.indexed_items != current.indexed_items
                    || previous.journal_id != current.journal_id
            })
}

#[derive(Default)]
struct ImeEnterGate {
    composition_seen: bool,
    pending_confirmation_enter: bool,
}

#[derive(Default)]
struct PointerSelectionGate {
    row: Option<usize>,
    position: Option<(f32, f32)>,
}

impl PointerSelectionGate {
    fn observe_move(&mut self, row: usize, position: (f32, f32)) -> bool {
        let should_select = self.row == Some(row) && self.position != Some(position);
        self.row = Some(row);
        self.position = Some(position);
        should_select
    }

    fn reset(&mut self) {
        self.row = None;
        self.position = None;
    }
}

impl ImeEnterGate {
    fn reset(&mut self) {
        self.composition_seen = false;
        self.pending_confirmation_enter = false;
    }

    fn observe_composition(&mut self, active: bool) {
        self.composition_seen |= active;
    }

    fn observe_change(&mut self) {
        self.pending_confirmation_enter = self.composition_seen;
        self.composition_seen = false;
    }

    fn consume_enter(&mut self) -> bool {
        !std::mem::take(&mut self.pending_confirmation_enter)
    }
}

struct QuickLaunchSearchUpdate {
    results: Vec<SearchResult>,
    pending: bool,
}

fn apply_search_update<T>(
    committed_results: &mut Vec<T>,
    update_results: Vec<T>,
    pending: bool,
) -> bool {
    if pending {
        return false;
    }
    *committed_results = update_results;
    true
}

fn should_hide_for_foreground(
    is_open: bool,
    grace_elapsed: bool,
    popup_window_handle: i64,
    preview_window_handle: i64,
    foreground_window_handle: i64,
) -> bool {
    is_open
        && grace_elapsed
        && foreground_window_handle != popup_window_handle
        && (preview_window_handle == 0 || foreground_window_handle != preview_window_handle)
}

fn should_hide_for_pointer(
    is_open: bool,
    popup_window_handle: i64,
    pointer_root_window_handle: i64,
    pointer_process_id: u32,
    desktop_process_id: u32,
) -> bool {
    is_open
        && pointer_root_window_handle != popup_window_handle
        && pointer_process_id != desktop_process_id
}

#[derive(Clone)]
struct QuickLaunchWindow {
    window: AnyWindowHandle,
    panel: WeakEntity<QuickLaunchPanel>,
    native_window_handle: i64,
}

#[derive(Default)]
struct QuickLaunchPreviewSession {
    window: Option<QuickLaunchPreviewWindow>,
    visible: bool,
    displayed_entry_id: Option<String>,
    generation: u64,
    hovered_entry_id: Option<String>,
    pointer_inside: bool,
    pending_stage: Option<QuickLaunchPreviewPendingStage>,
    image_asset: Option<Arc<Image>>,
    exit_generation: u64,
}

#[derive(Clone)]
struct QuickLaunchPreviewCandidate {
    result: SearchResult,
    panel_bounds: Bounds<Pixels>,
    panel_scale_factor: f32,
}

#[derive(Clone)]
struct QuickLaunchPreviewWindow {
    identity: QuickLaunchPreviewWindowIdentity,
    view: WeakEntity<QuickLaunchPreview>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct QuickLaunchPreviewWindowIdentity {
    gpui: AnyWindowHandle,
    native: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QuickLaunchPreviewStagePhase {
    MoveOffscreen,
    ShowOffscreen,
    RevealOnscreen,
}

struct QuickLaunchPreviewPendingStage {
    command_id: uuid::Uuid,
    phase: QuickLaunchPreviewStagePhase,
    window_identity: QuickLaunchPreviewWindowIdentity,
    generation: u64,
    entry_id: String,
    payload: Option<QuickLaunchPreviewPayload>,
    preview_size: Size<Pixels>,
    bounds: Bounds<Pixels>,
    panel_scale_factor: f32,
}

struct QuickLaunchPreview {
    payload: QuickLaunchPreviewPayload,
    application: WeakEntity<QuickLaunchApplication>,
    text_scroll: ScrollHandle,
    appearance_generation: u64,
    is_exiting: bool,
    motion_suppressed: bool,
}

impl QuickLaunchPreview {
    fn new(
        payload: QuickLaunchPreviewPayload,
        application: WeakEntity<QuickLaunchApplication>,
    ) -> Self {
        Self {
            payload,
            application,
            text_scroll: ScrollHandle::new(),
            appearance_generation: 1,
            is_exiting: false,
            motion_suppressed: false,
        }
    }

    fn set_payload(&mut self, payload: QuickLaunchPreviewPayload, cx: &mut Context<Self>) {
        self.release_image_asset(cx);
        self.payload = payload;
        self.text_scroll.set_offset(point(px(0.0), px(0.0)));
        self.begin_enter(cx);
    }

    fn begin_enter(&mut self, cx: &mut Context<Self>) {
        self.is_exiting = false;
        self.motion_suppressed = false;
        self.appearance_generation = self.appearance_generation.wrapping_add(1).max(1);
        cx.notify();
    }

    fn cancel_exit(&mut self, cx: &mut Context<Self>) {
        if self.is_exiting {
            self.is_exiting = false;
            self.motion_suppressed = true;
            cx.notify();
        }
    }

    fn begin_exit(&mut self, cx: &mut Context<Self>) {
        if !self.is_exiting {
            self.is_exiting = true;
            self.motion_suppressed = false;
            self.appearance_generation = self.appearance_generation.wrapping_add(1).max(1);
            cx.notify();
        }
    }

    fn clear_image_asset(&mut self, cx: &mut Context<Self>) {
        if matches!(self.payload.body, QuickLaunchPreviewBody::Image(_)) {
            self.release_image_asset(cx);
            self.payload.body = QuickLaunchPreviewBody::Metadata {
                icon: MaterialIcon::Image,
            };
            cx.notify();
        }
    }

    fn release_image_asset(&self, cx: &mut Context<Self>) {
        if let QuickLaunchPreviewBody::Image(image) = &self.payload.body {
            ImageSource::from(Arc::clone(image)).remove_asset(cx);
        }
    }
}

impl Render for QuickLaunchPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::zed_dark();
        let application = self.application.clone();
        let appearance_generation = self.appearance_generation;
        let is_exiting = self.is_exiting;
        let motion_suppressed = self.motion_suppressed;
        let has_timestamps =
            self.payload.created_at.is_some() || self.payload.modified_at.is_some();
        let body = match &self.payload.body {
            QuickLaunchPreviewBody::Image(image) => preview_media_surface(theme)
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    img(Arc::clone(image))
                        .size_full()
                        .object_fit(ObjectFit::Contain),
                )
                .into_any_element(),
            QuickLaunchPreviewBody::Text { text, truncated } => preview_media_surface(theme)
                .id("quick-launch-preview-text")
                .size_full()
                .track_scroll(&self.text_scroll)
                .overflow_y_scroll()
                .vertical_scrollbar(&self.text_scroll)
                .p(px(SPACING_12))
                .text_size(px(TEXT_BODY))
                .line_height(px(LINE_HEIGHT_BODY))
                .text_color(theme.text_secondary)
                .font_family(UI_MONO_FONT_FAMILY)
                .whitespace_normal()
                .flex()
                .flex_col()
                .items_start()
                .justify_start()
                .child(
                    div()
                        .debug_selector(|| "quick-launch-preview-text-copy".into())
                        .w_full()
                        .child(text.clone()),
                )
                .when(*truncated, |content| {
                    content.child(
                        div()
                            .mt(px(SPACING_8))
                            .pt(px(SPACING_8))
                            .border_t_1()
                            .border_color(theme.border_subtle)
                            .text_size(px(TEXT_LABEL))
                            .line_height(px(LINE_HEIGHT_LABEL))
                            .text_color(theme.text_muted)
                            .child("仅显示文件开头的 96 KB"),
                    )
                })
                .into_any_element(),
            QuickLaunchPreviewBody::Metadata { icon } => preview_media_surface(theme)
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(empty_state(
                    theme,
                    "quick-launch-preview-unavailable",
                    *icon,
                    "没有轻量预览",
                    "仍可直接打开、定位或执行操作。",
                ))
                .into_any_element(),
        };

        let preview = preview_shell(theme)
            .id("quick-launch-preview-root")
            .w(px(QUICK_LAUNCH_PREVIEW_WIDTH))
            .h(px(QUICK_LAUNCH_PREVIEW_HEIGHT))
            .on_hover(move |hovered, _window, cx| {
                let inside = *hovered;
                schedule_application_update(
                    application.clone(),
                    move |application, cx| {
                        application.set_quick_launch_preview_pointer_inside(inside, cx)
                    },
                    cx,
                );
            })
            .child(
                div()
                    .debug_selector(|| "quick-launch-preview-header".into())
                    .h(px(QUICK_LAUNCH_PREVIEW_HEADER_HEIGHT))
                    .flex_none()
                    .px(px(SPACING_12))
                    .border_b_1()
                    .border_color(theme.border_subtle)
                    .flex()
                    .flex_col()
                    .justify_center()
                    .gap(px(SPACING_2))
                    .child(
                        div()
                            .h(px(LINE_HEIGHT_BODY))
                            .text_size(px(TEXT_BODY))
                            .line_height(px(LINE_HEIGHT_BODY))
                            .truncate()
                            .child(self.payload.name.clone()),
                    )
                    .child(
                        div()
                            .h(px(LINE_HEIGHT_LABEL))
                            .text_size(px(TEXT_LABEL))
                            .line_height(px(LINE_HEIGHT_LABEL))
                            .text_color(theme.text_secondary)
                            .truncate()
                            .child(self.payload.detail.clone()),
                    )
                    .when(has_timestamps, |header| {
                        header.child(
                            div()
                                .w_full()
                                .flex()
                                .items_center()
                                .gap(px(SPACING_12))
                                .text_size(px(TEXT_LABEL))
                                .line_height(px(LINE_HEIGHT_LABEL))
                                .text_color(theme.text_muted)
                                .when_some(self.payload.created_at.clone(), |timestamps, value| {
                                    timestamps.child(
                                        div()
                                            .min_w_0()
                                            .flex_1()
                                            .truncate()
                                            .child(format!("创建 {value}")),
                                    )
                                })
                                .when_some(
                                    self.payload.modified_at.clone(),
                                    |timestamps, value| {
                                        timestamps.child(
                                            div()
                                                .min_w_0()
                                                .flex_1()
                                                .truncate()
                                                .child(format!("修改 {value}")),
                                        )
                                    },
                                ),
                        )
                    }),
            )
            .child(
                div()
                    .debug_selector(|| "quick-launch-preview-content".into())
                    .min_h_0()
                    .flex_1()
                    .overflow_hidden()
                    .p(px(SPACING_8))
                    .child(body),
            )
            .child(
                div()
                    .debug_selector(|| "quick-launch-preview-footer".into())
                    .h(px(QUICK_LAUNCH_PREVIEW_FOOTER_HEIGHT))
                    .flex_none()
                    .px(px(SPACING_12))
                    .border_t_1()
                    .border_color(theme.border_subtle)
                    .flex()
                    .items_center()
                    .font_family(UI_MONO_FONT_FAMILY)
                    .text_size(px(TEXT_LABEL))
                    .line_height(px(LINE_HEIGHT_LABEL))
                    .text_color(theme.text_muted)
                    .truncate()
                    .child(self.payload.target.clone()),
            );
        if motion_suppressed {
            preview.into_any_element()
        } else {
            preview
                .with_animation(
                    ("quick-launch-preview-motion", appearance_generation),
                    Animation::new(if is_exiting {
                        QUICK_LAUNCH_PREVIEW_DISAPPEAR_DURATION
                    } else {
                        PREVIEW_APPEAR_DURATION
                    })
                    .with_easing(ease_out_quint()),
                    move |preview, delta| {
                        if is_exiting {
                            preview
                                .relative()
                                .top(px(-2.0 * delta))
                                .opacity(1.0 - 0.1 * delta)
                        } else {
                            preview
                                .relative()
                                .top(px(3.0 * (1.0 - delta)))
                                .opacity(0.9 + 0.1 * delta)
                        }
                    },
                )
                .into_any_element()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum QuickLaunchApplicationEvent {
    CustomCommandProgramPicked { index: usize, path: String },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct QuickLaunchPerformanceSnapshot {
    pub last_local_search_us: Option<u64>,
    pub last_index_search_us: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum QuickLaunchHotkeyLifecycle {
    Applied,
    WaitingForAdapter { status: String },
    WaitingForRecording { status: String },
    Recording,
    Applying { candidate: Option<String> },
    Failed { error: String },
}

impl QuickLaunchHotkeyLifecycle {
    pub(crate) fn owns_recording_session(&self) -> bool {
        matches!(self, Self::WaitingForRecording { .. } | Self::Recording)
    }
}

pub(crate) struct QuickLaunchApplication {
    catalog: Rc<RefCell<FinderCatalog>>,
    catalog_refresh_in_progress: bool,
    platform: Rc<PlatformBridge>,
    indexer: Rc<IndexerBridge>,
    config: AppConfig,
    config_path: PathBuf,
    log_dir: PathBuf,
    notifications: Entity<NotificationCenter>,
    popup: Option<QuickLaunchWindow>,
    preview: QuickLaunchPreviewSession,
    pending_configuration: HashSet<uuid::Uuid>,
    pending_hotkey: Option<(uuid::Uuid, ClipboardHotKey)>,
    hotkey_error: Option<String>,
    pending_recording: Option<(uuid::Uuid, bool)>,
    recording_active: bool,
    recording_desired: bool,
    pending_actions: HashMap<uuid::Uuid, PendingQuickLaunchAction>,
    pending_auto_quick_switches: HashMap<uuid::Uuid, (i64, String)>,
    file_dialog_search: FileDialogSearchSession,
    navigation_foreground_guard: Option<NavigationForegroundGuard>,
    popup_session_id: uuid::Uuid,
    active_summon: Option<(i64, i64)>,
    pending_shell_queries: HashMap<uuid::Uuid, PendingShellQuery>,
    pending_window_activation: HashMap<uuid::Uuid, PendingWindowActivation>,
    icon_resources: IconResourceCache,
    pending_icons: HashMap<uuid::Uuid, PendingIconResource>,
    pending_icon_resources: HashMap<String, uuid::Uuid>,
    failed_icon_resources: HashMap<String, Instant>,
    indexer_service_bootstrap: IndexerServiceBootstrap,
    pending_exclusion_picker: Option<uuid::Uuid>,
    pending_program_picker: Option<(uuid::Uuid, usize)>,
    elevated_helper_request: Option<ElevatedDirectoryHelperRequest>,
    pending_elevated_helper: Option<uuid::Uuid>,
    acknowledged_elevated_helper_pipe: Option<String>,
    last_sent_index_settings: Option<IndexSettings>,
    configuration_status: String,
    index_status: Option<IndexStatus>,
    index_search: IndexSearchSession,
    last_local_search_us: Option<u64>,
}

impl EventEmitter<QuickLaunchApplicationEvent> for QuickLaunchApplication {}

#[derive(Default)]
struct IndexSearchSession {
    generation: u64,
    query: String,
    items: Vec<IndexSearchItem>,
    complete: bool,
    last_completed_elapsed_us: Option<u64>,
}

#[derive(Default)]
struct IndexerServiceBootstrap {
    pending: Option<uuid::Uuid>,
    pending_automatic: bool,
    automatic_attempted: bool,
}

impl IndexerServiceBootstrap {
    fn on_platform_connected(&mut self) -> bool {
        // A correlation id belongs to one PlatformHost pipe generation. A reconnect can never
        // deliver the old result, so retaining it would permanently disable the manual retry.
        self.pending = None;
        self.pending_automatic = false;
        if self.automatic_attempted {
            return false;
        }
        self.automatic_attempted = true;
        true
    }

    fn begin(&mut self, correlation_id: uuid::Uuid, automatic: bool) {
        self.pending = Some(correlation_id);
        self.pending_automatic = automatic;
    }

    fn finish(&mut self, correlation_id: uuid::Uuid) -> Option<bool> {
        if self.pending != Some(correlation_id) {
            return None;
        }
        self.pending = None;
        Some(std::mem::take(&mut self.pending_automatic))
    }
}

impl IndexSearchSession {
    fn begin(&mut self, query: impl Into<String>, limit: usize) -> IndexSearchRequest {
        self.invalidate();
        self.query = query.into();
        IndexSearchRequest {
            generation: self.generation,
            query: self.query.clone(),
            limit,
            context: IndexSearchContext {
                directory: None,
                preferred_directory: None,
                include_files: true,
                include_directories: true,
            },
        }
    }

    fn invalidate(&mut self) {
        self.generation = self.generation.wrapping_add(1).max(1);
        self.query.clear();
        self.items.clear();
        self.complete = false;
    }

    fn apply(&mut self, batch: IndexSearchBatch) -> bool {
        if batch.generation != self.generation {
            return false;
        }
        self.items.extend(batch.items);
        self.complete = batch.complete;
        if batch.complete {
            self.last_completed_elapsed_us = Some(batch.elapsed_us);
        }
        true
    }
}

#[derive(Clone)]
struct PendingQuickLaunchAction {
    correlation_id: uuid::Uuid,
    popup_session_id: uuid::Uuid,
    entry_id: String,
    display_name: String,
    target: String,
    query: String,
    action: QuickLaunchAction,
    directory_visit: Option<String>,
    trigger: QuickLaunchActionTrigger,
    source: ApplicationSource,
    started_at: Instant,
}

fn pending_action_targets_session(
    pending_action: &PendingQuickLaunchAction,
    popup_session_id: uuid::Uuid,
) -> bool {
    pending_action.popup_session_id == popup_session_id
}

fn pending_action_owns_foreground(
    pending_action: &PendingQuickLaunchAction,
    popup_session_id: uuid::Uuid,
    foreground_window_handle: i64,
) -> bool {
    pending_action_targets_session(pending_action, popup_session_id)
        && matches!(
            pending_action.action,
            QuickLaunchAction::SwitchDirectory { window_handle, .. }
                if window_handle == foreground_window_handle
        )
}

#[derive(Clone, Copy)]
struct NavigationForegroundGuard {
    popup_session_id: uuid::Uuid,
    window_handle: i64,
    expires_at: Instant,
}

fn navigation_guard_owns_foreground(
    guard: &mut Option<NavigationForegroundGuard>,
    popup_session_id: uuid::Uuid,
    foreground_window_handle: i64,
    now: Instant,
) -> bool {
    let Some(current) = *guard else {
        return false;
    };
    if current.popup_session_id != popup_session_id || now >= current.expires_at {
        *guard = None;
        return false;
    }
    current.window_handle == foreground_window_handle
}

#[derive(Clone)]
struct PendingShellQuery {
    mode_id: uuid::Uuid,
    level_id: uuid::Uuid,
    entry_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum QuickLaunchAction {
    Open,
    SwitchDirectory {
        host: DirectoryNavigationHost,
        window_handle: i64,
        target_directory: String,
        host_context_id: Option<String>,
    },
    Reveal,
    CopyPath,
    CopyResult,
    CopyCommand,
    RunElevated,
    Shell {
        session_id: String,
        command_id: u32,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum QuickLaunchActionTrigger {
    Enter,
    Shortcut,
    Pointer,
    ActionMode,
    RecentDirectory,
}

impl QuickLaunchActionTrigger {
    const fn diagnostic_label(self) -> &'static str {
        match self {
            Self::Enter => "enter",
            Self::Shortcut => "shortcut",
            Self::Pointer => "pointer",
            Self::ActionMode => "action_mode",
            Self::RecentDirectory => "recent_directory",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ActionSuccessPolicy {
    RecordAndHide,
    HideWithoutRecording,
    KeepOpen,
}

fn action_success_policy(action: &QuickLaunchAction) -> ActionSuccessPolicy {
    match action {
        QuickLaunchAction::CopyPath | QuickLaunchAction::CopyCommand => {
            ActionSuccessPolicy::KeepOpen
        }
        QuickLaunchAction::CopyResult => ActionSuccessPolicy::HideWithoutRecording,
        QuickLaunchAction::Open
        | QuickLaunchAction::SwitchDirectory { .. }
        | QuickLaunchAction::Reveal
        | QuickLaunchAction::RunElevated
        | QuickLaunchAction::Shell { .. } => ActionSuccessPolicy::RecordAndHide,
    }
}

fn quick_launch_action_label(action: &QuickLaunchAction) -> &str {
    if matches!(action, QuickLaunchAction::SwitchDirectory { .. }) {
        return "切换到此目录";
    }
    if matches!(action, QuickLaunchAction::Shell { .. }) {
        return "Windows 操作";
    }
    if matches!(action, QuickLaunchAction::CopyResult) {
        return "复制结果";
    }
    if matches!(action, QuickLaunchAction::CopyCommand) {
        return "复制命令";
    }
    QUICK_LAUNCH_ACTIONS
        .iter()
        .find(|item| &item.action == action)
        .map(|item| item.label)
        .unwrap_or("执行操作")
}

fn quick_launch_action_diagnostic_label(action: &QuickLaunchAction) -> &'static str {
    match action {
        QuickLaunchAction::Open => "open",
        QuickLaunchAction::SwitchDirectory { .. } => "switch_directory",
        QuickLaunchAction::Reveal => "reveal",
        QuickLaunchAction::CopyPath => "copy_path",
        QuickLaunchAction::CopyResult => "copy_result",
        QuickLaunchAction::CopyCommand => "copy_command",
        QuickLaunchAction::RunElevated => "run_elevated",
        QuickLaunchAction::Shell { .. } => "shell_action",
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct QuickLaunchBuiltinActionItem {
    action: QuickLaunchAction,
    label: &'static str,
    icon: MaterialIcon,
}

const QUICK_LAUNCH_ACTIONS: [QuickLaunchBuiltinActionItem; 4] = [
    QuickLaunchBuiltinActionItem {
        action: QuickLaunchAction::Open,
        label: "打开",
        icon: MaterialIcon::OpenInNew,
    },
    QuickLaunchBuiltinActionItem {
        action: QuickLaunchAction::Reveal,
        label: "打开文件所在位置",
        icon: MaterialIcon::Folder,
    },
    QuickLaunchBuiltinActionItem {
        action: QuickLaunchAction::CopyPath,
        label: "复制路径",
        icon: MaterialIcon::ContentCopy,
    },
    QuickLaunchBuiltinActionItem {
        action: QuickLaunchAction::RunElevated,
        label: "以管理员身份运行",
        icon: MaterialIcon::ProgressActivity,
    },
];

#[derive(Clone, Debug, Eq, PartialEq)]
struct QuickLaunchActionItem {
    action: Option<QuickLaunchAction>,
    label: String,
    icon: Option<MaterialIcon>,
    icon_path: Option<PathBuf>,
    enabled: bool,
    submenu: Option<QuickLaunchShellSubmenu>,
    section_label: Option<String>,
    divider_before: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct QuickLaunchShellSubmenu {
    session_id: String,
    submenu_id: u32,
}

impl QuickLaunchActionItem {
    fn builtin(item: QuickLaunchBuiltinActionItem) -> Self {
        Self {
            action: Some(item.action),
            label: item.label.into(),
            icon: Some(item.icon),
            icon_path: None,
            enabled: true,
            submenu: None,
            section_label: None,
            divider_before: false,
        }
    }

    fn is_activatable(&self) -> bool {
        self.enabled && (self.action.is_some() || self.submenu.is_some())
    }

    fn switch_directory(
        host: DirectoryNavigationHost,
        window_handle: i64,
        target_directory: String,
        host_context_id: Option<String>,
    ) -> Self {
        Self::builtin(QuickLaunchBuiltinActionItem {
            action: QuickLaunchAction::SwitchDirectory {
                host,
                window_handle,
                target_directory,
                host_context_id,
            },
            label: "切换到此目录",
            icon: MaterialIcon::Folder,
        })
    }
}

fn shell_action_items(session_id: &str, items: Vec<ShellActionItem>) -> Vec<QuickLaunchActionItem> {
    let mut converted = Vec::new();
    let mut divider_before = false;
    for item in items {
        if item.separator {
            divider_before = !converted.is_empty();
            continue;
        }
        let submenu = item.submenu_id.map(|submenu_id| QuickLaunchShellSubmenu {
            session_id: session_id.into(),
            submenu_id,
        });
        let action = if submenu.is_none() {
            item.command_id.map(|command_id| QuickLaunchAction::Shell {
                session_id: session_id.into(),
                command_id,
            })
        } else {
            None
        };
        converted.push(QuickLaunchActionItem {
            action,
            label: item.title,
            icon: None,
            icon_path: item
                .icon_path
                .map(PathBuf::from)
                .filter(|path| path.is_file()),
            enabled: item.enabled,
            submenu,
            section_label: None,
            divider_before,
        });
        divider_before = false;
    }
    converted
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct QuickLaunchResultsContext {
    query: String,
    selected: usize,
    selected_entry_id: String,
}

fn selection_for_results_context(
    results: &[SearchResult],
    context: &QuickLaunchResultsContext,
) -> usize {
    results
        .iter()
        .position(|result| result.entry.id == context.selected_entry_id)
        .unwrap_or_else(|| context.selected.min(results.len().saturating_sub(1)))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum QuickLaunchEscapeAction {
    ExitActions,
    ClearQuery,
    Close,
}

fn quick_launch_escape_action(
    action_mode_active: bool,
    query_is_empty: bool,
) -> QuickLaunchEscapeAction {
    if action_mode_active {
        QuickLaunchEscapeAction::ExitActions
    } else if query_is_empty {
        QuickLaunchEscapeAction::Close
    } else {
        QuickLaunchEscapeAction::ClearQuery
    }
}

fn quick_launch_dismiss_commands(
    popup_window_handle: i64,
    restore_window_handle: Option<i64>,
) -> Vec<PlatformCommand> {
    let mut commands = Vec::with_capacity(3);
    if popup_window_handle != 0 {
        commands.push(PlatformCommand::SetWindowVisibility(
            SetWindowVisibilityCommand {
                window_handle: popup_window_handle,
                visible: false,
            },
        ));
        commands.push(PlatformCommand::SetWindowTopmost(SetWindowTopmostCommand {
            window_handle: popup_window_handle,
            enabled: false,
        }));
    }
    if let Some(window_handle) = restore_window_handle
        && window_handle != 0
        && window_handle != popup_window_handle
    {
        commands.push(PlatformCommand::ActivateWindow(ActivateWindowCommand {
            window_handle,
        }));
    }
    commands
}

#[derive(Clone)]
struct QuickLaunchActionLevel {
    id: uuid::Uuid,
    title: String,
    items: Vec<QuickLaunchActionItem>,
    filter: String,
    selected: usize,
    shell_pending: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingShellSubmenuLevel {
    level_id: uuid::Uuid,
    session_id: String,
    submenu_id: u32,
}

#[derive(Clone)]
struct QuickLaunchActionMode {
    id: uuid::Uuid,
    target: SearchResult,
    parent: QuickLaunchResultsContext,
    navigation_context: Option<nxtools_protocol::DirectoryNavigationContext>,
    levels: Vec<QuickLaunchActionLevel>,
}

impl QuickLaunchActionMode {
    #[cfg(test)]
    fn enter(target: SearchResult, query: String, selected: usize) -> Self {
        Self::enter_in_context(target, query, selected, None)
    }

    fn enter_in_context(
        target: SearchResult,
        query: String,
        selected: usize,
        navigation_context: Option<&nxtools_protocol::DirectoryNavigationContext>,
    ) -> Self {
        let selected_entry_id = target.entry.id.clone();
        let copy_text = target.copy_text().is_some();
        let items = if target.entry.source == ApplicationSource::CustomCommand {
            vec![
                QuickLaunchActionItem::builtin(QuickLaunchBuiltinActionItem {
                    action: QuickLaunchAction::Open,
                    label: "运行命令",
                    icon: MaterialIcon::OpenInNew,
                }),
                QuickLaunchActionItem::builtin(QuickLaunchBuiltinActionItem {
                    action: QuickLaunchAction::CopyCommand,
                    label: "复制命令",
                    icon: MaterialIcon::ContentCopy,
                }),
            ]
        } else if target.entry.source == ApplicationSource::EnvironmentVariable && copy_text {
            vec![QuickLaunchActionItem::builtin(
                QuickLaunchBuiltinActionItem {
                    action: QuickLaunchAction::CopyResult,
                    label: "复制变量值",
                    icon: MaterialIcon::ContentCopy,
                },
            )]
        } else if copy_text {
            vec![QuickLaunchActionItem::builtin(
                QuickLaunchBuiltinActionItem {
                    action: QuickLaunchAction::CopyResult,
                    label: "复制结果",
                    icon: MaterialIcon::ContentCopy,
                },
            )]
        } else if target.entry.source == ApplicationSource::WebUrl {
            vec![
                QuickLaunchActionItem::builtin(QuickLaunchBuiltinActionItem {
                    action: QuickLaunchAction::Open,
                    label: "打开网址",
                    icon: MaterialIcon::OpenInNew,
                }),
                QuickLaunchActionItem::builtin(QuickLaunchBuiltinActionItem {
                    action: QuickLaunchAction::CopyPath,
                    label: "复制网址",
                    icon: MaterialIcon::ContentCopy,
                }),
            ]
        } else {
            QUICK_LAUNCH_ACTIONS
                .iter()
                .filter(|item| quick_launch_action_available(&target, &item.action))
                .cloned()
                .map(QuickLaunchActionItem::builtin)
                .collect::<Vec<_>>()
        };
        let shell_pending = quick_launch_result_capabilities(&target).shell_actions;
        Self {
            id: uuid::Uuid::new_v4(),
            target,
            parent: QuickLaunchResultsContext {
                query,
                selected,
                selected_entry_id,
            },
            navigation_context: navigation_context
                .filter(|context| context.window_handle != 0)
                .cloned(),
            levels: vec![QuickLaunchActionLevel {
                id: uuid::Uuid::new_v4(),
                title: "操作".into(),
                items,
                filter: String::new(),
                selected: 0,
                shell_pending,
            }],
        }
    }

    fn target(&self) -> &SearchResult {
        &self.target
    }

    fn id(&self) -> uuid::Uuid {
        self.id
    }

    fn matches_shell_query(
        &self,
        mode_id: uuid::Uuid,
        level_id: uuid::Uuid,
        entry_id: &str,
    ) -> bool {
        self.id == mode_id
            && self.current_level().id == level_id
            && self.target.entry.id == entry_id
    }

    fn exit(self) -> QuickLaunchResultsContext {
        self.parent
    }

    fn current_level(&self) -> &QuickLaunchActionLevel {
        self.levels
            .last()
            .expect("action mode always has a root level")
    }

    fn current_level_mut(&mut self) -> &mut QuickLaunchActionLevel {
        self.levels
            .last_mut()
            .expect("action mode always has a root level")
    }

    fn depth(&self) -> usize {
        self.levels.len()
    }

    fn title(&self) -> &str {
        &self.current_level().title
    }

    fn current_level_id(&self) -> uuid::Uuid {
        self.current_level().id
    }

    fn shell_pending(&self) -> bool {
        self.current_level().shell_pending
    }

    fn set_filter(&mut self, filter: impl Into<String>) {
        let level = self.current_level_mut();
        level.filter = filter.into();
        level.selected = first_activatable_index(&visible_action_items(level)).unwrap_or(0);
    }

    fn visible_actions(&self) -> Vec<QuickLaunchActionItem> {
        visible_action_items(self.current_level())
    }

    fn selected(&self) -> usize {
        self.current_level().selected
    }

    fn set_selected(&mut self, selected: usize) {
        let visible = self.visible_actions();
        if visible
            .get(selected)
            .is_some_and(QuickLaunchActionItem::is_activatable)
        {
            self.current_level_mut().selected = selected;
        }
    }

    fn apply_shell_menu(&mut self, level_id: uuid::Uuid, menu: Option<ShellActionMenu>) {
        if self.current_level().id != level_id {
            return;
        }
        self.current_level_mut().shell_pending = false;
        let navigation_directory = menu
            .as_ref()
            .and_then(|menu| menu.navigation_directory.clone())
            .filter(|directory| Path::new(directory).is_absolute());
        let navigation_action = self
            .navigation_context
            .as_ref()
            .map(|context| {
                (
                    context.host,
                    context.window_handle,
                    context.host_context_id.clone(),
                )
            })
            .zip(navigation_directory);
        if self.depth() == 1
            && let Some(((host, window_handle, host_context_id), target_directory)) =
                navigation_action
        {
            self.current_level_mut().items.insert(
                0,
                QuickLaunchActionItem::switch_directory(
                    host,
                    window_handle,
                    target_directory,
                    host_context_id,
                ),
            );
        }
        let Some(menu) = menu else {
            return;
        };
        let mut shell_items = shell_action_items(&menu.session_id, menu.items);
        if self.depth() == 1 {
            if let Some(first) = shell_items.first_mut() {
                first.section_label = Some("Windows 操作".into());
                first.divider_before = true;
            }
            self.current_level_mut().items.extend(shell_items);
        } else {
            self.current_level_mut().items = shell_items;
        }
        let level = self.current_level_mut();
        level.selected = first_activatable_index(&visible_action_items(level)).unwrap_or(0);
    }

    fn push_selected_submenu(&mut self) -> Option<PendingShellSubmenuLevel> {
        let selected = self
            .visible_actions()
            .get(self.selected())
            .cloned()
            .filter(QuickLaunchActionItem::is_activatable)?;
        let submenu = selected.submenu?;
        let level_id = uuid::Uuid::new_v4();
        self.levels.push(QuickLaunchActionLevel {
            id: level_id,
            title: selected.label,
            items: Vec::new(),
            filter: String::new(),
            selected: 0,
            shell_pending: true,
        });
        Some(PendingShellSubmenuLevel {
            level_id,
            session_id: submenu.session_id,
            submenu_id: submenu.submenu_id,
        })
    }

    fn pop_level(&mut self) -> Option<String> {
        if self.levels.len() <= 1 {
            return None;
        }
        self.levels.pop();
        Some(self.current_level().filter.clone())
    }

    fn move_selection(&mut self, delta: isize) {
        let visible = self.visible_actions();
        let selectable = visible
            .iter()
            .enumerate()
            .filter_map(|(index, item)| item.is_activatable().then_some(index))
            .collect::<Vec<_>>();
        if selectable.is_empty() {
            self.current_level_mut().selected = 0;
            return;
        }
        let current = selectable
            .iter()
            .position(|index| *index == self.selected())
            .unwrap_or(0);
        let next = (current as isize + delta).rem_euclid(selectable.len() as isize) as usize;
        self.current_level_mut().selected = selectable[next];
    }

    fn selected_action(&self) -> Option<QuickLaunchAction> {
        self.visible_actions()
            .get(self.selected())
            .filter(|item| item.enabled && item.submenu.is_none())
            .and_then(|item| item.action.clone())
    }
}

fn visible_action_items(level: &QuickLaunchActionLevel) -> Vec<QuickLaunchActionItem> {
    if level.shell_pending {
        return Vec::new();
    }
    let filter = level.filter.trim().to_lowercase();
    level
        .items
        .iter()
        .filter(|item| filter.is_empty() || item.label.to_lowercase().contains(&filter))
        .cloned()
        .collect()
}

fn first_activatable_index(items: &[QuickLaunchActionItem]) -> Option<usize> {
    items.iter().position(QuickLaunchActionItem::is_activatable)
}

fn quick_launch_action_available(result: &SearchResult, action: &QuickLaunchAction) -> bool {
    match action {
        QuickLaunchAction::Open => result.opens_target(),
        QuickLaunchAction::SwitchDirectory {
            host,
            window_handle,
            target_directory,
            host_context_id,
        } => {
            *window_handle != 0
                && Path::new(target_directory).is_absolute()
                && (*host != DirectoryNavigationHost::ThirdPartyFileManager
                    || host_context_id.is_some())
        }
        QuickLaunchAction::Reveal => {
            result.opens_target()
                && !matches!(
                    result.entry.source,
                    ApplicationSource::System | ApplicationSource::WebUrl
                )
        }
        QuickLaunchAction::CopyPath => {
            result.opens_target() && !result.entry.target.trim().is_empty()
        }
        QuickLaunchAction::CopyResult => result.copy_text().is_some_and(|text| !text.is_empty()),
        QuickLaunchAction::CopyCommand => {
            result.entry.source == ApplicationSource::CustomCommand
                && result
                    .subtitle
                    .as_ref()
                    .is_some_and(|text| !text.is_empty())
        }
        QuickLaunchAction::RunElevated => {
            result.opens_target()
                && (result.entry.source == ApplicationSource::System
                    || PathBuf::from(&result.entry.target)
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .is_some_and(|extension| {
                            extension.eq_ignore_ascii_case("exe")
                                || extension.eq_ignore_ascii_case("lnk")
                        }))
        }
        QuickLaunchAction::Shell { .. } => true,
    }
}

fn quick_launch_action_for_enter(secondary: bool, shift: bool) -> QuickLaunchAction {
    match (secondary, shift) {
        (true, true) => QuickLaunchAction::RunElevated,
        (true, false) => QuickLaunchAction::Reveal,
        _ => QuickLaunchAction::Open,
    }
}

fn quick_launch_primary_action(
    result: &SearchResult,
    secondary: bool,
    shift: bool,
) -> QuickLaunchAction {
    if result.prepared_command().is_some() {
        QuickLaunchAction::Open
    } else if result.copy_text().is_some() {
        QuickLaunchAction::CopyResult
    } else if result.entry.source == ApplicationSource::WebUrl {
        QuickLaunchAction::Open
    } else {
        quick_launch_action_for_enter(secondary, shift)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct QuickLaunchResultCapabilities {
    preview: bool,
    shell_actions: bool,
    shell_icon: bool,
}

fn quick_launch_result_capabilities(result: &SearchResult) -> QuickLaunchResultCapabilities {
    let uses_native_file_integration =
        result.opens_target() && result.entry.source != ApplicationSource::WebUrl;
    QuickLaunchResultCapabilities {
        preview: uses_native_file_integration,
        shell_actions: uses_native_file_integration,
        shell_icon: uses_native_file_integration
            || result.entry.source == ApplicationSource::CustomCommand,
    }
}

fn shell_action_for(
    source: ApplicationSource,
    action: &QuickLaunchAction,
) -> Option<ShellOpenAction> {
    if source == ApplicationSource::Calculator {
        return None;
    }
    if source == ApplicationSource::CustomCommand {
        return None;
    }
    match action {
        QuickLaunchAction::Open if source == ApplicationSource::System => {
            Some(ShellOpenAction::SystemTool)
        }
        QuickLaunchAction::Open => Some(ShellOpenAction::Open),
        QuickLaunchAction::SwitchDirectory { .. } => None,
        QuickLaunchAction::Reveal if source == ApplicationSource::System => None,
        QuickLaunchAction::Reveal => Some(ShellOpenAction::SelectInExplorer),
        QuickLaunchAction::CopyPath
        | QuickLaunchAction::CopyResult
        | QuickLaunchAction::CopyCommand => None,
        QuickLaunchAction::RunElevated => Some(ShellOpenAction::RunElevated),
        QuickLaunchAction::Shell { .. } => None,
    }
}

fn platform_command_for_action(
    result: &SearchResult,
    action: &QuickLaunchAction,
    owner_window_handle: i64,
) -> Option<PlatformCommand> {
    match action {
        QuickLaunchAction::Open => result
            .prepared_command()
            .map(|prepared| {
                PlatformCommand::LaunchProcess(LaunchProcessCommand {
                    executable: prepared.executable.clone(),
                    arguments: prepared.arguments.clone(),
                    working_directory: prepared.working_directory.clone(),
                    run_as_admin: prepared.run_as_admin,
                    run_silently: prepared.run_silently,
                })
            })
            .or_else(|| {
                shell_action_for(result.entry.source, action).map(|shell_action| {
                    PlatformCommand::OpenShellTarget(OpenShellTargetCommand {
                        target: result.entry.target.clone(),
                        action: shell_action,
                    })
                })
            }),
        QuickLaunchAction::CopyPath => Some(PlatformCommand::PasteText(PasteTextCommand {
            text: result.entry.target.clone(),
            paste_after_write: false,
            target_window_handle: 0,
            target_focus_window_handle: 0,
        })),
        QuickLaunchAction::CopyResult => Some(PlatformCommand::PasteText(PasteTextCommand {
            text: result.copy_text()?.to_owned(),
            paste_after_write: false,
            target_window_handle: 0,
            target_focus_window_handle: 0,
        })),
        QuickLaunchAction::CopyCommand => Some(PlatformCommand::PasteText(PasteTextCommand {
            text: result.subtitle.clone()?,
            paste_after_write: false,
            target_window_handle: 0,
            target_focus_window_handle: 0,
        })),
        QuickLaunchAction::SwitchDirectory {
            host,
            window_handle,
            target_directory,
            host_context_id,
        } => Some(PlatformCommand::NavigateDirectory(
            NavigateDirectoryCommand {
                host: *host,
                window_handle: *window_handle,
                target_directory: target_directory.clone(),
                host_context_id: host_context_id.clone(),
            },
        )),
        QuickLaunchAction::Shell {
            session_id,
            command_id,
        } => Some(PlatformCommand::InvokeShellAction(
            InvokeShellActionCommand {
                session_id: session_id.clone(),
                command_id: *command_id,
                owner_window_handle,
            },
        )),
        _ => shell_action_for(result.entry.source, action).map(|shell_action| {
            PlatformCommand::OpenShellTarget(OpenShellTargetCommand {
                target: result.entry.target.clone(),
                action: shell_action,
            })
        }),
    }
}

fn selection_after_results_refresh(
    current_results: &[SearchResult],
    current_index: usize,
    refreshed_results: &[SearchResult],
) -> usize {
    current_results
        .get(current_index)
        .map(|result| result.entry.id.as_str())
        .and_then(|selected_id| {
            refreshed_results
                .iter()
                .position(|result| result.entry.id == selected_id)
        })
        .unwrap_or(0)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum QuickLaunchWindowActivationStep {
    PreTopmost(u8),
    Activate(u8),
    ConfirmTopmost(u8),
}

#[derive(Clone, Copy)]
struct PendingWindowActivation {
    window_handle: i64,
    step: QuickLaunchWindowActivationStep,
}

fn next_window_activation_step(
    step: QuickLaunchWindowActivationStep,
    success: bool,
) -> Option<QuickLaunchWindowActivationStep> {
    use QuickLaunchWindowActivationStep::*;

    match step {
        PreTopmost(attempt) if !success && attempt < 1 => Some(PreTopmost(attempt + 1)),
        PreTopmost(_) => Some(Activate(0)),
        Activate(attempt) if !success && attempt < 1 => Some(Activate(attempt + 1)),
        Activate(_) => Some(ConfirmTopmost(0)),
        ConfirmTopmost(_) if success => None,
        ConfirmTopmost(attempt) if attempt < 2 => Some(ConfirmTopmost(attempt + 1)),
        ConfirmTopmost(_) => None,
    }
}

fn should_focus_search_after_activation(
    step: QuickLaunchWindowActivationStep,
    success: bool,
) -> bool {
    matches!(step, QuickLaunchWindowActivationStep::Activate(_)) && success
}

fn schedule_application_update<F>(
    application: WeakEntity<QuickLaunchApplication>,
    update: F,
    cx: &mut App,
) where
    F: FnOnce(&mut QuickLaunchApplication, &mut Context<QuickLaunchApplication>) + 'static,
{
    cx.defer(move |cx| {
        let _ = application.update(cx, update);
    });
}

impl QuickLaunchApplication {
    pub(crate) fn new(
        catalog: FinderCatalog,
        platform: Rc<PlatformBridge>,
        indexer: Rc<IndexerBridge>,
        config: AppConfig,
        config_path: PathBuf,
        log_dir: PathBuf,
        notifications: Entity<NotificationCenter>,
    ) -> Self {
        Self {
            catalog: Rc::new(RefCell::new(catalog)),
            catalog_refresh_in_progress: false,
            platform,
            indexer,
            config,
            config_path,
            log_dir,
            notifications,
            popup: None,
            preview: QuickLaunchPreviewSession::default(),
            pending_configuration: HashSet::new(),
            pending_hotkey: None,
            hotkey_error: None,
            pending_recording: None,
            recording_active: false,
            recording_desired: false,
            pending_actions: HashMap::new(),
            pending_auto_quick_switches: HashMap::new(),
            file_dialog_search: FileDialogSearchSession::default(),
            navigation_foreground_guard: None,
            popup_session_id: uuid::Uuid::new_v4(),
            active_summon: None,
            pending_shell_queries: HashMap::new(),
            pending_window_activation: HashMap::new(),
            icon_resources: IconResourceCache::bounded(ICON_RESOURCE_CACHE_LIMIT),
            pending_icons: HashMap::new(),
            pending_icon_resources: HashMap::new(),
            failed_icon_resources: HashMap::new(),
            indexer_service_bootstrap: IndexerServiceBootstrap::default(),
            pending_exclusion_picker: None,
            pending_program_picker: None,
            elevated_helper_request: None,
            pending_elevated_helper: None,
            acknowledged_elevated_helper_pipe: None,
            last_sent_index_settings: None,
            configuration_status: "等待 Windows 适配器".into(),
            index_status: None,
            index_search: IndexSearchSession::default(),
            last_local_search_us: None,
        }
    }

    pub(crate) fn settings(&self) -> QuickLaunchSettings {
        self.config.quick_launch.clone()
    }

    pub(crate) fn catalog_status(&self) -> CatalogStatus {
        self.catalog.borrow().status().clone()
    }

    pub(crate) fn index_status(&self) -> Option<IndexStatus> {
        self.index_status.clone()
    }

    pub(crate) fn indexer_connected(&self) -> bool {
        self.indexer.is_connected()
    }

    pub(crate) fn history_summary(&self) -> QuickLaunchHistorySummary {
        self.catalog.borrow().history_summary()
    }

    pub(crate) fn performance_snapshot(&self) -> QuickLaunchPerformanceSnapshot {
        QuickLaunchPerformanceSnapshot {
            last_local_search_us: self.last_local_search_us,
            last_index_search_us: self.index_search.last_completed_elapsed_us,
        }
    }

    pub(crate) fn log_directory(&self) -> &Path {
        &self.log_dir
    }

    pub(crate) fn clear_history(&mut self, cx: &mut Context<Self>) {
        let clear_result = self.catalog.borrow_mut().clear_history();
        match clear_result {
            Ok(()) => {
                let panel = self.popup.as_ref().map(|popup| popup.panel.clone());
                let notifications = self.notifications.clone();
                cx.defer(move |cx| {
                    if let Some(panel) = panel {
                        let _ = panel.update(cx, |panel, cx| {
                            panel.recent_directories.clear();
                            if panel.recent_directory_mode {
                                panel.recent_directory_mode = false;
                                panel.results.clear();
                                panel.selected = 0;
                            }
                            cx.notify();
                        });
                    }
                    notifications.update(cx, |notifications, cx| {
                        notifications.show_success(
                            "quick-launch-history",
                            "速启历史已清空",
                            "排序学习、关键词关联和最近目录已重置。",
                            cx,
                        );
                    });
                });
                cx.notify();
            }
            Err(error) => {
                error!(%error, "failed to clear quick-launch history");
                let notifications = self.notifications.clone();
                let message = error.to_string();
                cx.defer(move |cx| {
                    notifications.update(cx, |notifications, cx| {
                        notifications.show_error(
                            "quick-launch-history",
                            "无法清空速启历史",
                            message,
                            cx,
                        );
                    });
                });
            }
        }
    }

    pub(crate) fn open_log_directory(&self) {
        if let Err(error) =
            self.platform
                .send_command(PlatformCommand::OpenShellTarget(OpenShellTargetCommand {
                    target: self.log_dir.to_string_lossy().into_owned(),
                    action: ShellOpenAction::Open,
                }))
        {
            error!(%error, path = %self.log_dir.display(), "failed to queue log directory open");
        }
    }

    pub(crate) fn prepare_file_index(&mut self, cx: &mut Context<Self>) {
        if self.indexer.is_connected() {
            self.refresh_index_volumes(cx);
            return;
        }
        self.ensure_indexer_service(false, cx);
    }

    fn ensure_indexer_service(&mut self, automatic: bool, cx: &mut Context<Self>) {
        if self.indexer_service_bootstrap.pending.is_some() {
            return;
        }
        match self
            .platform
            .send_command(PlatformCommand::EnsureIndexerService)
        {
            Ok(command_id) => {
                self.indexer_service_bootstrap.begin(command_id, automatic);
                if !automatic {
                    self.notifications.update(cx, |notifications, cx| {
                        notifications.show_info(
                            "quick-launch-indexer-service",
                            "正在准备索引服务",
                            "首次安装会显示一次 Windows 授权窗口，主程序仍以普通权限运行。",
                            cx,
                        );
                    });
                }
            }
            Err(error) => {
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_error(
                        "quick-launch-indexer-service",
                        "无法准备索引服务",
                        error.to_string(),
                        cx,
                    );
                });
            }
        }
        cx.notify();
    }

    pub(crate) fn refresh_index_volumes(&mut self, cx: &mut Context<Self>) {
        match self
            .indexer
            .send_command(nxtools_protocol::IndexerCommand::RefreshVolumes)
        {
            Ok(_) => {
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_info(
                        "quick-launch-volume-refresh",
                        "重建本地文件索引",
                        "正在重新发现 NTFS 卷并建立新的 MFT 快照",
                        cx,
                    );
                });
            }
            Err(error) => {
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_error(
                        "quick-launch-volume-refresh",
                        "无法重建本地文件索引",
                        error.to_string(),
                        cx,
                    );
                });
            }
        }
    }

    pub(crate) fn set_indexed_volumes(
        &mut self,
        indexed_volumes: Vec<String>,
        cx: &mut Context<Self>,
    ) {
        self.config.quick_launch.indexed_volumes = indexed_volumes;
        self.persist_and_sync_index_settings(cx);
    }

    pub(crate) fn set_auto_quick_switch(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.config.quick_launch.auto_quick_switch == enabled {
            return;
        }
        self.config.quick_launch.auto_quick_switch = enabled;
        self.persist_and_sync_quick_launch_behavior(cx);
    }

    pub(crate) fn set_file_dialog_search(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.config.quick_launch.file_dialog_search == enabled {
            return;
        }
        self.config.quick_launch.file_dialog_search = enabled;
        if !enabled {
            self.hide_file_dialog_search_window(cx);
        }
        self.persist_and_sync_quick_launch_behavior(cx);
    }

    pub(crate) fn add_custom_command(&mut self, cx: &mut Context<Self>) -> usize {
        let index = self.config.quick_launch.custom_commands.len();
        self.config
            .quick_launch
            .custom_commands
            .push(nxtools_core::CustomCommandSettings {
                enabled: false,
                title: "新命令".into(),
                ..nxtools_core::CustomCommandSettings::default()
            });
        self.persist_custom_commands(cx);
        index
    }

    pub(crate) fn update_custom_command(
        &mut self,
        index: usize,
        command: nxtools_core::CustomCommandSettings,
        cx: &mut Context<Self>,
    ) -> bool {
        let mut next = self.config.quick_launch.custom_commands.clone();
        let Some(slot) = next.get_mut(index) else {
            return false;
        };
        *slot = command;
        let mut settings = self.config.quick_launch.clone();
        settings.custom_commands = next.clone();
        settings.normalize();
        let validation = settings.validate_custom_commands().and_then(|()| {
            settings
                .custom_commands
                .iter()
                .filter(|command| command.enabled)
                .try_for_each(|command| {
                    nxtools_finder::validate_custom_command_template(&command.argument_template)
                        .map_err(|error| format!("命令“{}”：{error}", command.keyword))
                })
        });
        if let Err(error) = validation {
            self.notifications.update(cx, |notifications, cx| {
                notifications.show_error(
                    "quick-launch-custom-command-validation",
                    "无法保存自定义命令",
                    error,
                    cx,
                );
            });
            return false;
        }
        self.config.quick_launch.custom_commands = settings.custom_commands;
        self.persist_custom_commands(cx)
    }

    pub(crate) fn remove_custom_command(&mut self, index: usize, cx: &mut Context<Self>) -> bool {
        if index >= self.config.quick_launch.custom_commands.len() {
            return false;
        }
        self.config.quick_launch.custom_commands.remove(index);
        self.persist_custom_commands(cx)
    }

    fn persist_custom_commands(&mut self, cx: &mut Context<Self>) -> bool {
        if let Err(error) = self.config.save_to(&self.config_path) {
            self.notifications.update(cx, |notifications, cx| {
                notifications.show_error(
                    "quick-launch-custom-command-save",
                    "无法保存自定义命令",
                    error.to_string(),
                    cx,
                );
            });
            return false;
        }
        self.index_search.invalidate();
        cx.notify();
        true
    }

    fn persist_and_sync_quick_launch_behavior(&mut self, cx: &mut Context<Self>) {
        if let Err(error) = self.config.save_to(&self.config_path) {
            self.notifications.update(cx, |notifications, cx| {
                notifications.show_error(
                    "quick-launch-context-settings",
                    "无法保存文件对话框设置",
                    error.to_string(),
                    cx,
                );
            });
            return;
        }
        self.send_configuration();
        cx.notify();
    }

    pub(crate) fn remove_excluded_path(&mut self, path: &str, cx: &mut Context<Self>) {
        self.config
            .quick_launch
            .excluded_paths
            .retain(|candidate| !candidate.eq_ignore_ascii_case(path));
        self.persist_and_sync_index_settings(cx);
    }

    pub(crate) fn request_index_exclusion_directory(&mut self, cx: &mut Context<Self>) {
        if self.pending_exclusion_picker.is_some() {
            return;
        }
        let current_directory = self
            .config
            .quick_launch
            .excluded_paths
            .last()
            .cloned()
            .unwrap_or_default();
        match self
            .platform
            .send_command(PlatformCommand::PickDirectory(PickDirectoryCommand {
                current_directory,
                title: "选择要从速启搜索中排除的目录".into(),
            })) {
            Ok(command_id) => self.pending_exclusion_picker = Some(command_id),
            Err(error) => {
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_error(
                        "quick-launch-index-exclusion-picker",
                        "无法打开目录选择器",
                        error.to_string(),
                        cx,
                    );
                });
            }
        }
        cx.notify();
    }

    pub(crate) fn request_custom_command_program(
        &mut self,
        index: usize,
        current_path: String,
        cx: &mut Context<Self>,
    ) {
        if self.pending_program_picker.is_some()
            || index >= self.config.quick_launch.custom_commands.len()
        {
            return;
        }
        match self
            .platform
            .send_command(PlatformCommand::PickProgram(PickProgramCommand {
                current_path,
            })) {
            Ok(command_id) => self.pending_program_picker = Some((command_id, index)),
            Err(error) => {
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_error(
                        "quick-launch-command-program-picker",
                        "无法打开程序选择器",
                        error.to_string(),
                        cx,
                    );
                });
            }
        }
        cx.notify();
    }

    fn persist_and_sync_index_settings(&mut self, cx: &mut Context<Self>) {
        self.config.quick_launch.normalize();
        if let Err(error) = self.config.save_to(&self.config_path) {
            self.notifications.update(cx, |notifications, cx| {
                notifications.show_error(
                    "quick-launch-index-settings",
                    "无法保存索引设置",
                    error.to_string(),
                    cx,
                );
            });
            return;
        }
        self.last_sent_index_settings = None;
        self.sync_index_settings();
        self.index_search.invalidate();
        cx.notify();
    }

    fn sync_index_settings(&mut self) {
        if !self.indexer.is_connected() {
            return;
        }
        let settings = index_settings_from_quick_launch(&self.config.quick_launch);
        if self.last_sent_index_settings.as_ref() == Some(&settings) {
            return;
        }
        match self
            .indexer
            .send_command(IndexerCommand::UpdateSettings(settings.clone()))
        {
            Ok(_) => self.last_sent_index_settings = Some(settings),
            Err(error) => warn!(%error, "could not synchronize quick-launch index settings"),
        }
    }

    pub(crate) fn handle_indexer_notification(
        &mut self,
        notification: IndexerNotification,
        cx: &mut Context<Self>,
    ) {
        if let IndexerNotification::Event(IndexerEvent::SearchBatch(batch)) = &notification
            && self.handle_file_dialog_search_batch(batch, cx)
        {
            return;
        }
        if matches!(&notification, IndexerNotification::Disconnected) {
            self.handle_file_dialog_search_disconnect(cx);
        }
        match &notification {
            IndexerNotification::Event(IndexerEvent::Status(status))
                if index_status_affects_presentation(self.index_status.as_ref(), status) =>
            {
                info!(
                phase = ?status.phase,
                indexed_items = status.indexed_items,
                "quick-launch index provider status changed"
                )
            }
            IndexerNotification::Event(IndexerEvent::Status(_)) => {}
            IndexerNotification::Event(IndexerEvent::SearchBatch(batch)) => info!(
                batch_generation = batch.generation,
                active_generation = self.index_search.generation,
                item_count = batch.items.len(),
                complete = batch.complete,
                accepted = batch.generation == self.index_search.generation,
                "quick-launch index provider batch received"
            ),
            IndexerNotification::Event(IndexerEvent::ElevatedDirectoryHelper(status)) => {
                if status.launched {
                    info!("elevated directory helper launch was accepted");
                } else {
                    warn!(
                        reason = status.error.as_deref().unwrap_or("unknown"),
                        "elevated directory helper launch failed"
                    );
                }
            }
            IndexerNotification::Disconnected => {
                warn!("quick-launch index provider disconnected")
            }
        }
        if matches!(
            &notification,
            IndexerNotification::Event(IndexerEvent::Status(_))
        ) {
            self.sync_index_settings();
            self.sync_elevated_directory_helper();
        } else if let IndexerNotification::Event(IndexerEvent::ElevatedDirectoryHelper(status)) =
            &notification
        {
            let is_current = elevated_helper_status_matches_request(
                self.elevated_helper_request.as_ref(),
                status,
            );
            if is_current {
                self.pending_elevated_helper = None;
                self.acknowledged_elevated_helper_pipe = Some(status.pipe_name.clone());
            }
        } else if matches!(&notification, IndexerNotification::Disconnected) {
            self.last_sent_index_settings = None;
            if self.pending_elevated_helper.is_some() {
                self.pending_elevated_helper = None;
                self.acknowledged_elevated_helper_pipe = None;
            }
        }
        let retry_current_query = match &notification {
            IndexerNotification::Event(IndexerEvent::Status(status)) => {
                should_retry_index_search_after_status(self.index_status.as_ref(), status)
            }
            _ => false,
        };
        let refresh_results = matches!(
            &notification,
            IndexerNotification::Event(IndexerEvent::SearchBatch(batch))
                if batch.generation == self.index_search.generation && batch.complete
        );
        let indexer_disconnected = matches!(&notification, IndexerNotification::Disconnected);
        if apply_indexer_notification(&mut self.index_status, &mut self.index_search, notification)
        {
            if retry_current_query {
                self.retry_index_search_after_ready(cx);
            }
            if refresh_results {
                self.refresh_indexed_results(cx);
            }
            if indexer_disconnected {
                self.refresh_results_after_indexer_disconnect(cx);
            }
            cx.notify();
        }
    }

    fn retry_index_search_after_ready(&mut self, cx: &mut Context<Self>) {
        let Some(popup) = self.popup.clone() else {
            return;
        };
        let panel_state = popup
            .panel
            .update(cx, |panel, cx| {
                (
                    panel.is_open,
                    panel.action_mode.is_none(),
                    panel.search_input.read(cx).value().to_string(),
                )
            })
            .ok();
        let Some((true, true, query)) = panel_state else {
            return;
        };
        if query.trim().is_empty() {
            return;
        }
        info!(
            query_length = query.chars().count(),
            "retrying visible quick-launch query after indexer became ready"
        );
        let update = self.search_and_request_icons(&query, cx);
        let _ = popup.panel.update(cx, |panel, cx| {
            panel.search_pending = update.pending;
            if update.pending {
                cx.notify();
            } else {
                panel.replace_results(update.results, cx);
            }
        });
    }

    pub(crate) fn hotkey_lifecycle(&self) -> QuickLaunchHotkeyLifecycle {
        if let Some((_, hotkey)) = &self.pending_hotkey {
            return QuickLaunchHotkeyLifecycle::Applying {
                candidate: Some(hotkey.display.clone()),
            };
        }
        if let Some(error) = &self.hotkey_error {
            return QuickLaunchHotkeyLifecycle::Failed {
                error: error.clone(),
            };
        }
        if self.hotkey_recording_active() {
            return if self.hotkey_recording_ready() {
                QuickLaunchHotkeyLifecycle::Recording
            } else {
                QuickLaunchHotkeyLifecycle::WaitingForRecording {
                    status: self.configuration_status.clone(),
                }
            };
        }
        if !self.pending_configuration.is_empty() {
            return QuickLaunchHotkeyLifecycle::Applying { candidate: None };
        }
        if self.configuration_status.starts_with("等待") {
            return QuickLaunchHotkeyLifecycle::WaitingForAdapter {
                status: self.configuration_status.clone(),
            };
        }
        if self.configuration_status.starts_with("应用失败") {
            return QuickLaunchHotkeyLifecycle::Failed {
                error: self.configuration_status.clone(),
            };
        }
        QuickLaunchHotkeyLifecycle::Applied
    }

    pub(crate) fn hotkey_recording_ready(&self) -> bool {
        self.recording_active && self.pending_recording.is_none()
    }

    pub(crate) fn hotkey_recording_active(&self) -> bool {
        self.recording_active || self.recording_desired || self.pending_recording.is_some()
    }

    pub(crate) fn set_hotkey_recording(&mut self, recording: bool) {
        if recording {
            self.hotkey_error = None;
        }
        self.recording_desired = recording;
        self.drive_hotkey_recording();
    }

    pub(crate) fn set_hotkey(&mut self, hotkey: ClipboardHotKey) {
        if self.pending_hotkey.is_some() || !self.hotkey_recording_ready() {
            return;
        }
        self.recording_desired = false;
        self.recording_active = false;
        self.hotkey_error = None;
        let mut settings = self.config.quick_launch.clone();
        settings.hotkey = hotkey.clone();
        if let Some(command_id) = self.send_configuration_for(&settings) {
            self.pending_hotkey = Some((command_id, hotkey));
        } else {
            self.hotkey_error = Some(self.configuration_status.clone());
        }
    }

    pub(crate) fn handle_platform_notification(
        &mut self,
        notification: PlatformNotification,
        cx: &mut Context<Self>,
    ) {
        match notification {
            PlatformNotification::Connected => {
                let prepare_indexer = self.indexer_service_bootstrap.on_platform_connected();
                self.send_configuration();
                if prepare_indexer {
                    self.ensure_indexer_service(true, cx);
                }
            }
            PlatformNotification::Event(event) => self.handle_platform_event(event, cx),
            PlatformNotification::CommandResult {
                correlation_id,
                result,
            } => self.handle_command_result(correlation_id, result, cx),
        }
    }

    fn handle_platform_event(&mut self, event: PlatformEvent, cx: &mut Context<Self>) {
        match event {
            PlatformEvent::ElevatedDirectoryHelperRequested(request) => {
                self.elevated_helper_request = Some(request);
                self.pending_elevated_helper = None;
                self.acknowledged_elevated_helper_pipe = None;
                self.sync_elevated_directory_helper();
            }
            PlatformEvent::QuickLaunchPanelRequested(request) => {
                let age = nxtools_ipc::unix_time_ms().saturating_sub(request.requested_at_unix_ms);
                if age <= REQUEST_MAX_AGE_MS && !self.hotkey_recording_active() {
                    self.toggle_panel(request, cx);
                }
            }
            PlatformEvent::QuickLaunchDirectoryContextResolved(resolution) => {
                self.apply_resolved_directory_context(resolution, cx);
            }
            PlatformEvent::QuickLaunchAutoSwitchRequested(request) => {
                self.handle_auto_quick_switch_request(request);
            }
            PlatformEvent::QuickLaunchFileDialogSearchHostChanged(host) => {
                self.handle_file_dialog_search_host(host, cx);
            }
            PlatformEvent::ForegroundChanged(foreground) => {
                let pending_navigation_owns_foreground =
                    self.pending_actions.values().any(|pending| {
                        pending_action_owns_foreground(
                            pending,
                            self.popup_session_id,
                            foreground.window_handle,
                        )
                    });
                if pending_navigation_owns_foreground
                    || navigation_guard_owns_foreground(
                        &mut self.navigation_foreground_guard,
                        self.popup_session_id,
                        foreground.window_handle,
                        Instant::now(),
                    )
                {
                    return;
                }
                let Some(popup) = self.popup.clone() else {
                    return;
                };
                let preview_window_handle = self
                    .preview
                    .window
                    .as_ref()
                    .map_or(0, |preview| preview.identity.native);
                let should_hide = popup
                    .window
                    .update(cx, |_, _window, app| {
                        popup.panel.update(app, |panel, _| {
                            panel.should_dismiss_for_foreground(
                                popup.native_window_handle,
                                preview_window_handle,
                                foreground.window_handle,
                            )
                        })
                    })
                    .ok()
                    .and_then(Result::ok)
                    .unwrap_or(false);
                if should_hide {
                    info!(
                        dismissal_source = "foreground_changed",
                        popup_window_handle = popup.native_window_handle,
                        preview_window_handle,
                        foreground_window_handle = foreground.window_handle,
                        foreground_process_id = foreground.process_id,
                        foreground_process_name = %foreground.process_name,
                        foreground_window_title = %foreground.window_title,
                        "quick-launch popup auto-dismissed"
                    );
                    self.hide_popup(cx);
                }
            }
            PlatformEvent::GlobalPointerPressed(pointer) => {
                let Some(popup) = self.popup.clone() else {
                    return;
                };
                if pointer.root_window_handle == popup.native_window_handle
                    && pointer.process_id == std::process::id()
                {
                    self.suspend_quick_launch_preview_for_scroll(cx);
                }
                let should_hide = popup
                    .window
                    .update(cx, |_, _window, app| {
                        popup.panel.update(app, |panel, _| {
                            panel.should_dismiss_for_pointer(
                                popup.native_window_handle,
                                pointer.root_window_handle,
                                pointer.process_id,
                            )
                        })
                    })
                    .ok()
                    .and_then(Result::ok)
                    .unwrap_or(false);
                if should_hide {
                    info!(
                        dismissal_source = "global_pointer_pressed",
                        popup_window_handle = popup.native_window_handle,
                        pointer_root_window_handle = pointer.root_window_handle,
                        pointer_process_id = pointer.process_id,
                        desktop_process_id = std::process::id(),
                        "quick-launch popup auto-dismissed"
                    );
                    self.hide_popup(cx);
                }
            }
            PlatformEvent::TrayActionRequested(_)
            | PlatformEvent::ClipboardCaptured(_)
            | PlatformEvent::ClipboardPanelRequested(_)
            | PlatformEvent::TaskbarOverlayEditGestureFinished(_) => {}
        }
    }

    fn handle_auto_quick_switch_request(
        &mut self,
        request: nxtools_protocol::QuickLaunchAutoSwitchRequest,
    ) {
        let Some(command) = auto_quick_switch_command(
            &self.config.quick_launch,
            &request,
            nxtools_ipc::unix_time_ms(),
        ) else {
            return;
        };
        match self
            .platform
            .send_command(PlatformCommand::NavigateDirectory(command))
        {
            Ok(correlation_id) => {
                self.pending_auto_quick_switches.insert(
                    correlation_id,
                    (
                        request.target.window_handle,
                        request.source_directory.clone(),
                    ),
                );
            }
            Err(error) => {
                warn!(
                    %error,
                    target_window_handle = request.target.window_handle,
                    source_directory = %request.source_directory,
                    "could not queue automatic file-dialog directory switch"
                );
            }
        }
    }

    fn sync_elevated_directory_helper(&mut self) {
        let Some(command) = elevated_directory_helper_command(
            self.elevated_helper_request.as_ref(),
            self.pending_elevated_helper.is_some(),
            self.acknowledged_elevated_helper_pipe.as_deref(),
            self.indexer.is_connected(),
        ) else {
            return;
        };
        match self.indexer.send_command(command) {
            Ok(command_id) => self.pending_elevated_helper = Some(command_id),
            Err(error) => {
                warn!(%error, "could not request the elevated directory helper");
            }
        }
    }

    fn apply_resolved_directory_context(
        &mut self,
        resolution: nxtools_protocol::QuickLaunchDirectoryContextResolution,
        cx: &mut Context<Self>,
    ) {
        if !directory_context_resolution_matches(
            self.active_summon,
            &resolution,
            nxtools_ipc::unix_time_ms(),
        ) {
            return;
        }
        let recent_directories = self
            .catalog
            .borrow()
            .recent_directories(RECENT_DIRECTORY_LIMIT);
        let context = resolution.directory_navigation_context;
        let Some(popup) = self.popup.clone() else {
            return;
        };
        let applied = popup
            .panel
            .update(cx, |panel, cx| {
                if !panel.is_open || panel.target.window_handle != resolution.target_window_handle {
                    return false;
                }
                panel.navigation_context = Some(context.clone());
                panel.recent_directories = recent_directories;
                cx.notify();
                true
            })
            .unwrap_or(false);
        if applied {
            self.schedule_captured_directory_record(Some(&context), cx);
        }
    }

    fn handle_command_result(
        &mut self,
        correlation_id: uuid::Uuid,
        result: PlatformCommandResult,
        cx: &mut Context<Self>,
    ) {
        if self.handle_quick_launch_preview_stage_result(
            correlation_id,
            result.success,
            result.error.as_deref(),
            cx,
        ) {
            return;
        }
        if let Some((target_window_handle, source_directory)) =
            self.pending_auto_quick_switches.remove(&correlation_id)
        {
            if result.success {
                self.update_file_dialog_search_directory(
                    target_window_handle,
                    source_directory,
                    cx,
                );
            } else {
                warn!(
                    reason = result.error.as_deref().unwrap_or("unknown Windows error"),
                    "automatic file-dialog directory switch failed"
                );
            }
            return;
        }
        if let Some(pending) = self.pending_shell_queries.remove(&correlation_id) {
            let menu = if result.success {
                result.data.as_deref().and_then(|data| {
                    serde_json::from_str::<ShellActionMenu>(data)
                        .map_err(|error| {
                            warn!(%error, "could not decode Windows Shell action menu");
                            error
                        })
                        .ok()
                })
            } else {
                warn!(
                    reason = result.error.as_deref().unwrap_or("unknown Windows error"),
                    "could not enumerate Windows Shell actions"
                );
                None
            };
            self.apply_shell_actions(
                pending.mode_id,
                pending.level_id,
                &pending.entry_id,
                menu,
                cx,
            );
            return;
        }
        if self.pending_exclusion_picker == Some(correlation_id) {
            self.pending_exclusion_picker = None;
            if result.success {
                if let Some(path) = result.data.as_deref().map(str::trim)
                    && !path.is_empty()
                    && !self
                        .config
                        .quick_launch
                        .excluded_paths
                        .iter()
                        .any(|candidate| candidate.eq_ignore_ascii_case(path))
                {
                    self.config.quick_launch.excluded_paths.push(path.into());
                    self.persist_and_sync_index_settings(cx);
                }
            } else {
                let reason = result
                    .error
                    .unwrap_or_else(|| "Windows 未返回错误详情".into());
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_error(
                        "quick-launch-index-exclusion-picker",
                        "无法选择排除目录",
                        reason,
                        cx,
                    );
                });
            }
            cx.notify();
            return;
        }
        if self
            .pending_program_picker
            .as_ref()
            .is_some_and(|(command_id, _)| *command_id == correlation_id)
        {
            let (_, index) = self
                .pending_program_picker
                .take()
                .expect("matching program picker must exist");
            if result.success {
                if let Some(path) = result.data.filter(|path| !path.trim().is_empty()) {
                    cx.emit(QuickLaunchApplicationEvent::CustomCommandProgramPicked {
                        index,
                        path,
                    });
                }
            } else {
                let reason = result
                    .error
                    .unwrap_or_else(|| "Windows 未返回错误详情".into());
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_error(
                        "quick-launch-command-program-picker",
                        "无法选择命令程序",
                        reason,
                        cx,
                    );
                });
            }
            cx.notify();
            return;
        }
        if let Some(automatic) = self.indexer_service_bootstrap.finish(correlation_id) {
            if result.success && result.data.as_deref() == Some("installed") {
                match crate::launch_protected_desktop_after_current_exit() {
                    Ok(true) => {
                        info!("restarting the desktop from the protected service package");
                        cx.quit();
                        return;
                    }
                    Ok(false) => {}
                    Err(error) => {
                        warn!(
                            %error,
                            "could not restart the protected desktop after service installation"
                        );
                    }
                }
            }
            if automatic && result.success && result.data.as_deref() == Some("running") {
                return;
            }
            let reason = result
                .error
                .clone()
                .unwrap_or_else(|| "Windows 未返回错误详情".into());
            self.notifications.update(cx, move |notifications, cx| {
                if result.success {
                    notifications.show_success(
                        "quick-launch-indexer-service",
                        "索引服务已就绪",
                        "正在连接服务并建立 NTFS 文件索引。",
                        cx,
                    );
                } else {
                    notifications.show_error(
                        "quick-launch-indexer-service",
                        "索引服务未安装",
                        reason,
                        cx,
                    );
                }
            });
            cx.notify();
            return;
        }
        if let Some(pending) = self.pending_window_activation.remove(&correlation_id) {
            self.handle_window_activation_result(pending, &result, cx);
            return;
        }
        if let Some(pending) = self.pending_icons.remove(&correlation_id) {
            let PendingIconResource {
                resource_key,
                entry_ids,
            } = pending;
            self.pending_icon_resources.remove(&resource_key);
            let icon_path = result
                .success
                .then_some(result.data.as_deref())
                .flatten()
                .map(PathBuf::from)
                .filter(|path| shell_icon_path_is_renderable(path));
            if let Some(path) = icon_path {
                self.failed_icon_resources.remove(&resource_key);
                self.icon_resources.insert(resource_key, path.clone());
                if let Some(popup) = self.popup.clone() {
                    let _ = popup.panel.update(cx, |panel, cx| {
                        panel.set_icons(
                            entry_ids
                                .into_iter()
                                .map(|entry_id| (entry_id, path.clone())),
                            cx,
                        );
                    });
                }
            } else {
                self.record_icon_failure(resource_key, cx);
            }
            return;
        }
        if self.pending_configuration.remove(&correlation_id) {
            self.configuration_status = if result.success {
                "已启用".into()
            } else {
                format!(
                    "应用失败：{}",
                    result.error.as_deref().unwrap_or("未知错误")
                )
            };
            cx.notify();
        }
        if self
            .pending_recording
            .as_ref()
            .is_some_and(|(command_id, _)| *command_id == correlation_id)
        {
            let Some((_, recording)) = self.pending_recording.take() else {
                return;
            };
            self.recording_active = result.success && recording;
            if !result.success {
                self.recording_desired = false;
                self.configuration_status = format!(
                    "热键录制失败：{}",
                    result.error.as_deref().unwrap_or("未知错误")
                );
                self.hotkey_error = Some(self.configuration_status.clone());
            }
            self.drive_hotkey_recording();
            cx.notify();
        }
        if self
            .pending_hotkey
            .as_ref()
            .is_some_and(|(command_id, _)| *command_id == correlation_id)
        {
            let Some((_, hotkey)) = self.pending_hotkey.take() else {
                return;
            };
            if result.success {
                self.config.quick_launch.hotkey = hotkey;
                if let Err(error) = self.config.save_to(&self.config_path) {
                    self.configuration_status = format!("热键已生效，但保存失败：{error}");
                    self.hotkey_error = Some(self.configuration_status.clone());
                } else {
                    self.configuration_status = "已启用".into();
                    self.hotkey_error = None;
                }
            } else {
                self.hotkey_error = Some(format!(
                    "热键应用失败：{}",
                    result.error.as_deref().unwrap_or("未知错误")
                ));
            }
            cx.notify();
        }
        let Some(pending_action) = self.pending_actions.remove(&correlation_id) else {
            return;
        };
        let elapsed_ms = pending_action
            .started_at
            .elapsed()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX);
        let action_kind = quick_launch_action_diagnostic_label(&pending_action.action);
        let action_correlation_id = pending_action.correlation_id;
        let trigger = pending_action.trigger.diagnostic_label();
        let source = pending_action.source.label();
        if result.success {
            info!(
                event = "quick_launch.action.completed",
                correlation_id = %action_correlation_id,
                action = action_kind,
                trigger,
                source,
                outcome = "success",
                elapsed_ms,
                native_error_code = ?result.native_error_code,
                "quick-launch action completed"
            );
        } else {
            error!(
                event = "quick_launch.action.completed",
                correlation_id = %action_correlation_id,
                action = action_kind,
                trigger,
                source,
                outcome = "failure",
                elapsed_ms,
                native_error_code = ?result.native_error_code,
                error_reported = result.error.is_some(),
                "quick-launch action failed"
            );
        }
        let targets_current_session =
            pending_action_targets_session(&pending_action, self.popup_session_id);
        if result.success {
            match action_success_policy(&pending_action.action) {
                ActionSuccessPolicy::RecordAndHide => {
                    let activated_at_unix_ms = nxtools_ipc::unix_time_ms();
                    {
                        let mut catalog = self.catalog.borrow_mut();
                        if let Err(error) = catalog.record_activation(
                            &pending_action.entry_id,
                            &pending_action.query,
                            activated_at_unix_ms,
                        ) {
                            warn!(%error, "failed to persist quick-launch activation");
                        }
                    }
                    if let Some(directory) = pending_action.directory_visit {
                        self.schedule_directory_record(
                            directory,
                            activated_at_unix_ms,
                            pending_action.popup_session_id,
                            cx,
                        );
                    }
                    if targets_current_session {
                        self.hide_popup(cx);
                    }
                }
                ActionSuccessPolicy::HideWithoutRecording => {
                    if targets_current_session {
                        self.hide_popup(cx);
                    }
                }
                ActionSuccessPolicy::KeepOpen => {
                    if targets_current_session && let Some(popup) = self.popup.clone() {
                        let message = format!("已复制路径：{}", pending_action.target);
                        let _ = popup.window.update(cx, |_, window, app| {
                            let _ = popup.panel.update(app, |panel, cx| {
                                panel.set_status(message, cx);
                                panel.focus_search(window, cx);
                            });
                        });
                    }
                }
            }
        } else {
            let reason = result
                .error
                .unwrap_or_else(|| "Windows 未返回错误详情".into());
            if targets_current_session && let Some(popup) = self.popup.clone() {
                if let QuickLaunchAction::SwitchDirectory { window_handle, .. } =
                    &pending_action.action
                {
                    self.navigation_foreground_guard = Some(NavigationForegroundGuard {
                        popup_session_id: self.popup_session_id,
                        window_handle: *window_handle,
                        expires_at: Instant::now() + FAILED_NAVIGATION_FOREGROUND_GUARD,
                    });
                }
                self.begin_window_activation(popup.native_window_handle);
                let _ = popup.window.update(cx, |_, window, app| {
                    let _ = popup.panel.update(app, |panel, cx| {
                        panel.set_error(
                            format!(
                                "{} {} 失败：{reason}",
                                quick_launch_action_label(&pending_action.action),
                                pending_action.display_name
                            ),
                            cx,
                        );
                        panel.focus_search(window, cx);
                    });
                });
            }
        }
    }

    fn handle_window_activation_result(
        &mut self,
        pending: PendingWindowActivation,
        result: &PlatformCommandResult,
        cx: &mut Context<Self>,
    ) {
        if self
            .popup
            .as_ref()
            .is_none_or(|popup| popup.native_window_handle != pending.window_handle)
        {
            return;
        }

        if should_focus_search_after_activation(pending.step, result.success)
            && let Some(popup) = self.popup.clone()
        {
            let _ = popup.window.update(cx, |_, window, app| {
                let _ = popup
                    .panel
                    .update(app, |panel, cx| panel.focus_search(window, cx));
            });
        }

        let Some(next) = next_window_activation_step(pending.step, result.success) else {
            if result.success {
                info!(
                    window_handle = pending.window_handle,
                    "quick-launch popup activation and topmost confirmed"
                );
            } else {
                warn!(
                    window_handle = pending.window_handle,
                    reason = result.error.as_deref().unwrap_or("unknown Windows error"),
                    "quick-launch popup stayed open after bounded topmost retries were exhausted"
                );
            }
            return;
        };
        self.queue_window_activation_step(pending.window_handle, next);
    }

    fn begin_window_activation(&mut self, window_handle: i64) {
        self.pending_window_activation.clear();
        if window_handle == 0 {
            warn!("quick-launch popup opened without a native Windows handle");
            return;
        }
        self.queue_window_activation_step(
            window_handle,
            QuickLaunchWindowActivationStep::PreTopmost(0),
        );
    }

    fn queue_window_activation_step(
        &mut self,
        window_handle: i64,
        initial_step: QuickLaunchWindowActivationStep,
    ) {
        let mut step = Some(initial_step);
        while let Some(current) = step {
            let command = match current {
                QuickLaunchWindowActivationStep::PreTopmost(_)
                | QuickLaunchWindowActivationStep::ConfirmTopmost(_) => {
                    PlatformCommand::SetWindowTopmost(SetWindowTopmostCommand {
                        window_handle,
                        enabled: true,
                    })
                }
                QuickLaunchWindowActivationStep::Activate(_) => {
                    PlatformCommand::ActivateWindow(ActivateWindowCommand { window_handle })
                }
            };
            match self.platform.send_command(command) {
                Ok(command_id) => {
                    self.pending_window_activation.insert(
                        command_id,
                        PendingWindowActivation {
                            window_handle,
                            step: current,
                        },
                    );
                    return;
                }
                Err(error) => {
                    warn!(
                        %error,
                        window_handle,
                        ?current,
                        "could not queue quick-launch window activation step"
                    );
                    step = next_window_activation_step(current, false);
                }
            }
        }
    }

    fn drive_hotkey_recording(&mut self) {
        if self.pending_recording.is_some() || self.recording_active == self.recording_desired {
            return;
        }
        let recording = self.recording_desired;
        match self
            .platform
            .send_command(PlatformCommand::SetQuickLaunchHotKeyRecording(
                SetQuickLaunchHotKeyRecordingCommand { recording },
            )) {
            Ok(command_id) => {
                self.pending_recording = Some((command_id, recording));
                self.configuration_status = if recording {
                    "正在暂停速启热键".into()
                } else {
                    "正在恢复速启热键".into()
                };
            }
            Err(error) => {
                self.recording_active = false;
                self.recording_desired = false;
                self.configuration_status = format!("热键录制失败：{error}");
                self.hotkey_error = Some(self.configuration_status.clone());
            }
        }
    }

    fn send_configuration(&mut self) {
        let settings = self.config.quick_launch.clone();
        let _ = self.send_configuration_for(&settings);
    }

    fn send_configuration_for(&mut self, settings: &QuickLaunchSettings) -> Option<uuid::Uuid> {
        let command = ConfigureQuickLaunchCommand {
            enabled: settings.enabled,
            auto_quick_switch_enabled: settings.auto_quick_switch,
            file_dialog_search_enabled: settings.file_dialog_search,
            hotkey_modifiers: settings
                .hotkey
                .modifiers
                .iter()
                .map(|modifier| match modifier {
                    ClipboardHotKeyModifier::Control => "control",
                    ClipboardHotKeyModifier::Alt => "alt",
                    ClipboardHotKeyModifier::Shift => "shift",
                    ClipboardHotKeyModifier::Win => "win",
                })
                .map(ToOwned::to_owned)
                .collect(),
            hotkey_key: settings.hotkey.key.clone(),
            hotkey_display: settings.hotkey.display.clone(),
        };
        match self
            .platform
            .send_command(PlatformCommand::ConfigureQuickLaunch(command))
        {
            Ok(command_id) => {
                self.pending_configuration.insert(command_id);
                self.configuration_status = "正在应用".into();
                Some(command_id)
            }
            Err(PlatformBridgeError::Disconnected) => {
                self.configuration_status = "等待 Windows 适配器".into();
                None
            }
            Err(error) => {
                self.configuration_status = format!("应用失败：{error}");
                None
            }
        }
    }

    fn combined_results(&self, query: &str) -> Vec<SearchResult> {
        if query.trim().is_empty() {
            return Vec::new();
        }
        let indexed = if self.index_search.query == query {
            self.index_search.items.as_slice()
        } else {
            &[]
        };
        let indexed = indexed
            .iter()
            .map(|item| {
                let normalized_path = item.path.replace('/', "\\").to_lowercase();
                ExternalSearchEntry {
                    id: format!("indexed:{normalized_path}"),
                    name: item.name.clone(),
                    target: item.path.clone(),
                    source: if item.is_directory {
                        ApplicationSource::IndexedDirectory
                    } else {
                        ApplicationSource::IndexedFile
                    },
                }
            })
            .collect::<Vec<_>>();
        self.catalog
            .borrow()
            .search_with_sources(
                query,
                &indexed,
                &self.config.quick_launch.custom_commands,
                RESULT_LIMIT,
            )
            .unwrap_or_else(|error| {
                error!(%error, %query, "quick-launch unified search failed");
                Vec::new()
            })
    }

    fn search_and_request_icons(
        &mut self,
        query: &str,
        cx: &mut Context<Self>,
    ) -> QuickLaunchSearchUpdate {
        let index_request = self.index_search.begin(query, INDEX_CANDIDATE_LIMIT);
        let generation = index_request.generation;
        let indexer_connected = self.indexer.is_connected();
        let index_available = index_status_has_searchable_snapshot(self.index_status.as_ref());
        let search_indexer = should_search_indexer(indexer_connected, index_available)
            && query_requests_file_index_with_commands(
                query,
                &self.config.quick_launch.custom_commands,
            );
        let search_queued = if query.trim().is_empty() || !search_indexer {
            false
        } else {
            match self
                .indexer
                .send_command(IndexerCommand::Search(index_request))
            {
                Ok(_) => true,
                Err(error) => {
                    warn!(%error, "could not queue indexed-file search");
                    false
                }
            }
        };
        info!(
            generation,
            query_length = query.chars().count(),
            indexer_connected,
            index_available,
            search_queued,
            "quick-launch query provider state"
        );
        if !should_publish_immediate_results(search_indexer, search_queued) {
            return QuickLaunchSearchUpdate {
                results: Vec::new(),
                pending: true,
            };
        }
        let local_search_started = Instant::now();
        let results = self.combined_results(query);
        self.last_local_search_us = Some(elapsed_microseconds(local_search_started));
        self.request_icons(&results[..results.len().min(ICON_PREFETCH_LIMIT)], cx);
        QuickLaunchSearchUpdate {
            results,
            pending: false,
        }
    }

    fn refresh_indexed_results(&mut self, cx: &mut Context<Self>) {
        let Some(popup) = self.popup.clone() else {
            return;
        };
        let panel_state = popup
            .panel
            .update(cx, |panel, cx| {
                (
                    panel.is_open,
                    panel.action_mode.is_none(),
                    panel.search_input.read(cx).value().to_string(),
                )
            })
            .ok();
        let Some((true, true, query)) = panel_state else {
            return;
        };
        if query != self.index_search.query {
            return;
        }
        let local_search_started = Instant::now();
        let results = self.combined_results(&query);
        self.last_local_search_us = Some(elapsed_microseconds(local_search_started));
        let indexed_visible = results
            .iter()
            .filter(|result| {
                matches!(
                    result.entry.source,
                    ApplicationSource::IndexedDirectory | ApplicationSource::IndexedFile
                )
            })
            .count();
        info!(
            generation = self.index_search.generation,
            indexed_candidates = self.index_search.items.len(),
            indexed_visible,
            result_count = results.len(),
            "quick-launch merged provider results published"
        );
        self.request_icons(&results[..results.len().min(ICON_PREFETCH_LIMIT)], cx);
        let _ = popup
            .panel
            .update(cx, |panel, cx| panel.replace_results(results, cx));
    }

    fn refresh_results_after_indexer_disconnect(&mut self, cx: &mut Context<Self>) {
        let Some(popup) = self.popup.clone() else {
            return;
        };
        let panel_state = popup
            .panel
            .update(cx, |panel, cx| {
                (
                    panel.is_open,
                    panel.action_mode.is_none(),
                    panel.search_input.read(cx).value().to_string(),
                )
            })
            .ok();
        let Some((true, true, query)) = panel_state else {
            return;
        };
        let results = self.combined_results(&query);
        self.request_icons(&results[..results.len().min(ICON_PREFETCH_LIMIT)], cx);
        let _ = popup
            .panel
            .update(cx, |panel, cx| panel.replace_results(results, cx));
    }

    fn request_catalog_refresh(&mut self, cx: &mut Context<Self>) {
        if self.catalog_refresh_in_progress {
            return;
        }
        self.catalog_refresh_in_progress = true;
        let roots = self.catalog.borrow().roots().to_vec();
        let background = cx.background_executor().clone();
        cx.spawn(async move |this, cx| {
            let snapshot = background
                .spawn(async move { scan_application_roots(&roots) })
                .await;
            if let Some(this) = this.upgrade() {
                let _ = this.update(cx, |application, cx| {
                    application.finish_catalog_refresh(snapshot, cx);
                });
            }
        })
        .detach();
    }

    fn finish_catalog_refresh(
        &mut self,
        snapshot: ApplicationCatalogSnapshot,
        cx: &mut Context<Self>,
    ) {
        self.catalog_refresh_in_progress = false;
        if let Err(errors) = self.catalog.borrow_mut().apply_refresh_snapshot(snapshot) {
            let reason = errors
                .first()
                .cloned()
                .unwrap_or_else(|| "未知目录读取错误".into());
            warn!(error_count = errors.len(), %reason, "quick-launch catalog refresh rejected");
            self.notifications.update(cx, |notifications, cx| {
                notifications.show_error(
                    "quick-launch-refresh",
                    "速启索引刷新失败",
                    format!("已保留上次可用结果：{reason}"),
                    cx,
                );
            });
            if let Some(popup) = self.popup.clone() {
                let _ = popup.panel.update(cx, |panel, cx| {
                    if panel.is_open {
                        panel.set_error(format!("索引刷新失败，已保留原结果：{reason}"), cx);
                    }
                });
            }
            cx.notify();
            return;
        }
        let Some(popup) = self.popup.clone() else {
            cx.notify();
            return;
        };
        let query = popup
            .panel
            .update(cx, |panel, cx| {
                quick_launch_catalog_refresh_query(
                    panel.is_open,
                    panel.action_mode.is_some(),
                    panel.search_input.read(cx).value().to_string(),
                )
            })
            .ok()
            .flatten();
        let Some(query) = query else {
            cx.notify();
            return;
        };
        if self.indexer.is_connected()
            && self.index_search.query == query
            && !self.index_search.complete
        {
            cx.notify();
            return;
        }
        let results = self.combined_results(&query);
        self.request_icons(&results[..results.len().min(ICON_PREFETCH_LIMIT)], cx);
        let _ = popup
            .panel
            .update(cx, |panel, cx| panel.replace_results(results, cx));
        info!(
            indexed_items = self.catalog.borrow().status().indexed_items,
            "quick-launch application catalog refreshed"
        );
        cx.notify();
    }

    fn schedule_quick_launch_preview(
        &mut self,
        entered: bool,
        result: SearchResult,
        panel_bounds: Bounds<Pixels>,
        panel_scale_factor: f32,
        cx: &mut Context<Self>,
    ) {
        if !quick_launch_result_capabilities(&result).preview {
            if entered {
                self.dismiss_quick_launch_preview(cx);
            }
            return;
        }
        let entry_id = result.entry.id.clone();
        if entered {
            self.preview.hovered_entry_id = Some(entry_id.clone());
            self.preview.pointer_inside = false;
            self.preview.generation = self.preview.generation.wrapping_add(1).max(1);
            let candidate = QuickLaunchPreviewCandidate {
                result,
                panel_bounds,
                panel_scale_factor,
            };
            let switching_target = self
                .preview
                .pending_stage
                .as_ref()
                .is_some_and(|stage| stage.entry_id != entry_id)
                || quick_launch_preview_target_changed(
                    self.preview.visible,
                    self.preview.displayed_entry_id.as_deref(),
                    &entry_id,
                );
            if self.preview.visible {
                self.cancel_quick_launch_preview_exit(cx);
            }
            if quick_launch_preview_open_action(
                self.preview.window.is_some(),
                self.preview.visible,
                self.preview.displayed_entry_id.as_deref(),
                &entry_id,
            ) != QuickLaunchPreviewOpenAction::Ignore
            {
                self.schedule_quick_launch_preview_candidate(
                    candidate,
                    if switching_target {
                        QUICK_LAUNCH_PREVIEW_SWITCH_DELAY
                    } else {
                        QUICK_LAUNCH_PREVIEW_HOVER_DELAY
                    },
                    cx,
                );
            }
        } else if self.preview.hovered_entry_id.as_deref() == Some(entry_id.as_str()) {
            self.preview.hovered_entry_id = None;
            self.schedule_quick_launch_preview_hide(QUICK_LAUNCH_PREVIEW_LEAVE_DELAY, cx);
        }
    }

    fn schedule_quick_launch_preview_candidate(
        &mut self,
        candidate: QuickLaunchPreviewCandidate,
        delay: Duration,
        cx: &mut Context<Self>,
    ) {
        let generation = self.preview.generation;
        let entry_id = candidate.result.entry.id.clone();
        let background = cx.background_executor().clone();
        cx.spawn(async move |this, cx| {
            background.timer(delay).await;
            let should_load = this
                .update(cx, |application, _| {
                    application.preview.generation == generation
                        && application.preview.hovered_entry_id.as_deref()
                            == Some(entry_id.as_str())
                })
                .unwrap_or(false);
            if !should_load {
                return;
            }
            let result = candidate.result.clone();
            let payload = background
                .spawn(async move { load_quick_launch_preview(&result) })
                .await;
            let _ = this.update(cx, |application, cx| {
                if application.preview.generation != generation
                    || application.preview.hovered_entry_id.as_deref() != Some(entry_id.as_str())
                {
                    return;
                }
                if let Some(payload) = payload {
                    application.show_quick_launch_preview(payload, candidate, cx);
                } else {
                    application.hide_quick_launch_preview_window(cx);
                }
            });
        })
        .detach();
    }

    fn set_quick_launch_preview_pointer_inside(&mut self, inside: bool, cx: &mut Context<Self>) {
        self.preview.pointer_inside = inside;
        self.preview.generation = self.preview.generation.wrapping_add(1).max(1);
        if inside {
            self.cancel_quick_launch_preview_exit(cx);
        } else if self.preview.hovered_entry_id.is_none() {
            self.schedule_quick_launch_preview_hide(QUICK_LAUNCH_PREVIEW_LEAVE_DELAY, cx);
        }
    }

    fn schedule_quick_launch_preview_hide(&mut self, delay: Duration, cx: &mut Context<Self>) {
        self.preview.generation = self.preview.generation.wrapping_add(1).max(1);
        let generation = self.preview.generation;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(delay).await;
            let _ = this.update(cx, |application, cx| {
                if application.preview.generation == generation
                    && application.preview.hovered_entry_id.is_none()
                    && !application.preview.pointer_inside
                {
                    application.animate_quick_launch_preview_exit(cx);
                }
            });
        })
        .detach();
    }

    fn cancel_quick_launch_preview_exit(&mut self, cx: &mut Context<Self>) {
        self.preview.exit_generation = self.preview.exit_generation.wrapping_add(1).max(1);
        if let Some(preview) = self.preview.window.as_ref() {
            let _ = preview
                .view
                .update(cx, |preview, cx| preview.cancel_exit(cx));
        }
    }

    fn animate_quick_launch_preview_exit(&mut self, cx: &mut Context<Self>) {
        if !self.preview.visible {
            self.hide_quick_launch_preview_window(cx);
            return;
        }
        self.preview.exit_generation = self.preview.exit_generation.wrapping_add(1).max(1);
        let exit_generation = self.preview.exit_generation;
        let motion_started = self.preview.window.as_ref().is_some_and(|preview| {
            preview
                .view
                .update(cx, |preview, cx| preview.begin_exit(cx))
                .is_ok()
        });
        if !motion_started {
            self.hide_quick_launch_preview_window(cx);
            return;
        }
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(QUICK_LAUNCH_PREVIEW_DISAPPEAR_DURATION)
                .await;
            let _ = this.update(cx, |application, cx| {
                if application.preview.exit_generation == exit_generation {
                    application.hide_quick_launch_preview_window(cx);
                }
            });
        })
        .detach();
    }

    fn suspend_quick_launch_preview_for_scroll(&mut self, cx: &mut Context<Self>) {
        self.preview.generation = self.preview.generation.wrapping_add(1).max(1);
        self.preview.hovered_entry_id = None;
        self.preview.pointer_inside = false;
        self.hide_quick_launch_preview_window(cx);
    }

    fn dismiss_quick_launch_preview(&mut self, cx: &mut Context<Self>) {
        self.preview.generation = self.preview.generation.wrapping_add(1).max(1);
        self.preview.hovered_entry_id = None;
        self.preview.pointer_inside = false;
        self.hide_quick_launch_preview_window(cx);
    }

    fn hide_quick_launch_preview_window(&mut self, cx: &mut Context<Self>) {
        self.preview.exit_generation = self.preview.exit_generation.wrapping_add(1).max(1);
        self.preview.pending_stage = None;
        if let Some(image) = self.preview.image_asset.take() {
            ImageSource::from(image).remove_asset(cx);
        }
        if let Some(preview) = self.preview.window.as_ref() {
            let _ = preview
                .view
                .update(cx, |preview, cx| preview.clear_image_asset(cx));
        }
        let native_window_handle = self
            .preview
            .window
            .as_ref()
            .map_or(0, |preview| preview.identity.native);
        if self.preview.visible
            && native_window_handle != 0
            && let Err(error) = self
                .platform
                .send_command(PlatformCommand::SetWindowVisibility(
                    SetWindowVisibilityCommand {
                        window_handle: native_window_handle,
                        visible: false,
                    },
                ))
        {
            warn!(%error, "failed to hide quick-launch preview");
        }
        self.preview.visible = false;
        self.preview.displayed_entry_id = None;
    }

    fn show_quick_launch_preview(
        &mut self,
        payload: QuickLaunchPreviewPayload,
        candidate: QuickLaunchPreviewCandidate,
        cx: &mut Context<Self>,
    ) {
        let display = cx
            .displays()
            .into_iter()
            .find(|display| display.bounds().contains(&candidate.panel_bounds.origin))
            .or_else(|| cx.primary_display());
        let screen = display
            .as_ref()
            .map(|display| display.bounds())
            .unwrap_or_else(|| Bounds::new(point(px(0.0), px(0.0)), size(px(1920.0), px(1080.0))));
        let preview_size = size(
            px(QUICK_LAUNCH_PREVIEW_WIDTH.min(screen.size.width.as_f32())),
            px(quick_launch_preview_height(&payload).min(screen.size.height.as_f32())),
        );
        let bounds = quick_launch_preview_bounds(candidate.panel_bounds, preview_size, screen);
        self.preview.exit_generation = self.preview.exit_generation.wrapping_add(1).max(1);
        let action = quick_launch_preview_open_action(
            self.preview.window.is_some(),
            self.preview.visible,
            self.preview.displayed_entry_id.as_deref(),
            &payload.entry_id,
        );
        if action == QuickLaunchPreviewOpenAction::Ignore {
            self.cancel_quick_launch_preview_exit(cx);
            return;
        }
        if action == QuickLaunchPreviewOpenAction::Reuse {
            self.stage_reused_quick_launch_preview(payload, preview_size, bounds, candidate, cx);
            return;
        }

        let application = cx.entity().downgrade();
        let platform = Rc::clone(&self.platform);
        let native_slot = Rc::new(RefCell::new(0_i64));
        let opened_native_slot = Rc::clone(&native_slot);
        let view_slot = Rc::new(RefCell::new(None));
        let opened_view_slot = Rc::clone(&view_slot);
        let displayed_entry_id = payload.entry_id.clone();
        let mut image_asset = quick_launch_preview_image_asset(&payload);
        let panel_scale_factor = candidate.panel_scale_factor;
        let device_x = (bounds.origin.x.as_f32() * panel_scale_factor).round() as i32;
        let device_y = (bounds.origin.y.as_f32() * panel_scale_factor).round() as i32;
        let device_width = (preview_size.width.as_f32() * panel_scale_factor)
            .round()
            .max(1.0) as i32;
        let device_height = (preview_size.height.as_f32() * panel_scale_factor)
            .round()
            .max(1.0) as i32;
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: None,
                focus: false,
                show: false,
                kind: WindowKind::PopUp,
                is_movable: false,
                is_resizable: false,
                is_minimizable: false,
                display_id: display.as_ref().map(|display| display.id()),
                window_background: WindowBackgroundAppearance::Opaque,
                app_id: Some("NxTools.QuickLaunchPreview".into()),
                window_min_size: Some(preview_size),
                window_decorations: Some(WindowDecorations::Client),
                tabbing_identifier: None,
            },
            move |window, cx| {
                let window_handle = if cfg!(test) {
                    0
                } else {
                    native_window_handle(window).unwrap_or_default()
                };
                *opened_native_slot.borrow_mut() = window_handle;
                if window_handle != 0 {
                    let _ = platform.send_command(PlatformCommand::SetWindowNoActivate(
                        SetWindowNoActivateCommand {
                            window_handle,
                            enabled: true,
                        },
                    ));
                    let _ = platform.send_command(PlatformCommand::SetWindowBounds(
                        SetWindowBoundsCommand {
                            window_handle,
                            x: device_x,
                            y: device_y,
                            width: device_width,
                            height: device_height,
                        },
                    ));
                    let _ = platform.send_command(PlatformCommand::SetWindowTopmost(
                        SetWindowTopmostCommand {
                            window_handle,
                            enabled: true,
                        },
                    ));
                    let _ = platform.send_command(PlatformCommand::SetWindowVisibility(
                        SetWindowVisibilityCommand {
                            window_handle,
                            visible: true,
                        },
                    ));
                }
                let view = cx.new(|_| QuickLaunchPreview::new(payload, application));
                *opened_view_slot.borrow_mut() = Some(view.downgrade());
                cx.new(|cx| Root::new(view, window, cx))
            },
        );
        match opened {
            Ok(window) => {
                let Some(view) = view_slot.borrow_mut().take() else {
                    let window: AnyWindowHandle = window.into();
                    let _ = window.update(cx, |_, window, _| window.remove_window());
                    if let Some(image) = image_asset.take() {
                        ImageSource::from(image).remove_asset(cx);
                    }
                    return;
                };
                self.preview.window = Some(QuickLaunchPreviewWindow {
                    identity: QuickLaunchPreviewWindowIdentity {
                        gpui: window.into(),
                        native: *native_slot.borrow(),
                    },
                    view,
                });
                self.preview.visible = true;
                self.preview.displayed_entry_id = Some(displayed_entry_id);
                self.preview.image_asset = image_asset;
                self.restore_quick_launch_panel_keyboard_focus(cx);
            }
            Err(error) => {
                self.preview.visible = false;
                self.preview.displayed_entry_id = None;
                if let Some(image) = image_asset {
                    ImageSource::from(image).remove_asset(cx);
                }
                error!(%error, "failed to open quick-launch preview");
            }
        }
    }

    fn restore_quick_launch_panel_keyboard_focus(&self, cx: &mut Context<Self>) {
        let Some(popup) = self.popup.clone() else {
            return;
        };
        if popup.native_window_handle != 0
            && let Err(error) =
                self.platform
                    .send_command(PlatformCommand::ActivateWindow(ActivateWindowCommand {
                        window_handle: popup.native_window_handle,
                    }))
        {
            warn!(%error, "failed to restore quick-launch keyboard target after preview creation");
        }
        let _ = popup.window.update(cx, |_, window, app| {
            let _ = popup
                .panel
                .update(app, |panel, cx| panel.focus_search(window, cx));
        });
    }

    fn stage_reused_quick_launch_preview(
        &mut self,
        payload: QuickLaunchPreviewPayload,
        preview_size: Size<Pixels>,
        bounds: Bounds<Pixels>,
        candidate: QuickLaunchPreviewCandidate,
        cx: &mut Context<Self>,
    ) {
        let Some(preview_window) = self.preview.window.as_ref() else {
            return;
        };
        let window_identity = preview_window.identity;
        let native_window_handle = window_identity.native;
        if native_window_handle == 0 {
            self.preview.window = None;
            self.show_quick_launch_preview(payload, candidate, cx);
            return;
        }
        let entry_id = payload.entry_id.clone();
        let mut stage = QuickLaunchPreviewPendingStage {
            command_id: uuid::Uuid::nil(),
            phase: QuickLaunchPreviewStagePhase::MoveOffscreen,
            window_identity,
            generation: self.preview.generation,
            entry_id,
            payload: Some(payload),
            preview_size,
            bounds,
            panel_scale_factor: candidate.panel_scale_factor,
        };
        match self
            .platform
            .send_command(PlatformCommand::SetWindowPosition(
                SetWindowPositionCommand {
                    window_handle: native_window_handle,
                    x: QUICK_LAUNCH_PREVIEW_STAGING_COORDINATE,
                    y: QUICK_LAUNCH_PREVIEW_STAGING_COORDINATE,
                },
            )) {
            Ok(command_id) => {
                stage.command_id = command_id;
                self.preview.pending_stage = Some(stage);
                self.preview.displayed_entry_id = None;
            }
            Err(error) => {
                warn!(%error, "failed to stage quick-launch preview outside the display");
                self.hide_quick_launch_preview_window(cx);
            }
        }
    }

    fn handle_quick_launch_preview_stage_result(
        &mut self,
        correlation_id: uuid::Uuid,
        success: bool,
        error_message: Option<&str>,
        cx: &mut Context<Self>,
    ) -> bool {
        if self
            .preview
            .pending_stage
            .as_ref()
            .is_none_or(|stage| stage.command_id != correlation_id)
        {
            return false;
        }
        let mut stage = self
            .preview
            .pending_stage
            .take()
            .expect("matching quick-launch preview stage must exist");
        if self
            .preview
            .window
            .as_ref()
            .is_none_or(|preview| preview.identity != stage.window_identity)
        {
            return true;
        }
        if self.preview.generation != stage.generation
            || self.preview.hovered_entry_id.as_deref() != Some(stage.entry_id.as_str())
        {
            return true;
        }
        if !success {
            warn!(
                %correlation_id,
                reason = error_message.unwrap_or("unknown Windows error"),
                phase = ?stage.phase,
                "quick-launch preview staging failed"
            );
            self.hide_quick_launch_preview_window(cx);
            return true;
        }
        match stage.phase {
            QuickLaunchPreviewStagePhase::MoveOffscreen if !self.preview.visible => {
                match self
                    .platform
                    .send_command(PlatformCommand::SetWindowVisibility(
                        SetWindowVisibilityCommand {
                            window_handle: stage.window_identity.native,
                            visible: true,
                        },
                    )) {
                    Ok(command_id) => {
                        stage.command_id = command_id;
                        stage.phase = QuickLaunchPreviewStagePhase::ShowOffscreen;
                        self.preview.pending_stage = Some(stage);
                        self.preview.visible = true;
                    }
                    Err(error) => {
                        warn!(%error, "failed to show staged quick-launch preview");
                        self.hide_quick_launch_preview_window(cx);
                    }
                }
            }
            QuickLaunchPreviewStagePhase::MoveOffscreen
            | QuickLaunchPreviewStagePhase::ShowOffscreen => {
                self.preview.visible = true;
                self.render_staged_quick_launch_preview(stage, cx);
            }
            QuickLaunchPreviewStagePhase::RevealOnscreen => {
                let _ = self
                    .platform
                    .send_command(PlatformCommand::SetWindowTopmost(SetWindowTopmostCommand {
                        window_handle: stage.window_identity.native,
                        enabled: true,
                    }));
                self.preview.visible = true;
                self.preview.displayed_entry_id = Some(stage.entry_id);
            }
        }
        true
    }

    fn render_staged_quick_launch_preview(
        &mut self,
        stage: QuickLaunchPreviewPendingStage,
        cx: &mut Context<Self>,
    ) {
        let Some(preview) = self.preview.window.clone() else {
            self.preview.visible = false;
            return;
        };
        let QuickLaunchPreviewPendingStage {
            window_identity,
            generation,
            entry_id,
            payload,
            preview_size,
            bounds,
            panel_scale_factor,
            ..
        } = stage;
        let Some(payload) = payload else {
            self.hide_quick_launch_preview_window(cx);
            return;
        };
        let image_asset = quick_launch_preview_image_asset(&payload);
        let view = preview.view.clone();
        let application = cx.entity().downgrade();
        let update = preview.identity.gpui.update(cx, move |_, window, app| {
            if view
                .update(app, |preview, cx| preview.set_payload(payload, cx))
                .is_err()
            {
                return;
            }
            window.on_next_frame(move |_, app| {
                let _ = application.update(app, |application, cx| {
                    if application
                        .preview
                        .window
                        .as_ref()
                        .is_none_or(|preview| preview.identity != window_identity)
                        || application.preview.generation != generation
                        || application.preview.hovered_entry_id.as_deref()
                            != Some(entry_id.as_str())
                    {
                        return;
                    }
                    let x = (bounds.origin.x.as_f32() * panel_scale_factor).round() as i32;
                    let y = (bounds.origin.y.as_f32() * panel_scale_factor).round() as i32;
                    match application
                        .platform
                        .send_command(PlatformCommand::SetWindowPosition(
                            SetWindowPositionCommand {
                                window_handle: window_identity.native,
                                x,
                                y,
                            },
                        )) {
                        Ok(command_id) => {
                            application.preview.pending_stage =
                                Some(QuickLaunchPreviewPendingStage {
                                    command_id,
                                    phase: QuickLaunchPreviewStagePhase::RevealOnscreen,
                                    window_identity,
                                    generation,
                                    entry_id,
                                    payload: None,
                                    preview_size,
                                    bounds,
                                    panel_scale_factor,
                                });
                        }
                        Err(error) => {
                            warn!(%error, "failed to reveal refreshed quick-launch preview");
                            application.hide_quick_launch_preview_window(cx);
                        }
                    }
                });
            });
            window.refresh();
        });
        if update.is_err() {
            self.preview.window = None;
            self.preview.visible = false;
            self.preview.displayed_entry_id = None;
            if let Some(image) = image_asset {
                ImageSource::from(image).remove_asset(cx);
            }
        } else {
            self.preview.image_asset = image_asset;
        }
    }

    fn request_icons(&mut self, results: &[SearchResult], cx: &mut Context<Self>) {
        let mut cached = Vec::new();
        for result in results {
            if !quick_launch_result_capabilities(result).shell_icon {
                continue;
            }
            let entry_id = result.entry.id.clone();
            let resource_key = icon_resource_key(result, 32);
            if let Some(path) = self.icon_resources.get(&resource_key) {
                cached.push((entry_id, path));
                continue;
            }
            if let Some(command_id) = self.pending_icon_resources.get(&resource_key).copied() {
                if let Some(pending) = self.pending_icons.get_mut(&command_id) {
                    pending.entry_ids.insert(entry_id);
                }
                continue;
            }
            if self
                .failed_icon_resources
                .get(&resource_key)
                .is_some_and(|failed_at| icon_retry_pending(*failed_at, Instant::now()))
            {
                continue;
            }
            self.failed_icon_resources.remove(&resource_key);
            match self
                .platform
                .send_command(PlatformCommand::ExtractShellIcon(ExtractShellIconCommand {
                    target: result.entry.target.clone(),
                    size: 32,
                })) {
                Ok(command_id) => {
                    self.pending_icon_resources
                        .insert(resource_key.clone(), command_id);
                    self.pending_icons.insert(
                        command_id,
                        PendingIconResource {
                            resource_key,
                            entry_ids: HashSet::from([entry_id]),
                        },
                    );
                }
                Err(error) => {
                    warn!(%error, %resource_key, "could not request a quick-launch icon");
                    self.record_icon_failure(resource_key, cx);
                }
            }
        }
        if !cached.is_empty()
            && let Some(popup) = self.popup.clone()
        {
            cx.defer(move |cx| {
                let _ = popup
                    .panel
                    .update(cx, |panel, cx| panel.set_icons(cached, cx));
            });
        }
    }

    fn record_icon_failure(&mut self, resource_key: String, cx: &mut Context<Self>) {
        let failed_at = Instant::now();
        self.failed_icon_resources
            .insert(resource_key.clone(), failed_at);
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(ICON_RETRY_DELAY).await;
            let Some(this) = this.upgrade() else {
                return;
            };
            let _ = this.update(cx, |application, cx| {
                if application
                    .failed_icon_resources
                    .get(&resource_key)
                    .is_none_or(|current| *current != failed_at)
                {
                    return;
                }
                application.failed_icon_resources.remove(&resource_key);
                if let Some(popup) = application.popup.clone() {
                    let _ = popup.panel.update(cx, |panel, cx| {
                        panel.last_presented_icon_range = None;
                        cx.notify();
                    });
                }
            });
        })
        .detach();
    }

    fn query_shell_actions(
        &mut self,
        mode_id: uuid::Uuid,
        level_id: uuid::Uuid,
        entry_id: String,
        target: String,
        cx: &mut Context<Self>,
    ) {
        match self
            .platform
            .send_command(PlatformCommand::QueryShellActions(
                QueryShellActionsCommand { target },
            )) {
            Ok(command_id) => {
                self.pending_shell_queries.insert(
                    command_id,
                    PendingShellQuery {
                        mode_id,
                        level_id,
                        entry_id,
                    },
                );
            }
            Err(error) => {
                warn!(%error, "could not queue Windows Shell action enumeration");
                self.apply_shell_actions(mode_id, level_id, &entry_id, None, cx);
            }
        }
    }

    fn query_shell_submenu(
        &mut self,
        mode_id: uuid::Uuid,
        level: PendingShellSubmenuLevel,
        entry_id: String,
        cx: &mut Context<Self>,
    ) {
        let command = PlatformCommand::QueryShellSubmenu(QueryShellSubmenuCommand {
            session_id: level.session_id,
            submenu_id: level.submenu_id,
        });
        match self.platform.send_command(command) {
            Ok(command_id) => {
                self.pending_shell_queries.insert(
                    command_id,
                    PendingShellQuery {
                        mode_id,
                        level_id: level.level_id,
                        entry_id,
                    },
                );
            }
            Err(error) => {
                warn!(%error, "could not queue Windows Shell submenu enumeration");
                self.apply_shell_actions(mode_id, level.level_id, &entry_id, None, cx);
            }
        }
    }

    fn apply_shell_actions(
        &mut self,
        mode_id: uuid::Uuid,
        level_id: uuid::Uuid,
        entry_id: &str,
        menu: Option<ShellActionMenu>,
        cx: &mut Context<Self>,
    ) {
        let Some(popup) = self.popup.clone() else {
            return;
        };
        let _ = popup.panel.update(cx, |panel, cx| {
            panel.apply_shell_actions(mode_id, level_id, entry_id, menu, cx);
        });
    }

    fn execute(
        &mut self,
        result: SearchResult,
        action: QuickLaunchAction,
        query: String,
        trigger: QuickLaunchActionTrigger,
        cx: &mut Context<Self>,
    ) {
        let started_at = Instant::now();
        let action_correlation_id = uuid::Uuid::new_v4();
        let owner_window_handle = self
            .popup
            .as_ref()
            .map_or(0, |popup| popup.native_window_handle);
        let directory_visit = match &action {
            QuickLaunchAction::SwitchDirectory {
                target_directory, ..
            } => Some(target_directory.clone()),
            QuickLaunchAction::Open
                if result.entry.source == ApplicationSource::IndexedDirectory =>
            {
                Some(result.entry.target.clone())
            }
            _ => None,
        };
        let Some(command) = platform_command_for_action(&result, &action, owner_window_handle)
        else {
            warn!(
                event = "quick_launch.action.rejected",
                correlation_id = %action_correlation_id,
                action = quick_launch_action_diagnostic_label(&action),
                trigger = trigger.diagnostic_label(),
                source = result.entry.source.label(),
                outcome = "failure",
                elapsed_ms = u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX),
                native_error_code = ?Option::<i32>::None,
                reason = "unsupported_action",
                "quick-launch action rejected before dispatch"
            );
            if let Some(popup) = self.popup.clone() {
                let _ = popup.panel.update(cx, |panel, cx| {
                    panel.set_error("系统工具没有可定位的快捷方式".into(), cx);
                });
            }
            return;
        };
        match self
            .platform
            .send_command_correlated(command, action_correlation_id)
        {
            Ok(command_id) => {
                info!(
                    event = "quick_launch.action.accepted",
                    correlation_id = %action_correlation_id,
                    action = quick_launch_action_diagnostic_label(&action),
                    trigger = trigger.diagnostic_label(),
                    source = result.entry.source.label(),
                    "quick-launch action accepted"
                );
                self.pending_actions.insert(
                    command_id,
                    PendingQuickLaunchAction {
                        correlation_id: action_correlation_id,
                        popup_session_id: self.popup_session_id,
                        entry_id: result.entry.id,
                        display_name: result.entry.name,
                        target: result.entry.target,
                        query,
                        action,
                        directory_visit,
                        trigger,
                        source: result.entry.source,
                        started_at,
                    },
                );
            }
            Err(error) => {
                warn!(
                    event = "quick_launch.action.rejected",
                    correlation_id = %action_correlation_id,
                    action = quick_launch_action_diagnostic_label(&action),
                    trigger = trigger.diagnostic_label(),
                    source = result.entry.source.label(),
                    outcome = "failure",
                    elapsed_ms = u64::try_from(started_at.elapsed().as_millis())
                        .unwrap_or(u64::MAX),
                    native_error_code = ?Option::<i32>::None,
                    reason = %error,
                    "quick-launch action rejected before dispatch"
                );
                if let Some(popup) = self.popup.clone() {
                    let _ = popup.panel.update(cx, |panel, cx| {
                        panel.set_error(format!("Windows 适配器不可用：{error}"), cx);
                    });
                }
            }
        }
    }

    fn toggle_panel(&mut self, request: QuickLaunchPanelRequest, cx: &mut Context<Self>) {
        if let Some(popup) = self.popup.clone() {
            let (is_open, captured_window_handle) = popup
                .panel
                .update(cx, |panel, _| (panel.is_open, panel.target.window_handle))
                .unwrap_or((false, 0));
            if is_open {
                info!(
                    dismissal_source = "quick_launch_trigger_toggled",
                    popup_window_handle = popup.native_window_handle,
                    "quick-launch popup dismissed"
                );
                self.hide_popup_and_restore(captured_window_handle, cx);
            } else {
                self.show_existing_popup(popup, request, cx);
            }
            return;
        }
        self.open_popup(request, cx);
    }

    fn show_existing_popup(
        &mut self,
        popup: QuickLaunchWindow,
        request: QuickLaunchPanelRequest,
        cx: &mut Context<Self>,
    ) {
        self.popup_session_id = uuid::Uuid::new_v4();
        self.active_summon = Some((request.target.window_handle, request.requested_at_unix_ms));
        self.navigation_foreground_guard = None;
        self.index_search.invalidate();
        let results = Vec::new();
        let recent_directories = if request.directory_navigation_context.is_some() {
            self.catalog
                .borrow()
                .recent_directories(RECENT_DIRECTORY_LIMIT)
        } else {
            Vec::new()
        };
        let captured_navigation_context = request.directory_navigation_context.clone();
        let (bounds, _) = quick_launch_bounds(&request, false, cx);
        let scale = request.cursor_monitor_dpi.max(96) as f32 / 96.0;
        let _ = self
            .platform
            .send_command(PlatformCommand::SetWindowPosition(
                SetWindowPositionCommand {
                    window_handle: popup.native_window_handle,
                    x: (bounds.origin.x.as_f32() * scale).round() as i32,
                    y: (bounds.origin.y.as_f32() * scale).round() as i32,
                },
            ));
        let _ = self
            .platform
            .send_command(PlatformCommand::SetWindowVisibility(
                SetWindowVisibilityCommand {
                    window_handle: popup.native_window_handle,
                    visible: true,
                },
            ));
        let _ = popup.window.update(cx, |_, window, app| {
            window.resize(size(px(PANEL_WIDTH), px(COLLAPSED_PANEL_HEIGHT)));
            let _ = popup.panel.update(app, |panel, cx| {
                panel.prepare_for_show(
                    request.target,
                    request.directory_navigation_context,
                    recent_directories,
                    results,
                    window,
                    cx,
                );
            });
        });
        self.begin_window_activation(popup.native_window_handle);
        self.schedule_captured_directory_record(captured_navigation_context.as_ref(), cx);
        info!(
            window_handle = popup.native_window_handle,
            "quick-launch popup reused"
        );
        self.request_catalog_refresh(cx);
    }

    fn open_popup(&mut self, request: QuickLaunchPanelRequest, cx: &mut Context<Self>) {
        self.popup_session_id = uuid::Uuid::new_v4();
        self.active_summon = Some((request.target.window_handle, request.requested_at_unix_ms));
        self.navigation_foreground_guard = None;
        self.index_search.invalidate();
        let results = Vec::new();
        let recent_directories = if request.directory_navigation_context.is_some() {
            self.catalog
                .borrow()
                .recent_directories(RECENT_DIRECTORY_LIMIT)
        } else {
            Vec::new()
        };
        let (bounds, display_id) = quick_launch_bounds(&request, false, cx);
        let application = cx.entity().downgrade();
        let target = request.target;
        let navigation_context = request.directory_navigation_context;
        let captured_navigation_context = navigation_context.clone();
        let panel_slot = Rc::new(RefCell::new(None));
        let native_slot = Rc::new(RefCell::new(0_i64));
        let opened_panel_slot = Rc::clone(&panel_slot);
        let opened_native_slot = Rc::clone(&native_slot);
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: None,
                focus: false,
                show: true,
                kind: WindowKind::PopUp,
                is_movable: false,
                is_resizable: false,
                is_minimizable: false,
                display_id,
                window_background: WindowBackgroundAppearance::Opaque,
                app_id: Some("NxTools.QuickLaunch".into()),
                window_min_size: Some(size(px(PANEL_WIDTH), px(COLLAPSED_PANEL_HEIGHT))),
                window_decorations: Some(WindowDecorations::Client),
                tabbing_identifier: None,
            },
            move |window, cx| {
                *opened_native_slot.borrow_mut() = native_window_handle(window).unwrap_or_default();
                let panel = cx.new(|cx| {
                    QuickLaunchPanel::new(
                        results,
                        target,
                        navigation_context,
                        recent_directories,
                        application,
                        window,
                        cx,
                    )
                });
                *opened_panel_slot.borrow_mut() = Some(panel.downgrade());
                cx.new(|cx| Root::new(panel, window, cx))
            },
        );
        match opened {
            Ok(window) => {
                let Some(panel) = panel_slot.borrow_mut().take() else {
                    let _ = window.update(cx, |_, window, _| window.remove_window());
                    return;
                };
                let popup = QuickLaunchWindow {
                    window: window.into(),
                    panel,
                    native_window_handle: *native_slot.borrow(),
                };
                let native_window_handle = popup.native_window_handle;
                self.popup = Some(popup);
                self.begin_window_activation(native_window_handle);
                self.schedule_captured_directory_record(captured_navigation_context.as_ref(), cx);
                self.request_catalog_refresh(cx);
                info!("quick-launch popup opened");
            }
            Err(error) => error!(%error, "failed to open quick-launch popup"),
        }
    }

    fn schedule_captured_directory_record(
        &mut self,
        context: Option<&nxtools_protocol::DirectoryNavigationContext>,
        cx: &mut Context<Self>,
    ) {
        let Some(directory) = context
            .and_then(|context| context.current_directory.as_deref())
            .filter(|directory| Path::new(directory).is_absolute())
            .map(ToOwned::to_owned)
        else {
            return;
        };
        self.schedule_directory_record(
            directory,
            nxtools_ipc::unix_time_ms(),
            self.popup_session_id,
            cx,
        );
    }

    fn schedule_directory_record(
        &mut self,
        directory: String,
        visited_at_unix_ms: i64,
        popup_session_id: uuid::Uuid,
        cx: &mut Context<Self>,
    ) {
        let history_path = self.catalog.borrow().history_path().to_path_buf();
        let background = cx.background_executor().clone();
        cx.spawn(async move |this, cx| {
            let recorded = background
                .spawn(async move {
                    FinderCatalog::persist_directory_visit(
                        history_path,
                        &directory,
                        visited_at_unix_ms,
                    )
                    .map_err(|error| (directory, error))
                })
                .await;
            let Some(this) = this.upgrade() else {
                return;
            };
            let _ = this.update(cx, |application, cx| {
                let recorded = match recorded {
                    Ok(Some(recorded)) => recorded,
                    Ok(None) => return,
                    Err((directory, error)) => {
                        warn!(%error, %directory, "failed to persist recent directory");
                        return;
                    }
                };
                let recent = {
                    let mut catalog = application.catalog.borrow_mut();
                    catalog.apply_recorded_directory_visit(recorded);
                    catalog.recent_directories(RECENT_DIRECTORY_LIMIT)
                };
                if application.popup_session_id != popup_session_id {
                    return;
                }
                let Some(popup) = application.popup.clone() else {
                    return;
                };
                let _ = popup.panel.update(cx, |panel, cx| {
                    if panel.is_open && panel.navigation_context.is_some() {
                        panel.recent_directories = recent;
                        cx.notify();
                    }
                });
            });
        })
        .detach();
    }

    pub(crate) fn hide_popup(&mut self, cx: &mut Context<Self>) {
        self.hide_popup_with_restore(None, cx);
    }

    fn hide_popup_and_restore(&mut self, captured_window_handle: i64, cx: &mut Context<Self>) {
        self.hide_popup_with_restore(Some(captured_window_handle), cx);
    }

    fn hide_popup_with_restore(
        &mut self,
        restore_window_handle: Option<i64>,
        cx: &mut Context<Self>,
    ) {
        self.popup_session_id = uuid::Uuid::new_v4();
        self.active_summon = None;
        self.navigation_foreground_guard = None;
        self.pending_window_activation.clear();
        self.dismiss_quick_launch_preview(cx);
        let Some(popup) = self.popup.clone() else {
            return;
        };
        let _ = popup.panel.update(cx, |panel, cx| {
            panel.is_open = false;
            cx.notify();
        });
        for command in
            quick_launch_dismiss_commands(popup.native_window_handle, restore_window_handle)
        {
            let _ = self.platform.send_command(command);
        }
    }
}

fn apply_indexer_notification(
    index_status: &mut Option<IndexStatus>,
    index_search: &mut IndexSearchSession,
    notification: IndexerNotification,
) -> bool {
    match notification {
        IndexerNotification::Event(IndexerEvent::Status(status)) => {
            let changed = index_status_affects_presentation(index_status.as_ref(), &status);
            *index_status = Some(status);
            changed
        }
        IndexerNotification::Event(IndexerEvent::SearchBatch(batch)) => index_search.apply(batch),
        IndexerNotification::Event(IndexerEvent::ElevatedDirectoryHelper(_)) => false,
        IndexerNotification::Disconnected => {
            *index_status = None;
            index_search.invalidate();
            true
        }
    }
}

pub(crate) struct QuickLaunchPanel {
    theme: Theme,
    application: WeakEntity<QuickLaunchApplication>,
    target: nxtools_protocol::ForegroundContext,
    navigation_context: Option<nxtools_protocol::DirectoryNavigationContext>,
    results: Vec<SearchResult>,
    selected: usize,
    action_mode: Option<QuickLaunchActionMode>,
    recent_directory_mode: bool,
    recent_directories: Vec<RecentDirectory>,
    search_input: Entity<InputState>,
    status: Option<String>,
    icons: HashMap<String, PathBuf>,
    is_open: bool,
    results_visible: bool,
    search_pending: bool,
    query_dispatch_generation: u64,
    results_appearance_generation: u64,
    result_scroll: UniformListScrollHandle,
    action_scroll: ScrollHandle,
    visible_result_range: Range<usize>,
    last_presented_icon_range: Option<Range<usize>>,
    pointer_selection_gate: PointerSelectionGate,
    ime_enter_gate: ImeEnterGate,
    hide_suppressed_until: Instant,
    _subscriptions: Vec<Subscription>,
}

fn quick_launch_row_shell(theme: Theme, selected: bool) -> Div {
    div()
        .w_full()
        .h(px(COMPACT_RESULT_ROW_HEIGHT))
        .px(px(ROW_HORIZONTAL_PADDING))
        .rounded(px(COMPACT_RADIUS))
        .flex()
        .items_center()
        .gap(px(SPACING_8))
        .bg(if selected {
            theme.element_selected
        } else {
            theme.canvas
        })
        .hover(move |row| {
            row.bg(if selected {
                theme.element_selected
            } else {
                theme.surface_hover
            })
        })
}

impl QuickLaunchPanel {
    fn new(
        results: Vec<SearchResult>,
        target: nxtools_protocol::ForegroundContext,
        navigation_context: Option<nxtools_protocol::DirectoryNavigationContext>,
        recent_directories: Vec<RecentDirectory>,
        application: WeakEntity<QuickLaunchApplication>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("搜索应用、文件或文件夹"));
        let change_subscription = cx.subscribe_in(
            &search_input,
            window,
            |this, state, event: &InputEvent, window, cx| match event {
                InputEvent::Change => {
                    let query = state.read(cx).value().to_string();
                    this.dismiss_preview(cx);
                    this.ime_enter_gate.observe_change();
                    this.query_dispatch_generation =
                        this.query_dispatch_generation.wrapping_add(1).max(1);
                    if let Some(mode) = this.action_mode.as_mut() {
                        mode.set_filter(&query);
                        this.search_pending = false;
                    } else if this.recent_directory_mode {
                        this.selected = 0;
                        this.results = recent_directory_results(&this.recent_directories, &query);
                        this.search_pending = false;
                        this.reset_presented_results();
                        this.result_scroll
                            .0
                            .borrow()
                            .base_handle
                            .set_offset(point(px(0.0), px(0.0)));
                    } else {
                        let _ = this.application.update(cx, |application, _| {
                            application.index_search.invalidate();
                        });
                        if query.trim().is_empty() {
                            this.run_search(query.clone(), cx);
                        } else {
                            this.schedule_search(query.clone(), cx);
                        }
                    }
                    this.sync_window_content_size(&query, window);
                    this.status = None;
                    cx.notify();
                }
                InputEvent::PressEnter { secondary } => {
                    if !this.ime_enter_gate.consume_enter() {
                        return;
                    }
                    if this.action_mode.is_some() {
                        this.activate_selected_action(window, cx);
                    } else if this.recent_directory_mode {
                        this.activate_selected_recent_directory(cx);
                    } else {
                        if let Some(result) = this.results.get(this.selected) {
                            this.activate_selected(
                                quick_launch_primary_action(
                                    result,
                                    *secondary,
                                    window.modifiers().shift,
                                ),
                                QuickLaunchActionTrigger::Enter,
                                cx,
                            );
                        }
                    }
                }
                InputEvent::Focus | InputEvent::Blur => {}
            },
        );
        let composition_subscription =
            cx.observe_in(&search_input, window, |this, state, window, cx| {
                let composing = state.update(cx, |state, cx| {
                    state.marked_text_range(window, cx).is_some()
                });
                this.ime_enter_gate.observe_composition(composing);
            });
        let key_listener = cx.listener(|this, event: &gpui::KeystrokeEvent, window, cx| {
            if !this.is_open || !this.search_input.focus_handle(cx).is_focused(window) {
                return;
            }
            if this.handle_navigation_key(
                event.keystroke.key.as_str(),
                event.keystroke.modifiers,
                window,
                cx,
            ) {
                cx.stop_propagation();
            }
        });
        let key_subscription = cx.intercept_keystrokes(key_listener);
        Self {
            theme: Theme::zed_dark(),
            application,
            target,
            navigation_context,
            results,
            selected: 0,
            action_mode: None,
            recent_directory_mode: false,
            recent_directories,
            search_input,
            status: None,
            icons: HashMap::new(),
            is_open: true,
            results_visible: false,
            search_pending: false,
            query_dispatch_generation: 0,
            results_appearance_generation: 0,
            result_scroll: UniformListScrollHandle::new(),
            action_scroll: ScrollHandle::new(),
            visible_result_range: 0..0,
            last_presented_icon_range: None,
            pointer_selection_gate: PointerSelectionGate::default(),
            ime_enter_gate: ImeEnterGate::default(),
            hide_suppressed_until: Instant::now() + OPEN_GRACE,
            _subscriptions: vec![
                change_subscription,
                composition_subscription,
                key_subscription,
            ],
        }
    }

    fn prepare_for_show(
        &mut self,
        target: nxtools_protocol::ForegroundContext,
        navigation_context: Option<nxtools_protocol::DirectoryNavigationContext>,
        recent_directories: Vec<RecentDirectory>,
        results: Vec<SearchResult>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dismiss_preview(cx);
        self.target = target;
        self.navigation_context = navigation_context;
        self.results = results;
        self.icons.clear();
        self.selected = 0;
        self.action_mode = None;
        self.recent_directory_mode = false;
        self.recent_directories = recent_directories;
        self.status = None;
        self.is_open = true;
        self.results_visible = false;
        self.search_pending = false;
        self.reset_presented_results();
        self.result_scroll
            .0
            .borrow()
            .base_handle
            .set_offset(point(px(0.0), px(0.0)));
        self.query_dispatch_generation = self.query_dispatch_generation.wrapping_add(1).max(1);
        self.ime_enter_gate.reset();
        self.hide_suppressed_until = Instant::now() + OPEN_GRACE;
        window.resize(size(px(PANEL_WIDTH), px(COLLAPSED_PANEL_HEIGHT)));
        self.search_input.update(cx, |state, cx| {
            state.set_placeholder("搜索应用、文件或文件夹", window, cx);
            state.set_value(String::new(), window, cx);
        });
        cx.notify();
    }

    fn sync_window_content_size(&mut self, query: &str, window: &mut Window) {
        let visible = quick_launch_results_visible(
            query,
            self.action_mode.is_some(),
            self.recent_directory_mode,
        );
        if visible == self.results_visible {
            return;
        }
        self.results_visible = visible;
        if visible {
            self.results_appearance_generation =
                self.results_appearance_generation.wrapping_add(1).max(1);
        }
        window.resize(size(
            px(PANEL_WIDTH),
            px(quick_launch_panel_height(
                query,
                self.action_mode.is_some(),
                self.recent_directory_mode,
            )),
        ));
    }

    fn schedule_search(&mut self, query: String, cx: &mut Context<Self>) {
        let generation = self.query_dispatch_generation;
        self.search_pending = true;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SEARCH_INPUT_DEBOUNCE).await;
            let _ = this.update(cx, |this, cx| {
                if !this.is_open
                    || this.action_mode.is_some()
                    || this.query_dispatch_generation != generation
                    || this.search_input.read(cx).value().as_ref() != query
                {
                    return;
                }
                this.run_search(query, cx);
            });
        })
        .detach();
    }

    fn run_search(&mut self, query: String, cx: &mut Context<Self>) {
        let update = self
            .application
            .update(cx, |application, cx| {
                application.search_and_request_icons(&query, cx)
            })
            .unwrap_or(QuickLaunchSearchUpdate {
                results: Vec::new(),
                pending: false,
            });
        self.search_pending = update.pending;
        if apply_search_update(&mut self.results, update.results, update.pending) {
            self.selected = 0;
            self.result_scroll
                .0
                .borrow()
                .base_handle
                .set_offset(point(px(0.0), px(0.0)));
            self.reset_presented_results();
            self.retain_icons_for_results();
            self.pointer_selection_gate.reset();
        }
        cx.notify();
    }

    fn focus_search(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.search_input.focus_handle(cx).focus(window);
    }

    fn enter_recent_directory_mode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.action_mode.is_some()
            || self.recent_directory_mode
            || self.navigation_context.is_none()
        {
            return;
        }
        let results = recent_directory_results(&self.recent_directories, "");
        if results.is_empty() {
            self.recent_directories.clear();
            cx.notify();
            return;
        }
        self.dismiss_preview(cx);
        self.recent_directory_mode = true;
        self.results = results;
        self.selected = 0;
        self.search_pending = false;
        self.reset_presented_results();
        self.result_scroll
            .0
            .borrow()
            .base_handle
            .set_offset(point(px(0.0), px(0.0)));
        self.search_input.update(cx, |state, cx| {
            state.set_placeholder("筛选最近目录", window, cx);
            state.set_value(String::new(), window, cx);
        });
        self.sync_window_content_size("", window);
        self.focus_search(window, cx);
        cx.notify();
    }

    fn exit_recent_directory_mode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.recent_directory_mode || self.action_mode.is_some() {
            return;
        }
        self.dismiss_preview(cx);
        self.recent_directory_mode = false;
        self.results.clear();
        self.selected = 0;
        self.reset_presented_results();
        self.search_input.update(cx, |state, cx| {
            state.set_placeholder("搜索应用、文件或文件夹", window, cx);
            state.set_value(String::new(), window, cx);
        });
        self.sync_window_content_size("", window);
        self.focus_search(window, cx);
        cx.notify();
    }

    fn handle_navigation_key(
        &mut self,
        key: &str,
        modifiers: gpui::Modifiers,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.action_mode.is_none()
            && !self.recent_directory_mode
            && let Some(index) = quick_launch_shortcut_target(
                key,
                modifiers,
                self.visible_result_range.clone(),
                self.results.len(),
            )
        {
            self.dismiss_preview(cx);
            self.selected = index;
            let action = quick_launch_primary_action(&self.results[index], false, false);
            cx.notify();
            self.activate_selected(action, QuickLaunchActionTrigger::Shortcut, cx);
            return true;
        }
        if modifiers.modified() {
            return false;
        }
        if key.eq_ignore_ascii_case("escape") {
            if self.recent_directory_mode && self.action_mode.is_none() {
                self.exit_recent_directory_mode(window, cx);
                return true;
            }
            match quick_launch_escape_action(
                self.action_mode.is_some(),
                self.search_input.read(cx).value().is_empty(),
            ) {
                QuickLaunchEscapeAction::ExitActions => self.exit_action_mode(window, cx),
                QuickLaunchEscapeAction::ClearQuery => {
                    self.search_input
                        .update(cx, |state, cx| state.set_value(String::new(), window, cx));
                }
                QuickLaunchEscapeAction::Close => self.close(cx),
            }
            return true;
        }
        if key.eq_ignore_ascii_case("up") {
            self.move_selection(-1, window, cx);
            return true;
        }
        if key.eq_ignore_ascii_case("down") {
            self.move_selection(1, window, cx);
            return true;
        }
        if self.action_mode.is_some()
            && (key.eq_ignore_ascii_case("left") || key.eq_ignore_ascii_case("backspace"))
            && self.search_input.read(cx).value().is_empty()
        {
            self.exit_action_mode(window, cx);
            return true;
        }
        if self.action_mode.is_some() && key.eq_ignore_ascii_case("right") {
            return self.enter_selected_submenu(window, cx);
        }
        if self.action_mode.is_none() && key.eq_ignore_ascii_case("right") {
            let caret_is_at_end = {
                let input = self.search_input.read(cx);
                input.cursor() == input.value().len()
            };
            if caret_is_at_end {
                self.enter_action_mode(self.selected, window, cx);
                return true;
            }
        }
        false
    }

    fn move_selection(&mut self, delta: isize, window: &mut Window, cx: &mut Context<Self>) {
        self.dismiss_preview(cx);
        if let Some(mode) = self.action_mode.as_mut() {
            mode.move_selection(delta);
            self.pointer_selection_gate.reset();
            self.action_scroll
                .scroll_to_item(mode.selected().saturating_add(1));
            cx.notify();
            return;
        }
        if self.results.is_empty() {
            return;
        }
        let count = self.results.len() as isize;
        self.selected = (self.selected as isize + delta).rem_euclid(count) as usize;
        self.pointer_selection_gate.reset();
        self.result_scroll
            .scroll_to_item(self.selected, ScrollStrategy::Center);
        self.schedule_keyboard_selection_preview(window, cx);
        cx.notify();
    }

    fn schedule_keyboard_selection_preview(&self, window: &Window, cx: &mut Context<Self>) {
        let Some(result) = self.results.get(self.selected).cloned() else {
            return;
        };
        let panel_bounds = window.bounds();
        let panel_scale_factor = window.scale_factor();
        schedule_application_update(
            self.application.clone(),
            move |application, cx| {
                application.schedule_quick_launch_preview(
                    true,
                    result,
                    panel_bounds,
                    panel_scale_factor,
                    cx,
                );
            },
            cx,
        );
    }

    fn pointer_moved_result(&mut self, index: usize, position: (f32, f32), cx: &mut Context<Self>) {
        if self.pointer_selection_gate.observe_move(index, position) {
            self.select_result(index, cx);
            if !self.recent_directory_mode
                && let Some(result) = self.results.get(index).cloned()
            {
                schedule_application_update(
                    self.application.clone(),
                    move |application, cx| {
                        application.request_icons(std::slice::from_ref(&result), cx);
                    },
                    cx,
                );
            }
        }
    }

    fn dismiss_preview(&self, cx: &mut Context<Self>) {
        schedule_application_update(
            self.application.clone(),
            |application, cx| application.dismiss_quick_launch_preview(cx),
            cx,
        );
    }

    fn reset_presented_results(&mut self) {
        self.visible_result_range = 0..0;
        self.last_presented_icon_range = None;
    }

    fn update_presented_results(&mut self, visible_range: Range<usize>, cx: &mut Context<Self>) {
        self.visible_result_range = visible_range.clone();
        if self.action_mode.is_some()
            || self.recent_directory_mode
            || visible_range.is_empty()
            || self.results.is_empty()
        {
            return;
        }
        let range = visible_icon_prefetch_range(
            visible_range.start,
            visible_range.end.saturating_sub(1),
            self.results.len(),
        );
        if self.last_presented_icon_range.as_ref() == Some(&range) {
            return;
        }
        self.last_presented_icon_range = Some(range.clone());
        let results = self.results[range].to_vec();
        if results.is_empty() {
            return;
        }
        schedule_application_update(
            self.application.clone(),
            move |application, cx| application.request_icons(&results, cx),
            cx,
        );
    }

    fn pointer_moved_action(&mut self, index: usize, position: (f32, f32), cx: &mut Context<Self>) {
        if self.pointer_selection_gate.observe_move(index, position) {
            self.select_action(index, cx);
        }
    }

    fn select_result(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.results.len() || self.selected == index {
            return;
        }
        self.selected = index;
        self.pointer_selection_gate.reset();
        cx.notify();
    }

    fn select_action(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(mode) = self.action_mode.as_mut() else {
            return;
        };
        if index >= mode.visible_actions().len() || mode.selected() == index {
            return;
        }
        mode.set_selected(index);
        cx.notify();
    }

    fn enter_action_mode(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.action_mode.is_some() {
            return;
        }
        self.dismiss_preview(cx);
        let Some(target) = self.results.get(index).cloned() else {
            return;
        };
        self.selected = index;
        let query = self.search_input.read(cx).value().to_string();
        let entry_id = target.entry.id.clone();
        let target_path = target.entry.target.clone();
        let query_shell_actions = quick_launch_result_capabilities(&target).shell_actions;
        let mode = QuickLaunchActionMode::enter_in_context(
            target,
            query,
            index,
            self.navigation_context.as_ref(),
        );
        let mode_id = mode.id();
        let level_id = mode.current_level_id();
        self.action_mode = Some(mode);
        self.status = None;
        self.search_input.update(cx, |state, cx| {
            state.set_placeholder("搜索操作", window, cx);
            state.set_value(String::new(), window, cx);
        });
        self.focus_search(window, cx);
        if query_shell_actions {
            schedule_application_update(
                self.application.clone(),
                move |application, cx| {
                    application.query_shell_actions(mode_id, level_id, entry_id, target_path, cx);
                },
                cx,
            );
        }
        cx.notify();
    }

    fn exit_action_mode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(mode) = self.action_mode.as_mut()
            && let Some(filter) = mode.pop_level()
        {
            self.action_scroll.set_offset(point(px(0.0), px(0.0)));
            self.pointer_selection_gate.reset();
            self.search_input.update(cx, |state, cx| {
                state.set_placeholder("搜索操作", window, cx);
                state.set_value(filter, window, cx);
            });
            self.focus_search(window, cx);
            cx.notify();
            return;
        }
        let Some(context) = self.action_mode.as_ref().map(|mode| mode.parent.clone()) else {
            return;
        };
        self.selected = selection_for_results_context(&self.results, &context);
        self.pointer_selection_gate.reset();
        self.action_scroll.set_offset(point(px(0.0), px(0.0)));
        let placeholder = if self.recent_directory_mode {
            "筛选最近目录"
        } else {
            "搜索应用、文件或文件夹"
        };
        self.search_input.update(cx, |state, cx| {
            state.set_placeholder(placeholder, window, cx);
            state.set_value(context.query, window, cx);
        });
        let restored = self
            .action_mode
            .take()
            .expect("action mode remains active while restoring its parent query")
            .exit();
        debug_assert_eq!(restored.selected_entry_id, context.selected_entry_id);
        self.focus_search(window, cx);
        cx.notify();
    }

    fn replace_results(&mut self, results: Vec<SearchResult>, cx: &mut Context<Self>) {
        self.dismiss_preview(cx);
        self.selected = selection_after_results_refresh(&self.results, self.selected, &results);
        self.results = results;
        self.retain_icons_for_results();
        self.reset_presented_results();
        self.result_scroll
            .scroll_to_item(self.selected, ScrollStrategy::Center);
        self.search_pending = false;
        self.status = None;
        cx.notify();
    }

    fn activate_selected(
        &mut self,
        action: QuickLaunchAction,
        trigger: QuickLaunchActionTrigger,
        cx: &mut Context<Self>,
    ) {
        let Some(result) = self.results.get(self.selected).cloned() else {
            return;
        };
        let query = self.search_input.read(cx).value().to_string();
        schedule_application_update(
            self.application.clone(),
            move |application, cx| application.execute(result, action, query, trigger, cx),
            cx,
        );
    }

    fn activate_selected_recent_directory(&mut self, cx: &mut Context<Self>) {
        let Some(result) = self.results.get(self.selected).cloned() else {
            return;
        };
        let Some(action) =
            recent_directory_navigation_action(&result, self.navigation_context.as_ref())
        else {
            return;
        };
        schedule_application_update(
            self.application.clone(),
            move |application, cx| {
                application.execute(
                    result,
                    action,
                    String::new(),
                    QuickLaunchActionTrigger::RecentDirectory,
                    cx,
                )
            },
            cx,
        );
    }

    fn enter_selected_submenu(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some((mode_id, entry_id, submenu)) = self.action_mode.as_mut().and_then(|mode| {
            let mode_id = mode.id();
            let entry_id = mode.target().entry.id.clone();
            mode.push_selected_submenu()
                .map(|submenu| (mode_id, entry_id, submenu))
        }) else {
            return false;
        };
        self.action_scroll.set_offset(point(px(0.0), px(0.0)));
        self.pointer_selection_gate.reset();
        self.search_input.update(cx, |state, cx| {
            state.set_placeholder("搜索子操作", window, cx);
            state.set_value(String::new(), window, cx);
        });
        self.focus_search(window, cx);
        schedule_application_update(
            self.application.clone(),
            move |application, cx| {
                application.query_shell_submenu(mode_id, submenu, entry_id, cx);
            },
            cx,
        );
        cx.notify();
        true
    }

    fn activate_selected_action(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.enter_selected_submenu(window, cx) {
            return;
        }
        let Some(mode) = self.action_mode.as_ref() else {
            return;
        };
        let Some(action) = mode.selected_action() else {
            return;
        };
        let result = mode.target().clone();
        let query = mode.parent.query.clone();
        schedule_application_update(
            self.application.clone(),
            move |application, cx| {
                application.execute(
                    result,
                    action,
                    query,
                    QuickLaunchActionTrigger::ActionMode,
                    cx,
                )
            },
            cx,
        );
    }

    fn apply_shell_actions(
        &mut self,
        mode_id: uuid::Uuid,
        level_id: uuid::Uuid,
        entry_id: &str,
        menu: Option<ShellActionMenu>,
        cx: &mut Context<Self>,
    ) {
        let Some(mode) = self.action_mode.as_mut() else {
            return;
        };
        if !mode.matches_shell_query(mode_id, level_id, entry_id) {
            return;
        }
        mode.apply_shell_menu(level_id, menu);
        self.action_scroll.set_offset(point(px(0.0), px(0.0)));
        self.pointer_selection_gate.reset();
        cx.notify();
    }

    fn close(&mut self, cx: &mut Context<Self>) {
        if !self.is_open {
            return;
        }
        let captured_window_handle = self.target.window_handle;
        self.is_open = false;
        self.query_dispatch_generation = self.query_dispatch_generation.wrapping_add(1).max(1);
        cx.notify();
        schedule_application_update(
            self.application.clone(),
            move |application, cx| {
                application.hide_popup_and_restore(captured_window_handle, cx);
            },
            cx,
        );
    }

    fn should_dismiss_for_foreground(
        &self,
        popup_window_handle: i64,
        preview_window_handle: i64,
        foreground_window_handle: i64,
    ) -> bool {
        should_hide_for_foreground(
            self.is_open,
            Instant::now() >= self.hide_suppressed_until,
            popup_window_handle,
            preview_window_handle,
            foreground_window_handle,
        )
    }

    fn should_dismiss_for_pointer(
        &self,
        popup_window_handle: i64,
        pointer_root_window_handle: i64,
        pointer_process_id: u32,
    ) -> bool {
        should_hide_for_pointer(
            self.is_open,
            popup_window_handle,
            pointer_root_window_handle,
            pointer_process_id,
            std::process::id(),
        )
    }

    fn set_error(&mut self, message: String, cx: &mut Context<Self>) {
        self.set_status(message, cx);
    }

    fn set_status(&mut self, message: String, cx: &mut Context<Self>) {
        self.search_pending = false;
        self.status = Some(message);
        cx.notify();
    }

    fn retain_icons_for_results(&mut self) {
        let visible_ids = self
            .results
            .iter()
            .map(|result| result.entry.id.as_str())
            .collect::<HashSet<_>>();
        self.icons
            .retain(|entry_id, _| visible_ids.contains(entry_id.as_str()));
    }

    fn set_icons(
        &mut self,
        icons: impl IntoIterator<Item = (String, PathBuf)>,
        cx: &mut Context<Self>,
    ) {
        let mut changed = false;
        for (entry_id, path) in icons {
            if self
                .icons
                .get(&entry_id)
                .is_none_or(|existing| existing != &path)
            {
                self.icons.insert(entry_id, path);
                changed = true;
            }
        }
        if changed {
            cx.notify();
        }
    }

    fn render_result(
        &self,
        index: usize,
        visible_position: usize,
        result: &SearchResult,
        query: String,
        panel: WeakEntity<QuickLaunchPanel>,
    ) -> AnyElement {
        let theme = self.theme;
        let selected = index == self.selected;
        let source = result.entry.source;
        let result_for_click = result.clone();
        let application = self.application.clone();
        let click_action = if self.recent_directory_mode {
            recent_directory_navigation_action(result, self.navigation_context.as_ref())
        } else {
            Some(quick_launch_primary_action(result, false, false))
        };
        let preview_application = self.application.clone();
        let preview_result = result.clone();
        let pointer_panel = panel.clone();
        let press_panel = panel.clone();
        let right_click_panel = panel;
        let icon_path = self.icons.get(&result.entry.id).cloned();
        let has_icon_path = icon_path.is_some();
        let subtitle = if let Some(subtitle) = &result.subtitle {
            subtitle.clone()
        } else if result.copy_text().is_some() {
            "按 Enter 复制结果".to_owned()
        } else {
            result.entry.target.clone()
        };
        let shortcut_hint = quick_launch_shortcut_hint(visible_position);
        let name_highlights = quick_launch_match_ranges(&result.entry.name, &query)
            .into_iter()
            .map(|range| {
                (
                    range,
                    HighlightStyle {
                        color: Some(theme.accent.into()),
                        font_weight: Some(gpui::FontWeight::SEMIBOLD),
                        ..Default::default()
                    },
                )
            })
            .collect::<Vec<_>>();
        quick_launch_row_shell(theme, selected)
            .id(("quick-launch-result", index))
            .debug_selector(|| "quick-launch-result-row".into())
            .cursor_pointer()
            .on_hover(move |hovered, window, cx| {
                let entered = *hovered;
                let panel_bounds = window.bounds();
                let panel_scale_factor = window.scale_factor();
                let result = preview_result.clone();
                schedule_application_update(
                    preview_application.clone(),
                    move |application, cx| {
                        application.schedule_quick_launch_preview(
                            entered,
                            result,
                            panel_bounds,
                            panel_scale_factor,
                            cx,
                        )
                    },
                    cx,
                );
            })
            .on_mouse_move(move |event, _window, cx| {
                let position = (event.position.x.as_f32(), event.position.y.as_f32());
                let _ = pointer_panel.update(cx, |panel, cx| {
                    panel.pointer_moved_result(index, position, cx);
                });
            })
            .on_mouse_down(gpui::MouseButton::Left, move |_event, _window, cx| {
                let _ = press_panel.update(cx, |panel, cx| {
                    panel.select_result(index, cx);
                });
            })
            .on_mouse_down(gpui::MouseButton::Right, move |_event, window, cx| {
                let _ = right_click_panel.update(cx, |panel, cx| {
                    panel.enter_action_mode(index, window, cx);
                });
                cx.stop_propagation();
            })
            .on_click(move |_event, _window, cx| {
                let Some(action) = click_action.clone() else {
                    return;
                };
                let result = result_for_click.clone();
                let query = query.clone();
                schedule_application_update(
                    application.clone(),
                    move |application, cx| {
                        application.execute(
                            result,
                            action,
                            query,
                            QuickLaunchActionTrigger::Pointer,
                            cx,
                        )
                    },
                    cx,
                );
            })
            .child(
                div()
                    .debug_selector(move || format!("quick-launch-result-icon-{index}"))
                    .size(px(COMPACT_RESULT_ICON_RAIL_WIDTH))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(if selected {
                        theme.icon
                    } else {
                        theme.icon_muted
                    })
                    .when_some(icon_path, |container, path| {
                        container.child(
                            img(path)
                                .size(px(21.0))
                                .object_fit(ObjectFit::Contain)
                                .with_loading(move || {
                                    quick_launch_result_fallback_icon(index, source)
                                })
                                .with_fallback(move || {
                                    quick_launch_result_fallback_icon(index, source)
                                }),
                        )
                    })
                    .when(!has_icon_path, |container| {
                        container.child(quick_launch_result_fallback_icon(index, source))
                    }),
            )
            .child(
                div()
                    .debug_selector(move || format!("quick-launch-result-copy-{index}"))
                    .min_w_0()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(SPACING_2))
                    .child(
                        div()
                            .debug_selector(move || format!("quick-launch-result-title-{index}"))
                            .h(px(LINE_HEIGHT_BODY))
                            .text_size(px(TEXT_BODY))
                            .line_height(px(LINE_HEIGHT_BODY))
                            .text_color(theme.text)
                            .truncate()
                            .child(
                                StyledText::new(result.entry.name.clone())
                                    .with_highlights(name_highlights),
                            ),
                    )
                    .child(
                        div()
                            .debug_selector(move || format!("quick-launch-result-subtitle-{index}"))
                            .h(px(LINE_HEIGHT_LABEL))
                            .text_size(px(TEXT_LABEL))
                            .line_height(px(LINE_HEIGHT_LABEL))
                            .text_color(if selected {
                                theme.text_secondary
                            } else {
                                theme.text_muted
                            })
                            .truncate()
                            .child(subtitle),
                    ),
            )
            .child(
                div()
                    .debug_selector(move || format!("quick-launch-result-shortcut-{index}"))
                    .w(px(COMPACT_RESULT_TRAILING_RAIL_WIDTH))
                    .h(px(LINE_HEIGHT_LABEL))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_end()
                    .text_size(px(TEXT_LABEL))
                    .line_height(px(LINE_HEIGHT_LABEL))
                    .text_color(theme.text_muted)
                    .when_some(shortcut_hint, |rail, hint| rail.child(hint)),
            )
            .into_any_element()
    }

    fn render_action(
        &self,
        index: usize,
        item: QuickLaunchActionItem,
        panel: WeakEntity<QuickLaunchPanel>,
    ) -> AnyElement {
        let theme = self.theme;
        let selected = self
            .action_mode
            .as_ref()
            .is_some_and(|mode| mode.selected() == index);
        let enabled = item.is_activatable();
        let has_submenu = item.submenu.is_some();
        let pointer_panel = panel.clone();
        let click_panel = panel;
        let section_label = item.section_label.clone();
        let divider_before = item.divider_before;
        let row = quick_launch_row_shell(theme, selected)
            .id(("quick-launch-action", index))
            .debug_selector(move || format!("quick-launch-action-row-{index}"))
            .when(enabled, |row| row.cursor_pointer())
            .when(enabled, |row| {
                row.on_mouse_move(move |event, _window, cx| {
                    let position = (event.position.x.as_f32(), event.position.y.as_f32());
                    let _ = pointer_panel.update(cx, |panel, cx| {
                        panel.pointer_moved_action(index, position, cx);
                    });
                })
                .on_click(move |_event, window, cx| {
                    let _ = click_panel.update(cx, |panel, cx| {
                        panel.select_action(index, cx);
                        panel.activate_selected_action(window, cx);
                    });
                })
            })
            .child(
                div()
                    .debug_selector(move || format!("quick-launch-action-icon-{index}"))
                    .size(px(COMPACT_RESULT_ICON_RAIL_WIDTH))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(if selected {
                        theme.icon
                    } else {
                        theme.icon_muted
                    })
                    .when_some(item.icon_path.clone(), |container, icon_path| {
                        container
                            .child(img(icon_path).size(px(18.0)).object_fit(ObjectFit::Contain))
                    })
                    .when_some(item.icon, |container, icon| {
                        container.child(gpui_component::Icon::new(icon).size(px(18.0)))
                    }),
            )
            .child(
                div()
                    .debug_selector(move || format!("quick-launch-action-copy-{index}"))
                    .min_w_0()
                    .flex_1()
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .debug_selector(move || format!("quick-launch-action-title-{index}"))
                            .h(px(LINE_HEIGHT_BODY))
                            .text_size(px(TEXT_BODY))
                            .line_height(px(LINE_HEIGHT_BODY))
                            .text_color(if item.enabled {
                                theme.text
                            } else {
                                theme.text_muted
                            })
                            .truncate()
                            .child(item.label.clone()),
                    ),
            )
            .child(
                div()
                    .debug_selector(move || format!("quick-launch-action-trailing-{index}"))
                    .w(px(COMPACT_RESULT_TRAILING_RAIL_WIDTH))
                    .h(px(LINE_HEIGHT_LABEL))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_end()
                    .when(has_submenu, |rail| {
                        rail.child(
                            gpui_component::Icon::new(MaterialIcon::ChevronRight)
                                .size(px(16.0))
                                .text_color(theme.icon_muted),
                        )
                    }),
            );
        div()
            .when(divider_before, |container| {
                container
                    .mt(px(4.0))
                    .pt(px(4.0))
                    .border_t_1()
                    .border_color(theme.border_subtle)
            })
            .when_some(section_label, |container, label| {
                container.child(
                    div()
                        .h(px(22.0))
                        .px(px(8.0))
                        .flex()
                        .items_center()
                        .text_size(px(10.0))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.text_muted)
                        .child(label),
                )
            })
            .child(row)
            .into_any_element()
    }
}

impl Render for QuickLaunchPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let panel = cx.entity().downgrade();
        let recent_button_panel = panel.clone();
        let preview_scroll_application = self.application.clone();
        let query = self.search_input.read(cx).value().to_string();
        let recent_button_visible = !self.recent_directories.is_empty()
            && self.navigation_context.is_some()
            && self.action_mode.is_none()
            && (self.recent_directory_mode || query.trim().is_empty());
        let recent_directory_mode = self.recent_directory_mode;
        let action_target = self.action_mode.as_ref().map(|mode| {
            (
                mode.title().to_string(),
                mode.target().entry.name.clone(),
                mode.shell_pending(),
            )
        });
        let action_entries = if let Some(mode) = self.action_mode.as_ref() {
            mode.visible_actions()
                .into_iter()
                .enumerate()
                .map(|(index, item)| self.render_action(index, item, panel.clone()))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let result_surface = if let Some((level_title, target, pending)) = action_target {
            div()
                .id("quick-launch-actions-scroll")
                .flex_1()
                .min_h_0()
                .track_scroll(&self.action_scroll)
                .overflow_y_scroll()
                .p(px(SURFACE_INSET))
                .flex()
                .flex_col()
                .child(
                    div()
                        .debug_selector(|| "quick-launch-action-context".into())
                        .h(px(QUICK_LAUNCH_ACTION_CONTEXT_HEIGHT))
                        .mb(px(SPACING_2))
                        .px(px(ROW_HORIZONTAL_PADDING))
                        .flex()
                        .items_center()
                        .gap(px(SPACING_8))
                        .text_size(px(TEXT_LABEL))
                        .line_height(px(LINE_HEIGHT_LABEL))
                        .text_color(theme.text_muted)
                        .child(
                            div()
                                .flex_none()
                                .text_color(theme.text_secondary)
                                .child(level_title),
                        )
                        .child(div().min_w_0().flex_1().truncate().child(target)),
                )
                .when(pending, |container| {
                    container.child(progress_state(
                        theme,
                        "quick-launch-actions-progress",
                        "正在加载 Windows 操作",
                        "正在读取所选目标的系统操作。",
                    ))
                })
                .when(action_entries.is_empty() && !pending, |container| {
                    container.child(empty_state(
                        theme,
                        "quick-launch-actions-empty",
                        MaterialIcon::Search,
                        "没有匹配的操作",
                        "尝试缩短关键词或返回上一级操作。",
                    ))
                })
                .children(action_entries)
                .vertical_scrollbar(&self.action_scroll)
                .into_any_element()
        } else if self.results.is_empty() {
            div()
                .flex_1()
                .min_h_0()
                .when(self.search_pending, |container| {
                    container.child(progress_state(
                        theme,
                        "quick-launch-results-progress",
                        "正在搜索",
                        "正在合并应用、本地文件与系统工具结果。",
                    ))
                })
                .when(!self.search_pending, |container| {
                    container.child(empty_state(
                        theme,
                        "quick-launch-results-empty",
                        MaterialIcon::Search,
                        QUICK_LAUNCH_EMPTY_RESULTS_LABEL,
                        "尝试缩短关键词，或输入应用、文件和命令名称。",
                    ))
                })
                .into_any_element()
        } else {
            let result_count = self.results.len();
            let result_query = query.clone();
            let result_panel = panel.clone();
            let list = uniform_list(
                "quick-launch-results",
                result_count,
                cx.processor(move |panel, visible_range: Range<usize>, _window, cx| {
                    let visible_start = visible_range.start;
                    panel.update_presented_results(visible_range.clone(), cx);
                    visible_range
                        .filter_map(|index| {
                            panel.results.get(index).map(|result| {
                                panel.render_result(
                                    index,
                                    index.saturating_sub(visible_start),
                                    result,
                                    result_query.clone(),
                                    result_panel.clone(),
                                )
                            })
                        })
                        .collect::<Vec<_>>()
                }),
            )
            .size_full()
            .p(px(SURFACE_INSET))
            .track_scroll(self.result_scroll.clone());
            div()
                .id("quick-launch-results-scroll")
                .relative()
                .flex_1()
                .min_h_0()
                .on_scroll_wheel(move |_event, _window, cx| {
                    schedule_application_update(
                        preview_scroll_application.clone(),
                        |application, cx| {
                            application.suspend_quick_launch_preview_for_scroll(cx);
                        },
                        cx,
                    );
                })
                .child(list)
                .vertical_scrollbar(&self.result_scroll)
                .into_any_element()
        };
        let result_surface = div()
            .debug_selector(|| "quick-launch-results-surface".into())
            .w_full()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(result_surface)
            .with_animation(
                ("quick-launch-results", self.results_appearance_generation),
                Animation::new(RESULTS_APPEAR_DURATION).with_easing(ease_out_quint()),
                |surface, delta| surface.relative().top(px(1.0 * (1.0 - delta))),
            );
        div()
            .id("quick-launch-window")
            .debug_selector(|| "quick-launch-window".into())
            .relative()
            .size_full()
            .overflow_hidden()
            .font_family(UI_FONT_FAMILY)
            .bg(theme.canvas)
            .text_color(theme.text)
            .border_1()
            .border_color(theme.window_border)
            .flex()
            .flex_col()
            .child(
                div()
                    .debug_selector(|| "quick-launch-search-header".into())
                    .h(px(SEARCH_HEADER_HEIGHT))
                    .px(px(SPACING_8))
                    .flex_none()
                    .border_b_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.canvas)
                    .flex()
                    .items_center()
                    .gap(px(SPACING_8))
                    .text_size(px(QUICK_LAUNCH_SEARCH_TEXT_SIZE))
                    .line_height(px(LINE_HEIGHT_BODY))
                    .child(
                        gpui_component::Icon::new(MaterialIcon::Search)
                            .size(px(16.0))
                            .text_color(theme.icon_muted),
                    )
                    .child(
                        div().min_w_0().flex_1().child(
                            Input::new(&self.search_input)
                                .appearance(false)
                                .bordered(false)
                                .focus_bordered(false)
                                .text_size(px(QUICK_LAUNCH_SEARCH_TEXT_SIZE))
                                .line_height(px(LINE_HEIGHT_BODY)),
                        ),
                    )
                    .when(recent_button_visible, |header| {
                        header.child(
                            div()
                                .id("quick-launch-recent-directories")
                                .size(px(27.0))
                                .rounded(px(COMPACT_RADIUS))
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .text_color(if recent_directory_mode {
                                    theme.icon
                                } else {
                                    theme.icon_muted
                                })
                                .bg(if recent_directory_mode {
                                    theme.element_selected
                                } else {
                                    theme.canvas
                                })
                                .hover(|button| button.bg(theme.surface_hover))
                                .on_click(move |_event, window, cx| {
                                    let _ = recent_button_panel.update(cx, |panel, cx| {
                                        if panel.recent_directory_mode {
                                            panel.exit_recent_directory_mode(window, cx);
                                        } else {
                                            panel.enter_recent_directory_mode(window, cx);
                                        }
                                    });
                                })
                                .child(
                                    gpui_component::Icon::new(MaterialIcon::History).size(px(15.0)),
                                ),
                        )
                    }),
            )
            .when(self.results_visible, |panel| panel.child(result_surface))
            .when_some(self.status.clone(), |panel, status| {
                panel.child(
                    div()
                        .absolute()
                        .left(px(12.0))
                        .right(px(12.0))
                        .bottom(px(8.0))
                        .h(px(28.0))
                        .px(px(12.0))
                        .rounded(px(COMPACT_RADIUS))
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.surface_raised)
                        .shadow_sm()
                        .flex()
                        .items_center()
                        .text_size(px(META_TEXT_SIZE))
                        .text_color(theme.text_secondary)
                        .child(status),
                )
            })
    }
}

fn quick_launch_result_fallback_icon(index: usize, source: ApplicationSource) -> AnyElement {
    div()
        .debug_selector(move || format!("quick-launch-result-icon-fallback-{index}"))
        .size(px(18.0))
        .flex_none()
        .child(gpui_component::Icon::new(source_icon(source)).size(px(18.0)))
        .into_any_element()
}

fn source_icon(source: ApplicationSource) -> MaterialIcon {
    match source {
        ApplicationSource::Calculator => MaterialIcon::Calculate,
        ApplicationSource::EnvironmentVariable => MaterialIcon::DataObject,
        ApplicationSource::CustomCommand => MaterialIcon::Terminal,
        ApplicationSource::WebUrl => MaterialIcon::Language,
        ApplicationSource::StartMenu => MaterialIcon::Dashboard,
        ApplicationSource::Desktop => MaterialIcon::OpenInNew,
        ApplicationSource::System => MaterialIcon::ProgressActivity,
        ApplicationSource::IndexedFile => MaterialIcon::TextFields,
        ApplicationSource::IndexedDirectory => MaterialIcon::Folder,
    }
}

fn quick_launch_bounds(
    request: &QuickLaunchPanelRequest,
    expanded: bool,
    cx: &App,
) -> (Bounds<gpui::Pixels>, Option<gpui::DisplayId>) {
    let desired = size(
        px(PANEL_WIDTH),
        px(if expanded {
            PANEL_HEIGHT
        } else {
            COLLAPSED_PANEL_HEIGHT
        }),
    );
    let display = cx
        .displays()
        .into_iter()
        .find(|display| u32::from(display.id()) == request.cursor_display_index)
        .or_else(|| cx.displays().into_iter().next());
    let Some(display) = display else {
        return (Bounds::new(point(px(100.0), px(100.0)), desired), None);
    };
    let screen = display.bounds();
    let panel_size = size(
        desired.width.min(screen.size.width),
        desired.height.min(screen.size.height),
    );
    let x = screen.left() + (screen.size.width - panel_size.width) / 2.0;
    let y = screen.top() + (screen.size.height - panel_size.height) / 4.0;
    (Bounds::new(point(x, y), panel_size), Some(display.id()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    #[gpui::test]
    fn production_result_row_keeps_height_through_animated_surface(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (_root, window) = cx.add_window_view(move |window, cx| {
            window.resize(size(px(PANEL_WIDTH), px(PANEL_HEIGHT)));
            let panel = cx.new(|cx| {
                let mut panel = QuickLaunchPanel::new(
                    vec![search_result(
                        "first-result",
                        ApplicationSource::IndexedFile,
                    )],
                    nxtools_protocol::ForegroundContext {
                        window_handle: 1,
                        focus_window_handle: 1,
                        process_id: 1,
                        process_name: "editor".into(),
                        window_title: "document".into(),
                    },
                    None,
                    Vec::new(),
                    WeakEntity::new_invalid(),
                    window,
                    cx,
                );
                panel.results_visible = true;
                panel
            });
            Root::new(panel, window, cx)
        });

        window
            .refresh()
            .expect("render populated quick-launch results");
        let row = window
            .debug_bounds("quick-launch-result-row")
            .expect("production quick-launch result row");
        assert_eq!(row.size.height, px(40.0));
    }

    #[gpui::test]
    fn production_keyboard_pointer_movement_and_press_share_one_selection(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (_temp, application, endpoint) = quick_launch_application_fixture(cx);
        let panel_slot = Rc::new(RefCell::new(None));
        let panel_slot_for_window = Rc::clone(&panel_slot);
        let application_for_window = application.clone();
        let (_root, window) = cx.add_window_view(move |window, cx| {
            window.resize(size(px(PANEL_WIDTH), px(PANEL_HEIGHT)));
            let panel = cx.new(|cx| {
                let mut panel = QuickLaunchPanel::new(
                    vec![
                        search_result("first", ApplicationSource::StartMenu),
                        search_result("second", ApplicationSource::IndexedFile),
                        search_result("third", ApplicationSource::Desktop),
                    ],
                    nxtools_protocol::ForegroundContext {
                        window_handle: 1,
                        focus_window_handle: 1,
                        process_id: 1,
                        process_name: "editor".into(),
                        window_title: "document".into(),
                    },
                    None,
                    Vec::new(),
                    application_for_window.downgrade(),
                    window,
                    cx,
                );
                panel.results_visible = true;
                panel
            });
            *panel_slot_for_window.borrow_mut() = Some(panel.clone());
            Root::new(panel, window, cx)
        });
        let panel = panel_slot
            .borrow_mut()
            .take()
            .expect("production quick-launch panel");

        window.refresh().expect("render quick-launch results");
        panel.update_in(window, |panel, window, cx| panel.focus_search(window, cx));
        window.simulate_keystrokes("down");
        panel.update_in(window, |panel, _, _| assert_eq!(panel.selected, 1));

        let third_row = window
            .debug_bounds("quick-launch-result-icon-2")
            .expect("third production result row");
        window.simulate_mouse_move(third_row.center(), None, gpui::Modifiers::default());
        panel.update_in(window, |panel, _, _| assert_eq!(panel.selected, 1));
        window.simulate_mouse_move(
            gpui::point(third_row.origin.x + px(2.0), third_row.center().y),
            None,
            gpui::Modifiers::default(),
        );
        panel.update_in(window, |panel, _, _| assert_eq!(panel.selected, 2));

        window.simulate_keystrokes("up");
        panel.update_in(window, |panel, _, _| assert_eq!(panel.selected, 1));
        window.simulate_mouse_down(
            third_row.center(),
            gpui::MouseButton::Left,
            gpui::Modifiers::default(),
        );

        panel.update_in(window, |panel, _, _| assert_eq!(panel.selected, 2));
        window.simulate_mouse_up(
            third_row.center(),
            gpui::MouseButton::Left,
            gpui::Modifiers::default(),
        );
        cx.run_until_parked();
        let command = (0..8)
            .filter_map(|_| endpoint.try_receive_command_for_contract_test().ok())
            .find(|envelope| {
                matches!(
                    &envelope.message,
                    nxtools_protocol::Message::PlatformCommand(
                        PlatformCommand::OpenShellTarget(open)
                    ) if open.target == "third.lnk" && open.action == ShellOpenAction::Open
                )
            })
            .expect("the directly clicked third result executes after becoming selected");
        application.read_with(cx, |application, _| {
            let pending = application
                .pending_actions
                .get(&command.id)
                .expect("direct click retains the correlated pending action");
            assert_eq!(pending.entry_id, "third");
            assert_eq!(pending.trigger, QuickLaunchActionTrigger::Pointer);
        });
    }

    #[gpui::test]
    fn production_heterogeneous_result_rows_keep_one_three_rail_geometry(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (_temp, application, endpoint) = quick_launch_application_fixture(cx);
        let results = [
            (
                "application",
                ApplicationSource::StartMenu,
                "NxTools",
                "应用",
            ),
            (
                "file",
                ApplicationSource::IndexedFile,
                "ui-remediation-plan.md",
                "D:\\RiderProject\\NxTools.Next\\docs",
            ),
            (
                "command",
                ApplicationSource::CustomCommand,
                "打开终端",
                "自定义命令",
            ),
            (
                "calculator",
                ApplicationSource::Calculator,
                "42",
                "计算结果",
            ),
            (
                "website",
                ApplicationSource::WebUrl,
                "OpenAI",
                "https://openai.com",
            ),
        ]
        .into_iter()
        .map(|(id, source, name, subtitle)| {
            let mut result = search_result(id, source);
            result.entry.name = name.into();
            result.subtitle = Some(subtitle.into());
            result
        })
        .collect::<Vec<_>>();
        let panel_slot = Rc::new(RefCell::new(None));
        let panel_slot_for_window = Rc::clone(&panel_slot);
        let application_for_window = application.clone();
        let (_root, window) = cx.add_window_view(move |window, cx| {
            window.resize(size(px(PANEL_WIDTH), px(PANEL_HEIGHT)));
            let panel = cx.new(|cx| {
                let mut panel = QuickLaunchPanel::new(
                    results,
                    nxtools_protocol::ForegroundContext {
                        window_handle: 1,
                        focus_window_handle: 1,
                        process_id: 1,
                        process_name: "editor".into(),
                        window_title: "document".into(),
                    },
                    None,
                    Vec::new(),
                    application_for_window.downgrade(),
                    window,
                    cx,
                );
                panel.results_visible = true;
                panel
            });
            *panel_slot_for_window.borrow_mut() = Some(panel.clone());
            Root::new(panel, window, cx)
        });
        let panel = panel_slot
            .borrow_mut()
            .take()
            .expect("heterogeneous quick-launch panel");

        window
            .refresh()
            .expect("render heterogeneous quick-launch results");
        assert!(
            window
                .debug_bounds("quick-launch-result-icon-fallback-0")
                .is_some(),
            "a result without a decoded Shell icon must keep a visible semantic fallback"
        );
        let mut expected_icon = None;
        let mut expected_copy = None;
        let mut expected_shortcut = None;
        let selectors = [
            (
                "quick-launch-result-icon-0",
                "quick-launch-result-copy-0",
                "quick-launch-result-title-0",
                "quick-launch-result-subtitle-0",
                "quick-launch-result-shortcut-0",
            ),
            (
                "quick-launch-result-icon-1",
                "quick-launch-result-copy-1",
                "quick-launch-result-title-1",
                "quick-launch-result-subtitle-1",
                "quick-launch-result-shortcut-1",
            ),
            (
                "quick-launch-result-icon-2",
                "quick-launch-result-copy-2",
                "quick-launch-result-title-2",
                "quick-launch-result-subtitle-2",
                "quick-launch-result-shortcut-2",
            ),
            (
                "quick-launch-result-icon-3",
                "quick-launch-result-copy-3",
                "quick-launch-result-title-3",
                "quick-launch-result-subtitle-3",
                "quick-launch-result-shortcut-3",
            ),
            (
                "quick-launch-result-icon-4",
                "quick-launch-result-copy-4",
                "quick-launch-result-title-4",
                "quick-launch-result-subtitle-4",
                "quick-launch-result-shortcut-4",
            ),
        ];
        for (icon_selector, copy_selector, title_selector, subtitle_selector, shortcut_selector) in
            selectors
        {
            let icon = window
                .debug_bounds(icon_selector)
                .expect("fixed result icon rail");
            let copy = window
                .debug_bounds(copy_selector)
                .expect("fixed result copy rail");
            let title = window.debug_bounds(title_selector).expect("result title");
            let subtitle = window
                .debug_bounds(subtitle_selector)
                .expect("result subtitle");
            let shortcut = window
                .debug_bounds(shortcut_selector)
                .expect("fixed shortcut rail");

            assert_eq!(icon.size.width, px(26.0));
            assert_eq!(title.size.height, px(LINE_HEIGHT_BODY));
            assert_eq!(subtitle.size.height, px(LINE_HEIGHT_LABEL));
            assert_eq!(shortcut.size.width, px(48.0));
            let icon_rail = (icon.origin.x, icon.size.width);
            let copy_rail = (copy.origin.x, copy.size.width);
            let shortcut_rail = (shortcut.origin.x, shortcut.size.width);
            assert_eq!(*expected_icon.get_or_insert(icon_rail), icon_rail);
            assert_eq!(*expected_copy.get_or_insert(copy_rail), copy_rail);
            assert_eq!(
                *expected_shortcut.get_or_insert(shortcut_rail),
                shortcut_rail
            );
        }

        panel.update_in(window, |panel, window, cx| panel.focus_search(window, cx));
        window.simulate_keystrokes("ctrl-2");
        panel.read_with(cx, |panel, _| assert_eq!(panel.selected, 1));
        cx.run_until_parked();
        let command = (0..8)
            .filter_map(|_| endpoint.try_receive_command_for_contract_test().ok())
            .find(|envelope| {
                matches!(
                    &envelope.message,
                    nxtools_protocol::Message::PlatformCommand(
                        PlatformCommand::OpenShellTarget(open)
                    ) if open.target == "file.lnk" && open.action == ShellOpenAction::Open
                )
            })
            .expect("typed command for the second visible result");
        let action_correlation_id = application.read_with(cx, |application, _| {
            let pending = application
                .pending_actions
                .get(&command.id)
                .expect("transport command id retains the pending action");
            assert_eq!(command.correlation_id, Some(pending.correlation_id));
            assert_eq!(pending.trigger, QuickLaunchActionTrigger::Shortcut);
            assert_eq!(pending.source, ApplicationSource::IndexedFile);
            assert_eq!(pending.action, QuickLaunchAction::Open);
            pending.correlation_id
        });
        application.update(cx, |application, cx| {
            application.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: command.id,
                    result: PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: None,
                    },
                },
                cx,
            );
        });
        application.read_with(cx, |application, _| {
            assert!(!application.pending_actions.contains_key(&command.id));
            assert_ne!(action_correlation_id, command.id);
        });
    }

    #[gpui::test]
    fn production_popup_keeps_search_only_collapsed_and_expands_one_stable_surface(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let panel_slot = Rc::new(RefCell::new(None));
        let panel_slot_for_window = Rc::clone(&panel_slot);
        let (_root, window) = cx.add_window_view(move |window, cx| {
            window.resize(size(px(PANEL_WIDTH), px(COLLAPSED_PANEL_HEIGHT)));
            let panel = cx.new(|cx| {
                QuickLaunchPanel::new(
                    Vec::new(),
                    nxtools_protocol::ForegroundContext {
                        window_handle: 1,
                        focus_window_handle: 1,
                        process_id: 1,
                        process_name: "editor".into(),
                        window_title: "document".into(),
                    },
                    None,
                    Vec::new(),
                    WeakEntity::new_invalid(),
                    window,
                    cx,
                )
            });
            *panel_slot_for_window.borrow_mut() = Some(panel.clone());
            Root::new(panel, window, cx)
        });
        let panel = panel_slot
            .borrow_mut()
            .take()
            .expect("production quick-launch panel");

        window
            .refresh()
            .expect("render collapsed quick-launch popup");
        assert!(window.debug_bounds("quick-launch-window").is_some());
        let collapsed_header = window
            .debug_bounds("quick-launch-search-header")
            .expect("collapsed quick-launch search header");
        window.update(|window, _| {
            assert_eq!(
                window.bounds().size,
                size(px(PANEL_WIDTH), px(COLLAPSED_PANEL_HEIGHT))
            );
        });
        assert_eq!(collapsed_header.size.height, px(SEARCH_HEADER_HEIGHT));
        assert!(
            window
                .debug_bounds("quick-launch-results-surface")
                .is_none()
        );

        panel.update_in(window, |panel, window, cx| {
            panel.sync_window_content_size("zed", window);
            cx.notify();
        });
        window
            .refresh()
            .expect("render expanded quick-launch popup");
        let expanded_root = window
            .debug_bounds("quick-launch-window")
            .expect("expanded quick-launch root");
        let expanded_header = window
            .debug_bounds("quick-launch-search-header")
            .expect("expanded quick-launch search header");
        let results_surface = window
            .debug_bounds("quick-launch-results-surface")
            .expect("expanded quick-launch results surface");
        window.update(|window, _| {
            assert_eq!(
                window.bounds().size,
                size(px(PANEL_WIDTH), px(PANEL_HEIGHT))
            );
        });
        assert_eq!(expanded_header, collapsed_header);
        assert_eq!(results_surface.top(), expanded_header.bottom() + px(1.0));
        assert_eq!(results_surface.bottom(), expanded_root.bottom());

        panel.update_in(window, |panel, window, cx| {
            let expanded_generation = panel.results_appearance_generation;
            panel.sync_window_content_size("zed editor", window);
            assert_eq!(panel.results_appearance_generation, expanded_generation);

            let mode = QuickLaunchActionMode::enter(
                search_result("first-result", ApplicationSource::IndexedFile),
                "zed editor".into(),
                0,
            );
            let mode_id = mode.id();
            let level_id = mode.current_level_id();
            let entry_id = mode.target().entry.id.clone();
            panel.action_mode = Some(mode);
            panel.apply_shell_actions(mode_id, level_id, &entry_id, None, cx);
            assert_eq!(panel.results_appearance_generation, expanded_generation);
        });
    }

    #[test]
    fn main_popup_search_uses_the_shared_body_type_role() {
        assert_eq!(QUICK_LAUNCH_SEARCH_TEXT_SIZE, TEXT_BODY);
        assert_eq!(LINE_HEIGHT_BODY, 18.0);
    }

    #[gpui::test]
    fn production_results_surface_uses_shared_empty_and_progress_states(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let panel_slot = Rc::new(RefCell::new(None));
        let panel_slot_for_window = Rc::clone(&panel_slot);
        let (_root, window) = cx.add_window_view(move |window, cx| {
            window.resize(size(px(PANEL_WIDTH), px(PANEL_HEIGHT)));
            let panel = cx.new(|cx| {
                let mut panel = QuickLaunchPanel::new(
                    Vec::new(),
                    nxtools_protocol::ForegroundContext {
                        window_handle: 1,
                        focus_window_handle: 1,
                        process_id: 1,
                        process_name: "editor".into(),
                        window_title: "document".into(),
                    },
                    None,
                    Vec::new(),
                    WeakEntity::new_invalid(),
                    window,
                    cx,
                );
                panel.results_visible = true;
                panel
            });
            *panel_slot_for_window.borrow_mut() = Some(panel.clone());
            Root::new(panel, window, cx)
        });
        let panel = panel_slot
            .borrow_mut()
            .take()
            .expect("production quick-launch panel");

        window.refresh().expect("render quick-launch empty state");
        window.update(|window, _| {
            assert_eq!(
                window.bounds().size,
                size(px(PANEL_WIDTH), px(PANEL_HEIGHT))
            );
        });
        let panel_surface = window
            .debug_bounds("quick-launch-window")
            .expect("quick-launch production surface");
        let empty = window
            .debug_bounds("quick-launch-results-empty")
            .expect("quick-launch shared empty state");
        let empty_icon = window
            .debug_bounds("quick-launch-results-empty-icon")
            .expect("quick-launch empty icon");
        let empty_title = window
            .debug_bounds("quick-launch-results-empty-title")
            .expect("quick-launch empty title");
        let empty_description = window
            .debug_bounds("quick-launch-results-empty-description")
            .expect("quick-launch empty description");
        assert_eq!(empty_icon.size, size(px(18.0), px(18.0)));
        assert_eq!(empty_title.size.height, px(18.0));
        assert_eq!(empty_description.size.height, px(16.0));
        assert_eq!(empty_title.left(), empty_description.left());
        assert_eq!(empty.left(), panel_surface.left() + px(1.0));
        assert_eq!(empty.right(), panel_surface.right() - px(1.0));

        panel.update_in(window, |panel, _window, cx| {
            panel.search_pending = true;
            cx.notify();
        });
        window
            .refresh()
            .expect("render quick-launch progress state");
        let progress = window
            .debug_bounds("quick-launch-results-progress")
            .expect("quick-launch shared progress state");
        let progress_icon = window
            .debug_bounds("quick-launch-results-progress-icon")
            .expect("quick-launch progress icon");
        let progress_title = window
            .debug_bounds("quick-launch-results-progress-title")
            .expect("quick-launch progress title");
        let progress_description = window
            .debug_bounds("quick-launch-results-progress-description")
            .expect("quick-launch progress description");
        assert_eq!(progress.size.width, empty.size.width);
        assert_eq!(progress_icon.size, empty_icon.size);
        assert_eq!(progress_title.size.height, empty_title.size.height);
        assert_eq!(
            progress_description.size.height,
            empty_description.size.height
        );
    }

    fn quick_launch_application_fixture(
        cx: &mut TestAppContext,
    ) -> (
        tempfile::TempDir,
        Entity<QuickLaunchApplication>,
        nxtools_ipc::PlatformServerEndpoint,
    ) {
        let temp = tempfile::tempdir().expect("temporary quick-launch application fixture");
        let catalog = FinderCatalog::open(temp.path().join("history.db"), &[])
            .expect("open quick-launch application fixture catalog");
        let config_path = temp.path().join("config.json");
        let logs_path = temp.path().join("logs");
        let (application, endpoint) = cx.update(|cx| {
            let notifications = cx.new(|_| NotificationCenter::default());
            let (platform, endpoint) = nxtools_ipc::platform_bridge(8);
            endpoint.set_connected_for_contract_test(true);
            let (indexer, _indexer_endpoint) = crate::runtime::indexer_bridge(8);
            let application = cx.new(|_| {
                QuickLaunchApplication::new(
                    catalog,
                    Rc::new(platform),
                    Rc::new(indexer),
                    AppConfig::default(),
                    config_path,
                    logs_path,
                    notifications,
                )
            });
            (application, endpoint)
        });
        (temp, application, endpoint)
    }

    fn quick_launch_panel_fixture(
        cx: &mut TestAppContext,
        results: Vec<SearchResult>,
    ) -> (Entity<QuickLaunchPanel>, &mut gpui::VisualTestContext) {
        let panel_slot = Rc::new(RefCell::new(None));
        let panel_slot_for_window = Rc::clone(&panel_slot);
        let (_root, window) = cx.add_window_view(move |window, cx| {
            window.resize(size(px(PANEL_WIDTH), px(PANEL_HEIGHT)));
            let panel = cx.new(|cx| {
                let mut panel = QuickLaunchPanel::new(
                    results,
                    nxtools_protocol::ForegroundContext {
                        window_handle: 1,
                        focus_window_handle: 1,
                        process_id: 1,
                        process_name: "editor".into(),
                        window_title: "document".into(),
                    },
                    None,
                    Vec::new(),
                    WeakEntity::new_invalid(),
                    window,
                    cx,
                );
                panel.results_visible = true;
                panel
            });
            *panel_slot_for_window.borrow_mut() = Some(panel.clone());
            Root::new(panel, window, cx)
        });
        let panel = panel_slot
            .borrow_mut()
            .take()
            .expect("production quick-launch panel");
        (panel, window)
    }

    fn search_result(id: &str, source: ApplicationSource) -> SearchResult {
        SearchResult {
            entry: nxtools_finder::ApplicationEntry {
                id: id.into(),
                name: id.into(),
                target: format!("{id}.lnk"),
                source,
            },
            action: if source == ApplicationSource::Calculator {
                SearchResultAction::CopyText { text: id.into() }
            } else {
                SearchResultAction::OpenTarget
            },
            subtitle: None,
            score: 0,
            activation_count: 0,
            last_activated_at_unix_ms: None,
            query_activation_count: 0,
            last_query_activation_at_unix_ms: None,
        }
    }

    #[test]
    fn result_shortcuts_address_only_the_visible_first_nine_rows() {
        let control = gpui::Modifiers {
            control: true,
            ..Default::default()
        };

        assert_eq!(quick_launch_shortcut_hint(0), Some("Ctrl+1"));
        assert_eq!(quick_launch_shortcut_hint(8), Some("Ctrl+9"));
        assert_eq!(quick_launch_shortcut_hint(9), None);
        assert_eq!(
            quick_launch_shortcut_target("1", control, 5..13, 20),
            Some(5)
        );
        assert_eq!(
            quick_launch_shortcut_target("8", control, 5..13, 20),
            Some(12)
        );
        assert_eq!(quick_launch_shortcut_target("9", control, 5..13, 20), None);
        assert_eq!(
            quick_launch_shortcut_target("1", gpui::Modifiers::default(), 5..13, 20),
            None
        );
    }

    fn finish_shell_loading(mode: &mut QuickLaunchActionMode) {
        let level_id = mode.current_level_id();
        mode.apply_shell_menu(level_id, None);
    }

    fn finish_shell_loading_with_navigation(
        mode: &mut QuickLaunchActionMode,
        navigation_directory: &str,
    ) {
        let level_id = mode.current_level_id();
        mode.apply_shell_menu(
            level_id,
            Some(ShellActionMenu {
                session_id: "navigation-session".into(),
                navigation_directory: Some(navigation_directory.into()),
                items: Vec::new(),
            }),
        );
    }

    #[test]
    fn result_name_highlights_follow_contiguous_and_fuzzy_matches() {
        assert_eq!(quick_launch_match_ranges("AyuGram.exe", "gram"), vec![3..7]);
        assert_eq!(
            quick_launch_match_ranges("AyuGram.exe", "agm"),
            vec![0..1, 3..4, 6..7]
        );
    }

    #[test]
    fn quick_launch_settings_map_to_the_indexer_contract() {
        let settings = QuickLaunchSettings {
            indexed_volumes: vec!["C:\\".into(), "D:\\".into()],
            excluded_paths: vec!["C:\\Build".into(), "D:\\Cache".into()],
            ..QuickLaunchSettings::default()
        };

        assert_eq!(
            index_settings_from_quick_launch(&settings),
            IndexSettings {
                indexed_volumes: vec!["C:\\".into(), "D:\\".into()],
                excluded_paths: vec!["C:\\Build".into(), "D:\\Cache".into()],
            }
        );
    }

    #[test]
    fn auto_quick_switch_request_becomes_one_file_dialog_navigation_command() {
        let settings = QuickLaunchSettings::default();
        let request = nxtools_protocol::QuickLaunchAutoSwitchRequest {
            target: nxtools_protocol::DirectoryNavigationContext {
                host: DirectoryNavigationHost::FileDialog,
                window_handle: 77,
                current_directory: Some(r"C:\Initial".into()),
                host_context_id: None,
            },
            source_directory: r"D:\Work".into(),
            requested_at_unix_ms: 10_000,
        };

        assert_eq!(
            auto_quick_switch_command(&settings, &request, 10_500),
            Some(NavigateDirectoryCommand {
                host: DirectoryNavigationHost::FileDialog,
                window_handle: 77,
                target_directory: r"D:\Work".into(),
                host_context_id: None,
            })
        );
        assert_eq!(auto_quick_switch_command(&settings, &request, 13_001), None);

        let mut disabled = settings;
        disabled.auto_quick_switch = false;
        assert_eq!(auto_quick_switch_command(&disabled, &request, 10_500), None);
    }

    #[test]
    fn icon_prefetch_tracks_the_visible_result_window_with_small_overscan() {
        assert_eq!(visible_icon_prefetch_range(0, 7, 50), 0..10);
        assert_eq!(visible_icon_prefetch_range(18, 25, 50), 16..28);
        assert_eq!(visible_icon_prefetch_range(47, 49, 50), 45..50);
    }

    #[test]
    fn failed_visible_icon_requests_retry_after_a_bounded_cooldown() {
        let failed_at = Instant::now();

        assert!(icon_retry_pending(
            failed_at,
            failed_at + ICON_RETRY_DELAY / 2
        ));
        assert!(!icon_retry_pending(failed_at, failed_at + ICON_RETRY_DELAY));
    }

    #[test]
    fn shell_icon_results_accept_valid_pngs_and_reject_undecodable_files() {
        let temp = tempfile::tempdir().expect("temporary Shell icon fixtures");
        let valid = temp.path().join("valid.png");
        let invalid = temp.path().join("invalid.png");
        let wrong_extension = temp.path().join("valid.svg");
        std::fs::write(&valid, crate::icons::ONE_PIXEL_PNG).expect("valid PNG fixture");
        std::fs::write(&invalid, b"not a decodable image").expect("invalid image fixture");
        std::fs::write(&wrong_extension, crate::icons::ONE_PIXEL_PNG)
            .expect("wrong-extension fixture");

        assert!(shell_icon_path_is_renderable(&valid));
        assert!(!shell_icon_path_is_renderable(&invalid));
        assert!(!shell_icon_path_is_renderable(&wrong_extension));
        assert!(!shell_icon_path_is_renderable(
            &temp.path().join("missing.png")
        ));
    }

    #[test]
    fn ordinary_indexed_files_reuse_one_type_icon_resource() {
        let mut first = search_result("first", ApplicationSource::IndexedFile);
        first.entry.target = r"C:\Images\first.PNG".into();
        let mut second = search_result("second", ApplicationSource::IndexedFile);
        second.entry.target = r"D:\Cache\second.png".into();
        let mut executable = search_result("tool", ApplicationSource::IndexedFile);
        executable.entry.target = r"C:\Tools\tool.exe".into();

        assert_eq!(
            icon_resource_key(&first, 32),
            icon_resource_key(&second, 32)
        );
        assert_ne!(
            icon_resource_key(&first, 32),
            icon_resource_key(&executable, 32)
        );
    }

    #[test]
    fn icon_resource_cache_is_bounded_and_keeps_recent_entries() {
        let mut cache = IconResourceCache::bounded(2);
        cache.insert("first".into(), PathBuf::from("first.png"));
        cache.insert("second".into(), PathBuf::from("second.png"));
        assert_eq!(cache.get("first"), Some(PathBuf::from("first.png")));
        cache.insert("third".into(), PathBuf::from("third.png"));

        assert!(cache.get("second").is_none());
        assert_eq!(cache.get("first"), Some(PathBuf::from("first.png")));
        assert_eq!(cache.get("third"), Some(PathBuf::from("third.png")));
    }

    #[test]
    fn action_mode_restores_the_parent_query_and_selection() {
        let target = search_result("alpha", ApplicationSource::StartMenu);
        let mode = QuickLaunchActionMode::enter(target.clone(), "alp".into(), 3);

        assert_eq!(mode.target().entry.id, target.entry.id);
        assert_eq!(
            mode.exit(),
            QuickLaunchResultsContext {
                query: "alp".into(),
                selected: 3,
                selected_entry_id: "alpha".into(),
            }
        );
    }

    #[test]
    fn recent_directory_mode_filters_and_navigates_the_frozen_host() {
        let recent = vec![
            nxtools_finder::RecentDirectory {
                path: r"C:\Work\Reports".into(),
                visit_count: 3,
                last_visited_at_unix_ms: 30,
            },
            nxtools_finder::RecentDirectory {
                path: r"D:\Archive".into(),
                visit_count: 1,
                last_visited_at_unix_ms: 20,
            },
        ];
        let results = recent_directory_results(&recent, "repo");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entry.target, r"C:\Work\Reports");
        assert_eq!(
            recent_directory_navigation_action(
                &results[0],
                Some(&nxtools_protocol::DirectoryNavigationContext {
                    host: DirectoryNavigationHost::Desktop,
                    window_handle: 66,
                    current_directory: Some(r"C:\Users\Test\Desktop".into()),
                    host_context_id: None,
                }),
            ),
            Some(QuickLaunchAction::SwitchDirectory {
                host: DirectoryNavigationHost::Desktop,
                window_handle: 66,
                target_directory: r"C:\Work\Reports".into(),
                host_context_id: None,
            })
        );
        assert_eq!(recent_directory_navigation_action(&results[0], None), None);
    }

    #[test]
    fn elevated_directory_helper_is_requested_once_after_both_processes_are_ready() {
        let request = ElevatedDirectoryHelperRequest {
            pipe_name: "nxtools-next-elevated-nav-0123456789abcdef".into(),
            authentication_token:
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into(),
        };
        assert_eq!(
            elevated_directory_helper_command(Some(&request), false, None, true),
            Some(IndexerCommand::EnsureElevatedDirectoryHelper(
                request.clone()
            ))
        );
        assert_eq!(
            elevated_directory_helper_command(Some(&request), true, None, true),
            None
        );
        assert_eq!(
            elevated_directory_helper_command(Some(&request), false, None, false),
            None
        );
        assert_eq!(
            elevated_directory_helper_command(
                Some(&request),
                false,
                Some(&request.pipe_name),
                true,
            ),
            None
        );
        assert_eq!(
            elevated_directory_helper_command(None, false, None, true),
            None
        );
        assert!(elevated_helper_status_matches_request(
            Some(&request),
            &nxtools_protocol::ElevatedDirectoryHelperStatus {
                pipe_name: request.pipe_name.clone(),
                launched: true,
                error: None,
            },
        ));
        assert!(!elevated_helper_status_matches_request(
            Some(&request),
            &nxtools_protocol::ElevatedDirectoryHelperStatus {
                pipe_name: "nxtools-next-elevated-nav-stale".into(),
                launched: true,
                error: None,
            },
        ));
    }

    #[test]
    fn deferred_directory_context_updates_only_the_matching_summon() {
        let resolution = nxtools_protocol::QuickLaunchDirectoryContextResolution {
            target_window_handle: 88,
            requested_at_unix_ms: 1_700_000_000_000,
            directory_navigation_context: nxtools_protocol::DirectoryNavigationContext {
                host: DirectoryNavigationHost::ThirdPartyFileManager,
                window_handle: 88,
                current_directory: Some(r"D:\Elevated".into()),
                host_context_id: Some("elevated:request-88".into()),
            },
        };
        assert!(directory_context_resolution_matches(
            Some((88, 1_700_000_000_000)),
            &resolution,
            1_700_000_000_500,
        ));
        assert!(!directory_context_resolution_matches(
            Some((77, 1_700_000_000_000)),
            &resolution,
            1_700_000_000_500,
        ));
        assert!(!directory_context_resolution_matches(
            None,
            &resolution,
            1_700_000_000_500,
        ));
        assert!(!directory_context_resolution_matches(
            Some((88, 1_700_000_000_000)),
            &resolution,
            1_700_000_001_001,
        ));
    }

    #[gpui::test]
    fn captured_directory_reaches_sqlite_after_the_popup_session_changes(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let temp = tempfile::tempdir().expect("temporary recent-directory catalog");
        let current_directory = temp.path().join("Current");
        std::fs::create_dir_all(&current_directory).expect("captured current directory");
        let history_path = temp.path().join("history.db");
        let catalog =
            FinderCatalog::open(&history_path, &[]).expect("open recent-directory catalog");
        let application = cx.update(|cx| {
            let notifications = cx.new(|_| NotificationCenter::default());
            let (platform, _endpoint) = nxtools_ipc::platform_bridge(8);
            let (indexer, _indexer_endpoint) = crate::runtime::indexer_bridge(8);
            cx.new(|_| {
                QuickLaunchApplication::new(
                    catalog,
                    Rc::new(platform),
                    Rc::new(indexer),
                    AppConfig::default(),
                    temp.path().join("config.json"),
                    temp.path().join("logs"),
                    notifications,
                )
            })
        });
        let context = nxtools_protocol::DirectoryNavigationContext {
            host: DirectoryNavigationHost::Desktop,
            window_handle: 66,
            current_directory: Some(current_directory.to_string_lossy().into_owned()),
            host_context_id: None,
        };

        application.update(cx, |application, cx| {
            application.schedule_captured_directory_record(Some(&context), cx);
            application.popup_session_id = uuid::Uuid::new_v4();
        });
        cx.run_until_parked();

        application.read_with(cx, |application, _| {
            let recent = application
                .catalog
                .borrow()
                .recent_directories(RECENT_DIRECTORY_LIMIT);
            assert_eq!(recent.len(), 1);
            assert_eq!(Path::new(&recent[0].path), current_directory.as_path());
        });
        let reopened =
            FinderCatalog::open(&history_path, &[]).expect("reopen persisted directory history");
        assert_eq!(
            reopened
                .recent_directories(RECENT_DIRECTORY_LIMIT)
                .first()
                .map(|entry| Path::new(&entry.path)),
            Some(current_directory.as_path())
        );
    }

    #[gpui::test]
    fn clearing_history_defers_feedback_and_persists_the_empty_state(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let temp = tempfile::tempdir().expect("temporary quick-launch history");
        let recent_directory = temp.path().join("Recent");
        std::fs::create_dir_all(&recent_directory).expect("recent directory");
        let history_path = temp.path().join("history.db");
        let mut catalog = FinderCatalog::open(&history_path, &[]).expect("open history");
        catalog
            .record_activation("editor", "edit", 1_700_000_000_000)
            .expect("record activation");
        catalog
            .record_directory_visit(&recent_directory, 1_700_000_001_000)
            .expect("record directory");
        let (application, notifications) = cx.update(|cx| {
            let notifications = cx.new(|_| NotificationCenter::default());
            let application_notifications = notifications.clone();
            let (platform, _endpoint) = nxtools_ipc::platform_bridge(8);
            let (indexer, _indexer_endpoint) = crate::runtime::indexer_bridge(8);
            let application = cx.new(|_| {
                QuickLaunchApplication::new(
                    catalog,
                    Rc::new(platform),
                    Rc::new(indexer),
                    AppConfig::default(),
                    temp.path().join("config.json"),
                    temp.path().join("logs"),
                    application_notifications,
                )
            });
            (application, notifications)
        });

        cx.update(|cx| {
            application.update(cx, |application, cx| {
                assert_eq!(application.history_summary().total_activations, 1);
                application.clear_history(cx);
                assert_eq!(
                    application.history_summary(),
                    QuickLaunchHistorySummary::default()
                );
            });
            assert!(
                notifications.read(cx).items().is_empty(),
                "cross-entity feedback must wait for the next application turn"
            );
        });

        cx.run_until_parked();
        notifications.read_with(cx, |notifications, _| {
            assert_eq!(notifications.items().len(), 1);
            assert_eq!(notifications.items()[0].title, "速启历史已清空");
        });
        let reopened = FinderCatalog::open(&history_path, &[]).expect("reopen cleared history");
        assert_eq!(
            reopened.history_summary(),
            QuickLaunchHistorySummary::default()
        );
    }

    #[test]
    fn action_mode_restores_the_same_result_after_a_background_reorder() {
        let target = search_result("alpha", ApplicationSource::StartMenu);
        let context = QuickLaunchActionMode::enter(target, "a".into(), 1).exit();
        let refreshed = vec![
            search_result("beta", ApplicationSource::StartMenu),
            search_result("alpha", ApplicationSource::StartMenu),
        ];

        assert_eq!(selection_for_results_context(&refreshed, &context), 1);
    }

    #[test]
    fn action_mode_filters_only_actions_available_for_the_target() {
        let mut shortcut = QuickLaunchActionMode::enter(
            search_result("alpha", ApplicationSource::StartMenu),
            String::new(),
            0,
        );
        finish_shell_loading(&mut shortcut);
        assert_eq!(
            shortcut
                .visible_actions()
                .iter()
                .map(|item| item.label.as_str())
                .collect::<Vec<_>>(),
            vec!["打开", "打开文件所在位置", "复制路径", "以管理员身份运行"]
        );

        shortcut.set_filter("管理");
        assert_eq!(
            shortcut
                .visible_actions()
                .iter()
                .filter_map(|item| item.action.clone())
                .collect::<Vec<_>>(),
            vec![QuickLaunchAction::RunElevated]
        );

        let mut system = QuickLaunchActionMode::enter(
            search_result("settings", ApplicationSource::System),
            String::new(),
            0,
        );
        finish_shell_loading(&mut system);
        assert!(
            !system
                .visible_actions()
                .iter()
                .any(|item| item.action == Some(QuickLaunchAction::Reveal))
        );

        let mut website = search_result("browser", ApplicationSource::StartMenu);
        website.entry.target = "browser.url".into();
        let mut website = QuickLaunchActionMode::enter(website, String::new(), 0);
        finish_shell_loading(&mut website);
        assert!(
            !website
                .visible_actions()
                .iter()
                .any(|item| item.action == Some(QuickLaunchAction::RunElevated))
        );
    }

    #[test]
    fn action_mode_exposes_copy_path_for_every_result_target() {
        for source in [
            ApplicationSource::StartMenu,
            ApplicationSource::System,
            ApplicationSource::IndexedFile,
            ApplicationSource::IndexedDirectory,
        ] {
            let mut mode =
                QuickLaunchActionMode::enter(search_result("alpha", source), String::new(), 0);
            finish_shell_loading(&mut mode);

            assert!(
                mode.visible_actions()
                    .iter()
                    .any(|item| item.label == "复制路径"),
                "copy-path action missing for {source:?}"
            );
        }
    }

    #[test]
    fn navigation_context_switches_files_to_their_parent_directory() {
        let context = nxtools_protocol::DirectoryNavigationContext {
            host: DirectoryNavigationHost::Explorer,
            window_handle: 88,
            current_directory: Some(r"C:\Before".into()),
            host_context_id: None,
        };
        let mut folder_result = search_result("Reports", ApplicationSource::IndexedDirectory);
        folder_result.entry.target = r"C:\Work\Reports".into();
        let mut folder = QuickLaunchActionMode::enter_in_context(
            folder_result.clone(),
            String::new(),
            0,
            Some(&context),
        );
        finish_shell_loading_with_navigation(&mut folder, r"C:\Work\Reports");
        assert!(folder.visible_actions().iter().any(|item| {
            item.action
                == Some(QuickLaunchAction::SwitchDirectory {
                    host: DirectoryNavigationHost::Explorer,
                    window_handle: 88,
                    target_directory: r"C:\Work\Reports".into(),
                    host_context_id: None,
                })
        }));

        let mut file_result = search_result("report.txt", ApplicationSource::IndexedFile);
        file_result.entry.target = r"C:\Work\Reports\report.txt".into();
        let mut file = QuickLaunchActionMode::enter_in_context(
            file_result.clone(),
            String::new(),
            0,
            Some(&context),
        );
        finish_shell_loading_with_navigation(&mut file, r"C:\Work\Reports");
        let file_action = file
            .visible_actions()
            .into_iter()
            .find_map(|item| match item.action {
                Some(action @ QuickLaunchAction::SwitchDirectory { .. }) => Some(action),
                _ => None,
            })
            .expect("file result must expose parent-directory navigation");
        let PlatformCommand::NavigateDirectory(file_command) =
            platform_command_for_action(&file_result, &file_action, 999)
                .expect("file parent-directory command")
        else {
            panic!("file navigation must use the directory adapter");
        };
        assert_eq!(file_command.target_directory, r"C:\Work\Reports");

        let mut ordinary = QuickLaunchActionMode::enter(folder_result, String::new(), 0);
        finish_shell_loading(&mut ordinary);
        assert!(!ordinary.visible_actions().iter().any(|item| {
            matches!(item.action, Some(QuickLaunchAction::SwitchDirectory { .. }))
        }));
    }

    #[test]
    fn file_dialog_navigation_is_available_for_local_application_results() {
        let context = nxtools_protocol::DirectoryNavigationContext {
            host: DirectoryNavigationHost::FileDialog,
            window_handle: 77,
            current_directory: Some(r"C:\Before".into()),
            host_context_id: None,
        };
        let mut application = search_result("AMD Software", ApplicationSource::StartMenu);
        application.entry.target = r"C:\Program Files\AMD\CNext\CNext\RadeonSoftware.exe".into();
        let mut mode = QuickLaunchActionMode::enter_in_context(
            application.clone(),
            String::new(),
            0,
            Some(&context),
        );
        finish_shell_loading_with_navigation(&mut mode, r"C:\Program Files\AMD\CNext\CNext");

        let action = mode
            .visible_actions()
            .into_iter()
            .find_map(|item| match item.action {
                Some(action @ QuickLaunchAction::SwitchDirectory { .. }) => Some(action),
                _ => None,
            })
            .expect("a local application result must expose file-dialog directory navigation");
        let PlatformCommand::NavigateDirectory(command) =
            platform_command_for_action(&application, &action, 999)
                .expect("local application directory command")
        else {
            panic!("application navigation must use the directory adapter");
        };
        assert_eq!(
            command.target_directory,
            r"C:\Program Files\AMD\CNext\CNext"
        );
    }

    #[test]
    fn shortcut_navigation_uses_the_resolved_target_directory_without_changing_launch_target() {
        let context = nxtools_protocol::DirectoryNavigationContext {
            host: DirectoryNavigationHost::FileDialog,
            window_handle: 77,
            current_directory: Some(r"C:\Before".into()),
            host_context_id: None,
        };
        let mut shortcut = search_result("AMD Software", ApplicationSource::StartMenu);
        shortcut.entry.target =
            r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs\AMD Software.lnk".into();
        let launch_target = shortcut.entry.target.clone();
        let mut mode = QuickLaunchActionMode::enter_in_context(
            shortcut.clone(),
            String::new(),
            0,
            Some(&context),
        );
        let level_id = mode.current_level_id();
        mode.apply_shell_menu(
            level_id,
            Some(ShellActionMenu {
                session_id: "shortcut-session".into(),
                navigation_directory: Some(r"C:\Program Files\AMD\CNext\CNext".into()),
                items: Vec::new(),
            }),
        );

        let action = mode
            .visible_actions()
            .into_iter()
            .find_map(|item| match item.action {
                Some(action @ QuickLaunchAction::SwitchDirectory { .. }) => Some(action),
                _ => None,
            })
            .expect("resolved shortcut must expose target-directory navigation");
        let PlatformCommand::NavigateDirectory(command) =
            platform_command_for_action(&shortcut, &action, 999)
                .expect("resolved shortcut directory command")
        else {
            panic!("shortcut navigation must use the directory adapter");
        };
        assert_eq!(
            command.target_directory,
            r"C:\Program Files\AMD\CNext\CNext"
        );

        let PlatformCommand::OpenShellTarget(open) =
            platform_command_for_action(&shortcut, &QuickLaunchAction::Open, 999)
                .expect("shortcut launch command")
        else {
            panic!("normal shortcut launch must use the Shell target adapter");
        };
        assert_eq!(open.target, launch_target);
    }

    #[test]
    fn file_dialog_context_uses_the_captured_dialog_for_folder_results() {
        let context = nxtools_protocol::DirectoryNavigationContext {
            host: DirectoryNavigationHost::FileDialog,
            window_handle: 77,
            current_directory: Some(r"C:\Before".into()),
            host_context_id: None,
        };
        let mut result = search_result("Reports", ApplicationSource::IndexedDirectory);
        result.entry.target = r"C:\Work\Reports".into();
        let mut mode = QuickLaunchActionMode::enter_in_context(
            result.clone(),
            String::new(),
            0,
            Some(&context),
        );
        finish_shell_loading_with_navigation(&mut mode, r"C:\Work\Reports");
        let action = mode
            .visible_actions()
            .into_iter()
            .find_map(|item| item.action)
            .expect("file-dialog directory action");
        assert_eq!(
            action,
            QuickLaunchAction::SwitchDirectory {
                host: DirectoryNavigationHost::FileDialog,
                window_handle: 77,
                target_directory: r"C:\Work\Reports".into(),
                host_context_id: None,
            }
        );

        let command = platform_command_for_action(&result, &action, 999)
            .expect("file-dialog navigation command");
        let PlatformCommand::NavigateDirectory(command) = command else {
            panic!("switch-directory must use the directory adapter");
        };
        assert_eq!(command.host, DirectoryNavigationHost::FileDialog);
        assert_eq!(command.window_handle, 77);
        assert_eq!(command.target_directory, result.entry.target);
    }

    #[test]
    fn file_navigation_uses_absolute_parents_for_every_directory_host() {
        let cases = [
            (r"C:\Work\Reports\report.txt", r"C:\Work\Reports"),
            (r"C:\report.txt", r"C:\"),
            (r"\\server\share\report.txt", r"\\server\share"),
        ];
        for (host, window_handle) in [
            (DirectoryNavigationHost::Explorer, 88),
            (DirectoryNavigationHost::FileDialog, 77),
            (DirectoryNavigationHost::ThirdPartyFileManager, 66),
        ] {
            let context = nxtools_protocol::DirectoryNavigationContext {
                host,
                window_handle,
                current_directory: Some(r"C:\Before".into()),
                host_context_id: (host == DirectoryNavigationHost::ThirdPartyFileManager)
                    .then(|| "pane-context-66".into()),
            };
            for (target, expected_parent) in cases {
                let mut result = search_result("report.txt", ApplicationSource::IndexedFile);
                result.entry.target = target.into();
                let mut mode = QuickLaunchActionMode::enter_in_context(
                    result.clone(),
                    String::new(),
                    0,
                    Some(&context),
                );
                finish_shell_loading_with_navigation(&mut mode, expected_parent);
                let action = mode
                    .visible_actions()
                    .into_iter()
                    .find_map(|item| match item.action {
                        Some(action @ QuickLaunchAction::SwitchDirectory { .. }) => Some(action),
                        _ => None,
                    })
                    .expect("absolute file result must expose parent-directory navigation");
                let PlatformCommand::NavigateDirectory(command) =
                    platform_command_for_action(&result, &action, 999)
                        .expect("absolute file parent-directory command")
                else {
                    panic!("file navigation must use the directory adapter");
                };
                assert_eq!(command.host, host);
                assert_eq!(command.window_handle, window_handle);
                assert_eq!(command.host_context_id, context.host_context_id);
                assert_eq!(
                    Path::new(&command.target_directory),
                    Path::new(expected_parent)
                );
                assert_ne!(command.target_directory, result.entry.target);
            }
        }
    }

    #[test]
    fn relative_files_and_missing_context_do_not_expose_directory_navigation() {
        let context = nxtools_protocol::DirectoryNavigationContext {
            host: DirectoryNavigationHost::FileDialog,
            window_handle: 77,
            current_directory: Some(r"C:\Before".into()),
            host_context_id: None,
        };
        for target in ["report.txt", r"Reports\report.txt", r"C:report.txt"] {
            let mut result = search_result("report.txt", ApplicationSource::IndexedFile);
            result.entry.target = target.into();
            let mut mode =
                QuickLaunchActionMode::enter_in_context(result, String::new(), 0, Some(&context));
            finish_shell_loading(&mut mode);
            assert!(!mode.visible_actions().iter().any(|item| {
                matches!(item.action, Some(QuickLaunchAction::SwitchDirectory { .. }))
            }));
        }

        for rejected_directory in [None, Some("relative"), Some(r"C:relative")] {
            let mut unresolved = search_result("Editor", ApplicationSource::StartMenu);
            unresolved.entry.target = r"C:\Apps\Editor.exe --safe-mode".into();
            let mut mode = QuickLaunchActionMode::enter_in_context(
                unresolved,
                String::new(),
                0,
                Some(&context),
            );
            let level_id = mode.current_level_id();
            mode.apply_shell_menu(
                level_id,
                Some(ShellActionMenu {
                    session_id: "rejected-navigation".into(),
                    navigation_directory: rejected_directory.map(str::to_owned),
                    items: Vec::new(),
                }),
            );
            assert!(!mode.visible_actions().iter().any(|item| {
                matches!(item.action, Some(QuickLaunchAction::SwitchDirectory { .. }))
            }));
        }

        let mut absolute_file = search_result("report.txt", ApplicationSource::IndexedFile);
        absolute_file.entry.target = r"C:\Work\Reports\report.txt".into();
        let mut unresolved = QuickLaunchActionMode::enter_in_context(
            absolute_file.clone(),
            String::new(),
            0,
            Some(&context),
        );
        finish_shell_loading(&mut unresolved);
        assert!(!unresolved.visible_actions().iter().any(|item| {
            matches!(item.action, Some(QuickLaunchAction::SwitchDirectory { .. }))
        }));

        let mut ordinary = QuickLaunchActionMode::enter(absolute_file, String::new(), 0);
        finish_shell_loading_with_navigation(&mut ordinary, r"C:\Work\Reports");
        assert!(!ordinary.visible_actions().iter().any(|item| {
            matches!(item.action, Some(QuickLaunchAction::SwitchDirectory { .. }))
        }));
    }

    #[test]
    fn switch_directory_command_uses_the_window_captured_at_summon() {
        let mut result = search_result("Reports", ApplicationSource::IndexedDirectory);
        result.entry.target = r"C:\Work\Reports".into();
        let command = platform_command_for_action(
            &result,
            &QuickLaunchAction::SwitchDirectory {
                host: DirectoryNavigationHost::Explorer,
                window_handle: 88,
                target_directory: result.entry.target.clone(),
                host_context_id: None,
            },
            999,
        )
        .expect("Explorer navigation command");

        let PlatformCommand::NavigateDirectory(command) = command else {
            panic!("switch-directory must use the directory adapter");
        };
        assert_eq!(command.host, DirectoryNavigationHost::Explorer);
        assert_eq!(command.window_handle, 88);
        assert_eq!(command.target_directory, result.entry.target);
    }

    #[test]
    fn copy_path_action_builds_a_copy_only_clipboard_command() {
        let result = search_result("alpha", ApplicationSource::IndexedFile);

        let command = platform_command_for_action(&result, &QuickLaunchAction::CopyPath, 0)
            .expect("copy-path command");

        let PlatformCommand::PasteText(command) = command else {
            panic!("copy-path must use the Windows clipboard adapter");
        };
        assert_eq!(command.text, result.entry.target);
        assert!(!command.paste_after_write);
        assert_eq!(command.target_window_handle, 0);
        assert_eq!(command.target_focus_window_handle, 0);
    }

    #[test]
    fn calculator_enter_uses_copy_result_without_searching_the_file_index() {
        let mut result = search_result("calculator", ApplicationSource::Calculator);
        result.entry.name = "6 * 7 = 42".into();
        result.entry.target.clear();
        result.action = SearchResultAction::CopyText { text: "42".into() };

        assert_eq!(
            quick_launch_primary_action(&result, false, false),
            QuickLaunchAction::CopyResult
        );
        assert!(!query_requests_file_index("=6 * 7"));
        assert!(query_requests_file_index("6 * 7"));

        let command = platform_command_for_action(&result, &QuickLaunchAction::CopyResult, 0)
            .expect("copy-result command");
        let PlatformCommand::PasteText(command) = command else {
            panic!("calculator must use the Windows clipboard adapter");
        };
        assert_eq!(command.text, "42");
        assert!(!command.paste_after_write);
        assert_eq!(
            action_success_policy(&QuickLaunchAction::CopyResult),
            ActionSuccessPolicy::HideWithoutRecording
        );
    }

    #[test]
    fn custom_command_uses_typed_launch_and_local_actions_only() {
        let result = SearchResult {
            entry: nxtools_finder::ApplicationEntry {
                id: "custom-command:tool".into(),
                name: "Run tool".into(),
                target: r"C:\Tools\tool.exe".into(),
                source: ApplicationSource::CustomCommand,
            },
            action: SearchResultAction::RunCommand {
                command: nxtools_finder::PreparedCustomCommand {
                    executable: r"C:\Tools\tool.exe".into(),
                    arguments: vec!["--name".into(), "two words".into()],
                    working_directory: Some(r"D:\Work".into()),
                    run_as_admin: true,
                    run_silently: true,
                },
            },
            subtitle: Some(r#"C:\Tools\tool.exe --name "two words""#.into()),
            score: 10_000,
            activation_count: 0,
            last_activated_at_unix_ms: None,
            query_activation_count: 0,
            last_query_activation_at_unix_ms: None,
        };

        assert_eq!(
            quick_launch_primary_action(&result, false, false),
            QuickLaunchAction::Open
        );
        assert_eq!(
            quick_launch_result_capabilities(&result),
            QuickLaunchResultCapabilities {
                preview: false,
                shell_actions: false,
                shell_icon: true,
            }
        );
        let mode = QuickLaunchActionMode::enter(result.clone(), "tool arg".into(), 0);
        assert!(!mode.current_level().shell_pending);
        assert_eq!(
            mode.visible_actions()
                .iter()
                .map(|item| (item.label.as_str(), item.action.clone()))
                .collect::<Vec<_>>(),
            vec![
                ("运行命令", Some(QuickLaunchAction::Open)),
                ("复制命令", Some(QuickLaunchAction::CopyCommand)),
            ]
        );

        let launch = platform_command_for_action(&result, &QuickLaunchAction::Open, 0)
            .expect("typed launch command");
        let PlatformCommand::LaunchProcess(launch) = launch else {
            panic!("custom command must use the typed process adapter");
        };
        assert_eq!(launch.executable, r"C:\Tools\tool.exe");
        assert_eq!(launch.arguments, ["--name", "two words"]);
        assert_eq!(launch.working_directory.as_deref(), Some(r"D:\Work"));
        assert!(launch.run_as_admin);
        assert!(launch.run_silently);

        let copy = platform_command_for_action(&result, &QuickLaunchAction::CopyCommand, 0)
            .expect("copy command");
        let PlatformCommand::PasteText(copy) = copy else {
            panic!("copy command must use the clipboard adapter");
        };
        assert_eq!(copy.text, result.subtitle.expect("display line"));
        assert!(!copy.paste_after_write);
    }

    #[test]
    fn web_url_uses_browser_action_without_searching_the_file_index() {
        let result = SearchResult {
            entry: nxtools_finder::ApplicationEntry {
                id: "web-url:https://example.com/docs".into(),
                name: "https://example.com/docs".into(),
                target: "https://example.com/docs".into(),
                source: ApplicationSource::WebUrl,
            },
            action: SearchResultAction::OpenTarget,
            subtitle: Some("用浏览器打开".into()),
            score: 1_000,
            activation_count: 0,
            last_activated_at_unix_ms: None,
            query_activation_count: 0,
            last_query_activation_at_unix_ms: None,
        };

        assert!(!query_requests_file_index("https://example.com/docs"));
        assert!(!query_requests_file_index("example.com/docs"));
        assert_eq!(source_icon(result.entry.source), MaterialIcon::Language);
        assert_eq!(
            quick_launch_result_capabilities(&result),
            QuickLaunchResultCapabilities {
                preview: false,
                shell_actions: false,
                shell_icon: false,
            }
        );
        assert_eq!(
            quick_launch_primary_action(&result, true, true),
            QuickLaunchAction::Open
        );
        assert_eq!(
            platform_command_for_action(&result, &QuickLaunchAction::Open, 42),
            Some(PlatformCommand::OpenShellTarget(OpenShellTargetCommand {
                target: "https://example.com/docs".into(),
                action: ShellOpenAction::Open,
            }))
        );
    }

    #[test]
    fn environment_variable_uses_copy_action_without_file_integration() {
        let result = SearchResult {
            entry: nxtools_finder::ApplicationEntry {
                id: "environment-variable:prompt".into(),
                name: "%PROMPT%".into(),
                target: String::new(),
                source: ApplicationSource::EnvironmentVariable,
            },
            action: SearchResultAction::CopyText {
                text: "$P$G".into(),
            },
            subtitle: Some("$P$G".into()),
            score: 1_000,
            activation_count: 0,
            last_activated_at_unix_ms: None,
            query_activation_count: 0,
            last_query_activation_at_unix_ms: None,
        };

        assert!(!query_requests_file_index("%PATH%"));
        assert!(!query_requests_file_index(
            "%NXTOOLS_VARIABLE_THAT_DOES_NOT_EXIST_7D3E2A1C%"
        ));
        assert!(query_requests_file_index("%PATH"));
        assert_eq!(source_icon(result.entry.source), MaterialIcon::DataObject);
        assert_eq!(
            quick_launch_result_capabilities(&result),
            QuickLaunchResultCapabilities {
                preview: false,
                shell_actions: false,
                shell_icon: false,
            }
        );
        assert_eq!(
            quick_launch_primary_action(&result, false, false),
            QuickLaunchAction::CopyResult
        );
        let command = platform_command_for_action(&result, &QuickLaunchAction::CopyResult, 0)
            .expect("environment variable copy command");
        let PlatformCommand::PasteText(command) = command else {
            panic!("environment variable must use the clipboard adapter");
        };
        assert_eq!(command.text, "$P$G");

        let mode = QuickLaunchActionMode::enter(result, "%PATH%".into(), 0);
        assert!(!mode.current_level().shell_pending);
        assert_eq!(
            mode.visible_actions()
                .iter()
                .map(|item| (item.label.as_str(), item.action.clone()))
                .collect::<Vec<_>>(),
            vec![("复制变量值", Some(QuickLaunchAction::CopyResult))]
        );
    }

    #[test]
    fn environment_directory_uses_open_and_native_file_actions() {
        let result = SearchResult {
            entry: nxtools_finder::ApplicationEntry {
                id: "environment-variable:temp:0".into(),
                name: r"C:\Temp".into(),
                target: r"C:\Temp".into(),
                source: ApplicationSource::EnvironmentVariable,
            },
            action: SearchResultAction::OpenTarget,
            subtitle: Some("%TEMP% · 打开文件夹".into()),
            score: 1_000,
            activation_count: 0,
            last_activated_at_unix_ms: None,
            query_activation_count: 0,
            last_query_activation_at_unix_ms: None,
        };

        assert_eq!(
            quick_launch_primary_action(&result, false, false),
            QuickLaunchAction::Open
        );
        assert_eq!(
            quick_launch_result_capabilities(&result),
            QuickLaunchResultCapabilities {
                preview: true,
                shell_actions: true,
                shell_icon: true,
            }
        );
        assert_eq!(
            platform_command_for_action(&result, &QuickLaunchAction::Open, 42),
            Some(PlatformCommand::OpenShellTarget(OpenShellTargetCommand {
                target: r"C:\Temp".into(),
                action: ShellOpenAction::Open,
            }))
        );

        let mut mode = QuickLaunchActionMode::enter(result, "%TEMP%".into(), 0);
        assert!(mode.current_level().shell_pending);
        finish_shell_loading(&mut mode);
        let actions = mode.visible_actions();
        assert!(actions.iter().any(|item| item.label == "打开"));
        assert!(actions.iter().any(|item| item.label == "复制路径"));
        assert!(!actions.iter().any(|item| item.label == "复制变量值"));
    }

    #[test]
    fn calculator_action_mode_has_one_local_action_and_no_shell_loading() {
        let result = search_result("calculator", ApplicationSource::Calculator);
        let mode = QuickLaunchActionMode::enter(result, "6 * 7".into(), 0);

        assert!(!mode.current_level().shell_pending);
        let actions = mode.visible_actions();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].label, "复制结果");
        assert_eq!(actions[0].action, Some(QuickLaunchAction::CopyResult));
        assert_eq!(
            source_icon(ApplicationSource::Calculator),
            MaterialIcon::Calculate
        );
    }

    #[test]
    fn web_url_action_mode_has_only_local_browser_actions() {
        let mut result = search_result("https://example.com", ApplicationSource::WebUrl);
        result.entry.target = "https://example.com".into();
        let mode = QuickLaunchActionMode::enter(result.clone(), "example.com".into(), 0);

        assert!(!quick_launch_result_capabilities(&result).shell_actions);
        assert!(!mode.current_level().shell_pending);
        assert_eq!(
            mode.visible_actions()
                .iter()
                .map(|item| (item.label.as_str(), item.action.clone()))
                .collect::<Vec<_>>(),
            vec![
                ("打开网址", Some(QuickLaunchAction::Open)),
                ("复制网址", Some(QuickLaunchAction::CopyPath)),
            ]
        );
    }

    #[test]
    fn copy_path_success_keeps_the_launcher_open_without_recording_a_launch() {
        assert_eq!(
            action_success_policy(&QuickLaunchAction::CopyPath),
            ActionSuccessPolicy::KeepOpen
        );
        for action in [
            QuickLaunchAction::Open,
            QuickLaunchAction::Reveal,
            QuickLaunchAction::RunElevated,
        ] {
            assert_eq!(
                action_success_policy(&action),
                ActionSuccessPolicy::RecordAndHide
            );
        }
    }

    #[test]
    fn a_late_action_result_cannot_target_a_reopened_popup_session() {
        let original_session = uuid::Uuid::new_v4();
        let reopened_session = uuid::Uuid::new_v4();
        let pending = PendingQuickLaunchAction {
            correlation_id: uuid::Uuid::new_v4(),
            popup_session_id: original_session,
            entry_id: "reports".into(),
            display_name: "Reports".into(),
            target: r"C:\Reports".into(),
            query: "reports".into(),
            action: QuickLaunchAction::SwitchDirectory {
                host: DirectoryNavigationHost::Explorer,
                window_handle: 88,
                target_directory: r"C:\Reports".into(),
                host_context_id: None,
            },
            directory_visit: Some(r"C:\Reports".into()),
            trigger: QuickLaunchActionTrigger::RecentDirectory,
            source: ApplicationSource::IndexedDirectory,
            started_at: Instant::now(),
        };

        assert!(pending_action_targets_session(&pending, original_session));
        assert!(!pending_action_targets_session(&pending, reopened_session));
    }

    #[test]
    fn pending_directory_navigation_owns_its_foreground_transition() {
        let session = uuid::Uuid::new_v4();
        let pending = PendingQuickLaunchAction {
            correlation_id: uuid::Uuid::new_v4(),
            popup_session_id: session,
            entry_id: "reports".into(),
            display_name: "Reports".into(),
            target: r"C:\Reports".into(),
            query: "reports".into(),
            action: QuickLaunchAction::SwitchDirectory {
                host: DirectoryNavigationHost::FileDialog,
                window_handle: 77,
                target_directory: r"C:\Reports".into(),
                host_context_id: None,
            },
            directory_visit: Some(r"C:\Reports".into()),
            trigger: QuickLaunchActionTrigger::RecentDirectory,
            source: ApplicationSource::IndexedDirectory,
            started_at: Instant::now(),
        };

        assert!(pending_action_owns_foreground(&pending, session, 77));
        assert!(!pending_action_owns_foreground(&pending, session, 78));
        assert!(!pending_action_owns_foreground(
            &pending,
            uuid::Uuid::new_v4(),
            77
        ));
    }

    #[test]
    fn failed_directory_navigation_guards_late_foreground_events_until_expiry() {
        let now = Instant::now();
        let session = uuid::Uuid::new_v4();
        let mut guard = Some(NavigationForegroundGuard {
            popup_session_id: session,
            window_handle: 77,
            expires_at: now + FAILED_NAVIGATION_FOREGROUND_GUARD,
        });

        assert!(navigation_guard_owns_foreground(
            &mut guard, session, 77, now
        ));
        assert!(navigation_guard_owns_foreground(
            &mut guard,
            session,
            77,
            now + FAILED_NAVIGATION_FOREGROUND_GUARD / 2
        ));
        assert!(!navigation_guard_owns_foreground(
            &mut guard,
            session,
            77,
            now + FAILED_NAVIGATION_FOREGROUND_GUARD
        ));
        assert!(guard.is_none());
    }

    #[test]
    fn action_mode_navigation_wraps_and_filtering_resets_selection() {
        let mut mode = QuickLaunchActionMode::enter(
            search_result("alpha", ApplicationSource::StartMenu),
            String::new(),
            0,
        );
        finish_shell_loading(&mut mode);

        assert_eq!(mode.selected_action(), Some(QuickLaunchAction::Open));
        mode.move_selection(-1);
        assert_eq!(mode.selected_action(), Some(QuickLaunchAction::RunElevated));
        mode.move_selection(1);
        assert_eq!(mode.selected_action(), Some(QuickLaunchAction::Open));

        mode.move_selection(1);
        mode.set_filter("管理");
        assert_eq!(mode.selected_action(), Some(QuickLaunchAction::RunElevated));
    }

    #[test]
    fn pending_root_action_mode_does_not_publish_a_partial_builtin_list() {
        let mode = QuickLaunchActionMode::enter(
            search_result("alpha", ApplicationSource::IndexedFile),
            "alp".into(),
            0,
        );

        assert!(mode.shell_pending());
        assert!(mode.visible_actions().is_empty());
    }

    #[test]
    fn action_mode_appends_shell_groups_and_enters_nested_commands() {
        let mut mode = QuickLaunchActionMode::enter(
            search_result("alpha", ApplicationSource::IndexedFile),
            "alp".into(),
            0,
        );
        let icon = tempfile::NamedTempFile::new().expect("staged Shell icon");
        let root_level_id = mode.current_level_id();
        mode.apply_shell_menu(
            root_level_id,
            Some(ShellActionMenu {
                session_id: "session-1".into(),
                navigation_directory: None,
                items: vec![
                    ShellActionItem {
                        command_id: None,
                        title: String::new(),
                        enabled: false,
                        separator: true,
                        submenu_id: None,
                        icon_path: None,
                    },
                    ShellActionItem {
                        command_id: None,
                        title: "打开方式".into(),
                        enabled: true,
                        separator: false,
                        submenu_id: Some(7),
                        icon_path: Some(icon.path().to_string_lossy().into_owned()),
                    },
                    ShellActionItem {
                        command_id: Some(43),
                        title: "不可用操作".into(),
                        enabled: false,
                        separator: false,
                        submenu_id: None,
                        icon_path: None,
                    },
                ],
            }),
        );

        let root = mode.visible_actions();
        assert!(root.len() > QUICK_LAUNCH_ACTIONS.len());
        let submenu_index = root
            .iter()
            .position(|item| item.label == "打开方式")
            .expect("Shell submenu");
        assert_eq!(
            root[submenu_index].section_label.as_deref(),
            Some("Windows 操作")
        );
        assert!(root[submenu_index].divider_before);
        assert_eq!(root[submenu_index].icon_path.as_deref(), Some(icon.path()));
        assert!(
            !root
                .iter()
                .find(|item| item.label == "不可用操作")
                .expect("disabled Shell item")
                .enabled
        );

        mode.set_selected(submenu_index);
        let submenu = mode.push_selected_submenu().expect("lazy Shell submenu");
        assert_eq!(submenu.session_id, "session-1");
        assert_eq!(submenu.submenu_id, 7);
        assert_eq!(mode.depth(), 2);
        assert!(mode.shell_pending());
        assert_eq!(mode.selected_action(), None);
        mode.apply_shell_menu(
            submenu.level_id,
            Some(ShellActionMenu {
                session_id: "session-1".into(),
                navigation_directory: None,
                items: vec![ShellActionItem {
                    command_id: Some(42),
                    title: "测试程序".into(),
                    enabled: true,
                    separator: false,
                    submenu_id: None,
                    icon_path: None,
                }],
            }),
        );
        assert_eq!(
            mode.selected_action(),
            Some(QuickLaunchAction::Shell {
                session_id: "session-1".into(),
                command_id: 42,
            })
        );
        assert_eq!(mode.pop_level(), Some(String::new()));
        assert_eq!(mode.depth(), 1);
    }

    #[test]
    fn shell_action_invocation_preserves_session_command_and_owner_window() {
        let result = search_result("alpha", ApplicationSource::IndexedFile);
        let action = QuickLaunchAction::Shell {
            session_id: "session-2".into(),
            command_id: 17,
        };
        let command =
            platform_command_for_action(&result, &action, 1234).expect("Shell action command");
        assert_eq!(
            command,
            PlatformCommand::InvokeShellAction(InvokeShellActionCommand {
                session_id: "session-2".into(),
                command_id: 17,
                owner_window_handle: 1234,
            })
        );
    }

    #[test]
    fn a_late_shell_menu_cannot_replace_a_reentered_action_mode() {
        let result = search_result("alpha", ApplicationSource::IndexedFile);
        let entry_id = result.entry.id.clone();
        let previous = QuickLaunchActionMode::enter(result.clone(), String::new(), 0);
        let current = QuickLaunchActionMode::enter(result, String::new(), 0);
        let current_level_id = current.current_level_id();

        assert_ne!(previous.id(), current.id());
        assert!(!current.matches_shell_query(previous.id(), current_level_id, &entry_id));
        assert!(current.matches_shell_query(current.id(), current_level_id, &entry_id));
    }

    #[test]
    fn escape_always_returns_from_action_mode_before_clearing_the_parent_query() {
        assert_eq!(
            quick_launch_escape_action(true, false),
            QuickLaunchEscapeAction::ExitActions
        );
        assert_eq!(
            quick_launch_escape_action(false, false),
            QuickLaunchEscapeAction::ClearQuery
        );
        assert_eq!(
            quick_launch_escape_action(false, true),
            QuickLaunchEscapeAction::Close
        );
    }

    #[test]
    fn escape_hides_the_popup_then_restores_the_captured_host_for_the_next_summon() {
        assert_eq!(
            quick_launch_dismiss_commands(900, Some(88)),
            vec![
                PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
                    window_handle: 900,
                    visible: false,
                }),
                PlatformCommand::SetWindowTopmost(SetWindowTopmostCommand {
                    window_handle: 900,
                    enabled: false,
                }),
                PlatformCommand::ActivateWindow(ActivateWindowCommand { window_handle: 88 }),
            ]
        );
        assert_eq!(
            quick_launch_dismiss_commands(900, None),
            vec![
                PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
                    window_handle: 900,
                    visible: false,
                }),
                PlatformCommand::SetWindowTopmost(SetWindowTopmostCommand {
                    window_handle: 900,
                    enabled: false,
                }),
            ]
        );
    }

    #[test]
    fn a_late_file_batch_cannot_replace_the_newer_query_generation() {
        let mut session = IndexSearchSession::default();
        let first = session.begin("report", 20);
        let second = session.begin("report final", 20);
        let batch = |generation, elapsed_us| nxtools_protocol::IndexSearchBatch {
            generation,
            items: Vec::new(),
            complete: true,
            elapsed_us,
        };

        assert!(!session.apply(batch(first.generation, 999)));
        assert_eq!(session.last_completed_elapsed_us, None);
        assert!(session.apply(batch(second.generation, 17)));
        assert!(session.complete);
        assert_eq!(session.last_completed_elapsed_us, Some(17));
    }

    #[test]
    fn an_indexer_disconnect_clears_visible_status_and_invalidates_search() {
        let mut status = Some(IndexStatus {
            phase: nxtools_protocol::IndexPhase::VolumesReady,
            indexed_items: 0,
            drives: Vec::new(),
            last_error: None,
        });
        let mut session = IndexSearchSession::default();
        let request = session.begin("report", 20);

        assert!(apply_indexer_notification(
            &mut status,
            &mut session,
            IndexerNotification::Disconnected,
        ));
        assert!(status.is_none());
        assert_ne!(session.generation, request.generation);
    }

    #[gpui::test]
    fn gpui_action_mode_handles_keyboard_mouse_and_parent_restoration(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let temp = tempfile::tempdir().expect("temporary quick-launch test directory");
        let root = temp.path().join("applications");
        std::fs::create_dir_all(&root).expect("application test root");
        std::fs::write(root.join("alpha.lnk"), b"").expect("test shortcut");
        let catalog = FinderCatalog::open(
            temp.path().join("history.db"),
            &[nxtools_finder::ApplicationRoot {
                path: root,
                source: ApplicationSource::StartMenu,
            }],
        )
        .expect("test quick-launch catalog");
        let results = catalog.search("", RESULT_LIMIT).expect("initial results");
        let application = cx.update(|cx| {
            let notifications = cx.new(|_| NotificationCenter::default());
            let (platform, _endpoint) = nxtools_ipc::platform_bridge(8);
            let (indexer, _indexer_endpoint) = crate::runtime::indexer_bridge(8);
            cx.new(|_| {
                QuickLaunchApplication::new(
                    catalog,
                    Rc::new(platform),
                    Rc::new(indexer),
                    AppConfig::default(),
                    temp.path().join("config.json"),
                    temp.path().join("logs"),
                    notifications,
                )
            })
        });
        let target = nxtools_protocol::ForegroundContext {
            window_handle: 1,
            focus_window_handle: 1,
            process_id: 1,
            process_name: "editor".into(),
            window_title: "document".into(),
        };
        let panel_slot = Rc::new(RefCell::new(None));
        let panel_slot_for_window = panel_slot.clone();
        let application_for_window = application.clone();
        let (_root, cx) = cx.add_window_view(move |window, cx| {
            let panel = cx.new(|cx| {
                QuickLaunchPanel::new(
                    results,
                    target,
                    None,
                    Vec::new(),
                    application_for_window.downgrade(),
                    window,
                    cx,
                )
            });
            *panel_slot_for_window.borrow_mut() = Some(panel.clone());
            Root::new(panel, window, cx)
        });
        let panel = panel_slot
            .borrow_mut()
            .take()
            .expect("quick-launch panel entity");
        panel.update_in(cx, |panel, window, cx| panel.focus_search(window, cx));

        cx.simulate_input("alp");
        panel.update_in(cx, |panel, _window, cx| {
            panel.run_search("alp".into(), cx);
        });
        panel.update_in(cx, |_panel, window, _cx| {
            assert_eq!(window.bounds().size.height, px(PANEL_HEIGHT));
        });
        panel.read_with(cx, |panel, cx| {
            assert_eq!(panel.search_input.read(cx).value().as_ref(), "alp");
            assert_eq!(panel.results.len(), 1);
        });
        cx.simulate_keystrokes("right");
        panel.read_with(cx, |panel, cx| {
            assert!(panel.action_mode.is_some());
            assert!(panel.search_input.read(cx).value().is_empty());
        });

        cx.simulate_input("管理");
        cx.simulate_keystrokes("escape");
        panel.read_with(cx, |panel, cx| {
            assert!(panel.action_mode.is_none());
            assert_eq!(panel.search_input.read(cx).value().as_ref(), "alp");
            assert_eq!(panel.results[panel.selected].entry.name, "alpha");
        });

        cx.simulate_mouse_down(
            point(px(20.0), px(60.0)),
            gpui::MouseButton::Right,
            gpui::Modifiers::none(),
        );
        panel.read_with(cx, |panel, _| assert!(panel.action_mode.is_some()));
        cx.simulate_keystrokes("backspace");
        panel.read_with(cx, |panel, _| assert!(panel.action_mode.is_none()));

        let keyboard_selected_id = panel.read_with(cx, |panel, _| {
            panel.results[panel.selected].entry.id.clone()
        });
        cx.simulate_keystrokes("down");
        cx.executor()
            .advance_clock(QUICK_LAUNCH_PREVIEW_HOVER_DELAY);
        cx.run_until_parked();
        application.read_with(cx, |application, _| {
            assert!(application.preview.visible);
            assert_eq!(
                application.preview.displayed_entry_id.as_deref(),
                Some(keyboard_selected_id.as_str())
            );
        });
        application.update(cx, |application, cx| {
            application.dismiss_quick_launch_preview(cx);
        });

        application.update(cx, |application, _| {
            application.preview.visible = true;
        });
        cx.simulate_event(gpui::ScrollWheelEvent {
            position: point(px(20.0), px(80.0)),
            delta: gpui::ScrollDelta::Lines(point(0.0, -1.0)),
            ..Default::default()
        });
        application.read_with(cx, |application, _| {
            assert!(!application.preview.visible);
        });

        cx.simulate_keystrokes("right");
        panel.update_in(cx, |panel, _window, cx| {
            let mode = panel.action_mode.as_ref().expect("pending action mode");
            let mode_id = mode.id();
            let level_id = mode.current_level_id();
            let entry_id = mode.target().entry.id.clone();
            panel.apply_shell_actions(mode_id, level_id, &entry_id, None, cx);
        });
        panel.read_with(cx, |panel, _| {
            assert_eq!(
                panel
                    .action_mode
                    .as_ref()
                    .and_then(QuickLaunchActionMode::selected_action),
                Some(QuickLaunchAction::Open)
            );
        });
        cx.simulate_keystrokes("escape escape");
        panel.update_in(cx, |panel, window, cx| {
            assert!(panel.action_mode.is_none());
            assert!(panel.search_input.read(cx).value().is_empty());
            assert_eq!(window.bounds().size.height, px(COLLAPSED_PANEL_HEIGHT));
        });
        panel.update_in(cx, |panel, _window, cx| {
            panel.search_pending = true;
            panel.replace_results(Vec::new(), cx);
            assert!(!panel.search_pending);
        });
    }

    #[gpui::test]
    fn action_mode_returns_to_the_parent_viewport_on_its_first_frame(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let results = (0..40)
            .map(|index| {
                search_result(
                    format!("item-{index:02}").as_str(),
                    ApplicationSource::Calculator,
                )
            })
            .collect::<Vec<_>>();
        let (panel, window) = quick_launch_panel_fixture(cx, results);

        panel.update_in(window, |panel, window, cx| panel.focus_search(window, cx));
        window.simulate_input("item");
        panel.update_in(window, |panel, _window, cx| {
            panel.selected = 24;
            panel
                .result_scroll
                .scroll_to_item_strict(24, ScrollStrategy::Center);
            cx.notify();
        });
        window
            .refresh()
            .expect("render the scrolled parent results");
        window
            .debug_bounds("quick-launch-result-title-24")
            .expect("the selected lower result is rendered before entering actions");
        let parent_scroll_offset = panel.update_in(window, |panel, _window, _cx| {
            panel.result_scroll.0.borrow().base_handle.offset()
        });
        assert!(
            parent_scroll_offset.y < px(0.0),
            "the parent viewport must start below the first page"
        );

        for exit_key in ["escape", "left", "backspace"] {
            window.simulate_keystrokes("right");
            panel.update_in(window, |panel, _window, _cx| {
                assert!(panel.action_mode.is_some())
            });
            window
                .refresh()
                .expect("render action mode before returning");
            window
                .debug_bounds("quick-launch-action-context")
                .expect("action mode is mounted before returning");

            window.simulate_keystrokes(exit_key);
            window
                .refresh()
                .expect("render the first returned parent frame");
            window
                .debug_bounds("quick-launch-result-title-24")
                .expect("the selected lower result is rendered on the first returned frame");
            panel.update_in(window, |panel, _window, cx| {
                assert!(panel.action_mode.is_none());
                assert_eq!(panel.selected, 24);
                assert_eq!(panel.search_input.read(cx).value().as_ref(), "item");
                assert_eq!(
                    panel.result_scroll.0.borrow().base_handle.offset(),
                    parent_scroll_offset,
                    "{exit_key} must return on the preserved parent viewport without a top flash"
                );
            });
        }
    }

    #[gpui::test]
    fn action_mode_reuses_the_result_row_rails_and_only_adds_a_lightweight_context(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let (panel, window) = quick_launch_panel_fixture(
            cx,
            vec![search_result("42", ApplicationSource::Calculator)],
        );

        panel.update_in(window, |panel, window, cx| panel.focus_search(window, cx));
        window.refresh().expect("render the ordinary result row");
        let result_row = window
            .debug_bounds("quick-launch-result-row")
            .expect("ordinary result row");
        let result_icon = window
            .debug_bounds("quick-launch-result-icon-0")
            .expect("ordinary result icon rail");
        let result_copy = window
            .debug_bounds("quick-launch-result-copy-0")
            .expect("ordinary result copy rail");
        let result_title = window
            .debug_bounds("quick-launch-result-title-0")
            .expect("ordinary result title");
        let result_trailing = window
            .debug_bounds("quick-launch-result-shortcut-0")
            .expect("ordinary result trailing rail");

        window.simulate_keystrokes("right");
        panel.update_in(window, |panel, _window, _cx| {
            assert!(panel.action_mode.is_some())
        });
        window.refresh().expect("render the action-mode row");
        let context = window
            .debug_bounds("quick-launch-action-context")
            .expect("lightweight action-mode context");
        let action_row = window
            .debug_bounds("quick-launch-action-row-0")
            .expect("action row reusing the result shell");
        let action_icon = window
            .debug_bounds("quick-launch-action-icon-0")
            .expect("action icon rail");
        let action_copy = window
            .debug_bounds("quick-launch-action-copy-0")
            .expect("action copy rail");
        let action_title = window
            .debug_bounds("quick-launch-action-title-0")
            .expect("action title");
        let action_trailing = window
            .debug_bounds("quick-launch-action-trailing-0")
            .expect("action trailing rail");

        assert_eq!(context.size.height, px(QUICK_LAUNCH_ACTION_CONTEXT_HEIGHT));
        assert_eq!(action_row.size.height, result_row.size.height);
        assert_eq!(action_icon.origin.x, result_icon.origin.x);
        assert_eq!(action_icon.size.width, result_icon.size.width);
        assert_eq!(action_copy.origin.x, result_copy.origin.x);
        assert_eq!(action_title.size.height, result_title.size.height);
        assert_eq!(action_trailing.origin.x, result_trailing.origin.x);
        assert_eq!(action_trailing.size.width, result_trailing.size.width);
    }

    #[gpui::test]
    fn production_quick_launch_preview_uses_shared_fixed_shell_and_media_surface(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let payload = QuickLaunchPreviewPayload {
            entry_id: "preview-image".into(),
            name: "NxTools.png".into(),
            detail: "PNG 图片 · 48 × 48".into(),
            target: r"C:\fixtures\NxTools.png".into(),
            created_at: Some("2026-08-24 12:00:00".into()),
            modified_at: Some("2026-08-24 12:01:00".into()),
            body: QuickLaunchPreviewBody::Image(Arc::new(Image::from_bytes(
                ImageFormat::Png,
                include_bytes!(
                    "../../../../browser_extension/nxtools-interceptor/icons/icon-48.png"
                )
                .to_vec(),
            ))),
        };
        let (_root, window) = cx.add_window_view(move |window, cx| {
            window.resize(size(
                px(QUICK_LAUNCH_PREVIEW_WIDTH),
                px(QUICK_LAUNCH_PREVIEW_HEIGHT),
            ));
            let preview = cx.new(|_| {
                QuickLaunchPreview::new(
                    payload,
                    WeakEntity::<QuickLaunchApplication>::new_invalid(),
                )
            });
            Root::new(preview, window, cx)
        });

        window
            .refresh()
            .expect("render production Quick Launch preview");
        let shell = window
            .debug_bounds("preview-shell")
            .expect("shared preview shell");
        let header = window
            .debug_bounds("quick-launch-preview-header")
            .expect("stable preview identity header");
        let media = window
            .debug_bounds("preview-media-surface")
            .expect("shared preview media surface");
        let footer = window
            .debug_bounds("quick-launch-preview-footer")
            .expect("stable preview path footer");

        assert_eq!(shell.size, size(px(400.0), px(410.0)));
        assert_eq!(header.size.height, px(72.0));
        assert_eq!(footer.size.height, px(30.0));
        assert_eq!(media.size, size(px(382.0), px(290.0)));
    }

    #[gpui::test]
    fn production_quick_launch_text_preview_starts_at_the_media_inset(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let payload = QuickLaunchPreviewPayload {
            entry_id: "preview-text".into(),
            name: "notes.txt".into(),
            detail: "TXT 文件 · 24 B".into(),
            target: r"C:\fixtures\notes.txt".into(),
            created_at: None,
            modified_at: None,
            body: QuickLaunchPreviewBody::Text {
                text: "first line\nsecond line".into(),
                truncated: true,
            },
        };
        let (_root, window) = cx.add_window_view(move |window, cx| {
            window.resize(size(
                px(QUICK_LAUNCH_PREVIEW_WIDTH),
                px(QUICK_LAUNCH_PREVIEW_HEIGHT),
            ));
            let preview = cx.new(|_| {
                QuickLaunchPreview::new(
                    payload,
                    WeakEntity::<QuickLaunchApplication>::new_invalid(),
                )
            });
            Root::new(preview, window, cx)
        });

        window.refresh().expect("render text preview");
        let media = window
            .debug_bounds("preview-media-surface")
            .expect("text preview media surface");
        let copy = window
            .debug_bounds("quick-launch-preview-text-copy")
            .expect("top-aligned text preview copy");

        assert_eq!(copy.left(), media.left() + px(SPACING_12));
        assert_eq!(copy.top(), media.top() + px(SPACING_12));
    }

    #[gpui::test]
    fn keyboard_navigation_retains_focus_after_first_preview_frame(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let temp = tempfile::tempdir().expect("temporary keyboard preview test directory");
        let root = temp.path().join("applications");
        std::fs::create_dir_all(&root).expect("application test root");
        std::fs::write(root.join("alpha.lnk"), b"").expect("first test shortcut");
        std::fs::write(root.join("alpine.lnk"), b"").expect("second test shortcut");
        let catalog = FinderCatalog::open(
            temp.path().join("history.db"),
            &[nxtools_finder::ApplicationRoot {
                path: root,
                source: ApplicationSource::StartMenu,
            }],
        )
        .expect("keyboard preview catalog");
        let results = catalog
            .search("alp", RESULT_LIMIT)
            .expect("preview results");
        assert_eq!(results.len(), 2);
        let application = cx.update(|cx| {
            let notifications = cx.new(|_| NotificationCenter::default());
            let (platform, _endpoint) = nxtools_ipc::platform_bridge(8);
            let (indexer, _indexer_endpoint) = crate::runtime::indexer_bridge(8);
            cx.new(|_| {
                QuickLaunchApplication::new(
                    catalog,
                    Rc::new(platform),
                    Rc::new(indexer),
                    AppConfig::default(),
                    temp.path().join("config.json"),
                    temp.path().join("logs"),
                    notifications,
                )
            })
        });
        let panel_slot = Rc::new(RefCell::new(None));
        let panel_slot_for_window = panel_slot.clone();
        let popup_window_slot = Rc::new(RefCell::new(None));
        let popup_window_slot_for_window = popup_window_slot.clone();
        let application_for_window = application.clone();
        let (_root, cx) = cx.add_window_view(move |window, cx| {
            *popup_window_slot_for_window.borrow_mut() = Some(window.window_handle());
            let panel = cx.new(|cx| {
                QuickLaunchPanel::new(
                    results,
                    nxtools_protocol::ForegroundContext {
                        window_handle: 1,
                        focus_window_handle: 1,
                        process_id: 1,
                        process_name: "editor".into(),
                        window_title: "document".into(),
                    },
                    None,
                    Vec::new(),
                    application_for_window.downgrade(),
                    window,
                    cx,
                )
            });
            *panel_slot_for_window.borrow_mut() = Some(panel.clone());
            Root::new(panel, window, cx)
        });
        let panel = panel_slot
            .borrow_mut()
            .take()
            .expect("keyboard preview panel");
        let popup_window = popup_window_slot
            .borrow_mut()
            .take()
            .expect("keyboard preview popup window");
        application.update(cx, |application, _| {
            application.popup = Some(QuickLaunchWindow {
                window: popup_window,
                panel: panel.downgrade(),
                native_window_handle: 1,
            });
        });
        panel.update_in(cx, |panel, window, cx| panel.focus_search(window, cx));
        cx.simulate_input("alp");
        panel.update_in(cx, |panel, _window, cx| panel.run_search("alp".into(), cx));

        let initial_selection = panel.read_with(cx, |panel, _| panel.selected);
        cx.simulate_keystrokes("down");
        let first_selection = panel.read_with(cx, |panel, _| panel.selected);
        assert_ne!(first_selection, initial_selection);
        cx.executor()
            .advance_clock(QUICK_LAUNCH_PREVIEW_HOVER_DELAY);
        cx.run_until_parked();

        let preview_window = application.read_with(cx, |application, _| {
            assert!(application.preview.visible);
            application
                .preview
                .window
                .as_ref()
                .expect("first keyboard preview window")
                .identity
                .gpui
        });
        let preview_bounds = preview_window
            .update(cx, |_, window, _| window.bounds())
            .expect("first preview window bounds");
        assert_eq!(preview_bounds.size.width, px(QUICK_LAUNCH_PREVIEW_WIDTH));
        assert_eq!(preview_bounds.size.height, px(QUICK_LAUNCH_PREVIEW_HEIGHT));
        panel.update_in(cx, |panel, window, cx| {
            assert!(panel.search_input.focus_handle(cx).is_focused(window));
        });

        let selection_before_second_key = panel.read_with(cx, |panel, _| panel.selected);
        cx.simulate_keystrokes("down");
        panel.read_with(cx, |panel, _| {
            assert_ne!(panel.selected, selection_before_second_key);
        });
    }

    #[gpui::test]
    fn quick_launch_preview_lifecycle_discards_stale_hover_and_honors_pointer_grace(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let temp = tempfile::tempdir().expect("temporary quick-launch preview test directory");
        let root = temp.path().join("applications");
        std::fs::create_dir_all(&root).expect("application root");
        let first_path = temp.path().join("first.txt");
        let second_path = temp.path().join("second.txt");
        std::fs::write(&first_path, "first preview").expect("first preview fixture");
        std::fs::write(&second_path, "second preview").expect("second preview fixture");
        let catalog = FinderCatalog::open(
            temp.path().join("history.db"),
            &[nxtools_finder::ApplicationRoot {
                path: root,
                source: ApplicationSource::StartMenu,
            }],
        )
        .expect("test quick-launch catalog");
        let application = cx.update(|cx| {
            let notifications = cx.new(|_| NotificationCenter::default());
            let (platform, _endpoint) = nxtools_ipc::platform_bridge(8);
            let (indexer, _indexer_endpoint) = crate::runtime::indexer_bridge(8);
            cx.new(|_| {
                QuickLaunchApplication::new(
                    catalog,
                    Rc::new(platform),
                    Rc::new(indexer),
                    AppConfig::default(),
                    temp.path().join("config.json"),
                    temp.path().join("logs"),
                    notifications,
                )
            })
        });
        let make_result = |id: &str, path: &Path| {
            let mut result = search_result(id, ApplicationSource::IndexedFile);
            result.entry.target = path.to_string_lossy().into_owned();
            result
        };
        let first = make_result("first", &first_path);
        let second = make_result("second", &second_path);
        let replacement = first.clone();
        let panel_bounds = Bounds::new(
            point(px(200.0), px(120.0)),
            size(px(PANEL_WIDTH), px(PANEL_HEIGHT)),
        );

        application.update(cx, |application, cx| {
            application.schedule_quick_launch_preview(true, first, panel_bounds, 1.0, cx);
            application.schedule_quick_launch_preview(true, second.clone(), panel_bounds, 1.0, cx);
        });
        cx.executor()
            .advance_clock(QUICK_LAUNCH_PREVIEW_HOVER_DELAY);
        cx.run_until_parked();
        application.read_with(cx, |application, _| {
            assert!(application.preview.visible);
            assert_eq!(
                application.preview.displayed_entry_id.as_deref(),
                Some("second")
            );
            assert!(application.preview.window.is_some());
        });

        let visible_preview = application.read_with(cx, |application, _| {
            application
                .preview
                .window
                .as_ref()
                .expect("visible reused preview window")
                .view
                .clone()
        });
        let interrupted_exit_generation = visible_preview
            .update(cx, |preview, cx| {
                preview.begin_exit(cx);
                preview.appearance_generation
            })
            .expect("begin preview exit before target switch");
        application.update(cx, |application, cx| {
            application.schedule_quick_launch_preview(true, replacement, panel_bounds, 1.0, cx);
        });
        visible_preview
            .read_with(cx, |preview, _| {
                assert!(
                    !preview.is_exiting,
                    "a visible reused shell must not play an exit before replacement"
                );
                assert_eq!(
                    preview.appearance_generation, interrupted_exit_generation,
                    "cancelling an old exit must not animate the old payload back in"
                );
                assert!(
                    preview.motion_suppressed,
                    "the interrupted old payload must render without an animation wrapper"
                );
            })
            .expect("visible reused preview view");
        application.update(cx, |application, cx| {
            application.schedule_quick_launch_preview(true, second.clone(), panel_bounds, 1.0, cx);
        });

        application.update(cx, |application, cx| {
            application.schedule_quick_launch_preview(false, second, panel_bounds, 1.0, cx);
            application.set_quick_launch_preview_pointer_inside(true, cx);
        });
        cx.executor()
            .advance_clock(QUICK_LAUNCH_PREVIEW_LEAVE_DELAY);
        cx.run_until_parked();
        application.read_with(cx, |application, _| {
            assert!(application.preview.visible);
        });

        application.update(cx, |application, cx| {
            application.set_quick_launch_preview_pointer_inside(false, cx);
        });
        cx.executor()
            .advance_clock(QUICK_LAUNCH_PREVIEW_LEAVE_DELAY);
        cx.run_until_parked();
        application.read_with(cx, |application, _| {
            assert!(application.preview.visible);
        });
        cx.executor()
            .advance_clock(QUICK_LAUNCH_PREVIEW_DISAPPEAR_DURATION);
        cx.run_until_parked();
        application.read_with(cx, |application, _| {
            assert!(!application.preview.visible);
        });

        let failed_reveal = uuid::Uuid::new_v4();
        application.update(cx, |application, cx| {
            let window_identity = application
                .preview
                .window
                .as_ref()
                .expect("failed reveal preview window")
                .identity;
            application.preview.generation = 20;
            application.preview.hovered_entry_id = Some("second".into());
            application.preview.visible = true;
            application.preview.pending_stage = Some(QuickLaunchPreviewPendingStage {
                command_id: failed_reveal,
                phase: QuickLaunchPreviewStagePhase::RevealOnscreen,
                window_identity,
                generation: 20,
                entry_id: "second".into(),
                payload: None,
                preview_size: size(
                    px(QUICK_LAUNCH_PREVIEW_WIDTH),
                    px(QUICK_LAUNCH_PREVIEW_HEIGHT),
                ),
                bounds: panel_bounds,
                panel_scale_factor: 1.0,
            });
            application.handle_command_result(
                failed_reveal,
                PlatformCommandResult {
                    success: false,
                    error: Some("move failed".into()),
                    native_error_code: None,
                    data: None,
                },
                cx,
            );
        });
        application.read_with(cx, |application, _| {
            assert!(!application.preview.visible);
            assert!(application.preview.displayed_entry_id.is_none());
            assert!(application.preview.pending_stage.is_none());
        });

        let successful_reveal = uuid::Uuid::new_v4();
        application.update(cx, |application, cx| {
            let window_identity = application
                .preview
                .window
                .as_ref()
                .expect("successful reveal preview window")
                .identity;
            application.preview.generation = 21;
            application.preview.hovered_entry_id = Some("second".into());
            application.preview.visible = true;
            application.preview.pending_stage = Some(QuickLaunchPreviewPendingStage {
                command_id: successful_reveal,
                phase: QuickLaunchPreviewStagePhase::RevealOnscreen,
                window_identity,
                generation: 21,
                entry_id: "second".into(),
                payload: None,
                preview_size: size(
                    px(QUICK_LAUNCH_PREVIEW_WIDTH),
                    px(QUICK_LAUNCH_PREVIEW_HEIGHT),
                ),
                bounds: panel_bounds,
                panel_scale_factor: 1.0,
            });
            application.handle_command_result(
                successful_reveal,
                PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data: None,
                },
                cx,
            );
        });
        application.read_with(cx, |application, _| {
            assert!(application.preview.visible);
            assert_eq!(
                application.preview.displayed_entry_id.as_deref(),
                Some("second")
            );
            assert!(application.preview.pending_stage.is_none());
        });

        let stale_window_stage = uuid::Uuid::new_v4();
        application.update(cx, |application, cx| {
            let preview_window = application
                .preview
                .window
                .as_mut()
                .expect("stale stage preview window");
            preview_window.identity.native = 111;
            let window_identity = preview_window.identity;
            application.preview.generation = 22;
            application.preview.hovered_entry_id = Some("second".into());
            application.preview.visible = true;
            application.preview.pending_stage = Some(QuickLaunchPreviewPendingStage {
                command_id: stale_window_stage,
                phase: QuickLaunchPreviewStagePhase::RevealOnscreen,
                window_identity,
                generation: 22,
                entry_id: "second".into(),
                payload: None,
                preview_size: size(
                    px(QUICK_LAUNCH_PREVIEW_WIDTH),
                    px(QUICK_LAUNCH_PREVIEW_HEIGHT),
                ),
                bounds: panel_bounds,
                panel_scale_factor: 1.0,
            });
            application
                .preview
                .window
                .as_mut()
                .expect("replacement preview window")
                .identity
                .native = 222;
            application.preview.displayed_entry_id = Some("current-window".into());
            application.handle_command_result(
                stale_window_stage,
                PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data: None,
                },
                cx,
            );
        });
        application.read_with(cx, |application, _| {
            assert_eq!(
                application.preview.displayed_entry_id.as_deref(),
                Some("current-window"),
                "a stale stage must not publish into a replacement native window"
            );
            assert!(application.preview.pending_stage.is_none());
        });

        let generation_before_unsupported =
            application.read_with(cx, |application, _| application.preview.generation);
        let mut unsupported = search_result("website", ApplicationSource::WebUrl);
        unsupported.entry.target = "https://example.com".into();
        application.update(cx, |application, cx| {
            application.schedule_quick_launch_preview(true, unsupported, panel_bounds, 1.0, cx);
        });
        application.read_with(cx, |application, _| {
            assert!(application.preview.generation > generation_before_unsupported);
            assert!(application.preview.hovered_entry_id.is_none());
            assert!(!application.preview.visible);
        });
    }

    #[test]
    fn popup_dimensions_are_compact_and_do_not_depend_on_the_main_window() {
        assert_eq!(PANEL_WIDTH, 620.0);
        assert_eq!(PANEL_HEIGHT, 410.0);
        assert_eq!(SEARCH_HEADER_HEIGHT, 42.0);
        assert_eq!(RESULT_LIMIT, 200);
        assert_eq!(INDEX_CANDIDATE_LIMIT, 200);
    }

    #[test]
    fn launcher_keeps_two_hundred_ranked_results_scrollable() {
        let mut session = IndexSearchSession::default();
        let request = session.begin("needle", INDEX_CANDIDATE_LIMIT);
        assert_eq!(request.limit, 200);

        let temp = tempfile::tempdir().expect("temporary quick-launch catalog");
        let root = temp.path().join("empty-applications");
        std::fs::create_dir_all(&root).expect("empty application root");
        let catalog = FinderCatalog::open(
            temp.path().join("history.db"),
            &[nxtools_finder::ApplicationRoot {
                path: root,
                source: ApplicationSource::StartMenu,
            }],
        )
        .expect("test quick-launch catalog");
        let external = (0..260)
            .map(|index| ExternalSearchEntry {
                id: format!("indexed:{index}"),
                name: format!("needle-{index:02}"),
                target: format!(r"C:\needle-{index:02}.txt"),
                source: ApplicationSource::IndexedFile,
            })
            .collect::<Vec<_>>();

        let results = catalog
            .search_with_external("needle", &external, RESULT_LIMIT)
            .expect("merged quick-launch results");

        assert_eq!(results.len(), 200);
    }

    #[test]
    fn stationary_pointer_does_not_steal_keyboard_selection() {
        let mut gate = PointerSelectionGate::default();

        assert!(!gate.observe_move(3, (120.0, 80.0)));
        gate.reset();
        assert!(!gate.observe_move(3, (120.0, 80.0)));
        assert!(!gate.observe_move(3, (120.0, 80.0)));
        assert!(gate.observe_move(3, (121.0, 80.0)));
        assert!(!gate.observe_move(4, (121.0, 94.0)));
        assert!(gate.observe_move(4, (122.0, 94.0)));
    }

    #[test]
    fn popup_hides_only_for_external_focus_or_pointer_after_opening_grace() {
        assert!(!should_hide_for_foreground(true, false, 10, 11, 20));
        assert!(!should_hide_for_foreground(true, true, 10, 11, 10));
        assert!(!should_hide_for_foreground(true, true, 10, 11, 11));
        assert!(should_hide_for_foreground(true, true, 10, 11, 20));
        assert!(!should_hide_for_foreground(false, true, 10, 11, 20));
        assert!(!should_hide_for_pointer(true, 10, 10, 200, 100));
        assert!(should_hide_for_pointer(true, 10, 20, 200, 100));
        assert!(!should_hide_for_pointer(true, 10, 20, 100, 100));
    }

    #[test]
    fn popup_activation_retries_topmost_and_finishes_with_post_activation_confirmation() {
        use QuickLaunchWindowActivationStep::*;

        assert_eq!(
            next_window_activation_step(PreTopmost(0), false),
            Some(PreTopmost(1))
        );
        assert_eq!(
            next_window_activation_step(PreTopmost(1), false),
            Some(Activate(0))
        );
        assert_eq!(
            next_window_activation_step(PreTopmost(0), true),
            Some(Activate(0))
        );
        assert_eq!(
            next_window_activation_step(Activate(0), false),
            Some(Activate(1))
        );
        assert_eq!(
            next_window_activation_step(Activate(1), false),
            Some(ConfirmTopmost(0))
        );
        assert_eq!(
            next_window_activation_step(Activate(0), true),
            Some(ConfirmTopmost(0))
        );
        assert_eq!(
            next_window_activation_step(ConfirmTopmost(0), false),
            Some(ConfirmTopmost(1))
        );
        assert_eq!(
            next_window_activation_step(ConfirmTopmost(1), false),
            Some(ConfirmTopmost(2))
        );
        assert_eq!(next_window_activation_step(ConfirmTopmost(2), false), None);
        assert_eq!(next_window_activation_step(ConfirmTopmost(0), true), None);
    }

    #[test]
    fn search_focus_waits_until_windows_confirms_foreground_activation() {
        use QuickLaunchWindowActivationStep::*;

        assert!(!should_focus_search_after_activation(PreTopmost(0), true));
        assert!(!should_focus_search_after_activation(Activate(0), false));
        assert!(should_focus_search_after_activation(Activate(0), true));
        assert!(!should_focus_search_after_activation(
            ConfirmTopmost(0),
            true
        ));
    }

    #[test]
    fn enter_modifiers_map_to_open_reveal_and_elevated_actions() {
        assert_eq!(
            quick_launch_action_for_enter(false, false),
            QuickLaunchAction::Open
        );
        assert_eq!(
            quick_launch_action_for_enter(true, false),
            QuickLaunchAction::Reveal
        );
        assert_eq!(
            quick_launch_action_for_enter(true, true),
            QuickLaunchAction::RunElevated
        );
        assert_eq!(
            shell_action_for(ApplicationSource::StartMenu, &QuickLaunchAction::Open),
            Some(ShellOpenAction::Open)
        );
        assert_eq!(
            shell_action_for(ApplicationSource::System, &QuickLaunchAction::Open),
            Some(ShellOpenAction::SystemTool)
        );
        assert_eq!(
            shell_action_for(ApplicationSource::System, &QuickLaunchAction::Reveal),
            None
        );
        assert_eq!(
            shell_action_for(ApplicationSource::System, &QuickLaunchAction::RunElevated),
            Some(ShellOpenAction::RunElevated)
        );
        assert_eq!(
            shell_action_for(ApplicationSource::IndexedFile, &QuickLaunchAction::Open),
            Some(ShellOpenAction::Open)
        );
        assert_eq!(
            shell_action_for(
                ApplicationSource::IndexedDirectory,
                &QuickLaunchAction::Reveal
            ),
            Some(ShellOpenAction::SelectInExplorer)
        );
    }

    #[test]
    fn catalog_refresh_preserves_the_selected_entry_and_falls_back_to_first() {
        fn result(id: &str) -> SearchResult {
            SearchResult {
                entry: nxtools_finder::ApplicationEntry {
                    id: id.into(),
                    name: id.into(),
                    target: format!("{id}.lnk"),
                    source: ApplicationSource::StartMenu,
                },
                action: SearchResultAction::OpenTarget,
                subtitle: None,
                score: 0,
                activation_count: 0,
                last_activated_at_unix_ms: None,
                query_activation_count: 0,
                last_query_activation_at_unix_ms: None,
            }
        }

        let current = vec![result("alpha"), result("beta")];
        let reordered = vec![result("beta"), result("alpha")];
        let replaced = vec![result("gamma")];

        assert_eq!(selection_after_results_refresh(&current, 1, &reordered), 0);
        assert_eq!(selection_after_results_refresh(&current, 1, &replaced), 0);
    }

    #[test]
    fn catalog_refresh_never_replaces_the_frozen_action_mode_parent() {
        assert_eq!(
            quick_launch_catalog_refresh_query(true, false, "parent query".into()),
            Some("parent query".into())
        );
        assert_eq!(
            quick_launch_catalog_refresh_query(true, true, "action filter".into()),
            None
        );
        assert_eq!(
            quick_launch_catalog_refresh_query(false, false, "closed query".into()),
            None
        );
    }

    #[test]
    fn empty_query_keeps_the_launcher_collapsed_until_the_user_types() {
        assert!(!quick_launch_results_visible("", false, false));
        assert!(!quick_launch_results_visible("   ", false, false));
        assert!(quick_launch_results_visible("zed", false, false));
        assert!(quick_launch_results_visible("", true, false));
        assert!(quick_launch_results_visible("", false, true));
        assert_eq!(
            quick_launch_panel_height("", false, false),
            COLLAPSED_PANEL_HEIGHT
        );
        assert_eq!(quick_launch_panel_height("zed", false, false), PANEL_HEIGHT);
    }

    #[test]
    fn empty_result_copy_is_provider_neutral() {
        assert_eq!(QUICK_LAUNCH_EMPTY_RESULTS_LABEL, "没有匹配项");
    }

    #[test]
    fn ime_confirmation_enter_is_consumed_but_the_following_enter_executes() {
        let mut gate = ImeEnterGate::default();
        gate.observe_composition(true);
        gate.observe_change();

        assert!(!gate.consume_enter());
        assert!(gate.consume_enter());
    }

    #[test]
    fn direct_unicode_input_does_not_look_like_an_ime_confirmation() {
        let mut gate = ImeEnterGate::default();
        gate.observe_change();

        assert!(gate.consume_enter());
    }

    #[test]
    fn connected_index_search_waits_for_the_combined_batch_before_first_publish() {
        assert!(!should_search_indexer(false, false));
        assert!(!should_search_indexer(true, false));
        assert!(should_search_indexer(true, true));
        assert!(should_publish_immediate_results(false, false));
        assert!(should_publish_immediate_results(true, false));
        assert!(!should_publish_immediate_results(true, true));
    }

    #[test]
    fn every_non_empty_query_keeps_the_indexed_file_provider_enabled() {
        assert!(!query_requests_file_index(""));
        assert!(query_requests_file_index("a"));
        assert!(query_requests_file_index("* A "));
        assert!(query_requests_file_index("ab"));
        assert!(query_requests_file_index("中"));
        assert!(query_requests_file_index("^a"));
    }

    #[test]
    fn a_pending_query_keeps_the_last_committed_rows_until_the_combined_batch_arrives() {
        let mut committed = vec!["previous-result"];

        let published = apply_search_update(&mut committed, Vec::new(), true);

        assert!(!published);
        assert_eq!(committed, vec!["previous-result"]);
    }

    #[test]
    fn a_rebuilding_index_remains_searchable_while_the_last_snapshot_is_available() {
        let rebuilding_with_snapshot = IndexStatus {
            phase: nxtools_protocol::IndexPhase::VolumesReady,
            indexed_items: 2_000_000,
            drives: Vec::new(),
            last_error: Some("rebuilding in background".into()),
        };
        let initial_build_without_snapshot = IndexStatus {
            indexed_items: 0,
            ..rebuilding_with_snapshot.clone()
        };

        assert!(index_status_has_searchable_snapshot(Some(
            &rebuilding_with_snapshot
        )));
        assert!(!index_status_has_searchable_snapshot(Some(
            &initial_build_without_snapshot
        )));
    }

    #[test]
    fn a_ready_status_retries_a_query_entered_during_indexer_reconnect() {
        let mut session = IndexSearchSession::default();
        session.begin("quick-launch-plan", RESULT_LIMIT);
        session.invalidate();
        assert!(
            session.query.is_empty(),
            "disconnect invalidation must not prevent the visible panel query from being retried"
        );
        let ready = IndexStatus {
            phase: nxtools_protocol::IndexPhase::Ready,
            indexed_items: 2_000_000,
            drives: Vec::new(),
            last_error: None,
        };

        assert!(should_retry_index_search_after_status(None, &ready));
        assert!(!should_retry_index_search_after_status(
            Some(&ready),
            &ready
        ));
        let building = IndexStatus {
            phase: nxtools_protocol::IndexPhase::VolumesReady,
            ..ready.clone()
        };
        assert!(!should_retry_index_search_after_status(None, &building));
    }

    #[test]
    fn a_cursor_only_index_status_update_does_not_repaint_the_launcher() {
        let mut status = Some(IndexStatus {
            phase: nxtools_protocol::IndexPhase::Ready,
            indexed_items: 42,
            drives: vec![nxtools_protocol::IndexDriveStatus {
                mount_point: r"C:\".into(),
                file_system: "NTFS".into(),
                indexed_items: 42,
                journal_id: Some(17),
                next_usn: Some(100),
            }],
            last_error: None,
        });
        let mut session = IndexSearchSession::default();
        let mut cursor_advanced = status.clone().expect("initial status");
        cursor_advanced.drives[0].next_usn = Some(101);

        let changed = apply_indexer_notification(
            &mut status,
            &mut session,
            IndexerNotification::Event(IndexerEvent::Status(cursor_advanced)),
        );

        assert!(!changed);
        assert_eq!(
            status.expect("updated status").drives[0].next_usn,
            Some(101)
        );
    }

    #[test]
    fn result_rows_have_one_selection_visual_and_no_type_suffix() {
        let source = include_str!("quick_launch.rs");
        let render_result = source
            .split("fn render_result(")
            .nth(1)
            .and_then(|source| source.split("fn render_action(").next())
            .expect("render_result source");

        assert!(render_result.contains(".on_hover("));
        assert!(render_result.contains(".on_mouse_move("));
        assert!(!render_result.contains("result.entry.source.label()"));
    }

    #[test]
    fn preview_header_uses_shared_body_and_label_roles() {
        let source = include_str!("quick_launch.rs");
        let render = source
            .split("impl Render for QuickLaunchPreview")
            .nth(1)
            .and_then(|source| {
                source
                    .split("pub(crate) enum QuickLaunchApplicationEvent")
                    .next()
            })
            .expect("Quick Launch preview render");

        assert!(render.contains(".text_size(px(TEXT_BODY))"));
        assert!(render.contains(".line_height(px(LINE_HEIGHT_BODY))"));
        assert!(render.contains(".text_size(px(TEXT_LABEL))"));
        assert!(render.contains(".line_height(px(LINE_HEIGHT_LABEL))"));
        assert!(!render.contains("QUICK_LAUNCH_PREVIEW_TITLE_SIZE"));
    }

    #[test]
    fn every_preview_payload_keeps_the_reused_window_at_the_readable_reference_height() {
        let metadata = QuickLaunchPreviewPayload {
            entry_id: String::new(),
            name: String::new(),
            target: String::new(),
            detail: String::new(),
            created_at: None,
            modified_at: None,
            body: QuickLaunchPreviewBody::Metadata {
                icon: MaterialIcon::Info,
            },
        };
        let text = QuickLaunchPreviewPayload {
            entry_id: String::new(),
            name: String::new(),
            target: String::new(),
            detail: String::new(),
            created_at: None,
            modified_at: None,
            body: QuickLaunchPreviewBody::Text {
                text: String::new(),
                truncated: false,
            },
        };

        assert_eq!(
            quick_launch_preview_height(&metadata),
            QUICK_LAUNCH_PREVIEW_HEIGHT
        );
        assert_eq!(
            quick_launch_preview_height(&text),
            QUICK_LAUNCH_PREVIEW_HEIGHT
        );
    }

    #[test]
    fn quick_launch_preview_classifies_lightweight_content_without_opening_missing_targets() {
        let temp = tempfile::tempdir().expect("temporary preview directory");
        let image = temp.path().join("sample.PNG");
        let animated_gif = temp.path().join("animated.gif");
        let animated_webp = temp.path().join("animated.webp");
        let text = temp.path().join("notes.md");
        let binary = temp.path().join("archive.zip");
        std::fs::write(&image, b"not decoded by the classifier").expect("image fixture");
        std::fs::write(&animated_gif, b"GIF89a").expect("GIF fixture");
        std::fs::write(&animated_webp, b"RIFF").expect("WebP fixture");
        std::fs::write(&text, "一段预览文本").expect("text fixture");
        std::fs::write(&binary, b"PK").expect("binary fixture");

        assert_eq!(
            quick_launch_preview_kind(&image),
            Some(QuickLaunchPreviewKind::Image)
        );
        assert_eq!(
            quick_launch_preview_kind(&text),
            Some(QuickLaunchPreviewKind::Text)
        );
        assert_eq!(
            quick_launch_preview_kind(&animated_gif),
            Some(QuickLaunchPreviewKind::Metadata)
        );
        assert_eq!(
            quick_launch_preview_kind(&animated_webp),
            Some(QuickLaunchPreviewKind::Metadata)
        );
        assert_eq!(
            quick_launch_preview_kind(&binary),
            Some(QuickLaunchPreviewKind::Metadata)
        );
        assert_eq!(
            quick_launch_preview_kind(&temp.path().join("missing.txt")),
            None
        );
    }

    #[test]
    fn quick_launch_unavailable_targets_still_produce_a_metadata_preview() {
        let temp = tempfile::tempdir().expect("temporary preview directory");
        let missing = temp.path().join("unavailable-result.exe");
        let mut result = search_result("unavailable", ApplicationSource::IndexedFile);
        result.entry.target = missing.to_string_lossy().into_owned();

        let payload = load_quick_launch_preview(&result)
            .expect("every visible result must still have a metadata preview");
        assert_eq!(payload.entry_id, result.entry.id);
        assert!(payload.detail.contains("不可访问"));
        assert!(payload.created_at.is_none());
        assert!(payload.modified_at.is_none());
        assert!(matches!(
            payload.body,
            QuickLaunchPreviewBody::Metadata { .. }
        ));
    }

    #[test]
    fn quick_launch_text_preview_is_bounded_and_marks_truncation() {
        let temp = tempfile::tempdir().expect("temporary preview directory");
        let path = temp.path().join("large.txt");
        std::fs::write(&path, vec![b'a'; QUICK_LAUNCH_TEXT_PREVIEW_LIMIT + 32])
            .expect("large text fixture");
        let mut result = search_result("large", ApplicationSource::IndexedFile);
        result.entry.target = path.to_string_lossy().into_owned();

        let payload = load_quick_launch_preview(&result).expect("preview payload");
        #[cfg(windows)]
        assert!(payload.created_at.is_some());
        assert!(payload.modified_at.is_some());
        assert!(payload.modified_at.as_deref().is_some_and(|value| {
            value
                .chars()
                .filter(|character| character.is_ascii_digit())
                .count()
                == 14
        }));
        let QuickLaunchPreviewBody::Text { text, truncated } = payload.body else {
            panic!("text files must use the bounded text preview body");
        };
        assert!(truncated);
        assert!(text.len() <= QUICK_LAUNCH_TEXT_PREVIEW_LIMIT);
    }

    #[test]
    fn quick_launch_image_preview_rejects_large_files_and_decode_surfaces() {
        assert!(quick_launch_image_within_budget(256 * 1024, 3840, 2160));
        assert!(!quick_launch_image_within_budget(
            QUICK_LAUNCH_IMAGE_FILE_LIMIT + 1,
            1920,
            1080
        ));
        assert!(!quick_launch_image_within_budget(1024, 8192, 8192));
        assert!(!quick_launch_image_within_budget(1024, 0, 1080));
        assert!(!quick_launch_image_within_budget(
            1024,
            usize::MAX,
            usize::MAX
        ));
    }

    #[test]
    fn quick_launch_static_image_preview_owns_the_validated_bounded_bytes() {
        let temp = tempfile::tempdir().expect("temporary preview directory");
        let path = temp.path().join("pixel.bmp");
        let bytes = [
            0x42, 0x4d, 58, 0, 0, 0, 0, 0, 0, 0, 54, 0, 0, 0, 40, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0,
            1, 0, 24, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 255, 0,
        ];
        std::fs::write(&path, bytes).expect("BMP fixture");
        let mut result = search_result("pixel", ApplicationSource::IndexedFile);
        result.entry.target = path.to_string_lossy().into_owned();

        let payload = load_quick_launch_preview(&result).expect("preview payload");
        let QuickLaunchPreviewBody::Image(image) = payload.body else {
            panic!("bounded static images must own the validated bytes");
        };
        assert_eq!(image.format, ImageFormat::Bmp);
        assert_eq!(image.bytes, bytes);
    }

    #[test]
    fn quick_launch_preview_window_is_reused_for_target_changes() {
        assert_eq!(
            quick_launch_preview_open_action(false, false, None, "a"),
            QuickLaunchPreviewOpenAction::Create
        );
        assert_eq!(
            quick_launch_preview_open_action(true, true, Some("a"), "a"),
            QuickLaunchPreviewOpenAction::Ignore
        );
        assert_eq!(
            quick_launch_preview_open_action(true, true, Some("a"), "b"),
            QuickLaunchPreviewOpenAction::Reuse
        );
        assert!(quick_launch_preview_target_changed(true, Some("a"), "b"));
        assert!(!quick_launch_preview_target_changed(true, Some("a"), "a"));
        assert!(QUICK_LAUNCH_PREVIEW_SWITCH_DELAY < QUICK_LAUNCH_PREVIEW_HOVER_DELAY);
    }

    #[test]
    fn reused_quick_launch_preview_waits_for_offscreen_confirmation_before_repainting() {
        let source = include_str!("quick_launch.rs");
        let stage = source
            .split("fn stage_reused_quick_launch_preview(")
            .nth(1)
            .and_then(|source| {
                source
                    .split("fn handle_quick_launch_preview_stage_result(")
                    .next()
            })
            .expect("quick-launch preview staging source");
        assert!(stage.contains("self.preview.pending_stage = Some(stage)"));
        assert!(!stage.contains("preview.set_payload"));

        let completion = source
            .split("fn handle_quick_launch_preview_stage_result(")
            .nth(1)
            .and_then(|source| {
                source
                    .split("fn render_staged_quick_launch_preview(")
                    .next()
            })
            .expect("quick-launch preview stage completion source");
        assert!(completion.contains("render_staged_quick_launch_preview"));
        assert!(completion.contains("QuickLaunchPreviewStagePhase::RevealOnscreen"));

        let set_payload = source
            .split("fn set_payload(")
            .nth(1)
            .and_then(|source| source.split("impl Render for QuickLaunchPreview").next())
            .expect("quick-launch preview target update source");
        assert!(set_payload.contains("self.release_image_asset(cx)"));
        assert!(set_payload.contains("self.begin_enter(cx)"));

        let hide = source
            .split("fn hide_quick_launch_preview_window(")
            .nth(1)
            .and_then(|source| source.split("fn show_quick_launch_preview(").next())
            .expect("quick-launch preview hide source");
        assert!(hide.contains("self.preview.image_asset.take()"));
        assert!(hide.contains("remove_asset(cx)"));
        assert!(hide.contains("preview.clear_image_asset(cx)"));
    }

    #[test]
    fn quick_launch_preview_has_short_generation_scoped_enter_and_exit_motion() {
        let source = include_str!("quick_launch.rs");
        let preview = source
            .split("impl QuickLaunchPreview {")
            .nth(1)
            .and_then(|source| {
                source
                    .split("pub(crate) struct QuickLaunchApplication")
                    .next()
            })
            .expect("quick-launch preview source");
        assert!(preview.contains("fn begin_exit("));
        assert!(preview.contains(".opacity("));
        assert!(preview.contains("QUICK_LAUNCH_PREVIEW_DISAPPEAR_DURATION"));

        let hide = source
            .split("fn animate_quick_launch_preview_exit(")
            .nth(1)
            .and_then(|source| {
                source
                    .split("fn suspend_quick_launch_preview_for_scroll")
                    .next()
            })
            .expect("quick-launch preview animated hide source");
        assert!(hide.contains("begin_exit(cx)"));
        assert!(hide.contains("timer(QUICK_LAUNCH_PREVIEW_DISAPPEAR_DURATION)"));
        assert!(PREVIEW_APPEAR_DURATION <= Duration::from_millis(140));
    }

    #[test]
    fn every_result_scroll_path_suspends_the_hover_preview() {
        let source = include_str!("quick_launch.rs");
        let render = source
            .split("impl Render for QuickLaunchPanel")
            .nth(1)
            .and_then(|source| source.split("fn source_icon(").next())
            .expect("quick-launch panel render source");
        assert!(render.contains(".on_scroll_wheel("));
        assert!(render.contains("suspend_quick_launch_preview_for_scroll"));

        let pointer_handler = source
            .split("PlatformEvent::GlobalPointerPressed(pointer)")
            .nth(1)
            .and_then(|source| source.split("PlatformEvent::ClipboardCaptured").next())
            .expect("quick-launch global pointer handler");
        assert!(
            pointer_handler.contains("pointer.root_window_handle == popup.native_window_handle")
        );
        assert!(pointer_handler.contains("suspend_quick_launch_preview_for_scroll"));
    }

    #[test]
    fn quick_launch_preview_docks_beside_the_panel_and_stays_inside_the_display() {
        let screen = Bounds::new(point(px(0.0), px(0.0)), size(px(1200.0), px(700.0)));
        let preview_size = size(
            px(QUICK_LAUNCH_PREVIEW_WIDTH),
            px(QUICK_LAUNCH_PREVIEW_HEIGHT),
        );
        let right_panel = Bounds::new(point(px(700.0), px(120.0)), size(px(480.0), px(410.0)));
        let left_panel = Bounds::new(point(px(20.0), px(620.0)), size(px(620.0), px(410.0)));

        let right = quick_launch_preview_bounds(right_panel, preview_size, screen);
        assert!(right.right() <= screen.right());
        assert!(right.origin.x < right_panel.origin.x);

        let left = quick_launch_preview_bounds(left_panel, preview_size, screen);
        assert!(left.origin.x >= screen.origin.x);
        assert!(left.bottom() <= screen.bottom());
        assert!(left.origin.x >= left_panel.right());
    }

    #[test]
    fn result_and_action_rows_share_one_full_width_shell() {
        let source = include_str!("quick_launch.rs");
        let row_shell = source
            .split("fn quick_launch_row_shell(")
            .nth(1)
            .and_then(|source| source.split("impl QuickLaunchPanel").next())
            .expect("shared quick-launch row shell");
        let render_result = source
            .split("fn render_result(")
            .nth(1)
            .and_then(|source| source.split("fn render_action(").next())
            .expect("render_result source");
        let render_action = source
            .split("fn render_action(")
            .nth(1)
            .and_then(|source| source.split("impl Render for QuickLaunchPanel").next())
            .expect("render_action source");

        assert!(render_result.contains(".id((\"quick-launch-result\", index))"));
        assert!(render_result.contains("quick_launch_row_shell(theme, selected)"));
        assert!(render_action.contains("quick_launch_row_shell(theme, selected)"));
        assert!(row_shell.contains(".w_full()"));
        assert!(row_shell.contains(".h(px(COMPACT_RESULT_ROW_HEIGHT))"));
    }

    #[test]
    fn result_appearance_motion_does_not_flash_opacity() {
        let source = include_str!("quick_launch.rs");
        let production_source = source
            .split("#[cfg(test)]")
            .next()
            .expect("production source");
        assert!(!production_source.contains(".opacity(0.82 + 0.18 * delta)"));
    }

    #[test]
    fn result_rows_use_semantic_selection_without_decorative_icon_tiles() {
        let source = include_str!("quick_launch.rs");
        let row_shell = source
            .split("fn quick_launch_row_shell(")
            .nth(1)
            .and_then(|source| source.split("impl QuickLaunchPanel").next())
            .expect("shared quick-launch row shell");

        assert!(row_shell.contains("theme.element_selected"));
        assert!(!row_shell.contains("theme.accent_soft"));
        assert!(!row_shell.contains(".bg(theme.surface_raised)"));
    }

    #[test]
    fn result_viewport_and_keyboard_navigation_share_one_scroll_handle() {
        let source = include_str!("quick_launch.rs");
        let render = source
            .split("impl Render for QuickLaunchPanel")
            .nth(1)
            .and_then(|source| source.split("fn source_icon(").next())
            .expect("quick-launch render source");

        assert!(render.contains("uniform_list("));
        assert!(source.contains("UniformListScrollHandle"));
        assert!(source.contains("ScrollStrategy::Center"));
        assert!(render.contains(".vertical_scrollbar(&self.result_scroll)"));
        assert!(!render.contains(".children(entries)"));
    }

    #[test]
    fn presented_result_range_drives_icon_loading_without_pointer_hover() {
        let source = include_str!("quick_launch.rs");
        let render = source
            .split("impl Render for QuickLaunchPanel")
            .nth(1)
            .and_then(|source| source.split("fn source_icon(").next())
            .expect("quick-launch render source");

        assert!(render.contains("update_presented_results"));
        assert!(render.contains("visible_range"));
        assert!(source.contains("fn update_presented_results("));
    }

    #[test]
    fn pointer_selection_preserves_the_current_result_scroll_position() {
        let source = include_str!("quick_launch.rs");
        let select_result = source
            .split("fn select_result(")
            .nth(1)
            .and_then(|source| source.split("fn select_action(").next())
            .expect("select_result source");

        assert!(!select_result.contains("result_scroll.set_offset"));
    }

    #[test]
    fn indexer_service_bootstrap_runs_once_and_drops_stale_pipe_commands() {
        let mut bootstrap = IndexerServiceBootstrap::default();
        let stale_command = uuid::Uuid::new_v4();
        bootstrap.pending = Some(stale_command);
        bootstrap.pending_automatic = true;

        assert!(bootstrap.on_platform_connected());
        assert_eq!(bootstrap.pending, None);
        assert!(!bootstrap.pending_automatic);
        assert!(!bootstrap.on_platform_connected());

        let mut already_connected = IndexerServiceBootstrap::default();
        assert!(already_connected.on_platform_connected());
        assert!(already_connected.automatic_attempted);
        let command = uuid::Uuid::new_v4();
        already_connected.begin(command, true);
        assert_eq!(already_connected.finish(command), Some(true));
        assert_eq!(already_connected.finish(command), None);
        assert!(!already_connected.on_platform_connected());
    }

    #[gpui::test]
    fn quick_launch_bounds_use_the_cursor_display_upper_quarter(cx: &mut TestAppContext) {
        let request = QuickLaunchPanelRequest {
            target: nxtools_protocol::ForegroundContext {
                window_handle: 1,
                focus_window_handle: 1,
                process_id: 1,
                process_name: "editor".into(),
                window_title: "document".into(),
            },
            directory_navigation_context: None,
            cursor_x: 10,
            cursor_y: 10,
            cursor_display_index: 0,
            cursor_monitor_dpi: 96,
            requested_at_unix_ms: 0,
        };
        cx.update(|cx| {
            let (bounds, _) = quick_launch_bounds(&request, false, cx);
            assert_eq!(
                bounds.size,
                size(px(PANEL_WIDTH), px(COLLAPSED_PANEL_HEIGHT))
            );
        });
    }
}
