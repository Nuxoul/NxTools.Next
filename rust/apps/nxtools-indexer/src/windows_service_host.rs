use std::ffi::OsString;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use nxtools_indexer::{IndexerService, ServiceCommandHandler, serve_session_with_command_handler};
use nxtools_ipc::{INDEXER_PIPE_NAME, accept_local_indexer_pipe, connect_local_indexer_pipe};
use tracing::{error, info, warn};
use windows_service::define_windows_service;
use windows_service::service::{
    ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::{Result, service_dispatcher};

use crate::elevated_helper_broker;

pub const SERVICE_NAME: &str = "NxToolsIndexerV1";
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

define_windows_service!(ffi_service_main, service_main);

pub fn run_dispatcher() -> Result<()> {
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
}

fn service_main(_arguments: Vec<OsString>) {
    if let Err(error) = run_service() {
        error!(%error, "indexer Windows service failed");
    }
}

fn run_service() -> Result<()> {
    let stopping = Arc::new(AtomicBool::new(false));
    let (stop_sender, stop_receiver) = mpsc::channel();
    let handler_stopping = Arc::clone(&stopping);
    let status_handle = service_control_handler::register(SERVICE_NAME, move |control| {
        match control {
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            ServiceControl::Stop | ServiceControl::Shutdown => {
                handler_stopping.store(true, Ordering::Release);
                let _ = stop_sender.send(());
                // A pending accept is the service loop's cancellation point.
                thread::spawn(|| {
                    let _ =
                        connect_local_indexer_pipe(INDEXER_PIPE_NAME, Duration::from_millis(500));
                });
                ServiceControlHandlerResult::NoError
            }
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    })?;

    status_handle.set_service_status(service_status(
        ServiceState::Running,
        ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
    ))?;
    info!(pipe = INDEXER_PIPE_NAME, "indexer Windows service ready");

    let initial_service = IndexerService::discover();
    let published_status = Arc::new(Mutex::new(initial_service.status().clone()));
    let service = Arc::new(Mutex::new(initial_service));
    while !stopping.load(Ordering::Acquire) {
        let connection = match accept_local_indexer_pipe(INDEXER_PIPE_NAME) {
            Ok(connection) => connection,
            Err(error) => {
                warn!(%error, "indexer pipe accept failed");
                if stop_receiver
                    .recv_timeout(Duration::from_millis(250))
                    .is_ok()
                {
                    break;
                }
                continue;
            }
        };
        if stopping.load(Ordering::Acquire) {
            break;
        }
        let Some(client_process_id) = connection.client_process_id else {
            warn!("rejected indexer pipe client without a process identity");
            continue;
        };
        let Some(client_session_id) = connection.client_session_id else {
            warn!(
                client_process_id,
                "rejected indexer pipe client without a session identity"
            );
            continue;
        };
        let service_path = match std::env::current_exe() {
            Ok(path) => path,
            Err(error) => {
                warn!(%error, "could not resolve the packaged indexer service path");
                continue;
            }
        };
        let service = Arc::clone(&service);
        let published_status = Arc::clone(&published_status);
        let command_service_path = service_path.clone();
        let command_handler: Arc<ServiceCommandHandler> = Arc::new(move |request| {
            elevated_helper_broker::handle_service_command(
                request,
                client_process_id,
                client_session_id,
                &command_service_path,
            )
        });
        thread::Builder::new()
            .name("indexer-client".into())
            .spawn(move || {
                // A successful handshake must mean that this connection really owns a search
                // session. A transient `try_lock` failure previously produced a false HelloAck
                // and then closed the pipe, so the desktop could queue a query that was lost.
                let mut service = service.lock().unwrap_or_else(|error| {
                    error!(%error, "indexer service state lock was poisoned");
                    error.into_inner()
                });
                if let Err(error) = serve_session_with_command_handler(
                    connection.reader,
                    connection.writer,
                    &mut service,
                    command_handler,
                ) {
                    warn!(%error, "indexer client session ended with an error");
                }
                *published_status
                    .lock()
                    .unwrap_or_else(|error| error.into_inner()) = service.status().clone();
            })
            .map_err(windows_service::Error::Winapi)?;
    }

    status_handle.set_service_status(service_status(
        ServiceState::Stopped,
        ServiceControlAccept::empty(),
    ))?;
    Ok(())
}

fn service_status(state: ServiceState, controls: ServiceControlAccept) -> ServiceStatus {
    ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: state,
        controls_accepted: controls,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    }
}
