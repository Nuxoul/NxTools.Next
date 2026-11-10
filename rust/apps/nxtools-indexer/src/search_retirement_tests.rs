use super::*;

fn cancel_frame(search: &Envelope) -> String {
    let Message::IndexerCommand(IndexerCommand::Search(request)) = &search.message else {
        panic!("search request");
    };
    let mut frame = serde_json::to_value(Envelope::new(Message::Ping(Ping { sent_at_unix_ms: 0 })))
        .expect("envelope shape");
    frame["type"] = "indexer_command".into();
    frame["payload"] = serde_json::json!({"command":"cancel_search","data":{
        "request_id": search.id, "generation": request.generation
    }});
    serde_json::to_string(&frame).expect("cancel frame") + "\n"
}

struct RetirementFixture {
    input: Option<mpsc::Sender<Vec<u8>>>,
    release: Option<mpsc::Sender<()>>,
    started: mpsc::Receiver<()>,
    snapshots: std::sync::Weak<Vec<MftSnapshot>>,
    output: SharedWriter,
    session: Option<thread::JoinHandle<Result<()>>>,
}

impl RetirementFixture {
    fn new() -> Self {
        let mut service = IndexerService::from_volume_descriptors([]);
        service.snapshots = Arc::new(vec![test_snapshot(
            r"C:\",
            Some(1),
            Some(2),
            vec![test_entry(r"C:\fixture\needle.txt", "needle.txt", false)],
        )]);
        let snapshots = Arc::downgrade(&service.snapshots);
        let (started_sender, started) = mpsc::channel();
        let (release, gate) = mpsc::channel();
        let gate = Mutex::new(gate);
        let calls = AtomicUsize::new(0);
        service.search_observer = Some(Arc::new(move |_| {
            if calls.fetch_add(1, AtomicOrdering::SeqCst) == 0 {
                started_sender.send(()).expect("worker owns active job");
                let _ = gate.lock().expect("worker gate").recv();
            }
        }));
        let (input, receiver) = mpsc::channel();
        let output = SharedWriter::default();
        let writer = output.clone();
        let session = thread::spawn(move || {
            serve_with_scanner(
                BufReader::new(ChannelReader::new(receiver)),
                writer,
                service,
                Arc::new(|_| panic!("search fixture must not scan a volume")),
            )
        });
        let fixture = Self {
            input: Some(input),
            release: Some(release),
            started,
            snapshots,
            output,
            session: Some(session),
        };
        fixture.send(m4_hello());
        fixture.fence();
        fixture
    }

    fn send(&self, request: Envelope) {
        self.send_wire(request.encode_line().expect("request frame"));
    }

    fn send_wire(&self, wire: String) {
        self.input
            .as_ref()
            .expect("live input")
            .send(wire.into_bytes())
            .expect("input frame");
    }

    fn fence(&self) {
        let ping = Envelope::new(Message::Ping(Ping { sent_at_unix_ms: 1 }));
        self.send(ping.clone());
        wait_for_frame(&self.output, |frame| frame.correlation_id == Some(ping.id));
    }

    fn wait_references(&self, expected: usize) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while self.snapshots.strong_count() != expected && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(
            self.snapshots.strong_count(),
            expected,
            "only service and genuinely live jobs may retain the snapshot"
        );
    }

    fn release_worker(&mut self) {
        self.release
            .take()
            .expect("first worker gate")
            .send(())
            .expect("release worker");
    }
}

impl Drop for RetirementFixture {
    fn drop(&mut self) {
        self.release.take();
        self.input.take();
        if let Some(session) = self.session.take() {
            session
                .join()
                .expect("fixture coordinator")
                .expect("fixture session");
        }
    }
}

#[test]
fn search_retirement_active_cancel_frame_releases_worker_snapshot() {
    let mut fixture = RetirementFixture::new();
    let search = m4_search(4, "needle");
    fixture.send(search.clone());
    fixture
        .started
        .recv_timeout(Duration::from_secs(2))
        .expect("active search");
    fixture.wait_references(2);
    let wire = cancel_frame(&search);
    Envelope::decode_line(&wire).expect("service must decode the Desktop cancellation contract");
    fixture.send_wire(wire);
    fixture.fence();
    fixture.release_worker();
    fixture.wait_references(1);
    assert!(
        !m4_frames(&fixture.output)
            .iter()
            .any(|frame| frame.correlation_id == Some(search.id)),
        "cancelled active job must not publish a terminal batch"
    );
}

