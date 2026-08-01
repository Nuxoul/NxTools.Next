use std::collections::HashSet;
use std::hash::Hash;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Local};
use gpui::{
    Animation, AnimationExt, AnyElement, App, Context, Corner, Entity, FocusHandle, IntoElement,
    KeyDownEvent, MouseButton, Render, ScrollHandle, ScrollStrategy, ScrollWheelEvent,
    Subscription, UniformListScrollHandle, Window, WindowControlArea, div, ease_in_out, point,
    prelude::*, px, uniform_list,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::menu::{ContextMenuExt, DropdownMenu, PopupMenu, PopupMenuItem};
use gpui_component::scroll::{ScrollableElement, Scrollbar};
use gpui_component::{Disableable, Icon, PixelsExt, Sizable};
use nxtools_clipboard::ClipboardHistoryCounts;
use nxtools_core::{
    ClipboardClickAction, ClipboardClickModifier, ClipboardHotKey, ClipboardHotKeyModifier,
    ClipboardSettings, CustomCommandSettings,
};
use nxtools_download::{
    DownloadBatchAction, DownloadEngineTask, DownloadProgress, DownloadQueuePlacement,
    DownloadSegment, DownloadSegmentState, DownloadStatus, DownloadTaskDetail, DownloadTaskId,
    DownloadTaskTimelineEntry, DownloadTaskTimelineEvent, RequestOrigin,
};
use nxtools_protocol::{IndexPhase, IndexStatus, ShellOpenAction};
use tracing::info;

use crate::clipboard::ClipboardApplication;
use crate::downloads::DownloadApplication;
use crate::icons::MaterialIcon;
use crate::notifications::{NOTIFICATION_EXIT_DURATION, NotificationCenter, NotificationLevel};
use crate::quick_launch::{
    QuickLaunchApplication, QuickLaunchApplicationEvent, QuickLaunchPerformanceSnapshot,
};
use crate::shell::{MainModule, ShellState};
use crate::theme::Theme;

const NOTIFICATION_ENTER_DURATION: Duration = Duration::from_millis(150);
const QUICK_FIND_SCROLL_FRAME_DURATION: Duration = Duration::from_millis(12);
const QUICK_FIND_SCROLL_LINE_HEIGHT: f32 = 38.0;
const QUICK_FIND_SCROLL_EASING: f32 = 0.42;
const QUICK_FIND_SCROLL_TARGET_SNAP_DISTANCE: f32 = 128.0;
const DOWNLOAD_PAGE_HEADER_HEIGHT: f32 = 48.0;
const DOWNLOAD_FILTER_STRIP_HEIGHT: f32 = 38.0;
const DOWNLOAD_TABLE_HEADER_HEIGHT: f32 = 28.0;
const DOWNLOAD_TASK_ROW_HEIGHT: f32 = 46.0;
const DOWNLOAD_DETAIL_HEIGHT: f32 = 176.0;
const DOWNLOAD_SEARCH_DIAGNOSTIC_DELAY: Duration = Duration::from_millis(250);
const DOWNLOAD_PROGRESS_COLUMN_WIDTH: f32 = 154.0;
const DOWNLOAD_STATUS_COLUMN_WIDTH: f32 = 68.0;

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

pub(crate) struct MainWindow {
    theme: Theme,
    shell: ShellState,
    clipboard: gpui::WeakEntity<ClipboardApplication>,
    quick_launch: gpui::WeakEntity<QuickLaunchApplication>,
    downloads: gpui::WeakEntity<DownloadApplication>,
    notifications: Entity<NotificationCenter>,
    download_search_input: gpui::Entity<InputState>,
    exclusions_input: gpui::Entity<InputState>,
    custom_command_keyword: gpui::Entity<InputState>,
    custom_command_title: gpui::Entity<InputState>,
    custom_command_program: gpui::Entity<InputState>,
    custom_command_arguments: gpui::Entity<InputState>,
    custom_command_working_directory: gpui::Entity<InputState>,
    quick_find_scroll: ScrollHandle,
    quick_find_scroll_target: f32,
    quick_find_scroll_animation: u64,
    selected_custom_command: Option<usize>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HotkeyRecordingTarget {
    Clipboard,
    QuickLaunch,
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
            Self::All => "全部任务",
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
        })
        .or_else(|| visible.first().map(id));
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
    *primary = Some(clicked);
    if shift {
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
        if !selected.remove(&clicked) {
            selected.insert(clicked);
        }
        *anchor = Some(clicked);
    } else {
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
        selected.remove(index);
    } else {
        selected.push(mount_point.to_owned());
    }
    let exactly_all_available = selected.len() == available.len()
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
                    let QuickLaunchApplicationEvent::CustomCommandProgramPicked { index, path } =
                        event;
                    if this.selected_custom_command == Some(*index) {
                        this.custom_command_program.update(cx, |input, cx| {
                            input.set_value(path.clone(), window, cx);
                        });
                        cx.notify();
                    }
                },
            )
        });
        let notification_subscription = cx.observe(&notifications, |_, _, cx| cx.notify());
        let hotkey_focus = cx.focus_handle();
        let hotkey_focus_subscription =
            cx.on_focus_out(&hotkey_focus, window, |this, _event, _window, cx| {
                if this.recording_hotkey.take().is_some() {
                    this.set_all_hotkey_recording(false, cx);
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
            notifications,
            download_search_input,
            exclusions_input,
            custom_command_keyword,
            custom_command_title,
            custom_command_program,
            custom_command_arguments,
            custom_command_working_directory,
            quick_find_scroll: ScrollHandle::new(),
            quick_find_scroll_target: 0.0,
            quick_find_scroll_animation: 0,
            selected_custom_command: initial_custom_command.as_ref().map(|_| 0),
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

    fn select_custom_command(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let command = self.quick_launch.upgrade().and_then(|application| {
            application
                .read(cx)
                .settings()
                .custom_commands
                .get(index)
                .cloned()
        });
        let Some(command) = command else {
            return;
        };
        self.selected_custom_command = Some(index);
        self.custom_command_run_as_admin = command.run_as_admin;
        self.custom_command_run_silently = command.run_silently;
        for (input, value) in [
            (&self.custom_command_keyword, command.keyword),
            (&self.custom_command_title, command.title),
            (&self.custom_command_program, command.program),
            (&self.custom_command_arguments, command.argument_template),
            (
                &self.custom_command_working_directory,
                command.working_directory,
            ),
        ] {
            input.update(cx, |state, cx| state.set_value(value, window, cx));
        }
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
        let _ = self.quick_launch.update(cx, |application, cx| {
            application.update_custom_command(index, command, cx);
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
        true
    }

    fn render_notifications(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let notifications = self.notifications.read(cx).items().to_vec();
        let mut stack = div()
            .absolute()
            .top(px(44.0))
            .right(px(10.0))
            .w(px(368.0))
            .flex()
            .flex_col()
            .gap(px(6.0));
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
            let mut body = div()
                .min_w_0()
                .flex_1()
                .pr(px(18.0))
                .flex()
                .flex_col()
                .gap(px(3.0))
                .child(
                    div()
                        .text_size(px(11.5))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.text)
                        .child(notification.title),
                )
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(theme.text_secondary)
                        .child(notification.message),
                );
            for detail in notification.details {
                body = body.child(
                    div()
                        .mt(px(4.0))
                        .pt(px(6.0))
                        .border_t_1()
                        .border_color(theme.border_subtle)
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .child(
                            div()
                                .text_size(px(9.0))
                                .text_color(theme.text_muted)
                                .child(detail.label),
                        )
                        .child(
                            div()
                                .w_full()
                                .whitespace_normal()
                                .line_clamp(2)
                                .font_family("JetBrains Mono, Cascadia Mono, Consolas")
                                .text_size(px(9.5))
                                .text_color(theme.text_secondary)
                                .child(detail.value),
                        ),
                );
            }
            if let Some(action) = notification.action {
                let action_id = notification_id.clone();
                let action_dismiss_id = notification_id.clone();
                let action_center = self.notifications.clone();
                let action_label = action.label.clone();
                body = body.child(
                    div().mt(px(3.0)).flex().justify_end().child(
                        div()
                            .id(gpui::SharedString::from(format!(
                                "notification-action-{}",
                                action_id
                            )))
                            .h(px(23.0))
                            .px(px(7.0))
                            .rounded(px(3.0))
                            .border_1()
                            .border_color(theme.border)
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .hover(move |style| style.bg(theme.surface_hover))
                            .child(
                                Icon::new(MaterialIcon::OpenInNew)
                                    .size(px(12.0))
                                    .text_color(theme.text_muted),
                            )
                            .child(
                                div()
                                    .text_size(px(9.5))
                                    .text_color(theme.text_secondary)
                                    .child(action_label),
                            )
                            .on_click(move |_event, window, cx| {
                                action_center.update(cx, |notifications, cx| {
                                    notifications.dismiss(action_dismiss_id.as_ref(), cx);
                                });
                                action.invoke(window, cx);
                                cx.stop_propagation();
                            }),
                    ),
                );
            }
            let card = div()
                .id(gpui::SharedString::from(format!(
                    "notification-{}",
                    notification_id
                )))
                .w_full()
                .relative()
                .min_h(px(58.0))
                .rounded(px(5.0))
                .border_1()
                .border_color(theme.border)
                .bg(theme.surface_raised)
                .shadow_md()
                .px(px(11.0))
                .py(px(9.0))
                .flex()
                .items_start()
                .gap(px(7.0))
                .occlude()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(|_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .size(px(18.0))
                        .mt(px(1.0))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(Icon::new(status_icon).size(px(14.0)).text_color(accent)),
                )
                .child(body)
                .child(
                    div()
                        .id(gpui::SharedString::from(format!(
                            "dismiss-notification-{}",
                            notification_id
                        )))
                        .absolute()
                        .top(px(6.0))
                        .right(px(6.0))
                        .size(px(24.0))
                        .rounded(px(3.0))
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .justify_center()
                        .hover(move |style| style.bg(theme.surface_hover))
                        .child(
                            Icon::new(MaterialIcon::Close)
                                .size(px(13.0))
                                .text_color(theme.text_muted),
                        )
                        .on_click(move |_event, _window, cx| {
                            notification_center.update(cx, |notifications, cx| {
                                notifications.dismiss(dismiss_id.as_ref(), cx);
                            });
                        }),
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
                    .font_family("JetBrains Mono, Cascadia Mono, Consolas")
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

    fn set_hotkey_recording(
        &self,
        target: HotkeyRecordingTarget,
        recording: bool,
        cx: &mut Context<Self>,
    ) {
        match target {
            HotkeyRecordingTarget::Clipboard => {
                let _ = self.clipboard.update(cx, |application, _| {
                    application.set_clipboard_hotkey_recording(recording);
                });
            }
            HotkeyRecordingTarget::QuickLaunch => {
                let _ = self.quick_launch.update(cx, |application, _| {
                    application.set_hotkey_recording(recording);
                });
            }
        }
    }

    fn set_all_hotkey_recording(&self, recording: bool, cx: &mut Context<Self>) {
        self.set_hotkey_recording(HotkeyRecordingTarget::Clipboard, recording, cx);
        self.set_hotkey_recording(HotkeyRecordingTarget::QuickLaunch, recording, cx);
    }

    fn handle_hotkey_recording(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let Some(target) = self.recording_hotkey else {
            return;
        };
        if event.keystroke.key.eq_ignore_ascii_case("escape") {
            self.recording_hotkey = None;
            self.set_all_hotkey_recording(false, cx);
            cx.stop_propagation();
            cx.notify();
            return;
        }
        let clipboard_ready = self
            .clipboard
            .upgrade()
            .is_some_and(|application| application.read(cx).clipboard_hotkey_recording_ready());
        let quick_launch_ready = self
            .quick_launch
            .upgrade()
            .is_some_and(|application| application.read(cx).hotkey_recording_ready());
        let recording_ready = clipboard_ready && quick_launch_ready;
        if !recording_ready {
            cx.stop_propagation();
            return;
        }
        let Some(hotkey) = hotkey_from_key_down(event) else {
            return;
        };
        match target {
            HotkeyRecordingTarget::Clipboard => {
                let _ = self.clipboard.update(cx, |application, _| {
                    application.set_clipboard_hotkey(hotkey);
                });
            }
            HotkeyRecordingTarget::QuickLaunch => {
                let _ = self.quick_launch.update(cx, |application, _| {
                    application.set_hotkey(hotkey);
                });
            }
        }
        let other = match target {
            HotkeyRecordingTarget::Clipboard => HotkeyRecordingTarget::QuickLaunch,
            HotkeyRecordingTarget::QuickLaunch => HotkeyRecordingTarget::Clipboard,
        };
        self.set_hotkey_recording(other, false, cx);
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
        div()
            .h(px(36.0))
            .w_full()
            .flex()
            .items_center()
            .border_b_1()
            .border_color(theme.border_subtle)
            .bg(theme.canvas)
            .child(
                div()
                    .h_full()
                    .flex_1()
                    .window_control_area(WindowControlArea::Drag)
                    .flex()
                    .items_center()
                    .gap(px(9.0))
                    .pl(px(12.0))
                    .child(
                        div()
                            .size(px(18.0))
                            .rounded(px(4.0))
                            .bg(theme.surface_raised)
                            .border_1()
                            .border_color(theme.border)
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(9.0))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(theme.accent)
                            .child("NX"),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme.text_secondary)
                            .child("NxTools"),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.text_muted)
                            .child(format!("· {}", self.shell.active_module().label())),
                    ),
            )
            .child(window_control(
                "\u{e921}",
                WindowControlArea::Min,
                false,
                theme,
            ))
            .child(window_control(
                if window.is_maximized() {
                    "\u{e923}"
                } else {
                    "\u{e922}"
                },
                WindowControlArea::Max,
                false,
                theme,
            ))
            .child(window_control(
                "\u{e8bb}",
                WindowControlArea::Close,
                true,
                theme,
            ))
            .into_any_element()
    }

    fn render_sidebar(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let active = self.shell.active_module();
        let mut modules = div().flex().flex_col().gap(px(4.0));
        for module in MainModule::ALL {
            let selected = module == active;
            modules = modules.child(
                div()
                    .id(gpui::SharedString::from(format!(
                        "main-module-{}",
                        module.label()
                    )))
                    .h(px(42.0))
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
                                    .text_size(px(13.0))
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
                                    .text_size(px(10.0))
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
            .w(px(208.0))
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
        let (total_count, counts, visible_tasks) = self
            .downloads
            .upgrade()
            .map(|application| {
                let tasks = application.read(cx).tasks();
                (
                    tasks.len(),
                    download_filter_counts(tasks.iter().map(|task| task.task.status())),
                    download_tasks_for_view(
                        tasks,
                        self.download_filter,
                        &search_query,
                        self.download_sort,
                    ),
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
        self.download_selection_anchor = self
            .download_selection_anchor
            .filter(|task_id| self.selected_download_tasks.contains(task_id))
            .or(selected_task_id);
        let selected_task = selected_task_id
            .and_then(|task_id| visible_tasks.iter().find(|task| task.task.id() == task_id))
            .cloned();
        let (detail_snapshot, detail_error, selected_progress) = selected_task_id
            .and_then(|task_id| {
                self.downloads.upgrade().map(|application| {
                    let application = application.read(cx);
                    (
                        application.task_detail(task_id).cloned(),
                        application.task_detail_error(task_id).map(str::to_owned),
                        application.progress(task_id).cloned(),
                    )
                })
            })
            .unwrap_or_default();
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
        let main_window = cx.entity();
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
        let virtual_main_window = main_window.clone();
        let task_list = uniform_list(
            "download-task-list",
            visible_count,
            cx.processor(move |this, range: std::ops::Range<usize>, _window, _cx| {
                range
                    .filter_map(|index| virtual_tasks.get(index).cloned())
                    .map(|task| {
                        download_task_row(
                            theme,
                            this.downloads.clone(),
                            virtual_main_window.clone(),
                            DownloadRowSelection {
                                selected: this.selected_download_tasks.contains(&task.task.id()),
                                visible_task_ids: virtual_task_ids.clone(),
                            },
                            task,
                        )
                    })
                    .collect::<Vec<_>>()
            }),
        )
        .size_full()
        .track_scroll(self.download_list_scroll.clone());
        let mut filter_rows = div().flex().items_center().gap(px(3.0));
        for filter in DownloadTaskFilter::ALL {
            let active = filter == self.download_filter;
            let filter_count = counts.get(filter);
            filter_rows = filter_rows.child(
                download_filter_chip(theme, filter.label(), filter_count.to_string(), active)
                    .id(filter.id())
                    .debug_selector(|| filter.id().to_owned())
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
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
                    })),
            );
        }
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
        div()
            .size_full()
            .bg(theme.canvas)
            .flex()
            .flex_col()
            .child(
                div()
                    .id("download-page-header")
                    .debug_selector(|| "download-page-header".into())
                    .h(px(DOWNLOAD_PAGE_HEADER_HEIGHT))
                    .px(px(14.0))
                    .border_b_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.canvas)
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .text_size(px(15.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child("下载"),
                            )
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(theme.text_muted)
                                    .child(format!("{total_count} 个任务")),
                            ),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .id("download-search")
                            .debug_selector(|| "download-search".into())
                            .w(px(176.0))
                            .h(px(26.0))
                            .mr(px(8.0))
                            .rounded(px(4.0))
                            .border_1()
                            .border_color(theme.border_subtle)
                            .bg(theme.surface)
                            .flex()
                            .items_center()
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
                                    Button::new("download-search-clear")
                                        .icon(Icon::new(MaterialIcon::Close))
                                        .xsmall()
                                        .compact()
                                        .ghost()
                                        .on_click(move |_, window, cx| {
                                            clear_search_input.update(cx, |input, cx| {
                                                input.set_value("", window, cx);
                                            });
                                        }),
                                )
                            }),
                    )
                    .child(
                        Button::new("download-settings")
                            .icon(Icon::new(MaterialIcon::Settings))
                            .label("设置")
                            .small()
                            .compact()
                            .ghost()
                            .on_click({
                                let download_application = self.downloads.clone();
                                move |_, _, cx| {
                                    let download_application = download_application.clone();
                                    cx.defer(move |cx| {
                                        let _ =
                                            download_application.update(cx, |application, cx| {
                                                application.open_settings_window(cx);
                                            });
                                    });
                                }
                            }),
                    )
                    .child(
                        Button::new("download-schedule")
                            .icon(Icon::new(MaterialIcon::History))
                            .label("计划")
                            .small()
                            .compact()
                            .ghost()
                            .on_click({
                                let download_application = self.downloads.clone();
                                move |_, _, cx| {
                                    let _ = download_application.update(cx, |application, cx| {
                                        application.open_schedule_window(cx);
                                    });
                                }
                            }),
                    )
                    .child(
                        Button::new("download-new-task")
                            .icon(Icon::new(MaterialIcon::Add))
                            .label("新建下载")
                            .small()
                            .compact()
                            .primary()
                            .on_click(move |_, _, cx| {
                                let _ = download_application.update(cx, |application, cx| {
                                    application.open_new_download_window(cx);
                                });
                            }),
                    ),
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
                    .child(filter_rows)
                    .child(div().flex_1())
                    .child(
                        Button::new("download-sort")
                            .label(sort_label)
                            .xsmall()
                            .compact()
                            .ghost()
                            .h(px(26.0))
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
                                                                    &mut window.download_sort,
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
                    )
                    .child(
                        div()
                            .ml(px(8.0))
                            .text_size(px(9.5))
                            .text_color(theme.text_muted)
                            .child(filter_summary),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .child(download_table_header(
                        theme,
                        self.download_sort,
                        main_window,
                    ))
                    .child(if visible_count == 0 {
                        if total_count == 0 {
                            empty_state(
                                theme,
                                MaterialIcon::Download,
                                "还没有下载任务",
                                "点击右上角新建下载，提交真实 HTTP 或 HTTPS 地址。",
                            )
                            .flex_1()
                            .into_any_element()
                        } else if !search_query.is_empty() {
                            empty_state(
                                theme,
                                MaterialIcon::Search,
                                "没有匹配的下载任务",
                                "尝试缩短关键词，或切换到其他状态分类。",
                            )
                            .flex_1()
                            .into_any_element()
                        } else {
                            empty_state(
                                theme,
                                MaterialIcon::Download,
                                self.download_filter.empty_title(),
                                "任务状态变化后会自动出现在这里。",
                            )
                            .flex_1()
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
            configuration_status,
            update_pending,
            recording_ready,
            recording_active,
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
                    application.configuration_status().to_owned(),
                    application.hotkey_update_pending(),
                    application.hotkey_recording_ready(),
                    application.hotkey_recording_active(),
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
                    "核心不可用".into(),
                    false,
                    false,
                    false,
                    false,
                    nxtools_finder::QuickLaunchHistorySummary::default(),
                    QuickLaunchPerformanceSnapshot::default(),
                    std::path::PathBuf::new(),
                )
            });
        let recording_quick = self.recording_hotkey == Some(HotkeyRecordingTarget::QuickLaunch);
        if recording_quick && !recording_active {
            self.recording_hotkey = None;
        }
        let hotkey_label = if update_pending {
            "正在应用…".to_owned()
        } else if recording_quick {
            if recording_ready {
                "按下组合键…".to_owned()
            } else {
                "正在暂停…".to_owned()
            }
        } else {
            settings.hotkey.display.clone()
        };
        let hotkey_control = div()
            .id("record-quick-launch-hotkey")
            .h(px(26.0))
            .min_w(px(116.0))
            .px(px(9.0))
            .rounded(px(4.0))
            .border_1()
            .border_color(if recording_quick {
                theme.accent
            } else {
                theme.border
            })
            .bg(if recording_quick {
                theme.accent_soft
            } else {
                theme.canvas
            })
            .cursor_pointer()
            .flex()
            .items_center()
            .justify_center()
            .font_family("JetBrains Mono, Cascadia Mono, Consolas")
            .text_size(px(10.5))
            .text_color(if recording_quick {
                theme.accent_text
            } else {
                theme.text_secondary
            })
            .child(hotkey_label)
            .on_click(cx.listener(move |this, _, window, cx| {
                if update_pending {
                    return;
                }
                let recording = this.recording_hotkey != Some(HotkeyRecordingTarget::QuickLaunch);
                if this.recording_hotkey.take().is_some() {
                    this.set_all_hotkey_recording(false, cx);
                }
                this.recording_hotkey = recording.then_some(HotkeyRecordingTarget::QuickLaunch);
                this.set_all_hotkey_recording(recording, cx);
                if recording {
                    this.hotkey_focus.focus(window);
                }
                cx.notify();
            }))
            .into_any_element();
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
            .map(|status| {
                status
                    .drives
                    .iter()
                    .map(|drive| drive.mount_point.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let configured_volumes = settings.indexed_volumes.clone();
        let set_all_volumes = self.quick_launch.clone();
        let mut index_volumes_control = div()
            .w(px(210.0))
            .flex()
            .flex_wrap()
            .justify_end()
            .gap(px(4.0))
            .child(
                div()
                    .id("quick-launch-index-all-volumes")
                    .h(px(24.0))
                    .px(px(7.0))
                    .rounded(px(4.0))
                    .border_1()
                    .border_color(if configured_volumes.is_empty() {
                        theme.accent
                    } else {
                        theme.border_subtle
                    })
                    .bg(if configured_volumes.is_empty() {
                        theme.accent_soft
                    } else {
                        theme.canvas
                    })
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .text_size(px(10.0))
                    .text_color(if configured_volumes.is_empty() {
                        theme.accent_text
                    } else {
                        theme.text_secondary
                    })
                    .when(configured_volumes.is_empty(), |chip| {
                        chip.child(Icon::new(MaterialIcon::Check).size(px(12.0)))
                    })
                    .child("全部")
                    .on_click(move |_event, _window, cx| {
                        let _ = set_all_volumes.update(cx, |application, cx| {
                            application.set_indexed_volumes(Vec::new(), cx);
                        });
                    }),
            );
        if available_volumes.is_empty() {
            index_volumes_control = index_volumes_control.child(
                div()
                    .h(px(24.0))
                    .px(px(6.0))
                    .flex()
                    .items_center()
                    .text_size(px(9.5))
                    .text_color(theme.text_muted)
                    .child("等待发现磁盘"),
            );
        }
        for mount_point in &available_volumes {
            let selected = volume_is_selected(&configured_volumes, mount_point);
            let label = mount_point.trim_end_matches('\\').to_owned();
            let mount_for_click = mount_point.clone();
            let configured_for_click = configured_volumes.clone();
            let available_for_click = available_volumes.clone();
            let quick_launch = self.quick_launch.clone();
            index_volumes_control = index_volumes_control.child(
                div()
                    .id(gpui::SharedString::from(format!(
                        "quick-launch-index-volume-{label}"
                    )))
                    .h(px(24.0))
                    .px(px(7.0))
                    .rounded(px(4.0))
                    .border_1()
                    .border_color(if selected {
                        theme.accent
                    } else {
                        theme.border_subtle
                    })
                    .bg(if selected {
                        theme.accent_soft
                    } else {
                        theme.canvas
                    })
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .text_size(px(10.0))
                    .text_color(if selected {
                        theme.accent_text
                    } else {
                        theme.text_secondary
                    })
                    .when(selected, |chip| {
                        chip.child(Icon::new(MaterialIcon::Check).size(px(12.0)))
                    })
                    .child(label)
                    .on_click(move |_event, _window, cx| {
                        let selected = toggle_indexed_volume_selection(
                            &configured_for_click,
                            &available_for_click,
                            &mount_for_click,
                        );
                        let _ = quick_launch.update(cx, |application, cx| {
                            application.set_indexed_volumes(selected, cx);
                        });
                    }),
            );
        }
        let mut index_exclusions_control = div()
            .w(px(210.0))
            .flex()
            .flex_col()
            .items_end()
            .gap(px(4.0));
        if settings.excluded_paths.is_empty() {
            index_exclusions_control = index_exclusions_control.child(
                div()
                    .w_full()
                    .h(px(24.0))
                    .px(px(7.0))
                    .rounded(px(4.0))
                    .border_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.canvas)
                    .flex()
                    .items_center()
                    .text_size(px(9.5))
                    .text_color(theme.text_muted)
                    .child("未排除目录"),
            );
        }
        for (index, path) in settings.excluded_paths.iter().enumerate() {
            let path_to_remove = path.clone();
            let quick_launch = self.quick_launch.clone();
            index_exclusions_control = index_exclusions_control.child(
                div()
                    .w_full()
                    .h(px(24.0))
                    .px(px(7.0))
                    .rounded(px(4.0))
                    .border_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.canvas)
                    .flex()
                    .items_center()
                    .gap(px(5.0))
                    .child(
                        Icon::new(MaterialIcon::Folder)
                            .size(px(13.0))
                            .text_color(theme.icon_muted),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .truncate()
                            .text_size(px(9.5))
                            .text_color(theme.text_secondary)
                            .child(path.clone()),
                    )
                    .child(
                        div()
                            .id(("quick-launch-remove-exclusion", index))
                            .size(px(18.0))
                            .rounded(px(3.0))
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                Icon::new(MaterialIcon::Close)
                                    .size(px(12.0))
                                    .text_color(theme.icon_muted),
                            )
                            .on_click(move |_event, _window, cx| {
                                let _ = quick_launch.update(cx, |application, cx| {
                                    application.remove_excluded_path(&path_to_remove, cx);
                                });
                                cx.stop_propagation();
                            }),
                    ),
            );
        }
        let add_exclusion = self.quick_launch.clone();
        index_exclusions_control = index_exclusions_control.child(
            Button::new("quick-launch-add-index-exclusion")
                .ghost()
                .compact()
                .xsmall()
                .h(px(24.0))
                .px(px(7.0))
                .rounded(px(4.0))
                .border_1()
                .border_color(theme.border_subtle)
                .bg(theme.surface_raised)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .child(Icon::new(MaterialIcon::Add).size(px(12.0)))
                        .child(div().text_size(px(10.0)).child("添加目录")),
                )
                .on_click(move |_event, _window, cx| {
                    let _ = add_exclusion.update(cx, |application, cx| {
                        application.request_index_exclusion_directory(cx);
                    });
                }),
        );
        let index_volumes_control = index_volumes_control.into_any_element();
        let index_exclusions_control = index_exclusions_control.into_any_element();
        if self
            .selected_custom_command
            .is_some_and(|index| index >= settings.custom_commands.len())
        {
            self.selected_custom_command = None;
        }
        let mut command_list = div()
            .id("quick-launch-custom-command-list")
            .min_h_0()
            .flex_1()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(3.0));
        for (index, command) in settings.custom_commands.iter().cloned().enumerate() {
            let selected = self.selected_custom_command == Some(index);
            let select_index = index;
            let toggle_command = command.clone();
            let toggle_quick_launch = self.quick_launch.clone();
            let delete_quick_launch = self.quick_launch.clone();
            command_list = command_list.child(
                div()
                    .id(gpui::SharedString::from(format!(
                        "quick-launch-command-{index}"
                    )))
                    .h(px(34.0))
                    .px(px(6.0))
                    .rounded(px(4.0))
                    .bg(if selected {
                        theme.element_selected
                    } else {
                        theme.canvas
                    })
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        div()
                            .id(gpui::SharedString::from(format!(
                                "quick-launch-command-enabled-{index}"
                            )))
                            .size(px(18.0))
                            .rounded(px(3.0))
                            .border_1()
                            .border_color(if command.enabled {
                                theme.accent
                            } else {
                                theme.border
                            })
                            .bg(if command.enabled {
                                theme.accent_soft
                            } else {
                                theme.surface
                            })
                            .flex()
                            .items_center()
                            .justify_center()
                            .when(command.enabled, |control| {
                                control.child(Icon::new(MaterialIcon::Check).size(px(11.0)))
                            })
                            .on_click(move |_event, _window, cx| {
                                let mut updated = toggle_command.clone();
                                updated.enabled = !updated.enabled;
                                let _ = toggle_quick_launch.update(cx, |application, cx| {
                                    application.update_custom_command(index, updated, cx);
                                });
                                cx.stop_propagation();
                            }),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(1.0))
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(10.5))
                                    .text_color(theme.text)
                                    .child(if command.title.is_empty() {
                                        "未命名命令".to_owned()
                                    } else {
                                        command.title.clone()
                                    }),
                            )
                            .child(
                                div()
                                    .truncate()
                                    .font_family("JetBrains Mono, Cascadia Mono, Consolas")
                                    .text_size(px(9.0))
                                    .text_color(theme.text_muted)
                                    .child(if command.keyword.is_empty() {
                                        "未设置关键词".to_owned()
                                    } else {
                                        command.keyword.clone()
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .id(gpui::SharedString::from(format!(
                                "quick-launch-command-delete-{index}"
                            )))
                            .size(px(20.0))
                            .rounded(px(3.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .hover(move |style| style.bg(theme.surface_hover))
                            .child(
                                Icon::new(MaterialIcon::Delete)
                                    .size(px(12.0))
                                    .text_color(theme.icon_muted),
                            )
                            .on_click(cx.listener(move |this, _event, _window, cx| {
                                let removed = delete_quick_launch
                                    .update(cx, |application, cx| {
                                        application.remove_custom_command(index, cx)
                                    })
                                    .unwrap_or(false);
                                if removed {
                                    this.selected_custom_command = None;
                                }
                                cx.stop_propagation();
                                cx.notify();
                            })),
                    )
                    .on_click(cx.listener(move |this, _event, window, cx| {
                        this.select_custom_command(select_index, window, cx);
                    })),
            );
        }
        if settings.custom_commands.is_empty() {
            command_list = command_list.child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(10.0))
                    .text_color(theme.text_muted)
                    .child("还没有自定义命令"),
            );
        }
        let add_command = Button::new("quick-launch-add-custom-command")
            .ghost()
            .compact()
            .xsmall()
            .h(px(24.0))
            .px(px(7.0))
            .rounded(px(4.0))
            .border_1()
            .border_color(theme.border_subtle)
            .bg(theme.surface_raised)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(Icon::new(MaterialIcon::Add).size(px(12.0)))
                    .child(div().text_size(px(10.0)).child("新增")),
            )
            .on_click(cx.listener(move |this, _event, window, cx| {
                if let Ok(index) = this
                    .quick_launch
                    .update(cx, |application, cx| application.add_custom_command(cx))
                {
                    this.select_custom_command(index, window, cx);
                }
            }));
        let command_editor = if self.selected_custom_command.is_some() {
            let admin_enabled = self.custom_command_run_as_admin;
            let silent_enabled = self.custom_command_run_silently;
            let selected_command = self.selected_custom_command;
            let pick_program = Button::new("quick-launch-pick-command-program")
                .ghost()
                .compact()
                .xsmall()
                .h(px(26.0))
                .w(px(28.0))
                .p_0()
                .rounded(px(4.0))
                .border_1()
                .border_color(theme.border_subtle)
                .bg(theme.surface_raised)
                .tooltip("选择程序")
                .child(Icon::new(MaterialIcon::Folder).size(px(13.0)))
                .on_click(cx.listener(move |this, _event, _window, cx| {
                    let Some(index) = selected_command else {
                        return;
                    };
                    let current_path = this.custom_command_program.read(cx).value().to_string();
                    let _ = this.quick_launch.update(cx, |application, cx| {
                        application.request_custom_command_program(index, current_path, cx);
                    });
                }));
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(
                    div()
                        .flex()
                        .gap(px(6.0))
                        .child(command_input_field(
                            theme,
                            "关键词",
                            &self.custom_command_keyword,
                            px(150.0),
                        ))
                        .child(command_input_field(
                            theme,
                            "显示名称",
                            &self.custom_command_title,
                            px(220.0),
                        )),
                )
                .child(
                    div()
                        .flex()
                        .items_end()
                        .gap(px(6.0))
                        .child(command_input_field(
                            theme,
                            "程序",
                            &self.custom_command_program,
                            px(342.0),
                        ))
                        .child(pick_program),
                )
                .child(
                    div()
                        .flex()
                        .gap(px(6.0))
                        .child(command_input_field(
                            theme,
                            "参数模板",
                            &self.custom_command_arguments,
                            px(240.0),
                        ))
                        .child(command_input_field(
                            theme,
                            "工作目录",
                            &self.custom_command_working_directory,
                            px(240.0),
                        )),
                )
                .child(
                    div()
                        .h(px(26.0))
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(command_flag(
                            theme,
                            "quick-launch-command-admin",
                            "管理员",
                            admin_enabled,
                            cx.listener(move |this, _, _, cx| {
                                this.custom_command_run_as_admin = !admin_enabled;
                                cx.notify();
                            }),
                        ))
                        .child(command_flag(
                            theme,
                            "quick-launch-command-silent",
                            "静默",
                            silent_enabled,
                            cx.listener(move |this, _, _, cx| {
                                this.custom_command_run_silently = !silent_enabled;
                                cx.notify();
                            }),
                        ))
                        .child(div().flex_1())
                        .child(
                            Button::new("quick-launch-save-custom-command")
                                .ghost()
                                .compact()
                                .xsmall()
                                .h(px(24.0))
                                .px(px(10.0))
                                .rounded(px(4.0))
                                .border_1()
                                .border_color(theme.accent)
                                .bg(theme.accent_soft)
                                .child(div().text_size(px(10.0)).child("保存"))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.save_selected_custom_command(cx);
                                })),
                        ),
                )
                .into_any_element()
        } else {
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(10.5))
                .text_color(theme.text_muted)
                .child("选择左侧命令后编辑")
                .into_any_element()
        };
        let custom_commands_control = div()
            .h(px(190.0))
            .rounded(px(5.0))
            .border_1()
            .border_color(theme.border_subtle)
            .bg(theme.surface)
            .flex()
            .child(
                div()
                    .w(px(238.0))
                    .min_h_0()
                    .p(px(8.0))
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .child(
                        div()
                            .h(px(24.0))
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .flex_1()
                                    .text_size(px(11.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.text_secondary)
                                    .child("自定义命令"),
                            )
                            .child(add_command),
                    )
                    .child(command_list),
            )
            .child(div().w(px(1.0)).bg(theme.border_subtle))
            .child(div().min_w_0().flex_1().p(px(9.0)).child(command_editor))
            .into_any_element();
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
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(page_header(
                theme,
                "速启",
                "独立弹窗 · 应用、文件与系统工具",
                None,
            ))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .relative()
                    .child(
                        div()
                            .id("quick-launch-settings-scroll")
                            .size_full()
                            .track_scroll(&self.quick_find_scroll)
                            .overflow_y_scroll()
                            .on_scroll_wheel(cx.listener(
                                |this, event: &ScrollWheelEvent, _window, cx| {
                                    let delta =
                                        event.delta.pixel_delta(px(QUICK_FIND_SCROLL_LINE_HEIGHT));
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
                            .p(px(12.0))
                            .flex()
                            .flex_col()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .rounded(px(5.0))
                                    .border_1()
                                    .border_color(theme.border_subtle)
                                    .bg(theme.surface)
                                    .flex()
                                    .child(div().flex_1().min_w_0().px(px(12.0)).child(
                                        compact_group(
                                            theme,
                                            "呼出与执行",
                                            vec![
                                        compact_setting_row(
                                            theme,
                                            "全局呼出热键",
                                            format!("Windows 注册状态 · {configuration_status}"),
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
                                    ))
                                    .child(div().w(px(1.0)).bg(theme.border_subtle))
                                    .child(div().w(px(410.0)).px(px(12.0)).child(compact_group(
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
                                                .gap(px(7.0))
                                                .child(
                                                    div()
                                                        .text_size(px(10.0))
                                                        .text_color(file_index_badge_color)
                                                        .child(file_index_badge),
                                                )
                                                .child(refresh_volumes_control)
                                                .into_any_element(),
                                        ),
                                        compact_setting_row(
                                            theme,
                                            "索引卷",
                                            "勾选要索引的磁盘；全部会自动包含新 NTFS 卷",
                                            index_volumes_control,
                                        ),
                                        compact_setting_row(
                                            theme,
                                            "排除目录",
                                            "通过选择器添加；目录及其所有子项不进入结果",
                                            index_exclusions_control,
                                        ),
                                    ],
                                    ))),
                            )
                            .child(history_diagnostics_control)
                            .child(custom_commands_control),
                    )
                    .child(
                        Scrollbar::vertical(&self.quick_find_scroll)
                            .id("quick-launch-settings-scrollbar"),
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
            .font_family("JetBrains Mono, Cascadia Mono, Consolas")
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
                        this.set_all_hotkey_recording(false, cx);
                    }
                    this.recording_hotkey = recording.then_some(HotkeyRecordingTarget::Clipboard);
                    this.set_all_hotkey_recording(recording, cx);
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
                div()
                    .h(px(48.0))
                    .px(px(14.0))
                    .border_b_1()
                    .border_color(theme.border_subtle)
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .child(
                        div()
                            .text_size(px(15.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("剪贴板"),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .font_family("JetBrains Mono, Cascadia Mono, Consolas")
                            .text_color(theme.text_muted)
                            .child(format!(
                                "ALL {:04}  TXT {:04}  IMG {:04}  FILE {:04}",
                                overview.total, overview.text, overview.images, overview.files
                            )),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(if settings.enabled {
                                theme.accent_text
                            } else {
                                theme.text_muted
                            })
                            .child(if settings.enabled {
                                "● CAPTURE ON"
                            } else {
                                "○ CAPTURE OFF"
                            }),
                    ),
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
            .size_full()
            .relative()
            .overflow_hidden()
            .track_focus(&self.hotkey_focus)
            .on_key_down(cx.listener(|this, event, _window, cx| {
                this.handle_hotkey_recording(event, cx);
            }))
            .font_family("Inter, Microsoft YaHei UI, Segoe UI")
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

fn command_input_field(
    theme: Theme,
    label: &'static str,
    input: &Entity<InputState>,
    width: gpui::Pixels,
) -> AnyElement {
    div()
        .w(width)
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .child(
            div()
                .text_size(px(9.0))
                .text_color(theme.text_muted)
                .child(label),
        )
        .child(
            div()
                .h(px(25.0))
                .rounded(px(4.0))
                .border_1()
                .border_color(theme.border)
                .bg(theme.canvas)
                .px(px(6.0))
                .flex()
                .items_center()
                .child(
                    Input::new(input)
                        .appearance(false)
                        .bordered(false)
                        .focus_bordered(false),
                ),
        )
        .into_any_element()
}

fn command_flag(
    theme: Theme,
    id: &'static str,
    label: &'static str,
    enabled: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    div()
        .id(id)
        .h(px(23.0))
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
        .gap(px(4.0))
        .when(enabled, |control| {
            control.child(Icon::new(MaterialIcon::Check).size(px(10.0)))
        })
        .text_size(px(9.5))
        .text_color(if enabled {
            theme.accent_text
        } else {
            theme.text_muted
        })
        .child(label)
        .on_click(on_click)
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

fn window_control(
    glyph: &'static str,
    control: WindowControlArea,
    close: bool,
    theme: Theme,
) -> impl IntoElement {
    div()
        .id(gpui::SharedString::from(format!("window-control-{glyph}")))
        .w(px(42.0))
        .h_full()
        .window_control_area(control)
        .flex()
        .items_center()
        .justify_center()
        .font_family("Segoe Fluent Icons, Segoe MDL2 Assets")
        .text_size(px(10.0))
        .text_color(theme.text_secondary)
        .hover(move |style| {
            if close {
                style.bg(gpui::rgb(0xc42b1c)).text_color(gpui::white())
            } else {
                style.bg(theme.surface_hover).text_color(theme.text)
            }
        })
        .child(glyph)
}

fn page_header(
    theme: Theme,
    title: &'static str,
    description: &'static str,
    action: Option<(MaterialIcon, &'static str)>,
) -> impl IntoElement {
    div()
        .h(px(68.0))
        .px(px(20.0))
        .border_b_1()
        .border_color(theme.border_subtle)
        .bg(theme.canvas)
        .flex()
        .items_center()
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(3.0))
                .child(
                    div()
                        .text_size(px(18.0))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child(title),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(theme.text_muted)
                        .child(description),
                ),
        )
        .child(div().flex_1())
        .when_some(action, |header, (icon, label)| {
            header.child(
                div()
                    .h(px(34.0))
                    .px(px(12.0))
                    .rounded(px(6.0))
                    .bg(theme.accent_soft)
                    .border_1()
                    .border_color(theme.accent)
                    .flex()
                    .items_center()
                    .gap(px(7.0))
                    .child(Icon::new(icon).size(px(16.0)).text_color(theme.accent))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme.accent_text)
                            .child(label),
                    ),
            )
        })
}

fn download_task_row(
    theme: Theme,
    application: gpui::WeakEntity<DownloadApplication>,
    main_window: Entity<MainWindow>,
    selection: DownloadRowSelection,
    task: DownloadEngineTask,
) -> AnyElement {
    let DownloadRowSelection {
        selected,
        visible_task_ids,
    } = selection;
    let task_id = task.task.id();
    let element_id = task_id.as_uuid().as_u64_pair().0;
    let status = task.task.status();
    let name = download_task_name(&task).to_owned();
    let path = task.final_path.as_ref().map_or_else(
        || task.task.request().source_url.clone(),
        |path| path.display().to_string(),
    );
    let (status_label, status_color) = download_status_presentation(theme, status);
    let progress = task
        .total_bytes
        .filter(|total| *total > 0)
        .map_or(0.0, |total| {
            (task.downloaded_bytes as f32 / total as f32).clamp(0.0, 1.0)
        });
    let progress_text = match task.total_bytes {
        Some(total) => format!(
            "{} / {}",
            format_download_bytes(task.downloaded_bytes),
            format_download_bytes(total)
        ),
        None => format_download_bytes(task.downloaded_bytes),
    };
    let (secondary_text, secondary_color) = task.error.as_ref().map_or_else(
        || (path, theme.text_muted),
        |error| (error.message.clone(), theme.danger),
    );
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
                menu,
                context_application.clone(),
                ordered_task_ids,
                capabilities,
            )
        })
        .child(
            div()
                .size_full()
                .px(px(8.0))
                .border_l_2()
                .border_color(if selected {
                    theme.accent
                } else {
                    gpui::rgba(0x00000000)
                })
                .flex()
                .items_center()
                .gap(px(10.0))
                .child(
                    div()
                        .size(px(26.0))
                        .rounded(px(4.0))
                        .bg(theme.surface_raised)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(Icon::new(MaterialIcon::Download).size(px(15.0)).text_color(
                            if status == DownloadStatus::Completed {
                                theme.success
                            } else {
                                theme.accent
                            },
                        )),
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
                        .gap(px(1.0))
                        .child(
                            div()
                                .when(selected, |text| {
                                    text.debug_selector(|| {
                                        "download-task-name-text-selected".into()
                                    })
                                })
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_size(px(11.5))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.text)
                                .child(name),
                        )
                        .child(
                            div()
                                .when(selected, |text| {
                                    text.debug_selector(|| {
                                        "download-task-path-text-selected".into()
                                    })
                                })
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_size(px(9.0))
                                .text_color(secondary_color)
                                .child(secondary_text),
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
                        .gap(px(5.0))
                        .child(
                            div()
                                .text_size(px(9.0))
                                .text_color(theme.text_muted)
                                .child(progress_text),
                        )
                        .child(
                            div()
                                .w_full()
                                .h(px(3.0))
                                .rounded(px(2.0))
                                .bg(theme.surface_raised)
                                .child(
                                    div()
                                        .w(px(DOWNLOAD_PROGRESS_COLUMN_WIDTH * progress))
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
                        .text_size(px(9.5))
                        .text_color(status_color)
                        .child(status_label),
                ),
        )
        .into_any_element()
}

fn download_popup_item(
    _selector: &'static str,
    label: &'static str,
    icon: MaterialIcon,
) -> PopupMenuItem {
    let item = if cfg!(test) {
        PopupMenuItem::element(move |_window, _cx| {
            div()
                .w_full()
                .debug_selector(move || _selector.into())
                .child(label)
        })
    } else {
        PopupMenuItem::new(label)
    };
    item.icon(Icon::new(icon))
}

fn download_task_context_menu(
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
        download_popup_item(selector, label, icon).on_click(move |_event, _window, cx| {
            let _ = application.update(cx, |application, cx| {
                application.batch_action(targets.clone(), action, cx);
            });
        })
    };
    let queue_item = |selector: &'static str,
                      label: &'static str,
                      placement: DownloadQueuePlacement,
                      icon: MaterialIcon| {
        let application = application.clone();
        let task_ids = task_ids.clone();
        download_popup_item(selector, label, icon).on_click(move |_event, _window, cx| {
            let _ = application.update(cx, |application, cx| {
                application.move_queue_tasks(task_ids.clone(), placement, cx);
            });
        })
    };
    let artifact_item = |task_id: DownloadTaskId,
                         selector: &'static str,
                         label: &'static str,
                         action: ShellOpenAction,
                         icon: MaterialIcon| {
        let application = application.clone();
        download_popup_item(selector, label, icon).on_click(move |_event, _window, cx| {
            let _ = application.update(cx, |application, cx| {
                application.open_task_artifact(task_id, action, cx);
            });
        })
    };
    let progress_application = application.clone();
    let progress_task = (task_ids.len() == 1).then(|| task_ids[0]);
    let delete_application = application.clone();
    let mut menu = menu.min_w(px(0.0));
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
            download_popup_item("download-menu-task-window", "任务窗口", MaterialIcon::Info)
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
                "download-menu-delete-file",
                "删除文件",
                MaterialIcon::Delete,
            )
            .on_click(move |_event, _window, cx| {
                let _ = delete_application.update(cx, |application, cx| {
                    application.request_delete_confirmation(task_id, cx);
                });
            }),
        );
        has_visible_items = true;
    }
    if plan.remove {
        menu = menu.item(batch_item(
            "download-menu-remove",
            "从列表移除",
            DownloadBatchAction::Remove,
            MaterialIcon::Close,
            task_ids,
        ));
        has_visible_items = true;
    }
    debug_assert!(
        has_visible_items,
        "task context menu requires a visible action"
    );
    menu
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
        .h(px(30.0))
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
                .text_size(px(10.0))
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

    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .flex()
                .gap(px(14.0))
                .child(download_detail_metric(
                    theme,
                    "状态",
                    status_label.into(),
                    status_color,
                ))
                .child(download_detail_metric(
                    theme,
                    "大小",
                    total.map_or_else(|| "未知".into(), format_download_bytes),
                    theme.text,
                ))
                .child(download_detail_metric(
                    theme,
                    "已下载",
                    format_download_bytes(downloaded),
                    theme.text,
                ))
                .child(download_detail_metric(
                    theme,
                    "速度",
                    if bytes_per_second == 0 {
                        "—".into()
                    } else {
                        format!("{}/s", format_download_bytes(bytes_per_second))
                    },
                    theme.text,
                ))
                .child(download_detail_metric(theme, "剩余", remaining, theme.text))
                .child(download_detail_metric(
                    theme,
                    "连接",
                    active_connections.to_string(),
                    theme.text,
                )),
        )
        .child(
            div()
                .flex()
                .gap(px(14.0))
                .child(download_detail_metric(
                    theme,
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
                .child(download_detail_metric(
                    theme,
                    "来源",
                    download_origin_label(task.task.origin()).into(),
                    theme.text_secondary,
                ))
                .child(download_detail_metric(
                    theme,
                    "计划",
                    request
                        .scheduled_at_unix_ms
                        .map_or_else(|| "立即".into(), format_download_timestamp),
                    theme.text_secondary,
                ))
                .child(div().flex_1())
                .child(div().flex_1())
                .child(div().flex_1()),
        )
        .child(download_detail_line(
            theme,
            "URL",
            request.source_url.clone(),
        ))
        .child(download_detail_line(theme, "文件", final_path))
        .when_some(task.error.as_ref(), |content, error| {
            content.child(download_detail_line_with_color(
                theme,
                "错误",
                error.message.clone(),
                theme.danger,
            ))
        })
        .into_any_element()
}

fn download_segment_detail(
    theme: Theme,
    detail: Option<&DownloadTaskDetail>,
    detail_error: Option<&str>,
) -> AnyElement {
    let Some(detail) = detail else {
        return detail_error.map_or_else(
            || download_detail_empty(theme, "正在加载分段详情"),
            |error| download_detail_error(theme, error),
        );
    };
    if detail.segments.is_empty() {
        return download_detail_empty(theme, "尚未生成分段计划");
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
                .text_size(px(9.5))
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
                .text_size(px(9.5))
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
            || download_detail_empty(theme, "正在加载任务日志"),
            |error| download_detail_error(theme, error),
        );
    };
    if detail.timeline.is_empty() {
        return download_detail_empty(theme, "暂无任务日志");
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
                .text_size(px(9.5))
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
                .text_size(px(10.0))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.text_secondary)
                .child(title),
        );
    for (label, value) in rows {
        section = section.child(download_detail_line_owned(theme, label, value));
    }
    section.into_any_element()
}

