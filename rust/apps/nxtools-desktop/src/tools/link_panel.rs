//! 链接识别 floating panel: capture identifies targets; rows open them and
//! a separate Copy link/path action writes the clipboard. The owning window
//! closes before either action is deferred to the clipboard application.

use crate::clipboard::ClipboardApplication;
use crate::theme::Theme;
use crate::tools::link_detector::DetectedLinkKind;
use crate::ui::metrics::{RADIUS_FLOATING, SPACING_4};
use gpui_kit::component::Icon;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::label::Label;
use gpui_kit::component::list::ListItem;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::status_bar::StatusBar;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    AnyWindowHandle, Context, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, WeakEntity, Window, div, px,
};
use tracing::info;

pub(crate) type DetectedLinkRow = crate::tools::link_detector::DetectedLink;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LinkPanelRoute {
    Copy,
    Browser,
    OpenShell {
        action: nxtools_protocol::ShellOpenAction,
    },
}

pub(crate) fn link_panel_route(kind: DetectedLinkKind) -> LinkPanelRoute {
    match kind {
        DetectedLinkKind::Web => LinkPanelRoute::Browser,
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
    let (target, action) = match (route, target_kind) {
        (LinkPanelRoute::Copy, _) => return None,
        (LinkPanelRoute::Browser, DetectedLinkKind::Web) => (
            if target.contains("://") {
                target.to_owned()
            } else {
                format!("https://{target}")
            },
            nxtools_protocol::ShellOpenAction::Open,
        ),
        (
            LinkPanelRoute::OpenShell { action },
            DetectedLinkKind::LocalFile | DetectedLinkKind::LocalDirectory,
        ) => (target.to_owned(), action),
        _ => return None,
    };
    Some(nxtools_protocol::PlatformCommand::OpenShellTarget(
        nxtools_protocol::OpenShellTargetCommand { target, action },
    ))
}

fn kind_is_download_target(kind: DetectedLinkKind, target: &str) -> bool {
    kind == DetectedLinkKind::Web
        && crate::downloads::DownloadApplication::admits_link_download(target)
}

pub(crate) const LINK_PANEL_WIDTH: f32 = 420.0;
/// Rendered row height (`.h(px(28.0))` on each link row below).
const LINK_PANEL_ROW_HEIGHT: f32 = 28.0;
/// Rendered expander height (`.h(px(24.0))` on the expander row below).
const LINK_PANEL_EXPANDER_HEIGHT: f32 = 24.0;
/// Released StatusBar text and vertical padding require a 29px rail.
const LINK_PANEL_FOOTER_HEIGHT: f32 = 29.0;
/// Footer's `.mt(px(2.0))`.
const LINK_PANEL_FOOTER_MARGIN_TOP: f32 = 2.0;
/// Container's `.py(px(4.0))` top and bottom.
const LINK_PANEL_VERTICAL_PADDING: f32 = 8.0;

/// Natural client height: visible rows, optional expander, count footer,
/// vertical padding and the one-pixel top/bottom borders.
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
    2.0 + LINK_PANEL_VERTICAL_PADDING
        + visible_rows as f32 * LINK_PANEL_ROW_HEIGHT
        + expander
        + LINK_PANEL_FOOTER_MARGIN_TOP
        + LINK_PANEL_FOOTER_HEIGHT
}

#[derive(Default)]
pub(crate) struct LinkPanelSession {
    pub(crate) window: Option<AnyWindowHandle>,
    pub(crate) native_window_handle: i64,
    pub(crate) monitor: Option<nxtools_protocol::CursorMonitorSnapshot>,
    #[cfg(test)]
    pub(crate) test_native_window_handle: Option<i64>,
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
    pub(crate) auto_hide_task: Option<gpui_kit::Task<()>>,
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
        // Dropping the previous task retires its capture before arming the next
        // dwell; an earlier timer must never close a restaged generation.
        self.auto_hide_task = Some(cx.spawn(async move |_, cx| {
            cx.background_executor().timer(dwell).await;
            let _ = window_handle.update(cx, |_, window, _| window.remove_window());
        }));
    }
}

