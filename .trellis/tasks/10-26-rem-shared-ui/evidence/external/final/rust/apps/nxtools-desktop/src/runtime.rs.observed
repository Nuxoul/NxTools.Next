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

use crate::clipboard_worker::ClipboardCaptureWorkerHandle;
use anyhow::{Context, Result};
use async_channel::{Receiver as AsyncReceiver, Sender as AsyncSender};
use nxtools_clipboard::ClipboardRetentionPolicy;
use nxtools_core::{AppConfig, AppPaths, LogGuard, SingleInstanceGuard, init_file_logging};
use nxtools_download::DownloadStatus;
use nxtools_download::{DownloadEngine, DownloadHttpPolicy};
use nxtools_download_ipc::{DOWNLOAD_PIPE_NAME, DownloadHostActivity, run_download_host};
use nxtools_finder::{FinderCatalog, default_application_roots};
use nxtools_ipc::{
    INDEXER_PIPE_NAME, connect_local_indexer_pipe, connect_local_pipe, read_bounded_sync_frame,
    write_envelope_sync,
};
use nxtools_network::NetworkPolicy;
use nxtools_protocol::{
    ActivateWindowCommand, DEFAULT_PLATFORM_PIPE_NAME, Envelope, Hello, IndexerCommand,
    IndexerEvent, Message, PlatformCommand, SetWindowVisibilityCommand,
};
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

#[cfg(test)]
impl IndexerProcessEndpoint {
    pub(crate) fn connect_for_test(&self) {
        self.connected.store(true, Ordering::Release);
    }

    pub(crate) fn try_recv_command_for_test(&self) -> Option<Envelope> {
        self.commands.try_recv().ok()
    }
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

#[derive(Debug)]
pub(crate) struct DesktopInstanceConflict;
impl std::fmt::Display for DesktopInstanceConflict {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("another desktop instance is already running")
    }
}
impl std::error::Error for DesktopInstanceConflict {}

fn acquire_desktop_instance(paths: &AppPaths, wait: Duration) -> Result<SingleInstanceGuard> {
    SingleInstanceGuard::acquire_with_timeout(paths, "desktop", wait).map_err(|error| {
        // Translate the core lock boundary once; callers never infer races from contextual startup errors.
        if error.to_string() == "another desktop instance is already running" {
            anyhow::Error::new(DesktopInstanceConflict)
        } else {
            error
        }
    })
}

