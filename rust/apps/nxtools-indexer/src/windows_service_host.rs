use std::ffi::OsString;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use nxtools_indexer::{IndexerService, ServiceCommandHandler, SharedIndexerRuntime};
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

use crate::{desktop_lifetime::DesktopLifetime, elevated_helper_broker};

pub const SERVICE_NAME: &str = "NxToolsIndexerV1";
const MAX_PENDING_SESSIONS: usize = 4;
const STARTUP_OWNER_WAIT: Duration = Duration::from_secs(15);
const OWNER_POLL_INTERVAL: Duration = Duration::from_millis(100);

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

// Only an admitted Hello may call this boundary. Hold the slot across discovery
// so concurrent desktop sessions cannot create two coordinators; recheck stop
// after the potentially slow cache restore, before spawning the coordinator.
fn start_admitted_runtime<S, T: Clone>(
    slot: &Mutex<Option<T>>,
    stopping: &AtomicBool,
    discover: impl FnOnce() -> S,
    start: impl FnOnce(S) -> T,
) -> Option<T> {
    let mut slot = slot.lock().unwrap_or_else(|poison| poison.into_inner());
    if stopping.load(Ordering::Acquire) {
        return None;
    }
    if slot.is_none() {
        let state = discover();
        if stopping.load(Ordering::Acquire) {
            return None;
        }
        *slot = Some(start(state));
    }
    slot.clone()
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
    #[test]
    fn lazy_runtime_requires_native_admission_and_starts_once_for_reconnects() {
        let stopped = AtomicBool::new(false);
        let slot = Mutex::new(None::<usize>);
        let calls = AtomicUsize::new(0);
        let mut owners = DesktopLifetime::new(Instant::now(), STARTUP_OWNER_WAIT);
        assert!(slot.lock().expect("slot").is_none());
        let started = Instant::now();
        let mut no_owner = DesktopLifetime::new(started, STARTUP_OWNER_WAIT);
        assert_eq!(
            no_owner
                .poll(started + STARTUP_OWNER_WAIT)
                .expect("deadline"),
            Some(crate::desktop_lifetime::StopReason::NoOwner)
        );
        assert!(
            slot.lock()
                .expect("no runtime before desktop Hello")
                .is_none()
        );
        assert_eq!(calls.load(Ordering::Relaxed), 0);
        assert_eq!(
            admitted_desktop_client(Some(std::process::id()), Some(0), |_| true),
            None
        );
        assert_eq!(
            admitted_desktop_client(Some(std::process::id()), Some(1), |_| false),
            None
        );
        let (pid, _) = admitted_desktop_client(Some(std::process::id()), Some(1), |_| true)
            .expect("native desktop identity admitted");
        // This exercises the same retained native process object used by the service.
        assert!(owners.admit(pid).expect("own process handle"));
        let first = start_admitted_runtime(
            &slot,
            &stopped,
            || {
                calls.fetch_add(1, Ordering::Relaxed);
                7
            },
            |state| state + 1,
        );
        assert_eq!(first, Some(8));
        assert!(!owners.admit(pid).expect("reconnect owner"));
        assert_eq!(
            start_admitted_runtime(
                &slot,
                &stopped,
                || {
                    calls.fetch_add(1, Ordering::Relaxed);
                    0
                },
                |state| state
            ),
            first
        );
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        assert_eq!(owners.count(), 1);
    }

    #[test]
    fn lazy_runtime_does_not_discover_or_spawn_after_stop() {
        let stopped = AtomicBool::new(true);
        let slot = Mutex::new(None::<usize>);
        let calls = AtomicUsize::new(0);
        assert_eq!(
            start_admitted_runtime(
                &slot,
                &stopped,
                || {
                    calls.fetch_add(1, Ordering::Relaxed);
                    7
                },
                |state| state
            ),
            None
        );
        assert_eq!(calls.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn lazy_runtime_does_not_spawn_if_stop_arrives_during_discovery() {
        let stopped = Arc::new(AtomicBool::new(false));
        let slot = Arc::new(Mutex::new(None::<usize>));
        let starts = Arc::new(AtomicUsize::new(0));
        let (entered, waiting) = mpsc::channel();
        let (resume, resumed) = mpsc::channel();
        let worker_slot = Arc::clone(&slot);
        let worker_stop = Arc::clone(&stopped);
        let worker_starts = Arc::clone(&starts);
        let worker = thread::spawn(move || {
            start_admitted_runtime(
                &worker_slot,
                &worker_stop,
                || {
                    entered.send(()).expect("discovery entered");
                    resumed
                        .recv_timeout(Duration::from_secs(2))
                        .expect("resume discovery");
                    7
                },
                |state| {
                    worker_starts.fetch_add(1, Ordering::Relaxed);
                    state
                },
            )
        });
        waiting
            .recv_timeout(Duration::from_secs(2))
            .expect("discovery reached");
        stopped.store(true, Ordering::Release);
        resume.send(()).expect("resume");
        assert_eq!(worker.join().expect("worker"), None);
        assert!(slot.lock().expect("slot").is_none());
        assert_eq!(starts.load(Ordering::Relaxed), 0);
    }
    #[test]
    fn lazy_runtime_two_admitted_sessions_share_one_initialization() {
        let slot = Arc::new(Mutex::new(None::<usize>));
        let stopping = Arc::new(AtomicBool::new(false));
        let discoveries = Arc::new(AtomicUsize::new(0));
        let starts = Arc::new(AtomicUsize::new(0));
        let mut workers = Vec::new();
        for _ in 0..2 {
            let slot = Arc::clone(&slot);
            let stopping = Arc::clone(&stopping);
            let discoveries = Arc::clone(&discoveries);
            let starts = Arc::clone(&starts);
            workers.push(thread::spawn(move || {
                start_admitted_runtime(
                    &slot,
                    &stopping,
                    || {
                        discoveries.fetch_add(1, Ordering::Relaxed);
                        11
                    },
                    |state| {
                        starts.fetch_add(1, Ordering::Relaxed);
                        state
                    },
                )
            }));
        }
        for worker in workers {
            assert_eq!(worker.join().expect("admitted session"), Some(11));
        }
        assert_eq!(discoveries.load(Ordering::Relaxed), 1);
        assert_eq!(starts.load(Ordering::Relaxed), 1);
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

// Both SCM and process-lifetime retirement wake the same blocking native accept.
fn wake_accept() {
    thread::spawn(|| {
        let _ = connect_local_indexer_pipe(INDEXER_PIPE_NAME, Duration::from_millis(500));
    });
}

fn run_service() -> Result<()> {
    let started = Instant::now();
    let related_id = format!(
        "indexer-service-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let stopping = Arc::new(AtomicBool::new(false));
    let stop_reason = Arc::new(Mutex::new("service_stopped"));
    let (stop_sender, stop_receiver) = mpsc::channel();
    let monitor_sender = stop_sender.clone();
    let handler_stopping = Arc::clone(&stopping);
    let handler_reason = Arc::clone(&stop_reason);
    let handler_id = related_id.clone();
    let status_handle = service_control_handler::register(
        SERVICE_NAME,
        move |control| match control {
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            ServiceControl::Stop | ServiceControl::Shutdown => {
                if !handler_stopping.swap(true, Ordering::AcqRel) {
                    *handler_reason
                        .lock()
                        .unwrap_or_else(|poison| poison.into_inner()) =
                        if matches!(control, ServiceControl::Stop) {
                            "scm_stop"
                        } else {
                            "scm_shutdown"
                        };
                    info!(event = "indexer.service.stop_requested", related_id = %handler_id,
                        reason = if matches!(control, ServiceControl::Stop) { "scm_stop" } else { "scm_shutdown" },
                        "indexer service stopping");
                    let _ = stop_sender.send(());
                    wake_accept();
                }
                ServiceControlHandlerResult::NoError
            }
            _ => ServiceControlHandlerResult::NotImplemented,
        },
    )?;

    status_handle.set_service_status(service_status(
        ServiceState::Running,
        ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
    ))?;
    info!(event = "indexer.service.started", %related_id, "indexer Windows service ready");

    // The native accept loop may run without a desktop; discovery/USN cannot.
    let runtime = Arc::new(Mutex::new(None::<SharedIndexerRuntime>));
    let owners = Arc::new(Mutex::new(DesktopLifetime::new(
        Instant::now(),
        STARTUP_OWNER_WAIT,
    )));
    let monitor_owners = Arc::clone(&owners);
    let monitor_stopping = Arc::clone(&stopping);
    let monitor_id = related_id.clone();
    let monitor_reason = Arc::clone(&stop_reason);
    let monitor = thread::spawn(move || {
        while !monitor_stopping.load(Ordering::Acquire) {
            // Serialize the last-owner decision with concurrent Hello admission.
            let mut owners = monitor_owners
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            let decision = owners.poll(Instant::now());
            match decision {
                Ok(Some(reason)) => {
                    if !monitor_stopping.swap(true, Ordering::AcqRel) {
                        let reason = match reason {
                            crate::desktop_lifetime::StopReason::NoOwner => "no_owner_deadline",
                            crate::desktop_lifetime::StopReason::LastOwnerExited => {
                                "last_owner_exited"
                            }
                        };
                        *monitor_reason
                            .lock()
                            .unwrap_or_else(|poison| poison.into_inner()) = reason;
                        info!(event = "indexer.service.stop_requested", related_id = %monitor_id,
                            reason, "desktop lifetime ended");
                        let _ = monitor_sender.send(());
                        wake_accept();
                    }
                    break;
                }
                Err(error) => {
                    warn!(event = "indexer.service.owner_wait_failed", related_id = %monitor_id,
                        native_code = error.raw_os_error(), reason = "process_wait_failed",
                        "cannot inspect desktop owner");
                }
                Ok(None) => {}
            }
            drop(owners);
            thread::sleep(OWNER_POLL_INTERVAL);
        }
    });
    let pending_sessions = Arc::new(AtomicUsize::new(0));
    let mut sessions = Vec::new();
    while !stopping.load(Ordering::Acquire) {
        let connection = match accept_local_indexer_pipe(INDEXER_PIPE_NAME) {
            Ok(connection) => connection,
            Err(error) => {
                warn!(
                    native_code = error
                        .chain()
                        .find_map(|cause| cause.downcast_ref::<std::io::Error>())
                        .and_then(std::io::Error::raw_os_error),
                    "indexer pipe accept failed"
                );
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
                warn!(
                    native_code = error.raw_os_error(),
                    "could not resolve the packaged indexer service path"
                );
                continue;
            }
        };
        if !reserve_session_slot(&pending_sessions) {
            warn!("rejected indexer pipe client: pending session admission full");
            continue;
        }
        let session_slot = SessionSlot(Arc::clone(&pending_sessions));
        let runtime = Arc::clone(&runtime);
        let owners = Arc::clone(&owners);
        let session_stopping = Arc::clone(&stopping);
        let session_id = related_id.clone();
        let command_handler: Arc<ServiceCommandHandler> = Arc::new(move |request| {
            elevated_helper_broker::handle_service_command(
                request,
                client_process_id,
                client_session_id,
                &service_path,
            )
        });
        sessions.retain(|session: &thread::JoinHandle<()>| !session.is_finished());
        sessions.push(thread::Builder::new()
            .name("indexer-client".into())
            .spawn(move || {
                let _session_slot = session_slot;
                let mut connection = connection;
                connection.reader.get_mut().set_initial_frame_timeout(Duration::from_secs(3));
                let handshake = match read_bounded_sync_frame(&mut connection.reader)
                    .and_then(|frame| frame.ok_or_else(|| anyhow::anyhow!("missing indexer Hello")))
                    .and_then(|frame| nxtools_protocol::Envelope::decode_line(&frame).map_err(Into::into)) {
                    Ok(handshake) if matches!(handshake.message, nxtools_protocol::Message::Hello(_)) => handshake,
                    Ok(_) => { warn!("rejected indexer pipe client without Hello"); return; }
                    Err(error) => { warn!(event = "indexer.service.hello_rejected", related_id = %session_id,
                        reason = "invalid_hello", native_code = error.chain()
                            .find_map(|cause| cause.downcast_ref::<std::io::Error>())
                            .and_then(std::io::Error::raw_os_error),
                        "rejected indexer pipe client with invalid Hello"); return; }
                };
                let retired = Arc::new(AtomicBool::new(false));
                let (reader, writer, signal) = match connection
                    .into_retiring(Arc::clone(&retired), Duration::from_millis(500)) {
                    Ok(parts) => parts,
                    Err(error) => {
                        warn!(native_code = error.raw_os_error(), "indexer retirement event creation failed");
                        return;
                    }
                };
                {
                    let mut owners = owners.lock().unwrap_or_else(|poison| poison.into_inner());
                    if session_stopping.load(Ordering::Acquire) {
                        return;
                    }
                    match owners.admit(client_process_id) {
                        Ok(fresh) => info!(event = "indexer.service.owner_admitted",
                            related_id = %session_id, correlation_id = %handshake.id,
                            fresh, owner_count = owners.count(), "authenticated desktop owns service lifetime"),
                        Err(error) => {
                            warn!(event = "indexer.service.owner_admission_failed",
                                related_id = %session_id, correlation_id = %handshake.id,
                                native_code = error.raw_os_error(), reason = "process_handle_unavailable",
                                "desktop process lease rejected");
                            return;
                        }
                    }
                }
                let Some(runtime) = start_admitted_runtime(
                    &runtime,
                    &session_stopping,
                    IndexerService::discover,
                    SharedIndexerRuntime::start,
                ) else { return; };
                if runtime.serve_admitted_native(
                    reader, writer, handshake, command_handler, retired, signal,
                ).is_err() {
                    warn!(event = "indexer.session.failed", related_id = %session_id,
                        reason = "session_ended", "indexer client session ended");
                }
            })
            .map_err(windows_service::Error::Winapi)?);
    }

    status_handle.set_service_status(service_status(
        ServiceState::StopPending,
        ServiceControlAccept::empty(),
    ))?;
    stopping.store(true, Ordering::Release);
    let _ = monitor.join();
    // Retire the active endpoint and wait for the coordinator/search worker to release
    // their state before reporting Stopped; native reader retirement wakes session joins.
    if let Some(runtime) = runtime
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .take()
    {
        runtime.shutdown();
    }
    for session in sessions {
        let _ = session.join();
    }
    owners
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .clear();
    info!(event = "indexer.service.stopped", %related_id,
        reason = *stop_reason.lock().unwrap_or_else(|poison| poison.into_inner()),
        elapsed_ms = started.elapsed().as_millis() as u64, "indexer runtime retired");
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
        checkpoint: u32::from(state == ServiceState::StopPending),
        wait_hint: if state == ServiceState::StopPending {
            Duration::from_secs(30)
        } else {
            Duration::default()
        },
        process_id: None,
    }
}
