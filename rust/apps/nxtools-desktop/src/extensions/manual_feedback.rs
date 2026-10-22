//! One observer owns the scroll body AND fixed actions; a terminal update repaints both.
use super::{ClipboardOrigin, ExtensionApplication};
use crate::extensions::manual_runs::{ManualRun, ManualState};
use gpui_kit::component::{
    ActiveTheme, Disableable, Sizable, Size, WindowExt,
    alert::Alert,
    button::Button,
    dialog::{DialogClose, DialogFooter},
};
use gpui_kit::{
    Context, Entity, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, Subscription, Window, div, px,
};

pub(super) struct ManualFeedback {
    pub(super) owner: Entity<ExtensionApplication>,
    pub(super) id: uuid::Uuid,
    pub(super) origin: Option<ClipboardOrigin>,
    pub(super) selected_result: usize,
    pub(super) _subscription: Subscription,
    pub(super) _origin_subscription: Option<Subscription>,
}

impl ManualFeedback {
    fn footer(
        &self,
        run: Option<&ManualRun>,
        origin: Option<ClipboardOrigin>,
        cx: &mut Context<Self>,
    ) -> DialogFooter {
        let mut actions = div().flex().items_center().justify_end().gap_2();
        if let Some(run) = run.filter(|run| run.state.pending()) {
            let owner = self.owner.downgrade();
            let id = self.id;
            actions = actions.child(
                Button::new("extension-cancel-this-run")
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
        actions = actions.child(div().flex_none().w_auto().child(DialogClose::new().trigger(
            |button| {
                button
                    .debug_selector(|| "extension-feedback-close".into())
                    .label("关闭")
                    .outline()
                    .with_size(Size::Medium)
            },
        )));
        let mut footer = DialogFooter::new().flex_col().items_stretch().gap_2();
        if let Some(run) = run.filter(|run| !run.items.is_empty()) {
            let index = self.selected_result.min(run.items.len() - 1);
            let item = run.items[index];
            let id = self.id;
            let mut results = div().flex().flex_col().items_end().gap_2();
            if run.items.len() > 1 {
                results = results.child(
                    div()
                        .w_full()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(
                            Button::new("extension-result-previous")
                                .label("上一项")
                                .outline()
                                .with_size(Size::Small)
                                .disabled(index == 0)
                                .on_click(cx.listener(|view, _, _, cx| {
                                    view.selected_result = view.selected_result.saturating_sub(1);
                                    cx.notify();
                                })),
                        )
                        .child(format!("{} / {}", index + 1, run.items.len()))
                        .child(
                            Button::new("extension-result-next")
                                .label("下一项")
                                .outline()
                                .with_size(Size::Small)
                                .disabled(index + 1 == run.items.len())
                                .on_click(cx.listener(|view, _, _, cx| {
                                    view.selected_result += 1;
                                    cx.notify();
                                })),
                        ),
                );
            }
            results = results.child(
                Button::new(format!("extension-view-result-{index}"))
                    .label("查看结果")
                    .outline()
                    .with_size(Size::Medium)
                    .disabled(origin.is_none())
                    .on_click(move |_, window, cx| {
                        let Some(origin) = origin.clone().filter(|origin| {
                            origin.panel.upgrade().is_some_and(|panel| {
                                panel.read(cx).extension_origin_current(origin.session_id)
                            })
                        }) else {
                            return;
                        };
                        window.close_dialog(cx);
                        let source_window = origin.window;
                        cx.defer(move |cx| {
                            let _ = source_window.update(cx, |_, window, cx| {
                                let _ = origin.panel.update(cx, |panel, cx| {
                                    panel.locate_extension_result(
                                        origin.session_id,
                                        id,
                                        item,
                                        window,
                                        cx,
                                    );
                                });
                            });
                        });
                    }),
            );
            footer = footer.child(results);
        }
        footer.child(actions)
    }
}

impl Render for ManualFeedback {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let run = self.owner.read(cx).manual_run(self.id);
        let origin = self.origin.clone().filter(|origin| {
            origin
                .panel
                .upgrade()
                .is_some_and(|panel| panel.read(cx).extension_origin_current(origin.session_id))
        });
        let has_results = run.as_ref().is_some_and(|run| !run.items.is_empty());
        let body_budget = (window.viewport_size().height
            - px(if has_results { 240.0 } else { 170.0 }))
        .clamp(px(48.0), px(300.0));
        let mut body = div()
            .id("extension-manual-scroll")
            .debug_selector(|| "extension-manual-scroll".into())
            .w_full()
            .min_w_0()
            .min_h_0()
            .max_h(body_budget)
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_2()
            .text_sm()
            .whitespace_normal();
        if let Some(run) = &run {
            body = body
                .child(
                    div()
                        .flex_shrink_0()
                        .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                        .child(run.action_name.clone()),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("{} · {}", run.package_name, run.state.label())),
                );
            let guidance = match &run.state {
                ManualState::Running => Some("正在处理，关闭详情不会取消。"),
                ManualState::Cancelling => Some("正在等待取消完成；已产生的效果不能撤回。"),
                ManualState::Cancelled => Some("已取消，已产生的效果不会撤回。"),
                ManualState::Failed(_) => Some("结果未确认，请检查后再决定是否重试。"),
                ManualState::Succeeded | ManualState::NotHandled => None,
            };
            if let Some(guidance) = guidance {
                body = body.child(div().flex_shrink_0().child(guidance));
            }
            if has_results {
                body = body.child(
                    Alert::success(
                        "extension-manual-saved",
                        format!("已确认保存 {} 个历史结果，原文保留。", run.items.len()),
                    )
                    .small()
                    .flex_shrink_0(),
                );
                if origin.is_none() {
                    body = body.child(
                        div()
                            .flex_shrink_0()
                            .child("原面板会话已结束，请手动打开历史查看。"),
                    );
                }
            }
            if let Some(message) = &run.message {
                body = body.child(
                    div()
                        .flex_shrink_0()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_color(cx.theme().muted_foreground)
                                .child("插件说明"),
                        )
                        .child(message.clone()),
                );
            }
            if let Some(elapsed) = run.elapsed_ms {
                body = body.child(
                    div()
                        .flex_shrink_0()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("用时 {elapsed} 毫秒")),
                );
            }
        } else {
            body = body.child("此调用已不在最近活动中。");
        }
        let footer = self.footer(run.as_ref(), origin, cx);
        div()
            .id("extension-manual-feedback")
            .debug_selector(|| "extension-manual-feedback".into())
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_3()
            .child(body)
            .child(
                div()
                    .debug_selector(|| "extension-manual-footer".into())
                    .flex_shrink_0()
                    .w_full()
                    .child(footer),
            )
    }
}
