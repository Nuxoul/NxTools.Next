//! Step4 ZIP Add interaction reaches original worker/candidate, not a second commit engine.
use std::{path::PathBuf, process::Command};
fn run(decision: &str, scenario: &str) {
    let base = std::env::var_os("PI_SCRATCH_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let root = base.join(format!("edit-zip-conflict-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&root).expect("owned fixture");
    let output = Command::new("python")
        .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/edit-zip-conflict.py"))
        .arg(env!("CARGO_BIN_EXE_nxtools-archive-worker"))
        .arg(&root)
        .arg(decision)
        .arg(scenario)
        .output()
        .expect("independent worker fixture");
    assert!(
        output.status.success(),
        "{decision}/{scenario}, evidence {}:\n{}\n{}",
        root.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
#[test]
fn step4_zip_add_original_conflict_tickets_and_one_candidate() {
    for decision in ["overwrite", "skip", "auto_rename", "cancel"] {
        run(decision, "single");
    }
}
#[test]
fn step4_zip_autorename_reserves_future_incoming_names() {
    run("auto_rename", "future-name");
}
