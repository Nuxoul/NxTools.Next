//! One header icon; runtime status remains a snapshot of the native capture owner.
use super::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::{Disableable as _, label::Label};

impl ClipboardPanel {
    #[cfg(test)]
    pub(crate) fn capture_pause_test_snapshot(&self) -> crate::clipboard::CapturePauseSnapshot {
        self.capture_pause.clone()
    }
    pub(crate) fn with_capture_pause(
        mut self,
        snapshot: crate::clipboard::CapturePauseSnapshot,
    ) -> Self {
        self.capture_pause = snapshot;
        self
    }
    pub(crate) fn update_capture_pause(
        &mut self,
        snapshot: crate::clipboard::CapturePauseSnapshot,
        cx: &mut Context<Self>,
    ) {
        self.capture_pause = snapshot;
        cx.notify();
    }
    fn request_capture_pause(&mut self, paused: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.capture_pause.pending {
            return;
        }
        self.dismiss_preview_for_input("capture_toggle", cx);
        // Block a second click immediately; never claim native application before its ACK.
        self.capture_pause.requested = paused;
        self.capture_pause.pending = true;
        self.capture_pause.failed = false;
        self.capture_pause.timed = false;
        self.focus_search_input(window, cx);
        schedule_entity_update(
            self.application.clone(),
            move |owner, cx| owner.set_capture_pause(paused, false, cx),
            cx,
        );
        cx.notify();
    }
    pub(super) fn render_capture_controls(
        &self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let snapshot = &self.capture_pause;
        let paused = snapshot.requested || snapshot.applied == Some(true);
        let label = if snapshot.failed {
            if snapshot.requested {
                "重试暂停记录"
            } else {
                "重试继续记录"
            }
        } else if paused {
            "继续记录"
        } else {
            "暂停记录"
        };
        let requested = if snapshot.failed {
            snapshot.requested
        } else {
            !paused
        };
        let panel = cx.entity().downgrade();
        div()
            .id("clipboard-capture-control-slot")
            .flex_none()
            .on_mouse_down(gpui_kit::MouseButton::Left, |_, _, cx| {
                cx.stop_propagation()
            })
            .child(
                Button::new("clipboard-capture-pause")
                    .debug_selector(|| "clipboard-capture-pause".into())
                    .small()
                    .ghost()
                    .icon(if paused {
                        IconName::Play
                    } else {
                        IconName::Pause
                    })
                    .accessibility_label(label)
                    .tooltip(format!("{} · {label}", snapshot.label()))
                    .disabled(snapshot.pending)
                    .on_click(move |_, window, cx| {
                        cx.stop_propagation();
                        let _ = panel.update(cx, |panel, cx| {
                            panel.request_capture_pause(requested, window, cx)
                        });
                    }),
            )
    }
    pub(super) fn render_capture_failure(&self) -> gpui_kit::Div {
        div().when(self.capture_pause.failed, |row| {
            row.debug_selector(|| "clipboard-capture-failure".into())
                .mb_2()
                .child(
                    Label::new(format!(
                        "{}，点击右上角图标重试",
                        self.capture_pause.label()
                    ))
                    .text_sm(),
                )
        })
    }
}
