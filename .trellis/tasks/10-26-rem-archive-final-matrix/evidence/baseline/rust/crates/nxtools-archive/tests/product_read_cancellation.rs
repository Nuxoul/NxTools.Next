use nxtools_archive::product::ArchiveReadClient;
use nxtools_ipc::{
    PlatformBridge, PlatformBridgeError, PlatformNotification, PlatformServerEndpoint,
    platform_bridge,
};
use nxtools_protocol::{
    ActivateWindowCommand, Envelope, Message, PlatformCommand, PlatformEvent, archive_read::*,
};
use std::sync::Arc;
use uuid::Uuid;

fn request() -> ArchiveReadCommand {
    ArchiveReadCommand {
        version: ARCHIVE_READ_VERSION,
        operation_id: Uuid::new_v4(),
        session_id: Uuid::new_v4(),
        generation: 1,
        action: ArchiveReadAction::Open,
        path: Some("C:\\owned\\synthetic.zip".into()),
        codepage: 0,
        limits: None,
        sequence: None,
    }
}
fn fill(bridge: &PlatformBridge) -> Uuid {
    bridge
        .send_command(PlatformCommand::ActivateWindow(ActivateWindowCommand {
            window_handle: 1,
        }))
        .expect("fill the same production command queue")
}
fn assert_cancel(envelope: Envelope, original: &ArchiveReadCommand) {
    assert_eq!(envelope.correlation_id, Some(original.operation_id));
    let Message::PlatformCommand(PlatformCommand::ArchiveRead(command)) = envelope.message else {
        panic!("expected original typed cancellation")
    };
    assert_eq!(command.action, ArchiveReadAction::Cancel);
    assert_eq!(command.operation_id, original.operation_id);
    assert_eq!(command.session_id, original.session_id);
    assert_eq!(command.generation, original.generation);
}
fn drain_cancel_and_filler(
    endpoint: &PlatformServerEndpoint,
    original: &ArchiveReadCommand,
    filler: Uuid,
) {
    let commands = (0..2)
        .map(|_| endpoint.try_receive_command_for_contract_test().expect("retained cancellation and ordinary filler must both be delivered without retry or polling"))
        .collect::<Vec<_>>();
    assert!(commands.iter().any(|command| command.id == filler));
    assert_cancel(
        commands
            .into_iter()
            .find(|command| command.id != filler)
            .expect("cancel command"),
        original,
    );
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
}

#[test]
fn full_shared_queue_operation_drop_retains_original_cancel() {
    let (bridge, endpoint) = platform_bridge(1);
    endpoint.set_connected_for_contract_test(true);
    let bridge = Arc::new(bridge);
    let client = ArchiveReadClient::new(bridge.clone()).expect("production owner");
    let command = request();
    let operation = client.start(command.clone()).expect("start");
    endpoint
        .try_receive_command_for_contract_test()
        .expect("remote received start");
    let filler = fill(&bridge);
    drop(operation);
    drain_cancel_and_filler(&endpoint, &command, filler);
    assert!(bridge.is_connected(), "original connection remains live");
}

#[test]
fn full_shared_queue_client_drop_retains_original_cancel() {
    let (bridge, endpoint) = platform_bridge(1);
    endpoint.set_connected_for_contract_test(true);
    let bridge = Arc::new(bridge);
    let client = ArchiveReadClient::new(bridge.clone()).expect("production owner");
    let command = request();
    let operation = client.start(command.clone()).expect("start");
    endpoint
        .try_receive_command_for_contract_test()
        .expect("remote received start");
    let filler = fill(&bridge);
    drop(client);
    let local = operation.terminal().expect("local consumer retired");
    assert_eq!(local.code, Some(ArchiveReadError::Cancelled));
    assert!(
        !local.cleanup_complete,
        "local cancellation is not native cleanup"
    );
    drain_cancel_and_filler(&endpoint, &command, filler);
    assert!(bridge.is_connected(), "original connection remains live");
}

