use super::*;
use crate::ui::metrics::PILL_PADDING_X;
use gpui::{Animation, AnimationExt, ease_out_quint};
pub(crate) struct DownloadTaskWindowPhaseLayout {
    pub(crate) size: gpui::Size<gpui::Pixels>,
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
            size: size(px(440.0), px(180.0)),
            selector: "download-task-window-phase-completed",
        },
    }
}

pub(crate) fn download_task_window_size(phase: DownloadWindowPhase) -> gpui::Size<gpui::Pixels> {
    download_task_window_phase_layout(phase).size
}

pub(crate) fn download_task_window_size_for_details(
    phase: DownloadWindowPhase,
    details_expanded: bool,
) -> gpui::Size<gpui::Pixels> {
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
    let animated = div().size_full().child(body).with_animation(
        ("download-task-window-phase", phase_key),
        Animation::new(std::time::Duration::from_millis(120)).with_easing(ease_out_quint()),
        |surface, delta| {
            surface
                .opacity(0.94 + 0.06 * delta)
                .relative()
                .top(px(3.0 * (1.0 - delta)))
        },
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
                .px(px(14.0))
                .border_t_1()
                .border_color(theme.border_subtle)
                .bg(theme.surface)
                .child(footer),
        )
}

pub(crate) struct DownloadTaskWindowView {
    pub(crate) theme: Theme,
    pub(crate) application: Entity<DownloadApplication>,
    pub(crate) task_id: DownloadTaskId,
    pub(crate) native_window_handle: i64,
    pub(crate) snapshot: Option<DownloadEngineTask>,
    pub(crate) progress: Option<DownloadProgress>,
    pub(crate) shutdown_armed: bool,
    pub(crate) details_expanded: bool,
}

