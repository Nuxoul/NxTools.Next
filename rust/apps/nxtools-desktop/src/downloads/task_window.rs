use super::progress_display::ProgressDisplaySampler;
use super::*;
use crate::ui::components::tooltip::bounded_text_tooltip;
use crate::ui::metrics::ordinary_rem;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::description_list::DescriptionList;
use gpui_kit::component::dialog::DialogFooter;
use gpui_kit::component::progress::Progress;
use gpui_kit::component::table::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use gpui_kit::component::toolbar::{Toolbar, ToolbarGroup};
#[path = "task_window_details.rs"]
mod details;
pub(crate) struct DownloadTaskWindowPhaseLayout {
    pub(crate) size: gpui_kit::Size<gpui_kit::Pixels>,
    pub(crate) selector: &'static str,
}

pub(crate) fn download_task_window_phase_layout(
    phase: DownloadWindowPhase,
) -> DownloadTaskWindowPhaseLayout {
    match phase {
        DownloadWindowPhase::Progress => DownloadTaskWindowPhaseLayout {
            size: size(px(440.0), px(184.0)),
            selector: "download-task-window-phase-progress",
        },
        DownloadWindowPhase::Failed => DownloadTaskWindowPhaseLayout {
            size: size(px(440.0), px(196.0)),
            selector: "download-task-window-phase-failed",
        },
        DownloadWindowPhase::Completed => DownloadTaskWindowPhaseLayout {
            size: size(px(440.0), px(184.0)),
            selector: "download-task-window-phase-completed",
        },
    }
}

pub(crate) fn task_window_minimum_size() -> gpui_kit::Size<gpui_kit::Pixels> {
    // Every phase can return to collapsed progress; never lock an initial
    // failed/expanded window to its taller starting size.
    download_task_window_size(DownloadWindowPhase::Progress)
}
pub(crate) fn download_task_window_size(
    phase: DownloadWindowPhase,
) -> gpui_kit::Size<gpui_kit::Pixels> {
    download_task_window_phase_layout(phase).size
}

pub(crate) fn download_task_window_size_for_details(
    phase: DownloadWindowPhase,
    details_expanded: bool,
) -> gpui_kit::Size<gpui_kit::Pixels> {
    if phase == DownloadWindowPhase::Progress {
        if details_expanded {
            size(px(440.0), px(TASK_WINDOW_PROGRESS_EXPANDED_HEIGHT))
        } else {
            size(px(440.0), px(TASK_WINDOW_PROGRESS_HEIGHT))
        }
    } else {
        download_task_window_size(phase)
    }
}

pub(crate) fn details_expanded_after_phase_transition(
    previous_phase: Option<DownloadWindowPhase>,
    next_phase: DownloadWindowPhase,
    expanded: bool,
) -> bool {
    if matches!(
        (previous_phase, next_phase),
        (
            Some(DownloadWindowPhase::Failed | DownloadWindowPhase::Completed),
            DownloadWindowPhase::Progress
        )
    ) {
        false
    } else {
        expanded
    }
}

fn download_task_window_phase_content(
    phase_selector: &'static str,
    phase_key: u64,
    body: impl IntoElement,
) -> impl IntoElement {
    let animated = crate::ui::motion::animated_surface(
        ("download-task-window-phase", phase_key),
        crate::ui::motion::ENTER_DURATION,
        div().size_full().child(body).into_any_element(),
        3.0,
        0.94,
    );
    div()
        .debug_selector(move || phase_selector.into())
        .size_full()
        .child(animated)
}
pub(crate) fn download_task_window_shell(
    theme: Theme,
    phase: DownloadWindowPhase,
    title: &'static str,
    body: impl IntoElement,
    footer: impl IntoElement,
) -> impl IntoElement {
    let phase_key = match phase {
        DownloadWindowPhase::Progress => 1,
        DownloadWindowPhase::Failed => 2,
        DownloadWindowPhase::Completed => 3,
    };
    let phase_selector = download_task_window_phase_layout(phase).selector;
    div()
        .debug_selector(|| "download-task-window-shell".into())
        .size_full()
        .bg(theme.canvas)
        .text_color(theme.text)
        .flex()
        .flex_col()
        .child(
            div()
                .debug_selector(|| "download-task-window-titlebar".into())
                .h(px(34.0))
                .flex_none()
                .child(download_window_titlebar(theme, title)),
        )
        .child(
            div()
                .debug_selector(|| "download-task-window-body".into())
                .flex_1()
                .min_h_0()
                .child(download_task_window_phase_content(
                    phase_selector,
                    phase_key,
                    body,
                )),
        )
        .child(
            div()
                .debug_selector(|| "download-task-window-footer".into())
                .h(px(44.0))
                .flex_none()
                .flex()
                .items_center()
                .px(px(SPACING_16))
                .border_t_1()
                .border_color(theme.border_subtle)
                .bg(theme.surface)
                .child(footer),
        )
}

