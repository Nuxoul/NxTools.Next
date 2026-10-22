//! Explicit resource acceptance executable; never part of the ordinary test collection.
mod resource_probe_support;
use nxtools_archive::{ArchiveEngine, ArchiveError, OpenRequest, OperationControl, ResourceLimits};
use resource_probe_support::{EngineMonitor, ProbeStatus};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

fn emit(value: Value) {
    println!("RESOURCE {value}");
}
fn main() {
    let mut args = std::env::args().skip(1);
    let scenario = args.next().expect("scenario");
    let root = PathBuf::from(args.next().expect("owned fixture root"));
    assert!(root.join("owned-resource-fixtures").is_file());
    assert_eq!(
        root.parent(),
        Some(PathBuf::from(std::env::var_os("PI_SCRATCH_DIR").expect("scratch")).as_path())
    );
    let status = ProbeStatus::from_env(&root).expect("fresh owned resource control");
    emit(
        json!({"phase":"ready","pid":std::process::id(),"scenario":scenario,"run_id":status.run_id()}),
    );
    // Only the initial barrier reads stdin. EOF is never interpreted as cancellation.
    // Driver must retain stdin after go; subsequent stop intent is correlated stop.json.
    let mut go = String::new();
    std::io::stdin().read_line(&mut go).expect("parent barrier");
    if go.trim() != "go" {
        status.error("barrier_failure", None);
        status.finish(true, false); // No archive/source/host was acquired.
        panic!("resource barrier failure");
    }
    if status.poll_stop() {
        status.operation(json!({"status":"NotStarted","reason":"stopped_before_dispatch"}));
        status.finish(true, false);
        emit(status.document());
        return;
    }
    if scenario == "engine100k" {
        engine(root, status);
    } else {
        product(root, scenario, status);
    }
}
fn engine(root: PathBuf, status: Arc<ProbeStatus>) {
    let control = OperationControl::default();
    status.bind_operation(control.operation_id());
    let monitor = EngineMonitor::start(status.clone(), control.clone());
    status.operation(json!({"status":"Running","operation_id":control.operation_id()}));
    // Unwinding drops snapshots/files inside this scope before cleanup is classified.
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        engine_measure(&root, &status, &control)
    }));
    drop(monitor); // Always joins, including failed setup/manifest assertions.
    status.poll_stop();
    status.fact("source_handles_dropped", json!(true));
    status.fact("monitor_joined", json!(true));
    let source_released = match std::fs::OpenOptions::new()
        .write(true)
        .open(root.join("100k.zip"))
    {
        Ok(handle) => {
            drop(handle);
            true
        }
        Err(failure) => {
            status.error("source_release_unconfirmed", failure.raw_os_error());
            false
        }
    };
    status.fact("source_released", json!(source_released));
    match outcome {
        Ok(result) => {
            status.operation(json!({"status":if status.stopped() {"Stopped"} else {"ExpectedResult"},"operation_id":control.operation_id(),"result":result}));
            status.finish(source_released, false);
            emit(status.document());
        }
        Err(payload) => {
            status.error("probe_panic", None);
            status.operation(json!({"status":"Failed","operation_id":control.operation_id()}));
            status.finish(source_released, false);
            emit(status.document());
            std::panic::resume_unwind(payload);
        }
    }
}
fn engine_measure(root: &Path, status: &ProbeStatus, control: &OperationControl) -> Value {
    let clock = Instant::now();
    let opened = ArchiveEngine::default().open(OpenRequest::new(root.join("100k.zip")), control);
    let snapshot = match opened {
        Ok(snapshot) => snapshot,
        Err(ArchiveError::Cancelled) if status.stopped() => {
            return json!({"scenario":"engine100k","cancelled":true});
        }
        Err(failure) => panic!("legal default100k failed: {failure:?}"),
    };
    let open_ms = clock.elapsed().as_secs_f64() * 1000.;
    assert_eq!(snapshot.entries().len(), 100000);
    let lines = BufReader::new(
        std::fs::File::open(root.join("entries.jsonl")).expect("independent manifest"),
    )
    .lines();
    let mut count = 0;
    for (entry, line) in snapshot.entries().iter().zip(lines) {
        if control.checkpoint().is_err() {
            return json!({"scenario":"engine100k","cancelled":true,"open_ms":open_ms});
        }
        let row: Value = serde_json::from_str(&line.expect("manifest line")).expect("manifest row");
        assert_eq!(entry.id().session, snapshot.session_id());
        assert_eq!(
            entry.id().ordinal,
            row["ordinal"].as_u64().expect("ordinal")
        );
        assert_eq!(entry.display_path(), row["name"].as_str().expect("name"));
        assert_eq!(
            format!("{:?}", entry.kind()),
            row["kind"].as_str().expect("kind")
        );
        assert_eq!(entry.size(), row["size"].as_u64());
        assert_eq!(entry.compressed_size(), row["packed_size"].as_u64());
        count += 1;
    }
    assert_eq!(count, 100000);
    let session = snapshot.session_id();
    drop(snapshot);
    if status.poll_stop() {
        return json!({"scenario":"engine100k","cancelled":true});
    }
    // Same original cooperative control: a stop cannot strand an unmonitored budget open.
    let entry_refusal = ArchiveEngine::with_limits(ResourceLimits {
        max_entries: 4096,
        ..Default::default()
    })
    .open(OpenRequest::new(root.join("100k.zip")), control);
    if status.poll_stop() {
        return json!({"scenario":"engine100k","cancelled":true});
    }
    assert!(matches!(entry_refusal, Err(ArchiveError::ResourceLimit)));
    let metadata_refusal = ArchiveEngine::with_limits(ResourceLimits {
        max_metadata_bytes: 4 << 20,
        ..Default::default()
    })
    .open(OpenRequest::new(root.join("100k.zip")), control);
    if status.poll_stop() {
        return json!({"scenario":"engine100k","cancelled":true});
    }
    assert!(matches!(metadata_refusal, Err(ArchiveError::ResourceLimit)));
    json!({"scenario":"engine100k","operation_id":control.operation_id(),"snapshot_id":session,"entries":count,"open_ms":open_ms,"first_batch_ms":null,"first_paint":"not_measured_no_ui","default_metadata_bytes":256u64<<20,"default_work_bytes":512u64<<20,"typed_entry_refusal":"ResourceLimit","typed_metadata_refusal":"ResourceLimit"})
}
#[cfg(windows)]
fn product(root: PathBuf, scenario: String, status: Arc<ProbeStatus>) {
    use resource_probe_support::{Fixture, PipeRuntime};
    let package_root = root
        .parent()
        .expect("scratch")
        .join(format!("archive-product-resource-{}", uuid::Uuid::new_v4()));
    let mut fixture = Fixture::new(package_root, status.clone());
    // Drop order is essential: runtime/pipe EOF precedes synchronous guard's host wait,
    // even if helper copy, host handshake, original operation, or assertions panic.
    let runtime = PipeRuntime::new(status.clone()).expect("owned runtime");
    let result = runtime.block_on(resource_probe_support::product::run(
        &root,
        &scenario,
        &mut fixture,
    ));
    drop(runtime);
    let cleaned = fixture.shutdown();
    emit(status.document());
    assert!(
        cleaned,
        "actual resource cleanup unconfirmed; retained owned evidence"
    );
    result.expect("resource product scenario failed");
}
#[cfg(not(windows))]
fn product(_: PathBuf, _: String, status: Arc<ProbeStatus>) {
    status.error("windows_required", None);
    status.finish(true, false);
    panic!("Windows typed product boundary required");
}
