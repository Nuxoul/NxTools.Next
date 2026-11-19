//! Bounded Markdown admission on the original platform pipe. PlatformHost is not its owner.
use async_channel::{Receiver, Sender};
use nxtools_protocol::desktop_markdown_intent::{DesktopMarkdownIntent, LaunchFailure};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::watch;
use uuid::Uuid;

const ADMISSION_TIMEOUT: Duration = Duration::from_secs(12);
const REPLAY_TTL: Duration = Duration::from_secs(60);
const MAX_PENDING: usize = 32;
const MAX_REPLAY: usize = 128;
#[cfg(windows)]
const FRAME_TIMEOUT: Duration = Duration::from_secs(2);
#[cfg(windows)]
const MAX_RECEIPT_FRAME_BYTES: usize = 1024;

type Outcome = Result<(), LaunchFailure>;

#[derive(Clone, Copy)]
enum AdmissionState {
    Checking,
    Dispatching,
    Finished(Outcome),
}

pub struct MarkdownAdmission {
    pub intent: DesktopMarkdownIntent,
    accepted: Instant,
    state: watch::Sender<AdmissionState>,
}

#[derive(Clone)]
pub struct MarkdownIntentEndpoint {
    records: Arc<Mutex<HashMap<Uuid, Arc<MarkdownAdmission>>>>,
    sender: Sender<Arc<MarkdownAdmission>>,
}

pub struct MarkdownIntentReceiver {
    endpoint: MarkdownIntentEndpoint,
    receiver: Receiver<Arc<MarkdownAdmission>>,
}

pub fn markdown_intent_channel() -> (MarkdownIntentEndpoint, MarkdownIntentReceiver) {
    let (sender, receiver) = async_channel::bounded(MAX_PENDING);
    let endpoint = MarkdownIntentEndpoint {
        records: Arc::new(Mutex::new(HashMap::new())),
        sender,
    };
    (
        endpoint.clone(),
        MarkdownIntentReceiver { endpoint, receiver },
    )
}

impl MarkdownIntentEndpoint {
    pub fn submit(
        &self,
        intent: DesktopMarkdownIntent,
    ) -> Result<Arc<MarkdownAdmission>, LaunchFailure> {
        self.submit_at(intent, Instant::now())
    }

    fn submit_at(
        &self,
        intent: DesktopMarkdownIntent,
        now: Instant,
    ) -> Result<Arc<MarkdownAdmission>, LaunchFailure> {
        intent.validate()?;
        let id = intent.request_id;
        let mut records = self
            .records
            .lock()
            .map_err(|_| LaunchFailure::Disconnected)?;
        if self.sender.is_closed() {
            return Err(LaunchFailure::Disconnected);
        }
        records.retain(|_, record| {
            now.saturating_duration_since(record.accepted) < REPLAY_TTL || record.is_pending()
        });
        if let Some(original) = records.get(&id) {
            return if original.intent == intent {
                Ok(original.clone())
            } else {
                Err(LaunchFailure::DuplicateId)
            };
        }
        if records.len() >= MAX_REPLAY
            || records
                .values()
                .filter(|record| record.is_pending())
                .count()
                >= MAX_PENDING
        {
            return Err(LaunchFailure::QueueFull);
        }
        let record = Arc::new(MarkdownAdmission {
            intent,
            accepted: now,
            state: watch::Sender::new(AdmissionState::Checking),
        });
        records.insert(id, record.clone());
        tracing::info!(event = "markdown.launch.accepted", correlation_id = %id);
        if let Err(error) = self.sender.try_send(record.clone()) {
            records.remove(&id);
            let failure = if error.is_closed() {
                LaunchFailure::Disconnected
            } else {
                LaunchFailure::QueueFull
            };
            record.finish(Err(failure), None);
            return Err(failure);
        }
        Ok(record)
    }
}

impl MarkdownIntentReceiver {
    pub async fn next(&self) -> Option<Arc<MarkdownAdmission>> {
        while let Ok(record) = self.receiver.recv().await {
            if record.is_pending() {
                tracing::info!(event = "markdown.launch.started", correlation_id = %record.intent.request_id, elapsed_ms = record.accepted.elapsed().as_millis());
                return Some(record);
            }
        }
        None
    }
}

