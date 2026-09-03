use super::*;
use crate::ui::metrics::{DETAIL_TAB_HEIGHT, ICON_GLYPH_SMALL, PROGRESS_TRACK_HEIGHT};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum DownloadTaskFilter {
    #[default]
    All,
    Downloading,
    Waiting,
    Paused,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum DownloadDetailTab {
    #[default]
    Overview,
    Segments,
    Headers,
    Verification,
    Timeline,
}

impl DownloadDetailTab {
    pub(crate) const ALL: [Self; 5] = [
        Self::Overview,
        Self::Segments,
        Self::Headers,
        Self::Verification,
        Self::Timeline,
    ];

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Overview => "概览",
            Self::Segments => "分段",
            Self::Headers => "Header",
            Self::Verification => "校验",
            Self::Timeline => "日志",
        }
    }

    pub(crate) const fn diagnostic_name(self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Segments => "segments",
            Self::Headers => "headers",
            Self::Verification => "verification",
            Self::Timeline => "timeline",
        }
    }
}

pub(crate) struct DownloadRowSelection {
    pub(crate) selected: bool,
    pub(crate) visible_task_ids: Arc<Vec<DownloadTaskId>>,
}

#[derive(Debug, PartialEq)]
pub(crate) struct DownloadRowTransferPresentation {
    pub(crate) fraction: f32,
    pub(crate) transferred: String,
    pub(crate) speed: String,
    pub(crate) remaining: String,
}

