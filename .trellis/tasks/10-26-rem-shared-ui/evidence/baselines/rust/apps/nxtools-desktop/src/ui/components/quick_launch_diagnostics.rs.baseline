use std::rc::Rc;

use gpui_kit::component::Disableable;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::description_list::DescriptionList;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::{AnyElement, App, IntoElement, SharedString, Window, div, prelude::*, px};

use crate::icons::MaterialIcon;
use crate::theme::Theme;
use crate::ui::components::action::{
    action_button, danger_hover_button_variant, labelled_icon_action,
};
use crate::ui::metrics::{CONTROL_HEIGHT_INPUT, SPACING_4, SPACING_12};
use crate::ui::typography::{
    LINE_HEIGHT_WORKBENCH_BODY as LINE_HEIGHT_BODY,
    LINE_HEIGHT_WORKBENCH_SUPPORT as LINE_HEIGHT_LABEL, TEXT_WORKBENCH_BODY as TEXT_BODY,
    TEXT_WORKBENCH_SUPPORT as TEXT_LABEL, UI_MONO_FONT_FAMILY,
};

const DIAGNOSTICS_ROW_HEIGHT: f32 = 48.0;

type DiagnosticsAction = Rc<dyn Fn(&gpui_kit::ClickEvent, &mut Window, &mut App)>;

pub(crate) struct QuickLaunchDiagnosticsData {
    pub ranked_items: usize,
    pub total_activations: u64,
    pub query_associations: usize,
    pub recent_directories: usize,
    pub local_search_elapsed: SharedString,
    pub index_search_elapsed: SharedString,
    pub log_path: SharedString,
    pub log_available: bool,
}

pub(crate) struct QuickLaunchDiagnosticsActions {
    pub clear_history: DiagnosticsAction,
    pub open_logs: DiagnosticsAction,
}

pub(crate) fn exact_log_path_tooltip(path: &str) -> SharedString {
    path.to_owned().into()
}

