//! Frosted palette: the selected colour scheme tinted over Windows acrylic.
//!
//! Every frosted window sits on one veil of the scheme's canvas colour over acrylic: DWM system
//! acrylic on Windows 11, and on Windows 10 the host backdrop run through the same Fluent recipe
//! (a luminosity layer, then a tint layer; see `ui/material/win10/host.rs`). Both carry the
//! luminosity layer that keeps text readable over any wallpaper, so one set of layers serves
//! both; Windows 10 adds a fine grain for the noise system acrylic has. Cards and controls are
//! low-alpha lifts, and menus and dialogs stay opaque.
use gpui_kit::component::ThemeColor;
use gpui_kit::{App, Hsla, Rgba, rgb};
use nxtools_core::MaterialTuning;

/// Exterior radius of a client-decorated window where DWM rounds windows, in DIP.
pub(crate) const WINDOW_RADIUS: f32 = 8.0;

/// Veil alpha over acrylic, `[light, dark]`.
const VEIL: [f32; 2] = [0.50, 0.55];
/// Share of body ink mixed into secondary and muted text on frosted surfaces. Scheme supporting
/// text is tuned for its own opaque canvas; over a translucent veil a bright or saturated desktop
/// erodes that margin.
const SUPPORTING_INK: f32 = 0.35;
/// Grain over the Windows 10 host acrylic, `[light, dark]`: the largest share of white or black a
/// grain pixel adds, standing in for system acrylic's noise.
pub(crate) const BLUR_GRAIN: [f32; 2] = [0.025, 0.03];
/// Fluent acrylic over the Windows 10 host backdrop: opacities of the luminosity and tint layers
/// in percent, `[light, dark]`. The luminosity layer gives the blurred desktop the tint's
/// brightness while keeping its hue; XAML window acrylic uses 85 / 96 %, which leaves little but
/// the tint. These keep body text at 7:1 over a black-to-white desktop under the veil alone.
pub(crate) const HOST_ACRYLIC: [(u8, u8); 2] = [(60, 0), (70, 15)];

/// Colour of the 1 DIP window contour the Root surface paints itself, so background and border
/// form one UnderBorder-compensated contour.
#[derive(Clone, Copy)]
pub(crate) struct Contour(pub Hsla);
impl gpui_kit::Global for Contour {}

pub(crate) fn contour(cx: &App) -> Hsla {
    cx.try_global::<Contour>().map_or_else(
        || gpui_kit::component::Theme::global(cx).colors.window_border,
        |contour| contour.0,
    )
}

/// Opaque scheme colours the frosted layers are tinted with, captured before the neutral roles
/// are rewritten.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Tint {
    canvas: Hsla,
    raised: Hsla,
}
impl gpui_kit::Global for Tint {}

impl Tint {
    fn from_scheme(colors: &ThemeColor, dark: bool) -> Self {
        let canvas = opaque(colors.background);
        let white = ink(0xFFFFFF, 1.0);
        Self {
            canvas,
            raised: mix(canvas, white, if dark { 0.10 } else { 0.70 }),
        }
    }

    pub(super) fn neutral(dark: bool) -> Self {
        if dark {
            Self {
                canvas: ink(0x202020, 1.0),
                raised: ink(0x363636, 1.0),
            }
        } else {
            Self {
                canvas: ink(0xF3F3F3, 1.0),
                raised: ink(0xFDFDFD, 1.0),
            }
        }
    }
}

/// The fills of one frosted window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Layers {
    /// Root fill of windows that paint their own content layer (title bar and sidebar), and of
    /// transient panels.
    pub veil: Hsla,
    /// Content area of a persistent window.
    pub content: Hsla,
    /// Root fill of transient panels.
    pub transient: Hsla,
    /// Base when the system withholds the backdrop.
    pub opaque: Hsla,
}

impl Layers {
    pub(crate) fn new(tint: Tint, dark: bool, tuning: MaterialTuning) -> Self {
        let veil = with_alpha(tint.canvas, VEIL[usize::from(dark)]);
        Self {
            veil,
            content: with_alpha(tint.raised, content_alpha(tuning, dark)),
            transient: veil,
            opaque: tint.canvas,
        }
    }

    /// Root fill of a persistent window without its own content area: content over veil.
    pub(crate) fn backdrop(self) -> Hsla {
        over(self.content, self.veil)
    }
}

/// Layers for the active scheme and appearance.
pub(crate) fn layers(cx: &App) -> Layers {
    let state = cx
        .try_global::<super::UiThemeState>()
        .copied()
        .unwrap_or_default();
    let dark = state.is_dark();
    let tint = cx
        .try_global::<Tint>()
        .copied()
        .unwrap_or_else(|| Tint::neutral(dark));
    Layers::new(tint, dark, state.tuning)
}

/// Frosted windows sit on the host backdrop (Windows 10) rather than system acrylic.
pub(crate) fn host_backdrop(cx: &App) -> bool {
    cx.try_global::<super::UiThemeState>()
        .is_some_and(|state| state.material_style.is_frosted())
        && !crate::ui::material::system_backdrop(cx)
}

/// Grain amplitude for frosted windows on the Windows 10 host backdrop; system acrylic carries
/// its own noise.
pub(crate) fn blur_grain(cx: &App) -> Option<f32> {
    host_backdrop(cx).then(|| {
        let dark = cx
            .try_global::<super::UiThemeState>()
            .copied()
            .unwrap_or_default()
            .is_dark();
        BLUR_GRAIN[usize::from(dark)]
    })
}

