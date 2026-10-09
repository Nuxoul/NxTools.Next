//! 链接识别 floating panel: one popup window with one click-to-copy row per
//! detected link. Auto-hidden by the owning clipboard application after the
//! dwell time; clicking a row copies the value and closes the panel.

use crate::clipboard::ClipboardApplication;
use crate::theme::Theme;
use crate::tools::link_detector::DetectedLinkKind;
use crate::ui::metrics::{RADIUS_CONTROL, RADIUS_FLOATING, SPACING_4, SPACING_8};
use crate::ui::typography::{TEXT_BODY, TEXT_LABEL, TEXT_META, UI_FONT_FAMILY};
use gpui::{
    AnyWindowHandle, Context, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, WeakEntity, Window, div, px,
};
use gpui_component::Icon;
use gpui_component::button::{Button, ButtonVariants};
use tracing::info;

pub(crate) type DetectedLinkRow = crate::tools::link_detector::DetectedLink;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LinkPanelRoute {
    CopyOnly,
    Browser,
    OpenShell {
        action: nxtools_protocol::ShellOpenAction,
    },
}

pub(crate) fn link_panel_route(
    kind: DetectedLinkKind,
    auto_open_browser: bool,
    copy_only: bool,
) -> LinkPanelRoute {
    if copy_only {
        return LinkPanelRoute::CopyOnly;
    }
    match kind {
        DetectedLinkKind::Web if auto_open_browser => LinkPanelRoute::Browser,
        DetectedLinkKind::Web => LinkPanelRoute::CopyOnly,
        DetectedLinkKind::LocalFile => LinkPanelRoute::OpenShell {
            action: nxtools_protocol::ShellOpenAction::SelectInExplorer,
        },
        DetectedLinkKind::LocalDirectory => LinkPanelRoute::OpenShell {
            action: nxtools_protocol::ShellOpenAction::Open,
        },
    }
}
pub(crate) fn link_panel_shell_command(
    route: LinkPanelRoute,
    target: &str,
    target_kind: DetectedLinkKind,
) -> Option<nxtools_protocol::PlatformCommand> {
    let LinkPanelRoute::OpenShell { action } = route else {
        return None;
    };
    if !matches!(
        target_kind,
        DetectedLinkKind::LocalFile | DetectedLinkKind::LocalDirectory
    ) {
        return None;
    }
    Some(nxtools_protocol::PlatformCommand::OpenShellTarget(
        nxtools_protocol::OpenShellTargetCommand {
            target: target.to_owned(),
            action,
        },
    ))
}

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
    pub(crate) auto_open_browser: bool,
    pub(crate) copy_only: bool,
}

