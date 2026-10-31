//! The Tools workspace lists every currently registered tool in continuous rows
//! and opens one settings window per tool. Presentation never changes persistence.

use super::*;
use crate::ui::components::settings::{
    FeatureSettingsWindowIds, FeatureSettingsWindowSpec, feature_settings_window,
};
use crate::ui::metrics::{FEATURE_WINDOW_TITLE_LANE_HEIGHT, ordinary_rem};
use gpui_kit::component::IndexPath;
use gpui_kit::component::form::{Field, Form};
use gpui_kit::component::input::{NumberInput, NumberInputEvent, StepAction};
use gpui_kit::component::label::Label;
use gpui_kit::component::list::ListItem;
use gpui_kit::component::select::{Select, SelectEvent, SelectItem, SelectState};
use gpui_kit::{
    WindowBackgroundAppearance, WindowBounds, WindowDecorations, WindowKind, WindowOptions, size,
};
use nxtools_core::LinkPanelPosition;
mod currency_settings;
pub(super) mod floating_assistant_settings;

/// Stable tool identity and copy shared by the workspace and settings window.
#[derive(Clone, Copy)]
struct ToolCard {
    id: &'static str,
    title: &'static str,
    description: &'static str,
    icon: MaterialIcon,
}
fn link_panel_position_label(position: LinkPanelPosition) -> &'static str {
    match position {
        LinkPanelPosition::TopLeft => "左上角",
        LinkPanelPosition::Center => "中间",
        LinkPanelPosition::TopRight => "右上角",
    }
}

#[derive(Clone)]
pub(super) struct PositionChoice(LinkPanelPosition);
impl SelectItem for PositionChoice {
    type Value = LinkPanelPosition;
    fn title(&self) -> gpui_kit::SharedString {
        link_panel_position_label(self.0).into()
    }
    fn value(&self) -> &Self::Value {
        &self.0
    }
}

struct ToolSettingInputs {
    dwell: Entity<InputState>,
    position: Entity<SelectState<Vec<PositionChoice>>>,
    _subscriptions: Vec<Subscription>,
}
/// One information/control grid; controls use the remaining width and their
/// released geometry rather than a fixed 24px control box.
const TOOL_SETTINGS_INFORMATION_WIDTH: f32 = 264.0;

/// One Form label lane, yielding ordinary width to its controls inside the fixed client.
fn tool_settings_label_width(window: &Window) -> gpui_kit::Pixels {
    let rem = window.rem_size();
    let label = ordinary_rem(TOOL_SETTINGS_INFORMATION_WIDTH).to_pixels(rem);
    let controls = ordinary_rem(168.0).to_pixels(rem);
    let insets = ordinary_rem(24.0).to_pixels(rem);
    // Released Small Field has a 4px horizontal label/control gap.
    label.min((window.viewport_size().width - insets - px(4.0) - controls).max(px(0.0)))
}
/// The original link panel dwell range and step.
const DWELL_MIN_SECONDS: u64 = 2;
const DWELL_MAX_SECONDS: u64 = 60;
const DWELL_STEP_SECONDS: u64 = 2;
fn compact_tool_field(
    title_id: &'static str,
    help_id: &'static str,
    control_id: &'static str,
    title: &'static str,
    help: &'static str,
    control: impl IntoElement,
) -> Field {
    Field::new()
        .items_center()
        .gap_0()
        .label_fn(move |_, _| {
            div()
                .min_w_0()
                .min_h(ordinary_rem(FEATURE_WINDOW_TITLE_LANE_HEIGHT))
                .flex()
                .items_center()
                .gap_1()
                .text_sm()
                .child(div().debug_selector(move || title_id.into()).child(title))
                .child(crate::ui::components::tooltip::description_help(
                    help_id, help,
                ))
        })
        .child(
            div()
                .debug_selector(move || control_id.into())
                .w_full()
                .min_h(ordinary_rem(FEATURE_WINDOW_TITLE_LANE_HEIGHT))
                .flex()
                .items_center()
                .justify_end()
                .child(control),
        )
}

fn tool_status(theme: Theme, enabled: bool, selector: &'static str) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap_1()
        .child(
            div()
                .debug_selector(move || selector.into())
                .size(px(6.0))
                .child(
                    gpui_kit::component::badge::Badge::new()
                        .dot()
                        .color(if enabled {
                            theme.success
                        } else {
                            theme.text_muted
                        })
                        .child(div().size(px(6.0))),
                ),
        )
        .child(
            Label::new(if enabled { "已启用" } else { "已停用" })
                .text_xs()
                .text_color(theme.text_secondary),
        )
        .into_any_element()
}

