use std::path::PathBuf;

use nxtools_download::{
    DownloadAdaptiveSegmentPolicy, DownloadArtifact, DownloadArtifactKind, DownloadArtifactState,
    DownloadAttempt, DownloadAttemptError, DownloadAttemptErrorCategory, DownloadDailySchedule,
    DownloadQueue, DownloadRequest, DownloadRequestError, DownloadRequestHeader, DownloadSchedule,
    DownloadSegment, DownloadSegmentState, DownloadStatus, DownloadTask, DownloadTaskError,
    DownloadWeekday, RequestOrigin, ResourceValidator, TransferPlan, TransferPlanError,
    TransferProtocol,
};

fn request() -> DownloadRequest {
    DownloadRequest {
        source_url: "https://example.test/archive.zip".to_owned(),
        save_directory: PathBuf::from(r"D:\Downloads"),
        save_directory_explicit: false,
        file_name: Some("archive.zip".to_owned()),
        headers: Vec::new(),
        cookie: None,
        referrer: None,
        source_page: None,
        certificate_policy: Default::default(),
        connection_limit: Some(8),
        speed_limit_bytes_per_second: None,
        expected_total_bytes: None,
        scheduled_at_unix_ms: None,
        adaptive_segment_policy: None,
        media: None,
        category_id: None,
        category_name: None,
        duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
    }
}

#[test]
fn task_lifecycle_keeps_scheduling_separate_from_transfer_state() {
    let mut task = DownloadTask::new(request(), RequestOrigin::Ui, 1_000);

    task.transition_to(DownloadStatus::Probing, 1_100)
        .expect("created task can enter probing");
    task.transition_to(DownloadStatus::Waiting, 1_200)
        .expect("probed task can wait for admission");
    task.transition_to(DownloadStatus::Downloading, 1_300)
        .expect("waiting task can begin downloading");
    task.transition_to(DownloadStatus::Finalizing, 1_400)
        .expect("downloaded bytes can enter finalization");
    task.transition_to(DownloadStatus::Verifying, 1_500)
        .expect("finalized output can be verified");
    task.transition_to(DownloadStatus::Completed, 1_600)
        .expect("verified output can complete");

    assert_eq!(task.status(), DownloadStatus::Completed);
    assert_eq!(task.updated_at_unix_ms(), 1_600);
    assert_eq!(task.completed_at_unix_ms(), Some(1_600));
    assert_eq!(task.request().scheduled_at_unix_ms, None);
}

#[test]
fn completed_task_is_terminal() {
    let mut task = DownloadTask::new(request(), RequestOrigin::Cli, 1_000);
    for (status, at) in [
        (DownloadStatus::Probing, 1_100),
        (DownloadStatus::Waiting, 1_200),
        (DownloadStatus::Downloading, 1_300),
        (DownloadStatus::Finalizing, 1_400),
        (DownloadStatus::Completed, 1_500),
    ] {
        task.transition_to(status, at)
            .expect("valid setup transition");
    }

    let error = task
        .transition_to(DownloadStatus::Downloading, 1_600)
        .expect_err("completed task cannot restart");

    assert_eq!(
        error,
        DownloadTaskError::InvalidTransition {
            from: DownloadStatus::Completed,
            to: DownloadStatus::Downloading,
        }
    );
}

#[test]
fn interrupted_execution_recovers_as_paused() {
    for interrupted_status in [
        DownloadStatus::Probing,
        DownloadStatus::Downloading,
        DownloadStatus::Finalizing,
        DownloadStatus::Verifying,
    ] {
        let mut task = DownloadTask::new(request(), RequestOrigin::Browser, 1_000);
        move_task_to(&mut task, interrupted_status);

        assert!(task.recover_interrupted(2_000));
        assert_eq!(task.status(), DownloadStatus::Paused);
        assert_eq!(task.updated_at_unix_ms(), 2_000);
    }

    let mut waiting = DownloadTask::new(request(), RequestOrigin::Ui, 1_000);
    move_task_to(&mut waiting, DownloadStatus::Waiting);
    assert!(!waiting.recover_interrupted(2_000));
    assert_eq!(waiting.status(), DownloadStatus::Waiting);
}