impl DownloadTaskWindowView {
    pub(crate) fn new(
        application: Entity<DownloadApplication>,
        task_id: DownloadTaskId,
        native_window_handle: i64,
        snapshot: Option<DownloadEngineTask>,
        progress: Option<DownloadProgress>,
        shutdown_armed: bool,
    ) -> Self {
        Self {
            theme: Theme::zed_dark(),
            application,
            task_id,
            native_window_handle,
            snapshot,
            progress,
            shutdown_armed,
            details_expanded: false,
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
        self.snapshot = snapshot;
        self.progress = progress;
        self.shutdown_armed = shutdown_armed;
        cx.notify();
    }

    pub(crate) fn reset_details_expanded(&mut self) {
        self.details_expanded = false;
    }
}

impl Render for DownloadTaskWindowView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
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
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(TEXT_LABEL))
                        .text_color(theme.text_muted)
                        .child("任务已被移除"),
                )
                .into_any_element();
        };

        let status = task.task.status();
        let shutdown_armed = self.shutdown_armed;
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

        if status == DownloadStatus::Completed {
            let open_application = application.clone();
            let reveal_application = application.clone();
            let drag_application = application.clone();
            let drag_spec = CompletedTaskWindowAction::Drag.spec();
            return download_task_window_shell(
                theme,
                DownloadWindowPhase::Completed,
                "下载完成",
                div()
                    .size_full()
                    .px(px(16.0))
                    .py(px(10.0))
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .child(
                        div()
                            .size(px(38.0))
                            .rounded(px(RADIUS_FLOATING))
                            .bg(theme.accent_soft)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                Icon::new(MaterialIcon::Check)
                                    .size(px(19.0))
                                    .text_color(theme.success),
                            ),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(3.0))
                            .child(
                                div()
                                    .text_size(px(TEXT_BODY))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .truncate()
                                    .child(name),
                            )
                            .child(
                                div()
                                    .text_size(px(TEXT_META))
                                    .text_color(theme.text_muted)
                                    .child(
                                        task.total_bytes.map(format_download_size).unwrap_or_else(
                                            || format_download_size(task.downloaded_bytes),
                                        ),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(TEXT_META))
                                    .text_color(theme.text_muted)
                                    .truncate()
                                    .child(final_path),
                            ),
                    ),
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(6.0))
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
                                cx.defer(move |cx| {
                                    application.update(cx, |application, cx| {
                                        application.open_task_artifact(
                                            task_id,
                                            ShellOpenAction::SelectInExplorer,
                                            cx,
                                        );
                                    });
                                    // Open/reveal confirms the terminal feedback; the
                                    // completed window has no further purpose after the
                                    // Shell action leaves this process.
                                    application.update(cx, |application, cx| {
                                        application.close_task_window(task_id, cx);
                                    });
                                });
                            }),
                    )
                    .child(
                        completed_task_window_action(CompletedTaskWindowAction::Open, theme, cx)
                            .on_click(move |_, _, cx| {
                                let application = open_application.clone();
                                cx.defer(move |cx| {
                                    application.update(cx, |application, cx| {
                                        application.open_task_artifact(
                                            task_id,
                                            ShellOpenAction::Open,
                                            cx,
                                        );
                                    });
                                    application.update(cx, |application, cx| {
                                        application.close_task_window(task_id, cx);
                                    });
                                });
                            }),
                    ),
            )
            .into_any_element();
        }

        if status == DownloadStatus::Failed {
            let cancel_application = application.clone();
            let retry_application = application.clone();
            let redownload_application = application.clone();
            let retryable = task.error.as_ref().is_some_and(|fault| fault.retryable);
            let error = task.error.as_ref().map_or_else(
                || "下载任务失败，未返回更多信息。".into(),
                |error| error.message.clone(),
            );
            return download_task_window_shell(
                theme,
                DownloadWindowPhase::Failed,
                "下载失败",
                div()
                    .size_full()
                    .px(px(16.0))
                    .py(px(10.0))
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                Icon::new(MaterialIcon::Download)
                                    .size(px(17.0))
                                    .text_color(theme.danger),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .text_size(px(TEXT_LABEL))
                                    .font_weight(FontWeight::MEDIUM)
                                    .truncate()
                                    .child(name),
                            ),
                    )
                    .child(
                        div()
                            .w_full()
                            .px(px(PILL_PADDING_X))
                            .py(px(7.0))
                            .rounded(px(RADIUS_CONTROL))
                            .bg(theme.danger_soft)
                            .text_size(px(TEXT_LABEL))
                            .text_color(theme.danger)
                            .truncate()
                            .child(error),
                    )
                    .child(
                        div()
                            .text_size(px(TEXT_META))
                            .text_color(theme.text_muted)
                            .truncate()
                            .child(final_path),
                    ),
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(6.0))
                    .child(
                        Button::new("download-window-failed-cancel")
                            .label("取消任务")
                            .small()
                            .h(px(CONTROL_HEIGHT_ACTION))
                            .outline()
                            .on_click(move |_, _, cx| {
                                let application = cancel_application.clone();
                                cx.defer(move |cx| {
                                    application.update(cx, |application, cx| {
                                        let _ = application.cancel(task_id, cx);
                                    });
                                });
                            }),
                    )
                    .when(retryable, |footer| {
                        footer.child(
                            Button::new("download-window-failed-retry")
                                .icon(Icon::new(MaterialIcon::Download))
                                .label("重试")
                                .small()
                                .h(px(CONTROL_HEIGHT_ACTION))
                                .custom(download_primary_button_variant(theme, cx))
                                .on_click(move |_, _, cx| {
                                    let application = retry_application.clone();
                                    cx.defer(move |cx| {
                                        application.update(cx, |application, cx| {
                                            let _ = application.retry(task_id, cx);
                                        });
                                    });
                                }),
                        )
                    })
                    .child(
                        Button::new("download-window-failed-redownload")
                            .icon(Icon::new(MaterialIcon::Download))
                            .label("重新下载")
                            .small()
                            .h(px(CONTROL_HEIGHT_ACTION))
                            .outline()
                            .on_click(move |_, _, cx| {
                                let application = redownload_application.clone();
                                cx.defer(move |cx| {
                                    application.update(cx, |application, cx| {
                                        let _ = application.redownload(task_id, cx);
                                    });
                                });
                            }),
                    ),
            )
            .into_any_element();
        }

        let total = task
            .total_bytes
            .or_else(|| task.probe.as_ref().and_then(|probe| probe.total_bytes));
        let downloaded = progress_event
            .as_ref()
            .map_or(task.downloaded_bytes, |progress| progress.downloaded_bytes);
        let ratio = total
            .filter(|total| *total > 0)
            .map(|total| (downloaded as f32 / total as f32).clamp(0.0, 1.0));
        let speed = progress_event
            .as_ref()
            .map_or(0, |progress| progress.bytes_per_second);
        let connections = progress_event
            .as_ref()
            .map_or(0, |progress| progress.active_connections);
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
        // A paused/probing task keeps the last progress event's sampler
        // value; showing it as live speed reads like the transfer still runs.
        let transferring = status == DownloadStatus::Downloading;
        let eta = if transferring {
            total
                .zip((speed > 0).then_some(speed))
                .map(|(total, speed)| total.saturating_sub(downloaded) / speed)
                .map(format_download_eta)
                .unwrap_or_else(|| "—".into())
        } else {
            "—".into()
        };
        let (status_label, status_color) = download_task_status(status);
        let progress_badge = ratio
            .map(|ratio| format!("{:.0}%", ratio * 100.0))
            .unwrap_or_else(|| status_label.to_owned());
        let action_application = application.clone();
        let secondary_application = application.clone();
        let shutdown_application = application.clone();
        let actions = match status {
            DownloadStatus::Created | DownloadStatus::Paused | DownloadStatus::Failed => div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    Button::new("download-window-cancel")
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
                .child(
                    Button::new("download-window-resume")
                        .icon(Icon::new(MaterialIcon::Download))
                        .label(if status == DownloadStatus::Failed {
                            "重试"
                        } else {
                            "继续"
                        })
                        .small()
                        .custom(download_primary_button_variant(theme, cx))
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
            DownloadStatus::Probing | DownloadStatus::Waiting | DownloadStatus::Downloading => {
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        Button::new("download-window-cancel-active")
                            .label("取消")
                            .small()
                            .h(px(CONTROL_HEIGHT_ACTION))
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
                        Button::new("download-window-pause")
                            .label("暂停")
                            .small()
                            .h(px(CONTROL_HEIGHT_ACTION))
                            .custom(download_primary_button_variant(theme, cx))
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

        let segments = progress_event
            .as_ref()
            .map(|progress| progress.segments.clone())
            .unwrap_or_default();
        let thread_label = if segments.is_empty() {
            "连接"
        } else {
            "分段"
        };
        let thread_value = if segments.is_empty() {
            connections.to_string()
        } else {
            segments.len().to_string()
        };
        let view_handle = cx.entity().downgrade();
        let details_expanded = self.details_expanded;
        download_task_window_shell(
            theme,
            DownloadWindowPhase::Progress,
            status_label,
            div()
                .size_full()
                .px(px(16.0))
                .py(px(8.0))
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(9.0))
                        .child(
                            Icon::new(MaterialIcon::Download)
                                .size(px(16.0))
                                .text_color(status_color),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .text_size(px(TEXT_LABEL))
                                .font_weight(FontWeight::MEDIUM)
                                .truncate()
                                .child(name),
                        )
                        .child(
                            div()
                                .text_size(px(TEXT_META))
                                .text_color(status_color)
                                .child(progress_badge),
                        ),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .flex_1()
                                .h(px(PROGRESS_TRACK_HEIGHT))
                                .rounded(px(RADIUS_CONTROL))
                                .bg(theme.surface_raised)
                                .when_some(ratio, |track, ratio| {
                                    track.child(
                                        div()
                                            .h_full()
                                            .w(gpui::relative(ratio))
                                            .rounded(px(RADIUS_CONTROL))
                                            .bg(status_color),
                                    )
                                }),
                        )
                        .child(
                            div()
                                .debug_selector(|| "download-window-details-toggle".into())
                                .id("download-window-details-toggle")
                                .size(px(20.0))
                                .rounded(px(RADIUS_CONTROL))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_color(theme.text_muted)
                                .hover(|style| style.bg(theme.surface_raised))
                                .child(
                                    Icon::new(if details_expanded {
                                        MaterialIcon::ExpandLess
                                    } else {
                                        MaterialIcon::ExpandMore
                                    })
                                    .size(px(16.0)),
                                )
                                .on_click({
                                    let view_handle = view_handle.clone();
                                    move |_, window, cx| {
                                        let Some(view) = view_handle.upgrade() else {
                                            return;
                                        };
                                        let expanded = view.read(cx).details_expanded;
                                        let next_expanded = !expanded;
                                        view.update(cx, |view, cx| {
                                            view.details_expanded = next_expanded;
                                            cx.notify();
                                        });
                                        window.resize(download_task_window_size_for_details(
                                            DownloadWindowPhase::Progress,
                                            next_expanded,
                                        ));
                                    }
                                }),
                        ),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .items_start()
                        .gap(px(12.0))
                        .child(download_window_metric(theme, "已下载", progress_text))
                        .child(download_window_metric(
                            theme,
                            "速度",
                            if transferring {
                                format!("{} /s", format_download_size(speed))
                            } else {
                                "—".into()
                            },
                        ))
                        .child(download_window_metric(theme, "剩余", eta))
                        .child(download_window_metric(theme, thread_label, thread_value)),
                )
                .when(!details_expanded, |body| {
                    body.child(
                        div()
                            .debug_selector(|| "download-window-collapsed-path".into())
                            .text_size(px(TEXT_META))
                            .text_color(theme.text_muted)
                            .truncate()
                            .child(final_path.clone()),
                    )
                })
                .when(details_expanded, |body| {
                    body.child(
                        div()
                            .debug_selector(|| "download-window-details".into())
                            .w_full()
                            .flex()
                            .flex_col()
                            .gap(px(6.0))
                            .pt(px(8.0))
                            .border_t_1()
                            .border_color(theme.border_subtle)
                            .child(
                                div()
                                    .w_full()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_size(px(TEXT_META))
                                            .font_weight(FontWeight::MEDIUM)
                                            .child("分段进度"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(TEXT_META))
                                            .text_color(theme.text_muted)
                                            .child(segment_progress_summary(&segments)),
                                    ),
                            )
                            .child(segment_progress_dots(theme, &segments, status_color))
                            .child(
                                div()
                                    .text_size(px(TEXT_META))
                                    .text_color(theme.text_muted)
                                    .truncate()
                                    .child(task.task.request().source_url.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(TEXT_META))
                                    .text_color(theme.text_muted)
                                    .truncate()
                                    .child(final_path),
                            ),
                    )
                }),
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_between()
                .when(
                    !matches!(
                        status,
                        DownloadStatus::Completed
                            | DownloadStatus::Failed
                            | DownloadStatus::Cancelled
                    ),
                    |footer| {
                        footer.child(
                            div()
                                .debug_selector(|| "download-window-shutdown-arm".into())
                                .child(
                                    Button::new("download-window-shutdown-arm-button")
                                        .debug_selector(|| {
                                            "download-window-shutdown-arm-button".into()
                                        })
                                        .label(if shutdown_armed {
                                            "已计划关机"
                                        } else {
                                            "完成后关机"
                                        })
                                        .small()
                                        .h(px(CONTROL_HEIGHT_ACTION))
                                        .px(px(10.0))
                                        .rounded(px(RADIUS_CONTROL))
                                        .when(shutdown_armed, |button| {
                                            button
                                                .custom(download_primary_button_variant(theme, cx))
                                        })
                                        .when(!shutdown_armed, |button| button.outline())
                                        .on_click(move |_, _, cx| {
                                            let application = shutdown_application.clone();
                                            cx.defer(move |cx| {
                                                application.update(cx, |application, cx| {
                                                    application.toggle_shutdown_armed(task_id, cx);
                                                });
                                            });
                                        }),
                                ),
                        )
                    },
                )
                .child(actions),
        )
        .into_any_element()
    }
}

