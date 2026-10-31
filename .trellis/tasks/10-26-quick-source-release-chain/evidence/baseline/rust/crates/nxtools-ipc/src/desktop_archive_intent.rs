//! Bounded Rust-only desktop handoff on the existing platform listener.
use async_channel::{Receiver, Sender};
use nxtools_protocol::desktop_archive_intent::{
    DesktopArchiveIntent, DesktopArchiveReceipt, LaunchFailure, LaunchReceiptState, MAX_PENDING,
};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use uuid::Uuid;

pub const READINESS_TIMEOUT: Duration = Duration::from_secs(10);
pub const COMPLETION_TIMEOUT: Duration = Duration::from_secs(15);
const REPLAY_TTL: Duration = Duration::from_secs(60);
const MAX_REPLAY: usize = 128;

struct Record {
    intent: DesktopArchiveIntent,
    receipt: DesktopArchiveReceipt,
    accepted: Instant,
    dispatched: bool,
}
struct State {
    records: HashMap<Uuid, Record>,
    order: VecDeque<Uuid>,
    closed: bool,
}
#[derive(Clone)]
pub struct ArchiveIntentEndpoint {
    state: Arc<Mutex<State>>,
    sender: Sender<Uuid>,
}
pub struct ArchiveIntentReceiver {
    endpoint: ArchiveIntentEndpoint,
    receiver: Receiver<Uuid>,
}
#[derive(Clone)]
pub struct LaunchCompletion {
    endpoint: ArchiveIntentEndpoint,
    id: Uuid,
}

