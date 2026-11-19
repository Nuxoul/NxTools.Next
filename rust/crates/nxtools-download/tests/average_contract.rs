//! R4 red-first contracts. All transfers are synthetic loopback fixtures.
//! JSON injection deliberately uses existing APIs: absent statistics fail an
//! assertion rather than making the baseline fail to compile.
mod support;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use nxtools_download::{
    DOWNLOAD_COMMAND_PROTOCOL_VERSION, DashDownloadRequest, DownloadCommand,
    DownloadCommandEnvelope, DownloadCommandOutcome, DownloadEngine, DownloadEngineTask,
    DownloadQuery, DownloadRequest, DownloadRequestMedia, DownloadStatus, DownloadTask,
    DownloadTaskId, HlsDownloadRequest, RequestOrigin, SqliteDownloadStore,
};
use serde_json::{Value, json};
use support::{DeterministicHttpServer, concurrent_fixture_bytes};

fn request(source_url: String, destination: PathBuf) -> DownloadRequest {
    DownloadRequest {
        source_url,
        save_directory: destination,
        save_directory_explicit: true,
        file_name: Some("average-fixture.bin".into()),
        headers: Vec::new(),
        cookie: None,
        referrer: None,
        source_page: None,
        certificate_policy: Default::default(),
        connection_limit: Some(1),
        speed_limit_bytes_per_second: None,
        expected_total_bytes: None,
        scheduled_at_unix_ms: None,
        adaptive_segment_policy: None,
        media: None,
        category_id: None,
        category_name: None,
        duplicate_strategy: Default::default(),
    }
}

fn statistics(bytes: u64, active_ms: u64) -> Value {
    json!({ "transferred_bytes": bytes, "active_download_ms": active_ms })
}

fn task_statistics(task: &DownloadTask) -> Value {
    serde_json::to_value(task).expect("serialize real task")["transfer_statistics"].clone()
}

fn completed_statistics(task: &DownloadEngineTask, expected_bytes: u64) -> Value {
    assert_eq!(
        task.task.status(),
        DownloadStatus::Completed,
        "{:?}",
        task.error
    );
    let stats = task_statistics(&task.task);
    assert_eq!(
        stats["transferred_bytes"].as_u64(),
        Some(expected_bytes),
        "completed tasks must carry measured bytes, not a live tick or output-size guess"
    );
    assert!(
        stats["active_download_ms"]
            .as_u64()
            .is_some_and(|ms| ms > 0),
        "completed tasks must carry a reliable cumulative active duration"
    );
    stats
}

fn seeded_task(destination: PathBuf, stats: Option<Value>) -> DownloadTask {
    let mut task = DownloadTask::new(
        request("http://127.0.0.1:9/not-requested".into(), destination),
        RequestOrigin::Internal,
        1_000,
    );
    // Synthetic lifecycle clock: 1s active, 1 day paused, 1 day waiting,
    // then 3s active. created/completed timestamps must never be the divisor.
    for (status, at) in [
        (DownloadStatus::Probing, 2_000),
        (DownloadStatus::Waiting, 3_000),
        (DownloadStatus::Downloading, 4_000),
        (DownloadStatus::Paused, 5_000),
        (DownloadStatus::Waiting, 86_405_000),
        (DownloadStatus::Downloading, 172_805_000),
        (DownloadStatus::Finalizing, 172_808_000),
        (DownloadStatus::Verifying, 172_900_000),
        (DownloadStatus::Completed, 173_000_000),
    ] {
        task.transition_to(status, at)
            .expect("synthetic lifecycle transition");
    }
    let mut wire = serde_json::to_value(task).expect("serialize completed fixture");
    if let Some(stats) = stats {
        wire["transfer_statistics"] = stats;
    } else {
        wire.as_object_mut()
            .expect("legacy task object")
            .remove("transfer_statistics");
    }
    serde_json::from_value(wire).expect("deserialize optional statistics on existing API")
}

#[test]
fn average_statistics_survive_public_sqlite_save_load_and_reopen() {
    let directory = tempfile::tempdir().expect("isolated statistics store");
    let database = directory.path().join("downloads.sqlite3");
    let expected = statistics(4_096, 4_000);
    let original = seeded_task(directory.path().join("output"), Some(expected.clone()));
    let task_id = original.id();
    {
        let store = SqliteDownloadStore::open(&database).expect("open real SQLite store");
        store
            .save_task(&original)
            .expect("save task and statistics");
        let restored = store
            .load_task(task_id)
            .expect("load task")
            .expect("saved task");
        assert_eq!(
            task_statistics(&restored),
            expected,
            "public save/load lost duration"
        );
    }
    let reopened = SqliteDownloadStore::open(&database).expect("reopen SQLite store");
    let restored = reopened
        .load_task(task_id)
        .expect("reload task")
        .expect("durable task");
    assert_eq!(
        task_statistics(&restored),
        expected,
        "restart lost measured statistics"
    );
}