pub(crate) struct DownloadTaskWindowView {
    pub(crate) theme: Theme,
    pub(crate) application: Entity<DownloadApplication>,
    pub(crate) platform: Rc<PlatformBridge>,
    pub(crate) task_id: DownloadTaskId,
    pub(crate) native_window_handle: i64,
    pub(crate) window_generation: uuid::Uuid,
    pub(crate) snapshot: Option<DownloadEngineTask>,
    pub(crate) progress: Option<DownloadProgress>,
    pub(super) persisted_range_progress: Vec<nxtools_download::DownloadSegmentProgress>,
    pub(super) live_range_progress: Vec<nxtools_download::DownloadSegmentProgress>,
    pub(super) details_tab: usize,
    pub(crate) shutdown_armed: bool,
    pub(crate) completion_intention: nxtools_download::DownloadCompletionIntention,
    pub(crate) details_expanded: bool,
    pub(super) display_sampler: ProgressDisplaySampler,
    pub(super) display_started_at: std::time::Instant,
    pub(super) effective_global_speed_limit: Option<u64>,
    pub(super) speed_limit_kib: Option<Entity<InputState>>,
    pub(super) speed_limit_edit: super::speed_limit_edit::SpeedLimitEdit,
    pub(super) speed_limit_subscription: Option<Subscription>,
    pub(super) source_draft: Option<super::paused_url::PausedUrlDraft>,
    pub(super) source_window_lifetime: tokio::sync::watch::Sender<bool>,
}

impl DownloadTaskWindowView {
    pub(crate) fn new(
        application: Entity<DownloadApplication>,
        platform: Rc<PlatformBridge>,
        task_id: DownloadTaskId,
        identity: TaskWindowIdentity,
        snapshot: Option<DownloadEngineTask>,
        progress: Option<DownloadProgress>,
        shutdown_armed: bool,
    ) -> Self {
        let display_started_at = std::time::Instant::now();
        let mut display_sampler = ProgressDisplaySampler::default();
        observe_task_display(
            &mut display_sampler,
            std::time::Duration::ZERO,
            snapshot.as_ref(),
            progress.as_ref(),
        );
        let speed_limit = snapshot
            .as_ref()
            .and_then(|task| task.task.request().speed_limit_bytes_per_second);
        let live_range_progress = progress
            .as_ref()
            .map(|progress| progress.segments.clone())
            .unwrap_or_default();
        Self {
            theme: Theme::zed_dark(),
            application,
            platform,
            task_id,
            native_window_handle: identity.native_window_handle,
            window_generation: identity.generation,
            snapshot,
            progress,
            persisted_range_progress: Vec::new(),
            live_range_progress,
            details_tab: 0,
            shutdown_armed,
            completion_intention: Default::default(),
            details_expanded: false,
            display_sampler,
            display_started_at,
            effective_global_speed_limit: None,
            speed_limit_kib: None,
            speed_limit_edit: super::speed_limit_edit::SpeedLimitEdit::new(speed_limit),
            speed_limit_subscription: None,
            source_draft: None,
            source_window_lifetime: tokio::sync::watch::channel(false).0,
        }
    }

    pub(crate) fn replace_snapshot(
        &mut self,
        snapshot: Option<DownloadEngineTask>,
        progress: Option<DownloadProgress>,
        shutdown_armed: bool,
        cx: &mut Context<Self>,
    ) {
        let previous_phase = self
            .snapshot
            .as_ref()
            .map(|task| download_window_phase(task.task.status()));
        let next_phase = snapshot
            .as_ref()
            .map(|task| download_window_phase(task.task.status()));
        if let Some(next_phase) = next_phase {
            self.details_expanded = details_expanded_after_phase_transition(
                previous_phase,
                next_phase,
                self.details_expanded,
            );
        }
        let same_transfer =
            self.snapshot
                .as_ref()
                .zip(snapshot.as_ref())
                .is_some_and(|(old, next)| {
                    old.task.id() == next.task.id()
                        && old.task.completion_attempt() == next.task.completion_attempt()
                        && old.task.request().source_url == next.task.request().source_url
                        && old.task.status() == DownloadStatus::Downloading
                        && next.task.status() == DownloadStatus::Downloading
                });
        if !same_transfer {
            self.live_range_progress.clear();
        }
        if snapshot
            .as_ref()
            .is_some_and(|task| task.task.status() == DownloadStatus::Downloading)
            && let Some(progress) = progress
                .as_ref()
                .filter(|progress| !progress.segments.is_empty())
        {
            self.live_range_progress.clone_from(&progress.segments);
        }
        observe_task_display(
            &mut self.display_sampler,
            self.display_started_at.elapsed(),
            snapshot.as_ref(),
            progress.as_ref(),
        );
        self.snapshot = snapshot;
        self.progress = progress;
        self.shutdown_armed = shutdown_armed;
        cx.notify();
    }

    pub(super) fn range_segments(&self) -> &[nxtools_download::DownloadSegmentProgress] {
        let live = self
            .progress
            .as_ref()
            .map_or(&[][..], |progress| progress.segments.as_slice());
        if self
            .snapshot
            .as_ref()
            .is_some_and(|task| task.task.status() == DownloadStatus::Downloading)
        {
            if !live.is_empty() {
                return live;
            }
            if !self.live_range_progress.is_empty() {
                return &self.live_range_progress;
            }
        }
        if self.persisted_range_progress.is_empty() {
            live
        } else {
            &self.persisted_range_progress
        }
    }

    pub(crate) fn reset_details_expanded(&mut self) {
        self.details_expanded = false;
    }
}

