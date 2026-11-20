use super::*;
use gpui_kit::component::label::Label;
use gpui_kit::{Bounds, Pixels, Point, rgb, transparent_black};
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
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut surface = div().size_full().relative().bg(transparent_black());
        if let (Some(viewport), Some(target)) = (self.viewport, self.candidate.as_ref()) {
            let bounds = highlight_bounds(viewport, target, window.scale_factor());
            let accent = rgb(0x2f8cff);
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
                            .text_color(rgb(0xffffff))
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