/// The collapsed progress window height (34 titlebar + 106 body + 44 footer).
pub(crate) const TASK_WINDOW_PROGRESS_HEIGHT: f32 = 184.0;
/// The expanded height adds the segment-dots panel plus source/destination lines.
pub(crate) const TASK_WINDOW_PROGRESS_EXPANDED_HEIGHT: f32 = 280.0;
/// Bounded dot rendering: a dynamic split can reach 256 segments, which would
/// never fit the fixed-size task window even when expanded.
const SEGMENT_DOT_LIMIT: usize = 64;
const SEGMENT_DOT_SIZE: f32 = 9.0;

pub(crate) fn segment_dot_ratio(segment: &nxtools_download::DownloadSegmentProgress) -> f32 {
    match segment.total_bytes {
        Some(total) if total > 0 => {
            (segment.downloaded_bytes as f32 / total as f32).clamp(0.0, 1.0)
        }
        Some(_) => 0.0,
        None => {
            if segment.downloaded_bytes > 0 {
                1.0
            } else {
                0.0
            }
        }
    }
}

pub(crate) fn segment_progress_summary(
    segments: &[nxtools_download::DownloadSegmentProgress],
) -> String {
    if segments.is_empty() {
        return "尚未分段".into();
    }
    let finished = segments
        .iter()
        .filter(|segment| segment_dot_ratio(segment) >= 1.0)
        .count();
    format!("{finished}/{} 段完成", segments.len())
}