impl Render for DownloadTaskWindowView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::ui::motion::observe_motion(window, cx);
        crate::theme::observe_theme(window, cx);
        self.theme = Theme::active(cx);
        let theme = self.theme;
        self.ensure_speed_limit_input(window, cx);
        let application = self.application.clone();
        let task_id = self.task_id;
        let source_window_handle = self.native_window_handle;
        let snapshot = self.snapshot.clone();
        let progress_event = self.progress.clone();
        let Some(task) = snapshot else {
            return div()
                .size_full()
                .bg(theme.canvas)
                .text_color(theme.text)
                .flex()
                .flex_col()
                .child(download_window_titlebar(theme, "下载任务"))
                .child(crate::ui::components::state::empty_state(
                    theme,
                    "download-task-removed-state",
                    MaterialIcon::Download,
                    "任务已被移除",
                    "",
                ))
                .into_any_element();
        };

        let status = task.task.status();
        let name = task
            .final_path
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
            .to_owned();
        let final_path = task.final_path.as_ref().map_or_else(
            || task.task.request().source_url.clone(),
            |path| path.display().to_string(),
        );
        let destination = task.final_path.as_ref().map_or_else(
            || task.task.request().save_directory.display().to_string(),
            |path| path.display().to_string(),
        );

        if status == DownloadStatus::Completed {
            let task_window_identity = TaskWindowIdentity {
                native_window_handle: source_window_handle,
                generation: self.window_generation,
            };
            let open_application = application.clone();
            let reveal_application = application.clone();
            let open_with_application = application.clone();
            let drag_application = application.clone();
            let drag_spec = CompletedTaskWindowAction::Drag.spec();
            return download_task_window_shell(
                theme,
                DownloadWindowPhase::Completed,
                "下载完成",
                div()
                    .id("download-complete-content-scroll")
                    .size_full()
                    .overflow_y_scroll()
                    .px_4()
                    .py_2()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .size(ordinary_rem(34.0))
                            .flex_none()
                            .rounded(px(RADIUS_FLOATING))
                            .bg(theme.accent_soft)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                Icon::new(MaterialIcon::Check)
                                    .size(ordinary_rem(18.0))
                                    .text_color(theme.success),
                            ),
                    )
                    .child(
                        div()
                            .debug_selector(|| "download-complete-summary".into())
                            .min_w_0()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(task_window_filename(
                                "download-complete-name",
                                name.clone(),
                                theme,
                                2,
                            ))
                            .child(
                                div()
                                    .debug_selector(|| "download-complete-size".into())
                                    .text_size(ordinary_rem(TEXT_LABEL))
                                    .line_height(ordinary_rem(LINE_HEIGHT_LABEL))
                                    .text_color(theme.success)
                                    .child(
                                        task.total_bytes.map(format_download_size).unwrap_or_else(
                                            || format_download_size(task.downloaded_bytes),
                                        ),
                                    ),
                            )
                            .child(
                                div()
                                    .id("download-complete-path")
                                    .debug_selector(|| "download-complete-path".into())
                                    .min_w_0()
                                    .text_size(ordinary_rem(TEXT_LABEL))
                                    .line_height(ordinary_rem(LINE_HEIGHT_LABEL))
                                    .text_color(theme.text_secondary)
                                    .truncate()
                                    .aria_label(final_path.clone())
                                    .tooltip(bounded_text_tooltip(
                                        "download-complete-path-tooltip",
                                        final_path.clone(),
                                        true,
                                    ))
                                    .child(final_path),
                            ),
                    ),
                DialogFooter::new()
                    .size_full()
                    .items_center()
                    .child(icon_action_tooltip_surface(
                        completed_task_window_action(CompletedTaskWindowAction::Drag, theme, cx)
                            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                drag_application.update(cx, |application, cx| {
                                    application.begin_task_file_drag(
                                        task_id,
                                        source_window_handle,
                                        cx,
                                    );
                                });
                                cx.stop_propagation();
                            }),
                        drag_spec.tooltip.expect("completed drag action tooltip"),
                        drag_spec
                            .tooltip_selector
                            .expect("completed drag action tooltip selector"),
                    ))
                    .child(
                        completed_task_window_action(CompletedTaskWindowAction::Reveal, theme, cx)
                            .on_click(move |_, _, cx| {
                                let application = reveal_application.clone();
                                let window = task_window_identity;
                                cx.defer(move |cx| {
                                    application.update(cx, |application, cx| {
                                        application.open_task_artifact_from_task_window(
                                            task_id,
                                            ShellOpenAction::SelectInExplorer,
                                            window,
                                            cx,
                                        );
                                    });
                                });
                            }),
                    )
                    .child(
                        completed_task_window_action(CompletedTaskWindowAction::Open, theme, cx)
                            .on_click(move |_, _, cx| {
                                let application = open_application.clone();
                                let window = task_window_identity;
                                cx.defer(move |cx| {
                                    application.update(cx, |application, cx| {
                                        application.open_task_artifact_from_task_window(
                                            task_id,
                                            ShellOpenAction::Open,
                                            window,
                                            cx,
                                        );
                                    });
                                });
                            }),
                    )
                    .child(
                        crate::ui::components::action::icon_action(
                            "download-complete-open-with-action",
                            MaterialIcon::ExpandMore,
                            "选择应用打开下载文件",
                        )
                        .outline()
                        .on_click(move |_, _, cx| {
                            let application = open_with_application.clone();
                            cx.defer(move |cx| {
                                application.update(cx, |application, cx| {
                                    application.open_task_artifact_from_task_window(
                                        task_id,
                                        ShellOpenAction::OpenWith,
                                        task_window_identity,
                                        cx,
                                    );
                                })
                            });
                        }),
                    ),
            )
            .into_any_element();
        }

        if status == DownloadStatus::Failed {
            let cancel_application = application.clone();
            let primary_application = application.clone();
            let redownload_application = application.clone();
            let fault = task.error.as_ref();
            use nxtools_download::DownloadFaultCode;
            #[derive(Clone, Copy, PartialEq, Eq)]
            enum Recovery {
                Rename,
                Browser,
                Restart,
                Retry,
                Redownload,
            }
            let recovery = match fault {
                Some(fault) if fault.code == DownloadFaultCode::DestinationExists => {
                    Recovery::Rename
                }
                Some(fault) if fault.code == DownloadFaultCode::UnexpectedWebPage => {
                    Recovery::Browser
                }
                Some(fault) if protocol_restart_summary(fault).is_some() => Recovery::Restart,
                Some(fault) if fault.retryable => Recovery::Retry,
                _ => Recovery::Redownload,
            };
            let conflict = recovery == Recovery::Rename;
            let offer_secondary_download = recovery == Recovery::Retry;
            let (action_label, visible_label) = match recovery {
                Recovery::Rename => ("自动重命名后重试", "自动重命名后重试"),
                Recovery::Browser => ("在浏览器中打开", "在浏览器中打开"),
                Recovery::Restart => ("从头下载", "从头下载"),
                Recovery::Retry => ("重试下载", "重试"),
                Recovery::Redownload => ("重新下载", "重新下载"),
            };
            let status_text = match fault {
                Some(fault) if fault.code == DownloadFaultCode::FileInUse => {
                    "目标文件正在被占用，请先关闭占用程序"
                }
                Some(fault) if fault.code == DownloadFaultCode::DiskFull => {
                    "磁盘空间不足，请释放保存位置的空间"
                }
                Some(fault) if fault.code == DownloadFaultCode::UnexpectedWebPage => {
                    "网站返回了网页，请在浏览器中完成登录或验证"
                }
                Some(fault) => crate::main_window::download_failure_summary(fault),
                None => "下载失败",
            };
            return download_task_window_shell(
                theme,
                DownloadWindowPhase::Failed,
                "下载失败",
                div()
                    .id("download-failed-content-scroll")
                    .size_full()
                    .overflow_y_scroll()
                    .px_4()
                    .py_2()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .flex()
                            .flex_none()
                            .items_center()
                            .gap_2()
                            .child(
                                Icon::new(MaterialIcon::Download)
                                    .size(ordinary_rem(16.0))
                                    .text_color(theme.danger),
                            )
                            .child(
                                task_window_filename(
                                    "download-window-failed-name",
                                    name.clone(),
                                    theme,
                                    2,
                                )
                                .flex_1(),
                            ),
                    )
                    .child(
                        div()
                            .debug_selector(|| "download-window-failure-detail".into())
                            .when(conflict, |body| {
                                body.debug_selector(|| {
                                    "download-window-destination-conflict".into()
                                })
                            })
                            .text_size(ordinary_rem(TEXT_LABEL))
                            .line_height(ordinary_rem(LINE_HEIGHT_LABEL))
                            .text_color(theme.danger)
                            .child(status_text),
                    )
                    .child(
                        div()
                            .id("download-window-failed-path")
                            .debug_selector(|| "download-window-failed-path".into())
                            .min_w_0()
                            .text_size(ordinary_rem(TEXT_LABEL))
                            .line_height(ordinary_rem(LINE_HEIGHT_LABEL))
                            .text_color(theme.text_secondary)
                            .truncate()
                            .flex_none()
                            .aria_label(destination.clone())
                            .tooltip(bounded_text_tooltip(
                                "download-failed-path-tooltip",
                                destination.clone(),
                                true,
                            ))
                            .child(destination.clone()),
                    ),
                DialogFooter::new()
                    .size_full()
                    .items_center()
                    .child(
                        Button::new("download-window-failed-cancel")
                            .label("取消任务")
                            .small()
                            .outline()
                            .on_click(move |_, _, cx| {
                                let application = cancel_application.clone();
                                cx.defer(move |cx| {
                                    application.update(cx, |application, cx| {
                                        if conflict {
                                            let _ = application
                                                .resolve_destination_conflict(task_id, false, cx);
                                        } else {
                                            let _ = application.cancel(task_id, cx);
                                        }
                                    });
                                });
                            }),
                    )
                    .child(
                        crate::ui::components::action::primary_action_button(
                            "download-window-failed-retry",
                            action_label,
                            visible_label,
                            theme,
                            false,
                            cx,
                        )
                        .debug_selector(|| "download-window-failed-primary".into())
                        .icon(Icon::new(MaterialIcon::Download))
                        .on_click(move |_, _, cx| {
                            let application = primary_application.clone();
                            cx.defer(move |cx| {
                                application.update(cx, |application, cx| match recovery {
                                    Recovery::Rename => {
                                        let _ = application
                                            .resolve_destination_conflict(task_id, true, cx);
                                    }
                                    Recovery::Browser => {
                                        application.open_task_source_in_browser(task_id, cx)
                                    }
                                    Recovery::Restart | Recovery::Redownload => {
                                        let _ = application.redownload(task_id, cx);
                                    }
                                    Recovery::Retry => {
                                        let _ = application.retry(task_id, cx);
                                    }
                                });
                            });
                        }),
                    )
                    .when(offer_secondary_download, |footer| {
                        footer.child(
                            Button::new("download-window-failed-redownload")
                                .icon(Icon::new(MaterialIcon::Download))
                                .label("重新下载")
                                .small()
                                .outline()
                                .on_click(move |_, _, cx| {
                                    let application = redownload_application.clone();
                                    cx.defer(move |cx| {
                                        application.update(cx, |application, cx| {
                                            let _ = application.redownload(task_id, cx);
                                        });
                                    });
                                }),
                        )
                    }),
            )
            .into_any_element();
        }

        let total = task
            .total_bytes
            .or_else(|| task.probe.as_ref().and_then(|probe| probe.total_bytes));
        let downloaded = progress_event
            .as_ref()
            .map_or(task.downloaded_bytes, |progress| progress.downloaded_bytes);
        let finishing = (status == DownloadStatus::Finalizing)
            .then(|| {
                progress_event
                    .as_ref()
                    .and_then(|event| event.finishing.as_ref())
            })
            .flatten();
        let ratio = if status == DownloadStatus::Finalizing {
            finishing.and_then(finishing_ratio)
        } else {
            total.filter(|total| *total > 0).map(|total| {
                (downloaded as f32 / total as f32).clamp(
                    0.0,
                    if status == DownloadStatus::Completed {
                        1.0
                    } else {
                        0.99
                    },
                )
            })
        };
        let display = self.display_sampler.current();
        let speed = display.bytes_per_second;
        let progress_text = total.map_or_else(
            || format_download_size(downloaded),
            |total| {
                format!(
                    "{} / {}",
                    format_download_size(downloaded),
                    format_download_size(total)
                )
            },
        );
        let transferring = status == DownloadStatus::Downloading;
        let eta = display
            .remaining_seconds
            .map(format_download_eta)
            .unwrap_or_else(|| "—".into());
        let (default_label, status_color) = download_task_status(status, theme);
        let status_label = finishing.map_or(default_label, finishing_stage_label);
        let progress_badge = if let Some(finishing) = finishing {
            finishing_ratio(finishing)
                .map(|ratio| format!("{} · {:.0}%", status_label, ratio * 100.0))
                .unwrap_or_else(|| status_label.to_owned())
        } else {
            ratio
                .map(|ratio| format!("{:.0}%", ratio * 100.0))
                .unwrap_or_else(|| status_label.to_owned())
        };
        let action_application = application.clone();
        let secondary_application = application.clone();
        let source_view = cx.entity().downgrade();
        let actions = match status {
            DownloadStatus::Created | DownloadStatus::Paused | DownloadStatus::Failed => div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    Button::new("download-window-cancel")
                        .debug_selector(|| "download-window-cancel".into())
                        .label("取消任务")
                        .small()
                        .outline()
                        .on_click(move |_, _, cx| {
                            let application = secondary_application.clone();
                            cx.defer(move |cx| {
                                application.update(cx, |application, cx| {
                                    let _ = application.cancel(task_id, cx);
                                });
                            });
                        }),
                )
                .when(
                    status == DownloadStatus::Paused && task.task.request().media.is_none(),
                    |actions| {
                        actions.child(
                            Button::new("download-window-replace-url")
                                .debug_selector(|| "download-window-replace-url".into())
                                .label("更换链接")
                                .small()
                                .outline()
                                .on_click(move |_, window, cx| {
                                    let view = source_view.clone();
                                    window.defer(cx, move |window, cx| {
                                        let _ = view.update(cx, |view, cx| {
                                            view.open_source_draft(window, cx)
                                        });
                                    });
                                }),
                        )
                    },
                )
                .child(
                    crate::ui::components::action::primary_action_button(
                        "download-window-resume",
                        if status == DownloadStatus::Failed {
                            "重试下载"
                        } else {
                            "继续下载"
                        },
                        if status == DownloadStatus::Failed {
                            "重试"
                        } else {
                            "继续"
                        },
                        theme,
                        false,
                        cx,
                    )
                    .icon(Icon::new(MaterialIcon::Download))
                    .on_click(move |_, _, cx| {
                        let application = action_application.clone();
                        cx.defer(move |cx| {
                            application.update(cx, |application, cx| {
                                if status == DownloadStatus::Failed {
                                    let _ = application.retry(task_id, cx);
                                } else {
                                    let _ = application.resume(task_id, cx);
                                }
                            });
                        });
                    }),
                )
                .into_any_element(),
            status
                if matches!(
                    status,
                    DownloadStatus::Probing | DownloadStatus::Waiting | DownloadStatus::Downloading
                ) || (status == DownloadStatus::Finalizing
                    && task.task.request().media.is_some()) =>
            {
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::new("download-window-cancel-active")
                            .debug_selector(|| "download-window-cancel-active".into())
                            .label("取消")
                            .small()
                            .outline()
                            .on_click(move |_, _, cx| {
                                let application = secondary_application.clone();
                                cx.defer(move |cx| {
                                    application.update(cx, |application, cx| {
                                        let _ = application.cancel(task_id, cx);
                                    });
                                });
                            }),
                    )
                    .child(
                        crate::ui::components::action::primary_action_button(
                            "download-window-pause",
                            "暂停下载",
                            "暂停",
                            theme,
                            false,
                            cx,
                        )
                        .on_click(move |_, _, cx| {
                            let application = action_application.clone();
                            cx.defer(move |cx| {
                                application.update(cx, |application, cx| {
                                    let _ = application.pause(task_id, cx);
                                });
                            });
                        }),
                    )
                    .into_any_element()
            }
            _ => div().into_any_element(),
        };

        let segments = self.range_segments().to_vec();
        let resume_label = task.probe.as_ref().map_or("续传待确认", |probe| {
            if probe.supports_ranges {
                "支持续传"
            } else {
                "不支持续传"
            }
        });
        let effective_limit = match (
            self.speed_limit_edit.applied,
            self.effective_global_speed_limit,
        ) {
            (Some(task), Some(global)) => Some(task.min(global)),
            (task, global) => task.or(global),
        };
        let limit_label = effective_limit
            .map(|limit| format!("限速 {}/s", format_download_size(limit)))
            .unwrap_or_else(|| "不限速".into());
        let capabilities =
            task_window_capabilities(resume_label, &limit_label, status, progress_event.as_ref());
        let details_expanded = self.details_expanded;
        let details = details_expanded
            .then(|| self.render_detail_pages(capabilities, total, &segments, status, cx));
        let options_label = self
            .completion_intention
            .action
            .map_or("选项".to_owned(), |action| {
                format!("选项 · {}", super::completion_action_label(action))
            });
        download_task_window_shell(
            theme,
            DownloadWindowPhase::Progress,
            status_label,
            div()
                .id("download-progress-content-scroll")
                .size_full()
                .overflow_hidden()
                .px_4()
                .pt_1()
                .pb_0()
                .flex()
                .flex_col()
                .gap_0()
                .child(
                    div()
                        .debug_selector(|| "download-window-progress-identity".into())
                        .w_full()
                        .min_w_0()
                        .h(ordinary_rem(36.0))
                        .flex_none()
                        .flex()
                        .items_start()
                        .gap_2()
                        .child(
                            Icon::new(MaterialIcon::Download)
                                .size(ordinary_rem(16.0))
                                .text_color(status_color),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .child(task_window_filename(
                                    "download-window-progress-name",
                                    name.clone(),
                                    theme,
                                    1,
                                ))
                                .child(
                                    DescriptionList::horizontal()
                                        .small()
                                        .bordered(false)
                                        .columns(1)
                                        .label_width(ordinary_rem(44.0))
                                        .item(
                                            monitor_metric_text("服务器", "label", "服务器".into()),
                                            server_information_value(&task, theme),
                                            1,
                                        ),
                                ),
                        )
                        .child(
                            Button::new("download-window-details-toggle")
                                .debug_selector(|| "download-window-details-toggle".into())
                                .icon(
                                    Icon::new(if details_expanded {
                                        MaterialIcon::ExpandLess
                                    } else {
                                        MaterialIcon::ExpandMore
                                    })
                                    .size(ordinary_rem(16.0)),
                                )
                                .accessibility_label(if details_expanded {
                                    "收起详情"
                                } else {
                                    "展开详情"
                                })
                                .tooltip(if details_expanded {
                                    "收起详情"
                                } else {
                                    "展开详情"
                                })
                                .ghost()
                                .xsmall()
                                .on_click(cx.listener(|this, _, window, cx| {
                                    if !this.details_expanded {
                                        this.details_tab = 0;
                                    }
                                    this.set_details_expanded(!this.details_expanded, window, cx);
                                })),
                        ),
                )
                .child(
                    div()
                        .debug_selector(|| "download-window-progress-track".into())
                        .w_full()
                        .h(ordinary_rem(16.0))
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .debug_selector(|| "download-window-progress-component".into())
                                .flex_1()
                                .min_w_0()
                                .h_1()
                                .child(
                                    Progress::new("download-task-window-progress")
                                        .xsmall()
                                        .value(ratio.unwrap_or(0.0) * 100.0)
                                        .loading(
                                            status == DownloadStatus::Finalizing && ratio.is_none(),
                                        )
                                        .color(status_color)
                                        .accessibility_label("任务汇总进度"),
                                ),
                        )
                        .child(
                            div()
                                .debug_selector(|| "download-window-progress-percent".into())
                                .id("download-window-progress-percent")
                                .role(gpui_kit::Role::Label)
                                .aria_label(progress_badge.clone())
                                .text_size(ordinary_rem(TEXT_BODY))
                                .line_height(ordinary_rem(LINE_HEIGHT_LABEL))
                                .flex_none()
                                .test_support()
                                .child(progress_badge),
                        ),
                )
                .child(
                    div()
                        .debug_selector(|| "download-window-progress-metrics".into())
                        .w_full()
                        .flex_none()
                        .child(
                            DescriptionList::horizontal()
                                .small()
                                .bordered(false)
                                .columns(2)
                                .label_width(ordinary_rem(44.0))
                                .item(
                                    monitor_metric_text("已下载", "label", "已下载".into()),
                                    monitor_metric_text("已下载", "value", progress_text),
                                    1,
                                )
                                .item(
                                    monitor_metric_text("速度", "label", "速度".into()),
                                    monitor_metric_text(
                                        "速度",
                                        "value",
                                        if transferring {
                                            format!("{}/s", format_download_size(speed))
                                        } else {
                                            "—".into()
                                        },
                                    ),
                                    1,
                                )
                                .item(
                                    monitor_metric_text("剩余", "label", "剩余".into()),
                                    monitor_metric_text("剩余", "value", eta),
                                    1,
                                ),
                        ),
                )
                .when_some(details, |body, details| body.child(details)),
            Toolbar::new("download-task-window-toolbar")
                .w_full()
                .when(!matches!(status, DownloadStatus::Cancelled), |footer| {
                    footer.content(
                        Button::new("download-window-options")
                            .debug_selector(|| "download-window-options".into())
                            .label(options_label)
                            .accessibility_label("下载选项")
                            .small()
                            .outline()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.details_tab = 1;
                                this.set_details_expanded(true, window, cx);
                            })),
                    )
                })
                .content(div().flex_1())
                .content(
                    ToolbarGroup::new("download-task-window-action-group")
                        .label("任务操作")
                        .content(actions),
                ),
        )
        .into_any_element()
    }
}

