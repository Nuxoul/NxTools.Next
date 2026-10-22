use super::*;
use nxtools_archive::{ArchiveError, OperationControl};
use serde_json::json;
use std::{cell::RefCell, rc::Rc};

fn control() -> (tempfile::TempDir, std::sync::Arc<ProbeStatus>) {
    let root = tempfile::tempdir_in(std::env::var_os("PI_SCRATCH_DIR").expect("session scratch"))
        .expect("test control owner");
    let directory = root.path().join("control");
    std::fs::create_dir(&directory).expect("fresh control");
    let status = ProbeStatus::new(&directory, uuid::Uuid::new_v4()).expect("control");
    (root, status)
}
fn stop(root: &tempfile::TempDir, run_id: &str) {
    std::fs::write(
        root.path().join("control/stop.tmp"),
        json!({"run_id":run_id,"reason":"driver_exception"}).to_string(),
    )
    .expect("stop");
    std::fs::rename(
        root.path().join("control/stop.tmp"),
        root.path().join("control/stop.json"),
    )
    .expect("atomic stop");
}

#[test]
fn stop_cancels_original_paused_control_and_monitor_joins() {
    let (root, status) = control();
    let original = OperationControl::default();
    original.set_paused(true);
    let monitor = EngineMonitor::start(status.clone(), original.clone());
    stop(&root, &status.run_id());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while !status.stopped() && std::time::Instant::now() < deadline {
        std::thread::yield_now();
    }
    drop(monitor); // joins; never owns or waits on stdin
    original.set_paused(false); // A regression must fail an assertion, never hang the test lane.
    assert!(matches!(
        original.checkpoint(),
        Err(ArchiveError::Cancelled)
    ));
    let document = status.document();
    assert_eq!(document["stop"]["reason"], "driver_exception");
    assert_eq!(document["stop"]["dispatch_count"], 1);
}

#[test]
fn duplicate_late_and_wrong_run_stop_do_not_dispatch_again() {
    let (root, status) = control();
    stop(&root, &uuid::Uuid::new_v4().to_string());
    assert!(!status.poll_stop());
    stop(&root, &status.run_id());
    assert!(status.poll_stop());
    status.request_stop("sampler_failure");
    status.finish(true, false);
    status.request_stop("driver_exception");
    assert!(status.poll_stop());
    assert_eq!(status.document()["stop"]["dispatch_count"], 1);
    assert_eq!(status.document()["state"], "Cleaned");
    assert_eq!(status.document()["stop"]["reason"], "driver_exception");
}

#[test]
fn oversized_or_unreadable_control_fails_closed_without_unbounded_details() {
    let (root, status) = control();
    std::fs::create_dir(root.path().join("control/stop.json")).expect("unreadable regular file");
    assert!(status.poll_stop());
    assert_eq!(status.document()["stop"]["reason"], "control_read_failure");
    for _ in 0..40 {
        status.error("control_read_failure", None);
    }
    assert!(
        status.document()["errors"]
            .as_array()
            .expect("errors")
            .len()
            <= 16
    );
    let (root, status) = control();
    std::fs::write(root.path().join("control/stop.json"), vec![b' '; 16385])
        .expect("oversized stop");
    assert!(status.poll_stop());
}

#[test]
fn completed_owner_ignores_a_late_new_stop() {
    let (root, status) = control();
    status.finish(true, false);
    stop(&root, &status.run_id());
    assert!(!status.poll_stop());
    assert_eq!(status.document()["stop"]["dispatch_count"], 0);
}

struct ObservedChild {
    events: Rc<RefCell<Vec<&'static str>>>,
    waits: usize,
    cooperative: bool,
}
impl OwnedChild for ObservedChild {
    fn exited(&mut self) -> std::io::Result<bool> {
        self.events.borrow_mut().push("wait");
        self.waits += 1;
        Ok(self.cooperative && self.waits > 1)
    }
    fn force(&mut self) -> std::io::Result<()> {
        self.events.borrow_mut().push("force");
        Ok(())
    }
}
#[test]
fn original_guard_symptom_eof_and_cooperation_precede_force() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut child = ObservedChild {
        events: events.clone(),
        waits: 0,
        cooperative: true,
    };
    let report = drain_owned_child(
        &mut child,
        || events.borrow_mut().push("pipe_eof"),
        3,
        || {},
        || false,
        || {},
    );
    assert!(report.exited);
    assert!(
        !report.forced,
        "cooperative child must never be killed first"
    );
    assert_eq!(events.borrow().as_slice(), ["pipe_eof", "wait", "wait"]);
}
#[test]
fn uncooperative_guard_is_forced_only_after_bounded_original_wait() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut child = ObservedChild {
        events: events.clone(),
        waits: 0,
        cooperative: false,
    };
    let report = drain_owned_child(
        &mut child,
        || events.borrow_mut().push("pipe_eof"),
        2,
        || {},
        || false,
        || {},
    );
    assert!(report.forced);
    assert!(!report.exited, "force receipt is not cleanup confirmation");
    let recorded = events.borrow();
    assert_eq!(
        &recorded[..5],
        ["pipe_eof", "wait", "wait", "wait", "force"]
    );
    assert_eq!(
        recorded[5..]
            .iter()
            .filter(|event| **event == "wait")
            .count(),
        50
    );
}

