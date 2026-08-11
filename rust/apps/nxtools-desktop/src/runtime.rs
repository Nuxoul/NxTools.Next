use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::{Context, Result};
use async_channel::{Receiver as AsyncReceiver, Sender as AsyncSender};
use nxtools_clipboard::ClipboardHistory;
use nxtools_core::{AppConfig, AppPaths, LogGuard, SingleInstanceGuard, init_file_logging};
use nxtools_download::DownloadEngine;
use nxtools_download::DownloadStatus;
use nxtools_download_ipc::{DOWNLOAD_PIPE_NAME, DownloadHostActivity, run_download_host};
use nxtools_finder::{FinderCatalog, default_application_roots};
use nxtools_ipc::{
    INDEXER_PIPE_NAME, connect_local_indexer_pipe, read_bounded_sync_frame, write_envelope_sync,
};
use nxtools_protocol::{Envelope, Hello, IndexerCommand, IndexerEvent, Message};
use tracing::{error, info, warn};

pub(crate) struct IndexerBridge {
    events: AsyncReceiver<IndexerNotification>,
    commands: AsyncSender<Envelope>,
    connected: Arc<AtomicBool>,
}

pub(crate) struct IndexerProcessEndpoint {
    events: AsyncSender<IndexerNotification>,
    commands: AsyncReceiver<Envelope>,
    connected: Arc<AtomicBool>,
}

#[derive(Clone)]
pub(crate) enum IndexerNotification {
    Event(IndexerEvent),
    Disconnected,
}

pub(crate) fn indexer_bridge(capacity: usize) -> (IndexerBridge, IndexerProcessEndpoint) {
    let (event_sender, event_receiver) = async_channel::bounded(capacity);
    let (command_sender, command_receiver) = async_channel::bounded(capacity);
    let connected = Arc::new(AtomicBool::new(false));
    (
        IndexerBridge {
            events: event_receiver,
            commands: command_sender,
            connected: Arc::clone(&connected),
        },
        IndexerProcessEndpoint {
            events: event_sender,
            commands: command_receiver,
            connected,
        },
    )
}

impl IndexerBridge {
    pub(crate) fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Acquire)
    }

    pub(crate) fn event_receiver(&self) -> AsyncReceiver<IndexerNotification> {
        self.events.clone()
    }

    pub(crate) fn send_command(&self, command: IndexerCommand) -> Result<uuid::Uuid> {
        if !self.is_connected() {
            anyhow::bail!("the indexer process is disconnected");
        }
        let envelope = Envelope::new(Message::IndexerCommand(command));
        let command_id = envelope.id;
        self.commands.try_send(envelope).map_err(|error| {
            anyhow::anyhow!("indexer command queue rejected the request: {error}")
        })?;
        Ok(command_id)
    }
}

pub struct DesktopRuntime {
    pub config: AppConfig,
    pub config_path: PathBuf,
    pub log_dir: PathBuf,
    pub platform_bridge: Option<nxtools_ipc::PlatformBridge>,
    pub indexer_bridge: Option<IndexerBridge>,
    pub clipboard_history: Option<ClipboardHistory>,
    pub clipboard_asset_dir: Option<PathBuf>,
    pub finder_catalog: Option<FinderCatalog>,
    pub download_engine: Option<DownloadEngine>,
    pub download_default_directory: PathBuf,
    pub download_database_path: PathBuf,
    pub ffmpeg_install_directory: PathBuf,
    _instance: SingleInstanceGuard,
    _log_guard: LogGuard,
    _platform_host: Option<PlatformHostSupervisor>,
    _indexer: Option<IndexerSupervisor>,
}

