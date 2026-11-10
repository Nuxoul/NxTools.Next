//! Actual worker/private-pipe contract; owned independent inputs, no native publication.
use std::{path::PathBuf, process::Command};

fn run(scenario: &str) {
    let base = std::env::var_os("PI_SCRATCH_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let root = base.join(format!("opt06-worker-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&root).expect("owned fixture root");
    let output = Command::new("python")
        .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/opt06-bulk.py"))
        .arg(env!("CARGO_BIN_EXE_nxtools-archive-worker"))
        .arg(&root)
        .arg(scenario)
        .output()
        .expect("actual worker pipe fixture");
    assert!(
        output.status.success(),
        "{scenario} evidence {}:\n{}\n{}",
        root.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn opt06_worker_negotiates_and_writes_exact_binary_payload() {
    run("extract");
}

#[test]
fn opt06_worker_reads_source_and_held_partial_with_short_io_and_eof() {
    run("create_short");
}

#[test]
fn opt06_worker_requires_exact_bulk_version_before_any_native_call() {
    for case in [
        "missing_negotiation",
        "unsupported_negotiation",
        "malformed_negotiation",
    ] {
        run(case);
    }
}

#[test]
fn opt06_worker_keeps_stop_and_wait_and_processes_pause_resume() {
    run("pause");
}

#[test]
fn opt06_worker_cancel_and_eof_wake_all_three_pending_directions() {
    for case in [
        "cancel_write",
        "eof_write",
        "cancel_source",
        "eof_source",
        "cancel_partial",
        "eof_partial",
    ] {
        run(case);
    }
}

#[test]
fn opt06_worker_rejects_wrong_ack_and_stale_duplicate_before_commit() {
    for case in [
        "ack_operation",
        "ack_call",
        "ack_token",
        "ack_sequence",
        "ack_offset",
        "ack_length",
        "duplicate",
    ] {
        run(case);
    }
}

#[test]
fn opt06_worker_rejects_raw_binding_and_json_read_fallback() {
    for case in [
        "raw_operation",
        "raw_call",
        "raw_token",
        "raw_sequence",
        "raw_offset",
        "raw_length",
        "raw_kind",
        "raw_direction",
        "json_read",
    ] {
        run(case);
    }
}

#[test]
fn opt06_worker_rejects_malformed_and_truncated_raw_headers() {
    for case in [
        "raw_zero_sequence",
        "raw_oversize",
        "raw_actual",
        "raw_version",
        "raw_reserved",
        "raw_nil_token",
        "raw_truncated",
    ] {
        run(case);
    }
}
