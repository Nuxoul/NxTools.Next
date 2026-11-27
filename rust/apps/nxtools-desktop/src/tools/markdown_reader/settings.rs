//! Tool preferences, observed Windows association state and the configuration surface.
use super::*;
use crate::notifications::{self, AppNotification, NotificationLevel};
use crate::ui::components::settings::{
    FeatureSettingsWindowIds, FeatureSettingsWindowSpec, feature_settings_window,
};
use gpui_kit::component::{
    form::{Field, Form},
    group_box::{GroupBox, GroupBoxVariants as _},
    label::Label,
    switch::Switch,
};
use gpui_kit::{AnyElement, Role};
use nxtools_core::{AppConfig, MarkdownReaderSettings};
use nxtools_protocol::{
    MarkdownAssociationAction, MarkdownAssociationCommand, MarkdownAssociationExtensionState,
    MarkdownAssociationState,
};

struct Settings {
    path: PathBuf,
    applied: MarkdownReaderSettings,
    saving: Option<Uuid>,
    association: Option<MarkdownAssociationState>,
    request: Option<Pending>,
}
impl gpui_kit::Global for Settings {}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Intent {
    Query,
    Refresh,
    Register,
    Unregister,
    DefaultApps,
    CheckDisable,
}

#[derive(Clone, Copy)]
struct Pending {
    id: Uuid,
    intent: Intent,
    started: Instant,
    source: AnyWindowHandle,
}

pub(crate) fn install_settings(config: &AppConfig, config_path: PathBuf, cx: &mut App) {
    cx.set_global(Settings {
        path: config_path,
        applied: config.markdown_reader,
        saving: None,
        association: None,
        request: None,
    });
}

pub(super) fn preferences(cx: &App) -> MarkdownReaderSettings {
    cx.try_global::<Settings>()
        .map(|state| state.applied)
        .unwrap_or_default()
}

pub(crate) fn enabled(cx: &App) -> bool {
    preferences(cx).enabled
}

#[derive(Clone, Copy)]
enum Change {
    Enabled(bool),
    Outline(bool),
    Font(u8),
    Wide(bool),
}
impl Change {
    fn apply(self, settings: &mut MarkdownReaderSettings) {
        match self {
            Self::Enabled(value) => settings.enabled = value,
            Self::Outline(value) => settings.show_outline = value,
            Self::Font(value) => settings.font_size = value,
            Self::Wide(value) => settings.wide_body = value,
        }
    }
}

fn save(change: Change, source: AnyWindowHandle, cx: &mut App) {
    save_operation(change, source, Uuid::new_v4(), cx);
}

fn save_operation(change: Change, source: AnyWindowHandle, id: Uuid, cx: &mut App) {
    let Some(state) = cx.try_global::<Settings>() else {
        return;
    };
    if state.saving.is_some() || state.request.is_some() {
        return;
    }
    let mut requested = state.applied;
    change.apply(&mut requested);
    if requested == state.applied {
        return;
    }
    let path = state.path.clone();
    let started = Instant::now();
    let state = cx.global_mut::<Settings>();
    state.saving = Some(id);
    tracing::info!(event = "markdown.settings.started", correlation_id = %id);
    let worker = cx.background_executor().spawn(async move {
        AppConfig::update_section(&path, |config| change.apply(&mut config.markdown_reader))
            .map(|config| config.markdown_reader)
    });
    cx.spawn(async move |cx| {
        let result = worker.await;
        cx.update(|cx| {
            if cx.try_global::<Settings>().and_then(|state| state.saving) != Some(id) { return; }
            let state = cx.global_mut::<Settings>();
            state.saving = None;
            let success = result.is_ok();
            let (level, message) = match result {
                Ok(settings) => {
                    state.applied = settings;
                    (NotificationLevel::Success, "阅读设置已保存。")
                }
                Err(_) => (NotificationLevel::Error, "配置保存失败，原设置仍然有效。"),
            };
            notify_result(id, source, level, message, cx);
            tracing::info!(event = "markdown.settings.finished", correlation_id = %id, success, elapsed_ms = started.elapsed().as_millis());
            cx.refresh_windows();
        });
    }).detach();
    cx.refresh_windows();
}

