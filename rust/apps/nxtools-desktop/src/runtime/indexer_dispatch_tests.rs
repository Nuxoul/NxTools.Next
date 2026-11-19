use super::*;
use nxtools_protocol::{
    HelloAck, IndexPhase, IndexSearchBatch, IndexSearchCancel, IndexSearchContext, IndexSearchItem,
    IndexSearchRequest, IndexStatus,
};
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::net::windows::named_pipe::ServerOptions;

enum PeerAction {
    Disconnect,
    Publish(IndexerEvent),
}

struct PipeFixture {
    bridge: IndexerBridge,
    observed: Receiver<Envelope>,
    stop: Option<AsyncSender<()>>,
    finished: Receiver<()>,
    release: AsyncSender<()>,
    actions: AsyncSender<PeerAction>,
    runtime: tokio::runtime::Runtime,
    supervisor: Option<JoinHandle<()>>,
    server: Option<JoinHandle<()>>,
}

impl PipeFixture {
    fn start() -> Self {
        Self::with_sessions(1)
    }

    fn with_sessions(sessions: usize) -> Self {
        let pipe = format!(
            r"\\.\pipe\nxtools-indexer-dispatch-{}",
            uuid::Uuid::new_v4()
        );
        let server_pipe = pipe.clone();
        let (listening, listening_receiver) = mpsc::channel();
        let (observed, observed_receiver) = mpsc::channel();
        let (release, released) = async_channel::bounded(1);
        let (actions, action_receiver) = async_channel::bounded(8);
        let server = thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("fixture pipe runtime");
            runtime.block_on(async move {
                for session in 0..sessions {
                    let server = ServerOptions::new()
                        .first_pipe_instance(session == 0)
                        .create(&server_pipe)
                        .expect("create fixture pipe");
                    if session == 0 {
                        listening.send(()).expect("announce fixture pipe");
                    }
                    tokio::select! {
                        biased;
                        _ = released.recv() => return,
                        result = tokio::time::timeout(Duration::from_secs(3), server.connect()) => {
                            result.expect("fixture connection deadline").expect("connect fixture desktop");
                        }
                    }
                    let mut reader = tokio::io::BufReader::new(server);
                    let mut line = String::new();
                    tokio::time::timeout(Duration::from_secs(3), reader.read_line(&mut line))
                        .await
                        .expect("fixture Hello deadline")
                        .expect("read actual desktop Hello");
                    let hello = Envelope::decode_line(&line).expect("decode actual desktop Hello");
                    assert!(matches!(hello.message, Message::Hello(_)));
                    for envelope in [
                        Envelope::reply_to(
                            &hello,
                            Message::HelloAck(HelloAck {
                                server_name: "dispatch_fixture".into(),
                                server_version: "1".into(),
                                accepted_capabilities: vec!["file_search".into()],
                            }),
                        ),
                        Envelope::new(Message::IndexerEvent(IndexerEvent::Status(IndexStatus {
                            phase: IndexPhase::Ready,
                            indexed_items: 1,
                            drives: Vec::new(),
                            last_error: None,
                        }))),
                    ] {
                        let frame = envelope.encode_line().expect("encode fixture event");
                        reader
                            .get_mut()
                            .write_all(frame.as_bytes())
                            .await
                            .expect("write fixture event");
                        reader.get_mut().flush().await.expect("flush fixture event");
                    }
                    loop {
                        line.clear();
                        tokio::select! {
                            biased;
                            _ = released.recv() => return,
                            action = action_receiver.recv() => match action.expect("fixture action") {
                                PeerAction::Disconnect => break,
                                PeerAction::Publish(event) => {
                                    let frame = Envelope::new(Message::IndexerEvent(event))
                                        .encode_line().expect("encode fixture publication");
                                    reader.get_mut().write_all(frame.as_bytes()).await
                                        .expect("publish actual event frame");
                                    reader.get_mut().flush().await.expect("flush publication");
                                }
                            },
                            result = reader.read_line(&mut line) => {
                                let size = result.expect("read original command frame");
                                if size == 0 { return; }
                                let envelope = Envelope::decode_line(&line).expect("decode original command");
                                observed.send(envelope).expect("observe command at peer");
                            }
                        }
                    }
                }
            });
        });
        listening_receiver
            .recv_timeout(Duration::from_secs(3))
            .expect("fixture server is listening");
        let (bridge, endpoint) = indexer_bridge(8);
        let (stop, stop_receiver) = async_channel::bounded(1);
        let (finished, finished_receiver) = mpsc::channel();
        let supervisor = thread::spawn(move || {
            supervise_indexer_service_at(&pipe, endpoint, stop_receiver);
            let _ = finished.send(());
        });
        let fixture = Self {
            bridge,
            observed: observed_receiver,
            stop: Some(stop),
            finished: finished_receiver,
            release,
            actions,
            runtime: tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()
                .expect("fixture event runtime"),
            supervisor: Some(supervisor),
            server: Some(server),
        };
        fixture.expect_ready();
        fixture
    }

    fn next_notification(&self) -> IndexerNotification {
        self.runtime.block_on(async {
            tokio::time::timeout(Duration::from_secs(3), self.bridge.events.recv())
                .await
                .expect("actual event notification deadline")
                .expect("actual event channel remains open")
        })
    }

    fn expect_ready(&self) {
        assert!(matches!(
            self.next_notification(),
            IndexerNotification::Event(IndexerEvent::Status(IndexStatus {
                phase: IndexPhase::Ready,
                ..
            }))
        ));
        assert!(self.bridge.is_connected(), "real HelloAck admitted desktop");
    }

    fn expect_stopped(&self) {
        self.finished
            .recv_timeout(Duration::from_millis(250))
            .expect("idle supervisor stops without waiting for pipe input or retry deadline");
        assert!(!self.bridge.is_connected());
    }
}

