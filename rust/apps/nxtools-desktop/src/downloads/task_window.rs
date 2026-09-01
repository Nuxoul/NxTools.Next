use super::*;

pub(crate) struct DownloadTaskWindowPhaseLayout {
    pub(crate) size: gpui::Size<gpui::Pixels>,
    pub(crate) selector: &'static str,
}

pub(crate) fn download_task_window_phase_layout(
    phase: DownloadWindowPhase,
) -> DownloadTaskWindowPhaseLayout {
    match phase {
        DownloadWindowPhase::Progress => DownloadTaskWindowPhaseLayout {
            size: size(px(440.0), px(176.0)),
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

pub(crate) fn centered_task_window_bounds_command(
    window_handle: i64,
    current_bounds: gpui::Bounds<gpui::Pixels>,
    new_size: gpui::Size<gpui::Pixels>,
    scale_factor: f32,
) -> SetWindowBoundsCommand {
    let centered_x = current_bounds.origin.x.as_f32()
        + (current_bounds.size.width.as_f32() - new_size.width.as_f32()) / 2.0;
    let centered_y = current_bounds.origin.y.as_f32()
        + (current_bounds.size.height.as_f32() - new_size.height.as_f32()) / 2.0;
    SetWindowBoundsCommand {
        window_handle,
        x: (centered_x * scale_factor).round() as i32,
        y: (centered_y * scale_factor).round() as i32,
        width: (new_size.width.as_f32() * scale_factor).round() as i32,
        height: (new_size.height.as_f32() * scale_factor).round() as i32,
    }
}

pub(crate) fn download_task_window_shell(
    theme: Theme,
    phase: DownloadWindowPhase,
    title: &'static str,
    body: impl IntoElement,
    footer: impl IntoElement,
) -> impl IntoElement {
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
                .child(
                    div()
                        .debug_selector(move || phase_selector.into())
                        .size_full()
                        .child(body),
                ),
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
        }
    }

    pub(crate) fn replace_snapshot(
        &mut self,
        snapshot: Option<DownloadEngineTask>,
        progress: Option<DownloadProgress>,
        shutdown_armed: bool,
        cx: &mut Context<Self>,
    ) {
        self.snapshot = snapshot;
        self.progress = progress;
        self.shutdown_armed = shutdown_armed;
        cx.notify();
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
            || task.task.request().save_directory.display().to_string(),
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
                                });
                            }),
                    ),
            )
            .into_any_element();
        }

        if status == DownloadStatus::Failed {
            let cancel_application = application.clone();
            let retry_application = application.clone();
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
                            .px(px(9.0))
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
                            .ghost()
                            .on_click(move |_, _, cx| {
                                let application = cancel_application.clone();
                                cx.defer(move |cx| {
                                    application.update(cx, |application, cx| {
                                        let _ = application.cancel(task_id, cx);
                                    });
                                });
                            }),
                    )
                    .child(
                        Button::new("download-window-failed-retry")
                            .icon(Icon::new(MaterialIcon::Download))
                            .label("重试")
                            .small()
                            .custom(download_primary_button_variant(theme, cx))
                            .on_click(move |_, _, cx| {
                                let application = retry_application.clone();
                                cx.defer(move |cx| {
                                    application.update(cx, |application, cx| {
                                        let _ = application.resume(task_id, cx);
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
        let eta = total
            .zip((speed > 0).then_some(speed))
            .map(|(total, speed)| total.saturating_sub(downloaded) / speed)
            .map(format_download_eta)
            .unwrap_or_else(|| "—".into());
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
                        .ghost()
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
                                    let _ = application.resume(task_id, cx);
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
                            .ghost()
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
                        .h(px(5.0))
                        .rounded(px(RADIUS_CONTROL))
                        .bg(theme.surface_raised)
                        .when_some(ratio, |track, ratio| {
                            track.child(
                                div()
                                    .h_full()
                                    .w(px(408.0 * ratio))
                                    .rounded(px(RADIUS_CONTROL))
                                    .bg(status_color),
                            )
                        }),
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
                            format!("{} /s", format_download_size(speed)),
                        ))
                        .child(download_window_metric(theme, "剩余", eta))
                        .child(download_window_metric(
                            theme,
                            "连接",
                            connections.to_string(),
                        )),
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
                                        .label(if shutdown_armed {
                                            "已计划关机"
                                        } else {
                                            "完成后关机"
                                        })
                                        .small()
                                        .h(px(28.0))
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