#[test]
fn first_release_request_validation_accepts_only_safe_http_inputs() {
    let valid = request();
    valid.validate().expect("HTTP request is valid");

    let mut unsupported = request();
    unsupported.source_url = "ftp://example.test/archive.zip".to_owned();
    assert_eq!(
        unsupported.validate(),
        Err(DownloadRequestError::UnsupportedScheme("ftp".to_owned()))
    );

    let mut invalid_header = request();
    invalid_header.headers.push(DownloadRequestHeader {
        name: "X-Test".to_owned(),
        value: "safe\r\nInjected: yes".to_owned(),
    });
    assert_eq!(
        invalid_header.validate(),
        Err(DownloadRequestError::InvalidHeader)
    );

    let mut invalid_connections = request();
    invalid_connections.connection_limit = Some(0);
    assert_eq!(
        invalid_connections.validate(),
        Err(DownloadRequestError::InvalidConnectionLimit(0))
    );

    let mut invalid_speed_limit = request();
    invalid_speed_limit.speed_limit_bytes_per_second = Some(0);
    assert_eq!(
        invalid_speed_limit.validate(),
        Err(DownloadRequestError::InvalidSpeedLimit)
    );

    for invalid_name in ["CON", "report?.txt", "trailing.", "trailing ", "a:b.txt"] {
        let mut invalid_file_name = request();
        invalid_file_name.file_name = Some(invalid_name.to_owned());
        assert_eq!(
            invalid_file_name.validate(),
            Err(DownloadRequestError::InvalidFileName),
            "{invalid_name} must be rejected on Windows"
        );
    }
}

#[test]
fn request_headers_preserve_order_and_duplicate_names() {
    let mut request = request();
    request.headers = vec![
        DownloadRequestHeader {
            name: "X-Trace".to_owned(),
            value: "first".to_owned(),
        },
        DownloadRequestHeader {
            name: "X-Trace".to_owned(),
            value: "second".to_owned(),
        },
    ];

    let json = serde_json::to_string(&request).expect("serialize request");
    let restored: DownloadRequest = serde_json::from_str(&json).expect("deserialize request");

    assert_eq!(restored.headers, request.headers);
}

#[test]
fn adaptive_segment_policy_is_optional_and_rejects_unsafe_ranges() {
    let mut too_few_segments = request();
    too_few_segments.adaptive_segment_policy = Some(DownloadAdaptiveSegmentPolicy {
        enabled: true,
        max_segments: 7,
        min_segment_mib: 3,
    });
    assert_eq!(
        too_few_segments.validate(),
        Err(DownloadRequestError::InvalidAdaptiveMaxSegments(7))
    );

    let mut invalid_minimum = request();
    invalid_minimum.adaptive_segment_policy = Some(DownloadAdaptiveSegmentPolicy {
        enabled: true,
        max_segments: 32,
        min_segment_mib: 0,
    });
    assert_eq!(
        invalid_minimum.validate(),
        Err(DownloadRequestError::InvalidAdaptiveMinSegmentMib(0))
    );

    let legacy_json = serde_json::json!({
        "source_url": "https://example.test/archive.zip",
        "save_directory": "D:\\Downloads",
        "file_name": "archive.zip",
        "headers": [],
        "cookie": null,
        "referrer": null,
        "connection_limit": 8,
        "speed_limit_bytes_per_second": null,
        "scheduled_at_unix_ms": null
    });
    let restored: DownloadRequest =
        serde_json::from_value(legacy_json).expect("legacy request remains readable");
    assert_eq!(restored.adaptive_segment_policy, None);
}

