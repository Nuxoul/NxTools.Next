use super::*;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::alert::Alert;
use gpui_kit::component::dialog::{DialogContent, DialogFooter};

pub(crate) struct DeleteDownloadView {
    pub(crate) theme: Theme,
    pub(crate) application: Entity<DownloadApplication>,
    pub(crate) task_id: DownloadTaskId,
    pub(crate) path: String,
    pub(super) batch: Option<BatchConfirmation>,
}

impl Render for DeleteDownloadView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::theme::observe_theme(window, cx);
        self.theme = Theme::active(cx);
        let theme = self.theme;
        let task_id = self.task_id;
        let close_application = self.application.clone();
        let cancel_application = self.application.clone();
        let confirm_application = self.application.clone();
        let path = self.path.clone();
        let batch = self.batch.clone();
        let cancelling = batch
            .as_ref()
            .is_some_and(|batch| batch.action == DownloadBatchAction::Cancel);
        let title = batch.as_ref().map_or_else(
            || "删除本地文件".to_string(),
            |batch| {
                format!(
                    "{} {} 个下载任务",
                    if cancelling { "取消" } else { "删除" },
                    batch.task_ids.len()
                )
            },
        );
        let consequence = if cancelling {
            "将停止所选任务并清理未完成的数据，且无法撤销。"
        } else {
            "将同时移除任务记录，且无法撤销。"
        };
        let path = batch.as_ref().map_or(path, |batch| {
            format!("所选 {} 个下载任务", batch.task_ids.len())
        });
        let titlebar = client_titlebar(
            theme,
            "download-delete-titlebar",
            "download-delete-close",
            title.clone(),
            move |_, _, cx| {
                let application = close_application.clone();
                cx.defer(move |cx| {
                    application.update(cx, |application, cx| {
                        application.close_delete_confirmation(task_id, cx);
                    });
                });
            },
        );
        let titlebar = div()
            .id("download-delete-labelled-titlebar")
            .role(gpui_kit::Role::Group)
            .aria_label(title)
            .test_support()
            .size_full()
            .child(titlebar)
            .into_any_element();
        let body = DialogContent::new()
            .size_full()
            .min_w_0()
            .px(px(SPACING_16))
            .py(px(SPACING_8))
            .gap_1()
            .child(
                div()
                    .debug_selector(|| "download-delete-consequence".into())
                    .child(
                        Alert::error("download-delete-risk", consequence)
                            .icon(Icon::new(MaterialIcon::Delete))
                            .xsmall(),
                    ),
            )
            .child(
                div()
                    .id("download-delete-path")
                    .debug_selector(|| "download-delete-path".into())
                    .min_w_0()
                    .w_full()
                    .text_size(px(TEXT_BODY))
                    .line_height(px(LINE_HEIGHT_BODY))
                    .text_color(theme.text_secondary)
                    .truncate()
                    .tooltip({
                        let path = path.clone();
                        move |window, cx| Tooltip::new(path.clone()).build(window, cx)
                    })
                    .child(path),
            )
            .into_any_element();
        let footer = dialog_footer(
            DialogFooter::new()
                .items_center()
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
                    action_button(
                        "download-delete-confirm",
                        if cancelling {
                            "取消任务"
                        } else {
                            "删除文件"
                        },
                    )
                    .icon(Icon::new(MaterialIcon::Delete).size(px(14.0)))
                    .debug_selector(|| "download-delete-danger-confirm-action".into())
                    .custom(danger_hover_button_variant(theme, cx))
                    .on_click(move |_, _, cx| {
                        let application = confirm_application.clone();
                        let batch = batch.clone();
                        cx.defer(move |cx| {
                            application.update(cx, |application, cx| {
                                if let Some(batch) = batch {
                                    application.confirm_batch_action(task_id, batch, cx);
                                } else {
                                    application.confirm_delete_artifact(task_id, cx);
                                }
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
