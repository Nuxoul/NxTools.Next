use super::*;
use gpui_kit::component::{
    alert::Alert,
    form::{Field, Form},
    input::Input,
    label::Label,
    list::ListItem,
    progress::Progress,
    scroll::ScrollableElement,
};
use gpui_kit::{Render, div};

fn parts(preview: &RawJoinPreview, id: &'static str) -> impl IntoElement {
    div()
        .id(id)
        .max_h(px(160.))
        .overflow_y_scrollbar()
        .children(preview.parts.iter().enumerate().map(|(index, part)| {
            ListItem::new((id, index)).child(
                div()
                    .w_full()
                    .min_w_0()
                    .flex()
                    .gap_2()
                    .child(Label::new(format!("{}", index + 1)))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(part.name.clone()),
                    )
                    .child(Label::new(format!("{} B", part.length))),
            )
        }))
}
pub(super) fn confirmation(preview: &RawJoinPreview, output: &str) -> impl IntoElement {
    div()
        .id("raw-join-confirmation-content")
        .max_h(px(280.))
        .overflow_y_scrollbar()
        .flex()
        .flex_col()
        .gap_3()
        .child(Alert::warning("raw-join-confirm-warning", LIMITATION))
        .child(Label::new(format!(
            "将按下列顺序连接 {} 个文件，共 {} 字节",
            preview.parts.len(),
            preview.total_bytes
        )))
        .child(parts(preview, "raw-join-confirm-parts"))
        .child(div().text_sm().child(format!("新输出文件：{output}")))
        .child(Label::new(
            "只创建一个新文件；不会覆盖现有文件，也不会删除源文件。",
        ))
}
pub(super) fn terminal_message(terminal: &RawJoinTerminal) -> String {
    if terminal.outcome == "succeeded" {
        return format!("合并完成：{} 字节。源分卷已保留。", terminal.bytes_written);
    }
    if !terminal.cleanup_complete {
        return "合并未完成，清理失败；请检查所选输出位置中的残留文件。源分卷已保留。".into();
    }
    if terminal.outcome == "cancelled" {
        return "已取消；本次未完成输出已清理，源分卷已保留。".into();
    }
    let reason = match terminal.code.as_deref() {
        Some("missing_part") => "编号中间存在缺卷",
        Some("ambiguous_family" | "invalid_family") => {
            "编号 family 无效或存在歧义；仅支持普通连续 .001 编号"
        }
        Some("source_changed") => "源分卷在预览后发生变化，请重新发现",
        Some("output_is_part") => "输出不能属于源编号 family，请更换文件名",
        Some("destination_missing" | "invalid_root") => "输出父目录不存在或不受支持",
        Some("insufficient_space") => "输出磁盘空间不足",
        Some("family_limit" | "length_limit") => "分卷数量、名称或总长度超过限制",
        Some("write_file" | "file_io") => "读写失败，请检查路径、权限与可用空间",
        _ => "原生端拒绝或未能完成操作，请检查源文件及新输出位置",
    };
    let native = terminal
        .native_error_code
        .map(|n| format!("（Windows 错误 {n}）"))
        .unwrap_or_default();
    format!("{reason}{native}。本次未完成输出已清理，源分卷已保留。")
}
impl Render for JoinWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let locked = self.locked();
        let working = self.working();
        let phase = match self.phase {
            Phase::Inspecting => "正在读取同名编号文件…",
            Phase::Running => "正在连接文件…",
            Phase::Cancelling => "正在取消，等待实际完成…",
            Phase::Preview | Phase::Confirming => "请核对文件列表与输出位置",
            Phase::Terminal => "操作已结束",
            Phase::Disconnected => "平台连接已断开",
            Phase::Idle => "选择一个编号文件后查看列表",
        };
        let fields = Form::vertical()
            .with_size(Size::Medium)
            .child(
                Field::new()
                    .label("要合并的分卷文件")
                    .description("选择同一目录中同名的 .001 / .002 / … 文件之一；.z01 不适用。")
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .min_w_0()
                            .child(
                                div()
                                    .debug_selector(|| "raw-join-source".into())
                                    .flex_1()
                                    .min_w_0()
                                    .child(
                                        Input::new(&self.source)
                                            .with_size(Size::Medium)
                                            .disabled(locked),
                                    ),
                            )
                            .child(
                                Button::new("raw-join-pick-source")
                                    .debug_selector(|| "raw-join-pick-source".into())
                                    .label("选择…")
                                    .outline()
                                    .with_size(Size::Medium)
                                    .disabled(locked)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.pick(ArchivePickerPurpose::Files, window, cx)
                                    })),
                            ),
                    ),
            )
            .child(
                Field::new()
                    .label("新输出文件")
                    .description("父目录必须已存在。不会覆盖现有文件，也不会删除源分卷。")
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .min_w_0()
                            .child(
                                div()
                                    .debug_selector(|| "raw-join-output".into())
                                    .flex_1()
                                    .min_w_0()
                                    .child(
                                        Input::new(&self.output)
                                            .with_size(Size::Medium)
                                            .disabled(locked),
                                    ),
                            )
                            .child(
                                Button::new("raw-join-pick-output")
                                    .debug_selector(|| "raw-join-pick-output".into())
                                    .label("另存为…")
                                    .outline()
                                    .with_size(Size::Medium)
                                    .disabled(locked)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.pick(ArchivePickerPurpose::Save, window, cx)
                                    })),
                            ),
                    ),
            );
        div()
            .size_full()
            .min_h_0()
            .min_w_0()
            .flex()
            .flex_col()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(crate::ui::components::titlebar::controlled_titlebar(
                "raw-join-titlebar",
                "raw-join-caption-close",
                "合并分卷文件（.001）",
                crate::ui::components::titlebar::CaptionCapabilities::RESIZABLE,
                cx.listener(|this, _, window, cx| this.close(window, cx)),
            ))
            .child(
                div()
                    .id("raw-join-viewport")
                    .debug_selector(|| "raw-join-viewport".into())
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .overflow_hidden()
                    .child(
                        div()
                            .id("raw-join-scroll")
                            .size_full()
                            .overflow_y_scrollbar()
                            .child(
                                div()
                                    .p_4()
                                    .flex()
                                    .flex_col()
                                    .gap_4()
                                    .child(
                                        Alert::info("raw-join-purpose", LIMITATION)
                                            .with_size(Size::Medium),
                                    )
                                    .child(fields)
                                    .when_some(self.preview.as_ref(), |body, preview| {
                                        body.child(Label::new(format!(
                                            "文件列表：{} 个文件 · {} 字节 · 将按下列顺序连接",
                                            preview.parts.len(),
                                            preview.total_bytes
                                        )))
                                        .child(parts(preview, "raw-join-parts"))
                                    })
                                    .when_some(self.message.clone(), |body, message| {
                                        let success = self
                                            .terminal
                                            .as_ref()
                                            .is_some_and(|t| t.outcome == "succeeded");
                                        body.child(if success {
                                            Alert::success("raw-join-result", message)
                                        } else {
                                            Alert::warning("raw-join-result", message)
                                        })
                                    }),
                            ),
                    ),
            )
            .child(
                div()
                    .id("raw-join-footer")
                    .debug_selector(|| "raw-join-footer".into())
                    .flex_none()
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .when(working, |footer| {
                        footer.child(
                            Progress::new("raw-join-progress")
                                .loading(true)
                                .accessibility_label("等待实际操作完成"),
                        )
                    })
                    .child(div().text_sm().child(phase))
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                Button::new("raw-join-close")
                                    .debug_selector(|| "raw-join-close".into())
                                    .label("关闭")
                                    .outline()
                                    .with_size(Size::Medium)
                                    .disabled(
                                        self.phase == Phase::Confirming || self.picker.is_some(),
                                    )
                                    .on_click(
                                        cx.listener(|this, _, window, cx| this.close(window, cx)),
                                    ),
                            )
                            .when(working, |row| {
                                row.child(
                                    Button::new("raw-join-cancel")
                                        .debug_selector(|| "raw-join-cancel".into())
                                        .label("取消合并")
                                        .outline()
                                        .with_size(Size::Medium)
                                        .disabled(
                                            self.phase == Phase::Cancelling
                                                && self.cancel.is_some(),
                                        )
                                        .on_click(cx.listener(|this, _, _, cx| this.cancel(cx))),
                                )
                            })
                            .when(!working, |row| {
                                row.child(
                                    Button::new("raw-join-inspect")
                                        .debug_selector(|| "raw-join-inspect".into())
                                        .label("查看文件列表")
                                        .outline()
                                        .with_size(Size::Medium)
                                        .disabled(locked)
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.inspect(window, cx)
                                        })),
                                )
                                .child(
                                    Button::new("raw-join-review")
                                        .debug_selector(|| "raw-join-review".into())
                                        .label("确认并合并…")
                                        .primary()
                                        .with_size(Size::Medium)
                                        .disabled(self.phase != Phase::Preview || locked)
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.confirm(window, cx)
                                        })),
                                )
                            }),
                    ),
            )
    }
}
