use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use nxtools_download::{
    DownloadCompletionAction, DownloadCompletionIntention, DownloadCompletionSnapshot,
    DownloadCompletionTask, DownloadStatus, DownloadTaskId,
};
use tracing::info;
#[derive(Clone)]
pub(crate) struct CompletionExitCorrelation(
    pub std::sync::Arc<std::sync::Mutex<Option<uuid::Uuid>>>,
);
impl gpui_kit::Global for CompletionExitCorrelation {}

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
    pub action: DownloadCompletionAction,
    pub generation: u64,
    pub work_revision: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShutdownTransition {
    None,
    Started(ShutdownCountdown),
    Cancelled(ShutdownCountdown, &'static str),
}

type Attempt = (DownloadTaskId, u64);
#[derive(Default)]
struct QueueRound {
    members: HashSet<Attempt>,
    blocked: bool,
    consumed: bool,
    invalidated_at: u64,
}
#[derive(Clone, PartialEq, Eq)]
struct TaskFact {
    attempt: u64,
    queue_id: String,
    status: DownloadStatus,
    intention: DownloadCompletionIntention,
}
impl From<&DownloadCompletionTask> for TaskFact {
    fn from(task: &DownloadCompletionTask) -> Self {
        Self {
            attempt: task.attempt,
            queue_id: task.queue_id.clone(),
            status: task.status,
            intention: task.intention,
        }
    }
}
#[derive(Default)]
struct CompletionGroup {
    members: HashSet<Attempt>,
    terminals: Vec<CompletionTerminal>,
    settled: bool,
    started_at: Option<Instant>,
}
#[derive(Default)]
pub(crate) struct CompletionActionCoordinator {
    pending_terminals: Vec<CompletionTerminal>,
    pending_terminal_ids: HashSet<DownloadTaskId>,
    groups: HashMap<uuid::Uuid, CompletionGroup>,
    grouped_attempts: HashMap<Attempt, uuid::Uuid>,
    explicit_directories: Vec<PathBuf>,
    generation: Option<u64>,
    work_revision: u64,
    observed: HashMap<DownloadTaskId, TaskFact>,
    queue_intentions: HashMap<String, DownloadCompletionIntention>,
    rounds: HashMap<String, QueueRound>,
    consumed_tasks: HashSet<Attempt>,
    ready_tasks: HashMap<Attempt, DownloadCompletionAction>,
    ready_queues: HashMap<String, DownloadCompletionAction>,
    active_countdown: Option<ShutdownCountdown>,
}

impl CompletionActionCoordinator {
    pub(crate) fn from_snapshot(snapshot: &DownloadCompletionSnapshot) -> Self {
        let mut owner = Self::default();
        owner.seed(snapshot);
        owner
    }
    fn seed(&mut self, snapshot: &DownloadCompletionSnapshot) {
        self.generation = Some(snapshot.generation);
        self.work_revision = snapshot.work_revision;
        self.observed = snapshot
            .tasks
            .iter()
            .map(|task| (task.task_id, TaskFact::from(task)))
            .collect();
        self.queue_intentions = snapshot
            .queues
            .iter()
            .map(|queue| (queue.id.clone(), queue.completion_intention))
            .collect();
        self.rounds.clear();
        self.ready_tasks.clear();
        self.ready_queues.clear();
        self.consumed_tasks = snapshot
            .tasks
            .iter()
            .filter(|task| !is_nonterminal(task.status))
            .map(|task| (task.task_id, task.attempt))
            .collect();
        for task in snapshot
            .tasks
            .iter()
            .filter(|task| is_nonterminal(task.status))
        {
            let invalidated_at = snapshot
                .queue_invalidations
                .get(&task.queue_id)
                .copied()
                .unwrap_or(0);
            let round = self
                .rounds
                .entry(task.queue_id.clone())
                .or_insert_with(|| QueueRound {
                    invalidated_at,
                    // Initial nonterminal membership establishes this owner's baseline.
                    ..Default::default()
                });
            round.members.insert((task.task_id, task.attempt));
        }
    }
    pub(crate) fn register_completion_members(&mut self, snapshot: &DownloadCompletionSnapshot) {
        let live: HashSet<_> = snapshot.tasks.iter().map(|task| task.task_id).collect();
        self.groups
            .retain(|_, group| group.members.iter().any(|(id, _)| live.contains(id)));
        self.grouped_attempts
            .retain(|(id, _), group| live.contains(id) && self.groups.contains_key(group));
        for task in &snapshot.tasks {
            let Some(group_id) = task.completion_batch_id else {
                continue;
            };
            let group = self.groups.entry(group_id).or_default();
            if group.settled {
                continue;
            }
            if task.attempt == 1 && group.members.remove(&(task.task_id, 0)) {
                self.grouped_attempts.remove(&(task.task_id, 0));
            } else if group.members.iter().any(|(id, _)| *id == task.task_id) {
                continue;
            }
            if task.attempt <= 1 && group.members.insert((task.task_id, task.attempt)) {
                group.started_at.get_or_insert_with(Instant::now);
                self.grouped_attempts
                    .insert((task.task_id, task.attempt), group_id);
            }
        }
    }
    pub(crate) fn settle_completion_members(&mut self, snapshot: &DownloadCompletionSnapshot) {
        let statuses: HashMap<_, _> = snapshot
            .tasks
            .iter()
            .map(|task| (task.task_id, (task.attempt, task.status)))
            .collect();
        for (group_id, group) in &mut self.groups {
            if group.settled || group.members.is_empty() {
                continue;
            }
            if group.members.iter().all(|(id, attempt)| {
                group
                    .terminals
                    .iter()
                    .any(|terminal| terminal.task_id == *id)
                    || statuses.get(id).is_none_or(|(current, status)| {
                        current != attempt
                            || matches!(
                                status,
                                DownloadStatus::Completed
                                    | DownloadStatus::Failed
                                    | DownloadStatus::Cancelled
                            )
                    })
            }) {
                group.settled = true;
                info!(event = "download.completion_group.settled", correlation_id = %group_id,
                    member_count = group.members.len(), completed_count = group.terminals.iter().filter(|t| t.status == DownloadStatus::Completed).count(),
                    failed_count = group.terminals.iter().filter(|t| t.status == DownloadStatus::Failed).count(),
                    elapsed_ms = group.started_at.map_or(0, |at| at.elapsed().as_millis()),
                    "download batch members settled");
            }
        }
    }
    pub(crate) fn record_terminal_attempt(&mut self, terminal: CompletionTerminal, attempt: u64) {
        if let Some(group_id) = self.grouped_attempts.get(&(terminal.task_id, attempt))
            && let Some(group) = self.groups.get_mut(group_id)
        {
            if !group.settled
                && !group
                    .terminals
                    .iter()
                    .any(|item| item.task_id == terminal.task_id)
            {
                group.terminals.push(terminal);
            }
        } else {
            self.record_terminal(terminal);
        }
    }
    pub(crate) fn is_grouped_attempt(&self, task_id: DownloadTaskId, attempt: u64) -> bool {
        self.grouped_attempts.contains_key(&(task_id, attempt))
    }
    pub(crate) fn has_ready_group_feedback(&self) -> bool {
        self.groups
            .values()
            .any(|group| group.settled && !group.terminals.is_empty())
    }
    pub(crate) fn take_ready_group_feedback(
        &mut self,
        global_directory: bool,
    ) -> Vec<(uuid::Uuid, CompletionFeedbackBatch)> {
        let settled: Vec<_> = self
            .groups
            .iter_mut()
            .filter(|(_, group)| group.settled)
            .map(|(id, group)| (*id, std::mem::take(&mut group.terminals)))
            .filter(|(_, items)| !items.is_empty())
            .collect();
        let pending = std::mem::take(&mut self.pending_terminals);
        let pending_ids = std::mem::take(&mut self.pending_terminal_ids);
        let directories = std::mem::take(&mut self.explicit_directories);
        let batches = settled
            .into_iter()
            .filter_map(|(id, terminals)| {
                self.pending_terminals = terminals;
                self.take_feedback_batch_for_directory_policy(global_directory)
                    .map(|batch| (id, batch))
            })
            .collect();
        self.pending_terminals = pending;
        self.pending_terminal_ids = pending_ids;
        self.explicit_directories = directories;
        batches
    }
    pub(crate) fn record_terminal(&mut self, terminal: CompletionTerminal) {
        if self.pending_terminal_ids.insert(terminal.task_id) {
            self.pending_terminals.push(terminal);
        }
    }
    pub(crate) fn has_pending_feedback(&self) -> bool {
        !self.pending_terminals.is_empty() || !self.explicit_directories.is_empty()
    }
    pub(crate) fn take_feedback_batch_for_directory_policy(
        &mut self,
        global_directory: bool,
    ) -> Option<CompletionFeedbackBatch> {
        if !self.has_pending_feedback() {
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
        let mut directories = std::mem::take(&mut self.explicit_directories);
        if global_directory {
            directories.extend(
                terminals
                    .iter()
                    .filter(|terminal| terminal.status == DownloadStatus::Completed)
                    .filter_map(|terminal| terminal.directory.clone()),
            );
        }
        let mut keys = HashSet::new();
        directories.retain(|directory| keys.insert(directory.to_string_lossy().to_lowercase()));
        let suppressed_directory_count = directories.len().saturating_sub(MAX_REVEALED_DIRECTORIES);
        directories.truncate(MAX_REVEALED_DIRECTORIES);
        Some(CompletionFeedbackBatch {
            completed_count,
            failed_count,
            directories,
            suppressed_directory_count,
            single_display_name: (terminals.len() == 1).then(|| terminals[0].display_name.clone()),
            single_error_message: (terminals.len() == 1)
                .then(|| terminals[0].error_message.clone())
                .flatten(),
        })
    }
    #[cfg(test)]
    pub(crate) fn take_feedback_batch(&mut self) -> Option<CompletionFeedbackBatch> {
        self.take_feedback_batch_for_directory_policy(true)
    }
    pub(crate) fn active_countdown(&self) -> Option<ShutdownCountdown> {
        self.active_countdown
    }
    pub(crate) fn clear_countdown(&mut self) -> Option<ShutdownCountdown> {
        self.active_countdown.take()
    }
    pub(crate) fn cancel_countdown(
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
    pub(crate) fn take_countdown_for_execution(
        &mut self,
        correlation_id: uuid::Uuid,
    ) -> Option<ShutdownCountdown> {
        self.cancel_countdown(correlation_id)
    }
    #[cfg(test)]
    pub(crate) fn evaluate_completion(
        &mut self,
        snapshot: &DownloadCompletionSnapshot,
        now: Instant,
    ) -> ShutdownTransition {
        self.evaluate_completion_in_slot(snapshot, now, true)
    }

    pub(crate) fn evaluate_completion_in_slot(
        &mut self,
        snapshot: &DownloadCompletionSnapshot,
        now: Instant,
        action_slot_available: bool,
    ) -> ShutdownTransition {
        if self.generation != Some(snapshot.generation) {
            let old = self.clear_countdown();
            self.seed(snapshot);
            return old.map_or(ShutdownTransition::None, |countdown| {
                ShutdownTransition::Cancelled(countdown, "generation_changed")
            });
        }
        let current: HashMap<_, _> = snapshot
            .tasks
            .iter()
            .map(|task| (task.task_id, TaskFact::from(task)))
            .collect();
        let intentions: HashMap<_, _> = snapshot
            .queues
            .iter()
            .map(|queue| (queue.id.clone(), queue.completion_intention))
            .collect();
        let cancelled = if self.active_countdown.is_some() {
            if snapshot.work_revision != self.work_revision {
                self.clear_countdown().map(|value| (value, "new_work"))
            } else if current != self.observed || intentions != self.queue_intentions {
                self.clear_countdown()
                    .map(|value| (value, "conditions_changed"))
            } else {
                None
            }
        } else {
            None
        };
        let mut fresh_by_queue: HashMap<String, HashSet<Attempt>> = HashMap::new();
        for task in &snapshot.tasks {
            let token = (task.task_id, task.attempt);
            let previous = self.observed.get(&task.task_id);
            let fresh = previous.is_none_or(|previous| {
                previous.attempt != task.attempt
                    || (previous.queue_id != task.queue_id && is_nonterminal(task.status))
            });
            let invalidated_at = snapshot
                .queue_invalidations
                .get(&task.queue_id)
                .copied()
                .unwrap_or(0);
            // First dispatch advances an already seeded admission, not its acceptance.
            if previous.is_some_and(|previous| {
                previous.attempt == 0 && task.attempt == 1 && previous.queue_id == task.queue_id
            }) && let Some(round) = self.rounds.get_mut(&task.queue_id)
                && round.invalidated_at == invalidated_at
                && round.members.remove(&(task.task_id, 0))
            {
                round.members.insert(token);
            }
            if fresh && (invalidated_at == 0 || task.accepted_revision > invalidated_at) {
                fresh_by_queue
                    .entry(task.queue_id.clone())
                    .or_default()
                    .insert(token);
            }
            if task.status == DownloadStatus::Completed && self.consumed_tasks.insert(token) {
                if let Some(action) = task.intention.action {
                    self.ready_tasks.insert(token, action);
                }
                if task.intention.open_directory {
                    self.explicit_directories.extend(task.directory.clone());
                }
            } else if matches!(
                task.status,
                DownloadStatus::Failed | DownloadStatus::Cancelled
            ) {
                self.consumed_tasks.insert(token);
                self.ready_tasks.remove(&token);
            }
        }
        self.ready_tasks.retain(|(id, attempt), action| {
            current.get(id).is_some_and(|fact| {
                fact.attempt == *attempt
                    && fact.status == DownloadStatus::Completed
                    && fact.intention.action == Some(*action)
            })
        });
        for queue in &snapshot.queues {
            let fresh = fresh_by_queue.remove(&queue.id).unwrap_or_default();
            let round = self.rounds.entry(queue.id.clone()).or_default();
            let invalidated_at = snapshot
                .queue_invalidations
                .get(&queue.id)
                .copied()
                .unwrap_or(0);
            if round.invalidated_at != invalidated_at {
                round.invalidated_at = invalidated_at;
                round.blocked = true;
                self.ready_queues.remove(&queue.id);
                info!(
                    event = "download.completion.condition_changed",
                    generation = snapshot.generation,
                    queue_id = queue.id,
                    condition = "membership_invalidated",
                    invalidated_at,
                    member_count = round.members.len(),
                    "queue completion round aborted"
                );
            }
            if !fresh.is_empty() {
                let previous_round_ended = round.consumed
                    || round.blocked
                    || round.members.iter().any(|(id, attempt)| {
                        current.get(id).is_none_or(|fact| {
                            fact.attempt != *attempt
                                || fact.queue_id != queue.id
                                || matches!(
                                    fact.status,
                                    DownloadStatus::Failed | DownloadStatus::Cancelled
                                )
                        })
                    });
                if previous_round_ended || round.members.is_empty() {
                    round.members = snapshot
                        .tasks
                        .iter()
                        .filter(|task| {
                            task.queue_id == queue.id
                                && (is_nonterminal(task.status)
                                    || fresh.contains(&(task.task_id, task.attempt)))
                        })
                        .map(|task| (task.task_id, task.attempt))
                        .collect();
                }
                round.blocked = false;
                round.consumed = false;
                self.ready_queues.remove(&queue.id);
            }
            round.members.extend(fresh);
            if round.consumed {
                continue;
            }
            let blocked = round.members.iter().any(|(id, attempt)| {
                current.get(id).is_none_or(|fact| {
                    fact.attempt != *attempt
                        || fact.queue_id != queue.id
                        || matches!(
                            fact.status,
                            DownloadStatus::Failed | DownloadStatus::Cancelled
                        )
                })
            });
            if blocked && !round.blocked {
                round.blocked = true;
                info!(
                    event = "download.completion.condition_changed",
                    generation = snapshot.generation,
                    queue_id = queue.id,
                    condition = "round_aborted",
                    member_count = round.members.len(),
                    "queue completion round is not successful"
                );
            }
            if round.blocked || round.members.is_empty() {
                continue;
            }
            if round.members.iter().all(|(id, _)| {
                current
                    .get(id)
                    .is_some_and(|fact| fact.status == DownloadStatus::Completed)
            }) {
                round.consumed = true;
                if let Some(action) = queue.completion_intention.action {
                    self.ready_queues.insert(queue.id.clone(), action);
                }
                if queue.completion_intention.open_directory {
                    self.explicit_directories.extend(
                        snapshot
                            .tasks
                            .iter()
                            .filter(|task| round.members.contains(&(task.task_id, task.attempt)))
                            .filter_map(|task| task.directory.clone()),
                    );
                }
                info!(
                    event = "download.completion.condition_changed",
                    generation = snapshot.generation,
                    queue_id = queue.id,
                    condition = "all_success",
                    member_count = round.members.len(),
                    "nonempty queue completion round succeeded"
                );
            }
        }
        self.ready_queues.retain(|id, action| {
            intentions
                .get(id)
                .is_some_and(|intention| intention.action == Some(*action))
                && self.rounds.get(id).is_some_and(|round| {
                    !round.members.is_empty()
                        && round.members.iter().all(|(task_id, attempt)| {
                            current.get(task_id).is_some_and(|fact| {
                                fact.attempt == *attempt
                                    && fact.queue_id == *id
                                    && fact.status == DownloadStatus::Completed
                            })
                        })
                })
        });
        self.rounds.retain(|id, _| intentions.contains_key(id));
        self.observed = current;
        self.queue_intentions = intentions;
        self.work_revision = snapshot.work_revision;
        if let Some((countdown, reason)) = cancelled {
            return ShutdownTransition::Cancelled(countdown, reason);
        }
        if !action_slot_available
            || self.active_countdown.is_some()
            || snapshot
                .tasks
                .iter()
                .any(|task| is_nonterminal(task.status))
        {
            return ShutdownTransition::None;
        }
        let action = self
            .ready_tasks
            .values()
            .chain(self.ready_queues.values())
            .copied()
            .max_by_key(|action| action_priority(*action));
        let Some(action) = action else {
            return ShutdownTransition::None;
        };
        self.ready_tasks.clear();
        self.ready_queues.clear();
        let countdown = ShutdownCountdown {
            correlation_id: uuid::Uuid::new_v4(),
            started_at: now,
            deadline: now + SHUTDOWN_COUNTDOWN_DURATION,
            duration: SHUTDOWN_COUNTDOWN_DURATION,
            action,
            generation: snapshot.generation,
            work_revision: snapshot.work_revision,
        };
        self.active_countdown = Some(countdown);
        ShutdownTransition::Started(countdown)
    }

    #[cfg(test)]
    pub(crate) fn set_shutdown_armed(&mut self, task_id: DownloadTaskId, armed: bool) {
        let fact = self.observed.entry(task_id).or_insert(TaskFact {
            attempt: 1,
            queue_id: "default".into(),
            status: DownloadStatus::Created,
            intention: DownloadCompletionIntention::default(),
        });
        fact.intention.action = armed.then_some(DownloadCompletionAction::Shutdown);
    }
    #[cfg(test)]
    pub(crate) fn evaluate_shutdown(
        &mut self,
        tasks: impl IntoIterator<Item = (DownloadTaskId, DownloadStatus)>,
        now: Instant,
    ) -> ShutdownTransition {
        let tasks: Vec<_> = tasks
            .into_iter()
            .map(|(task_id, status)| DownloadCompletionTask {
                task_id,
                status,
                attempt: self.observed.get(&task_id).map_or(1, |fact| fact.attempt),
                accepted_revision: 0,
                queue_id: "default".into(),
                intention: self
                    .observed
                    .get(&task_id)
                    .map_or(Default::default(), |fact| fact.intention),
                directory: None,
                completion_batch_id: None,
            })
            .collect();
        if tasks.is_empty() {
            return ShutdownTransition::None;
        }
        let revision = self.work_revision
            + u64::from(
                tasks
                    .iter()
                    .any(|task| !self.observed.contains_key(&task.task_id)),
            );
        self.generation = Some(0);
        let snapshot = DownloadCompletionSnapshot {
            generation: 0,
            work_revision: revision,
            queue_invalidations: HashMap::new(),
            tasks,
            queues: Vec::new(),
        };
        self.evaluate_completion(&snapshot, now)
    }
}
fn action_priority(action: DownloadCompletionAction) -> u8 {
    match action {
        DownloadCompletionAction::Shutdown => 4,
        DownloadCompletionAction::Hibernate => 3,
        DownloadCompletionAction::Sleep => 2,
        DownloadCompletionAction::ExitApplication => 1,
    }
}
pub(crate) fn completion_action_label(action: DownloadCompletionAction) -> &'static str {
    match action {
        DownloadCompletionAction::Shutdown => "关机",
        DownloadCompletionAction::Hibernate => "休眠",
        DownloadCompletionAction::Sleep => "睡眠",
        DownloadCompletionAction::ExitApplication => "退出应用",
    }
}
fn is_nonterminal(status: DownloadStatus) -> bool {
    !matches!(
        status,
        DownloadStatus::Completed | DownloadStatus::Failed | DownloadStatus::Cancelled
    )
}

#[cfg(test)]
#[path = "completion_actions_b5_tests.rs"]
mod b5_tests;

#[cfg(test)]
#[path = "completion_actions_grouped_tests.rs"]
mod grouped_tests;

#[cfg(test)]
mod tests {
    use super::*;
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
                now
            ),
            ShutdownTransition::None
        );
        let ShutdownTransition::Started(countdown) = coordinator.evaluate_shutdown(
            [
                (armed, DownloadStatus::Completed),
                (task_id(42), DownloadStatus::Completed),
            ],
            now,
        ) else {
            panic!("idle queue must start a countdown")
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
                now
            ),
            ShutdownTransition::Cancelled(_, "new_work")
        ));
        let restarted = CompletionActionCoordinator::default();
        assert!(!restarted.observed.contains_key(&armed));
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
                    (task_id(82), DownloadStatus::Failed)
                ],
                now
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
            panic!("completed armed task must start countdown")
        };
        assert_eq!(
            coordinator.take_countdown_for_execution(countdown.correlation_id),
            Some(countdown)
        );
        assert_eq!(
            coordinator.evaluate_shutdown(
                [
                    (armed, DownloadStatus::Completed),
                    (task_id(92), DownloadStatus::Downloading)
                ],
                now
            ),
            ShutdownTransition::None
        );
    }
}