impl Drop for MarkdownIntentReceiver {
    fn drop(&mut self) {
        self.endpoint.sender.close();
        if let Ok(records) = self.endpoint.records.lock() {
            for record in records.values() {
                record.finish(Err(LaunchFailure::Disconnected), None);
            }
        }
    }
}

impl MarkdownAdmission {
    pub fn remaining(&self) -> Duration {
        ADMISSION_TIMEOUT.saturating_sub(self.accepted.elapsed())
    }

    pub fn is_pending(&self) -> bool {
        if self.remaining().is_zero() {
            self.expire();
        }
        !matches!(*self.state.borrow(), AdmissionState::Finished(_))
    }

    fn expire(&self) {
        self.state.send_if_modified(|state| {
            if !matches!(*state, AdmissionState::Checking) {
                return false;
            }
            self.finish_state(state, Err(LaunchFailure::AdmissionTimeout), None);
            true
        });
    }

    // Called immediately before handing over to the original reader on the GPUI turn.
    // Timeout may retire checking, but cannot invalidate an already-started owner callback.
    pub fn begin_dispatch(&self) -> bool {
        let mut claimed = false;
        self.state.send_if_modified(|state| {
            if !matches!(*state, AdmissionState::Checking) { return false; }
            if self.remaining().is_zero() {
                self.finish_state(state, Err(LaunchFailure::AdmissionTimeout), None);
            } else {
                *state = AdmissionState::Dispatching;
                claimed = true;
                tracing::info!(event = "markdown.launch.dispatching", correlation_id = %self.intent.request_id, elapsed_ms = self.accepted.elapsed().as_millis());
            }
            true
        });
        claimed
    }

    pub fn finish(&self, result: Outcome, native_code: Option<i32>) {
        self.state.send_if_modified(|state| {
            if matches!(*state, AdmissionState::Finished(_)) {
                return false;
            }
            if matches!(*state, AdmissionState::Checking) && self.remaining().is_zero() {
                self.finish_state(state, Err(LaunchFailure::AdmissionTimeout), None);
            } else {
                self.finish_state(state, result, native_code);
            }
            true
        });
    }

    fn finish_state(&self, state: &mut AdmissionState, result: Outcome, native_code: Option<i32>) {
        *state = AdmissionState::Finished(result);
        tracing::info!(event = "markdown.launch.finished", correlation_id = %self.intent.request_id, admitted = result.is_ok(), failure = ?result.err(), ?native_code, elapsed_ms = self.accepted.elapsed().as_millis());
    }

    #[cfg(any(test, windows))]
    fn outcome(&self) -> Option<Outcome> {
        match *self.state.borrow() {
            AdmissionState::Finished(outcome) => Some(outcome),
            _ => None,
        }
    }

    #[cfg(windows)]
    async fn wait(&self) -> Outcome {
        let mut changes = self.state.subscribe();
        let finished = |state: &AdmissionState| matches!(state, AdmissionState::Finished(_));
        let _ = tokio::time::timeout(self.remaining(), changes.wait_for(finished)).await;
        self.expire();
        if let Some(outcome) = self.outcome() {
            return outcome;
        }
        // Only a synchronous GPUI admission callback can already be in flight here.
        let _ = tokio::time::timeout(FRAME_TIMEOUT, changes.wait_for(finished)).await;
        self.outcome()
            .unwrap_or(Err(LaunchFailure::TransportTimeout))
    }
}

#[cfg(windows)]
pub(crate) async fn serve<W: tokio::io::AsyncWrite + Unpin>(
    request: nxtools_protocol::Envelope,
    endpoint: Option<&MarkdownIntentEndpoint>,
    authorized: bool,
    writer: &mut W,
) -> anyhow::Result<()> {
    use nxtools_protocol::desktop_markdown_intent::DesktopMarkdownReceipt;
    use nxtools_protocol::{Envelope, Message};
    let Message::DesktopMarkdownIntent(intent) = &request.message else {
        anyhow::bail!("invalid Markdown launch message");
    };
    let submitted = if !authorized {
        Err(LaunchFailure::Unauthorized)
    } else if request.id != intent.request_id || request.correlation_id.is_some() {
        Err(LaunchFailure::InvalidRequest)
    } else {
        endpoint
            .ok_or(LaunchFailure::OwnerUnavailable)
            .and_then(|endpoint| endpoint.submit(intent.clone()))
    };
    let result = match submitted {
        Ok(operation) => operation.wait().await,
        Err(failure) => {
            tracing::warn!(event = "markdown.launch.rejected", correlation_id = %intent.request_id, ?failure);
            Err(failure)
        }
    };
    let receipt = DesktopMarkdownReceipt::new(intent.request_id, result);
    let reply = Envelope::reply_to(&request, Message::DesktopMarkdownReceipt(receipt));
    let result = tokio::time::timeout(FRAME_TIMEOUT, super::write_envelope(writer, &reply))
        .await
        .map_err(|_| anyhow::anyhow!("Markdown receipt write expired"))
        .and_then(|result| result);
    if let Err(error) = &result {
        let native_code = error
            .downcast_ref::<std::io::Error>()
            .and_then(std::io::Error::raw_os_error);
        tracing::warn!(event = "markdown.launch.receipt_write_failed", correlation_id = %intent.request_id, ?native_code);
    }
    result
}

