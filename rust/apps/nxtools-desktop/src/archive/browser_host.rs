//! One native browser host; original browser entities keep each source's state and leases.
use super::{ArchiveApplication, ArchiveBrowser};
use gpui_kit::component::{
    Sizable as _, Size,
    button::{Button, ButtonVariants as _},
    tab::{Tab, TabBar},
};
use gpui_kit::{
    Context, Entity, IntoElement, MouseButton, Render, ScrollHandle, WeakEntity, Window, div,
    prelude::*, px,
};
use uuid::Uuid;

#[cfg(test)]
#[path = "browser_icon_layout_tests.rs"]
mod icon_layout_tests;

pub(super) struct BrowserHost {
    pub(super) id: Uuid,
    owner: WeakEntity<ArchiveApplication>,
    pub(super) tabs: Vec<(Uuid, Entity<ArchiveBrowser>)>,
    pub(super) active: Option<Uuid>,
    scroll: ScrollHandle,
}

impl BrowserHost {
    pub(super) fn new(id: Uuid, owner: WeakEntity<ArchiveApplication>) -> Self {
        Self {
            id,
            owner,
            tabs: vec![],
            active: None,
            scroll: ScrollHandle::new(),
        }
    }

    pub(super) fn add(
        &mut self,
        id: Uuid,
        view: Entity<ArchiveBrowser>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.tabs.push((id, view));
        self.activate(id, window, cx);
    }

    pub(super) fn activate(
        &mut self,
        id: Uuid,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(index) = self.tabs.iter().position(|(session, _)| *session == id) else {
            return false;
        };
        if self.active != Some(id) {
            if let Some((_, previous)) = self
                .tabs
                .iter()
                .find(|(session, _)| Some(*session) == self.active)
            {
                let previous = previous.downgrade();
                cx.defer(move |cx| {
                    let _ = previous.update(cx, |view, cx| view.suspend_tab(cx));
                });
            }
            self.active = Some(id);
            tracing::info!(event="archive.browser.tab_activated", correlation_id=%id, host_id=%self.id, "archive tab activated");
        }
        let title = self.tabs[index].1.read(cx).tab_title();
        window.set_window_title(&title);
        self.scroll.scroll_to_item(index);
        let owner = self.owner.clone();
        cx.defer(move |cx| {
            let _ = owner.update(cx, |app, cx| app.tab_activated(id, cx));
        });
        cx.notify();
        true
    }

    pub(super) fn remove(&mut self, id: Uuid, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.tabs.iter().position(|(session, _)| *session == id) else {
            return;
        };
        self.tabs.remove(index);
        if self.active == Some(id) {
            self.active = None;
            if let Some((next, _)) = self.tabs.get(index.min(self.tabs.len().saturating_sub(1))) {
                self.activate(*next, window, cx);
            }
        }
        cx.notify();
    }
}

impl Render for BrowserHost {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self
            .tabs
            .iter()
            .position(|(id, _)| Some(*id) == self.active)
            .unwrap_or(0);
        let ids = self.tabs.iter().map(|(id, _)| *id).collect::<Vec<_>>();
        let tabs = self
            .tabs
            .iter()
            .map(|(id, view)| {
                let id = *id;
                let label = view
                    .read(cx)
                    .path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                let owner = self.owner.clone();
                Tab::new()
                    .label(label.clone())
                    .aria_label(label)
                    .debug_selector(move || format!("archive-tab-{id}"))
                    .when(cfg!(test), |tab| {
                        // Zero-width marker in the public content slot observes the official
                        // label's trailing boundary without replacing its render/ellipsis.
                        tab.child(
                            div()
                                .debug_selector(move || format!("archive-tab-label-end-{id}"))
                                .w_0()
                                .h(px(16.))
                                .flex_none(),
                        )
                    })
                    .prefix(
                        // Released Medium Tab pads its label by12DIP inside the prefix/suffix.
                        // Move8DIP of that padding into each public slot: one8DIP content gap,
                        // with12DIP outer insets instead of artwork/close touching the edges.
                        div()
                            .flex_none()
                            .ml(px(12.))
                            .mr(px(-8.))
                            .size(px(16.))
                            .child(
                                gpui_kit::img("brand/archive.png")
                                    .debug_selector(move || format!("archive-tab-icon-{id}"))
                                    .size_full()
                                    .flex_none()
                                    .object_fit(gpui_kit::ObjectFit::Contain),
                            ),
                    )
                    .suffix(
                        div().flex_none().ml(px(-8.)).mr(px(12.)).child(
                            Button::new(gpui_kit::SharedString::from(format!(
                                "archive-tab-close-{id}"
                            )))
                            .debug_selector(move || format!("archive-tab-close-{id}"))
                            .icon(gpui_kit::component::IconName::Close)
                            .accessibility_label("关闭归档标签页")
                            .ghost()
                            .with_size(Size::XSmall)
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .on_click(move |_, _, cx| {
                                cx.stop_propagation();
                                let owner = owner.clone();
                                cx.defer(move |cx| {
                                    let _ = owner.update(cx, |app, cx| app.close_browser(id, cx));
                                });
                            }),
                        ),
                    )
            })
            .collect::<Vec<_>>();
        let active = self
            .tabs
            .iter()
            .find(|(id, _)| Some(*id) == self.active)
            .map(|(_, view)| view.clone());
        let title = active
            .as_ref()
            .map(|view| view.read(cx).tab_title())
            .unwrap_or_else(|| "压缩包 — NxTools".into());
        let host_id = self.id;
        let close_owner = self.owner.clone();
        div()
            .id("archive-browser-host")
            .size_full()
            .flex()
            .flex_col()
            .min_h_0()
            .min_w_0()
            .child(
                crate::ui::components::titlebar::prefixed_controlled_titlebar(
                    "archive-browser-titlebar",
                    "archive-browser-window-close",
                    title,
                    crate::ui::components::titlebar::CaptionCapabilities::RESIZABLE,
                    div()
                        .debug_selector(|| "archive-caption-app-icon".into())
                        .size(px(20.))
                        .flex_none()
                        .child(
                            gpui_kit::img("brand/nxtools-40.png")
                                .size_full()
                                .flex_none()
                                .object_fit(gpui_kit::ObjectFit::Contain),
                        ),
                    move |_, _, cx| {
                        let owner = close_owner.clone();
                        cx.defer(move |cx| {
                            let _ = owner.update(cx, |app, cx| app.close_host(host_id, cx));
                        });
                    },
                ),
            )
            .child(
                TabBar::new("archive-browser-tabs")
                    .with_size(Size::Medium)
                    .flex_none()
                    .min_w_0()
                    .max_width(px(220.))
                    .menu(true)
                    .track_scroll(&self.scroll)
                    .selected_index(selected)
                    .children(tabs)
                    .on_click(cx.listener(move |this, index: &usize, window, cx| {
                        if let Some(id) = ids.get(*index) {
                            this.activate(*id, window, cx);
                        }
                    })),
            )
            .when_some(active, |this, view| {
                this.child(div().flex_1().min_h_0().min_w_0().child(view))
            })
    }
}
