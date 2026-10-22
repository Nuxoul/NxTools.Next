use super::*;

impl MainWindow {
    pub(crate) fn render_notifications(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        crate::notifications::presentation::render(
            self.notifications.clone(),
            true,
            TITLEBAR_HEIGHT + 48.0,
            window,
            cx,
        )
    }
}
