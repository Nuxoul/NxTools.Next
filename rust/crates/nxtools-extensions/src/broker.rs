//! Ordinary-process transport only. Windows process admission remains exclusively in C#.
use crate::{InvocationIdentity, invocation::MAX_FRAME_BYTES};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::Path,
    process::Stdio,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
    sync::{OwnedSemaphorePermit, mpsc, watch},
};
use uuid::Uuid;

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum BrokerError {
    #[error("extension broker unavailable")]
    Unavailable,
    #[error("extension broker queue full")]
    QueueFull,
    #[error("extension broker run already pending")]
    Duplicate,
    #[error("extension broker invalid frame")]
    InvalidFrame,
    #[error("extension broker slot remains occupied")]
    CleanupPending,
}

#[derive(Clone, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BrokerStart {
    pub version: u16,
    pub op: String,
    pub run_id: Uuid,
    pub plugin_id: String,
    pub executable: String,
    pub arguments: Vec<String>,
    pub working_directory: String,
    pub timeout_ms: u32,
    pub memory_bytes: u64,
    pub max_processes: u8,
    pub stdin: String,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrokerEvent {
    pub version: u16,
    pub op: String,
    pub run_id: Uuid,
    pub success: bool,
    pub elapsed_ms: u64,
    pub cleanup_complete: bool,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub native_error_code: Option<i32>,
    #[serde(default)]
    pub exit_code: Option<u32>,
    #[serde(default)]
    pub data: Option<String>,
    #[serde(default)]
    pub process_id: Option<u32>,
}

struct Pending {
    identity: InvocationIdentity,
    events: Option<mpsc::Sender<Result<BrokerEvent, BrokerError>>>,
    cancel_requested: bool,
    // Runtime execution slots belong to native cleanup, not the consumer future.
    _execution_slots: Option<[OwnedSemaphorePermit; 2]>,
    input_bytes: usize,
    cleanup_pending: bool,
}
struct Shared {
    credentials: Mutex<
        HashMap<
            Uuid,
            tokio::sync::oneshot::Sender<crate::credential_transport::CredentialResponse>,
        >,
    >,
    pending: Mutex<HashMap<Uuid, Pending>>,
    controls: mpsc::Sender<Vec<u8>>,
    stop: watch::Sender<bool>,
    retired: AtomicBool,
}
struct Lease(watch::Sender<bool>);
impl Drop for Lease {
    fn drop(&mut self) {
        let _ = self.0.send(true);
    }
}

#[derive(Clone)]
pub struct BrokerClient {
    shared: Arc<Shared>,
    _lease: Arc<Lease>,
}
pub struct BrokerRun {
    pub identity: InvocationIdentity,
    events: mpsc::Receiver<Result<BrokerEvent, BrokerError>>,
    client: BrokerClient,
    drained: bool,
}
impl BrokerRun {
    pub async fn next(&mut self) -> Result<BrokerEvent, BrokerError> {
        let event = self.events.recv().await.ok_or(BrokerError::Unavailable)??;
        if matches!(event.op.as_str(), "exited" | "rejected") && event.cleanup_complete {
            self.drained = true;
        }
        Ok(event)
    }
    pub fn write(&self, data: String) -> Result<(), BrokerError> {
        self.client.write(self.identity.run_id, data)
    }
    pub fn cancel(&self) -> Result<(), BrokerError> {
        self.client.cancel(self.identity.run_id)
    }
}
impl Drop for BrokerRun {
    fn drop(&mut self) {
        if !self.drained {
            let _ = self.client.cancel(self.identity.run_id);
        }
    }
}

impl BrokerClient {
    pub fn launch(executable: &Path) -> Result<Self, BrokerError> {
        if !executable.is_absolute() {
            return Err(BrokerError::Unavailable);
        }
        let own = std::env::current_exe().map_err(|_| BrokerError::Unavailable)?;
        if executable.parent() != own.parent()
            || !executable
                .file_name()
                .is_some_and(|name| name.eq_ignore_ascii_case("NxTools.PlatformHost.exe"))
        {
            return Err(BrokerError::Unavailable);
        }
        Self::launch_inner(
            executable,
            &[
                "--extension-host".into(),
                "--extension-parent-pid".into(),
                std::process::id().to_string(),
            ],
        )
    }

