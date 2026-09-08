//! 链接识别 floating panel: one popup window with one click-to-copy row per
//! detected link. Auto-hidden by the owning clipboard application after the
//! dwell time; clicking a row copies the value and closes the panel.

use crate::clipboard::ClipboardApplication;
use gpui::{
    div, px, rgb, AnyWindowHandle, Context, InteractiveElement, IntoElement, ParentElement,
    Render, StatefulInteractiveElement, Styled, WeakEntity, Window,
};
use gpui_component::Icon;

pub(crate) type DetectedLinkRow = crate::tools::link_detector::DetectedLink;

pub(crate) const LINK_PANEL_WIDTH: f32 = 420.0;
const LINK_PANEL_ROW_HEIGHT: f32 = 30.0;
const LINK_PANEL_VERTICAL_PADDING: f32 = 12.0;
pub(crate) const LINK_PANEL_AUTO_HIDE_MS: u64 = 6_000;

pub(crate) const fn link_panel_height(rows: usize) -> f32 {
    LINK_PANEL_VERTICAL_PADDING * 2.0 + rows as f32 * LINK_PANEL_ROW_HEIGHT
}

#[derive(Default)]
pub(crate) struct LinkPanelSession {
    pub(crate) window: Option<AnyWindowHandle>,
    pub(crate) native_window_handle: i64,
    pub(crate) generation: u64,
}

pub(crate) struct LinkPanelView {
    pub(crate) links: Vec<DetectedLinkRow>,
    pub(crate) application: WeakEntity<ClipboardApplication>,
    pub(crate) expanded: bool,
}

impl LinkPanelView {
    /// Restages the reused panel window with a fresh capture's links.
    pub(crate) fn swap_links(&mut self, links: Vec<DetectedLinkRow>) {
        self.links = links;
        self.expanded = false;
    }
}

impl Render for LinkPanelView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Spec palette (v1.0): floating panel = bg-surface-3 #27272a, 6px
        // radius, 1px #3f3f46 border, text-primary/secondary hierarchy,
        // accent-primary hover rows, footer with auto-hide countdown.
        let surface = rgb(0x27272a);
        let border = rgb(0x3f3f46);
        let text_primary = rgb(0xf4f4f5);
        let text_secondary = rgb(0xa1a1aa);
        let text_muted = rgb(0x71717a);
        let accent = rgb(0x0ea5e9);
        let accent_hover = rgb(0x38bdf8);
        let success = rgb(0x10b981);
        let application = self.application.clone();
        let visible_count = if self.expanded || self.links.len() <= 1 {
            self.links.len()
        } else {
            1
        };
        let needs_expander = self.links.len() > 1 && !self.expanded;
        let mut container = div()
            .debug_selector(|| "link-panel".into())
            .size_full()
            .bg(surface)
            .border_1()
            .border_color(border)
            .rounded(px(6.0))
            .overflow_hidden()
            .flex()
            .flex_col()
            .py(px(4.0));
        for (index, link) in self.links.iter().enumerate().take(visible_count) {
            let application = application.clone();
            let copy_value = link.copy_value.clone();
            container = container.child(
                div()
                    .id(("link-panel-row", index))
                    .debug_selector(move || format!("link-panel-row-{index}"))
                    .h(px(28.0))
                    .min_w_0()
                    .mx(px(4.0))
                    .px(px(8.0))
                    .rounded(px(6.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .cursor_pointer()
                    .hover(move |style| style.bg(rgb(0x3f3f46)))
                    .on_click(move |_event, window, cx| {
                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                            copy_value.clone(),
                        ));
                        if let Some(application) = application.upgrade() {
                            application.update(cx, |application, cx| {
                                application.close_link_panel(cx);
                            });
                        }
                        window.refresh();
                    })
                    .child(
                        Icon::new(crate::icons::MaterialIcon::ContentCopy)
                            .size(px(12.0))
                            .text_color(text_muted),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .text_size(px(12.0))
                            .text_color(text_primary)
                            .truncate()
                            .child(link.display.clone()),
                    ),
            );
        }
        if needs_expander {
            container = container.child(
                div()
                    .id("link-panel-expander")
                    .debug_selector(|| "link-panel-expander".into())
                    .h(px(24.0))
                    .mx(px(4.0))
                    .px(px(8.0))
                    .rounded(px(6.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .cursor_pointer()
                    .text_size(px(11.0))
                    .text_color(text_secondary)
                    .hover(move |style| style.bg(rgb(0x3f3f46)))
                    .on_click(cx.listener(move |this, _event, _window, cx| {
                        this.expanded = true;
                        cx.notify();
                    }))
                    .child(format!(
                        "展开其余 {} 个链接",
                        self.links.len() - visible_count
                    )),
            );
        }
        container = container.child(
            div()
                .debug_selector(|| "link-panel-footer".into())
                .h(px(22.0))
                .mt(px(2.0))
                .mx(px(4.0))
                .px(px(8.0))
                .flex()
                .items_center()
                .justify_between()
                .border_t_1()
                .border_color(border)
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(text_secondary)
                        .child(if self.links.len() > 1 {
                            format!("{} 个链接", self.links.len())
                        } else {
                            "1 个链接".to_owned()
                        }),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .child(
                            div()
                                .text_size(px(10.0))
                                .text_color(success)
                                .child("点击复制"),
                        )
                        .child(
                            div()
                                .text_size(px(10.0))
                                .text_color(text_muted)
                                .child("自动消失"),
                        ),
                ),
        );
        let _ = (accent, accent_hover, application);
        container
    }
}
