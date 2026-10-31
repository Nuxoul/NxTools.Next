use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

use nxtools_download::{DownloadStatus, DownloadTaskId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DownloadWindowPhase {
    Progress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DownloadWindowDirective {
    None,
    Open(DownloadWindowPhase),
    OpenAndFocus(DownloadWindowPhase),
    FocusExisting(DownloadWindowPhase),
    RefreshExisting,
    TransformExisting(DownloadWindowPhase),
    CloseExisting,
}

const MAX_BACKGROUND_PROGRESS_WINDOWS: usize = 3;

#[derive(Default)]
pub(crate) struct DownloadWindowLifecycle {
    open: HashMap<DownloadTaskId, DownloadWindowPhase>,
    handled_terminal: HashSet<(DownloadTaskId, DownloadStatus)>,
    background_open: HashSet<DownloadTaskId>,
}

impl DownloadWindowLifecycle {
    pub(crate) fn restore_existing(&mut self, task_id: DownloadTaskId, status: DownloadStatus) {
        if matches!(status, DownloadStatus::Completed | DownloadStatus::Failed) {
            self.handled_terminal.insert((task_id, status));
        }
    }

    pub(crate) fn request_progress(&mut self, task_id: DownloadTaskId) -> DownloadWindowDirective {
        self.request_window(task_id, DownloadWindowPhase::Progress)
    }

    pub(crate) fn request_background_progress(
        &mut self,
        task_id: DownloadTaskId,
    ) -> DownloadWindowDirective {
        if let Some(phase) = self.open.get_mut(&task_id) {
            if *phase != DownloadWindowPhase::Progress {
                *phase = DownloadWindowPhase::Progress;
                return DownloadWindowDirective::TransformExisting(DownloadWindowPhase::Progress);
            }
            return DownloadWindowDirective::RefreshExisting;
        }
        if self.background_open.len() >= MAX_BACKGROUND_PROGRESS_WINDOWS {
            return DownloadWindowDirective::None;
        }
        self.open.insert(task_id, DownloadWindowPhase::Progress);
        self.background_open.insert(task_id);
        DownloadWindowDirective::Open(DownloadWindowPhase::Progress)
    }

    pub(crate) fn request_window(
        &mut self,
        task_id: DownloadTaskId,
        phase: DownloadWindowPhase,
    ) -> DownloadWindowDirective {
        self.background_open.remove(&task_id);
        match self.open.entry(task_id) {
            Entry::Occupied(_) => DownloadWindowDirective::FocusExisting(phase),
            Entry::Vacant(entry) => {
                entry.insert(phase);
                DownloadWindowDirective::OpenAndFocus(phase)
            }
        }
    }

    pub(crate) fn window_closed(&mut self, task_id: DownloadTaskId) {
        self.open.remove(&task_id);
        self.background_open.remove(&task_id);
    }

    pub(crate) fn reopen_terminal_fallback(
        &mut self,
        task_id: DownloadTaskId,
        phase: DownloadWindowPhase,
    ) -> DownloadWindowDirective {
        self.background_open.remove(&task_id);
        match self.open.entry(task_id) {
            Entry::Occupied(_) => DownloadWindowDirective::RefreshExisting,
            Entry::Vacant(entry) => {
                entry.insert(phase);
                DownloadWindowDirective::Open(phase)
            }
        }
    }

    pub(crate) fn task_changed(
        &mut self,
        task_id: DownloadTaskId,
        status: DownloadStatus,
    ) -> DownloadWindowDirective {
        // Cancellation is an explicit user action: the open task window served
        // its purpose and must close instead of lingering in the progress view
        // with stale speed/ETA metrics. It is not a passive terminal phase
        // (no completed/failed fallback window is opened for it).
        if status == DownloadStatus::Cancelled {
            self.handled_terminal
                .retain(|(candidate, _)| *candidate != task_id);
            self.background_open.remove(&task_id);
            return match self.open.remove(&task_id) {
                Some(_) => DownloadWindowDirective::CloseExisting,
                None => DownloadWindowDirective::None,
            };
        }

        let terminal_phase = match status {
            DownloadStatus::Completed => Some(DownloadWindowPhase::Completed),
            DownloadStatus::Failed => Some(DownloadWindowPhase::Failed),
            _ => None,
        };
        let Some(terminal_phase) = terminal_phase else {
            self.handled_terminal
                .retain(|(candidate, _)| *candidate != task_id);
            return match self.open.get_mut(&task_id) {
                Some(phase) if *phase != DownloadWindowPhase::Progress => {
                    *phase = DownloadWindowPhase::Progress;
                    DownloadWindowDirective::TransformExisting(DownloadWindowPhase::Progress)
                }
                Some(_) => DownloadWindowDirective::RefreshExisting,
                None => DownloadWindowDirective::None,
            };
        };

        self.background_open.remove(&task_id);
        if !self.handled_terminal.insert((task_id, status)) {
            return if self.open.contains_key(&task_id) {
                DownloadWindowDirective::RefreshExisting
            } else {
                DownloadWindowDirective::None
            };
        }

        match self.open.insert(task_id, terminal_phase) {
            Some(previous) if previous != terminal_phase => {
                DownloadWindowDirective::TransformExisting(terminal_phase)
            }
            Some(_) => DownloadWindowDirective::RefreshExisting,
            None => DownloadWindowDirective::Open(terminal_phase),
        }
    }

    pub(crate) fn task_removed(&mut self, task_id: DownloadTaskId) -> DownloadWindowDirective {
        self.handled_terminal
            .retain(|(candidate, _)| *candidate != task_id);
        self.background_open.remove(&task_id);
        if self.open.remove(&task_id).is_some() {
            DownloadWindowDirective::CloseExisting
        } else {
            DownloadWindowDirective::None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_task_has_one_progress_window_and_reopen_focuses_it() {
        let task_id = DownloadTaskId::new();
        let mut lifecycle = DownloadWindowLifecycle::default();

        assert_eq!(
            lifecycle.request_progress(task_id),
            DownloadWindowDirective::OpenAndFocus(DownloadWindowPhase::Progress)
        );
        assert_eq!(
            lifecycle.request_progress(task_id),
            DownloadWindowDirective::FocusExisting(DownloadWindowPhase::Progress)
        );
    }

    #[test]
    fn closing_progress_is_presentation_only_and_completion_falls_back_once() {
        let task_id = DownloadTaskId::new();
        let mut lifecycle = DownloadWindowLifecycle::default();
        lifecycle.request_progress(task_id);
        lifecycle.window_closed(task_id);

        assert_eq!(
            lifecycle.task_changed(task_id, DownloadStatus::Downloading),
            DownloadWindowDirective::None
        );
        assert_eq!(
            lifecycle.task_changed(task_id, DownloadStatus::Completed),
            DownloadWindowDirective::Open(DownloadWindowPhase::Completed)
        );
        lifecycle.window_closed(task_id);
        assert_eq!(
            lifecycle.task_changed(task_id, DownloadStatus::Completed),
            DownloadWindowDirective::None
        );
    }

    #[test]
    fn completion_transforms_an_open_progress_window_in_place() {
        let task_id = DownloadTaskId::new();
        let mut lifecycle = DownloadWindowLifecycle::default();
        lifecycle.request_progress(task_id);

        assert_eq!(
            lifecycle.task_changed(task_id, DownloadStatus::Downloading),
            DownloadWindowDirective::RefreshExisting
        );
        assert_eq!(
            lifecycle.task_changed(task_id, DownloadStatus::Completed),
            DownloadWindowDirective::TransformExisting(DownloadWindowPhase::Completed)
        );
        assert_eq!(
            lifecycle.request_progress(task_id),
            DownloadWindowDirective::FocusExisting(DownloadWindowPhase::Progress)
        );
    }

    #[test]
    fn failure_uses_the_same_window_and_removal_closes_it() {
        let task_id = DownloadTaskId::new();
        let mut lifecycle = DownloadWindowLifecycle::default();
        lifecycle.request_progress(task_id);

        assert_eq!(
            lifecycle.task_changed(task_id, DownloadStatus::Failed),
            DownloadWindowDirective::TransformExisting(DownloadWindowPhase::Failed)
        );
        assert_eq!(
            lifecycle.task_removed(task_id),
            DownloadWindowDirective::CloseExisting
        );
        assert_eq!(
            lifecycle.task_removed(task_id),
            DownloadWindowDirective::None
        );
    }

    #[test]
    fn cancelling_a_task_closes_its_open_window_instead_of_lingering() {
        let task_id = DownloadTaskId::new();
        let mut lifecycle = DownloadWindowLifecycle::default();
        lifecycle.request_progress(task_id);

        assert_eq!(
            lifecycle.task_changed(task_id, DownloadStatus::Cancelled),
            DownloadWindowDirective::CloseExisting,
            "cancellation must close the open progress window"
        );
        assert_eq!(
            lifecycle.task_changed(task_id, DownloadStatus::Cancelled),
            DownloadWindowDirective::None,
            "a second cancellation event must be a no-op"
        );
        assert_eq!(
            lifecycle.task_changed(task_id, DownloadStatus::Downloading),
            DownloadWindowDirective::None,
            "a stale in-progress event after cancellation must not reopen the window"
        );
    }

    #[test]
    fn a_stale_native_window_is_replaced_by_one_terminal_fallback() {
        let task_id = DownloadTaskId::new();
        let mut lifecycle = DownloadWindowLifecycle::default();
        lifecycle.request_progress(task_id);
        assert_eq!(
            lifecycle.task_changed(task_id, DownloadStatus::Completed),
            DownloadWindowDirective::TransformExisting(DownloadWindowPhase::Completed)
        );
        lifecycle.window_closed(task_id);

        assert_eq!(
            lifecycle.reopen_terminal_fallback(task_id, DownloadWindowPhase::Completed),
            DownloadWindowDirective::Open(DownloadWindowPhase::Completed)
        );
        assert_eq!(
            lifecycle.reopen_terminal_fallback(task_id, DownloadWindowPhase::Completed),
            DownloadWindowDirective::RefreshExisting
        );
    }

    #[test]
    fn a_stale_completed_window_reopen_keeps_the_requested_terminal_phase() {
        let task_id = DownloadTaskId::new();
        let mut lifecycle = DownloadWindowLifecycle::default();

        assert_eq!(
            lifecycle.request_window(task_id, DownloadWindowPhase::Completed),
            DownloadWindowDirective::OpenAndFocus(DownloadWindowPhase::Completed)
        );
        assert_eq!(
            lifecycle.request_window(task_id, DownloadWindowPhase::Completed),
            DownloadWindowDirective::FocusExisting(DownloadWindowPhase::Completed)
        );
        lifecycle.window_closed(task_id);
        assert_eq!(
            lifecycle.request_window(task_id, DownloadWindowPhase::Completed),
            DownloadWindowDirective::OpenAndFocus(DownloadWindowPhase::Completed)
        );
    }

    #[test]
    fn retry_expands_a_failed_window_and_a_second_failure_transforms_again() {
        let task_id = DownloadTaskId::new();
        let mut lifecycle = DownloadWindowLifecycle::default();
        lifecycle.request_progress(task_id);
        assert_eq!(
            lifecycle.task_changed(task_id, DownloadStatus::Failed),
            DownloadWindowDirective::TransformExisting(DownloadWindowPhase::Failed)
        );
        assert_eq!(
            lifecycle.task_changed(task_id, DownloadStatus::Waiting),
            DownloadWindowDirective::TransformExisting(DownloadWindowPhase::Progress)
        );
        assert_eq!(
            lifecycle.task_changed(task_id, DownloadStatus::Failed),
            DownloadWindowDirective::TransformExisting(DownloadWindowPhase::Failed)
        );
    }
    #[test]
    fn retrying_a_completed_window_transforms_back_to_progress() {
        let task_id = DownloadTaskId::new();
        let mut lifecycle = DownloadWindowLifecycle::default();
        lifecycle.request_progress(task_id);

        assert_eq!(
            lifecycle.task_changed(task_id, DownloadStatus::Completed),
            DownloadWindowDirective::TransformExisting(DownloadWindowPhase::Completed)
        );
        assert_eq!(
            lifecycle.task_changed(task_id, DownloadStatus::Waiting),
            DownloadWindowDirective::TransformExisting(DownloadWindowPhase::Progress)
        );
    }

    #[test]
    fn restored_terminal_tasks_do_not_open_fallback_windows_on_startup() {
        let completed = DownloadTaskId::new();
        let failed = DownloadTaskId::new();
        let mut lifecycle = DownloadWindowLifecycle::default();

        lifecycle.restore_existing(completed, DownloadStatus::Completed);
        lifecycle.restore_existing(failed, DownloadStatus::Failed);

        assert_eq!(
            lifecycle.task_changed(completed, DownloadStatus::Completed),
            DownloadWindowDirective::None
        );
        assert_eq!(
            lifecycle.task_changed(failed, DownloadStatus::Failed),
            DownloadWindowDirective::None
        );
    }

    #[test]
    fn background_progress_windows_are_bounded_and_never_focus() {
        let task_ids = [
            DownloadTaskId::new(),
            DownloadTaskId::new(),
            DownloadTaskId::new(),
            DownloadTaskId::new(),
        ];
        let mut lifecycle = DownloadWindowLifecycle::default();

        for task_id in task_ids.iter().take(MAX_BACKGROUND_PROGRESS_WINDOWS) {
            assert_eq!(
                lifecycle.request_background_progress(*task_id),
                DownloadWindowDirective::Open(DownloadWindowPhase::Progress)
            );
        }
        assert_eq!(
            lifecycle.request_background_progress(task_ids[3]),
            DownloadWindowDirective::None,
            "automatic admission must cap visible progress windows"
        );
        assert_eq!(
            lifecycle.request_background_progress(task_ids[0]),
            DownloadWindowDirective::RefreshExisting,
            "repeated admission must refresh without reopening or focusing"
        );

        assert_eq!(
            lifecycle.request_window(task_ids[0], DownloadWindowPhase::Progress),
            DownloadWindowDirective::FocusExisting(DownloadWindowPhase::Progress)
        );
        lifecycle.window_closed(task_ids[0]);
        assert_eq!(
            lifecycle.request_background_progress(task_ids[3]),
            DownloadWindowDirective::Open(DownloadWindowPhase::Progress),
            "closing a window releases only its presentation admission"
        );
    }
}
