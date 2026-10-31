use std::ffi::OsString;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use nxtools_indexer::{IndexerService, ServiceCommandHandler, serve_admitted_session};
use nxtools_ipc::{
    INDEXER_PIPE_NAME, accept_local_indexer_pipe, connect_local_indexer_pipe,
    read_bounded_sync_frame,
};
use tracing::{error, info, warn};
use windows_service::define_windows_service;
use windows_service::service::{
    ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::{Result, service_dispatcher};

use crate::elevated_helper_broker;

pub const SERVICE_NAME: &str = "NxToolsIndexerV1";
const MAX_PENDING_SESSIONS: usize = 4;

fn reserve_session_slot(pending: &AtomicUsize) -> bool {
    pending
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
            (count < MAX_PENDING_SESSIONS).then_some(count + 1)
        })
        .is_ok()
}

struct SessionSlot(Arc<AtomicUsize>);

impl Drop for SessionSlot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Release);
    }
}
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;
// Session 0 is reserved for services; desktop clients must have a real interactive session.
fn admitted_desktop_client(
    process_id: Option<u32>,
    session_id: Option<u32>,
    authorized: impl FnOnce(u32) -> bool,
) -> Option<(u32, u32)> {
    match (process_id, session_id) {
        (Some(process_id), Some(session_id)) if process_id != 0 && session_id != 0 => {
            authorized(process_id).then_some((process_id, session_id))
        }
        _ => None,
    }
}

#[cfg(test)]
mod admission_tests {
    use super::*;

    #[test]
    fn unauthorized_or_unidentified_pipe_client_never_enters_the_service_session() {
        use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

        let checks = AtomicUsize::new(0);
        let verify = |_: u32| {
            checks.fetch_add(1, AtomicOrdering::Relaxed);
            false
        };
        assert_eq!(admitted_desktop_client(Some(42), Some(1), verify), None);
        assert_eq!(admitted_desktop_client(None, Some(1), verify), None);
        assert_eq!(admitted_desktop_client(Some(42), None, verify), None);
        assert_eq!(admitted_desktop_client(Some(42), Some(0), verify), None);
        assert_eq!(checks.load(AtomicOrdering::Relaxed), 1);
        assert_eq!(
            admitted_desktop_client(Some(42), Some(1), |_| true),
            Some((42, 1))
        );
    }
    #[test]
    fn pending_session_slots_are_bounded_and_released() {
        let pending = Arc::new(AtomicUsize::new(0));
        let leases = (0..MAX_PENDING_SESSIONS)
            .map(|_| {
                assert!(reserve_session_slot(&pending));
                SessionSlot(Arc::clone(&pending))
            })
            .collect::<Vec<_>>();
        assert!(!reserve_session_slot(&pending));
        drop(leases);
        assert_eq!(pending.load(Ordering::Acquire), 0);
        assert!(reserve_session_slot(&pending));
    }
}

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
    let pending_sessions = Arc::new(AtomicUsize::new(0));
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
        let Some((client_process_id, client_session_id)) = admitted_desktop_client(
            connection.client_process_id,
            connection.client_session_id,
            elevated_helper_broker::authorized_packaged_desktop,
        ) else {
            warn!("rejected indexer pipe client without an approved desktop identity/session");
            continue;
        };
        let service_path = match std::env::current_exe() {
            Ok(path) => path,
            Err(error) => {
                warn!(%error, "could not resolve the packaged indexer service path");
                continue;
            }
        };
        if !reserve_session_slot(&pending_sessions) {
            warn!("rejected indexer pipe client: pending session admission full");
            continue;
        }
        let session_slot = SessionSlot(Arc::clone(&pending_sessions));
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
                let _session_slot = session_slot;
                // An unresponsive Hello must not lease the shared indexer state.
                let mut reader = connection.reader;
                reader
                    .get_mut()
                    .set_initial_frame_timeout(Duration::from_secs(3));
                let handshake = match read_bounded_sync_frame(&mut reader)
                    .and_then(|frame| frame.ok_or_else(|| anyhow::anyhow!("missing indexer Hello")))
                    .and_then(|frame| {
                        nxtools_protocol::Envelope::decode_line(&frame).map_err(Into::into)
                    }) {
                    Ok(handshake)
                        if matches!(handshake.message, nxtools_protocol::Message::Hello(_)) =>
                    {
                        handshake
                    }
                    Ok(_) => {
                        warn!("rejected indexer pipe client without Hello");
                        return;
                    }
                    Err(error) => {
                        warn!(%error, "rejected indexer pipe client with invalid Hello");
                        return;
                    }
                };
                // A successful handshake means that this connection really owns a search session;
                // never send HelloAck for a transient try_lock failure.
                let mut service = service.lock().unwrap_or_else(|error| {
                    error!(%error, "indexer service state lock was poisoned");
                    error.into_inner()
                });
                if let Err(error) = serve_admitted_session(
                    reader,
                    connection.writer,
                    &mut service,
                    handshake,
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