/// Renders one dot per segment in the old NxTools style: hollow while pending,
/// filling left-to-right with the segment's byte ratio, solid when complete.
fn segment_progress_dots(
    theme: Theme,
    segments: &[nxtools_download::DownloadSegmentProgress],
    active_color: gpui::Rgba,
) -> AnyElement {
    if segments.is_empty() {
        return div()
            .text_size(px(TEXT_META))
            .text_color(theme.text_muted)
            .child("该任务为单连接传输")
            .into_any_element();
    }
    let row = div()
        .debug_selector(|| "download-window-segment-dots".into())
        .w_full()
        .flex()
        .flex_wrap()
        .gap(px(3.0));
    let dots = segments
        .iter()
        .take(SEGMENT_DOT_LIMIT)
        .fold(row, |row, segment| {
            let ratio = segment_dot_ratio(segment);
            row.child(
                div()
                    .size(px(SEGMENT_DOT_SIZE))
                    .rounded_full()
                    .bg(theme.surface_raised)
                    .border_1()
                    .border_color(theme.border_subtle)
                    .overflow_hidden()
                    .when(ratio > 0.0, |dot| {
                        dot.child(div().h_full().w(gpui::relative(ratio)).bg(if ratio >= 1.0 {
                            theme.success
                        } else {
                            active_color
                        }))
                    }),
            )
        });
    if segments.len() > SEGMENT_DOT_LIMIT {
        dots.child(
            div()
                .text_size(px(TEXT_META))
                .text_color(theme.text_muted)
                .child(format!("+{} 段", segments.len() - SEGMENT_DOT_LIMIT)),
        )
        .into_any_element()
    } else {
        dots.into_any_element()
    }
}