#[test]
fn average_legacy_completed_record_keeps_unknown_duration_through_restart() {
    let directory = tempfile::tempdir().expect("isolated legacy store");
    let database = directory.path().join("downloads.sqlite3");
    let legacy = seeded_task(directory.path().join("output"), None);
    let task_id = legacy.id();
    {
        let store = SqliteDownloadStore::open(&database).expect("open legacy fixture store");
        store.save_task(&legacy).expect("save legacy history");
    }
    let engine = DownloadEngine::open(&database).expect("open real recovery/projection boundary");
    let restored = engine
        .task(task_id)
        .expect("read legacy task")
        .expect("legacy task");
    assert!(
        task_statistics(&restored.task).is_null(),
        "timestamps cannot fabricate duration"
    );
    let snapshot = compact(&engine);
    assert!(
        serde_json::to_value(&snapshot.task_summaries[0]).expect("serialize legacy summary")
            ["transfer_statistics"].is_null(),
        "compact projection must preserve unknown duration"
    );
}

fn compact(engine: &DownloadEngine) -> nxtools_download::DownloadSnapshot {
    let response = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Internal,
        command: DownloadCommand::Snapshot {
            query: DownloadQuery::compact(),
        },
    });
    let DownloadCommandOutcome::Snapshot(snapshot) = response.outcome else {
        panic!(
            "real compact snapshot command failed: {:?}",
            response.outcome
        );
    };
    snapshot
}

#[test]
fn average_compact_projection_and_reconstruction_preserve_measured_pair() {
    let directory = tempfile::tempdir().expect("isolated compact statistics store");
    let database = directory.path().join("downloads.sqlite3");
    let expected = statistics(4_096, 4_000);
    let original = seeded_task(directory.path().join("output"), Some(expected.clone()));
    {
        let store = SqliteDownloadStore::open(&database).expect("open projection fixture store");
        store
            .save_task(&original)
            .expect("save measured completed task");
    }
    let engine = DownloadEngine::open(&database).expect("open engine from durable state");
    let snapshot = compact(&engine);
    assert_eq!(snapshot.task_summaries.len(), 1);
    let summary = &snapshot.task_summaries[0];
    assert_eq!(
        serde_json::to_value(summary).expect("serialize real compact summary")["transfer_statistics"],
        expected,
        "compact protocol must not strip the measured byte/time pair"
    );
    let reconstructed = summary
        .clone()
        .into_engine_task()
        .expect("real Desktop reconstruction");
    assert_eq!(task_statistics(&reconstructed.task), expected);
    assert_eq!(reconstructed.task.id(), original.id());
}

#[tokio::test]
async fn average_http_completion_has_durable_statistics_before_publication() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("isolated HTTP statistics fixture");
    let database = directory.path().join("downloads.sqlite3");
    let engine = DownloadEngine::open(&database).expect("open engine");
    let task_id = engine
        .submit(
            request(
                format!("http://{}/range", server.address()),
                directory.path().join("output"),
            ),
            RequestOrigin::Internal,
        )
        .expect("submit localhost fixture");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(5))
        .await
        .expect("wait for completed fixture");
    assert_eq!(
        std::fs::read(completed.final_path.as_ref().expect("final path"))
            .expect("read actual HTTP output"),
        b"0123456789abcdef"
    );
    let expected = completed_statistics(&completed, 16);
    // A separate store reads the committed pair while Completed is already observable.
    let store = SqliteDownloadStore::open(&database).expect("read committed store");
    let persisted = store
        .load_task(task_id)
        .expect("load committed task")
        .expect("committed task");
    assert_eq!(task_statistics(&persisted), expected);
    let snapshot = compact(&engine);
    assert_eq!(
        serde_json::to_value(&snapshot.task_summaries[0]).expect("compact HTTP summary")["transfer_statistics"],
        expected
    );
    let reopened = DownloadEngine::open(&database).expect("reopen completed engine");
    assert_eq!(
        task_statistics(
            &reopened
                .task(task_id)
                .expect("restored task")
                .expect("task")
                .task
        ),
        expected
    );
}