impl LinkPanelView {
    /// Restages the reused panel window with a fresh capture's links and
    /// re-arms the window-scoped auto-hide.
    pub(crate) fn swap_links(
        &mut self,
        links: Vec<DetectedLinkRow>,
        window_handle: AnyWindowHandle,
        auto_open_browser: bool,
        copy_only: bool,
        cx: &mut Context<Self>,
    ) {
        self.links = links;
        self.expanded = false;
        self.auto_open_browser = auto_open_browser;
        self.copy_only = copy_only;
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::zed_dark();
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
            .font_family(UI_FONT_FAMILY)
            .bg(theme.surface)
            .border_1()
            .border_color(theme.border_subtle)
            .rounded(px(RADIUS_FLOATING))
            .overflow_hidden()
            .flex()
            .flex_col()
            .py(px(SPACING_4));
        for (index, link) in self.links.iter().enumerate().take(visible_count) {
            let application = application.clone();
            let copy_value = link.copy_value.clone();
            let kind = link.kind;
            let route = link_panel_route(kind, self.auto_open_browser, self.copy_only);
            let focus = window
                .use_keyed_state(("link-panel-focus", index), cx, |_, cx| {
                    cx.focus_handle().tab_stop(true)
                })
                .read(cx)
                .clone();
            container = container.child(
                div()
                    .id(("link-panel-row", index))
                    .track_focus(&focus)
                    .border_2()
                    .border_color(if focus.is_focused(window) {
                        theme.focus_ring
                    } else {
                        gpui::transparent_black().into()
                    })
                    .on_key_down(move |event, window, cx| {
                        if focus.is_focused(window)
                            && event.keystroke.key == "tab"
                            && !event.keystroke.modifiers.control
                            && !event.keystroke.modifiers.alt
                            && !event.keystroke.modifiers.platform
                        {
                            if event.keystroke.modifiers.shift {
                                window.focus_prev();
                            } else {
                                window.focus_next();
                            }
                            window.prevent_default();
                            cx.stop_propagation();
                        }
                    })
                    .debug_selector(move || format!("link-panel-row-{index}"))
                    .h(px(LINK_PANEL_ROW_HEIGHT))
                    .min_w_0()
                    .mx(px(SPACING_4))
                    .px(px(SPACING_8))
                    .rounded(px(RADIUS_CONTROL))
                    .flex()
                    .items_center()
                    .gap(px(SPACING_4))
                    .cursor_pointer()
                    .hover(move |style| style.bg(theme.surface_hover))
                    .active(move |style| style.bg(theme.surface_pressed))
                    .on_click(move |_event, window, cx| {
                        let route = route;
                        let copy_value = copy_value.clone();
                        let value_length = copy_value.len();
                        info!(
                            event = "tools.link_panel.row_clicked",
                            target_kind = kind.label(),
                            value_length,
                            route = ?route,
                            "link panel row clicked"
                        );
                        // Close FIRST. The panel's own clipboard echo must not
                        // be able to restage this window.
                        window.remove_window();
                        let application = application.clone();
                        cx.defer(move |cx| {
                            if let Some(application) = application.upgrade() {
                                application.update(cx, |application, _cx| {
                                    application.note_panel_copied(copy_value.clone());
                                    application.forget_link_panel();
                                    application.dispatch_link_panel_route(
                                        route,
                                        &copy_value,
                                        kind,
                                        value_length,
                                    );
                                });
                            }
                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(copy_value));
                        });
                    })
                    .child(
                        Icon::new(match kind {
                            DetectedLinkKind::Web => crate::icons::MaterialIcon::Language,
                            DetectedLinkKind::LocalFile | DetectedLinkKind::LocalDirectory => {
                                crate::icons::MaterialIcon::Folder
                            }
                        })
                        .size(px(16.0))
                        .text_color(theme.text_secondary),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .text_size(px(TEXT_BODY))
                            .text_color(theme.text)
                            .truncate()
                            .child(link.display.clone()),
                    ),
            );
        }
        if needs_expander {
            container = container.child(
                Button::new("link-panel-expander")
                    .ghost()
                    .debug_selector(|| "link-panel-expander".into())
                    .h(px(LINK_PANEL_EXPANDER_HEIGHT))
                    .mx(px(SPACING_4))
                    .px(px(SPACING_8))
                    .rounded(px(RADIUS_CONTROL))
                    .flex()
                    .items_center()
                    .gap(px(SPACING_4))
                    .text_size(px(TEXT_LABEL))
                    .text_color(theme.text_secondary)
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
                    .label(format!(
                        "展开其余 {} 个链接",
                        self.links.len() - visible_count
                    )),
            );
        }
        container = container.child(
            div()
                .debug_selector(|| "link-panel-footer".into())
                .h(px(LINK_PANEL_FOOTER_HEIGHT))
                .mt(px(LINK_PANEL_FOOTER_MARGIN_TOP))
                .mx(px(SPACING_4))
                .px(px(SPACING_8))
                .flex()
                .items_center()
                .justify_between()
                .border_t_1()
                .border_color(theme.border_subtle)
                .child(
                    div()
                        .text_size(px(TEXT_META))
                        .text_color(theme.text_secondary)
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
                        .gap(px(SPACING_4))
                        .child(
                            div()
                                .text_size(px(TEXT_META))
                                .text_color(theme.text_secondary)
                                .child(if self.copy_only {
                                    "点击复制"
                                } else if self.auto_open_browser {
                                    "网页可打开 · 本地可打开"
                                } else {
                                    "网页仅复制 · 本地可打开"
                                }),
                        )
                        .child(
                            div()
                                .text_size(px(TEXT_META))
                                .text_color(theme.text_muted)
                                .child("自动消失"),
                        ),
                ),
        );
        let _ = application;
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
    #[test]
    fn local_files_and_directories_use_shell_routes() {
        assert_eq!(
            link_panel_route(DetectedLinkKind::LocalFile, true, false),
            LinkPanelRoute::OpenShell {
                action: nxtools_protocol::ShellOpenAction::SelectInExplorer,
            }
        );
        assert_eq!(
            link_panel_route(DetectedLinkKind::LocalDirectory, true, false),
            LinkPanelRoute::OpenShell {
                action: nxtools_protocol::ShellOpenAction::Open,
            }
        );

        let file_target = r"C:\Users\Administrator\Throne.exe";
        assert_eq!(
            link_panel_shell_command(
                link_panel_route(DetectedLinkKind::LocalFile, true, false),
                file_target,
                DetectedLinkKind::LocalFile,
            ),
            Some(nxtools_protocol::PlatformCommand::OpenShellTarget(
                nxtools_protocol::OpenShellTargetCommand {
                    target: file_target.to_owned(),
                    action: nxtools_protocol::ShellOpenAction::SelectInExplorer,
                }
            )),
            "an existing .exe path is passed unchanged to Explorer selection"
        );
        let directory_target = r"C:\Users\Administrator\Downloads";
        assert_eq!(
            link_panel_shell_command(
                link_panel_route(DetectedLinkKind::LocalDirectory, true, false),
                directory_target,
                DetectedLinkKind::LocalDirectory,
            ),
            Some(nxtools_protocol::PlatformCommand::OpenShellTarget(
                nxtools_protocol::OpenShellTargetCommand {
                    target: directory_target.to_owned(),
                    action: nxtools_protocol::ShellOpenAction::Open,
                }
            ))
        );
        assert!(
            link_panel_shell_command(
                link_panel_route(DetectedLinkKind::LocalFile, true, false),
                "https://example.test/app.exe",
                DetectedLinkKind::Web,
            )
            .is_none()
        );
        assert!(!file_target.starts_with("https://"));
    }

    #[test]
    fn web_targets_keep_browser_behavior_but_copy_only_never_launches() {
        assert_eq!(
            link_panel_route(DetectedLinkKind::Web, true, false),
            LinkPanelRoute::Browser
        );
        assert_eq!(
            link_panel_route(DetectedLinkKind::Web, false, false),
            LinkPanelRoute::CopyOnly
        );
        for kind in [
            DetectedLinkKind::Web,
            DetectedLinkKind::LocalFile,
            DetectedLinkKind::LocalDirectory,
        ] {
            assert_eq!(
                link_panel_route(kind, true, true),
                LinkPanelRoute::CopyOnly,
                "copy-only mode must not launch any target kind"
            );
        }
    }
}
