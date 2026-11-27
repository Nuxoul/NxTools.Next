//! Read-only rendering projection of the existing preparation owner's accepted result.
use super::*;
use gpui_kit::{
    Context, Div, Hsla, IntoElement, ParentElement as _, Refineable as _, Render, Stateful,
    Styled as _, Window, WindowBackgroundAppearance,
};
use nxtools_protocol::window_material::EffectiveWindowMaterial;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy)]
struct Projection {
    command: ApplyWindowMaterialCommand,
    effective: EffectiveWindowMaterial,
}
#[derive(Default)]
struct MaterialPaint(HashMap<i64, Projection>);
impl Global for MaterialPaint {}

/// Windows whose own content area paints the content layer, so the root carries only the veil
/// under their title bar and sidebar.
#[derive(Default)]
struct LayeredWindows(HashSet<i64>);
impl Global for LayeredWindows {}

/// The GPUI background last chosen per window and the surface kind it was chosen for.
/// Product windows are created `Transparent`.
#[derive(Clone, Copy)]
struct WindowBackground {
    appearance: WindowBackgroundAppearance,
    surface: Option<MaterialSurface>,
}
#[derive(Default)]
struct Backgrounds(HashMap<i64, WindowBackground>);
impl Global for Backgrounds {}

pub(crate) fn publish(
    command: ApplyWindowMaterialCommand,
    receipt: AppliedWindowMaterial,
    cx: &mut App,
) {
    if !is_current(&command, cx) || !receipt.matches(&command) {
        return;
    }
    let paints = cx.default_global::<MaterialPaint>();
    if paints.0.get(&command.window_handle).is_some_and(|current| {
        current.command.generation > command.generation
            || (current.command == command && current.effective == receipt.effective)
    }) {
        return;
    }
    paints.0.insert(
        command.window_handle,
        Projection {
            command,
            effective: receipt.effective,
        },
    );
    cx.refresh_windows();
}

pub(super) fn retire(hwnd: i64, cx: &mut App) {
    if cx
        .try_global::<MaterialPaint>()
        .is_some_and(|paints| paints.0.contains_key(&hwnd))
    {
        cx.global_mut::<MaterialPaint>().0.remove(&hwnd);
    }
    if cx
        .try_global::<LayeredWindows>()
        .is_some_and(|layered| layered.0.contains(&hwnd))
    {
        cx.global_mut::<LayeredWindows>().0.remove(&hwnd);
    }
    if cx
        .try_global::<Backgrounds>()
        .is_some_and(|backgrounds| backgrounds.0.contains_key(&hwnd))
    {
        cx.global_mut::<Backgrounds>().0.remove(&hwnd);
    }
}

fn projection(hwnd: i64, cx: &App) -> Option<Projection> {
    cx.try_global::<MaterialPaint>()?
        .0
        .get(&hwnd)
        .filter(|projection| is_current(&projection.command, cx))
        .copied()
}

pub(crate) fn effective(hwnd: i64, cx: &App) -> Option<EffectiveWindowMaterial> {
    projection(hwnd, cx).map(|projection| projection.effective)
}

/// A persistent window whose content area paints the content layer itself; title bar and
/// sidebar then show the veiled material.
pub(crate) fn mark_layered(hwnd: i64, cx: &mut App) {
    if hwnd != 0 {
        cx.default_global::<LayeredWindows>().0.insert(hwnd);
    }
}

/// The content-layer fill for a layered window's content area while its frosted material is
/// live; `None` when the window is solid or the system withholds the backdrop.
pub(crate) fn content_layer(window: &Window, cx: &App) -> Option<Hsla> {
    let hwnd = crate::ui::native_window_handle(window);
    #[cfg(test)]
    let hwnd = runtime::test_window_handle(window.window_handle(), cx).or(hwnd);
    let frosted = cx
        .try_global::<crate::theme::UiThemeState>()
        .copied()
        .unwrap_or_default()
        .material_style
        .is_frosted();
    (effective(hwnd?, cx) == Some(EffectiveWindowMaterial::Acrylic) && frosted)
        .then(|| crate::theme::material::layers(cx).content)
}

fn is_layered(hwnd: i64, cx: &App) -> bool {
    cx.try_global::<LayeredWindows>()
        .is_some_and(|layered| layered.0.contains(&hwnd))
}