async fn wait_for_partial(engine: &DownloadEngine, task_id: DownloadTaskId, total: u64) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let task = engine
            .task(task_id)
            .expect("read active fixture")
            .expect("active task");
        if task.task.status() == DownloadStatus::Downloading
            && task.downloaded_bytes > 0
            && task.downloaded_bytes < total
        {
            return;
        }
        assert!(
            !matches!(
                task.task.status(),
                DownloadStatus::Failed | DownloadStatus::Completed | DownloadStatus::Finalizing
            ),
            "fixture terminated before partial bytes: {:?}",
            task.error
        );
        assert!(Instant::now() < deadline, "bounded localhost progress wait");
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

#[tokio::test]
async fn average_http_pause_resume_restart_does_not_count_restored_prefix_twice() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("isolated resume statistics fixture");
    let database = directory.path().join("downloads.sqlite3");
    let engine = DownloadEngine::open(&database).expect("open resumable engine");
    let expected = concurrent_fixture_bytes();
    let mut download = request(
        format!(
            "http://{}/range-resume-required?checkpoint=1",
            server.address()
        ),
        directory.path().join("output"),
    );
    download.speed_limit_bytes_per_second = Some(64 * 1024);
    let task_id = engine
        .submit(download, RequestOrigin::Internal)
        .expect("submit resumable localhost fixture");
    wait_for_partial(&engine, task_id, expected.len() as u64).await;
    engine.pause(task_id).expect("pause real transfer");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(5))
        .await
        .expect("join paused writers");
    assert_eq!(paused.task.status(), DownloadStatus::Paused);
    assert!(paused.downloaded_bytes > 0 && paused.downloaded_bytes < expected.len() as u64);
    let paused_stats = task_statistics(&paused.task);
    drop(engine);
    let reopened = DownloadEngine::open(&database).expect("reopen paused transfer");
    assert_eq!(
        task_statistics(
            &reopened
                .task(task_id)
                .expect("restored task")
                .expect("task")
                .task
        ),
        paused_stats
    );
    reopened
        .set_task_speed_limit(task_id, None)
        .expect("remove checkpoint rate limit before resume");
    reopened
        .resume(task_id)
        .expect("resume saved Range checkpoint");
    let completed = reopened
        .wait_for_terminal(task_id, Duration::from_secs(5))
        .await
        .expect("complete resumed transfer");
    assert_eq!(
        std::fs::read(completed.final_path.as_ref().expect("resumed path"))
            .expect("resumed output"),
        expected
    );
    let stats = completed_statistics(&completed, expected.len() as u64);
    assert!(stats["active_download_ms"].as_u64() >= paused_stats["active_download_ms"].as_u64());
}

#[tokio::test]
async fn average_http_retry_retains_a_reliable_completed_measurement() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("isolated retry fixture");
    let engine =
        DownloadEngine::open(directory.path().join("downloads.sqlite3")).expect("retry engine");
    let task_id = engine
        .submit(
            request(
                format!("http://{}/range-transient", server.address()),
                directory.path().join("output"),
            ),
            RequestOrigin::Internal,
        )
        .expect("submit transient localhost fixture");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(5))
        .await
        .expect("finish bounded retry");
    assert_eq!(
        server.transient_range_attempts(),
        2,
        "fixture must actually retry"
    );
    completed_statistics(&completed, 16);
}