fn download_detail_empty(theme: Theme, message: &'static str) -> AnyElement {
    div()
        .h(px(88.0))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(10.0))
        .text_color(theme.text_muted)
        .child(message)
        .into_any_element()
}

fn download_detail_error(theme: Theme, message: &str) -> AnyElement {
    div()
        .h(px(88.0))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(10.0))
        .text_color(theme.danger)
        .child(format!("详情加载失败：{message}"))
        .into_any_element()
}

fn download_detail_metric(
    theme: Theme,
    label: &'static str,
    value: String,
    value_color: gpui::Rgba,
) -> AnyElement {
    div()
        .min_w_0()
        .flex_1()
        .flex()
        .flex_col()
        .gap(px(1.0))
        .child(
            div()
                .text_size(px(8.5))
                .text_color(theme.text_muted)
                .child(label),
        )
        .child(
            div()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_size(px(9.5))
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
        .text_size(px(9.5))
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
        .min_h(px(19.0))
        .flex()
        .items_start()
        .gap(px(8.0))
        .text_size(px(9.5))
        .child(
            div()
                .w(px(44.0))
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
    let progress_header = download_sort_header_cell(
        theme,
        "进度",
        DownloadSortKey::Progress,
        sort,
        main_window.clone(),
    )
    .w(px(DOWNLOAD_PROGRESS_COLUMN_WIDTH))
    .flex_none();
    let status_header =
        download_sort_header_cell(theme, "状态", DownloadSortKey::Status, sort, main_window)
            .w(px(DOWNLOAD_STATUS_COLUMN_WIDTH))
            .flex_none();
    div()
        .id("download-table-header")
        .debug_selector(|| "download-table-header".into())
        .h(px(DOWNLOAD_TABLE_HEADER_HEIGHT))
        .px(px(10.0))
        .border_b_1()
        .border_color(theme.border_subtle)
        .bg(theme.surface)
        .flex()
        .items_center()
        .gap(px(10.0))
        .text_size(px(9.5))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(theme.text_muted)
        .child(name_header)
        .child(progress_header)
        .child(status_header)
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

fn download_filter_chip(
    theme: Theme,
    label: &'static str,
    count: impl Into<String>,
    active: bool,
) -> gpui::Div {
    div()
        .h(px(26.0))
        .px(px(8.0))
        .rounded(px(4.0))
        .border_1()
        .border_color(if active {
            theme.border_active
        } else {
            gpui::rgba(0x00000000)
        })
        .bg(if active {
            theme.accent_soft
        } else {
            theme.surface
        })
        .when(!active, |chip| {
            chip.hover(move |style| style.bg(theme.surface_hover))
        })
        .flex()
        .items_center()
        .gap(px(6.0))
        .child(
            div()
                .text_size(px(10.5))
                .text_color(if active {
                    theme.accent_text
                } else {
                    theme.text_secondary
                })
                .child(label),
        )
        .child(
            div()
                .min_w(px(17.0))
                .h(px(17.0))
                .px(px(4.0))
                .rounded(px(3.0))
                .bg(if active {
                    theme.surface_hover
                } else {
                    theme.surface_raised
                })
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(9.0))
                .text_color(if active {
                    theme.accent_text
                } else {
                    theme.text_muted
                })
                .child(count.into()),
        )
}

fn empty_state(
    theme: Theme,
    icon: MaterialIcon,
    title: &'static str,
    description: &'static str,
) -> gpui::Div {
    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(8.0))
        .child(
            div()
                .size(px(42.0))
                .rounded(px(10.0))
                .bg(theme.surface_raised)
                .flex()
                .items_center()
                .justify_center()
                .child(Icon::new(icon).size(px(21.0)).text_color(theme.text_muted)),
        )
        .child(
            div()
                .text_size(px(13.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme.text_secondary)
                .child(title),
        )
        .child(
            div()
                .max_w(px(430.0))
                .text_center()
                .text_size(px(11.0))
                .text_color(theme.text_muted)
                .child(description),
        )
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use gpui::{
        AppContext, KeyDownEvent, Keystroke, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent,
        TestAppContext, px, size,
    };
    use gpui_component::PixelsExt;
    use nxtools_core::{AppConfig, ClipboardHotKeyModifier};
    use nxtools_download::{
        DownloadEngine, DownloadRequest, DownloadSegment, DownloadSegmentState, DownloadStatus,
        DownloadTaskId, RequestOrigin,
    };

    use nxtools_protocol::{ActivateWindowCommand, IndexPhase, IndexStatus, PlatformCommand};

    use crate::clipboard::ClipboardApplication;
    use crate::downloads::{DownloadApplication, DownloadRuntimeContext};
    use crate::notifications::NotificationCenter;
    use crate::quick_launch::QuickLaunchApplication;
    use crate::shell::MainModule;

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
        assert_eq!(primary, Some(tasks[2]));
        assert_eq!(anchor, Some(tasks[2]));
    }

    use super::{
        DOWNLOAD_DETAIL_HEIGHT, DOWNLOAD_FILTER_STRIP_HEIGHT, DOWNLOAD_PAGE_HEADER_HEIGHT,
        DOWNLOAD_TABLE_HEADER_HEIGHT, DOWNLOAD_TASK_ROW_HEIGHT, DownloadContextCapabilities,
        DownloadContextMenuPlan, DownloadDetailTab, DownloadSortKey, DownloadSortState,
        DownloadTaskFilter, MainWindow, download_context_capabilities, download_context_menu_plan,
        download_filter_counts, download_queue_placement_capabilities,
        download_segment_strip_cells, download_tasks_for_view, file_index_summary,
        format_query_elapsed, hotkey_from_key_down, masked_request_header_value,
        notification_transition, prepare_download_context_selection, quick_find_scroll_step,
        quick_find_scroll_target, reconcile_download_selection, set_download_sort,
        toggle_indexed_volume_selection, update_download_selection,
    };

    #[test]
    fn download_workspace_uses_the_compact_editor_density_scale() {
        assert_eq!(DOWNLOAD_PAGE_HEADER_HEIGHT, 48.0);
        assert_eq!(DOWNLOAD_FILTER_STRIP_HEIGHT, 38.0);
        assert_eq!(DOWNLOAD_TABLE_HEADER_HEIGHT, 28.0);
        assert_eq!(DOWNLOAD_TASK_ROW_HEIGHT, 46.0);
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
                    file_name: Some("Zeta.bin".into()),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
                    connection_limit: Some(1),
                    speed_limit_bytes_per_second: None,
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
                },
                RequestOrigin::Internal,
            )
            .expect("enqueue first");
        let second = engine
            .enqueue(
                DownloadRequest {
                    source_url: "https://mirror.invalid/alpha.bin".into(),
                    save_directory: r"C:\Downloads".into(),
                    file_name: Some("Alpha.bin".into()),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
                    connection_limit: Some(1),
                    speed_limit_bytes_per_second: None,
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
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
                    file_name: Some("completed.bin".into()),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
                    connection_limit: Some(1),
                    speed_limit_bytes_per_second: None,
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
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
                    },
                    cx,
                )
            })
        });
        let downloads_for_window = downloads.downgrade();
        let (_main_window, window) = cx.add_window_view(move |window, cx| {
            let mut main_window = MainWindow::new(
                gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
                gpui::WeakEntity::<QuickLaunchApplication>::new_invalid(),
                downloads_for_window,
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

        for selector in [
            "download-menu-open-file",
            "download-menu-reveal-file",
            "download-menu-task-window",
            "download-menu-delete-file",
            "download-menu-remove",
        ] {
            assert!(
                window.debug_bounds(selector).is_some(),
                "completed task menu must render {selector}"
            );
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
                notifications,
                window,
                cx,
            );
            main_window.shell.activate(MainModule::Downloads);
            main_window
        });
        let name_header = window
            .debug_bounds("download-sort-header-name")
            .expect("rendered sortable name header");

        window.simulate_click(name_header.center(), Modifiers::default());
        window.simulate_click(name_header.center(), Modifiers::default());
        assert_eq!(
            cx.read(|cx| main_window.read(cx).download_sort),
            DownloadSortState {
                key: DownloadSortKey::Name,
                descending: true,
            }
        );
    }

    #[gpui::test]
    fn production_download_rows_and_detail_remain_coherent_at_compact_width(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let engine = DownloadEngine::open_in_memory().expect("open in-memory download engine");
        let first_task = engine
            .enqueue(
                DownloadRequest {
                    source_url: "https://example.com/first.bin".into(),
                    save_directory: r"C:\Downloads".into(),
                    file_name: Some("first.bin".into()),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
                    connection_limit: Some(4),
                    speed_limit_bytes_per_second: None,
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
                },
                RequestOrigin::Internal,
            )
            .expect("enqueue first task");
        let second_task = engine
            .enqueue(
                DownloadRequest {
                    source_url: "https://example.com/second.bin".into(),
                    save_directory: r"C:\Downloads".into(),
                    file_name: Some("second.bin".into()),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
                    connection_limit: Some(4),
                    speed_limit_bytes_per_second: None,
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
                },
                RequestOrigin::Internal,
            )
            .expect("enqueue second task");
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
                notifications,
                window,
                cx,
            );
            main_window.shell.activate(MainModule::Downloads);
            main_window
        });
        window.simulate_resize(size(px(720.0), px(500.0)));

        let page_header = window
            .debug_bounds("download-page-header")
            .expect("download page header bounds");
        let search = window
            .debug_bounds("download-search")
            .expect("download search bounds");
        let segment_tab = window
            .debug_bounds("download-detail-tab-segments")
            .expect("download segment detail tab bounds");
        assert!(page_header.contains(&search.origin));
        assert!(page_header.contains(&search.bottom_right()));
        window.simulate_click(segment_tab.center(), Modifiers::default());
        assert_eq!(
            window.read(|cx| main_window.read(cx).download_detail_tab),
            DownloadDetailTab::Segments
        );

        let selected_before = window
            .read(|cx| main_window.read(cx).selected_download_task)
            .expect("first visible task is selected");
        assert!(selected_before == first_task || selected_before == second_task);
        let other_task = if selected_before == first_task {
            second_task
        } else {
            first_task
        };
        let other_element_id = other_task.as_uuid().as_u64_pair().0;
        let other_row_selector: &'static str =
            Box::leak(format!("download-task-row-{other_element_id}").into_boxed_str());
        let other_row = window
            .debug_bounds(other_row_selector)
            .expect("second production task row bounds");
        assert_eq!(
            other_row.size.height.as_f32(),
            DOWNLOAD_TASK_ROW_HEIGHT,
            "the main table must keep its compact row density"
        );
        window.simulate_mouse_down(
            other_row.center(),
            gpui::MouseButton::Right,
            Modifiers::default(),
        );
        assert_eq!(
            window.read(|cx| main_window.read(cx).selected_download_task),
            Some(other_task)
        );
        assert_eq!(
            window.read(|cx| main_window.read(cx).selected_download_tasks.clone()),
            HashSet::from([other_task]),
            "secondary click replaces a selection that did not contain the clicked row"
        );

        let name = window
            .debug_bounds("download-task-name-selected")
            .expect("selected name column bounds");
        let name_text = window
            .debug_bounds("download-task-name-text-selected")
            .expect("selected file-name text bounds");
        let path_text = window
            .debug_bounds("download-task-path-text-selected")
            .expect("selected path text bounds");
        let progress = window
            .debug_bounds("download-task-progress-selected")
            .expect("selected progress column bounds");
        let status = window
            .debug_bounds("download-task-status-selected")
            .expect("selected status column bounds");
        let detail = window
            .debug_bounds("download-task-detail")
            .expect("production detail dock bounds");
        assert!(
            name.size.width.as_f32() >= 180.0,
            "the file-name column must retain enough width to render its content"
        );
        assert_eq!(name_text.size.width, name.size.width);
        assert_eq!(path_text.size.width, name.size.width);
        assert!(name.right() <= progress.left());
        assert!(progress.right() <= status.left());
        assert!(detail.size.width.as_f32() > 0.0);
        assert_eq!(detail.size.height.as_f32(), DOWNLOAD_DETAIL_HEIGHT);
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
                    file_name: Some("reopen.bin".into()),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
                    connection_limit: Some(1),
                    speed_limit_bytes_per_second: None,
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
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
    fn download_filter_rows_are_interactive_in_the_gpui_main_window() {
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
        assert!(downloads.contains(".id(filter.id())"));
        assert!(downloads.contains(".on_click(cx.listener("));
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
    }

    #[test]
    fn index_scope_settings_use_toggles_and_a_directory_picker_not_text_inputs() {
        let source = include_str!("main_window.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("production main-window source");

        assert!(!production.contains("index_volumes_input"));
        assert!(!production.contains("index_exclusions_input"));
        assert!(production.contains("quick-launch-index-all-volumes"));
        assert!(production.contains("quick-launch-add-index-exclusion"));
        assert!(production.contains("request_index_exclusion_directory"));
    }

    #[test]
    fn quick_launch_settings_scroll_as_one_viewport() {
        let source = include_str!("main_window.rs");
        let quick_find = source
            .split("fn render_quick_find")
            .nth(1)
            .and_then(|source| source.split("fn render_clipboard").next())
            .expect("quick-launch settings source");

        assert!(quick_find.contains("quick-launch-settings-scroll"));
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
        let quick_find = source
            .split("fn render_quick_find")
            .nth(1)
            .and_then(|source| source.split("fn render_clipboard").next())
            .expect("quick-launch settings source");

        assert!(quick_find.contains("quick-launch-pick-command-program"));
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
