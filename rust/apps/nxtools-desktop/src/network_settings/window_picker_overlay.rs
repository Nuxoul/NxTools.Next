use super::*;
use gpui_kit::component::label::Label;
use gpui_kit::{Bounds, Pixels, Point, transparent_black};
use nxtools_protocol::process_window_pick::{WindowPickCandidate, WindowPickRect};

pub(super) struct WindowPickerOverlay {
    pub window: AnyWindowHandle,
    pub view: Entity<WindowPickerOverlayView>,
    pub native_window_handle: i64,
}

pub(super) struct WindowPickerOverlayView {
    pub viewport: Option<WindowPickRect>,
    pub candidate: Option<WindowPickCandidate>,
}

impl WindowPickerOverlay {
    pub fn open(cx: &mut Context<NetworkSettingsApplication>) -> anyhow::Result<Self> {
        let retained = Rc::new(RefCell::new(None));
        let slot = retained.clone();
        let handle = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    gpui_kit::point(px(0.0), px(0.0)),
                    size(px(1.0), px(1.0)),
                ))),
                titlebar: None,
                focus: false,
                show: false,
                kind: WindowKind::PopUp,
                is_movable: false,
                is_resizable: false,
                is_minimizable: false,
                window_background: WindowBackgroundAppearance::Transparent,
                window_decorations: Some(WindowDecorations::Client),
                app_id: Some("NxTools.ProcessWindowPick".into()),
                ..Default::default()
            },
            move |window, cx| {
                let view = cx.new(|_| WindowPickerOverlayView {
                    viewport: None,
                    candidate: None,
                });
                let native = crate::ui::native_window_handle(window).unwrap_or_default();
                *slot.borrow_mut() = Some((view.clone(), native));
                let handle = window.window_handle();
                cx.defer(move |cx| {
                    let _ = handle.update(cx, |_, window, cx| {
                        if let Some(root) = window.root::<Root>().flatten() {
                            root.update(cx, |root, cx| {
                                root.style().background = Some(transparent_black().into());
                                cx.notify();
                            });
                        }
                    });
                });
                cx.new(|cx| Root::new(view, window, cx))
            },
        )?;
        let (view, native_window_handle) =
            retained.borrow_mut().take().expect("created picker view");
        Ok(Self {
            window: handle.into(),
            view,
            native_window_handle,
        })
    }
}

pub(super) fn highlight_bounds(
    viewport: WindowPickRect,
    target: &WindowPickCandidate,
    scale: f32,
) -> Bounds<Pixels> {
    let scale = scale.max(1.0);
    let target_left = i64::from(target.left) - i64::from(viewport.left);
    let target_top = i64::from(target.top) - i64::from(viewport.top);
    let left = target_left.clamp(0, i64::from(viewport.width));
    let top = target_top.clamp(0, i64::from(viewport.height));
    let right = (target_left + i64::from(target.width)).clamp(0, i64::from(viewport.width));
    let bottom = (target_top + i64::from(target.height)).clamp(0, i64::from(viewport.height));
    Bounds::new(
        Point {
            x: px(left as f32 / scale),
            y: px(top as f32 / scale),
        },
        size(
            px((right - left) as f32 / scale),
            px((bottom - top) as f32 / scale),
        ),
    )
}

impl Render for WindowPickerOverlayView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut surface = div().size_full().relative().bg(transparent_black());
        if let (Some(viewport), Some(target)) = (self.viewport, self.candidate.as_ref()) {
            let bounds = highlight_bounds(viewport, target, window.scale_factor());
            let colors = &gpui_kit::component::Theme::global(cx).colors;
            let accent = colors.primary;
            surface = surface.child(
                div()
                    .debug_selector(|| "blacklist-window-picker-highlight".into())
                    .absolute()
                    .left(bounds.origin.x)
                    .top(bounds.origin.y)
                    .w(bounds.size.width)
                    .h(bounds.size.height)
                    .border_2()
                    .border_color(accent)
                    .overflow_hidden()
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .left_0()
                            .max_w_full()
                            .px_2()
                            .bg(accent)
                            .text_color(colors.primary_foreground)
                            .child(
                                Label::new(format!(
                                    "{}  {} × {} · 左键选取，Esc 取消",
                                    target.process_name, target.width, target.height
                                ))
                                .text_sm(),
                            ),
                    ),
            );
        }
        surface
    }
}

#[cfg(test)]
mod material_tests {
    use super::*;

    #[gpui_kit::test]
    fn material_picker_highlight_uses_public_accent_without_desktop_fill(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        cx.update(crate::theme::apply_component_theme);
        let (_, surface) = cx.add_window_view(|window, cx| {
            let view = cx.new(|_| WindowPickerOverlayView {
                viewport: Some(WindowPickRect {
                    left: 0,
                    top: 0,
                    width: 400,
                    height: 300,
                }),
                candidate: Some(WindowPickCandidate {
                    window_handle: 201,
                    process_id: 900,
                    process_name: "fixture.exe".into(),
                    left: 20,
                    top: 40,
                    width: 200,
                    height: 120,
                }),
            });
            Root::new(view, window, cx).bg(transparent_black())
        });
        surface.simulate_resize(size(px(400.0), px(300.0)));
        for style in nxtools_core::MaterialStyle::ALL {
            surface.update(|window, cx| {
                crate::theme::set_appearance_preferences(
                    nxtools_core::ThemePreference::Dark,
                    style,
                    cx,
                );
                window.refresh();
                window.draw(cx).clear(cx);
                let accent = gpui_kit::component::Theme::global(cx).colors.primary;
                let quads = window.painted_quads();
                assert!(quads.iter().any(|quad| quad.border_color == accent));
                assert!(
                    quads.iter().all(|quad| quad.background.is_transparent()
                        || quad.bounds.size.width.0 < 400.0 * window.scale_factor()
                        || quad.bounds.size.height.0 < 300.0 * window.scale_factor()),
                    "sparse host never paints the desktop-sized canvas"
                );
            });
        }
    }
}
