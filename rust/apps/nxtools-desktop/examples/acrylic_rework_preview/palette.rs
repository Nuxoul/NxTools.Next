//! Fluent-style neutral material palette. The color scheme only contributes accent/selection.
use gpui_kit::component::ThemeColor;
use gpui_kit::{Hsla, rgb};

fn ink(color: u32, alpha: f32) -> Hsla {
    let mut value: Hsla = rgb(color).into();
    value.a = alpha;
    value
}

/// Surfaces the preview paints itself; everything else flows through the Kit theme.
#[derive(Clone, Copy, Debug)]
pub struct Layers {
    /// Root window fill under title bar and sidebar.
    pub window: Hsla,
    /// The single content layer, already scaled by the slider.
    pub content: Hsla,
    /// Opaque control fill for text inputs.
    pub input: Hsla,
    /// 1px window contour drawn on the root element.
    pub contour: Hsla,
    /// Card stroke.
    pub card_stroke: Hsla,
}

pub const MIN_CONTENT_PERCENT: f32 = 50.0;
pub const DEFAULT_CONTENT_PERCENT: f32 = 50.0;

/// Content-layer alpha for a slider percentage; dark mode runs at 0.9 of the light value.
pub fn content_alpha(percent: f32, dark: bool) -> f32 {
    let light = percent.clamp(MIN_CONTENT_PERCENT, 100.0) / 100.0;
    if dark { light * 0.9 } else { light }
}

pub fn frosted_layers(dark: bool, percent: f32) -> Layers {
    let alpha = content_alpha(percent, dark);
    if dark {
        Layers {
            window: ink(0x000000, 0.0),
            content: ink(0x3A3A3A, alpha),
            input: ink(0x2D2D2D, 1.0),
            contour: ink(0xFFFFFF, 0.10),
            card_stroke: ink(0x000000, 0.10),
        }
    } else {
        Layers {
            window: ink(0x000000, 0.0),
            content: ink(0xFFFFFF, alpha),
            input: ink(0xFFFFFF, 1.0),
            contour: ink(0x000000, 0.10),
            card_stroke: ink(0x000000, 0.06),
        }
    }
}

pub fn solid_layers(colors: &ThemeColor) -> Layers {
    Layers {
        window: colors.background,
        content: colors.background,
        input: colors.background,
        contour: colors.border,
        card_stroke: colors.border,
    }
}

struct Neutral {
    text: Hsla,
    secondary: Hsla,
    divider: Hsla,
    hover: Hsla,
    card: Hsla,
    control: Hsla,
    control_hover: Hsla,
    control_pressed: Hsla,
    input_stroke: Hsla,
    flyout: Hsla,
    thumb: Hsla,
    thumb_hover: Hsla,
}

impl Neutral {
    fn new(dark: bool) -> Self {
        if dark {
            Self {
                text: ink(0xFFFFFF, 1.0),
                secondary: ink(0xFFFFFF, 0.786),
                divider: ink(0xFFFFFF, 0.08),
                hover: ink(0xFFFFFF, 0.06),
                card: ink(0xFFFFFF, 0.05),
                control: ink(0xFFFFFF, 0.061),
                control_hover: ink(0xFFFFFF, 0.084),
                control_pressed: ink(0xFFFFFF, 0.033),
                input_stroke: ink(0xFFFFFF, 0.09),
                flyout: ink(0x2C2C2C, 1.0),
                thumb: ink(0xFFFFFF, 0.36),
                thumb_hover: ink(0xFFFFFF, 0.54),
            }
        } else {
            Self {
                text: ink(0x000000, 0.894),
                secondary: ink(0x000000, 0.62),
                divider: ink(0x000000, 0.08),
                hover: ink(0x000000, 0.04),
                card: ink(0xFFFFFF, 0.70),
                control: ink(0xFFFFFF, 0.70),
                control_hover: ink(0xF9F9F9, 0.50),
                control_pressed: ink(0xF9F9F9, 0.30),
                input_stroke: ink(0x000000, 0.10),
                flyout: ink(0xF9F9F9, 1.0),
                thumb: ink(0x000000, 0.36),
                thumb_hover: ink(0x000000, 0.54),
            }
        }
    }
}

/// Rewrites the neutral roles of a Catppuccin-derived palette into the material palette.
pub fn apply_frosted(colors: &mut ThemeColor, dark: bool) {
    let clear = ink(0x000000, 0.0);
    let neutral = Neutral::new(dark);
    let mut selected = colors.primary;
    selected.a = if dark { 0.20 } else { 0.14 };
    let mut text_selection = colors.primary;
    text_selection.a = 0.30;

    colors.background = clear;
    colors.foreground = neutral.text;
    colors.muted = neutral.hover;
    colors.muted_foreground = neutral.secondary;
    colors.border = neutral.divider;
    colors.input = neutral.input_stroke;

    for role in [
        &mut colors.title_bar,
        &mut colors.title_bar_border,
        &mut colors.sidebar,
        &mut colors.sidebar_border,
        &mut colors.status_bar,
        &mut colors.status_bar_border,
        &mut colors.tab_bar,
        &mut colors.list,
        &mut colors.list_even,
        &mut colors.list_head,
        &mut colors.table,
        &mut colors.table_even,
        &mut colors.table_head,
        &mut colors.window_border,
        &mut colors.scrollbar,
        &mut colors.list_active_border,
        &mut colors.table_active_border,
    ] {
        *role = clear;
    }
    colors.sidebar_foreground = neutral.text;
    colors.sidebar_accent = neutral.hover;
    colors.sidebar_accent_foreground = neutral.text;

    colors.accent = neutral.hover;
    colors.accent_foreground = neutral.text;
    colors.list_hover = neutral.hover;
    colors.table_hover = neutral.hover;
    colors.list_active = selected;
    colors.table_active = selected;
    colors.selection = text_selection;

    colors.group_box = neutral.card;
    colors.group_box_foreground = neutral.text;

    colors.button = neutral.control;
    colors.button_hover = neutral.control_hover;
    colors.button_active = neutral.control_pressed;
    colors.button_foreground = neutral.text;
    colors.secondary = neutral.control;
    colors.secondary_hover = neutral.control_hover;
    colors.secondary_active = neutral.control_pressed;
    colors.secondary_foreground = neutral.text;

    colors.popover = neutral.flyout;
    colors.popover_foreground = neutral.text;

    colors.scrollbar_thumb = neutral.thumb;
    colors.scrollbar_thumb_hover = neutral.thumb_hover;
}