fn association_request(
    platform: Option<&PlatformBridge>,
    intent: Intent,
    source: AnyWindowHandle,
    cx: &mut App,
) {
    let Some(state) = cx.try_global::<Settings>() else {
        return;
    };
    if state.request.is_some()
        || state.saving.is_some()
        || (intent == Intent::Register && !state.applied.enabled)
    {
        return;
    }
    let id = Uuid::new_v4();
    let started = Instant::now();
    let command = PlatformCommand::MarkdownAssociation(MarkdownAssociationCommand {
        action: match intent {
            Intent::DefaultApps => MarkdownAssociationAction::OpenDefaults,
            Intent::Register => MarkdownAssociationAction::Register,
            Intent::Unregister => MarkdownAssociationAction::Unregister,
            Intent::Query | Intent::Refresh | Intent::CheckDisable => {
                MarkdownAssociationAction::Query
            }
        },
    });
    let sent = platform.is_some_and(|platform| platform.send_command_with_id(command, id).is_ok());
    let state = cx.global_mut::<Settings>();
    if sent {
        state.request = Some(Pending {
            id,
            intent,
            started,
            source,
        });
        tracing::info!(event = "markdown.association.started", correlation_id = %id);
    } else {
        if intent != Intent::Query {
            notify_result(
                id,
                source,
                NotificationLevel::Error,
                "暂时无法读取或更改 Windows 关联，请稍后重试。",
                cx,
            );
        }
        tracing::warn!(event = "markdown.association.finished", correlation_id = %id, outcome = "not_submitted", elapsed_ms = started.elapsed().as_millis());
    }
    cx.refresh_windows();
}

pub(super) fn platform_notification(event: &PlatformNotification, cx: &mut App) {
    let Some(pending) = cx.try_global::<Settings>().and_then(|state| state.request) else {
        return;
    };
    let result = match event {
        PlatformNotification::CommandResult {
            correlation_id,
            result,
        } if *correlation_id == pending.id => Some(result.clone()),
        PlatformNotification::Disconnected {
            pending_command_ids,
            ..
        } if pending_command_ids.contains(&pending.id) => None,
        _ => return,
    };
    cx.defer(move |cx| {
        if cx.try_global::<Settings>().and_then(|state| state.request).map(|request| request.id) != Some(pending.id) { return; }
        let state = cx.global_mut::<Settings>();
        state.request = None;
        let observed = result.as_ref().filter(|result| result.success).and_then(|result| {
            serde_json::from_str::<MarkdownAssociationState>(result.data.as_deref()?).ok()
        });
        let opened = pending.intent == Intent::DefaultApps
            && result.as_ref().is_some_and(|result| result.success && result.data.as_deref() == Some("opened"));
        let mut disable = false;
        let mut outcome = "failed";
        let notice;
        if let Some(observed) = observed {
            outcome = "observed";
            let is_default = observed.md.is_default || observed.markdown.is_default;
            state.association = Some(observed);
            notice = match pending.intent {
                Intent::CheckDisable if is_default => Some((NotificationLevel::Error, change_default_first())),
                Intent::CheckDisable => { disable = true; None }
                Intent::Register => Some((NotificationLevel::Success, "已添加打开方式。在 Windows 默认应用中选择 NxTools，即可双击预览。")),
                Intent::Unregister => Some((NotificationLevel::Success, "已移除 NxTools 的 Markdown 打开方式。")),
                Intent::Refresh => Some((NotificationLevel::Info, "已刷新 Windows 文件关联状态。")),
                Intent::Query | Intent::DefaultApps => None,
            };
        } else if opened {
            outcome = "opened_settings";
            notice = Some((NotificationLevel::Info, "请在 Windows 设置中为 .md 和 .markdown 分别选择 NxTools Markdown Reader。返回后会核实是否生效。"));
        } else {
            let status = result.as_ref().filter(|reply| !reply.success).and_then(|reply| reply.data.as_deref());
            if pending.intent != Intent::DefaultApps && status != Some("still_default") {
                state.association = None;
            }
            notice = (pending.intent != Intent::Query).then(|| (NotificationLevel::Error, match status {
                Some("still_default") => change_default_first(),
                Some("identity_unavailable") => "未能验证已安装的 NxTools，不能注册文件关联。",
                Some("registration_conflict") => "检测到冲突的 Markdown 注册，原有设置未更改。",
                Some("open_defaults_failed") => "无法打开 Windows 设置。可在「应用 → 默认应用」中搜索 .md 或 .markdown。",
                _ => "Windows 关联操作未完成。请刷新状态后重试。",
            }));
        }
        tracing::info!(event = "markdown.association.finished", correlation_id = %pending.id, outcome,
            native_error = ?result.and_then(|result| result.native_error_code), elapsed_ms = pending.started.elapsed().as_millis());
        if let Some((level, message)) = notice {
            notify_result(pending.id, pending.source, level, message, cx);
        }
        if disable { save_operation(Change::Enabled(false), pending.source, pending.id, cx); }
        cx.refresh_windows();
    });
}

fn change_default_first() -> &'static str {
    "NxTools 仍是当前默认应用。请先在 Windows 中选回其他应用，再停用或解除关联。"
}

