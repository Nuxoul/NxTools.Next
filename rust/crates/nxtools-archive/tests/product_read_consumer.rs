use nxtools_archive::product::ArchiveReadClient;
use nxtools_ipc::{PlatformNotification, PlatformServerEndpoint, platform_bridge};
use nxtools_protocol::{PlatformEvent, archive_read::*};
use std::sync::Arc;
use uuid::Uuid;
fn request() -> ArchiveReadCommand {
    ArchiveReadCommand {
        version: 1,
        operation_id: Uuid::new_v4(),
        session_id: Uuid::new_v4(),
        generation: 2,
        action: ArchiveReadAction::Open,
        path: Some("C:\\owned\\synthetic.zip".into()),
        codepage: 0,
        limits: None,
        sequence: None,
    }
}
fn event(command: &ArchiveReadCommand, kind: ArchiveReadEventKind) -> ArchiveReadEvent {
    ArchiveReadEvent {
        version: command.version,
        operation_id: command.operation_id,
        session_id: command.session_id,
        generation: command.generation,
        kind,
        capabilities: Vec::new(),
        batch: None,
        terminal: None,
    }
}
async fn send(
    endpoint: &PlatformServerEndpoint,
    client: &ArchiveReadClient,
    event: ArchiveReadEvent,
) {
    endpoint
        .send_notification_for_contract_test(PlatformNotification::Event(
            PlatformEvent::ArchiveRead(event),
        ))
        .expect("same bounded production channel");
    client.poll().await.expect("consumer turn");
}
async fn negotiate(
    endpoint: &PlatformServerEndpoint,
    client: &ArchiveReadClient,
    command: &ArchiveReadCommand,
) {
    send(
        endpoint,
        client,
        event(command, ArchiveReadEventKind::Accepted),
    )
    .await;
    let mut negotiated = event(command, ArchiveReadEventKind::Negotiated);
    negotiated.capabilities = archive_read_capabilities(command.version)
        .iter()
        .map(|v| (*v).into())
        .collect();
    send(endpoint, client, negotiated).await;
}
fn batch(command: &ArchiveReadCommand) -> ArchiveReadEvent {
    let mut value = event(command, ArchiveReadEventKind::Batch);
    value.batch = Some(ArchiveReadBatch {
        snapshot_id: Uuid::new_v4(),
        sequence: 0,
        format: "Zip".into(),
        entries: vec![ArchiveReadEntry {
            ordinal: 0,
            name: "literal.txt".into(),
            kind: "File".into(),
            size: Some(7),
            packed_size: Some(7),
            encrypted: Some(false),
            warning_codes: Vec::new(),
            provenance: "Utf8Flag".into(),
            codepage: Some(65001),
            modified: None,
            method: None,
            crc32: None,
            attributes: None,
        }],
    });
    value
}
fn complete(command: &ArchiveReadCommand) -> ArchiveReadEvent {
    let mut value = event(command, ArchiveReadEventKind::Terminal);
    value.terminal = Some(ArchiveReadTerminal {
        status: ArchiveReadStatus::Succeeded,
        code: None,
        files_verified: 1,
        bytes_verified: 0,
        entry_count: 1,
        batch_count: 1,
        elapsed_ms: 1,
        native_error_code: None,
        cleanup_complete: true,
        transfer: None,
        browser: None,
    });
    value
}
#[tokio::test]
async fn metadata_credit_is_exact_after_move_and_terminal_is_once_only() {
    let (bridge, endpoint) = platform_bridge(32);
    endpoint.set_connected_for_contract_test(true);
    let client = ArchiveReadClient::new(Arc::new(bridge)).expect("owner");
    let command = request();
    let operation = client.start(command.clone()).expect("pending installed");
    let start = endpoint
        .try_receive_command_for_contract_test()
        .expect("typed start");
    assert_eq!(start.correlation_id, Some(command.operation_id));
    negotiate(&endpoint, &client, &command).await;
    send(&endpoint, &client, batch(&command)).await;
    assert_eq!(
        operation.acknowledge(0),
        Err(ArchiveReadError::Busy),
        "retained batch must be consumed first"
    );
    let first = operation.take_batch().expect("one bounded batch");
    assert_eq!(first.entries[0].name, "literal.txt");
    assert_eq!(
        operation.acknowledge(1),
        Err(ArchiveReadError::ProtocolViolation)
    );
    operation.acknowledge(0).expect("exact credit");
    assert_eq!(
        operation.acknowledge(0),
        Err(ArchiveReadError::ProtocolViolation)
    );
    let ack = endpoint
        .try_receive_command_for_contract_test()
        .expect("one typed ack");
    assert_ne!(ack.id, start.id);
    send(&endpoint, &client, complete(&command)).await;
    let original = operation.terminal().expect("terminal");
    let mut late = complete(&command);
    late.terminal.as_mut().expect("terminal").code = Some(ArchiveReadError::CorruptArchive);
    send(&endpoint, &client, late).await;
    assert_eq!(operation.terminal(), Some(original));
    assert!(client.start(command).is_err());
}
#[tokio::test]
async fn old_session_generation_and_retired_connection_cannot_write_new_owner() {
    let (bridge, endpoint) = platform_bridge(32);
    endpoint.set_connected_for_contract_test(true);
    let client = ArchiveReadClient::new(Arc::new(bridge)).expect("owner");
    let command = request();
    let operation = client.start(command.clone()).expect("start");
    let mut old = event(&command, ArchiveReadEventKind::Terminal);
    old.generation = 1;
    send(&endpoint, &client, old).await;
    assert!(operation.terminal().is_none());
    endpoint.set_connected_for_contract_test(false);
    client.poll().await.expect("disconnect");
    assert_eq!(
        operation.terminal().expect("original disconnect").code,
        Some(ArchiveReadError::Disconnected)
    );
    endpoint.set_connected_for_contract_test(true);
    let mut next = request();
    next.session_id = command.session_id;
    next.generation = 3;
    let replacement = client.start(next.clone()).expect("new generation");
    send(&endpoint, &client, complete(&command)).await;
    assert!(replacement.terminal().is_none());
    negotiate(&endpoint, &client, &next).await;
    replacement.cancel().expect("cancel once");
    replacement.cancel().expect("cancel twice idempotent");
}
#[tokio::test]
async fn invalid_requests_and_metadata_retention_fail_without_unbounded_memory() {
    let (bridge, endpoint) = platform_bridge(32);
    endpoint.set_connected_for_contract_test(true);
    let client = ArchiveReadClient::new(Arc::new(bridge)).expect("owner");
    let mut command = request();
    command.generation = 0;
    assert!(matches!(
        client.start(command.clone()),
        Err(ArchiveReadError::InvalidRequest)
    ));
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    command.generation = 1;
    command.limits = Some(ArchiveReadLimits {
        max_retained_bytes: 1,
        ..Default::default()
    });
    let operation = client.start(command.clone()).expect("bounded start");
    negotiate(&endpoint, &client, &command).await;
    send(&endpoint, &client, batch(&command)).await;
    assert_eq!(
        operation.terminal().expect("typed finite reject").code,
        Some(ArchiveReadError::MetadataBudget)
    );
    assert!(operation.take_batch().is_none());
}
#[tokio::test]
async fn full_product_event_channel_wakes_and_cancels_without_ui_receiver() {
    let (bridge, endpoint) = platform_bridge(32);
    endpoint.set_connected_for_contract_test(true);
    let client = ArchiveReadClient::new(Arc::new(bridge)).expect("owner");
    let command = request();
    let operation = client.start(command.clone()).expect("start");
    for _ in 0..20 {
        endpoint
            .send_notification_for_contract_test(PlatformNotification::Event(
                PlatformEvent::ArchiveRead(event(&command, ArchiveReadEventKind::Accepted)),
            ))
            .expect("nonblocking bounded injection");
    }
    assert_eq!(client.queued_events(), 8);
    client.poll().await.expect("overflow wake");
    assert_eq!(
        operation.terminal().expect("typed queue failure").code,
        Some(ArchiveReadError::ConsumerQueueFull)
    );
}