impl DesktopRuntime {
    pub fn initialize(instance_wait: Duration) -> Result<Self> {
        let paths = AppPaths::discover()?;
        let instance = SingleInstanceGuard::acquire_with_timeout(&paths, "desktop", instance_wait)?;
        let log_guard = init_file_logging(&paths, "desktop")?;
        let config = AppConfig::load_or_create(&paths)?;
        let config_path = paths.config_file();
        let clipboard_storage_root = if config.clipboard.storage_directory.is_empty() {
            paths.data_dir.clone()
        } else {
            PathBuf::from(&config.clipboard.storage_directory)
        };
        std::fs::create_dir_all(&clipboard_storage_root)
            .context("failed to create clipboard storage directory")?;
        let clipboard_history = ClipboardHistory::open(clipboard_storage_root.join("clipboard.db"))
            .context("failed to open clipboard history")?;
        // Keep the existing directory name so upgrades retain previously captured images.
        let clipboard_asset_dir = clipboard_storage_root.join("clipboard-images");
        std::fs::create_dir_all(&clipboard_asset_dir)
            .context("failed to create clipboard image directory")?;
        let finder_catalog = FinderCatalog::open(
            paths.data_dir.join("quick-launch.db"),
            &default_application_roots(),
        )
        .context("failed to initialize the application launcher catalog")?;
        let download_database_path = paths.data_dir.join("downloads.sqlite3");
        let ffmpeg_install_directory = paths.data_dir.join("tools").join("ffmpeg");
        let download_engine = DownloadEngine::open(download_database_path.clone())
            .context("failed to initialize the download engine")?;
        start_download_host(download_engine.clone())?;
        let download_default_directory = std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .map(|home| home.join("Downloads"))
            .unwrap_or_else(|| paths.data_dir.join("downloads"));
        let (platform_bridge, platform_server) = nxtools_ipc::platform_bridge(256);
        start_platform_server(config.platform_pipe_name.clone(), platform_server)?;
        let platform_host = PlatformHostSupervisor::launch_if_packaged()?;
        let (indexer_bridge, indexer_endpoint) = indexer_bridge(256);
        let indexer = IndexerSupervisor::launch_if_packaged(indexer_endpoint)?;
        info!("desktop runtime initialized");

        Ok(Self {
            config,
            config_path,
            log_dir: paths.log_dir,
            platform_bridge: Some(platform_bridge),
            indexer_bridge: Some(indexer_bridge),
            clipboard_history: Some(clipboard_history),
            clipboard_asset_dir: Some(clipboard_asset_dir),
            finder_catalog: Some(finder_catalog),
            download_engine: Some(download_engine),
            download_default_directory,
            download_database_path,
            ffmpeg_install_directory,
            _instance: instance,
            _log_guard: log_guard,
            _platform_host: platform_host,
            _indexer: indexer,
        })
    }
}

pub fn run_background_download_host(instance_wait: Duration) -> Result<()> {
    let paths = AppPaths::discover()?;
    let _instance = SingleInstanceGuard::acquire_with_timeout(&paths, "desktop", instance_wait)?;
    let _log_guard = init_file_logging(&paths, "download-host")?;
    let engine = DownloadEngine::open(paths.data_dir.join("downloads.sqlite3"))
        .context("failed to initialize the background download engine")?;
    let background_events = engine
        .take_event_receiver()
        .context("failed to attach the background engine event drain")?;
    thread::Builder::new()
        .name("download-background-events".into())
        .spawn(move || while background_events.recv_blocking().is_ok() {})
        .context("failed to start the background engine event drain")?;
    let activity = DownloadHostActivity::default();
    start_download_host_with_activity(engine.clone(), activity.clone())?;
    info!("background download host initialized");

    const IDLE_EXIT_AFTER: Duration = Duration::from_secs(5);
    loop {
        std::thread::sleep(Duration::from_millis(250));
        let has_active_download = engine
            .tasks()
            .context("failed to inspect background download tasks")?
            .iter()
            .any(|task| {
                matches!(
                    task.task.status(),
                    DownloadStatus::Probing
                        | DownloadStatus::Waiting
                        | DownloadStatus::Downloading
                        | DownloadStatus::Finalizing
                        | DownloadStatus::Verifying
                )
            });
        let idle_for = current_unix_ms().saturating_sub(activity.last_activity_unix_ms());
        if !has_active_download
            && activity.connected_clients() == 0
            && idle_for >= IDLE_EXIT_AFTER.as_millis() as u64
        {
            info!("background download host exited after becoming idle");
            return Ok(());
        }
    }
}

