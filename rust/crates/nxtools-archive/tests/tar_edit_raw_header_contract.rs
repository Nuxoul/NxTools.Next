//! TAR-AC01 at the actual worker/candidate boundary; independent raw fixture bytes are authority.
#![cfg(feature = "isolated-codecs")]
use std::{path::PathBuf, process::Command};

fn exercise(mode: &str) {
    let root = if let Some(base) = std::env::var_os("NXTOOLS_TAR_AC01_EVIDENCE") {
        let path = PathBuf::from(base).join(format!("{mode}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).expect("owned evidence");
        path
    } else {
        tempfile::tempdir().expect("owned fixture").keep()
    };
    let script =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tar-edit-raw-headers.py");
    let output = Command::new("python")
        .arg(script)
        .arg(env!("CARGO_BIN_EXE_nxtools-archive-worker"))
        .arg(&root)
        .arg(mode)
        .output()
        .expect("independent actual-worker raw-header oracle");
    std::fs::write(root.join("oracle.stdout"), &output.stdout).expect("evidence stdout");
    std::fs::write(root.join("oracle.stderr"), &output.stderr).expect("evidence stderr");
    assert!(
        output.status.success(),
        "TAR-AC01 {mode}, evidence {}:\n{}\n{}",
        root.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn tar_ac01_pax_only_rename_preserves_untouched_main_and_extension_fields() {
    exercise("pax");
}
#[test]
fn tar_ac01_gnu_only_rename_preserves_unrelated_pax_and_main_header() {
    exercise("gnu");
}
#[test]
fn tar_ac01_replacement_preserves_unrelated_extension_for_same_and_changed_size() {
    exercise("replace_unrelated");
    exercise("replace_gnu");
}
#[test]
fn tar_ac01_replacement_updates_only_affected_pax_size_record_and_dependent_header() {
    exercise("replace_pax");
}
