mod archive_authorization;
pub mod archive_job;
pub mod archive_read;
mod archive_virtual_drag;
pub mod desktop_archive_intent;
pub mod desktop_markdown_intent;
mod floating_cleanup;
#[cfg(windows)]
pub mod install_archive_integration;
pub mod raw_numbered_join;
use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use async_channel::{Receiver, Sender};
use nxtools_protocol::{
    Envelope, HelloAck, MAX_FRAME_BYTES, Message, PlatformCommand, PlatformCommandResult,
    PlatformEvent, Pong,
};
use thiserror::Error;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWriteExt, BufReader};
#[cfg(windows)]
use tokio::net::windows::named_pipe::NamedPipeServer;
use tokio::sync::Mutex as AsyncMutex;
use tracing::{info, warn};

#[cfg(windows)]
mod local_pipe;
#[cfg(windows)]
pub use local_pipe::{
    INDEXER_PIPE_NAME, LocalPipeConnection, PipeRetirement, accept_current_user_pipe,
    accept_local_indexer_pipe, connect_local_indexer_pipe, connect_local_pipe,
};

const PENDING_COMMAND_TIMEOUT: Duration = Duration::from_secs(10);
const PENDING_COMMAND_SWEEP: Duration = Duration::from_secs(1);

struct PendingCommand {
    generation: u64,
    accepted_at: Instant,
    timeout: Option<Duration>,
    summon_reply: Option<Sender<PlatformCommandResult>>,
}

#[derive(Debug)]
struct PendingTerminal {
    id: uuid::Uuid,
    summon_reply: Option<Sender<PlatformCommandResult>>,
}

struct ActiveConnection {
    generation: u64,
    quick_launch_panel_hotkey_available: Option<bool>,
    archive_multi_folder_picker_available: Option<bool>,
    commands: Sender<Envelope>,
    command_wake: Sender<()>,
    archive_reads: Vec<archive_read::ReadReservation>,
    archive_jobs: Vec<archive_job::JobReservation>,
    raw_join: Option<raw_numbered_join::JoinReservation>,
    archive_drag: Option<archive_virtual_drag::DragReservation>,
    archive_authorizations: Vec<archive_authorization::AuthorizationReservation>,
    cancel_turn: bool,
    commands_paused: bool,
    floating_cleanup: Option<Envelope>,
}

/// Original-connection command consumer with bounded, fair cancellation delivery.
#[derive(Clone)]
pub struct PlatformCommandReceiver {
    commands: Receiver<Envelope>,
    wake: Receiver<()>,
    router: Arc<Mutex<CommandRouter>>,
    generation: u64,
}
impl PlatformCommandReceiver {
    pub fn try_recv(&self) -> Result<Envelope, async_channel::TryRecvError> {
        let mut router = self
            .router
            .lock()
            .map_err(|_| async_channel::TryRecvError::Closed)?;
        let Some(active) = router
            .active
            .as_mut()
            .filter(|active| active.generation == self.generation)
        else {
            return self.commands.try_recv();
        };
        if active.commands_paused {
            return Err(async_channel::TryRecvError::Empty);
        }
        if active.cancel_turn
            && let Some(cancel) = active
                .take_archive_cancel()
                .or_else(|| active.take_job_cancel())
                .or_else(|| active.take_join_cancel())
                .or_else(|| active.take_drag_cancel())
                .or_else(|| active.take_authorization_revoke())
        {
            return Ok(cancel);
        }
        let result = match self.commands.try_recv() {
            Ok(envelope) => {
                active.note_archive_start(envelope.id);
                active.note_job_start(envelope.id);
                active.note_join_start(envelope.id);
                active.note_drag_start(envelope.id);
                active.note_authorization_start(envelope.id);
                active.cancel_turn = true;
                Ok(envelope)
            }
            Err(error) => active
                .take_archive_cancel()
                .or_else(|| active.take_job_cancel())
                .or_else(|| active.take_join_cancel())
                .or_else(|| active.take_drag_cancel())
                .or_else(|| active.take_authorization_revoke())
                .ok_or(error),
        };
        if result.is_ok() {
            router.flush_floating_cleanup();
        }
        result
    }
    pub async fn recv(&self) -> Result<Envelope, async_channel::RecvError> {
        loop {
            match self.try_recv() {
                Ok(envelope) => return Ok(envelope),
                Err(async_channel::TryRecvError::Closed) => return Err(async_channel::RecvError),
                Err(async_channel::TryRecvError::Empty) => {}
            }
            // Both admissions and cancels signal this coalesced wake. No lock spans await.
            if self.wake.recv().await.is_err() {
                return self.try_recv().map_err(|_| async_channel::RecvError);
            }
        }
    }
}

/// RAII ordering seam for a self-owned pipe fixture; never changes command admission.
#[doc(hidden)]
pub struct PlatformCommandPause {
    router: Arc<Mutex<CommandRouter>>,
    generation: u64,
}
impl Drop for PlatformCommandPause {
    fn drop(&mut self) {
        if let Ok(mut router) = self.router.lock()
            && let Some(active) = router
                .active
                .as_mut()
                .filter(|active| active.generation == self.generation)
        {
            active.commands_paused = false;
            let _ = active.command_wake.try_send(());
        }
    }
}

struct CommandRouter {
    capacity: usize,
    active: Option<ActiveConnection>,
    pending: HashMap<uuid::Uuid, PendingCommand>,
}

#[derive(Debug)]
enum QueueAdmissionError {
    Disconnected,
    Full,
    Closed,
}

impl CommandRouter {
    /// Both entry paths register before enqueue under this router's existing lock.
    /// Caller-specific duplicate-ID and error policies remain at the entry path.
    fn enqueue(
        &mut self,
        envelope: Envelope,
        summon_reply: Option<Sender<PlatformCommandResult>>,
    ) -> std::result::Result<uuid::Uuid, QueueAdmissionError> {
        let command_id = envelope.id;
        let timeout = match &envelope.message {
            Message::PlatformCommand(PlatformCommand::FloatingAssistant(command))
                if matches!(
                    command.operation,
                    nxtools_protocol::floating_assistant::FloatingAssistantOperation::Retire
                        | nxtools_protocol::floating_assistant::FloatingAssistantOperation::PickImage
                ) =>
            {
                None
            }
            Message::PlatformCommand(PlatformCommand::OpenShellTarget(command))
                if matches!(
                    command.action,
                    nxtools_protocol::ShellOpenAction::OpenWith
                        | nxtools_protocol::ShellOpenAction::RunElevated
                ) =>
            {
                None
            }
            Message::PlatformCommand(PlatformCommand::LaunchProcess(command))
                if command.run_as_admin =>
            {
                None
            }
            Message::PlatformCommand(
                PlatformCommand::RawNumberedJoin(_)
                | PlatformCommand::InvokeShellAction(_)
                | PlatformCommand::EnsureIndexerService
                | PlatformCommand::BeginFileDrag(_),
            ) => None,
            Message::PlatformCommand(
                PlatformCommand::ArchivePicker(_)
                | PlatformCommand::PickArchivePath(_)
                | PlatformCommand::PickClipboardArchivePath(_)
                | PlatformCommand::PickClipboardStorageDirectory(_)
                | PlatformCommand::PickDirectory(_)
                | PlatformCommand::PickProgram(_)
                | PlatformCommand::PickMarkdownDocument(_)
                | PlatformCommand::PickMarkdownExport(_)
                | PlatformCommand::PickExtensionManifest(_)
                | PlatformCommand::PickExtensionRuntime(_)
                | PlatformCommand::PickExtensionSettingPath(_)
                | PlatformCommand::PickBrowserExtensionExportPath(_),
            ) => None,
            // The native PDF STA has a 30-second deadline; allow the result to return after it.
            Message::PlatformCommand(PlatformCommand::PrintMarkdownPdf(_)) => {
                Some(Duration::from_secs(35))
            }
            // The P206 native STA has a 15-second reply bound.
            Message::PlatformCommand(PlatformCommand::ArchiveShellIntegration(_)) => {
                Some(Duration::from_secs(20))
            }
            _ => Some(PENDING_COMMAND_TIMEOUT),
        };
        let active = self
            .active
            .as_ref()
            .map(|active| (active.generation, active.commands.clone()))
            .ok_or(QueueAdmissionError::Disconnected)?;
        self.pending.insert(
            command_id,
            PendingCommand {
                generation: active.0,
                accepted_at: Instant::now(),
                summon_reply,
                timeout,
            },
        );
        match active.1.try_send(envelope) {
            Ok(()) => {
                if let Some(connection) = &self.active {
                    let _ = connection.command_wake.try_send(());
                }
                Ok(command_id)
            }
            Err(error) => {
                self.pending.remove(&command_id);
                Err(match error {
                    async_channel::TrySendError::Full(_) => QueueAdmissionError::Full,
                    async_channel::TrySendError::Closed(_) => QueueAdmissionError::Closed,
                })
            }
        }
    }
}

type ReplacedConnection = Option<(u64, Vec<PendingTerminal>)>;
type OpenConnection = (u64, PlatformCommandReceiver, ReplacedConnection);

fn failed_platform_result(error: impl Into<String>) -> PlatformCommandResult {
    PlatformCommandResult {
        success: false,
        error: Some(error.into()),
        native_error_code: None,
        data: None,
    }
}
// The ordinary notification receiver remains owned by the Desktop dispatcher.
pub struct PlatformBridge {
    notifications: Receiver<PlatformNotification>,
    router: Arc<Mutex<CommandRouter>>,
    connected: Arc<AtomicBool>,
    archive: Arc<archive_read::ArchiveTransport>,
}

impl PlatformBridge {
    /// Shared archive router only: never clones/competes for ordinary UI notifications.
    pub fn archive_client_bridge(&self) -> Arc<Self> {
        let (sender, notifications) = async_channel::bounded(1);
        drop(sender);
        Arc::new(Self {
            notifications,
            router: Arc::clone(&self.router),
            connected: Arc::clone(&self.connected),
            archive: Arc::clone(&self.archive),
        })
    }
}
impl PlatformBridge {
    /// The active PlatformHost Hello capability snapshot, absent for synthetic test connections.
    pub fn quick_launch_panel_hotkey_available(&self) -> Option<bool> {
        self.router
            .lock()
            .ok()?
            .active
            .as_ref()?
            .quick_launch_panel_hotkey_available
    }
    pub fn archive_multi_folder_picker_available(&self) -> Option<bool> {
        self.router
            .lock()
            .ok()?
            .active
            .as_ref()?
            .archive_multi_folder_picker_available
    }
}

