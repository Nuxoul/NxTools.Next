pub(crate) struct PopupLifecycle<T> {
    current: Option<T>,
    retained: Option<T>,
    pending_topmost: Option<uuid::Uuid>,
}

pub(crate) enum TopmostResolution<T> {
    Confirmed { previous: Option<T> },
    Rejected { previous: Option<T> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PopupCloseReason {
    Foreground {
        window_handle: i64,
        process_id: u32,
        process_name: String,
    },
    Pointer {
        window_handle: i64,
        root_window_handle: i64,
        process_id: u32,
    },
    PasteSuccess {
        command_id: uuid::Uuid,
    },
    Escape,
    /// Reserved for a future explicit close command; current UI commands use Escape or action reasons.
    #[allow(dead_code)]
    Explicit,
}

impl PopupCloseReason {
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Self::Foreground { .. } => "foreground",
            Self::Pointer { .. } => "pointer",
            Self::PasteSuccess { .. } => "paste_success",
            Self::Escape => "escape",
            Self::Explicit => "explicit",
        }
    }
}

impl<T> Default for PopupLifecycle<T> {
    fn default() -> Self {
        Self {
            current: None,
            retained: None,
            pending_topmost: None,
        }
    }
}

impl<T> PopupLifecycle<T> {
    pub(crate) fn current(&self) -> Option<&T> {
        self.current.as_ref()
    }

    pub(crate) fn replacement_pending(&self) -> bool {
        self.pending_topmost.is_some()
    }

    pub(crate) fn stage(&mut self, popup: T, topmost_command: uuid::Uuid) {
        debug_assert!(self.retained.is_none());
        self.retained = self.current.take();
        self.current = Some(popup);
        self.pending_topmost = Some(topmost_command);
    }

    pub(crate) fn resolve_topmost(
        &mut self,
        correlation_id: uuid::Uuid,
        success: bool,
    ) -> Option<TopmostResolution<T>> {
        if self.pending_topmost != Some(correlation_id) {
            return None;
        }
        self.pending_topmost = None;
        if success {
            return Some(TopmostResolution::Confirmed {
                previous: self.retained.take(),
            });
        }
        Some(TopmostResolution::Rejected {
            previous: self.retained.take(),
        })
    }

    pub(crate) fn reset_pending(&mut self) -> Option<T> {
        self.pending_topmost = None;
        self.retained.take()
    }

    pub(crate) fn replace_pending_topmost(&mut self, command_id: uuid::Uuid) {
        if self.current.is_some() {
            self.pending_topmost = Some(command_id);
        }
    }
    pub(crate) fn replace_without_topmost_confirmation(&mut self, popup: T) -> Option<T> {
        debug_assert!(self.retained.is_none());
        self.current.replace(popup)
    }

    pub(crate) fn take_current(&mut self) -> Option<T> {
        self.pending_topmost = None;
        self.current.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn previous_popup_is_retained_until_replacement_is_confirmed_topmost() {
        let first_command = uuid::Uuid::new_v4();
        let second_command = uuid::Uuid::new_v4();
        let mut lifecycle = PopupLifecycle::default();

        lifecycle.stage(1_u8, first_command);
        assert!(matches!(
            lifecycle.resolve_topmost(first_command, true),
            Some(TopmostResolution::Confirmed { previous: None })
        ));

        lifecycle.stage(2_u8, second_command);
        assert_eq!(lifecycle.current(), Some(&2));
        assert!(lifecycle.replacement_pending());
    }

    #[test]
    fn rejected_topmost_keeps_new_popup_and_only_releases_previous_popup() {
        let first_command = uuid::Uuid::new_v4();
        let second_command = uuid::Uuid::new_v4();
        let mut lifecycle = PopupLifecycle::default();
        lifecycle.stage(1_u8, first_command);
        let _ = lifecycle.resolve_topmost(first_command, true);
        lifecycle.stage(2_u8, second_command);

        let resolution = lifecycle.resolve_topmost(second_command, false);

        assert!(matches!(
            resolution,
            Some(TopmostResolution::Rejected { previous: Some(1) })
        ));
        assert_eq!(lifecycle.current(), Some(&2));
    }

    #[test]
    fn confirmed_replacement_releases_previous_popup() {
        let first_command = uuid::Uuid::new_v4();
        let second_command = uuid::Uuid::new_v4();
        let mut lifecycle = PopupLifecycle::default();
        lifecycle.stage(1_u8, first_command);
        let _ = lifecycle.resolve_topmost(first_command, true);
        lifecycle.stage(2_u8, second_command);

        let resolution = lifecycle.resolve_topmost(second_command, true);

        assert!(matches!(
            resolution,
            Some(TopmostResolution::Confirmed { previous: Some(1) })
        ));
        assert_eq!(lifecycle.current(), Some(&2));
    }

    #[test]
    fn popup_remains_current_when_topmost_request_cannot_be_created() {
        let mut lifecycle = PopupLifecycle::default();

        let previous = lifecycle.replace_without_topmost_confirmation(1_u8);

        assert_eq!(previous, None);
        assert_eq!(lifecycle.current(), Some(&1));
    }

    #[test]
    fn reconnect_clears_old_topmost_and_retained_popup_state() {
        let first_command = uuid::Uuid::new_v4();
        let second_command = uuid::Uuid::new_v4();
        let mut lifecycle = PopupLifecycle::default();
        lifecycle.stage(1_u8, first_command);
        let _ = lifecycle.resolve_topmost(first_command, true);
        lifecycle.stage(2_u8, second_command);

        assert_eq!(lifecycle.reset_pending(), Some(1));
        assert!(!lifecycle.replacement_pending());
        lifecycle.replace_pending_topmost(first_command);
        assert!(lifecycle.replacement_pending());
    }
    #[test]
    fn unconfirmed_replacement_keeps_new_popup_and_releases_previous_popup() {
        let mut lifecycle = PopupLifecycle::default();
        let _ = lifecycle.replace_without_topmost_confirmation(1_u8);

        let previous = lifecycle.replace_without_topmost_confirmation(2_u8);

        assert_eq!(previous, Some(1));
        assert_eq!(lifecycle.current(), Some(&2));
    }
    #[test]
    fn popup_close_reasons_have_stable_diagnostic_kinds() {
        assert_eq!(PopupCloseReason::Escape.kind(), "escape");
        assert_eq!(PopupCloseReason::Explicit.kind(), "explicit");
        assert_eq!(
            PopupCloseReason::Foreground {
                window_handle: 10,
                process_id: 20,
                process_name: "explorer".into(),
            }
            .kind(),
            "foreground"
        );
        assert_eq!(
            PopupCloseReason::Pointer {
                window_handle: 11,
                root_window_handle: 12,
                process_id: 21,
            }
            .kind(),
            "pointer"
        );
        assert_eq!(
            PopupCloseReason::PasteSuccess {
                command_id: uuid::Uuid::nil(),
            }
            .kind(),
            "paste_success"
        );
    }
}
