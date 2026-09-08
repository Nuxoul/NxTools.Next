//! 链接识别 floating panel: one popup window with one click-to-copy row per
//! detected link. Auto-hidden by the owning clipboard application after the
//! dwell time; clicking a row copies the value and closes the panel.

use crate::clipboard::ClipboardApplication;
use gpui::{
    div, px, AnyWindowHandle, Context, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, WeakEntity, Window,
};

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
    pub(crate) generation: u64,
}

pub(crate) struct LinkPanelView {
    pub(crate) links: Vec<DetectedLinkRow>,
    pub(crate) application: WeakEntity<ClipboardApplication>,
    pub(crate) expanded: bool,
}

impl Render for LinkPanelView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = crate::theme::Theme::zed_dark();
        let application = self.application.clone();
        // Multiple targets start collapsed on the first row; the expander
        // reveals the rest (the user's spec for multi-link captures).
        let visible_count = if self.expanded || self.links.len() <= 1 {
            self.links.len()
        } else {
            1
        };
        let needs_expander = self.links.len() > 1 && !self.expanded;
        let mut container = div()
            .debug_selector(|| "link-panel".into())
            .size_full()
            .bg(theme.surface_raised)
            .border_1()
            .border_color(theme.border_subtle)
            .rounded(px(8.0))
            .overflow_hidden()
            .p(px(6.0))
            .flex()
            .flex_col()
            .gap(px(2.0));
        for (index, link) in self.links.iter().enumerate().take(visible_count) {
            let application = application.clone();
            let copy_value = link.copy_value.clone();
            container = container.child(
                div()
                    .id(("link-panel-row", index))
                    .debug_selector(move || format!("link-panel-row-{index}"))
                    .h(px(LINK_PANEL_ROW_HEIGHT))
                    .min_w_0()
                    .px(px(6.0))
                    .rounded(px(6.0))
                    .flex()
                    .items_center()
                    .hover(move |style| style.bg(theme.surface_hover))
                    .on_click(move |_event, window, cx| {
                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(copy_value.clone()));
                        if let Some(application) = application.upgrade() {
                            application.update(cx, |application, cx| {
                                application.close_link_panel(cx);
                            });
                        }
                        window.refresh();
                    })
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .text_size(px(crate::ui::typography::TEXT_LABEL))
                            .text_color(theme.text)
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
                    .h(px(LINK_PANEL_ROW_HEIGHT))
                    .px(px(6.0))
                    .rounded(px(6.0))
                    .flex()
                    .items_center()
                    .text_size(px(crate::ui::typography::TEXT_META))
                    .text_color(theme.text_muted)
                    .hover(move |style| style.bg(theme.surface_hover))
                    .on_click(cx.listener(move |this, _event, _window, cx| {
                        this.expanded = true;
                        cx.notify();
                    }))
                    .child(format!("展开其余 {} 个链接", self.links.len() - visible_count)),
            );
        }
        container
    }
}