pub(crate) fn download_row_transfer_presentation(
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
pub(crate) struct DownloadContextCapabilities {
    pub(crate) resume_targets: Vec<DownloadTaskId>,
    pub(crate) pause_targets: Vec<DownloadTaskId>,
    pub(crate) cancel_targets: Vec<DownloadTaskId>,
    pub(crate) can_move_queue: bool,
    pub(crate) queue_top: bool,
    pub(crate) queue_up: bool,
    pub(crate) queue_down: bool,
    pub(crate) queue_bottom: bool,
    pub(crate) can_remove: bool,
    pub(crate) single_artifact: Option<DownloadTaskId>,
    /// True when every resume target has failed, so the resume action reads
    /// as an explicit retry in the menu.
    pub(crate) resume_all_failed: bool,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct DownloadContextMenuPlan {
    pub(crate) artifact: bool,
    pub(crate) copy_url: bool,
    pub(crate) task_window: bool,
    pub(crate) resume: bool,
    pub(crate) pause: bool,
    pub(crate) cancel: bool,
    pub(crate) queue_top: bool,
    pub(crate) queue_up: bool,
    pub(crate) queue_down: bool,
    pub(crate) queue_bottom: bool,
    pub(crate) remove: bool,
}

pub(crate) struct DownloadDetailView<'a> {
    pub(crate) task: &'a DownloadEngineTask,
    pub(crate) detail: Option<&'a DownloadTaskDetail>,
    pub(crate) detail_error: Option<&'a str>,
    pub(crate) progress: Option<&'a DownloadProgress>,
    pub(crate) active_tab: DownloadDetailTab,
    pub(crate) scroll: ScrollHandle,
    pub(crate) main_window: Entity<MainWindow>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum DownloadSortKey {
    #[default]
    Queue,
    Name,
    Size,
    Progress,
    Status,
    Created,
}

impl DownloadSortKey {
    pub(crate) const ALL: [Self; 6] = [
        Self::Queue,
        Self::Name,
        Self::Size,
        Self::Progress,
        Self::Status,
        Self::Created,
    ];

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Queue => "队列顺序",
            Self::Name => "名称",
            Self::Size => "大小",
            Self::Progress => "进度",
            Self::Status => "状态",
            Self::Created => "添加时间",
        }
    }

    pub(crate) const fn diagnostic_name(self) -> &'static str {
        match self {
            Self::Queue => "queue",
            Self::Name => "name",
            Self::Size => "size",
            Self::Progress => "progress",
            Self::Status => "status",
            Self::Created => "created",
        }
    }

    pub(crate) fn from_diagnostic_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|key| key.diagnostic_name() == name)
    }

    pub(crate) const fn default_descending(self) -> bool {
        matches!(self, Self::Size | Self::Progress | Self::Created)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct DownloadSortState {
    pub(crate) key: DownloadSortKey,
    pub(crate) descending: bool,
}

pub(crate) fn download_task_name(task: &DownloadEngineTask) -> &str {
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

pub(crate) fn download_failure_summary(error: &DownloadFault) -> &'static str {
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

pub(crate) fn download_task_matches_search(task: &DownloadEngineTask, query: &str) -> bool {
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

pub(crate) fn download_status_sort_rank(status: DownloadStatus) -> u8 {
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

pub(crate) fn download_tasks_for_view(
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

pub(crate) fn set_download_sort(
    state: &mut DownloadSortState,
    key: DownloadSortKey,
    toggle: bool,
) -> bool {
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
    pub(crate) const ALL: [Self; 6] = [
        Self::All,
        Self::Downloading,
        Self::Waiting,
        Self::Paused,
        Self::Completed,
        Self::Failed,
    ];

    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::All => "download-filter-all",
            Self::Downloading => "download-filter-downloading",
            Self::Waiting => "download-filter-waiting",
            Self::Paused => "download-filter-paused",
            Self::Completed => "download-filter-completed",
            Self::Failed => "download-filter-failed",
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::All => "全部",
            Self::Downloading => "下载中",
            Self::Waiting => "等待",
            Self::Paused => "暂停",
            Self::Completed => "已完成",
            Self::Failed => "失败",
        }
    }

    pub(crate) const fn diagnostic_name(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Downloading => "downloading",
            Self::Waiting => "waiting",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    pub(crate) const fn empty_title(self) -> &'static str {
        match self {
            Self::All => "还没有下载任务",
            Self::Downloading => "暂无正在下载的任务",
            Self::Waiting => "暂无等待任务",
            Self::Paused => "暂无暂停任务",
            Self::Completed => "暂无已完成任务",
            Self::Failed => "暂无失败任务",
        }
    }

    pub(crate) const fn matches(self, status: DownloadStatus) -> bool {
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
pub(crate) struct DownloadFilterCounts {
    pub(crate) all: usize,
    pub(crate) downloading: usize,
    pub(crate) waiting: usize,
    pub(crate) paused: usize,
    pub(crate) completed: usize,
    pub(crate) failed: usize,
}

impl DownloadFilterCounts {
    pub(crate) const fn get(&self, filter: DownloadTaskFilter) -> usize {
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

pub(crate) fn download_filter_counts(
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

pub(crate) fn reconcile_download_selection<T, Id: Copy + Eq + Hash>(
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

pub(crate) fn select_download_filter(
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

pub(crate) fn update_download_selection(
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

pub(crate) fn prepare_download_context_selection(
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

pub(crate) fn download_context_capabilities(
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
        resume_all_failed: !selected_tasks.is_empty()
            && selected_tasks.iter().all(|task| {
                matches!(task.task.status(), DownloadStatus::Failed)
                    || !matches!(
                        task.task.status(),
                        DownloadStatus::Created | DownloadStatus::Paused | DownloadStatus::Failed
                    )
            })
            && selected_tasks
                .iter()
                .any(|task| task.task.status() == DownloadStatus::Failed),
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

pub(crate) fn download_queue_placement_capabilities(
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

pub(crate) fn download_context_menu_plan(
    capabilities: &DownloadContextCapabilities,
    selected_count: usize,
) -> DownloadContextMenuPlan {
    let artifact = capabilities.single_artifact.is_some();
    DownloadContextMenuPlan {
        artifact,
        copy_url: selected_count > 0,
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

impl MainWindow {
    pub(crate) fn render_downloads(&mut self, cx: &mut Context<Self>) -> AnyElement {
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
                            .size(px(ICON_GLYPH_SMALL))
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
                                                                            // Persist the
                                                                            // preference for the
                                                                            // next launch.
                                                                            let sort_application =
                                                                                window
                                                                                    .downloads
                                                                                    .clone();
                                                                            let sort_key = window
                                                                                .download_sort
                                                                                .key
                                                                                .diagnostic_name()
                                                                                .to_owned();
                                                                            let sort_descending =
                                                                                window
                                                                                    .download_sort
                                                                                    .descending;
                                                                            cx.defer(move |cx| {
                                                                                let _ =
                                                                                    sort_application
                                                                                        .update(
                                                                                            cx,
                                                                                            |application,
                                                                                             cx| {
                                                                                                application
                                                                                                    .set_task_list_sort(
                                                                                                        Some(sort_key),
                                                                                                        Some(sort_descending),
                                                                                                        cx,
                                                                                                    );
                                                                                            },
                                                                                        );
                                                                            });
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
                            // Clickable guidance: when the application entity
                            // is reachable, the whole empty state opens the
                            // new-download dialog; detached render paths keep
                            // the static empty state.
                            let empty = empty_state(
                                theme,
                                "download-empty-state",
                                MaterialIcon::Download,
                                "还没有下载任务",
                                "点击右上角新建下载，提交真实 HTTP 或 HTTPS 地址；也可以从浏览器直接发起下载。",
                            );
                            match self.downloads.upgrade() {
                                Some(open_application) => div()
                                    .id("download-empty-state-action")
                                    .size_full()
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        let application = open_application.clone();
                                        cx.defer(move |cx| {
                                            application.update(cx, |application, cx| {
                                                application.open_new_download_window(cx);
                                            });
                                        });
                                        let _ = this;
                                    }))
                                    .child(empty)
                                    .into_any_element(),
                                None => empty.into_any_element(),
                            }
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
}

pub(crate) fn download_task_row(
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
                        match application.row_double_click_action() {
                            nxtools_core::DownloadRowDoubleClickActionSettings::OpenFile => {
                                application.open_task_artifact(task_id, ShellOpenAction::Open, cx);
                            }
                            nxtools_core::DownloadRowDoubleClickActionSettings::OpenDirectory => {
                                application.open_task_artifact(
                                    task_id,
                                    ShellOpenAction::SelectInExplorer,
                                    cx,
                                );
                            }
                            nxtools_core::DownloadRowDoubleClickActionSettings::OpenTaskWindow => {
                                application.request_task_window(task_id, cx);
                            }
                        }
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
                                .rounded(px(RADIUS_CONTROL))
                                .bg(theme.surface_raised)
                                .child(
                                    div()
                                        .w(px(DOWNLOAD_PROGRESS_COLUMN_WIDTH * transfer.fraction))
                                        .h_full()
                                        .rounded(px(RADIUS_CONTROL))
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

pub(crate) fn download_popup_item(
    theme: Theme,
    selector: &'static str,
    label: &'static str,
    icon: MaterialIcon,
    tone: MenuTone,
) -> PopupMenuItem {
    context_menu_item(theme, selector, label, icon, tone)
}

pub(crate) fn download_task_context_menu(
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
    if plan.copy_url {
        let copy_application = application.clone();
        let copy_task_ids = task_ids.clone();
        menu = menu.item(
            download_popup_item(
                theme,
                "download-menu-copy-url",
                "复制下载链接",
                MaterialIcon::ContentCopy,
                MenuTone::Normal,
            )
            .on_click(move |_event, _window, cx| {
                let application = copy_application.clone();
                let task_ids = copy_task_ids.clone();
                cx.defer(move |cx| {
                    let _ = application.update(cx, |application, cx| {
                        application.copy_task_urls(&task_ids, cx);
                    });
                });
            }),
        );
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
            // When every resume target has failed, the same action reads as
            // an explicit retry.
            menu = menu.item(batch_item(
                "download-menu-resume",
                if capabilities.resume_all_failed {
                    "重试"
                } else {
                    "开始 / 继续"
                },
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

pub(crate) fn download_batch_toolbar(
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

pub(crate) fn download_task_detail(theme: Theme, view: DownloadDetailView<'_>) -> AnyElement {
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
                .h(px(DETAIL_TAB_HEIGHT))
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

pub(crate) fn download_overview_detail(
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

pub(crate) fn download_segment_detail(
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
        .h(px(PROGRESS_TRACK_HEIGHT))
        .w_full()
        .rounded(px(RADIUS_CONTROL))
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
                .h(px(BADGE_HEIGHT))
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

pub(crate) fn download_header_detail(theme: Theme, task: &DownloadEngineTask) -> AnyElement {
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

pub(crate) fn download_verification_detail(
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
        .gap(px(PROGRESS_TRACK_HEIGHT))
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

pub(crate) fn download_timeline_detail(
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

pub(crate) fn download_detail_section(
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

pub(crate) fn download_detail_field(
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

pub(crate) fn download_detail_line(theme: Theme, label: &'static str, value: String) -> AnyElement {
    download_detail_line_with_color(theme, label, value, theme.text)
}

pub(crate) fn download_detail_line_owned(theme: Theme, label: String, value: String) -> AnyElement {
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

pub(crate) fn download_detail_line_with_color(
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

pub(crate) fn masked_request_header_value(name: &str, value: &str) -> String {
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

pub(crate) fn download_origin_label(origin: RequestOrigin) -> &'static str {
    match origin {
        RequestOrigin::Ui => "界面",
        RequestOrigin::Cli => "CLI",
        RequestOrigin::Browser => "浏览器",
        RequestOrigin::Internal => "内部",
    }
}

pub(crate) fn download_segment_state_label(state: DownloadSegmentState) -> &'static str {
    match state {
        DownloadSegmentState::Waiting => "等待",
        DownloadSegmentState::Active => "下载中",
        DownloadSegmentState::Completed => "完成",
    }
}

pub(crate) fn download_segment_color(theme: Theme, state: DownloadSegmentState) -> gpui::Rgba {
    match state {
        DownloadSegmentState::Waiting => theme.text_muted,
        DownloadSegmentState::Active => theme.accent,
        DownloadSegmentState::Completed => theme.success,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DownloadSegmentStripCell {
    pub(crate) segment_index: u32,
    pub(crate) state: DownloadSegmentState,
    pub(crate) downloaded: bool,
}

pub(crate) fn download_segment_strip_cells(
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

pub(crate) fn download_timeline_label(entry: &DownloadTaskTimelineEntry) -> String {
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

pub(crate) fn download_status_log_label(status: DownloadStatus) -> &'static str {
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

pub(crate) fn format_download_timestamp(timestamp: i64) -> String {
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

pub(crate) fn format_download_created_compact(timestamp: i64) -> String {
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

pub(crate) fn format_download_log_time(timestamp: i64) -> String {
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

pub(crate) fn format_duration(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

pub(crate) fn download_status_presentation(
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

pub(crate) fn format_download_bytes(bytes: u64) -> String {
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

pub(crate) fn format_download_transfer(downloaded: u64, total: u64) -> String {
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

pub(crate) fn format_download_rate(bytes_per_second: u64) -> String {
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

pub(crate) fn format_download_remaining(seconds: u64) -> String {
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

pub(crate) fn format_compact_transfer_value(value: f64) -> String {
    if value >= 100.0 || value.fract().abs() < 0.05 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

pub(crate) fn download_table_header(
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

pub(crate) fn download_sort_header_cell(
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