pub(crate) fn render_tools(
    this: &mut MainWindow,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) -> AnyElement {
    let theme = this.theme;
    let enabled = this
        .clipboard
        .upgrade()
        .map(|application| application.read(cx).link_detector_enabled())
        .unwrap_or_default();
    let card = ToolCard {
        id: "tool-link-detector",
        title: "链接识别",
        description: "复制内容后识别网页链接及现有文件路径；点击打开，也可单独复制链接或路径",
        icon: MaterialIcon::Language,
    };
    let focus = this.tool_focus.clone();
    let toggle_enabled = cx.listener(move |this, _, _, cx| {
        let clipboard = this.clipboard.clone();
        cx.defer(move |cx| {
            if let Some(clipboard) = clipboard.upgrade() {
                clipboard.update(cx, |application, cx| {
                    application.set_link_detector_enabled(!application.link_detector_enabled(), cx);
                });
            }
        });
    });
    // Body and Configure have distinct single activation owners.
    let entry = focused_action(card.id, &focus, theme, false, window, cx, toggle_enabled)
        .debug_selector(|| "tool-link-detector-card".into())
        .tooltip(move |window, cx| Tooltip::new(card.description).build(window, cx))
        .w(px(180.0))
        .h(px(160.0))
        .flex_none()
        .child(
            ListItem::new("tool-link-detector-item")
                .accessibility_label("链接识别工具")
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
                            div()
                                .debug_selector(|| "tool-link-detector-icon".into())
                                .size(px(32.0))
                                .child(
                                    Icon::new(card.icon)
                                        .size(px(32.0))
                                        .text_color(theme.text_secondary),
                                ),
                        )
                        .child(
                            div()
                                .debug_selector(|| "tool-link-detector-title".into())
                                .child(Label::new(card.title)),
                        )
                        .child(
                            div()
                                .debug_selector(|| "tool-link-detector-status".into())
                                .child(tool_status(
                                    theme,
                                    enabled,
                                    "tool-link-detector-status-dot",
                                )),
                        )
                        .child(
                            Button::new("tool-link-detector-configure")
                                .debug_selector(|| "tool-link-detector-configure".into())
                                .label("配置…")
                                .small()
                                .outline()
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    open_tool_settings_window(this, card, cx);
                                    cx.notify();
                                })),
                        ),
                ),
        );
    div()
        .id("tools-page")
        .debug_selector(|| "tools-page".into())
        .size_full()
        .min_h_0()
        .flex()
        .flex_col()
        .bg(theme.canvas)
        .child(
            ModuleHeader::new(
                "tools-module-header",
                "tools-module-header-title",
                "tools-module-header-metadata",
                "其他工具",
                "按需启用与配置",
            )
            .render(theme),
        )
        .child(
            div()
                .id("tools-content")
                .debug_selector(|| "tools-content".into())
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .track_scroll(&this.tools_scroll)
                .p_4()
                .child(
                    div()
                        .debug_selector(|| "tools-grid".into())
                        .w_full()
                        .flex()
                        .flex_wrap()
                        .items_start()
                        .gap_3()
                        .child(entry)
                        .child(currency_settings::render_card(this, window, cx))
                        .child(floating_assistant_settings::render_card(this, window, cx)),
                ),
        )
        .into_any_element()
}

/// Settings for the tool opened from the grid card.
pub(crate) struct ToolSettingsWindowBody {
    title: &'static str,
    description: &'static str,
    enabled: bool,
    auto_hide_seconds: u64,
    position: LinkPanelPosition,
    inputs: Option<ToolSettingInputs>,
    clipboard: gpui_kit::WeakEntity<ClipboardApplication>,
}

impl ToolSettingsWindowBody {
    #[cfg(test)]
    pub(super) fn foundation_fixture() -> Self {
        Self {
            title: "链接识别",
            description: "复制内容后识别网页链接及现有文件路径；点击打开，也可单独复制链接或路径",
            enabled: false,
            auto_hide_seconds: 6,
            position: LinkPanelPosition::Center,
            inputs: None,
            clipboard: gpui_kit::WeakEntity::new_invalid(),
        }
    }
    #[cfg(test)]
    pub(super) fn dwell_input(&self) -> Entity<InputState> {
        self.inputs
            .as_ref()
            .expect("initialized controls")
            .dwell
            .clone()
    }

    #[cfg(test)]
    pub(super) fn position_input(&self) -> Entity<SelectState<Vec<PositionChoice>>> {
        self.inputs
            .as_ref()
            .expect("initialized controls")
            .position
            .clone()
    }

    #[cfg(test)]
    pub(super) fn foundation_values(&self) -> (bool, u64, LinkPanelPosition) {
        (self.enabled, self.auto_hide_seconds, self.position)
    }

