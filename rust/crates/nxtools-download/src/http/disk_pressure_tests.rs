use super::*;
use crate::{DownloadSegment, DownloadSegmentState, DownloadTaskId};
use std::io::{Read, Write};

async fn transfer_with_disk_delay(
    delay: Duration,
    stall: Duration,
    pause: bool,
    stop_body: bool,
) -> (
    Result<SegmentReceiveOutcome, HttpDownloadError>,
    u64,
    Duration,
) {
    let size = 4 * 1024 * 1024_u64;
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("local HTTP");
    let address = listener.local_addr().expect("address");
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().expect("segment connection");
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("read bound");
        socket
            .set_write_timeout(Some(Duration::from_secs(5)))
            .expect("write bound");
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") && request.len() < 8192 {
            let mut byte = [0];
            if socket.read(&mut byte).ok() != Some(1) {
                return;
            }
            request.push(byte[0]);
        }
        assert!(String::from_utf8_lossy(&request).contains("bytes=0-4194303"));
        if write!(socket, "HTTP/1.1 206 Partial Content\r\nConnection: close\r\nContent-Length: {size}\r\nContent-Range: bytes 0-{}/{size}\r\n\r\n", size - 1).is_err() { return; }
        let block = vec![0x5a; 64 * 1024];
        for _ in 0..size / block.len() as u64 {
            if socket.write_all(&block).is_err() {
                return;
            }
            if stop_body {
                std::thread::sleep(Duration::from_secs(3));
                return;
            }
        }
    });
    let directory = tempfile::tempdir().expect("artifact directory");
    let request: DownloadRequest = serde_json::from_value(serde_json::json!({
        "source_url": format!("http://{address}/file"),
        "file_name": "disk.bin", "save_directory": directory.path()
    }))
    .expect("request");
    let probe: DownloadProbe = serde_json::from_value(serde_json::json!({
        "final_url": request.source_url, "total_bytes": size, "supports_ranges": true,
        "file_name": "disk.bin", "content_type": null, "validator": {}
    }))
    .expect("probe");
    let segment =
        DownloadSegment::new(0, 0, size - 1, 0, DownloadSegmentState::Waiting).expect("range");
    let mut planner = SegmentPlanner::new(std::slice::from_ref(&segment), Duration::ZERO);
    let assignment = planner.assign(0, None).expect("assignment");
    let state = SegmentReceiveState::new(planner);
    state.note_speed(256 * 1024, Duration::from_millis(1));
    let mut artifact = SegmentedArtifact::create(directory.path().join("disk.part"), size)
        .await
        .expect("artifact");
    artifact.write_delay = delay;
    let control = DownloadSegmentTransferControl::new(DownloadTransferControl::new());
    let pause_control = control.task.clone();
    let mut pause_task = None;
    let mut receiving = false;
    let started = Instant::now();
    let client = HttpDownloadClient::with_stall_timeout(stall).expect("HTTP client");
    let result = client
        .download_planned_segment(
            DownloadSegmentTransferRequest::new(
                DownloadTransferContext::new(DownloadTaskId::new(), uuid::Uuid::new_v4()),
                &request,
                &probe,
                &segment,
                &artifact,
                &control,
            ),
            assignment,
            &state,
            0,
            |_| {
                if pause && !receiving {
                    receiving = true;
                    let control = pause_control.clone();
                    pause_task = Some(tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        control.pause();
                    }));
                }
            },
        )
        .await;
    let elapsed = started.elapsed();
    if let Some(task) = pause_task {
        task.await.expect("pause task");
    }
    let checkpoint = state
        .lock()
        .expect("planner")
        .checkpoints()
        .expect("synced checkpoint")[0]
        .downloaded_bytes();
    let bytes = tokio::fs::read(artifact.path())
        .await
        .expect("actual bytes");
    assert!(
        bytes[..checkpoint as usize]
            .iter()
            .all(|byte| *byte == 0x5a)
    );
    assert!(bytes[checkpoint as usize..].iter().all(|byte| *byte == 0));
    server.join().expect("server drained");
    (result, checkpoint, elapsed)
}

#[tokio::test]
async fn disk_wait_does_not_consume_the_body_stall_budget() {
    let (result, checkpoint, _) = transfer_with_disk_delay(
        Duration::from_millis(300),
        Duration::from_millis(150),
        false,
        false,
    )
    .await;
    assert!(
        matches!(result, Ok(SegmentReceiveOutcome::Completed { .. })),
        "disk delay was classified as network failure: {result:?}"
    );
    assert_eq!(checkpoint, 4 * 1024 * 1024);
}

#[tokio::test]
async fn disk_wait_does_not_retire_a_healthy_network_connection() {
    let (result, checkpoint, _) = transfer_with_disk_delay(
        Duration::from_millis(600),
        Duration::from_secs(10),
        false,
        false,
    )
    .await;
    assert!(
        matches!(result, Ok(SegmentReceiveOutcome::Completed { .. })),
        "disk delay was classified as slow connection: {result:?}"
    );
    assert_eq!(checkpoint, 4 * 1024 * 1024);
}

#[tokio::test]
async fn pause_during_disk_wait_drains_and_syncs_only_the_owned_prefix() {
    let (result, checkpoint, _) = transfer_with_disk_delay(
        Duration::from_millis(300),
        Duration::from_secs(10),
        true,
        false,
    )
    .await;
    assert!(
        matches!(result, Ok(SegmentReceiveOutcome::Paused)),
        "{result:?}"
    );
    assert!(checkpoint > 0 && checkpoint < 4 * 1024 * 1024);
}

#[tokio::test]
async fn empty_periodic_flushes_do_not_hide_a_stalled_network() {
    let (result, checkpoint, elapsed) = transfer_with_disk_delay(
        Duration::from_millis(100),
        Duration::from_millis(1100),
        false,
        true,
    )
    .await;
    assert!(
        matches!(result, Err(HttpDownloadError::Stalled { .. })),
        "{result:?}"
    );
    assert_eq!(checkpoint, 64 * 1024);
    assert!(elapsed < Duration::from_millis(2500), "{elapsed:?}");
}