impl Drop for PipeFixture {
    fn drop(&mut self) {
        if let Some(stop) = &self.stop {
            let _ = stop.try_send(());
        }
        let _ = self.release.try_send(());
        if let Some(supervisor) = self.supervisor.take() {
            supervisor.join().expect("fixture supervisor did not panic");
        }
        if let Some(server) = self.server.take() {
            server.join().expect("fixture server did not panic");
        }
    }
}

fn search_request(generation: u64) -> IndexSearchRequest {
    IndexSearchRequest {
        generation,
        query: format!("fixture-{generation}"),
        limit: 20,
        context: IndexSearchContext {
            include_files: true,
            include_directories: true,
            ..Default::default()
        },
    }
}

#[test]
fn indexer_search_dispatch_wakes_without_periodic_tick() {
    let fixture = PipeFixture::start();
    let request = search_request(41);
    let started = Instant::now();
    let request_id = fixture
        .bridge
        .send_command(IndexerCommand::Search(request.clone()))
        .expect("admit actual search command");
    let observed = fixture.observed.recv_timeout(Duration::from_millis(50));
    assert!(
        observed.is_ok(),
        "an admitted search must reach the real pipe peer without a 100 ms command-pump tick"
    );
    let observed = observed.expect("timely command frame");
    eprintln!("search enqueue-to-peer: {:?}", started.elapsed());
    assert_eq!(observed.id, request_id);
    assert_eq!(
        observed.message,
        Message::IndexerCommand(IndexerCommand::Search(request))
    );
}

#[test]
fn indexer_cancel_dispatch_wakes_without_a_followup_search() {
    let fixture = PipeFixture::start();
    let request_id = fixture
        .bridge
        .send_command(IndexerCommand::Search(search_request(41)))
        .expect("admit original search");
    fixture
        .observed
        .recv_timeout(Duration::from_secs(1))
        .expect("original search reached peer");
    let cancel = IndexSearchCancel {
        request_id,
        generation: 41,
    };
    let started = Instant::now();
    fixture
        .runtime
        .block_on(fixture.bridge.cancel_search(cancel))
        .expect("admit original asynchronous cancellation");
    let observed = fixture
        .observed
        .recv_timeout(Duration::from_millis(50))
        .expect("cancellation alone wakes the command receiver");
    eprintln!("cancel enqueue-to-peer: {:?}", started.elapsed());
    assert_eq!(observed.correlation_id, Some(request_id));
    assert_eq!(
        observed.message,
        Message::IndexerCommand(IndexerCommand::CancelSearch(cancel))
    );
}