#[test]
fn init_guard_preserves_unknown_package_entries_and_partial_evidence() {
    let (root, status) = control();
    let package = root.path().join("package");
    std::fs::create_dir(&package).expect("owned package");
    std::fs::write(package.join("NxTools.ArchiveWorker.exe"), b"owned helper").expect("helper");
    std::fs::write(package.join("unconfirmed.partial"), b"partial evidence").expect("partial");
    status.fact("pipe_closed", json!(true));
    let mut fixture = Fixture::new(package.clone(), status.clone());
    fixture.created = true;
    assert!(!fixture.shutdown());
    assert!(package.join("unconfirmed.partial").is_file());
    assert!(package.join("NxTools.ArchiveWorker.exe").is_file());
    assert_eq!(status.document()["state"], "CleanupIncomplete");
}

#[test]
fn initialization_without_host_removes_only_known_owned_package_files() {
    let (root, status) = control();
    let package = root.path().join("package");
    std::fs::create_dir(&package).expect("owned package");
    std::fs::write(package.join("NxTools.ArchiveWorker.exe"), b"owned helper").expect("helper");
    status.fact("pipe_closed", json!(true));
    let mut fixture = Fixture::new(package.clone(), status.clone());
    fixture.created = true;
    assert!(fixture.shutdown());
    assert!(!package.exists());
    assert_eq!(status.document()["cleanup"]["status"], "Confirmed");
}

#[test]
fn status_is_bounded_atomic_and_exact_run_correlated() {
    let (root, status) = control();
    status.bind_operation(uuid::Uuid::new_v4());
    let bytes =
        std::fs::read(root.path().join("control/status.json")).expect("single published status");
    assert!(bytes.len() <= 16384);
    let document: serde_json::Value = serde_json::from_slice(&bytes).expect("atomic JSON");
    assert_eq!(document["schema"], 2);
    assert_eq!(document["run_id"], status.run_id());
    assert!(document["operation_id"].is_string());
    assert_eq!(document["identities"][0]["role"], "probe_owner");
    assert!(!root.path().join("control/status.tmp").exists());
    assert!(ProbeStatus::new(&root.path().join("control"), uuid::Uuid::new_v4()).is_err());
}

#[test]
fn original_operation_without_remote_terminal_is_never_cleanup_even_if_source_is_writable() {
    let (root, status) = control();
    let package = root.path().join("package");
    std::fs::create_dir(&package).expect("owned package");
    std::fs::write(
        package.join("NxTools.ArchiveWorker.exe"),
        b"helper evidence",
    )
    .expect("helper");
    let source = root.path().join("source.zip");
    std::fs::write(&source, b"tiny source").expect("source");
    status.fact("pipe_closed", json!(true));
    let mut fixture = Fixture::new(package.clone(), status.clone());
    fixture.created = true;
    fixture.sources.push((uuid::Uuid::new_v4(), source));
    assert!(!fixture.shutdown());
    assert!(package.join("NxTools.ArchiveWorker.exe").is_file());
    assert_eq!(
        status.document()["cleanup"]["remote_cleanup_complete"],
        false
    );
    assert_eq!(status.document()["cleanup"]["confirmed"], false);
}

#[test]
fn missing_owned_control_directory_is_not_an_absent_stop_request() {
    let (root, status) = control();
    std::fs::rename(
        root.path().join("control"),
        root.path().join("retained-control"),
    )
    .expect("inject lost control directory");
    assert!(status.poll_stop());
    assert_eq!(status.document()["stop"]["reason"], "control_read_failure");
}
