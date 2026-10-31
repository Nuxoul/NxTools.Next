use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use nxtools_download::{DownloadStatus, DownloadTaskId};

pub(crate) const COMPLETION_FEEDBACK_DELAY: Duration = Duration::from_millis(400);
pub(crate) const COMPLETION_PLATFORM_TIMEOUT: Duration = Duration::from_secs(15);
pub(crate) const SHUTDOWN_COUNTDOWN_DURATION: Duration = Duration::from_secs(30);
pub(crate) const MAX_REVEALED_DIRECTORIES: usize = 3;

#[derive(Debug, Clone)]
pub(crate) struct CompletionTerminal {
    pub task_id: DownloadTaskId,
    pub status: DownloadStatus,
    pub directory: Option<PathBuf>,
    pub display_name: String,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct CompletionFeedbackBatch {
    pub completed_count: usize,
    pub failed_count: usize,
    pub directories: Vec<PathBuf>,
    pub suppressed_directory_count: usize,
    pub single_display_name: Option<String>,
    pub single_error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ShutdownCountdown {
    pub correlation_id: uuid::Uuid,
    pub started_at: Instant,
    pub deadline: Instant,
    pub duration: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShutdownTransition {
    None,
    Started(ShutdownCountdown),
    Cancelled(ShutdownCountdown, &'static str),
}

#[derive(Default)]
pub(crate) struct CompletionActionCoordinator {
    pending_terminals: Vec<CompletionTerminal>,
    pending_terminal_ids: HashSet<DownloadTaskId>,
    shutdown_armed_tasks: HashSet<DownloadTaskId>,
    active_countdown: Option<ShutdownCountdown>,
    active_countdown_task_ids: HashSet<DownloadTaskId>,
}

impl CompletionActionCoordinator {
    pub(crate) fn record_terminal(&mut self, terminal: CompletionTerminal) {
        if self.pending_terminal_ids.insert(terminal.task_id) {
            self.pending_terminals.push(terminal);
        }
    }

    pub(crate) fn take_feedback_batch(&mut self) -> Option<CompletionFeedbackBatch> {
        if self.pending_terminals.is_empty() {
            return None;
        }
        let terminals = std::mem::take(&mut self.pending_terminals);
        self.pending_terminal_ids.clear();
        let completed_count = terminals
            .iter()
            .filter(|terminal| terminal.status == DownloadStatus::Completed)
            .count();
        let failed_count = terminals
            .iter()
            .filter(|terminal| terminal.status == DownloadStatus::Failed)
            .count();
        let mut directory_keys = HashSet::new();
        let mut all_directories = Vec::new();
        for directory in terminals
            .iter()
            .filter(|terminal| terminal.status == DownloadStatus::Completed)
            .filter_map(|terminal| terminal.directory.as_ref())
        {
            let key = directory.to_string_lossy().to_lowercase();
            if directory_keys.insert(key) {
                all_directories.push(directory.clone());
            }
        }
        let suppressed_directory_count = all_directories
            .len()
            .saturating_sub(MAX_REVEALED_DIRECTORIES);
        all_directories.truncate(MAX_REVEALED_DIRECTORIES);
        let single_display_name = (terminals.len() == 1).then(|| terminals[0].display_name.clone());
        let single_error_message = (terminals.len() == 1)
            .then(|| terminals[0].error_message.clone())
            .flatten();
        Some(CompletionFeedbackBatch {
            completed_count,
            failed_count,
            directories: all_directories,
            suppressed_directory_count,
            single_display_name,
            single_error_message,
        })
    }

    pub(crate) fn set_shutdown_armed(&mut self, task_id: DownloadTaskId, armed: bool) {
        if armed {
            self.shutdown_armed_tasks.insert(task_id);
        } else {
            self.shutdown_armed_tasks.remove(&task_id);
        }
    }

    pub(crate) fn is_shutdown_armed(&self, task_id: DownloadTaskId) -> bool {
        self.shutdown_armed_tasks.contains(&task_id)
    }

    pub(crate) fn active_countdown(&self) -> Option<ShutdownCountdown> {
        self.active_countdown
    }

    pub(crate) fn clear_countdown(&mut self) -> Option<ShutdownCountdown> {
        self.active_countdown_task_ids.clear();
        self.active_countdown.take()
    }

    pub(crate) fn take_countdown_for_execution(
        &mut self,
        correlation_id: uuid::Uuid,
    ) -> Option<ShutdownCountdown> {
        if self
            .active_countdown
            .is_none_or(|countdown| countdown.correlation_id != correlation_id)
        {
            return None;
        }
        self.clear_countdown()
    }

    pub(crate) fn evaluate_shutdown(
        &mut self,
        tasks: impl IntoIterator<Item = (DownloadTaskId, DownloadStatus)>,
        now: Instant,
    ) -> ShutdownTransition {
        let statuses = tasks.into_iter().collect::<HashMap<_, _>>();
        let has_nonterminal = statuses.values().any(|status| is_nonterminal(*status));
        if self.active_countdown.is_some() {
            let has_new_task = statuses
                .keys()
                .any(|task_id| !self.active_countdown_task_ids.contains(task_id));
            if has_new_task || has_nonterminal {
                let countdown = self
                    .active_countdown
                    .take()
                    .expect("active countdown was checked above");
                self.active_countdown_task_ids.clear();
                self.shutdown_armed_tasks.clear();
                return ShutdownTransition::Cancelled(countdown, "new_work");
            }
            return ShutdownTransition::None;
        }

        self.shutdown_armed_tasks.retain(|task_id| {
            statuses.get(task_id).is_some_and(|status| {
                is_nonterminal(*status) || *status == DownloadStatus::Completed
            })
        });
        let armed_task_completed = self
            .shutdown_armed_tasks
            .iter()
            .any(|task_id| statuses.get(task_id) == Some(&DownloadStatus::Completed));
        if !armed_task_completed || has_nonterminal {
            return ShutdownTransition::None;
        }
        self.shutdown_armed_tasks.clear();
        let countdown = ShutdownCountdown {
            correlation_id: uuid::Uuid::new_v4(),
            started_at: now,
            deadline: now + SHUTDOWN_COUNTDOWN_DURATION,
            duration: SHUTDOWN_COUNTDOWN_DURATION,
        };
        self.active_countdown = Some(countdown);
        self.active_countdown_task_ids = statuses.keys().copied().collect();
        ShutdownTransition::Started(countdown)
    }
}

fn is_nonterminal(status: DownloadStatus) -> bool {
    !matches!(
        status,
        DownloadStatus::Completed | DownloadStatus::Failed | DownloadStatus::Cancelled
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    use nxtools_download::{DownloadStatus, DownloadTaskId};

    use super::{CompletionActionCoordinator, CompletionTerminal, ShutdownTransition};

    fn task_id(value: u128) -> DownloadTaskId {
        DownloadTaskId::from_uuid(uuid::Uuid::from_u128(value))
    }

    #[test]
    fn completion_burst_is_deduplicated_and_limits_distinct_directory_actions() {
        let mut coordinator = CompletionActionCoordinator::default();
        for index in 0..5 {
            coordinator.record_terminal(CompletionTerminal {
                task_id: task_id(index + 1),
                status: DownloadStatus::Completed,
                directory: Some(PathBuf::from(format!(r"C:\Downloads\batch-{index}"))),
                display_name: format!("file-{index}.bin"),
                error_message: None,
            });
        }
        coordinator.record_terminal(CompletionTerminal {
            task_id: task_id(1),
            status: DownloadStatus::Completed,
            directory: Some(PathBuf::from(r"C:\Downloads\batch-0")),
            display_name: "duplicate.bin".into(),
            error_message: None,
        });

        let batch = coordinator
            .take_feedback_batch()
            .expect("one completion batch");
        assert_eq!(batch.completed_count, 5);
        assert_eq!(batch.failed_count, 0);
        assert_eq!(batch.directories.len(), 3);
        assert_eq!(batch.suppressed_directory_count, 2);
        assert!(coordinator.take_feedback_batch().is_none());
    }

    #[test]
    fn armed_completed_task_starts_one_transient_countdown_only_after_queue_is_idle() {
        let mut coordinator = CompletionActionCoordinator::default();
        let armed = task_id(41);
        coordinator.set_shutdown_armed(armed, true);
        let now = Instant::now();

        assert_eq!(
            coordinator.evaluate_shutdown(
                [
                    (armed, DownloadStatus::Completed),
                    (task_id(42), DownloadStatus::Downloading)
                ],
                now,
            ),
            ShutdownTransition::None
        );
        let transition = coordinator.evaluate_shutdown(
            [
                (armed, DownloadStatus::Completed),
                (task_id(42), DownloadStatus::Completed),
            ],
            now,
        );
        let ShutdownTransition::Started(countdown) = transition else {
            panic!("idle queue must start a countdown");
        };
        assert_eq!(countdown.duration, Duration::from_secs(30));
        assert_eq!(
            coordinator.evaluate_shutdown([], now),
            ShutdownTransition::None
        );
    }

    #[test]
    fn new_nonterminal_work_cancels_countdown_and_restart_has_no_armed_intent() {
        let mut coordinator = CompletionActionCoordinator::default();
        let armed = task_id(71);
        coordinator.set_shutdown_armed(armed, true);
        let now = Instant::now();
        assert!(matches!(
            coordinator.evaluate_shutdown([(armed, DownloadStatus::Completed)], now),
            ShutdownTransition::Started(_)
        ));
        assert!(matches!(
            coordinator.evaluate_shutdown(
                [
                    (armed, DownloadStatus::Completed),
                    (task_id(72), DownloadStatus::Waiting)
                ],
                now,
            ),
            ShutdownTransition::Cancelled(_, "new_work")
        ));

        let restarted = CompletionActionCoordinator::default();
        assert!(!restarted.is_shutdown_armed(armed));
        assert!(restarted.active_countdown().is_none());
    }

    #[test]
    fn a_new_task_cancels_countdown_even_if_it_is_already_terminal_in_the_next_snapshot() {
        let mut coordinator = CompletionActionCoordinator::default();
        let armed = task_id(81);
        coordinator.set_shutdown_armed(armed, true);
        let now = Instant::now();
        assert!(matches!(
            coordinator.evaluate_shutdown([(armed, DownloadStatus::Completed)], now),
            ShutdownTransition::Started(_)
        ));

        assert!(matches!(
            coordinator.evaluate_shutdown(
                [
                    (armed, DownloadStatus::Completed),
                    (task_id(82), DownloadStatus::Failed),
                ],
                now,
            ),
            ShutdownTransition::Cancelled(_, "new_work")
        ));
    }

    #[test]
    fn a_shutdown_command_in_flight_is_no_longer_an_active_cancellable_countdown() {
        let mut coordinator = CompletionActionCoordinator::default();
        let armed = task_id(91);
        coordinator.set_shutdown_armed(armed, true);
        let now = Instant::now();
        let ShutdownTransition::Started(countdown) =
            coordinator.evaluate_shutdown([(armed, DownloadStatus::Completed)], now)
        else {
            panic!("completed armed task must start countdown");
        };
        assert_eq!(
            coordinator.take_countdown_for_execution(countdown.correlation_id),
            Some(countdown)
        );

        assert_eq!(
            coordinator.evaluate_shutdown(
                [
                    (armed, DownloadStatus::Completed),
                    (task_id(92), DownloadStatus::Downloading),
                ],
                now,
            ),
            ShutdownTransition::None
        );
    }
}