impl Render for LinkPanelView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::theme::observe_theme(window, cx);
        let theme = Theme::active(cx);
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
            .bg(theme.surface)
            .border_1()
            .border_color(theme.border_subtle)
            .rounded(px(RADIUS_FLOATING))
            .overflow_hidden()
            .flex()
            .flex_col()
            .py(px(SPACING_4));
        let mut rows = div().w_full().flex().flex_col();
        for (index, link) in self.links.iter().enumerate().take(visible_count) {
            let application = application.clone();
            let copy_application = application.clone();
            let download_application = application.clone();
            let explicit_download_value = link.copy_value.clone();
            let download_enabled = kind_is_download_target(link.kind, &link.copy_value);
            let explicit_copy_value = link.copy_value.clone();
            let copy_value = link.copy_value.clone();
            let kind = link.kind;
            let tooltip_value = link.copy_value.clone();
            let route = link_panel_route(kind);
            let panel_window = window.window_handle();
            let focus = window
                .use_keyed_state(("link-panel-focus", index), cx, |_, cx| {
                    cx.focus_handle().tab_stop(true)
                })
                .read(cx)
                .clone();
            rows = rows.child(
                ListItem::new(("link-panel-row", index))
                    .accessibility_label(format!("链接 {}", index + 1))
                    .track_focus(&focus)
                    .focus_visible(move |style| style.bg(theme.element_selected))
                    .on_key_down(move |event, window, cx| {
                        if focus.is_focused(window)
                            && event.keystroke.key == "tab"
                            && !event.keystroke.modifiers.control
                            && !event.keystroke.modifiers.alt
                            && !event.keystroke.modifiers.platform
                        {
                            if event.keystroke.modifiers.shift {
                                window.focus_prev(cx);
                            } else {
                                window.focus_next(cx);
                            }
                            window.prevent_default();
                            cx.stop_propagation();
                        }
                    })
                    .debug_selector(move || format!("link-panel-row-{index}"))
                    .h(px(LINK_PANEL_ROW_HEIGHT))
                    .flex_none()
                    .min_w_0()
                    .mx(px(SPACING_4))
                    .tooltip(move |window, cx| Tooltip::new(tooltip_value.clone()).build(window, cx))
                    .child(
                        div()
                            .w_full()
                            .child(
                                div()
                                    .w_full()
                                    .min_w_0()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        div()
                                            .debug_selector(move || {
                                                format!("link-panel-icon-{index}")
                                            })
                                            .size(px(16.0))
                                            .flex_none()
                                            .child(
                                                Icon::new(match kind {
                                                    DetectedLinkKind::Web => {
                                                        crate::icons::MaterialIcon::Language
                                                    }
                                                    DetectedLinkKind::LocalFile
                                                    | DetectedLinkKind::LocalDirectory => {
                                                        crate::icons::MaterialIcon::Folder
                                                    }
                                                })
                                                .size(px(16.0))
                                                .text_color(theme.text_secondary),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .debug_selector(move || {
                                                format!("link-panel-name-{index}")
                                            })
                                            .min_w_0()
                                            .flex_1()
                                            .child(Label::new(link.display.clone()).truncate()),
                                    )
                                    .child(
                                        Button::new(("link-panel-copy", index))
                                            .debug_selector(move || format!("link-panel-copy-{index}"))
                                            .xsmall()
                                            .ghost()
                                            .flex_none()
                                            .label(if kind == DetectedLinkKind::Web { "复制链接" } else { "复制路径" })
                                            .accessibility_label(if kind == DetectedLinkKind::Web { "复制链接" } else { "复制路径" })
                                            .on_click(move |_, window, cx| {
                                                cx.stop_propagation();
                                                let value = explicit_copy_value.clone();
                                                let application = copy_application.clone();
                                                let correlation_id = uuid::Uuid::new_v4();
                                                let started_at = std::time::Instant::now();
                                                info!(event = "tools.link_panel.copy_started", %correlation_id,
                                                    target_kind = kind.label(), value_length = value.len(),
                                                    "explicit link panel copy accepted");
                                                window.remove_window();
                                                cx.defer(move |cx| {
                                                    let current = application.upgrade().is_none_or(|application| {
                                                        application.update(cx, |application, _| {
                                                            if application.forget_link_panel_if_current(panel_window) {
                                                                application.note_panel_copied(value.clone());
                                                                true
                                                            } else {
                                                                false
                                                            }
                                                        })
                                                    });
                                                    if current {
                                                        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(value));
                                                        info!(event = "tools.link_panel.copy_completed", %correlation_id,
                                                            elapsed_ms = started_at.elapsed().as_millis() as u64,
                                                            "explicit link panel copy completed");
                                                    } else {
                                                        info!(event = "tools.link_panel.copy_cancelled", %correlation_id,
                                                            reason = "panel_replaced",
                                                            elapsed_ms = started_at.elapsed().as_millis() as u64,
                                                            "explicit link panel copy cancelled");
                                                    }
                                                });
                                            }),
                                    )
                                    .when(download_enabled, |row| row.child(
                                        Button::new(("link-panel-download", index))
                                            .debug_selector(move || format!("link-panel-download-{index}"))
                                            .xsmall()
                                            .ghost()
                                            .flex_none()
                                            .label("使用 NxTools 下载")
                                            .accessibility_label("使用 NxTools 下载")
                                            .on_click(move |_, window, cx| {
                                                cx.stop_propagation();
                                                let value = explicit_download_value.clone();
                                                let application = download_application.clone();
                                                let correlation_id = uuid::Uuid::new_v4();
                                                let started_at = std::time::Instant::now();
                                                info!(event = "tools.link_panel.download_started", %correlation_id,
                                                    "explicit link download intake accepted");
                                                window.remove_window();
                                                cx.defer(move |cx| {
                                                    let downloads = application.upgrade().and_then(|application| {
                                                        application.update(cx, |application, _| {
                                                            application.take_link_panel_download_owner(panel_window)
                                                        })
                                                    }).and_then(|owner| owner.upgrade());
                                                    if let Some(downloads) = downloads {
                                                        downloads.update(cx, |downloads, cx| {
                                                            downloads.open_link_download(value, correlation_id, started_at, cx);
                                                        });
                                                    } else {
                                                        info!(event = "tools.link_panel.download_finished", %correlation_id,
                                                            outcome = "source_or_owner_retired",
                                                            elapsed_ms = started_at.elapsed().as_millis() as u64,
                                                            "explicit link download intake retired");
                                                    }
                                                });
                                            })
                                    )),
                            ),
                    )
                    .on_click(move |_event, window, cx| {
                        let copy_value = copy_value.clone();
                        let value_length = copy_value.len();
                        info!(
                            event = "tools.link_panel.row_clicked",
                            target_kind = kind.label(),
                            value_length,
                            "link panel row clicked"
                        );
                        window.remove_window();
                        let application = application.clone();
                        cx.defer(move |cx| {
                            if let Some(application) = application.upgrade() {
                                application.update(cx, |application, cx| {
                                    if application.forget_link_panel_if_current(panel_window) {
                                        application.dispatch_link_panel_route(
                                            route, &copy_value, kind, value_length, cx,
                                        );
                                    }
                                });
                            }
                        });
                    }),
            );
        }
        container = container.child(
            div()
                .debug_selector(|| "link-panel-list".into())
                .flex_1()
                .min_h_0()
                .child(rows.overflow_y_scrollbar().id("link-panel-scroll")),
        );
        if needs_expander {
            container = container.child(
                Button::new("link-panel-expander")
                    .ghost()
                    .debug_selector(|| "link-panel-expander".into())
                    .small()
                    .flex_none()
                    .mx(px(SPACING_4))
                    .on_click(cx.listener(move |this, _event, window, cx| {
                        this.expanded = true;
                        let source = this.window_handle.unwrap_or_else(|| window.window_handle());
                        let total_rows = this.links.len();
                        let application = this.application.clone();
                        cx.defer(move |cx| {
                            if let Some(application) = application.upgrade() {
                                application.update(cx, |application, cx| {
                                    application
                                        .request_link_panel_expansion(source, total_rows, cx);
                                });
                            }
                        });
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
                .flex_none()
                .mt(px(LINK_PANEL_FOOTER_MARGIN_TOP))
                .mx(px(SPACING_4))
                .child(
                    StatusBar::new()
                        .h_full()
                        .flex_none()
                        .left(Label::new(format!("{} 个链接", self.links.len()))),
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
        // Two borders + py(4)+py(4) + one 28px row + footer(2px margin + 29px).
        assert_eq!(link_panel_height(1, false), 69.0);
        // Collapsed multi-link: one row plus the 24px expander.
        assert_eq!(link_panel_height(3, false), 93.0);
        // Expanded: every row, no expander slot.
        assert_eq!(link_panel_height(3, true), 125.0);
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
            link_panel_route(DetectedLinkKind::LocalFile),
            LinkPanelRoute::OpenShell {
                action: nxtools_protocol::ShellOpenAction::SelectInExplorer,
            }
        );
        assert_eq!(
            link_panel_route(DetectedLinkKind::LocalDirectory),
            LinkPanelRoute::OpenShell {
                action: nxtools_protocol::ShellOpenAction::Open,
            }
        );

        let file_target = r"C:\Users\Administrator\Throne.exe";
        assert_eq!(
            link_panel_shell_command(
                link_panel_route(DetectedLinkKind::LocalFile),
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
                link_panel_route(DetectedLinkKind::LocalDirectory),
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
                link_panel_route(DetectedLinkKind::LocalFile),
                "https://example.test/app.exe",
                DetectedLinkKind::Web,
            )
            .is_none()
        );
        assert!(!file_target.starts_with("https://"));
    }

    #[test]
    fn web_targets_use_browser_route_without_a_runtime_gate() {
        assert_eq!(
            link_panel_route(DetectedLinkKind::Web),
            LinkPanelRoute::Browser
        );
        assert_eq!(
            link_panel_shell_command(
                LinkPanelRoute::Browser,
                "example.invalid",
                DetectedLinkKind::Web
            ),
            Some(nxtools_protocol::PlatformCommand::OpenShellTarget(
                nxtools_protocol::OpenShellTargetCommand {
                    target: "https://example.invalid".into(),
                    action: nxtools_protocol::ShellOpenAction::Open,
                }
            ))
        );
        assert!(
            link_panel_shell_command(
                LinkPanelRoute::Browser,
                r"C:\synthetic",
                DetectedLinkKind::LocalFile
            )
            .is_none()
        );
    }
}
