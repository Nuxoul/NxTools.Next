use super::*;

fn direct_settings() -> NetworkSettings {
    let mut settings = NetworkSettings::default();
    settings.proxy.mode = nxtools_core::NetworkProxyMode::Direct;
    settings
}

fn engine(server: &Server) -> (EngineGuard, HttpDownloadClient) {
    let resolver = Arc::new(FixtureResolver {
        state: Arc::clone(&server.state),
        address: SocketAddr::from((Ipv4Addr::LOCALHOST, server.port)),
    });
    let http = HttpDownloadClient::new()
        .expect("original HTTP client")
        .with_fixture_resolver(resolver);
    let observations = http.clone();
    let engine = DownloadEngine::from_store_with_http_and_capacity_and_staging_root(
        SqliteDownloadStore::open_in_memory().expect("SQLite"),
        http,
        Arc::new(SystemStorageCapacityProbe),
        None,
        None,
        NetworkPolicy::new(direct_settings()).expect("direct policy"),
        DownloadHttpPolicy::from_download_settings(&DownloadSettings::default())
            .expect("HTTP policy"),
    )
    .expect("engine");
    (EngineGuard(engine), observations)
}

fn completed(engine: &EngineGuard, server: &mut Server, root: &Path, ordinal: usize, limit: u16) {
    server.round(limit);
    let id = engine
        .0
        .submit(request(root, server.port, ordinal, 4), RequestOrigin::Ui)
        .expect("public submit");
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        let task = engine.0.task(id).expect("query").expect("same task");
        match task.task.status() {
            DownloadStatus::Completed => break,
            DownloadStatus::Failed | DownloadStatus::Paused | DownloadStatus::Cancelled => {
                panic!("unexpected terminal {:?}", task.task.status());
            }
            _ => assert!(Instant::now() < deadline, "task deadline"),
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    server.idle_stats();
    exact_output(&root.join(format!("owned-{ordinal}.bin")));
}

#[test]
fn probe_statuses_without_assigned_peers_never_prove_a_connection_ceiling() {
    let root = tempfile::tempdir().expect("output");
    let mut server = Server::start();
    let (engine, _) = engine(&server);
    let client = engine
        .0
        .inner
        .http_for_operation(Default::default())
        .expect("routed client");
    let request = request(root.path(), server.port, 0, 4);
    let ceiling = engine
        .0
        .inner
        .connection_ceiling(&client, &request.source_url);
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    for status in [401, 403, 408, 425, 429, 500, 503] {
        server.command(
            serde_json::json!({"command":"round","limit":64,"status":status}),
            "round",
        );
        let failure = runtime
            .block_on(client.probe(&request))
            .expect_err("controlled HTTP error");
        assert!(matches!(failure, HttpDownloadError::Status(actual) if actual == status));
        assert_eq!(failure.is_refused_connection(), status == 403);
        assert_eq!(
            connection_ceiling::observe_refusal(&ceiling, &failure),
            None, // Probe failures have no assigned peer or receiving payload.
        );
        assert_eq!(connection_ceiling::limit(&ceiling, 4), 4);
    }
    for failure in [
        HttpDownloadError::LengthMismatch {
            expected_bytes: 8,
            downloaded_bytes: 4,
        },
        HttpDownloadError::ResourceValidatorMismatch {
            validator: "ETag",
            expected: "fixed".into(),
            actual: None,
        },
        HttpDownloadError::Stalled { timeout_ms: 1 },
        HttpDownloadError::Io(std::io::Error::from(std::io::ErrorKind::Interrupted)),
    ] {
        assert!(!failure.is_refused_connection());
        assert_eq!(
            connection_ceiling::observe_refusal(&ceiling, &failure),
            None
        );
    }
    let stats = server.finish();
    assert_eq!(stats["requests"], 21);
    let failure = runtime
        .block_on(client.probe(&request))
        .expect_err("closed listener");
    assert!(failure.is_refused_connection(), "typed real native refusal");
    for _ in 0..3 {
        assert_eq!(
            connection_ceiling::observe_refusal(&ceiling, &failure),
            None
        );
    }
    assert_eq!(
        connection_ceiling::limit(&ceiling, 4),
        4,
        "no receiving peer"
    );
    engine.0.shutdown_and_wait().expect("actual drain");
    println!(
        "B6_CONTROL={}",
        serde_json::json!({"ref":"REF-13","statuses":7,"probe_403_refused_without_peer":true,"native_refusal_without_peer":true,"server":stats,"drained":true})
    );
}

#[test]
#[ignore = "explicit real ten-minute observation"]
fn real_expiry_route_generation_and_recovery_release_the_learned_width() {
    let root = tempfile::tempdir().expect("output");
    let mut server = Server::start();
    let (engine, observations) = engine(&server);
    completed(&engine, &mut server, root.path(), 0, 2);
    assert_eq!(observations.fixture_error_counts().1, 2);
    let client = engine
        .0
        .inner
        .http_for_operation(Default::default())
        .expect("same route");
    let url = request(root.path(), server.port, 0, 4).source_url;
    let first = engine.0.inner.connection_ceiling(&client, &url);
    assert_eq!(connection_ceiling::limit(&first, 4), 2);
    completed(&engine, &mut server, root.path(), 1, 64);
    assert_eq!(
        server.idle_stats()["maximum"],
        2,
        "recovered server remains bounded until expiry"
    );
    engine
        .0
        .inner
        .network_policy
        .update(direct_settings())
        .expect("original route update");
    let changed_client = engine
        .0
        .inner
        .http_for_operation(Default::default())
        .expect("new route snapshot");
    let changed = engine.0.inner.connection_ceiling(&changed_client, &url);
    assert!(!Arc::ptr_eq(&first, &changed));
    assert_eq!(connection_ceiling::limit(&changed, 4), 4);
    completed(&engine, &mut server, root.path(), 2, 2);
    assert_eq!(
        observations.fixture_error_counts().1,
        4,
        "new generation trains independently"
    );
    assert_eq!(connection_ceiling::limit(&changed, 4), 2);
    let expiry_started = Instant::now();
    println!("B6_EXPIRY_WAIT_START");
    std::thread::sleep(Duration::from_secs(600));
    let waited_ms = expiry_started.elapsed().as_millis();
    assert!(waited_ms >= 600_000);
    assert_eq!(
        connection_ceiling::limit(&changed, 4),
        4,
        "actual ten-minute expiry"
    );
    completed(&engine, &mut server, root.path(), 3, 64);
    assert_eq!(
        server.idle_stats()["maximum"],
        4,
        "actual recovered receiving width"
    );
    assert_eq!(observations.fixture_error_counts().1, 4);
    engine.0.shutdown_and_wait().expect("actual drain");
    let stats = server.finish();
    println!(
        "B6_CONTROL={}",
        serde_json::json!({"ref":"REF-13","waited_ms":waited_ms,"route_independent":true,"recovered_width":4,"exact_output":true,"server":stats,"drained":true})
    );
}

fn wait_task(
    engine: &EngineGuard,
    id: DownloadTaskId,
    status: Option<DownloadStatus>,
) -> DownloadEngineTask {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let task = engine.0.task(id).expect("query").expect("same task");
        if status.map_or(task.downloaded_bytes > 0, |expected| {
            task.task.status() == expected
        }) {
            return task;
        }
        assert!(
            Instant::now() < deadline,
            "task status {:?}: {:?}",
            task.task.status(),
            task.error
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn learned_ceiling_preserves_pause_resume_plan_and_actual_cancel_drain() {
    let root = tempfile::tempdir().expect("output");
    let mut server = Server::start();
    let (engine, observations) = engine(&server);
    completed(&engine, &mut server, root.path(), 0, 2);
    let learned_refusals = observations.fixture_error_counts().1;
    assert!(learned_refusals >= 2);
    server.round(64);
    server.command(
        serde_json::json!({"command":"round","limit":64,"allow_retirement":true}),
        "round",
    );
    let id = engine
        .0
        .submit(request(root.path(), server.port, 1, 4), RequestOrigin::Ui)
        .expect("submit");
    wait_task(&engine, id, None);
    engine.0.pause(id).expect("pause");
    let paused = wait_task(&engine, id, Some(DownloadStatus::Paused));
    assert!(paused.downloaded_bytes > 0 && paused.downloaded_bytes < 8 * 1024 * 1024);
    server.idle_stats();
    let before = engine
        .0
        .inner
        .store
        .load_transfer_plan(id)
        .expect("durable plan")
        .expect("plan");
    let ranges = before
        .segments()
        .iter()
        .map(|segment| (segment.index(), segment.start_byte(), segment.end_byte()))
        .collect::<Vec<_>>();
    assert!(
        before
            .segments()
            .iter()
            .any(|segment| segment.downloaded_bytes() > 0)
    );
    engine.0.resume(id).expect("resume");
    let finished = wait_task(&engine, id, Some(DownloadStatus::Completed));
    assert_eq!(finished.task.id(), id);
    exact_output(&root.path().join("owned-1.bin"));
    let after = engine
        .0
        .inner
        .store
        .load_transfer_plan(id)
        .expect("same durable plan")
        .expect("same plan");
    for (index, start, end) in ranges {
        let retained = after
            .segments()
            .iter()
            .find(|segment| segment.index() == index)
            .expect("original persisted segment identity survives resume");
        assert_eq!(retained.start_byte(), start);
        assert!(retained.end_byte() <= end);
        let covered: u64 = after
            .segments()
            .iter()
            .filter(|segment| segment.start_byte() >= start && segment.end_byte() <= end)
            .map(DownloadSegment::len)
            .sum();
        assert_eq!(
            covered,
            end - start + 1,
            "new tails preserve saved coverage"
        );
    }
    server.idle_stats();
    server.round(64);
    server.command(
        serde_json::json!({"command":"round","limit":64,"allow_retirement":true}),
        "round",
    );
    let cancelled_id = engine
        .0
        .submit(request(root.path(), server.port, 2, 4), RequestOrigin::Ui)
        .expect("submit cancel task");
    wait_task(&engine, cancelled_id, None);
    engine.0.cancel(cancelled_id).expect("cancel accepted");
    wait_task(&engine, cancelled_id, Some(DownloadStatus::Cancelled));
    server.idle_stats();
    assert!(!root.path().join("owned-2.bin").exists());
    assert_eq!(
        observations.fixture_error_counts().1,
        learned_refusals,
        "no new refused connections"
    );
    engine.0.shutdown_and_wait().expect("actual worker join");
    let stats = server.finish();
    assert_eq!(stats["maximum"], 2);
    println!(
        "B6_CONTROL={}",
        serde_json::json!({"ref":"REF-13","pause_resume_exact":true,"original_segment_identity":true,"cancel_drained":true,"server":stats})
    );
}

#[test]
fn learned_direct_width_does_not_cross_actual_manual_proxy_route() {
    let root = tempfile::tempdir().expect("output");
    let mut server = Server::start();
    let (engine, observations) = engine(&server);
    completed(&engine, &mut server, root.path(), 0, 2);
    let learned_refusals = observations.fixture_error_counts().1;
    assert!(learned_refusals >= 2);
    let old_client = engine
        .0
        .inner
        .http_for_operation(Default::default())
        .expect("direct client");
    let url = request(root.path(), server.port, 0, 4).source_url;
    let old = engine.0.inner.connection_ceiling(&old_client, &url);
    assert_eq!(connection_ceiling::limit(&old, 4), 2);
    let mut network = direct_settings();
    network.proxy.mode = nxtools_core::NetworkProxyMode::ManualHttp;
    network.proxy.host = "b6.proxy.test".into();
    network.proxy.port = server.port;
    engine
        .0
        .inner
        .network_policy
        .update(network)
        .expect("actual proxy route update");
    let new_client = engine
        .0
        .inner
        .http_for_operation(Default::default())
        .expect("proxy client");
    assert_eq!(
        new_client.network_proxy_mode(),
        nxtools_core::NetworkProxyMode::ManualHttp
    );
    let new = engine.0.inner.connection_ceiling(&new_client, &url);
    assert!(!Arc::ptr_eq(&old, &new));
    assert_eq!(connection_ceiling::limit(&new, 4), 4);
    completed(&engine, &mut server, root.path(), 1, 64);
    assert_eq!(observations.fixture_error_counts().1, learned_refusals);
    engine.0.shutdown_and_wait().expect("actual drain");
    let stats = server.finish();
    assert_eq!(stats["maximum"], 4);
    assert!(
        stats["proxy_requests"]
            .as_u64()
            .expect("actual proxy requests")
            >= 6
    );
    println!(
        "B6_CONTROL={}",
        serde_json::json!({"ref":"REF-13","actual_route_independent":true,"exact_output":true,"server":stats,"drained":true})
    );
}

#[test]
fn active_run_workers_keep_the_same_observation_after_cache_eviction() {
    let root = tempfile::tempdir().expect("output");
    let mut server = Server::start();
    server.round(1);
    server.command(
        serde_json::json!({"command":"round","limit":1,"transfer_seconds":10,"allow_retirement":true}),
        "round",
    );
    let (engine, _) = engine(&server);
    let client = engine
        .0
        .inner
        .http_for_operation(Default::default())
        .expect("captured route");
    let request = request(root.path(), server.port, 0, 1);
    let ceiling = engine
        .0
        .inner
        .connection_ceiling(&client, &request.source_url);
    let id = engine
        .0
        .submit(request.clone(), RequestOrigin::Ui)
        .expect("public submit");
    let first = wait_task(&engine, id, None).downloaded_bytes;
    assert!(first > 0);
    assert_eq!(
        server.command(serde_json::json!({"command":"stats"}), "stats")["active"],
        1
    );
    assert!(
        Arc::strong_count(&ceiling) >= 3,
        "table, captured run and receiving worker retain the same observation"
    );

    for ordinal in 0..256 {
        engine
            .0
            .inner
            .connection_ceiling(&client, &format!("http://b6-eviction-{ordinal}.test/file"));
    }
    let subsequent_run = engine.0.inner.connection_ceiling(
        &client,
        &format!("http://b6.ceiling.test:{}/file", server.port),
    );
    assert!(
        !Arc::ptr_eq(&ceiling, &subsequent_run),
        "bounded cache evicted the active run entry"
    );
    assert_eq!(connection_ceiling::limit(&subsequent_run, 4), 4);
    assert!(
        Arc::strong_count(&ceiling) >= 3,
        "original run and payload lease survive eviction"
    );
    let closed_epoch = server.state.expected_epoch.load(Ordering::Acquire);
    let deadline = Instant::now() + Duration::from_secs(3);
    while server.state.closed_epoch.load(Ordering::Acquire) < closed_epoch {
        assert!(
            Instant::now() < deadline,
            "listener closes with receiving worker"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    let runtime = tokio::runtime::Runtime::new().expect("owned probe runtime");
    let refusal = runtime
        .block_on(client.probe(&request))
        .expect_err("real closed listener");
    assert!(
        refusal.native_connection_refusal().is_some(),
        "typed native refusal"
    );
    assert_eq!(
        connection_ceiling::observe_run_refusal(&ceiling, &refusal, 1, 1),
        None
    );
    assert_eq!(connection_ceiling::limit(&ceiling, 4), 4);
    assert_eq!(
        connection_ceiling::observe_run_refusal(&ceiling, &refusal, 1, 1),
        Some(1)
    );
    assert_eq!(connection_ceiling::limit(&ceiling, 4), 1);
    assert_eq!(connection_ceiling::limit(&subsequent_run, 4), 4);
    drop(subsequent_run);
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let task = engine.0.task(id).expect("query").expect("same task");
        if task.downloaded_bytes > first {
            assert!(
                Arc::strong_count(&ceiling) >= 3,
                "receiving worker still owns original observation"
            );
            break;
        }
        assert!(
            Instant::now() < deadline,
            "original worker progress after eviction"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    wait_task(&engine, id, Some(DownloadStatus::Completed));
    exact_output(&root.path().join("owned-0.bin"));
    server.idle_stats();
    engine
        .0
        .shutdown_and_wait()
        .expect("engine worker/writer drain");
    assert_eq!(
        Arc::strong_count(&ceiling),
        1,
        "original run and receiving lease released"
    );
    let stats = server.finish();
    assert_eq!(stats["maximum"], 1);
    println!(
        "B6_CONTROL={}",
        serde_json::json!({
            "ref":"REF-13", "cache_evicted":true, "same_run_observation":true,
            "original_worker_progress_after_eviction":true, "original_ceiling":1,
            "server":stats, "drained":true
        })
    );
}
