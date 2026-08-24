use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Local};
use gpui::{
    Animation, AnimationExt, AnyElement, AnyWindowHandle, App, Context, Corner, Entity,
    FocusHandle, IntoElement, KeyDownEvent, MouseButton, ObjectFit, Render, ScrollHandle,
    ScrollStrategy, ScrollWheelEvent, Subscription, UniformListScrollHandle, Window, div,
    ease_in_out, img, point, prelude::*, px, uniform_list,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::menu::{ContextMenuExt, DropdownMenu, PopupMenu, PopupMenuItem};
use gpui_component::scroll::{ScrollableElement, Scrollbar};
use gpui_component::tooltip::Tooltip;
use gpui_component::{Disableable, Icon, PixelsExt, Sizable};
use nxtools_clipboard::ClipboardHistoryCounts;
use nxtools_core::{
    ClipboardClickAction, ClipboardClickModifier, ClipboardHotKey, ClipboardHotKeyModifier,
    ClipboardSettings, CustomCommandSettings, SystemSettings,
};
use nxtools_download::{
    DownloadBatchAction, DownloadEngineTask, DownloadFault, DownloadFaultCode, DownloadProgress,
    DownloadQueuePlacement, DownloadSegment, DownloadSegmentState, DownloadStatus,
    DownloadTaskDetail, DownloadTaskId, DownloadTaskTimelineEntry, DownloadTaskTimelineEvent,
    RequestOrigin,
};
use nxtools_ipc::PlatformBridge;
use nxtools_protocol::{
    BeginWindowMoveCommand, PlatformCommand, SetWindowMinimizedCommand, SetWindowVisibilityCommand,
    ToggleWindowMaximizedCommand,
};
use nxtools_protocol::{IndexPhase, IndexStatus, ShellOpenAction};
use tracing::info;

use crate::clipboard::ClipboardApplication;
use crate::downloads::DownloadApplication;
use crate::icons::{CaptionIcon, MaterialIcon};
use crate::network_settings::NetworkSettingsApplication;
use crate::notifications::{NOTIFICATION_EXIT_DURATION, NotificationCenter, NotificationLevel};
use crate::quick_launch::{
    QuickLaunchApplication, QuickLaunchApplicationEvent, QuickLaunchHotkeyLifecycle,
    QuickLaunchPerformanceSnapshot,
};
use crate::shell::{MainModule, ShellState};
use crate::theme::Theme;
use crate::ui::components::action::{
    SegmentedOption, action_button, icon_action, labelled_icon_action, segmented_control,
};
use crate::ui::components::custom_command_editor::{
    CustomCommandEditorActions, CustomCommandEditorInputs, CustomCommandEditorState,
    custom_command_editor,
};
use crate::ui::components::hotkey::{HotkeyFieldState, hotkey_field, hotkey_setting_row};
use crate::ui::components::menu::{MenuTone, context_menu_item, context_menu_shell};
use crate::ui::components::path_list::path_exclusion_setting;
use crate::ui::components::state::{empty_state, inline_error_state, progress_state};
use crate::ui::components::titlebar::ModuleHeader;
use crate::ui::components::volume_scope::{
    automatic_volume_scope_row, empty_volume_scope_row, volume_scope_row, volume_scope_setting,
};
use crate::ui::metrics::{
    CONTROL_HEIGHT_ACTION, RADIUS_CONTROL, SIDEBAR_ROW_HEIGHT, SIDEBAR_WIDTH, SPACING_2, SPACING_4,
    SPACING_6, SPACING_8, SPACING_12, SPACING_16, SPACING_24, TITLEBAR_HEIGHT,
};
use crate::ui::typography::{
    LINE_HEIGHT_BODY, LINE_HEIGHT_LABEL, LINE_HEIGHT_META, LINE_HEIGHT_TITLE, TEXT_BODY,
    TEXT_LABEL, TEXT_META, TEXT_TITLE, UI_FONT_FAMILY, UI_MONO_FONT_FAMILY,
};

const NOTIFICATION_ENTER_DURATION: Duration = Duration::from_millis(150);
const NOTIFICATION_CARD_WIDTH: f32 = 336.0;
const NOTIFICATION_CARD_MAX_HEIGHT: f32 = 112.0;
const QUICK_FIND_SCROLL_FRAME_DURATION: Duration = Duration::from_millis(12);
const QUICK_FIND_SCROLL_LINE_HEIGHT: f32 = 38.0;
const QUICK_FIND_SCROLL_EASING: f32 = 0.42;
const QUICK_FIND_SCROLL_TARGET_SNAP_DISTANCE: f32 = 128.0;
const QUICK_LAUNCH_SETTINGS_NAV_WIDTH: f32 = 160.0;
const QUICK_LAUNCH_SETTINGS_NAV_ROW_HEIGHT: f32 = 40.0;
const QUICK_LAUNCH_SETTINGS_PAGE_HEADER_HEIGHT: f32 = 48.0;
const DOWNLOAD_FILTER_STRIP_HEIGHT: f32 = 38.0;
const DOWNLOAD_FILTER_SEGMENTED_WIDTH: f32 = 408.0;
const DOWNLOAD_TABLE_HEADER_HEIGHT: f32 = 28.0;
const DOWNLOAD_TASK_ROW_HEIGHT: f32 = 46.0;
const DOWNLOAD_BATCH_TOOLBAR_HEIGHT: f32 = 36.0;
const DOWNLOAD_DETAIL_HEIGHT: f32 = 176.0;
const DOWNLOAD_DETAIL_TAB_HEIGHT: f32 = 30.0;
const DOWNLOAD_DETAIL_LABEL_WIDTH: f32 = 52.0;
const DOWNLOAD_SEARCH_DIAGNOSTIC_DELAY: Duration = Duration::from_millis(250);
const DOWNLOAD_TRANSFERRED_COLUMN_WIDTH: f32 = 90.0;
const DOWNLOAD_SPEED_COLUMN_WIDTH: f32 = 54.0;
const DOWNLOAD_REMAINING_COLUMN_WIDTH: f32 = 36.0;
const DOWNLOAD_ERROR_TOOLTIP_WIDTH: f32 = 320.0;
const DOWNLOAD_ERROR_TOOLTIP_MAX_HEIGHT: f32 = 144.0;
const DOWNLOAD_ERROR_TOOLTIP_SURFACE_MAX_WIDTH: f32 = 340.0;
const DOWNLOAD_ERROR_TOOLTIP_SURFACE_MAX_HEIGHT: f32 = 170.0;
const DOWNLOAD_PROGRESS_COLUMN_WIDTH: f32 = DOWNLOAD_TRANSFERRED_COLUMN_WIDTH
    + DOWNLOAD_SPEED_COLUMN_WIDTH
    + DOWNLOAD_REMAINING_COLUMN_WIDTH;
const DOWNLOAD_STATUS_COLUMN_WIDTH: f32 = 68.0;
const DOWNLOAD_CREATED_COLUMN_WIDTH: f32 = 68.0;

fn quick_find_scroll_target(current: f32, previous_target: f32, maximum: f32, delta: f32) -> f32 {
    if maximum <= 0.0 {
        return 0.0;
    }
    let previous_target = previous_target.clamp(-maximum, 0.0);
    let base = if (previous_target - current).abs() <= QUICK_FIND_SCROLL_TARGET_SNAP_DISTANCE {
        previous_target
    } else {
        current
    };
    (base + delta).clamp(-maximum, 0.0)
}

fn quick_find_scroll_step(current: f32, target: f32) -> (f32, bool) {
    let remaining = target - current;
    if remaining.abs() <= 0.5 {
        (target, true)
    } else {
        (current + remaining * QUICK_FIND_SCROLL_EASING, false)
    }
}

fn exclusion_selection_after_removal(item_count: usize, removed_index: usize) -> Option<usize> {
    (item_count > 1).then(|| removed_index.min(item_count - 2))
}

fn custom_command_selection_after_removal(
    remaining_item_count: usize,
    removed_index: usize,
) -> Option<usize> {
    (remaining_item_count > 0).then(|| removed_index.min(remaining_item_count - 1))
}

#[derive(Clone, Copy)]
struct PendingCustomCommandProgramPicker {
    correlation_id: uuid::Uuid,
    command_index: usize,
    draft_generation: u64,
}

pub(crate) struct MainWindow {
    theme: Theme,
    shell: ShellState,
    clipboard: gpui::WeakEntity<ClipboardApplication>,
    quick_launch: gpui::WeakEntity<QuickLaunchApplication>,
    downloads: gpui::WeakEntity<DownloadApplication>,
    network_settings: Option<Entity<NetworkSettingsApplication>>,
    platform: Option<Rc<PlatformBridge>>,
    native_window_handle: i64,
    system_settings: Rc<RefCell<SystemSettings>>,
    notifications: Entity<NotificationCenter>,
    download_search_input: gpui::Entity<InputState>,
    exclusions_input: gpui::Entity<InputState>,
    custom_command_keyword: gpui::Entity<InputState>,
    custom_command_title: gpui::Entity<InputState>,
    custom_command_program: gpui::Entity<InputState>,
    custom_command_arguments: gpui::Entity<InputState>,
    custom_command_working_directory: gpui::Entity<InputState>,
    quick_launch_settings_page: QuickLaunchSettingsPage,
    quick_find_scroll: ScrollHandle,
    quick_find_scroll_target: f32,
    quick_find_scroll_animation: u64,
    selected_index_exclusion: Option<usize>,
    selected_custom_command: Option<usize>,
    custom_command_draft_generation: u64,
    pending_custom_command_program_picker: Option<PendingCustomCommandProgramPicker>,
    selected_download_task: Option<DownloadTaskId>,
    selected_download_tasks: HashSet<DownloadTaskId>,
    download_selection_anchor: Option<DownloadTaskId>,
    download_filter: DownloadTaskFilter,
    download_sort: DownloadSortState,
    download_list_scroll: UniformListScrollHandle,
    download_search_log_generation: u64,
    download_detail_tab: DownloadDetailTab,
    download_detail_scroll: ScrollHandle,
    requested_download_detail: Option<DownloadTaskId>,
    custom_command_run_as_admin: bool,
    custom_command_run_silently: bool,
    hotkey_focus: FocusHandle,
    recording_hotkey: Option<HotkeyRecordingTarget>,
    _subscriptions: Vec<Subscription>,
}

pub(crate) struct MainWindowShellContext {
    pub(crate) settings: Option<Entity<NetworkSettingsApplication>>,
    pub(crate) platform: Option<Rc<PlatformBridge>>,
    pub(crate) system_settings: Rc<RefCell<SystemSettings>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HotkeyRecordingTarget {
    Clipboard,
    QuickLaunch,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum QuickLaunchSettingsPage {
    #[default]
    General,
    Index,
    Context,
    Commands,
    Diagnostics,
}

#[derive(Debug, Clone, Copy)]
struct QuickLaunchSettingsPageMetadata {
    label: &'static str,
    description: &'static str,
    navigation_id: &'static str,
    content_id: &'static str,
}

impl QuickLaunchSettingsPage {
    const ALL: [Self; 5] = [
        Self::General,
        Self::Index,
        Self::Context,
        Self::Commands,
        Self::Diagnostics,
    ];

    const fn metadata(self) -> QuickLaunchSettingsPageMetadata {
        match self {
            Self::General => QuickLaunchSettingsPageMetadata {
                label: "常规",
                description: "呼出方式与固定键盘操作",
                navigation_id: "quick-launch-settings-nav-general",
                content_id: "quick-launch-settings-page-general",
            },
            Self::Index => QuickLaunchSettingsPageMetadata {
                label: "索引",
                description: "应用目录、文件索引与排除范围",
                navigation_id: "quick-launch-settings-nav-index",
                content_id: "quick-launch-settings-page-index",
            },
            Self::Context => QuickLaunchSettingsPageMetadata {
                label: "上下文",
                description: "资源管理器与文件对话框联动",
                navigation_id: "quick-launch-settings-nav-context",
                content_id: "quick-launch-settings-page-context",
            },
            Self::Commands => QuickLaunchSettingsPageMetadata {
                label: "命令",
                description: "维护关键词驱动的自定义动作",
                navigation_id: "quick-launch-settings-nav-commands",
                content_id: "quick-launch-settings-page-commands",
            },
            Self::Diagnostics => QuickLaunchSettingsPageMetadata {
                label: "诊断",
                description: "历史学习、查询性能与运行日志",
                navigation_id: "quick-launch-settings-nav-diagnostics",
                content_id: "quick-launch-settings-page-diagnostics",
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum DownloadTaskFilter {
    #[default]
    All,
    Downloading,
    Waiting,
    Paused,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum DownloadDetailTab {
    #[default]
    Overview,
    Segments,
    Headers,
    Verification,
    Timeline,
}

impl DownloadDetailTab {
    const ALL: [Self; 5] = [
        Self::Overview,
        Self::Segments,
        Self::Headers,
        Self::Verification,
        Self::Timeline,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::Overview => "概览",
            Self::Segments => "分段",
            Self::Headers => "Header",
            Self::Verification => "校验",
            Self::Timeline => "日志",
        }
    }

    const fn diagnostic_name(self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Segments => "segments",
            Self::Headers => "headers",
            Self::Verification => "verification",
            Self::Timeline => "timeline",
        }
    }
}

struct DownloadRowSelection {
    selected: bool,
    visible_task_ids: Arc<Vec<DownloadTaskId>>,
}

#[derive(Debug, PartialEq)]
struct DownloadRowTransferPresentation {
    fraction: f32,
    transferred: String,
    speed: String,
    remaining: String,
}

fn download_row_transfer_presentation(
    status: DownloadStatus,
    durable_downloaded_bytes: u64,
    durable_total_bytes: Option<u64>,
    live_progress: Option<&DownloadProgress>,
) -> DownloadRowTransferPresentation {
    let use_live_progress = matches!(
        status,
        DownloadStatus::Probing
            | DownloadStatus::Downloading
            | DownloadStatus::Finalizing
            | DownloadStatus::Verifying
    );
    let live_progress = use_live_progress.then_some(live_progress).flatten();
    let downloaded_bytes = live_progress.map_or(durable_downloaded_bytes, |progress| {
        progress.downloaded_bytes
    });
    let total_bytes = live_progress
        .and_then(|progress| progress.total_bytes)
        .or(durable_total_bytes);
    let fraction = total_bytes.filter(|total| *total > 0).map_or(0.0, |total| {
        (downloaded_bytes as f32 / total as f32).clamp(0.0, 1.0)
    });
    let transferred = total_bytes.map_or_else(
        || format_download_bytes(downloaded_bytes),
        |total| format_download_transfer(downloaded_bytes, total),
    );
    let bytes_per_second = if status == DownloadStatus::Downloading {
        live_progress.map_or(0, |progress| progress.bytes_per_second)
    } else {
        0
    };
    let speed = if bytes_per_second == 0 {
        "—".into()
    } else {
        format_download_rate(bytes_per_second)
    };
    let remaining = total_bytes
        .filter(|_| bytes_per_second > 0)
        .map(|total| {
            total
                .saturating_sub(downloaded_bytes)
                .div_ceil(bytes_per_second)
        })
        .filter(|remaining| *remaining > 0)
        .map_or_else(|| "—".into(), format_download_remaining);
    DownloadRowTransferPresentation {
        fraction,
        transferred,
        speed,
        remaining,
    }
}

#[derive(Debug, Default)]
struct DownloadContextCapabilities {
    resume_targets: Vec<DownloadTaskId>,
    pause_targets: Vec<DownloadTaskId>,
    cancel_targets: Vec<DownloadTaskId>,
    can_move_queue: bool,
    queue_top: bool,
    queue_up: bool,
    queue_down: bool,
    queue_bottom: bool,
    can_remove: bool,
    single_artifact: Option<DownloadTaskId>,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct DownloadContextMenuPlan {
    artifact: bool,
    task_window: bool,
    resume: bool,
    pause: bool,
    cancel: bool,
    queue_top: bool,
    queue_up: bool,
    queue_down: bool,
    queue_bottom: bool,
    remove: bool,
}

struct DownloadDetailView<'a> {
    task: &'a DownloadEngineTask,
    detail: Option<&'a DownloadTaskDetail>,
    detail_error: Option<&'a str>,
    progress: Option<&'a DownloadProgress>,
    active_tab: DownloadDetailTab,
    scroll: ScrollHandle,
    main_window: Entity<MainWindow>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum DownloadSortKey {
    #[default]
    Queue,
    Name,
    Size,
    Progress,
    Status,
    Created,
}

impl DownloadSortKey {
    const ALL: [Self; 6] = [
        Self::Queue,
        Self::Name,
        Self::Size,
        Self::Progress,
        Self::Status,
        Self::Created,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::Queue => "队列顺序",
            Self::Name => "名称",
            Self::Size => "大小",
            Self::Progress => "进度",
            Self::Status => "状态",
            Self::Created => "添加时间",
        }
    }

    const fn diagnostic_name(self) -> &'static str {
        match self {
            Self::Queue => "queue",
            Self::Name => "name",
            Self::Size => "size",
            Self::Progress => "progress",
            Self::Status => "status",
            Self::Created => "created",
        }
    }

    const fn default_descending(self) -> bool {
        matches!(self, Self::Size | Self::Progress | Self::Created)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct DownloadSortState {
    key: DownloadSortKey,
    descending: bool,
}

fn download_task_name(task: &DownloadEngineTask) -> &str {
    task.final_path
        .as_ref()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .or(task.task.request().file_name.as_deref())
        .or_else(|| {
            task.probe
                .as_ref()
                .and_then(|probe| probe.file_name.as_deref())
        })
        .unwrap_or("正在解析文件名")
}

fn download_failure_summary(error: &DownloadFault) -> &'static str {
    let message = error.message.to_ascii_lowercase();
    if error.code == DownloadFaultCode::Protocol {
        if message.contains("resource changed")
            || message.contains("validator changed")
            || message.contains("etag changed")
        {
            return "远程文件已变化，请重新下载";
        }
        if message.contains("ignored the requested byte range")
            || message.contains("range") && message.contains("http 200")
        {
            return "服务器不支持断点续传，请重新开始";
        }
    }
    match error.code {
        DownloadFaultCode::InvalidRequest => "下载请求无效，请检查地址",
        DownloadFaultCode::NotFound => "下载任务或文件不存在",
        DownloadFaultCode::InvalidState => "当前状态无法执行此操作",
        DownloadFaultCode::Storage => "无法写入下载文件，请检查保存位置",
        DownloadFaultCode::Network => "网络连接失败，请检查网络后重试",
        DownloadFaultCode::Protocol => "服务器响应异常，请重试",
        DownloadFaultCode::Permission => "没有权限写入保存位置",
        DownloadFaultCode::DiskFull => "磁盘空间不足",
        DownloadFaultCode::FileInUse => "目标文件正在被其他程序使用",
        DownloadFaultCode::Cancelled => "下载已取消",
        DownloadFaultCode::Unavailable => "下载服务暂时不可用，请稍后重试",
        DownloadFaultCode::Internal => "下载发生内部错误，请重试",
    }
}

fn download_task_matches_search(task: &DownloadEngineTask, query: &str) -> bool {
    let query = query.trim();
    if query.is_empty() {
        return true;
    }
    let query = query.to_lowercase();
    download_task_name(task).to_lowercase().contains(&query)
        || task
            .task
            .request()
            .source_url
            .to_lowercase()
            .contains(&query)
        || task
            .final_path
            .as_ref()
            .is_some_and(|path| path.to_string_lossy().to_lowercase().contains(&query))
}

fn download_status_sort_rank(status: DownloadStatus) -> u8 {
    match status {
        DownloadStatus::Downloading => 0,
        DownloadStatus::Probing => 1,
        DownloadStatus::Finalizing => 2,
        DownloadStatus::Verifying => 3,
        DownloadStatus::Waiting => 4,
        DownloadStatus::Created => 5,
        DownloadStatus::Paused => 6,
        DownloadStatus::Failed => 7,
        DownloadStatus::Completed => 8,
        DownloadStatus::Cancelled => 9,
    }
}

fn download_tasks_for_view(
    tasks: &[DownloadEngineTask],
    filter: DownloadTaskFilter,
    query: &str,
    sort: DownloadSortState,
) -> Vec<DownloadEngineTask> {
    let mut visible = tasks
        .iter()
        .filter(|task| filter.matches(task.task.status()))
        .filter(|task| download_task_matches_search(task, query))
        .cloned()
        .collect::<Vec<_>>();
    if sort.key == DownloadSortKey::Queue {
        return visible;
    }
    visible.sort_by(|left, right| {
        let ordering = match sort.key {
            DownloadSortKey::Queue => std::cmp::Ordering::Equal,
            DownloadSortKey::Name => download_task_name(left)
                .to_lowercase()
                .cmp(&download_task_name(right).to_lowercase()),
            DownloadSortKey::Size => left.total_bytes.cmp(&right.total_bytes),
            DownloadSortKey::Progress => {
                let left_progress = left
                    .total_bytes
                    .filter(|total| *total > 0)
                    .map_or(0.0, |total| left.downloaded_bytes as f64 / total as f64);
                let right_progress = right
                    .total_bytes
                    .filter(|total| *total > 0)
                    .map_or(0.0, |total| right.downloaded_bytes as f64 / total as f64);
                left_progress.total_cmp(&right_progress)
            }
            DownloadSortKey::Status => download_status_sort_rank(left.task.status())
                .cmp(&download_status_sort_rank(right.task.status())),
            DownloadSortKey::Created => left
                .task
                .created_at_unix_ms()
                .cmp(&right.task.created_at_unix_ms()),
        };
        if sort.descending {
            ordering.reverse()
        } else {
            ordering
        }
    });
    visible
}

fn set_download_sort(state: &mut DownloadSortState, key: DownloadSortKey, toggle: bool) -> bool {
    let next = if state.key == key && toggle && key != DownloadSortKey::Queue {
        DownloadSortState {
            key,
            descending: !state.descending,
        }
    } else {
        DownloadSortState {
            key,
            descending: key.default_descending(),
        }
    };
    if *state == next {
        return false;
    }
    *state = next;
    info!(
        event = "download.list.sort_changed",
        sort_key = key.diagnostic_name(),
        descending = state.descending,
        "download task presentation sort changed"
    );
    true
}

impl DownloadTaskFilter {
    const ALL: [Self; 6] = [
        Self::All,
        Self::Downloading,
        Self::Waiting,
        Self::Paused,
        Self::Completed,
        Self::Failed,
    ];

    const fn id(self) -> &'static str {
        match self {
            Self::All => "download-filter-all",
            Self::Downloading => "download-filter-downloading",
            Self::Waiting => "download-filter-waiting",
            Self::Paused => "download-filter-paused",
            Self::Completed => "download-filter-completed",
            Self::Failed => "download-filter-failed",
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::All => "全部",
            Self::Downloading => "下载中",
            Self::Waiting => "等待",
            Self::Paused => "暂停",
            Self::Completed => "已完成",
            Self::Failed => "失败",
        }
    }

    const fn diagnostic_name(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Downloading => "downloading",
            Self::Waiting => "waiting",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    const fn empty_title(self) -> &'static str {
        match self {
            Self::All => "还没有下载任务",
            Self::Downloading => "暂无正在下载的任务",
            Self::Waiting => "暂无等待任务",
            Self::Paused => "暂无暂停任务",
            Self::Completed => "暂无已完成任务",
            Self::Failed => "暂无失败任务",
        }
    }

    const fn matches(self, status: DownloadStatus) -> bool {
        match self {
            Self::All => true,
            Self::Downloading => matches!(
                status,
                DownloadStatus::Probing
                    | DownloadStatus::Downloading
                    | DownloadStatus::Finalizing
                    | DownloadStatus::Verifying
            ),
            Self::Waiting => {
                matches!(status, DownloadStatus::Created | DownloadStatus::Waiting)
            }
            Self::Paused => matches!(status, DownloadStatus::Paused),
            Self::Completed => matches!(status, DownloadStatus::Completed),
            Self::Failed => matches!(status, DownloadStatus::Failed),
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
struct DownloadFilterCounts {
    all: usize,
    downloading: usize,
    waiting: usize,
    paused: usize,
    completed: usize,
    failed: usize,
}

impl DownloadFilterCounts {
    const fn get(&self, filter: DownloadTaskFilter) -> usize {
        match filter {
            DownloadTaskFilter::All => self.all,
            DownloadTaskFilter::Downloading => self.downloading,
            DownloadTaskFilter::Waiting => self.waiting,
            DownloadTaskFilter::Paused => self.paused,
            DownloadTaskFilter::Completed => self.completed,
            DownloadTaskFilter::Failed => self.failed,
        }
    }
}

fn download_filter_counts(
    statuses: impl IntoIterator<Item = DownloadStatus>,
) -> DownloadFilterCounts {
    let mut counts = DownloadFilterCounts::default();
    for status in statuses {
        counts.all += 1;
        if DownloadTaskFilter::Downloading.matches(status) {
            counts.downloading += 1;
        }
        if DownloadTaskFilter::Waiting.matches(status) {
            counts.waiting += 1;
        }
        if DownloadTaskFilter::Paused.matches(status) {
            counts.paused += 1;
        }
        if DownloadTaskFilter::Completed.matches(status) {
            counts.completed += 1;
        }
        if DownloadTaskFilter::Failed.matches(status) {
            counts.failed += 1;
        }
    }
    counts
}

fn reconcile_download_selection<T, Id: Copy + Eq + Hash>(
    primary: Option<Id>,
    selected: &mut HashSet<Id>,
    visible: &[T],
    id: impl Fn(&T) -> Id,
) -> Option<Id> {
    selected.retain(|selected_id| visible.iter().any(|item| id(item) == *selected_id));
    let primary = primary
        .filter(|primary_id| visible.iter().any(|item| id(item) == *primary_id))
        .or_else(|| {
            visible
                .iter()
                .map(&id)
                .find(|visible_id| selected.contains(visible_id))
        });
    if let Some(primary_id) = primary {
        selected.insert(primary_id);
    }
    primary
}

fn select_download_filter(
    current: &mut DownloadTaskFilter,
    next: DownloadTaskFilter,
    visible_count: usize,
    total_count: usize,
) -> bool {
    if *current == next {
        return false;
    }
    *current = next;
    info!(
        filter = next.diagnostic_name(),
        visible_count, total_count, "download task filter changed"
    );
    true
}

fn update_download_selection(
    selected: &mut HashSet<DownloadTaskId>,
    primary: &mut Option<DownloadTaskId>,
    anchor: &mut Option<DownloadTaskId>,
    visible: &[DownloadTaskId],
    clicked: DownloadTaskId,
    control: bool,
    shift: bool,
) {
    if shift {
        *primary = Some(clicked);
        let anchor_id = anchor.unwrap_or(clicked);
        let start = visible
            .iter()
            .position(|candidate| *candidate == anchor_id)
            .unwrap_or(0);
        let end = visible
            .iter()
            .position(|candidate| *candidate == clicked)
            .unwrap_or(start);
        selected.clear();
        selected.extend(visible[start.min(end)..=start.max(end)].iter().copied());
    } else if control {
        if selected.remove(&clicked) {
            *primary = visible
                .iter()
                .copied()
                .find(|task_id| selected.contains(task_id));
        } else {
            selected.insert(clicked);
            *primary = Some(clicked);
        }
        *anchor = Some(clicked);
    } else {
        *primary = Some(clicked);
        selected.clear();
        selected.insert(clicked);
        *anchor = Some(clicked);
    }
}

fn prepare_download_context_selection(
    selected: &mut HashSet<DownloadTaskId>,
    primary: &mut Option<DownloadTaskId>,
    anchor: &mut Option<DownloadTaskId>,
    visible: &[DownloadTaskId],
    clicked: DownloadTaskId,
) -> Vec<DownloadTaskId> {
    if !selected.contains(&clicked) {
        selected.clear();
        selected.insert(clicked);
        *primary = Some(clicked);
        *anchor = Some(clicked);
    }
    visible
        .iter()
        .filter(|task_id| selected.contains(task_id))
        .copied()
        .collect()
}

fn download_context_capabilities(
    tasks: &[DownloadEngineTask],
    selected: &[DownloadTaskId],
) -> DownloadContextCapabilities {
    let selected_tasks = selected
        .iter()
        .filter_map(|task_id| tasks.iter().find(|task| task.task.id() == *task_id))
        .collect::<Vec<_>>();
    DownloadContextCapabilities {
        resume_targets: selected_tasks
            .iter()
            .filter(|task| {
                matches!(
                    task.task.status(),
                    DownloadStatus::Created | DownloadStatus::Paused | DownloadStatus::Failed
                )
            })
            .map(|task| task.task.id())
            .collect(),
        pause_targets: selected_tasks
            .iter()
            .filter(|task| {
                matches!(
                    task.task.status(),
                    DownloadStatus::Created
                        | DownloadStatus::Probing
                        | DownloadStatus::Waiting
                        | DownloadStatus::Downloading
                )
            })
            .map(|task| task.task.id())
            .collect(),
        cancel_targets: selected_tasks
            .iter()
            .filter(|task| {
                matches!(
                    task.task.status(),
                    DownloadStatus::Created
                        | DownloadStatus::Probing
                        | DownloadStatus::Waiting
                        | DownloadStatus::Downloading
                        | DownloadStatus::Paused
                )
            })
            .map(|task| task.task.id())
            .collect(),
        can_move_queue: !selected_tasks.is_empty()
            && selected_tasks.iter().all(|task| {
                task.queue_eligible
                    && matches!(
                        task.task.status(),
                        DownloadStatus::Created
                            | DownloadStatus::Probing
                            | DownloadStatus::Waiting
                            | DownloadStatus::Downloading
                            | DownloadStatus::Paused
                    )
            }),
        queue_top: false,
        queue_up: false,
        queue_down: false,
        queue_bottom: false,
        can_remove: !selected_tasks.is_empty()
            && selected_tasks.iter().all(|task| {
                !matches!(
                    task.task.status(),
                    DownloadStatus::Probing
                        | DownloadStatus::Waiting
                        | DownloadStatus::Downloading
                        | DownloadStatus::Finalizing
                        | DownloadStatus::Verifying
                )
            }),
        single_artifact: (selected_tasks.len() == 1)
            .then(|| selected_tasks[0])
            .filter(|task| task.task.status() == DownloadStatus::Completed)
            .and_then(|task| task.final_path.as_ref().map(|_| task.task.id())),
    }
}

fn download_queue_placement_capabilities(
    queue_order: &[DownloadTaskId],
    selected: &[DownloadTaskId],
) -> (bool, bool, bool, bool) {
    if queue_order.is_empty() || selected.is_empty() {
        return (false, false, false, false);
    }
    let selected = selected.iter().copied().collect::<HashSet<_>>();
    let indices = queue_order
        .iter()
        .enumerate()
        .filter_map(|(index, task_id)| selected.contains(task_id).then_some(index))
        .collect::<Vec<_>>();
    if indices.len() != selected.len() {
        return (false, false, false, false);
    }
    let first = indices.iter().copied().min().unwrap_or_default();
    let last = indices.iter().copied().max().unwrap_or_default();
    let can_move_up = first > 0;
    let can_move_down = last + 1 < queue_order.len();
    (can_move_up, can_move_up, can_move_down, can_move_down)
}

fn download_context_menu_plan(
    capabilities: &DownloadContextCapabilities,
    selected_count: usize,
) -> DownloadContextMenuPlan {
    let artifact = capabilities.single_artifact.is_some();
    DownloadContextMenuPlan {
        artifact,
        task_window: selected_count == 1,
        resume: !capabilities.resume_targets.is_empty(),
        pause: !capabilities.pause_targets.is_empty(),
        cancel: !capabilities.cancel_targets.is_empty(),
        queue_top: capabilities.can_move_queue && capabilities.queue_top,
        queue_up: capabilities.can_move_queue && capabilities.queue_up,
        queue_down: capabilities.can_move_queue && capabilities.queue_down,
        queue_bottom: capabilities.can_move_queue && capabilities.queue_bottom,
        remove: selected_count > 0 && capabilities.can_remove,
    }
}

#[derive(Default)]
struct ClipboardOverview {
    total: usize,
    text: usize,
    images: usize,
    files: usize,
}

fn notification_transition(delta: f32, exiting: bool) -> (f32, f32) {
    if exiting {
        (1.0 - delta, -4.0 * delta)
    } else {
        (delta, -6.0 * (1.0 - delta))
    }
}

fn grouped_count(value: u64) -> String {
    let digits = value.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, character) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(character);
    }
    grouped
}

fn format_query_elapsed(elapsed_us: Option<u64>) -> String {
    match elapsed_us {
        None => "尚无样本".into(),
        Some(elapsed_us) if elapsed_us < 1_000 => format!("{elapsed_us} µs"),
        Some(elapsed_us) => format!("{:.2} ms", elapsed_us as f64 / 1_000.0),
    }
}

fn file_index_summary(status: Option<&IndexStatus>) -> (String, &'static str) {
    let Some(status) = status else {
        return ("等待 NxTools.Indexer 连接".into(), "OFFLINE");
    };
    if let Some(error) = status.last_error.as_deref() {
        return (
            if status.indexed_items == 0 {
                format!("MFT 索引不可用：{error}")
            } else {
                format!("已索引 {} 项；部分卷失败：{error}", status.indexed_items)
            },
            if status.indexed_items == 0 {
                "ERROR"
            } else {
                "PARTIAL"
            },
        );
    }
    if status.phase == IndexPhase::Ready {
        return (
            format!(
                "已建立 {} 个 NTFS 卷快照 · {} 项",
                status.drives.len(),
                grouped_count(status.indexed_items)
            ),
            "READY",
        );
    }
    if status.indexed_items > 0 {
        return (
            format!(
                "正在更新新快照 · 旧索引 {} 项仍可搜索",
                grouped_count(status.indexed_items)
            ),
            "UPDATING",
        );
    }
    let drives = status
        .drives
        .iter()
        .map(|drive| drive.mount_point.trim_end_matches('\\'))
        .collect::<Vec<_>>()
        .join(" · ");
    if drives.is_empty() {
        ("未发现可建立 MFT 索引的 NTFS 卷".into(), "EMPTY")
    } else {
        (
            format!("已发现 {} 个 NTFS 卷：{drives}", status.drives.len()),
            "VOLUMES",
        )
    }
}

fn same_volume(left: &str, right: &str) -> bool {
    left.trim_end_matches(['\\', '/'])
        .eq_ignore_ascii_case(right.trim_end_matches(['\\', '/']))
}

fn volume_is_selected(configured: &[String], mount_point: &str) -> bool {
    configured.is_empty()
        || configured
            .iter()
            .any(|candidate| same_volume(candidate, mount_point))
}

fn volume_scope_selector_key(mount_point: &str) -> String {
    let key = mount_point
        .trim_end_matches(['\\', '/'])
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    key.trim_matches('-').to_owned()
}

fn toggle_indexed_volume_selection(
    configured: &[String],
    available: &[String],
    mount_point: &str,
) -> Vec<String> {
    let mut selected = if configured.is_empty() {
        available.to_vec()
    } else {
        configured.to_vec()
    };
    if let Some(index) = selected
        .iter()
        .position(|candidate| same_volume(candidate, mount_point))
    {
        let selected_available_count = available
            .iter()
            .filter(|available_mount| {
                selected
                    .iter()
                    .any(|candidate| same_volume(candidate, available_mount))
            })
            .count();
        if selected_available_count == 1 {
            return configured.to_vec();
        }
        selected.remove(index);
    } else {
        selected.push(mount_point.to_owned());
    }
    let exactly_all_available = !available.is_empty()
        && available.iter().all(|mount| {
            selected
                .iter()
                .any(|candidate| same_volume(candidate, mount))
        });
    if exactly_all_available {
        Vec::new()
    } else {
        selected
    }
}

impl MainWindow {
    pub(crate) fn new(
        clipboard: gpui::WeakEntity<ClipboardApplication>,
        quick_launch: gpui::WeakEntity<QuickLaunchApplication>,
        downloads: gpui::WeakEntity<DownloadApplication>,
        shell_context: MainWindowShellContext,
        notifications: Entity<NotificationCenter>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let excluded_processes = clipboard
            .upgrade()
            .map(|application| {
                application
                    .read(cx)
                    .clipboard_settings()
                    .excluded_processes
                    .join("; ")
            })
            .unwrap_or_default();
        let exclusions_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(excluded_processes)
                .placeholder("KeePass.exe; Bitwarden.exe")
        });
        let download_search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("搜索文件名或地址"));
        let initial_custom_command = quick_launch.upgrade().and_then(|application| {
            application
                .read(cx)
                .settings()
                .custom_commands
                .first()
                .cloned()
        });
        let initial_keyword = initial_custom_command
            .as_ref()
            .map_or_else(String::new, |command| command.keyword.clone());
        let initial_title = initial_custom_command
            .as_ref()
            .map_or_else(String::new, |command| command.title.clone());
        let initial_program = initial_custom_command
            .as_ref()
            .map_or_else(String::new, |command| command.program.clone());
        let initial_arguments = initial_custom_command
            .as_ref()
            .map_or_else(String::new, |command| command.argument_template.clone());
        let initial_working_directory = initial_custom_command
            .as_ref()
            .map_or_else(String::new, |command| command.working_directory.clone());
        let custom_command_keyword = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(initial_keyword.clone())
                .placeholder("关键词")
        });
        let custom_command_title = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(initial_title.clone())
                .placeholder("显示名称")
        });
        let custom_command_program = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(initial_program.clone())
                .placeholder(r"C:\Tools\tool.exe")
        });
        let custom_command_arguments = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(initial_arguments.clone())
                .placeholder("--query %s 或 --name %s1")
        });
        let custom_command_working_directory = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(initial_working_directory.clone())
                .placeholder("可选工作目录")
        });
        let exclusions_subscription = cx.subscribe_in(
            &exclusions_input,
            window,
            |this, input, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Blur | InputEvent::PressEnter { .. }) {
                    let value = input.read(cx).value().to_string();
                    let _ = this.clipboard.update(cx, |application, _| {
                        application.set_clipboard_excluded_processes(&value);
                    });
                    cx.notify();
                }
            },
        );
        let download_search_subscription = cx.subscribe_in(
            &download_search_input,
            window,
            |this, input, event: &InputEvent, _window, cx| {
                if !matches!(event, InputEvent::Change) {
                    return;
                }
                let query = input.read(cx).value().trim().to_owned();
                this.download_list_scroll
                    .scroll_to_item(0, ScrollStrategy::Top);
                this.download_search_log_generation =
                    this.download_search_log_generation.wrapping_add(1).max(1);
                let generation = this.download_search_log_generation;
                cx.notify();
                let background = cx.background_executor().clone();
                cx.spawn(async move |this, cx| {
                    background.timer(DOWNLOAD_SEARCH_DIAGNOSTIC_DELAY).await;
                    let _ = this.update(cx, |this, cx| {
                        if this.download_search_log_generation != generation
                            || this.download_search_input.read(cx).value().trim() != query
                        {
                            return;
                        }
                        let result_count = this.downloads.upgrade().map_or(0, |application| {
                            application
                                .read(cx)
                                .tasks()
                                .iter()
                                .filter(|task| this.download_filter.matches(task.task.status()))
                                .filter(|task| download_task_matches_search(task, &query))
                                .count()
                        });
                        info!(
                            event = "download.list.search_changed",
                            query_length = query.chars().count(),
                            result_count,
                            "download task search changed"
                        );
                    });
                })
                .detach();
            },
        );
        let clipboard_subscription = clipboard
            .upgrade()
            .map(|application| cx.observe(&application, |_, _, cx| cx.notify()));
        let quick_launch_subscription = quick_launch
            .upgrade()
            .map(|application| cx.observe(&application, |_, _, cx| cx.notify()));
        let downloads_subscription = downloads
            .upgrade()
            .map(|application| cx.observe(&application, |_, _, cx| cx.notify()));
        let quick_launch_event_subscription = quick_launch.upgrade().map(|application| {
            cx.subscribe_in(
                &application,
                window,
                |this, _application, event: &QuickLaunchApplicationEvent, window, cx| {
                    let QuickLaunchApplicationEvent::CustomCommandProgramPicked {
                        correlation_id,
                        index,
                        path,
                    } = event;
                    let matches_current_draft = this
                        .pending_custom_command_program_picker
                        .is_some_and(|pending| {
                            pending.correlation_id == *correlation_id
                                && pending.command_index == *index
                                && pending.draft_generation == this.custom_command_draft_generation
                        })
                        && this.selected_custom_command == Some(*index);
                    if matches_current_draft {
                        let generation = this.custom_command_draft_generation;
                        this.pending_custom_command_program_picker = None;
                        let index = *index;
                        let path = path.clone();
                        let input = this.custom_command_program.clone();
                        let view = cx.entity().downgrade();
                        let source_window = window.window_handle();
                        cx.defer(move |cx| {
                            let still_selected = view
                                .update(cx, |this, _| {
                                    this.selected_custom_command == Some(index)
                                        && this.custom_command_draft_generation == generation
                                })
                                .unwrap_or(false);
                            if !still_selected {
                                return;
                            }
                            cx.defer(move |cx| {
                                let _ = source_window.update(cx, |_, window, cx| {
                                    input.update(cx, |input, cx| {
                                        input.set_value(path, window, cx);
                                    });
                                });
                            });
                        });
                    }
                },
            )
        });
        let notification_subscription = cx.observe(&notifications, |_, _, cx| cx.notify());
        let hotkey_focus = cx.focus_handle();
        let hotkey_focus_subscription =
            cx.on_focus_out(&hotkey_focus, window, |this, _event, _window, cx| {
                if this.recording_hotkey.take().is_some() {
                    this.defer_all_hotkey_recording(false, cx);
                    cx.notify();
                }
            });
        let mut subscriptions = vec![exclusions_subscription, download_search_subscription];
        subscriptions.extend(clipboard_subscription);
        subscriptions.extend(quick_launch_subscription);
        subscriptions.extend(downloads_subscription);
        subscriptions.extend(quick_launch_event_subscription);
        subscriptions.push(notification_subscription);
        subscriptions.push(hotkey_focus_subscription);
        Self {
            theme: Theme::zed_dark(),
            shell: ShellState::default(),
            clipboard,
            quick_launch,
            downloads,
            network_settings: shell_context.settings,
            platform: shell_context.platform,
            native_window_handle: crate::ui::native_window_handle(window).unwrap_or_default(),
            system_settings: shell_context.system_settings,
            notifications,
            download_search_input,
            exclusions_input,
            custom_command_keyword,
            custom_command_title,
            custom_command_program,
            custom_command_arguments,
            custom_command_working_directory,
            quick_launch_settings_page: QuickLaunchSettingsPage::default(),
            quick_find_scroll: ScrollHandle::new(),
            quick_find_scroll_target: 0.0,
            quick_find_scroll_animation: 0,
            selected_index_exclusion: None,
            selected_custom_command: initial_custom_command.as_ref().map(|_| 0),
            custom_command_draft_generation: 0,
            pending_custom_command_program_picker: None,
            selected_download_task: None,
            selected_download_tasks: HashSet::new(),
            download_selection_anchor: None,
            download_filter: DownloadTaskFilter::default(),
            download_sort: DownloadSortState::default(),
            download_list_scroll: UniformListScrollHandle::new(),
            download_search_log_generation: 0,
            download_detail_tab: DownloadDetailTab::default(),
            download_detail_scroll: ScrollHandle::new(),
            requested_download_detail: None,
            custom_command_run_as_admin: initial_custom_command
                .as_ref()
                .is_some_and(|command| command.run_as_admin),
            custom_command_run_silently: initial_custom_command
                .as_ref()
                .is_some_and(|command| command.run_silently),
            hotkey_focus,
            recording_hotkey: None,
            _subscriptions: subscriptions,
        }
    }

    fn clipboard_overview(&self, cx: &App) -> ClipboardOverview {
        let Ok(application) = self.clipboard.upgrade().ok_or(()) else {
            return ClipboardOverview::default();
        };
        let Ok(ClipboardHistoryCounts {
            total,
            text,
            images,
            files,
        }) = application.read(cx).clipboard_counts()
        else {
            return ClipboardOverview::default();
        };
        ClipboardOverview {
            total,
            text,
            images,
            files,
        }
    }

    fn clipboard_settings(&self, cx: &App) -> ClipboardSettings {
        self.clipboard
            .upgrade()
            .map(|application| application.read(cx).clipboard_settings())
            .unwrap_or_default()
    }

    fn select_custom_command_draft(
        &mut self,
        index: usize,
        command: CustomCommandSettings,
        source_window: AnyWindowHandle,
        cx: &mut Context<Self>,
    ) {
        self.selected_custom_command = Some(index);
        self.custom_command_draft_generation = self.custom_command_draft_generation.wrapping_add(1);
        self.pending_custom_command_program_picker = None;
        self.custom_command_run_as_admin = command.run_as_admin;
        self.custom_command_run_silently = command.run_silently;
        let fields = [
            (&self.custom_command_keyword, command.keyword),
            (&self.custom_command_title, command.title),
            (&self.custom_command_program, command.program),
            (&self.custom_command_arguments, command.argument_template),
            (
                &self.custom_command_working_directory,
                command.working_directory,
            ),
        ]
        .map(|(input, value)| (input.clone(), value));
        cx.defer(move |cx| {
            let _ = source_window.update(cx, |_, window, cx| {
                for (input, value) in fields {
                    input.update(cx, |state, cx| state.set_value(value, window, cx));
                }
            });
        });
        cx.notify();
    }

    fn save_selected_custom_command(&mut self, cx: &mut Context<Self>) {
        let Some(index) = self.selected_custom_command else {
            return;
        };
        let enabled = self
            .quick_launch
            .upgrade()
            .and_then(|application| {
                application
                    .read(cx)
                    .settings()
                    .custom_commands
                    .get(index)
                    .map(|command| command.enabled)
            })
            .unwrap_or(false);
        let command = CustomCommandSettings {
            enabled,
            keyword: self.custom_command_keyword.read(cx).value().to_string(),
            title: self.custom_command_title.read(cx).value().to_string(),
            program: self.custom_command_program.read(cx).value().to_string(),
            argument_template: self.custom_command_arguments.read(cx).value().to_string(),
            working_directory: self
                .custom_command_working_directory
                .read(cx)
                .value()
                .to_string(),
            run_as_admin: self.custom_command_run_as_admin,
            run_silently: self.custom_command_run_silently,
        };
        let quick_launch = self.quick_launch.clone();
        cx.defer(move |cx| {
            let _ = quick_launch.update(cx, |application, cx| {
                application.update_custom_command(index, command, cx);
            });
        });
        cx.notify();
    }

    fn smooth_scroll_quick_find(&mut self, delta: f32, cx: &mut Context<Self>) -> bool {
        let maximum = self.quick_find_scroll.max_offset().height.as_f32().max(0.0);
        if maximum <= 0.0 || delta == 0.0 {
            return false;
        }
        let current = self.quick_find_scroll.offset().y.as_f32();
        let target =
            quick_find_scroll_target(current, self.quick_find_scroll_target, maximum, delta);
        self.animate_quick_find_scroll_to(target, cx);
        true
    }

    fn animate_quick_find_scroll_to(&mut self, target: f32, cx: &mut Context<Self>) {
        self.quick_find_scroll_target = target;
        self.quick_find_scroll_animation = self.quick_find_scroll_animation.wrapping_add(1).max(1);
        let generation = self.quick_find_scroll_animation;
        let scroll = self.quick_find_scroll.clone();
        let background = cx.background_executor().clone();
        cx.spawn(async move |this, cx| {
            for _ in 0..12 {
                background.timer(QUICK_FIND_SCROLL_FRAME_DURATION).await;
                let done = this
                    .update(cx, |this, cx| {
                        if this.quick_find_scroll_animation != generation {
                            return true;
                        }
                        let current = scroll.offset().y.as_f32();
                        let (next, done) =
                            quick_find_scroll_step(current, this.quick_find_scroll_target);
                        scroll.set_offset(point(px(0.0), px(next)));
                        cx.notify();
                        done
                    })
                    .unwrap_or(true);
                if done {
                    break;
                }
            }
            let _ = this.update(cx, |this, cx| {
                if this.quick_find_scroll_animation == generation {
                    scroll.set_offset(point(px(0.0), px(this.quick_find_scroll_target)));
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn select_quick_launch_settings_page(
        &mut self,
        page: QuickLaunchSettingsPage,
        cx: &mut Context<Self>,
    ) {
        if self.quick_launch_settings_page == page {
            return;
        }
        self.quick_launch_settings_page = page;
        self.selected_index_exclusion = None;
        self.quick_find_scroll_animation = self.quick_find_scroll_animation.wrapping_add(1).max(1);
        self.quick_find_scroll_target = 0.0;
        self.quick_find_scroll.set_offset(point(px(0.0), px(0.0)));
        cx.notify();
    }

    fn handle_index_exclusion_keyboard(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.shell.active_module() != MainModule::QuickFind
            || self.quick_launch_settings_page != QuickLaunchSettingsPage::Index
            || event.keystroke.modifiers.modified()
        {
            return false;
        }
        let Some(application) = self.quick_launch.upgrade() else {
            self.selected_index_exclusion = None;
            return false;
        };
        let paths = application.read(cx).settings().excluded_paths.clone();
        if paths.is_empty() {
            self.selected_index_exclusion = None;
            return false;
        }
        let key = event.keystroke.key.to_ascii_lowercase();
        match key.as_str() {
            "up" => {
                self.selected_index_exclusion = Some(
                    self.selected_index_exclusion
                        .unwrap_or_default()
                        .saturating_sub(1),
                );
            }
            "down" => {
                self.selected_index_exclusion = Some(
                    self.selected_index_exclusion
                        .map_or(0, |index| (index + 1).min(paths.len() - 1)),
                );
            }
            "escape" => {
                if self.selected_index_exclusion.take().is_none() {
                    return false;
                }
            }
            "delete" | "backspace" => {
                let Some(index) = self
                    .selected_index_exclusion
                    .filter(|index| *index < paths.len())
                else {
                    return false;
                };
                let path = paths[index].clone();
                self.selected_index_exclusion =
                    exclusion_selection_after_removal(paths.len(), index);
                let quick_launch = self.quick_launch.clone();
                cx.defer(move |cx| {
                    let _ = quick_launch.update(cx, |application, cx| {
                        application.remove_excluded_path(&path, cx);
                    });
                });
            }
            _ => return false,
        }
        cx.stop_propagation();
        cx.notify();
        true
    }

    fn render_notifications(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let notifications = self.notifications.read(cx).items().to_vec();
        let mut stack = div()
            .id("notification-stack")
            .debug_selector(|| "notification-stack".into())
            .absolute()
            .right(px(SPACING_12))
            .bottom(px(SPACING_12))
            .w(px(NOTIFICATION_CARD_WIDTH))
            .flex()
            .flex_col()
            .gap(px(SPACING_6));
        for notification in notifications {
            let accent = match notification.level {
                NotificationLevel::Progress | NotificationLevel::Info => theme.accent,
                NotificationLevel::Success => theme.success,
                NotificationLevel::Error => theme.danger,
            };
            let status_icon = match notification.level {
                NotificationLevel::Progress => MaterialIcon::ProgressActivity,
                NotificationLevel::Info => MaterialIcon::Info,
                NotificationLevel::Success => MaterialIcon::Check,
                NotificationLevel::Error => MaterialIcon::Cancel,
            };
            let notification_id = notification.id.clone();
            let notification_generation = notification.generation();
            let notification_exiting = notification.exiting;
            let dismiss_id = notification.id.clone();
            let notification_center = self.notifications.clone();
            let title_selector = format!("notification-title-{}", notification_id);
            let message_selector = format!("notification-message-{}", notification_id);
            let mut body = div()
                .min_w_0()
                .flex_1()
                .pr(px(SPACING_24))
                .flex()
                .flex_col()
                .gap(px(SPACING_2))
                .child(
                    div()
                        .debug_selector(move || title_selector.clone())
                        .text_size(px(TEXT_BODY))
                        .line_height(px(LINE_HEIGHT_BODY))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.text)
                        .child(notification.title),
                )
                .child(
                    div()
                        .debug_selector(move || message_selector.clone())
                        .text_size(px(TEXT_LABEL))
                        .line_height(px(LINE_HEIGHT_LABEL))
                        .text_color(theme.text_secondary)
                        .child(notification.message),
                );
            for (detail_index, detail) in notification.details.into_iter().enumerate() {
                let detail_tooltip = detail.value.clone();
                let detail_id = gpui::SharedString::from(format!(
                    "notification-detail-{}-{detail_index}",
                    notification_id
                ));
                body = body.child(
                    div()
                        .pt(px(SPACING_4))
                        .border_t_1()
                        .border_color(theme.border_subtle)
                        .flex()
                        .flex_col()
                        .gap(px(SPACING_2))
                        .child(
                            div()
                                .text_size(px(TEXT_META))
                                .line_height(px(LINE_HEIGHT_META))
                                .text_color(theme.text_muted)
                                .child(detail.label),
                        )
                        .child(
                            div()
                                .id(detail_id)
                                .w_full()
                                .whitespace_normal()
                                .line_clamp(1)
                                .font_family(UI_MONO_FONT_FAMILY)
                                .text_size(px(TEXT_LABEL))
                                .line_height(px(LINE_HEIGHT_LABEL))
                                .text_color(theme.text_secondary)
                                .tooltip(move |window, cx| {
                                    Tooltip::new(detail_tooltip.clone()).build(window, cx)
                                })
                                .child(detail.value),
                        ),
                );
            }
            if let Some(action) = notification.action {
                let action_id = notification_id.clone();
                let action_dismiss_id = notification_id.clone();
                let action_center = self.notifications.clone();
                let action_label = action.label.clone();
                let action_selector = format!("notification-action-{}", action_id);
                body = body.child(
                    div().flex().justify_end().child(
                        div()
                            .debug_selector(move || action_selector.clone())
                            .h(px(CONTROL_HEIGHT_ACTION))
                            .child(
                                Button::new(gpui::SharedString::from(format!(
                                    "notification-action-{}",
                                    action_id
                                )))
                                .label(action_label)
                                .icon(Icon::new(MaterialIcon::OpenInNew).size(px(12.0)))
                                .ghost()
                                .small()
                                .h_full()
                                .px(px(SPACING_8))
                                .rounded(px(RADIUS_CONTROL))
                                .on_click(
                                    move |_event, window, cx| {
                                        action.invoke(window, cx);
                                        let action_center = action_center.clone();
                                        let action_dismiss_id = action_dismiss_id.clone();
                                        cx.defer(move |cx| {
                                            action_center.update(cx, |notifications, cx| {
                                                notifications.dismiss_generation(
                                                    action_dismiss_id.clone(),
                                                    notification_generation,
                                                    cx,
                                                );
                                            });
                                        });
                                        cx.stop_propagation();
                                    },
                                ),
                            ),
                    ),
                );
            }
            let card_selector = format!("notification-{}", notification_id);
            let marker_selector = format!("notification-marker-{}", notification_id);
            let dismiss_selector = format!("dismiss-notification-{}", notification_id);
            let card = div()
                .id(gpui::SharedString::from(format!(
                    "notification-{}",
                    notification_id
                )))
                .debug_selector(move || card_selector.clone())
                .w_full()
                .relative()
                .min_h(px(56.0))
                .max_h(px(NOTIFICATION_CARD_MAX_HEIGHT))
                .rounded(px(RADIUS_CONTROL))
                .border_1()
                .border_color(theme.border_subtle)
                .bg(theme.surface_raised)
                .shadow_sm()
                .overflow_hidden()
                .pl(px(SPACING_12))
                .pr(px(SPACING_8))
                .py(px(SPACING_4))
                .flex()
                .items_start()
                .gap(px(SPACING_8))
                .occlude()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(|_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .debug_selector(move || marker_selector.clone())
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left_0()
                        .w(px(3.0))
                        .bg(accent),
                )
                .child(
                    div()
                        .size(px(16.0))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(Icon::new(status_icon).size(px(14.0)).text_color(accent)),
                )
                .child(body)
                .child(
                    div()
                        .debug_selector(move || dismiss_selector.clone())
                        .absolute()
                        .top(px(SPACING_4))
                        .right(px(SPACING_4))
                        .size(px(CONTROL_HEIGHT_ACTION))
                        .child(
                            Button::new(gpui::SharedString::from(format!(
                                "dismiss-notification-{}",
                                notification_id
                            )))
                            .icon(Icon::new(MaterialIcon::Close).size(px(13.0)))
                            .tooltip("关闭通知")
                            .ghost()
                            .small()
                            .size(px(CONTROL_HEIGHT_ACTION))
                            .rounded(px(RADIUS_CONTROL))
                            .on_click(move |_event, _window, cx| {
                                let notification_center = notification_center.clone();
                                let dismiss_id = dismiss_id.clone();
                                cx.defer(move |cx| {
                                    notification_center.update(cx, |notifications, cx| {
                                        notifications.dismiss_generation(
                                            dismiss_id.clone(),
                                            notification_generation,
                                            cx,
                                        );
                                    });
                                });
                            }),
                        ),
                );
            let transition_duration = if notification_exiting {
                NOTIFICATION_EXIT_DURATION
            } else {
                NOTIFICATION_ENTER_DURATION
            };
            stack = stack.child(card.with_animation(
                gpui::SharedString::from(format!(
                    "notification-transition-{}-{}-{}",
                    notification_id, notification_generation, notification_exiting
                )),
                Animation::new(transition_duration).with_easing(ease_in_out),
                move |card, delta| {
                    let (opacity, vertical_offset) =
                        notification_transition(delta, notification_exiting);
                    card.opacity(opacity).relative().top(px(vertical_offset))
                },
            ));
        }
        stack.into_any_element()
    }

    fn render_modifier_action_selector(
        &self,
        modifier: ClipboardClickModifier,
        current: ClipboardClickAction,
    ) -> AnyElement {
        let theme = self.theme;
        let clipboard = self.clipboard.clone();
        Button::new(match modifier {
            ClipboardClickModifier::Control => "clipboard-modifier-control",
            ClipboardClickModifier::Alt => "clipboard-modifier-alt",
            ClipboardClickModifier::Shift => "clipboard-modifier-shift",
        })
        .ghost()
        .compact()
        .xsmall()
        .h(px(26.0))
        .w(px(154.0))
        .px(px(8.0))
        .rounded(px(4.0))
        .border_1()
        .border_color(theme.border_subtle)
        .bg(theme.surface_raised)
        .child(div().text_size(px(11.0)).child(click_action_label(current)))
        .text_color(theme.text_secondary)
        .dropdown_menu_with_anchor(Corner::TopRight, move |menu, _window, _cx| {
            ClipboardClickAction::ALL.into_iter().fold(
                menu.min_w(px(154.0)).max_w(px(154.0)),
                |menu, action| {
                    let clipboard = clipboard.clone();
                    let label = click_action_label(action);
                    menu.item(
                        PopupMenuItem::element(move |_window, _cx| {
                            div().text_size(px(11.0)).child(label)
                        })
                        .checked(action == current)
                        .on_click(move |_event, _window, cx| {
                            let _ = clipboard.update(cx, |application, _| {
                                application.set_clipboard_modifier_action(modifier, action);
                            });
                        }),
                    )
                },
            )
        })
        .into_any_element()
    }

    fn render_retention_stepper(
        &mut self,
        id: &'static str,
        value: usize,
        step: usize,
        edits_days: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        div()
            .id(id)
            .h(px(26.0))
            .rounded(px(4.0))
            .border_1()
            .border_color(theme.border)
            .bg(theme.canvas)
            .flex()
            .items_center()
            .child(
                div()
                    .id(gpui::SharedString::from(format!("{id}-minus")))
                    .w(px(28.0))
                    .h_full()
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(theme.text_muted)
                    .hover(move |style| style.bg(theme.surface_hover).text_color(theme.text))
                    .child("−")
                    .on_click(cx.listener(move |this, _event, _window, cx| {
                        cx.stop_propagation();
                        let _ = this.clipboard.update(cx, |application, _| {
                            let settings = application.clipboard_settings();
                            if edits_days {
                                application.set_clipboard_retention(
                                    settings.max_items,
                                    settings.retention_days.saturating_sub(step as u32),
                                );
                            } else {
                                application.set_clipboard_retention(
                                    settings.max_items.saturating_sub(step),
                                    settings.retention_days,
                                );
                            }
                        });
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .w(px(72.0))
                    .h_full()
                    .border_l_1()
                    .border_r_1()
                    .border_color(theme.border_subtle)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(11.0))
                    .font_family(UI_MONO_FONT_FAMILY)
                    .child(value.to_string()),
            )
            .child(
                div()
                    .id(gpui::SharedString::from(format!("{id}-plus")))
                    .w(px(28.0))
                    .h_full()
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(theme.text_muted)
                    .hover(move |style| style.bg(theme.surface_hover).text_color(theme.text))
                    .child("+")
                    .on_click(cx.listener(move |this, _event, _window, cx| {
                        cx.stop_propagation();
                        let _ = this.clipboard.update(cx, |application, _| {
                            let settings = application.clipboard_settings();
                            if edits_days {
                                application.set_clipboard_retention(
                                    settings.max_items,
                                    settings.retention_days.saturating_add(step as u32),
                                );
                            } else {
                                application.set_clipboard_retention(
                                    settings.max_items.saturating_add(step),
                                    settings.retention_days,
                                );
                            }
                        });
                        cx.notify();
                    })),
            )
            .into_any_element()
    }

    fn defer_hotkey_recording(
        &self,
        target: HotkeyRecordingTarget,
        recording: bool,
        cx: &mut Context<Self>,
    ) {
        let clipboard = self.clipboard.clone();
        let quick_launch = self.quick_launch.clone();
        cx.defer(move |cx| match target {
            HotkeyRecordingTarget::Clipboard => {
                let _ = clipboard.update(cx, |application, _| {
                    application.set_clipboard_hotkey_recording(recording);
                });
            }
            HotkeyRecordingTarget::QuickLaunch => {
                let _ = quick_launch.update(cx, |application, _| {
                    application.set_hotkey_recording(recording);
                });
            }
        });
    }

    fn defer_recorded_hotkey(
        &self,
        target: HotkeyRecordingTarget,
        hotkey: ClipboardHotKey,
        cx: &mut Context<Self>,
    ) {
        let clipboard = self.clipboard.clone();
        let quick_launch = self.quick_launch.clone();
        cx.defer(move |cx| match target {
            HotkeyRecordingTarget::Clipboard => {
                let _ = clipboard.update(cx, |application, _| {
                    application.set_clipboard_hotkey(hotkey);
                });
            }
            HotkeyRecordingTarget::QuickLaunch => {
                let _ = quick_launch.update(cx, |application, _| {
                    application.set_hotkey(hotkey);
                });
            }
        });
    }

    fn defer_all_hotkey_recording(&self, recording: bool, cx: &mut Context<Self>) {
        let clipboard = self.clipboard.clone();
        let quick_launch = self.quick_launch.clone();
        cx.defer(move |cx| {
            let _ = clipboard.update(cx, |application, _| {
                application.set_clipboard_hotkey_recording(recording);
            });
            let _ = quick_launch.update(cx, |application, _| {
                application.set_hotkey_recording(recording);
            });
        });
    }

    fn handle_hotkey_recording(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let Some(target) = self.recording_hotkey else {
            return;
        };
        if event.keystroke.key.eq_ignore_ascii_case("escape") {
            self.recording_hotkey = None;
            self.defer_all_hotkey_recording(false, cx);
            cx.stop_propagation();
            cx.notify();
            return;
        }
        let clipboard_ready = self
            .clipboard
            .upgrade()
            .is_none_or(|application| application.read(cx).clipboard_hotkey_recording_ready());
        let quick_launch_ready = self
            .quick_launch
            .upgrade()
            .is_none_or(|application| application.read(cx).hotkey_recording_ready());
        let recording_ready = clipboard_ready && quick_launch_ready;
        if !recording_ready {
            cx.stop_propagation();
            return;
        }
        let Some(hotkey) = hotkey_from_key_down(event) else {
            return;
        };
        self.defer_recorded_hotkey(target, hotkey, cx);
        let other = match target {
            HotkeyRecordingTarget::Clipboard => HotkeyRecordingTarget::QuickLaunch,
            HotkeyRecordingTarget::QuickLaunch => HotkeyRecordingTarget::Clipboard,
        };
        self.defer_hotkey_recording(other, false, cx);
        self.recording_hotkey = None;
        cx.stop_propagation();
        cx.notify();
    }

    fn module_icon(module: MainModule) -> MaterialIcon {
        match module {
            MainModule::Downloads => MaterialIcon::Download,
            MainModule::QuickFind => MaterialIcon::Search,
            MainModule::Clipboard => MaterialIcon::ContentPaste,
        }
    }

    fn render_titlebar(&self, window: &Window) -> AnyElement {
        let theme = self.theme;
        let network_settings = self.network_settings.clone();
        let global_settings_action = div()
            .id("global-network-settings")
            .debug_selector(|| "global-network-settings".into())
            .h(px(CONTROL_HEIGHT_ACTION))
            .child(
                labelled_icon_action(
                    "global-network-settings-button",
                    MaterialIcon::Settings,
                    "全局设置",
                )
                .ghost()
                .on_click(move |_, _, cx| {
                    let Some(network_settings) = network_settings.clone() else {
                        return;
                    };
                    cx.defer(move |cx| {
                        network_settings.update(cx, |application, cx| {
                            application.open_window(cx);
                        });
                    });
                }),
            );
        div()
            .id("main-titlebar")
            .debug_selector(|| "main-titlebar".into())
            .h(px(TITLEBAR_HEIGHT))
            .w_full()
            .flex()
            .items_center()
            .border_b_1()
            .border_color(theme.border_subtle)
            .bg(theme.canvas)
            .child(
                div()
                    .id("main-titlebar-drag-region")
                    .debug_selector(|| "main-titlebar-drag-region".into())
                    .h_full()
                    .flex_1()
                    .on_mouse_down(MouseButton::Left, {
                        let platform = self.platform.clone();
                        let window_handle = self.native_window_handle;
                        move |_, _, cx| {
                            if window_handle == 0 {
                                return;
                            }
                            if let Some(platform) = platform.as_ref() {
                                let _ = platform.send_command(PlatformCommand::BeginWindowMove(
                                    BeginWindowMoveCommand { window_handle },
                                ));
                            }
                            cx.stop_propagation();
                        }
                    })
                    .flex()
                    .items_center()
                    .gap(px(SPACING_8))
                    .pl(px(SPACING_12))
                    .child(
                        div()
                            .id("main-brand-icon")
                            .debug_selector(|| "main-brand-icon".into())
                            .size(px(18.0))
                            .rounded(px(4.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                img("brand/nxtools.png")
                                    .size(px(18.0))
                                    .object_fit(ObjectFit::Contain),
                            ),
                    )
                    .child(
                        div()
                            .debug_selector(|| "main-brand-title".into())
                            .h(px(LINE_HEIGHT_BODY))
                            .text_size(px(TEXT_BODY))
                            .line_height(px(LINE_HEIGHT_BODY))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme.text_secondary)
                            .child("NxTools"),
                    )
                    .child(
                        div()
                            .debug_selector(|| "main-active-module".into())
                            .h(px(LINE_HEIGHT_LABEL))
                            .text_size(px(TEXT_LABEL))
                            .line_height(px(LINE_HEIGHT_LABEL))
                            .text_color(theme.text_muted)
                            .child(format!("· {}", self.shell.active_module().label())),
                    ),
            )
            .child(
                div()
                    .debug_selector(|| "main-titlebar-utilities".into())
                    .h_full()
                    .flex()
                    .items_center()
                    .gap(px(SPACING_4))
                    .child(global_settings_action)
                    .child(
                        div()
                            .debug_selector(|| "main-window-controls".into())
                            .h_full()
                            .flex()
                            .child(self.window_control(MainWindowControl::Minimize, theme))
                            .child(
                                self.window_control(maximize_control(window.is_maximized()), theme),
                            )
                            .child(self.window_control(MainWindowControl::Close, theme)),
                    ),
            )
            .into_any_element()
    }

    fn window_control(&self, control: MainWindowControl, theme: Theme) -> AnyElement {
        let platform = self.platform.clone();
        let window_handle = self.native_window_handle;
        let system_settings = Rc::clone(&self.system_settings);
        window_control(control, theme, move |_, window, cx| match control {
            MainWindowControl::Minimize if window_handle != 0 => {
                if let Some(platform) = platform.as_ref() {
                    let _ = platform.send_command(PlatformCommand::SetWindowMinimized(
                        SetWindowMinimizedCommand { window_handle },
                    ));
                }
            }
            MainWindowControl::Maximize | MainWindowControl::Restore if window_handle != 0 => {
                if let Some(platform) = platform.as_ref() {
                    let _ = platform.send_command(PlatformCommand::ToggleWindowMaximized(
                        ToggleWindowMaximizedCommand { window_handle },
                    ));
                }
            }
            MainWindowControl::Minimize => window.minimize_window(),
            MainWindowControl::Maximize | MainWindowControl::Restore => window.zoom_window(),
            MainWindowControl::Close
                if system_settings.borrow().close_to_tray && window_handle != 0 =>
            {
                if let Some(platform) = platform.as_ref() {
                    if platform
                        .send_command(PlatformCommand::SetWindowVisibility(
                            SetWindowVisibilityCommand {
                                window_handle,
                                visible: false,
                            },
                        ))
                        .is_err()
                    {
                        cx.quit();
                    }
                } else {
                    cx.quit();
                }
            }
            MainWindowControl::Close => cx.quit(),
        })
        .into_any_element()
    }

    fn render_sidebar(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let active = self.shell.active_module();
        let mut modules = div().flex().flex_col().gap(px(4.0));
        for module in MainModule::ALL {
            let selected = module == active;
            let selector = gpui::SharedString::from(format!("main-module-{}", module.label()));
            let debug_selector = selector.clone();
            let title_selector = format!("main-module-{}-title", module.label());
            let description_selector = format!("main-module-{}-description", module.label());
            modules = modules.child(
                div()
                    .id(selector)
                    .debug_selector(move || debug_selector.to_string())
                    .h(px(SIDEBAR_ROW_HEIGHT))
                    .px(px(10.0))
                    .rounded(px(4.0))
                    .border_l_2()
                    .border_color(if selected { theme.accent } else { theme.canvas })
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .cursor_pointer()
                    .bg(if selected {
                        theme.surface_raised
                    } else {
                        theme.canvas
                    })
                    .hover(move |style| style.bg(theme.surface_hover))
                    .child(
                        Icon::new(Self::module_icon(module))
                            .size(px(18.0))
                            .text_color(if selected {
                                theme.accent
                            } else {
                                theme.text_secondary
                            }),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(1.0))
                            .child(
                                div()
                                    .debug_selector(move || title_selector.clone())
                                    .text_size(px(TEXT_BODY))
                                    .line_height(px(LINE_HEIGHT_BODY))
                                    .font_weight(if selected {
                                        gpui::FontWeight::MEDIUM
                                    } else {
                                        gpui::FontWeight::NORMAL
                                    })
                                    .text_color(theme.text)
                                    .child(module.label()),
                            )
                            .child(
                                div()
                                    .debug_selector(move || description_selector.clone())
                                    .text_size(px(TEXT_LABEL))
                                    .line_height(px(LINE_HEIGHT_LABEL))
                                    .text_color(theme.text_muted)
                                    .child(module.description()),
                            ),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.shell.activate(module);
                        cx.notify();
                    })),
            );
        }

        div()
            .debug_selector(|| "main-sidebar".into())
            .w(px(SIDEBAR_WIDTH))
            .h_full()
            .p(px(10.0))
            .border_r_1()
            .border_color(theme.border_subtle)
            .bg(theme.canvas)
            .flex()
            .flex_col()
            .child(
                div().px(px(8.0)).pt(px(8.0)).pb(px(14.0)).child(
                    div()
                        .text_size(px(11.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(theme.text_muted)
                        .child("工作区"),
                ),
            )
            .child(modules)
            .child(div().flex_1())
            .into_any_element()
    }

    fn render_downloads(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let search_query = self
            .download_search_input
            .read(cx)
            .value()
            .trim()
            .to_owned();
        let (total_count, counts, visible_tasks, visible_progress, visible_icons) = self
            .downloads
            .upgrade()
            .map(|application| {
                let application = application.read(cx);
                let tasks = application.tasks();
                let visible_tasks = download_tasks_for_view(
                    tasks,
                    self.download_filter,
                    &search_query,
                    self.download_sort,
                );
                let visible_progress = visible_tasks
                    .iter()
                    .filter_map(|task| {
                        application
                            .progress(task.task.id())
                            .cloned()
                            .map(|progress| (task.task.id(), progress))
                    })
                    .collect::<HashMap<_, _>>();
                let visible_icons = visible_tasks
                    .iter()
                    .filter_map(|task| {
                        application
                            .task_icon_path(task.task.id())
                            .map(|path| (task.task.id(), path.to_path_buf()))
                    })
                    .collect::<HashMap<_, _>>();
                (
                    tasks.len(),
                    download_filter_counts(tasks.iter().map(|task| task.task.status())),
                    visible_tasks,
                    visible_progress,
                    visible_icons,
                )
            })
            .unwrap_or_default();
        let selected_task_id = reconcile_download_selection(
            self.selected_download_task,
            &mut self.selected_download_tasks,
            &visible_tasks,
            |task| task.task.id(),
        );
        self.selected_download_task = selected_task_id;
        let visible_task_ids = visible_tasks
            .iter()
            .map(|task| task.task.id())
            .collect::<Vec<_>>();
        let selected_task_ids = visible_task_ids
            .iter()
            .filter(|task_id| self.selected_download_tasks.contains(task_id))
            .copied()
            .collect::<Vec<_>>();
        self.download_selection_anchor = self
            .download_selection_anchor
            .filter(|task_id| self.selected_download_tasks.contains(task_id))
            .or(selected_task_id);
        let selected_task = selected_task_id
            .and_then(|task_id| visible_tasks.iter().find(|task| task.task.id() == task_id))
            .cloned();
        let (detail_snapshot, detail_error) = selected_task_id
            .and_then(|task_id| {
                self.downloads.upgrade().map(|application| {
                    let application = application.read(cx);
                    (
                        application.task_detail(task_id).cloned(),
                        application.task_detail_error(task_id).map(str::to_owned),
                    )
                })
            })
            .unwrap_or_default();
        let selected_progress = selected_task_id
            .and_then(|task_id| visible_progress.get(&task_id))
            .cloned();
        let main_window = cx.entity();
        let batch_toolbar = (selected_task_ids.len() > 1).then(|| {
            let mut capabilities =
                download_context_capabilities(&visible_tasks, &selected_task_ids);
            if capabilities.can_move_queue
                && let Some(application) = self.downloads.upgrade()
            {
                let (top, up, down, bottom) = download_queue_placement_capabilities(
                    &application.read(cx).default_queue_order(),
                    &selected_task_ids,
                );
                capabilities.queue_top = top;
                capabilities.queue_up = up;
                capabilities.queue_down = down;
                capabilities.queue_bottom = bottom;
            }
            download_batch_toolbar(
                theme,
                self.downloads.clone(),
                main_window.clone(),
                selected_task_ids.clone(),
                capabilities,
            )
        });
        if let Some(task_id) = selected_task_id {
            if detail_snapshot.is_some() {
                self.requested_download_detail = Some(task_id);
            } else if self.requested_download_detail != Some(task_id) {
                self.requested_download_detail = Some(task_id);
                self.download_detail_scroll
                    .set_offset(point(px(0.0), px(0.0)));
                let application = self.downloads.clone();
                cx.defer(move |cx| {
                    let _ = application.update(cx, |application, cx| {
                        application.request_task_detail(task_id, cx);
                    });
                });
            }
        } else {
            self.requested_download_detail = None;
        }
        let detail_tab = self.download_detail_tab;
        let detail_scroll = self.download_detail_scroll.clone();
        let download_application = self.downloads.clone();
        let detail_main_window = main_window.clone();
        let selected_task_detail = selected_task.map(|task| {
            download_task_detail(
                theme,
                DownloadDetailView {
                    task: &task,
                    detail: detail_snapshot.as_ref(),
                    detail_error: detail_error.as_deref(),
                    progress: selected_progress.as_ref(),
                    active_tab: detail_tab,
                    scroll: detail_scroll,
                    main_window: detail_main_window,
                },
            )
            .into_any_element()
        });
        let visible_count = visible_tasks.len();
        let virtual_tasks = Arc::new(visible_tasks);
        let virtual_task_ids = Arc::new(visible_task_ids);
        let virtual_progress = Arc::new(visible_progress);
        let virtual_icons = Arc::new(visible_icons);
        let virtual_main_window = main_window.clone();
        let task_list = uniform_list(
            "download-task-list",
            visible_count,
            cx.processor(move |this, range: std::ops::Range<usize>, _window, _cx| {
                range
                    .filter_map(|index| virtual_tasks.get(index).cloned())
                    .map(|task| {
                        let progress = virtual_progress.get(&task.task.id()).cloned();
                        let icon_path = virtual_icons.get(&task.task.id()).cloned();
                        download_task_row(
                            theme,
                            this.downloads.clone(),
                            virtual_main_window.clone(),
                            DownloadRowSelection {
                                selected: this.selected_download_tasks.contains(&task.task.id()),
                                visible_task_ids: virtual_task_ids.clone(),
                            },
                            task,
                            progress,
                            icon_path,
                        )
                    })
                    .collect::<Vec<_>>()
            }),
        )
        .size_full()
        .track_scroll(self.download_list_scroll.clone());
        let filter_segments = segmented_control(
            theme,
            "download-filter-segments",
            self.download_filter.id(),
            DownloadTaskFilter::ALL
                .into_iter()
                .map(|filter| {
                    let filter_count = counts.get(filter);
                    SegmentedOption::new(
                        filter.id(),
                        format!("{} {filter_count}", filter.label()),
                        cx.listener(move |this, _, _, cx| {
                            if select_download_filter(
                                &mut this.download_filter,
                                filter,
                                filter_count,
                                total_count,
                            ) {
                                this.download_list_scroll
                                    .scroll_to_item(0, ScrollStrategy::Top);
                                cx.notify();
                            }
                        }),
                    )
                })
                .collect(),
            cx,
        );
        let filter_summary =
            if self.download_filter == DownloadTaskFilter::All && search_query.is_empty() {
                format!("共 {total_count} 项")
            } else {
                format!("显示 {visible_count} / {total_count}")
            };
        let search_input = self.download_search_input.clone();
        let clear_search_input = self.download_search_input.clone();
        let sort_state = self.download_sort;
        let sort_main_window = main_window.clone();
        let sort_label = if sort_state.key == DownloadSortKey::Queue {
            sort_state.key.label().to_owned()
        } else {
            format!(
                "{} {}",
                sort_state.key.label(),
                if sort_state.descending { "↓" } else { "↑" }
            )
        };
        let settings_download_application = self.downloads.clone();
        let schedule_download_application = self.downloads.clone();
        let new_download_application = download_application.clone();
        let download_header_trailing = div()
            .debug_selector(|| "download-module-header-actions".into())
            .flex()
            .items_center()
            .gap(px(SPACING_4))
            .child(
                div()
                    .id("download-search")
                    .debug_selector(|| "download-search".into())
                    .w(px(188.0))
                    .h(px(CONTROL_HEIGHT_ACTION))
                    .rounded(px(RADIUS_CONTROL))
                    .border_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.surface)
                    .px(px(SPACING_6))
                    .flex()
                    .items_center()
                    .gap(px(SPACING_4))
                    .child(
                        Icon::new(MaterialIcon::Search)
                            .size(px(13.0))
                            .text_color(theme.text_muted),
                    )
                    .child(
                        div().min_w_0().flex_1().child(
                            Input::new(&search_input)
                                .appearance(false)
                                .bordered(false)
                                .focus_bordered(false)
                                .xsmall(),
                        ),
                    )
                    .when(!search_query.is_empty(), |search| {
                        search.child(
                            div()
                                .debug_selector(|| "download-search-clear".into())
                                .size(px(18.0))
                                .flex_none()
                                .child(
                                    Button::new("download-search-clear")
                                        .icon(Icon::new(MaterialIcon::Close))
                                        .tooltip("清除搜索")
                                        .xsmall()
                                        .compact()
                                        .ghost()
                                        .size_full()
                                        .on_click(move |_, window, cx| {
                                            clear_search_input.update(cx, |input, cx| {
                                                input.set_value("", window, cx);
                                            });
                                        }),
                                ),
                        )
                    }),
            )
            .child(
                div()
                    .debug_selector(|| "download-settings-action".into())
                    .h(px(CONTROL_HEIGHT_ACTION))
                    .child(
                        icon_action("download-settings", MaterialIcon::Settings, "下载设置")
                            .ghost()
                            .on_click(move |_, _, cx| {
                                let application = settings_download_application.clone();
                                cx.defer(move |cx| {
                                    let _ = application.update(cx, |application, cx| {
                                        application.open_settings_window(cx);
                                    });
                                });
                            }),
                    ),
            )
            .child(
                div()
                    .debug_selector(|| "download-schedule-action".into())
                    .h(px(CONTROL_HEIGHT_ACTION))
                    .child(
                        icon_action("download-schedule", MaterialIcon::History, "下载计划")
                            .ghost()
                            .on_click(move |_, _, cx| {
                                let application = schedule_download_application.clone();
                                cx.defer(move |cx| {
                                    let _ = application.update(cx, |application, cx| {
                                        application.open_schedule_window(cx);
                                    });
                                });
                            }),
                    ),
            )
            .child(
                div()
                    .debug_selector(|| "download-new-task-action".into())
                    .h(px(CONTROL_HEIGHT_ACTION))
                    .child(
                        labelled_icon_action("download-new-task", MaterialIcon::Add, "新建下载")
                            .primary()
                            .on_click(move |_, _, cx| {
                                let application = new_download_application.clone();
                                cx.defer(move |cx| {
                                    let _ = application.update(cx, |application, cx| {
                                        application.open_new_download_window(cx);
                                    });
                                });
                            }),
                    ),
            )
            .into_any_element();
        div()
            .size_full()
            .bg(theme.canvas)
            .flex()
            .flex_col()
            .child(
                ModuleHeader::new(
                    "download-module-header",
                    "download-module-header-title",
                    "download-module-header-metadata",
                    "下载",
                    filter_summary.clone(),
                )
                .trailing(download_header_trailing)
                .render(theme),
            )
            .child(
                div()
                    .id("download-filter-strip")
                    .debug_selector(|| "download-filter-strip".into())
                    .h(px(DOWNLOAD_FILTER_STRIP_HEIGHT))
                    .px(px(12.0))
                    .border_b_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.surface)
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .w(px(DOWNLOAD_FILTER_SEGMENTED_WIDTH))
                            .flex_none()
                            .child(filter_segments),
                    )
                    .child(
                        div()
                            .debug_selector(|| "download-sort".into())
                            .h(px(CONTROL_HEIGHT_ACTION))
                            .flex_none()
                            .child(
                                action_button("download-sort", sort_label)
                                    .dropdown_menu_with_anchor(
                                        Corner::TopRight,
                                        move |menu, _window, _cx| {
                                            DownloadSortKey::ALL.into_iter().fold(
                                                menu.min_w(px(118.0)),
                                                |menu, key| {
                                                    let sort_main_window = sort_main_window.clone();
                                                    menu.item(
                                                        PopupMenuItem::new(key.label())
                                                            .checked(key == sort_state.key)
                                                            .on_click(move |_event, _window, cx| {
                                                                sort_main_window.update(
                                                                    cx,
                                                                    |window, cx| {
                                                                        if set_download_sort(
                                                                            &mut window
                                                                                .download_sort,
                                                                            key,
                                                                            false,
                                                                        ) {
                                                                            window
                                                                                .download_list_scroll
                                                                                .scroll_to_item(
                                                                                    0,
                                                                                    ScrollStrategy::Top,
                                                                                );
                                                                            cx.notify();
                                                                        }
                                                                    },
                                                                );
                                                            }),
                                                    )
                                                },
                                            )
                                        },
                                    ),
                            ),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .when_some(batch_toolbar, |workspace, toolbar| {
                        workspace.child(toolbar)
                    })
                    .child(download_table_header(theme, self.download_sort, main_window))
                    .child(if visible_count == 0 {
                        if total_count == 0 {
                            empty_state(
                                theme,
                                "download-empty-state",
                                MaterialIcon::Download,
                                "还没有下载任务",
                                "点击右上角新建下载，提交真实 HTTP 或 HTTPS 地址。",
                            )
                            .into_any_element()
                        } else if !search_query.is_empty() {
                            empty_state(
                                theme,
                                "download-search-empty-state",
                                MaterialIcon::Search,
                                "没有匹配的下载任务",
                                "尝试缩短关键词，或切换到其他状态分类。",
                            )
                            .into_any_element()
                        } else {
                            empty_state(
                                theme,
                                "download-filter-empty-state",
                                MaterialIcon::Download,
                                self.download_filter.empty_title(),
                                "任务状态变化后会自动出现在这里。",
                            )
                            .into_any_element()
                        }
                    } else {
                        div()
                            .id("download-task-list-scroll")
                            .relative()
                            .flex_1()
                            .min_h_0()
                            .bg(theme.canvas)
                            .child(task_list)
                            .vertical_scrollbar(&self.download_list_scroll)
                            .into_any_element()
                    })
                    .when_some(selected_task_detail, |workspace, detail| {
                        workspace.child(detail)
                    }),
            )
            .into_any_element()
    }

    fn render_quick_find(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let (
            settings,
            status,
            index_status,
            hotkey_lifecycle,
            indexer_connected,
            history_summary,
            performance,
            log_directory,
        ) = self
            .quick_launch
            .upgrade()
            .map(|application| {
                let application = application.read(cx);
                (
                    application.settings(),
                    application.catalog_status(),
                    application.index_status(),
                    application.hotkey_lifecycle(),
                    application.indexer_connected(),
                    application.history_summary(),
                    application.performance_snapshot(),
                    application.log_directory().to_path_buf(),
                )
            })
            .unwrap_or_else(|| {
                (
                    nxtools_core::QuickLaunchSettings::default(),
                    nxtools_finder::CatalogStatus::default(),
                    None,
                    QuickLaunchHotkeyLifecycle::Failed {
                        error: "速启核心不可用".into(),
                    },
                    false,
                    nxtools_finder::QuickLaunchHistorySummary::default(),
                    QuickLaunchPerformanceSnapshot::default(),
                    std::path::PathBuf::new(),
                )
            });
        let recording_quick = self.recording_hotkey == Some(HotkeyRecordingTarget::QuickLaunch);
        if recording_quick && !hotkey_lifecycle.owns_recording_session() {
            self.recording_hotkey = None;
        }
        let (hotkey_state, hotkey_value, hotkey_status) = match hotkey_lifecycle {
            QuickLaunchHotkeyLifecycle::Applied => (
                HotkeyFieldState::Applied,
                settings.hotkey.display.clone(),
                "已应用到 Windows 全局热键".to_owned(),
            ),
            QuickLaunchHotkeyLifecycle::WaitingForAdapter { status } => (
                HotkeyFieldState::Waiting,
                settings.hotkey.display.clone(),
                status,
            ),
            QuickLaunchHotkeyLifecycle::WaitingForRecording { status } => (
                HotkeyFieldState::Waiting,
                settings.hotkey.display.clone(),
                status,
            ),
            QuickLaunchHotkeyLifecycle::Recording => (
                HotkeyFieldState::Recording,
                "按下组合键".to_owned(),
                "请按下包含修饰键的组合键 · Esc 取消".to_owned(),
            ),
            QuickLaunchHotkeyLifecycle::Applying { candidate } => (
                HotkeyFieldState::Applying,
                candidate.unwrap_or_else(|| settings.hotkey.display.clone()),
                "正在向 Windows 应用全局热键".to_owned(),
            ),
            QuickLaunchHotkeyLifecycle::Failed { error } => (
                HotkeyFieldState::Failed,
                settings.hotkey.display.clone(),
                error,
            ),
        };
        let hotkey_control = hotkey_field(
            theme,
            "quick-launch-hotkey-field",
            hotkey_value,
            hotkey_status,
            hotkey_state,
            cx.listener(move |this, _, window, cx| {
                let recording = this.recording_hotkey != Some(HotkeyRecordingTarget::QuickLaunch);
                if this.recording_hotkey.take().is_some() {
                    this.defer_all_hotkey_recording(false, cx);
                }
                this.recording_hotkey = recording.then_some(HotkeyRecordingTarget::QuickLaunch);
                this.defer_all_hotkey_recording(recording, cx);
                if recording {
                    this.hotkey_focus.focus(window);
                }
                cx.notify();
            }),
        );
        let auto_quick_switch = self.quick_launch.clone();
        let auto_quick_switch_enabled = settings.auto_quick_switch;
        let auto_quick_switch_control = compact_toggle(
            theme,
            "quick-launch-auto-quick-switch",
            auto_quick_switch_enabled,
            move |_event, _window, cx| {
                let _ = auto_quick_switch.update(cx, |application, cx| {
                    application.set_auto_quick_switch(!auto_quick_switch_enabled, cx);
                });
            },
        );
        let file_dialog_search = self.quick_launch.clone();
        let file_dialog_search_enabled = settings.file_dialog_search;
        let file_dialog_search_control = compact_toggle(
            theme,
            "quick-launch-file-dialog-search",
            file_dialog_search_enabled,
            move |_event, _window, cx| {
                let _ = file_dialog_search.update(cx, |application, cx| {
                    application.set_file_dialog_search(!file_dialog_search_enabled, cx);
                });
            },
        );
        let refresh_volumes = self.quick_launch.clone();
        let refresh_volumes_control = Button::new("quick-launch-refresh-volumes")
            .ghost()
            .compact()
            .xsmall()
            .h(px(26.0))
            .px(px(9.0))
            .rounded(px(4.0))
            .border_1()
            .border_color(theme.border_subtle)
            .bg(theme.surface_raised)
            .child(div().text_size(px(10.5)).child(if indexer_connected {
                "刷新"
            } else {
                "安装"
            }))
            .on_click(move |_event, _window, cx| {
                let _ = refresh_volumes.update(cx, |application, cx| {
                    application.prepare_file_index(cx);
                });
            })
            .into_any_element();
        let available_volumes = index_status
            .as_ref()
            .map(|status| status.drives.clone())
            .unwrap_or_default();
        let available_mounts = available_volumes
            .iter()
            .map(|drive| drive.mount_point.clone())
            .collect::<Vec<_>>();
        let configured_volumes = settings.indexed_volumes.clone();
        let set_all_volumes = self.quick_launch.clone();
        let mut volume_scope_rows = vec![automatic_volume_scope_row(
            theme,
            configured_volumes.is_empty(),
            move |checked, _window, cx| {
                if !checked {
                    return;
                }
                let set_all_volumes = set_all_volumes.clone();
                cx.defer(move |cx| {
                    let _ = set_all_volumes.update(cx, |application, cx| {
                        application.set_indexed_volumes(Vec::new(), cx);
                    });
                });
            },
        )];
        if available_volumes.is_empty() {
            volume_scope_rows.push(empty_volume_scope_row(
                theme,
                "等待发现可建立索引的 NTFS 卷",
            ));
        }
        for (index, drive) in available_volumes.iter().enumerate() {
            let selected = volume_is_selected(&configured_volumes, &drive.mount_point);
            let label = drive.mount_point.trim_end_matches(['\\', '/']).to_owned();
            let selector = format!(
                "quick-launch-index-volume-{}",
                volume_scope_selector_key(&drive.mount_point)
            );
            let metadata = if drive.indexed_items == 0 {
                format!("{} · 尚未建立索引", drive.file_system)
            } else {
                format!(
                    "{} · {} 项",
                    drive.file_system,
                    grouped_count(drive.indexed_items)
                )
            };
            let mount_for_click = drive.mount_point.clone();
            let configured_for_click = configured_volumes.clone();
            let available_for_click = available_mounts.clone();
            let quick_launch = self.quick_launch.clone();
            volume_scope_rows.push(volume_scope_row(
                theme,
                selector,
                label,
                metadata,
                selected,
                index + 1 < available_volumes.len(),
                move |_checked, _window, cx| {
                    let selected = toggle_indexed_volume_selection(
                        &configured_for_click,
                        &available_for_click,
                        &mount_for_click,
                    );
                    if selected == configured_for_click {
                        return;
                    }
                    let quick_launch = quick_launch.clone();
                    cx.defer(move |cx| {
                        let _ = quick_launch.update(cx, |application, cx| {
                            application.set_indexed_volumes(selected, cx);
                        });
                    });
                },
            ));
        }
        let index_volumes_control = volume_scope_setting(
            theme,
            "自动模式会持续包含新卷；取消任一卷后改为仅索引勾选项",
            volume_scope_rows,
        );
        if self
            .selected_index_exclusion
            .is_some_and(|index| index >= settings.excluded_paths.len())
        {
            self.selected_index_exclusion = settings.excluded_paths.len().checked_sub(1);
        }
        let main_window = cx.entity();
        let select_main_window = main_window.clone();
        let remove_main_window = main_window.clone();
        let remove_quick_launch = self.quick_launch.clone();
        let add_quick_launch = self.quick_launch.clone();
        let exclusion_count = settings.excluded_paths.len();
        let index_exclusions_control = path_exclusion_setting(
            theme,
            &settings.excluded_paths,
            self.selected_index_exclusion,
            move |index, window, cx| {
                select_main_window.update(cx, |this, cx| {
                    this.selected_index_exclusion = Some(index);
                    this.hotkey_focus.focus(window);
                    cx.notify();
                });
            },
            move |index, path, window, cx| {
                remove_main_window.update(cx, |this, cx| {
                    this.selected_index_exclusion =
                        exclusion_selection_after_removal(exclusion_count, index);
                    this.hotkey_focus.focus(window);
                    cx.notify();
                });
                let quick_launch = remove_quick_launch.clone();
                cx.defer(move |cx| {
                    let _ = quick_launch.update(cx, |application, cx| {
                        application.remove_excluded_path(&path, cx);
                    });
                });
            },
            move |_event, _window, cx| {
                let quick_launch = add_quick_launch.clone();
                cx.defer(move |cx| {
                    let _ = quick_launch.update(cx, |application, cx| {
                        application.request_index_exclusion_directory(cx);
                    });
                });
            },
            cx,
        );
        if self
            .selected_custom_command
            .is_some_and(|index| index >= settings.custom_commands.len())
        {
            self.selected_custom_command = None;
            self.custom_command_draft_generation =
                self.custom_command_draft_generation.wrapping_add(1);
            self.pending_custom_command_program_picker = None;
        }
        let selected_command = self
            .selected_custom_command
            .and_then(|index| settings.custom_commands.get(index).cloned());
        let custom_commands_control = custom_command_editor(
            theme,
            CustomCommandEditorState {
                commands: &settings.custom_commands,
                selected_index: self.selected_custom_command,
                inputs: CustomCommandEditorInputs {
                    keyword: self.custom_command_keyword.clone(),
                    title: self.custom_command_title.clone(),
                    program: self.custom_command_program.clone(),
                    arguments: self.custom_command_arguments.clone(),
                    working_directory: self.custom_command_working_directory.clone(),
                },
                run_as_admin: self.custom_command_run_as_admin,
                run_silently: self.custom_command_run_silently,
            },
            CustomCommandEditorActions {
                select: Rc::new(cx.listener(
                    move |this, event: &(usize, CustomCommandSettings), window, cx| {
                        let (index, command) = event;
                        this.select_custom_command_draft(
                            *index,
                            command.clone(),
                            window.window_handle(),
                            cx,
                        );
                    },
                )),
                add: Rc::new(cx.listener(move |this, _, window, cx| {
                    let quick_launch = this.quick_launch.clone();
                    let view = cx.entity().downgrade();
                    let source_window = window.window_handle();
                    cx.defer(move |cx| {
                        let added = quick_launch
                            .update(cx, |application, cx| {
                                let index = application.add_custom_command(cx);
                                let command =
                                    application.settings().custom_commands.get(index).cloned();
                                command.map(|command| (index, command))
                            })
                            .ok()
                            .flatten();
                        if let Some((index, command)) = added {
                            let _ = view.update(cx, |this, cx| {
                                this.select_custom_command_draft(index, command, source_window, cx);
                            });
                        }
                    });
                })),
                set_enabled: Rc::new(cx.listener({
                    let selected_command = selected_command.clone();
                    move |this, enabled, _window, cx| {
                        let Some(index) = this.selected_custom_command else {
                            return;
                        };
                        let Some(mut command) = selected_command.clone() else {
                            return;
                        };
                        command.enabled = *enabled;
                        let quick_launch = this.quick_launch.clone();
                        cx.defer(move |cx| {
                            let _ = quick_launch.update(cx, |application, cx| {
                                application.update_custom_command(index, command, cx);
                            });
                        });
                    }
                })),
                delete: Rc::new(cx.listener(move |this, _, window, cx| {
                    let Some(index) = this.selected_custom_command else {
                        return;
                    };
                    let quick_launch = this.quick_launch.clone();
                    let view = cx.entity().downgrade();
                    let source_window = window.window_handle();
                    cx.defer(move |cx| {
                        let removal = quick_launch
                            .update(cx, |application, cx| {
                                if !application.remove_custom_command(index, cx) {
                                    return (false, None);
                                }
                                let commands = application.settings().custom_commands;
                                let next_index =
                                    custom_command_selection_after_removal(commands.len(), index);
                                let next = next_index.and_then(|next_index| {
                                    commands
                                        .get(next_index)
                                        .cloned()
                                        .map(|command| (next_index, command))
                                });
                                (true, next)
                            })
                            .ok();
                        let Some((true, next)) = removal else {
                            return;
                        };
                        let _ = view.update(cx, |this, cx| {
                            if let Some((next_index, command)) = next {
                                this.select_custom_command_draft(
                                    next_index,
                                    command,
                                    source_window,
                                    cx,
                                );
                            } else {
                                this.selected_custom_command = None;
                                this.custom_command_draft_generation =
                                    this.custom_command_draft_generation.wrapping_add(1);
                                this.pending_custom_command_program_picker = None;
                                cx.notify();
                            }
                        });
                    });
                })),
                pick_program: Rc::new(cx.listener(move |this, _, _window, cx| {
                    let Some(index) = this.selected_custom_command else {
                        return;
                    };
                    let generation = this.custom_command_draft_generation;
                    let current_path = this.custom_command_program.read(cx).value().to_string();
                    let quick_launch = this.quick_launch.clone();
                    let view = cx.entity().downgrade();
                    cx.defer(move |cx| {
                        let correlation_id = quick_launch
                            .update(cx, |application, cx| {
                                application.request_custom_command_program(index, current_path, cx)
                            })
                            .ok()
                            .flatten();
                        if let Some(correlation_id) = correlation_id {
                            let _ = view.update(cx, |this, _| {
                                if this.selected_custom_command == Some(index)
                                    && this.custom_command_draft_generation == generation
                                {
                                    this.pending_custom_command_program_picker =
                                        Some(PendingCustomCommandProgramPicker {
                                            correlation_id,
                                            command_index: index,
                                            draft_generation: generation,
                                        });
                                }
                            });
                        }
                    });
                })),
                set_run_as_admin: Rc::new(cx.listener(move |this, enabled, _window, cx| {
                    this.custom_command_run_as_admin = *enabled;
                    cx.notify();
                })),
                set_run_silently: Rc::new(cx.listener(move |this, enabled, _window, cx| {
                    this.custom_command_run_silently = *enabled;
                    cx.notify();
                })),
                save: Rc::new(cx.listener(move |this, _, _window, cx| {
                    this.save_selected_custom_command(cx);
                })),
            },
            cx,
        );
        let history_is_empty =
            history_summary.total_activations == 0 && history_summary.recent_directories == 0;
        let history_description = format!(
            "{} 个排序项目 · {} 次启动 · {} 个关键词关联 · {} 个最近目录",
            history_summary.ranked_items,
            history_summary.total_activations,
            history_summary.query_associations,
            history_summary.recent_directories
        );
        let clear_history = self.quick_launch.clone();
        let clear_history_control = Button::new("quick-launch-clear-history")
            .ghost()
            .compact()
            .xsmall()
            .h(px(24.0))
            .px(px(8.0))
            .rounded(px(4.0))
            .border_1()
            .border_color(theme.border_subtle)
            .bg(theme.surface_raised)
            .disabled(history_is_empty)
            .child(div().text_size(px(10.0)).child("清空"))
            .on_click(move |_event, _window, cx| {
                let _ = clear_history.update(cx, |application, cx| {
                    application.clear_history(cx);
                });
            })
            .into_any_element();
        let performance_description = format!(
            "本地合并 {} · MFT/USN {}",
            format_query_elapsed(performance.last_local_search_us),
            format_query_elapsed(performance.last_index_search_us)
        );
        let performance_control = div()
            .text_size(px(10.0))
            .text_color(theme.accent_text)
            .child(
                if performance.last_local_search_us.is_some()
                    || performance.last_index_search_us.is_some()
                {
                    "LIVE"
                } else {
                    "IDLE"
                },
            )
            .into_any_element();
        let log_path = if log_directory.as_os_str().is_empty() {
            "日志目录不可用".to_owned()
        } else {
            log_directory.display().to_string()
        };
        let open_logs = self.quick_launch.clone();
        let open_logs_control = Button::new("quick-launch-open-logs")
            .ghost()
            .compact()
            .xsmall()
            .h(px(24.0))
            .px(px(8.0))
            .rounded(px(4.0))
            .border_1()
            .border_color(theme.border_subtle)
            .bg(theme.surface_raised)
            .disabled(log_directory.as_os_str().is_empty())
            .child(div().text_size(px(10.0)).child("打开日志"))
            .on_click(move |_event, _window, cx| {
                let _ = open_logs.update(cx, |application, _cx| {
                    application.open_log_directory();
                });
            })
            .into_any_element();
        let history_diagnostics_control = div()
            .rounded(px(5.0))
            .border_1()
            .border_color(theme.border_subtle)
            .bg(theme.surface)
            .px(px(12.0))
            .child(compact_group(
                theme,
                "历史与诊断",
                vec![
                    compact_setting_row(
                        theme,
                        "排序学习",
                        history_description,
                        clear_history_control,
                    ),
                    compact_setting_row(
                        theme,
                        "最近查询",
                        performance_description,
                        performance_control,
                    ),
                    compact_setting_row(theme, "运行日志", log_path, open_logs_control),
                ],
            ))
            .into_any_element();
        let scan_state = if status.errors.is_empty() {
            format!(
                "已索引 {} 个入口 · {} 个目录",
                status.indexed_items, status.scanned_roots
            )
        } else {
            format!(
                "已索引 {} 个入口 · {} 个目录读取异常",
                status.indexed_items,
                status.errors.len()
            )
        };
        let (file_index_state, file_index_badge) = file_index_summary(index_status.as_ref());
        let file_index_badge_color = match file_index_badge {
            "ERROR" | "PARTIAL" => theme.danger,
            "OFFLINE" | "EMPTY" => theme.text_muted,
            "UPDATING" => theme.accent_text,
            _ => theme.accent_text,
        };
        let general_page = quick_launch_settings_group_page(
            theme,
            QuickLaunchSettingsPage::General.metadata().content_id,
            compact_group(
                theme,
                "呼出与执行",
                vec![
                    hotkey_setting_row(
                        theme,
                        "quick-launch-hotkey-setting",
                        "全局呼出热键",
                        "点击字段开始录制；Esc 或失焦取消",
                        hotkey_control,
                    ),
                    compact_setting_row(
                        theme,
                        "键盘动作",
                        "↑↓ 选择；Enter 打开；Ctrl+Enter 定位；Esc 清空或隐藏",
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.text_muted)
                            .child("FIXED")
                            .into_any_element(),
                    ),
                ],
            ),
        );
        let context_page = quick_launch_settings_group_page(
            theme,
            QuickLaunchSettingsPage::Context.metadata().content_id,
            compact_group(
                theme,
                "上下文联动",
                vec![
                    compact_setting_row(
                        theme,
                        "自动快速切换",
                        "从资源管理器返回已出现的打开/保存对话框时同步目录",
                        auto_quick_switch_control,
                    ),
                    compact_setting_row(
                        theme,
                        "文件对话框搜索栏",
                        "在受支持的打开/保存窗口底部显示当前目录搜索",
                        file_dialog_search_control,
                    ),
                ],
            ),
        );
        let index_page = quick_launch_settings_group_page(
            theme,
            QuickLaunchSettingsPage::Index.metadata().content_id,
            compact_group(
                theme,
                "应用目录",
                vec![
                    compact_setting_row(
                        theme,
                        "扫描状态",
                        scan_state,
                        div()
                            .text_size(px(10.0))
                            .text_color(if status.errors.is_empty() {
                                theme.success
                            } else {
                                theme.danger
                            })
                            .child(if status.errors.is_empty() {
                                "READY"
                            } else {
                                "PARTIAL"
                            })
                            .into_any_element(),
                    ),
                    compact_setting_row(
                        theme,
                        "当前范围",
                        "开始菜单、桌面、系统工具与 NTFS 文件索引",
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.text_muted)
                            .child(format!("{} ITEMS", status.indexed_items))
                            .into_any_element(),
                    ),
                    compact_setting_row(
                        theme,
                        "本地文件索引",
                        file_index_state,
                        div()
                            .flex()
                            .items_center()
                            .gap(px(SPACING_6))
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(file_index_badge_color)
                                    .child(file_index_badge),
                            )
                            .child(refresh_volumes_control)
                            .into_any_element(),
                    ),
                    index_volumes_control,
                    index_exclusions_control,
                ],
            ),
        );
        let commands_page = div()
            .id(QuickLaunchSettingsPage::Commands.metadata().content_id)
            .debug_selector(|| {
                QuickLaunchSettingsPage::Commands
                    .metadata()
                    .content_id
                    .into()
            })
            .w_full()
            .child(custom_commands_control)
            .into_any_element();
        let diagnostics_page = div()
            .id(QuickLaunchSettingsPage::Diagnostics.metadata().content_id)
            .debug_selector(|| {
                QuickLaunchSettingsPage::Diagnostics
                    .metadata()
                    .content_id
                    .into()
            })
            .w_full()
            .child(history_diagnostics_control)
            .into_any_element();
        let selected_page = self.quick_launch_settings_page;
        let selected_page_content = match selected_page {
            QuickLaunchSettingsPage::General => general_page,
            QuickLaunchSettingsPage::Index => index_page,
            QuickLaunchSettingsPage::Context => context_page,
            QuickLaunchSettingsPage::Commands => commands_page,
            QuickLaunchSettingsPage::Diagnostics => diagnostics_page,
        };
        let mut settings_navigation = div()
            .id("quick-launch-settings-navigation")
            .debug_selector(|| "quick-launch-settings-navigation".into())
            .w(px(QUICK_LAUNCH_SETTINGS_NAV_WIDTH))
            .flex_none()
            .border_r_1()
            .border_color(theme.border_subtle)
            .bg(theme.canvas)
            .pt(px(SPACING_8))
            .flex()
            .flex_col();
        for page in QuickLaunchSettingsPage::ALL {
            let page_metadata = page.metadata();
            let selected = page == selected_page;
            settings_navigation = settings_navigation.child(
                div()
                    .id(page_metadata.navigation_id)
                    .debug_selector(move || page_metadata.navigation_id.into())
                    .relative()
                    .h(px(QUICK_LAUNCH_SETTINGS_NAV_ROW_HEIGHT))
                    .px(px(SPACING_12))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .bg(if selected {
                        theme.element_selected
                    } else {
                        theme.canvas
                    })
                    .hover(move |style| style.bg(theme.surface_hover))
                    .when(selected, |row| {
                        row.child(
                            div()
                                .absolute()
                                .left_0()
                                .top(px(SPACING_8))
                                .bottom(px(SPACING_8))
                                .w(px(SPACING_2))
                                .bg(theme.accent),
                        )
                    })
                    .child(
                        div()
                            .text_size(px(TEXT_BODY))
                            .line_height(px(LINE_HEIGHT_BODY))
                            .text_color(if selected {
                                theme.text
                            } else {
                                theme.text_secondary
                            })
                            .child(page_metadata.label),
                    )
                    .on_click(cx.listener(move |this, _event, _window, cx| {
                        this.select_quick_launch_settings_page(page, cx);
                    })),
            );
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                ModuleHeader::new(
                    "quick-launch-module-header",
                    "quick-launch-module-header-title",
                    "quick-launch-module-header-metadata",
                    "速启",
                    "应用、文件与系统工具",
                )
                .render(theme),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(settings_navigation)
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .id("quick-launch-settings-page-header")
                                    .debug_selector(|| "quick-launch-settings-page-header".into())
                                    .h(px(QUICK_LAUNCH_SETTINGS_PAGE_HEADER_HEIGHT))
                                    .flex_none()
                                    .px(px(SPACING_16))
                                    .border_b_1()
                                    .border_color(theme.border_subtle)
                                    .flex()
                                    .items_center()
                                    .gap(px(SPACING_12))
                                    .child(
                                        div()
                                            .debug_selector(|| {
                                                "quick-launch-settings-page-title".into()
                                            })
                                            .text_size(px(TEXT_TITLE))
                                            .line_height(px(LINE_HEIGHT_TITLE))
                                            .text_color(theme.text)
                                            .child(selected_page.metadata().label),
                                    )
                                    .child(
                                        div()
                                            .debug_selector(|| {
                                                "quick-launch-settings-page-description".into()
                                            })
                                            .min_w_0()
                                            .truncate()
                                            .text_size(px(TEXT_LABEL))
                                            .line_height(px(LINE_HEIGHT_LABEL))
                                            .text_color(theme.text_muted)
                                            .child(selected_page.metadata().description),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_h_0()
                                    .relative()
                                    .child(
                                        div()
                                            .id("quick-launch-settings-scroll")
                                            .debug_selector(|| {
                                                "quick-launch-settings-scroll".into()
                                            })
                                            .size_full()
                                            .track_scroll(&self.quick_find_scroll)
                                            .overflow_y_scroll()
                                            .on_scroll_wheel(cx.listener(
                                                |this, event: &ScrollWheelEvent, _window, cx| {
                                                    let delta = event.delta.pixel_delta(px(
                                                        QUICK_FIND_SCROLL_LINE_HEIGHT,
                                                    ));
                                                    let delta = if delta.y.as_f32() != 0.0 {
                                                        delta.y.as_f32()
                                                    } else {
                                                        delta.x.as_f32()
                                                    };
                                                    if this.smooth_scroll_quick_find(delta, cx) {
                                                        cx.stop_propagation();
                                                    }
                                                },
                                            ))
                                            .p(px(SPACING_16))
                                            .child(selected_page_content),
                                    )
                                    .child(
                                        Scrollbar::vertical(&self.quick_find_scroll)
                                            .id("quick-launch-settings-scrollbar"),
                                    ),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn render_clipboard(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let overview = self.clipboard_overview(cx);
        let settings = self.clipboard_settings(cx);
        let configuration_status = self
            .clipboard
            .upgrade()
            .map(|application| {
                application
                    .read(cx)
                    .clipboard_configuration_status()
                    .to_owned()
            })
            .unwrap_or_else(|| "不可用".into());
        let management_busy = self
            .clipboard
            .upgrade()
            .is_some_and(|application| application.read(cx).clipboard_management_busy());
        let storage_directory = self
            .clipboard
            .upgrade()
            .map(|application| application.read(cx).clipboard_storage_directory())
            .unwrap_or_else(|| "不可用".into());
        let hotkey_update_pending = self
            .clipboard
            .upgrade()
            .is_some_and(|application| application.read(cx).clipboard_hotkey_update_pending());
        let recording_ready = self
            .clipboard
            .upgrade()
            .is_some_and(|application| application.read(cx).clipboard_hotkey_recording_ready());
        let recording_active = self
            .clipboard
            .upgrade()
            .is_some_and(|application| application.read(cx).clipboard_hotkey_recording_active());
        let recording_clipboard = self.recording_hotkey == Some(HotkeyRecordingTarget::Clipboard);
        if recording_clipboard && !recording_active {
            self.recording_hotkey = None;
        }
        let hotkey_label = if hotkey_update_pending {
            "正在应用…".to_owned()
        } else if recording_clipboard {
            if recording_ready {
                "按下组合键…".to_owned()
            } else {
                "正在暂停…".to_owned()
            }
        } else {
            settings.hotkey.display.clone()
        };
        let hotkey_control = div()
            .id("record-clipboard-hotkey")
            .h(px(26.0))
            .min_w(px(116.0))
            .px(px(9.0))
            .rounded(px(4.0))
            .border_1()
            .border_color(if recording_clipboard {
                theme.accent
            } else {
                theme.border
            })
            .bg(if recording_clipboard {
                theme.accent_soft
            } else {
                theme.canvas
            })
            .cursor_pointer()
            .flex()
            .items_center()
            .justify_center()
            .font_family(UI_MONO_FONT_FAMILY)
            .text_size(px(10.5))
            .text_color(if recording_clipboard {
                theme.accent_text
            } else {
                theme.text_secondary
            })
            .child(hotkey_label)
            .on_click(cx.listener(move |this, _, window, cx| {
                if !hotkey_update_pending {
                    let recording = this.recording_hotkey != Some(HotkeyRecordingTarget::Clipboard);
                    if this.recording_hotkey.take().is_some() {
                        this.defer_all_hotkey_recording(false, cx);
                    }
                    this.recording_hotkey = recording.then_some(HotkeyRecordingTarget::Clipboard);
                    this.defer_all_hotkey_recording(recording, cx);
                    if recording {
                        this.hotkey_focus.focus(window);
                    }
                    cx.notify();
                }
            }))
            .into_any_element();
        let max_items_control = self.render_retention_stepper(
            "clipboard-max-items",
            settings.max_items,
            100,
            false,
            cx,
        );
        let retention_control = self.render_retention_stepper(
            "clipboard-retention-days",
            settings.retention_days as usize,
            1,
            true,
            cx,
        );
        let exclusions_control = div()
            .w(px(250.0))
            .h(px(28.0))
            .rounded(px(4.0))
            .border_1()
            .border_color(theme.border)
            .bg(theme.canvas)
            .px(px(7.0))
            .flex()
            .items_center()
            .child(
                Input::new(&self.exclusions_input)
                    .appearance(false)
                    .bordered(false)
                    .focus_bordered(false),
            )
            .into_any_element();
        let export_clipboard = self.clipboard.clone();
        let import_clipboard = self.clipboard.clone();
        let archive_controls = div()
            .flex()
            .items_center()
            .gap(px(5.0))
            .child(
                Button::new("clipboard-export-archive")
                    .ghost()
                    .compact()
                    .xsmall()
                    .h(px(26.0))
                    .px(px(9.0))
                    .rounded(px(4.0))
                    .border_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.surface_raised)
                    .disabled(management_busy)
                    .child(div().text_size(px(10.5)).child("导出"))
                    .on_click(move |_event, _window, cx| {
                        let _ = export_clipboard.update(cx, |application, cx| {
                            application.request_clipboard_export(cx);
                        });
                    }),
            )
            .child(
                Button::new("clipboard-import-archive")
                    .ghost()
                    .compact()
                    .xsmall()
                    .h(px(26.0))
                    .px(px(9.0))
                    .rounded(px(4.0))
                    .border_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.surface_raised)
                    .disabled(management_busy)
                    .child(div().text_size(px(10.5)).child("导入"))
                    .on_click(move |_event, _window, cx| {
                        let _ = import_clipboard.update(cx, |application, cx| {
                            application.request_clipboard_import(cx);
                        });
                    }),
            )
            .into_any_element();
        let backup_clipboard = self.clipboard.clone();
        let compact_clipboard = self.clipboard.clone();
        let database_controls = div()
            .flex()
            .items_center()
            .gap(px(5.0))
            .child(
                Button::new("clipboard-database-backup")
                    .ghost()
                    .compact()
                    .xsmall()
                    .h(px(26.0))
                    .px(px(9.0))
                    .rounded(px(4.0))
                    .border_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.surface_raised)
                    .disabled(management_busy)
                    .child(div().text_size(px(10.5)).child("备份"))
                    .on_click(move |_event, _window, cx| {
                        let _ = backup_clipboard.update(cx, |application, cx| {
                            application.request_clipboard_database_backup(cx);
                        });
                    }),
            )
            .child(
                Button::new("clipboard-database-compact")
                    .ghost()
                    .compact()
                    .xsmall()
                    .h(px(26.0))
                    .px(px(9.0))
                    .rounded(px(4.0))
                    .border_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.surface_raised)
                    .disabled(management_busy)
                    .child(div().text_size(px(10.5)).child("压缩"))
                    .on_click(move |_event, _window, cx| {
                        let _ = compact_clipboard.update(cx, |application, cx| {
                            application.request_clipboard_database_compaction(cx);
                        });
                    }),
            )
            .into_any_element();
        let migrate_clipboard = self.clipboard.clone();
        let storage_controls = Button::new("clipboard-storage-migrate")
            .ghost()
            .compact()
            .xsmall()
            .h(px(26.0))
            .px(px(9.0))
            .rounded(px(4.0))
            .border_1()
            .border_color(theme.border_subtle)
            .bg(theme.surface_raised)
            .disabled(management_busy)
            .child(div().text_size(px(10.5)).child("迁移"))
            .on_click(move |_event, _window, cx| {
                let _ = migrate_clipboard.update(cx, |application, cx| {
                    application.request_clipboard_storage_migration(cx);
                });
            })
            .into_any_element();
        let interaction_rows = vec![
            compact_setting_row(
                theme,
                "全局呼出热键",
                format!("Windows 注册状态 · {configuration_status}"),
                hotkey_control,
            ),
            compact_setting_row(
                theme,
                "Ctrl + 单击",
                "按住 Ctrl 点击历史条目时执行",
                self.render_modifier_action_selector(
                    ClipboardClickModifier::Control,
                    settings.modifier_actions.control,
                ),
            ),
            compact_setting_row(
                theme,
                "Alt + 单击",
                "按住 Alt 点击历史条目时执行",
                self.render_modifier_action_selector(
                    ClipboardClickModifier::Alt,
                    settings.modifier_actions.alt,
                ),
            ),
            compact_setting_row(
                theme,
                "Shift + 单击",
                "按住 Shift 点击历史条目时执行",
                self.render_modifier_action_selector(
                    ClipboardClickModifier::Shift,
                    settings.modifier_actions.shift,
                ),
            ),
        ];
        let policy_rows = vec![
            compact_setting_row(
                theme,
                "最大普通记录",
                "100–100000；收藏和置顶不计入淘汰队列",
                max_items_control,
            ),
            compact_setting_row(
                theme,
                "自动清理天数",
                "过期记录先进入回收状态，不立即物理删除",
                retention_control,
            ),
            compact_setting_row(
                theme,
                "排除来源进程",
                "分号或逗号分隔；失焦或回车后应用",
                exclusions_control,
            ),
            compact_setting_row(
                theme,
                "历史归档",
                "便携 .nxtclip；图片与原生格式随包迁移",
                archive_controls,
            ),
            compact_setting_row(
                theme,
                "数据库维护",
                "在线备份；压缩前自动备份",
                database_controls,
            ),
            compact_setting_row(theme, "存储位置", storage_directory, storage_controls),
        ];
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                ModuleHeader::new(
                    "clipboard-module-header",
                    "clipboard-module-header-title",
                    "clipboard-module-header-metadata",
                    "剪贴板",
                    format!(
                        "ALL {:04}  TXT {:04}  IMG {:04}  FILE {:04}",
                        overview.total, overview.text, overview.images, overview.files
                    ),
                )
                .metadata_monospace()
                .trailing(
                    div()
                        .text_size(px(TEXT_LABEL))
                        .line_height(px(LINE_HEIGHT_LABEL))
                        .text_color(if settings.enabled {
                            theme.accent_text
                        } else {
                            theme.text_muted
                        })
                        .child(if settings.enabled {
                            "● CAPTURE ON"
                        } else {
                            "○ CAPTURE OFF"
                        })
                        .into_any_element(),
                )
                .render(theme),
            )
            .child(
                div().flex_1().min_h_0().p(px(12.0)).child(
                    div()
                        .w_full()
                        .rounded(px(5.0))
                        .border_1()
                        .border_color(theme.border_subtle)
                        .bg(theme.surface)
                        .flex()
                        .child(div().flex_1().min_w_0().px(px(12.0)).child(compact_group(
                            theme,
                            "呼出与快捷操作",
                            interaction_rows,
                        )))
                        .child(div().w(px(1.0)).bg(theme.border_subtle))
                        .child(div().w(px(410.0)).px(px(12.0)).child(compact_group(
                            theme,
                            "保留与隐私",
                            policy_rows,
                        ))),
                ),
            )
            .into_any_element()
    }
}

impl Render for MainWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = match self.shell.active_module() {
            MainModule::Downloads => self.render_downloads(cx),
            MainModule::QuickFind => self.render_quick_find(cx),
            MainModule::Clipboard => self.render_clipboard(cx),
        };
        div()
            .id("nxtools-main-window")
            .debug_selector(|| "nxtools-main-window".into())
            .size_full()
            .relative()
            .overflow_hidden()
            .track_focus(&self.hotkey_focus)
            .on_key_down(cx.listener(|this, event, _window, cx| {
                if this.recording_hotkey.is_some() {
                    this.handle_hotkey_recording(event, cx);
                } else {
                    this.handle_index_exclusion_keyboard(event, cx);
                }
            }))
            .font_family(UI_FONT_FAMILY)
            .bg(self.theme.canvas)
            .text_color(self.theme.text)
            .flex()
            .flex_col()
            .child(self.render_titlebar(window))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(self.render_sidebar(cx))
                    .child(div().flex_1().min_w_0().h_full().child(content)),
            )
            .child(self.render_notifications(cx))
    }
}

fn click_action_label(action: ClipboardClickAction) -> &'static str {
    match action {
        ClipboardClickAction::OriginalPaste => "原格式粘贴",
        ClipboardClickAction::CopyOnly => "仅复制",
        ClipboardClickAction::PlainText => "纯文本／路径粘贴",
        ClipboardClickAction::WithoutLineBreaks => "去换行粘贴",
        ClipboardClickAction::Disabled => "不执行",
    }
}

fn hotkey_from_key_down(event: &KeyDownEvent) -> Option<ClipboardHotKey> {
    let modifiers = event.keystroke.modifiers;
    let mut semantic_modifiers = Vec::new();
    let mut display = Vec::new();
    if modifiers.control {
        semantic_modifiers.push(ClipboardHotKeyModifier::Control);
        display.push("Ctrl".to_owned());
    }
    if modifiers.alt {
        semantic_modifiers.push(ClipboardHotKeyModifier::Alt);
        display.push("Alt".to_owned());
    }
    if modifiers.shift {
        semantic_modifiers.push(ClipboardHotKeyModifier::Shift);
        display.push("Shift".to_owned());
    }
    if modifiers.platform {
        semantic_modifiers.push(ClipboardHotKeyModifier::Win);
        display.push("Win".to_owned());
    }
    if semantic_modifiers.is_empty() {
        return None;
    }

    let key = event.keystroke.key.trim().to_ascii_lowercase();
    let key_display = if key.len() == 1 {
        let character = key.as_bytes()[0];
        if !character.is_ascii_alphanumeric() {
            return None;
        }
        char::from(character.to_ascii_uppercase()).to_string()
    } else if let Some(number) = key
        .strip_prefix('f')
        .and_then(|value| value.parse::<u32>().ok())
        && (1..=24).contains(&number)
    {
        format!("F{number}")
    } else {
        match key.as_str() {
            "space" => "Space".into(),
            "tab" => "Tab".into(),
            "insert" => "Insert".into(),
            "delete" => "Delete".into(),
            "home" => "Home".into(),
            "end" => "End".into(),
            "pageup" => "PageUp".into(),
            "pagedown" => "PageDown".into(),
            "up" => "Up".into(),
            "down" => "Down".into(),
            "left" => "Left".into(),
            "right" => "Right".into(),
            _ => return None,
        }
    };
    display.push(key_display);
    Some(ClipboardHotKey::new(
        semantic_modifiers,
        display.last().cloned().unwrap_or_default(),
        display.join("+"),
    ))
}

fn compact_setting_row(
    theme: Theme,
    title: impl Into<gpui::SharedString>,
    description: impl Into<gpui::SharedString>,
    control: AnyElement,
) -> AnyElement {
    div()
        .min_h(px(42.0))
        .py(px(6.0))
        .border_b_1()
        .border_color(theme.border_subtle)
        .flex()
        .items_center()
        .gap(px(12.0))
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(div().text_size(px(11.5)).child(title.into()))
                .child(
                    div()
                        .text_size(px(9.5))
                        .text_color(theme.text_muted)
                        .child(description.into()),
                ),
        )
        .child(control)
        .into_any_element()
}

fn compact_toggle(
    theme: Theme,
    id: &'static str,
    enabled: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    div()
        .id(id)
        .h(px(24.0))
        .min_w(px(54.0))
        .px(px(7.0))
        .rounded(px(4.0))
        .border_1()
        .border_color(if enabled {
            theme.accent
        } else {
            theme.border_subtle
        })
        .bg(if enabled {
            theme.accent_soft
        } else {
            theme.canvas
        })
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_center()
        .gap(px(4.0))
        .text_size(px(9.5))
        .text_color(if enabled {
            theme.accent_text
        } else {
            theme.text_muted
        })
        .when(enabled, |control| {
            control.child(Icon::new(MaterialIcon::Check).size(px(11.0)))
        })
        .child(if enabled { "开启" } else { "关闭" })
        .on_click(on_click)
        .into_any_element()
}

fn quick_launch_settings_group_page(
    theme: Theme,
    selector: &'static str,
    group: AnyElement,
) -> AnyElement {
    div()
        .id(selector)
        .debug_selector(move || selector.into())
        .w_full()
        .border_1()
        .border_color(theme.border_subtle)
        .bg(theme.surface)
        .px(px(SPACING_12))
        .child(group)
        .into_any_element()
}

fn compact_group(theme: Theme, title: &'static str, rows: Vec<AnyElement>) -> AnyElement {
    div()
        .pb(px(2.0))
        .child(
            div()
                .h(px(30.0))
                .border_b_1()
                .border_color(theme.border_subtle)
                .flex()
                .items_center()
                .text_size(px(11.0))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.text_secondary)
                .child(title),
        )
        .children(rows)
        .into_any_element()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MainWindowControl {
    Minimize,
    Maximize,
    Restore,
    Close,
}

impl MainWindowControl {
    fn id(self) -> &'static str {
        match self {
            Self::Minimize => "window-control-minimize",
            Self::Maximize => "window-control-maximize",
            Self::Restore => "window-control-restore",
            Self::Close => "window-control-close",
        }
    }

    fn icon(self) -> CaptionIcon {
        match self {
            Self::Minimize => CaptionIcon::Minimize,
            Self::Maximize => CaptionIcon::Maximize,
            Self::Restore => CaptionIcon::Restore,
            Self::Close => CaptionIcon::Close,
        }
    }

    fn tooltip(self) -> &'static str {
        match self {
            Self::Minimize => "最小化",
            Self::Maximize => "最大化",
            Self::Restore => "还原",
            Self::Close => "关闭",
        }
    }
}

fn maximize_control(is_maximized: bool) -> MainWindowControl {
    if is_maximized {
        MainWindowControl::Restore
    } else {
        MainWindowControl::Maximize
    }
}

fn window_control(
    control: MainWindowControl,
    theme: Theme,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let close = matches!(control, MainWindowControl::Close);
    let id = control.id();
    let icon_selector = format!("{id}-icon");
    let tooltip_selector = format!("{id}-tooltip");
    let icon = Icon::new(control.icon()).size(px(12.0));
    let tooltip = control.tooltip();
    div()
        .id(id)
        .debug_selector(move || id.into())
        .w(px(TITLEBAR_HEIGHT))
        .h_full()
        .flex()
        .items_center()
        .justify_center()
        .text_color(theme.text_secondary)
        .hover(move |style| {
            if close {
                style.bg(gpui::rgb(0xc42b1c)).text_color(gpui::white())
            } else {
                style.bg(theme.surface_hover).text_color(theme.text)
            }
        })
        .tooltip(move |window, cx| {
            let tooltip_selector = tooltip_selector.clone();
            Tooltip::element(move |_, _| {
                let debug_selector = tooltip_selector.clone();
                div()
                    .debug_selector(move || debug_selector.clone())
                    .child(tooltip)
            })
            .build(window, cx)
        })
        .on_click(on_click)
        .child(
            div()
                .debug_selector(move || icon_selector.clone())
                .size(px(12.0))
                .child(icon),
        )
}

fn download_task_row(
    theme: Theme,
    application: gpui::WeakEntity<DownloadApplication>,
    main_window: Entity<MainWindow>,
    selection: DownloadRowSelection,
    task: DownloadEngineTask,
    live_progress: Option<DownloadProgress>,
    icon_path: Option<PathBuf>,
) -> AnyElement {
    let DownloadRowSelection {
        selected,
        visible_task_ids,
    } = selection;
    let task_id = task.task.id();
    let element_id = task_id.as_uuid().as_u64_pair().0;
    let status = task.task.status();
    let created_at_unix_ms = task.task.created_at_unix_ms();
    let name = task.task.request().category_name.as_ref().map_or_else(
        || download_task_name(&task).to_owned(),
        |category| format!("{}  ·  {category}", download_task_name(&task)),
    );
    let path = task.final_path.as_ref().map_or_else(
        || task.task.request().source_url.clone(),
        |path| path.display().to_string(),
    );
    let (status_label, status_color) = download_status_presentation(theme, status);
    let transfer = download_row_transfer_presentation(
        status,
        task.downloaded_bytes,
        task.total_bytes,
        live_progress.as_ref(),
    );
    let failed = task.error.is_some();
    let (secondary_text, secondary_color) = task.error.as_ref().map_or_else(
        || (path, theme.text_secondary),
        |error| (download_failure_summary(error).to_owned(), theme.danger),
    );
    let error_tooltip = task.error.as_ref().map(|error| error.message.clone());
    let has_icon_path = icon_path.is_some();
    let row_open_application = application.clone();
    let context_application = application;
    let click_main_window = main_window.clone();
    let click_visible_task_ids = visible_task_ids.clone();
    let right_click_main_window = main_window.clone();
    let right_click_visible_task_ids = visible_task_ids.clone();
    let context_main_window = main_window.clone();
    let context_visible_task_ids = visible_task_ids.clone();

    div()
        .id(("download-task-row", element_id))
        .debug_selector(move || format!("download-task-row-{element_id}"))
        .h(px(DOWNLOAD_TASK_ROW_HEIGHT))
        .flex_none()
        .border_b_1()
        .border_color(theme.border_subtle)
        .bg(if selected {
            theme.element_selected
        } else {
            theme.canvas
        })
        .hover(move |style| style.bg(theme.surface_hover))
        .on_click(move |event, _, cx| {
            let modifiers = event.modifiers();
            let visible_task_ids = click_visible_task_ids.clone();
            click_main_window.update(cx, |window, cx| {
                update_download_selection(
                    &mut window.selected_download_tasks,
                    &mut window.selected_download_task,
                    &mut window.download_selection_anchor,
                    visible_task_ids.as_ref(),
                    task_id,
                    modifiers.control,
                    modifiers.shift,
                );
                cx.notify();
            });
            if event.click_count() >= 2 {
                let application = row_open_application.clone();
                cx.defer(move |cx| {
                    let _ = application.update(cx, |application, cx| {
                        application.request_task_window(task_id, cx);
                    });
                });
            }
        })
        .on_mouse_down(MouseButton::Right, move |_event, _window, cx| {
            right_click_main_window.update(cx, |window, cx| {
                let targets = prepare_download_context_selection(
                    &mut window.selected_download_tasks,
                    &mut window.selected_download_task,
                    &mut window.download_selection_anchor,
                    right_click_visible_task_ids.as_ref(),
                    task_id,
                );
                info!(
                    event = "download.list.context_menu_requested",
                    selected_count = targets.len(),
                    "download task context menu requested"
                );
                cx.notify();
            });
        })
        .context_menu(move |menu, _window, cx| {
            let task_ids = context_main_window.read(cx).selected_download_tasks.clone();
            let ordered_task_ids = context_visible_task_ids
                .iter()
                .filter(|task_id| task_ids.contains(task_id))
                .copied()
                .collect::<Vec<_>>();
            let capabilities = context_application
                .upgrade()
                .map(|application| {
                    let application = application.read(cx);
                    let mut capabilities =
                        download_context_capabilities(application.tasks(), &ordered_task_ids);
                    if capabilities.can_move_queue {
                        let (top, up, down, bottom) = download_queue_placement_capabilities(
                            &application.default_queue_order(),
                            &ordered_task_ids,
                        );
                        capabilities.queue_top = top;
                        capabilities.queue_up = up;
                        capabilities.queue_down = down;
                        capabilities.queue_bottom = bottom;
                    }
                    capabilities
                })
                .unwrap_or_default();
            download_task_context_menu(
                theme,
                menu.action_context(context_main_window.read(cx).hotkey_focus.clone()),
                context_application.clone(),
                ordered_task_ids,
                capabilities,
            )
        })
        .child(
            div()
                .size_full()
                .pl(px(SPACING_8))
                .pr(px(SPACING_8))
                .border_l_2()
                .border_color(if selected {
                    theme.accent
                } else {
                    gpui::rgba(0x00000000)
                })
                .flex()
                .items_center()
                .gap(px(SPACING_8))
                .child(
                    div()
                        .size(px(26.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .when_some(icon_path, |slot, path| {
                            slot.child(
                                div()
                                    .when(selected, |icon| {
                                        icon.debug_selector(|| {
                                            "download-task-shell-icon-selected".into()
                                        })
                                    })
                                    .size(px(20.0))
                                    .child(img(path).size_full().object_fit(ObjectFit::Contain)),
                            )
                        })
                        .when(!has_icon_path, |slot| {
                            slot.child(
                                div()
                                    .when(selected, |icon| {
                                        icon.debug_selector(|| {
                                            "download-task-icon-fallback-selected".into()
                                        })
                                    })
                                    .size(px(15.0))
                                    .child(
                                        Icon::new(MaterialIcon::Download)
                                            .size_full()
                                            .text_color(theme.icon_muted),
                                    ),
                            )
                        }),
                )
                .child(
                    div()
                        .when(selected, |column| {
                            column.debug_selector(|| "download-task-name-selected".into())
                        })
                        .min_w_0()
                        .flex_1()
                        .overflow_hidden()
                        .flex()
                        .flex_col()
                        .gap(px(SPACING_2))
                        .child(
                            div()
                                .when(selected, |text| {
                                    text.debug_selector(|| {
                                        "download-task-name-text-selected".into()
                                    })
                                })
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_size(px(TEXT_BODY))
                                .line_height(px(LINE_HEIGHT_BODY))
                                .text_color(theme.text)
                                .child(name),
                        )
                        .child(
                            div()
                                .id(("download-task-secondary", element_id))
                                .when(selected, |text| {
                                    text.debug_selector(|| {
                                        "download-task-path-text-selected".into()
                                    })
                                })
                                .when(failed, |text| {
                                    text.debug_selector(|| "download-task-error-text".into())
                                })
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_size(px(TEXT_LABEL))
                                .line_height(px(LINE_HEIGHT_LABEL))
                                .text_color(secondary_color)
                                .child(secondary_text)
                                .when_some(error_tooltip, |line, message| {
                                    line.tooltip(move |window, cx| {
                                        Tooltip::element({
                                            let message = message.clone();
                                            move |_window, _cx| {
                                                div()
                                                    .debug_selector(|| {
                                                        "download-task-error-tooltip".into()
                                                    })
                                                    .w(px(DOWNLOAD_ERROR_TOOLTIP_WIDTH))
                                                    .h(px(DOWNLOAD_ERROR_TOOLTIP_MAX_HEIGHT))
                                                    .flex_none()
                                                    .overflow_hidden()
                                                    .child(
                                                        div()
                                                            .size_full()
                                                            .overflow_y_scrollbar()
                                                            .text_size(px(TEXT_LABEL))
                                                            .line_height(px(LINE_HEIGHT_LABEL))
                                                            .child(message.clone()),
                                                    )
                                            }
                                        })
                                        .max_w(px(DOWNLOAD_ERROR_TOOLTIP_SURFACE_MAX_WIDTH))
                                        .max_h(px(DOWNLOAD_ERROR_TOOLTIP_SURFACE_MAX_HEIGHT))
                                        .build(window, cx)
                                    })
                                }),
                        ),
                )
                .child(
                    div()
                        .when(selected, |column| {
                            column.debug_selector(|| "download-task-progress-selected".into())
                        })
                        .w(px(DOWNLOAD_PROGRESS_COLUMN_WIDTH))
                        .flex_none()
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .child(
                            div()
                                .h(px(15.0))
                                .flex()
                                .items_center()
                                .font_family(UI_MONO_FONT_FAMILY)
                                .text_size(px(TEXT_META))
                                .text_color(theme.text_secondary)
                                .child(
                                    div()
                                        .when(selected, |value| {
                                            value.debug_selector(|| {
                                                "download-task-transferred-selected".into()
                                            })
                                        })
                                        .w(px(DOWNLOAD_TRANSFERRED_COLUMN_WIDTH))
                                        .flex_none()
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .child(transfer.transferred),
                                )
                                .child(
                                    div()
                                        .when(selected, |value| {
                                            value.debug_selector(|| {
                                                "download-task-speed-selected".into()
                                            })
                                        })
                                        .w(px(DOWNLOAD_SPEED_COLUMN_WIDTH))
                                        .flex_none()
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .child(transfer.speed),
                                )
                                .child(
                                    div()
                                        .when(selected, |value| {
                                            value.debug_selector(|| {
                                                "download-task-remaining-selected".into()
                                            })
                                        })
                                        .w(px(DOWNLOAD_REMAINING_COLUMN_WIDTH))
                                        .flex_none()
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .child(transfer.remaining),
                                ),
                        )
                        .child(
                            div()
                                .when(selected, |track| {
                                    track.debug_selector(|| {
                                        "download-task-progress-track-selected".into()
                                    })
                                })
                                .w_full()
                                .h(px(3.0))
                                .rounded(px(2.0))
                                .bg(theme.surface_raised)
                                .child(
                                    div()
                                        .w(px(DOWNLOAD_PROGRESS_COLUMN_WIDTH * transfer.fraction))
                                        .h_full()
                                        .rounded(px(2.0))
                                        .bg(status_color),
                                ),
                        ),
                )
                .child(
                    div()
                        .when(selected, |column| {
                            column.debug_selector(|| "download-task-status-selected".into())
                        })
                        .w(px(DOWNLOAD_STATUS_COLUMN_WIDTH))
                        .flex_none()
                        .text_size(px(TEXT_LABEL))
                        .line_height(px(LINE_HEIGHT_LABEL))
                        .text_color(status_color)
                        .child(status_label),
                )
                .child(
                    div()
                        .id(("download-task-created", element_id))
                        .when(selected, |column| {
                            column.debug_selector(|| "download-task-created-selected".into())
                        })
                        .w(px(DOWNLOAD_CREATED_COLUMN_WIDTH))
                        .flex_none()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .font_family(UI_MONO_FONT_FAMILY)
                        .text_size(px(TEXT_LABEL))
                        .line_height(px(LINE_HEIGHT_LABEL))
                        .text_color(theme.text_secondary)
                        .tooltip(move |window, cx| {
                            Tooltip::new(format_download_timestamp(created_at_unix_ms))
                                .build(window, cx)
                        })
                        .child(format_download_created_compact(created_at_unix_ms)),
                ),
        )
        .into_any_element()
}

fn download_popup_item(
    theme: Theme,
    selector: &'static str,
    label: &'static str,
    icon: MaterialIcon,
    tone: MenuTone,
) -> PopupMenuItem {
    context_menu_item(theme, selector, label, icon, tone)
}

fn download_task_context_menu(
    theme: Theme,
    menu: PopupMenu,
    application: gpui::WeakEntity<DownloadApplication>,
    task_ids: Vec<DownloadTaskId>,
    capabilities: DownloadContextCapabilities,
) -> PopupMenu {
    let plan = download_context_menu_plan(&capabilities, task_ids.len());
    let artifact_task = capabilities.single_artifact;
    let batch_item = |selector: &'static str,
                      label: &'static str,
                      action: DownloadBatchAction,
                      icon: MaterialIcon,
                      targets: Vec<DownloadTaskId>| {
        let application = application.clone();
        download_popup_item(theme, selector, label, icon, MenuTone::Normal).on_click(
            move |_event, _window, cx| {
                let application = application.clone();
                let targets = targets.clone();
                cx.defer(move |cx| {
                    let _ = application.update(cx, |application, cx| {
                        application.batch_action(targets, action, cx);
                    });
                });
            },
        )
    };
    let queue_item = |selector: &'static str,
                      label: &'static str,
                      placement: DownloadQueuePlacement,
                      icon: MaterialIcon| {
        let application = application.clone();
        let task_ids = task_ids.clone();
        download_popup_item(theme, selector, label, icon, MenuTone::Normal).on_click(
            move |_event, _window, cx| {
                let application = application.clone();
                let task_ids = task_ids.clone();
                cx.defer(move |cx| {
                    let _ = application.update(cx, |application, cx| {
                        application.move_queue_tasks(task_ids, placement, cx);
                    });
                });
            },
        )
    };
    let artifact_item = |task_id: DownloadTaskId,
                         selector: &'static str,
                         label: &'static str,
                         action: ShellOpenAction,
                         icon: MaterialIcon| {
        let application = application.clone();
        download_popup_item(theme, selector, label, icon, MenuTone::Normal).on_click(
            move |_event, _window, cx| {
                let application = application.clone();
                cx.defer(move |cx| {
                    let _ = application.update(cx, |application, cx| {
                        application.open_task_artifact(task_id, action, cx);
                    });
                });
            },
        )
    };
    let progress_application = application.clone();
    let progress_task = (task_ids.len() == 1).then(|| task_ids[0]);
    let delete_application = application.clone();
    let mut menu = context_menu_shell(menu);
    let mut has_visible_items = false;

    if plan.artifact {
        let task_id = artifact_task.expect("artifact menu plan requires one completed artifact");
        menu = menu
            .item(artifact_item(
                task_id,
                "download-menu-open-file",
                "打开文件",
                ShellOpenAction::Open,
                MaterialIcon::OpenInNew,
            ))
            .item(artifact_item(
                task_id,
                "download-menu-reveal-file",
                "打开所在目录",
                ShellOpenAction::SelectInExplorer,
                MaterialIcon::Folder,
            ));
        has_visible_items = true;
    }
    if plan.task_window {
        let task_id = progress_task.expect("task-window menu plan requires one selected task");
        menu = menu.item(
            download_popup_item(
                theme,
                "download-menu-task-window",
                "任务窗口",
                MaterialIcon::Info,
                MenuTone::Normal,
            )
            .on_click(move |_event, _window, cx| {
                let application = progress_application.clone();
                cx.defer(move |cx| {
                    let _ = application.update(cx, |application, cx| {
                        application.request_task_window(task_id, cx);
                    });
                });
            }),
        );
        has_visible_items = true;
    }
    if plan.resume || plan.pause || plan.cancel {
        if has_visible_items {
            menu = menu.separator();
        }
        if plan.resume {
            menu = menu.item(batch_item(
                "download-menu-resume",
                "开始 / 继续",
                DownloadBatchAction::Resume,
                MaterialIcon::Download,
                capabilities.resume_targets.clone(),
            ));
        }
        if plan.pause {
            menu = menu.item(batch_item(
                "download-menu-pause",
                "暂停",
                DownloadBatchAction::Pause,
                MaterialIcon::History,
                capabilities.pause_targets.clone(),
            ));
        }
        if plan.cancel {
            menu = menu.item(batch_item(
                "download-menu-cancel",
                "取消",
                DownloadBatchAction::Cancel,
                MaterialIcon::Cancel,
                capabilities.cancel_targets.clone(),
            ));
        }
        has_visible_items = true;
    }
    if plan.queue_top || plan.queue_up || plan.queue_down || plan.queue_bottom {
        if has_visible_items {
            menu = menu.separator();
        }
        if plan.queue_top {
            menu = menu.item(queue_item(
                "download-menu-prioritize",
                "优先下载",
                DownloadQueuePlacement::Top,
                MaterialIcon::ArrowUpward,
            ));
        }
        if plan.queue_up {
            menu = menu.item(queue_item(
                "download-menu-move-up",
                "上移",
                DownloadQueuePlacement::Up,
                MaterialIcon::ArrowUpward,
            ));
        }
        if plan.queue_down {
            menu = menu.item(queue_item(
                "download-menu-move-down",
                "下移",
                DownloadQueuePlacement::Down,
                MaterialIcon::History,
            ));
        }
        if plan.queue_bottom {
            menu = menu.item(queue_item(
                "download-menu-move-bottom",
                "移到底部",
                DownloadQueuePlacement::Bottom,
                MaterialIcon::History,
            ));
        }
        has_visible_items = true;
    }
    if (plan.artifact || plan.remove) && has_visible_items {
        menu = menu.separator();
    }
    if plan.artifact {
        let task_id = artifact_task.expect("delete-file menu plan requires one completed artifact");
        menu = menu.item(
            download_popup_item(
                theme,
                "download-menu-delete-file",
                "删除文件",
                MaterialIcon::Delete,
                MenuTone::Danger,
            )
            .on_click(move |_event, _window, cx| {
                let application = delete_application.clone();
                cx.defer(move |cx| {
                    let _ = application.update(cx, |application, cx| {
                        application.request_delete_confirmation(task_id, cx);
                    });
                });
            }),
        );
        has_visible_items = true;
    }
    if plan.remove {
        let application = application.clone();
        menu = menu.item(
            download_popup_item(
                theme,
                "download-menu-remove",
                "从列表移除",
                MaterialIcon::Close,
                MenuTone::Danger,
            )
            .on_click(move |_event, _window, cx| {
                let application = application.clone();
                let task_ids = task_ids.clone();
                cx.defer(move |cx| {
                    let _ = application.update(cx, |application, cx| {
                        application.batch_action(task_ids, DownloadBatchAction::Remove, cx);
                    });
                });
            }),
        );
        has_visible_items = true;
    }
    debug_assert!(
        has_visible_items,
        "task context menu requires a visible action"
    );
    menu
}

fn download_batch_toolbar(
    theme: Theme,
    application: gpui::WeakEntity<DownloadApplication>,
    main_window: Entity<MainWindow>,
    task_ids: Vec<DownloadTaskId>,
    capabilities: DownloadContextCapabilities,
) -> AnyElement {
    let plan = download_context_menu_plan(&capabilities, task_ids.len());
    let batch_action = |id: &'static str,
                        icon: MaterialIcon,
                        tooltip: &'static str,
                        action: DownloadBatchAction,
                        targets: Vec<DownloadTaskId>,
                        danger: bool| {
        let application = application.clone();
        let button = icon_action(id, icon, tooltip)
            .ghost()
            .when(danger, |button| button.danger())
            .on_click(move |_, _, cx| {
                let application = application.clone();
                let targets = targets.clone();
                cx.defer(move |cx| {
                    let _ = application.update(cx, |application, cx| {
                        application.batch_action(targets, action, cx);
                    });
                });
            });
        div()
            .debug_selector(move || id.into())
            .size(px(CONTROL_HEIGHT_ACTION))
            .child(button)
            .into_any_element()
    };
    let queue_action = |id: &'static str,
                        icon: MaterialIcon,
                        tooltip: &'static str,
                        placement: DownloadQueuePlacement| {
        let application = application.clone();
        let task_ids = task_ids.clone();
        let button = icon_action(id, icon, tooltip)
            .ghost()
            .on_click(move |_, _, cx| {
                let application = application.clone();
                let task_ids = task_ids.clone();
                cx.defer(move |cx| {
                    let _ = application.update(cx, |application, cx| {
                        application.move_queue_tasks(task_ids, placement, cx);
                    });
                });
            });
        div()
            .debug_selector(move || id.into())
            .size(px(CONTROL_HEIGHT_ACTION))
            .child(button)
            .into_any_element()
    };
    let clear_selection = main_window;
    let has_queue_actions = plan.queue_top || plan.queue_up || plan.queue_down || plan.queue_bottom;

    div()
        .debug_selector(|| "download-batch-toolbar".into())
        .h(px(DOWNLOAD_BATCH_TOOLBAR_HEIGHT))
        .px(px(12.0))
        .flex_none()
        .border_b_1()
        .border_color(theme.border_subtle)
        .bg(theme.surface)
        .flex()
        .items_center()
        .gap(px(4.0))
        .child(
            div()
                .mr(px(6.0))
                .text_size(px(TEXT_LABEL))
                .line_height(px(LINE_HEIGHT_LABEL))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme.text_secondary)
                .child(format!("已选择 {} 项", task_ids.len())),
        )
        .when(plan.resume, |toolbar| {
            toolbar.child(batch_action(
                "download-batch-resume",
                MaterialIcon::Download,
                "开始 / 继续可运行任务",
                DownloadBatchAction::Resume,
                capabilities.resume_targets.clone(),
                false,
            ))
        })
        .when(plan.pause, |toolbar| {
            toolbar.child(batch_action(
                "download-batch-pause",
                MaterialIcon::History,
                "暂停可暂停任务",
                DownloadBatchAction::Pause,
                capabilities.pause_targets.clone(),
                false,
            ))
        })
        .when(plan.cancel, |toolbar| {
            toolbar.child(batch_action(
                "download-batch-cancel",
                MaterialIcon::Cancel,
                "取消可取消任务",
                DownloadBatchAction::Cancel,
                capabilities.cancel_targets.clone(),
                false,
            ))
        })
        .when(has_queue_actions, |toolbar| {
            toolbar.child(
                div()
                    .mx(px(4.0))
                    .h(px(18.0))
                    .w(px(1.0))
                    .bg(theme.border_subtle),
            )
        })
        .when(plan.queue_top, |toolbar| {
            toolbar.child(queue_action(
                "download-batch-queue-top",
                MaterialIcon::ArrowUpward,
                "移到队列顶部",
                DownloadQueuePlacement::Top,
            ))
        })
        .when(plan.queue_up, |toolbar| {
            toolbar.child(queue_action(
                "download-batch-queue-up",
                MaterialIcon::ArrowUpward,
                "队列上移",
                DownloadQueuePlacement::Up,
            ))
        })
        .when(plan.queue_down, |toolbar| {
            toolbar.child(queue_action(
                "download-batch-queue-down",
                MaterialIcon::History,
                "队列下移",
                DownloadQueuePlacement::Down,
            ))
        })
        .when(plan.queue_bottom, |toolbar| {
            toolbar.child(queue_action(
                "download-batch-queue-bottom",
                MaterialIcon::History,
                "移到队列底部",
                DownloadQueuePlacement::Bottom,
            ))
        })
        .when(plan.remove, |toolbar| {
            toolbar.child(batch_action(
                "download-batch-remove",
                MaterialIcon::Delete,
                "从列表移除",
                DownloadBatchAction::Remove,
                task_ids.clone(),
                true,
            ))
        })
        .child(div().flex_1())
        .child({
            let button = icon_action("download-batch-clear", MaterialIcon::Close, "清除多选")
                .ghost()
                .on_click(move |_, _, cx| {
                    clear_selection.update(cx, |window, cx| {
                        window.selected_download_tasks.clear();
                        window.selected_download_task = None;
                        window.download_selection_anchor = None;
                        window.requested_download_detail = None;
                        window
                            .download_detail_scroll
                            .set_offset(point(px(0.0), px(0.0)));
                        cx.notify();
                    });
                });
            div()
                .debug_selector(|| "download-batch-clear".into())
                .size(px(CONTROL_HEIGHT_ACTION))
                .child(button)
        })
        .into_any_element()
}

fn download_task_detail(theme: Theme, view: DownloadDetailView<'_>) -> AnyElement {
    let DownloadDetailView {
        task,
        detail,
        detail_error,
        progress,
        active_tab,
        scroll,
        main_window,
    } = view;
    let task_id = task.task.id();
    let mut tabs = div()
        .debug_selector(|| "download-detail-tabs".into())
        .h(px(DOWNLOAD_DETAIL_TAB_HEIGHT))
        .px(px(10.0))
        .border_b_1()
        .border_color(theme.border_subtle)
        .flex()
        .items_end()
        .gap(px(2.0));
    for tab in DownloadDetailTab::ALL {
        let active = tab == active_tab;
        let tab_main_window = main_window.clone();
        let tab_scroll = scroll.clone();
        tabs = tabs.child(
            div()
                .id(("download-detail-tab", tab as usize))
                .debug_selector(move || format!("download-detail-tab-{}", tab.diagnostic_name()))
                .h(px(29.0))
                .px(px(10.0))
                .border_b_2()
                .border_color(if active {
                    theme.accent
                } else {
                    gpui::rgba(0x00000000)
                })
                .cursor_pointer()
                .flex()
                .items_center()
                .text_size(px(TEXT_LABEL))
                .line_height(px(LINE_HEIGHT_LABEL))
                .font_weight(if active {
                    gpui::FontWeight::SEMIBOLD
                } else {
                    gpui::FontWeight::NORMAL
                })
                .text_color(if active { theme.text } else { theme.text_muted })
                .hover(move |style| style.bg(theme.surface_hover))
                .child(tab.label())
                .on_click(move |_, _, cx| {
                    tab_main_window.update(cx, |window, cx| {
                        if window.download_detail_tab != tab {
                            window.download_detail_tab = tab;
                            tab_scroll.set_offset(point(px(0.0), px(0.0)));
                            info!(
                                event = "download.detail.tab_changed",
                                task_id = %task_id.as_uuid(),
                                tab = tab.diagnostic_name(),
                                "download detail tab changed"
                            );
                            cx.notify();
                        }
                    });
                }),
        );
    }
    let content = match active_tab {
        DownloadDetailTab::Overview => download_overview_detail(theme, task, progress),
        DownloadDetailTab::Segments => download_segment_detail(theme, detail, detail_error),
        DownloadDetailTab::Headers => download_header_detail(theme, task),
        DownloadDetailTab::Verification => {
            download_verification_detail(theme, task, detail, detail_error)
        }
        DownloadDetailTab::Timeline => download_timeline_detail(theme, detail, detail_error),
    };

    div()
        .debug_selector(|| "download-task-detail".into())
        .h(px(DOWNLOAD_DETAIL_HEIGHT))
        .flex_none()
        .border_t_1()
        .border_color(theme.border_subtle)
        .bg(theme.surface)
        .flex()
        .flex_col()
        .child(tabs)
        .child(
            div()
                .relative()
                .flex_1()
                .min_h_0()
                .child(
                    div()
                        .id("download-detail-scroll")
                        .debug_selector(|| "download-detail-scroll".into())
                        .size_full()
                        .track_scroll(&scroll)
                        .overflow_y_scroll()
                        .px(px(12.0))
                        .py(px(8.0))
                        .child(content),
                )
                .child(Scrollbar::vertical(&scroll).id("download-detail-scrollbar")),
        )
        .into_any_element()
}

fn download_overview_detail(
    theme: Theme,
    task: &DownloadEngineTask,
    progress: Option<&DownloadProgress>,
) -> AnyElement {
    let request = task.task.request();
    let total = task.total_bytes;
    let downloaded = progress.map_or(task.downloaded_bytes, |progress| progress.downloaded_bytes);
    let bytes_per_second = progress.map_or(0, |progress| progress.bytes_per_second);
    let active_connections = progress.map_or(0, |progress| progress.active_connections);
    let remaining = total
        .filter(|total| bytes_per_second > 0 && *total > downloaded)
        .map(|total| format_duration((total - downloaded) / bytes_per_second))
        .unwrap_or_else(|| "—".into());
    let final_path = task
        .final_path
        .as_ref()
        .map_or_else(|| "尚未解析".into(), |path| path.display().to_string());
    let (status_label, status_color) = download_status_presentation(theme, task.task.status());

    let primary = div()
        .debug_selector(|| "download-detail-overview-primary".into())
        .min_w_0()
        .flex_1()
        .flex()
        .flex_col()
        .child(download_detail_field(
            theme,
            "download-detail-field-status",
            "状态",
            status_label.into(),
            status_color,
        ))
        .child(download_detail_field(
            theme,
            "download-detail-field-downloaded",
            "已下载",
            format_download_bytes(downloaded),
            theme.text,
        ))
        .child(download_detail_field(
            theme,
            "download-detail-field-remaining",
            "剩余",
            remaining,
            theme.text,
        ))
        .child(download_detail_field(
            theme,
            "download-detail-field-resume",
            "断点",
            task.probe.as_ref().map_or_else(
                || "未知".into(),
                |probe| {
                    if probe.supports_ranges {
                        "支持".into()
                    } else {
                        "不支持".into()
                    }
                },
            ),
            theme.text_secondary,
        ))
        .child(download_detail_field(
            theme,
            "download-detail-field-category",
            "分类",
            request
                .category_name
                .clone()
                .unwrap_or_else(|| "未分类".into()),
            theme.text_secondary,
        ));
    let secondary = div()
        .debug_selector(|| "download-detail-overview-secondary".into())
        .min_w_0()
        .flex_1()
        .flex()
        .flex_col()
        .child(download_detail_field(
            theme,
            "download-detail-field-size",
            "大小",
            total.map_or_else(|| "未知".into(), format_download_bytes),
            theme.text,
        ))
        .child(download_detail_field(
            theme,
            "download-detail-field-speed",
            "速度",
            if bytes_per_second == 0 {
                "—".into()
            } else {
                format!("{}/s", format_download_bytes(bytes_per_second))
            },
            theme.text,
        ))
        .child(download_detail_field(
            theme,
            "download-detail-field-connections",
            "连接",
            active_connections.to_string(),
            theme.text,
        ))
        .child(download_detail_field(
            theme,
            "download-detail-field-origin",
            "来源",
            download_origin_label(task.task.origin()).into(),
            theme.text_secondary,
        ))
        .child(download_detail_field(
            theme,
            "download-detail-field-schedule",
            "计划",
            request
                .scheduled_at_unix_ms
                .map_or_else(|| "立即".into(), format_download_timestamp),
            theme.text_secondary,
        ));

    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(div().flex().gap(px(24.0)).child(primary).child(secondary))
        .when_some(task.error.as_ref(), |content, error| {
            content.child(download_detail_line_with_color(
                theme,
                "错误",
                error.message.clone(),
                theme.danger,
            ))
        })
        .child(download_detail_line(
            theme,
            "URL",
            request.source_url.clone(),
        ))
        .child(download_detail_line(theme, "文件", final_path))
        .into_any_element()
}

fn download_segment_detail(
    theme: Theme,
    detail: Option<&DownloadTaskDetail>,
    detail_error: Option<&str>,
) -> AnyElement {
    let Some(detail) = detail else {
        return detail_error.map_or_else(
            || {
                progress_state(
                    theme,
                    "download-detail-segments-progress",
                    "正在加载分段详情",
                    "正在读取持久化分段与连接状态。",
                )
                .into_any_element()
            },
            |error| {
                inline_error_state(
                    theme,
                    "download-detail-segments-error",
                    "分段详情加载失败",
                    error.to_owned(),
                )
                .into_any_element()
            },
        );
    };
    if detail.segments.is_empty() {
        return empty_state(
            theme,
            "download-detail-segments-empty",
            MaterialIcon::Info,
            "尚未生成分段计划",
            "单连接任务或尚未开始的任务不会显示分段。",
        )
        .into_any_element();
    }
    let total_bytes = detail
        .segments
        .iter()
        .map(DownloadSegment::len)
        .sum::<u64>();
    let downloaded_bytes = detail
        .segments
        .iter()
        .map(DownloadSegment::downloaded_bytes)
        .sum::<u64>();
    let mut aggregate = div()
        .h(px(5.0))
        .w_full()
        .rounded(px(3.0))
        .overflow_hidden()
        .bg(theme.surface_raised)
        .flex();
    for cell in download_segment_strip_cells(&detail.segments, 96) {
        aggregate = aggregate.child(div().flex_1().h_full().bg(if cell.downloaded {
            download_segment_color(theme, cell.state)
        } else {
            theme.surface_hover
        }));
    }
    let mut rows = div().flex().flex_col();
    for segment in &detail.segments {
        let percent = segment.downloaded_bytes().saturating_mul(100) / segment.len();
        rows = rows.child(
            div()
                .h(px(24.0))
                .border_b_1()
                .border_color(theme.border_subtle)
                .flex()
                .items_center()
                .text_size(px(TEXT_LABEL))
                .line_height(px(LINE_HEIGHT_LABEL))
                .child(
                    div()
                        .w(px(38.0))
                        .text_color(theme.text_muted)
                        .child(format!("#{}", segment.index() + 1)),
                )
                .child(
                    div()
                        .w(px(172.0))
                        .text_color(theme.text_secondary)
                        .child(format!(
                            "{}–{}",
                            format_download_bytes(segment.start_byte()),
                            format_download_bytes(segment.end_byte().saturating_add(1))
                        )),
                )
                .child(div().w(px(180.0)).text_color(theme.text).child(format!(
                    "{} / {}",
                    format_download_bytes(segment.downloaded_bytes()),
                    format_download_bytes(segment.len())
                )))
                .child(
                    div()
                        .w(px(72.0))
                        .text_color(download_segment_color(theme, segment.state))
                        .child(download_segment_state_label(segment.state)),
                )
                .child(
                    div()
                        .text_color(theme.text_muted)
                        .child(format!("{percent}%")),
                ),
        );
    }
    div()
        .flex()
        .flex_col()
        .gap(px(7.0))
        .child(
            div()
                .text_size(px(TEXT_LABEL))
                .line_height(px(LINE_HEIGHT_LABEL))
                .text_color(theme.text_muted)
                .child(format!(
                    "{} 个分段 · {} / {}",
                    detail.segments.len(),
                    format_download_bytes(downloaded_bytes),
                    format_download_bytes(total_bytes)
                )),
        )
        .child(aggregate)
        .child(rows)
        .into_any_element()
}

fn download_header_detail(theme: Theme, task: &DownloadEngineTask) -> AnyElement {
    let request = task.task.request();
    let mut request_rows = Vec::new();
    for header in &request.headers {
        request_rows.push((
            header.name.clone(),
            masked_request_header_value(&header.name, &header.value),
        ));
    }
    if request.cookie.is_some() {
        request_rows.push(("Cookie".into(), "已设置（内容已隐藏）".into()));
    }
    if let Some(referrer) = &request.referrer {
        request_rows.push(("Referer".into(), referrer.clone()));
    }
    if request_rows.is_empty() {
        request_rows.push(("请求 Header".into(), "未设置自定义 Header".into()));
    }
    let mut response_rows = Vec::new();
    if let Some(probe) = &task.probe {
        response_rows.push(("Final URL".into(), probe.final_url.clone()));
        response_rows.push((
            "Content-Type".into(),
            probe.content_type.clone().unwrap_or_else(|| "未知".into()),
        ));
        response_rows.push((
            "Content-Length".into(),
            probe
                .total_bytes
                .map_or_else(|| "未知".into(), format_download_bytes),
        ));
        response_rows.push((
            "Accept-Ranges".into(),
            if probe.supports_ranges {
                "bytes"
            } else {
                "否"
            }
            .into(),
        ));
        if let Some(etag) = &probe.validator.etag {
            response_rows.push(("ETag".into(), etag.clone()));
        }
        if let Some(last_modified) = &probe.validator.last_modified {
            response_rows.push(("Last-Modified".into(), last_modified.clone()));
        }
    } else {
        response_rows.push(("响应 Header".into(), "尚未探测".into()));
    }
    div()
        .flex()
        .gap(px(24.0))
        .child(download_detail_section(theme, "请求", request_rows))
        .child(download_detail_section(theme, "响应", response_rows))
        .into_any_element()
}

fn download_verification_detail(
    theme: Theme,
    task: &DownloadEngineTask,
    detail: Option<&DownloadTaskDetail>,
    detail_error: Option<&str>,
) -> AnyElement {
    let validator = task.probe.as_ref().map(|probe| &probe.validator);
    let identity = validator.map_or_else(
        || "尚未探测资源身份".into(),
        |validator| match (&validator.etag, &validator.last_modified) {
            (Some(_), Some(_)) => "ETag + Last-Modified".into(),
            (Some(_), None) => "ETag".into(),
            (None, Some(_)) => "Last-Modified".into(),
            (None, None) => "服务器未提供资源身份标识".into(),
        },
    );
    let segment_state = detail.map_or_else(
        || detail_error.unwrap_or("正在加载").to_owned(),
        |detail| {
            if detail.segments.is_empty() {
                "单连接或尚未生成计划".into()
            } else if detail
                .segments
                .iter()
                .all(|segment| segment.downloaded_bytes() == segment.len())
            {
                "所有分段长度一致".into()
            } else {
                "分段边界已锁定，传输尚未完成".into()
            }
        },
    );
    let (integrity, color) = match task.task.status() {
        DownloadStatus::Completed => ("传输完整性已通过", theme.success),
        DownloadStatus::Verifying => ("正在校验落盘结果", theme.accent),
        DownloadStatus::Failed => ("任务失败，请查看错误与日志", theme.danger),
        _ => ("等待传输完成", theme.text_secondary),
    };
    div()
        .flex()
        .flex_col()
        .gap(px(5.0))
        .child(download_detail_line_with_color(
            theme,
            "状态",
            integrity.into(),
            color,
        ))
        .child(download_detail_line(theme, "资源", identity))
        .child(download_detail_line(theme, "分段", segment_state))
        .child(download_detail_line(
            theme,
            "范围",
            task.probe.as_ref().map_or_else(
                || "未知".into(),
                |probe| {
                    if probe.supports_ranges {
                        "Content-Range 严格校验".into()
                    } else {
                        "单连接长度校验".into()
                    }
                },
            ),
        ))
        .into_any_element()
}

fn download_timeline_detail(
    theme: Theme,
    detail: Option<&DownloadTaskDetail>,
    detail_error: Option<&str>,
) -> AnyElement {
    let Some(detail) = detail else {
        return detail_error.map_or_else(
            || {
                progress_state(
                    theme,
                    "download-detail-log-progress",
                    "正在加载任务日志",
                    "正在读取该任务的有界事件时间线。",
                )
                .into_any_element()
            },
            |error| {
                inline_error_state(
                    theme,
                    "download-detail-log-error",
                    "任务日志加载失败",
                    error.to_owned(),
                )
                .into_any_element()
            },
        );
    };
    if detail.timeline.is_empty() {
        return empty_state(
            theme,
            "download-detail-log-empty",
            MaterialIcon::History,
            "暂无任务日志",
            "任务产生状态变化后会在这里显示有界事件。",
        )
        .into_any_element();
    }
    let mut rows = div().flex().flex_col();
    for entry in &detail.timeline {
        rows = rows.child(
            div()
                .min_h(px(24.0))
                .py(px(4.0))
                .border_b_1()
                .border_color(theme.border_subtle)
                .flex()
                .items_start()
                .gap(px(10.0))
                .text_size(px(TEXT_LABEL))
                .line_height(px(LINE_HEIGHT_LABEL))
                .child(
                    div()
                        .w(px(64.0))
                        .flex_none()
                        .text_color(theme.text_muted)
                        .child(format_download_log_time(entry.occurred_at_unix_ms)),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .text_color(theme.text_secondary)
                        .child(download_timeline_label(entry)),
                ),
        );
    }
    rows.into_any_element()
}

fn download_detail_section(
    theme: Theme,
    title: &'static str,
    rows: Vec<(String, String)>,
) -> AnyElement {
    let mut section = div()
        .min_w_0()
        .flex_1()
        .flex()
        .flex_col()
        .gap(px(3.0))
        .child(
            div()
                .pb(px(3.0))
                .text_size(px(TEXT_LABEL))
                .line_height(px(LINE_HEIGHT_LABEL))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.text_secondary)
                .child(title),
        );
    for (label, value) in rows {
        section = section.child(download_detail_line_owned(theme, label, value));
    }
    section.into_any_element()
}

fn download_detail_field(
    theme: Theme,
    selector: &'static str,
    label: &'static str,
    value: String,
    value_color: gpui::Rgba,
) -> AnyElement {
    let label_selector = format!("{selector}-label");
    let value_selector = format!("{selector}-value");
    div()
        .debug_selector(move || selector.into())
        .min_h(px(20.0))
        .min_w_0()
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(
            div()
                .debug_selector(move || label_selector.clone())
                .w(px(DOWNLOAD_DETAIL_LABEL_WIDTH))
                .flex_none()
                .text_size(px(TEXT_LABEL))
                .line_height(px(LINE_HEIGHT_LABEL))
                .text_color(theme.text_muted)
                .child(label),
        )
        .child(
            div()
                .debug_selector(move || value_selector.clone())
                .min_w_0()
                .flex_1()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_size(px(TEXT_BODY))
                .line_height(px(LINE_HEIGHT_BODY))
                .text_color(value_color)
                .child(value),
        )
        .into_any_element()
}

fn download_detail_line(theme: Theme, label: &'static str, value: String) -> AnyElement {
    download_detail_line_with_color(theme, label, value, theme.text)
}

fn download_detail_line_owned(theme: Theme, label: String, value: String) -> AnyElement {
    div()
        .min_h(px(19.0))
        .flex()
        .items_start()
        .gap(px(8.0))
        .text_size(px(TEXT_BODY))
        .line_height(px(LINE_HEIGHT_BODY))
        .child(
            div()
                .w(px(92.0))
                .flex_none()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_color(theme.text_muted)
                .child(label),
        )
        .child(div().min_w_0().flex_1().text_color(theme.text).child(value))
        .into_any_element()
}

fn download_detail_line_with_color(
    theme: Theme,
    label: &'static str,
    value: String,
    value_color: gpui::Rgba,
) -> AnyElement {
    div()
        .when(label == "错误", |line| {
            line.debug_selector(|| "download-task-detail-error".into())
        })
        .min_h(px(19.0))
        .flex()
        .items_start()
        .gap(px(8.0))
        .text_size(px(TEXT_BODY))
        .line_height(px(LINE_HEIGHT_BODY))
        .child(
            div()
                .w(px(DOWNLOAD_DETAIL_LABEL_WIDTH))
                .flex_none()
                .text_color(theme.text_muted)
                .child(label),
        )
        .child(
            div()
                .min_w_0()
                .flex_1()
                .text_color(value_color)
                .child(value),
        )
        .into_any_element()
}

fn masked_request_header_value(name: &str, value: &str) -> String {
    let name = name.trim().to_ascii_lowercase();
    let normalized = name
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>();
    if matches!(
        name.as_str(),
        "authorization" | "proxy-authorization" | "cookie" | "set-cookie" | "x-api-key" | "api-key"
    ) || name.contains("auth")
        || name.contains("token")
        || name.contains("secret")
        || name.ends_with("-key")
        || normalized.contains("apikey")
    {
        "已设置（内容已隐藏）".into()
    } else {
        value.to_owned()
    }
}

fn download_origin_label(origin: RequestOrigin) -> &'static str {
    match origin {
        RequestOrigin::Ui => "界面",
        RequestOrigin::Cli => "CLI",
        RequestOrigin::Browser => "浏览器",
        RequestOrigin::Internal => "内部",
    }
}

fn download_segment_state_label(state: DownloadSegmentState) -> &'static str {
    match state {
        DownloadSegmentState::Waiting => "等待",
        DownloadSegmentState::Active => "下载中",
        DownloadSegmentState::Completed => "完成",
    }
}

fn download_segment_color(theme: Theme, state: DownloadSegmentState) -> gpui::Rgba {
    match state {
        DownloadSegmentState::Waiting => theme.text_muted,
        DownloadSegmentState::Active => theme.accent,
        DownloadSegmentState::Completed => theme.success,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DownloadSegmentStripCell {
    segment_index: u32,
    state: DownloadSegmentState,
    downloaded: bool,
}

fn download_segment_strip_cells(
    segments: &[DownloadSegment],
    cell_count: usize,
) -> Vec<DownloadSegmentStripCell> {
    let total_bytes = segments.iter().map(DownloadSegment::len).sum::<u64>();
    if total_bytes == 0 || cell_count == 0 {
        return Vec::new();
    }
    let mut cells = Vec::with_capacity(cell_count);
    for cell_index in 0..cell_count {
        let midpoint = (u128::from(total_bytes)
            * u128::try_from(cell_index.saturating_mul(2).saturating_add(1)).unwrap_or(u128::MAX)
            / u128::try_from(cell_count.saturating_mul(2)).unwrap_or(1))
        .min(u128::from(total_bytes.saturating_sub(1))) as u64;
        let mut preceding_bytes = 0_u64;
        if let Some(segment) = segments.iter().find(|segment| {
            preceding_bytes = preceding_bytes.saturating_add(segment.len());
            midpoint < preceding_bytes
        }) {
            let segment_start = preceding_bytes.saturating_sub(segment.len());
            cells.push(DownloadSegmentStripCell {
                segment_index: segment.index(),
                state: segment.state,
                downloaded: midpoint.saturating_sub(segment_start) < segment.downloaded_bytes(),
            });
        }
    }
    cells
}

fn download_timeline_label(entry: &DownloadTaskTimelineEntry) -> String {
    match entry.event {
        DownloadTaskTimelineEvent::TaskCreated => "任务已创建".into(),
        DownloadTaskTimelineEvent::StatusChanged { status } => {
            format!("状态变更为 {}", download_status_log_label(status))
        }
        DownloadTaskTimelineEvent::TransferPlanChanged { segment_count } => {
            format!("传输计划更新 · {segment_count} 个分段")
        }
    }
}

fn download_status_log_label(status: DownloadStatus) -> &'static str {
    match status {
        DownloadStatus::Created => "已创建",
        DownloadStatus::Probing => "探测中",
        DownloadStatus::Waiting => "等待",
        DownloadStatus::Downloading => "下载中",
        DownloadStatus::Paused => "已暂停",
        DownloadStatus::Finalizing => "正在落盘",
        DownloadStatus::Verifying => "正在校验",
        DownloadStatus::Completed => "已完成",
        DownloadStatus::Failed => "失败",
        DownloadStatus::Cancelled => "已取消",
    }
}

fn format_download_timestamp(timestamp: i64) -> String {
    DateTime::from_timestamp_millis(timestamp).map_or_else(
        || "无效时间".into(),
        |timestamp| {
            timestamp
                .with_timezone(&Local)
                .format("%Y-%m-%d %H:%M")
                .to_string()
        },
    )
}

fn format_download_created_compact(timestamp: i64) -> String {
    DateTime::from_timestamp_millis(timestamp).map_or_else(
        || "--".into(),
        |timestamp| {
            let timestamp = timestamp.with_timezone(&Local);
            if timestamp.date_naive() == Local::now().date_naive() {
                timestamp.format("%H:%M").to_string()
            } else {
                timestamp.format("%m-%d").to_string()
            }
        },
    )
}

fn format_download_log_time(timestamp: i64) -> String {
    DateTime::from_timestamp_millis(timestamp).map_or_else(
        || "--:--:--".into(),
        |timestamp| {
            timestamp
                .with_timezone(&Local)
                .format("%H:%M:%S")
                .to_string()
        },
    )
}

fn format_duration(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

fn download_status_presentation(
    theme: Theme,
    status: DownloadStatus,
) -> (&'static str, gpui::Rgba) {
    match status {
        DownloadStatus::Created => ("已创建", theme.text_muted),
        DownloadStatus::Probing => ("解析中", theme.accent),
        DownloadStatus::Waiting => ("等待", theme.text_secondary),
        DownloadStatus::Downloading => ("下载中", theme.accent),
        DownloadStatus::Paused => ("已暂停", theme.text_secondary),
        DownloadStatus::Finalizing => ("正在完成", theme.accent),
        DownloadStatus::Verifying => ("正在校验", theme.accent),
        DownloadStatus::Completed => ("已完成", theme.success),
        DownloadStatus::Failed => ("失败", theme.danger),
        DownloadStatus::Cancelled => ("已取消", theme.text_muted),
    }
}

fn format_download_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = 1024.0 * KIB;
    const GIB: f64 = 1024.0 * MIB;
    let bytes = bytes as f64;
    if bytes >= GIB {
        format!("{:.2} GB", bytes / GIB)
    } else if bytes >= MIB {
        format!("{:.1} MB", bytes / MIB)
    } else if bytes >= KIB {
        format!("{:.1} KB", bytes / KIB)
    } else {
        format!("{} B", bytes as u64)
    }
}

fn format_download_transfer(downloaded: u64, total: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = 1024.0 * KIB;
    const GIB: f64 = 1024.0 * MIB;
    let downloaded = downloaded as f64;
    let total = total as f64;
    if total >= GIB {
        format!(
            "{}/{} GB",
            format_compact_transfer_value(downloaded / GIB),
            format_compact_transfer_value(total / GIB)
        )
    } else if total >= MIB {
        format!(
            "{}/{} MB",
            format_compact_transfer_value(downloaded / MIB),
            format_compact_transfer_value(total / MIB)
        )
    } else if total >= KIB {
        format!(
            "{}/{} KB",
            format_compact_transfer_value(downloaded / KIB),
            format_compact_transfer_value(total / KIB)
        )
    } else {
        format!("{}/{} B", downloaded as u64, total as u64)
    }
}

fn format_download_rate(bytes_per_second: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = 1024.0 * KIB;
    const GIB: f64 = 1024.0 * MIB;
    let rate = bytes_per_second as f64;
    if rate >= GIB {
        format!("{} GB/s", format_compact_transfer_value(rate / GIB))
    } else if rate >= MIB {
        format!("{} MB/s", format_compact_transfer_value(rate / MIB))
    } else if rate >= KIB {
        format!("{} KB/s", format_compact_transfer_value(rate / KIB))
    } else {
        format!("{bytes_per_second} B/s")
    }
}

fn format_download_remaining(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    if hours == 0 {
        format!("{minutes:02}:{:02}", seconds % 60)
    } else if hours < 100 {
        format!("{hours}h{minutes:02}m")
    } else {
        "99h+".into()
    }
}

fn format_compact_transfer_value(value: f64) -> String {
    if value >= 100.0 || value.fract().abs() < 0.05 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

fn download_table_header(
    theme: Theme,
    sort: DownloadSortState,
    main_window: Entity<MainWindow>,
) -> AnyElement {
    let name_header = download_sort_header_cell(
        theme,
        "名称",
        DownloadSortKey::Name,
        sort,
        main_window.clone(),
    )
    .min_w_0()
    .flex_1()
    .pl(px(36.0));
    let progress_header = div()
        .debug_selector(|| "download-transfer-header".into())
        .w(px(DOWNLOAD_PROGRESS_COLUMN_WIDTH))
        .flex_none()
        .flex()
        .items_center()
        .child(
            download_sort_header_cell(
                theme,
                "进度",
                DownloadSortKey::Progress,
                sort,
                main_window.clone(),
            )
            .w(px(DOWNLOAD_TRANSFERRED_COLUMN_WIDTH))
            .flex_none(),
        )
        .child(
            div()
                .debug_selector(|| "download-speed-header".into())
                .w(px(DOWNLOAD_SPEED_COLUMN_WIDTH))
                .flex_none()
                .child("速度"),
        )
        .child(
            div()
                .debug_selector(|| "download-remaining-header".into())
                .w(px(DOWNLOAD_REMAINING_COLUMN_WIDTH))
                .flex_none()
                .child("剩余"),
        );
    let status_header = download_sort_header_cell(
        theme,
        "状态",
        DownloadSortKey::Status,
        sort,
        main_window.clone(),
    )
    .w(px(DOWNLOAD_STATUS_COLUMN_WIDTH))
    .flex_none();
    div()
        .id("download-table-header")
        .debug_selector(|| "download-table-header".into())
        .h(px(DOWNLOAD_TABLE_HEADER_HEIGHT))
        .px(px(SPACING_8))
        .border_b_1()
        .border_color(theme.border_subtle)
        .bg(theme.surface)
        .flex()
        .items_center()
        .gap(px(SPACING_8))
        .text_size(px(TEXT_META))
        .text_color(theme.text_muted)
        .child(name_header)
        .child(progress_header)
        .child(status_header)
        .child(
            download_sort_header_cell(
                theme,
                "添加时间",
                DownloadSortKey::Created,
                sort,
                main_window,
            )
            .w(px(DOWNLOAD_CREATED_COLUMN_WIDTH))
            .flex_none(),
        )
        .into_any_element()
}

fn download_sort_header_cell(
    theme: Theme,
    label: &'static str,
    key: DownloadSortKey,
    sort: DownloadSortState,
    main_window: Entity<MainWindow>,
) -> gpui::Stateful<gpui::Div> {
    let active = sort.key == key;
    let label = if active {
        format!("{label} {}", if sort.descending { "↓" } else { "↑" })
    } else {
        label.to_owned()
    };
    div()
        .id(("download-sort-header", key as usize))
        .debug_selector(move || format!("download-sort-header-{}", key.diagnostic_name()))
        .h_full()
        .cursor_pointer()
        .flex()
        .items_center()
        .text_color(if active {
            theme.accent_text
        } else {
            theme.text_muted
        })
        .hover(move |style| style.text_color(theme.text_secondary))
        .child(label)
        .on_click(move |_event, _window, cx| {
            main_window.update(cx, |window, cx| {
                if set_download_sort(&mut window.download_sort, key, true) {
                    window
                        .download_list_scroll
                        .scroll_to_item(0, ScrollStrategy::Top);
                    cx.notify();
                }
            });
        })
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::collections::HashSet;
    use std::rc::Rc;
    use std::time::Duration;

    use gpui::{
        AppContext, KeyDownEvent, Keystroke, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent,
        TestAppContext, px, size,
    };
    use gpui_component::PixelsExt;
    use nxtools_core::{AppConfig, ClipboardHotKeyModifier, CustomCommandSettings, SystemSettings};
    use nxtools_download::{
        DownloadEngine, DownloadFault, DownloadFaultCode, DownloadProgress, DownloadRequest,
        DownloadSegment, DownloadSegmentState, DownloadStatus, DownloadTaskId, RequestOrigin,
    };
    use nxtools_finder::FinderCatalog;
    use nxtools_network::NetworkPolicy;

    use nxtools_ipc::{PlatformNotification, platform_bridge};
    use nxtools_protocol::{
        ActivateWindowCommand, IndexDriveStatus, IndexPhase, IndexStatus, IndexerCommand,
        IndexerEvent, Message, PlatformCommand, PlatformCommandResult,
    };

    use crate::clipboard::ClipboardApplication;
    use crate::downloads::{DownloadApplication, DownloadRuntimeContext};
    use crate::network_settings::NetworkSettingsApplication;
    use crate::notifications::{NotificationAction, NotificationCenter, NotificationDetail};
    use crate::quick_launch::QuickLaunchApplication;
    use crate::runtime::IndexerNotification;
    use crate::shell::MainModule;

    fn assert_single_index_settings_command(
        endpoint: &crate::runtime::IndexerProcessEndpoint,
        expected_volumes: &[&str],
        expected_exclusions: &[&str],
    ) {
        let envelope = endpoint
            .try_recv_command_for_test()
            .expect("one index settings command");
        let Message::IndexerCommand(IndexerCommand::UpdateSettings(settings)) = envelope.message
        else {
            panic!("volume click must emit one typed index settings command");
        };
        assert_eq!(
            settings.indexed_volumes,
            expected_volumes
                .iter()
                .map(|mount| (*mount).to_owned())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            settings.excluded_paths,
            expected_exclusions
                .iter()
                .map(|path| (*path).to_owned())
                .collect::<Vec<_>>()
        );
        assert!(
            endpoint.try_recv_command_for_test().is_none(),
            "one pointer gesture must not persist and synchronize twice"
        );
    }

    #[test]
    fn download_failure_summary_localizes_typed_errors_without_exposing_engine_ids() {
        let resource_changed = DownloadFault {
            code: DownloadFaultCode::Protocol,
            message: "download resource changed while task DownloadTaskId(d3660bd8-3e06-480e-b738-d783e303165e) was active".into(),
            retryable: false,
        };
        assert_eq!(
            super::download_failure_summary(&resource_changed),
            "远程文件已变化，请重新下载"
        );

        let ignored_range = DownloadFault {
            code: DownloadFaultCode::Protocol,
            message: "server ignored the requested byte range and returned HTTP 200".into(),
            retryable: false,
        };
        assert_eq!(
            super::download_failure_summary(&ignored_range),
            "服务器不支持断点续传，请重新开始"
        );
        assert!(!super::download_failure_summary(&resource_changed).contains("DownloadTaskId"));
    }

    #[test]
    fn download_failure_summary_covers_every_typed_category() {
        let expected = [
            (
                DownloadFaultCode::InvalidRequest,
                "下载请求无效，请检查地址",
            ),
            (DownloadFaultCode::NotFound, "下载任务或文件不存在"),
            (DownloadFaultCode::InvalidState, "当前状态无法执行此操作"),
            (DownloadFaultCode::Network, "网络连接失败，请检查网络后重试"),
            (DownloadFaultCode::Protocol, "服务器响应异常，请重试"),
            (
                DownloadFaultCode::Storage,
                "无法写入下载文件，请检查保存位置",
            ),
            (DownloadFaultCode::Permission, "没有权限写入保存位置"),
            (DownloadFaultCode::DiskFull, "磁盘空间不足"),
            (DownloadFaultCode::FileInUse, "目标文件正在被其他程序使用"),
            (DownloadFaultCode::Cancelled, "下载已取消"),
            (
                DownloadFaultCode::Unavailable,
                "下载服务暂时不可用，请稍后重试",
            ),
            (DownloadFaultCode::Internal, "下载发生内部错误，请重试"),
        ];
        for (code, summary) in expected {
            let error = DownloadFault {
                code,
                message: "internal engine detail".into(),
                retryable: false,
            };
            assert_eq!(super::download_failure_summary(&error), summary);
        }
    }

    #[test]
    fn download_row_transfer_uses_live_values_only_for_active_tasks() {
        let task_id = DownloadTaskId::new();
        let progress = DownloadProgress {
            task_id,
            downloaded_bytes: 25 * 1024 * 1024,
            total_bytes: Some(100 * 1024 * 1024),
            bytes_per_second: 5 * 1024 * 1024,
            active_connections: 4,
            segments: Vec::new(),
        };
        let active = super::download_row_transfer_presentation(
            DownloadStatus::Downloading,
            20 * 1024 * 1024,
            Some(100 * 1024 * 1024),
            Some(&progress),
        );
        assert_eq!(active.fraction, 0.25);
        assert_eq!(active.transferred, "25/100 MB");
        assert_eq!(active.speed, "5 MB/s");
        assert_eq!(active.remaining, "00:15");

        let completed = super::download_row_transfer_presentation(
            DownloadStatus::Completed,
            100 * 1024 * 1024,
            Some(100 * 1024 * 1024),
            Some(&progress),
        );
        assert_eq!(completed.fraction, 1.0);
        assert_eq!(completed.transferred, "100/100 MB");
        assert_eq!(completed.speed, "—");
        assert_eq!(completed.remaining, "—");

        assert_eq!(super::format_download_rate(1023 * 1024 * 1024), "1023 MB/s");
        assert_eq!(super::format_download_rate(1024 * 1024 * 1024), "1 GB/s");
        assert_eq!(super::format_download_remaining(3_600), "1h00m");
        assert_eq!(super::format_download_remaining(359_999), "99h59m");
        assert_eq!(super::format_download_remaining(360_000), "99h+");
    }

    #[gpui::test]
    fn titlebar_global_settings_click_keeps_its_controller_alive(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
        let config = std::rc::Rc::new(std::cell::RefCell::new(AppConfig::default()));
        let config_path = std::env::temp_dir().join(format!(
            "nxtools-global-settings-{}.json",
            uuid::Uuid::new_v4()
        ));
        let network_settings = cx.update(|cx| {
            cx.new(|_| {
                NetworkSettingsApplication::new(
                    NetworkPolicy::new(config.borrow().network.clone())
                        .expect("create network policy"),
                    config.borrow().clone(),
                    config_path,
                    notifications.clone(),
                    None,
                    Rc::new(RefCell::new(SystemSettings::default())),
                    false,
                )
            })
        });
        let settings_observer = network_settings.downgrade();
        let settings_for_window = Some(network_settings.clone());
        let (_main_window, window) = cx.add_window_view(move |window, cx| {
            MainWindow::new(
                gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
                gpui::WeakEntity::<QuickLaunchApplication>::new_invalid(),
                gpui::WeakEntity::<DownloadApplication>::new_invalid(),
                MainWindowShellContext {
                    settings: settings_for_window,
                    platform: None,
                    system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                },
                notifications,
                window,
                cx,
            )
        });
        drop(network_settings);

        window.refresh().expect("render main title bar");
        assert!(
            window.debug_bounds("main-titlebar").is_some(),
            "main title bar must render"
        );
        let settings = window
            .debug_bounds("global-network-settings")
            .expect("global settings trigger");
        window.simulate_click(settings.center(), Modifiers::default());
        window.run_until_parked();

        assert!(
            settings_observer
                .update(cx, |application, _| application.window_is_open())
                .expect("main window must strongly own global settings")
        );
    }

    #[gpui::test]
    fn module_switching_keeps_one_compact_header_and_sidebar_rhythm(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
        let (_main_window, window) = cx.add_window_view(move |window, cx| {
            window.resize(gpui::size(px(1060.0), px(500.0)));
            MainWindow::new(
                gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
                gpui::WeakEntity::<QuickLaunchApplication>::new_invalid(),
                gpui::WeakEntity::<DownloadApplication>::new_invalid(),
                MainWindowShellContext {
                    settings: None,
                    platform: None,
                    system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                },
                notifications,
                window,
                cx,
            )
        });
        window.refresh().expect("render download workspace");

        let sidebar = window
            .debug_bounds("main-sidebar")
            .expect("rendered production sidebar");
        assert_eq!(sidebar.size.width, px(SIDEBAR_WIDTH));
        for selector in ["main-module-下载", "main-module-速启", "main-module-剪贴板"] {
            let row = window
                .debug_bounds(selector)
                .unwrap_or_else(|| panic!("rendered sidebar row {selector}"));
            assert_eq!(row.size.height, px(SIDEBAR_ROW_HEIGHT));
        }
        for module in ["下载", "速启", "剪贴板"] {
            let title_selector: &'static str =
                Box::leak(format!("main-module-{module}-title").into_boxed_str());
            let description_selector: &'static str =
                Box::leak(format!("main-module-{module}-description").into_boxed_str());
            assert_eq!(
                window
                    .debug_bounds(title_selector)
                    .expect("sidebar module title")
                    .size
                    .height,
                px(LINE_HEIGHT_BODY)
            );
            assert_eq!(
                window
                    .debug_bounds(description_selector)
                    .expect("sidebar module description")
                    .size
                    .height,
                px(LINE_HEIGHT_LABEL)
            );
        }

        let download_header = window
            .debug_bounds("download-module-header")
            .expect("download module header");
        assert_eq!(download_header.size.height, px(48.0));
        assert_eq!(
            window
                .debug_bounds("download-module-header-title")
                .expect("download module title")
                .size
                .height,
            px(20.0)
        );
        assert_eq!(
            window
                .debug_bounds("download-module-header-metadata")
                .expect("download module metadata")
                .size
                .height,
            px(LINE_HEIGHT_LABEL)
        );
        let search = window
            .debug_bounds("download-search")
            .expect("download header search");
        let settings = window
            .debug_bounds("download-settings-action")
            .expect("download header settings");
        assert_eq!(settings.left() - search.right(), px(SPACING_4));
        let download_empty = window
            .debug_bounds("download-empty-state")
            .expect("download work-surface empty state");
        let download_empty_icon = window
            .debug_bounds("download-empty-state-icon")
            .expect("download empty icon slot");
        let download_empty_title = window
            .debug_bounds("download-empty-state-title")
            .expect("download empty title");
        let download_empty_description = window
            .debug_bounds("download-empty-state-description")
            .expect("download empty description");
        assert_eq!(download_empty_icon.size, size(px(18.0), px(18.0)));
        assert_eq!(download_empty_title.size.height, px(LINE_HEIGHT_BODY));
        assert_eq!(
            download_empty_description.size.height,
            px(LINE_HEIGHT_LABEL)
        );
        assert_eq!(
            download_empty_title.left(),
            download_empty_description.left()
        );
        assert!(download_empty.left() >= sidebar.right());

        let quick_switch = window
            .debug_bounds("main-module-速启")
            .expect("quick-launch sidebar row");
        window.simulate_click(quick_switch.center(), Modifiers::default());
        window.refresh().expect("render quick-launch workspace");
        let quick_header = window
            .debug_bounds("quick-launch-module-header")
            .expect("quick-launch module header");
        assert_eq!(quick_header.size.height, download_header.size.height);
        assert_eq!(quick_header.origin.y, download_header.origin.y);
        assert_eq!(
            window
                .debug_bounds("quick-launch-module-header-title")
                .expect("quick-launch module title")
                .size
                .height,
            px(20.0)
        );
        assert_eq!(
            window
                .debug_bounds("quick-launch-module-header-metadata")
                .expect("quick-launch module metadata")
                .size
                .height,
            px(LINE_HEIGHT_LABEL)
        );

        let clipboard_switch = window
            .debug_bounds("main-module-剪贴板")
            .expect("clipboard sidebar row");
        window.simulate_click(clipboard_switch.center(), Modifiers::default());
        window.refresh().expect("render clipboard workspace");
        let clipboard_header = window
            .debug_bounds("clipboard-module-header")
            .expect("clipboard module header");
        assert_eq!(clipboard_header.size.height, download_header.size.height);
        assert_eq!(clipboard_header.origin.y, download_header.origin.y);
        assert_eq!(
            window
                .debug_bounds("clipboard-module-header-title")
                .expect("clipboard module title")
                .size
                .height,
            px(20.0)
        );
        assert_eq!(
            window
                .debug_bounds("clipboard-module-header-metadata")
                .expect("clipboard module metadata")
                .size
                .height,
            px(LINE_HEIGHT_LABEL)
        );
    }

    #[gpui::test]
    fn quick_launch_settings_navigation_mounts_only_the_selected_page(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let executor = cx.executor();
        let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
        let temp = tempfile::tempdir().expect("temporary Quick Launch settings fixture");
        let catalog = FinderCatalog::open(temp.path().join("history.db"), &[])
            .expect("open Quick Launch settings catalog");
        let config_path = temp.path().join("config.json");
        let log_directory = temp.path().join("logs");
        let mut application_config = AppConfig::default();
        application_config.quick_launch.excluded_paths = (0..20)
            .map(|index| format!(r"C:\Build\Generated\{index}"))
            .collect();
        let quick_launch = cx.update(|cx| {
            let (platform, _platform_endpoint) = platform_bridge(8);
            let (indexer, _indexer_endpoint) = crate::runtime::indexer_bridge(8);
            cx.new(|_| {
                QuickLaunchApplication::new(
                    catalog,
                    Rc::new(platform),
                    Rc::new(indexer),
                    application_config,
                    config_path,
                    log_directory,
                    notifications.clone(),
                )
            })
        });
        let settings_before_navigation = cx.read(|cx| quick_launch.read(cx).settings());
        assert_eq!(settings_before_navigation.excluded_paths.len(), 20);
        let quick_launch_for_window = quick_launch.downgrade();
        let notifications_for_window = notifications.clone();
        let (main_window, window) = cx.add_window_view(move |window, cx| {
            window.resize(gpui::size(px(1060.0), px(500.0)));
            let mut main_window = MainWindow::new(
                gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
                quick_launch_for_window,
                gpui::WeakEntity::<DownloadApplication>::new_invalid(),
                MainWindowShellContext {
                    settings: None,
                    platform: None,
                    system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                },
                notifications_for_window,
                window,
                cx,
            );
            main_window.shell.activate(MainModule::QuickFind);
            main_window
        });

        window.refresh().expect("render Quick Launch settings");
        let navigation = window
            .debug_bounds("quick-launch-settings-navigation")
            .expect("stable Quick Launch settings navigation");
        assert_eq!(navigation.size.width, px(160.0));
        for selector in [
            "quick-launch-settings-nav-general",
            "quick-launch-settings-nav-index",
            "quick-launch-settings-nav-context",
            "quick-launch-settings-nav-commands",
            "quick-launch-settings-nav-diagnostics",
        ] {
            assert_eq!(
                window
                    .debug_bounds(selector)
                    .unwrap_or_else(|| panic!("missing {selector}"))
                    .size
                    .height,
                px(40.0)
            );
        }
        assert!(
            window
                .debug_bounds("quick-launch-settings-page-general")
                .is_some()
        );
        assert!(
            window
                .debug_bounds("quick-launch-settings-page-index")
                .is_none()
        );
        assert_eq!(
            window
                .debug_bounds("quick-launch-settings-page-header")
                .expect("fixed Quick Launch settings page header")
                .size
                .height,
            px(48.0)
        );
        assert_eq!(
            window
                .debug_bounds("quick-launch-settings-page-title")
                .expect("Quick Launch settings page title")
                .size
                .height,
            px(LINE_HEIGHT_TITLE)
        );
        assert_eq!(
            window
                .debug_bounds("quick-launch-settings-page-description")
                .expect("Quick Launch settings page description")
                .size
                .height,
            px(LINE_HEIGHT_LABEL)
        );

        let index_navigation = window
            .debug_bounds("quick-launch-settings-nav-index")
            .expect("Index navigation row");
        window.simulate_click(index_navigation.center(), Modifiers::default());
        window.run_until_parked();
        assert_eq!(
            window.read(|cx| main_window.read(cx).quick_launch_settings_page),
            QuickLaunchSettingsPage::Index
        );
        window.refresh().expect("render Index settings page");
        assert!(
            window
                .debug_bounds("quick-launch-settings-page-index")
                .is_some()
        );

        window.update(|_, cx| {
            main_window.update(cx, |main_window, cx| {
                main_window.animate_quick_find_scroll_to(-38.0, cx);
            });
        });
        let scrolling_generation = window.read(|cx| {
            let main_window = main_window.read(cx);
            assert!(main_window.quick_find_scroll_target < 0.0);
            main_window.quick_find_scroll_animation
        });

        let context_navigation = window
            .debug_bounds("quick-launch-settings-nav-context")
            .expect("Context navigation row");
        window.simulate_click(context_navigation.center(), Modifiers::default());
        window.run_until_parked();
        assert_eq!(
            window.read(|cx| main_window.read(cx).quick_launch_settings_page),
            QuickLaunchSettingsPage::Context
        );
        assert!(
            window.read(|cx| main_window.read(cx).quick_find_scroll_animation)
                > scrolling_generation
        );
        executor.advance_clock(Duration::from_millis(250));
        executor.run_until_parked();
        window.read(|cx| {
            let main_window = main_window.read(cx);
            assert_eq!(main_window.quick_find_scroll_target, 0.0);
            assert_eq!(main_window.quick_find_scroll.offset().y, px(0.0));
        });
        window.refresh().expect("render Context settings page");
        assert!(
            window
                .debug_bounds("quick-launch-settings-page-context")
                .is_some()
        );

        for (navigation_selector, page, content_selector) in [
            (
                "quick-launch-settings-nav-commands",
                QuickLaunchSettingsPage::Commands,
                "quick-launch-settings-page-commands",
            ),
            (
                "quick-launch-settings-nav-diagnostics",
                QuickLaunchSettingsPage::Diagnostics,
                "quick-launch-settings-page-diagnostics",
            ),
            (
                "quick-launch-settings-nav-general",
                QuickLaunchSettingsPage::General,
                "quick-launch-settings-page-general",
            ),
            (
                "quick-launch-settings-nav-index",
                QuickLaunchSettingsPage::Index,
                "quick-launch-settings-page-index",
            ),
        ] {
            let navigation = window
                .debug_bounds(navigation_selector)
                .unwrap_or_else(|| panic!("missing {navigation_selector}"));
            window.simulate_click(navigation.center(), Modifiers::default());
            window.run_until_parked();
            assert_eq!(
                window.read(|cx| main_window.read(cx).quick_launch_settings_page),
                page
            );
            window.refresh().expect("render selected settings page");
            assert!(
                window.debug_bounds(content_selector).is_some(),
                "missing {content_selector}"
            );
        }
        assert_eq!(
            cx.read(|cx| quick_launch.read(cx).settings()),
            settings_before_navigation,
            "settings navigation must not mutate Quick Launch settings"
        );
    }

    #[gpui::test]
    fn every_quick_launch_settings_frame_mounts_exactly_one_page(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));

        for selected_page in QuickLaunchSettingsPage::ALL {
            let notifications_for_window = notifications.clone();
            let (_main_window, window) = cx.add_window_view(move |window, cx| {
                window.resize(gpui::size(px(1060.0), px(500.0)));
                let mut main_window = MainWindow::new(
                    gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
                    gpui::WeakEntity::<QuickLaunchApplication>::new_invalid(),
                    gpui::WeakEntity::<DownloadApplication>::new_invalid(),
                    MainWindowShellContext {
                        settings: None,
                        platform: None,
                        system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                    },
                    notifications_for_window,
                    window,
                    cx,
                );
                main_window.shell.activate(MainModule::QuickFind);
                main_window.quick_launch_settings_page = selected_page;
                main_window
            });

            window.refresh().expect("render selected Quick Launch page");
            for candidate_page in QuickLaunchSettingsPage::ALL {
                assert_eq!(
                    window
                        .debug_bounds(candidate_page.metadata().content_id)
                        .is_some(),
                    candidate_page == selected_page,
                    "only {} may mount in the selected-page viewport",
                    selected_page.metadata().label
                );
            }
        }
    }

    #[gpui::test]
    fn quick_launch_volume_scope_uses_one_checkbox_list_and_preserves_automatic_semantics(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
        let temp = tempfile::tempdir().expect("temporary Quick Launch volume fixture");
        let catalog = FinderCatalog::open(temp.path().join("history.db"), &[])
            .expect("open Quick Launch volume catalog");
        let (platform, _platform_endpoint) = platform_bridge(8);
        let (indexer, indexer_endpoint) = crate::runtime::indexer_bridge(8);
        indexer_endpoint.connect_for_test();
        let quick_launch = cx.update(|cx| {
            cx.new(|_| {
                QuickLaunchApplication::new(
                    catalog,
                    Rc::new(platform),
                    Rc::new(indexer),
                    AppConfig::default(),
                    temp.path().join("config.json"),
                    temp.path().join("logs"),
                    notifications.clone(),
                )
            })
        });
        quick_launch.update(cx, |application, cx| {
            application.handle_indexer_notification(
                IndexerNotification::Event(IndexerEvent::Status(IndexStatus {
                    phase: IndexPhase::Ready,
                    indexed_items: 3_750_000,
                    drives: vec![
                        IndexDriveStatus {
                            mount_point: r"C:\".into(),
                            file_system: "NTFS".into(),
                            indexed_items: 2_500_000,
                            journal_id: Some(1),
                            next_usn: Some(10),
                        },
                        IndexDriveStatus {
                            mount_point: r"D:\".into(),
                            file_system: "NTFS".into(),
                            indexed_items: 1_250_000,
                            journal_id: Some(2),
                            next_usn: Some(20),
                        },
                        IndexDriveStatus {
                            mount_point: r"E:\".into(),
                            file_system: "NTFS".into(),
                            indexed_items: 0,
                            journal_id: None,
                            next_usn: None,
                        },
                    ],
                    last_error: None,
                })),
                cx,
            );
        });
        assert_single_index_settings_command(&indexer_endpoint, &[], &[]);
        let quick_launch_for_window = quick_launch.downgrade();
        let notifications_for_window = notifications.clone();
        let (_main_window, window) = cx.add_window_view(move |window, cx| {
            window.resize(size(px(1060.0), px(500.0)));
            let mut main_window = MainWindow::new(
                gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
                quick_launch_for_window,
                gpui::WeakEntity::<DownloadApplication>::new_invalid(),
                MainWindowShellContext {
                    settings: None,
                    platform: None,
                    system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                },
                notifications_for_window,
                window,
                cx,
            );
            main_window.shell.activate(MainModule::QuickFind);
            main_window.quick_launch_settings_page = QuickLaunchSettingsPage::Index;
            main_window
        });

        window.refresh().expect("render automatic volume scope");
        let scope = window
            .debug_bounds("quick-launch-volume-scope")
            .expect("one volume checkbox list");
        let automatic = window
            .debug_bounds("quick-launch-volume-scope-automatic")
            .expect("automatic volume mode row");
        assert_eq!(automatic.size.height, px(48.0));
        assert!(
            window
                .debug_bounds("quick-launch-volume-scope-automatic-checked")
                .is_some()
        );
        for (drive, row_selector, checked_selector) in [
            (
                "C",
                "quick-launch-index-volume-C",
                "quick-launch-index-volume-C-checked",
            ),
            (
                "D",
                "quick-launch-index-volume-D",
                "quick-launch-index-volume-D-checked",
            ),
            (
                "E",
                "quick-launch-index-volume-E",
                "quick-launch-index-volume-E-checked",
            ),
        ] {
            let row = window
                .debug_bounds(row_selector)
                .unwrap_or_else(|| panic!("missing {drive}: volume row"));
            assert_eq!(row.size.height, px(40.0));
            assert_eq!(row.left(), automatic.left());
            assert_eq!(row.size.width, automatic.size.width);
            assert!(
                window.debug_bounds(checked_selector).is_some(),
                "automatic mode must visibly include {drive}:"
            );
        }
        assert_eq!(automatic.left() - scope.left(), px(1.0));
        assert_eq!(scope.size.width - automatic.size.width, px(2.0));
        assert_eq!(
            window
                .debug_bounds("quick-launch-index-volume-C-title")
                .expect("volume title")
                .size
                .height,
            px(LINE_HEIGHT_BODY)
        );
        assert_eq!(
            window
                .debug_bounds("quick-launch-index-volume-C-metadata")
                .expect("volume metadata")
                .size
                .height,
            px(LINE_HEIGHT_LABEL)
        );

        let c_volume = window
            .debug_bounds("quick-launch-index-volume-C")
            .expect("C: volume row");
        window.simulate_click(
            gpui::point(c_volume.left() + px(19.0), c_volume.center().y),
            Modifiers::default(),
        );
        window.run_until_parked();
        assert_eq!(
            window.update(|_, cx| quick_launch.read(cx).settings().indexed_volumes),
            vec![r"D:\".to_owned(), r"E:\".to_owned()],
            "disabling one volume must materialize an explicit selected-volume set"
        );
        assert_single_index_settings_command(&indexer_endpoint, &[r"D:\", r"E:\"], &[]);
        window.refresh().expect("render specified volume scope");
        assert!(
            window
                .debug_bounds("quick-launch-volume-scope-automatic-unchecked")
                .is_some()
        );
        assert!(
            window
                .debug_bounds("quick-launch-index-volume-C-unchecked")
                .is_some()
        );

        let d_volume = window
            .debug_bounds("quick-launch-index-volume-D")
            .expect("D: volume row");
        window.simulate_click(
            gpui::point(d_volume.right() - px(20.0), d_volume.center().y),
            Modifiers::default(),
        );
        window.run_until_parked();
        assert_eq!(
            window.update(|_, cx| quick_launch.read(cx).settings().indexed_volumes),
            vec![r"E:\".to_owned()],
            "the non-indicator row area must share the same single toggle path"
        );
        assert_single_index_settings_command(&indexer_endpoint, &[r"E:\"], &[]);
        window.refresh().expect("render final explicit volume");

        let e_volume = window
            .debug_bounds("quick-launch-index-volume-E")
            .expect("E: volume row");
        window.simulate_click(
            gpui::point(e_volume.left() + px(19.0), e_volume.center().y),
            Modifiers::default(),
        );
        window.run_until_parked();
        assert_eq!(
            window.update(|_, cx| quick_launch.read(cx).settings().indexed_volumes),
            vec![r"E:\".to_owned()],
            "the last available selected volume cannot encode an empty explicit scope"
        );
        assert!(
            indexer_endpoint.try_recv_command_for_test().is_none(),
            "a rejected last-volume toggle must not persist or synchronize"
        );

        let automatic = window
            .debug_bounds("quick-launch-volume-scope-automatic")
            .expect("automatic volume mode row");
        window.simulate_click(automatic.center(), Modifiers::default());
        window.run_until_parked();
        assert!(
            window
                .update(|_, cx| quick_launch.read(cx).settings().indexed_volumes)
                .is_empty(),
            "automatic mode must retain the empty-vector persistence contract"
        );
        assert_single_index_settings_command(&indexer_endpoint, &[], &[]);
    }

    #[gpui::test]
    fn quick_launch_exclusion_paths_use_one_keyboard_operable_list_with_bounded_actions(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let executor = cx.executor();
        let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
        let temp = tempfile::tempdir().expect("temporary Quick Launch exclusion fixture");
        let catalog = FinderCatalog::open(temp.path().join("history.db"), &[])
            .expect("open Quick Launch exclusion catalog");
        let (platform, platform_endpoint) = platform_bridge(8);
        platform_endpoint.set_connected_for_contract_test(true);
        let (indexer, indexer_endpoint) = crate::runtime::indexer_bridge(8);
        indexer_endpoint.connect_for_test();
        let long_path = r"C:\Users\Administrator\Documents\NxTools\Projects\Archived\Search Corpus";
        let build_path = r"D:\Build\Cache";
        let temp_path = r"E:\Temp";
        let mut config = AppConfig::default();
        config.quick_launch.excluded_paths = vec![
            long_path.to_owned(),
            build_path.to_owned(),
            temp_path.to_owned(),
        ];
        let quick_launch = cx.update(|cx| {
            cx.new(|_| {
                QuickLaunchApplication::new(
                    catalog,
                    Rc::new(platform),
                    Rc::new(indexer),
                    config,
                    temp.path().join("config.json"),
                    temp.path().join("logs"),
                    notifications.clone(),
                )
            })
        });
        let quick_launch_for_window = quick_launch.downgrade();
        let notifications_for_window = notifications.clone();
        let (main_window, window) = cx.add_window_view(move |window, cx| {
            window.resize(size(px(1060.0), px(720.0)));
            let mut main_window = MainWindow::new(
                gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
                quick_launch_for_window,
                gpui::WeakEntity::<DownloadApplication>::new_invalid(),
                MainWindowShellContext {
                    settings: None,
                    platform: None,
                    system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                },
                notifications_for_window,
                window,
                cx,
            );
            main_window.shell.activate(MainModule::QuickFind);
            main_window.quick_launch_settings_page = QuickLaunchSettingsPage::Index;
            main_window
        });

        window.refresh().expect("render exclusion path list");
        let list = window
            .debug_bounds("quick-launch-exclusion-list")
            .expect("one continuous exclusion list");
        let first_row = window
            .debug_bounds("quick-launch-exclusion-row-0")
            .expect("first exclusion row");
        let second_row = window
            .debug_bounds("quick-launch-exclusion-row-1")
            .expect("second exclusion row");
        let third_row = window
            .debug_bounds("quick-launch-exclusion-row-2")
            .expect("third exclusion row");
        for row in [first_row, second_row, third_row] {
            assert_eq!(row.size.height, px(40.0));
            assert_eq!(row.left(), list.left() + px(1.0));
            assert_eq!(row.size.width, list.size.width - px(2.0));
        }
        let first_path = window
            .debug_bounds("quick-launch-exclusion-path-0")
            .expect("truncated path rail");
        let first_delete = window
            .debug_bounds("quick-launch-exclusion-delete-0")
            .expect("bounded delete hit target");
        assert_eq!(first_path.size.height, px(LINE_HEIGHT_BODY));
        assert_eq!(first_delete.size, size(px(40.0), px(40.0)));
        assert!(first_path.right() <= first_delete.left());
        assert_eq!(
            crate::ui::components::path_list::exact_path_tooltip(long_path).to_string(),
            long_path,
            "the production tooltip seam must preserve the exact original path"
        );

        window.simulate_mouse_move(first_path.center(), None, Modifiers::default());
        executor.advance_clock(Duration::from_millis(500));
        window.run_until_parked();
        assert!(
            window
                .debug_bounds("quick-launch-exclusion-path-tooltip-0")
                .is_some(),
            "the exact long path must remain available through a tooltip"
        );

        window.simulate_click(second_row.center(), Modifiers::default());
        assert_eq!(
            window.read(|cx| main_window.read(cx).selected_index_exclusion),
            Some(1)
        );
        window.simulate_keystrokes("up");
        assert_eq!(
            window.read(|cx| main_window.read(cx).selected_index_exclusion),
            Some(0)
        );
        window.simulate_keystrokes("down");
        assert_eq!(
            window.read(|cx| main_window.read(cx).selected_index_exclusion),
            Some(1)
        );
        window.simulate_keystrokes("escape");
        assert_eq!(
            window.read(|cx| main_window.read(cx).selected_index_exclusion),
            None
        );
        window.simulate_click(second_row.center(), Modifiers::default());
        window.simulate_keystrokes("backspace");
        window.run_until_parked();
        assert_eq!(
            window.update(|_, cx| quick_launch.read(cx).settings().excluded_paths),
            vec![long_path.to_owned(), temp_path.to_owned()]
        );
        assert_single_index_settings_command(&indexer_endpoint, &[], &[long_path, temp_path]);

        window
            .refresh()
            .expect("render list after keyboard removal");
        let first_delete = window
            .debug_bounds("quick-launch-exclusion-delete-0")
            .expect("first delete target after keyboard removal");
        window.simulate_click(first_delete.center(), Modifiers::default());
        window.run_until_parked();
        assert_eq!(
            window.update(|_, cx| quick_launch.read(cx).settings().excluded_paths),
            vec![temp_path.to_owned()]
        );
        assert_single_index_settings_command(&indexer_endpoint, &[], &[temp_path]);

        window.refresh().expect("render add directory action");
        let add = window
            .debug_bounds("quick-launch-add-index-exclusion")
            .expect("standard add-directory action");
        assert_eq!(add.size.height, px(28.0));
        window.simulate_click(add.center(), Modifiers::default());
        window.run_until_parked();
        let picker = platform_endpoint
            .try_receive_command_for_contract_test()
            .expect("one typed directory picker command");
        assert!(matches!(
            &picker.message,
            Message::PlatformCommand(PlatformCommand::PickDirectory(_))
        ));
        assert!(
            platform_endpoint
                .try_receive_command_for_contract_test()
                .is_err(),
            "one add gesture must open only one directory picker"
        );

        window.update(|_, cx| {
            quick_launch.update(cx, |application, cx| {
                application.handle_platform_notification(
                    PlatformNotification::CommandResult {
                        correlation_id: uuid::Uuid::new_v4(),
                        result: PlatformCommandResult {
                            success: true,
                            error: None,
                            native_error_code: None,
                            data: Some(r"F:\Archive".into()),
                        },
                    },
                    cx,
                );
            });
        });
        assert_eq!(
            window.update(|_, cx| quick_launch.read(cx).settings().excluded_paths),
            vec![temp_path.to_owned()],
            "a stale picker result must not mutate the current exclusion list"
        );
        window.update(|_, cx| {
            quick_launch.update(cx, |application, cx| {
                application.handle_platform_notification(
                    PlatformNotification::CommandResult {
                        correlation_id: picker.id,
                        result: PlatformCommandResult {
                            success: true,
                            error: None,
                            native_error_code: None,
                            data: Some(r"e:\TEMP".into()),
                        },
                    },
                    cx,
                );
            });
        });
        assert!(
            indexer_endpoint.try_recv_command_for_test().is_none(),
            "a case-insensitive duplicate picker result must not synchronize"
        );

        window.refresh().expect("render add action after duplicate");
        let add = window
            .debug_bounds("quick-launch-add-index-exclusion")
            .expect("add action after duplicate");
        window.simulate_click(add.center(), Modifiers::default());
        window.run_until_parked();
        let selected_picker = platform_endpoint
            .try_receive_command_for_contract_test()
            .expect("second typed directory picker command");
        window.update(|_, cx| {
            quick_launch.update(cx, |application, cx| {
                application.handle_platform_notification(
                    PlatformNotification::CommandResult {
                        correlation_id: selected_picker.id,
                        result: PlatformCommandResult {
                            success: true,
                            error: None,
                            native_error_code: None,
                            data: Some(r"F:\Archive".into()),
                        },
                    },
                    cx,
                );
            });
        });
        window.run_until_parked();
        assert_eq!(
            window.update(|_, cx| quick_launch.read(cx).settings().excluded_paths),
            vec![temp_path.to_owned(), r"F:\Archive".to_owned()]
        );
        assert_single_index_settings_command(&indexer_endpoint, &[], &[temp_path, r"F:\Archive"]);

        window.refresh().expect("render add action after selection");
        let add = window
            .debug_bounds("quick-launch-add-index-exclusion")
            .expect("add action after selection");
        window.simulate_click(add.center(), Modifiers::default());
        window.run_until_parked();
        let cancelled_picker = platform_endpoint
            .try_receive_command_for_contract_test()
            .expect("third typed directory picker command");
        window.update(|_, cx| {
            quick_launch.update(cx, |application, cx| {
                application.handle_platform_notification(
                    PlatformNotification::CommandResult {
                        correlation_id: cancelled_picker.id,
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
        });
        assert!(
            indexer_endpoint.try_recv_command_for_test().is_none(),
            "picker cancellation must not persist or synchronize"
        );

        window
            .refresh()
            .expect("render add action after cancellation");
        let add = window
            .debug_bounds("quick-launch-add-index-exclusion")
            .expect("add action after cancellation");
        window.simulate_click(add.center(), Modifiers::default());
        window.run_until_parked();
        let failed_picker = platform_endpoint
            .try_receive_command_for_contract_test()
            .expect("fourth typed directory picker command");
        window.update(|_, cx| {
            quick_launch.update(cx, |application, cx| {
                application.handle_platform_notification(
                    PlatformNotification::CommandResult {
                        correlation_id: failed_picker.id,
                        result: PlatformCommandResult {
                            success: false,
                            error: Some("directory picker unavailable".into()),
                            native_error_code: Some(5),
                            data: None,
                        },
                    },
                    cx,
                );
            });
        });
        assert!(
            indexer_endpoint.try_recv_command_for_test().is_none(),
            "picker failure must not persist or synchronize"
        );
        assert_eq!(
            window.update(|_, cx| quick_launch.read(cx).settings().excluded_paths),
            vec![temp_path.to_owned(), r"F:\Archive".to_owned()]
        );

        window.refresh().expect("render final keyboard removal");
        let second_row = window
            .debug_bounds("quick-launch-exclusion-row-1")
            .expect("second exclusion row before Delete");
        window.simulate_click(second_row.center(), Modifiers::default());
        window.simulate_keystrokes("delete");
        window.run_until_parked();
        assert_eq!(
            window.update(|_, cx| quick_launch.read(cx).settings().excluded_paths),
            vec![temp_path.to_owned()]
        );
        assert_single_index_settings_command(&indexer_endpoint, &[], &[temp_path]);

        window.refresh().expect("render final exclusion row");
        let final_row = window
            .debug_bounds("quick-launch-exclusion-row-0")
            .expect("final exclusion row");
        window.simulate_click(final_row.center(), Modifiers::default());
        window.simulate_keystrokes("delete");
        window.run_until_parked();
        assert!(
            window
                .update(|_, cx| quick_launch.read(cx).settings().excluded_paths)
                .is_empty(),
            "removing the last exclusion must leave a valid empty list"
        );
        assert_single_index_settings_command(&indexer_endpoint, &[], &[]);
        window.refresh().expect("render empty exclusion list");
        assert_eq!(
            window
                .debug_bounds("quick-launch-exclusion-empty")
                .expect("neutral empty row")
                .size
                .height,
            px(40.0)
        );
    }

    #[gpui::test]
    fn quick_launch_custom_commands_use_one_master_detail_editor_with_separated_actions(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
        let temp = tempfile::tempdir().expect("temporary custom-command fixture");
        let catalog = FinderCatalog::open(temp.path().join("history.db"), &[])
            .expect("open custom-command catalog");
        let (platform, platform_endpoint) = platform_bridge(8);
        platform_endpoint.set_connected_for_contract_test(true);
        let (indexer, _indexer_endpoint) = crate::runtime::indexer_bridge(8);
        let mut config = AppConfig::default();
        config.quick_launch.custom_commands = vec![
            CustomCommandSettings {
                enabled: true,
                keyword: "docs".into(),
                title: "Documentation".into(),
                program: r"C:\Tools\docs.exe".into(),
                argument_template: "%s".into(),
                working_directory: r"C:\Docs".into(),
                run_as_admin: false,
                run_silently: false,
            },
            CustomCommandSettings {
                enabled: false,
                keyword: "build".into(),
                title: "Build workspace".into(),
                program: r"D:\Tools\build.exe".into(),
                argument_template: "--target %s1".into(),
                working_directory: r"D:\Workspace".into(),
                run_as_admin: true,
                run_silently: true,
            },
        ];
        let quick_launch = cx.update(|cx| {
            cx.new(|_| {
                QuickLaunchApplication::new(
                    catalog,
                    Rc::new(platform),
                    Rc::new(indexer),
                    config,
                    temp.path().join("config.json"),
                    temp.path().join("logs"),
                    notifications.clone(),
                )
            })
        });
        let quick_launch_for_window = quick_launch.downgrade();
        let notifications_for_window = notifications.clone();
        let (main_window, window) = cx.add_window_view(move |window, cx| {
            window.resize(size(px(1060.0), px(720.0)));
            let mut main_window = MainWindow::new(
                gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
                quick_launch_for_window,
                gpui::WeakEntity::<DownloadApplication>::new_invalid(),
                MainWindowShellContext {
                    settings: None,
                    platform: None,
                    system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                },
                notifications_for_window,
                window,
                cx,
            );
            main_window.shell.activate(MainModule::QuickFind);
            main_window.quick_launch_settings_page = QuickLaunchSettingsPage::Commands;
            main_window
        });

        window
            .refresh()
            .expect("render custom-command master-detail");
        let shell = window
            .debug_bounds("quick-launch-custom-command-master-detail")
            .expect("one continuous custom-command shell");
        let master = window
            .debug_bounds("quick-launch-custom-command-master")
            .expect("stable master rail");
        let detail = window
            .debug_bounds("quick-launch-custom-command-detail")
            .expect("one detail editor");
        assert_eq!(master.right(), detail.left());
        assert_eq!(master.top(), shell.top() + px(1.0));
        assert_eq!(detail.bottom(), shell.bottom() - px(1.0));

        for index in 0..2 {
            let row_selector: &'static str =
                Box::leak(format!("quick-launch-command-{index}").into_boxed_str());
            let title_selector: &'static str =
                Box::leak(format!("quick-launch-command-title-{index}").into_boxed_str());
            let context_selector: &'static str =
                Box::leak(format!("quick-launch-command-context-{index}").into_boxed_str());
            assert_eq!(
                window
                    .debug_bounds(row_selector)
                    .expect("master row")
                    .size
                    .height,
                px(42.0)
            );
            assert_eq!(
                window
                    .debug_bounds(title_selector)
                    .expect("Body row title")
                    .size
                    .height,
                px(LINE_HEIGHT_BODY)
            );
            assert_eq!(
                window
                    .debug_bounds(context_selector)
                    .expect("Label row context")
                    .size
                    .height,
                px(LINE_HEIGHT_LABEL)
            );
            assert!(
                window
                    .debug_bounds(Box::leak(
                        format!("quick-launch-command-enabled-{index}").into_boxed_str()
                    ))
                    .is_none(),
                "enable must belong to the detail heading, not each master row"
            );
            assert!(
                window
                    .debug_bounds(Box::leak(
                        format!("quick-launch-command-delete-{index}").into_boxed_str()
                    ))
                    .is_none(),
                "delete must not crowd every master row"
            );
        }
        let selected_marker = window
            .debug_bounds("quick-launch-command-selection-0")
            .expect("non-shifting selected-command marker");
        assert_eq!(selected_marker.size.width, px(2.0));
        assert_eq!(
            selected_marker.size.height,
            window
                .debug_bounds("quick-launch-command-0")
                .expect("selected master row")
                .size
                .height
        );
        assert_eq!(
            window
                .debug_bounds("quick-launch-command-title-0")
                .expect("selected command title")
                .left(),
            window
                .debug_bounds("quick-launch-command-title-1")
                .expect("unselected command title")
                .left(),
            "the selected edge must not shift the command copy rail"
        );

        for selector in [
            "quick-launch-add-custom-command",
            "quick-launch-command-enabled",
            "quick-launch-delete-custom-command",
            "quick-launch-pick-command-program",
            "quick-launch-save-custom-command",
        ] {
            assert_eq!(
                window
                    .debug_bounds(selector)
                    .unwrap_or_else(|| panic!("missing bounded action {selector}"))
                    .size
                    .height,
                px(28.0)
            );
        }
        for selector in [
            "quick-launch-command-field-keyword",
            "quick-launch-command-field-title",
            "quick-launch-command-field-program",
            "quick-launch-command-field-arguments",
            "quick-launch-command-field-working-directory",
        ] {
            assert_eq!(
                window
                    .debug_bounds(selector)
                    .unwrap_or_else(|| panic!("missing shared field shell {selector}"))
                    .size
                    .height,
                px(32.0)
            );
        }

        window.update(|window, cx| {
            main_window.update(cx, |main_window, cx| {
                main_window
                    .custom_command_keyword
                    .update(cx, |input, cx| input.set_value("unsaved-docs", window, cx));
            });
        });
        let second_row = window
            .debug_bounds("quick-launch-command-1")
            .expect("second command row");
        window.simulate_click(second_row.center(), Modifiers::default());
        assert_eq!(
            window.read(|cx| main_window.read(cx).custom_command_keyword.read(cx).value()),
            "build"
        );
        assert_eq!(
            window.update(|_, cx| {
                quick_launch.read(cx).settings().custom_commands[0]
                    .keyword
                    .clone()
            }),
            "docs",
            "changing master selection must not implicitly save the previous draft"
        );

        window.refresh().expect("render selected command editor");
        let enabled = window
            .debug_bounds("quick-launch-command-enabled")
            .expect("detail enabled checkbox");
        window.simulate_click(enabled.center(), Modifiers::default());
        window.run_until_parked();
        assert!(
            window.update(|_, cx| quick_launch.read(cx).settings().custom_commands[1].enabled),
            "the detail checkbox must preserve explicit immediate persistence"
        );

        window.refresh().expect("render enabled command editor");
        let picker = window
            .debug_bounds("quick-launch-pick-command-program")
            .expect("program picker action");
        window.simulate_click(picker.center(), Modifiers::default());
        window.run_until_parked();
        let picker_command = platform_endpoint
            .try_receive_command_for_contract_test()
            .expect("one typed program picker command");
        assert!(matches!(
            picker_command.message,
            Message::PlatformCommand(PlatformCommand::PickProgram(_))
        ));
        assert!(
            platform_endpoint
                .try_receive_command_for_contract_test()
                .is_err(),
            "one picker gesture must emit one typed command"
        );
        window.update(|_, cx| {
            quick_launch.update(cx, |application, cx| {
                application.handle_platform_notification(
                    PlatformNotification::CommandResult {
                        correlation_id: uuid::Uuid::new_v4(),
                        result: PlatformCommandResult {
                            success: true,
                            error: None,
                            native_error_code: None,
                            data: Some(r"E:\Picked\wrong-correlation.exe".into()),
                        },
                    },
                    cx,
                );
            });
        });
        window.run_until_parked();
        assert_eq!(
            window.read(|cx| main_window.read(cx).custom_command_program.read(cx).value()),
            r"D:\Tools\build.exe",
            "a non-matching platform correlation must not reach the editor draft"
        );
        let first_row = window
            .debug_bounds("quick-launch-command-0")
            .expect("first command row");
        window.simulate_click(first_row.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("render first command draft");
        let second_row = window
            .debug_bounds("quick-launch-command-1")
            .expect("second command row after leaving it");
        window.simulate_click(second_row.center(), Modifiers::default());
        window.run_until_parked();
        window.update(|_, cx| {
            quick_launch.update(cx, |application, cx| {
                application.handle_platform_notification(
                    PlatformNotification::CommandResult {
                        correlation_id: picker_command.id,
                        result: PlatformCommandResult {
                            success: true,
                            error: None,
                            native_error_code: None,
                            data: Some(r"E:\Picked\replacement.exe".into()),
                        },
                    },
                    cx,
                );
            });
        });
        window.run_until_parked();
        assert_eq!(
            window.read(|cx| main_window.read(cx).custom_command_program.read(cx).value()),
            r"D:\Tools\build.exe",
            "leaving and returning to the same index must still reject the stale picker identity"
        );

        window.refresh().expect("render cancellation picker");
        let picker = window
            .debug_bounds("quick-launch-pick-command-program")
            .expect("cancellation picker action");
        window.simulate_click(picker.center(), Modifiers::default());
        window.run_until_parked();
        let cancelled_picker = platform_endpoint
            .try_receive_command_for_contract_test()
            .expect("one picker to cancel");
        window.update(|_, cx| {
            quick_launch.update(cx, |application, cx| {
                application.handle_platform_notification(
                    PlatformNotification::CommandResult {
                        correlation_id: cancelled_picker.id,
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
        });
        window.run_until_parked();
        assert_eq!(
            window.read(|cx| main_window.read(cx).custom_command_program.read(cx).value()),
            r"D:\Tools\build.exe",
            "picker cancellation must not modify the current draft"
        );

        window.refresh().expect("render first command selection");
        let first_row = window
            .debug_bounds("quick-launch-command-0")
            .expect("first command row after picker cancellation");
        window.simulate_click(first_row.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("render current command picker");
        let picker = window
            .debug_bounds("quick-launch-pick-command-program")
            .expect("current command picker action");
        window.simulate_click(picker.center(), Modifiers::default());
        window.run_until_parked();
        let current_picker = platform_endpoint
            .try_receive_command_for_contract_test()
            .expect("one picker for the current draft");
        window.update(|_, cx| {
            quick_launch.update(cx, |application, cx| {
                application.handle_platform_notification(
                    PlatformNotification::CommandResult {
                        correlation_id: current_picker.id,
                        result: PlatformCommandResult {
                            success: true,
                            error: None,
                            native_error_code: None,
                            data: Some(r"E:\Picked\docs.exe".into()),
                        },
                    },
                    cx,
                );
            });
        });
        window.run_until_parked();
        assert_eq!(
            window.read(|cx| main_window.read(cx).custom_command_program.read(cx).value()),
            r"E:\Picked\docs.exe"
        );
        assert_eq!(
            window.update(|_, cx| {
                quick_launch.read(cx).settings().custom_commands[0]
                    .program
                    .clone()
            }),
            r"C:\Tools\docs.exe",
            "a matching picker result must remain an unsaved editor draft"
        );

        window.update(|window, cx| {
            main_window.update(cx, |main_window, cx| {
                main_window
                    .custom_command_title
                    .update(cx, |input, cx| input.set_value("Docs search", window, cx));
            });
        });
        window.refresh().expect("render edited draft");
        let save = window
            .debug_bounds("quick-launch-save-custom-command")
            .expect("save command action");
        window.simulate_click(save.center(), Modifiers::default());
        window.run_until_parked();
        assert_eq!(
            window.update(|_, cx| {
                quick_launch.read(cx).settings().custom_commands[0]
                    .title
                    .clone()
            }),
            "Docs search"
        );
        assert_eq!(
            window.update(|_, cx| {
                quick_launch.read(cx).settings().custom_commands[0]
                    .program
                    .clone()
            }),
            r"E:\Picked\docs.exe"
        );

        window.refresh().expect("render command before delete");
        let delete = window
            .debug_bounds("quick-launch-delete-custom-command")
            .expect("detail delete action");
        window.simulate_click(delete.center(), Modifiers::default());
        window.run_until_parked();
        assert_eq!(
            window.update(|_, cx| quick_launch.read(cx).settings().custom_commands.len()),
            1
        );
        assert_eq!(
            window.read(|cx| main_window.read(cx).selected_custom_command),
            Some(0)
        );

        window.refresh().expect("render add command action");
        let add = window
            .debug_bounds("quick-launch-add-custom-command")
            .expect("master add action");
        window.simulate_click(add.center(), Modifiers::default());
        window.run_until_parked();
        assert_eq!(
            window.update(|_, cx| quick_launch.read(cx).settings().custom_commands.len()),
            2
        );
        assert_eq!(
            window.read(|cx| main_window.read(cx).selected_custom_command),
            Some(1)
        );
    }

    #[gpui::test]
    fn quick_launch_hotkey_field_exposes_the_native_registration_lifecycle(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
        let temp = tempfile::tempdir().expect("temporary Quick Launch hotkey fixture");
        let catalog = FinderCatalog::open(temp.path().join("history.db"), &[])
            .expect("open Quick Launch hotkey catalog");
        let (platform, platform_endpoint) = platform_bridge(16);
        platform_endpoint.set_connected_for_contract_test(true);
        let (indexer, _indexer_endpoint) = crate::runtime::indexer_bridge(8);
        let quick_launch = cx.update(|cx| {
            cx.new(|_| {
                QuickLaunchApplication::new(
                    catalog,
                    Rc::new(platform),
                    Rc::new(indexer),
                    AppConfig::default(),
                    temp.path().join("config.json"),
                    temp.path().join("logs"),
                    notifications.clone(),
                )
            })
        });
        quick_launch.update(cx, |application, cx| {
            application.handle_platform_notification(PlatformNotification::Connected, cx);
        });
        let initial_configuration = platform_endpoint
            .try_receive_command_for_contract_test()
            .expect("apply the initial Quick Launch configuration");
        assert!(matches!(
            initial_configuration.message,
            Message::PlatformCommand(PlatformCommand::ConfigureQuickLaunch(_))
        ));
        let original_hotkey = cx.read(|cx| quick_launch.read(cx).settings().hotkey);
        let quick_launch_for_window = quick_launch.downgrade();
        let notifications_for_window = notifications.clone();
        let (_main_window, window) = cx.add_window_view(move |window, cx| {
            window.resize(gpui::size(px(1060.0), px(500.0)));
            let mut main_window = MainWindow::new(
                gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
                quick_launch_for_window,
                gpui::WeakEntity::<DownloadApplication>::new_invalid(),
                MainWindowShellContext {
                    settings: None,
                    platform: None,
                    system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                },
                notifications_for_window,
                window,
                cx,
            );
            main_window.shell.activate(MainModule::QuickFind);
            main_window
        });

        window
            .refresh()
            .expect("render initial applying Quick Launch hotkey");
        assert!(
            window
                .debug_bounds("quick-launch-hotkey-field-state-applying")
                .is_some()
        );
        window.update(|_, cx| {
            quick_launch.update(cx, |application, cx| {
                application.handle_platform_notification(
                    PlatformNotification::CommandResult {
                        correlation_id: initial_configuration.id,
                        result: PlatformCommandResult {
                            success: false,
                            error: Some("initial hotkey registration was rejected".into()),
                            native_error_code: Some(1409),
                            data: None,
                        },
                    },
                    cx,
                );
            });
        });
        window.run_until_parked();
        window
            .refresh()
            .expect("render failed initial Quick Launch hotkey");
        assert!(
            window
                .debug_bounds("quick-launch-hotkey-field-state-failed")
                .is_some()
        );
        assert_eq!(
            window.update(|_, cx| quick_launch.read(cx).settings().hotkey),
            original_hotkey,
            "a rejected initial registration must keep the configured hotkey"
        );

        window.update(|_, cx| {
            quick_launch.update(cx, |application, cx| {
                application.handle_platform_notification(PlatformNotification::Connected, cx);
            });
        });
        let retry_configuration = (0..8)
            .filter_map(|_| {
                platform_endpoint
                    .try_receive_command_for_contract_test()
                    .ok()
            })
            .find(|envelope| {
                matches!(
                    envelope.message,
                    Message::PlatformCommand(PlatformCommand::ConfigureQuickLaunch(_))
                )
            })
            .expect("retry the initial Quick Launch configuration");
        window.update(|_, cx| {
            quick_launch.update(cx, |application, cx| {
                application.handle_platform_notification(
                    PlatformNotification::CommandResult {
                        correlation_id: retry_configuration.id,
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
        });
        window.run_until_parked();
        window
            .refresh()
            .expect("render applied Quick Launch hotkey");
        let field = window
            .debug_bounds("quick-launch-hotkey-field-control")
            .expect("shared Quick Launch HotkeyField");
        assert_eq!(field.size.height, px(32.0));
        assert_eq!(
            window
                .debug_bounds("quick-launch-hotkey-field-value")
                .expect("HotkeyField value")
                .size
                .height,
            px(LINE_HEIGHT_BODY)
        );
        assert_eq!(
            window
                .debug_bounds("quick-launch-hotkey-field-status")
                .expect("HotkeyField status")
                .size
                .height,
            px(LINE_HEIGHT_LABEL)
        );
        assert_eq!(
            window
                .debug_bounds("quick-launch-hotkey-setting-title")
                .expect("HotkeyField setting title")
                .size
                .height,
            px(LINE_HEIGHT_BODY)
        );
        assert_eq!(
            window
                .debug_bounds("quick-launch-hotkey-setting-description")
                .expect("HotkeyField setting description")
                .size
                .height,
            px(LINE_HEIGHT_LABEL)
        );
        assert!(
            window
                .debug_bounds("quick-launch-hotkey-field-state-applied")
                .is_some()
        );

        window.simulate_click(field.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("render waiting HotkeyField");
        assert!(
            window
                .debug_bounds("quick-launch-hotkey-field-state-waiting")
                .is_some()
        );
        let recording_envelope = (0..8)
            .filter_map(|_| {
                platform_endpoint
                    .try_receive_command_for_contract_test()
                    .ok()
            })
            .find(|envelope| {
                matches!(
                    &envelope.message,
                    Message::PlatformCommand(PlatformCommand::SetQuickLaunchHotKeyRecording(
                        command
                    )) if command.recording
                )
            })
            .expect("pause the native Quick Launch trigger before recording");
        window.update(|_, cx| {
            quick_launch.update(cx, |application, cx| {
                application.handle_platform_notification(
                    PlatformNotification::CommandResult {
                        correlation_id: recording_envelope.id,
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
        });
        window.run_until_parked();
        window.refresh().expect("render recording HotkeyField");
        assert!(
            window
                .debug_bounds("quick-launch-hotkey-field-state-recording")
                .is_some()
        );
        while platform_endpoint
            .try_receive_command_for_contract_test()
            .is_ok()
        {}
        window.simulate_click(field.center(), Modifiers::default());
        window.run_until_parked();
        window
            .refresh()
            .expect("keep the recording HotkeyField stable after pointer input");
        assert!(
            window
                .debug_bounds("quick-launch-hotkey-field-state-recording")
                .is_some(),
            "recording may be cancelled only through Escape or focus loss"
        );
        assert!(
            platform_endpoint
                .try_receive_command_for_contract_test()
                .is_err(),
            "clicking the recording field must not send a second native command"
        );

        window.simulate_keystrokes("ctrl-alt-v");
        window.run_until_parked();
        let apply_envelope = (0..8)
            .filter_map(|_| {
                platform_endpoint
                    .try_receive_command_for_contract_test()
                    .ok()
            })
            .find(|envelope| {
                matches!(
                    &envelope.message,
                    Message::PlatformCommand(PlatformCommand::ConfigureQuickLaunch(command))
                        if command.hotkey_display == "Ctrl+Alt+V"
                )
            })
            .expect("apply the recorded Quick Launch hotkey");
        window.refresh().expect("render applying HotkeyField");
        assert!(
            window
                .debug_bounds("quick-launch-hotkey-field-state-applying")
                .is_some()
        );
        window.update(|_, cx| {
            quick_launch.update(cx, |application, cx| {
                application.handle_platform_notification(
                    PlatformNotification::CommandResult {
                        correlation_id: uuid::Uuid::new_v4(),
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
        });
        window.run_until_parked();
        window
            .refresh()
            .expect("ignore a stale successful hotkey result");
        assert!(
            window
                .debug_bounds("quick-launch-hotkey-field-state-applying")
                .is_some(),
            "an unrelated success cannot complete the pending hotkey"
        );
        assert_eq!(
            window.update(|_, cx| quick_launch.read(cx).settings().hotkey),
            original_hotkey,
            "an unrelated success cannot replace the applied hotkey"
        );

        window.update(|_, cx| {
            quick_launch.update(cx, |application, cx| {
                application.handle_platform_notification(
                    PlatformNotification::CommandResult {
                        correlation_id: apply_envelope.id,
                        result: PlatformCommandResult {
                            success: false,
                            error: Some("hotkey is already registered".into()),
                            native_error_code: Some(1409),
                            data: None,
                        },
                    },
                    cx,
                );
            });
        });
        window.run_until_parked();
        window.refresh().expect("render failed HotkeyField");
        assert!(
            window
                .debug_bounds("quick-launch-hotkey-field-state-failed")
                .is_some()
        );
        assert_eq!(
            cx.read(|cx| quick_launch.read(cx).settings().hotkey),
            original_hotkey,
            "a rejected native registration must keep the applied hotkey"
        );
    }

    #[gpui::test]
    fn notifications_use_a_bounded_bottom_layer_without_covering_shell_actions(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
        let action_invoked = Rc::new(Cell::new(false));
        let action_invoked_for_handler = Rc::clone(&action_invoked);
        let notifications_for_action = notifications.clone();
        notifications.update(cx, |center, cx| {
            center.show_progress("transfer", "正在下载", "已完成 42%", cx);
            center.show_error("failure", "下载失败", "服务器暂时不可用", cx);
            center.show_success_with_details(
                "backup",
                "备份完成",
                "已创建可恢复备份",
                vec![NotificationDetail::new(
                    "保存位置",
                    r"C:\Users\Administrator\Downloads\NxTools Backup",
                )],
                Some(NotificationAction::new(
                    "打开目录",
                    move |_window, cx| {
                        action_invoked_for_handler.set(true);
                        let notifications_for_action = notifications_for_action.clone();
                        cx.defer(move |cx| {
                            notifications_for_action.update(cx, |center, cx| {
                                center.show_success(
                                    "backup",
                                    "新的备份通知",
                                    "替换旧 generation",
                                    cx,
                                );
                            });
                        });
                    },
                )),
                cx,
            );
        });
        let notifications_for_window = notifications.clone();
        let (_main_window, window) = cx.add_window_view(move |window, cx| {
            window.resize(size(px(920.0), px(480.0)));
            MainWindow::new(
                gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
                gpui::WeakEntity::<QuickLaunchApplication>::new_invalid(),
                gpui::WeakEntity::<DownloadApplication>::new_invalid(),
                MainWindowShellContext {
                    settings: None,
                    platform: None,
                    system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                },
                notifications_for_window,
                window,
                cx,
            )
        });
        window
            .refresh()
            .expect("render production notification layer");
        window.update(|window, _| {
            assert_eq!(window.bounds().size, size(px(920.0), px(480.0)));
        });

        let titlebar = window.debug_bounds("main-titlebar").expect("main titlebar");
        let window_controls = window
            .debug_bounds("main-window-controls")
            .expect("window control group");
        let header_actions = window
            .debug_bounds("download-module-header-actions")
            .expect("module primary actions");
        let stack = window
            .debug_bounds("notification-stack")
            .expect("notification stack");
        let main_surface = window
            .debug_bounds("nxtools-main-window")
            .expect("main window surface");
        assert!(stack.top() >= header_actions.bottom());
        assert!(stack.top() >= titlebar.bottom());
        assert!(stack.top() >= window_controls.bottom());
        assert!(
            stack.right() <= main_surface.right() - px(SPACING_12) + px(1.0),
            "notification stack right edge is {}px",
            stack.right().as_f32()
        );
        assert!(
            stack.bottom() <= main_surface.bottom() - px(SPACING_12) + px(1.0),
            "notification stack bottom edge is {}px",
            stack.bottom().as_f32()
        );

        for id in ["backup", "failure", "transfer"] {
            let card = window
                .debug_bounds(Box::leak(format!("notification-{id}").into_boxed_str()))
                .unwrap_or_else(|| panic!("notification card {id}"));
            let marker = window
                .debug_bounds(Box::leak(
                    format!("notification-marker-{id}").into_boxed_str(),
                ))
                .unwrap_or_else(|| panic!("semantic marker {id}"));
            let title = window
                .debug_bounds(Box::leak(
                    format!("notification-title-{id}").into_boxed_str(),
                ))
                .unwrap_or_else(|| panic!("notification title {id}"));
            let message = window
                .debug_bounds(Box::leak(
                    format!("notification-message-{id}").into_boxed_str(),
                ))
                .unwrap_or_else(|| panic!("notification message {id}"));
            let dismiss = window
                .debug_bounds(Box::leak(
                    format!("dismiss-notification-{id}").into_boxed_str(),
                ))
                .unwrap_or_else(|| panic!("notification dismiss {id}"));
            assert_eq!(card.size.width, px(336.0));
            assert!(card.size.height <= px(112.0));
            assert_eq!(marker.size.width, px(3.0));
            assert_eq!(marker.top(), card.top() + px(1.0));
            assert_eq!(marker.bottom(), card.bottom() - px(1.0));
            assert_eq!(title.size.height, px(LINE_HEIGHT_BODY));
            assert_eq!(message.size.height, px(LINE_HEIGHT_LABEL));
            assert_eq!(dismiss.size, size(px(28.0), px(28.0)));
        }
        let backup = window
            .debug_bounds("notification-backup")
            .expect("detailed notification");
        let action = window
            .debug_bounds("notification-action-backup")
            .expect("notification action");
        assert_eq!(action.size.height, px(28.0));
        assert!(backup.contains(&action.origin));
        assert!(backup.contains(&action.bottom_right()));

        window.simulate_click(action.center(), Modifiers::default());
        window.run_until_parked();
        assert!(action_invoked.get());
        assert!(
            window.read(|cx| notifications.read(cx).items().iter().any(|item| {
                item.id.as_ref() == "backup"
                    && item.title.as_ref() == "新的备份通知"
                    && !item.exiting
            }))
        );

        let dismiss = window
            .debug_bounds("dismiss-notification-transfer")
            .expect("notification dismiss hit target");
        window.simulate_click(dismiss.center(), Modifiers::default());
        window.update(|_, cx| {
            notifications.update(cx, |center, cx| {
                center.show_progress("transfer", "新的下载通知", "替换旧 generation", cx);
            });
        });
        window.run_until_parked();
        assert!(window.read(|cx| {
            notifications.read(cx).items().iter().any(|item| {
                item.id.as_ref() == "transfer"
                    && item.title.as_ref() == "新的下载通知"
                    && !item.exiting
            })
        }));
    }

    #[gpui::test]
    fn titlebar_move_and_close_reach_transport_without_destroying_the_main_entity(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let executor = cx.executor();
        let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
        let (platform, endpoint) = platform_bridge(8);
        endpoint.set_connected_for_contract_test(true);
        let platform = Rc::new(platform);
        let platform_for_window = Rc::clone(&platform);
        let system_settings = Rc::new(RefCell::new(SystemSettings {
            start_on_login_silently: false,
            close_to_tray: true,
        }));
        let (main_window, window) = cx.add_window_view(move |window, cx| {
            window.resize(size(px(920.0), px(480.0)));
            MainWindow::new(
                gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
                gpui::WeakEntity::<QuickLaunchApplication>::new_invalid(),
                gpui::WeakEntity::<DownloadApplication>::new_invalid(),
                MainWindowShellContext {
                    settings: None,
                    platform: Some(platform_for_window),
                    system_settings,
                },
                notifications,
                window,
                cx,
            )
        });
        window.update(|_, cx| {
            main_window.update(cx, |main_window, _| {
                // TestAppContext has no Windows HWND; inject the opaque value that the real
                // native boundary supplies, then verify it survives the typed transport.
                main_window.native_window_handle = 42;
            });
        });
        window.refresh().expect("render real main titlebar");

        let titlebar_shell = window
            .debug_bounds("main-titlebar")
            .expect("main titlebar shell");
        assert_eq!(titlebar_shell.size.height, px(36.0));
        let brand_icon = window
            .debug_bounds("main-brand-icon")
            .expect("real NxTools brand icon");
        assert_eq!(brand_icon.size, gpui::size(px(18.0), px(18.0)));
        assert_eq!(
            window
                .debug_bounds("main-brand-title")
                .expect("brand title")
                .size
                .height,
            px(LINE_HEIGHT_BODY)
        );
        assert_eq!(
            window
                .debug_bounds("main-active-module")
                .expect("active module metadata")
                .size
                .height,
            px(LINE_HEIGHT_LABEL)
        );
        let global_settings = window
            .debug_bounds("global-network-settings")
            .expect("global settings action");
        assert_eq!(global_settings.size.height, px(28.0));
        let utilities = window
            .debug_bounds("main-titlebar-utilities")
            .expect("right-side titlebar utility group");
        let control_group = window
            .debug_bounds("main-window-controls")
            .expect("window control group");
        for bounds in [global_settings, control_group] {
            assert!(utilities.contains(&bounds.origin));
            assert!(utilities.contains(&bounds.bottom_right()));
            assert!(titlebar_shell.contains(&bounds.origin));
            assert!(titlebar_shell.contains(&bounds.bottom_right()));
        }

        let titlebar = window
            .debug_bounds("main-titlebar-drag-region")
            .expect("titlebar drag bounds");
        assert!(titlebar.right() <= utilities.left());
        window.simulate_mouse_down(titlebar.center(), MouseButton::Left, Modifiers::default());
        let move_envelope = endpoint
            .try_receive_command_for_contract_test()
            .expect("typed move command");
        assert!(matches!(
            move_envelope.message,
            Message::PlatformCommand(PlatformCommand::BeginWindowMove(_))
        ));

        let minimize = window
            .debug_bounds("window-control-minimize")
            .expect("minimize control");
        let maximize = window
            .debug_bounds("window-control-maximize")
            .expect("maximize control");
        let close = window
            .debug_bounds("window-control-close")
            .expect("close control");
        for control in [minimize, maximize, close] {
            assert_eq!(control.size.width, px(TITLEBAR_HEIGHT));
            assert_eq!(control.size.height, titlebar_shell.size.height - px(1.0));
        }
        assert_eq!(minimize.right(), maximize.left());
        assert_eq!(maximize.right(), close.left());
        assert_eq!(minimize.left() - global_settings.right(), px(SPACING_4));
        for selector in [
            "window-control-minimize-icon",
            "window-control-maximize-icon",
            "window-control-close-icon",
        ] {
            assert_eq!(
                window
                    .debug_bounds(selector)
                    .unwrap_or_else(|| panic!("rendered caption asset {selector}"))
                    .size,
                size(px(12.0), px(12.0))
            );
        }
        window.simulate_mouse_move(minimize.center(), None, Modifiers::default());
        executor.advance_clock(Duration::from_millis(600));
        executor.run_until_parked();
        window.refresh().expect("render caption tooltip");
        assert!(
            window
                .debug_bounds("window-control-minimize-tooltip")
                .is_some(),
            "caption control exposes its visible tooltip"
        );

        window.simulate_click(minimize.center(), Modifiers::default());
        let minimize_envelope = endpoint
            .try_receive_command_for_contract_test()
            .expect("typed minimize command");
        assert!(matches!(
            minimize_envelope.message,
            Message::PlatformCommand(PlatformCommand::SetWindowMinimized(_))
        ));

        window.simulate_click(maximize.center(), Modifiers::default());
        let maximize_envelope = endpoint
            .try_receive_command_for_contract_test()
            .expect("typed maximize command");
        assert!(matches!(
            maximize_envelope.message,
            Message::PlatformCommand(PlatformCommand::ToggleWindowMaximized(_))
        ));
        let second_toggle = window
            .debug_bounds("window-control-maximize")
            .expect("maximize toggle control remains until native state changes");
        window.simulate_click(second_toggle.center(), Modifiers::default());
        let second_toggle_envelope = endpoint
            .try_receive_command_for_contract_test()
            .expect("typed second toggle envelope");
        assert!(matches!(
            second_toggle_envelope.message,
            Message::PlatformCommand(PlatformCommand::ToggleWindowMaximized(_))
        ));

        window.simulate_click(close.center(), Modifiers::default());
        let close_envelope = endpoint
            .try_receive_command_for_contract_test()
            .expect("typed close-to-tray command");
        assert!(matches!(
            close_envelope.message,
            Message::PlatformCommand(PlatformCommand::SetWindowVisibility(ref command))
                if !command.visible
        ));
        window.update(|window, cx| {
            main_window.update(cx, |_, _| ());
            window.activate_window();
            main_window.update(cx, |_, _| ());
        });
    }

    #[test]
    fn download_multi_selection_matches_windows_ctrl_and_shift_conventions() {
        let tasks = (0..4).map(|_| DownloadTaskId::new()).collect::<Vec<_>>();
        let mut selected = HashSet::new();
        let mut primary = None;
        let mut anchor = None;

        update_download_selection(
            &mut selected,
            &mut primary,
            &mut anchor,
            &tasks,
            tasks[1],
            false,
            false,
        );
        update_download_selection(
            &mut selected,
            &mut primary,
            &mut anchor,
            &tasks,
            tasks[3],
            false,
            true,
        );
        assert_eq!(selected, HashSet::from([tasks[1], tasks[2], tasks[3]]));

        update_download_selection(
            &mut selected,
            &mut primary,
            &mut anchor,
            &tasks,
            tasks[2],
            true,
            false,
        );
        assert!(!selected.contains(&tasks[2]));
        assert_eq!(primary, Some(tasks[1]));
        assert_eq!(anchor, Some(tasks[2]));

        selected.clear();
        selected.insert(tasks[0]);
        primary = Some(tasks[0]);
        anchor = Some(tasks[0]);
        update_download_selection(
            &mut selected,
            &mut primary,
            &mut anchor,
            &tasks,
            tasks[0],
            true,
            false,
        );
        assert!(selected.is_empty());
        assert_eq!(primary, None);
    }

    use super::{
        DOWNLOAD_BATCH_TOOLBAR_HEIGHT, DOWNLOAD_DETAIL_HEIGHT, DOWNLOAD_FILTER_STRIP_HEIGHT,
        DOWNLOAD_TABLE_HEADER_HEIGHT, DOWNLOAD_TASK_ROW_HEIGHT, DownloadContextCapabilities,
        DownloadContextMenuPlan, DownloadDetailTab, DownloadSortKey, DownloadSortState,
        DownloadTaskFilter, MainWindow, MainWindowControl, MainWindowShellContext,
        QuickLaunchSettingsPage, download_context_capabilities, download_context_menu_plan,
        download_filter_counts, download_queue_placement_capabilities,
        download_segment_strip_cells, download_tasks_for_view, file_index_summary,
        format_query_elapsed, hotkey_from_key_down, masked_request_header_value, maximize_control,
        notification_transition, prepare_download_context_selection, quick_find_scroll_step,
        quick_find_scroll_target, reconcile_download_selection, set_download_sort,
        toggle_indexed_volume_selection, update_download_selection,
    };
    use crate::ui::metrics::{
        MODULE_HEADER_HEIGHT, SIDEBAR_ROW_HEIGHT, SIDEBAR_WIDTH, SPACING_4, SPACING_12,
        TITLEBAR_HEIGHT,
    };
    use crate::ui::typography::{LINE_HEIGHT_BODY, LINE_HEIGHT_LABEL, LINE_HEIGHT_TITLE};

    #[test]
    fn maximize_state_selects_a_distinct_restore_caption_asset() {
        let maximize = maximize_control(false);
        let restore = maximize_control(true);

        assert_eq!(maximize, MainWindowControl::Maximize);
        assert_eq!(restore, MainWindowControl::Restore);
        assert_ne!(maximize.icon(), restore.icon());
        assert_ne!(maximize.id(), restore.id());
        assert_eq!(MainWindowControl::Minimize.tooltip(), "最小化");
        assert_eq!(maximize.tooltip(), "最大化");
        assert_eq!(restore.tooltip(), "还原");
        assert_eq!(MainWindowControl::Close.tooltip(), "关闭");
    }

    #[test]
    fn download_workspace_uses_the_compact_editor_density_scale() {
        assert_eq!(MODULE_HEADER_HEIGHT, 48.0);
        assert_eq!(DOWNLOAD_FILTER_STRIP_HEIGHT, 38.0);
        assert_eq!(DOWNLOAD_TABLE_HEADER_HEIGHT, 28.0);
        assert_eq!(DOWNLOAD_TASK_ROW_HEIGHT, 46.0);
        assert_eq!(DOWNLOAD_BATCH_TOOLBAR_HEIGHT, 36.0);
        assert_eq!(DOWNLOAD_DETAIL_HEIGHT, 176.0);
    }

    #[test]
    fn download_detail_masks_credential_headers_but_keeps_ordinary_values() {
        assert_eq!(
            masked_request_header_value("Authorization", "Bearer private"),
            "已设置（内容已隐藏）"
        );
        assert_eq!(
            masked_request_header_value("X-API-Key", "private"),
            "已设置（内容已隐藏）"
        );
        assert_eq!(
            masked_request_header_value("X-Auth-Token", "private"),
            "已设置（内容已隐藏）"
        );
        assert_eq!(
            masked_request_header_value("X-ApiKey", "private"),
            "已设置（内容已隐藏）"
        );
        assert_eq!(
            masked_request_header_value("X_API_KEY", "private"),
            "已设置（内容已隐藏）"
        );
        assert_eq!(
            masked_request_header_value("User-Agent", "NxTools-Test"),
            "NxTools-Test"
        );
    }

    #[test]
    fn download_segment_strip_is_proportional_and_shows_partial_progress() {
        let segments = vec![
            DownloadSegment::new(0, 0, 74, 38, DownloadSegmentState::Active)
                .expect("first segment"),
            DownloadSegment::new(1, 75, 99, 25, DownloadSegmentState::Completed)
                .expect("second segment"),
        ];

        let cells = download_segment_strip_cells(&segments, 40);

        assert_eq!(cells.len(), 40);
        assert_eq!(
            cells.iter().filter(|cell| cell.segment_index == 0).count(),
            30
        );
        assert_eq!(
            cells.iter().filter(|cell| cell.segment_index == 1).count(),
            10
        );
        assert_eq!(
            cells
                .iter()
                .filter(|cell| cell.segment_index == 0 && cell.downloaded)
                .count(),
            15
        );
        assert!(
            cells
                .iter()
                .filter(|cell| cell.segment_index == 1)
                .all(|cell| cell.downloaded && cell.state == DownloadSegmentState::Completed)
        );
    }

    #[test]
    fn selected_download_detail_query_is_deferred_across_the_gpui_entity_boundary() {
        let source = include_str!("main_window.rs");
        let downloads = source
            .split("fn render_downloads")
            .nth(1)
            .and_then(|source| source.split("fn render_quick_find").next())
            .expect("download render source");
        let defer = downloads
            .find("cx.defer(move |cx|")
            .expect("deferred detail query");
        let request = downloads
            .find("application.request_task_detail(task_id, cx)")
            .expect("typed detail request");

        assert!(defer < request);
        assert!(
            !downloads[..defer].contains("application.request_task_detail(task_id, cx)"),
            "render must not synchronously re-enter DownloadApplication"
        );
    }

    #[test]
    fn download_workspace_is_one_filter_strip_flat_table_and_docked_inspector() {
        let source = include_str!("main_window.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("production main-window source");
        let downloads = source
            .split("fn render_downloads")
            .nth(1)
            .and_then(|source| source.split("fn render_quick_find").next())
            .expect("download workspace source");
        let row = source
            .split("fn download_task_row")
            .nth(1)
            .and_then(|source| source.split("fn download_task_detail").next())
            .expect("download task row source");
        let detail = source
            .split("fn download_task_detail")
            .nth(1)
            .and_then(|source| source.split("fn download_detail_line").next())
            .expect("download detail source");

        assert!(downloads.contains("download-filter-strip"));
        assert!(downloads.contains("download_table_header("));
        assert!(production.contains("download-table-header"));
        assert!(!downloads.contains("download-filter-sidebar"));
        assert!(row.contains(".border_b_1()"));
        assert!(row.contains(".border_l_2()"));
        assert!(!row.contains(".rounded(px(6.0))"));
        assert!(!row.contains("Button::new"));
        assert!(!row.contains("DOWNLOAD_ACTIONS_COLUMN_WIDTH"));
        assert!(
            !row.contains(".truncate()"),
            "nested percentage-width text must not collapse to an ellipsis-only layout"
        );
        assert!(row.contains(".whitespace_nowrap()"));
        assert!(!production.contains(".child(\"操作\")"));
        assert!(detail.contains(".border_t_1()"));
        assert!(!detail.contains(".rounded(px(6.0))"));
        assert!(!detail.contains("\"无错误\""));
    }

    #[test]
    fn download_table_is_virtualized_and_keeps_one_scroll_handle() {
        let source = include_str!("main_window.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("production main-window source");
        let downloads = source
            .split("fn render_downloads")
            .nth(1)
            .and_then(|source| source.split("fn render_quick_find").next())
            .expect("download workspace source");

        assert!(production.contains("UniformListScrollHandle"));
        assert!(downloads.contains("uniform_list("));
        assert!(downloads.contains("track_scroll(self.download_list_scroll.clone())"));
        assert!(!downloads.contains(".children(task_rows)"));
    }

    #[test]
    fn download_search_sort_and_context_selection_follow_reference_semantics() {
        let engine = DownloadEngine::open_in_memory().expect("open in-memory download engine");
        let first = engine
            .enqueue(
                DownloadRequest {
                    source_url: "https://example.com/releases/Zeta.bin".into(),
                    save_directory: r"C:\Downloads".into(),
                    save_directory_explicit: false,
                    file_name: Some("Zeta.bin".into()),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
                    certificate_policy: Default::default(),
                    connection_limit: Some(1),
                    speed_limit_bytes_per_second: None,
                    expected_total_bytes: None,
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
                    media: None,
                    category_id: None,
                    category_name: None,
                    duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
                },
                RequestOrigin::Internal,
            )
            .expect("enqueue first");
        let second = engine
            .enqueue(
                DownloadRequest {
                    source_url: "https://mirror.invalid/alpha.bin".into(),
                    save_directory: r"C:\Downloads".into(),
                    save_directory_explicit: false,
                    file_name: Some("Alpha.bin".into()),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
                    certificate_policy: Default::default(),
                    connection_limit: Some(1),
                    speed_limit_bytes_per_second: None,
                    expected_total_bytes: None,
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
                    media: None,
                    category_id: None,
                    category_name: None,
                    duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
                },
                RequestOrigin::Internal,
            )
            .expect("enqueue second");
        let tasks = engine.tasks().expect("read tasks");

        let queue_order = download_tasks_for_view(
            &tasks,
            DownloadTaskFilter::All,
            "",
            DownloadSortState::default(),
        );
        assert_eq!(
            queue_order
                .iter()
                .map(|task| task.task.id())
                .collect::<Vec<_>>(),
            vec![first, second]
        );
        let search = download_tasks_for_view(
            &tasks,
            DownloadTaskFilter::All,
            "EXAMPLE.COM/RELEASES",
            DownloadSortState::default(),
        );
        assert_eq!(search.len(), 1);
        assert_eq!(search[0].task.id(), first);
        let name_order = download_tasks_for_view(
            &tasks,
            DownloadTaskFilter::All,
            "",
            DownloadSortState {
                key: DownloadSortKey::Name,
                descending: false,
            },
        );
        assert_eq!(
            name_order
                .iter()
                .map(|task| task.task.id())
                .collect::<Vec<_>>(),
            vec![second, first]
        );
        let mut sort = DownloadSortState::default();
        assert!(set_download_sort(&mut sort, DownloadSortKey::Name, true));
        assert_eq!(
            sort,
            DownloadSortState {
                key: DownloadSortKey::Name,
                descending: false,
            }
        );
        assert!(set_download_sort(&mut sort, DownloadSortKey::Name, true));
        assert!(sort.descending);

        let visible = vec![first, second];
        let mut selected = HashSet::from([first]);
        let mut primary = Some(first);
        let mut anchor = Some(first);
        assert_eq!(
            prepare_download_context_selection(
                &mut selected,
                &mut primary,
                &mut anchor,
                &visible,
                second,
            ),
            vec![second]
        );
        selected.insert(first);
        assert_eq!(
            prepare_download_context_selection(
                &mut selected,
                &mut primary,
                &mut anchor,
                &visible,
                second,
            ),
            vec![first, second]
        );

        let mut unfinished_with_target = tasks[0].clone();
        unfinished_with_target.final_path = Some(r"C:\Downloads\Zeta.bin".into());
        let capabilities = download_context_capabilities(&[unfinished_with_target], &[first]);
        assert_eq!(
            capabilities.single_artifact, None,
            "a resolved destination is not yet a committed final artifact"
        );

        let mut paused = tasks[0].clone();
        paused
            .task
            .transition_to(DownloadStatus::Paused, 1)
            .expect("advance fixture to paused");
        paused.queue_eligible = true;
        let paused_capabilities = download_context_capabilities(&[paused], &[first]);
        assert_eq!(paused_capabilities.resume_targets, vec![first]);
        assert!(paused_capabilities.pause_targets.is_empty());
        assert_eq!(paused_capabilities.cancel_targets, vec![first]);
        assert!(paused_capabilities.can_move_queue);
        assert!(paused_capabilities.can_remove);

        let mut failed = tasks[0].clone();
        failed
            .task
            .transition_to(DownloadStatus::Probing, 1)
            .expect("advance failed fixture to probing");
        failed
            .task
            .transition_to(DownloadStatus::Failed, 2)
            .expect("advance fixture to failed");
        failed.queue_eligible = true;
        let failed_capabilities =
            download_context_capabilities(std::slice::from_ref(&failed), &[first]);
        assert_eq!(failed_capabilities.resume_targets, vec![first]);
        assert!(failed_capabilities.pause_targets.is_empty());
        assert!(failed_capabilities.cancel_targets.is_empty());
        assert!(!failed_capabilities.can_move_queue);
        assert!(failed_capabilities.can_remove);

        let mut downloading = tasks[1].clone();
        for status in [
            DownloadStatus::Probing,
            DownloadStatus::Waiting,
            DownloadStatus::Downloading,
        ] {
            downloading
                .task
                .transition_to(status, 1)
                .expect("advance fixture to downloading");
        }
        downloading.queue_eligible = true;
        let downloading_capabilities =
            download_context_capabilities(std::slice::from_ref(&downloading), &[second]);
        assert!(downloading_capabilities.resume_targets.is_empty());
        assert_eq!(downloading_capabilities.pause_targets, vec![second]);
        assert_eq!(downloading_capabilities.cancel_targets, vec![second]);
        assert!(downloading_capabilities.can_move_queue);
        assert!(!downloading_capabilities.can_remove);

        let mixed_capabilities =
            download_context_capabilities(&[failed, downloading], &[first, second]);
        assert_eq!(mixed_capabilities.resume_targets, vec![first]);
        assert_eq!(mixed_capabilities.pause_targets, vec![second]);
        assert_eq!(mixed_capabilities.cancel_targets, vec![second]);
        assert!(!mixed_capabilities.can_move_queue);
        assert!(!mixed_capabilities.can_remove);

        let mut completed = tasks[0].clone();
        for status in [
            DownloadStatus::Probing,
            DownloadStatus::Waiting,
            DownloadStatus::Downloading,
            DownloadStatus::Finalizing,
            DownloadStatus::Completed,
        ] {
            completed
                .task
                .transition_to(status, 1)
                .expect("advance fixture to completed");
        }
        completed.final_path = Some(r"C:\Downloads\Zeta.bin".into());
        completed.queue_eligible = true;
        let completed_capabilities = download_context_capabilities(&[completed], &[first]);
        assert_eq!(
            download_context_menu_plan(&completed_capabilities, 1),
            DownloadContextMenuPlan {
                artifact: true,
                task_window: true,
                resume: false,
                pause: false,
                cancel: false,
                remove: true,
                ..Default::default()
            },
            "completed tasks must not retain lifecycle or queue actions in the menu"
        );
    }

    #[test]
    fn download_context_menu_omits_actions_that_cannot_run_for_the_selection() {
        let active_id = DownloadTaskId::new();
        let active = DownloadContextCapabilities {
            pause_targets: vec![active_id],
            cancel_targets: vec![active_id],
            can_move_queue: true,
            queue_top: true,
            queue_up: true,
            queue_down: true,
            queue_bottom: true,
            ..Default::default()
        };
        assert_eq!(
            download_context_menu_plan(&active, 1),
            DownloadContextMenuPlan {
                task_window: true,
                pause: true,
                cancel: true,
                queue_top: true,
                queue_up: true,
                queue_down: true,
                queue_bottom: true,
                ..Default::default()
            }
        );

        let failed_id = DownloadTaskId::new();
        let failed = DownloadContextCapabilities {
            resume_targets: vec![failed_id],
            can_remove: true,
            ..Default::default()
        };
        assert_eq!(
            download_context_menu_plan(&failed, 1),
            DownloadContextMenuPlan {
                task_window: true,
                resume: true,
                remove: true,
                ..Default::default()
            }
        );

        let paused_id = DownloadTaskId::new();
        let mixed_selection = DownloadContextCapabilities {
            resume_targets: vec![failed_id, paused_id],
            pause_targets: vec![active_id],
            cancel_targets: vec![active_id, paused_id],
            ..Default::default()
        };
        assert_eq!(
            download_context_menu_plan(&mixed_selection, 2),
            DownloadContextMenuPlan {
                resume: true,
                pause: true,
                cancel: true,
                ..Default::default()
            },
            "multi-selection has no single-task or queue-only actions unless every task supports them"
        );

        let source = include_str!("main_window.rs");
        let menu = source
            .split("\nfn download_task_context_menu(")
            .nth(1)
            .and_then(|body| body.split("\nfn download_task_detail(").next())
            .expect("download task context menu");
        assert!(
            !menu.contains(".disabled("),
            "unavailable download actions must be omitted instead of rendered disabled"
        );
    }

    #[test]
    fn queue_position_actions_are_omitted_at_the_real_edges() {
        let first = DownloadTaskId::new();
        let middle = DownloadTaskId::new();
        let last = DownloadTaskId::new();
        let order = [first, middle, last];

        assert_eq!(
            download_queue_placement_capabilities(&order, &[first]),
            (false, false, true, true)
        );
        assert_eq!(
            download_queue_placement_capabilities(&order, &[middle]),
            (true, true, true, true)
        );
        assert_eq!(
            download_queue_placement_capabilities(&order, &[last]),
            (true, true, false, false)
        );
        assert_eq!(
            download_queue_placement_capabilities(&order, &[first, last]),
            (false, false, false, false)
        );
    }

    #[gpui::test]
    fn completed_download_context_menu_renders_only_terminal_actions(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let engine = DownloadEngine::open_in_memory().expect("open in-memory download engine");
        let task_id = engine
            .enqueue(
                DownloadRequest {
                    source_url: "https://example.com/completed.bin".into(),
                    save_directory: r"C:\Downloads".into(),
                    save_directory_explicit: false,
                    file_name: Some("completed.bin".into()),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
                    certificate_policy: Default::default(),
                    connection_limit: Some(1),
                    speed_limit_bytes_per_second: None,
                    expected_total_bytes: None,
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
                    media: None,
                    category_id: None,
                    category_name: None,
                    duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
                },
                RequestOrigin::Internal,
            )
            .expect("enqueue completed-menu fixture");
        let mut completed = engine
            .tasks()
            .expect("load completed-menu fixture")
            .into_iter()
            .next()
            .expect("completed-menu task");
        for status in [
            DownloadStatus::Probing,
            DownloadStatus::Waiting,
            DownloadStatus::Downloading,
            DownloadStatus::Finalizing,
            DownloadStatus::Completed,
        ] {
            completed
                .task
                .transition_to(status, 1)
                .expect("advance completed-menu fixture");
        }
        completed.final_path = Some(r"C:\Downloads\completed.bin".into());
        completed.queue_eligible = true;

        let (platform, platform_endpoint) = nxtools_ipc::platform_bridge(16);
        let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
        let downloads = cx.update(|cx| {
            cx.new(|cx| {
                DownloadApplication::new(
                    engine,
                    std::rc::Rc::new(platform),
                    notifications.clone(),
                    DownloadRuntimeContext {
                        fallback_default_directory: r"C:\Downloads".into(),
                        config: AppConfig::default(),
                        config_path: std::env::temp_dir().join("nxtools-test-config.json"),
                        log_directory: std::env::temp_dir(),
                        database_path: std::env::temp_dir().join("downloads.sqlite3"),
                        ffmpeg_install_directory: std::env::temp_dir()
                            .join("nxtools-test-tools")
                            .join("ffmpeg"),
                    },
                    cx,
                )
            })
        });
        let downloads_for_window = downloads.downgrade();
        let (main_window, window) = cx.add_window_view(move |window, cx| {
            let mut main_window = MainWindow::new(
                gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
                gpui::WeakEntity::<QuickLaunchApplication>::new_invalid(),
                downloads_for_window,
                MainWindowShellContext {
                    settings: None,
                    platform: None,
                    system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                },
                notifications,
                window,
                cx,
            );
            main_window.shell.activate(MainModule::Downloads);
            main_window
        });
        window.update(|_, cx| {
            downloads.update(cx, |application, _| {
                application.replace_tasks_for_test(vec![completed]);
            });
        });
        window.refresh().expect("render completed download fixture");
        window.run_until_parked();
        let element_id = task_id.as_uuid().as_u64_pair().0;
        let row_selector: &'static str =
            Box::leak(format!("download-task-row-{element_id}").into_boxed_str());
        let row = window
            .debug_bounds(row_selector)
            .expect("completed production task row bounds");

        window.simulate_mouse_down(row.center(), MouseButton::Right, Modifiers::default());
        window
            .refresh()
            .expect("refresh completed task context menu");
        window.run_until_parked();

        let visible_selectors = [
            "download-menu-open-file",
            "download-menu-reveal-file",
            "download-menu-task-window",
            "download-menu-delete-file",
            "download-menu-remove",
        ];
        let mut item_width = None;
        let mut open_file_bounds = None;
        for selector in visible_selectors {
            let bounds = window
                .debug_bounds(selector)
                .unwrap_or_else(|| panic!("completed task menu must render {selector}"));
            assert_eq!(
                bounds.size.height,
                px(28.0),
                "shared menu actions must keep the 28 px action rhythm: {selector}"
            );
            assert!(
                (px(160.0)..=px(216.0)).contains(&bounds.size.width),
                "the bounded 192–240 px menu shell must leave a stable content width: {selector}={bounds:?}"
            );
            if let Some(expected) = item_width {
                assert_eq!(
                    bounds.size.width, expected,
                    "all shared menu actions must use one icon/text grid"
                );
            } else {
                item_width = Some(bounds.size.width);
            }
            if selector == "download-menu-open-file" {
                open_file_bounds = Some(bounds);
            }
        }
        for selector in [
            "download-menu-resume",
            "download-menu-pause",
            "download-menu-cancel",
            "download-menu-prioritize",
            "download-menu-move-up",
            "download-menu-move-down",
            "download-menu-move-bottom",
        ] {
            assert!(
                window.debug_bounds(selector).is_none(),
                "completed task menu must omit {selector}"
            );
        }

        window.simulate_keystrokes("escape");
        window.run_until_parked();
        window
            .refresh()
            .expect("refresh dismissed completed task context menu");
        window.run_until_parked();
        window
            .refresh()
            .expect("settle dismissed completed task context menu");
        window.update(|window, cx| {
            assert!(
                main_window.read(cx).hotkey_focus.is_focused(window),
                "dismissal must return keyboard focus to the invoking workspace"
            );
        });
        while platform_endpoint
            .try_receive_command_for_contract_test()
            .is_ok()
        {}
        window.simulate_click(
            open_file_bounds
                .expect("open-file action bounds before dismissal")
                .center(),
            Modifiers::default(),
        );
        window.run_until_parked();
        assert!(
            platform_endpoint
                .try_receive_command_for_contract_test()
                .is_err(),
            "the old menu hit target must not dispatch after Escape dismissal"
        );
    }

    #[test]
    fn download_status_filters_match_the_phase_three_contract() {
        assert!(DownloadTaskFilter::All.matches(DownloadStatus::Cancelled));
        for status in [
            DownloadStatus::Probing,
            DownloadStatus::Downloading,
            DownloadStatus::Finalizing,
            DownloadStatus::Verifying,
        ] {
            assert!(DownloadTaskFilter::Downloading.matches(status));
        }
        for status in [DownloadStatus::Created, DownloadStatus::Waiting] {
            assert!(DownloadTaskFilter::Waiting.matches(status));
        }
        assert!(DownloadTaskFilter::Paused.matches(DownloadStatus::Paused));
        assert!(DownloadTaskFilter::Completed.matches(DownloadStatus::Completed));
        assert!(DownloadTaskFilter::Failed.matches(DownloadStatus::Failed));

        for filter in [
            DownloadTaskFilter::Downloading,
            DownloadTaskFilter::Waiting,
            DownloadTaskFilter::Paused,
            DownloadTaskFilter::Completed,
            DownloadTaskFilter::Failed,
        ] {
            assert!(!filter.matches(DownloadStatus::Cancelled));
        }
    }

    #[test]
    fn download_filter_counts_and_visible_selection_share_the_filtered_snapshot() {
        let counts = download_filter_counts([
            DownloadStatus::Created,
            DownloadStatus::Waiting,
            DownloadStatus::Downloading,
            DownloadStatus::Paused,
            DownloadStatus::Completed,
            DownloadStatus::Failed,
            DownloadStatus::Cancelled,
        ]);
        assert_eq!(counts.all, 7);
        assert_eq!(counts.downloading, 1);
        assert_eq!(counts.waiting, 2);
        assert_eq!(counts.paused, 1);
        assert_eq!(counts.completed, 1);
        assert_eq!(counts.failed, 1);

        let mut selected = HashSet::from([2, 3]);
        assert_eq!(
            reconcile_download_selection(Some(3), &mut selected, &[1, 2, 3], |id| *id),
            Some(3)
        );
        assert_eq!(selected, HashSet::from([2, 3]));

        assert_eq!(
            reconcile_download_selection(Some(3), &mut selected, &[1, 2], |id| *id),
            Some(2),
            "a still-visible member of the multi-selection becomes primary"
        );
        assert_eq!(selected, HashSet::from([2]));

        assert_eq!(
            reconcile_download_selection::<u64, u64>(Some(2), &mut selected, &[], |id| *id),
            None
        );
        assert!(selected.is_empty());

        assert_eq!(
            reconcile_download_selection(None, &mut selected, &[1, 2, 3], |id| *id),
            None,
            "visible rows must not create a selection without an explicit user action"
        );
        assert!(selected.is_empty());
    }

    #[test]
    fn filtered_empty_states_name_the_active_status() {
        assert_eq!(
            DownloadTaskFilter::Downloading.empty_title(),
            "暂无正在下载的任务"
        );
        assert_eq!(DownloadTaskFilter::Waiting.empty_title(), "暂无等待任务");
        assert_eq!(DownloadTaskFilter::Paused.empty_title(), "暂无暂停任务");
        assert_eq!(
            DownloadTaskFilter::Completed.empty_title(),
            "暂无已完成任务"
        );
        assert_eq!(DownloadTaskFilter::Failed.empty_title(), "暂无失败任务");
    }

    #[gpui::test]
    fn clicking_a_download_filter_row_changes_the_real_main_window_state(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
        let (main_window, window) = cx.add_window_view(move |window, cx| {
            let mut main_window = MainWindow::new(
                gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
                gpui::WeakEntity::<QuickLaunchApplication>::new_invalid(),
                gpui::WeakEntity::<DownloadApplication>::new_invalid(),
                MainWindowShellContext {
                    settings: None,
                    platform: None,
                    system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                },
                notifications,
                window,
                cx,
            );
            main_window.shell.activate(MainModule::Downloads);
            main_window
        });
        let bounds = window
            .debug_bounds("download-filter-waiting")
            .expect("rendered waiting filter bounds");
        let filter_strip_bounds = window
            .debug_bounds("download-filter-strip")
            .expect("rendered horizontal filter strip bounds");
        let filter_segments = window
            .debug_bounds("download-filter-segments")
            .expect("status filters must share one segmented-control boundary");
        let table_header_bounds = window
            .debug_bounds("download-table-header")
            .expect("rendered download table header bounds");
        assert_eq!(
            filter_strip_bounds.size.height.as_f32(),
            DOWNLOAD_FILTER_STRIP_HEIGHT
        );
        assert_eq!(
            table_header_bounds.size.height.as_f32(),
            DOWNLOAD_TABLE_HEADER_HEIGHT
        );
        assert_eq!(
            filter_strip_bounds.size.width,
            table_header_bounds.size.width
        );
        let segment_bounds = DownloadTaskFilter::ALL.map(|filter| {
            window
                .debug_bounds(filter.id())
                .unwrap_or_else(|| panic!("rendered status segment {}", filter.id()))
        });
        assert_eq!(filter_segments.size.height.as_f32(), 28.0);
        for (index, segment) in segment_bounds.iter().enumerate() {
            assert_eq!(segment.size.height, segment_bounds[0].size.height);
            assert!(
                (segment.size.width.as_f32() - segment_bounds[0].size.width.as_f32()).abs() <= 0.5,
                "pixel rounding must be the only width difference between status segments"
            );
            assert!(filter_segments.contains(&segment.origin));
            assert!(filter_segments.contains(&segment.bottom_right()));
            if index > 0 {
                assert_eq!(segment.left(), segment_bounds[index - 1].right());
            }
        }
        assert_eq!(
            segment_bounds[0].left().as_f32(),
            filter_segments.left().as_f32() + 1.0
        );
        assert_eq!(
            segment_bounds[5].right().as_f32(),
            filter_segments.right().as_f32() - 1.0
        );
        window.simulate_click(bounds.center(), Modifiers::default());

        assert_eq!(
            cx.read(|cx| main_window.read(cx).download_filter),
            DownloadTaskFilter::Waiting
        );
        assert_eq!(
            cx.read(|cx| main_window.read(cx).selected_download_task),
            None
        );
    }

    #[gpui::test]
    fn clicking_download_table_headers_changes_only_the_view_sort(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
        let (main_window, window) = cx.add_window_view(move |window, cx| {
            let mut main_window = MainWindow::new(
                gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
                gpui::WeakEntity::<QuickLaunchApplication>::new_invalid(),
                gpui::WeakEntity::<DownloadApplication>::new_invalid(),
                MainWindowShellContext {
                    settings: None,
                    platform: None,
                    system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                },
                notifications,
                window,
                cx,
            );
            main_window.shell.activate(MainModule::Downloads);
            main_window
        });
        window.simulate_resize(size(px(920.0), px(480.0)));
        window
            .refresh()
            .expect("render minimum-width download table");
        let name_header = window
            .debug_bounds("download-sort-header-name")
            .expect("rendered sortable name header");
        let created_header = window
            .debug_bounds("download-sort-header-created")
            .expect("minimum-width download table exposes the sortable created header");
        assert_eq!(
            created_header.size.width,
            px(super::DOWNLOAD_CREATED_COLUMN_WIDTH)
        );

        window.simulate_click(name_header.center(), Modifiers::default());
        window.simulate_click(name_header.center(), Modifiers::default());
        assert_eq!(
            window.read(|cx| main_window.read(cx).download_sort),
            DownloadSortState {
                key: DownloadSortKey::Name,
                descending: true,
            }
        );
        window.simulate_click(created_header.center(), Modifiers::default());
        assert_eq!(
            window.read(|cx| main_window.read(cx).download_sort),
            DownloadSortState {
                key: DownloadSortKey::Created,
                descending: true,
            }
        );
    }

    #[gpui::test]
    fn production_download_row_renders_resolved_shell_icon_without_generic_tile(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let engine = DownloadEngine::open_in_memory().expect("open download icon test engine");
        let task_id = engine
            .enqueue(
                DownloadRequest {
                    source_url: "https://example.test/archive.zip".into(),
                    save_directory: r"C:\Downloads".into(),
                    save_directory_explicit: false,
                    file_name: Some("archive.zip".into()),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
                    certificate_policy: Default::default(),
                    connection_limit: Some(1),
                    speed_limit_bytes_per_second: None,
                    expected_total_bytes: None,
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
                    media: None,
                    category_id: None,
                    category_name: None,
                    duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
                },
                RequestOrigin::Internal,
            )
            .expect("enqueue download icon task");
        let (platform, _platform_endpoint) = nxtools_ipc::platform_bridge(16);
        let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
        let downloads = cx.update(|cx| {
            cx.new(|cx| {
                DownloadApplication::new(
                    engine,
                    Rc::new(platform),
                    notifications.clone(),
                    DownloadRuntimeContext {
                        fallback_default_directory: r"C:\Downloads".into(),
                        config: AppConfig::default(),
                        config_path: std::env::temp_dir().join("nxtools-test-config.json"),
                        log_directory: std::env::temp_dir(),
                        database_path: std::env::temp_dir().join("downloads.sqlite3"),
                        ffmpeg_install_directory: std::env::temp_dir()
                            .join("nxtools-test-tools")
                            .join("ffmpeg"),
                    },
                    cx,
                )
            })
        });
        let icon_fixture = tempfile::tempdir().expect("download row icon fixture directory");
        let icon_path = icon_fixture.path().join("archive-zip.png");
        std::fs::write(&icon_path, crate::icons::ONE_PIXEL_PNG)
            .expect("write production row icon fixture");
        cx.update(|cx| {
            downloads.update(cx, |application, cx| {
                application.replace_task_icon_for_test(task_id, icon_path, cx);
            });
        });
        let downloads_for_window = downloads.downgrade();
        let (_main_window, window) = cx.add_window_view(move |window, cx| {
            let mut main_window = MainWindow::new(
                gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
                gpui::WeakEntity::<QuickLaunchApplication>::new_invalid(),
                downloads_for_window,
                MainWindowShellContext {
                    settings: None,
                    platform: None,
                    system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                },
                notifications,
                window,
                cx,
            );
            main_window.shell.activate(MainModule::Downloads);
            main_window
        });
        window.simulate_resize(size(px(1060.0), px(500.0)));
        window.refresh().expect("render wide download icon row");
        let row_selector: &'static str = Box::leak(
            format!("download-task-row-{}", task_id.as_uuid().as_u64_pair().0).into_boxed_str(),
        );
        let row = window
            .debug_bounds(row_selector)
            .expect("production download icon row");
        window.simulate_click(row.center(), Modifiers::default());

        let shell_icon = window
            .debug_bounds("download-task-shell-icon-selected")
            .expect("selected download row must render its resolved Shell icon");
        assert_eq!(shell_icon.size, size(px(20.0), px(20.0)));
        let created_header = window
            .debug_bounds("download-sort-header-created")
            .expect("wide task list created header");
        let created_value = window
            .debug_bounds("download-task-created-selected")
            .expect("selected task created value");
        assert_eq!(
            created_value.size.width,
            px(super::DOWNLOAD_CREATED_COLUMN_WIDTH)
        );
        assert_eq!(created_header.left(), created_value.left());
        assert_eq!(created_header.right(), created_value.right());
        assert!(
            window
                .debug_bounds("download-task-icon-fallback-selected")
                .is_none(),
            "a resolved Shell icon must replace the generic fallback"
        );
        let row_source = include_str!("main_window.rs")
            .split("fn download_task_row(")
            .nth(1)
            .and_then(|source| source.split("fn download_popup_item(").next())
            .expect("download task row source");
        let icon_source = row_source
            .split(".when_some(icon_path")
            .nth(1)
            .and_then(|source| source.split(".child(\n                    div()\n                        .when(selected").next())
            .expect("resolved Shell icon slot source");
        assert!(icon_source.contains("img(path)"));
        assert!(
            !icon_source.contains(".bg("),
            "Shell icons must stay on a transparent slot"
        );
    }

    #[gpui::test]
    fn production_download_rows_and_detail_remain_coherent_at_compact_width(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let executor = cx.executor();
        let engine = DownloadEngine::open_in_memory().expect("open in-memory download engine");
        let first_task = engine
            .enqueue(
                DownloadRequest {
                    source_url: "https://example.com/first.bin".into(),
                    save_directory: r"C:\Downloads".into(),
                    save_directory_explicit: false,
                    file_name: Some("first.bin".into()),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
                    certificate_policy: Default::default(),
                    connection_limit: Some(4),
                    speed_limit_bytes_per_second: None,
                    expected_total_bytes: None,
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
                    media: None,
                    category_id: None,
                    category_name: None,
                    duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
                },
                RequestOrigin::Internal,
            )
            .expect("enqueue first task");
        let second_task = engine
            .enqueue(
                DownloadRequest {
                    source_url: "https://example.com/second.bin".into(),
                    save_directory: r"C:\Downloads".into(),
                    save_directory_explicit: false,
                    file_name: Some("second.bin".into()),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
                    certificate_policy: Default::default(),
                    connection_limit: Some(4),
                    speed_limit_bytes_per_second: None,
                    expected_total_bytes: None,
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
                    media: None,
                    category_id: None,
                    category_name: None,
                    duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
                },
                RequestOrigin::Internal,
            )
            .expect("enqueue second task");
        let failure_message =
            "server returned a bounded but deliberately long failure message ".repeat(12);
        let mut task_snapshots = engine.tasks().expect("read task snapshots");
        task_snapshots
            .iter_mut()
            .find(|task| task.task.id() == first_task)
            .expect("first task snapshot")
            .error = Some(DownloadFault {
            code: DownloadFaultCode::Protocol,
            message: failure_message.clone(),
            retryable: false,
        });
        let (platform, _platform_endpoint) = nxtools_ipc::platform_bridge(16);
        let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
        let downloads = cx.update(|cx| {
            cx.new(|cx| {
                DownloadApplication::new(
                    engine,
                    std::rc::Rc::new(platform),
                    notifications.clone(),
                    DownloadRuntimeContext {
                        fallback_default_directory: r"C:\Downloads".into(),
                        config: AppConfig::default(),
                        config_path: std::env::temp_dir().join("nxtools-test-config.json"),
                        log_directory: std::env::temp_dir(),
                        database_path: std::env::temp_dir().join("downloads.sqlite3"),
                        ffmpeg_install_directory: std::env::temp_dir()
                            .join("nxtools-test-tools")
                            .join("ffmpeg"),
                    },
                    cx,
                )
            })
        });
        let downloads_for_window = downloads.downgrade();
        let failed_task_snapshots = task_snapshots.clone();
        downloads.update(cx, |application, _| {
            application.replace_tasks_for_test(task_snapshots);
        });
        let (main_window, window) = cx.add_window_view(move |window, cx| {
            let mut main_window = MainWindow::new(
                gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
                gpui::WeakEntity::<QuickLaunchApplication>::new_invalid(),
                downloads_for_window,
                MainWindowShellContext {
                    settings: None,
                    platform: None,
                    system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                },
                notifications,
                window,
                cx,
            );
            main_window.shell.activate(MainModule::Downloads);
            main_window
        });
        window.simulate_resize(size(px(920.0), px(480.0)));

        let authoritative_task_ids = window.read(|cx| {
            downloads
                .read(cx)
                .tasks()
                .iter()
                .map(|task| task.task.id())
                .collect::<Vec<_>>()
        });
        let filter_segments = window
            .debug_bounds("download-filter-segments")
            .expect("compact production workspace status group bounds");
        let segment_bounds = DownloadTaskFilter::ALL.map(|filter| {
            window
                .debug_bounds(filter.id())
                .unwrap_or_else(|| panic!("compact production status segment {}", filter.id()))
        });
        assert_eq!(filter_segments.size.height.as_f32(), 28.0);
        for (index, segment) in segment_bounds.iter().enumerate() {
            assert_eq!(segment.size.height, segment_bounds[0].size.height);
            assert!(
                (segment.size.width.as_f32() - segment_bounds[0].size.width.as_f32()).abs() <= 0.5
            );
            if index > 0 {
                assert_eq!(segment.left(), segment_bounds[index - 1].right());
            }
        }
        window.simulate_click(segment_bounds[2].center(), Modifiers::default());
        assert_eq!(
            window.read(|cx| main_window.read(cx).download_filter),
            DownloadTaskFilter::Waiting
        );
        window.simulate_click(segment_bounds[0].center(), Modifiers::default());
        assert_eq!(
            window.read(|cx| main_window.read(cx).download_filter),
            DownloadTaskFilter::All
        );

        let page_header = window
            .debug_bounds("download-module-header")
            .expect("download page header bounds");
        let search = window
            .debug_bounds("download-search")
            .expect("download search bounds");
        let settings_action = window
            .debug_bounds("download-settings-action")
            .expect("download settings action bounds");
        let schedule_action = window
            .debug_bounds("download-schedule-action")
            .expect("download schedule action bounds");
        let new_task_action = window
            .debug_bounds("download-new-task-action")
            .expect("download new-task action bounds");
        let sort = window
            .debug_bounds("download-sort")
            .expect("download sort action bounds");
        assert!(page_header.contains(&search.origin));
        assert!(page_header.contains(&search.bottom_right()));
        assert_eq!(search.size.height.as_f32(), 28.0);
        for action in [settings_action, schedule_action, new_task_action] {
            assert_eq!(action.size.height.as_f32(), 28.0);
            assert!(page_header.contains(&action.origin));
            assert!(page_header.contains(&action.bottom_right()));
        }
        assert_eq!(sort.size.height.as_f32(), 28.0);
        assert!(
            (sort.left() - filter_segments.right()) <= px(8.0),
            "sort must remain grouped with the status filter instead of floating at the far edge"
        );

        let first_element_id = first_task.as_uuid().as_u64_pair().0;
        let second_element_id = second_task.as_uuid().as_u64_pair().0;
        let first_row_selector: &'static str =
            Box::leak(format!("download-task-row-{first_element_id}").into_boxed_str());
        let second_row_selector: &'static str =
            Box::leak(format!("download-task-row-{second_element_id}").into_boxed_str());
        let search_input = window.read(|cx| main_window.read(cx).download_search_input.clone());
        window.update(|window, cx| {
            search_input.update(cx, |input, cx| input.set_value("first", window, cx));
            main_window.update(cx, |_, cx| cx.notify());
        });
        window.run_until_parked();
        window
            .refresh()
            .expect("render filtered production task list");
        assert_eq!(
            window.read(|cx| search_input.read(cx).value().to_string()),
            "first"
        );
        assert!(window.debug_bounds(first_row_selector).is_some());
        let clear_search = window
            .debug_bounds("download-search-clear")
            .expect("non-empty search exposes the bounded clear action");
        window.simulate_click(clear_search.center(), Modifiers::default());
        window.run_until_parked();
        window.update(|_, cx| main_window.update(cx, |_, cx| cx.notify()));
        window.refresh().expect("render cleared production search");
        assert!(window.read(|cx| search_input.read(cx).value().is_empty()));
        assert!(window.debug_bounds(second_row_selector).is_some());

        window.simulate_click(sort.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("open compact sort menu");
        window.simulate_keystrokes("down down enter");
        window.run_until_parked();
        assert_eq!(
            window.read(|cx| main_window.read(cx).download_sort),
            DownloadSortState {
                key: DownloadSortKey::Name,
                descending: false,
            },
            "the compact sort menu must feed the same local view sort as table headers"
        );
        assert_eq!(
            window.read(|cx| {
                downloads
                    .read(cx)
                    .tasks()
                    .iter()
                    .map(|task| task.task.id())
                    .collect::<Vec<_>>()
            }),
            authoritative_task_ids,
            "filter, search and sort must not mutate the authoritative task snapshot"
        );
        assert_eq!(
            window.read(|cx| main_window.read(cx).download_detail_tab),
            DownloadDetailTab::Overview
        );
        assert_eq!(
            window.read(|cx| main_window.read(cx).selected_download_task),
            None,
            "opening the download page must not invent a task selection"
        );
        assert!(
            window.debug_bounds("download-task-detail").is_none(),
            "the detail dock must stay hidden until the user selects a task"
        );
        assert!(
            window.debug_bounds("download-batch-toolbar").is_none(),
            "empty selection must not reserve a batch toolbar row"
        );

        let selected_task = first_task;
        let selected_row_selector = first_row_selector;
        let selected_row = window
            .debug_bounds(selected_row_selector)
            .expect("production task row bounds");
        assert_eq!(
            selected_row.size.height.as_f32(),
            DOWNLOAD_TASK_ROW_HEIGHT,
            "the main table must keep its compact row density"
        );
        window.update(|_, cx| {
            downloads.update(cx, |application, _| {
                application.replace_tasks_for_test(failed_task_snapshots);
            });
            main_window.update(cx, |window, _| {
                window.requested_download_detail = Some(selected_task);
            });
        });
        window.simulate_click(selected_row.center(), Modifiers::default());
        assert_eq!(
            window.read(|cx| main_window.read(cx).selected_download_task),
            Some(selected_task)
        );
        assert_eq!(
            window.read(|cx| main_window.read(cx).selected_download_tasks.clone()),
            HashSet::from([selected_task])
        );
        let detail_error = window
            .debug_bounds("download-task-detail-error")
            .expect("selected failure detail bounds");
        let detail = window
            .debug_bounds("download-task-detail")
            .expect("production detail dock bounds");

        let detail_tabs = window
            .debug_bounds("download-detail-tabs")
            .expect("production detail tab strip bounds");
        let detail_scroll = window
            .debug_bounds("download-detail-scroll")
            .expect("production detail scroll viewport bounds");
        let overview_primary = window
            .debug_bounds("download-detail-overview-primary")
            .expect("primary overview field column bounds");
        let overview_secondary = window
            .debug_bounds("download-detail-overview-secondary")
            .expect("secondary overview field column bounds");
        let status_label = window
            .debug_bounds("download-detail-field-status-label")
            .expect("fixed overview status label bounds");
        let downloaded_label = window
            .debug_bounds("download-detail-field-downloaded-label")
            .expect("fixed overview downloaded label bounds");
        let status_value = window
            .debug_bounds("download-detail-field-status-value")
            .expect("overview status value bounds");
        let downloaded_value = window
            .debug_bounds("download-detail-field-downloaded-value")
            .expect("overview downloaded value bounds");
        assert_eq!(detail_tabs.size.height.as_f32(), 30.0);
        let tab_bounds = DownloadDetailTab::ALL.map(|tab| {
            window
                .debug_bounds(Box::leak(
                    format!("download-detail-tab-{}", tab.diagnostic_name()).into_boxed_str(),
                ))
                .unwrap_or_else(|| panic!("production {:?} detail tab bounds", tab))
        });
        for tab in &tab_bounds {
            assert_eq!(tab.top(), tab_bounds[0].top());
            assert_eq!(tab.size.height.as_f32(), 29.0);
            assert!(detail_tabs.contains(&tab.origin));
            assert!(detail_tabs.contains(&tab.bottom_right()));
        }
        assert_eq!(detail_scroll.top(), detail_tabs.bottom());
        assert_eq!(detail_scroll.bottom(), detail.bottom());
        assert!(
            (overview_primary.size.width.as_f32() - overview_secondary.size.width.as_f32()).abs()
                <= 1.0,
            "overview inspector columns must split the available content width evenly"
        );
        assert_eq!(status_label.size.width.as_f32(), 52.0);
        assert_eq!(downloaded_label.size.width, status_label.size.width);
        assert_eq!(status_value.left(), downloaded_value.left());

        let segment_tab = window
            .debug_bounds("download-detail-tab-segments")
            .expect("download segment detail tab bounds after selecting a task");
        window.simulate_click(segment_tab.center(), Modifiers::default());
        assert_eq!(
            window.read(|cx| main_window.read(cx).download_detail_tab),
            DownloadDetailTab::Segments
        );
        window.refresh().expect("render detail progress state");
        let detail_progress = window
            .debug_bounds("download-detail-segments-progress")
            .expect("production detail progress state");
        let detail_progress_icon = window
            .debug_bounds("download-detail-segments-progress-icon")
            .expect("detail progress icon");
        let detail_progress_title = window
            .debug_bounds("download-detail-segments-progress-title")
            .expect("detail progress title");
        let detail_progress_description = window
            .debug_bounds("download-detail-segments-progress-description")
            .expect("detail progress description");
        assert_eq!(detail_progress.size.height, px(72.0));

        window.update(|_, cx| {
            downloads.update(cx, |application, cx| {
                application.replace_task_detail_error_for_test(
                    selected_task,
                    "detail fixture failure",
                    cx,
                );
            });
        });
        window.run_until_parked();
        window.refresh().expect("render detail error state");
        let detail_error_state = window
            .debug_bounds("download-detail-segments-error")
            .expect("production detail error state");
        let detail_error_marker = window
            .debug_bounds("download-detail-segments-error-marker")
            .expect("detail error semantic edge");
        let detail_error_icon = window
            .debug_bounds("download-detail-segments-error-icon")
            .expect("detail error icon");
        let detail_error_title = window
            .debug_bounds("download-detail-segments-error-title")
            .expect("detail error title");
        let detail_error_description = window
            .debug_bounds("download-detail-segments-error-description")
            .expect("detail error description");
        assert_eq!(detail_error_state.size, detail_progress.size);
        assert_eq!(detail_error_marker.size.width, px(2.0));
        assert_eq!(detail_error_icon.origin.x, detail_progress_icon.origin.x);
        assert_eq!(detail_error_title.origin.x, detail_progress_title.origin.x);
        assert_eq!(
            detail_error_description.origin.x,
            detail_progress_description.origin.x
        );

        let name = window
            .debug_bounds("download-task-name-selected")
            .expect("selected name column bounds");
        let name_text = window
            .debug_bounds("download-task-name-text-selected")
            .expect("selected file-name text bounds");
        let failure_text = window
            .debug_bounds("download-task-error-text")
            .expect("failed task must replace the auxiliary path line");
        assert!(
            window
                .debug_bounds("download-task-path-text-selected")
                .is_none(),
            "a failed task must not leave the ordinary path presentation active"
        );
        let progress = window
            .debug_bounds("download-task-progress-selected")
            .expect("selected progress column bounds");
        let transferred = window
            .debug_bounds("download-task-transferred-selected")
            .expect("selected downloaded/total subcolumn bounds");
        let speed = window
            .debug_bounds("download-task-speed-selected")
            .expect("selected speed subcolumn bounds");
        let remaining = window
            .debug_bounds("download-task-remaining-selected")
            .expect("selected remaining-time subcolumn bounds");
        let progress_track = window
            .debug_bounds("download-task-progress-track-selected")
            .expect("selected progress track bounds");
        let status = window
            .debug_bounds("download-task-status-selected")
            .expect("selected status column bounds");
        let transfer_header = window
            .debug_bounds("download-transfer-header")
            .expect("transfer header bounds");
        let speed_header = window
            .debug_bounds("download-speed-header")
            .expect("speed header bounds");
        let remaining_header = window
            .debug_bounds("download-remaining-header")
            .expect("remaining header bounds");
        let status_header = window
            .debug_bounds("download-sort-header-status")
            .expect("status header bounds");
        assert!(
            name.size.width.as_f32() >= 180.0,
            "the file-name column must retain enough width to render its content"
        );
        assert_eq!(name_text.size.width, name.size.width);
        assert_eq!(failure_text.size.width, name.size.width);
        assert!(failure_text.size.height.as_f32() >= 16.0);
        assert!(name.right() <= progress.left());
        assert_eq!(transferred.left(), progress.left());
        assert_eq!(transferred.right(), speed.left());
        assert_eq!(speed.right(), remaining.left());
        assert_eq!(remaining.right(), progress.right());
        assert_eq!(progress_track.left(), progress.left());
        assert_eq!(progress_track.right(), progress.right());
        assert_eq!(progress_track.size.height.as_f32(), 3.0);
        assert_eq!(transfer_header.left(), progress.left());
        assert_eq!(speed_header.left(), speed.left());
        assert_eq!(remaining_header.left(), remaining.left());
        assert_eq!(status_header.left(), status.left());
        assert!(progress.right() <= status.left());
        assert!(detail.size.width.as_f32() > 0.0);
        assert_eq!(detail.size.height.as_f32(), DOWNLOAD_DETAIL_HEIGHT);
        assert!(detail_error.size.width.as_f32() > 0.0);

        window.simulate_mouse_move(failure_text.center(), None, Modifiers::default());
        executor.advance_clock(Duration::from_millis(600));
        window.run_until_parked();
        let error_tooltip = window
            .debug_bounds("download-task-error-tooltip")
            .expect("bounded failure tooltip after hovering the auxiliary line");
        assert_eq!(
            error_tooltip.size.width.as_f32(),
            super::DOWNLOAD_ERROR_TOOLTIP_WIDTH
        );
        assert!(error_tooltip.size.height.as_f32() <= super::DOWNLOAD_ERROR_TOOLTIP_MAX_HEIGHT);
    }

    #[gpui::test]
    fn production_download_batch_toolbar_appears_for_multi_select_and_dispatches_one_batch(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let engine = DownloadEngine::open_in_memory().expect("open in-memory download engine");
        let request = |name: &str| DownloadRequest {
            source_url: format!("https://example.com/{name}"),
            save_directory: r"C:\Downloads".into(),
            save_directory_explicit: false,
            file_name: Some(name.into()),
            headers: Vec::new(),
            cookie: None,
            referrer: None,
            certificate_policy: Default::default(),
            connection_limit: Some(2),
            speed_limit_bytes_per_second: None,
            expected_total_bytes: None,
            scheduled_at_unix_ms: None,
            adaptive_segment_policy: None,
            media: None,
            category_id: None,
            category_name: None,
            duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
        };
        let first = engine
            .enqueue(request("batch-first.bin"), RequestOrigin::Internal)
            .expect("enqueue first batch task");
        let second = engine
            .enqueue(request("batch-second.bin"), RequestOrigin::Internal)
            .expect("enqueue second batch task");
        let mut snapshots = engine.tasks().expect("read batch task snapshots");
        let failed_snapshot = snapshots
            .iter_mut()
            .find(|task| task.task.id() == first)
            .expect("first mixed-state task snapshot");
        failed_snapshot
            .task
            .transition_to(DownloadStatus::Probing, 1)
            .expect("advance first task to probing");
        failed_snapshot
            .task
            .transition_to(DownloadStatus::Failed, 2)
            .expect("advance first task to failed");
        let downloading_snapshot = snapshots
            .iter_mut()
            .find(|task| task.task.id() == second)
            .expect("second mixed-state task snapshot");
        for (status, revision) in [
            (DownloadStatus::Probing, 1),
            (DownloadStatus::Waiting, 2),
            (DownloadStatus::Downloading, 3),
        ] {
            downloading_snapshot
                .task
                .transition_to(status, revision)
                .expect("advance second task to downloading");
        }
        let mixed_snapshots = snapshots.clone();
        let fixture_id = uuid::Uuid::new_v4();
        let (platform, _platform_endpoint) = nxtools_ipc::platform_bridge(16);
        let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
        let downloads = cx.update(|cx| {
            cx.new(|cx| {
                DownloadApplication::new(
                    engine,
                    Rc::new(platform),
                    notifications.clone(),
                    DownloadRuntimeContext {
                        fallback_default_directory: r"C:\Downloads".into(),
                        config: AppConfig::default(),
                        config_path: std::env::temp_dir()
                            .join(format!("nxtools-batch-toolbar-config-{fixture_id}.json")),
                        log_directory: std::env::temp_dir(),
                        database_path: std::env::temp_dir()
                            .join(format!("nxtools-batch-toolbar-{fixture_id}.sqlite3")),
                        ffmpeg_install_directory: std::env::temp_dir()
                            .join("nxtools-test-tools")
                            .join("ffmpeg"),
                    },
                    cx,
                )
            })
        });
        downloads.update(cx, |application, _| {
            application.replace_tasks_for_test(snapshots);
        });
        let downloads_for_window = downloads.downgrade();
        let (main_window, window) = cx.add_window_view(move |window, cx| {
            let mut main_window = MainWindow::new(
                gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
                gpui::WeakEntity::<QuickLaunchApplication>::new_invalid(),
                downloads_for_window,
                MainWindowShellContext {
                    settings: None,
                    platform: None,
                    system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                },
                notifications,
                window,
                cx,
            );
            main_window.shell.activate(MainModule::Downloads);
            main_window
        });
        window.simulate_resize(size(px(720.0), px(500.0)));
        window.update(|_, cx| {
            downloads.update(cx, |application, _| {
                application.replace_tasks_for_test(mixed_snapshots);
            });
            main_window.update(cx, |_, cx| cx.notify());
        });
        window.refresh().expect("render mixed-state task snapshots");

        assert!(window.debug_bounds("download-batch-toolbar").is_none());
        let first_selector: &'static str = Box::leak(
            format!("download-task-row-{}", first.as_uuid().as_u64_pair().0).into_boxed_str(),
        );
        let second_selector: &'static str = Box::leak(
            format!("download-task-row-{}", second.as_uuid().as_u64_pair().0).into_boxed_str(),
        );
        let first_row = window
            .debug_bounds(first_selector)
            .expect("first production task row");
        window.simulate_click(first_row.center(), Modifiers::default());
        assert!(window.debug_bounds("download-batch-toolbar").is_none());
        let second_row = window
            .debug_bounds(second_selector)
            .expect("second production task row");
        window.simulate_click(
            second_row.center(),
            Modifiers {
                control: true,
                ..Modifiers::default()
            },
        );
        window.run_until_parked();
        window.refresh().expect("render production batch toolbar");

        assert_eq!(
            window.read(|cx| main_window.read(cx).selected_download_tasks.clone()),
            HashSet::from([first, second])
        );
        let toolbar = window
            .debug_bounds("download-batch-toolbar")
            .expect("real multi-selection toolbar bounds");
        let cancel = window
            .debug_bounds("download-batch-cancel")
            .expect("mixed selection cancel action for the active subset");
        assert!(window.debug_bounds("download-batch-resume").is_some());
        assert!(window.debug_bounds("download-batch-pause").is_some());
        for unavailable in [
            "download-batch-queue-top",
            "download-batch-queue-up",
            "download-batch-queue-down",
            "download-batch-queue-bottom",
            "download-batch-remove",
        ] {
            assert!(
                window.debug_bounds(unavailable).is_none(),
                "mixed-state selection must omit unavailable toolbar action {unavailable}"
            );
        }
        assert_eq!(toolbar.size.height.as_f32(), DOWNLOAD_BATCH_TOOLBAR_HEIGHT);
        assert_eq!(cancel.size.height.as_f32(), 28.0);
        assert!(toolbar.contains(&cancel.origin));
        assert!(toolbar.contains(&cancel.bottom_right()));
        let header_with_toolbar = window
            .debug_bounds("download-table-header")
            .expect("table header below batch toolbar");

        window.simulate_click(cancel.center(), Modifiers::default());
        window.run_until_parked();
        assert!(window.read(|cx| {
            let tasks = downloads.read(cx).tasks();
            tasks
                .iter()
                .find(|task| task.task.id() == first)
                .is_some_and(|task| task.task.status() != DownloadStatus::Cancelled)
                && tasks
                    .iter()
                    .find(|task| task.task.id() == second)
                    .is_some_and(|task| task.task.status() == DownloadStatus::Cancelled)
        }));
        window.update(|_, cx| main_window.update(cx, |_, cx| cx.notify()));
        window.refresh().expect("refresh terminal batch actions");
        let clear = window
            .debug_bounds("download-batch-clear")
            .expect("clear-selection action after batch completion");
        window.simulate_click(clear.center(), Modifiers::default());
        window.run_until_parked();
        window.update(|_, cx| main_window.update(cx, |_, cx| cx.notify()));
        window.refresh().expect("reclaim batch toolbar row");
        assert!(window.read(|cx| main_window.read(cx).selected_download_tasks.is_empty()));
        assert_eq!(
            window.read(|cx| main_window.read(cx).selected_download_task),
            None
        );
        let header_without_toolbar = window
            .debug_bounds("download-table-header")
            .expect("table header after clearing multi-selection");
        assert_eq!(
            header_without_toolbar.top() + px(DOWNLOAD_BATCH_TOOLBAR_HEIGHT),
            header_with_toolbar.top(),
            "single/empty selection must reclaim the complete batch toolbar height"
        );
    }

    #[gpui::test]
    fn double_clicking_a_download_row_reuses_one_deferred_task_window(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let engine = DownloadEngine::open_in_memory().expect("open in-memory download engine");
        let task_id = engine
            .enqueue(
                DownloadRequest {
                    source_url: "https://example.com/reopen.bin".into(),
                    save_directory: r"C:\Downloads".into(),
                    save_directory_explicit: false,
                    file_name: Some("reopen.bin".into()),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
                    certificate_policy: Default::default(),
                    connection_limit: Some(1),
                    speed_limit_bytes_per_second: None,
                    expected_total_bytes: None,
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
                    media: None,
                    category_id: None,
                    category_name: None,
                    duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
                },
                RequestOrigin::Internal,
            )
            .expect("enqueue task");
        let (platform, _platform_endpoint) = nxtools_ipc::platform_bridge(16);
        let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
        let downloads = cx.update(|cx| {
            cx.new(|cx| {
                DownloadApplication::new(
                    engine,
                    std::rc::Rc::new(platform),
                    notifications.clone(),
                    DownloadRuntimeContext {
                        fallback_default_directory: r"C:\Downloads".into(),
                        config: AppConfig::default(),
                        config_path: std::env::temp_dir().join("nxtools-test-config.json"),
                        log_directory: std::env::temp_dir(),
                        database_path: std::env::temp_dir().join("downloads.sqlite3"),
                        ffmpeg_install_directory: std::env::temp_dir()
                            .join("nxtools-test-tools")
                            .join("ffmpeg"),
                    },
                    cx,
                )
            })
        });
        let activation_commands = cx.update(|cx| {
            downloads.update(cx, |application, _| {
                application.configure_task_window_activation_test(42)
            })
        });
        let downloads_for_window = downloads.downgrade();
        let (_main_window, window) = cx.add_window_view(move |window, cx| {
            let mut main_window = MainWindow::new(
                gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
                gpui::WeakEntity::<QuickLaunchApplication>::new_invalid(),
                downloads_for_window,
                MainWindowShellContext {
                    settings: None,
                    platform: None,
                    system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                },
                notifications,
                window,
                cx,
            );
            main_window.shell.activate(MainModule::Downloads);
            main_window
        });
        let element_id = task_id.as_uuid().as_u64_pair().0;
        let row_selector: &'static str =
            Box::leak(format!("download-task-row-{element_id}").into_boxed_str());
        let row = window
            .debug_bounds(row_selector)
            .expect("production task row bounds");
        let double_click = |window: &mut gpui::VisualTestContext| {
            window.simulate_event(MouseDownEvent {
                button: MouseButton::Left,
                position: row.center(),
                modifiers: Modifiers::default(),
                click_count: 2,
                first_mouse: false,
            });
            window.simulate_event(MouseUpEvent {
                button: MouseButton::Left,
                position: row.center(),
                modifiers: Modifiers::default(),
                click_count: 2,
            });
            window.run_until_parked();
        };

        let initial_windows = window.windows();
        assert_eq!(initial_windows.len(), 1);
        double_click(window);
        let first_reopen_windows = window.windows();
        assert_eq!(first_reopen_windows.len(), 2);
        let task_window = *first_reopen_windows
            .iter()
            .find(|candidate| !initial_windows.contains(candidate))
            .expect("new keyed task window");
        double_click(window);
        let second_reopen_windows = window.windows();
        assert_eq!(
            second_reopen_windows.len(),
            2,
            "the second reopen must focus the keyed window instead of creating a duplicate"
        );
        assert!(
            second_reopen_windows.contains(&task_window),
            "the second reopen must retain the same keyed GPUI window"
        );
        assert_eq!(
            activation_commands.borrow().as_slice(),
            &[PlatformCommand::ActivateWindow(ActivateWindowCommand {
                window_handle: 42,
            })],
            "one in-flight native activation is reused instead of queueing duplicates"
        );
    }

    #[test]
    fn download_task_window_entrypoints_defer_cross_entity_work() {
        let source = include_str!("main_window.rs");
        let row = source
            .split("\nfn download_task_row(")
            .nth(1)
            .and_then(|body| body.split("\nfn download_task_context_menu(").next())
            .expect("download row renderer");
        let menu = source
            .split("\nfn download_task_context_menu(")
            .nth(1)
            .and_then(|body| body.split("\nfn download_task_detail(").next())
            .expect("download task context menu");

        for (name, body) in [("double click", row), ("context menu", menu)] {
            let request = body
                .find("application.request_task_window(task_id, cx)")
                .unwrap_or_else(|| panic!("{name} task-window request"));
            let deferred = body[..request]
                .rfind("cx.defer(move |cx|")
                .unwrap_or_else(|| panic!("{name} must defer its cross-entity update"));
            assert!(deferred < request);
        }
    }

    #[test]
    fn download_filter_segments_are_wired_to_the_gpui_main_window() {
        let source = include_str!("main_window.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("production main-window source");
        let downloads = source
            .split("fn render_downloads")
            .nth(1)
            .and_then(|source| source.split("fn render_quick_find").next())
            .expect("download main-window source");

        assert!(production.contains("download-filter-waiting"));
        assert!(production.contains("download-filter-paused"));
        assert!(downloads.contains("segmented_control("));
        assert!(downloads.contains("SegmentedOption::new("));
        assert!(downloads.contains("cx.listener(move |this"));
        assert!(downloads.contains("select_download_filter("));
        assert!(downloads.contains("visible_tasks"));
    }

    #[test]
    fn volume_toggles_materialize_all_then_collapse_back_to_automatic_all() {
        let available = vec![r"C:\".to_owned(), r"D:\".to_owned()];

        let without_c = toggle_indexed_volume_selection(&[], &available, r"C:\");
        assert_eq!(without_c, vec![r"D:\".to_owned()]);

        let all_again = toggle_indexed_volume_selection(&without_c, &available, r"C:\");
        assert!(
            all_again.is_empty(),
            "all selected is stored as automatic all"
        );

        let last_selected = vec![r"D:\".to_owned()];
        assert_eq!(
            toggle_indexed_volume_selection(&last_selected, &available, r"D:\"),
            last_selected,
            "the empty vector is reserved for automatic all, not an explicit empty scope"
        );

        let stale_and_current = vec![r"Z:\".to_owned(), r"C:\".to_owned()];
        assert!(
            toggle_indexed_volume_selection(&stale_and_current, &available, r"D:\").is_empty(),
            "all currently discovered volumes canonicalize to automatic all even with a stale mount"
        );
    }

    #[test]
    fn index_scope_settings_use_a_checkbox_list_and_a_directory_picker_not_text_inputs() {
        let source = include_str!("main_window.rs");
        let path_list = include_str!("ui/components/path_list.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("production main-window source");

        assert!(!production.contains("index_volumes_input"));
        assert!(!production.contains("index_exclusions_input"));
        assert!(production.contains("volume_scope_setting("));
        assert!(production.contains("automatic_volume_scope_row("));
        assert!(production.contains("volume_scope_row("));
        assert!(!production.contains("quick-launch-index-all-volumes"));
        assert!(production.contains("path_exclusion_setting("));
        assert!(path_list.contains("quick-launch-add-index-exclusion"));
        assert!(production.contains("request_index_exclusion_directory"));
    }

    #[test]
    fn quick_launch_settings_scrolls_only_the_selected_page_viewport() {
        let source = include_str!("main_window.rs");
        let quick_find = source
            .split("fn render_quick_find")
            .nth(1)
            .and_then(|source| source.split("fn render_clipboard").next())
            .expect("quick-launch settings source");

        assert!(quick_find.contains("quick-launch-settings-scroll"));
        assert!(quick_find.contains("quick-launch-settings-navigation"));
        assert!(quick_find.contains("QuickLaunchSettingsPage::ALL"));
        for selector in [
            "quick-launch-settings-page-general",
            "quick-launch-settings-page-index",
            "quick-launch-settings-page-context",
            "quick-launch-settings-page-commands",
            "quick-launch-settings-page-diagnostics",
        ] {
            assert!(source.contains(selector), "missing {selector}");
        }
        assert!(quick_find.contains(".overflow_y_scroll()"));
        assert!(quick_find.contains(".on_scroll_wheel(cx.listener("));
        assert!(quick_find.contains("Scrollbar::vertical(&self.quick_find_scroll)"));
        assert!(!quick_find.contains(".vertical_scrollbar(&self.quick_find_scroll)"));
    }

    #[test]
    fn quick_launch_wheel_scroll_accumulates_and_eases_to_a_clamped_target() {
        let first_target = quick_find_scroll_target(0.0, 0.0, 300.0, -38.0);
        let second_target = quick_find_scroll_target(-15.0, first_target, 300.0, -38.0);
        assert_eq!(first_target, -38.0);
        assert_eq!(second_target, -76.0);
        assert_eq!(
            quick_find_scroll_target(-290.0, -290.0, 300.0, -38.0),
            -300.0
        );

        let (next, done) = quick_find_scroll_step(-15.0, second_target);
        assert!(!done);
        assert!(next < -15.0 && next > second_target);
        assert_eq!(quick_find_scroll_step(-75.8, second_target), (-76.0, true));
    }

    #[test]
    fn custom_command_program_uses_the_windows_picker() {
        let source = include_str!("main_window.rs");
        let editor = include_str!("ui/components/custom_command_editor.rs");
        let quick_find = source
            .split("fn render_quick_find")
            .nth(1)
            .and_then(|source| source.split("fn render_clipboard").next())
            .expect("quick-launch settings source");

        assert!(editor.contains("quick-launch-pick-command-program"));
        assert!(quick_find.contains("request_custom_command_program"));
    }

    #[test]
    fn quick_launch_settings_expose_history_performance_and_diagnostics() {
        let source = include_str!("main_window.rs");
        let quick_find = source
            .split("fn render_quick_find")
            .nth(1)
            .and_then(|source| source.split("fn render_clipboard").next())
            .expect("quick-launch settings source");

        assert!(quick_find.contains("quick-launch-clear-history"));
        assert!(quick_find.contains("clear_history"));
        assert!(quick_find.contains("quick-launch-open-logs"));
        assert!(quick_find.contains("open_log_directory"));
        assert!(quick_find.contains("最近查询"));
        assert!(quick_find.contains("历史与诊断"));
    }

    #[test]
    fn file_index_summary_surfaces_privilege_failure_instead_of_claiming_only_volume_discovery() {
        let status = IndexStatus {
            phase: nxtools_protocol::IndexPhase::Ready,
            indexed_items: 0,
            drives: vec![nxtools_protocol::IndexDriveStatus {
                mount_point: r"C:\".into(),
                file_system: "NTFS".into(),
                indexed_items: 0,
                journal_id: None,
                next_usn: None,
            }],
            last_error: Some("Windows denied raw volume access".into()),
        };

        let (summary, badge) = file_index_summary(Some(&status));

        assert!(summary.contains("Windows denied raw volume access"));
        assert_eq!(badge, "ERROR");
    }

    #[test]
    fn query_elapsed_summary_keeps_microseconds_readable_and_promotes_milliseconds() {
        assert_eq!(format_query_elapsed(None), "尚无样本");
        assert_eq!(format_query_elapsed(Some(240)), "240 µs");
        assert_eq!(format_query_elapsed(Some(12_345)), "12.35 ms");
    }

    #[test]
    fn file_index_summary_keeps_a_last_good_snapshot_available_during_maintenance() {
        let status = IndexStatus {
            phase: IndexPhase::VolumesReady,
            indexed_items: 2_784_460,
            drives: Vec::new(),
            last_error: None,
        };

        let (summary, badge) = file_index_summary(Some(&status));

        assert_eq!(badge, "UPDATING");
        assert!(summary.contains("2,784,460"));
        assert!(summary.contains("仍可搜索"));
    }

    #[test]
    fn notification_motion_has_distinct_enter_and_exit_endpoints() {
        assert_eq!(notification_transition(0.0, false), (0.0, -6.0));
        assert_eq!(notification_transition(1.0, false), (1.0, 0.0));
        assert_eq!(notification_transition(0.0, true), (1.0, 0.0));
        assert_eq!(notification_transition(1.0, true), (0.0, -4.0));
    }

    #[test]
    fn hotkey_recording_requires_a_modifier_and_maps_windows_virtual_keys() {
        let recorded = hotkey_from_key_down(&KeyDownEvent {
            keystroke: Keystroke {
                modifiers: Modifiers {
                    control: true,
                    shift: true,
                    ..Modifiers::default()
                },
                key: "v".into(),
                key_char: Some("V".into()),
            },
            is_held: false,
        })
        .expect("Ctrl+Shift+V must be recordable");
        assert_eq!(
            recorded.modifiers,
            vec![
                ClipboardHotKeyModifier::Control,
                ClipboardHotKeyModifier::Shift
            ]
        );
        assert_eq!(recorded.key, "V");
        assert_eq!(recorded.display, "Ctrl+Shift+V");

        assert!(
            hotkey_from_key_down(&KeyDownEvent {
                keystroke: Keystroke {
                    modifiers: Modifiers::default(),
                    key: "c".into(),
                    key_char: Some("c".into()),
                },
                is_held: false,
            })
            .is_none()
        );
    }
}
