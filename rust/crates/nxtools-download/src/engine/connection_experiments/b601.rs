use super::*;

#[derive(Default)]
struct AttemptGate {
    calls: AtomicUsize,
    permitted: AtomicUsize,
}

struct Resolver {
    gate: Arc<AttemptGate>,
    address: SocketAddr,
}
impl reqwest::dns::Resolve for Resolver {
    fn resolve(&self, _name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let ordinal = self.gate.calls.fetch_add(1, Ordering::AcqRel);
        let gate = Arc::clone(&self.gate);
        let address = self.address;
        Box::pin(async move {
            if ordinal >= 3 {
                let deadline = Instant::now() + Duration::from_secs(15);
                while gate.permitted.load(Ordering::Acquire) < ordinal {
                    if Instant::now() >= deadline {
                        return Err("owned attempt gate deadline".into());
                    }
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
            }
            Ok(Box::new(std::iter::once(address)) as reqwest::dns::Addrs)
        })
    }
}

fn wait_for(mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !predicate() {
        assert!(
            Instant::now() < deadline,
            "owned connection boundary deadline"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn http_503_then_native_refusal_learns_and_retires_extra_worker_with_peer_progress() {
    let root = tempfile::tempdir().expect("owned output");
    let mut server = Server::start_fixture(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/b601_server.py"
    ));
    let gate = Arc::new(AttemptGate::default());
    let http = HttpDownloadClient::new()
        .expect("HTTP client")
        .with_fixture_resolver(Arc::new(Resolver {
            gate: Arc::clone(&gate),
            address: SocketAddr::from((Ipv4Addr::LOCALHOST, server.port)),
        }));
    let observations = http.clone();
    let mut settings = NetworkSettings::default();
    settings.proxy.mode = nxtools_core::NetworkProxyMode::Direct;
    let engine = EngineGuard(
        DownloadEngine::from_store_with_http_and_capacity_and_staging_root(
            SqliteDownloadStore::open_in_memory().expect("real SQLite"),
            http,
            Arc::new(SystemStorageCapacityProbe),
            None,
            None,
            NetworkPolicy::new(settings).expect("Direct policy"),
            DownloadHttpPolicy::from_download_settings(&DownloadSettings::default())
                .expect("HTTP policy"),
        )
        .expect("engine"),
    );
    let request = request(root.path(), server.port, 0, 2);
    let client = engine
        .0
        .inner
        .http_for_operation(Default::default())
        .expect("operation route");
    let ceiling = engine
        .0
        .inner
        .connection_ceiling(&client, &request.source_url);
    let task_id = engine
        .0
        .submit(request, RequestOrigin::Ui)
        .expect("public submit and durable task identity");

    // The first two connections probe the resource. Gate the target's first
    // request until the original peer has started sending its ranged payload.
    wait_for(|| gate.calls.load(Ordering::Acquire) == 4);
    wait_for(|| server.command(serde_json::json!({"command":"stats"}), "stats")["active"] == 1);
    gate.permitted.store(3, Ordering::Release);
    wait_for(|| gate.calls.load(Ordering::Acquire) == 5);
    wait_for(|| server.state.closed_epoch.load(Ordering::Acquire) == 1);
    let first_stats = server.command(serde_json::json!({"command":"stats"}), "stats");
    assert_eq!(first_stats["status503"], 1);
    assert_eq!(first_stats["peer_requests"], 1);
    assert_eq!(first_stats["active"], 1);
    assert!(first_stats["failure"].is_null());
    assert_eq!(observations.fixture_error_counts(), (0, 0));
    assert_eq!(
        connection_ceiling::limit(&ceiling, 2),
        2,
        "one 503 is not a learned cap"
    );
    let before_native = engine
        .0
        .task(task_id)
        .expect("task")
        .expect("same task")
        .downloaded_bytes;

    // Only the second refused attempt is released through DNS. It reaches the
    // closed native listener, provides the second observation and retires this worker.
    gate.permitted.store(4, Ordering::Release);
    wait_for(|| connection_ceiling::limit(&ceiling, 2) == 1);
    assert_eq!(observations.fixture_error_counts(), (1, 1));
    assert_eq!(
        gate.calls.load(Ordering::Acquire),
        5,
        "no third target connection attempt"
    );
    wait_for(|| {
        engine
            .0
            .task(task_id)
            .expect("task")
            .expect("same task")
            .downloaded_bytes
            > before_native
    });
    let peer_bytes = engine
        .0
        .task(task_id)
        .expect("task")
        .expect("same task")
        .downloaded_bytes;
    assert!(peer_bytes > before_native && peer_bytes < 4 * 1024 * 1024);
    let live = server.command(serde_json::json!({"command":"stats"}), "stats");
    assert_eq!(live["active"], 1);
    assert_eq!(live["status503"], 1);
    assert!(live["failure"].is_null());

    // The listener deliberately remains closed. Pause before the peer finishes,
    // so the public engine joins both worker and writer without another attempt.
    engine.0.pause(task_id).expect("public pause");
    wait_for(|| {
        engine
            .0
            .task(task_id)
            .expect("task")
            .expect("same task")
            .task
            .status()
            == DownloadStatus::Paused
    });
    engine
        .0
        .cancel(task_id)
        .expect("public cancel after writer drain");
    wait_for(|| {
        engine
            .0
            .task(task_id)
            .expect("task")
            .expect("same task")
            .task
            .status()
            == DownloadStatus::Cancelled
    });
    assert_eq!(gate.calls.load(Ordering::Acquire), 5);
    assert_eq!(connection_ceiling::limit(&ceiling, 2), 1);
    assert!(
        !root.path().join("owned-0.bin").exists(),
        "partial download is not published"
    );
    engine
        .0
        .shutdown_and_wait()
        .expect("all engine workers and writers joined");
    let stats = server.finish();
    assert_eq!(stats["accepted"], 4);
    assert_eq!(stats["requests"], 4);
    assert_eq!(stats["status503"], 1);
    assert_eq!(stats["maximum"], 1);
    assert_eq!(stats["listener_exited"], true);
    assert_eq!(stats["connection_threads_exited"], true);
    println!(
        "B601_RESULT={}",
        serde_json::json!({
            "sequence":["HTTP503","ConnectionRefused"], "target_attempts":2,
            "native_refusals":1, "peer_bytes_after_native":peer_bytes,
            "learned_width":connection_ceiling::limit(&ceiling, 2), "server":stats,
            "engine_drained":true, "server_process_and_reader_joined":true, "published":false
        })
    );
}