/// Solid windows render `Opaque` so GPUI keeps ClearType text; frosted and self-drawn windows
/// stay `Transparent`. A window's surface kind is remembered from its last confirmed material,
/// so a style change switches it before the next receipt, and an unknown (transparent) host is
/// never shown with an opaque clear. Returning to `Transparent` rewrites the WCA accent, so the
/// native material is re-applied afterwards.
fn sync_background(
    hwnd: i64,
    surface: Option<MaterialSurface>,
    style: nxtools_core::MaterialStyle,
    window: &mut Window,
    cx: &mut App,
) {
    let known = cx
        .try_global::<Backgrounds>()
        .and_then(|backgrounds| backgrounds.0.get(&hwnd).copied());
    let current = known.map_or(WindowBackgroundAppearance::Transparent, |known| {
        known.appearance
    });
    let Some(surface) = surface.or(known.and_then(|known| known.surface)) else {
        return;
    };
    let desired = if surface != MaterialSurface::Transparent && !style.is_frosted() {
        WindowBackgroundAppearance::Opaque
    } else {
        WindowBackgroundAppearance::Transparent
    };
    cx.default_global::<Backgrounds>().0.insert(
        hwnd,
        WindowBackground {
            appearance: desired,
            surface: Some(surface),
        },
    );
    if desired == current {
        return;
    }
    window.set_background_appearance(desired);
    if desired == WindowBackgroundAppearance::Transparent {
        cx.defer(move |cx| native_appearance::invalidate(hwnd, cx));
    }
}

/// Windows 10 shape for a window: framed surfaces (the last confirmed kind, kept across a
/// material re-application) are region-rounded where DWM does not round, and the frosted style
/// sits on the host backdrop where system acrylic is missing.
fn win10_shape(
    hwnd: i64,
    effective: Option<EffectiveWindowMaterial>,
    cx: &App,
) -> super::win10::Wanted {
    let framed = cx
        .try_global::<Backgrounds>()
        .and_then(|backgrounds| backgrounds.0.get(&hwnd))
        .and_then(|known| known.surface)
        .is_some_and(|surface| surface != MaterialSurface::Transparent);
    let appearance = cx
        .try_global::<crate::theme::UiThemeState>()
        .copied()
        .unwrap_or_default();
    super::win10::Wanted {
        round: framed && region_corners(cx),
        acrylic: (framed
            && effective == Some(EffectiveWindowMaterial::Acrylic)
            && crate::theme::material::host_backdrop(cx))
        .then(|| {
            let (tint, luminosity, tinted) = crate::theme::material::host_acrylic(cx);
            super::win10::Recipe {
                tint,
                luminosity_percent: luminosity,
                tint_percent: tinted,
                luminosity_mode: super::win10::BLEND_COLOR,
                tint_mode: super::win10::BLEND_LUMINOSITY,
            }
        }),
        dark: appearance.is_dark(),
    }
}

pub(crate) fn init(cx: &mut App) {
    gpui_kit::base::Root::register_plugin(cx, |_, _| MaterialRoot);
}

/// Grain tile edge in device pixels; tiles are painted 1:1 so the grain stays one pixel fine.
const GRAIN_TILE: u32 = 256;

/// Grain amplitude for this window: frosted on the Windows 10 host backdrop, confirmed live.
fn grain(window: &Window, cx: &App) -> Option<f32> {
    let hwnd = crate::ui::native_window_handle(window);
    #[cfg(test)]
    let hwnd = runtime::test_window_handle(window.window_handle(), cx).or(hwnd);
    let projection = projection(hwnd?, cx)?;
    if projection.effective != EffectiveWindowMaterial::Acrylic
        || projection.command.surface == MaterialSurface::Transparent
    {
        return None;
    }
    crate::theme::material::blur_grain(cx)
}

/// Pixels of a grain tile: each adds white or black at up to `amplitude` alpha, from a fixed
/// seed so every window and frame shows the same texture.
fn grain_pixels(amplitude: f32) -> image::RgbaImage {
    let mut state: u32 = 0x9E37_79B9;
    image::RgbaImage::from_fn(GRAIN_TILE, GRAIN_TILE, |_, _| {
        let mut uniform = || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state as f32 / u32::MAX as f32
        };
        // Two uniforms give a triangular distribution: fine grain without harsh outliers.
        let value = uniform() + uniform() - 1.0;
        let level = if value >= 0.0 { 255 } else { 0 };
        let alpha = (value.abs() * amplitude * 255.0).round() as u8;
        image::Rgba([level, level, level, alpha])
    })
}