#[derive(Clone)]
pub struct PlatformServerEndpoint {
    notifications: Sender<PlatformNotification>,
    router: Arc<Mutex<CommandRouter>>,
    connected: Arc<AtomicBool>,
    connection_generation: Arc<AtomicU64>,
    contract_receiver: Arc<Mutex<Option<PlatformCommandReceiver>>>,
    primary_connection_lock: Arc<AsyncMutex<()>>,
    archive: Arc<archive_read::ArchiveTransport>,
    archive_intents: Option<desktop_archive_intent::ArchiveIntentEndpoint>,
    markdown_intents: Option<desktop_markdown_intent::MarkdownIntentEndpoint>,
}

pub fn platform_bridge(capacity: usize) -> (PlatformBridge, PlatformServerEndpoint) {
    let (notification_sender, notification_receiver) = async_channel::bounded(capacity);
    let connected = Arc::new(AtomicBool::new(false));
    let connection_generation = Arc::new(AtomicU64::new(0));
    let router = Arc::new(Mutex::new(CommandRouter {
        capacity,
        active: None,
        pending: HashMap::new(),
    }));
    let primary_connection_lock = Arc::new(AsyncMutex::new(()));
    let contract_receiver = Arc::new(Mutex::new(None));
    let archive = Arc::new(archive_read::ArchiveTransport::new());
    (
        PlatformBridge {
            notifications: notification_receiver,
            router: Arc::clone(&router),
            connected: Arc::clone(&connected),
            archive: Arc::clone(&archive),
        },
        PlatformServerEndpoint {
            notifications: notification_sender,
            router,
            connected,
            connection_generation,
            contract_receiver,
            primary_connection_lock,
            archive,
            archive_intents: None,
            markdown_intents: None,
        },
    )
}

impl PlatformServerEndpoint {
    pub fn with_archive_intents(
        mut self,
        endpoint: desktop_archive_intent::ArchiveIntentEndpoint,
    ) -> Self {
        self.archive_intents = Some(endpoint);
        self
    }

    pub fn with_markdown_intents(
        mut self,
        endpoint: desktop_markdown_intent::MarkdownIntentEndpoint,
    ) -> Self {
        self.markdown_intents = Some(endpoint);
        self
    }

    /// Pause only the original command consumer while a fixture fills its queue.
    #[doc(hidden)]
    pub fn pause_commands_for_contract_test(
        &self,
    ) -> Result<PlatformCommandPause, PlatformBridgeError> {
        let mut router = self
            .router
            .lock()
            .map_err(|_| PlatformBridgeError::Disconnected)?;
        let active = router
            .active
            .as_mut()
            .ok_or(PlatformBridgeError::Disconnected)?;
        active.commands_paused = true;
        Ok(PlatformCommandPause {
            router: self.router.clone(),
            generation: active.generation,
        })
    }
    /// In-memory lifecycle seam used by GPUI/native-boundary contract tests.
    #[doc(hidden)]
    pub fn set_connected_for_contract_test(&self, connected: bool) {
        if connected {
            let (_, _, replaced) = self.open_connection();
            self.try_send_disconnected(replaced);
            return;
        }
        let generation = self
            .router
            .lock()
            .ok()
            .and_then(|router| router.active.as_ref().map(|active| active.generation));
        if let Some(generation) = generation {
            if let Some(pending_command_ids) = self.disconnect_connection(generation) {
                self.try_send_disconnected(Some((generation, pending_command_ids)));
            }
        } else {
            self.connected.store(false, Ordering::Release);
        }
    }

    /// Publishes a fixture Hello capability on the current test connection.
    #[doc(hidden)]
    pub fn set_quick_launch_panel_hotkey_available_for_contract_test(&self, available: bool) {
        let generation = self.connection_generation.load(Ordering::Acquire);
        self.record_quick_launch_panel_hotkey(generation, available);
    }
    #[doc(hidden)]
    pub fn set_archive_multi_folder_picker_available_for_contract_test(&self, available: bool) {
        let generation = self.connection_generation.load(Ordering::Acquire);
        self.record_archive_multi_folder_picker(generation, available);
    }

    /// Opens an isolated command queue for a contract test or a live pipe connection.
    #[doc(hidden)]
    pub fn open_connection_for_contract_test(&self) -> PlatformCommandReceiver {
        let (_, receiver, replaced) = self.open_connection();
        self.try_send_disconnected(replaced);
        receiver
    }

    /// Observe routing progress without retrying or changing command dispatch.
    #[doc(hidden)]
    pub fn command_pending_for_contract_test(&self, id: uuid::Uuid) -> bool {
        self.router
            .lock()
            .expect("contract router")
            .pending
            .contains_key(&id)
    }

    /// Receives one command from the active contract-test connection queue.
    #[doc(hidden)]
    pub fn try_receive_command_for_contract_test(&self) -> Result<Envelope, String> {
        let receiver = self
            .contract_receiver
            .lock()
            .map_err(|error| error.to_string())?
            .clone()
            .ok_or_else(|| "no contract-test connection is open".to_owned())?;
        receiver.try_recv().map_err(|error| error.to_string())
    }

    /// Publishes one observed notification through the same bounded transport as PlatformHost.
    #[doc(hidden)]
    pub fn send_notification_for_contract_test(
        &self,
        notification: PlatformNotification,
    ) -> Result<(), String> {
        self.observe_archive_cleanup(&notification);
        self.observe_job_cleanup(&notification);
        self.observe_join_cleanup(&notification);
        self.observe_virtual_drag_cleanup(&notification);
        self.observe_authorization_cleanup(&notification);
        if self.archive.intercept(&notification) {
            return Ok(());
        }
        self.notifications
            .try_send(notification)
            .map_err(|error| error.to_string())
    }

    fn open_connection(&self) -> OpenConnection {
        let Ok(mut router) = self.router.lock() else {
            self.connected.store(false, Ordering::Release);
            let generation = self
                .connection_generation
                .fetch_add(1, Ordering::AcqRel)
                .wrapping_add(1);
            let (_, commands) = async_channel::bounded(1);
            let (_, wake) = async_channel::bounded(1);
            return (
                generation,
                PlatformCommandReceiver {
                    commands,
                    wake,
                    router: self.router.clone(),
                    generation,
                },
                None,
            );
        };
        let generation = self
            .connection_generation
            .fetch_add(1, Ordering::AcqRel)
            .wrapping_add(1);
        let (sender, receiver) = async_channel::bounded(router.capacity);
        let (command_wake, wake) = async_channel::bounded(1);
        let replaced = router.active.take().map(|active| {
            let pending_command_ids = router
                .pending
                .extract_if(|_, pending| pending.generation == active.generation)
                .map(|(id, pending)| PendingTerminal {
                    id,
                    summon_reply: pending.summon_reply,
                })
                .collect::<Vec<_>>();
            (active.generation, pending_command_ids)
        });
        router.active = Some(ActiveConnection {
            generation,
            commands_paused: false,
            floating_cleanup: None,
            commands: sender,
            command_wake,
            archive_reads: Vec::new(),
            archive_jobs: Vec::new(),
            raw_join: None,
            archive_drag: None,
            archive_authorizations: Vec::new(),
            cancel_turn: true,
            quick_launch_panel_hotkey_available: None,
            archive_multi_folder_picker_available: None,
        });
        let receiver = PlatformCommandReceiver {
            commands: receiver,
            wake,
            router: self.router.clone(),
            generation,
        };
        drop(router);
        self.archive.generation.store(generation, Ordering::Release);
        self.connected.store(true, Ordering::Release);
        if let Ok(mut contract_receiver) = self.contract_receiver.lock() {
            *contract_receiver = Some(receiver.clone());
        }
        (generation, receiver, replaced)
    }

    fn record_quick_launch_panel_hotkey(&self, generation: u64, available: bool) {
        if let Ok(mut router) = self.router.lock()
            && let Some(active) = router
                .active
                .as_mut()
                .filter(|active| active.generation == generation)
        {
            active.quick_launch_panel_hotkey_available = Some(available);
        }
    }
    fn record_archive_multi_folder_picker(&self, generation: u64, available: bool) {
        if let Ok(mut router) = self.router.lock()
            && let Some(active) = router
                .active
                .as_mut()
                .filter(|active| active.generation == generation)
        {
            active.archive_multi_folder_picker_available = Some(available);
        }
    }

    fn try_send_disconnected(&self, disconnected: ReplacedConnection) {
        if let Some((generation, pending)) = disconnected {
            self.archive.intercept(&PlatformNotification::Disconnected {
                generation,
                pending_command_ids: Vec::new(),
            });
            let pending_command_ids = pending.iter().map(|pending| pending.id).collect::<Vec<_>>();
            for pending_command in pending {
                if let Some(reply) = pending_command.summon_reply {
                    let _ = reply.try_send(failed_platform_result("platform host disconnected"));
                }
            }
            let _ = self
                .notifications
                .try_send(PlatformNotification::Disconnected {
                    generation,
                    pending_command_ids,
                });
        }
    }

    async fn send_notification(&self, notification: PlatformNotification) -> Result<()> {
        self.observe_archive_cleanup(&notification);
        self.observe_job_cleanup(&notification);
        self.observe_join_cleanup(&notification);
        self.observe_virtual_drag_cleanup(&notification);
        self.observe_authorization_cleanup(&notification);
        if self.archive.intercept(&notification) {
            return Ok(());
        }
        // Waiting for an unrelated consumer here also stops archive cancellation writes.
        // Fail the ORIGINAL pipe owner instead: handle_connection retires this generation
        // and drops both halves, so the C# session sees real EOF and drains its original Jobs.
        self.notifications
            .try_send(notification)
            .map_err(|failure| {
                warn!(
                    event = "platform.notification.retired",
                    connection_generation = self.archive.generation.load(Ordering::Acquire),
                    reason = if failure.is_full() {
                        "notification_queue_full"
                    } else {
                        "notification_receiver_closed"
                    },
                    "platform pipe notification delivery failed"
                );
                anyhow::anyhow!("desktop platform notification delivery unavailable")
            })
    }

    fn send_summon_command(
        &self,
        envelope: Envelope,
        reply: Sender<PlatformCommandResult>,
    ) -> Result<()> {
        let mut router = self
            .router
            .lock()
            .map_err(|_| anyhow::anyhow!("platform command router is poisoned"))?;
        router
            .enqueue(envelope, Some(reply))
            .map(|_| ())
            .map_err(|error| {
                anyhow::anyhow!(match error {
                    QueueAdmissionError::Disconnected => "platform host is disconnected",
                    QueueAdmissionError::Full => "platform host command queue is full",
                    QueueAdmissionError::Closed => "platform host command queue is closed",
                })
            })
    }