pub(crate) fn quick_launch_diagnostics(
    theme: Theme,
    data: QuickLaunchDiagnosticsData,
    actions: QuickLaunchDiagnosticsActions,
    cx: &App,
) -> AnyElement {
    let history_is_empty = data.total_activations == 0 && data.recent_directories == 0;
    let clear_history = actions.clear_history;
    let open_logs = actions.open_logs;
    let log_tooltip = exact_log_path_tooltip(data.log_path.as_ref());
    let clear_variant = danger_hover_button_variant(theme, cx);

    div()
        .id("quick-launch-diagnostics-surface")
        .debug_selector(|| "quick-launch-diagnostics-surface".into())
        .w_full()
        .overflow_hidden()
        .border_1()
        .border_color(theme.border_subtle)
        .bg(theme.surface)
        .flex()
        .flex_col()
        .child(diagnostics_section_header(
            theme,
            "quick-launch-diagnostics-history-header",
            "排序学习",
            "只清理启动排序、关键词关联和最近目录，不影响索引与设置",
            Some(
                div()
                    .debug_selector(|| "quick-launch-clear-history".into())
                    .h(px(CONTROL_HEIGHT_INPUT))
                    .flex_none()
                    .child(
                        action_button("quick-launch-clear-history-button", "清空历史")
                            .custom(clear_variant)
                            // Official Small tier; the wrapper reserves the existing action slot.
                            .disabled(history_is_empty)
                            .on_click(move |event, window, cx| {
                                clear_history(event, window, cx);
                            }),
                    )
                    .into_any_element(),
            ),
        ))
        .child(
            div()
                .id("quick-launch-diagnostics-history-metrics")
                .debug_selector(|| "quick-launch-diagnostics-history-metrics".into())
                .w_full()
                .h(px(DIAGNOSTICS_ROW_HEIGHT))
                .flex_none()
                .border_b_1()
                .border_color(theme.border_subtle)
                .flex()
                .child(diagnostics_metric(
                    theme,
                    "quick-launch-diagnostics-ranked-items",
                    "quick-launch-diagnostics-ranked-items-value",
                    "排序项目",
                    data.ranked_items.to_string(),
                    false,
                ))
                .child(diagnostics_metric(
                    theme,
                    "quick-launch-diagnostics-activations",
                    "quick-launch-diagnostics-activations-value",
                    "启动次数",
                    data.total_activations.to_string(),
                    true,
                ))
                .child(diagnostics_metric(
                    theme,
                    "quick-launch-diagnostics-query-associations",
                    "quick-launch-diagnostics-query-associations-value",
                    "关键词关联",
                    data.query_associations.to_string(),
                    true,
                ))
                .child(diagnostics_metric(
                    theme,
                    "quick-launch-diagnostics-recent-directories",
                    "quick-launch-diagnostics-recent-directories-value",
                    "最近目录",
                    data.recent_directories.to_string(),
                    true,
                )),
        )
        .child(diagnostics_section_header(
            theme,
            "quick-launch-diagnostics-performance-header",
            "最近查询性能",
            "读取现有内存快照，不启动额外采样或轮询",
            None,
        ))
        .child(
            div()
                .id("quick-launch-diagnostics-performance-metrics")
                .debug_selector(|| "quick-launch-diagnostics-performance-metrics".into())
                .w_full()
                .h(px(DIAGNOSTICS_ROW_HEIGHT))
                .flex_none()
                .border_b_1()
                .border_color(theme.border_subtle)
                .flex()
                .child(diagnostics_metric(
                    theme,
                    "quick-launch-diagnostics-local-search",
                    "quick-launch-diagnostics-local-search-value",
                    "本地合并",
                    data.local_search_elapsed,
                    false,
                ))
                .child(diagnostics_metric(
                    theme,
                    "quick-launch-diagnostics-index-search",
                    "quick-launch-diagnostics-index-search-value",
                    "MFT / USN",
                    data.index_search_elapsed,
                    true,
                )),
        )
        .child(
            div()
                .id("quick-launch-diagnostics-log-row")
                .debug_selector(|| "quick-launch-diagnostics-log-row".into())
                .w_full()
                .h(px(DIAGNOSTICS_ROW_HEIGHT))
                .flex_none()
                .px(px(SPACING_12))
                .flex()
                .items_center()
                .gap(px(SPACING_12))
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .gap(px(SPACING_4))
                        .child(
                            div()
                                .h(px(LINE_HEIGHT_BODY))
                                .text_size(px(TEXT_BODY))
                                .line_height(px(LINE_HEIGHT_BODY))
                                .text_color(theme.text)
                                .child("运行日志"),
                        )
                        .child(
                            div()
                                .id("quick-launch-diagnostics-log-path")
                                .debug_selector(|| "quick-launch-diagnostics-log-path".into())
                                .h(px(LINE_HEIGHT_LABEL))
                                .truncate()
                                .font_family(UI_MONO_FONT_FAMILY)
                                .text_size(px(TEXT_LABEL))
                                .line_height(px(LINE_HEIGHT_LABEL))
                                .text_color(theme.text_muted)
                                .tooltip(move |window, cx| {
                                    let log_tooltip = log_tooltip.clone();
                                    Tooltip::element(move |_, _| {
                                        div()
                                            .debug_selector(|| {
                                                "quick-launch-diagnostics-log-tooltip".into()
                                            })
                                            .child(log_tooltip.clone())
                                    })
                                    .build(window, cx)
                                })
                                .child(data.log_path),
                        ),
                )
                .child(
                    div()
                        .debug_selector(|| "quick-launch-open-logs".into())
                        .h(px(CONTROL_HEIGHT_INPUT))
                        .flex_none()
                        .child(
                            labelled_icon_action(
                                "quick-launch-open-logs-button",
                                MaterialIcon::Folder,
                                "打开日志",
                            )
                            .outline()
                            .disabled(!data.log_available)
                            .on_click(move |event, window, cx| {
                                open_logs(event, window, cx);
                            }),
                        ),
                ),
        )
        .into_any_element()
}

