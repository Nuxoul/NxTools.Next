//! Replaceable admitted I/O endpoints for the service-owned coordinator.
use super::*;

const ENDPOINT_OUTPUT_CAPACITY: usize = 16;
const ENDPOINT_IDLE_WAIT: Duration = Duration::from_millis(8);

#[derive(Clone)]
pub struct SharedIndexerRuntime(Arc<RuntimeHandle>);

struct RuntimeHandle {
    events: mpsc::SyncSender<RuntimeEvent>,
    stopping: Arc<AtomicBool>,
    coordinator: Mutex<Option<thread::JoinHandle<()>>>,
}
impl Drop for RuntimeHandle {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Release);
    }
}

impl SharedIndexerRuntime {
    pub fn start(service: IndexerService) -> Self {
        Self::with_sources(
            service,
            Arc::new(mft::scan_volume_and_persist),
            Arc::new(usn::poll_snapshot),
        )
    }

    pub(super) fn with_sources(
        mut service: IndexerService,
        scanner: Arc<SnapshotScanner>,
        journal_poller: Arc<JournalPoller>,
    ) -> Self {
        let (events, runtime) = mpsc::sync_channel(RUNTIME_QUEUE_CAPACITY);
        let stopping = Arc::new(AtomicBool::new(false));
        let coordinator_stopping = Arc::clone(&stopping);
        let coordinator_events = events.clone();
        let coordinator = thread::spawn(move || {
            let output = RuntimeOutput::<std::io::Sink>::shared();
            if run_coordinator(
                &mut service,
                scanner,
                journal_poller,
                output,
                coordinator_events,
                runtime,
                coordinator_stopping,
            )
            .is_err()
            {
                warn!(
                    event = "indexer.runtime.failed",
                    reason = "coordinator_failed",
                    "indexer coordinator ended"
                );
            }
        });
        Self(Arc::new(RuntimeHandle {
            events,
            stopping,
            coordinator: Mutex::new(Some(coordinator)),
        }))
    }

    /// Stop the service coordinator before SCM reports Stopped. The coordinator's
    /// endpoint drop signals native reader retirement; its search worker is joined.
    pub fn shutdown(&self) {
        self.0.stopping.store(true, Ordering::Release);
        let worker = self
            .0
            .coordinator
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .take();
        if let Some(worker) = worker {
            let _ = worker.join();
        }
    }
    #[cfg(test)]
    pub(super) fn events_for_test(&self) -> mpsc::SyncSender<RuntimeEvent> {
        self.0.events.clone()
    }

    #[cfg(windows)]
    pub fn serve_admitted_native<R, W>(
        &self,
        reader: R,
        writer: W,
        handshake: Envelope,
        handler: Arc<ServiceCommandHandler>,
        retired: Arc<AtomicBool>,
        signal: nxtools_ipc::PipeRetirement,
    ) -> Result<()>
    where
        R: BufRead + Send + 'static,
        W: Write + Send + 'static,
    {
        self.serve_admitted_inner(reader, writer, handshake, handler, retired, Some(signal))
    }

    /// The host authenticates the native identity and reads the deadline-bounded
    /// Hello before this entry. The caller owns one admission slot through both
    /// I/O workers' termination. Reader and writer must observe `retired`.
    pub fn serve_admitted<R, W>(
        &self,
        reader: R,
        writer: W,
        handshake: Envelope,
        handler: Arc<ServiceCommandHandler>,
        retired: Arc<AtomicBool>,
    ) -> Result<()>
    where
        R: BufRead + Send + 'static,
        W: Write + Send + 'static,
    {
        self.serve_admitted_inner(reader, writer, handshake, handler, retired, None)
    }