    fn complete_command(&self, generation: u64, command_id: uuid::Uuid) -> Option<PendingCommand> {
        self.router.lock().ok().and_then(|mut router| {
            router
                .pending
                .get(&command_id)
                .is_some_and(|pending| pending.generation == generation)
                .then(|| {
                    router
                        .pending
                        .remove(&command_id)
                        .expect("pending command exists")
                })
        })
    }

    fn disconnect_connection(&self, generation: u64) -> Option<Vec<PendingTerminal>> {
        let pending = self.router.lock().ok().and_then(|mut router| {
            if router
                .active
                .as_ref()
                .is_none_or(|active| active.generation != generation)
            {
                return None;
            }
            router.active = None;
            Some(
                router
                    .pending
                    .extract_if(|_, pending| pending.generation == generation)
                    .map(|(id, pending)| PendingTerminal {
                        id,
                        summon_reply: pending.summon_reply,
                    })
                    .collect::<Vec<_>>(),
            )
        });
        if pending.is_some() {
            self.connected.store(false, Ordering::Release);
            if let Some(intents) = &self.archive_intents {
                intents.native_disconnected();
            }
        }
        pending
    }

    fn is_active_generation(&self, generation: u64) -> bool {
        self.router.lock().ok().is_some_and(|router| {
            router
                .active
                .as_ref()
                .is_some_and(|active| active.generation == generation)
        })
    }

