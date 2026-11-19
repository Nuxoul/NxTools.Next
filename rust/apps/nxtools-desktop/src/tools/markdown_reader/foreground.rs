//! Foreground completion for an explicit Explorer open, separate from file admission.
use super::*;
use nxtools_protocol::ActivateWindowCommand;

impl Reader {
    pub(super) fn request_foreground(&mut self, id: Uuid, window: &mut Window) {
        if let Some((previous, started)) = self.foreground.take() {
            tracing::info!(event = "markdown.foreground.finished", correlation_id = %previous,
                outcome = "superseded", elapsed_ms = started.elapsed().as_millis());
        }
        let started = Instant::now();
        tracing::info!(event = "markdown.foreground.started", correlation_id = %id);
        if let Some((platform, window_handle)) = self
            .platform
            .as_ref()
            .zip(crate::ui::native_window_handle(window))
            && platform
                .send_command_with_id(
                    PlatformCommand::ActivateWindow(ActivateWindowCommand { window_handle }),
                    id,
                )
                .is_ok()
        {
            self.foreground = Some((id, started));
        } else {
            window.activate_window();
            tracing::info!(event = "markdown.foreground.finished", correlation_id = %id,
                outcome = "requested_unverified", elapsed_ms = started.elapsed().as_millis());
        }
    }

    pub(super) fn foreground_receipt(&mut self, event: &PlatformNotification) {
        let Some((id, started)) = self.foreground else {
            return;
        };
        let (outcome, native_error) = match event {
            PlatformNotification::CommandResult {
                correlation_id,
                result,
            } if *correlation_id == id => (
                if result.success {
                    "activated"
                } else {
                    "not_foreground"
                },
                result.native_error_code,
            ),
            PlatformNotification::Disconnected {
                pending_command_ids,
                ..
            } if pending_command_ids.contains(&id) => ("disconnected", None),
            _ => return,
        };
        self.foreground = None;
        tracing::info!(event = "markdown.foreground.finished", correlation_id = %id,
            outcome, ?native_error, elapsed_ms = started.elapsed().as_millis());
    }
}