#[cfg(windows)]
pub fn forward_markdown_intent(
    pipe_name: &str,
    intent: &DesktopMarkdownIntent,
) -> Result<(), LaunchFailure> {
    use nxtools_protocol::{Envelope, Message};
    use std::io::Read as _;
    use std::os::windows::io::AsRawHandle;

    intent.validate()?;
    let started = Instant::now();
    let id = intent.request_id;
    let transport_error = |stage, error: anyhow::Error| {
        let native_code = error
            .downcast_ref::<std::io::Error>()
            .and_then(std::io::Error::raw_os_error);
        tracing::warn!(event = "markdown.launch.transport_failed", correlation_id = %id, stage, ?native_code, elapsed_ms = started.elapsed().as_millis());
        LaunchFailure::TransportTimeout
    };
    let pipe = format!(r"\\.\pipe\{pipe_name}");
    let mut connection = super::connect_local_pipe(&pipe, Duration::from_secs(8))
        .map_err(|error| transport_error("connect", error))?;
    super::local_pipe::authorize_desktop_peer(connection.writer.as_raw_handle(), false)
        .map_err(|_| LaunchFailure::Unauthorized)?;
    let mut request = Envelope::new(Message::DesktopMarkdownIntent(intent.clone()));
    request.id = id;
    let encoded = request
        .encode_line()
        .map_err(|_| LaunchFailure::InvalidRequest)?;
    super::local_pipe::write_desktop_frame(
        &mut connection.writer,
        encoded.as_bytes(),
        FRAME_TIMEOUT,
    )
    .map_err(|error| transport_error("write_request", error))?;

    connection
        .reader
        .get_mut()
        .set_initial_frame_timeout(ADMISSION_TIMEOUT + FRAME_TIMEOUT * 2 + Duration::from_secs(1));
    let mut receipt_reader = (&mut connection.reader).take(MAX_RECEIPT_FRAME_BYTES as u64);
    let response = super::read_bounded_sync_frame(&mut receipt_reader)
        .map_err(|error| transport_error("read_receipt", error))?
        .ok_or(LaunchFailure::Disconnected)?;
    decode_receipt(&response, id, id)
}