fn notify_result(
    id: Uuid,
    source: AnyWindowHandle,
    level: NotificationLevel,
    message: &'static str,
    cx: &mut App,
) {
    let Some(center) = notifications::current(cx) else {
        return;
    };
    let notice = AppNotification::new(
        format!("markdown-settings-{id}"),
        level,
        "Markdown 阅读器",
        message,
    )
    .in_window(source);
    cx.defer(move |cx| center.update(cx, |center, cx| center.push(notice, cx)));
}

fn association_label(state: Option<&MarkdownAssociationExtensionState>) -> &'static str {
    match state {
        Some(state) if state.is_default => "当前默认",
        Some(state) if state.registered => "已注册，非默认",
        Some(_) => "未注册",
        None => "未核实",
    }
}

pub(crate) struct SettingsView {
    platform: Option<Rc<PlatformBridge>>,
    _activation: Subscription,
}
impl SettingsView {
    pub(crate) fn new(
        platform: Option<Rc<PlatformBridge>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.defer_in(window, |view, window, cx| {
            association_request(
                view.platform.as_deref(),
                Intent::Query,
                window.window_handle(),
                cx,
            )
        });
        let activation = cx.observe_window_activation(window, |_, window, cx| {
            if window.is_window_active() {
                cx.defer_in(window, |view, window, cx| {
                    association_request(
                        view.platform.as_deref(),
                        Intent::Query,
                        window.window_handle(),
                        cx,
                    )
                });
            }
        });
        Self {
            platform,
            _activation: activation,
        }
    }
}

impl Render for SettingsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let settings = preferences(cx);
        let state = cx.try_global::<Settings>();
        let busy = state.is_some_and(|state| state.saving.is_some() || state.request.is_some());
        let association = state.and_then(|state| state.association.as_ref());
        let registered =
            association.is_some_and(|state| state.md.registered || state.markdown.registered);
        let mut defaults = Form::horizontal().small().label_width(px(264.));
        let font = settings.font_size;
        let fonts = div()
            .flex()
            .items_center()
            .gap_2()
            .child(preference_button(
                "markdown-font-default-smaller",
                "−",
                "减小默认字号",
                Change::Font(font.saturating_sub(1)),
                busy || font <= MarkdownReaderSettings::MIN_FONT_SIZE,
            ))
            .child(Label::new(font.to_string()))
            .child(preference_button(
                "markdown-font-default-larger",
                "+",
                "增大默认字号",
                Change::Font(font.saturating_add(1)),
                busy || font >= MarkdownReaderSettings::MAX_FONT_SIZE,
            ));
        defaults = defaults
            .child(preference_field("markdown-default-font", "默认字号", fonts))
            .child(preference_field(
                "markdown-default-wide",
                "宽版正文",
                Switch::new("markdown-pref-wide")
                    .accessibility_label("宽版正文")
                    .checked(settings.wide_body)
                    .disabled(busy)
                    .on_change(|value, window, cx| {
                        let value = *value;
                        window.defer(cx, move |window, cx| {
                            save(Change::Wide(value), window.window_handle(), cx)
                        });
                    }),
            ))
            .child(preference_field(
                "markdown-default-outline",
                "宽窗默认固定大纲",
                Switch::new("markdown-pref-outline")
                    .accessibility_label("宽窗默认固定大纲")
                    .checked(settings.show_outline)
                    .disabled(busy)
                    .on_change(|value, window, cx| {
                        let value = *value;
                        window.defer(cx, move |window, cx| {
                            save(Change::Outline(value), window.window_handle(), cx)
                        });
                    }),
            ));
        let association_rows = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                Form::horizontal()
                    .small()
                    .label_width(px(264.))
                    .child(association_row(
                        "markdown-assoc-md",
                        ".md",
                        association_label(association.map(|state| &state.md)),
                    ))
                    .child(association_row(
                        "markdown-assoc-markdown",
                        ".markdown",
                        association_label(association.map(|state| &state.markdown)),
                    )),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(association_button(
                        "markdown-register",
                        "添加打开方式",
                        Intent::Register,
                        busy || !settings.enabled,
                        self.platform.clone(),
                    ))
                    .child(association_button(
                        "markdown-default-apps",
                        "设置为默认…",
                        Intent::DefaultApps,
                        busy || !registered,
                        self.platform.clone(),
                    ))
                    .child(association_button(
                        "markdown-unregister",
                        "解除注册",
                        Intent::Unregister,
                        busy || !registered,
                        self.platform.clone(),
                    ))
                    .child(association_button(
                        "markdown-assoc-refresh",
                        "刷新状态",
                        Intent::Refresh,
                        busy,
                        self.platform.clone(),
                    )),
            );
        let preview_platform = self.platform.clone();
        let body = div()
            .w_full()
            .min_w_0()
            .flex_none()
            .flex()
            .flex_col()
            .gap_3()
            .child(settings_group(
                "markdown-settings-reading",
                "阅读偏好",
                false,
                defaults.into_any_element(),
            ))
            .child(settings_group(
                "markdown-settings-association",
                "Windows 文件关联",
                busy,
                association_rows.into_any_element(),
            ))
            .child(
                div().flex_none().flex().child(
                    Button::new("markdown-settings-preview")
                        .debug_selector(|| "markdown-settings-preview".into())
                        .label("打开预览…")
                        .small()
                        .outline()
                        .disabled(!settings.enabled)
                        .on_click(move |_, window, cx| {
                            let platform = preview_platform.clone();
                            window.defer(cx, move |_, cx| {
                                show(platform, cx);
                            });
                        }),
                ),
            );
        let toggle_platform = self.platform.clone();
        feature_settings_window(
            Theme::active(cx),
            FeatureSettingsWindowSpec {
                ids: FeatureSettingsWindowIds {
                    root: "markdown-settings",
                    title: "markdown-settings-title",
                    description: "markdown-settings-description",
                    status: "markdown-settings-status",
                    enabled_toggle: "markdown-settings-enabled",
                    close: "markdown-settings-close",
                    content: "markdown-settings-content",
                },
                title: "Markdown 阅读器".into(),
                description: "设置文档阅读方式和资源管理器文件关联".into(),
                enabled_label: "启用 Markdown 阅读器".into(),
                enabled: settings.enabled,
            },
            move |value, window, cx| {
                let enabled = *value;
                let platform = toggle_platform.clone();
                window.defer(cx, move |window, cx| {
                    let source = window.window_handle();
                    if enabled {
                        save(Change::Enabled(true), source, cx);
                    } else {
                        association_request(platform.as_deref(), Intent::CheckDisable, source, cx);
                    }
                });
            },
            |_, window, _| window.remove_window(),
            body.into_any_element(),
        )
    }
}

