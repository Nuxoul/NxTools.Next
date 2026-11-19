use super::*;

fn id(value: u128) -> DownloadTaskId {
    DownloadTaskId::from_uuid(uuid::Uuid::from_u128(value))
}

fn task(
    value: u128,
    group: Option<uuid::Uuid>,
    attempt: u64,
    status: DownloadStatus,
) -> DownloadCompletionTask {
    DownloadCompletionTask {
        task_id: id(value),
        status,
        attempt,
        accepted_revision: 1,
        queue_id: "default".into(),
        intention: DownloadCompletionIntention::default(),
        directory: None,
        completion_batch_id: group,
    }
}

fn snapshot(tasks: Vec<DownloadCompletionTask>) -> DownloadCompletionSnapshot {
    DownloadCompletionSnapshot {
        generation: 1,
        work_revision: 1,
        queue_invalidations: HashMap::new(),
        tasks,
        queues: Vec::new(),
    }
}

fn terminal(value: u128, status: DownloadStatus) -> CompletionTerminal {
    CompletionTerminal {
        task_id: id(value),
        status,
        directory: None,
        display_name: format!("{value}.bin"),
        error_message: None,
    }
}

#[test]
fn overlapping_original_groups_settle_once_and_keep_standalone_feedback_separate() {
    let a = uuid::Uuid::new_v4();
    let b = uuid::Uuid::new_v4();
    let cancelled = uuid::Uuid::new_v4();
    let mut owner = CompletionActionCoordinator::default();
    owner.register_completion_members(&snapshot(vec![
        task(1, Some(a), 0, DownloadStatus::Created),
        task(2, Some(a), 0, DownloadStatus::Created),
        task(3, Some(b), 0, DownloadStatus::Created),
        task(4, Some(b), 0, DownloadStatus::Created),
        task(5, Some(cancelled), 0, DownloadStatus::Created),
        task(6, Some(cancelled), 0, DownloadStatus::Created),
    ]));
    owner.record_terminal_attempt(terminal(1, DownloadStatus::Completed), 0);
    owner.record_terminal_attempt(terminal(3, DownloadStatus::Failed), 0);
    owner.record_terminal(terminal(9, DownloadStatus::Completed));
    owner.settle_completion_members(&snapshot(vec![
        task(1, Some(a), 0, DownloadStatus::Completed),
        task(2, Some(a), 0, DownloadStatus::Created),
        task(3, Some(b), 0, DownloadStatus::Failed),
        task(4, Some(b), 0, DownloadStatus::Created),
        task(5, Some(cancelled), 0, DownloadStatus::Created),
        task(6, Some(cancelled), 0, DownloadStatus::Created),
    ]));
    assert!(!owner.has_ready_group_feedback());
    let standalone = owner
        .take_feedback_batch_for_directory_policy(false)
        .expect("standalone feedback");
    assert_eq!(
        (standalone.completed_count, standalone.failed_count),
        (1, 0)
    );
    owner.record_terminal_attempt(terminal(2, DownloadStatus::Failed), 0);
    owner.settle_completion_members(&snapshot(vec![
        task(1, Some(a), 0, DownloadStatus::Completed),
        task(2, Some(a), 0, DownloadStatus::Failed),
        task(3, Some(b), 0, DownloadStatus::Failed),
        task(5, Some(cancelled), 0, DownloadStatus::Cancelled),
    ]));
    let mut batches = owner.take_ready_group_feedback(false);
    assert_eq!(batches.len(), 2);
    batches.sort_by_key(|(_, batch)| batch.completed_count);
    assert_eq!(
        (batches[0].1.completed_count, batches[0].1.failed_count),
        (0, 1)
    );
    assert_eq!(
        (batches[1].1.completed_count, batches[1].1.failed_count),
        (1, 1)
    );
    assert_eq!(
        batches.iter().map(|(id, _)| *id).collect::<HashSet<_>>(),
        HashSet::from([a, b])
    );
    assert!(owner.take_ready_group_feedback(false).is_empty());
    assert!(!owner.has_pending_feedback());
}

#[test]
fn admission_zero_to_first_dispatch_is_one_membership_but_retry_is_standalone() {
    let group_id = uuid::Uuid::new_v4();
    let mut owner = CompletionActionCoordinator::default();
    owner.register_completion_members(&snapshot(vec![task(
        1,
        Some(group_id),
        0,
        DownloadStatus::Created,
    )]));
    owner.register_completion_members(&snapshot(vec![task(
        1,
        Some(group_id),
        1,
        DownloadStatus::Downloading,
    )]));
    assert!(owner.is_grouped_attempt(id(1), 1));
    assert!(!owner.is_grouped_attempt(id(1), 0));
    owner.record_terminal_attempt(terminal(1, DownloadStatus::Failed), 1);
    owner.settle_completion_members(&snapshot(vec![task(
        1,
        Some(group_id),
        1,
        DownloadStatus::Failed,
    )]));
    let group_feedback = owner.take_ready_group_feedback(false);
    assert_eq!(group_feedback[0].0, group_id);
    assert_eq!(group_feedback[0].1.failed_count, 1);
    owner.register_completion_members(&snapshot(vec![task(
        1,
        Some(group_id),
        2,
        DownloadStatus::Downloading,
    )]));
    assert!(!owner.is_grouped_attempt(id(1), 2));
    owner.record_terminal_attempt(terminal(1, DownloadStatus::Completed), 2);
    assert_eq!(
        owner
            .take_feedback_batch_for_directory_policy(false)
            .expect("retry feedback")
            .completed_count,
        1
    );
    assert!(owner.take_ready_group_feedback(false).is_empty());
}

#[test]
fn removal_empty_history_do_not_replay_or_change_group_authority() {
    let group_id = uuid::Uuid::new_v4();
    let baseline = snapshot(vec![task(1, Some(group_id), 0, DownloadStatus::Completed)]);
    let mut owner = CompletionActionCoordinator::from_snapshot(&baseline);
    owner.register_completion_members(&baseline);
    owner.settle_completion_members(&baseline);
    assert!(owner.take_ready_group_feedback(false).is_empty());
    owner.register_completion_members(&snapshot(Vec::new()));
    assert!(owner.groups.is_empty());
}
