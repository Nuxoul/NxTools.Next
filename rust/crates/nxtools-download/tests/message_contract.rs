use std::path::PathBuf;

use nxtools_download::{
    DOWNLOAD_COMMAND_PROTOCOL_VERSION, DownloadAggregateMetrics, DownloadCommand,
    DownloadCommandEnvelope, DownloadCommandOutcome, DownloadEvent, DownloadEventEnvelope,
    DownloadEventSequencer, DownloadFault, DownloadFaultCode, DownloadProbe, DownloadQueue,
    DownloadRequest, DownloadRequestHeader, DownloadSnapshot, DownloadSubmitMode,
    DownloadTaskAction, DownloadTaskId, RequestOrigin, ResourceValidator,
};
use serde_json::json;
use uuid::Uuid;

fn request() -> DownloadRequest {
    DownloadRequest {
        source_url: "https://example.test/file.bin".to_owned(),
        save_directory: PathBuf::from(r"D:\Downloads"),
        file_name: None,
        headers: vec![DownloadRequestHeader {
            name: "Accept".to_owned(),
            value: "*/*".to_owned(),
        }],
        cookie: None,
        referrer: Some("https://example.test/".to_owned()),
        connection_limit: Some(4),
        speed_limit_bytes_per_second: None,
        scheduled_at_unix_ms: None,
    }
}

#[test]
fn every_command_carries_a_correlation_id_origin_and_version() {
    let request_id =
        Uuid::parse_str("12345678-1234-5678-1234-567812345678").expect("fixed request id is valid");
    let envelope = DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id,
        origin: RequestOrigin::Cli,
        command: DownloadCommand::Submit {
            request: request(),
            mode: DownloadSubmitMode::QueueOnly,
        },
    };

    let value = serde_json::to_value(envelope).expect("serialize command");

    assert_eq!(value["protocol_version"], json!(1));
    assert_eq!(value["request_id"], json!(request_id));
    assert_eq!(value["origin"], json!("cli"));
    assert_eq!(value["command"], json!("submit"));
    assert_eq!(value["data"]["mode"], json!("queue_only"));
    assert_eq!(
        value["data"]["request"]["source_url"],
        json!("https://example.test/file.bin")
    );
}

#[test]
fn task_action_contract_does_not_expose_protocol_implementation() {
    let task_id = DownloadTaskId::from_uuid(
        Uuid::parse_str("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee").expect("fixed task id is valid"),
    );
    let command = DownloadCommand::Task {
        task_id,
        action: DownloadTaskAction::Remove {
            delete_artifact: true,
        },
    };

    let value = serde_json::to_value(command).expect("serialize task action");

    assert_eq!(value["command"], json!("task"));
    assert_eq!(value["data"]["action"]["action"], json!("remove"));
    assert_eq!(
        value["data"]["action"]["data"]["delete_artifact"],
        json!(true)
    );
    assert!(value.to_string().find("http_client").is_none());
    assert!(value.to_string().find("segment_writer").is_none());
}

#[test]
fn speed_limit_commands_are_typed_and_use_none_for_unlimited() {
    let global = serde_json::to_value(DownloadCommand::SetGlobalSpeedLimit {
        bytes_per_second: Some(512 * 1024),
    })
    .expect("serialize global speed limit");
    let task = serde_json::to_value(DownloadCommand::Task {
        task_id: DownloadTaskId::new(),
        action: DownloadTaskAction::SetSpeedLimit {
            bytes_per_second: None,
        },
    })
    .expect("serialize task speed limit");

    assert_eq!(global["command"], json!("set_global_speed_limit"));
    assert_eq!(global["data"]["bytes_per_second"], json!(512 * 1024));
    assert_eq!(task["data"]["action"]["action"], json!("set_speed_limit"));
    assert_eq!(
        task["data"]["action"]["data"]["bytes_per_second"],
        json!(null)
    );
}

#[test]
fn events_are_sequence_numbered_and_faults_are_machine_readable() {
    let event = DownloadEventEnvelope {
        sequence: 41,
        emitted_at_unix_ms: 9_000,
        event: DownloadEvent::CommandRejected {
            request_id: Uuid::nil(),
            fault: DownloadFault {
                code: DownloadFaultCode::InvalidState,
                message: "task is already complete".to_owned(),
                retryable: false,
            },
        },
    };

    let value = serde_json::to_value(event).expect("serialize event");

    assert_eq!(value["sequence"], json!(41));
    assert_eq!(value["event"], json!("command_rejected"));
    assert_eq!(value["data"]["fault"]["code"], json!("invalid_state"));
    assert_eq!(value["data"]["fault"]["retryable"], json!(false));
}

#[test]
fn probe_outcome_returns_metadata_without_creating_a_task() {
    let outcome = DownloadCommandOutcome::Probe(DownloadProbe {
        final_url: "https://cdn.example.test/file.bin".to_owned(),
        file_name: Some("file.bin".to_owned()),
        total_bytes: Some(16),
        supports_ranges: true,
        content_type: Some("application/octet-stream".to_owned()),
        validator: ResourceValidator {
            etag: Some("\"v1\"".to_owned()),
            last_modified: None,
        },
    });

    let value = serde_json::to_value(outcome).expect("serialize probe outcome");

    assert_eq!(value["outcome"], json!("probe"));
    assert_eq!(value["data"]["file_name"], json!("file.bin"));
    assert_eq!(value["data"]["total_bytes"], json!(16));
    assert!(value["data"].get("task_id").is_none());
}

#[test]
fn snapshot_contains_queues_and_uses_the_frozen_metrics_name() {
    let snapshot = DownloadSnapshot {
        generation: 7,
        sequence: 12,
        tasks: Vec::new(),
        queues: vec![DownloadQueue {
            id: "main".to_owned(),
            name: "主队列".to_owned(),
            position: 0,
            max_concurrent_tasks: Some(4),
            connection_limit: Some(8),
            speed_limit_bytes_per_second: None,
            default_save_directory: None,
            schedule: None,
        }],
        aggregate_metrics: DownloadAggregateMetrics::default(),
        task_details: Vec::new(),
    };

    let value = serde_json::to_value(snapshot).expect("serialize snapshot");

    assert_eq!(value["queues"][0]["id"], json!("main"));
    assert!(value.get("aggregate_metrics").is_some());
    assert!(value.get("aggregate").is_none());
}

#[test]
fn event_sequencer_produces_strictly_increasing_values() {
    let sequencer = DownloadEventSequencer::new(40);
    let first = sequencer
        .next(
            1_000,
            DownloadEvent::TaskRemoved {
                task_id: DownloadTaskId::new(),
            },
        )
        .expect("first sequence exists");
    let second = sequencer
        .next(
            1_001,
            DownloadEvent::TaskRemoved {
                task_id: DownloadTaskId::new(),
            },
        )
        .expect("second sequence exists");

    assert_eq!(first.sequence, 41);
    assert_eq!(second.sequence, 42);
    assert!(second.sequence > first.sequence);
}