#[test]
fn cancel_is_coalesced_and_never_overtakes_its_queued_start() {
    let (bridge, endpoint) = platform_bridge(1);
    endpoint.set_connected_for_contract_test(true);
    let bridge = Arc::new(bridge);
    let client = ArchiveReadClient::new(bridge.clone()).expect("owner");
    let command = request();
    let operation = client.start(command.clone()).expect("queued start");
    for _ in 0..100 {
        operation
            .cancel()
            .expect("reserved cancel does not need ordinary space");
    }
    drop(operation);
    drop(client);
    let start = endpoint
        .try_receive_command_for_contract_test()
        .expect("start precedes cancel");
    assert!(matches!(
        start.message,
        Message::PlatformCommand(PlatformCommand::ArchiveRead(ArchiveReadCommand {
            action: ArchiveReadAction::Open,
            ..
        }))
    ));
    assert_cancel(
        endpoint
            .try_receive_command_for_contract_test()
            .expect("one cancel"),
        &command,
    );
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    assert!(bridge.is_connected());
}

#[test]
fn two_reserved_cancels_and_normal_commands_make_bounded_fair_progress() {
    let (bridge, endpoint) = platform_bridge(1);
    endpoint.set_connected_for_contract_test(true);
    let bridge = Arc::new(bridge);
    let client = ArchiveReadClient::new(bridge.clone()).expect("owner");
    let first = request();
    let one = client.start(first.clone()).expect("first");
    endpoint
        .try_receive_command_for_contract_test()
        .expect("first start");
    let second = request();
    let two = client.start(second.clone()).expect("second");
    endpoint
        .try_receive_command_for_contract_test()
        .expect("second start");
    one.cancel().expect("first cancel");
    two.cancel().expect("second cancel");
    let filler = fill(&bridge);
    assert_cancel(
        endpoint
            .try_receive_command_for_contract_test()
            .expect("cancel priority"),
        &first,
    );
    assert_eq!(
        endpoint
            .try_receive_command_for_contract_test()
            .expect("ordinary turn")
            .id,
        filler
    );
    let next = fill(&bridge);
    assert_cancel(
        endpoint
            .try_receive_command_for_contract_test()
            .expect("next cancel cannot starve"),
        &second,
    );
    assert_eq!(
        endpoint
            .try_receive_command_for_contract_test()
            .expect("next ordinary")
            .id,
        next
    );
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
}

fn terminal(command: &ArchiveReadCommand, cleanup: bool) -> ArchiveReadEvent {
    ArchiveReadEvent {
        version: ARCHIVE_READ_VERSION,
        operation_id: command.operation_id,
        session_id: command.session_id,
        generation: command.generation,
        kind: ArchiveReadEventKind::Terminal,
        capabilities: Vec::new(),
        batch: None,
        terminal: Some(ArchiveReadTerminal {
            status: ArchiveReadStatus::Cancelled,
            code: Some(ArchiveReadError::Cancelled),
            files_verified: 0,
            bytes_verified: 0,
            entry_count: 0,
            batch_count: 0,
            elapsed_ms: 0,
            native_error_code: None,
            cleanup_complete: cleanup,
            transfer: None,
            browser: None,
        }),
    }
}
fn emit(endpoint: &PlatformServerEndpoint, event: ArchiveReadEvent) {
    endpoint
        .send_notification_for_contract_test(PlatformNotification::Event(
            PlatformEvent::ArchiveRead(event),
        ))
        .expect("bounded production events");
}