#[test]
fn remaining_phase_zero_domain_types_have_explicit_meaning() {
    let segment = DownloadSegment::new(0, 0, 15, 4, DownloadSegmentState::Active)
        .expect("valid inclusive byte range");
    let plan = TransferPlan::new(
        TransferProtocol::Http,
        "https://cdn.example.test/file.bin".to_owned(),
        Some(16),
        true,
        ResourceValidator {
            etag: Some("\"v1\"".to_owned()),
            last_modified: None,
        },
        vec![segment.clone()],
    )
    .expect("valid transfer plan");
    let queue = DownloadQueue {
        id: "main".to_owned(),
        name: "主队列".to_owned(),
        position: 0,
        max_concurrent_tasks: Some(4),
        connection_limit: Some(8),
        speed_limit_bytes_per_second: None,
        default_save_directory: Some(PathBuf::from(r"D:\Downloads")),
        schedule: None,
        is_running: true,
        completion_intention: Default::default(),
    };
    let schedule = DownloadSchedule {
        daily: DownloadDailySchedule {
            days: vec![DownloadWeekday::Monday],
            start_minute: 9 * 60,
            stop_minute: 17 * 60,
            enabled_start: true,
            enabled_stop: true,
        },
    };
    let attempt = DownloadAttempt {
        number: 1,
        started_at_unix_ms: 1_000,
        finished_at_unix_ms: None,
        error: Some(DownloadAttemptError {
            category: DownloadAttemptErrorCategory::Network,
            message: "connection reset".to_owned(),
            retryable: true,
        }),
        retry_at_unix_ms: Some(2_000),
    };
    let artifact = DownloadArtifact {
        id: "artifact-1".to_owned(),
        kind: DownloadArtifactKind::FinalFile,
        state: DownloadArtifactState::Pending,
        path: PathBuf::from(r"D:\Downloads\file.bin"),
    };

    assert_eq!(plan.total_bytes(), Some(segment.len()));
    assert_eq!(segment.downloaded_bytes(), 4);
    assert_eq!(queue.id, "main");
    assert_eq!(schedule.daily.start_minute, 9 * 60);
    assert_eq!(attempt.number, 1);
    assert_eq!(attempt.retry_at_unix_ms, Some(2_000));
    assert_eq!(artifact.state, DownloadArtifactState::Pending);
}

#[test]
fn transfer_plan_rejects_overlapping_or_out_of_bounds_segments() {
    let overlapping = TransferPlan::new(
        TransferProtocol::Http,
        "https://cdn.example.test/file.bin".to_owned(),
        Some(16),
        true,
        ResourceValidator::default(),
        vec![
            DownloadSegment::new(0, 0, 7, 0, DownloadSegmentState::Waiting).expect("valid segment"),
            DownloadSegment::new(1, 7, 15, 0, DownloadSegmentState::Waiting)
                .expect("valid segment"),
        ],
    );
    assert_eq!(
        overlapping,
        Err(TransferPlanError::OverlappingSegments {
            previous_index: 0,
            next_index: 1,
        })
    );

    let out_of_bounds = TransferPlan::new(
        TransferProtocol::Http,
        "https://cdn.example.test/file.bin".to_owned(),
        Some(16),
        true,
        ResourceValidator::default(),
        vec![
            DownloadSegment::new(0, 0, 16, 0, DownloadSegmentState::Waiting)
                .expect("valid standalone segment"),
        ],
    );
    assert_eq!(
        out_of_bounds,
        Err(TransferPlanError::SegmentOutOfBounds {
            segment_index: 0,
            end_byte: 16,
            total_bytes: 16,
        })
    );

    let invalid_json = serde_json::json!({
        "protocol": "http",
        "final_url": "https://cdn.example.test/file.bin",
        "total_bytes": 16,
        "supports_ranges": true,
        "validator": {},
        "segments": [
            {
                "index": 0,
                "start_byte": 0,
                "end_byte": 7,
                "downloaded_bytes": 0,
                "state": "waiting"
            },
            {
                "index": 1,
                "start_byte": 7,
                "end_byte": 15,
                "downloaded_bytes": 0,
                "state": "waiting"
            }
        ]
    });
    assert!(
        serde_json::from_value::<TransferPlan>(invalid_json).is_err(),
        "deserialization must not bypass transfer plan validation"
    );
}

#[test]
fn transfer_plan_splits_only_the_unwritten_tail_after_a_durable_checkpoint() {
    let mut plan = TransferPlan::new(
        TransferProtocol::Http,
        "https://cdn.example.test/large.bin".to_owned(),
        Some(1_024),
        true,
        ResourceValidator::default(),
        vec![
            DownloadSegment::new(0, 0, 1_023, 256, DownloadSegmentState::Waiting)
                .expect("valid checkpoint"),
        ],
    )
    .expect("valid transfer plan");

    plan.split_segment_after_checkpoint(0, 1, 128)
        .expect("split durable tail");

    assert_eq!(plan.segments().len(), 2);
    assert_eq!(
        plan.segments()[0],
        DownloadSegment::new(0, 0, 639, 256, DownloadSegmentState::Waiting)
            .expect("valid reduced segment")
    );
    assert_eq!(
        plan.segments()[1],
        DownloadSegment::new(1, 640, 1_023, 0, DownloadSegmentState::Waiting)
            .expect("valid new segment")
    );
}