fn server_information_value(task: &DownloadEngineTask, theme: Theme) -> AnyElement {
    let probe = task.probe.as_ref();
    let observation = probe.and_then(|probe| probe.server_observation.as_ref());
    let (endpoint, route, response) = if let Some(value) = observation {
        let provenance = if probe.is_some_and(|probe| probe.media.is_some()) {
            "清单探测端点 "
        } else {
            ""
        };
        let route = match value.route_kind {
            nxtools_download::DownloadRouteKind::Direct => "直连",
            nxtools_download::DownloadRouteKind::ManualProxy => "代理",
            nxtools_download::DownloadRouteKind::SystemPolicy => "系统策略",
        };
        (
            format!("{provenance}{}", value.host),
            Some(route),
            Some(value.response_ms),
        )
    } else {
        let source = probe.map_or(task.task.request().source_url.as_str(), |probe| {
            probe.final_url.as_str()
        });
        let state = if !source.starts_with("http://") && !source.starts_with("https://") {
            "不可用"
        } else if probe.is_none()
            && matches!(
                task.task.status(),
                DownloadStatus::Created | DownloadStatus::Probing | DownloadStatus::Waiting
            )
        {
            "待探测"
        } else {
            "未记录"
        };
        (state.to_owned(), None, None)
    };
    let response_text = response.map(|response_ms| format!("{response_ms} ms"));
    let label = [
        Some(format!("服务器：{endpoint}")),
        route.map(ToOwned::to_owned),
        response_text.clone(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" · ");
    let explanation = format!(
        "{label}\n探测响应是该次 HTTP 请求从发出到响应头到达的耗时，包含 DNS、连接、TLS 和服务器处理时间，不是 Ping 或 RTT。代理和系统策略仅说明请求路由配置；清单探测端点不代表所有媒体分段的来源。"
    );
    div()
        .id("download-window-server-value")
        .debug_selector(|| "download-window-server-value".into())
        .role(gpui_kit::Role::Label)
        .min_w_0()
        .w_full()
        .h(ordinary_rem(16.0))
        .flex()
        .items_center()
        .gap_1()
        .overflow_hidden()
        .text_size(ordinary_rem(TEXT_BODY))
        .line_height(ordinary_rem(LINE_HEIGHT_LABEL))
        .aria_label(label)
        .tooltip(bounded_text_tooltip(
            "download-window-server-tooltip",
            explanation,
            false,
        ))
        .child(
            div()
                .debug_selector(|| "download-window-server-host".into())
                .flex_1()
                .min_w_0()
                .truncate()
                .child(endpoint),
        )
        .when_some(route, |row, route| {
            row.child(
                div()
                    .debug_selector(|| "download-window-server-route".into())
                    .flex_none()
                    .child(route),
            )
        })
        .when_some(
            response.zip(response_text),
            |row, (response_ms, response_text)| {
                let response_color = if response_ms <= 100 {
                    theme.success
                } else if response_ms <= 300 {
                    theme.warning
                } else {
                    theme.danger
                };
                row.child(
                    div()
                        .debug_selector(|| "download-window-server-time".into())
                        .flex_none()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(response_color)
                        .child(response_text),
                )
            },
        )
        .test_support()
        .into_any_element()
}

/// The collapsed progress window height (34 titlebar + 106 body + 44 footer).
pub(crate) const TASK_WINDOW_PROGRESS_HEIGHT: f32 = 184.0;
/// The expanded height adds a bounded, vertically scrollable semantic part table.
pub(crate) const TASK_WINDOW_PROGRESS_EXPANDED_HEIGHT: f32 = 400.0;

fn observe_task_display(
    sampler: &mut ProgressDisplaySampler,
    now: std::time::Duration,
    task: Option<&DownloadEngineTask>,
    progress: Option<&DownloadProgress>,
) {
    let bytes = progress.map_or_else(
        || task.map_or(0, |task| task.downloaded_bytes),
        |progress| progress.downloaded_bytes,
    );
    let total = task.and_then(|task| {
        task.total_bytes
            .or_else(|| task.probe.as_ref().and_then(|probe| probe.total_bytes))
    });
    let transferring = task.is_some_and(|task| task.task.status() == DownloadStatus::Downloading);
    sampler.observe(now, bytes, total, transferring);
}

fn task_window_filename(
    selector: &'static str,
    name: String,
    theme: Theme,
    lines: usize,
) -> impl IntoElement + gpui_kit::Styled {
    let (text_size, line_height) = if lines == 1 {
        (TEXT_TITLE, LINE_HEIGHT_TITLE)
    } else {
        (TEXT_BODY, LINE_HEIGHT_LABEL)
    };
    div()
        .id(selector)
        .debug_selector(move || selector.into())
        .role(gpui_kit::Role::Label)
        .min_w_0()
        .max_h(ordinary_rem(line_height * lines as f32))
        .flex_none()
        .text_size(ordinary_rem(text_size))
        .line_height(ordinary_rem(line_height))
        .font_weight(FontWeight::MEDIUM)
        .text_color(theme.text)
        .line_clamp(lines)
        .aria_label(name.clone())
        .tooltip(bounded_text_tooltip(
            format!("{selector}-tooltip"),
            name.clone(),
            true,
        ))
        .child(name)
        .test_support()
}

fn monitor_metric_text(label: &'static str, role: &'static str, text: String) -> AnyElement {
    div()
        .id(gpui_kit::SharedString::from(format!(
            "download-window-metric-{label}-{role}"
        )))
        .debug_selector(move || format!("download-window-metric-{label}-{role}"))
        .min_w_0()
        .h(ordinary_rem(16.0))
        .text_size(ordinary_rem(TEXT_BODY))
        .line_height(ordinary_rem(LINE_HEIGHT_LABEL))
        .aria_label(text.clone())
        .tooltip(bounded_text_tooltip(
            format!("download-window-metric-{label}-{role}-tooltip"),
            text.clone(),
            false,
        ))
        .truncate()
        .child(text)
        .into_any_element()
}

pub(crate) fn segment_display_status(
    segment: &nxtools_download::DownloadSegmentProgress,
    status: DownloadStatus,
) -> &'static str {
    if segment
        .total_bytes
        .is_some_and(|total| total > 0 && segment.downloaded_bytes >= total)
    {
        "已完成"
    } else if status == DownloadStatus::Paused {
        "已暂停"
    } else if status == DownloadStatus::Failed {
        "失败"
    } else if status == DownloadStatus::Cancelled {
        "已取消"
    } else if segment.downloaded_bytes > 0 {
        "传输中"
    } else {
        "等待"
    }
}

