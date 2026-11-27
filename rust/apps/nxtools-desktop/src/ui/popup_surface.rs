//! Exterior geometry for the Quick Launch and Clipboard popup instances.
use gpui_kit::base::Root;
use gpui_kit::{
    AnyView, App, Context, Corners, Decorations, Pixels, Refineable as _, Styled, Window, px,
};

pub(crate) fn root(view: impl Into<AnyView>, window: &mut Window, cx: &mut Context<Root>) -> Root {
    let radii = std::rc::Rc::new(std::cell::Cell::new(corners(window, cx)));
    let refresh = {
        let radii = radii.clone();
        move |root: &mut Root, window: &mut Window, cx: &mut Context<Root>| {
            let next = corners(window, cx);
            if next != radii.get() {
                radii.set(next);
                apply(root, next);
                cx.notify();
            }
        }
    };
    cx.observe_window_bounds(window, refresh.clone()).detach();
    // The radius follows the system: Windows 11 rounds windows, Windows 10 keeps them square.
    cx.observe_global_in::<gpui_kit::component::Theme>(window, refresh)
        .detach();
    let mut root = Root::new(view, window, cx);
    apply(&mut root, radii.get());
    root
}

pub(crate) fn corners(window: &Window, cx: &App) -> Corners<Pixels> {
    corner_radii(
        window.window_decorations(),
        window.is_maximized(),
        window.is_fullscreen(),
        gpui_kit::component::Theme::global(cx).window_radius,
    )
}

pub(super) fn corner_radii(
    decorations: Decorations,
    maximized: bool,
    fullscreen: bool,
    radius: Pixels,
) -> Corners<Pixels> {
    if maximized || fullscreen {
        return Corners::default();
    }
    let tiling = match decorations {
        Decorations::Client { tiling } => tiling,
        Decorations::Server => Default::default(),
    };
    let corner = |square| if square { px(0.0) } else { radius };
    Corners {
        top_left: corner(tiling.top || tiling.left),
        top_right: corner(tiling.top || tiling.right),
        bottom_left: corner(tiling.bottom || tiling.left),
        bottom_right: corner(tiling.bottom || tiling.right),
    }
}

pub(crate) fn round<T: Styled>(mut surface: T, radii: Corners<Pixels>) -> T {
    apply(&mut surface, radii);
    surface
}

fn apply(surface: &mut impl Styled, radii: Corners<Pixels>) {
    surface.style().refine(
        &gpui_kit::StyleRefinement::default()
            .rounded_tl(radii.top_left)
            .rounded_tr(radii.top_right)
            .rounded_bl(radii.bottom_left)
            .rounded_br(radii.bottom_right),
    );
}