    fn serve_admitted_inner<R, W>(
        &self,
        reader: R,
        mut writer: W,
        handshake: Envelope,
        handler: Arc<ServiceCommandHandler>,
        retired: Arc<AtomicBool>,
        signal: Option<nxtools_ipc::PipeRetirement>,
    ) -> Result<()>
    where
        R: BufRead + Send + 'static,
        W: Write + Send + 'static,
    {
        if !matches!(handshake.message, Message::Hello(_)) {
            bail!("desktop did not begin the indexer session with hello");
        }
        let correlation_id = handshake.id.to_string();
        let started = Instant::now();
        let (frames, output) = mpsc::sync_channel(ENDPOINT_OUTPUT_CAPACITY);
        let (registered, registration) = mpsc::sync_channel(1);
        let endpoint = SessionEndpoint {
            lease: 0,
            frames,
            retired: Arc::clone(&retired),
            signal: signal.clone(),
            handler,
            correlation_id: handshake.id.to_string(),
            started: Instant::now(),
        };
        self.0
            .events
            .send(RuntimeEvent::Attach {
                endpoint,
                handshake,
                registered,
            })
            .context("indexer coordinator unavailable")?;
        let lease = registration.recv().context("indexer registration failed")?;
        let reader_events = self.0.events.clone();
        let reader_retired = Arc::clone(&retired);
        let reader_worker = thread::spawn(move || {
            pump_requests(reader, lease, reader_events, reader_retired);
        });
        let mut result = Ok(());
        while !retired.load(Ordering::Acquire) {
            match output.recv_timeout(ENDPOINT_IDLE_WAIT) {
                Ok(frame) => {
                    if let Err(error) = writer.write_all(&frame).and_then(|()| writer.flush()) {
                        if retired.load(Ordering::Acquire) {
                            break;
                        }
                        warn!(event = "indexer.session.io_failed", lease, %correlation_id,
                            reason = if error.kind() == std::io::ErrorKind::TimedOut { "write_deadline" } else { "write_failed" },
                            native_code = error.raw_os_error(), elapsed_ms = started.elapsed().as_millis() as u64,
                            "indexer endpoint write failed");
                        result = Err(error.into());
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        retired.store(true, Ordering::Release);
        if let Some(signal) = signal {
            signal.retire();
        }
        // The retirement event wakes the native reader directly before the join.
        reader_worker
            .join()
            .map_err(|_| anyhow::anyhow!("indexer reader worker panicked"))?;
        info!(event = "indexer.session.io_completed", lease, %correlation_id,
            elapsed_ms = started.elapsed().as_millis() as u64, "indexer endpoint workers released");
        result
    }
}

pub(super) struct SessionEndpoint {
    lease: u64,
    frames: mpsc::SyncSender<Vec<u8>>,
    retired: Arc<AtomicBool>,
    signal: Option<nxtools_ipc::PipeRetirement>,
    handler: Arc<ServiceCommandHandler>,
    correlation_id: String,
    started: Instant,
}

pub(super) struct RuntimeOutput<W> {
    writer: Option<W>,
    handler: Option<Arc<ServiceCommandHandler>>,
    endpoint: Option<SessionEndpoint>,
    next_lease: u64,
}

impl<W: Write> RuntimeOutput<W> {
    pub(super) fn standalone(writer: W, handler: Option<Arc<ServiceCommandHandler>>) -> Self {
        Self {
            writer: Some(writer),
            handler,
            endpoint: None,
            next_lease: 0,
        }
    }

    fn shared() -> Self {
        Self {
            writer: None,
            handler: None,
            endpoint: None,
            next_lease: 0,
        }
    }

    pub(super) fn persistent(&self) -> bool {
        self.writer.is_none()
    }

    pub(super) fn accepts(&self, lease: u64) -> bool {
        if self.writer.is_some() {
            return lease == 0;
        }
        self.endpoint.as_ref().is_some_and(|endpoint| {
            endpoint.lease == lease && !endpoint.retired.load(Ordering::Acquire)
        })
    }

    pub(super) fn command_handler(&self) -> Option<Arc<ServiceCommandHandler>> {
        self.endpoint
            .as_ref()
            .map(|endpoint| Arc::clone(&endpoint.handler))
            .or_else(|| self.handler.clone())
    }

    pub(super) fn attach(&mut self, mut endpoint: SessionEndpoint) -> u64 {
        self.retire("superseded");
        self.next_lease += 1;
        endpoint.lease = self.next_lease;
        info!(event = "indexer.session.accepted", lease = endpoint.lease,
            correlation_id = %endpoint.correlation_id, "indexer session registered");
        self.endpoint = Some(endpoint);
        self.next_lease
    }

    pub(super) fn retire(&mut self, reason: &'static str) {
        if let Some(endpoint) = self.endpoint.take() {
            endpoint.retired.store(true, Ordering::Release);
            if let Some(signal) = &endpoint.signal {
                signal.retire();
            }
            info!(event = "indexer.session.retired", lease = endpoint.lease,
                correlation_id = %endpoint.correlation_id, reason,
                elapsed_ms = endpoint.started.elapsed().as_millis() as u64,
                "indexer session retired");
        }
    }

    pub(super) fn reap_retired(&mut self) {
        if self
            .endpoint
            .as_ref()
            .is_some_and(|endpoint| endpoint.retired.load(Ordering::Acquire))
        {
            self.retire("io_closed");
        }
    }

    pub(super) fn reader_failed(&mut self, error: &anyhow::Error) {
        if let Some(endpoint) = &self.endpoint {
            warn!(event = "indexer.session.io_failed", lease = endpoint.lease,
                correlation_id = %endpoint.correlation_id, reason = "read_failed",
                native_code = error.downcast_ref::<std::io::Error>().and_then(std::io::Error::raw_os_error),
                elapsed_ms = endpoint.started.elapsed().as_millis() as u64,
                "indexer endpoint read failed");
        }
        self.retire("reader_failed");
    }
}

impl<W: Write> Write for RuntimeOutput<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if let Some(writer) = &mut self.writer {
            return writer.write(bytes);
        }
        if let Some(endpoint) = &self.endpoint {
            if endpoint.retired.load(Ordering::Acquire) {
                self.retire("io_closed");
            } else if let Err(error) = endpoint.frames.try_send(bytes.to_vec()) {
                self.retire(match error {
                    mpsc::TrySendError::Full(_) => "output_full",
                    mpsc::TrySendError::Disconnected(_) => "output_closed",
                });
            }
        }
        // Delivery failure retires only the endpoint. It cannot discard a
        // service-owned build/compaction result or terminate maintenance.
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.as_mut().map_or(Ok(()), Write::flush)
    }
}

impl<W> Drop for RuntimeOutput<W> {
    fn drop(&mut self) {
        if let Some(endpoint) = self.endpoint.take() {
            endpoint.retired.store(true, Ordering::Release);
            if let Some(signal) = &endpoint.signal {
                signal.retire();
            }
        }
    }
}

fn send_endpoint_event(
    events: &mpsc::SyncSender<RuntimeEvent>,
    mut event: RuntimeEvent,
    retired: &AtomicBool,
) -> bool {
    while !retired.load(Ordering::Acquire) {
        match events.try_send(event) {
            Ok(()) => return true,
            Err(mpsc::TrySendError::Disconnected(_)) => return false,
            Err(mpsc::TrySendError::Full(returned)) => {
                event = returned;
                thread::sleep(ENDPOINT_IDLE_WAIT);
            }
        }
    }
    false
}

pub(super) fn pump_requests<R: BufRead>(
    mut reader: R,
    lease: u64,
    events: mpsc::SyncSender<RuntimeEvent>,
    retired: Arc<AtomicBool>,
) {
    while !retired.load(Ordering::Acquire) {
        let event = match read_bounded_sync_frame(&mut reader) {
            Ok(Some(frame)) => match Envelope::decode_line(&frame) {
                Ok(request) => RuntimeEvent::Request { lease, request },
                Err(error) => RuntimeEvent::ReaderFailed {
                    lease,
                    error: error.into(),
                },
            },
            Ok(None) => RuntimeEvent::ReaderClosed { lease },
            Err(error) => RuntimeEvent::ReaderFailed { lease, error },
        };
        let terminal = !matches!(event, RuntimeEvent::Request { .. });
        if !send_endpoint_event(&events, event, &retired) || terminal {
            break;
        }
    }
}
#[cfg(all(test, windows))]
mod native_retirement_tests {
    use super::*;
    use nxtools_ipc::{accept_current_user_pipe, connect_local_pipe};

    #[test]
    fn coordinator_retirement_wakes_the_native_reader_without_peer_disconnect() {
        let name = format!(
            r"\\.\pipe\nxtools-indexer-retire-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );
        let server_name = name.clone();
        let (ready_tx, ready_rx) = mpsc::channel();
        let (finished_tx, finished_rx) = mpsc::channel();
        let server = thread::spawn(move || {
            let connection = accept_current_user_pipe(&server_name).expect("accept fixture");
            let retired = Arc::new(AtomicBool::new(false));
            let (mut reader, _writer, signal) = connection
                .into_retiring(Arc::clone(&retired), Duration::from_millis(500))
                .expect("native retirement event");
            let (frames, _receiver) = mpsc::sync_channel(1);
            let handler: Arc<ServiceCommandHandler> = Arc::new(|_| None);
            let mut output = RuntimeOutput::<std::io::Sink>::shared();
            output.endpoint = Some(SessionEndpoint {
                lease: 1,
                frames,
                retired,
                signal: Some(signal),
                handler,
                correlation_id: "synthetic".into(),
                started: Instant::now(),
            });
            ready_tx.send(()).expect("reader ready");
            let worker = thread::spawn(move || {
                let mut frame = String::new();
                reader.read_line(&mut frame).expect("retired EOF")
            });
            thread::sleep(Duration::from_millis(25));
            output.retire("superseded");
            finished_tx
                .send(worker.join().expect("reader worker"))
                .expect("retired result");
        });
        let client = connect_local_pipe(&name, Duration::from_secs(2)).expect("connect fixture");
        ready_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("server ready");
        let outcome = finished_rx.recv_timeout(Duration::from_millis(500));
        drop(client);
        server.join().expect("server worker");
        assert_eq!(outcome.expect("native wait must be signalled"), 0);
    }
}

#[cfg(test)]
mod shutdown_tests {
    use super::*;

    #[test]
    fn service_stop_joins_coordinator_even_while_a_session_clone_exists() {
        let runtime = SharedIndexerRuntime::with_sources(
            IndexerService::from_volume_descriptors([]),
            Arc::new(|_| anyhow::bail!("unexpected scan")),
            Arc::new(|_| anyhow::bail!("unexpected poll")),
        );
        let session = runtime.clone();
        let (finished, receiver) = mpsc::channel();
        thread::spawn(move || {
            runtime.shutdown();
            finished.send(()).expect("shutdown acknowledgement");
        });
        receiver
            .recv_timeout(Duration::from_secs(3))
            .expect("coordinator stopped");
        assert!(
            session
                .0
                .coordinator
                .lock()
                .expect("coordinator lock")
                .is_none()
        );
    }
}
