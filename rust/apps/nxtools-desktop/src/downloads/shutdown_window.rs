use super::*;

pub(crate) struct DownloadShutdownCountdownView {
    pub(crate) theme: Theme,
    pub(crate) application: Entity<DownloadApplication>,
    pub(crate) correlation_id: uuid::Uuid,
    pub(crate) remaining_seconds: u64,
    pub(crate) executing: bool,
}

impl DownloadShutdownCountdownView {
    pub(crate) fn new(
        application: Entity<DownloadApplication>,
        correlation_id: uuid::Uuid,
        remaining_seconds: u64,
    ) -> Self {
        Self {
            theme: Theme::zed_dark(),
            application,
            correlation_id,
            remaining_seconds,
            executing: false,
        }
    }
}

impl Render for DownloadShutdownCountdownView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let correlation_id = self.correlation_id;
        let cancel_application = self.application.clone();
        let close_application = self.application.clone();
        let execute_application = self.application.clone();
        let titlebar = client_titlebar(
            theme,
            "download-shutdown-titlebar",
            "download-shutdown-close",
            "自动关机",
            move |_, _, cx| {
                let application = close_application.clone();
                cx.defer(move |cx| {
                    application.update(cx, |application, cx| {
                        application.cancel_shutdown_countdown(correlation_id, "window_closed", cx);
                    });
                });
            },
        );
        let status_badge_size = 76.0_f32;
        let body = div()
            .size_full()
            .px(px(SPACING_16))
            .py(px(SPACING_12))
            .flex()
            .items_center()
            .gap(px(SPACING_16))
            .child(
                div()
                    .size(px(status_badge_size))
                    .rounded(px(status_badge_size / 2.0))
                    .border_1()
                    .border_color(theme.danger)
                    .bg(theme.surface_raised)
                    .flex()
                    .items_end()
                    .justify_center()
                    .pb(px(SPACING_16))
                    .child(
                        div()
                            .text_size(px(TEXT_STATUS_GLYPH))
                            .font_weight(FontWeight::BOLD)
                            .text_color(theme.danger)
                            .child(if self.executing {
                                "…".into()
                            } else {
                                format!("{:02}", self.remaining_seconds)
                            }),
                    ),
            )
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(SPACING_6))
                    .child(
                        div()
                            .text_size(px(TEXT_TITLE))
                            .line_height(px(LINE_HEIGHT_TITLE))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.danger)
                            .child(if self.executing {
                                "正在请求 Windows 关机"
                            } else {
                                "所有下载已完成，即将关机"
                            }),
                    )
                    .child(
                        div()
                            .text_size(px(TEXT_LABEL))
                            .line_height(px(LINE_HEIGHT_LABEL))
                            .text_color(theme.text_muted)
                            .child("这是一次性操作。取消、新建下载或重新启动 NxTools 都不会恢复该计划。"),
                    ),
            )
            .into_any_element();
        let footer = dialog_footer(
            div()
                .flex()
                .items_center()
                .gap(px(SPACING_8))
                .child(
                    action_button("download-shutdown-now-button", "立即关机")
                        .debug_selector(|| "download-shutdown-danger-now-action".into())
                        .custom(danger_hover_button_variant(theme, cx))
                        .disabled(self.executing)
                        .on_click(move |_, _, cx| {
                            let application = execute_application.clone();
                            cx.defer(move |cx| {
                                application.update(cx, |application, cx| {
                                    application.execute_shutdown(
                                        correlation_id,
                                        "user_requested",
                                        cx,
                                    );
                                });
                            });
                        }),
                )
                .child(
                    action_button("download-shutdown-cancel-button", "取消")
                        .debug_selector(|| "download-shutdown-secondary-cancel-action".into())
                        .outline()
                        .disabled(self.executing)
                        .on_click(move |_, _, cx| {
                            let application = cancel_application.clone();
                            cx.defer(move |cx| {
                                application.update(cx, |application, cx| {
                                    application.cancel_shutdown_countdown(
                                        correlation_id,
                                        "user_cancelled",
                                        cx,
                                    );
                                });
                            });
                        }),
                )
                .into_any_element(),
        );
        dialog_shell(
            theme,
            DialogShellIds {
                root: "download-shutdown-dialog-shell",
                titlebar: "download-shutdown-dialog-titlebar",
                body: "download-shutdown-dialog-body",
                footer: "download-shutdown-dialog-footer",
            },
            titlebar,
            body,
            footer,
        )
    }
}
