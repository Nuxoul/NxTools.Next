use std::path::PathBuf;

use nxtools_download::{
    BrowserDownloadSubmission, DOWNLOAD_COMMAND_PROTOCOL_VERSION, DashDownloadRequest,
    DownloadAggregateMetrics, DownloadBatchAction, DownloadCommand, DownloadCommandEnvelope,
    DownloadCommandOutcome, DownloadEvent, DownloadEventEnvelope, DownloadEventSequencer,
    DownloadFault, DownloadFaultCode, DownloadProbe, DownloadQuery, DownloadQueue,
    DownloadQueueAction, DownloadQueuePlacement, DownloadRequest, DownloadRequestHeader,
    DownloadRequestMedia, DownloadSnapshot, DownloadSubmitMode, DownloadTaskAction, DownloadTaskId,
    HlsDownloadRequest, RequestOrigin, ResourceValidator,
};
use serde_json::json;
use uuid::Uuid;

fn request() -> DownloadRequest {
    DownloadRequest {
        source_url: "https://example.test/file.bin".to_owned(),
        save_directory: PathBuf::from(r"D:\Downloads"),
        save_directory_explicit: false,
        file_name: None,
        headers: vec![DownloadRequestHeader {
            name: "Accept".to_owned(),
            value: "*/*".to_owned(),
        }],
        cookie: None,
        referrer: Some("https://example.test/".to_owned()),
        certificate_policy: Default::default(),
        connection_limit: Some(4),
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
fn hls_selection_round_trips_as_typed_download_request_state() {
    let mut request = request();
    request.source_url = "https://example.test/master.m3u8".into();
    request.media = Some(DownloadRequestMedia::Hls(HlsDownloadRequest {
        playlist_url: "https://cdn.example.test/1080p/index.m3u8".into(),
    }));
    let command = DownloadCommand::Submit {
        request,
        mode: DownloadSubmitMode::StartNow,
        idempotency_key: None,
    };

    let encoded = serde_json::to_value(&command).expect("serialize typed HLS request");
    let decoded: DownloadCommand =
        serde_json::from_value(encoded.clone()).expect("deserialize typed HLS request");

    assert_eq!(encoded["data"]["request"]["media"]["kind"], json!("hls"));
    assert_eq!(
        encoded["data"]["request"]["media"]["data"]["playlist_url"],
        json!("https://cdn.example.test/1080p/index.m3u8")
    );
    assert_eq!(decoded, command);
}

#[test]
fn idempotent_submit_command_round_trips_its_business_key() {
    let key = Uuid::parse_str("12345678-1234-5678-1234-567812345678")
        .expect("fixed idempotency key is valid");
    let command = DownloadCommand::Submit {
        idempotency_key: Some(key),
        request: request(),
        mode: DownloadSubmitMode::QueueOnly,
    };
    let encoded = serde_json::to_value(&command).expect("serialize idempotent submit");
    let decoded: DownloadCommand =
        serde_json::from_value(encoded.clone()).expect("deserialize idempotent submit");

    assert_eq!(encoded["command"], json!("submit"));
    assert_eq!(encoded["data"]["idempotency_key"], json!(key));
    assert_eq!(encoded["data"]["mode"], json!("queue_only"));
    assert_eq!(decoded, command);
}

#[test]
fn dash_track_selection_round_trips_as_typed_download_request_state() {
    let mut request = request();
    request.source_url = "https://media.example.test/manifest.mpd".into();
    request.media = Some(DownloadRequestMedia::Dash(DashDownloadRequest {
        manifest_url: "https://cdn.example.test/vod/manifest.mpd?token=fresh".into(),
        video_representation_id: Some("video-1080".into()),
        audio_representation_id: Some("audio-zh-128".into()),
    }));
    let command = DownloadCommand::Submit {
        request,
        mode: DownloadSubmitMode::StartNow,
        idempotency_key: None,
    };

    let encoded = serde_json::to_value(&command).expect("serialize typed DASH request");
    let decoded: DownloadCommand =
        serde_json::from_value(encoded.clone()).expect("deserialize typed DASH request");

    assert_eq!(encoded["data"]["request"]["media"]["kind"], json!("dash"));
    assert_eq!(
        encoded["data"]["request"]["media"]["data"]["video_representation_id"],
        json!("video-1080")
    );
    assert_eq!(
        encoded["data"]["request"]["media"]["data"]["audio_representation_id"],
        json!("audio-zh-128")
    );
    assert_eq!(decoded, command);
}

#[test]
fn queue_and_batch_commands_are_typed_and_preserve_target_order() {
    let first = DownloadTaskId::new();
    let second = DownloadTaskId::new();
    let batch = serde_json::to_value(DownloadCommand::Batch {
        task_ids: vec![first, second],
        action: DownloadBatchAction::Pause,
    })
    .expect("serialize batch command");
    let queue = serde_json::to_value(DownloadCommand::Queue {
        action: DownloadQueueAction::Move {
            queue_id: "default".into(),
            task_ids: vec![first, second],
            placement: DownloadQueuePlacement::Top,
        },
    })
    .expect("serialize queue command");

    assert_eq!(batch["command"], json!("batch"));
    assert_eq!(batch["data"]["task_ids"][0], json!(first));
    assert_eq!(batch["data"]["task_ids"][1], json!(second));
    assert_eq!(batch["data"]["action"], json!("pause"));
    assert_eq!(queue["command"], json!("queue"));
    assert_eq!(queue["data"]["action"]["action"], json!("move"));
    assert_eq!(queue["data"]["action"]["data"]["placement"], json!("top"));
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
            idempotency_key: None,
        },
    };
    let value = serde_json::to_value(envelope).expect("serialize command");

    assert_eq!(value["protocol_version"], json!(8));
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
fn legacy_submit_json_without_idempotency_key_still_decodes() {
    let command = DownloadCommand::Submit {
        request: request(),
        mode: DownloadSubmitMode::QueueOnly,
        idempotency_key: None,
    };
    let mut value = serde_json::to_value(&command).expect("serialize legacy-compatible submit");
    value["data"]
        .as_object_mut()
        .expect("submit data object")
        .remove("idempotency_key");
    let decoded: DownloadCommand =
        serde_json::from_value(value).expect("decode submit without idempotency key");
    assert_eq!(decoded, command);
}

#[test]
fn browser_batch_command_round_trips_stable_item_identities_in_queue_only_mode() {
    let handoff_ids = [Uuid::new_v4(), Uuid::new_v4()];
    let command = DownloadCommand::BrowserBatchSubmit {
        submissions: handoff_ids
            .into_iter()
            .map(|handoff_id| BrowserDownloadSubmission {
                handoff_id,
                request: request(),
            })
            .collect(),
    };

    let encoded = serde_json::to_value(&command).expect("serialize browser batch command");
    let decoded: DownloadCommand =
        serde_json::from_value(encoded.clone()).expect("deserialize browser batch command");
    let mut forged_start_mode = encoded.clone();
    forged_start_mode["data"]["mode"] = json!("start_now");
    let decoded_forged: DownloadCommand = serde_json::from_value(forged_start_mode)
        .expect("ignore unsupported browser batch start mode");

    assert_eq!(encoded["command"], json!("browser_batch_submit"));
    assert!(encoded["data"].get("mode").is_none());
    assert_eq!(
        encoded["data"]["submissions"][0]["handoff_id"],
        json!(handoff_ids[0])
    );
    assert_eq!(decoded, command);
    assert_eq!(decoded_forged, command);
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
fn retry_and_redownload_actions_round_trip_as_additive_wire_variants() {
    for (action, name) in [
        (DownloadTaskAction::Retry, "retry"),
        (DownloadTaskAction::Redownload, "redownload"),
    ] {
        let command = DownloadCommand::Task {
            task_id: DownloadTaskId::new(),
            action,
        };
        let value = serde_json::to_value(&command).expect("serialize explicit task action");
        let decoded: DownloadCommand =
            serde_json::from_value(value.clone()).expect("deserialize explicit task action");
        assert_eq!(value["data"]["action"]["action"], json!(name));
        assert_eq!(decoded, command);
    }
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
        generation: 11,
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
        media: None,
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
            is_running: true,
        }],
        aggregate_metrics: DownloadAggregateMetrics::default(),
        task_details: Vec::new(),
        task_summaries: Vec::new(),
    };

    let value = serde_json::to_value(snapshot).expect("serialize snapshot");

    assert_eq!(value["queues"][0]["id"], json!("main"));
    assert!(value.get("aggregate_metrics").is_some());
    assert!(value.get("aggregate").is_none());
}
#[test]
fn compact_query_is_opt_in_and_legacy_snapshot_json_stays_decodable() {
    assert!(!DownloadQuery::default().compact);
    assert!(DownloadQuery::compact().compact);
    let legacy = serde_json::json!({
        "generation": 1,
        "sequence": 2,
        "tasks": [],
        "queues": [],
        "aggregate_metrics": {
            "active_tasks": 0,
            "downloaded_bytes_per_second": 0,
            "disk_write_bytes_per_second": 0,
            "active_connections": 0
        }
    });
    let decoded: DownloadSnapshot =
        serde_json::from_value(legacy).expect("legacy snapshot remains decodable");
    assert!(decoded.task_summaries.is_empty());
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
