//! Stateless presentation support for the download windows: settings-window
//! geometry, completed-window actions, window chrome, status labels and the
//! taskbar progress/overlay command builders. Everything here is a pure
//! function of its arguments; stateful task-window management stays in the
//! parent module.
use super::*;

pub(crate) const DOWNLOAD_SETTINGS_WINDOW_WIDTH: f32 = 984.0;
pub(crate) const DOWNLOAD_SETTINGS_WINDOW_HEIGHT: f32 = 650.0;
pub(crate) const DOWNLOAD_REQUEST_HEADER_EDITOR_HEIGHT: f32 = 120.0;

pub(crate) fn download_settings_window_options(cx: &App) -> WindowOptions {
    // Match the reference settings workspace: a stable navigation rail plus a content column
    // wide enough for two compact settings rows without relying on child min-width to size it.
    let window_size = size(
        px(DOWNLOAD_SETTINGS_WINDOW_WIDTH),
        px(DOWNLOAD_SETTINGS_WINDOW_HEIGHT),
    );
    WindowOptions {
        window_bounds: Some(WindowBounds::centered(window_size, cx)),
        titlebar: None,
        focus: true,
        show: true,
        kind: WindowKind::Normal,
        is_movable: true,
        is_resizable: false,
        is_minimizable: false,
        display_id: None,
        window_background: WindowBackgroundAppearance::Opaque,
        app_id: Some("NxTools.DownloadSettings".into()),
        window_min_size: Some(window_size),
        window_decorations: Some(WindowDecorations::Client),
        tabbing_identifier: None,
    }
}

pub(crate) fn task_window_activation_command(window_handle: i64) -> Option<PlatformCommand> {
    (window_handle != 0).then_some(PlatformCommand::ActivateWindow(ActivateWindowCommand {
        window_handle,
    }))
}

pub(crate) fn replacement_targets_current_window(
    current: Option<TaskWindowIdentity>,
    stale: TaskWindowIdentity,
) -> bool {
    current == Some(stale)
}

pub(crate) fn clear_pending_task_window_work(
    task_id: DownloadTaskId,
    activations: &mut HashMap<uuid::Uuid, PendingTaskWindowActivation>,
    replacements: &mut HashMap<DownloadTaskId, TaskWindowIdentity>,
) {
    activations.retain(|_, pending| pending.task_id != task_id);
    replacements.remove(&task_id);
}

pub(crate) fn download_window_phase(status: DownloadStatus) -> DownloadWindowPhase {
    match status {
        DownloadStatus::Completed => DownloadWindowPhase::Completed,
        DownloadStatus::Failed => DownloadWindowPhase::Failed,
        _ => DownloadWindowPhase::Progress,
    }
}

pub(crate) fn raise_terminal_window_without_activation(
    platform: &PlatformBridge,
    window_handle: i64,
) {
    if window_handle == 0 {
        return;
    }
    let _ = platform.send_command(PlatformCommand::SetWindowTopmost(SetWindowTopmostCommand {
        window_handle,
        enabled: true,
    }));
}

pub(crate) fn load_download_snapshot(engine: &DownloadEngine) -> Result<DownloadSnapshot, String> {
    let result = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Ui,
        command: DownloadCommand::Snapshot {
            query: DownloadQuery::compact(),
        },
    });
    match result.outcome {
        DownloadCommandOutcome::Snapshot(snapshot) => Ok(snapshot),
        DownloadCommandOutcome::Rejected { fault } => Err(fault.message),
        _ => Err("下载引擎返回了无效的快照结果".into()),
    }
}

pub(crate) fn download_snapshot_tasks(
    snapshot: &DownloadSnapshot,
) -> Result<Vec<DownloadEngineTask>, String> {
    if !snapshot.task_summaries.is_empty() {
        return snapshot
            .task_summaries
            .iter()
            .cloned()
            .map(|summary| {
                summary
                    .into_engine_task()
                    .map_err(|error| error.to_string())
            })
            .collect();
    }
    Ok(snapshot.task_details.clone())
}

pub(crate) fn load_download_task_detail(
    engine: &DownloadEngine,
    task_id: DownloadTaskId,
) -> Result<DownloadTaskDetail, String> {
    let result = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Ui,
        command: DownloadCommand::TaskDetail { task_id },
    });
    match result.outcome {
        DownloadCommandOutcome::TaskDetail(detail) => Ok(*detail),
        DownloadCommandOutcome::Rejected { fault } => Err(fault.message),
        _ => Err("下载引擎返回了无效的任务详情结果".into()),
    }
}