pub fn archive_intent_channel() -> (ArchiveIntentEndpoint, ArchiveIntentReceiver) {
    let (sender, receiver) = async_channel::bounded(MAX_PENDING);
    let endpoint = ArchiveIntentEndpoint {
        state: Arc::new(Mutex::new(State {
            records: HashMap::new(),
            order: VecDeque::new(),
            closed: false,
        })),
        sender,
    };
    (
        endpoint.clone(),
        ArchiveIntentReceiver { endpoint, receiver },
    )
}
impl ArchiveIntentEndpoint {
    // Transport MUST authorize actual Windows peer before calling this method.
    pub fn submit(&self, intent: DesktopArchiveIntent) -> DesktopArchiveReceipt {
        self.submit_at(intent, Instant::now())
    }
    fn submit_at(&self, intent: DesktopArchiveIntent, now: Instant) -> DesktopArchiveReceipt {
        let id = intent.request_id;
        if let Err(failure) = intent.validate() {
            return DesktopArchiveReceipt::new(id, Err(failure));
        }
        let Ok(mut state) = self.state.lock() else {
            return DesktopArchiveReceipt::new(id, Err(LaunchFailure::Disconnected));
        };
        sweep(&mut state, now);
        if let Some(record) = state.records.get(&id) {
            return if record.intent == intent {
                record.receipt.clone()
            } else {
                DesktopArchiveReceipt::new(id, Err(LaunchFailure::DuplicateId))
            };
        }
        if state.closed || self.sender.is_closed() {
            return DesktopArchiveReceipt::new(id, Err(LaunchFailure::Disconnected));
        }
        if state
            .records
            .values()
            .filter(|record| record.receipt.state == LaunchReceiptState::Queued)
            .count()
            >= MAX_PENDING
        {
            return DesktopArchiveReceipt::new(id, Err(LaunchFailure::QueueFull));
        }
        // Never evict a live replay ID early: within 60s, lost acknowledgements cannot
        // start a second job even under pressure. New unique IDs get typed backpressure.
        if state.records.len() >= MAX_REPLAY {
            return DesktopArchiveReceipt::new(id, Err(LaunchFailure::QueueFull));
        }
        let receipt = DesktopArchiveReceipt::new(id, Ok(false));
        state.records.insert(
            id,
            Record {
                intent,
                receipt: receipt.clone(),
                accepted: now,
                dispatched: false,
            },
        );
        state.order.push_back(id);
        // Ownership is installed under this same lock before becoming observable.
        if let Err(error) = self.sender.try_send(id) {
            state.records.remove(&id);
            state.order.retain(|value| *value != id);
            return DesktopArchiveReceipt::new(
                id,
                Err(if error.is_closed() {
                    LaunchFailure::Disconnected
                } else {
                    LaunchFailure::QueueFull
                }),
            );
        }
        tracing::info!(event = "archive.launch.accepted", request_id = %id, "archive launch accepted");
        receipt
    }
    pub fn receipt(&self, id: Uuid) -> Option<DesktopArchiveReceipt> {
        let mut state = self.state.lock().ok()?;
        sweep(&mut state, Instant::now());
        state.records.get(&id).map(|record| record.receipt.clone())
    }
    pub fn expire(&self) {
        if let Ok(mut state) = self.state.lock() {
            sweep(&mut state, Instant::now());
        }
    }
    pub(crate) fn native_disconnected(&self) {
        if let Ok(mut state) = self.state.lock() {
            for record in state
                .records
                .values_mut()
                .filter(|r| r.receipt.state == LaunchReceiptState::Queued)
            {
                finish_record(record, Err(LaunchFailure::Disconnected));
            }
        }
    }
    pub fn disconnect(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.closed = true;
            for record in state
                .records
                .values_mut()
                .filter(|r| r.receipt.state == LaunchReceiptState::Queued)
            {
                finish_record(record, Err(LaunchFailure::Disconnected));
            }
        }
        self.sender.close();
    }
}
fn finish_record(record: &mut Record, result: Result<(), LaunchFailure>) {
    record.receipt = DesktopArchiveReceipt::new(record.intent.request_id, result.map(|()| true));
    tracing::info!(event = "archive.launch.finished", request_id = %record.intent.request_id, state = ?record.receipt.state, failure = ?record.receipt.failure, elapsed_ms = record.accepted.elapsed().as_millis(), "archive launch handoff finished");
}
fn sweep(state: &mut State, now: Instant) {
    for record in state.records.values_mut() {
        let elapsed = now.saturating_duration_since(record.accepted);
        if record.receipt.state == LaunchReceiptState::Queued {
            if !record.dispatched && elapsed >= READINESS_TIMEOUT {
                finish_record(record, Err(LaunchFailure::ReadinessTimeout));
            } else if elapsed >= COMPLETION_TIMEOUT {
                finish_record(record, Err(LaunchFailure::TransportTimeout));
            }
        }
    }
    state.order.retain(|id| {
        let retain = state.records.get(id).is_some_and(|record| {
            record.receipt.state == LaunchReceiptState::Queued
                || now.saturating_duration_since(record.accepted) < REPLAY_TTL
        });
        if !retain {
            state.records.remove(id);
        }
        retain
    });
}
impl ArchiveIntentReceiver {
    pub fn expire(&self) {
        self.endpoint.expire();
    }
    pub fn try_next(&self) -> Option<(DesktopArchiveIntent, LaunchCompletion)> {
        while let Ok(id) = self.receiver.try_recv() {
            let mut state = self.endpoint.state.lock().ok()?;
            sweep(&mut state, Instant::now());
            let Some(record) = state.records.get_mut(&id) else {
                continue;
            };
            if record.receipt.state != LaunchReceiptState::Queued || record.dispatched {
                continue;
            }
            record.dispatched = true;
            tracing::info!(event = "archive.launch.started", request_id = %id, elapsed_ms = record.accepted.elapsed().as_millis(), "archive launch handed to owner");
            return Some((
                record.intent.clone(),
                LaunchCompletion {
                    endpoint: self.endpoint.clone(),
                    id,
                },
            ));
        }
        None
    }
}
impl Drop for ArchiveIntentReceiver {
    fn drop(&mut self) {
        self.endpoint.disconnect();
    }
}
impl LaunchCompletion {
    pub fn is_pending(&self) -> bool {
        self.endpoint
            .receipt(self.id)
            .is_some_and(|r| r.state == LaunchReceiptState::Queued)
    }
    pub fn finish(&self, result: Result<(), LaunchFailure>) -> bool {
        let Ok(mut state) = self.endpoint.state.lock() else {
            return false;
        };
        sweep(&mut state, Instant::now());
        let Some(record) = state.records.get_mut(&self.id) else {
            return false;
        };
        if record.receipt.state != LaunchReceiptState::Queued {
            return false;
        }
        finish_record(record, result);
        true
    }
}

