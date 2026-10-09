use super::*;
use crate::ui::metrics::ICON_GLYPH_SMALL;

pub(crate) fn notification_transition(delta: f32, exiting: bool) -> (f32, f32) {
    if exiting {
        (1.0 - delta, -4.0 * delta)
    } else {
        (delta, -6.0 * (1.0 - delta))
    }
}

impl MainWindow {
    pub(crate) fn render_notifications(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let notifications = self.notifications.read(cx).items().to_vec();
        let mut stack = div()
            .id("notification-stack")
            .debug_selector(|| "notification-stack".into())
            .absolute()
            .right(px(SPACING_12))
            .bottom(px(SPACING_12))
            .w(px(NOTIFICATION_CARD_WIDTH))
            .flex()
            .flex_col()
            .gap(px(SPACING_8));
        for notification in notifications {
            if notification.exiting && crate::ui::motion::reduced_motion(cx) {
                continue;
            }
            let accent = match notification.level {
                NotificationLevel::Progress | NotificationLevel::Info => theme.accent,
                NotificationLevel::Success => theme.success,
                NotificationLevel::Error => theme.danger,
            };
            let status_icon = match notification.level {
                NotificationLevel::Progress => MaterialIcon::ProgressActivity,
                NotificationLevel::Info => MaterialIcon::Info,
                NotificationLevel::Success => MaterialIcon::Check,
                NotificationLevel::Error => MaterialIcon::Cancel,
            };
            let notification_id = notification.id.clone();
            let notification_generation = notification.generation();
            let notification_exiting = notification.exiting;
            let dismiss_id = notification.id.clone();
            let notification_center = self.notifications.clone();
            let title_selector = format!("notification-title-{}", notification_id);
            let message_selector = format!("notification-message-{}", notification_id);
            let mut body = div()
                .min_w_0()
                .flex_1()
                .pr(px(SPACING_24))
                .flex()
                .flex_col()
                .gap(px(SPACING_4))
                .child(
                    div()
                        .debug_selector(move || title_selector.clone())
                        .text_size(px(TEXT_BODY))
                        .line_height(px(LINE_HEIGHT_BODY))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.text)
                        .child(notification.title),
                )
                .child(
                    div()
                        .debug_selector(move || message_selector.clone())
                        .text_size(px(TEXT_LABEL))
                        .line_height(px(LINE_HEIGHT_LABEL))
                        .text_color(theme.text_secondary)
                        .child(notification.message),
                );
            for (detail_index, detail) in notification.details.into_iter().enumerate() {
                let detail_tooltip = detail.value.clone();
                let detail_id = gpui::SharedString::from(format!(
                    "notification-detail-{}-{detail_index}",
                    notification_id
                ));
                body = body.child(
                    div()
                        .pt(px(SPACING_4))
                        .border_t_1()
                        .border_color(theme.border_subtle)
                        .flex()
                        .items_center()
                        .gap(px(SPACING_8))
                        .child(
                            div()
                                .text_size(px(TEXT_META))
                                .line_height(px(LINE_HEIGHT_META))
                                .text_color(theme.text_muted)
                                .child(detail.label),
                        )
                        .child(
                            div()
                                .id(detail_id)
                                .min_w_0()
                                .flex_1()
                                .truncate()
                                .font_family(UI_MONO_FONT_FAMILY)
                                .text_size(px(TEXT_LABEL))
                                .line_height(px(LINE_HEIGHT_LABEL))
                                .text_color(theme.text_secondary)
                                .tooltip(move |window, cx| {
                                    Tooltip::new(detail_tooltip.clone()).build(window, cx)
                                })
                                .child(detail.value),
                        ),
                );
            }
            if let Some(action) = notification.action {
                let action_id = notification_id.clone();
                let action_dismiss_id = notification_id.clone();
                let action_center = self.notifications.clone();
                let action_label = action.label.clone();
                let action_selector = format!("notification-action-{}", action_id);
                body = body.child(
                    div().flex().justify_end().child(
                        div()
                            .debug_selector(move || action_selector.clone())
                            .h(px(CONTROL_HEIGHT_ACTION))
                            .child(
                                Button::new(gpui::SharedString::from(format!(
                                    "notification-action-{}",
                                    action_id
                                )))
                                .label(action_label)
                                .icon(Icon::new(MaterialIcon::OpenInNew).size(px(12.0)))
                                .ghost()
                                .small()
                                .h_full()
                                .px(px(SPACING_8))
                                .rounded(px(RADIUS_CONTROL))
                                .on_click(
                                    move |_event, window, cx| {
                                        action.invoke(window, cx);
                                        let action_center = action_center.clone();
                                        let action_dismiss_id = action_dismiss_id.clone();
                                        cx.defer(move |cx| {
                                            action_center.update(cx, |notifications, cx| {
                                                notifications.dismiss_generation(
                                                    action_dismiss_id.clone(),
                                                    notification_generation,
                                                    cx,
                                                );
                                            });
                                        });
                                        cx.stop_propagation();
                                    },
                                ),
                            ),
                    ),
                );
            }
            let card_selector = format!("notification-{}", notification_id);
            let marker_selector = format!("notification-marker-{}", notification_id);
            let dismiss_selector = format!("dismiss-notification-{}", notification_id);
            let card = div()
                .id(gpui::SharedString::from(format!(
                    "notification-{}",
                    notification_id
                )))
                .debug_selector(move || card_selector.clone())
                .w_full()
                .relative()
                .min_h(px(56.0))
                .max_h(px(NOTIFICATION_CARD_MAX_HEIGHT))
                .rounded(px(RADIUS_CONTROL))
                .border_1()
                .border_color(theme.border_subtle)
                .bg(theme.surface_raised)
                .shadow_sm()
                .overflow_hidden()
                .pl(px(SPACING_12))
                .pr(px(SPACING_8))
                .py(px(SPACING_4))
                .flex()
                .items_start()
                .gap(px(SPACING_8))
                .occlude()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(|_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .debug_selector(move || marker_selector.clone())
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left_0()
                        .w(px(3.0))
                        .bg(accent),
                )
                .child(
                    div()
                        .size(px(16.0))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(Icon::new(status_icon).size(px(14.0)).text_color(accent)),
                )
                .child(body)
                .child(
                    div()
                        .debug_selector(move || dismiss_selector.clone())
                        .absolute()
                        .top(px(SPACING_4))
                        .right(px(SPACING_4))
                        .size(px(CONTROL_HEIGHT_ACTION))
                        .child(
                            Button::new(gpui::SharedString::from(format!(
                                "dismiss-notification-{}",
                                notification_id
                            )))
                            .icon(Icon::new(MaterialIcon::Close).size(px(ICON_GLYPH_SMALL)))
                            .tooltip("关闭通知")
                            .ghost()
                            .small()
                            .size(px(CONTROL_HEIGHT_ACTION))
                            .rounded(px(RADIUS_CONTROL))
                            .on_click(move |_event, _window, cx| {
                                let notification_center = notification_center.clone();
                                let dismiss_id = dismiss_id.clone();
                                cx.defer(move |cx| {
                                    notification_center.update(cx, |notifications, cx| {
                                        notifications.dismiss_generation(
                                            dismiss_id.clone(),
                                            notification_generation,
                                            cx,
                                        );
                                    });
                                });
                            }),
                        ),
                );
            let transition_duration = if notification_exiting {
                NOTIFICATION_EXIT_DURATION
            } else {
                NOTIFICATION_ENTER_DURATION
            };
            if crate::ui::motion::motion_allowed(cx) {
                stack = stack.child(card.with_animation(
                    gpui::SharedString::from(format!(
                        "notification-transition-{}-{}-{}",
                        notification_id, notification_generation, notification_exiting
                    )),
                    Animation::new(transition_duration).with_easing(ease_in_out),
                    move |card, delta| {
                        let (opacity, vertical_offset) =
                            notification_transition(delta, notification_exiting);
                        card.opacity(opacity).relative().top(px(vertical_offset))
                    },
                ));
            } else {
                stack = stack.child(card);
            }
        }
        stack.into_any_element()
    }
}
