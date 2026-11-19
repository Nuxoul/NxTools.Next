use gpui_kit::component::button::{Button, ButtonCustomVariant, ButtonVariants};
use gpui_kit::component::{ActiveTheme, Icon, IconName, Sizable};
use gpui_kit::{
    AnyElement, App, ClickEvent, IntoElement, MouseButton, RenderOnce, SharedString, Window,
    WindowControlArea, WindowOptions, div, prelude::*, px,
};

use crate::theme::Theme;
use crate::ui::metrics::{MODULE_HEADER_HEIGHT, ordinary_rem};
use crate::ui::typography::{
    LINE_HEIGHT_LABEL, LINE_HEIGHT_PAGE_TITLE, TEXT_LABEL, TEXT_PAGE_TITLE, UI_MONO_FONT_FAMILY,
};

pub(crate) struct ModuleHeader {
    selector: &'static str,
    title_selector: &'static str,
    metadata_selector: &'static str,
    title: SharedString,
    metadata: SharedString,
    metadata_monospace: bool,
    trailing: Option<AnyElement>,
}

impl ModuleHeader {
    pub(crate) fn new(
        selector: &'static str,
        title_selector: &'static str,
        metadata_selector: &'static str,
        title: impl Into<SharedString>,
        metadata: impl Into<SharedString>,
    ) -> Self {
        Self {
            selector,
            title_selector,
            metadata_selector,
            title: title.into(),
            metadata: metadata.into(),
            metadata_monospace: false,
            trailing: None,
        }
    }

    pub(crate) fn metadata_monospace(mut self) -> Self {
        self.metadata_monospace = true;
        self
    }

    pub(crate) fn trailing(mut self, trailing: AnyElement) -> Self {
        self.trailing = Some(trailing);
        self
    }

    pub(crate) fn render(self, theme: Theme) -> AnyElement {
        let Self {
            selector,
            title_selector,
            metadata_selector,
            title,
            metadata,
            metadata_monospace,
            trailing,
        } = self;
        div()
            .id(selector)
            .debug_selector(move || selector.into())
            .h(ordinary_rem(MODULE_HEADER_HEIGHT))
            .w_full()
            .flex_none()
            .px_3()
            .border_b_1()
            .border_color(theme.border_subtle)
            .bg(theme.canvas)
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .id(title_selector)
                    .debug_selector(move || title_selector.into())
                    .flex_none()
                    .h(ordinary_rem(LINE_HEIGHT_PAGE_TITLE))
                    .text_size(ordinary_rem(TEXT_PAGE_TITLE))
                    .line_height(ordinary_rem(LINE_HEIGHT_PAGE_TITLE))
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .child(title),
            )
            .child(
                div()
                    .id(metadata_selector)
                    .debug_selector(move || metadata_selector.into())
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .h(ordinary_rem(LINE_HEIGHT_LABEL))
                    .text_size(ordinary_rem(TEXT_LABEL))
                    .line_height(ordinary_rem(LINE_HEIGHT_LABEL))
                    .text_color(theme.text_secondary)
                    .when(metadata_monospace, |metadata| {
                        metadata.font_family(UI_MONO_FONT_FAMILY)
                    })
                    .child(metadata),
            )
            .child(div().flex_1().min_w_0())
            .when_some(trailing, |header, trailing| header.child(trailing))
            .into_any_element()
    }
}

/// One explicit native policy for a window drawing a client caption. Do not apply to passive
/// previews/popups. The original caller still owns bounds, visibility, activation and close.
pub(crate) fn caption_window_options() -> WindowOptions {
    WindowOptions {
        titlebar: Some(gpui_kit::component::TitleBar::title_bar_options()),
        window_decorations: Some(gpui_kit::WindowDecorations::Client),
        app_owns_titlebar_drag: false,
        ..Default::default()
    }
}