#[tokio::test]
async fn overflow_wakeup_cannot_retire_a_later_request_after_old_events_drain() {
    let (bridge, endpoint) = platform_bridge(32);
    endpoint.set_connected_for_contract_test(true);
    let client = ArchiveReadClient::new(Arc::new(bridge)).expect("owner");
    let command = request();
    let operation = client.start(command.clone()).expect("start");
    for _ in 0..9 {
        endpoint
            .send_notification_for_contract_test(PlatformNotification::Event(
                PlatformEvent::ArchiveRead(event(&command, ArchiveReadEventKind::Accepted)),
            ))
            .expect("bounded event");
    }
    client.poll().await.expect("overflow");
    assert_eq!(
        operation.terminal().expect("failure").code,
        Some(ArchiveReadError::ConsumerQueueFull)
    );
    assert_eq!(
        client.queued_events(),
        0,
        "overflow must drain old bounded events and its wake"
    );
    let next = request();
    let replacement = client
        .start(next.clone())
        .expect("new request after bounded drain");
    negotiate(&endpoint, &client, &next).await;
    assert!(
        replacement.terminal().is_none(),
        "old wake cannot fail a fresh operation"
    );
}

#[tokio::test]
async fn v2_missing_negotiation_and_cumulative_receive_after_ack_fail_closed() {
    let (bridge, endpoint) = platform_bridge(32);
    endpoint.set_connected_for_contract_test(true);
    let client = ArchiveReadClient::new(Arc::new(bridge)).expect("owner");
    let mut command = request();
    command.version = 2;
    command.limits = Some(ArchiveReadLimits {
        max_retained_bytes: 500,
        ..Default::default()
    });
    let operation = client.start(command.clone()).expect("explicit v2");
    send(
        &endpoint,
        &client,
        event(&command, ArchiveReadEventKind::Accepted),
    )
    .await;
    let mut missing = event(&command, ArchiveReadEventKind::Negotiated);
    missing.capabilities = ARCHIVE_READ_CAPABILITIES
        .iter()
        .map(|v| (*v).into())
        .collect();
    send(&endpoint, &client, missing).await;
    assert_eq!(
        operation.terminal().expect("no fallback").code,
        Some(ArchiveReadError::MissingCapability)
    );

    let (bridge, endpoint) = platform_bridge(32);
    endpoint.set_connected_for_contract_test(true);
    let client = ArchiveReadClient::new(Arc::new(bridge)).expect("owner");
    command.operation_id = Uuid::new_v4();
    let first = batch(&command);
    let cost = first
        .batch
        .as_ref()
        .expect("batch")
        .validate_for(2)
        .expect("serialized size") as u64;
    command.limits.as_mut().expect("limits").max_retained_bytes = cost;
    let operation = client.start(command.clone()).expect("explicit v2");
    negotiate(&endpoint, &client, &command).await;
    send(&endpoint, &client, first.clone()).await;
    let accepted = operation.take_batch().expect("first bounded batch");
    operation.acknowledge(0).expect("ack is NOT memory release");
    let mut second = first;
    let batch = second.batch.as_mut().expect("batch");
    batch.sequence = 1;
    batch.entries[0].ordinal = 1;
    send(&endpoint, &client, second).await;
    assert_eq!(
        operation.terminal().expect("cumulative budget").code,
        Some(ArchiveReadError::MetadataBudget)
    );
    assert!(operation.take_batch().is_none());
    drop(accepted);
}