fn diagnostics_section_header(
    theme: Theme,
    selector: &'static str,
    title: &'static str,
    description: &'static str,
    action: Option<AnyElement>,
) -> AnyElement {
    div()
        .id(selector)
        .debug_selector(move || selector.into())
        .w_full()
        .h(px(DIAGNOSTICS_ROW_HEIGHT))
        .flex_none()
        .px(px(SPACING_12))
        .border_b_1()
        .border_color(theme.border_subtle)
        .flex()
        .items_center()
        .gap(px(SPACING_12))
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .items_center()
                .gap(px(SPACING_4))
                .child(
                    div()
                        .h(px(LINE_HEIGHT_BODY))
                        .text_size(px(TEXT_BODY))
                        .line_height(px(LINE_HEIGHT_BODY))
                        .text_color(theme.text)
                        .child(title),
                )
                .child(super::tooltip::description_help(
                    format!("{selector}-description"),
                    description,
                )),
        )
        .when_some(action, |header, action| header.child(action))
        .into_any_element()
}

fn diagnostics_metric(
    theme: Theme,
    selector: &'static str,
    value_selector: &'static str,
    label: &'static str,
    value: impl Into<SharedString>,
    show_leading_divider: bool,
) -> AnyElement {
    let value = value.into();
    let full_value = value.clone();
    div()
        .id(selector)
        .debug_selector(move || selector.into())
        .min_w_0()
        .h_full()
        .flex_1()
        .px(px(SPACING_12))
        .border_l_1()
        .border_color(if show_leading_divider {
            theme.border_subtle
        } else {
            gpui_kit::transparent_black().into()
        })
        .child(
            DescriptionList::vertical().columns(1).bordered(false).item(
                label,
                div()
                    .id(value_selector)
                    .debug_selector(move || value_selector.into())
                    .min_h(px(LINE_HEIGHT_BODY))
                    .min_w_0()
                    .w_full()
                    .truncate()
                    .font_family(UI_MONO_FONT_FAMILY)
                    .text_sm()
                    .text_color(theme.text_secondary)
                    .tooltip(move |window, cx| Tooltip::new(full_value.clone()).build(window, cx))
                    .child(value)
                    .into_any_element(),
                1,
            ),
        )
        .into_any_element()
}

#[cfg(test)]
mod queue01_tests {
    use super::*;
    struct MetricFixture;
    impl gpui_kit::Render for MetricFixture {
        fn render(&mut self, _: &mut Window, _: &mut gpui_kit::Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .child(div().w(px(300.0)).h(px(48.0)).child(diagnostics_metric(
                    Theme::zed_dark(),
                    "queue01-metric",
                    "queue01-value",
                    "启动次数",
                    "123",
                    false,
                )))
                .child(
                    div()
                        .debug_selector(|| "queue01-reference".into())
                        .w(px(300.0))
                        .child(
                            DescriptionList::vertical().columns(1).bordered(false).item(
                                "启动次数",
                                div()
                                    .debug_selector(|| "queue01-reference-value".into())
                                    .min_h(px(LINE_HEIGHT_BODY))
                                    .text_sm()
                                    .child("123")
                                    .into_any_element(),
                                1,
                            ),
                        ),
                )
        }
    }
    #[gpui_kit::test]
    fn queue01_diagnostics_value_uses_released_description_layout(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        cx.update(crate::theme::apply_component_theme);
        let (_, window) = cx.add_window_view(|_, _| MetricFixture);
        window.refresh().expect("real production diagnostic metric");
        let cell = window.debug_bounds("queue01-metric").expect("metric cell");
        let value = window.debug_bounds("queue01-value").expect("visible value");
        let reference = window
            .debug_bounds("queue01-reference")
            .expect("released reference");
        let reference_value = window
            .debug_bounds("queue01-reference-value")
            .expect("released value");
        assert_eq!(
            value.top() - cell.top(),
            reference_value.top() - reference.top(),
            "production metadata must use the real released DescriptionList layout"
        );
        assert!(value.bottom() <= cell.bottom());
    }
}