    pub(crate) fn launch_inner(
        executable: &Path,
        arguments: &[String],
    ) -> Result<Self, BrokerError> {
        let mut command = Command::new(executable);
        command
            .args(arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        #[cfg(windows)]
        command.creation_flags(0x0800_0000);
        let mut child = command.spawn().map_err(|_| BrokerError::Unavailable)?;
        let stdin = child.stdin.take().ok_or(BrokerError::Unavailable)?;
        let stdout = child.stdout.take().ok_or(BrokerError::Unavailable)?;
        let (controls, mut control_rx) = mpsc::channel::<Vec<u8>>(32);
        let (stop, mut stop_rx) = watch::channel(false);
        let shared = Arc::new(Shared {
            pending: Mutex::new(HashMap::new()),
            credentials: Mutex::new(HashMap::new()),
            controls,
            stop: stop.clone(),
            retired: AtomicBool::new(false),
        });
        let client = Self {
            shared: shared.clone(),
            _lease: Arc::new(Lease(stop)),
        };
        let writer_shared = shared.clone();
        let mut writer_stop = stop_rx.clone();
        let writer = tokio::spawn(async move {
            let mut stdin = stdin;
            loop {
                tokio::select! {
                    _ = writer_stop.changed() => break,
                    frame = control_rx.recv() => {
                        let Some(mut frame) = frame else { break; };
                        frame.push(b'\n');
                        let result = tokio::time::timeout(Duration::from_secs(2), async { stdin.write_all(&frame).await?; stdin.flush().await }).await;
                        if !matches!(result, Ok(Ok(()))) { let _ = writer_shared.stop.send(true); break; }
                    }
                }
            }
        });
        let reader_shared = shared.clone();
        let reader = tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            loop {
                let result = read_frame(&mut reader)
                    .await
                    .and_then(|bytes| parse_message(&bytes));
                let Ok(message) = result else {
                    let _ = reader_shared.stop.send(true);
                    break;
                };
                let event = match message {
                    BrokerMessage::Process(event) => event,
                    BrokerMessage::Credential(response) => {
                        if let Ok(mut pending) = reader_shared.credentials.lock()
                            && let Some(reply) = pending.remove(&response.request_id)
                        {
                            let _ = reply.send(response);
                        }
                        continue;
                    }
                };
                if event.run_id.is_nil() {
                    let _ = reader_shared.stop.send(true);
                    break;
                }
                let Ok(mut pending) = reader_shared.pending.lock() else {
                    let _ = reader_shared.stop.send(true);
                    break;
                };
                let Some(run) = pending.get_mut(&event.run_id) else {
                    continue;
                };
                let terminal = matches!(event.op.as_str(), "exited" | "rejected");
                if terminal {
                    run.cleanup_pending = !event.cleanup_complete;
                }
                // Do not log stdout/stderr frames or copy their payload into diagnostics.
                if !matches!(event.op.as_str(), "stdout" | "stderr") {
                    tracing::info!(event="extension.native.transition", correlation_id=%run.identity.correlation_id, run_id=%event.run_id, transition=%event.op, success=event.success, cleanup_complete=event.cleanup_complete, native_error_code=?event.native_error_code, elapsed_ms=event.elapsed_ms, "extension process transition");
                }
                let delivery_failed = run
                    .events
                    .as_ref()
                    .is_some_and(|events| events.try_send(Ok(event.clone())).is_err());
                if delivery_failed {
                    // A slow/dropped run consumer is not a lost broker transport.
                    // Detach and cancel only this run; retain its identity and slots.
                    run.events = None;
                }
                let cancel = delivery_failed
                    && !run.cancel_requested
                    && !(terminal && event.cleanup_complete);
                if cancel {
                    run.cancel_requested = true;
                }
                if terminal && event.cleanup_complete {
                    pending.remove(&event.run_id);
                }
                drop(pending);
                if cancel {
                    enqueue_cancel(&reader_shared, event.run_id);
                }
            }
        });
        tokio::spawn(async move {
            tokio::select! { _ = stop_rx.changed() => {}, _ = child.wait() => {} }
            let _ = shared.stop.send(true);
            let _ = child.start_kill();
            let cleanup = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
            writer.abort();
            reader.abort();
            if matches!(cleanup, Ok(Ok(_))) {
                if let Ok(mut pending) = shared.pending.lock() {
                    for (_, run) in pending.drain() {
                        if let Some(events) = run.events {
                            let _ = events.try_send(Err(BrokerError::Unavailable));
                        }
                    }
                }
                // Replacement is safe only after the exact old process is reaped.
                shared.retired.store(true, Ordering::Release);
            }
            if let Ok(mut credentials) = shared.credentials.lock() {
                credentials.clear();
            }
        });
        Ok(client)
    }

    pub(crate) fn is_available(&self) -> bool {
        !*self.shared.stop.borrow()
    }

    pub(crate) fn can_replace(&self) -> bool {
        !self.is_available() && self.shared.retired.load(Ordering::Acquire)
    }

    pub fn start(
        &self,
        identity: InvocationIdentity,
        request: BrokerStart,
    ) -> Result<BrokerRun, BrokerError> {
        self.start_with_slots(identity, request, None)
    }

    pub(crate) fn start_with_slots(
        &self,
        identity: InvocationIdentity,
        request: BrokerStart,
        execution_slots: Option<[OwnedSemaphorePermit; 2]>,
    ) -> Result<BrokerRun, BrokerError> {
        if *self.shared.stop.borrow() {
            return Err(BrokerError::Unavailable);
        }
        if request.version != 1
            || request.op != "start"
            || request.run_id != identity.run_id
            || request.plugin_id != identity.package_id
            || request.run_id.is_nil()
            || request.stdin.contains(['\r', '\n'])
            || request.stdin.len() + 1 > MAX_FRAME_BYTES
        {
            return Err(BrokerError::InvalidFrame);
        }
        let frame = encode(&request)?;
        let (events, event_rx) = mpsc::channel(32);
        let mut pending = self
            .shared
            .pending
            .lock()
            .map_err(|_| BrokerError::Unavailable)?;
        if pending.contains_key(&identity.run_id) {
            return Err(BrokerError::Duplicate);
        }
        if pending.len() >= 4
            || pending
                .values()
                .any(|run| run.identity.package_id == identity.package_id)
        {
            return Err(BrokerError::QueueFull);
        }
        // Pending must exist before a fast native output can become observable.
        pending.insert(
            identity.run_id,
            Pending {
                identity: identity.clone(),
                events: Some(events),
                cancel_requested: false,
                _execution_slots: execution_slots,
                input_bytes: request.stdin.len() + 1,
                cleanup_pending: false,
            },
        );
        if self.shared.controls.try_send(frame).is_err() {
            pending.remove(&identity.run_id);
            return Err(BrokerError::QueueFull);
        }
        tracing::info!(event="extension.run.accepted", correlation_id=%identity.correlation_id, run_id=%identity.run_id, "extension native run admitted");
        Ok(BrokerRun {
            identity,
            events: event_rx,
            client: self.clone(),
            drained: false,
        })
    }

    fn write(&self, run_id: Uuid, data: String) -> Result<(), BrokerError> {
        if data.contains(['\r', '\n']) || data.len() + 1 > MAX_FRAME_BYTES {
            return Err(BrokerError::InvalidFrame);
        }
        let frame =
            encode(&serde_json::json!({"version":1,"op":"write","run_id":run_id,"data":data}))?;
        let mut pending = self
            .shared
            .pending
            .lock()
            .map_err(|_| BrokerError::Unavailable)?;
        let run = pending.get_mut(&run_id).ok_or(BrokerError::Unavailable)?;
        if run.cleanup_pending || run.cancel_requested {
            return Err(BrokerError::CleanupPending);
        }
        let input_bytes = run
            .input_bytes
            .checked_add(data.len() + 1)
            .ok_or(BrokerError::InvalidFrame)?;
        if input_bytes > 1024 * 1024 {
            return Err(BrokerError::InvalidFrame);
        }
        self.shared
            .controls
            .try_send(frame)
            .map_err(|_| BrokerError::QueueFull)?;
        run.input_bytes = input_bytes;
        Ok(())
    }

    fn cancel(&self, run_id: Uuid) -> Result<(), BrokerError> {
        let mut pending = self
            .shared
            .pending
            .lock()
            .map_err(|_| BrokerError::Unavailable)?;
        let Some(run) = pending.get_mut(&run_id) else {
            return Ok(());
        };
        if !run.cancel_requested {
            run.cancel_requested = true;
            enqueue_cancel(&self.shared, run_id);
        }
        Ok(())
    }

    pub(crate) async fn credential(
        &self,
        request: crate::credential_transport::CredentialRequest,
    ) -> Result<crate::credential_transport::CredentialResponse, BrokerError> {
        if *self.shared.stop.borrow() {
            return Err(BrokerError::Unavailable);
        }
        let frame = encode(&request)?;
        let (reply, response) = tokio::sync::oneshot::channel();
        {
            let mut pending = self
                .shared
                .credentials
                .lock()
                .map_err(|_| BrokerError::Unavailable)?;
            if pending.len() >= 8 {
                return Err(BrokerError::QueueFull);
            }
            pending.insert(request.request_id, reply);
            if self.shared.controls.try_send(frame).is_err() {
                pending.remove(&request.request_id);
                return Err(BrokerError::QueueFull);
            }
        }
        let result = tokio::time::timeout(Duration::from_secs(5), response).await;
        if let Ok(mut pending) = self.shared.credentials.lock() {
            pending.remove(&request.request_id);
        }
        result
            .map_err(|_| BrokerError::Unavailable)?
            .map_err(|_| BrokerError::Unavailable)
    }
    pub fn shutdown(&self) {
        let _ = self.shared.stop.send(true);
    }
}