fn start_download_host(engine: DownloadEngine) -> Result<()> {
    start_download_host_with_activity(engine, DownloadHostActivity::default())
}

fn start_download_host_with_activity(
    engine: DownloadEngine,
    activity: DownloadHostActivity,
) -> Result<()> {
    thread::Builder::new()
        .name("download-ipc".into())
        .spawn(move || {
            if let Err(error) = run_download_host(DOWNLOAD_PIPE_NAME, engine, activity) {
                error!(%error, "download IPC server stopped");
            }
        })
        .context("failed to start download IPC thread")?;
    Ok(())
}

fn current_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn start_platform_server(
    pipe_name: String,
    platform_server: nxtools_ipc::PlatformServerEndpoint,
) -> Result<()> {
    thread::Builder::new()
        .name("platform-ipc".into())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            match runtime {
                Ok(runtime) => {
                    if let Err(error) = runtime.block_on(nxtools_ipc::run_platform_server(
                        &pipe_name,
                        platform_server,
                    )) {
                        error!(%error, "platform IPC server stopped");
                    }
                }
                Err(error) => error!(%error, "failed to create IPC runtime"),
            }
        })
        .context("failed to start platform IPC thread")?;
    Ok(())
}

struct PlatformHostSupervisor {
    stop: Sender<()>,
    thread: Option<JoinHandle<()>>,
}

struct IndexerSupervisor {
    stop: Sender<()>,
    thread: Option<JoinHandle<()>>,
}

impl IndexerSupervisor {
    fn launch_if_packaged(endpoint: IndexerProcessEndpoint) -> Result<Option<Self>> {
        let executable = std::env::current_exe().context("failed to locate desktop executable")?;
        let Some(directory) = executable.parent() else {
            return Ok(None);
        };
        let indexer_path = directory.join("NxTools.Indexer.exe");
        if !indexer_path.exists() {
            info!(path = %indexer_path.display(), "packaged indexer not present; development launcher owns it");
            return Ok(None);
        }

        let (stop, receiver) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("indexer-supervisor".into())
            .spawn(move || supervise_indexer_service(endpoint, receiver))
            .context("failed to start indexer supervisor")?;
        Ok(Some(Self {
            stop,
            thread: Some(thread),
        }))
    }
}

impl Drop for IndexerSupervisor {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            warn!("indexer supervisor panicked while stopping");
        }
    }
}

impl PlatformHostSupervisor {
    fn launch_if_packaged() -> Result<Option<Self>> {
        let executable = std::env::current_exe().context("failed to locate desktop executable")?;
        let Some(directory) = executable.parent() else {
            return Ok(None);
        };
        let host_path = directory.join("NxTools.PlatformHost.exe");
        if !host_path.exists() {
            info!(path = %host_path.display(), "packaged PlatformHost not present; development launcher owns it");
            return Ok(None);
        }

        let (stop, receiver) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("platform-host-supervisor".into())
            .spawn(move || supervise_platform_host(host_path, receiver))
            .context("failed to start PlatformHost supervisor")?;
        Ok(Some(Self {
            stop,
            thread: Some(thread),
        }))
    }
}

impl Drop for PlatformHostSupervisor {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            warn!("PlatformHost supervisor panicked while stopping");
        }
    }
}

