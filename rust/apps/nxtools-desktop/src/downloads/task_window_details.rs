use super::*;
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::form::{Field, Form};
use gpui_kit::component::tab::{Tab, TabBar};

impl DownloadTaskWindowView {
    pub(super) fn set_details_expanded(
        &mut self,
        expanded: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let changed = self.details_expanded != expanded;
        self.details_expanded = expanded;
        cx.notify();
        if !changed {
            return;
        }
        let target = download_task_window_size_for_details(DownloadWindowPhase::Progress, expanded);
        if self.native_window_handle == 0 {
            window.resize(target);
        } else {
            let scale = window.scale_factor();
            let _ = self
                .platform
                .send_command(PlatformCommand::SetWindowBoundsCentered(
                    SetWindowBoundsCenteredCommand {
                        window_handle: self.native_window_handle,
                        width: (target.width.as_f32() * scale).round() as i32,
                        height: (target.height.as_f32() * scale).round() as i32,
                    },
                ));
        }
    }

    pub(super) fn render_detail_pages(
        &self,
        capabilities: String,
        total: Option<u64>,
        segments: &[nxtools_download::DownloadSegmentProgress],
        status: DownloadStatus,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let page = match self.details_tab {
            1 => div()
                .id("download-task-speed-page")
                .size_full()
                .min_h_0()
                .overflow_y_scroll()
                .child(self.render_speed_limit_editor(cx))
                .into_any_element(),
            2 => self.render_completion_page(cx),
            _ => div()
                .size_full()
                .min_h_0()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .id("download-window-progress-capabilities")
                        .test_support()
                        .debug_selector(|| "download-window-progress-capabilities".into())
                        .w_full()
                        .min_w_0()
                        .flex_none()
                        .text_xs()
                        .text_color(self.theme.text_secondary)
                        .truncate()
                        .aria_label(capabilities.clone())
                        .tooltip(bounded_text_tooltip(
                            "download-window-capabilities-tooltip",
                            capabilities.clone(),
                            true,
                        ))
                        .child(capabilities),
                )
                .child(super::super::range_coverage::range_coverage_slot(
                    total, segments, self.theme,
                ))
                .child(
                    div()
                        .w_full()
                        .flex_1()
                        .min_h_0()
                        .child(segment_progress_table(segments, status)),
                )
                .into_any_element(),
        };
        div()
            .id("download-window-details")
            .debug_selector(|| "download-window-details".into())
            .w_full()
            .flex_1()
            .min_h_0()
            .pt_2()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                TabBar::new("download-task-detail-tabs")
                    .underline()
                    .small()
                    .w_full()
                    .flex_none()
                    .selected_index(self.details_tab)
                    .child(Tab::new().label("连接详情"))
                    .child(Tab::new().label("限速"))
                    .child(Tab::new().label("完成后"))
                    .on_click(cx.listener(|this, index: &usize, _, cx| {
                        this.details_tab = *index;
                        cx.notify();
                    })),
            )
            .child(div().w_full().flex_1().min_h_0().pb_2().child(page))
            .into_any_element()
    }

    fn render_completion_page(&self, _: &mut Context<Self>) -> AnyElement {
        let application = self.application.clone();
        let task_id = self.task_id;
        let open_directory = self.completion_intention.open_directory;
        div()
            .id("download-task-completion-page")
            .size_full()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                Form::vertical()
                    .small()
                    .child(Field::new().label("电源或应用操作").child(
                        super::super::completion_intention_menu(
                            "download-window-completion-menu",
                            self.application.clone(),
                            super::super::CompletionIntentionTarget::Task(task_id),
                            self.completion_intention,
                        ),
                    )),
            )
            .child(
                Checkbox::new("download-task-completion-directory")
                    .label("下载成功后打开所在文件夹")
                    .checked(open_directory)
                    .small()
                    .on_click(move |_, _, cx| {
                        let application = application.clone();
                        cx.defer(move |cx| {
                            application.update(cx, |app, cx| {
                                app.save_completion_intention(
                                    super::super::CompletionIntentionTarget::Task(task_id),
                                    None,
                                    Some(!open_directory),
                                    cx,
                                );
                            })
                        });
                    }),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(self.theme.text_secondary)
                    .child("关闭窗口会保留设置。电源及退出动作执行前会显示可取消的倒计时。"),
            )
            .into_any_element()
    }
}