#[tokio::test]
async fn average_hls_pause_resume_restart_measures_bytes_not_fragment_count() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("isolated HLS statistics fixture");
    let database = directory.path().join("downloads.sqlite3");
    let destination = directory.path().join("output");
    let engine = DownloadEngine::open(&database).expect("HLS engine");
    let mut download = request(
        format!("http://{}/hls/master.m3u8", server.address()),
        destination.clone(),
    );
    download.file_name = Some("average.ts".into());
    download.media = Some(DownloadRequestMedia::Hls(HlsDownloadRequest {
        playlist_url: format!("http://{}/hls/concurrent/index.m3u8", server.address()),
    }));
    server.hold_hls_second_segment(true);
    let task_id = engine
        .submit(download, RequestOrigin::Internal)
        .expect("submit HLS fixture");
    let checkpoint = destination
        .join(format!(".nxtools-hls-{}", task_id.as_uuid()))
        .join("00000000.segment");
    let deadline = Instant::now() + Duration::from_secs(5);
    while !checkpoint.exists() || server.hls_second_segment_attempts() == 0 {
        let task = engine
            .task(task_id)
            .expect("read HLS stage")
            .expect("HLS task");
        assert_ne!(
            task.task.status(),
            DownloadStatus::Failed,
            "{:?}",
            task.error
        );
        assert!(
            Instant::now() < deadline,
            "bounded durable HLS checkpoint wait"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    engine.pause(task_id).expect("pause HLS workers");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(5))
        .await
        .expect("join HLS workers");
    assert_eq!(paused.task.status(), DownloadStatus::Paused);
    server.hold_hls_second_segment(false);
    drop(engine);
    let reopened = DownloadEngine::open(&database).expect("reopen HLS checkpoint");
    reopened.resume(task_id).expect("resume HLS fixture");
    let completed = reopened
        .wait_for_terminal(task_id, Duration::from_secs(5))
        .await
        .expect("complete HLS fixture");
    let expected = b"segment-a|segment-b|segment-c|segment-d";
    assert_eq!(
        std::fs::read(destination.join("average.ts")).expect("real HLS output"),
        expected
    );
    assert_eq!(
        server.hls_first_segment_attempts(),
        1,
        "restored fragment must not be retransferred"
    );
    completed_statistics(&completed, expected.len() as u64);
}

#[tokio::test]
async fn average_dash_completion_measures_actual_fragment_bytes() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("isolated DASH statistics fixture");
    let engine =
        DownloadEngine::open(directory.path().join("downloads.sqlite3")).expect("DASH engine");
    let mut download = request(
        format!("http://{}/dash/static.mpd", server.address()),
        directory.path().join("output"),
    );
    download.file_name = Some("average.mp4".into());
    download.connection_limit = Some(2);
    download.media = Some(DownloadRequestMedia::Dash(DashDownloadRequest {
        manifest_url: download.source_url.clone(),
        video_representation_id: Some("v1080".into()),
        audio_representation_id: None,
    }));
    let task_id = engine
        .submit(download, RequestOrigin::Internal)
        .expect("submit DASH fixture");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(5))
        .await
        .expect("complete DASH fixture");
    assert_eq!(
        std::fs::read(completed.final_path.as_ref().expect("DASH output path"))
            .expect("real DASH output"),
        b"VINIT|V0|V1"
    );
    completed_statistics(&completed, b"VINIT|V0|V1".len() as u64);
}

#[tokio::test]
async fn average_legacy_resumed_prefix_keeps_unknown_duration_instead_of_overstating_rate() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("isolated legacy resume fixture");
    let database = directory.path().join("downloads.sqlite3");
    let engine = DownloadEngine::open(&database).expect("open legacy resume engine");
    let expected = concurrent_fixture_bytes();
    let mut download = request(
        format!(
            "http://{}/range-resume-required?checkpoint=1",
            server.address()
        ),
        directory.path().join("output"),
    );
    download.speed_limit_bytes_per_second = Some(64 * 1024);
    let task_id = engine
        .submit(download, RequestOrigin::Internal)
        .expect("submit legacy resume fixture");
    wait_for_partial(&engine, task_id, expected.len() as u64).await;
    engine.pause(task_id).expect("pause legacy fixture");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(5))
        .await
        .expect("join paused legacy writers");
    assert_eq!(paused.task.status(), DownloadStatus::Paused);
    assert!(paused.downloaded_bytes > 0 && paused.downloaded_bytes < expected.len() as u64);
    let mut legacy = serde_json::to_value(&paused.task).expect("serialize old task shape");
    legacy
        .as_object_mut()
        .expect("task wire object")
        .remove("transfer_statistics");
    let legacy: DownloadTask = serde_json::from_value(legacy).expect("read legacy wire");
    drop(engine);
    {
        let store = SqliteDownloadStore::open(&database).expect("open legacy checkpoint store");
        store
            .save_task(&legacy)
            .expect("replace only task metadata with legacy record");
    }
    let reopened = DownloadEngine::open(&database).expect("reopen legacy partial");
    reopened
        .set_task_speed_limit(task_id, None)
        .expect("remove checkpoint rate limit before legacy resume");
    reopened
        .resume(task_id)
        .expect("resume genuine legacy prefix");
    let completed = reopened
        .wait_for_terminal(task_id, Duration::from_secs(5))
        .await
        .expect("finish legacy partial");
    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "{:?}",
        completed.error
    );
    assert_eq!(completed.downloaded_bytes, expected.len() as u64);
    assert_eq!(
        std::fs::read(completed.final_path.as_ref().expect("legacy output path"))
            .expect("legacy resumed output"),
        expected
    );
    assert!(
        task_statistics(&completed.task).is_null(),
        "unknown prefix duration cannot be paired with only the resumed suffix time"
    );
}