#[test]
fn indexer_dispatch_preserves_command_order_and_query_identity() {
    let fixture = PipeFixture::start();
    let first = search_request(41);
    let first_id = fixture
        .bridge
        .send_command(IndexerCommand::Search(first.clone()))
        .expect("first query");
    let cancel = IndexSearchCancel {
        request_id: first_id,
        generation: first.generation,
    };
    fixture
        .runtime
        .block_on(fixture.bridge.cancel_search(cancel))
        .expect("retire first query");
    let second = search_request(42);
    let second_id = fixture
        .bridge
        .send_command(IndexerCommand::Search(second.clone()))
        .expect("replacement query");
    let expected = [
        Message::IndexerCommand(IndexerCommand::Search(first)),
        Message::IndexerCommand(IndexerCommand::CancelSearch(cancel)),
        Message::IndexerCommand(IndexerCommand::Search(second)),
    ];
    for (index, expected) in expected.into_iter().enumerate() {
        let observed = fixture
            .observed
            .recv_timeout(Duration::from_secs(1))
            .expect("every command reached actual peer");
        assert_eq!(observed.message, expected);
        match index {
            0 => assert_eq!(observed.id, first_id),
            1 => assert_eq!(observed.correlation_id, Some(first_id)),
            2 => assert_eq!(observed.id, second_id),
            _ => unreachable!(),
        }
    }
}

#[test]
fn indexer_idle_dispatch_keeps_partial_and_terminal_reader_events() {
    let fixture = PipeFixture::start();
    for (generation, complete) in [(41, false), (42, false), (42, true)] {
        let event = IndexerEvent::SearchBatch(IndexSearchBatch {
            generation,
            items: vec![IndexSearchItem {
                path: format!(r"C:\fixture\result-{generation}.txt"),
                name: format!("result-{generation}.txt"),
                is_directory: false,
                size_bytes: Some(32),
                modified_at_unix_ms: None,
                score: 100,
            }],
            complete,
            elapsed_us: 1000,
        });
        fixture
            .actions
            .try_send(PeerAction::Publish(event.clone()))
            .expect("publish actual partial/terminal frame");
        match fixture.next_notification() {
            IndexerNotification::Event(observed) => assert_eq!(observed, event),
            IndexerNotification::Disconnected => panic!("reader disconnected during publication"),
        }
    }
}

#[test]
fn indexer_idle_reader_disconnect_reconnects_without_a_command() {
    let fixture = PipeFixture::with_sessions(2);
    fixture
        .actions
        .try_send(PeerAction::Disconnect)
        .expect("close actual service-side pipe");
    assert!(matches!(
        fixture.next_notification(),
        IndexerNotification::Disconnected
    ));
    assert!(!fixture.bridge.is_connected());
    assert!(
        fixture
            .bridge
            .send_command(IndexerCommand::RefreshVolumes)
            .is_err()
    );
    fixture.expect_ready();
    assert!(fixture.observed.try_recv().is_err(), "no stale replay");
    let request = search_request(43);
    let request_id = fixture
        .bridge
        .send_command(IndexerCommand::Search(request.clone()))
        .expect("admit query after real reconnect handshake");
    let observed = fixture
        .observed
        .recv_timeout(Duration::from_millis(50))
        .expect("reconnected command receiver is event-driven");
    assert_eq!(observed.id, request_id);
    assert_eq!(
        observed.message,
        Message::IndexerCommand(IndexerCommand::Search(request))
    );
}

#[test]
fn indexer_idle_stop_wakes_without_pipe_input() {
    let fixture = PipeFixture::start();
    fixture
        .stop
        .as_ref()
        .expect("stop sender")
        .try_send(())
        .expect("stop");
    fixture.expect_stopped();
}

#[test]
fn indexer_stop_sender_closure_wakes_without_pipe_input() {
    let mut fixture = PipeFixture::start();
    drop(fixture.stop.take());
    fixture.expect_stopped();
}

#[test]
fn indexer_stop_interrupts_reconnect_backoff() {
    let fixture = PipeFixture::with_sessions(2);
    fixture
        .actions
        .try_send(PeerAction::Disconnect)
        .expect("close service-side pipe before retry");
    assert!(matches!(
        fixture.next_notification(),
        IndexerNotification::Disconnected
    ));
    fixture
        .stop
        .as_ref()
        .expect("stop sender")
        .try_send(())
        .expect("stop during retry");
    fixture.expect_stopped();
}
