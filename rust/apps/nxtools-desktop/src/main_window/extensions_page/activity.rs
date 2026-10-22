//! Bounded session activity, not a second execution/notification owner.
use super::*;
use crate::extensions::manual_runs::ManualState;
use gpui_kit::component::{
    ActiveTheme, Disableable, Sizable, Size, WindowExt,
    dialog::{DialogClose, DialogFooter},
    empty::{Empty, EmptyDescription, EmptyHeader, EmptyTitle},
    list::ListItem,
    tag::Tag,
};
impl ExtensionManagerView {
    pub(super) fn activity_button(&self) -> Button {
        let owner = self.application.clone();
        Button::new("extension-recent-activity")
            .label("最近活动")
            .outline()
            .with_size(Size::Medium)
            .on_click(move |_, window, cx| {
                let owner = owner.clone();
                let view = cx.new(|cx| {
                    let subscription = cx.observe(&owner, |_, _, cx| cx.notify());
                    ActivityView {
                        owner,
                        _subscription: subscription,
                    }
                });
                window.open_dialog(cx, move |dialog, window, _| {
                    let view = view.clone();
                    dialog
                        .title("最近手动调用")
                        .margin_top(px(8.0))
                        .w((window.viewport_size().width - px(24.0)).min(px(480.0)))
                        .content(move |content, _, _| content.min_h_0().child(view.clone()))
                        .footer(DialogFooter::new().child(div().flex_none().w_auto().child(
                            DialogClose::new().trigger(|button| {
                                button.label("关闭").outline().with_size(Size::Medium)
                            }),
                        )))
                });
            })
    }
}
struct ActivityView {
    owner: Entity<ExtensionApplication>,
    _subscription: Subscription,
}
impl Render for ActivityView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let runs = self.owner.read(cx).recent_manual_runs();
        let mut body = div()
            .id("extension-session-activity")
            .debug_selector(|| "extension-session-activity".into())
            .max_h((window.viewport_size().height - px(160.0)).max(px(64.0)))
            .min_w_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_2()
            .text_sm()
            .child(
                div()
                    .flex_shrink_0()
                    .text_color(cx.theme().muted_foreground)
                    .child("仅本次会话的近期手动调用，关闭程序后清空。"),
            );
        if runs.is_empty() {
            body = body.child(
                div()
                    .debug_selector(|| "extension-activity-empty".into())
                    .child(
                        Empty::new().p_3().header(
                            EmptyHeader::new()
                                .title(EmptyTitle::new().child("还没有手动调用"))
                                .description(
                                    EmptyDescription::new().child("启用插件不会自动执行手动动作。"),
                                ),
                        ),
                    ),
            );
        }
        for run in runs {
            let owner = self.owner.clone();
            let id = run.id;
            let status = match &run.state {
                ManualState::Running => Tag::info().child("执行中"),
                ManualState::Cancelling => Tag::warning().child("正在取消"),
                ManualState::Succeeded => Tag::success().child("插件报告完成"),
                ManualState::NotHandled => Tag::secondary().child("未处理 / 无需修改"),
                ManualState::Cancelled => Tag::secondary().child("已取消"),
                ManualState::Failed(_) => Tag::danger().child("失败 / 结果待确认"),
            };
            let elapsed = run
                .elapsed_ms
                .map_or_else(|| "等待结束".to_owned(), |ms| format!("{ms} 毫秒"));
            let mut actions = div().flex().flex_wrap().items_center().gap_2().child(
                Button::new(format!("extension-activity-{id}"))
                    .label("查看调用")
                    .outline()
                    .with_size(Size::Medium)
                    .on_click(move |_, window, cx| {
                        window.close_dialog(cx);
                        crate::extensions::manual_actions::show_run(
                            owner.clone(),
                            id,
                            None,
                            window,
                            cx,
                        );
                    }),
            );
            if run.state.pending() {
                let owner = self.owner.downgrade();
                actions = actions.child(
                    Button::new(format!("extension-activity-cancel-{id}"))
                        .label(if matches!(run.state, ManualState::Cancelling) {
                            "正在取消…"
                        } else {
                            "取消这次调用"
                        })
                        .outline()
                        .with_size(Size::Medium)
                        .disabled(matches!(run.state, ManualState::Cancelling))
                        .on_click(move |_, _, cx| {
                            let owner = owner.clone();
                            cx.defer(move |cx| {
                                let _ = owner.update(cx, |app, cx| app.cancel_manual_run(id, cx));
                            });
                        }),
                );
            }
            body = body.child(
                ListItem::new(format!("extension-activity-row-{id}"))
                    .accessibility_label(format!(
                        "{}，{}，{}，已保存 {} 个结果，{elapsed}",
                        run.package_name,
                        run.action_name,
                        run.state.label(),
                        run.items.len(),
                    ))
                    .w_full()
                    .min_w_0()
                    .flex_shrink_0()
                    .py_3()
                    .child(
                        div()
                            .w_full()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .text_sm()
                            .whitespace_normal()
                            .child(format!("{} · {}", run.package_name, run.action_name))
                            .child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .items_center()
                                    .gap_2()
                                    .child(status.small())
                                    .child(format!("已保存 {} 个 · {elapsed}", run.items.len())),
                            )
                            .child(actions),
                    ),
            );
        }
        body
    }
}