#[tokio::test]
async fn local_retirement_preserves_two_slots_until_original_remote_cleanup() {
    let (bridge, endpoint) = platform_bridge(1);
    endpoint.set_connected_for_contract_test(true);
    let bridge = Arc::new(bridge);
    let client = ArchiveReadClient::new(bridge.clone()).expect("owner");
    let mut originals = Vec::new();
    for _ in 0..2 {
        let command = request();
        let operation = client
            .start(command.clone())
            .expect("bounded read reservation");
        endpoint
            .try_receive_command_for_contract_test()
            .expect("remote start");
        let mut malformed = terminal(&command, false);
        malformed.version = 99;
        emit(&endpoint, malformed);
        client.poll().await.expect("local protocol error");
        assert_eq!(
            operation.terminal().expect("local terminal").code,
            Some(ArchiveReadError::IncompatibleVersion)
        );
        assert_cancel(
            endpoint
                .try_receive_command_for_contract_test()
                .expect("retained cancel"),
            &command,
        );
        originals.push(command);
    }
    assert!(
        matches!(client.start(request()), Err(ArchiveReadError::Busy)),
        "local removal cannot free a live native read reservation"
    );
    emit(&endpoint, terminal(&originals[0], false));
    client.poll().await.expect("cleanup incomplete is not free");
    let mut stale = terminal(&originals[0], true);
    stale.generation += 1;
    emit(&endpoint, stale);
    client.poll().await.expect("wrong generation");
    assert!(matches!(
        client.start(request()),
        Err(ArchiveReadError::Busy)
    ));
    emit(&endpoint, terminal(&originals[0], true));
    client.poll().await.expect("remote cleanup observation");
    let replacement = client
        .start(request())
        .expect("exact original slot is now free");
    drop(replacement);
}

#[test]
fn original_cancel_is_not_replayed_to_replacement_connection() {
    let (bridge, endpoint) = platform_bridge(1);
    endpoint.set_connected_for_contract_test(true);
    let bridge = Arc::new(bridge);
    let client = ArchiveReadClient::new(bridge.clone()).expect("owner");
    let command = request();
    let old_generation = bridge.connection_generation().expect("original");
    let operation = client.start(command.clone()).expect("original read");
    endpoint
        .try_receive_command_for_contract_test()
        .expect("remote start");
    let _filler = fill(&bridge);
    operation.cancel().expect("retained original cancel");
    let replacement = endpoint.open_connection_for_contract_test();
    drop(operation);
    drop(client);
    let mut old_cancel = command;
    old_cancel.action = ArchiveReadAction::Cancel;
    old_cancel.path = None;
    assert_eq!(
        bridge.send_archive_read(old_cancel, Uuid::new_v4(), old_generation),
        Err(PlatformBridgeError::Disconnected)
    );
    assert!(
        replacement.try_recv().is_err(),
        "old cancellation never enters the new generation"
    );
    assert!(bridge.is_connected());
}

#[tokio::test]
async fn full_shared_queue_local_error_and_overflow_keep_original_cancel() {
    for overflow in [false, true] {
        let (bridge, endpoint) = platform_bridge(1);
        endpoint.set_connected_for_contract_test(true);
        let bridge = Arc::new(bridge);
        let client = ArchiveReadClient::new(bridge.clone()).expect("owner");
        let command = request();
        let operation = client.start(command.clone()).expect("read");
        endpoint
            .try_receive_command_for_contract_test()
            .expect("remote start");
        let filler = fill(&bridge);
        for _ in 0..if overflow { 9 } else { 1 } {
            let mut malformed = terminal(&command, false);
            malformed.version = 99;
            emit(&endpoint, malformed);
        }
        client.poll().await.expect("local retirement");
        let local = operation.terminal().expect("typed local terminal");
        assert_eq!(
            local.code,
            Some(if overflow {
                ArchiveReadError::ConsumerQueueFull
            } else {
                ArchiveReadError::IncompatibleVersion
            })
        );
        assert!(!local.cleanup_complete);
        drop(client);
        drain_cancel_and_filler(&endpoint, &command, filler);
    }
}

#[tokio::test]
async fn cancelled_reader_wakes_waiting_original_command_consumer_without_polling() {
    let (bridge, endpoint) = platform_bridge(1);
    let commands = endpoint.open_connection_for_contract_test();
    let client = ArchiveReadClient::new(Arc::new(bridge)).expect("owner");
    let command = request();
    let operation = client.start(command.clone()).expect("read");
    commands.recv().await.expect("remote start");
    let receive = commands.recv();
    tokio::pin!(receive);
    assert!(matches!(
        std::future::poll_fn(|cx| std::task::Poll::Ready(receive.as_mut().poll(cx))).await,
        std::task::Poll::Pending
    ));
    drop(client);
    let cancel = tokio::time::timeout(std::time::Duration::from_secs(1), receive)
        .await
        .expect("cancel wake, not polling")
        .expect("cancel");
    assert_cancel(cancel, &command);
    assert!(
        !operation
            .terminal()
            .expect("local retired")
            .cleanup_complete
    );
}

