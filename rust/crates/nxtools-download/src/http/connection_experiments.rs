use super::*;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, sync_channel};

struct Server {
    child: Option<Child>,
    input: ChildStdin,
    reader: Option<std::thread::JoinHandle<()>>,
    replies: Receiver<serde_json::Value>,
    port: u16,
}
impl Server {
    fn start(scenario: &str) -> Self {
        Self::start_fixture(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/support/b6_reuse_server.py"
            ),
            scenario,
        )
    }
    fn start_fixture(script: &str, scenario: &str) -> Self {
        let mut child = Command::new("python")
            .args(["-u", script, scenario])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("start owned response fixture");
        let input = child.stdin.take().expect("fixture input");
        let output = child.stdout.take().expect("fixture output");
        let (sender, replies) = sync_channel(8);
        let reader = std::thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                let value =
                    serde_json::from_str(&line.expect("fixture frame")).expect("fixture JSON");
                if sender.send(value).is_err() {
                    break;
                }
            }
        });
        let ready: serde_json::Value = replies.recv_timeout(Duration::from_secs(5)).expect("ready");
        let port = u16::try_from(ready["port"].as_u64().expect("port")).expect("u16 port");
        println!("B6_SERVER_PID={}", child.id());
        Self {
            child: Some(child),
            input,
            reader: Some(reader),
            replies,
            port,
        }
    }
    fn command(&mut self, command: &str) -> serde_json::Value {
        writeln!(self.input, "{{\"command\":\"{command}\"}}").expect("control command");
        self.input.flush().expect("flush control");
        self.replies
            .recv_timeout(Duration::from_secs(5))
            .expect("control reply")
    }
    fn idle(&mut self) -> serde_json::Value {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let stats = self.command("stats");
            assert!(stats["failure"].is_null(), "fixture failure {stats}");
            if stats["open_sockets"] == 0 {
                return stats;
            }
            assert!(
                Instant::now() < deadline,
                "response socket did not retire: {stats}"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    fn finish(&mut self) -> serde_json::Value {
        let stats = self.command("stop");
        assert!(
            self.child
                .take()
                .expect("child")
                .wait()
                .expect("join server")
                .success()
        );
        self.reader
            .take()
            .expect("reader")
            .join()
            .expect("join reader");
        assert_eq!(stats["open_sockets"], 0);
        assert_eq!(stats["active"], 0);
        assert!(stats["failure"].is_null());
        stats
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = writeln!(self.input, "{{\"command\":\"stop\"}}");
            let _ = self.input.flush();
            let deadline = Instant::now() + Duration::from_secs(5);
            while child.try_wait().ok().flatten().is_none() {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            let _ = child.wait();
        }
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}
fn request(root: &Path, port: u16) -> DownloadRequest {
    DownloadRequest {
        source_url: format!("http://127.0.0.1:{port}/file"),
        save_directory: root.to_owned(),
        save_directory_explicit: true,
        file_name: Some("owned.bin".into()),
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
fn exact_output(path: &Path) {
    let mut file = std::fs::File::open(path).expect("completed artifact");
    assert_eq!(file.metadata().expect("metadata").len(), 8 * 1024 * 1024);
    let mut buffer = [0_u8; 65536];
    let mut offset = 0_usize;
    loop {
        let length = file.read(&mut buffer).expect("outside-timing output read");
        if length == 0 {
            break;
        }
        for (index, value) in buffer[..length].iter().enumerate() {
            assert_eq!(
                *value,
                ((offset + index) % 251) as u8,
                "offset {}",
                offset + index
            );
        }
        offset += length;
    }
    assert_eq!(offset, 8 * 1024 * 1024);
}
fn error_kind(error: &HttpDownloadError) -> String {
    match error {
        HttpDownloadError::Status(status) => format!("http_{status}"),
        HttpDownloadError::Client(_) => "client_body".into(),
        HttpDownloadError::LengthMismatch { .. } => "length_mismatch".into(),
        HttpDownloadError::Stalled { .. } => "stalled".into(),
        _ => panic!("unexpected boundary failure: {error:?}"),
    }
}
async fn run(scenario: &str) -> serde_json::Value {
    let directory = tempfile::tempdir().expect("output root");
    let mut server = Server::start(scenario);
    let client = direct_client();
    let request = request(directory.path(), server.port);
    let id = DownloadTaskId::new();
    let control = DownloadTransferControl::new();
    println!("B6_MEASUREMENT_START");
    let started = Instant::now();
    let probe = client.probe(&request).await.expect("initial actual probe");
    let probe_us = started.elapsed().as_micros() as u64;
    let (temporary, final_path) =
        HttpDownloadClient::artifact_paths(&request, &probe, id).expect("paths");
    assert!(
        !temporary.exists() && !final_path.exists(),
        "no artifact before confirmation"
    );
    if scenario == "delayed" {
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
    assert!(
        !temporary.exists() && !final_path.exists(),
        "confirmation owns first write"
    );
    let before_confirmation = server.command("stats");
    let confirmed = Instant::now();
    let result = match client.probe(&request).await {
        Ok(probe) => {
            client
                .download(&request, &probe, id, &control, |_| {})
                .await
        }
        Err(error) => Err(error),
    };
    let confirmation_us = confirmed.elapsed().as_micros() as u64;
    let elapsed_us = started.elapsed().as_micros() as u64;
    println!("B6_MEASUREMENT_END");
    let (completed, failure) = match result {
        Ok(DownloadTransferOutcome::Completed {
            downloaded_bytes, ..
        }) => {
            assert_eq!(downloaded_bytes, 8 * 1024 * 1024);
            exact_output(&final_path);
            assert!(!temporary.exists());
            (true, None)
        }
        Err(error) => {
            assert!(!final_path.exists(), "failed body must not publish");
            (false, Some(error_kind(&error)))
        }
        other => panic!("unexpected transfer result: {other:?}"),
    };
    match scenario {
        "single_use" => assert_eq!(failure.as_deref(), Some("http_410")),
        "truncated" => assert!(!completed, "truncated body must fail"),
        _ => assert!(completed, "baseline completes {scenario}: {failure:?}"),
    }
    drop(client);
    server.idle();
    let stats = server.finish();
    serde_json::json!({"scenario": scenario, "version": "A", "elapsed_us": elapsed_us,
        "probe_us": probe_us, "confirmation_us": confirmation_us, "completed": completed,
        "failure": failure, "before_confirmation": before_confirmation,
        "exact_output": completed, "server": stats, "tcp_retransmitted_bytes": null,
        "server_joined": true, "reader_joined": true, "artifact_before_confirmation": false})
}
fn direct_client() -> HttpDownloadClient {
    let mut settings = NetworkSettings::default();
    settings.proxy.mode = NetworkProxyMode::Direct;
    let network = NetworkPolicy::new(settings).expect("direct policy");
    HttpDownloadClient::with_stall_timeout_and_network(
        DEFAULT_STALL_TIMEOUT,
        &network.snapshot().expect("snapshot"),
    )
    .expect("original client")
}
#[tokio::test]
#[ignore = "serial controlled HTTP measurement"]
async fn ref14_trial() {
    let scenario = std::env::var("B6_SCENARIO").expect("explicit frozen scenario");
    assert!(
        [
            "immediate",
            "delayed",
            "single_use",
            "ineligible",
            "truncated",
            "unknown"
        ]
        .contains(&scenario.as_str())
    );
    let result = tokio::time::timeout(Duration::from_secs(120), run(&scenario))
        .await
        .expect("bounded trial");
    println!("B6_RESULT={result}");
}
#[tokio::test]
async fn ref14_current_probe_and_probe_cancellation_release_unconfirmed_sockets() {
    for cancel_during_probe in [false, true] {
        let directory = tempfile::tempdir().expect("unconfirmed root");
        let mut server = Server::start("immediate");
        let client = direct_client();
        let request = request(directory.path(), server.port);
        if cancel_during_probe {
            let future = client.probe(&request);
            tokio::pin!(future);
            tokio::select! {
                _ = &mut future => panic!("controlled headers must still be pending"),
                () = tokio::time::sleep(Duration::from_millis(50)) => {}
            }
        } else {
            let probe = client.probe(&request).await.expect("real fallback probe");
            assert_eq!(probe.total_bytes, Some(8 * 1024 * 1024));
        }
        let retirement = Instant::now();
        loop {
            let stats = server.command("stats");
            assert!(stats["failure"].is_null());
            if stats["open_sockets"] == 0 {
                break;
            }
            assert!(
                retirement.elapsed() < Duration::from_secs(1),
                "unconfirmed response must retire within one second"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(
            std::fs::read_dir(directory.path())
                .expect("artifact root")
                .count(),
            0
        );
        drop(client);
        let stats = server.finish();
        println!(
            "B6_REF14_RETIREMENT={}",
            serde_json::json!({"cancel_during_probe": cancel_during_probe,
            "retirement_us": retirement.elapsed().as_micros() as u64, "server": stats,
            "no_artifact": true, "actual_join": true})
        );
    }
}

pub(crate) mod ref16;