pub struct DesktopRuntime {
    pub config: AppConfig,
    pub config_path: PathBuf,
    pub log_dir: PathBuf,
    pub platform_bridge: Option<nxtools_ipc::PlatformBridge>,
    pub indexer_bridge: Option<IndexerBridge>,
    pub archive_launches: Option<nxtools_ipc::desktop_archive_intent::ArchiveIntentReceiver>,
    pub clipboard_worker: Option<ClipboardCaptureWorkerHandle>,
    pub finder_catalog: Option<FinderCatalog>,
    pub download_engine: Option<DownloadEngine>,
    pub(crate) extension_service: crate::extensions::service::ExtensionService,
    pub(crate) extension_ui_calls:
        Option<AsyncReceiver<crate::extensions::feature_adapters::UiHostCall>>,
    pub network_policy: NetworkPolicy,
    pub download_default_directory: PathBuf,
    pub download_database_path: PathBuf,
    pub ffmpeg_install_directory: PathBuf,
    _instance: SingleInstanceGuard,
    _log_guard: LogGuard,
    _platform_host: Option<PlatformHostSupervisor>,
    _indexer: Option<IndexerSupervisor>,
}
impl DesktopRuntime {
    pub fn initialize(
        instance_wait: Duration,
        initial_archive: Option<nxtools_protocol::desktop_archive_intent::DesktopArchiveIntent>,
    ) -> Result<Self> {
        let paths = AppPaths::discover()?;
        let instance = acquire_desktop_instance(&paths, instance_wait)?;
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
        // Keep the existing directory name so upgrades retain previously captured images.
        let clipboard_asset_dir = clipboard_storage_root.join("clipboard-images");
        std::fs::create_dir_all(&clipboard_asset_dir)
            .context("failed to create clipboard image directory")?;
        let clipboard_worker = ClipboardCaptureWorkerHandle::spawn(
            clipboard_storage_root.join("clipboard.db"),
            &clipboard_asset_dir,
            ClipboardRetentionPolicy {
                max_items: config.clipboard.max_items,
                retention_days: config.clipboard.retention_days,
            },
            nxtools_ipc::unix_time_ms(),
        )
        .context("failed to start clipboard capture worker")?;
        let finder_catalog = FinderCatalog::open(
            paths.data_dir.join("quick-launch.db"),
            &default_application_roots(),
        )
        .context("failed to initialize the application launcher catalog")?;
        let download_database_path = paths.data_dir.join("downloads.sqlite3");
        let ffmpeg_install_directory = paths.data_dir.join("tools").join("ffmpeg");
        let (download_engine, network_policy) =
            open_download_engine_from_config(download_database_path.clone(), &config)
                .context("failed to initialize the download engine")?;
        let (extension_service, extension_ui_calls) =
            crate::extensions::service::ExtensionService::new_with_storage(
                &download_engine,
                config.extensions.clone(),
                Some(clipboard_worker.clone()),
                true,
                config_path.clone(),
                paths.data_dir.join("extensions-data"),
            )?;
        start_download_host(download_engine.clone())?;
        let download_default_directory = std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .map(|home| home.join("Downloads"))
            .unwrap_or_else(|| paths.data_dir.join("downloads"));
        let (platform_bridge, platform_server) = nxtools_ipc::platform_bridge(256);
        let (archive_intents, archive_launches) =
            nxtools_ipc::desktop_archive_intent::archive_intent_channel();
        if let Some(intent) = initial_archive {
            let receipt = archive_intents.submit(intent);
            if let Some(failure) = receipt.failure {
                return Err(failure.into());
            }
        }
        let platform_server = platform_server.with_archive_intents(archive_intents);
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
            archive_launches: Some(archive_launches),
            clipboard_worker: Some(clipboard_worker),
            finder_catalog: Some(finder_catalog),
            download_engine: Some(download_engine),
            extension_service,
            extension_ui_calls,
            network_policy,
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
    let _instance = acquire_desktop_instance(&paths, instance_wait)?;
    let _log_guard = init_file_logging(&paths, "download-host")?;
    let config = AppConfig::load_or_create(&paths)?;
    let (engine, _network_policy) =
        open_download_engine_from_config(paths.data_dir.join("downloads.sqlite3"), &config)
            .context("failed to initialize the background download engine")?;
    let (_extensions, _) = crate::extensions::service::ExtensionService::new_with_storage(
        &engine,
        config.extensions.clone(),
        None,
        false,
        paths.config_file(),
        paths.data_dir.join("extensions-data"),
    )?;
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
            activity.begin_draining();
            info!("background download host entered draining before idle exit");
            activity.mark_stopped();
            info!("background download host exited after becoming idle");
            return Ok(());
        }
    }
}

fn open_download_engine_from_config(
    database_path: impl AsRef<Path>,
    config: &AppConfig,
) -> Result<(DownloadEngine, NetworkPolicy)> {
    let network_policy = NetworkPolicy::new(config.network.clone())
        .context("failed to initialize the product network policy")?;
    let request_policy = DownloadHttpPolicy::from_download_settings(&config.download)
        .context("failed to initialize the download HTTP policy")?;
    let engine =
        DownloadEngine::open_with_policies(database_path, network_policy.clone(), request_policy)?;
    engine
        .update_download_organization_policy(&config.download)
        .context("failed to initialize the download organization policy")?;
    Ok((engine, network_policy))
}

fn start_download_host(engine: DownloadEngine) -> Result<()> {
    start_download_host_with_activity(engine, DownloadHostActivity::default())
}