#[cfg(windows)]
pub(crate) async fn serve<W: tokio::io::AsyncWrite + Unpin>(
    request: nxtools_protocol::Envelope,
    endpoint: Option<&ArchiveIntentEndpoint>,
    authorized: bool,
    writer: &mut W,
) -> anyhow::Result<()> {
    use nxtools_protocol::{Envelope, Message};
    let Message::DesktopArchiveIntent(intent) = &request.message else {
        anyhow::bail!("invalid archive launch message");
    };
    let receipt = if !authorized {
        DesktopArchiveReceipt::new(intent.request_id, Err(LaunchFailure::Unauthorized))
    } else if let Some(endpoint) = endpoint {
        endpoint.submit(intent.clone())
    } else {
        DesktopArchiveReceipt::new(intent.request_id, Err(LaunchFailure::OwnerUnavailable))
    };
    if receipt.state == LaunchReceiptState::Rejected {
        tracing::warn!(event = "archive.launch.rejected", request_id = %intent.request_id, failure = ?receipt.failure, "archive launch rejected");
    }
    let response = Envelope::reply_to(&request, Message::DesktopArchiveReceipt(receipt.clone()));
    tokio::time::timeout(
        Duration::from_secs(2),
        super::write_envelope(writer, &response),
    )
    .await
    .map_err(|_| anyhow::anyhow!("archive receipt write expired"))??;
    if receipt.state != LaunchReceiptState::Queued {
        return Ok(());
    }
    // Sender disconnect never revokes an accepted request; the ID retains its bounded replay receipt.
    let endpoint = endpoint.ok_or_else(|| anyhow::anyhow!("archive owner unavailable"))?;
    let deadline = Instant::now() + COMPLETION_TIMEOUT + Duration::from_secs(1);
    while Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(50)).await;
        if let Some(receipt) = endpoint.receipt(intent.request_id) {
            if receipt.state != LaunchReceiptState::Queued {
                let response =
                    Envelope::reply_to(&request, Message::DesktopArchiveReceipt(receipt));
                tokio::time::timeout(
                    Duration::from_secs(2),
                    super::write_envelope(writer, &response),
                )
                .await
                .map_err(|_| anyhow::anyhow!("archive terminal write expired"))??;
                return Ok(());
            }
        } else {
            anyhow::bail!("archive receipt retired");
        }
    }
    anyhow::bail!("archive handoff deadline expired")
}

