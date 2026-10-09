use super::*;
use crate::icons::MaterialIcon;
use crate::ui::components::action::focusable_content;
use crate::ui::metrics::RADIUS_CONTROL;
use crate::ui::typography::{LINE_HEIGHT_BODY, LINE_HEIGHT_TITLE, TEXT_BODY, TEXT_TITLE};
use gpui::{AnyElement, App, FontWeight, SharedString};
use gpui_component::Icon;

const CLIPBOARD_COLUMN: f32 = 96.0;
const QUICK_COLUMN: f32 = 80.0;
const ACTION_COLUMN: f32 = 56.0;
const ROW_HEIGHT: f32 = 40.0;
const CONTROL_HEIGHT: f32 = 32.0;

fn scope_action(
    id: SharedString,
    theme: Theme,
    checked: bool,
    pending: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let mark_selector = format!("{id}-mark");
    let marker = div()
        .debug_selector(move || mark_selector.clone())
        .size(px(16.0))
        .flex_none()
        .rounded(px(RADIUS_CONTROL))
        .border_1()
        .border_color(if checked {
            theme.accent
        } else {
            theme.text_muted
        })
        .bg(if checked { theme.accent } else { theme.canvas })
        .flex()
        .items_center()
        .justify_center()
        .when(checked, |mark| {
            mark.child(
                Icon::new(MaterialIcon::Check)
                    .size(px(12.0))
                    .text_color(theme.image_well),
            )
        });
    let action_selector = id.to_string();
    focusable_content(
        id,
        theme,
        pending,
        div()
            .debug_selector(move || action_selector.clone())
            .h(px(CONTROL_HEIGHT))
            .w_full()
            .flex()
            .items_center()
            .justify_center()
            .when(pending, |element| element.opacity(0.5))
            .child(marker)
            .into_any_element(),
        on_click,
    )
}

fn centered_column(selector: &'static str, width: f32, label: &'static str) -> AnyElement {
    div()
        .debug_selector(move || selector.into())
        .w(px(width))
        .flex_none()
        .h(px(ROW_HEIGHT))
        .flex()
        .items_center()
        .justify_center()
        .child(label)
        .into_any_element()
}

