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
    FocusExisting,
    RefreshExisting,
    TransformExisting(DownloadWindowPhase),
    CloseExisting,
}

#[derive(Default)]
pub(crate) struct DownloadWindowLifecycle {
    open: HashMap<DownloadTaskId, DownloadWindowPhase>,
    handled_terminal: HashSet<(DownloadTaskId, DownloadStatus)>,
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

    pub(crate) fn request_window(
        &mut self,
        task_id: DownloadTaskId,
        phase: DownloadWindowPhase,
    ) -> DownloadWindowDirective {
        match self.open.entry(task_id) {
            Entry::Occupied(_) => DownloadWindowDirective::FocusExisting,
            Entry::Vacant(entry) => {
                entry.insert(phase);
                DownloadWindowDirective::Open(phase)
            }
        }
    }

    pub(crate) fn window_closed(&mut self, task_id: DownloadTaskId) {
        self.open.remove(&task_id);
    }

    pub(crate) fn reopen_terminal_fallback(
        &mut self,
        task_id: DownloadTaskId,
        phase: DownloadWindowPhase,
    ) -> DownloadWindowDirective {
        match self.open.entry(task_id) {
            Entry::Occupied(_) => DownloadWindowDirective::FocusExisting,
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
            DownloadWindowDirective::Open(DownloadWindowPhase::Progress)
        );
        assert_eq!(
            lifecycle.request_progress(task_id),
            DownloadWindowDirective::FocusExisting
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
            DownloadWindowDirective::FocusExisting
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
            DownloadWindowDirective::FocusExisting
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
}
