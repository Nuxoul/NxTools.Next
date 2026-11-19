use std::path::PathBuf;
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use nxtools_download::{
    DownloadCommand, DownloadCommandOutcome, DownloadDuplicateStrategy, DownloadFaultCode,
    DownloadQuery, DownloadRequest, DownloadSubmitMode,
};

use super::*;

#[test]
fn normal_stop_retires_idle_and_subscribed_clients_before_host_join() {
    let pipe_name = format!(
        r"\\.\pipe\nxtools-download-completion-stop-{}",
        uuid::Uuid::new_v4()
    );
    let engine = DownloadEngine::open_in_memory().expect("open completion IPC engine");
    let activity = DownloadHostActivity::default();
    let host = start_test_host(&pipe_name, engine, activity.clone());

    let (idle_connected_tx, idle_connected_rx) = mpsc::sync_channel(1);
    let idle_name = pipe_name.clone();
    let idle_client = std::thread::spawn(move || {
        let mut client = DownloadIpcClient::connect(&idle_name, Duration::from_secs(2))
            .expect("connect idle client");
        idle_connected_tx.send(()).expect("publish idle connection");
        client.next_event()
    });
    idle_connected_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("idle client handshake");

    let (subscribed_tx, subscribed_rx) = mpsc::sync_channel(1);
    let (first_event_tx, first_event_rx) = mpsc::sync_channel(1);
    let subscriber_name = pipe_name.clone();
    let subscriber = std::thread::spawn(move || {
        let mut client = DownloadIpcClient::connect(&subscriber_name, Duration::from_secs(2))
            .expect("connect subscriber");
        let result = client
            .command(command_envelope(DownloadCommand::Subscribe {
                after_sequence: None,
                generation: None,
            }))
            .expect("subscribe to download events");
        assert!(matches!(
            result.outcome,
            DownloadCommandOutcome::Accepted { task_id: None }
                | DownloadCommandOutcome::Subscribed { .. }
        ));
        subscribed_tx.send(()).expect("publish subscription");
        let event = client
            .next_event()
            .expect("read queued task event")
            .expect("task event before stop");
        first_event_tx
            .send(event.sequence)
            .expect("publish observed task event");
        client.next_event()
    });
    subscribed_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("subscriber handshake");

    wait_for_clients(&activity, 2);
    let mut actor = DownloadIpcClient::connect(&pipe_name, Duration::from_secs(2))
        .expect("connect queue-only actor");
    let accepted = actor
        .command(command_envelope(DownloadCommand::Submit {
            request: queue_only_request("queued-before-drain.bin"),
            mode: DownloadSubmitMode::QueueOnly,
            idempotency_key: None,
        }))
        .expect("enqueue queue-only task");
    assert!(matches!(
        accepted.outcome,
        DownloadCommandOutcome::Accepted { task_id: Some(_) }
    ));
    first_event_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("subscriber observes queue-only event");

    activity.begin_draining();
    let rejected = actor
        .command(command_envelope(DownloadCommand::Submit {
            request: queue_only_request("rejected-during-drain.bin"),
            mode: DownloadSubmitMode::QueueOnly,
            idempotency_key: None,
        }))
        .expect("receive draining rejection");
    assert!(matches!(
        rejected.outcome,
        DownloadCommandOutcome::Rejected { fault }
            if fault.code == DownloadFaultCode::Unavailable && fault.retryable
    ));
    let snapshot = actor
        .command(command_envelope(DownloadCommand::Snapshot {
            query: DownloadQuery::default(),
        }))
        .expect("read snapshot while draining");
    assert!(matches!(
        snapshot.outcome,
        DownloadCommandOutcome::Snapshot(_)
    ));

    stop_download_host(&pipe_name, &activity).expect("request real host stop");
    drop(actor);
    host.join()
        .expect("download host thread did not panic")
        .expect("download host stopped cleanly");

    assert!(matches!(
        idle_client.join().expect("join idle client"),
        Ok(None)
    ));
    assert!(matches!(
        subscriber.join().expect("join subscriber"),
        Ok(None)
    ));
    assert_eq!(activity.connected_clients(), 0);
    assert_eq!(activity.readiness(), DownloadHostReadiness::Stopped);
    assert!(
        DownloadIpcClient::connect(&pipe_name, Duration::from_millis(150)).is_err(),
        "the stopped host must not leave an accepting endpoint"
    );
}

fn start_test_host(
    pipe_name: &str,
    engine: DownloadEngine,
    activity: DownloadHostActivity,
) -> JoinHandle<Result<()>> {
    let host_name = pipe_name.to_owned();
    std::thread::spawn(move || run_download_host(&host_name, engine, activity))
}

fn wait_for_clients(activity: &DownloadHostActivity, expected: usize) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while activity.connected_clients() != expected {
        assert!(
            Instant::now() < deadline,
            "download IPC client registration timed out"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn queue_only_request(file_name: &str) -> DownloadRequest {
    DownloadRequest {
        source_url: format!("https://example.test/{file_name}"),
        save_directory: PathBuf::from(r"C:\Downloads"),
        save_directory_explicit: false,
        file_name: Some(file_name.into()),
        headers: Vec::new(),
        cookie: None,
        referrer: None,
        source_page: None,
        certificate_policy: Default::default(),
        connection_limit: None,
        speed_limit_bytes_per_second: None,
        expected_total_bytes: None,
        scheduled_at_unix_ms: None,
        adaptive_segment_policy: None,
        media: None,
        category_id: None,
        category_name: None,
        duplicate_strategy: DownloadDuplicateStrategy::UseDefault,
    }
}
