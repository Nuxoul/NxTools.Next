use super::*;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, sync_channel};

const SIZE: u64 = 8 * 1024 * 1024;

struct Fixture {
    child: Option<Child>,
    input: ChildStdin,
    replies: Receiver<serde_json::Value>,
    reader: Option<std::thread::JoinHandle<()>>,
    port: u16,
}
impl Fixture {
    fn start(config: serde_json::Value) -> Self {
        let mut child = Command::new("python")
            .args([
                "-u",
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/tests/support/fault_recovery_server.py"
                ),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("owned fault fixture");
        let input = child.stdin.take().expect("control input");
        let output = child.stdout.take().expect("control output");
        let (sender, replies) = sync_channel(8);
        let reader = std::thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                let frame: serde_json::Value =
                    serde_json::from_str(&line.expect("fixture frame")).expect("fixture JSON");
                if sender.send(frame).is_err() {
                    break;
                }
            }
        });
        let ready = replies
            .recv_timeout(Duration::from_secs(5))
            .expect("fixture ready");
        assert_eq!(ready["event"], "ready");
        let port = u16::try_from(ready["port"].as_u64().expect("port")).expect("port fits");
        let mut result = Self {
            child: Some(child),
            input,
            replies,
            reader: Some(reader),
            port,
        };
        result.command(
            serde_json::json!({"command":"configure", "config":config}),
            "configured",
        );
        result
    }
    fn command(&mut self, frame: serde_json::Value, event: &str) -> serde_json::Value {
        writeln!(self.input, "{frame}").expect("write fixture command");
        self.input.flush().expect("flush fixture command");
        let reply = self
            .replies
            .recv_timeout(Duration::from_secs(5))
            .expect("fixture response");
        assert_eq!(reply["event"], event, "fixture control order");
        reply
    }
    fn stats(&mut self) -> serde_json::Value {
        let result = self.command(serde_json::json!({"command":"stats"}), "stats");
        assert!(
            result["failure"].is_null(),
            "fixture failure: {}",
            result["failure"]
        );
        result
    }
    fn idle(&mut self) -> serde_json::Value {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let stats = self.stats();
            if stats["active"] == 0
                && stats["records"]
                    .as_array()
                    .expect("records")
                    .iter()
                    .all(|row| !row["ended"].is_null())
            {
                return stats;
            }
            assert!(Instant::now() < deadline, "server transfers must retire");
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    fn finish(mut self) -> serde_json::Value {
        self.idle();
        let stats = self.command(serde_json::json!({"command":"stop"}), "stopped");
        let mut child = self.child.take().expect("owned process");
        assert!(child.wait().expect("join server").success());
        self.reader
            .take()
            .expect("reader")
            .join()
            .expect("join reader");
        assert_eq!(stats["active"], 0);
        assert!(stats["failure"].is_null());
        stats
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = writeln!(self.input, "{{\"command\":\"stop\"}}");
            let _ = self.input.flush();
            let deadline = Instant::now() + Duration::from_secs(5);
            while child.try_wait().ok().flatten().is_none() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(10));
            }
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
            }
            let _ = child.wait();
        }
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}
struct EngineGuard(DownloadEngine);
impl Drop for EngineGuard {
    fn drop(&mut self) {
        let _ = self.0.shutdown_and_wait();
    }
}
fn new_engine() -> EngineGuard {
    let mut network = NetworkSettings::default();
    network.proxy.mode = nxtools_core::NetworkProxyMode::Direct;
    EngineGuard(
        DownloadEngine::from_store_with_http_and_capacity_and_staging_root(
            SqliteDownloadStore::open_in_memory().expect("owned SQLite"),
            HttpDownloadClient::new().expect("HTTP client"),
            Arc::new(SystemStorageCapacityProbe),
            None,
            None,
            NetworkPolicy::new(network).expect("direct route"),
            DownloadHttpPolicy::from_download_settings(&DownloadSettings::default())
                .expect("HTTP policy"),
        )
        .expect("real engine"),
    )
}
fn request(root: &Path, port: u16, name: &str, width: u16) -> DownloadRequest {
    DownloadRequest {
        source_url: format!("http://127.0.0.1:{port}/file"),
        save_directory: root.to_owned(),
        save_directory_explicit: true,
        file_name: Some(name.to_owned()),
        headers: Vec::new(),
        cookie: None,
        referrer: None,
        source_page: None,
        certificate_policy: Default::default(),
        connection_limit: Some(width),
        speed_limit_bytes_per_second: None,
        expected_total_bytes: None,
        scheduled_at_unix_ms: None,
        adaptive_segment_policy: Some(crate::DownloadAdaptiveSegmentPolicy {
            enabled: false,
            ..Default::default()
        }),
        media: None,
        category_id: None,
        category_name: None,
        duplicate_strategy: Default::default(),
    }
}
fn wait_for(
    engine: &EngineGuard,
    id: DownloadTaskId,
    predicate: impl Fn(&DownloadEngineTask) -> bool,
    deadline: Duration,
) -> DownloadEngineTask {
    let until = Instant::now() + deadline;
    loop {
        let snapshot = engine.0.task(id).expect("task query").expect("same task");
        if predicate(&snapshot) {
            return snapshot;
        }
        assert!(
            !matches!(
                snapshot.task.status(),
                DownloadStatus::Failed | DownloadStatus::Cancelled
            ) && Instant::now() < until,
            "task stalled/failed: {:?} {:?}",
            snapshot.task.status(),
            snapshot.error
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn completed(engine: &EngineGuard, id: DownloadTaskId) -> DownloadEngineTask {
    wait_for(
        engine,
        id,
        |task| task.task.status() == DownloadStatus::Completed,
        Duration::from_secs(45),
    )
}
fn output(root: &Path, name: &str) {
    let mut file = std::fs::File::open(root.join(name)).expect("published artifact");
    assert_eq!(file.metadata().expect("output metadata").len(), SIZE);
    let mut buffer = [0; 65536];
    let mut offset = 0_u64;
    loop {
        let count = file.read(&mut buffer).expect("read exact output");
        if count == 0 {
            break;
        }
        for (i, byte) in buffer[..count].iter().enumerate() {
            assert_eq!(
                *byte,
                ((offset + i as u64) % 251) as u8,
                "offset {}",
                offset + i as u64
            );
        }
        offset += count as u64;
    }
    assert_eq!(offset, SIZE);
}
fn rows(stats: &serde_json::Value) -> &[serde_json::Value] {
    stats["records"].as_array().expect("transfer records")
}
fn number(row: &serde_json::Value, key: &str) -> u64 {
    row[key].as_u64().expect("numeric transfer field")
}
fn assert_plan(engine: &EngineGuard, id: DownloadTaskId, complete: bool) {
    let plan = engine
        .0
        .inner
        .store
        .load_transfer_plan(id)
        .expect("saved plan")
        .expect("task plan");
    let mut ordered = plan.segments().to_vec();
    ordered.sort_by_key(DownloadSegment::start_byte);
    let mut next = 0;
    for segment in ordered {
        assert_eq!(
            segment.start_byte(),
            next,
            "no gaps or overlapping ownership"
        );
        assert!(
            segment.downloaded_bytes() <= segment.len(),
            "checkpoint inside range"
        );
        if complete {
            assert_eq!(segment.downloaded_bytes(), segment.len());
        }
        next = segment.end_byte() + 1;
    }
    assert_eq!(next, SIZE);
}

#[test]
fn body_drop_resumes_from_written_offset() {
    let root = tempfile::tempdir().expect("output");
    let server_config = serde_json::json!({"drop_every":2,"drop_after":1024*1024,"rate":128*1024});
    let server = Fixture::start(server_config);
    let engine = new_engine();
    let id = engine
        .0
        .submit(
            request(root.path(), server.port, "drops.bin", 4),
            RequestOrigin::Ui,
        )
        .expect("submit");
    completed(&engine, id);
    engine.0.shutdown_and_wait().expect("worker drain");
    output(root.path(), "drops.bin");
    let stats = server.finish();
    let records = rows(&stats);
    let drops: Vec<_> = records
        .iter()
        .filter(|r| r["outcome"] == "dropped")
        .collect();
    assert!(!drops.is_empty(), "fixture must cut a real response body");
    for dropped in drops {
        let written_end = number(dropped, "start") + number(dropped, "sent");
        assert!(
            records
                .iter()
                .any(|next| number(next, "ordinal") > number(dropped, "ordinal")
                    && number(next, "start") == written_end - 64),
            "resume from written prefix with64byte verification overlap: {records:?}"
        );
    }
    let total: u64 = records.iter().map(|r| number(r, "sent")).sum();
    assert!(
        total <= SIZE + records.len() as u64 * 64,
        "no repeated body bytes: {total}; requests: {records:?}"
    );

    let server = Fixture::start(
        serde_json::json!({"drop_every":1,"drop_after":1024*1024,"rate":8*1024*1024}),
    );
    let engine = new_engine();
    let id = engine
        .0
        .submit(
            request(root.path(), server.port, "progress-budget.bin", 1),
            RequestOrigin::Ui,
        )
        .expect("single connection with repeated forward progress");
    completed(&engine, id);
    engine.0.shutdown_and_wait().expect("single-worker drain");
    output(root.path(), "progress-budget.bin");
    let stats = server.finish();
    let records = rows(&stats);
    assert!(
        records
            .iter()
            .filter(|row| row["outcome"] == "dropped")
            .count()
            > 5,
        "forward progress must renew the budget beyond five consecutive disconnects"
    );
    let total: u64 = records.iter().map(|row| number(row, "sent")).sum();
    assert!(
        total <= SIZE + records.len() as u64 * 64,
        "no prefix replay while renewing retry budget"
    );
}

#[test]
fn throttle_learns_cap_and_retires_extra_connections() {
    let root = tempfile::tempdir().expect("output");
    let first = Fixture::start(serde_json::json!({"max_conns":2,"rate":2*1024*1024}));
    let engine = new_engine();
    let id = engine
        .0
        .submit(
            request(root.path(), first.port, "limited.bin", 8),
            RequestOrigin::Ui,
        )
        .expect("submit");
    completed(&engine, id);
    output(root.path(), "limited.bin");
    let first_port = first.port;
    let first_stats = first.finish();
    assert!(
        rows(&first_stats)
            .iter()
            .filter(|r| r["status"] == 503)
            .count()
            <= 8
    );
    assert_eq!(first_stats["peak"], 2);
    let client = engine
        .0
        .inner
        .http_for_operation(Default::default())
        .expect("captured client");
    let url = format!("http://127.0.0.1:{first_port}/file");
    let ceiling = engine.0.inner.connection_ceiling(&client, &url);
    assert_eq!(
        connection_ceiling::limit(&ceiling, 8),
        2,
        "learned host width"
    );
    let second = Fixture::start(serde_json::json!({"max_conns":2,"rate":2*1024*1024}));
    let second_id = engine
        .0
        .submit(
            request(root.path(), second.port, "next-port.bin", 8),
            RequestOrigin::Ui,
        )
        .expect("same-host task");
    completed(&engine, second_id);
    engine.0.shutdown_and_wait().expect("worker drain");
    output(root.path(), "next-port.bin");
    let stats = second.finish();
    assert_eq!(
        rows(&stats).iter().filter(|r| r["status"] == 503).count(),
        0
    );
    assert_eq!(
        stats["peak"], 2,
        "second task admits at most two initial workers"
    );
}

#[test]
fn retry_after_is_honoured_and_cancellable() {
    let root = tempfile::tempdir().expect("output");
    let mut server = Fixture::start(serde_json::json!({"retry_first":2,"rate":8*1024*1024}));
    let engine = new_engine();
    let id = engine
        .0
        .submit(
            request(root.path(), server.port, "retry.bin", 1),
            RequestOrigin::Ui,
        )
        .expect("submit");
    completed(&engine, id);
    output(root.path(), "retry.bin");
    let stats = server.idle();
    let records = rows(&stats);
    assert_eq!(records.iter().filter(|r| r["status"] == 503).count(), 2);
    assert!(records.len() >= 3);
    for pair in records.windows(2).take(2) {
        assert!(
            pair[1]["started"].as_f64().expect("next request start")
                - pair[0]["started"]
                    .as_f64()
                    .expect("throttled request start")
                >= 0.99,
            "each Retry-After wait lasts at least one second"
        );
    }
    server.command(
        serde_json::json!({"command":"configure", "config":{"retry_first":2,"rate":8*1024*1024}}),
        "configured",
    );
    let id = engine
        .0
        .submit(
            request(root.path(), server.port, "cancel-wait.bin", 1),
            RequestOrigin::Ui,
        )
        .expect("submit cancellable wait");
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        if rows(&server.stats())
            .iter()
            .any(|r| r["status"] == 503 && !r["ended"].is_null())
        {
            break;
        }
        assert!(Instant::now() < until, "first throttle response");
        std::thread::sleep(Duration::from_millis(5));
    }
    let start = Instant::now();
    engine.0.pause(id).expect("pause during Retry-After wait");
    wait_for(
        &engine,
        id,
        |task| task.task.status() == DownloadStatus::Paused,
        Duration::from_secs(2),
    );
    assert!(
        start.elapsed() < Duration::from_millis(200),
        "pause must wake retry wait"
    );
    server.idle();
    engine.0.resume(id).expect("resume");
    completed(&engine, id);
    engine.0.shutdown_and_wait().expect("worker drain");
    output(root.path(), "cancel-wait.bin");
    let stats = server.finish();
    assert_eq!(
        rows(&stats).iter().filter(|r| r["status"] == 503).count(),
        2
    );
}

#[test]
fn exhausted_connection_retires_while_others_work() {
    let root = tempfile::tempdir().expect("output");
    let server = Fixture::start(serde_json::json!({"first_range_500":true,"rate":4*1024*1024}));
    let engine = new_engine();
    let id = engine
        .0
        .submit(
            request(root.path(), server.port, "retired.bin", 4),
            RequestOrigin::Ui,
        )
        .expect("submit");
    completed(&engine, id);
    engine.0.shutdown_and_wait().expect("worker drain");
    output(root.path(), "retired.bin");
    let stats = server.finish();
    assert_eq!(
        rows(&stats).iter().filter(|r| r["status"] == 500).count(),
        6
    );
    assert!(rows(&stats).iter().any(|r| r["status"] == 206));

    let failing = Fixture::start(serde_json::json!({"all_500":true}));
    let engine = new_engine();
    let id = engine
        .0
        .submit(
            request(root.path(), failing.port, "failed.bin", 4),
            RequestOrigin::Ui,
        )
        .expect("submit failing task");
    let deadline = Instant::now() + Duration::from_secs(35);
    let failed = loop {
        let task = engine.0.task(id).expect("query").expect("task");
        if task.task.status() == DownloadStatus::Failed {
            break task;
        }
        assert!(
            task.task.status() != DownloadStatus::Completed && Instant::now() < deadline,
            "all-500 task must fail in finite time: {:?}",
            task.task.status()
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    let fault = failed.error.expect("terminal HTTP fault");
    assert!(
        fault.message.contains("500"),
        "status 500 preserved: {fault:?}"
    );
    engine.0.shutdown_and_wait().expect("worker drain");
    let stats = failing.finish();
    assert!(rows(&stats).iter().all(|r| r["status"] == 500));
    assert!(rows(&stats).len() <= 40, "bounded total retries");
}

#[test]
fn slow_connection_is_shed_and_tail_is_split() {
    let root = tempfile::tempdir().expect("output");
    let config = serde_json::json!({"slow_every":3,"slow_rate":65536,"rate":4*1024*1024});
    let server = Fixture::start(config.clone());
    let engine = new_engine();
    let start = Instant::now();
    let id = engine
        .0
        .submit(
            request(root.path(), server.port, "slow.bin", 8),
            RequestOrigin::Ui,
        )
        .expect("submit");
    completed(&engine, id);
    let elapsed = start.elapsed();
    engine.0.shutdown_and_wait().expect("worker drain");
    output(root.path(), "slow.bin");
    assert_plan(&engine, id, true);
    let stats = server.finish();
    assert!(
        elapsed < Duration::from_secs(4),
        "live splitting/reconnect: {elapsed:?}"
    );
    assert!(
        rows(&stats)
            .iter()
            .any(|r| number(r, "ordinal").is_multiple_of(3) && r["outcome"] == "aborted"),
        "slow connection was closed before full body"
    );
    assert!(
        rows(&stats).iter().any(|r| r["outcome"] == "aborted"),
        "shortened response is discarded"
    );

    let limited = Fixture::start(config);
    let engine = new_engine();
    let mut input = request(root.path(), limited.port, "limited-slow.bin", 8);
    input.speed_limit_bytes_per_second = Some(1024 * 1024);
    let id = engine
        .0
        .submit(input, RequestOrigin::Ui)
        .expect("limited submit");
    completed(&engine, id);
    engine.0.shutdown_and_wait().expect("limited drain");
    output(root.path(), "limited-slow.bin");
    let stats = limited.finish();
    assert!(
        rows(&stats)
            .iter()
            .any(|r| number(r, "ordinal").is_multiple_of(3) && r["status"] == 206)
    );
    assert!(
        !rows(&stats)
            .iter()
            .filter(|r| number(r, "ordinal").is_multiple_of(3)
                && r["outcome"] == "aborted"
                && number(r, "sent") > 64)
            .any(|slow| rows(&stats)
                .iter()
                .any(|next| number(next, "ordinal") > number(slow, "ordinal")
                    && number(next, "start")
                        == number(slow, "start") + number(slow, "sent").saturating_sub(64))),
        "task limit suppresses slow-worker reconnects while allowing live tail splits: {stats}"
    );
}

#[test]
fn split_while_writing_never_overlaps_and_survives_pause() {
    let root = tempfile::tempdir().expect("output");
    let mut server =
        Fixture::start(serde_json::json!({"slow_every":3,"slow_rate":65536,"rate":4*1024*1024}));
    let engine = new_engine();
    let id = engine
        .0
        .submit(
            request(root.path(), server.port, "split.bin", 8),
            RequestOrigin::Ui,
        )
        .expect("submit");
    let until = Instant::now() + Duration::from_secs(7);
    loop {
        let task = engine.0.task(id).expect("task").expect("same task");
        let plan = engine
            .0
            .inner
            .store
            .load_transfer_plan(id)
            .expect("plan read");
        if let Some(plan) = plan
            && plan.segments().len() > 8
            && task.downloaded_bytes > 0
        {
            break;
        }
        assert!(
            task.task.status() != DownloadStatus::Failed && Instant::now() < until,
            "live split must persist while original workers write: {:?} {:?}",
            task.task.status(),
            task.error
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    engine.0.pause(id).expect("pause during split");
    wait_for(
        &engine,
        id,
        |task| task.task.status() == DownloadStatus::Paused,
        Duration::from_secs(3),
    );
    server.idle();
    assert_plan(&engine, id, false);
    engine.0.resume(id).expect("resume persisted ownership");
    completed(&engine, id);
    engine.0.shutdown_and_wait().expect("writer drain");
    output(root.path(), "split.bin");
    assert_plan(&engine, id, true);
    let stats = server.finish();
    assert!(rows(&stats).iter().any(|r| r["outcome"] == "aborted"));
}

#[test]
fn overlap_mismatch_is_fatal() {
    let root = tempfile::tempdir().expect("output");
    let mut server = Fixture::start(serde_json::json!({"rate":1024*1024}));
    let engine = new_engine();
    let id = engine
        .0
        .submit(
            request(root.path(), server.port, "changed.bin", 4),
            RequestOrigin::Ui,
        )
        .expect("submit");
    wait_for(
        &engine,
        id,
        |task| task.downloaded_bytes >= 256 * 1024,
        Duration::from_secs(8),
    );
    engine.0.pause(id).expect("pause");
    let paused = wait_for(
        &engine,
        id,
        |task| task.task.status() == DownloadStatus::Paused,
        Duration::from_secs(3),
    );
    server.idle();
    let plan = engine
        .0
        .inner
        .store
        .load_transfer_plan(id)
        .expect("plan")
        .expect("saved plan");
    let segment = plan
        .segments()
        .iter()
        .find(|s| s.downloaded_bytes() >= 64 && s.downloaded_bytes() < s.len())
        .expect("partial persisted range");
    let corrupt_at = segment.start_byte() + segment.downloaded_bytes() - 32;
    let temporary = paused.temporary_path.expect("paused artifact path");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .open(temporary)
        .expect("owned partial file");
    file.seek(SeekFrom::Start(corrupt_at))
        .expect("seek overlap");
    file.write_all(&[255]).expect("alter one verified byte");
    file.sync_all().expect("persist fixture alteration");
    drop(file);
    let previous = server.stats()["transfers"].as_u64().expect("count");
    engine.0.resume(id).expect("resume");
    let deadline = Instant::now() + Duration::from_secs(8);
    let failed = loop {
        let task = engine.0.task(id).expect("query").expect("same task");
        if task.task.status() == DownloadStatus::Failed {
            break task;
        }
        assert!(
            task.task.status() != DownloadStatus::Completed && Instant::now() < deadline,
            "overlap mismatch must stop the whole task: {:?} {:?}",
            task.task.status(),
            task.error
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    let fault = failed.error.expect("terminal mismatch fault");
    assert_eq!(fault.code, DownloadFaultCode::Protocol);
    assert!(!fault.retryable, "changed resource is fatal");
    assert!(
        fault.message.contains("changed") || fault.message.contains("FileChanged"),
        "{fault:?}"
    );
    engine.0.shutdown_and_wait().expect("stop all workers");
    let stats = server.finish();
    assert!(
        number(&stats, "transfers") - previous <= 4,
        "no retry after fatal overlap"
    );
    assert!(!root.path().join("changed.bin").exists());
}