fn grain_tile(amplitude: f32) -> std::sync::Arc<gpui_kit::RenderImage> {
    thread_local! {
        static TILES: std::cell::RefCell<Vec<(u32, std::sync::Arc<gpui_kit::RenderImage>)>> =
            const { std::cell::RefCell::new(Vec::new()) };
    }
    TILES.with_borrow_mut(|tiles| {
        let key = amplitude.to_bits();
        if let Some((_, tile)) = tiles.iter().find(|(bits, _)| *bits == key) {
            return tile.clone();
        }
        let tile = std::sync::Arc::new(gpui_kit::RenderImage::new(vec![image::Frame::new(
            grain_pixels(amplitude),
        )]));
        tiles.push((key, tile.clone()));
        tile
    })
}

fn paint_grain(
    bounds: gpui_kit::Bounds<gpui_kit::Pixels>,
    tile: &std::sync::Arc<gpui_kit::RenderImage>,
    window: &mut Window,
) {
    let edge = gpui_kit::px(GRAIN_TILE as f32 / window.scale_factor());
    let mut y = bounds.top();
    while y < bounds.bottom() {
        let mut x = bounds.left();
        while x < bounds.right() {
            let tile_bounds =
                gpui_kit::Bounds::new(gpui_kit::point(x, y), gpui_kit::size(edge, edge));
            if window
                .paint_image(
                    bounds,
                    tile_bounds,
                    gpui_kit::Corners::default(),
                    tile.clone(),
                    0,
                    false,
                )
                .is_err()
            {
                return;
            }
            x += edge;
        }
        y += edge;
    }
}

struct MaterialRoot;
impl Render for MaterialRoot {
    /// The plugin overlay carries the Windows 10 grain over the whole window; it has no hitbox.
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let layer = gpui_kit::div().size_full();
        match grain(window, cx).map(grain_tile) {
            Some(tile) => layer.child(
                gpui_kit::canvas(
                    |_, _, _| {},
                    move |bounds, (), window, _| paint_grain(bounds, &tile, window),
                )
                .size_full(),
            ),
            None => layer,
        }
    }
}
impl gpui_kit::base::RootPlugin for MaterialRoot {
    fn style(&self, surface: &mut Stateful<Div>, window: &mut Window, cx: &mut App) {
        let hwnd = crate::ui::native_window_handle(window);
        #[cfg(test)]
        let hwnd = runtime::test_window_handle(window.window_handle(), cx).or(hwnd);
        let projection = hwnd.and_then(|hwnd| projection(hwnd, cx));
        let effective = projection.map(|projection| projection.effective);
        let kind = projection.map(|projection| projection.command.surface);
        let appearance = cx
            .try_global::<crate::theme::UiThemeState>()
            .copied()
            .unwrap_or_default();
        if let Some(hwnd) = hwnd {
            sync_background(hwnd, kind, appearance.material_style, window, cx);
        }
        if let Some(native) = crate::ui::native_window_handle(window) {
            super::win10::sync(native, win10_shape(native, effective, cx));
        }
        let clear = gpui_kit::transparent_black();
        let layers = crate::theme::material::layers(cx);
        let fill = match (effective, kind) {
            (Some(EffectiveWindowMaterial::Transparent), _) => Some(clear),
            _ if !appearance.material_style.is_frosted() => None,
            (Some(EffectiveWindowMaterial::Acrylic), Some(MaterialSurface::Transient)) => {
                Some(layers.transient)
            }
            (Some(EffectiveWindowMaterial::Acrylic), _)
                if hwnd.is_some_and(|hwnd| is_layered(hwnd, cx)) =>
            {
                Some(layers.veil)
            }
            (Some(EffectiveWindowMaterial::Acrylic), _) => Some(layers.backdrop()),
            // No confirmed backdrop yet, or the system withholds it.
            _ => Some(layers.opaque),
        };
        let mut style = gpui_kit::StyleRefinement::default();
        if let Some(fill) = fill {
            style = style.bg(fill);
        }
        // A client-decorated window draws its contour on the Root surface itself: the surface
        // covers the whole window rectangle (over the frame ring the official border reserves) and
        // carries fill and 1 DIP border as ONE element, so UnderBorder compensation applies and
        // the arc has no doubled coverage.
        if effective != Some(EffectiveWindowMaterial::Transparent)
            && matches!(
                window.window_decorations(),
                gpui_kit::Decorations::Client { tiling } if !tiling.is_tiled()
            )
        {
            let edge = gpui_kit::px(-1.0);
            style = style
                .absolute()
                .top(edge)
                .left(edge)
                .right(edge)
                .bottom(edge)
                .w_auto()
                .h_auto()
                .border_1()
                .border_color(crate::theme::material::contour(cx));
        }
        surface.style().refine(&style);
    }
}

#[cfg(test)]
mod tests;