#[test]
fn synced_shrink_merges_a_waiting_tail_into_the_surviving_waiting_range() {
    let mut plan = TransferPlan::new(
        TransferProtocol::Http,
        "https://cdn.example.test/large.bin".to_owned(),
        Some(1024),
        true,
        ResourceValidator::default(),
        vec![
            DownloadSegment::new(0, 0, 639, 256, DownloadSegmentState::Waiting)
                .expect("synced left prefix"),
            DownloadSegment::new(1, 640, 1023, 0, DownloadSegmentState::Waiting)
                .expect("unwritten right tail"),
        ],
    )
    .expect("durable split plan");
    plan.merge_waiting_segment(1, 0)
        .expect("merge synced waiting worker");
    assert_eq!(
        plan.segments(),
        &[
            DownloadSegment::new(0, 0, 1023, 256, DownloadSegmentState::Waiting)
                .expect("expected merged prefix")
        ]
    );
}

#[test]
fn synced_shrink_rejects_prepend_to_a_progressing_right_range() {
    let mut plan = TransferPlan::new(
        TransferProtocol::Http,
        "https://cdn.example.test/large.bin".to_owned(),
        Some(1024),
        true,
        ResourceValidator::default(),
        vec![
            DownloadSegment::new(0, 0, 639, 0, DownloadSegmentState::Waiting)
                .expect("unwritten left range"),
            DownloadSegment::new(1, 640, 1023, 128, DownloadSegmentState::Waiting)
                .expect("synced right prefix"),
        ],
    )
    .expect("durable split plan");
    assert!(
        plan.merge_waiting_segment(0, 1).is_err(),
        "a right-hand checkpoint cannot become a prefix of the combined range"
    );
    assert_eq!(plan.segments().len(), 2);
}

#[test]
fn deserialization_cannot_bypass_task_or_segment_invariants() {
    let task = DownloadTask::new(request(), RequestOrigin::Ui, 1_000);
    let mut task_json = serde_json::to_value(task).expect("serialize task");
    task_json["status"] = serde_json::json!("completed");
    task_json["completed_at_unix_ms"] = serde_json::Value::Null;
    assert!(
        serde_json::from_value::<DownloadTask>(task_json).is_err(),
        "completed task without completion timestamp must be rejected"
    );

    let invalid_segment = serde_json::json!({
        "index": 0,
        "start_byte": 10,
        "end_byte": 5,
        "downloaded_bytes": 0,
        "state": "waiting"
    });
    assert!(
        serde_json::from_value::<DownloadSegment>(invalid_segment).is_err(),
        "invalid byte range must not bypass DownloadSegment::new"
    );
}

fn move_task_to(task: &mut DownloadTask, target: DownloadStatus) {
    let path: &[DownloadStatus] = match target {
        DownloadStatus::Created => &[],
        DownloadStatus::Probing => &[DownloadStatus::Probing],
        DownloadStatus::Waiting => &[DownloadStatus::Probing, DownloadStatus::Waiting],
        DownloadStatus::Downloading => &[
            DownloadStatus::Probing,
            DownloadStatus::Waiting,
            DownloadStatus::Downloading,
        ],
        DownloadStatus::Finalizing => &[
            DownloadStatus::Probing,
            DownloadStatus::Waiting,
            DownloadStatus::Downloading,
            DownloadStatus::Finalizing,
        ],
        DownloadStatus::Verifying => &[
            DownloadStatus::Probing,
            DownloadStatus::Waiting,
            DownloadStatus::Downloading,
            DownloadStatus::Finalizing,
            DownloadStatus::Verifying,
        ],
        _ => panic!("test helper does not support target {target:?}"),
    };

    for (offset, status) in path.iter().copied().enumerate() {
        task.transition_to(status, 1_100 + offset as i64)
            .expect("test transition is valid");
    }
}