type CloseCaption = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// Browser reference: released Component 0.8.0 title_bar.rs, 34DIP caption/controls,
/// 12DIP left inset, Small window icons, official title_bar/secondary/danger state colors.
/// TitleBar's Windows close callback and private control slots cannot preserve custom owners;
/// this single shared composition uses real styled Buttons for owner-dispatched captions.
#[derive(IntoElement)]
pub(crate) struct WindowCaption {
    capabilities: CaptionCapabilities,
    selector: &'static str,
    close_id: &'static str,
    title: SharedString,
    prefix: Option<AnyElement>,
    content: Option<AnyElement>,
    on_close: Option<CloseCaption>,
}
#[derive(Clone, Copy)]
pub(crate) struct CaptionCapabilities {
    pub minimize: bool,
    pub maximize: bool,
}
impl CaptionCapabilities {
    pub(crate) const CLOSE: Self = Self {
        minimize: false,
        maximize: false,
    };
    pub(crate) const MINIMIZE: Self = Self {
        minimize: true,
        maximize: false,
    };
    pub(crate) const RESIZABLE: Self = Self {
        minimize: true,
        maximize: true,
    };
}
pub(crate) fn controlled_titlebar(
    selector: &'static str,
    close_id: &'static str,
    title: impl Into<SharedString>,
    capabilities: CaptionCapabilities,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    WindowCaption {
        selector,
        close_id,
        title: title.into(),
        prefix: None,
        content: None,
        capabilities,
        on_close: Some(Box::new(on_close)),
    }
    .into_any_element()
}
/// Optional presentation slot; the original caption shell and window controls remain owners.
pub(crate) fn prefixed_controlled_titlebar(
    selector: &'static str,
    close_id: &'static str,
    title: impl Into<SharedString>,
    capabilities: CaptionCapabilities,
    prefix: impl IntoElement,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    WindowCaption {
        selector,
        close_id,
        title: title.into(),
        prefix: Some(prefix.into_any_element()),
        content: None,
        capabilities,
        on_close: Some(Box::new(on_close)),
    }
    .into_any_element()
}

/// Caption content supplies its own empty drag area so interactive tabs and commands remain client hits.
pub(crate) fn slotted_controlled_titlebar(
    selector: &'static str,
    close_id: &'static str,
    content: impl IntoElement,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    WindowCaption {
        selector,
        close_id,
        title: SharedString::default(),
        prefix: None,
        content: Some(content.into_any_element()),
        capabilities: CaptionCapabilities::RESIZABLE,
        on_close: Some(Box::new(on_close)),
    }
    .into_any_element()
}

pub(crate) fn client_titlebar(
    _theme: Theme,
    selector: &'static str,
    close_id: &'static str,
    title: impl Into<SharedString>,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    controlled_titlebar(
        selector,
        close_id,
        title,
        CaptionCapabilities::CLOSE,
        on_close,
    )
}
impl RenderOnce for WindowCaption {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let controls = self.capabilities;
        let maximized = window.is_maximized();
        let selector = self.selector;
        let close = self.on_close;
        caption_shell(
            selector,
            div()
                .size_full()
                .flex()
                .items_center()
                .child(self.content.unwrap_or_else(|| {
                    div()
                        .id("caption-drag-region")
                        .debug_selector(|| "caption-drag-region".into())
                        .h_full()
                        .flex_1()
                        .min_w_0()
                        .pl(px(12.))
                        .flex()
                        .items_center()
                        .window_control_area(WindowControlArea::Drag)
                        .when_some(self.prefix, |bar, prefix| bar.gap_2().child(prefix))
                        .child(div().min_w_0().truncate().child(self.title))
                        .into_any_element()
                }))
                .when(controls.minimize, |bar| {
                    bar.child(caption_control(
                        "window-minimize",
                        IconName::WindowMinimize,
                        "最小化",
                        false,
                        Some(WindowControlArea::Min),
                        cx,
                        |_, window, _| window.minimize_window(),
                    ))
                })
                .when(controls.maximize, |bar| {
                    bar.child(caption_control(
                        "window-maximize",
                        if maximized {
                            IconName::WindowRestore
                        } else {
                            IconName::WindowMaximize
                        },
                        if maximized { "还原" } else { "最大化" },
                        false,
                        Some(WindowControlArea::Max),
                        cx,
                        |_, window, _| window.zoom_window(),
                    ))
                })
                .child(caption_control(
                    self.close_id,
                    IconName::WindowClose,
                    "关闭",
                    true,
                    None,
                    cx,
                    move |event, window, cx| {
                        if let Some(close) = &close {
                            close(event, window, cx);
                        }
                    },
                ))
                .into_any_element(),
        )
    }
}

