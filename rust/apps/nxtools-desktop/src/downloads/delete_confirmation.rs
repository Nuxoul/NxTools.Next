use super::*;

#[allow(
    dead_code,
    reason = "retained for the Phase 3 task context-menu delete action"
)]
pub(crate) struct DeleteDownloadView {
    pub(crate) theme: Theme,
    pub(crate) application: Entity<DownloadApplication>,
    pub(crate) task_id: DownloadTaskId,
    pub(crate) path: String,
}

impl Render for DeleteDownloadView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let task_id = self.task_id;
        let close_application = self.application.clone();
        let cancel_application = self.application.clone();
        let confirm_application = self.application.clone();
        let path = self.path.clone();
        let titlebar = client_titlebar(
            theme,
            "download-delete-titlebar",
            "download-delete-close",
            "删除本地文件",
            move |_, _, cx| {
                let application = close_application.clone();
                cx.defer(move |cx| {
                    application.update(cx, |application, cx| {
                        application.close_delete_confirmation(task_id, cx);
                    });
                });
            },
        );
        let body = div()
            .size_full()
            .px(px(SPACING_16))
            .py(px(SPACING_12))
            .flex()
            .flex_col()
            .gap(px(SPACING_6))
            .child(
                div()
                    .text_size(px(TEXT_BODY))
                    .line_height(px(LINE_HEIGHT_BODY))
                    .font_weight(FontWeight::MEDIUM)
                    .child("将同时移除任务记录，且无法撤销。"),
            )
            .child(
                div()
                    .text_size(px(TEXT_LABEL))
                    .line_height(px(LINE_HEIGHT_LABEL))
                    .text_color(theme.text_muted)
                    .truncate()
                    .child(path),
            )
            .into_any_element();
        let footer = dialog_footer(
            div()
                .flex()
                .items_center()
                .gap(px(SPACING_8))
                .child(
                    action_button("download-delete-cancel", "取消")
                        .debug_selector(|| "download-delete-secondary-cancel-action".into())
                        .outline()
                        .on_click(move |_, _, cx| {
                            let application = cancel_application.clone();
                            cx.defer(move |cx| {
                                application.update(cx, |application, cx| {
                                    application.close_delete_confirmation(task_id, cx);
                                });
                            });
                        }),
                )
                .child(
                    action_button("download-delete-confirm", "删除文件")
                        .icon(Icon::new(MaterialIcon::Delete).size(px(14.0)))
                        .debug_selector(|| "download-delete-danger-confirm-action".into())
                        .custom(danger_hover_button_variant(theme, cx))
                        .on_click(move |_, _, cx| {
                            let application = confirm_application.clone();
                            cx.defer(move |cx| {
                                application.update(cx, |application, cx| {
                                    application.confirm_delete_artifact(task_id, cx);
                                });
                            });
                        }),
                )
                .into_any_element(),
        );
        dialog_shell(
            theme,
            DialogShellIds {
                root: "download-delete-dialog-shell",
                titlebar: "download-delete-dialog-titlebar",
                body: "download-delete-dialog-body",
                footer: "download-delete-dialog-footer",
            },
            titlebar,
            body,
            footer,
        )
    }
}
