use super::*;
use gpui_kit::component::button::Button;
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::label::Label;
use gpui_kit::{AnyElement, SharedString};

const CLIPBOARD_COLUMN: f32 = 96.0;
const QUICK_COLUMN: f32 = 80.0;
const ACTION_COLUMN: f32 = 56.0;
const ROW_HEIGHT: f32 = 40.0;

fn scope_checkbox(
    id: SharedString,
    checked: bool,
    pending: bool,
    on_change: impl Fn(&bool, &mut Window, &mut gpui_kit::App) + 'static,
) -> AnyElement {
    let selector = id.to_string();
    let help = if id.contains("clipboard") {
        "禁用此应用的剪贴板捕获"
    } else {
        "禁用此应用的快速启动"
    };
    div()
        .debug_selector(move || selector.clone())
        .h(px(32.0))
        .w_full()
        .flex()
        .items_center()
        .justify_center()
        .child(
            Checkbox::new(id.clone())
                .accessibility_label(help)
                .tooltip(help)
                .checked(checked)
                .disabled(pending)
                .on_change(on_change),
        )
        .into_any_element()
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
        .child(Label::new(label).text_sm())
        .into_any_element()
}
impl NetworkSettingsView {
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
    pub(super) fn render_blacklist_table(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let pending = self.saving;
        let header = div()
            .w_full()
            .h(px(ROW_HEIGHT))
            .flex_none()
            .px_3()
            .flex()
            .items_center()
            .child(
                div()
                    .debug_selector(|| "global-blacklist-header-app".into())
                    .flex_1()
                    .min_w_0()
                    .child(Label::new("应用进程").text_sm()),
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
            let app_action =
                Button::new(SharedString::from(format!("global-blacklist-edit-{index}")))
                    .ghost()
                    .small()
                    .w_full()
                    .justify_start()
                    .disabled(pending)
                    .debug_selector(move || format!("global-blacklist-label-{index}"))
                    .child(
                        div()
                            .min_w_0()
                            .w_full()
                            .truncate()
                            .child(format!("{}.exe", rule.process)),
                    )
                    .tooltip(format!("{}.exe · 点击编辑", rule.process))
                    .on_click(cx.listener(move |this, _, window, cx| {
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
                    }));
            rows = rows.child(div().debug_selector(move || format!("global-blacklist-row-{index}"))
                .w_full().h(px(ROW_HEIGHT)).flex_none().px_3().flex().items_center()
                .when(selected, |row| row.bg(theme.element_selected))
                .child(div().flex_1().min_w_0().overflow_hidden().child(app_action))
                .child(div().debug_selector(move || format!("global-blacklist-clipboard-cell-{index}"))
                    .w(px(CLIPBOARD_COLUMN)).flex_none().child(scope_checkbox(
                        format!("global-blacklist-clipboard-{index}").into(), clipboard, pending,
                        cx.listener(move |this, requested: &bool, _, cx| {
                            if !this.saving {
                                this.rules[index].disable_clipboard = *requested; cx.notify();
                                info!(event = "global.blacklist.draft_scope_changed", correlation_id = %uuid::Uuid::new_v4(), clipboard = true, disabled = *requested, elapsed_ms = 0, "application rule scope updated");
                            }
                        }))))
                .child(div().debug_selector(move || format!("global-blacklist-quick-cell-{index}"))
                    .w(px(QUICK_COLUMN)).flex_none().child(scope_checkbox(
                        format!("global-blacklist-quick-{index}").into(), quick, pending,
                        cx.listener(move |this, requested: &bool, _, cx| {
                            if !this.saving {
                                this.rules[index].disable_quick_launch = *requested; cx.notify();
                                info!(event = "global.blacklist.draft_scope_changed", correlation_id = %uuid::Uuid::new_v4(), clipboard = false, disabled = *requested, elapsed_ms = 0, "application rule scope updated");
                            }
                        }))))
                .child(div().debug_selector(move || format!("global-blacklist-remove-cell-{index}"))
                    .w(px(ACTION_COLUMN)).flex_none().child(Button::new(SharedString::from(format!("global-blacklist-remove-{index}")))
                        .ghost().small().label("移除").disabled(pending)
                        .on_click(cx.listener(move |this, _, _, cx| {
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
                    .px_3()
                    .flex()
                    .items_center()
                    .child(Label::new("暂无应用规则。添加应用后选择要禁用的功能。").text_sm()),
            );
        }
        div()
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_2()
            .child(Label::new("按应用独立禁用功能，勾选项在应用设置后生效。").text_sm())
            .child(
                div()
                    .debug_selector(|| "global-blacklist-table".into())
                    .w_full()
                    .border_1()
                    .border_color(theme.border_subtle)
                    .overflow_hidden()
                    .child(header)
                    .child(rows),
            )
            .into_any_element()
    }
    pub(super) fn render_blacklist_editor(&self, cx: &mut Context<Self>) -> AnyElement {
        let pending = self.saving;
        let editor = div()
            .debug_selector(|| "global-blacklist-new-row".into())
            .w_full()
            .min_w_0()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .debug_selector(|| "global-blacklist-process-field".into())
                    .child(
                        Input::new(&self.process_input)
                            .aria_label("应用进程名")
                            .disabled(pending),
                    ),
            )
            .child(div().w(px(64.0)).flex_none().child(scope_checkbox(
                "global-blacklist-new-clipboard".into(),
                self.new_disable_clipboard,
                pending,
                cx.listener(|this, requested: &bool, _, cx| {
                    if !this.saving {
                        this.new_disable_clipboard = *requested;
                        cx.notify();
                    }
                }),
            )))
            .child(div().w(px(64.0)).flex_none().child(scope_checkbox(
                "global-blacklist-new-quick".into(),
                self.new_disable_quick_launch,
                pending,
                cx.listener(|this, requested: &bool, _, cx| {
                    if !this.saving {
                        this.new_disable_quick_launch = *requested;
                        cx.notify();
                    }
                }),
            )))
            .child(
                div()
                    .debug_selector(|| "global-blacklist-add-action".into())
                    .flex_none()
                    .child(
                        Button::new("global-blacklist-add")
                            .small()
                            .outline()
                            .label(if self.editing_rule.is_some() {
                                "更新"
                            } else {
                                "添加"
                            })
                            .disabled(pending)
                            .on_click(cx.listener(|this, _, window, cx| this.add_rule(window, cx))),
                    ),
            );
        Form::vertical().small().child(Field::new()
            .label(if self.editing_rule.is_some() { "编辑应用" } else { "添加应用" })
            .description("填写进程名，例如 notepad.exe；点击已有应用可编辑。左侧勾选禁用剪贴板，右侧勾选禁用速启。")
            .child(editor)).into_any_element()
    }
}