#[test]
fn search_retirement_pending_cancel_frame_releases_snapshot_without_cancelling_new_query() {
    let mut fixture = RetirementFixture::new();
    fixture.send(m4_search(4, "needle"));
    fixture
        .started
        .recv_timeout(Duration::from_secs(2))
        .expect("active search");
    let pending = m4_search(5, "needle");
    fixture.send(pending.clone());
    fixture.fence();
    fixture.wait_references(3);
    let wire = cancel_frame(&pending);
    Envelope::decode_line(&wire).expect("service must decode the Desktop cancellation contract");
    fixture.send_wire(wire.clone());
    fixture.fence();
    fixture.wait_references(2);
    let newer = m4_search(6, "needle");
    fixture.send(newer.clone());
    fixture.send_wire(wire);
    fixture.fence();
    fixture.wait_references(3);
    fixture.release_worker();
    wait_for_frame(&fixture.output, |frame| {
        frame.correlation_id == Some(newer.id)
    });
    fixture.wait_references(1);
    assert!(
        !m4_frames(&fixture.output)
            .iter()
            .any(|frame| frame.correlation_id == Some(pending.id)),
        "cancelled pending job must never execute or publish"
    );
}

#[test]
fn search_retirement_backpressure_releases_job_and_matches_lease_id_and_generation() {
    let (events, runtime) = mpsc::sync_channel(1);
    events
        .send(RuntimeEvent::ReaderClosed { lease: 0 })
        .expect("full completion queue");
    let active = Arc::new(AtomicU64::new(0));
    let executor = SearchExecutor::start(Arc::clone(&active), events);
    let (started, picked_up) = mpsc::channel();
    executor.state.0.lock().expect("worker state").observer = Some(Arc::new(move |_| {
        let _ = started.send(());
    }));
    let snapshots = Arc::new(vec![test_snapshot(
        r"C:\",
        Some(1),
        Some(2),
        vec![test_entry(r"C:\fixture\needle.txt", "needle.txt", false)],
    )]);
    let weak = Arc::downgrade(&snapshots);
    let request = m4_search(3, "needle");
    let cancel = nxtools_protocol::IndexSearchCancel {
        request_id: request.id,
        generation: 3,
    };
    executor.submit(SearchJob {
        lease: 8,
        task_id: 3,
        request,
        snapshots,
        settings: IndexSettings::default(),
    });
    picked_up
        .recv_timeout(Duration::from_secs(2))
        .expect("actual worker picked up job");
    thread::sleep(Duration::from_millis(10));
    assert_eq!(
        weak.strong_count(),
        1,
        "full completion queue still retains the active job"
    );
    assert!(!executor.cancel(7, &cancel));
    assert!(!executor.cancel(
        8,
        &nxtools_protocol::IndexSearchCancel {
            generation: 2,
            ..cancel
        }
    ));
    assert!(!executor.cancel(
        8,
        &nxtools_protocol::IndexSearchCancel {
            request_id: m4_search(3, "other").id,
            ..cancel
        }
    ));
    assert_eq!(
        active.load(Ordering::Acquire),
        3,
        "unrelated cancellation cannot change the current task"
    );
    assert!(executor.cancel(8, &cancel));
    let deadline = Instant::now() + Duration::from_secs(2);
    while weak.strong_count() != 0 && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(
        weak.strong_count(),
        0,
        "cancel must release the blocked job without draining output or closing executor"
    );
    assert!(matches!(
        runtime.try_recv().expect("original queue filler"),
        RuntimeEvent::ReaderClosed { .. }
    ));
    executor.submit(SearchJob {
        lease: 8,
        task_id: 4,
        request: m4_search(4, "next"),
        snapshots: Arc::new(Vec::new()),
        settings: IndexSettings::default(),
    });
    assert!(matches!(
        runtime
            .recv_timeout(Duration::from_secs(2))
            .expect("same worker handles the next query"),
        RuntimeEvent::SearchCompleted { task_id: 4, .. }
    ));
}