#[tokio::test]
async fn average_encrypted_hls_counts_actual_ciphertext_bytes_not_plaintext_output_length() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("isolated encrypted HLS statistics");
    let engine =
        DownloadEngine::open(directory.path().join("downloads.sqlite3")).expect("AES HLS engine");
    let mut download = request(
        format!("http://{}/hls/master.m3u8", server.address()),
        directory.path().join("output"),
    );
    download.file_name = Some("average-encrypted.ts".into());
    download.connection_limit = Some(3);
    download.media = Some(DownloadRequestMedia::Hls(HlsDownloadRequest {
        playlist_url: format!("http://{}/hls/aes/index.m3u8", server.address()),
    }));
    let task_id = engine
        .submit(download, RequestOrigin::Internal)
        .expect("submit AES HLS fixture");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(5))
        .await
        .expect("complete AES HLS fixture");
    let output = std::fs::read(completed.final_path.as_ref().expect("AES output path"))
        .expect("read decrypted artifact");
    assert_eq!(
        output,
        b"encrypted-one|encrypted-two|rotated-three|clear-four"
    );
    // Three 16-byte ciphertext bodies plus one 10-byte clear fragment.
    // Manifest/key/probe control requests are not transfer payload bytes.
    completed_statistics(&completed, 58);
    assert_ne!(
        output.len(),
        58,
        "fixture must distinguish payload bytes from artifact length"
    );
}

#[tokio::test]
async fn average_partial_body_retry_counts_retransmitted_payload_not_final_size() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind isolated retry server");
    listener
        .set_nonblocking(true)
        .expect("bounded listener shutdown");
    let address = listener.local_addr().expect("retry fixture address");
    let stopped = Arc::new(AtomicBool::new(false));
    let attempts = Arc::new(AtomicU64::new(0));
    let server_stopped = Arc::clone(&stopped);
    let server_attempts = Arc::clone(&attempts);
    let worker = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(15);
        while !server_stopped.load(Ordering::Acquire) && Instant::now() < deadline {
            let (mut socket, _) = match listener.accept() {
                Ok(connection) => connection,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(2));
                    continue;
                }
                Err(error) => panic!("fixture accept failed: {error}"),
            };
            socket
                .set_nonblocking(false)
                .expect("accepted blocking socket");
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .expect("bounded read");
            socket
                .set_write_timeout(Some(Duration::from_secs(2)))
                .expect("bounded write");
            let mut headers = Vec::new();
            let mut byte = [0_u8; 1];
            while headers.len() < 16_384 && !headers.ends_with(b"\r\n\r\n") {
                if socket
                    .read(&mut byte)
                    .expect("bounded fixture request read")
                    == 0
                {
                    break;
                }
                headers.push(byte[0]);
            }
            let headers = String::from_utf8_lossy(&headers).to_lowercase();
            let probe = headers.starts_with("head ") || headers.contains("\r\nrange:");
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 16\r\nConnection: close\r\n\r\n")
                .expect("fixture response headers");
            if headers.starts_with("head ") {
                continue;
            }
            if !probe && server_attempts.fetch_add(1, Ordering::AcqRel) == 0 {
                socket.write_all(b"01234567").expect("first partial body");
                socket.flush().expect("make partial bytes observable");
                std::thread::sleep(Duration::from_millis(100));
                // Drop closes a deliberately truncated successful-status body.
            } else {
                let _ = socket.write_all(b"0123456789abcdef");
            }
        }
    });
    let directory = tempfile::tempdir().expect("isolated retransmission output");
    let engine = DownloadEngine::open(directory.path().join("downloads.sqlite3"))
        .expect("body retry engine");
    let task_id = engine
        .submit(
            request(
                format!("http://{address}/body"),
                directory.path().join("output"),
            ),
            RequestOrigin::Internal,
        )
        .expect("submit local partial-body fixture");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(8))
        .await
        .expect("bounded body retry");
    stopped.store(true, Ordering::Release);
    worker.join().expect("join local retry server");
    assert_eq!(
        attempts.load(Ordering::Acquire),
        2,
        "actual failed-body retry must occur"
    );
    assert_eq!(
        std::fs::read(completed.final_path.as_ref().expect("retry path")).expect("final output"),
        b"0123456789abcdef"
    );
    completed_statistics(&completed, 24);
}