#[cfg(windows)]
fn decode_receipt(frame: &str, reply_to: Uuid, operation: Uuid) -> Outcome {
    use nxtools_protocol::desktop_markdown_intent::MarkdownOutcome;
    use nxtools_protocol::{Envelope, Message};
    if frame.len() >= MAX_RECEIPT_FRAME_BYTES {
        return Err(LaunchFailure::InvalidResponse);
    }
    let reply = Envelope::decode_line(frame).map_err(|_| LaunchFailure::InvalidResponse)?;
    if reply.correlation_id != Some(reply_to) {
        return Err(LaunchFailure::InvalidResponse);
    }
    let Message::DesktopMarkdownReceipt(receipt) = reply.message else {
        return Err(LaunchFailure::InvalidResponse);
    };
    receipt.validate(operation)?;
    match receipt.outcome {
        MarkdownOutcome::Admitted => Ok(()),
        MarkdownOutcome::Rejected(failure) => Err(failure),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intent() -> DesktopMarkdownIntent {
        DesktopMarkdownIntent::new(r"C:\synthetic\中文 notes.md".into(), Uuid::new_v4())
    }

    #[test]
    fn coalesces_replay_but_rejects_reuse_for_another_path_and_bounds_pending() {
        let (endpoint, receiver) = markdown_intent_channel();
        let first = intent();
        let pending = endpoint.submit(first.clone()).expect("first request");
        let replay = endpoint.submit(first.clone()).expect("pending replay");
        let mut altered = first.clone();
        altered.path = r"C:\synthetic\other.md".into();
        assert_eq!(
            endpoint.submit(altered).err(),
            Some(LaunchFailure::DuplicateId)
        );
        for _ in 1..MAX_PENDING {
            endpoint.submit(intent()).expect("fill queue");
        }
        assert_eq!(
            endpoint.submit(intent()).err(),
            Some(LaunchFailure::QueueFull)
        );
        let dispatch = receiver.receiver.try_recv().expect("original queue head");
        assert_eq!(dispatch.intent, first);
        assert!(dispatch.begin_dispatch());
        assert!(!dispatch.begin_dispatch());
        dispatch.finish(Ok(()), None);
        assert_eq!(pending.outcome(), Some(Ok(())));
        assert_eq!(replay.outcome(), pending.outcome());
        assert_eq!(
            endpoint.submit(first).expect("terminal replay").outcome(),
            Some(Ok(()))
        );
        assert_eq!(receiver.receiver.len(), MAX_PENDING - 1);
    }

    #[test]
    fn a_terminal_timeout_or_shutdown_cannot_be_overwritten_by_late_preflight() {
        let (endpoint, receiver) = markdown_intent_channel();
        let expired = endpoint
            .submit_at(intent(), Instant::now() - ADMISSION_TIMEOUT)
            .expect("queued");
        assert!(!expired.is_pending());
        expired.finish(Ok(()), None);
        assert!(!expired.begin_dispatch());
        assert_eq!(
            expired.outcome(),
            Some(Err(LaunchFailure::AdmissionTimeout))
        );

        let pending = endpoint.submit(intent()).expect("pending request");
        drop(receiver);
        assert!(!pending.is_pending());
        assert_eq!(pending.outcome(), Some(Err(LaunchFailure::Disconnected)));
        assert_eq!(
            endpoint.submit(intent()).err(),
            Some(LaunchFailure::Disconnected)
        );
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn actual_listener_handoff_preserves_operation_id_without_a_native_connection() {
        let pipe_name = format!("nxtools-test-markdown-{}", Uuid::new_v4());
        let (bridge, server) = super::super::platform_bridge(4);
        let (endpoint, receiver) = markdown_intent_channel();
        let server_name = pipe_name.clone();
        let server_task = tokio::spawn(async move {
            super::super::run_platform_server(&server_name, server.with_markdown_intents(endpoint))
                .await
        });
        let original = intent();
        let sent = original.clone();
        let client =
            tokio::task::spawn_blocking(move || forward_markdown_intent(&pipe_name, &sent));
        let dispatch = tokio::time::timeout(Duration::from_secs(4), receiver.next())
            .await
            .expect("bounded listener dispatch")
            .expect("listener delivered the request");
        assert!(!bridge.is_connected());
        assert_eq!(dispatch.intent, original);
        dispatch.finish(Err(LaunchFailure::NotAdmitted), None);
        assert_eq!(
            client.await.expect("client finished"),
            Err(LaunchFailure::NotAdmitted)
        );
        server_task.abort();
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn handshake_requires_peer_authority_and_exact_envelope_operation_id() {
        use nxtools_protocol::{Envelope, Message};
        let (endpoint, receiver) = markdown_intent_channel();
        let request = intent();
        let mut envelope = Envelope::new(Message::DesktopMarkdownIntent(request.clone()));
        for (authorized, matching, failure) in [
            (false, true, LaunchFailure::Unauthorized),
            (true, false, LaunchFailure::InvalidRequest),
        ] {
            envelope.id = if matching {
                request.request_id
            } else {
                Uuid::new_v4()
            };
            let mut written = Vec::new();
            serve(envelope.clone(), Some(&endpoint), authorized, &mut written)
                .await
                .expect("receipt");
            assert_eq!(
                decode_receipt(
                    std::str::from_utf8(&written).expect("UTF-8 reply"),
                    envelope.id,
                    request.request_id
                ),
                Err(failure),
            );
            assert!(receiver.receiver.is_empty());
        }
    }
}