#[test]
fn ordinary_archive_receipt_pressure_cannot_consume_reserved_cancellation() {
    let (bridge, endpoint) = platform_bridge(1);
    endpoint.set_connected_for_contract_test(true);
    let generation = bridge.connection_generation().expect("original");
    let command = request();
    bridge
        .send_archive_read(command.clone(), Uuid::new_v4(), generation)
        .expect("read start");
    endpoint
        .try_receive_command_for_contract_test()
        .expect("remote start");
    let mut ack = command.clone();
    ack.action = ArchiveReadAction::Ack;
    ack.path = None;
    ack.sequence = Some(0);
    for _ in 0..31 {
        bridge
            .send_archive_read(ack.clone(), Uuid::new_v4(), generation)
            .expect("bounded pending receipts");
        endpoint
            .try_receive_command_for_contract_test()
            .expect("remote ack");
    }
    assert_eq!(
        bridge.send_archive_read(ack, Uuid::new_v4(), generation),
        Err(PlatformBridgeError::Busy)
    );
    let mut cancel = command.clone();
    cancel.action = ArchiveReadAction::Cancel;
    cancel.path = None;
    let id = bridge
        .send_archive_read(cancel.clone(), Uuid::new_v4(), generation)
        .expect("reserved cancel despite receipt pressure");
    for _ in 0..100 {
        assert_eq!(
            bridge.send_archive_read(cancel.clone(), Uuid::new_v4(), generation),
            Ok(id)
        );
    }
    assert_cancel(
        endpoint
            .try_receive_command_for_contract_test()
            .expect("single cancel"),
        &command,
    );
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
}

#[tokio::test]
async fn full_queue_metadata_and_bad_receipt_retirement_preserve_cancel() {
    for metadata in [false, true] {
        let (bridge, endpoint) = platform_bridge(1);
        endpoint.set_connected_for_contract_test(true);
        let bridge = Arc::new(bridge);
        let client = ArchiveReadClient::new(bridge.clone()).expect("owner");
        let mut command = request();
        command.limits = Some(ArchiveReadLimits {
            max_retained_bytes: 1,
            ..Default::default()
        });
        let operation = client.start(command.clone()).expect("read");
        let start = endpoint
            .try_receive_command_for_contract_test()
            .expect("remote start");
        if metadata {
            for kind in [
                ArchiveReadEventKind::Accepted,
                ArchiveReadEventKind::Negotiated,
            ] {
                let mut notification = terminal(&command, false);
                notification.kind = kind;
                notification.terminal = None;
                if kind == ArchiveReadEventKind::Negotiated {
                    notification.capabilities = ARCHIVE_READ_CAPABILITIES
                        .iter()
                        .map(|c| (*c).into())
                        .collect();
                }
                emit(&endpoint, notification);
                client.poll().await.expect("negotiate");
            }
        }
        let filler = fill(&bridge);
        if metadata {
            let mut notification = terminal(&command, false);
            notification.kind = ArchiveReadEventKind::Batch;
            notification.terminal = None;
            notification.batch = Some(ArchiveReadBatch {
                snapshot_id: Uuid::new_v4(),
                sequence: 0,
                format: "Zip".into(),
                entries: vec![ArchiveReadEntry {
                    ordinal: 0,
                    name: "literal.txt".into(),
                    kind: "File".into(),
                    size: Some(0),
                    packed_size: Some(0),
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
            emit(&endpoint, notification);
        } else {
            endpoint
                .send_notification_for_contract_test(PlatformNotification::CommandResult {
                    correlation_id: start.id,
                    result: nxtools_protocol::PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: None,
                    },
                })
                .expect("bad receipt through production routing");
        }
        client.poll().await.expect("local retirement");
        assert_eq!(
            operation.terminal().expect("local typed error").code,
            Some(if metadata {
                ArchiveReadError::MetadataBudget
            } else {
                ArchiveReadError::ProtocolViolation
            })
        );
        drop(client);
        drain_cancel_and_filler(&endpoint, &command, filler);
    }
}