/// The host-backdrop acrylic recipe for the active scheme: its canvas as `0xRRGGBB` and the
/// luminosity and tint layer opacities in percent.
pub(crate) fn host_acrylic(cx: &App) -> (u32, u8, u8) {
    let dark = cx
        .try_global::<super::UiThemeState>()
        .copied()
        .unwrap_or_default()
        .is_dark();
    let canvas = cx
        .try_global::<Tint>()
        .copied()
        .unwrap_or_else(|| Tint::neutral(dark))
        .canvas
        .to_rgb();
    let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u32;
    let rgb = (channel(canvas.r) << 16) | (channel(canvas.g) << 8) | channel(canvas.b);
    let (luminosity, tint) = HOST_ACRYLIC[usize::from(dark)];
    (rgb, luminosity, tint)
}

/// Content-layer alpha: the slider value, and 0.9 of it on dark palettes.
pub(crate) fn content_alpha(tuning: MaterialTuning, dark: bool) -> f32 {
    let light = f32::from(
        tuning
            .content_percent
            .clamp(MaterialTuning::MIN_PERCENT, MaterialTuning::MAX_PERCENT),
    ) / 100.0;
    if dark { light * 0.9 } else { light }
}

/// Edge of the content layer.
pub(crate) fn card_stroke(dark: bool) -> Hsla {
    ink(0x000000, if dark { 0.18 } else { 0.06 })
}

fn ink(color: u32, alpha: f32) -> Hsla {
    let mut value: Hsla = rgb(color).into();
    value.a = alpha;
    value
}

fn with_alpha(color: Hsla, alpha: f32) -> Hsla {
    Hsla { a: alpha, ..color }
}

fn opaque(color: Hsla) -> Hsla {
    with_alpha(color, 1.0)
}

/// `a` moved toward `b` by `t`, in sRGB.
fn mix(a: Hsla, b: Hsla, t: f32) -> Hsla {
    let (a, b) = (a.to_rgb(), b.to_rgb());
    let lerp = |x: f32, y: f32| x + (y - x) * t;
    Rgba {
        r: lerp(a.r, b.r),
        g: lerp(a.g, b.g),
        b: lerp(a.b, b.b),
        a: lerp(a.a, b.a),
    }
    .into()
}

/// One flat colour equal to `top` composited over `bottom`.
fn over(top: Hsla, bottom: Hsla) -> Hsla {
    let (top, bottom) = (top.to_rgb(), bottom.to_rgb());
    let alpha = top.a + bottom.a * (1.0 - top.a);
    if alpha <= 0.0 {
        return gpui_kit::transparent_black();
    }
    let channel = |t: f32, b: f32| (t * top.a + b * bottom.a * (1.0 - top.a)) / alpha;
    Rgba {
        r: channel(top.r, bottom.r),
        g: channel(top.g, bottom.g),
        b: channel(top.b, bottom.b),
        a: alpha,
    }
    .into()
}

/// Rewrites the neutral surface roles of the selected scheme for the frosted layers and returns
/// the window contour and the layer tint. Text, accent, selection and status colours keep the
/// scheme's hues; translucent roles are its ink at low alpha.
pub(super) fn apply(colors: &mut ThemeColor, dark: bool) -> (Hsla, Tint) {
    let tint = Tint::from_scheme(colors, dark);
    let supporting = SUPPORTING_INK;
    let clear = ink(0x000000, 0.0);
    let text = opaque(colors.foreground);
    let lift = if dark {
        ink(0xFFFFFF, 1.0)
    } else {
        tint.raised
    };
    let ink_at = |alpha: f32| with_alpha(text, alpha);
    let lift_at = |alpha: f32| with_alpha(lift, alpha);
    let (divider, hover, input) = if dark {
        (ink_at(0.10), ink_at(0.07), ink_at(0.16))
    } else {
        (ink_at(0.10), ink_at(0.05), ink_at(0.14))
    };
    let (card, control, control_hover, control_pressed) = if dark {
        (lift_at(0.05), lift_at(0.07), lift_at(0.10), lift_at(0.04))
    } else {
        (lift_at(0.72), lift_at(0.80), lift_at(0.60), lift_at(0.40))
    };
    let mut selected = colors.primary;
    selected.a = if dark { 0.22 } else { 0.14 };
    let mut text_selection = colors.primary;
    text_selection.a = 0.30;

    colors.background = clear;
    colors.border = divider;
    colors.input = input;
    colors.muted = hover;
    colors.muted_foreground = mix(colors.muted_foreground, text, supporting);
    colors.tab_foreground = mix(colors.tab_foreground, text, supporting);
    colors.table_head_foreground = mix(colors.table_head_foreground, text, supporting);

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
    colors.sidebar_accent = hover;
    colors.table_row_border = divider;

    colors.accent = hover;
    colors.list_hover = hover;
    colors.table_hover = hover;
    colors.list_active = selected;
    colors.table_active = selected;
    colors.selection = text_selection;

    colors.group_box = card;
    colors.accordion = card;

    colors.button = control;
    colors.button_hover = control_hover;
    colors.button_active = control_pressed;
    colors.secondary = control;
    colors.secondary_hover = control_hover;
    colors.secondary_active = control_pressed;

    colors.popover = opaque(colors.popover);

    colors.scrollbar_thumb = ink_at(if dark { 0.45 } else { 0.36 });
    colors.scrollbar_thumb_hover = ink_at(if dark { 0.62 } else { 0.54 });

    let contour = if dark {
        ink(0xFFFFFF, 0.10)
    } else {
        ink(0x000000, 0.10)
    };
    (contour, tint)
}

#[cfg(test)]
pub(crate) fn composite(top: Hsla, bottom: Hsla) -> Hsla {
    over(top, bottom)
}