    fn ensure_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.inputs.is_some() {
            return;
        }
        let seconds = self.auto_hide_seconds.to_string();
        let dwell = cx.new(|cx| {
            let mut state = InputState::new(window, cx).validate(|value, _| {
                value.is_empty()
                    || (value.len() <= 2 && value.bytes().all(|byte| byte.is_ascii_digit()))
            });
            state.set_value(seconds, window, cx);
            state.set_step(None, window, cx);
            state
        });
        let selected_index = LinkPanelPosition::ALL
            .iter()
            .position(|position| *position == self.position)
            .map(|index| IndexPath::default().row(index));
        let position = cx.new(|cx| {
            SelectState::new(
                LinkPanelPosition::ALL
                    .into_iter()
                    .map(PositionChoice)
                    .collect::<Vec<_>>(),
                selected_index,
                window,
                cx,
            )
        });
        let input_subscription = cx.subscribe_in(
            &dwell,
            window,
            |_, input, event: &InputEvent, window, cx| {
                let value = input.read(cx).value().to_string();
                let commit = matches!(event, InputEvent::Change);
                let restore = matches!(event, InputEvent::Blur | InputEvent::PressEnter { .. });
                if commit || restore {
                    cx.defer_in(window, move |this, window, cx| {
                        if let Ok(seconds) = value.parse::<u64>()
                            && (DWELL_MIN_SECONDS..=DWELL_MAX_SECONDS).contains(&seconds)
                        {
                            this.apply_dwell(seconds, cx);
                        }
                        if restore {
                            this.restore_dwell_input(window, cx);
                        }
                    });
                }
            },
        );
        let step_subscription = cx.subscribe_in(
            &dwell,
            window,
            |_, _, event: &NumberInputEvent, window, cx| {
                let NumberInputEvent::Step(action) = *event;
                cx.defer_in(window, move |this, window, cx| {
                    let seconds = match action {
                        StepAction::Decrement => this
                            .auto_hide_seconds
                            .saturating_sub(DWELL_STEP_SECONDS)
                            .max(DWELL_MIN_SECONDS),
                        StepAction::Increment => {
                            (this.auto_hide_seconds + DWELL_STEP_SECONDS).min(DWELL_MAX_SECONDS)
                        }
                    };
                    this.apply_dwell(seconds, cx);
                    this.restore_dwell_input(window, cx);
                });
            },
        );
        let position_subscription = cx.subscribe_in(
            &position,
            window,
            |_, _, event: &SelectEvent<Vec<PositionChoice>>, window, cx| {
                let SelectEvent::Confirm(position) = event;
                if let Some(position) = *position {
                    cx.defer_in(window, move |this, _, cx| {
                        if this.position != position {
                            this.position = position;
                            Self::defer_clipboard_update(
                                this.clipboard.clone(),
                                cx,
                                move |clipboard, cx| {
                                    clipboard.set_link_panel_position(position, cx)
                                },
                            );
                            cx.notify();
                        }
                    });
                }
            },
        );
        self.inputs = Some(ToolSettingInputs {
            dwell,
            position,
            _subscriptions: vec![input_subscription, step_subscription, position_subscription],
        });
    }

    fn apply_dwell(&mut self, seconds: u64, cx: &mut Context<Self>) {
        if self.auto_hide_seconds == seconds {
            return;
        }
        self.auto_hide_seconds = seconds;
        Self::defer_clipboard_update(self.clipboard.clone(), cx, move |clipboard, cx| {
            clipboard.set_link_panel_auto_hide_ms(seconds * 1_000, cx)
        });
        cx.notify();
    }

    fn restore_dwell_input(&self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(inputs) = &self.inputs {
            inputs.dwell.update(cx, |input, cx| {
                input.set_value(self.auto_hide_seconds.to_string(), window, cx)
            });
        }
    }
    fn defer_clipboard_update(
        clipboard: gpui_kit::WeakEntity<ClipboardApplication>,
        cx: &mut Context<Self>,
        update: impl FnOnce(&mut ClipboardApplication, &mut Context<ClipboardApplication>) + 'static,
    ) {
        cx.defer(move |cx| {
            if let Some(clipboard) = clipboard.upgrade() {
                clipboard.update(cx, update);
            }
        });
    }
}

impl Render for ToolSettingsWindowBody {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_inputs(window, cx);
        crate::theme::observe_theme(window, cx);
        let theme = Theme::active(cx);
        let inputs = self.inputs.as_ref().expect("tool controls initialized");
        let auto_hide_control = div()
            .id("tool-settings-duration")
            .debug_selector(|| "tool-settings-duration".into())
            .w_full()
            .child(NumberInput::new(&inputs.dwell).suffix("秒"))
            .into_any_element();
        let position_control = div()
            .id("tool-settings-position")
            .debug_selector(|| "tool-settings-position".into())
            .w_full()
            .child(Select::new(&inputs.position).accessibility_label("悬浮窗位置"))
            .into_any_element();
        let enabled_toggle = cx.listener(|this, _event, _window, cx| {
            this.enabled = !this.enabled;
            let enabled = this.enabled;
            Self::defer_clipboard_update(this.clipboard.clone(), cx, move |clipboard, cx| {
                clipboard.set_link_detector_enabled(enabled, cx);
            });
            cx.notify();
        });