#[derive(IntoElement)]
pub(crate) struct CaptionAction {
    id: &'static str,
    icon: IconName,
    label: &'static str,
    close: bool,
    on_click: CloseCaption,
}
pub(crate) fn caption_action(
    id: &'static str,
    icon: IconName,
    label: &'static str,
    close: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> CaptionAction {
    CaptionAction {
        id,
        icon,
        label,
        close,
        on_click: Box::new(on_click),
    }
}
impl RenderOnce for CaptionAction {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        caption_control(
            self.id,
            self.icon,
            self.label,
            self.close,
            None,
            cx,
            self.on_click,
        )
    }
}

#[derive(IntoElement)]
pub(crate) struct CaptionShell {
    selector: &'static str,
    content: AnyElement,
}
/// Public content seam for the primary shell's existing brand/utilities/native command owners.
pub(crate) fn caption_shell(selector: &'static str, content: AnyElement) -> CaptionShell {
    CaptionShell { selector, content }
}
impl RenderOnce for CaptionShell {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors;
        let title_rgb = colors.title_bar.to_rgb();
        let background_rgb = colors.background.to_rgb();
        let mixed = gpui_kit::Hsla::from(gpui_kit::Rgba {
            r: title_rgb.r * 0.55 + background_rgb.r * 0.45,
            g: title_rgb.g * 0.55 + background_rgb.g * 0.45,
            b: title_rgb.b * 0.55 + background_rgb.b * 0.45,
            a: title_rgb.a * 0.55 + background_rgb.a * 0.45,
        });
        let selector = self.selector;
        div()
            .id(selector)
            .debug_selector(move || selector.into())
            .h(px(crate::ui::metrics::TITLEBAR_HEIGHT))
            .w_full()
            .flex_none()
            .flex()
            .items_center()
            .border_b_1()
            .border_color(colors.title_bar_border)
            .bg(gpui_kit::linear_gradient(
                180.,
                gpui_kit::linear_color_stop(mixed, 0.),
                gpui_kit::linear_color_stop(colors.title_bar, 1.),
            ))
            .child(self.content)
    }
}
fn caption_control(
    id: &'static str,
    icon: IconName,
    label: &'static str,
    close: bool,
    native: Option<WindowControlArea>,
    cx: &App,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let colors = cx.theme().colors;
    let variant = ButtonCustomVariant::new(cx)
        .color(cx.theme().transparent)
        .foreground(colors.foreground)
        .hover(if close {
            colors.danger
        } else {
            colors.secondary_hover
        })
        .active(if close {
            colors.danger_active
        } else {
            colors.secondary_active
        });
    let mut button = Button::new(id).debug_selector(move || id.into())
        .accessibility_label(label).tooltip(label).custom(variant)
        .w(px(crate::ui::metrics::TITLEBAR_HEIGHT)).h_full().p_0().flex_none().rounded(px(0.))
        .child(div().debug_selector(move || format!("{id}-icon")).size(px(16.)).child(Icon::new(icon).small()))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(move |event, window, cx| {
            cx.stop_propagation();
            let correlation_id = uuid::Uuid::new_v4();
            tracing::info!(event="window.caption.action", %correlation_id, action=label, "window caption action requested");
            on_click(event, window, cx);
            tracing::info!(event="window.caption.dispatched", %correlation_id, action=label, "window caption returned to its original owner");
        });
    if cfg!(target_os = "windows")
        && let Some(native) = native
    {
        button = button.window_control_area(native);
    }
    button.into_any_element()
}

#[cfg(test)]
#[path = "titlebar_tests.rs"]
mod tests;