pub(crate) fn apply_progress_to_task_detail(
    detail: &mut DownloadTaskDetail,
    progress: &DownloadProgress,
) {
    detail.task.downloaded_bytes = progress.downloaded_bytes;
    detail.task.total_bytes = progress.total_bytes;
    for segment_progress in &progress.segments {
        let Some(position) = detail
            .segments
            .iter()
            .position(|segment| segment.index() == segment_progress.segment_index)
        else {
            continue;
        };
        let segment = &detail.segments[position];
        let downloaded_bytes = segment_progress.downloaded_bytes.min(segment.len());
        let state = if downloaded_bytes == segment.len() {
            DownloadSegmentState::Completed
        } else if downloaded_bytes > segment.downloaded_bytes() {
            DownloadSegmentState::Active
        } else {
            segment.state
        };
        if let Ok(updated) = DownloadSegment::new(
            segment.index(),
            segment.start_byte(),
            segment.end_byte(),
            downloaded_bytes,
            state,
        ) {
            detail.segments[position] = updated;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CompletedTaskWindowAction {
    Drag,
    Reveal,
    Open,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CompletedTaskWindowEmphasis {
    Primary,
    Outline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CompletedTaskWindowActionSpec {
    pub(crate) button_id: &'static str,
    pub(crate) role_selector: &'static str,
    pub(crate) icon: MaterialIcon,
    pub(crate) label: Option<&'static str>,
    pub(crate) emphasis: CompletedTaskWindowEmphasis,
    pub(crate) tooltip: Option<&'static str>,
    pub(crate) tooltip_selector: Option<&'static str>,
}

impl CompletedTaskWindowAction {
    #[cfg(test)]
    pub(crate) const ALL: [Self; 3] = [Self::Drag, Self::Reveal, Self::Open];

    pub(crate) fn spec(self) -> CompletedTaskWindowActionSpec {
        match self {
            Self::Drag => CompletedTaskWindowActionSpec {
                button_id: "download-complete-drag",
                role_selector: "download-complete-secondary-drag-action",
                icon: MaterialIcon::OpenInFull,
                label: None,
                emphasis: CompletedTaskWindowEmphasis::Outline,
                tooltip: Some("拖拽文件"),
                tooltip_selector: Some("download-complete-drag-tooltip"),
            },
            Self::Reveal => CompletedTaskWindowActionSpec {
                button_id: "download-complete-reveal",
                role_selector: "download-complete-secondary-reveal-action",
                icon: MaterialIcon::Folder,
                label: Some("打开所在文件夹"),
                emphasis: CompletedTaskWindowEmphasis::Outline,
                tooltip: None,
                tooltip_selector: None,
            },
            Self::Open => CompletedTaskWindowActionSpec {
                button_id: "download-complete-open",
                role_selector: "download-complete-primary-open-action",
                icon: MaterialIcon::OpenInNew,
                label: Some("打开文件"),
                emphasis: CompletedTaskWindowEmphasis::Primary,
                tooltip: None,
                tooltip_selector: None,
            },
        }
    }
}

pub(crate) fn completed_task_window_action(
    action: CompletedTaskWindowAction,
    theme: Theme,
    cx: &App,
) -> Button {
    let spec = action.spec();
    let button = match spec.label {
        Some(label) => {
            action_button(spec.button_id, label).icon(Icon::new(spec.icon).size(px(14.0)))
        }
        None => icon_action_button(spec.button_id, spec.icon),
    };
    let button = match spec.emphasis {
        CompletedTaskWindowEmphasis::Primary => button.custom(primary_button_variant(theme, cx)),
        CompletedTaskWindowEmphasis::Outline => button.outline(),
    };
    button.debug_selector(move || spec.role_selector.into())
}

pub(crate) fn download_window_metric(
    theme: Theme,
    label: &'static str,
    value: String,
) -> impl IntoElement {
    div()
        .min_w_0()
        .flex_1()
        .flex()
        .flex_col()
        .gap(px(0.0))
        .child(
            div()
                .text_size(px(TEXT_LABEL))
                .line_height(px(LINE_HEIGHT_LABEL))
                .text_color(theme.text_secondary)
                .child(label),
        )
        .child(
            div()
                .text_size(px(TEXT_LABEL))
                .line_height(px(LINE_HEIGHT_LABEL))
                .font_weight(FontWeight::MEDIUM)
                .truncate()
                .child(value),
        )
}

pub(crate) fn download_window_titlebar(theme: Theme, title: &'static str) -> impl IntoElement {
    div()
        .h(px(34.0))
        .pl(px(SPACING_16))
        .border_b_1()
        .border_color(theme.border_subtle)
        .bg(theme.surface)
        .window_control_area(WindowControlArea::Drag)
        .flex()
        .items_center()
        .child(
            div()
                .flex_1()
                .text_size(px(TEXT_BODY))
                .font_weight(FontWeight::SEMIBOLD)
                .child(title),
        )
        .child(download_window_close_button(theme))
}

pub(crate) fn download_settings_titlebar(
    theme: Theme,
    application: Entity<DownloadApplication>,
    editing: bool,
) -> impl IntoElement {
    client_titlebar(
        theme,
        "download-settings-titlebar",
        "download-settings-close",
        "下载设置",
        move |_, window, cx| {
            if editing {
                let application = application.clone();
                let correlation_id = uuid::Uuid::new_v4();
                cx.defer(move |cx| {
                    application.update(cx, |application, cx| {
                        application.finish_taskbar_overlay_edit(false, correlation_id, cx);
                    });
                });
            }
            window.remove_window();
        },
    )
}

pub(crate) fn download_primary_button_variant(theme: Theme, cx: &gpui::App) -> ButtonCustomVariant {
    crate::ui::components::action::primary_button_variant(theme, cx)
}

pub(crate) fn download_window_close_button(theme: Theme) -> impl IntoElement {
    icon_action("download-window-close", MaterialIcon::Close, "关闭下载窗口")
        .danger()
        .ghost()
        .w(px(40.0))
        .text_color(theme.text_secondary)
        .h(px(32.0))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(|_, window, _| window.remove_window())
}

pub(crate) fn download_task_status(status: DownloadStatus) -> (&'static str, gpui::Rgba) {
    let theme = Theme::zed_dark();
    match status {
        DownloadStatus::Created => ("等待开始", theme.text_muted),
        DownloadStatus::Probing => ("正在探测", theme.accent),
        DownloadStatus::Waiting => ("等待下载", theme.accent),
        DownloadStatus::Downloading => ("正在下载", theme.accent),
        DownloadStatus::Paused => ("已暂停", theme.text_secondary),
        DownloadStatus::Finalizing => ("正在落盘", theme.accent),
        DownloadStatus::Verifying => ("正在校验", theme.accent),
        DownloadStatus::Completed => ("下载完成", theme.success),
        DownloadStatus::Failed => ("下载失败", theme.danger),
        DownloadStatus::Cancelled => ("已取消", theme.text_muted),
    }
}

pub(crate) fn format_download_eta(seconds: u64) -> String {
    if seconds >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{:02}:{:02}", seconds / 60, seconds % 60)
    }
}

pub(crate) fn taskbar_progress_command(
    window_handle: i64,
    task: &DownloadEngineTask,
    progress: Option<&DownloadProgress>,
) -> SetTaskbarProgressCommand {
    let downloaded = progress.map_or(task.downloaded_bytes, |progress| progress.downloaded_bytes);
    let total = progress
        .and_then(|progress| progress.total_bytes)
        .or(task.total_bytes);
    let status = task.task.status();
    let state = match status {
        DownloadStatus::Created | DownloadStatus::Probing | DownloadStatus::Waiting
            if total.is_none() =>
        {
            TaskbarProgressState::Indeterminate
        }
        DownloadStatus::Downloading
        | DownloadStatus::Finalizing
        | DownloadStatus::Verifying
        | DownloadStatus::Probing
        | DownloadStatus::Waiting => {
            if total.is_some() {
                TaskbarProgressState::Normal
            } else {
                TaskbarProgressState::Indeterminate
            }
        }
        DownloadStatus::Paused => TaskbarProgressState::Paused,
        DownloadStatus::Failed => TaskbarProgressState::Error,
        DownloadStatus::Completed | DownloadStatus::Cancelled => TaskbarProgressState::None,
        DownloadStatus::Created => TaskbarProgressState::Indeterminate,
    };
    let (completed, total) = match state {
        TaskbarProgressState::Normal | TaskbarProgressState::Paused => {
            let total = total.unwrap_or(1).max(1);
            (downloaded.min(total), total)
        }
        TaskbarProgressState::Error => {
            let total = total.unwrap_or(1).max(1);
            (downloaded.min(total).max(1), total)
        }
        TaskbarProgressState::None | TaskbarProgressState::Indeterminate => (0, 0),
    };
    SetTaskbarProgressCommand {
        window_handle,
        state,
        completed,
        total,
    }
}

pub(crate) fn taskbar_overlay_placement_command(
    window_handle: i64,
    settings: &DownloadTaskbarProgressOverlaySettings,
    visible: bool,
    interactive: bool,
) -> PlaceTaskbarProgressOverlayCommand {
    PlaceTaskbarProgressOverlayCommand {
        window_handle,
        enabled: settings.enabled,
        visible,
        anchor: match settings.anchor {
            DownloadTaskbarOverlayAnchor::Left => TaskbarOverlayAnchor::Left,
            DownloadTaskbarOverlayAnchor::Right => TaskbarOverlayAnchor::Right,
        },
        offset_x: settings.offset_x,
        offset_y: settings.offset_y,
        width: i32::try_from(settings.width).unwrap_or(640),
        height: i32::try_from(settings.height).unwrap_or(72),
        interactive,
    }
}
