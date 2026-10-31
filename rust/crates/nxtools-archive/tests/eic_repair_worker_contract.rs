//! Deterministic EIC native rendezvous; no real publication or user data.
#![cfg(feature = "isolated-codecs")]
use std::{fs::File, io::Cursor, path::PathBuf, process::Command};
fn run(scenario: &str, format: &str, error: &str) {
    let base = std::env::var_os("PI_SCRATCH_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let root = base.join(format!("eic-worker-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&root).expect("GUID owned fixture");
    if format == "SevenZ" {
        let mut archive =
            sevenz_rust2::ArchiveWriter::new(File::create(root.join("owned.7z")).expect("seed"))
                .expect("writer");
        archive.set_encrypt_header(false);
        archive.set_content_methods(vec![
            sevenz_rust2::encoder_options::Lzma2Options::from_level(1).into(),
        ]);
        archive
            .push_archive_entry(
                sevenz_rust2::ArchiveEntry::new_file("old.txt"),
                Some(Cursor::new(b"payload")),
            )
            .expect("member");
        archive.finish().expect("finish");
    }
    let output = Command::new("python")
        .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/eic-edit-repair.py"))
        .arg(env!("CARGO_BIN_EXE_nxtools-archive-worker"))
        .arg(&root)
        .args([scenario, format, error])
        .output()
        .expect("actual worker/native reply fixture");
    assert!(
        output.status.success(),
        "{scenario}/{format}/{error} evidence {}:\n{}\n{}",
        root.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
#[test]
fn eic_ac01_implicit_and_explicit_directory_merge_all_skip_never_builds_candidate() {
    for scenario in ["implicit-skip", "explicit-skip"] {
        run(scenario, "Zip", "");
    }
}
#[test]
fn eic_ac01_new_empty_directory_and_real_file_survive_merge_without_prefix_confusion() {
    for scenario in ["new-empty", "real-file", "boundary-new", "kind-conflict"] {
        run(scenario, "Zip", "");
    }
}
#[test]
fn eic_ac02_native_reply_only_cancelled_preserves_zip_tar_sevenz_semantic() {
    for format in ["Zip", "Tar", "SevenZ"] {
        run("reply-only", format, "cancelled");
    }
}
#[test]
fn eic_ac02_other_native_failures_are_not_relabelled_cancelled() {
    for error in [
        "source_changed",
        "target_exists",
        "write_limit",
        "replace_file",
        "commit_outcome_unknown",
    ] {
        run("reply-only", "Tar", error);
    }
}