fn preference_button(
    id: &'static str,
    label: &'static str,
    name: &'static str,
    change: Change,
    disabled: bool,
) -> Button {
    Button::new(id)
        .debug_selector(move || id.into())
        .label(label)
        .accessibility_label(name)
        .small()
        .outline()
        .disabled(disabled)
        .on_click(move |_, window, cx| {
            window.defer(cx, move |window, cx| {
                save(change, window.window_handle(), cx)
            })
        })
}

fn association_button(
    id: &'static str,
    label: &'static str,
    intent: Intent,
    disabled: bool,
    platform: Option<Rc<PlatformBridge>>,
) -> Button {
    Button::new(id)
        .debug_selector(move || id.into())
        .label(label)
        .small()
        .outline()
        .disabled(disabled || platform.is_none())
        .on_click(move |_, window, cx| {
            let platform = platform.clone();
            window.defer(cx, move |window, cx| {
                association_request(platform.as_deref(), intent, window.window_handle(), cx)
            });
        })
}

fn settings_group(
    id: &'static str,
    title: &'static str,
    busy: bool,
    content: AnyElement,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("{id}-semantic")))
        .debug_selector(move || id.into())
        .role(Role::Group)
        .aria_label(title)
        .w_full()
        .min_w_0()
        .flex_none()
        .child(
            GroupBox::new()
                .id(id)
                .normal()
                .gap_2()
                .title(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(title)
                        .when(busy, |title| {
                            title.child(
                                div()
                                    .debug_selector(|| "markdown-settings-busy".into())
                                    .child(Label::new("正在应用设置…").text_sm()),
                            )
                        }),
                )
                .child(content),
        )
}

fn preference_field(id: &'static str, title: &'static str, control: impl IntoElement) -> Field {
    Field::new()
        .items_center()
        .gap_0()
        .label_fn(move |_, _| {
            div()
                .debug_selector(move || format!("{id}-title"))
                .min_h_8()
                .flex()
                .items_center()
                .child(title)
        })
        .child(
            div()
                .debug_selector(move || format!("{id}-control"))
                .w_full()
                .min_w_0()
                .min_h_8()
                .flex()
                .items_center()
                .justify_end()
                .child(control),
        )
}

fn association_row(id: &'static str, ext: &'static str, label: &'static str) -> Field {
    preference_field(
        id,
        ext,
        div()
            .id(id)
            .role(Role::Group)
            .aria_label(format!("{ext}：{label}"))
            .child(Label::new(label)),
    )
}

#[cfg(test)]
#[path = "settings_notification_tests.rs"]
mod notification_tests;

#[cfg(test)]
#[path = "settings_layout_tests.rs"]
mod layout_tests;