#[tokio::test]
async fn completed_operation_retains_original_connection_after_replacement() {
    let (bridge, endpoint) = platform_bridge(32);
    endpoint.set_connected_for_contract_test(true);
    let bridge = Arc::new(bridge);
    let origin = bridge.connection_generation().expect("original connection");
    let client = ArchiveReadClient::new(bridge.clone()).expect("owner");
    let command = request();
    let operation = client.start(command.clone()).expect("read admission");
    assert_eq!(operation.connection_generation(), origin);
    endpoint
        .try_receive_command_for_contract_test()
        .expect("start");
    negotiate(&endpoint, &client, &command).await;
    send(&endpoint, &client, batch(&command)).await;
    let first = operation.take_batch().expect("batch");
    operation.acknowledge(first.sequence).expect("credit");
    endpoint
        .try_receive_command_for_contract_test()
        .expect("ack");
    send(&endpoint, &client, complete(&command)).await;
    assert_eq!(
        operation.terminal().expect("completion").status,
        ArchiveReadStatus::Succeeded
    );
    let _receiver = endpoint.open_connection_for_contract_test();
    assert_ne!(bridge.connection_generation(), Some(origin));
    assert_eq!(
        operation.connection_generation(),
        origin,
        "origin survives removed read pending and current shared bridge replacement"
    );
}