    fn expire_pending_commands(&self, generation: u64, now: Instant) -> Vec<PendingTerminal> {
        self.router
            .lock()
            .map(|mut router| {
                let held_drag_receipts = router
                    .active
                    .as_ref()
                    .and_then(|active| active.archive_drag.as_ref());
                let held_drag_ids = router
                    .pending
                    .keys()
                    .copied()
                    .filter(|id| {
                        held_drag_receipts.is_some_and(|drag| drag.receipt(*id))
                            || router
                                .active
                                .as_ref()
                                .is_some_and(|active| active.holds_authorization_receipt(*id))
                    })
                    .collect::<Vec<_>>();
                router
                    .pending
                    .extract_if(|id, pending| {
                        pending.generation == generation
                            && !held_drag_ids.contains(id)
                            && pending.timeout.is_some_and(|timeout| {
                                now.duration_since(pending.accepted_at) >= timeout
                            })
                    })
                    .map(|(id, pending)| PendingTerminal {
                        id,
                        summon_reply: pending.summon_reply,
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    async fn send_timeout_results(&self, pending: Vec<PendingTerminal>) -> Result<()> {
        for pending in pending {
            let result = failed_platform_result("platform command timed out");
            if let Some(reply) = pending.summon_reply {
                let _ = reply.send(result).await;
            } else {
                self.send_notification(PlatformNotification::CommandResult {
                    correlation_id: pending.id,
                    result,
                })
                .await?;
            }
        }
        Ok(())
    }

    async fn send_disconnected_terminals(
        &self,
        generation: u64,
        pending: Vec<PendingTerminal>,
    ) -> Result<()> {
        let pending_command_ids = pending.iter().map(|pending| pending.id).collect();
        for pending in pending {
            if let Some(reply) = pending.summon_reply {
                let _ = reply
                    .send(failed_platform_result("platform host disconnected"))
                    .await;
            }
        }
        self.send_notification(PlatformNotification::Disconnected {
            generation,
            pending_command_ids,
        })
        .await
    }
}

impl PlatformBridge {
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Acquire)
    }

    pub fn notification_receiver(&self) -> Receiver<PlatformNotification> {
        self.notifications.clone()
    }

    pub fn send_command(
        &self,
        command: PlatformCommand,
    ) -> Result<uuid::Uuid, PlatformBridgeError> {
        self.send_command_envelope(Envelope::new(Message::PlatformCommand(command)))
    }

    /// Sends a command using an ID already installed in caller-owned pending state.
    pub fn send_command_with_id(
        &self,
        command: PlatformCommand,
        command_id: uuid::Uuid,
    ) -> Result<uuid::Uuid, PlatformBridgeError> {
        let mut envelope = Envelope::new(Message::PlatformCommand(command));
        envelope.id = command_id;
        self.send_command_envelope(envelope)
    }

    /// Uses caller-installed receipt identity while preserving one cross-process operation ID.
    pub fn send_command_with_id_correlated(
        &self,
        command: PlatformCommand,
        command_id: uuid::Uuid,
        correlation_id: uuid::Uuid,
    ) -> Result<uuid::Uuid, PlatformBridgeError> {
        let mut envelope = Envelope::new(Message::PlatformCommand(command));
        envelope.id = command_id;
        envelope.correlation_id = Some(correlation_id);
        self.send_command_envelope(envelope)
    }

    pub fn send_command_correlated(
        &self,
        command: PlatformCommand,
        correlation_id: uuid::Uuid,
    ) -> Result<uuid::Uuid, PlatformBridgeError> {
        let mut envelope = Envelope::new(Message::PlatformCommand(command));
        envelope.correlation_id = Some(correlation_id);
        self.send_command_envelope(envelope)
    }

    /// Passive archive demand is bound to the original read connection, under the same router lock.
    pub fn send_archive_preview(
        &self,
        command: nxtools_protocol::archive_browser::ArchiveEntryCommand,
        receipt: uuid::Uuid,
        origin: u64,
    ) -> Result<uuid::Uuid, PlatformBridgeError> {
        if !matches!(
            command.action,
            nxtools_protocol::archive_browser::ArchiveEntryAction::Preview
                | nxtools_protocol::archive_browser::ArchiveEntryAction::PreviewText
        ) || command.validate().is_err()
        {
            return Err(PlatformBridgeError::Disconnected);
        }
        let operation = command.operation_id;
        let mut envelope = Envelope::new(Message::PlatformCommand(PlatformCommand::ArchiveEntry(
            command,
        )));
        envelope.id = receipt;
        envelope.correlation_id = Some(operation);
        let mut router = self
            .router
            .lock()
            .map_err(|_| PlatformBridgeError::Disconnected)?;
        if router
            .active
            .as_ref()
            .is_none_or(|active| active.generation != origin)
        {
            return Err(PlatformBridgeError::Disconnected);
        }
        if router.pending.contains_key(&receipt) {
            return Err(PlatformBridgeError::DuplicateCommandId);
        }
        router.enqueue(envelope, None).map_err(|error| match error {
            QueueAdmissionError::Full => PlatformBridgeError::Busy,
            _ => PlatformBridgeError::Disconnected,
        })
    }
    fn send_command_envelope(&self, envelope: Envelope) -> Result<uuid::Uuid, PlatformBridgeError> {
        if matches!(
            &envelope.message,
            Message::PlatformCommand(PlatformCommand::RawNumberedJoin(_))
        ) {
            return Err(PlatformBridgeError::Disconnected);
        }
        if matches!(&envelope.message,Message::PlatformCommand(PlatformCommand::ArchiveEntry(command)) if matches!(command.action,nxtools_protocol::archive_browser::ArchiveEntryAction::Authorize|nxtools_protocol::archive_browser::ArchiveEntryAction::Revoke))
        {
            return Err(PlatformBridgeError::Disconnected); // Preparation/revoke require captured origin.
        }
        let virtual_drag = match &envelope.message {
            Message::PlatformCommand(
                PlatformCommand::ArchiveVirtualDrag(command)
                | PlatformCommand::ArchiveVirtualDragSpike(command),
            ) => Some((**command).clone()),
            _ => None,
        };
        if let Some(command) = virtual_drag {
            return self.send_virtual_drag(envelope, &command, None);
        }
        let command_id = envelope.id;
        let mut router = self
            .router
            .lock()
            .map_err(|_| PlatformBridgeError::Disconnected)?;
        if router.pending.contains_key(&command_id) {
            return Err(PlatformBridgeError::DuplicateCommandId);
        }
        router.enqueue(envelope, None).map_err(|error| match error {
            QueueAdmissionError::Full => PlatformBridgeError::Busy,
            QueueAdmissionError::Disconnected | QueueAdmissionError::Closed => {
                PlatformBridgeError::Disconnected
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformNotification {
    Connected,
    Disconnected {
        generation: u64,
        pending_command_ids: Vec<uuid::Uuid>,
    },
    Event(PlatformEvent),
    CommandResult {
        correlation_id: uuid::Uuid,
        result: PlatformCommandResult,
    },
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PlatformBridgeError {
    #[error("the Windows platform host is disconnected")]
    Disconnected,
    #[error("the Windows platform command queue is full")]
    Busy,
    #[error("the caller-supplied platform command ID is already pending")]
    DuplicateCommandId,
}

pub fn dispatch(message: &Envelope) -> Option<Envelope> {
    let response = match &message.message {
        Message::Hello(hello) => {
            const SUPPORTED_CAPABILITIES: [&str; 36] = [
                "archive_read_v1",
                "archive_job_large_count_v1",
                "clipboard_file_drop_read_v1",
                "archive_create_destination_v1",
                nxtools_protocol::archive_picker::ARCHIVE_MULTI_FOLDER_PICKER_CAPABILITY,
                "foreground_context",
                "clipboard_text_capture",
                "clipboard_image_capture",
                "clipboard_files_capture",
                "clipboard_native_formats",
                "paste_text",
                "paste_image",
                "paste_files",
                "clipboard_panel_hotkey",
                "clipboard_configuration",
                "quick_launch_panel_hotkey",
                "quick_launch_keyboard_health_v1",
                "quick_launch_configuration",
                "clipboard_archive_picker",
                "clipboard_storage_picker",
                "window_topmost",
                "window_no_activate",
                "window_bounds",
                "taskbar_progress",
                "taskbar_progress_overlay",
                "window_visibility",
                "window_minimized",
                "window_position",
                "shell_icon_extraction",
                "shell_context_actions",
                "directory_navigation",
                "file_dialog_search",
                "window_activation",
                "indexer_service_management",
                "desktop_shell",
                "tray_events",
            ];
            let accepted_capabilities = hello
                .capabilities
                .iter()
                .filter(|capability| SUPPORTED_CAPABILITIES.contains(&capability.as_str()))
                .cloned()
                .collect();
            info!(
                client_name = %hello.client_name,
                client_version = %hello.client_version,
                "platform host handshake accepted"
            );
            Message::HelloAck(HelloAck {
                server_name: "nxtools_desktop".into(),
                server_version: env!("CARGO_PKG_VERSION").into(),
                accepted_capabilities,
            })
        }
        Message::Ping(ping) => Message::Pong(Pong {
            sent_at_unix_ms: ping.sent_at_unix_ms,
        }),
        Message::PlatformEvent(_) => return None,
        Message::HelloAck(_)
        | Message::Pong(_)
        | Message::PlatformCommand(_)
        | Message::PlatformCommandResult(_)
        | Message::IndexerCommand(_)
        | Message::DesktopArchiveIntent(_)
        | Message::DesktopArchiveReceipt(_)
        | Message::DesktopMarkdownIntent(_)
        | Message::DesktopMarkdownReceipt(_)
        | Message::IndexerEvent(_)
        | Message::Error(_) => return None,
    };
    Some(Envelope::reply_to(message, response))
}

#[cfg(windows)]
pub async fn run_platform_server(pipe_name: &str, endpoint: PlatformServerEndpoint) -> Result<()> {
    let pipe_path = format!(r"\\.\pipe\{pipe_name}");
    info!(%pipe_path, "platform IPC server started");
    // One primary plus a finite number of short-lived secondary/handshake connections.
    let slots = Arc::new(tokio::sync::Semaphore::new(40));
    let mut server = local_pipe::create_current_user_async_pipe(&pipe_path, true)?;
    let mut retry_delay = Duration::from_millis(10);
    let mut last_transient_log = std::time::Instant::now() - Duration::from_secs(1);
    loop {
        if let Err(error) = server.connect().await {
            if !local_pipe::transient_accept_error(&error) {
                return Err(error).context("platform pipe accept failed");
            }
            if last_transient_log.elapsed() >= Duration::from_secs(1) {
                warn!(
                    event = "platform.pipe.accept.transient",
                    native_code = error.raw_os_error(),
                    "transient platform accept; recreating instance"
                );
                last_transient_log = std::time::Instant::now();
            }
            drop(server);
            tokio::time::sleep(retry_delay).await;
            retry_delay = (retry_delay * 2).min(Duration::from_millis(100));
            server = local_pipe::create_current_user_async_pipe(&pipe_path, false)?;
            continue;
        }
        retry_delay = Duration::from_millis(10);
        let next = local_pipe::create_current_user_async_pipe(&pipe_path, false)?;
        let connection = std::mem::replace(&mut server, next);
        let Ok(slot) = slots.clone().try_acquire_owned() else {
            drop(connection);
            continue;
        };
        let connection_endpoint = endpoint.clone();
        tokio::spawn(async move {
            let _slot = slot;
            if handle_connection(connection, &connection_endpoint)
                .await
                .is_err()
            {
                warn!("platform connection ended with an error");
            }
        });
    }
}

#[cfg(not(windows))]
pub async fn run_platform_server(
    _pipe_name: &str,
    _endpoint: PlatformServerEndpoint,
) -> Result<()> {
    anyhow::bail!("the platform host named pipe is only available on Windows")
}

#[cfg(windows)]
async fn handle_connection(
    server: NamedPipeServer,
    endpoint: &PlatformServerEndpoint,
) -> Result<()> {
    use std::os::windows::io::AsRawHandle;
    let desktop_authorized =
        local_pipe::authorize_desktop_peer(server.as_raw_handle(), true).is_ok();
    info!("platform host connected");
    let (reader, mut writer) = tokio::io::split(server);
    let mut reader = BufReader::new(reader);

    let Some(handshake_frame) =
        tokio::time::timeout(Duration::from_secs(3), read_bounded_frame(&mut reader))
            .await
            .context("platform handshake expired")??
    else {
        anyhow::bail!("platform host disconnected before handshake");
    };
    let handshake = Envelope::decode_line(&handshake_frame).context("invalid handshake frame")?;
    if matches!(handshake.message, Message::DesktopArchiveIntent(_)) {
        return desktop_archive_intent::serve(
            handshake,
            endpoint.archive_intents.as_ref(),
            desktop_authorized,
            &mut writer,
        )
        .await;
    }
    if matches!(handshake.message, Message::DesktopMarkdownIntent(_)) {
        return desktop_markdown_intent::serve(
            handshake,
            endpoint.markdown_intents.as_ref(),
            desktop_authorized,
            &mut writer,
        )
        .await;
    }
    if matches!(
        handshake.message,
        Message::PlatformCommand(PlatformCommand::ArchiveShellIntegration(_))
    ) {
        return install_archive_integration::serve(
            handshake,
            endpoint,
            desktop_authorized,
            &mut writer,
        )
        .await;
    }
    let Message::Hello(hello) = &handshake.message else {
        anyhow::bail!("platform host did not begin with a hello message");
    };
    let acknowledgement =
        dispatch(&handshake).context("hello did not produce an acknowledgement")?;
    tokio::time::timeout(
        Duration::from_secs(2),
        write_envelope(&mut writer, &acknowledgement),
    )
    .await
    .context("platform handshake acknowledgement expired")??;

    if hello.client_name == "nxtools-desktop-summon" {
        return handle_summon_connection(reader, &mut writer, endpoint).await;
    }
    let _primary_connection_guard = tokio::time::timeout(
        Duration::from_secs(3),
        endpoint.primary_connection_lock.lock(),
    )
    .await
    .context("platform primary connection admission expired")?;

    let (generation, command_receiver, replaced) = endpoint.open_connection();
    endpoint.record_quick_launch_panel_hotkey(
        generation,
        hello
            .capabilities
            .iter()
            .any(|capability| capability == "quick_launch_panel_hotkey"),
    );
    endpoint.record_archive_multi_folder_picker(
        generation,
        hello.capabilities.iter().any(|capability| {
            capability == nxtools_protocol::archive_picker::ARCHIVE_MULTI_FOLDER_PICKER_CAPABILITY
        }),
    );
    if let Some((old_generation, pending)) = replaced
        && let Err(error) = endpoint
            .send_disconnected_terminals(old_generation, pending)
            .await
    {
        let _ = endpoint.disconnect_connection(generation);
        return Err(error);
    }
    if let Err(error) = endpoint
        .send_notification(PlatformNotification::Connected)
        .await
    {
        let _ = endpoint.disconnect_connection(generation);
        return Err(error);
    }

    let mut sweep = tokio::time::interval(PENDING_COMMAND_SWEEP);
    // select may cancel a pending read for sweep/commands. Consumed bytes belong to the
    // ORIGINAL connection, not to the disposable read future.
    let mut pending_frame = Vec::with_capacity(4096);
    let connection_result: Result<()> = async {
        loop {
            tokio::select! {
                _ = sweep.tick() => {
                    let expired = endpoint.expire_pending_commands(generation, Instant::now());
                    endpoint.send_timeout_results(expired).await?;
                }
                frame = read_bounded_frame_into(&mut reader, &mut pending_frame) => {
                    let Some(frame) = frame? else {
                        info!(generation, "platform host disconnected");
                        break Ok(());
                    };
                    let envelope = Envelope::decode_line(&frame).context("invalid platform frame")?;
                    match &envelope.message {
                        Message::PlatformEvent(event) => {
                            if endpoint.is_active_generation(generation) {
                                if matches!(event, PlatformEvent::ClipboardPanelRequested(_)) {
                                    let correlation_id = envelope.correlation_id.unwrap_or(envelope.id);
                                    info!(event = "clipboard.summon_received", %correlation_id, generation,
                                        "clipboard panel request received");
                                }
                                endpoint
                                    .send_notification(PlatformNotification::Event(event.clone()))
                                    .await?;
                            } else {
                                warn!(generation, "ignored platform event from a stale connection");
                            }
                        }
                        Message::PlatformCommandResult(result) => {
                            let correlation_id = envelope
                                .correlation_id
                                .context("platform command result had no correlation ID")?;
                            if let Some(pending) = endpoint.complete_command(generation, correlation_id) {
                                if let Some(reply) = pending.summon_reply {
                                    let _ = reply.send(result.clone()).await;
                                } else {
                                    endpoint
                                        .send_notification(PlatformNotification::CommandResult {
                                            correlation_id,
                                            result: result.clone(),
                                        })
                                        .await?;
                                }
                            } else {
                                warn!(
                                    generation,
                                    %correlation_id,
                                    "ignored platform command result for a stale or unknown command"
                                );
                            }
                        }
                        _ => {
                            if let Some(response) = dispatch(&envelope) {
                                write_envelope(&mut writer, &response).await?;
                            }
                        }
                    }
                }
                command = command_receiver.recv() => {
                    let envelope = command.context("desktop platform command sender closed")?;
                    write_envelope(&mut writer, &envelope).await?;
                }
            }
        }
    }
    .await;

    if let Some(pending) = endpoint.disconnect_connection(generation) {
        let disconnect_result = endpoint
            .send_disconnected_terminals(generation, pending)
            .await;
        return connection_result.and(disconnect_result);
    }
    connection_result
}

#[cfg(windows)]
async fn handle_summon_connection(
    mut reader: BufReader<tokio::io::ReadHalf<NamedPipeServer>>,
    writer: &mut tokio::io::WriteHalf<NamedPipeServer>,
    endpoint: &PlatformServerEndpoint,
) -> Result<()> {
    loop {
        let Some(frame) = read_bounded_frame(&mut reader).await? else {
            return Ok(());
        };
        let envelope = Envelope::decode_line(&frame).context("invalid summon frame")?;
        if matches!(
            envelope.message,
            Message::PlatformCommand(PlatformCommand::ArchiveShellIntegration(_))
        ) {
            anyhow::bail!("archive integration requires the authenticated installer boundary");
        }
        if matches!(envelope.message, Message::PlatformCommand(_)) {
            let request = envelope.clone();
            let (reply_sender, reply_receiver) = async_channel::bounded(1);
            let result = match endpoint.send_summon_command(envelope, reply_sender) {
                Ok(()) => reply_receiver
                    .recv()
                    .await
                    .unwrap_or_else(|_| failed_platform_result("summon result channel closed")),
                Err(error) => failed_platform_result(error.to_string()),
            };
            let response = Envelope::reply_to(&request, Message::PlatformCommandResult(result));
            write_envelope(writer, &response).await?;
        }
    }
}

#[cfg(windows)]
async fn write_envelope<W>(writer: &mut W, envelope: &Envelope) -> Result<()>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    let mut encoded = envelope.encode_line()?.into_bytes();
    let result = writer.write_all(&encoded).await;
    encoded.fill(0);
    std::hint::black_box(&encoded);
    result.context("failed to write platform frame")?;
    writer
        .flush()
        .await
        .context("failed to flush platform frame")
}

async fn read_bounded_frame<R>(reader: &mut R) -> Result<Option<String>>
where
    R: AsyncBufRead + Unpin,
{
    read_bounded_frame_into(reader, &mut Vec::with_capacity(4096)).await
}

async fn read_bounded_frame_into<R>(reader: &mut R, frame: &mut Vec<u8>) -> Result<Option<String>>
where
    R: AsyncBufRead + Unpin,
{
    loop {
        let available = reader.fill_buf().await.context("failed to read frame")?;
        if available.is_empty() {
            if frame.is_empty() {
                return Ok(None);
            }
            anyhow::bail!("platform host disconnected in the middle of a frame");
        }

        let newline = available.iter().position(|byte| *byte == b'\n');
        let payload_bytes = newline.unwrap_or(available.len());
        if frame.len() + payload_bytes > MAX_FRAME_BYTES {
            anyhow::bail!("platform host sent a frame larger than {MAX_FRAME_BYTES} bytes");
        }
        frame.extend_from_slice(&available[..payload_bytes]);
        reader.consume(payload_bytes + usize::from(newline.is_some()));

        if newline.is_some() {
            return String::from_utf8(std::mem::take(frame))
                .map(Some)
                .context("platform host frame was not valid UTF-8");
        }
    }
}

pub fn unix_time_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

pub fn read_bounded_sync_frame(reader: &mut impl BufRead) -> Result<Option<String>> {
    let mut frame = Vec::with_capacity(4096);
    loop {
        let available = reader.fill_buf().context("failed to read IPC frame")?;
        if available.is_empty() {
            if frame.is_empty() {
                return Ok(None);
            }
            anyhow::bail!("IPC peer disconnected in the middle of a frame");
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let payload_bytes = newline.unwrap_or(available.len());
        if frame.len() + payload_bytes > MAX_FRAME_BYTES {
            anyhow::bail!("IPC peer sent a frame larger than {MAX_FRAME_BYTES} bytes");
        }
        frame.extend_from_slice(&available[..payload_bytes]);
        reader.consume(payload_bytes + usize::from(newline.is_some()));
        if newline.is_some() {
            return String::from_utf8(frame)
                .map(Some)
                .context("IPC frame was not valid UTF-8");
        }
    }
}

pub fn write_envelope_sync(writer: &mut impl Write, envelope: &Envelope) -> Result<()> {
    let mut encoded = envelope.encode_line()?.into_bytes();
    let result = writer.write_all(&encoded);
    encoded.fill(0);
    std::hint::black_box(&encoded);
    result.context("failed to write IPC frame")?;
    writer.flush().context("failed to flush IPC frame")
}

#[cfg(test)]
mod tests {
    use nxtools_protocol::{Hello, Message, PasteTextCommand, Ping, PlatformCommand};
    use tokio::io::BufReader;

    use super::*;
    fn summon_request(text: &str) -> Envelope {
        Envelope::new(Message::PlatformCommand(PlatformCommand::PasteText(
            PasteTextCommand {
                text: text.into(),
                paste_after_write: true,
                target_window_handle: 42,
                target_focus_window_handle: 43,
            },
        )))
    }

    fn human_shell_commands() -> Vec<(&'static str, PlatformCommand)> {
        use nxtools_protocol::{
            BeginFileDragCommand, InvokeShellActionCommand, LaunchProcessCommand,
            OpenShellTargetCommand, ShellOpenAction,
        };
        let shell = |action| {
            PlatformCommand::OpenShellTarget(OpenShellTargetCommand {
                target: "C:/fixture/app.exe".into(),
                action,
            })
        };
        vec![
            ("open_with", shell(ShellOpenAction::OpenWith)),
            ("run_elevated", shell(ShellOpenAction::RunElevated)),
            (
                "admin_process",
                PlatformCommand::LaunchProcess(LaunchProcessCommand {
                    executable: "C:/fixture/app.exe".into(),
                    arguments: vec![],
                    working_directory: None,
                    run_as_admin: true,
                    run_silently: false,
                }),
            ),
            (
                "indexer_authorization",
                PlatformCommand::EnsureIndexerService,
            ),
            (
                "shell_invoke",
                PlatformCommand::InvokeShellAction(InvokeShellActionCommand {
                    session_id: uuid::Uuid::new_v4().to_string(),
                    command_id: 1,
                    owner_window_handle: 42,
                }),
            ),
            (
                "file_drag",
                PlatformCommand::BeginFileDrag(BeginFileDragCommand {
                    path: "C:/fixture/file.bin".into(),
                    source_window_handle: 42,
                }),
            ),
        ]
    }

    #[tokio::test]
    async fn human_shell_commands_outlive_machine_deadlines_and_route_terminal_once() {
        for (name, command) in human_shell_commands() {
            for cancelled in [false, true] {
                let (bridge, server) = platform_bridge(1);
                let receiver = server.open_connection_for_contract_test();
                let generation = bridge.connection_generation().expect("original connection");
                let id = uuid::Uuid::new_v4();
                bridge
                    .send_command_with_id(command.clone(), id)
                    .expect("admit human interaction");
                let envelope = receiver.try_recv().expect("original command dispatch");
                assert_eq!(envelope.id, id);
                assert_eq!(envelope.message, Message::PlatformCommand(command.clone()));
                let accepted = server.router.lock().expect("router").pending[&id].accepted_at;
                for seconds in [11, 121, 601, 3601] {
                    assert!(
                        server
                            .expire_pending_commands(
                                generation,
                                accepted + Duration::from_secs(seconds),
                            )
                            .is_empty(),
                        "{name}: human wait must retain the original pending request"
                    );
                }
                assert!(bridge.notification_receiver().try_recv().is_err());
                assert!(server.complete_command(generation, id).is_some());
                let terminal = PlatformCommandResult {
                    success: !cancelled,
                    error: None,
                    native_error_code: None,
                    data: Some(if cancelled { "cancelled" } else { "completed" }.into()),
                };
                server
                    .send_notification(PlatformNotification::CommandResult {
                        correlation_id: id,
                        result: terminal.clone(),
                    })
                    .await
                    .expect("forward original delayed terminal");
                assert!(matches!(
                    bridge.notification_receiver().try_recv(),
                    Ok(PlatformNotification::CommandResult { correlation_id, result })
                        if correlation_id == id && result == terminal
                ));
                assert!(server.complete_command(generation, id).is_none());
            }
        }
    }

    #[test]
    fn human_shell_commands_retire_on_disconnect_without_consuming_replacement_results() {
        for (name, command) in human_shell_commands() {
            let (bridge, server) = platform_bridge(1);
            let receiver = server.open_connection_for_contract_test();
            let original = bridge.connection_generation().expect("original connection");
            let id = bridge.send_command(command.clone()).expect("human command");
            assert_eq!(receiver.try_recv().expect("original dispatch").id, id);
            server.set_connected_for_contract_test(false);
            assert!(!server.command_pending_for_contract_test(id), "{name}");
            assert!(matches!(
                bridge.notification_receiver().try_recv(),
                Ok(PlatformNotification::Disconnected { generation, pending_command_ids })
                    if generation == original && pending_command_ids == vec![id]
            ));
            let replacement = server.open_connection_for_contract_test();
            let current = bridge
                .connection_generation()
                .expect("replacement connection");
            bridge
                .send_command_with_id(command, id)
                .expect("replacement command");
            assert_eq!(replacement.try_recv().expect("replacement dispatch").id, id);
            assert!(server.complete_command(original, id).is_none(), "{name}");
            assert!(server.command_pending_for_contract_test(id), "{name}");
            assert!(server.complete_command(current, id).is_some(), "{name}");
            assert!(server.complete_command(current, id).is_none(), "{name}");
        }
    }

    #[tokio::test]
    async fn machine_shell_commands_keep_their_original_timeout() {
        use nxtools_protocol::{
            LaunchProcessCommand, OpenShellTargetCommand, QueryShellActionsCommand,
            QueryShellSubmenuCommand, ShellOpenAction,
        };
        let shell = |action| {
            PlatformCommand::OpenShellTarget(OpenShellTargetCommand {
                target: "C:/fixture/app.exe".into(),
                action,
            })
        };
        let commands = [
            shell(ShellOpenAction::Open),
            shell(ShellOpenAction::SelectInExplorer),
            PlatformCommand::LaunchProcess(LaunchProcessCommand {
                executable: "C:/fixture/app.exe".into(),
                arguments: vec![],
                working_directory: None,
                run_as_admin: false,
                run_silently: false,
            }),
            PlatformCommand::QueryShellActions(QueryShellActionsCommand {
                target: "C:/fixture/file.bin".into(),
            }),
            PlatformCommand::QueryShellSubmenu(QueryShellSubmenuCommand {
                session_id: uuid::Uuid::new_v4().to_string(),
                submenu_id: 1,
            }),
        ];
        for command in commands {
            let (bridge, server) = platform_bridge(1);
            let receiver = server.open_connection_for_contract_test();
            let generation = bridge.connection_generation().expect("machine connection");
            let id = bridge.send_command(command).expect("machine command");
            assert_eq!(receiver.try_recv().expect("machine dispatch").id, id);
            let accepted = server.router.lock().expect("router").pending[&id].accepted_at;
            let expired =
                server.expire_pending_commands(generation, accepted + Duration::from_secs(11));
            assert_eq!(expired.len(), 1);
            assert_eq!(expired[0].id, id);
            server
                .send_timeout_results(expired)
                .await
                .expect("machine timeout");
            assert!(matches!(
                bridge.notification_receiver().try_recv(),
                Ok(PlatformNotification::CommandResult { correlation_id, result })
                    if correlation_id == id && !result.success
                        && result.error.as_deref() == Some("platform command timed out")
            ));
            assert!(server.complete_command(generation, id).is_none());
        }
    }

    #[test]
    fn human_pickers_outlive_machine_deadlines_and_obey_generation() {
        use nxtools_protocol::{
            ClipboardArchivePickerMode, ExportBrowserExtensionCommand, MarkdownExportFormat,
            PickArchivePathCommand, PickBrowserExtensionExportPathCommand,
            PickClipboardArchivePathCommand, PickClipboardStorageDirectoryCommand,
            PickDirectoryCommand, PickExtensionManifestCommand, PickExtensionRuntimeCommand,
            PickExtensionSettingPathCommand, PickMarkdownDocumentCommand,
            PickMarkdownExportCommand, PickProgramCommand,
        };
        let pickers = [
            PlatformCommand::PickDirectory(PickDirectoryCommand {
                current_directory: String::new(),
                title: String::new(),
                source_window_handle: 1,
                source_process_id: 2,
            }),
            PlatformCommand::PickProgram(PickProgramCommand {
                current_path: String::new(),
                source_window_handle: 1,
                source_process_id: 2,
            }),
            PlatformCommand::PickMarkdownDocument(PickMarkdownDocumentCommand {
                source_window_handle: 1,
                source_process_id: 2,
            }),
            PlatformCommand::PickMarkdownExport(PickMarkdownExportCommand {
                source_window_handle: 1,
                source_process_id: 2,
                format: MarkdownExportFormat::Html,
                suggested_name: "report".into(),
            }),
            PlatformCommand::PickExtensionManifest(PickExtensionManifestCommand {
                current_path: String::new(),
                source_window_handle: 1,
                source_process_id: 2,
            }),
            PlatformCommand::PickExtensionRuntime(PickExtensionRuntimeCommand {
                program: "node".into(),
                current_path: String::new(),
                source_window_handle: 1,
                source_process_id: 2,
            }),
            PlatformCommand::PickExtensionSettingPath(PickExtensionSettingPathCommand {
                kind: "file".into(),
                current_path: String::new(),
                source_window_handle: 1,
                source_process_id: 2,
            }),
            PlatformCommand::PickClipboardStorageDirectory(PickClipboardStorageDirectoryCommand {
                current_directory: String::new(),
                source_window_handle: 1,
                source_process_id: 2,
            }),
            PlatformCommand::PickClipboardArchivePath(PickClipboardArchivePathCommand {
                mode: ClipboardArchivePickerMode::Export,
                suggested_file_name: String::new(),
                source_window_handle: 1,
                source_process_id: 2,
            }),
            PlatformCommand::PickArchivePath(PickArchivePathCommand { source_hwnd: 1 }),
            PlatformCommand::PickBrowserExtensionExportPath(
                PickBrowserExtensionExportPathCommand {
                    format: "firefox".into(),
                    source_window_handle: 1,
                    source_process_id: 2,
                },
            ),
            PlatformCommand::ArchivePicker(
                nxtools_protocol::archive_picker::ArchivePickerCommand {
                    version: 1,
                    operation_id: uuid::Uuid::new_v4(),
                    session_id: uuid::Uuid::new_v4(),
                    generation: 1,
                    owner_window_handle: 1,
                    purpose: nxtools_protocol::archive_picker::ArchivePickerPurpose::Files,
                    current_directory: None,
                    suggested_name: None,
                },
            ),
        ];
        for command in pickers {
            let (bridge, server) = platform_bridge(4);
            server.set_connected_for_contract_test(true);
            let id = bridge.send_command(command).expect("queue human picker");
            let generation = server.connection_generation.load(Ordering::Acquire);
            let accepted = server.router.lock().expect("router").pending[&id].accepted_at;
            for seconds in [11, 121, 601] {
                assert!(
                    server
                        .expire_pending_commands(
                            generation,
                            accepted + Duration::from_secs(seconds)
                        )
                        .is_empty()
                );
            }
            assert!(server.complete_command(generation, id).is_some());
            assert!(server.complete_command(generation, id).is_none());
        }
        let (bridge, server) = platform_bridge(4);
        server.set_connected_for_contract_test(true);
        let machine = bridge
            .send_command(PlatformCommand::ExportBrowserExtension(
                ExportBrowserExtensionCommand {
                    format: "firefox".into(),
                    destination: "C:/fixture.xpi".into(),
                },
            ))
            .expect("machine");
        let generation = server.connection_generation.load(Ordering::Acquire);
        let accepted = server.router.lock().expect("router").pending[&machine].accepted_at;
        assert_eq!(
            server
                .expire_pending_commands(generation, accepted + Duration::from_secs(11))
                .len(),
            1
        );
    }

    #[test]
    fn delayed_picker_completion_cannot_cross_a_replaced_pipe_generation() {
        use nxtools_protocol::{PickBrowserExtensionExportPathCommand, PickDirectoryCommand};
        let (bridge, server) = platform_bridge(4);
        server.set_connected_for_contract_test(true);
        let old_id = bridge
            .send_command(PlatformCommand::PickDirectory(PickDirectoryCommand {
                current_directory: String::new(),
                title: String::new(),
                source_window_handle: 1,
                source_process_id: 2,
            }))
            .expect("old picker");
        let old_generation = server.connection_generation.load(Ordering::Acquire);
        server.open_connection_for_contract_test();
        assert!(server.complete_command(old_generation, old_id).is_none());
        assert!(!server.command_pending_for_contract_test(old_id));
        let id = bridge
            .send_command(PlatformCommand::PickBrowserExtensionExportPath(
                PickBrowserExtensionExportPathCommand {
                    format: "firefox".into(),
                    source_window_handle: 3,
                    source_process_id: 2,
                },
            ))
            .expect("new picker");
        let current = server.connection_generation.load(Ordering::Acquire);
        assert!(server.complete_command(old_generation, id).is_none());
        assert!(server.complete_command(current, id).is_some());
        assert!(server.complete_command(current, id).is_none());
        let disconnected = bridge
            .send_command(PlatformCommand::PickBrowserExtensionExportPath(
                PickBrowserExtensionExportPathCommand {
                    format: "chrome_store".into(),
                    source_window_handle: 3,
                    source_process_id: 2,
                },
            ))
            .expect("disconnect picker");
        server.set_connected_for_contract_test(false);
        assert!(!server.command_pending_for_contract_test(disconnected));
        assert!(server.complete_command(current, disconnected).is_none());
    }

    #[tokio::test]
    async fn summon_pending_routes_the_actual_result_with_original_request_id() {
        let (bridge, server) = platform_bridge(4);
        server.set_connected_for_contract_test(true);
        let mut request = summon_request("summon actual result");
        let original_request_id = request.id;
        request.correlation_id = Some(uuid::Uuid::new_v4());
        let (reply_sender, reply_receiver) = async_channel::bounded(1);

        server
            .send_summon_command(request.clone(), reply_sender)
            .expect("queue summon command");
        assert_eq!(
            server
                .try_receive_command_for_contract_test()
                .expect("summon queue command")
                .id,
            original_request_id
        );
        let generation = server.connection_generation.load(Ordering::Acquire);
        let pending = server
            .complete_command(generation, original_request_id)
            .expect("summon pending command");
        let actual_result = PlatformCommandResult {
            success: false,
            error: Some("C# native operation failed".into()),
            native_error_code: Some(5),
            data: None,
        };
        pending
            .summon_reply
            .expect("summon reply channel")
            .send(actual_result.clone())
            .await
            .expect("deliver summon result");
        let received = reply_receiver.recv().await.expect("receive summon result");
        let response =
            Envelope::reply_to(&request, Message::PlatformCommandResult(received.clone()));
        assert_eq!(response.correlation_id, Some(original_request_id));
        assert_eq!(received, actual_result);
        assert!(!bridge.is_connected() || server.is_active_generation(generation));
    }

    #[tokio::test]
    async fn summon_queue_rejection_returns_failure_without_pending_state() {
        let (bridge, server) = platform_bridge(1);
        server.set_connected_for_contract_test(true);
        bridge
            .send_command(PlatformCommand::PasteText(PasteTextCommand {
                text: "fill queue".into(),
                paste_after_write: true,
                target_window_handle: 1,
                target_focus_window_handle: 2,
            }))
            .expect("fill primary queue");
        let request = summon_request("queue rejection");
        let (reply_sender, _reply_receiver) = async_channel::bounded(1);
        let error = server
            .send_summon_command(request, reply_sender)
            .expect_err("full summon queue must reject");
        assert!(error.to_string().contains("queue is full"));
    }

    #[tokio::test]
    async fn summon_disconnect_and_timeout_deliver_terminal_failures() {
        let (bridge, server) = platform_bridge(4);
        server.set_connected_for_contract_test(true);
        let request = summon_request("disconnect");
        let (reply_sender, reply_receiver) = async_channel::bounded(1);
        server
            .send_summon_command(request.clone(), reply_sender)
            .expect("queue disconnect summon");
        let generation = server.connection_generation.load(Ordering::Acquire);
        let pending = server
            .disconnect_connection(generation)
            .expect("disconnect active connection");
        server
            .send_disconnected_terminals(generation, pending)
            .await
            .expect("send disconnect terminals");
        let disconnect_failure = reply_receiver.recv().await.expect("disconnect failure");
        assert!(!disconnect_failure.success);
        assert_eq!(
            disconnect_failure.error.as_deref(),
            Some("platform host disconnected")
        );
        assert!(matches!(
            bridge.notification_receiver().recv().await.expect("disconnect notice"),
            PlatformNotification::Disconnected { generation: 1, pending_command_ids }
                if pending_command_ids == vec![request.id]
        ));

        server.set_connected_for_contract_test(true);
        let timeout_request = summon_request("timeout");
        let (timeout_sender, timeout_receiver) = async_channel::bounded(1);
        server
            .send_summon_command(timeout_request, timeout_sender)
            .expect("queue timeout summon");
        let generation = server.connection_generation.load(Ordering::Acquire);
        let pending = server.expire_pending_commands(
            generation,
            Instant::now() + PENDING_COMMAND_TIMEOUT + Duration::from_millis(1),
        );
        server
            .send_timeout_results(pending)
            .await
            .expect("send timeout terminal");
        let timeout_failure = timeout_receiver.recv().await.expect("timeout failure");
        assert!(!timeout_failure.success);
        assert_eq!(
            timeout_failure.error.as_deref(),
            Some("platform command timed out")
        );
    }

    #[tokio::test]
    async fn stale_generation_cannot_complete_a_new_generation_summon_command() {
        let (_bridge, server) = platform_bridge(4);
        server.set_connected_for_contract_test(true);
        let old_request = summon_request("old generation");
        let (reply_sender, _reply_receiver) = async_channel::bounded(1);
        server
            .send_summon_command(old_request.clone(), reply_sender)
            .expect("queue old summon command");
        let old_generation = server.connection_generation.load(Ordering::Acquire);
        server.open_connection_for_contract_test();
        assert!(
            server
                .complete_command(old_generation, old_request.id)
                .is_none()
        );
    }

    #[test]
    fn hello_negotiates_requested_capabilities() {
        let request = Envelope::new(Message::Hello(Hello {
            client_name: "platform_host".into(),
            client_version: "0.1.0".into(),
            capabilities: vec![
                "archive_job_large_count_v1".into(),
                "clipboard_text_capture".into(),
                "unknown".into(),
                "paste_text".into(),
                "clipboard_panel_hotkey".into(),
                "clipboard_configuration".into(),
                "quick_launch_panel_hotkey".into(),
                "quick_launch_keyboard_health_v1".into(),
                "quick_launch_configuration".into(),
                "foreground_context".into(),
                "window_topmost".into(),
                "window_visibility".into(),
                "window_minimized".into(),
                "window_position".into(),
                "shell_context_actions".into(),
                "directory_navigation".into(),
                "file_dialog_search".into(),
            ],
        }));
        let response = dispatch(&request).expect("hello response");

        let Message::HelloAck(ack) = response.message else {
            panic!("expected hello acknowledgement")
        };
        assert_eq!(
            ack.accepted_capabilities,
            vec![
                "archive_job_large_count_v1",
                "clipboard_text_capture",
                "paste_text",
                "clipboard_panel_hotkey",
                "clipboard_configuration",
                "quick_launch_panel_hotkey",
                "quick_launch_keyboard_health_v1",
                "quick_launch_configuration",
                "foreground_context",
                "window_topmost",
                "window_visibility",
                "window_minimized",
                "window_position",
                "shell_context_actions",
                "directory_navigation",
                "file_dialog_search"
            ]
        );
        assert_eq!(response.correlation_id, Some(request.id));
    }
    #[test]
    fn quick_launch_hello_hotkey_snapshot_is_generation_scoped() {
        let (bridge, endpoint) = platform_bridge(4);
        assert_eq!(bridge.quick_launch_panel_hotkey_available(), None);
        endpoint.set_connected_for_contract_test(true);
        let generation = endpoint.connection_generation.load(Ordering::Acquire);
        endpoint.record_quick_launch_panel_hotkey(generation, false);
        assert_eq!(bridge.quick_launch_panel_hotkey_available(), Some(false));
        endpoint.open_connection_for_contract_test();
        assert_eq!(bridge.quick_launch_panel_hotkey_available(), None);
        endpoint.record_quick_launch_panel_hotkey(generation, false);
        assert_eq!(bridge.quick_launch_panel_hotkey_available(), None);
        let new_generation = endpoint.connection_generation.load(Ordering::Acquire);
        endpoint.record_quick_launch_panel_hotkey(new_generation, true);
        assert_eq!(bridge.quick_launch_panel_hotkey_available(), Some(true));
        endpoint.set_connected_for_contract_test(false);
        assert_eq!(bridge.quick_launch_panel_hotkey_available(), None);
    }

    #[test]
    fn ping_preserves_timestamp() {
        let request = Envelope::new(Message::Ping(Ping {
            sent_at_unix_ms: 42,
        }));
        let response = dispatch(&request).expect("pong response");
        assert!(matches!(
            response.message,
            Message::Pong(Pong {
                sent_at_unix_ms: 42
            })
        ));
    }

    #[test]
    fn bridge_rejects_commands_while_platform_host_is_disconnected() {
        let (bridge, _server) = platform_bridge(4);
        let error = bridge
            .send_command(PlatformCommand::PasteText(PasteTextCommand {
                text: "must not become stale".into(),
                paste_after_write: true,
                target_window_handle: 42,
                target_focus_window_handle: 43,
            }))
            .expect_err("disconnected command must be rejected");
        assert!(matches!(error, PlatformBridgeError::Disconnected));
    }

    #[test]
    fn caller_supplied_command_id_is_used_for_result_routing() {
        let (bridge, server) = platform_bridge(4);
        server.set_connected_for_contract_test(true);
        let command_id = uuid::Uuid::new_v4();
        let result = PlatformCommandResult {
            success: true,
            error: None,
            native_error_code: None,
            data: None,
        };

        assert_eq!(
            bridge
                .send_command_with_id(
                    PlatformCommand::PasteText(PasteTextCommand {
                        text: "routed".into(),
                        paste_after_write: true,
                        target_window_handle: 42,
                        target_focus_window_handle: 43,
                    }),
                    command_id,
                )
                .expect("caller-ID command"),
            command_id
        );
        let envelope = server
            .try_receive_command_for_contract_test()
            .expect("queued command");
        assert_eq!(envelope.id, command_id);

        server
            .send_notification_for_contract_test(PlatformNotification::CommandResult {
                correlation_id: command_id,
                result,
            })
            .expect("publish command result");
        assert!(matches!(
            bridge.notification_receiver().try_recv(),
            Ok(PlatformNotification::CommandResult { correlation_id, .. }) if correlation_id == command_id
        ));
    }

    #[test]
    fn caller_supplied_command_id_cannot_replace_an_inflight_command() {
        let (bridge, server) = platform_bridge(4);
        server.set_connected_for_contract_test(true);
        let command_id = uuid::Uuid::new_v4();
        let command = || {
            PlatformCommand::PasteText(PasteTextCommand {
                text: "same id".into(),
                paste_after_write: true,
                target_window_handle: 42,
                target_focus_window_handle: 43,
            })
        };
        bridge
            .send_command_with_id(command(), command_id)
            .expect("first command");

        assert_eq!(
            bridge.send_command_with_id(command(), command_id),
            Err(PlatformBridgeError::DuplicateCommandId)
        );
        assert_eq!(
            server
                .try_receive_command_for_contract_test()
                .expect("original queued command")
                .id,
            command_id
        );
        assert!(server.try_receive_command_for_contract_test().is_err());
    }

    #[test]
    fn bridge_preserves_a_caller_owned_lifecycle_correlation() {
        let (bridge, server) = platform_bridge(4);
        server.set_connected_for_contract_test(true);
        let lifecycle_id = uuid::Uuid::new_v4();

        bridge
            .send_command_correlated(
                PlatformCommand::PasteText(PasteTextCommand {
                    text: "correlated".into(),
                    paste_after_write: false,
                    target_window_handle: 42,
                    target_focus_window_handle: 43,
                }),
                lifecycle_id,
            )
            .expect("connected command");

        let envelope = server
            .try_receive_command_for_contract_test()
            .expect("queued command");
        assert_eq!(envelope.correlation_id, Some(lifecycle_id));
        assert_ne!(envelope.id, lifecycle_id);
    }

    #[test]
    fn old_connection_queue_is_not_consumed_by_a_new_connection() {
        let (bridge, server) = platform_bridge(4);
        let old_receiver = server.open_connection_for_contract_test();
        let old_id = bridge
            .send_command(PlatformCommand::PasteText(PasteTextCommand {
                text: "old".into(),
                paste_after_write: true,
                target_window_handle: 1,
                target_focus_window_handle: 2,
            }))
            .expect("old command");

        let new_receiver = server.open_connection_for_contract_test();
        let new_id = bridge
            .send_command(PlatformCommand::PasteText(PasteTextCommand {
                text: "new".into(),
                paste_after_write: true,
                target_window_handle: 3,
                target_focus_window_handle: 4,
            }))
            .expect("new command");

        assert_eq!(
            old_receiver.try_recv().expect("old queue command").id,
            old_id
        );
        assert_eq!(
            new_receiver.try_recv().expect("new queue command").id,
            new_id
        );
        assert!(old_receiver.try_recv().is_err());
    }

    #[test]
    fn disconnect_emits_pending_command_ids_for_the_active_generation() {
        let (bridge, server) = platform_bridge(4);
        server.set_connected_for_contract_test(true);
        let command_id = bridge
            .send_command(PlatformCommand::PasteText(PasteTextCommand {
                text: "disconnect".into(),
                paste_after_write: true,
                target_window_handle: 5,
                target_focus_window_handle: 6,
            }))
            .expect("pending command");

        server.set_connected_for_contract_test(false);
        let notification = bridge
            .notification_receiver()
            .try_recv()
            .expect("disconnect notification");
        assert!(matches!(
            notification,
            PlatformNotification::Disconnected {
                generation: 1,
                pending_command_ids
            } if pending_command_ids == vec![command_id]
        ));
        assert!(!bridge.is_connected());
    }

    #[test]
    fn full_command_queue_returns_busy_without_leaking_pending_state() {
        let (bridge, server) = platform_bridge(1);
        server.set_connected_for_contract_test(true);
        bridge
            .send_command(PlatformCommand::PasteText(PasteTextCommand {
                text: "fills queue".into(),
                paste_after_write: true,
                target_window_handle: 7,
                target_focus_window_handle: 8,
            }))
            .expect("first command");
        let error = bridge
            .send_command(PlatformCommand::PasteText(PasteTextCommand {
                text: "busy".into(),
                paste_after_write: true,
                target_window_handle: 9,
                target_focus_window_handle: 10,
            }))
            .expect_err("full queue must reject the command");
        assert_eq!(error, PlatformBridgeError::Busy);
    }

    #[tokio::test]
    async fn pending_command_timeout_emits_a_correlated_failure() {
        let (bridge, server) = platform_bridge(4);
        server.set_connected_for_contract_test(true);
        let command_id = bridge
            .send_command(PlatformCommand::PasteText(PasteTextCommand {
                text: "timeout".into(),
                paste_after_write: true,
                target_window_handle: 11,
                target_focus_window_handle: 12,
            }))
            .expect("pending command");

        let generation = server.connection_generation.load(Ordering::Acquire);
        let expired = server.expire_pending_commands(
            generation,
            Instant::now() + PENDING_COMMAND_TIMEOUT + Duration::from_millis(1),
        );
        assert_eq!(
            expired.iter().map(|pending| pending.id).collect::<Vec<_>>(),
            vec![command_id]
        );
        server
            .send_timeout_results(expired)
            .await
            .expect("timeout notification");
        let notification = bridge
            .notification_receiver()
            .recv()
            .await
            .expect("timeout result");
        assert!(matches!(
            notification,
            PlatformNotification::CommandResult {
                correlation_id,
                result
            } if correlation_id == command_id
                && !result.success
                && result.error.as_deref() == Some("platform command timed out")
        ));
    }

    #[test]
    fn archive_picker_waits_for_real_modal_completion() {
        use nxtools_protocol::archive_picker::{ArchivePickerCommand, ArchivePickerPurpose};
        let (bridge, server) = platform_bridge(4);
        server.set_connected_for_contract_test(true);
        let operation = uuid::Uuid::new_v4();
        bridge
            .send_command_with_id_correlated(
                PlatformCommand::ArchivePicker(ArchivePickerCommand {
                    version: 1,
                    operation_id: operation,
                    session_id: uuid::Uuid::new_v4(),
                    generation: 1,
                    owner_window_handle: 1,
                    purpose: ArchivePickerPurpose::Destination,
                    current_directory: None,
                    suggested_name: None,
                }),
                operation,
                operation,
            )
            .expect("dialog admitted");
        let wire = server
            .try_receive_command_for_contract_test()
            .expect("actual picker envelope");
        assert_eq!(wire.id, operation);
        assert_eq!(wire.correlation_id, Some(operation));
        assert!(
            matches!(wire.message, Message::PlatformCommand(PlatformCommand::ArchivePicker(command)) if command.operation_id == operation)
        );
        let generation = server.connection_generation.load(Ordering::Acquire);
        assert!(
            server
                .expire_pending_commands(
                    generation,
                    Instant::now() + PENDING_COMMAND_TIMEOUT + Duration::from_millis(1),
                )
                .is_empty()
        );
        assert!(
            server
                .expire_pending_commands(generation, Instant::now() + Duration::from_secs(601))
                .is_empty()
        );
        assert!(server.complete_command(generation, operation).is_some());
        assert!(server.complete_command(generation, operation).is_none());
    }
    #[test]
    fn timeout_sweep_is_scoped_to_the_connection_generation() {
        let (bridge, server) = platform_bridge(4);
        server.set_connected_for_contract_test(true);
        let old_generation = server.connection_generation.load(Ordering::Acquire);
        bridge
            .send_command(PlatformCommand::PasteText(PasteTextCommand {
                text: "old generation".into(),
                paste_after_write: true,
                target_window_handle: 13,
                target_focus_window_handle: 14,
            }))
            .expect("old generation command");

        server.open_connection_for_contract_test();
        let new_generation = server.connection_generation.load(Ordering::Acquire);
        let new_command_id = bridge
            .send_command(PlatformCommand::PasteText(PasteTextCommand {
                text: "new generation".into(),
                paste_after_write: true,
                target_window_handle: 15,
                target_focus_window_handle: 16,
            }))
            .expect("new generation command");
        let expiration = Instant::now() + PENDING_COMMAND_TIMEOUT + Duration::from_millis(1);
        assert!(
            server
                .expire_pending_commands(old_generation, expiration)
                .is_empty()
        );
        assert_eq!(
            server
                .expire_pending_commands(new_generation, expiration)
                .into_iter()
                .map(|pending| pending.id)
                .collect::<Vec<_>>(),
            vec![new_command_id]
        );
    }

    #[test]
    fn stale_connection_disconnect_does_not_clear_a_newer_connection() {
        let (bridge, server) = platform_bridge(4);
        let _old_receiver = server.open_connection_for_contract_test();
        let old_generation = server.connection_generation.load(Ordering::Acquire);
        let _new_receiver = server.open_connection_for_contract_test();
        assert!(server.disconnect_connection(old_generation).is_none());
        assert!(bridge.is_connected());
    }

    #[tokio::test]
    async fn frame_reader_stops_at_the_limit_before_newline() {
        let oversized = vec![b'x'; MAX_FRAME_BYTES + 1];
        let mut reader = BufReader::with_capacity(4096, oversized.as_slice());
        let error = read_bounded_frame(&mut reader)
            .await
            .expect_err("oversized frame must fail");
        assert!(error.to_string().contains("larger"));
    }

    #[tokio::test]
    async fn frame_reader_preserves_following_frame() {
        let input = b"first\nsecond\n";
        let mut reader = BufReader::with_capacity(4, input.as_slice());
        assert_eq!(
            read_bounded_frame(&mut reader).await.expect("first"),
            Some("first".into())
        );
        assert_eq!(
            read_bounded_frame(&mut reader).await.expect("second"),
            Some("second".into())
        );
    }

    #[tokio::test]
    async fn interrupted_large_frame_read_retains_consumed_prefix() {
        let (mut producer, consumer) = tokio::io::duplex(16384);
        let mut reader = BufReader::with_capacity(4096, consumer);
        let prefix = "x".repeat(8192);
        let mut pending = Vec::with_capacity(4096);
        producer
            .write_all(prefix.as_bytes())
            .await
            .expect("partial frame");
        assert!(
            tokio::time::timeout(
                std::time::Duration::from_millis(20),
                read_bounded_frame_into(&mut reader, &mut pending)
            )
            .await
            .is_err(),
            "other select branch interrupts pending suffix"
        );
        producer.write_all(b"tail\nnext\n").await.expect("suffix");
        assert_eq!(
            read_bounded_frame_into(&mut reader, &mut pending)
                .await
                .expect("resumed frame"),
            Some(prefix + "tail")
        );
        assert_eq!(
            read_bounded_frame_into(&mut reader, &mut pending)
                .await
                .expect("next frame"),
            Some("next".into())
        );
    }
}

#[cfg(test)]
mod admission_refactor_tests {
    use super::*;

    #[test]
    fn both_entry_paths_clean_pending_on_full_and_closed_queue() {
        for summon in [false, true] {
            for closed in [false, true] {
                let (bridge, server) = platform_bridge(1);
                server.set_connected_for_contract_test(true);
                if closed {
                    server
                        .router
                        .lock()
                        .expect("router lock")
                        .active
                        .as_ref()
                        .expect("active connection")
                        .commands
                        .close();
                } else {
                    bridge
                        .send_command(PlatformCommand::ActivateWindow(
                            nxtools_protocol::ActivateWindowCommand { window_handle: 1 },
                        ))
                        .expect("fill queue");
                }
                let envelope =
                    Envelope::new(Message::PlatformCommand(PlatformCommand::ActivateWindow(
                        nxtools_protocol::ActivateWindowCommand { window_handle: 2 },
                    )));
                let rejected_id = envelope.id;
                if summon {
                    let (reply, _) = async_channel::bounded(1);
                    let error = server
                        .send_summon_command(envelope, reply)
                        .expect_err("summon rejection");
                    assert_eq!(
                        error.to_string(),
                        if closed {
                            "platform host command queue is closed"
                        } else {
                            "platform host command queue is full"
                        }
                    );
                } else {
                    assert_eq!(
                        bridge.send_command_envelope(envelope),
                        Err(if closed {
                            PlatformBridgeError::Disconnected
                        } else {
                            PlatformBridgeError::Busy
                        })
                    );
                }
                let router = server.router.lock().expect("pending lock");
                assert!(!router.pending.contains_key(&rejected_id));
                assert_eq!(
                    router.pending.len(),
                    usize::from(!closed),
                    "existing accepted command remains registered"
                );
            }
        }
    }
}

#[cfg(test)]
mod notification_pressure_tests {
    use super::*;
    use std::future::Future;
    #[tokio::test]
    async fn full_ordinary_notifications_cannot_suspend_original_pipe_owner() {
        let (bridge, endpoint) = platform_bridge(1);
        endpoint.set_connected_for_contract_test(true);
        let notification = || PlatformNotification::CommandResult {
            correlation_id: uuid::Uuid::new_v4(),
            result: failed_platform_result("fixture"),
        };
        endpoint
            .send_notification_for_contract_test(notification())
            .expect("full unrelated consumer queue");
        let delivery = endpoint.send_notification(notification());
        tokio::pin!(delivery);
        assert!(
            matches!(
                std::future::poll_fn(|cx| std::task::Poll::Ready(delivery.as_mut().poll(cx))).await,
                std::task::Poll::Ready(Err(_))
            ),
            "ordinary backpressure must fail the real pipe owner rather than suspend cancellation delivery forever"
        );
        assert!(bridge.is_connected(), "caller owns actual pipe retirement");
    }
}

#[cfg(test)]
mod archive_bridge_ownership_tests {
    use super::*;

    #[test]
    fn archive_bridge_shares_router_without_competing_for_desktop_notifications() {
        let (bridge, endpoint) = platform_bridge(4);
        endpoint.set_connected_for_contract_test(true);
        let archive = bridge.archive_client_bridge();
        assert!(Arc::ptr_eq(&archive.router, &bridge.router));
        assert!(Arc::ptr_eq(&archive.archive, &bridge.archive));
        let receiver = archive
            .take_archive_read_receiver()
            .expect("one archive owner");
        assert!(bridge.take_archive_read_receiver().is_err());
        endpoint
            .send_notification_for_contract_test(PlatformNotification::Connected)
            .expect("ordinary notification");
        assert_eq!(
            bridge.notification_receiver().try_recv(),
            Ok(PlatformNotification::Connected)
        );
        assert!(archive.notification_receiver().try_recv().is_err());
        drop(receiver);
    }
}

#[cfg(test)]
mod default_apps_command_tests {
    use super::*;
    #[tokio::test]
    async fn shell_settings_fixed_command_retains_original_router_timeout() {
        let (bridge, server) = platform_bridge(4);
        server.set_connected_for_contract_test(true);
        let id = uuid::Uuid::new_v4();
        bridge
            .send_command_with_id(
                PlatformCommand::OpenDefaultAppsSettings(
                    nxtools_protocol::OpenDefaultAppsSettingsCommand {},
                ),
                id,
            )
            .expect("original command admission");
        let queued = server
            .try_receive_command_for_contract_test()
            .expect("queued");
        assert_eq!(queued.id, id);
        let generation = server.connection_generation.load(Ordering::Acquire);
        let expired = server.expire_pending_commands(
            generation,
            Instant::now() + PENDING_COMMAND_TIMEOUT + Duration::from_millis(1),
        );
        server
            .send_timeout_results(expired)
            .await
            .expect("original timeout");
        let result = bridge
            .notification_receiver()
            .recv()
            .await
            .expect("terminal");
        assert!(
            matches!(result,PlatformNotification::CommandResult{correlation_id,result} if correlation_id==id && !result.success)
        );
    }
}

#[cfg(test)]
mod opt03_tests {
    use super::*;
    #[test]
    fn opt03_hello_retains_native_intake_capabilities() {
        let capabilities = vec![
            "clipboard_file_drop_read_v1".to_owned(),
            "archive_create_destination_v1".to_owned(),
            nxtools_protocol::archive_picker::ARCHIVE_MULTI_FOLDER_PICKER_CAPABILITY.to_owned(),
        ];
        let request = Envelope::new(Message::Hello(nxtools_protocol::Hello {
            client_name: "platform_host".into(),
            client_version: "0.1.0".into(),
            capabilities: capabilities.clone(),
        }));
        let Message::HelloAck(ack) = dispatch(&request).expect("original dispatch").message else {
            panic!("hello ack")
        };
        assert_eq!(ack.accepted_capabilities, capabilities);
    }
}

#[cfg(test)]
mod opt07_tests {
    use super::*;
    #[test]
    fn opt07_folder_capability_retires_with_original_connection() {
        let (bridge, endpoint) = platform_bridge(8);
        let (generation, _, _) = endpoint.open_connection();
        endpoint.record_archive_multi_folder_picker(generation, true);
        assert_eq!(bridge.archive_multi_folder_picker_available(), Some(true));
        let (replacement, _, _) = endpoint.open_connection();
        assert_eq!(bridge.archive_multi_folder_picker_available(), None);
        endpoint.record_archive_multi_folder_picker(generation, true);
        assert_eq!(bridge.archive_multi_folder_picker_available(), None);
        endpoint.record_archive_multi_folder_picker(replacement, false);
        assert_eq!(bridge.archive_multi_folder_picker_available(), Some(false));
        endpoint.set_connected_for_contract_test(false);
        assert_eq!(bridge.archive_multi_folder_picker_available(), None);
    }
}