impl NetworkSettingsView {
    pub(super) fn render_navigation(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let mut rail = div()
            .debug_selector(|| "global-settings-navigation".into())
            .w(px(128.0))
            .h_full()
            .flex_none()
            .px(px(8.0))
            .py(px(16.0))
            .border_r_1()
            .border_color(theme.border_subtle)
            .flex()
            .flex_col()
            .gap(px(4.0))
            .text_size(px(TEXT_BODY))
            .line_height(px(LINE_HEIGHT_BODY));
        for (id, label, tab) in [
            (
                "global-settings-tab-general",
                "常规",
                GlobalSettingsTab::General,
            ),
            (
                "global-settings-tab-network",
                "网络",
                GlobalSettingsTab::Network,
            ),
            (
                "global-settings-tab-blacklist",
                "应用黑名单",
                GlobalSettingsTab::Blacklist,
            ),
        ] {
            let selected = self.tab == tab;
            rail = rail.child(focusable_content(
                id,
                theme,
                self.saving,
                div()
                    .debug_selector(move || id.into())
                    .w_full()
                    .h(px(32.0))
                    .px(px(12.0))
                    .rounded(px(RADIUS_CONTROL))
                    .flex()
                    .items_center()
                    .bg(if selected {
                        theme.element_selected
                    } else {
                        theme.canvas
                    })
                    .text_color(if selected {
                        theme.text
                    } else {
                        theme.text_secondary
                    })
                    .font_weight(if selected {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .hover(move |style| style.bg(theme.surface_hover))
                    .child(label)
                    .into_any_element(),
                cx.listener(move |this, _, _, cx| {
                    if !this.saving {
                        this.tab = tab;
                        cx.notify();
                    }
                }),
            ));
        }
        rail.into_any_element()
    }

    pub(super) fn add_rule(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.saving {
            return;
        }
        let correlation_id = uuid::Uuid::new_v4();
        let started_at = Instant::now();
        info!(event = "global.blacklist.draft_started", %correlation_id, rule_count = self.rules.len(), editing = self.editing_rule.is_some(), "application rule edit accepted");
        let rule = match ProcessBlacklistRule::new(
            &self.process_input.read(cx).value(),
            self.new_disable_clipboard,
            self.new_disable_quick_launch,
        ) {
            Ok(rule) => rule,
            Err(error) => {
                self.error = Some(error);
                info!(event = "global.blacklist.draft_failed", %correlation_id, failure_kind = "validation", elapsed_ms = started_at.elapsed().as_millis(), "application rule rejected");
                cx.notify();
                return;
            }
        };
        // Updating a selected row replaces its scopes; duplicates merge only with other rows.
        if let Some(index) = self.editing_rule.take() {
            self.rules.remove(index);
        }
        if let Some(existing) = self
            .rules
            .iter_mut()
            .find(|existing| existing.process == rule.process)
        {
            existing.disable_clipboard |= rule.disable_clipboard;
            existing.disable_quick_launch |= rule.disable_quick_launch;
        } else {
            self.rules.push(rule);
        }
        self.process_input
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.new_disable_clipboard = true;
        self.new_disable_quick_launch = false;
        self.error = None;
        info!(event = "global.blacklist.draft_completed", %correlation_id, rule_count = self.rules.len(), elapsed_ms = started_at.elapsed().as_millis(), "application rule draft updated");
        cx.notify();
    }

    pub(super) fn render_blacklist(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let pending = self.saving;
        let header = div()
            .w_full()
            .h(px(ROW_HEIGHT))
            .flex_none()
            .px(px(12.0))
            .flex()
            .items_center()
            .border_b_1()
            .border_color(theme.border_subtle)
            .text_color(theme.text_secondary)
            .font_weight(FontWeight::MEDIUM)
            .child(
                div()
                    .debug_selector(|| "global-blacklist-header-app".into())
                    .flex_1()
                    .min_w_0()
                    .child("应用进程"),
            )
            .child(centered_column(
                "global-blacklist-header-clipboard",
                CLIPBOARD_COLUMN,
                "禁用剪贴板",
            ))
            .child(centered_column(
                "global-blacklist-header-quick",
                QUICK_COLUMN,
                "禁用速启",
            ))
            .child(centered_column(
                "global-blacklist-header-actions",
                ACTION_COLUMN,
                "操作",
            ));
        let mut rows = div()
            .id("global-blacklist-scroll")
            .max_h(px(160.0))
            .overflow_y_scrollbar()
            .flex()
            .flex_col();
        for (index, rule) in self.rules.iter().enumerate() {
            let process = rule.process.clone();
            let clipboard = rule.disable_clipboard;
            let quick = rule.disable_quick_launch;
            let selected = self.editing_rule == Some(index);
            let label_selector = format!("global-blacklist-label-{index}");
            let row_selector = format!("global-blacklist-row-{index}");
            let app_action = focusable_content(
                SharedString::from(label_selector.clone()),
                theme,
                pending,
                div()
                    .debug_selector(move || label_selector.clone())
                    .w_full()
                    .h(px(CONTROL_HEIGHT))
                    .min_w_0()
                    .flex()
                    .items_center()
                    .text_color(theme.text)
                    .child(div().truncate().child(format!("{}.exe", rule.process)))
                    .into_any_element(),
                cx.listener(move |this, _, window, cx| {
                    if this.saving {
                        return;
                    }
                    this.editing_rule = Some(index);
                    this.new_disable_clipboard = clipboard;
                    this.new_disable_quick_launch = quick;
                    this.process_input.update(cx, |input, cx| {
                        input.focus(window, cx);
                        input.set_value(format!("{process}.exe"), window, cx)
                    });
                    cx.notify();
                }),
            );
            rows = rows.child(div().debug_selector(move || row_selector.clone())
                .w_full().h(px(ROW_HEIGHT)).flex_none().px(px(12.0)).flex().items_center()
                .border_b_1().border_color(theme.border_subtle)
                .when(selected, |row| row.bg(theme.element_selected))
                .hover(move |style| style.bg(theme.surface_hover))
                .child(div().flex_1().min_w_0().child(app_action))
                .child(div().debug_selector(move || format!("global-blacklist-clipboard-cell-{index}"))
                    .w(px(CLIPBOARD_COLUMN)).flex_none().child(scope_action(
                        format!("global-blacklist-clipboard-{index}").into(), theme, clipboard, pending,
                        cx.listener(move |this, _, _, cx| {
                            if !this.saving {
                                this.rules[index].disable_clipboard = !clipboard; cx.notify();
                                info!(event = "global.blacklist.draft_scope_changed", correlation_id = %uuid::Uuid::new_v4(), clipboard = true, disabled = !clipboard, elapsed_ms = 0, "application rule scope updated");
                            }
                        }))))
                .child(div().debug_selector(move || format!("global-blacklist-quick-cell-{index}"))
                    .w(px(QUICK_COLUMN)).flex_none().child(scope_action(
                        format!("global-blacklist-quick-{index}").into(), theme, quick, pending,
                        cx.listener(move |this, _, _, cx| {
                            if !this.saving {
                                this.rules[index].disable_quick_launch = !quick; cx.notify();
                                info!(event = "global.blacklist.draft_scope_changed", correlation_id = %uuid::Uuid::new_v4(), clipboard = false, disabled = !quick, elapsed_ms = 0, "application rule scope updated");
                            }
                        }))))
                .child(div().debug_selector(move || format!("global-blacklist-remove-cell-{index}"))
                    .w(px(ACTION_COLUMN)).flex_none().child(focusable_content(
                        SharedString::from(format!("global-blacklist-remove-{index}")), theme, pending,
                        div().h(px(CONTROL_HEIGHT)).w_full().flex().items_center().justify_center()
                            .text_color(theme.text_secondary).child("移除").into_any_element(),
                        cx.listener(move |this, _, _, cx| {
                            if this.saving { return; }
                            this.rules.remove(index); this.editing_rule = None;
                            info!(event = "global.blacklist.draft_removed", correlation_id = %uuid::Uuid::new_v4(), rule_count = this.rules.len(), elapsed_ms = 0, "application rule removed from draft");
                            cx.notify();
                        })))));
        }
        if self.rules.is_empty() {
            rows = rows.child(
                div()
                    .h(px(80.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .text_color(theme.text_muted)
                    .child("暂无应用规则。添加应用后选择要禁用的功能。"),
            );
        }
        let input = Input::new(&self.process_input)
            .appearance(false)
            .bordered(false)
            .disabled(pending)
            .text_size(px(TEXT_BODY));
        let editor = div()
            .debug_selector(|| "global-blacklist-new-row".into())
            .w_full()
            .h(px(ROW_HEIGHT))
            .px(px(12.0))
            .flex()
            .items_center()
            .child(div().flex_1().min_w_0().pr(px(8.0)).child(field_shell(
                theme,
                "global-blacklist-process-field",
                input,
            )))
            .child(
                div()
                    .w(px(CLIPBOARD_COLUMN))
                    .flex_none()
                    .child(scope_action(
                        "global-blacklist-new-clipboard".into(),
                        theme,
                        self.new_disable_clipboard,
                        pending,
                        cx.listener(|this, _, _, cx| {
                            if !this.saving {
                                this.new_disable_clipboard = !this.new_disable_clipboard;
                                cx.notify();
                            }
                        }),
                    )),
            )
            .child(div().w(px(QUICK_COLUMN)).flex_none().child(scope_action(
                "global-blacklist-new-quick".into(),
                theme,
                self.new_disable_quick_launch,
                pending,
                cx.listener(|this, _, _, cx| {
                    if !this.saving {
                        this.new_disable_quick_launch = !this.new_disable_quick_launch;
                        cx.notify();
                    }
                }),
            )))
            .child(
                div()
                    .debug_selector(|| "global-blacklist-add-action".into())
                    .w(px(ACTION_COLUMN))
                    .h(px(CONTROL_HEIGHT))
                    .flex_none()
                    .child(
                        action_button(
                            "global-blacklist-add",
                            if self.editing_rule.is_some() {
                                "更新"
                            } else {
                                "添加"
                            },
                        )
                        .h(px(CONTROL_HEIGHT))
                        .w_full()
                        .text_size(px(TEXT_BODY))
                        .disabled(pending)
                        .on_click(cx.listener(|this, _, window, cx| this.add_rule(window, cx))),
                    ),
            );
        div()
            .w_full()
            .flex_none()
            .flex()
            .flex_col()
            .gap(px(16.0))
            .text_size(px(TEXT_BODY))
            .line_height(px(LINE_HEIGHT_BODY))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .debug_selector(|| "global-blacklist-page-title".into())
                            .text_size(px(TEXT_TITLE))
                            .line_height(px(LINE_HEIGHT_TITLE))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("应用黑名单"),
                    )
                    .child(
                        div()
                            .text_color(theme.text_secondary)
                            .child("按应用独立禁用功能，勾选项在应用设置后生效。"),
                    ),
            )
            .child(
                div()
                    .debug_selector(|| "global-blacklist-table".into())
                    .w_full()
                    .border_1()
                    .border_color(theme.border_subtle)
                    .rounded(px(RADIUS_CONTROL))
                    .overflow_hidden()
                    .child(header)
                    .child(rows),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(div().px(px(12.0)).font_weight(FontWeight::MEDIUM).child(
                        if self.editing_rule.is_some() {
                            "编辑应用"
                        } else {
                            "添加应用"
                        },
                    ))
                    .child(editor)
                    .child(
                        div()
                            .px(px(12.0))
                            .text_color(theme.text_muted)
                            .child("填写进程名，例如 notepad.exe；点击已有应用可编辑。"),
                    ),
            )
            .into_any_element()
    }
}