#[cfg(windows)]
pub fn forward_archive_intent(
    pipe_name: &str,
    intent: &DesktopArchiveIntent,
) -> Result<DesktopArchiveReceipt, LaunchFailure> {
    use nxtools_protocol::{Envelope, Message};
    use std::os::windows::io::AsRawHandle;
    intent.validate()?;
    let pipe = format!(r"\\.\pipe\{pipe_name}");
    let mut connection = super::connect_local_pipe(&pipe, Duration::from_secs(3))
        .map_err(|_| LaunchFailure::TransportTimeout)?;
    super::local_pipe::authorize_desktop_peer(connection.writer.as_raw_handle(), false)?;
    let request = Envelope::new(Message::DesktopArchiveIntent(intent.clone()));
    let encoded = request
        .encode_line()
        .map_err(|_| LaunchFailure::InvalidRequest)?;
    super::local_pipe::write_desktop_frame(
        &mut connection.writer,
        encoded.as_bytes(),
        Duration::from_secs(3),
    )
    .map_err(|_| LaunchFailure::TransportTimeout)?;
    let deadline = Instant::now() + COMPLETION_TIMEOUT + Duration::from_secs(3);
    loop {
        connection
            .reader
            .get_mut()
            .set_initial_frame_timeout(deadline.saturating_duration_since(Instant::now()));
        let frame = super::read_bounded_sync_frame(&mut connection.reader)
            .map_err(|_| LaunchFailure::TransportTimeout)?
            .ok_or(LaunchFailure::Disconnected)?;
        let response = Envelope::decode_line(&frame).map_err(|_| LaunchFailure::InvalidResponse)?;
        if response.correlation_id != Some(request.id) {
            return Err(LaunchFailure::InvalidResponse);
        }
        let Message::DesktopArchiveReceipt(receipt) = response.message else {
            return Err(LaunchFailure::InvalidResponse);
        };
        receipt.validate(intent.request_id)?;
        if let Some(failure) = receipt.failure {
            return Err(failure);
        }
        if receipt.state == LaunchReceiptState::Handled {
            return Ok(receipt);
        }
        if Instant::now() >= deadline {
            return Err(LaunchFailure::TransportTimeout);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nxtools_protocol::desktop_archive_intent::{ArchiveIntentAction, VERSION};
    fn intent() -> DesktopArchiveIntent {
        DesktopArchiveIntent {
            version: VERSION,
            request_id: Uuid::new_v4(),
            action: ArchiveIntentAction::Open,
            paths: vec![r"C:\synthetic.zip".into()],
            mode: None,
        }
    }
    #[test]
    fn production_admission_32_then_overflow_and_duplicate_never_replaces_original() {
        let (endpoint, receiver) = archive_intent_channel();
        let first = intent();
        assert_eq!(
            endpoint.submit(first.clone()).state,
            LaunchReceiptState::Queued
        );
        assert_eq!(
            endpoint.submit(first.clone()).state,
            LaunchReceiptState::Queued
        );
        let mut altered = first.clone();
        altered.paths[0] = r"C:\other.zip".into();
        assert_eq!(
            endpoint.submit(altered).failure,
            Some(LaunchFailure::DuplicateId)
        );
        for _ in 1..MAX_PENDING {
            assert_eq!(endpoint.submit(intent()).state, LaunchReceiptState::Queued);
        }
        assert_eq!(
            endpoint.submit(intent()).failure,
            Some(LaunchFailure::QueueFull)
        );
        let (actual, completion) = receiver.try_next().expect("first owned request");
        assert!(actual == first);
        assert!(completion.finish(Ok(())));
        assert!(!completion.finish(Err(LaunchFailure::OpenFailed)));
        assert_eq!(endpoint.submit(first).state, LaunchReceiptState::Handled);
    }
    #[test]
    fn production_readiness_expiry_disconnect_and_bounded_replay() {
        let (endpoint, receiver) = archive_intent_channel();
        let request = intent();
        endpoint.submit_at(request.clone(), Instant::now() - READINESS_TIMEOUT);
        assert_eq!(
            endpoint
                .receipt(request.request_id)
                .expect("receipt")
                .failure,
            Some(LaunchFailure::ReadinessTimeout)
        );
        assert!(receiver.try_next().is_none());
        for _ in 1..MAX_REPLAY {
            endpoint.submit(intent());
            let (_, completion) = receiver.try_next().expect("dispatch");
            completion.finish(Ok(()));
        }
        assert_eq!(
            endpoint.state.lock().expect("state").records.len(),
            MAX_REPLAY
        );
        assert_eq!(
            endpoint.submit(intent()).failure,
            Some(LaunchFailure::QueueFull)
        );
        assert_eq!(
            endpoint.submit(request).failure,
            Some(LaunchFailure::ReadinessTimeout)
        );
        {
            let mut state = endpoint.state.lock().expect("state");
            sweep(&mut state, Instant::now() + REPLAY_TTL);
            assert!(state.records.is_empty());
        }
        let request = intent();
        endpoint.submit(request.clone());
        drop(receiver);
        assert_eq!(
            endpoint
                .receipt(request.request_id)
                .expect("disconnect")
                .failure,
            Some(LaunchFailure::Disconnected)
        );
        assert_eq!(
            endpoint.submit(intent()).failure,
            Some(LaunchFailure::Disconnected)
        );
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn actual_listener_authenticates_and_replays_lost_ack_without_second_dispatch() {
        use nxtools_protocol::{Envelope, Message};
        let pipe_name = format!("nxtools-archive-intent-fixture-{}", Uuid::new_v4());
        let (_bridge, server) = crate::platform_bridge(4);
        let (endpoint, receiver) = archive_intent_channel();
        let server = server.with_archive_intents(endpoint.clone());
        let listener_name = pipe_name.clone();
        let listener =
            tokio::spawn(async move { crate::run_platform_server(&listener_name, server).await });
        let request = intent();
        let lost = request.clone();
        let lost_id = lost.request_id;
        let lost_pipe = pipe_name.clone();
        let ownership = endpoint.clone();
        tokio::task::spawn_blocking(move || {
            let mut connection = crate::connect_local_pipe(
                &format!(r"\\.\pipe\{lost_pipe}"),
                Duration::from_secs(3),
            )
            .expect("fixture connect");
            let frame = Envelope::new(Message::DesktopArchiveIntent(lost))
                .encode_line()
                .expect("frame");
            crate::local_pipe::write_desktop_frame(
                &mut connection.writer,
                frame.as_bytes(),
                Duration::from_secs(1),
            )
            .expect("send before lost ack");
            let deadline = Instant::now() + Duration::from_secs(3);
            while ownership.receipt(lost_id).is_none() {
                assert!(
                    Instant::now() < deadline,
                    "server must own request before disconnect"
                );
                std::thread::sleep(Duration::from_millis(5));
            }
            // Close without reading Queued; accepted ownership must survive.
        })
        .await
        .expect("lost-ack client");
        let deadline = Instant::now() + Duration::from_secs(4);
        let completion = loop {
            if let Some((actual, completion)) = receiver.try_next() {
                assert!(actual == request);
                break completion;
            }
            assert!(
                Instant::now() < deadline,
                "authenticated actual listener did not admit"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        };
        assert!(completion.finish(Ok(())));
        let forwarded = request.clone();
        let receipt =
            tokio::task::spawn_blocking(move || forward_archive_intent(&pipe_name, &forwarded))
                .await
                .expect("client worker")
                .expect("authenticated replay");
        assert_eq!(receipt.state, LaunchReceiptState::Handled);
        assert!(
            receiver.try_next().is_none(),
            "lost acknowledgement must not create a second request"
        );
        listener.abort();
        let _ = listener.await;
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn actual_listener_rejects_other_executable_despite_current_user_acl_access() {
        use nxtools_protocol::{Envelope, Message};
        let pipe_name = format!("nxtools-archive-wrong-peer-fixture-{}", Uuid::new_v4());
        let (_bridge, server) = crate::platform_bridge(4);
        let (endpoint, receiver) = archive_intent_channel();
        let server = server.with_archive_intents(endpoint);
        let listener_name = pipe_name.clone();
        let listener =
            tokio::spawn(async move { crate::run_platform_server(&listener_name, server).await });
        let frame = Envelope::new(Message::DesktopArchiveIntent(intent()))
            .encode_line()
            .expect("encode");
        let result = tokio::task::spawn_blocking(move || {
            let script = format!(r#"$client = [System.IO.Pipes.NamedPipeClientStream]::new('.', '{pipe_name}', [System.IO.Pipes.PipeDirection]::InOut); $client.Connect(3000); $writer = [System.IO.StreamWriter]::new($client); $writer.AutoFlush = $true; $writer.Write('{frame}'); $reader = [System.IO.StreamReader]::new($client); $read = $reader.ReadLineAsync(); if (-not $read.Wait(4000)) {{ exit 9 }}; [Console]::WriteLine($read.Result); $client.Dispose()"#);
            std::process::Command::new("powershell.exe").args(["-NoProfile", "-NonInteractive", "-Command", &script]).output().expect("isolated peer process")
        }).await.expect("peer worker");
        assert!(
            result.status.success(),
            "isolated peer must obtain typed rejection"
        );
        let response = Envelope::decode_line(
            std::str::from_utf8(&result.stdout)
                .expect("response UTF8")
                .trim_start_matches('\u{feff}'),
        )
        .expect("response envelope");
        let Message::DesktopArchiveReceipt(receipt) = response.message else {
            panic!("typed receipt required");
        };
        assert_eq!(receipt.failure, Some(LaunchFailure::Unauthorized));
        assert!(receiver.try_next().is_none());
        listener.abort();
        let _ = listener.await;
    }

    #[test]
    fn original_native_disconnect_retires_pending_and_late_completion() {
        let (endpoint, receiver) = archive_intent_channel();
        let request = intent();
        endpoint.submit(request.clone());
        let (_, completion) = receiver.try_next().expect("dispatch");
        endpoint.native_disconnected();
        assert!(!completion.is_pending());
        assert!(!completion.finish(Ok(())));
        assert_eq!(
            endpoint
                .receipt(request.request_id)
                .expect("receipt")
                .failure,
            Some(LaunchFailure::Disconnected)
        );
    }
}
