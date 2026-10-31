//! The floating assistant's Other Tools cell and compact, section-owned configuration.
use super::*;
use crate::tools::floating_assistant::{self, FloatingAssistantApplication};
use gpui_kit::Focusable;
use gpui_kit::component::alert::Alert;
use gpui_kit::component::switch::Switch;
use nxtools_core::{ClickAction, FloatingAssistantSettings, ImageDropAction, TextDropAction};

pub(super) const CARD: ToolCard = ToolCard {
    id: "tool-floating-assistant",
    title: "悬浮窗",
    description: "显示可拖动的悬浮球；可配置点击、拖入与鼠标穿透。",
    icon: MaterialIcon::Language,
};

fn owner(cx: &App) -> Option<Entity<FloatingAssistantApplication>> {
    floating_assistant::current(cx)
}

pub(super) fn render_card(
    this: &MainWindow,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) -> AnyElement {
    // The keyed subscription follows the window, not the card's render pass. An owner
    // notification can originate while the owner is leased: repaint on the next turn.
    let view = cx.entity().downgrade();
    let _ = window.use_keyed_state("floating-assistant-card-observer", cx, |_, cx| {
        owner(cx).map(|owner| {
            cx.observe(&owner, move |_, _, cx| {
                let view = view.clone();
                cx.defer(move |cx| {
                    let _ = view.update(cx, |_, cx| cx.notify());
                });
            })
        })
    });
    let enabled = owner(cx).is_some_and(|owner| owner.read(cx).settings().enabled);
    let theme = this.theme;
    let toggle = cx.listener(|_, _, _, cx| {
        let current = owner(cx).map(|owner| owner.downgrade());
        cx.defer(move |cx| {
            if let Some(owner) = current.as_ref().and_then(|owner| owner.upgrade()) {
                owner.update(cx, |owner, cx| {
                    let mut settings = owner.settings().clone();
                    settings.enabled = !settings.enabled;
                    owner.change_settings(settings, cx);
                });
            }
        });
    });
    focused_action(
        CARD.id,
        &this.floating_tool_focus,
        theme,
        false,
        window,
        cx,
        toggle,
    )
    .debug_selector(|| "tool-floating-assistant-card".into())
    .tooltip(|window, cx| Tooltip::new(CARD.description).build(window, cx))
    .w(px(180.0))
    .h(px(160.0))
    .flex_none()
    .child(
        ListItem::new("tool-floating-assistant-item")
            .accessibility_label("悬浮窗工具")
            .size_full()
            .rounded_lg()
            .bg(theme.surface)
            .child(
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .child(
                        Icon::new(CARD.icon)
                            .size(px(32.0))
                            .text_color(theme.text_secondary),
                    )
                    .child(Label::new(CARD.title))
                    .child(tool_status(
                        theme,
                        enabled,
                        "tool-floating-assistant-status-dot",
                    ))
                    .child(
                        Button::new("tool-floating-assistant-configure")
                            .debug_selector(|| "tool-floating-assistant-configure".into())
                            .label("配置…")
                            .small()
                            .outline()
                            .on_click(cx.listener(|this, _, _, cx| {
                                cx.stop_propagation();
                                open_tool_settings_window(this, CARD, cx);
                                cx.notify();
                            })),
                    ),
            ),
    )
    .into_any_element()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum NumericField {
    Size,
    Fps,
    BaseOpacity,
    HoverOpacity,
    SnapDistance,
}
impl NumericField {
    const ALL: [Self; 5] = [
        Self::Size,
        Self::Fps,
        Self::BaseOpacity,
        Self::HoverOpacity,
        Self::SnapDistance,
    ];
    fn index(self) -> usize {
        match self {
            Self::Size => 0,
            Self::Fps => 1,
            Self::BaseOpacity => 2,
            Self::HoverOpacity => 3,
            Self::SnapDistance => 4,
        }
    }
    fn range(self) -> (u32, u32) {
        match self {
            Self::Size => (20, 300),
            Self::Fps => (1, 60),
            Self::BaseOpacity | Self::HoverOpacity => (10, 100),
            Self::SnapDistance => (4, 120),
        }
    }
    fn value(self, settings: &FloatingAssistantSettings) -> u32 {
        match self {
            Self::Size => settings.size,
            Self::Fps => settings.frame_rate,
            Self::BaseOpacity => settings.base_opacity,
            Self::HoverOpacity => settings.hover_opacity,
            Self::SnapDistance => settings.edge_snap_distance,
        }
    }
    fn assign(self, settings: &mut FloatingAssistantSettings, value: u32) {
        match self {
            Self::Size => settings.size = value,
            Self::Fps => settings.frame_rate = value,
            Self::BaseOpacity => settings.base_opacity = value,
            Self::HoverOpacity => settings.hover_opacity = value,
            Self::SnapDistance => settings.edge_snap_distance = value,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Choice {
    Click(ClickAction),
    Text(TextDropAction),
    Image(ImageDropAction),
}
#[derive(Clone)]
struct ActionChoice {
    value: Choice,
    label: &'static str,
}
impl SelectItem for ActionChoice {
    type Value = Choice;
    fn title(&self) -> gpui_kit::SharedString {
        self.label.into()
    }
    fn value(&self) -> &Choice {
        &self.value
    }
}
fn click_choices() -> Vec<ActionChoice> {
    [
        (ClickAction::None, "无"),
        (ClickAction::OpenMain, "打开主窗口"),
        (ClickAction::AddDownload, "添加下载"),
    ]
    .into_iter()
    .map(|(value, label)| ActionChoice {
        value: Choice::Click(value),
        label,
    })
    .collect()
}
fn text_choices() -> Vec<ActionChoice> {
    [
        (TextDropAction::None, "无"),
        (TextDropAction::Automatic, "自动判断"),
        (TextDropAction::AddDownload, "添加到下载"),
        (TextDropAction::QuickLaunchSearch, "速启搜索"),
    ]
    .into_iter()
    .map(|(value, label)| ActionChoice {
        value: Choice::Text(value),
        label,
    })
    .collect()
}
#[cfg(test)]
pub(crate) fn choice_labels() -> [Vec<&'static str>; 4] {
    [
        click_choices(),
        click_choices(),
        text_choices(),
        image_choices(),
    ]
    .map(|choices| choices.into_iter().map(|choice| choice.label).collect())
}
fn image_choices() -> Vec<ActionChoice> {
    [
        (ImageDropAction::None, "无"),
        (ImageDropAction::Automatic, "自动判断"),
        (ImageDropAction::AddDownload, "添加到下载"),
        (ImageDropAction::WriteClipboard, "写入剪切板"),
    ]
    .into_iter()
    .map(|(value, label)| ActionChoice {
        value: Choice::Image(value),
        label,
    })
    .collect()
}
fn selected(choices: &[ActionChoice], value: Choice) -> Option<IndexPath> {
    choices
        .iter()
        .position(|choice| choice.value == value)
        .map(|row| IndexPath::default().row(row))
}

#[derive(Clone, Copy)]
enum SettingChange {
    Numeric(NumericField, u32),
    Enabled(bool),
    SyncSpeed(bool),
    AutoHide(bool),
    Fullscreen(bool),
    PositionLocked(bool),
    ClickThrough(bool),
    Left(ClickAction),
    Right(ClickAction),
    Text(TextDropAction),
    Image(ImageDropAction),
}
impl SettingChange {
    fn apply(self, settings: &mut FloatingAssistantSettings) {
        match self {
            Self::Numeric(field, value) => field.assign(settings, value),
            Self::Enabled(value) => settings.enabled = value,
            Self::SyncSpeed(value) => settings.synchronize_download_speed = value,
            Self::AutoHide(value) => settings.auto_hide_at_edge = value,
            Self::Fullscreen(value) => settings.hide_in_fullscreen = value,
            Self::PositionLocked(value) => settings.position_locked = value,
            Self::ClickThrough(value) => settings.click_through = value,
            Self::Left(value) => settings.left_click_action = value,
            Self::Right(value) => settings.right_click_action = value,
            Self::Text(value) => settings.text_drop_action = value,
            Self::Image(value) => settings.image_drop_action = value,
        }
        settings.normalize();
    }
}

pub(crate) struct FloatingAssistantSettingsBody {
    owner: gpui_kit::WeakEntity<FloatingAssistantApplication>,
    settings: FloatingAssistantSettings,
    numbers: [Entity<InputState>; 5],
    actions: [Entity<SelectState<Vec<ActionChoice>>>; 4],
    validation: Option<NumericField>,
    _subscriptions: Vec<Subscription>,
}
impl FloatingAssistantSettingsBody {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let current = owner(cx);
        let settings = current
            .as_ref()
            .map(|owner| owner.read(cx).settings().clone())
            .unwrap_or_default();
        let numbers = NumericField::ALL.map(|field| {
            let value = field.value(&settings).to_string();
            cx.new(|cx| {
                let mut input = InputState::new(window, cx).validate(|text, _| {
                    text.is_empty()
                        || (text.len() <= 3 && text.bytes().all(|byte| byte.is_ascii_digit()))
                });
                input.set_value(value, window, cx);
                // The project's Step callback is the only increment/decrement owner.
                input.set_step(None, window, cx);
                input
            })
        });
        let choices = [
            click_choices(),
            click_choices(),
            text_choices(),
            image_choices(),
        ];
        let values = [
            Choice::Click(settings.left_click_action),
            Choice::Click(settings.right_click_action),
            Choice::Text(settings.text_drop_action),
            Choice::Image(settings.image_drop_action),
        ];
        let actions: [Entity<SelectState<Vec<ActionChoice>>>; 4] = choices
            .into_iter()
            .zip(values)
            .map(|(choices, value)| {
                let index = selected(&choices, value);
                cx.new(|cx| SelectState::new(choices, index, window, cx))
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap_or_else(|_| unreachable!("four selectors"));
        let mut subscriptions = Vec::new();
        for field in NumericField::ALL {
            let input = &numbers[field.index()];
            subscriptions.push(cx.subscribe_in(
                input,
                window,
                move |_, input, event: &InputEvent, window, cx| {
                    let text = input.read(cx).value().to_string();
                    let commit = matches!(event, InputEvent::Change);
                    let finish = matches!(event, InputEvent::Blur | InputEvent::PressEnter { .. });
                    if commit || finish {
                        cx.defer_in(window, move |this, window, cx| {
                            this.numeric_input(field, &text, finish, window, cx)
                        });
                    }
                },
            ));
            subscriptions.push(cx.subscribe_in(
                input,
                window,
                move |_, _, event: &NumberInputEvent, window, cx| {
                    let NumberInputEvent::Step(action) = *event;
                    cx.defer_in(window, move |this, window, cx| {
                        let (min, max) = field.range();
                        let value = field.value(&this.settings);
                        let value = match action {
                            StepAction::Increment => value.saturating_add(1).min(max),
                            StepAction::Decrement => value.saturating_sub(1).max(min),
                        };
                        this.change(SettingChange::Numeric(field, value), cx);
                        this.numbers[field.index()].update(cx, |input, cx| {
                            input.set_value(value.to_string(), window, cx)
                        });
                    });
                },
            ));
        }
        for (index, select) in actions.iter().enumerate() {
            subscriptions.push(cx.subscribe_in(
                select,
                window,
                move |_, _, event: &SelectEvent<Vec<ActionChoice>>, window, cx| {
                    let SelectEvent::Confirm(Some(value)) = event else {
                        return;
                    };
                    let value = *value;
                    cx.defer_in(window, move |this, _, cx| {
                        let change = match (index, value) {
                            (0, Choice::Click(value)) => Some(SettingChange::Left(value)),
                            (1, Choice::Click(value)) => Some(SettingChange::Right(value)),
                            (2, Choice::Text(value)) => Some(SettingChange::Text(value)),
                            (3, Choice::Image(value)) => Some(SettingChange::Image(value)),
                            _ => None,
                        };
                        if let Some(change) = change {
                            this.change(change, cx);
                        }
                    });
                },
            ));
        }
        if let Some(current) = &current {
            subscriptions.push(cx.observe_in(current, window, |_, owner, window, cx| {
                let settings = owner.read(cx).settings().clone();
                cx.defer_in(window, move |this, window, cx| {
                    this.sync(settings, window, cx)
                });
            }));
        }
        Self {
            owner: current
                .map_or_else(gpui_kit::WeakEntity::new_invalid, |owner| owner.downgrade()),
            settings,
            numbers,
            actions,
            validation: None,
            _subscriptions: subscriptions,
        }
    }
    fn change(&mut self, change: SettingChange, cx: &mut Context<Self>) {
        let owner = self.owner.clone();
        // Mutate the owner's most recent projection, never a stale full form snapshot.
        cx.defer(move |cx| {
            if let Some(owner) = owner.upgrade() {
                owner.update(cx, |owner, cx| {
                    let mut settings = owner.settings().clone();
                    change.apply(&mut settings);
                    owner.change_settings(settings, cx);
                });
            }
        });
    }
    fn numeric_input(
        &mut self,
        field: NumericField,
        text: &str,
        finish: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (min, max) = field.range();
        let parsed = text.parse::<u32>().ok();
        if let Some(value) = parsed.filter(|value| (min..=max).contains(value)) {
            self.validation = None;
            if field.value(&self.settings) != value {
                self.change(SettingChange::Numeric(field, value), cx);
            }
        } else if !finish {
            self.validation = Some(field);
        } else if let Some(value) = parsed {
            let value = value.clamp(min, max);
            self.validation = None;
            self.change(SettingChange::Numeric(field, value), cx);
        } else {
            self.validation = None;
        }
        if finish {
            let value = parsed.map_or_else(
                || field.value(&self.settings),
                |value| value.clamp(min, max),
            );
            self.numbers[field.index()].update(cx, |input, cx| {
                input.set_value(value.to_string(), window, cx)
            });
        }
        cx.notify();
    }
    fn sync(
        &mut self,
        settings: FloatingAssistantSettings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        for field in NumericField::ALL {
            if field.value(&settings) != field.value(&self.settings)
                && !self.numbers[field.index()]
                    .read(cx)
                    .focus_handle(cx)
                    .is_focused(window)
            {
                self.numbers[field.index()].update(cx, |input, cx| {
                    input.set_value(field.value(&settings).to_string(), window, cx);
                });
            }
        }
        let values = [
            Choice::Click(settings.left_click_action),
            Choice::Click(settings.right_click_action),
            Choice::Text(settings.text_drop_action),
            Choice::Image(settings.image_drop_action),
        ];
        for (select, value) in self.actions.iter().zip(values) {
            if select.read(cx).selected_value() != Some(&value) {
                select.update(cx, |state, cx| state.set_selected_value(&value, window, cx));
            }
        }
        self.settings = settings;
        cx.notify();
    }
    fn number(&self, field: NumericField, name: &'static str, suffix: &'static str) -> AnyElement {
        div()
            .debug_selector(move || name.into())
            .w_full()
            .child(
                NumberInput::new(&self.numbers[field.index()])
                    .accessibility_label(name)
                    .suffix(suffix),
            )
            .into_any_element()
    }
    fn select(&self, index: usize, name: &'static str) -> AnyElement {
        div()
            .debug_selector(move || name.into())
            .w_full()
            .child(
                Select::new(&self.actions[index])
                    .accessibility_label(name)
                    .menu_max_h(px(144.0)),
            )
            .into_any_element()
    }
    fn switch(
        &self,
        id: &'static str,
        checked: bool,
        change: fn(bool) -> SettingChange,
        cx: &mut Context<Self>,
    ) -> Switch {
        let owner = cx.entity().downgrade();
        Switch::new(id)
            .accessibility_label(id)
            .checked(checked)
            .on_change(move |requested, _, cx| {
                let requested = *requested;
                let owner = owner.clone();
                cx.defer(move |cx| {
                    let _ = owner.update(cx, |this, cx| this.change(change(requested), cx));
                });
            })
    }
}
impl Render for FloatingAssistantSettingsBody {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::theme::observe_theme(window, cx);
        let theme = Theme::active(cx);
        let settings = self.settings.clone();
        let (error, busy) = self
            .owner
            .upgrade()
            .map(|owner| {
                let owner = owner.read(cx);
                (owner.error().map(str::to_owned), owner.busy())
            })
            .unwrap_or((Some("悬浮窗控制器不可用，请重试。".into()), false));
        let picker = self.owner.clone();
        let reset = self.owner.clone();
        let retry = self.owner.clone();
        let path = settings
            .image_path
            .clone()
            .unwrap_or_else(|| "默认悬浮球".into());
        let image_controls = div()
            .w_full()
            .flex()
            .items_center()
            .justify_end()
            .gap_1()
            .child(
                Button::new("floating-image-pick")
                    .debug_selector(|| "floating-image-pick".into())
                    .label(if busy {
                        "选择中…"
                    } else {
                        "选择图片…"
                    })
                    .small()
                    .outline()
                    .disabled(busy)
                    .on_click(cx.listener(move |_, _, _, cx| {
                        let picker = picker.clone();
                        cx.defer(move |cx| {
                            if let Some(owner) = picker.upgrade() {
                                owner.update(cx, |owner, cx| owner.pick_image(cx));
                            }
                        });
                    })),
            )
            .child(
                Button::new("floating-image-reset")
                    .debug_selector(|| "floating-image-reset".into())
                    .label("恢复默认")
                    .small()
                    .outline()
                    .disabled(busy || settings.image_path.is_none())
                    .on_click(cx.listener(move |_, _, _, cx| {
                        let reset = reset.clone();
                        cx.defer(move |cx| {
                            if let Some(owner) = reset.upgrade() {
                                owner.update(cx, |owner, cx| owner.reset_image(cx));
                            }
                        });
                    })),
            );
        let mut form = Form::horizontal()
            .small()
            .label_width(tool_settings_label_width(window));
        form = form.child(compact_tool_field(
            "floating-image-label",
            "floating-image-help",
            "floating-image-control",
            "悬浮球图片",
            "PNG、JPG、JPEG、BMP、WebP 或 GIF；恢复默认只清除图片。",
            image_controls,
        ));
        form = form.child(compact_tool_field(
            "floating-size-label",
            "floating-size-help",
            "floating-size-control",
            "尺寸",
            "20–300 逻辑像素。",
            self.number(NumericField::Size, "floating-size", "px"),
        ));
        form = form.child(compact_tool_field(
            "floating-fps-label",
            "floating-fps-help",
            "floating-fps-control",
            "GIF 帧率上限",
            "1–60 帧/秒；保留原 GIF 时间间隔。",
            self.number(NumericField::Fps, "floating-fps", "FPS"),
        ));
        form = form.child(compact_tool_field(
            "floating-sync-label",
            "floating-sync-help",
            "floating-sync-control",
            "下载速度联动",
            "仅调整 GIF 播放速度，不影响下载速度。",
            self.switch(
                "floating-sync",
                settings.synchronize_download_speed,
                SettingChange::SyncSpeed,
                cx,
            ),
        ));
        form = form.child(compact_tool_field(
            "floating-base-label",
            "floating-base-help",
            "floating-base-control",
            "基础不透明度",
            "10–100%；悬停不透明度不能低于基础值。",
            self.number(NumericField::BaseOpacity, "floating-base", "%"),
        ));
        form = form.child(compact_tool_field(
            "floating-hover-label",
            "floating-hover-help",
            "floating-hover-control",
            "悬停不透明度",
            "10–100%；不低于基础不透明度。",
            self.number(NumericField::HoverOpacity, "floating-hover", "%"),
        ));
        form = form.child(compact_tool_field(
            "floating-snap-label",
            "floating-snap-help",
            "floating-snap-control",
            "贴边距离",
            "4–120 逻辑像素。",
            self.number(NumericField::SnapDistance, "floating-snap", "px"),
        ));
        form = form.child(compact_tool_field(
            "floating-auto-hide-label",
            "floating-auto-hide-help",
            "floating-auto-hide-control",
            "贴边自动隐藏",
            "鼠标靠近边缘时可重新显示。",
            self.switch(
                "floating-auto-hide",
                settings.auto_hide_at_edge,
                SettingChange::AutoHide,
                cx,
            ),
        ));
        form = form.child(compact_tool_field(
            "floating-fullscreen-label",
            "floating-fullscreen-help",
            "floating-fullscreen-control",
            "全屏时隐藏",
            "离开全屏后恢复悬浮球。",
            self.switch(
                "floating-fullscreen",
                settings.hide_in_fullscreen,
                SettingChange::Fullscreen,
                cx,
            ),
        ));
        form = form.child(compact_tool_field(
            "floating-lock-label",
            "floating-lock-help",
            "floating-lock-control",
            "锁定位置",
            "阻止拖动，不影响左键或右键操作。",
            self.switch(
                "floating-lock",
                settings.position_locked,
                SettingChange::PositionLocked,
                cx,
            ),
        ));
        form = form.child(compact_tool_field(
            "floating-left-label",
            "floating-left-help",
            "floating-left-control",
            "左键操作",
            "一次点击只执行所选操作。",
            self.select(0, "floating-left"),
        ));
        form = form.child(compact_tool_field(
            "floating-right-label",
            "floating-right-help",
            "floating-right-control",
            "右键操作",
            "添加下载打开普通下载确认窗口。",
            self.select(1, "floating-right"),
        ));
        form = form.child(compact_tool_field(
            "floating-text-label",
            "floating-text-help",
            "floating-text-control",
            "文本拖入",
            "自动判断：下载链接进入添加下载，其余文本进入速启搜索。",
            self.select(2, "floating-text"),
        ));
        form = form.child(compact_tool_field(
            "floating-image-drop-label",
            "floating-image-drop-help",
            "floating-image-drop-control",
            "图片拖入",
            "自动判断：网络图片添加下载，本地图片写入剪切板。",
            self.select(3, "floating-image-drop"),
        ));
        form = form.child(compact_tool_field(
            "floating-click-through-label",
            "floating-click-through-help",
            "floating-click-through-control",
            "鼠标穿透",
            "启用后不能在悬浮球上点击或拖入；在其他工具中关闭。",
            self.switch(
                "floating-click-through",
                settings.click_through,
                SettingChange::ClickThrough,
                cx,
            ),
        ));
        let content = div()
            .debug_selector(|| "floating-settings-form".into())
            .w_full()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .min_w_0()
                    .w_full()
                    .overflow_hidden()
                    .child(Label::new(path).text_sm().text_color(theme.text_secondary)),
            )
            .child(form)
            .when_some(self.validation, |content, field| {
                let (min, max) = field.range();
                content.child(Alert::warning(
                    "floating-validation",
                    format!("请输入 {min}–{max} 范围内的整数。"),
                ))
            })
            .when_some(error, |content, error| {
                content.child(
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(Alert::warning("floating-error", error))
                        .child(
                            Button::new("floating-retry")
                                .debug_selector(|| "floating-retry".into())
                                .label("重试")
                                .small()
                                .outline()
                                .on_click(cx.listener(move |_, _, _, cx| {
                                    let retry = retry.clone();
                                    cx.defer(move |cx| {
                                        if let Some(owner) = retry.upgrade() {
                                            owner.update(cx, |owner, cx| owner.retry(cx));
                                        }
                                    });
                                })),
                        ),
                )
            });
        let enabled = settings.enabled;
        let body = cx.entity().downgrade();
        feature_settings_window(
            theme,
            FeatureSettingsWindowSpec {
                ids: FeatureSettingsWindowIds {
                    root: "floating-settings-window",
                    title: "floating-settings-title",
                    description: "floating-settings-description",
                    status: "floating-settings-status",
                    enabled_toggle: "floating-settings-toggle",
                    close: "floating-settings-close",
                    content: "floating-settings-content",
                },
                title: CARD.title.into(),
                description: CARD.description.into(),
                enabled_label: "启用悬浮窗".into(),
                enabled,
            },
            cx.listener(move |_, requested: &bool, _, cx| {
                let requested = *requested;
                let body = body.clone();
                cx.defer(move |cx| {
                    let _ = body.update(cx, |this, cx| {
                        this.change(SettingChange::Enabled(requested), cx)
                    });
                });
            }),
            cx.listener(|_, _, window, _| window.remove_window()),
            content.into_any_element(),
        )
    }
}