fn start_download_host_with_activity(
    engine: DownloadEngine,
    activity: DownloadHostActivity,
) -> Result<()> {
    let activity_for_thread = activity.clone();
    thread::Builder::new()
        .name("download-ipc".into())
        .spawn(move || {
            if let Err(error) =
                run_download_host(DOWNLOAD_PIPE_NAME, engine, activity_for_thread.clone())
            {
                activity_for_thread.mark_failed("ipc_server_stopped");
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

pub(crate) fn desktop_instance_is_running() -> Result<bool> {
    let paths = AppPaths::discover()?;
    Ok(SingleInstanceGuard::is_locked(&paths, "desktop"))
}

impl PlatformHostSupervisor {
    fn launch_if_packaged() -> Result<Option<Self>> {
        let executable = std::env::current_exe().context("failed to locate desktop executable")?;
        let Some(host_path) = platform_host_path(&executable) else {
            info!(
                desktop = %executable.display(),
                "PlatformHost executable was not found beside the desktop or in the development output"
            );
            return Ok(None);
        };

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

fn platform_host_path(executable: &Path) -> Option<PathBuf> {
    let directory = executable.parent()?;
    let packaged_path = directory.join("NxTools.PlatformHost.exe");
    if packaged_path.is_file() {
        return Some(packaged_path);
    }

    // Direct Debug launches run from target\debug while the .NET adapter is
    // built under csharp\NxTools.PlatformHost\bin. Let the desktop supervise
    // that development binary so the executable is self-starting for users.
    let repository_root = directory.parent()?.parent()?;
    let development_path = repository_root
        .join("csharp")
        .join("NxTools.PlatformHost")
        .join("bin")
        .join("Debug")
        .join("net8.0-windows")
        .join("win-x64")
        .join("NxTools.PlatformHost.exe");
    development_path.is_file().then_some(development_path)
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
                        warn!(pid = child.id(), %status, "PlatformHost exited; restarting");
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

const DESKTOP_EXECUTABLE_ENVIRONMENT_VARIABLE: &str = "NXTOOLS_DESKTOP_EXECUTABLE";

#[cfg(windows)]
fn spawn_platform_host(path: &Path) -> std::io::Result<Child> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let desktop_executable = std::env::current_exe()?;
    Command::new(path)
        .env(DESKTOP_EXECUTABLE_ENVIRONMENT_VARIABLE, desktop_executable)
        .args(["--parent-pid", &std::process::id().to_string()])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
}

/// No main-HWND hint is needed: a cold owner's archive queue is ready before GPUI registration.
pub(crate) fn forward_running_archive(
    intent: &nxtools_protocol::desktop_archive_intent::DesktopArchiveIntent,
) -> Result<bool> {
    let paths = AppPaths::discover()?;
    if !SingleInstanceGuard::is_locked(&paths, "desktop") {
        return Ok(false);
    }
    let pipe_name = if paths.config_file().is_file() {
        AppConfig::load_from(&paths.config_file())
            .context("archive launch configuration unavailable")?
            .platform_pipe_name
    } else {
        DEFAULT_PLATFORM_PIPE_NAME.into()
    };
    nxtools_ipc::desktop_archive_intent::forward_archive_intent(&pipe_name, intent)?;
    Ok(true)
}

/// Summons the main window of an already-running desktop instance. A second
/// launch uses this instead of failing on the single-instance lock: it reads
/// the recorded main-window handle, asks the running instance's platform
/// host to reveal and focus that window, and reports whether a summon was
/// attempted.
pub(crate) fn summon_running_desktop() -> Result<bool> {
    let paths = AppPaths::discover()?;
    if !SingleInstanceGuard::is_locked(&paths, "desktop") {
        // A stale desktop.hints file is expected after a crash or forced
        // termination. Only a live lock owner can be summoned.
        return Ok(false);
    }
    let main_window_handle = SingleInstanceGuard::read_main_window_handle(&paths, "desktop");
    if main_window_handle == 0 {
        return Ok(false);
    }
    let pipe_name = format!(r"\\.\pipe\{DEFAULT_PLATFORM_PIPE_NAME}");
    let mut connection = connect_local_pipe(&pipe_name, Duration::from_millis(800))?;

    write_envelope_sync(
        &mut connection.writer,
        &Envelope::new(Message::Hello(Hello {
            client_name: "nxtools-desktop-summon".into(),
            client_version: env!("CARGO_PKG_VERSION").into(),
            capabilities: vec!["desktop_shell".into()],
        })),
    )?;
    let Some(frame) = read_bounded_sync_frame(&mut connection.reader)? else {
        anyhow::bail!("summon handshake returned no acknowledgement");
    };
    let acknowledgement =
        Envelope::decode_line(&frame).context("summon handshake returned an invalid frame")?;
    if !matches!(acknowledgement.message, Message::HelloAck { .. }) {
        anyhow::bail!("summon handshake did not return a hello ack");
    }
    let visibility = Envelope::new(Message::PlatformCommand(
        PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
            window_handle: main_window_handle,
            visible: true,
        }),
    ));
    let activation = Envelope::new(Message::PlatformCommand(PlatformCommand::ActivateWindow(
        ActivateWindowCommand {
            window_handle: main_window_handle,
        },
    )));
    for request in [&visibility, &activation] {
        write_envelope_sync(&mut connection.writer, request)?;
    }
    for request in [&visibility, &activation] {
        let Some(frame) = read_bounded_sync_frame(&mut connection.reader)? else {
            anyhow::bail!("summon command returned no result");
        };
        let response =
            Envelope::decode_line(&frame).context("summon command returned an invalid frame")?;
        let correlation_id = response.correlation_id;
        let Message::PlatformCommandResult(result) = response.message else {
            anyhow::bail!("summon command returned an unexpected message");
        };
        if correlation_id != Some(request.id) || !result.success {
            anyhow::bail!(
                "summon command was rejected: {}",
                result.error.as_deref().unwrap_or("unknown platform error")
            );
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_host_path_prefers_the_packaged_sibling() {
        let root = tempfile::tempdir().expect("create runtime path fixture");
        let bin = root.path().join("artifacts").join("win-x64");
        std::fs::create_dir_all(&bin).expect("create package directory");
        let executable = bin.join("NxTools.exe");
        let packaged_host = bin.join("NxTools.PlatformHost.exe");
        std::fs::File::create(&executable).expect("create desktop fixture");
        std::fs::File::create(&packaged_host).expect("create host fixture");

        assert_eq!(platform_host_path(&executable), Some(packaged_host));
    }

    #[test]
    fn platform_host_path_falls_back_to_the_repository_debug_output() {
        let root = tempfile::tempdir().expect("create runtime path fixture");
        let executable = root
            .path()
            .join("target")
            .join("debug")
            .join("nxtools-desktop.exe");
        let development_host = root
            .path()
            .join("csharp")
            .join("NxTools.PlatformHost")
            .join("bin")
            .join("Debug")
            .join("net8.0-windows")
            .join("win-x64")
            .join("NxTools.PlatformHost.exe");
        std::fs::create_dir_all(executable.parent().expect("desktop parent"))
            .expect("create target directory");
        std::fs::create_dir_all(development_host.parent().expect("host parent"))
            .expect("create host directory");
        std::fs::File::create(&executable).expect("create desktop fixture");
        std::fs::File::create(&development_host).expect("create host fixture");

        assert_eq!(platform_host_path(&executable), Some(development_host));
    }

    #[test]
    fn foreground_and_background_engine_factory_applies_download_http_settings_before_intake() {
        let directory = tempfile::tempdir().expect("create runtime policy fixture");
        let mut config = AppConfig::default();
        config.download.default_request_headers = "X-NxTools-Runtime: configured".into();
        config.download.default_cookie = "runtime=1".into();
        config.download.accept_invalid_certificates = true;
        let (engine, _) =
            open_download_engine_from_config(directory.path().join("downloads.sqlite3"), &config)
                .expect("open configured download engine");
        let task_id = engine
            .enqueue(
                nxtools_download::DownloadRequest {
                    source_url: "https://example.test/file.bin".into(),
                    save_directory: directory.path().join("artifacts"),
                    save_directory_explicit: false,
                    file_name: Some("file.bin".into()),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
                    certificate_policy: Default::default(),
                    connection_limit: Some(1),
                    speed_limit_bytes_per_second: None,
                    expected_total_bytes: None,
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
                    media: None,
                    category_id: None,
                    category_name: None,
                    duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
                },
                nxtools_download::RequestOrigin::Internal,
            )
            .expect("accept configured request");
        let task = engine
            .task(task_id)
            .expect("read accepted task")
            .expect("task exists");

        assert_eq!(task.task.request().cookie.as_deref(), Some("runtime=1"));
        assert_eq!(
            task.task.request().certificate_policy,
            nxtools_download::DownloadCertificatePolicy::AcceptInvalid
        );
        assert!(task.task.request().headers.iter().any(|header| {
            header.name.eq_ignore_ascii_case("x-nxtools-runtime") && header.value == "configured"
        }));
    }

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
    let desktop_executable = std::env::current_exe()?;
    Command::new(path)
        .env(DESKTOP_EXECUTABLE_ENVIRONMENT_VARIABLE, desktop_executable)
        .args(["--parent-pid", &std::process::id().to_string()])
        .spawn()
}