fn supervise_platform_host(host_path: PathBuf, receiver: Receiver<()>) {
    loop {
        let mut child = match spawn_platform_host(&host_path) {
            Ok(child) => child,
            Err(error) => {
                error!(%error, path = %host_path.display(), "failed to launch PlatformHost");
                if receiver.recv_timeout(Duration::from_secs(2)) != Err(RecvTimeoutError::Timeout) {
                    return;
                }
                continue;
            }
        };
        info!(pid = child.id(), "PlatformHost launched");

        loop {
            match receiver.recv_timeout(Duration::from_secs(1)) {
                Ok(()) | Err(RecvTimeoutError::Disconnected) => {
                    stop_child(&mut child);
                    return;
                }
                Err(RecvTimeoutError::Timeout) => match child.try_wait() {
                    Ok(Some(status)) => {
                        warn!(%status, "PlatformHost exited; restarting");
                        break;
                    }
                    Ok(None) => {}
                    Err(error) => {
                        error!(%error, "failed to inspect PlatformHost");
                        stop_child(&mut child);
                        break;
                    }
                },
            }
        }

        if receiver.recv_timeout(Duration::from_secs(1)) != Err(RecvTimeoutError::Timeout) {
            return;
        }
    }
}

fn supervise_indexer_service(endpoint: IndexerProcessEndpoint, receiver: Receiver<()>) {
    supervise_indexer_service_at(INDEXER_PIPE_NAME, endpoint, receiver);
}

fn supervise_indexer_service_at(
    pipe_name: &str,
    endpoint: IndexerProcessEndpoint,
    receiver: Receiver<()>,
) {
    loop {
        let mut connection = match connect_local_indexer_pipe(pipe_name, Duration::from_millis(750))
        {
            Ok(connection) => connection,
            Err(error) => {
                endpoint.connected.store(false, Ordering::Release);
                if receiver.recv_timeout(Duration::from_secs(1)) != Err(RecvTimeoutError::Timeout) {
                    return;
                }
                warn!(%error, "indexer service is not available; retrying local pipe");
                continue;
            }
        };
        if let Err(error) = write_indexer_handshake(&mut connection.writer) {
            error!(%error, "failed to begin indexer handshake");
            if receiver.recv_timeout(Duration::from_secs(1)) != Err(RecvTimeoutError::Timeout) {
                return;
            }
            continue;
        }

        let events = endpoint.events.clone();
        let connected = Arc::clone(&endpoint.connected);
        let reader = thread::Builder::new()
            .name("indexer-events".into())
            .spawn(move || read_indexer_events(connection.reader, events, connected))
            .ok();
        info!(pipe = pipe_name, "connected to indexer Windows service");

        let reconnect = 'session: loop {
            match receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(()) | Err(RecvTimeoutError::Disconnected) => {
                    break false;
                }
                Err(RecvTimeoutError::Timeout) => {}
            }
            while let Ok(command) = endpoint.commands.try_recv() {
                if let Err(error) = write_indexer_envelope(&mut connection.writer, &command) {
                    warn!(%error, "failed to send command to indexer");
                    break 'session true;
                }
            }
            if reader.as_ref().is_some_and(JoinHandle::is_finished) {
                warn!("indexer service stream ended; reconnecting");
                break true;
            }
        };

        endpoint.connected.store(false, Ordering::Release);
        while endpoint.commands.try_recv().is_ok() {}
        if !reconnect {
            // The service intentionally outlives the desktop. Do not join a synchronous reader
            // that may still be blocked on that persistent pipe during application shutdown.
            drop(reader);
            return;
        }
        if let Some(reader) = reader {
            let _ = reader.join();
        }
        if receiver.recv_timeout(Duration::from_secs(1)) != Err(RecvTimeoutError::Timeout) {
            return;
        }
    }
}

fn write_indexer_handshake(writer: &mut impl Write) -> Result<()> {
    write_indexer_envelope(
        writer,
        &Envelope::new(Message::Hello(Hello {
            client_name: "nxtools_desktop".into(),
            client_version: env!("CARGO_PKG_VERSION").into(),
            capabilities: vec!["index_status".into(), "file_search".into()],
        })),
    )
}

fn write_indexer_envelope(writer: &mut impl std::io::Write, envelope: &Envelope) -> Result<()> {
    write_envelope_sync(writer, envelope)
}