fn enqueue_cancel(shared: &Shared, run_id: Uuid) {
    let frame = format!("{{\"version\":1,\"op\":\"cancel\",\"run_id\":\"{run_id}\"}}").into_bytes();
    if let Err(mpsc::error::TrySendError::Full(frame)) = shared.controls.try_send(frame) {
        // At most one deferred cancellation per native run (four total). Never
        // turn ordinary control backpressure into cancellation of unrelated jobs.
        let controls = shared.controls.clone();
        let mut stop = shared.stop.subscribe();
        tokio::spawn(async move {
            tokio::select! {
                _ = stop.changed() => {},
                _ = controls.send(frame) => {},
            }
        });
    }
}

fn encode(value: &impl Serialize) -> Result<Vec<u8>, BrokerError> {
    let bytes = serde_json::to_vec(value).map_err(|_| BrokerError::InvalidFrame)?;
    if bytes.len() + 1 > MAX_FRAME_BYTES {
        return Err(BrokerError::InvalidFrame);
    }
    Ok(bytes)
}
enum BrokerMessage {
    Process(BrokerEvent),
    Credential(crate::credential_transport::CredentialResponse),
}
fn parse_message(bytes: &[u8]) -> Result<BrokerMessage, BrokerError> {
    let crate::strict_json::StrictValue(value) =
        serde_json::from_slice(bytes).map_err(|_| BrokerError::InvalidFrame)?;
    if value.get("op").and_then(serde_json::Value::as_str) == Some("credential_result") {
        let response: crate::credential_transport::CredentialResponse =
            serde_json::from_value(value).map_err(|_| BrokerError::InvalidFrame)?;
        if !response.validate() {
            return Err(BrokerError::InvalidFrame);
        }
        response.observe_metadata();
        return Ok(BrokerMessage::Credential(response));
    }
    let event: BrokerEvent =
        serde_json::from_value(value).map_err(|_| BrokerError::InvalidFrame)?;
    if event.version != 1
        || !matches!(
            event.op.as_str(),
            "started" | "stdout" | "stderr" | "exited" | "rejected"
        )
    {
        return Err(BrokerError::InvalidFrame);
    }
    Ok(BrokerMessage::Process(event))
}
async fn read_frame(
    reader: &mut BufReader<tokio::process::ChildStdout>,
) -> Result<Vec<u8>, BrokerError> {
    let mut frame = Vec::new();
    loop {
        let available = reader
            .fill_buf()
            .await
            .map_err(|_| BrokerError::Unavailable)?;
        if available.is_empty() {
            return Err(BrokerError::Unavailable);
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let count = newline.map_or(available.len(), |position| position + 1);
        if frame.len() + count > MAX_FRAME_BYTES {
            return Err(BrokerError::InvalidFrame);
        }
        frame.extend_from_slice(&available[..count]);
        reader.consume(count);
        if newline.is_some() {
            frame.pop();
            if frame.last() == Some(&b'\r') {
                frame.pop();
            }
            return Ok(frame);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Invocation;
    #[tokio::test(flavor = "current_thread")]
    async fn owned_csharp_broker_start_output_exit_and_cancel_cross_actual_stdio() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(3).expect("workspace root").join("csharp/NxTools.PlatformHost.Tests/bin/Debug/net8.0-windows/NxTools.PlatformHost.Tests.exe");
        let parent = std::env::current_exe().expect("current owned test process");
        let broker = BrokerClient::launch_inner(
            &fixture,
            &[
                "--extension-fixture-broker".into(),
                std::process::id().to_string(),
                parent.to_string_lossy().into_owned(),
            ],
        )
        .expect("fixture broker");
        let root = tempfile::tempdir().expect("owned data");
        let identity = InvocationIdentity {
            run_id: Uuid::new_v4(),
            correlation_id: Uuid::new_v4(),
            generation: 1,
            package_id: "fixture.protocol".into(),
            handler_id: "run".into(),
            content_hash: "a".repeat(64),
        };
        let invocation = Invocation::new(identity.clone());
        let input = invocation
            .invoke_frame(
                "tools",
                serde_json::json!({}),
                serde_json::json!({}),
                serde_json::json!({}),
            )
            .expect("invoke frame");
        let request = BrokerStart {
            version: 1,
            op: "start".into(),
            run_id: identity.run_id,
            plugin_id: identity.package_id.clone(),
            executable: fixture.to_string_lossy().into_owned(),
            arguments: vec![
                "--extension-fixture-child".into(),
                "echo".into(),
                root.path().join("marker").to_string_lossy().into_owned(),
            ],
            working_directory: root.path().to_string_lossy().into_owned(),
            timeout_ms: 3000,
            memory_bytes: 512 * 1024 * 1024,
            max_processes: 8,
            stdin: input,
        };
        let mut run = broker
            .start(identity, request)
            .expect("pending-before-start");
        let events = tokio::time::timeout(Duration::from_secs(10), async {
            let mut events = Vec::new();
            loop {
                let event = run.next().await.expect("native event");
                let terminal = event.op == "exited" || event.op == "rejected";
                events.push(event);
                if terminal {
                    break;
                }
            }
            events
        })
        .await
        .expect("bounded native fixture");
        assert_eq!(events[0].op, "started");
        assert!(events.iter().any(|event| event.op == "stdout" && event.data.as_deref() == Some("fixture-ready")));
        let last = events.last().expect("terminal");
        assert_eq!(last.op, "exited");
        assert!(last.success && last.cleanup_complete);
        assert!(root.path().join("marker").exists());
        let cancel_identity = InvocationIdentity {
            run_id: Uuid::new_v4(),
            correlation_id: Uuid::new_v4(),
            generation: 2,
            package_id: "fixture.cancel".into(),
            handler_id: "run".into(),
            content_hash: "b".repeat(64),
        };
        let request = BrokerStart {
            version: 1,
            op: "start".into(),
            run_id: cancel_identity.run_id,
            plugin_id: cancel_identity.package_id.clone(),
            executable: fixture.to_string_lossy().into_owned(),
            arguments: vec!["--extension-fixture-child".into(), "sleep".into()],
            working_directory: root.path().to_string_lossy().into_owned(),
            timeout_ms: 3000,
            memory_bytes: 512 * 1024 * 1024,
            max_processes: 8,
            stdin: "{}".into(),
        };
        let mut cancel_run = broker
            .start(cancel_identity, request)
            .expect("cancel fixture");
        assert_eq!(
            cancel_run.next().await.expect("started cancel fixture").op,
            "started"
        );
        cancel_run.cancel().expect("send cancel");
        let terminal = tokio::time::timeout(Duration::from_secs(10), cancel_run.next())
            .await
            .expect("bounded cancel")
            .expect("cancel terminal");
        assert_eq!(terminal.op, "exited");
        assert!(!terminal.success && terminal.cleanup_complete);
        use crate::{BrokerSecretStore, InvocationNotice, SecretStore};
        let store = BrokerSecretStore(broker.clone());
        assert_eq!(
            store
                .get("fixture.credentials", "token")
                .await
                .expect("fake missing credential"),
            None
        );
        store
            .set("fixture.credentials", "token", Some("owned-fake-secret"))
            .await
            .expect("fake write");
        assert_eq!(
            store
                .get("fixture.credentials", "token")
                .await
                .expect("fake read"),
            Some("owned-fake-secret".into())
        );
        store
            .set("fixture.credentials", "token", None)
            .await
            .expect("fake delete");
        assert_eq!(
            store
                .get("fixture.credentials", "token")
                .await
                .expect("fake deleted"),
            None
        );
        let duplex_identity = InvocationIdentity {
            run_id: Uuid::new_v4(),
            correlation_id: Uuid::new_v4(),
            generation: 3,
            package_id: "fixture.duplex".into(),
            handler_id: "run".into(),
            content_hash: "c".repeat(64),
        };
        let mut invocation = Invocation::new(duplex_identity.clone());
        let input = invocation
            .invoke_frame(
                "tools",
                serde_json::json!({}),
                serde_json::json!({}),
                serde_json::json!({}),
            )
            .expect("invoke");
        let request = BrokerStart {
            version: 1,
            op: "start".into(),
            run_id: duplex_identity.run_id,
            plugin_id: duplex_identity.package_id.clone(),
            executable: fixture.to_string_lossy().into_owned(),
            arguments: vec![
                "--extension-fixture-child".into(),
                "protocol-host-call".into(),
            ],
            working_directory: root.path().to_string_lossy().into_owned(),
            timeout_ms: 3000,
            memory_bytes: 512 * 1024 * 1024,
            max_processes: 8,
            stdin: input,
        };
        let mut duplex = broker
            .start(duplex_identity.clone(), request)
            .expect("duplex run");
        let mut completed = false;
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let event = duplex.next().await.expect("duplex native event");
                if event.op == "stdout" {
                    match invocation
                        .accept(
                            event.data.expect("protocol frame").as_bytes(),
                            &duplex_identity,
                        )
                        .expect("current protocol frame")
                    {
                        InvocationNotice::HostCall {
                            call_id, method, ..
                        } => {
                            assert_eq!(method, "host.describe");
                            duplex
                                .write(
                                    invocation
                                        .host_result(
                                            &call_id,
                                            true,
                                            serde_json::json!({"version":1}),
                                            &duplex_identity,
                                        )
                                        .expect("typed reply"),
                                )
                                .expect("write reply");
                        }
                        InvocationNotice::Completed { value, .. } => {
                            assert_eq!(value["text"], "fixture");
                            completed = true;
                        }
                        InvocationNotice::ResultBatch { .. } => panic!("unexpected result batch"),
                    }
                } else if event.op == "exited" {
                    invocation
                        .process_exit(event.success, event.cleanup_complete)
                        .expect("complete and drained");
                    break;
                } else if event.op == "rejected" {
                    panic!("fixture run rejected");
                }
            }
        })
        .await
        .expect("bounded duplex protocol");
        assert!(completed);
        broker.shutdown();
    }
    #[tokio::test(flavor = "current_thread")]
    async fn dropped_run_does_not_stop_other_package_or_next_invocation() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .expect("workspace")
            .join("csharp/NxTools.PlatformHost.Tests/bin/Debug/net8.0-windows/NxTools.PlatformHost.Tests.exe");
        let parent = std::env::current_exe().expect("actual parent");
        let broker = BrokerClient::launch_inner(
            &fixture,
            &[
                "--extension-fixture-broker".into(),
                std::process::id().to_string(),
                parent.to_string_lossy().into_owned(),
            ],
        )
        .expect("owned C# broker with fake credential store");
        let root = tempfile::tempdir().expect("owned data");
        let start = |package: &str, mode: &str| {
            let identity = InvocationIdentity {
                run_id: Uuid::new_v4(),
                correlation_id: Uuid::new_v4(),
                generation: 1,
                package_id: package.into(),
                handler_id: "run".into(),
                content_hash: "a".repeat(64),
            };
            let request = BrokerStart {
                version: 1,
                op: "start".into(),
                run_id: identity.run_id,
                plugin_id: package.into(),
                executable: fixture.to_string_lossy().into_owned(),
                arguments: vec!["--extension-fixture-child".into(), mode.into()],
                working_directory: root.path().to_string_lossy().into_owned(),
                timeout_ms: 30_000,
                memory_bytes: 512 * 1024 * 1024,
                max_processes: 8,
                stdin: "{}".into(),
            };
            let plugin_slot = Arc::new(tokio::sync::Semaphore::new(1))
                .try_acquire_owned()
                .expect("plugin slot");
            let global_slot = Arc::new(tokio::sync::Semaphore::new(4))
                .try_acquire_owned()
                .expect("global slot");
            broker.start_with_slots(identity, request, Some([plugin_slot, global_slot]))
        };
        tokio::time::timeout(Duration::from_secs(20), async {
            let mut cancelled = start("fixture.cancelled", "sleep").expect("first run");
            assert_eq!(cancelled.next().await.expect("first started").op, "started");
            let cancelled_id = cancelled.identity.run_id;
            let mut healthy = start("fixture.healthy", "duplex").expect("second package");
            assert_eq!(healthy.next().await.expect("second started").op, "started");
            assert_eq!(
                healthy.next().await.expect("second awaiting reply").op,
                "stdout"
            );
            cancelled.cancel().expect("cancel only first run");
            drop(cancelled); // Exactly the receiver-drop boundary used by runtime cancellation.
            {
                let pending = broker.shared.pending.lock().expect("cleanup accounting");
                let cancelled = pending
                    .get(&cancelled_id)
                    .expect("still owns native cleanup");
                assert!(cancelled.cancel_requested);
                assert!(
                    cancelled._execution_slots.is_some(),
                    "drop cannot free execution slots"
                );
            }
            while broker
                .shared
                .pending
                .lock()
                .expect("pending")
                .contains_key(&cancelled_id)
            {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            healthy
                .write("fixture-reply".into())
                .expect("unrelated run still writable");
            let mut reply = false;
            loop {
                let event = healthy.next().await.expect("unrelated broker event");
                if event.op == "stdout" {
                    reply |= event.data.as_deref() == Some("fixture-reply");
                }
                if event.op == "exited" {
                    assert!(event.success && event.cleanup_complete);
                    break;
                }
            }
            assert!(reply);
            let mut third =
                start("fixture.cancelled", "duplex").expect("third reuses cleaned package");
            assert_eq!(third.next().await.expect("third started").op, "started");
            assert_eq!(third.next().await.expect("third ready").op, "stdout");
            third.write("third-reply".into()).expect("third reply");
            loop {
                let event = third.next().await.expect("third event");
                if event.op == "exited" {
                    assert!(event.success && event.cleanup_complete);
                    break;
                }
            }
        })
        .await
        .expect("bounded cancellation isolation");
        broker.shutdown();
    }
}
