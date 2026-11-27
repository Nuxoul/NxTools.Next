//! Opt-in hidden Windows GPU fixture: an ordinary Acrylic window's contour (background and 1 DIP
//! border) must equal the single-element UnderBorder reference, pixel for pixel.
use gpui_kit::component::Root;
use gpui_kit::{
    AppContext, Bounds, IntoElement, Render, Styled as _, WindowBackgroundAppearance, WindowBounds,
    WindowDecorations, WindowOptions, div, point, px, size,
};
use std::cell::RefCell;
use std::rc::Rc;

struct Empty;
impl Render for Empty {
    fn render(
        &mut self,
        _: &mut gpui_kit::Window,
        _: &mut gpui_kit::Context<Self>,
    ) -> impl IntoElement {
        div().size_full()
    }
}

/// Reference: one element carrying fill, 1 px border and the 8 DIP radius.
struct Reference {
    fill: gpui_kit::Rgba,
    border: gpui_kit::Rgba,
}
impl Render for Reference {
    fn render(
        &mut self,
        _: &mut gpui_kit::Window,
        _: &mut gpui_kit::Context<Self>,
    ) -> impl IntoElement {
        div()
            .size_full()
            .rounded(px(8.0))
            .bg(self.fill)
            .border_1()
            .border_color(self.border)
    }
}

fn open<V: Render + 'static>(
    cx: &mut gpui_kit::App,
    client: bool,
    build: impl FnOnce(&mut gpui_kit::Window, &mut gpui_kit::App) -> gpui_kit::Entity<V> + 'static,
) -> gpui_kit::AnyWindowHandle {
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                point(px(40.0), px(40.0)),
                size(px(360.0), px(240.0)),
            ))),
            titlebar: None,
            window_decorations: Some(if client {
                WindowDecorations::Client
            } else {
                WindowDecorations::Server
            }),
            window_background: WindowBackgroundAppearance::Transparent,
            show: false,
            focus: false,
            ..Default::default()
        },
        move |window, cx| build(window, cx),
    )
    .expect("hidden GPU window")
    .into()
}

fn capture(handle: gpui_kit::AnyWindowHandle, cx: &mut gpui_kit::App) -> image::RgbaImage {
    handle
        .update(cx, |_, window, cx| {
            for _ in 0..2 {
                window.refresh();
                window.draw(cx).clear(cx);
            }
            window.render_to_image().expect("GPU readback")
        })
        .expect("window update")
}

/// Colour and coverage differences over the four 24 px corners. Colour is compared where both
/// frames cover the pixel (straight alpha, so a doubled-coverage rim shows as a colour error);
/// coverage may differ by the official frame's faint drop shadow only.
struct Difference {
    colour: f32,
    coverage: f32,
    samples: Vec<(u32, u32, [u8; 4], [u8; 4])>,
}

fn corner_difference(a: &image::RgbaImage, b: &image::RgbaImage) -> Difference {
    let (w, h) = (a.width().min(b.width()), a.height().min(b.height()));
    let mut out = Difference {
        colour: 0.0,
        coverage: 0.0,
        samples: Vec::new(),
    };
    for (ox, oy) in [(0, 0), (w - 24, 0), (0, h - 24), (w - 24, h - 24)] {
        for y in oy..oy + 24 {
            for x in ox..ox + 24 {
                let (p, q) = (a.get_pixel(x, y).0, b.get_pixel(x, y).0);
                out.coverage = out.coverage.max((p[3] as f32 - q[3] as f32).abs());
                if p[3] >= 32 && q[3] >= 32 {
                    let d = (0..3)
                        .map(|c| (p[c] as f32 - q[c] as f32).abs())
                        .fold(0.0, f32::max);
                    out.colour = out.colour.max(d);
                    if d > 3.0 && out.samples.len() < 8 {
                        out.samples.push((x, y, p, q));
                    }
                }
            }
        }
    }
    out
}
#[test]
#[ignore = "explicit owned hidden Windows GPU fixture"]
fn ordinary_acrylic_window_contour_matches_the_single_element_under_border_reference() {
    let outcome = Rc::new(RefCell::new(None));
    let recorded = outcome.clone();
    gpui_kit::application()
        .with_assets(crate::assets::NxToolsAssets)
        .run(move |cx| {
            let result = (|| -> anyhow::Result<Difference> {
                crate::assets::register_fonts(cx)?;
                gpui_kit::init(cx);
                crate::theme::apply_component_theme(cx);
                crate::theme::set_appearance_preferences(
                    nxtools_core::ThemePreference::Dark,
                    nxtools_core::MaterialStyle::Acrylic,
                    cx,
                );
                let colors = gpui_kit::component::Theme::global(cx).colors;
                let fill = gpui_kit::Rgba::from(colors.background);
                let fill = gpui_kit::Rgba { a: 1.0, ..fill };
                let border = gpui_kit::Rgba::from(crate::theme::material::contour(cx));
                let production = open(cx, true, |window, cx| {
                    let view = cx.new(|_| Empty);
                    cx.new(|cx| Root::new(view, window, cx))
                });
                let reference = open(cx, false, move |_, cx| {
                    cx.new(|_| Reference { fill, border })
                });
                let a = capture(production, cx);
                let b = capture(reference, cx);
                anyhow::ensure!(
                    (a.width(), a.height()) == (b.width(), b.height()),
                    "same canvas: {:?} vs {:?}",
                    a.dimensions(),
                    b.dimensions()
                );
                if let Ok(dir) = std::env::var("PI_SCRATCH_DIR") {
                    let _ = a.save(std::path::Path::new(&dir).join("corner-production.png"));
                    let _ = b.save(std::path::Path::new(&dir).join("corner-reference.png"));
                }
                // The 1 DIP contour must sit on the window rectangle itself, not 1 px inside it.
                let (w, h) = a.dimensions();
                for (x, y) in [(w / 2, 0), (w / 2, h - 1), (0, h / 2), (w - 1, h / 2)] {
                    anyhow::ensure!(
                        a.get_pixel(x, y).0 == b.get_pixel(x, y).0,
                        "edge pixel ({x},{y}) {:?} != reference {:?}",
                        a.get_pixel(x, y).0,
                        b.get_pixel(x, y).0
                    );
                }
                Ok(corner_difference(&a, &b))
            })();
            *recorded.borrow_mut() = Some(result);
            cx.quit();
        });
    let difference = outcome
        .borrow_mut()
        .take()
        .expect("fixture completed")
        .expect("GPU frames");
    println!(
        "corner.gpu.colour_difference={} coverage_difference={}",
        difference.colour, difference.coverage
    );
    for case in &difference.samples {
        println!(
            "corner.gpu.sample x={} y={} production={:?} reference={:?}",
            case.0, case.1, case.2, case.3
        );
    }
    assert!(
        difference.colour <= 3.0,
        "ordinary window contour colour differs from the UnderBorder reference by {}/255",
        difference.colour
    );
    assert!(
        difference.coverage <= 32.0,
        "coverage differs by {}/255; only the frame shadow may add coverage",
        difference.coverage
    );
}