fn read_indexer_events(
    reader: impl BufRead,
    events: AsyncSender<IndexerNotification>,
    connected: Arc<AtomicBool>,
) {
    let mut reader = reader;
    loop {
        let frame = match read_bounded_sync_frame(&mut reader) {
            Ok(Some(frame)) => frame,
            Ok(None) => break,
            Err(error) => {
                warn!(%error, "invalid indexer output");
                break;
            }
        };
        let envelope = match Envelope::decode_line(&frame) {
            Ok(envelope) => envelope,
            Err(error) => {
                warn!(%error, "indexer emitted an invalid protocol frame");
                break;
            }
        };
        match envelope.message {
            Message::HelloAck(_) => connected.store(true, Ordering::Release),
            Message::IndexerEvent(event) => {
                if events
                    .send_blocking(IndexerNotification::Event(event))
                    .is_err()
                {
                    break;
                }
            }
            _ => {}
        }
    }
    connected.store(false, Ordering::Release);
    let _ = events.send_blocking(IndexerNotification::Disconnected);
}

fn stop_child(child: &mut Child) {
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill();
    }
    let _ = child.wait();
}

#[cfg(windows)]
fn spawn_platform_host(path: &Path) -> std::io::Result<Child> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    Command::new(path)
        .args(["--parent-pid", &std::process::id().to_string()])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexer_commands_are_bounded_and_rejected_while_disconnected() {
        let (bridge, endpoint) = indexer_bridge(1);
        assert!(bridge.send_command(IndexerCommand::RefreshVolumes).is_err());

        endpoint.connected.store(true, Ordering::Release);
        let command_id = bridge
            .send_command(IndexerCommand::RefreshVolumes)
            .expect("connected indexer accepts one command");
        assert!(bridge.send_command(IndexerCommand::RefreshVolumes).is_err());
        let queued = endpoint
            .commands
            .try_recv()
            .expect("indexer command queued");
        assert_eq!(queued.id, command_id);
        assert!(matches!(
            queued.message,
            Message::IndexerCommand(IndexerCommand::RefreshVolumes)
        ));
    }

    #[cfg(windows)]
    #[test]
    fn desktop_shutdown_does_not_wait_for_the_persistent_service_reader() {
        let pipe_name = format!(r"\\.\pipe\nxtools-desktop-stop-{}", uuid::Uuid::new_v4());
        let server_name = pipe_name.clone();
        let (accepted, accepted_receiver) = mpsc::channel();
        let (close_server, close_receiver) = mpsc::channel();
        let server = thread::spawn(move || {
            let mut connection = nxtools_ipc::accept_local_indexer_pipe(&server_name)
                .expect("accept desktop connector");
            let frame = read_bounded_sync_frame(&mut connection.reader)
                .expect("read handshake")
                .expect("desktop sent handshake");
            let hello = Envelope::decode_line(&frame).expect("valid handshake");
            write_envelope_sync(
                &mut connection.writer,
                &Envelope::reply_to(
                    &hello,
                    Message::HelloAck(nxtools_protocol::HelloAck {
                        server_name: "test_indexer".into(),
                        server_version: "0.1.0".into(),
                        accepted_capabilities: vec!["index_status".into()],
                    }),
                ),
            )
            .expect("write handshake acknowledgement");
            accepted.send(()).expect("signal accepted connection");
            close_receiver.recv().expect("close test server");
        });

        let (_bridge, endpoint) = indexer_bridge(8);
        let (stop, stop_receiver) = mpsc::channel();
        let (finished, finished_receiver) = mpsc::channel();
        let supervisor_pipe = pipe_name.clone();
        thread::spawn(move || {
            supervise_indexer_service_at(&supervisor_pipe, endpoint, stop_receiver);
            let _ = finished.send(());
        });
        accepted_receiver
            .recv_timeout(Duration::from_secs(2))
            .expect("desktop connected to test service");

        stop.send(()).expect("request desktop shutdown");
        finished_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("supervisor stopped without joining its persistent pipe reader");
        close_server.send(()).expect("close test service");
        server.join().expect("join test service");
    }
}

#[cfg(not(windows))]
fn spawn_platform_host(path: &Path) -> std::io::Result<Child> {
    Command::new(path)
        .args(["--parent-pid", &std::process::id().to_string()])
        .spawn()
}
