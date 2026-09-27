//! 链接识别 floating panel: one popup window with one click-to-copy row per
//! detected link. Auto-hidden by the owning clipboard application after the
//! dwell time; clicking a row copies the value and closes the panel.

use crate::clipboard::ClipboardApplication;
use gpui::{
    AnyWindowHandle, Context, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, WeakEntity, Window, div, px, rgb,
};
use gpui_component::Icon;

pub(crate) type DetectedLinkRow = crate::tools::link_detector::DetectedLink;

pub(crate) const LINK_PANEL_WIDTH: f32 = 420.0;
/// Rendered row height (`.h(px(28.0))` on each link row below).
const LINK_PANEL_ROW_HEIGHT: f32 = 28.0;
/// Rendered expander height (`.h(px(24.0))` on the expander row below).
const LINK_PANEL_EXPANDER_HEIGHT: f32 = 24.0;
/// Rendered footer height (`.h(px(22.0))` on the footer below).
const LINK_PANEL_FOOTER_HEIGHT: f32 = 22.0;
/// Footer's `.mt(px(2.0))`.
const LINK_PANEL_FOOTER_MARGIN_TOP: f32 = 2.0;
/// Container's `.py(px(4.0))` top and bottom.
const LINK_PANEL_VERTICAL_PADDING: f32 = 8.0;

/// Exact window height for the panel's rendered chrome. The window is
/// fixed-size and non-resizable, so this must mirror the render below row
/// for row: a collapsed multi-link panel shows ONE row plus the expander,
/// never the hidden rows. Sizing the window for all rows (the old
/// `rows * 30 + 24` budget) left dead empty space below the footer —
/// the reported "multiple links break the layout" bug.
pub(crate) fn link_panel_height(total_rows: usize, expanded: bool) -> f32 {
    let visible_rows = if expanded || total_rows <= 1 {
        total_rows
    } else {
        1
    };
    let expander = if total_rows > 1 && !expanded {
        LINK_PANEL_EXPANDER_HEIGHT
    } else {
        0.0
    };
    LINK_PANEL_VERTICAL_PADDING
        + visible_rows as f32 * LINK_PANEL_ROW_HEIGHT
        + expander
        + LINK_PANEL_FOOTER_MARGIN_TOP
        + LINK_PANEL_FOOTER_HEIGHT
}

#[derive(Default)]
pub(crate) struct LinkPanelSession {
    pub(crate) window: Option<AnyWindowHandle>,
    pub(crate) window_handle: Option<AnyWindowHandle>,
    pub(crate) native_window_handle: i64,
    pub(crate) generation: u64,
    pub(crate) last_fingerprint: Option<(bool, String)>,
    pub(crate) last_fingerprint_at_unix_ms: i64,
    /// Text the panel itself just put on the clipboard; captures carrying
    /// this value are the panel's own echo and must not reopen it.
    pub(crate) last_panel_copied: Option<(String, i64)>,
}

pub(crate) struct LinkPanelView {
    pub(crate) links: Vec<DetectedLinkRow>,
    pub(crate) application: WeakEntity<ClipboardApplication>,
    pub(crate) expanded: bool,
    pub(crate) auto_hide_ms: u64,
    pub(crate) window_handle: Option<AnyWindowHandle>,
}

impl LinkPanelView {
    /// Restages the reused panel window with a fresh capture's links and
    /// re-arms the window-scoped auto-hide.
    pub(crate) fn swap_links(
        &mut self,
        links: Vec<DetectedLinkRow>,
        window_handle: AnyWindowHandle,
        cx: &mut Context<Self>,
    ) {
        self.links = links;
        self.expanded = false;
        self.arm_auto_hide(window_handle, cx);
    }

    /// The window closes ITSELF after the dwell time. Every app-side close
    /// went through an entity update; when the stored handle went stale the
    /// removal silently failed and the visible window leaked forever — the
    /// reported "never disappears". A window-scoped timer needs no entity.
    /// Arms the window-scoped auto-hide. The handle must come from the
    /// owning Window (Context cannot reach it), so construction sites pass
    /// it in.
    pub(crate) fn arm_auto_hide(&mut self, window_handle: AnyWindowHandle, cx: &mut Context<Self>) {
        let dwell = std::time::Duration::from_millis(self.auto_hide_ms);
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(dwell).await;
            // Closing through the captured handle is immune to stale
            // app-entity state, which is what leaked windows before.
            let _ = window_handle.update(cx, |_, window, _| window.remove_window());
            let _ = this;
        })
        .detach();
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
                        tracing::info!(
                            event = "tools.link_panel.row_clicked",
                            %copy_value,
                            "link panel row clicked"
                        );
                        // Close FIRST. Writing the link to the clipboard
                        // raises a fresh capture; if the panel is still open
                        // the echo capture restages it and the click looks
                        // like a no-op.
                        // Close via the window's own remove_window: bouncing
                        // through the app entity re-enters THIS window's update
                        // context, which errors and left the panel on screen.
                        window.remove_window();
                        if let Some(application) = application.upgrade() {
                            application.update(cx, |application, _cx| {
                                application.note_panel_copied(copy_value.clone());
                                application.forget_link_panel();
                                if application.link_panel_should_open_browser() {
                                    let url = if copy_value.contains("://") {
                                        copy_value.clone()
                                    } else {
                                        format!("https://{}", copy_value)
                                    };
                                    use std::os::windows::process::CommandExt as _;
                                    tracing::info!(
                                        event = "tools.link_panel.browser_opened",
                                        %url,
                                        "opening the clicked link in the browser"
                                    );
                                    let _ = std::process::Command::new("cmd")
                                        .args(["/C", "start", "", &url])
                                        .creation_flags(0x0800_0000)
                                        .spawn();
                                }
                            });
                        }
                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(copy_value.clone()));
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
                    .h(px(LINK_PANEL_EXPANDER_HEIGHT))
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
                        // Grow the window to the expanded layout: every row
                        // plus the footer, no expander slot.
                        if let Some(handle) = this.window_handle {
                            let new_height = link_panel_height(this.links.len(), true);
                            let _ = handle.update(cx, |_, window, _| {
                                let mut bounds = window.window_bounds().get_bounds();
                                bounds.size.height = px(new_height);
                                window.resize(bounds.size);
                            });
                        }
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
                .child(div().text_size(px(10.0)).text_color(text_secondary).child(
                    if self.links.len() > 1 {
                        format!("{} 个链接", self.links.len())
                    } else {
                        "1 个链接".to_owned()
                    },
                ))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_height_matches_the_rendered_chrome_exactly() {
        // py(4)+py(4) + one 28px row + footer(2px margin + 22px).
        assert_eq!(link_panel_height(1, false), 60.0);
        // Collapsed multi-link: one row + the 24px expander, never the
        // hidden rows. The old `rows * 30 + 24` budget reserved space for
        // rows it did not render and left dead empty space below the footer.
        assert_eq!(link_panel_height(3, false), 84.0);
        // Expanded: every row, no expander slot.
        assert_eq!(link_panel_height(3, true), 116.0);
    }

    #[test]
    fn collapsed_height_never_depends_on_hidden_rows_and_expanding_grows() {
        for rows in 2..=12 {
            let collapsed = link_panel_height(rows, false);
            let expanded = link_panel_height(rows, true);
            assert_eq!(
                collapsed,
                link_panel_height(2, false),
                "a collapsed {rows}-link panel must reserve no space for its hidden rows"
            );
            assert!(
                expanded > collapsed,
                "expanding {rows} links must grow the window past the collapsed height"
            );
        }
    }
}
