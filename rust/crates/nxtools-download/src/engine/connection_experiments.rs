use super::*;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddr};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::AtomicUsize;
use std::sync::mpsc::{Receiver, sync_channel};

#[derive(Default)]
struct ConnectGate {
    calls: AtomicUsize,
    closed_epoch: AtomicU64,
    expected_epoch: AtomicU64,
    gate_extra: AtomicBool,
}

struct FixtureResolver {
    state: Arc<ConnectGate>,
    address: SocketAddr,
}
impl reqwest::dns::Resolve for FixtureResolver {
    fn resolve(&self, _name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let ordinal = self.state.calls.fetch_add(1, Ordering::AcqRel);
        let state = Arc::clone(&self.state);
        let address = self.address;
        Box::pin(async move {
            if state.gate_extra.load(Ordering::Acquire) && matches!(ordinal, 4 | 5) {
                let deadline = Instant::now() + Duration::from_secs(2);
                while state.closed_epoch.load(Ordering::Acquire)
                    < state.expected_epoch.load(Ordering::Acquire)
                {
                    if Instant::now() >= deadline {
                        return Err(Box::<dyn std::error::Error + Send + Sync>::from(
                            "owned listener did not close after two receiving peers",
                        ));
                    }
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
            }
            Ok(Box::new(std::iter::once(address)) as reqwest::dns::Addrs)
        })
    }
}

struct Server {
    child: Option<Child>,
    input: ChildStdin,
    reader: Option<std::thread::JoinHandle<()>>,
    replies: Receiver<serde_json::Value>,
    state: Arc<ConnectGate>,
    port: u16,
}
impl Server {
    fn start() -> Self {
        Self::start_fixture(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/b6_server.py"
        ))
    }
    fn start_fixture(script: &str) -> Self {
        let mut child = Command::new("python")
            .args(["-u", script])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("start owned HTTP fixture");
        let input = child.stdin.take().expect("owned fixture input");
        let output = child.stdout.take().expect("owned fixture output");
        let state = Arc::new(ConnectGate::default());
        let worker_state = Arc::clone(&state);
        let (sender, replies) = sync_channel(8);
        let reader = std::thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                let value: serde_json::Value =
                    serde_json::from_str(&line.expect("fixture frame")).expect("fixture JSON");
                if value["event"] == "closed" {
                    worker_state
                        .closed_epoch
                        .fetch_max(value["epoch"].as_u64().expect("epoch"), Ordering::AcqRel);
                } else if sender.send(value).is_err() {
                    break;
                }
            }
        });
        let ready = replies
            .recv_timeout(Duration::from_secs(5))
            .expect("fixture ready");
        assert_eq!(ready["event"], "ready");
        let port = u16::try_from(ready["port"].as_u64().expect("port")).expect("u16 port");
        println!("B6_SERVER_PID={}", child.id());
        Self {
            child: Some(child),
            input,
            reader: Some(reader),
            replies,
            state,
            port,
        }
    }
    fn command(&mut self, command: serde_json::Value, event: &str) -> serde_json::Value {
        writeln!(self.input, "{command}").expect("send fixture control");
        self.input.flush().expect("flush fixture control");
        let reply = self
            .replies
            .recv_timeout(Duration::from_secs(5))
            .expect("fixture control reply");
        assert_eq!(reply["event"], event);
        reply
    }
    fn round(&mut self, limit: u16) {
        self.state.calls.store(0, Ordering::Release);
        self.state.expected_epoch.store(
            self.state.closed_epoch.load(Ordering::Acquire) + 1,
            Ordering::Release,
        );
        self.state.gate_extra.store(limit == 2, Ordering::Release);
        self.command(
            serde_json::json!({"command":"round", "limit":limit, "allow_retirement":true}),
            "round",
        );
    }
    fn idle_stats(&mut self) -> serde_json::Value {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let reply = self.command(serde_json::json!({"command":"stats"}), "stats");
            assert!(
                reply["failure"].is_null(),
                "owned server failure: {}",
                reply["failure"]
            );
            if reply["active"] == 0 {
                return reply;
            }
            assert!(
                Instant::now() < deadline,
                "actual fixture streams did not retire"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    fn finish(&mut self) -> serde_json::Value {
        let reply = self.command(serde_json::json!({"command":"stop"}), "stopped");
        let mut child = self.child.take().expect("owned child");
        assert!(child.wait().expect("join HTTP fixture").success());
        self.reader
            .take()
            .expect("fixture reader")
            .join()
            .expect("join reader");
        assert_eq!(reply["active"], 0);
        assert!(reply["failure"].is_null());
        reply
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = writeln!(self.input, "{{\"command\":\"stop\"}}");
            let _ = self.input.flush();
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                if child.try_wait().ok().flatten().is_some() {
                    break;
                }
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

struct EngineGuard(DownloadEngine);
impl Drop for EngineGuard {
    fn drop(&mut self) {
        let _ = self.0.shutdown_and_wait();
    }
}
fn request(root: &Path, port: u16, ordinal: usize, width: u16) -> DownloadRequest {
    DownloadRequest {
        source_url: format!("http://b6.ceiling.test:{port}/file"),
        save_directory: root.to_owned(),
        save_directory_explicit: true,
        file_name: Some(format!("owned-{ordinal}.bin")),
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
fn exact_output(path: &Path) {
    let mut file = std::fs::File::open(path).expect("real completed artifact");
    assert_eq!(
        file.metadata().expect("artifact metadata").len(),
        8 * 1024 * 1024
    );
    let mut buffer = [0_u8; 65536];
    let mut offset = 0_usize;
    loop {
        let read = file.read(&mut buffer).expect("read output outside timing");
        if read == 0 {
            break;
        }
        for (index, byte) in buffer[..read].iter().enumerate() {
            assert_eq!(
                *byte,
                ((offset + index) % 251) as u8,
                "exact output offset {}",
                offset + index
            );
        }
        offset += read;
    }
    assert_eq!(offset, 8 * 1024 * 1024);
}
fn ref13_run(limit: u16, width: u16) -> serde_json::Value {
    let directory = tempfile::tempdir().expect("owned output root");
    let mut server = Server::start();
    let resolver = Arc::new(FixtureResolver {
        state: Arc::clone(&server.state),
        address: SocketAddr::from((Ipv4Addr::LOCALHOST, server.port)),
    });
    let http = HttpDownloadClient::new()
        .expect("client template")
        .with_fixture_resolver(resolver);
    let observations = http.clone();
    let mut network = NetworkSettings::default();
    network.proxy.mode = nxtools_core::NetworkProxyMode::Direct;
    let engine = EngineGuard(
        DownloadEngine::from_store_with_http_and_capacity_and_staging_root(
            SqliteDownloadStore::open_in_memory().expect("original SQLite"),
            http,
            Arc::new(SystemStorageCapacityProbe),
            None,
            None,
            NetworkPolicy::new(network).expect("direct snapshot"),
            DownloadHttpPolicy::from_download_settings(&DownloadSettings::default())
                .expect("original HTTP policy"),
        )
        .expect("original engine"),
    );
    let mut durations = Vec::new();
    let mut refusals = Vec::new();
    let mut outputs = Vec::new();
    println!("B6_MEASUREMENT_START");
    let batch = Instant::now();
    for ordinal in 0..4 {
        server.round(limit);
        let before = observations.fixture_error_counts();
        let started = Instant::now();
        let id = engine
            .0
            .submit(
                request(directory.path(), server.port, ordinal, width),
                RequestOrigin::Ui,
            )
            .expect("public submit");
        let deadline = Instant::now() + Duration::from_secs(120);
        loop {
            let current = engine
                .0
                .task(id)
                .expect("public task query")
                .expect("same task");
            match current.task.status() {
                DownloadStatus::Completed => break,
                DownloadStatus::Failed | DownloadStatus::Cancelled | DownloadStatus::Paused => {
                    panic!("unexpected terminal {:?}", current.task.status())
                }
                _ => assert!(Instant::now() < deadline, "bounded task deadline"),
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        durations.push(started.elapsed().as_micros() as u64);
        let after = observations.fixture_error_counts();
        assert_eq!(
            after.0 - before.0,
            after.1 - before.1,
            "only actual native ConnectionRefused is valid ceiling evidence"
        );
        refusals.push(after.1 - before.1);
        server.idle_stats();
        outputs.push(directory.path().join(format!("owned-{ordinal}.bin")));
    }
    let elapsed = batch.elapsed().as_micros() as u64;
    println!("B6_MEASUREMENT_STOP");
    engine
        .0
        .shutdown_and_wait()
        .expect("actual original worker drain");
    for path in &outputs {
        exact_output(path);
    }
    let stats = server.finish();
    let result = serde_json::json!({"ref":"REF-13", "limit":limit,"width":width,"elapsed_us":elapsed,"task_us":durations,"refusals":refusals,"server":stats,"exact_output":true,"workers_drained":true,"server_exited":true});
    println!("B6_RESULT={result}");
    result
}

#[test]
fn ref13_successful_width_prevents_repeated_extra_connection_refusals() {
    let result = ref13_run(2, 4);
    assert!(
        result["refusals"][0].as_u64().expect("training refusals") >= 2,
        "real initial refusals required"
    );
    for value in result["refusals"]
        .as_array()
        .expect("rounds")
        .iter()
        .skip(1)
    {
        assert_eq!(
            value.as_u64(),
            Some(0),
            "learned receiving width must prevent repeated refused connections"
        );
    }
}

#[test]
#[ignore = "explicit serial connection measurement"]
fn ref13_trial() {
    let scenario = std::env::var("B6_SCENARIO").expect("explicit experiment scenario");
    match scenario.as_str() {
        "limited" => {
            ref13_run(2, 4);
        }
        "unrestricted" => {
            ref13_run(64, 4);
        }
        "lower_cap" => {
            ref13_run(2, 1);
        }
        _ => panic!("unknown fixed scenario"),
    }
}

mod ref13_controls;

mod b601;