fn segment_progress_table(
    segments: &[nxtools_download::DownloadSegmentProgress],
    status: DownloadStatus,
) -> AnyElement {
    let widths = [40.0, 88.0, 140.0, 140.0];
    let header = TableRow::new().children(
        ["序号", "状态", "已下载", "总大小"]
            .into_iter()
            .zip(widths)
            .map(|(label, width)| {
                TableHead::new()
                    .w(gpui_kit::relative(width / 408.0))
                    .min_w_0()
                    .h(ordinary_rem(24.0))
                    .py_0()
                    .child(label)
            }),
    );
    let body = TableBody::new().children(segments.iter().map(|segment| {
        let index = segment.segment_index;
        let values = [
            format!("{}", index + 1),
            segment_display_status(segment, status).into(),
            format_download_size(segment.downloaded_bytes),
            segment
                .total_bytes
                .map(format_download_size)
                .unwrap_or_else(|| "未知".into()),
        ];
        TableRow::new().children(values.into_iter().zip(widths).enumerate().map(
            move |(column, (value, width))| {
                TableCell::new()
                    .w(gpui_kit::relative(width / 408.0))
                    .min_w_0()
                    .h(ordinary_rem(24.0))
                    .py_0()
                    .child(
                        div()
                            .debug_selector(move || {
                                format!("download-window-segment-{index}-{column}")
                            })
                            .min_w_0()
                            .truncate()
                            .child(value),
                    )
            },
        ))
    }));
    div()
        .id("download-window-segment-scroll")
        .debug_selector(|| "download-window-segment-table".into())
        .size_full()
        .min_h_0()
        .overflow_y_scroll()
        .child(
            Table::new()
                .xsmall()
                .accessibility_label("分段进度：序号、状态、已下载、总大小")
                .child(TableHeader::new().child(header))
                .child(body),
        )
        .when(segments.is_empty(), |table| {
            table.child(
                div()
                    .debug_selector(|| "download-window-single-connection".into())
                    .text_size(ordinary_rem(TEXT_BODY))
                    .line_height(ordinary_rem(LINE_HEIGHT_LABEL))
                    .child("尚未分段 · 单连接传输"),
            )
        })
        .into_any_element()
}

pub(super) fn task_window_capabilities(
    resume_label: &str,
    limit_label: &str,
    status: DownloadStatus,
    progress: Option<&DownloadProgress>,
) -> String {
    // In-flight worker count is independent of the complete segment plan.
    // Retained bytes are not evidence of a live connection after pause.
    let connections = if status != DownloadStatus::Downloading {
        if progress.is_some() {
            "0".into()
        } else {
            "—".into()
        }
    } else {
        progress
            .map(|progress| progress.active_connections.to_string())
            .unwrap_or_else(|| "—".into())
    };
    let segments = progress
        .map(|progress| progress.segments.len().to_string())
        .unwrap_or_else(|| "—".into());
    format!("{resume_label} · {limit_label} · 活动连接 {connections} · 总分段 {segments}")
}

#[cfg(test)]
#[path = "auxiliary_theme_tests.rs"]
mod auxiliary_theme_tests;

#[cfg(test)]
#[path = "task_window_layout_tests.rs"]
pub(crate) mod layout_tests;