        // Kit owns field labels/help and compact spacing. The existing feature
        // shell remains the sole scroll/native-chrome owner inside 460 × 320.
        let content = div()
            .debug_selector(|| "tool-settings-panel-section".into())
            .w_full()
            .child(
                div()
                    .debug_selector(|| "tool-settings-form".into())
                    .w_full()
                    .child(
                        Form::horizontal()
                            .small()
                            .label_width(tool_settings_label_width(window))
                            .child(compact_tool_field(
                                "tool-dwell-label",
                                "tool-dwell-help",
                                "tool-dwell-control",
                                "自动关闭时间",
                                "2–60 秒；每次调整 2 秒。",
                                auto_hide_control,
                            ))
                            .child(compact_tool_field(
                                "tool-position-label",
                                "tool-position-help",
                                "tool-position-control",
                                "悬浮窗位置",
                                "位置限制在所选显示器内。",
                                position_control,
                            )),
                    ),
            )
            .into_any_element();
        let on_close = cx.listener(|_, _event, window, _cx| window.remove_window());
        feature_settings_window(
            theme,
            FeatureSettingsWindowSpec {
                ids: FeatureSettingsWindowIds {
                    root: "tool-settings-window",
                    title: "tool-settings-title",
                    description: "tool-settings-description",
                    status: "tool-settings-enabled-status",
                    enabled_toggle: "tool-settings-toggle",
                    close: "tool-settings-close",
                    content: "tool-settings-content",
                },
                title: self.title.into(),
                description: self.description.into(),
                enabled_label: "启用链接识别".into(),
                enabled: self.enabled,
            },
            enabled_toggle,
            on_close,
            content,
        )
    }
}

fn open_tool_settings_window(this: &mut MainWindow, card: ToolCard, cx: &mut Context<MainWindow>) {
    if let Some(existing) = this.tool_settings_windows.get(card.id).copied() {
        if existing
            .update(cx, |_, window, _| {
                window.activate_window();
                window.refresh();
            })
            .is_ok()
        {
            return;
        }
        this.tool_settings_windows.remove(card.id);
    }

    let (enabled, auto_hide_seconds, position) = this
        .clipboard
        .upgrade()
        .map(|application| application.read(cx).link_panel_settings())
        .unwrap_or((false, 6, LinkPanelPosition::Center));
    let clipboard = this.clipboard.clone();
    let window_size = size(px(460.0), px(320.0));
    let window_bounds = WindowBounds::centered(window_size, cx);
    let opened = cx.open_window(
        WindowOptions {
            window_bounds: Some(window_bounds),
            titlebar: None,
            focus: true,
            show: true,
            kind: WindowKind::Normal,
            is_movable: true,
            is_resizable: false,
            is_minimizable: true,
            display_id: None,
            window_background: WindowBackgroundAppearance::Opaque,
            app_id: Some(format!("NxTools.ToolSettings.{}", card.id)),
            window_min_size: Some(window_size),
            window_decorations: Some(WindowDecorations::Client),
            app_owns_titlebar_drag: false,
            inactive_frame_interval: None,
            icon: None,
            tabbing_identifier: None,
        },
        move |window, cx| {
            if card.id == currency_settings::CARD.id {
                let body = cx.new(|cx| {
                    currency_settings::CurrencySettingsBody::new(clipboard.clone(), window, cx)
                });
                cx.new(|cx| gpui_kit::base::Root::new(body, window, cx))
            } else if card.id == floating_assistant_settings::CARD.id {
                let body = cx.new(|cx| {
                    floating_assistant_settings::FloatingAssistantSettingsBody::new(window, cx)
                });
                cx.new(|cx| gpui_kit::base::Root::new(body, window, cx))
            } else {
                let body = cx.new(|_| ToolSettingsWindowBody {
                    title: card.title,
                    description: card.description,
                    enabled,
                    auto_hide_seconds,
                    position,
                    inputs: None,
                    clipboard: clipboard.clone(),
                });
                cx.new(|cx| gpui_kit::base::Root::new(body, window, cx))
            }
        },
    );
    if let Ok(window) = opened {
        this.tool_settings_windows.insert(card.id, window.into());
    } else {
        tracing::error!(tool = card.id, "failed to open tool settings window");
    }
}
