use super::*;
use gpui_kit::component::alert::Alert;
use gpui_kit::component::dialog::{DialogContent, DialogFooter};

pub(crate) struct DownloadShutdownCountdownView {
    pub(crate) theme: Theme,
    pub(crate) application: Entity<DownloadApplication>,
    pub(crate) correlation_id: uuid::Uuid,
    pub(crate) remaining_seconds: u64,
    pub(crate) executing: bool,
    pub(crate) action: nxtools_download::DownloadCompletionAction,
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
            action: nxtools_download::DownloadCompletionAction::Shutdown,
        }
    }
}

impl Render for DownloadShutdownCountdownView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::theme::observe_theme(window, cx);
        self.theme = Theme::active(cx);
        let theme = self.theme;
        let correlation_id = self.correlation_id;
        let action_label = completion_action_label(self.action);
        let escape_application = self.application.clone();
        let cancel_application = self.application.clone();
        let close_application = self.application.clone();
        let execute_application = self.application.clone();
        let titlebar = client_titlebar(
            theme,
            "download-shutdown-titlebar",
            "download-shutdown-close",
            format!("完成后{action_label}"),
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
        let body = DialogContent::new().size_full().min_w_0()
            .px(px(SPACING_16)).py(px(SPACING_8))
            .child(div().flex().items_center().gap(px(SPACING_16))
            .child(
                div()
                    .size(px(status_badge_size))
                    .flex_none()
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
                    .gap(px(SPACING_8))
                    .child(
                        div()
                            .debug_selector(|| "download-shutdown-heading".into())
                            .w_full()
                            .text_size(px(TEXT_TITLE))
                            .line_height(px(LINE_HEIGHT_TITLE))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.danger)
                            .child(if self.executing {
                                format!("正在请求{action_label}")
                            } else {
                                format!("下载已成功完成，即将{action_label}")
                            }),
                    )
                    .child(div().debug_selector(|| "download-shutdown-consequence".into())
                        .w_full().child(Alert::warning("download-shutdown-risk",
                            "取消会停止本次动作；持久化意图保留，仅供后续成功完成使用。")
                            .icon(Icon::new(MaterialIcon::Info)).xsmall())),
            ))
            .into_any_element();
        let footer = dialog_footer(
            DialogFooter::new()
                .items_center()
                .child(
                    action_button(
                        "download-shutdown-now-button",
                        format!("立即{action_label}"),
                    )
                    .debug_selector(|| "download-shutdown-danger-now-action".into())
                    .custom(danger_hover_button_variant(theme, cx))
                    .disabled(self.executing)
                    .on_click(move |_, _, cx| {
                        let application = execute_application.clone();
                        cx.defer(move |cx| {
                            application.update(cx, |application, cx| {
                                application.execute_shutdown(correlation_id, "user_requested", cx);
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
        div()
            .size_full()
            .on_key_down(move |event, _, cx| {
                if event.keystroke.key == "escape" {
                    let application = escape_application.clone();
                    cx.stop_propagation();
                    cx.defer(move |cx| {
                        application.update(cx, |app, cx| {
                            app.cancel_shutdown_countdown(correlation_id, "escape", cx)
                        })
                    });
                }
            })
            .child(dialog_shell(
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
            ))
    }
}
