//! OPT-02 through actual worker adapters; native publication is covered by C# fixtures.
#![cfg(feature = "isolated-codecs")]
use std::{path::PathBuf, process::Command};

fn run(format: &str) {
    let base = std::env::var_os("PI_SCRATCH_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let mut scenarios = Vec::new();
    for shape in ["duplicates", "apply-all", "per-ticket"] {
        for decision in ["overwrite", "skip", "auto_rename", "cancel"] {
            scenarios.push((shape, decision));
        }
    }
    for shape in ["future", "future-reverse", "case-future"] {
        scenarios.push((shape, "auto_rename"));
    }
    for decision in ["overwrite", "skip", "auto_rename", "cancel"] {
        scenarios.push(("directories", decision));
    }
    for shape in [
        "merge-only",
        "explicit-merge",
        "new-empty",
        "kind-original",
        "kind-incoming",
        "kind-implicit",
        "source-change",
        "stale",
        "readback",
        "commit-unknown",
    ] {
        scenarios.push((shape, "skip"));
    }
    for (scenario, decision) in &scenarios {
        let root = base.join(format!("opt02-worker-{format}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).expect("owned Add fixture root");
        let output = Command::new("python")
            .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/add-namespace.py"))
            .arg(env!("CARGO_BIN_EXE_nxtools-archive-worker"))
            .arg(&root)
            .args([format, scenario, decision])
            .output()
            .expect("actual worker and independent archive oracle");
        assert!(
            output.status.success(),
            "{format}/{scenario}/{decision}, evidence {}:\n{}\n{}",
            root.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }
    println!(
        "{format}: {} actual worker scenarios passed",
        scenarios.len()
    );
}
#[test]
fn opt02_zip_namespace() {
    run("Zip");
}
#[test]
fn opt02_sevenz_namespace() {
    run("SevenZ");
}
#[test]
fn opt02_tar_namespace() {
    run("Tar");
}
#[test]
fn opt02_tgz_namespace() {
    run("TarGz");
}
#[test]
fn opt02_txz_namespace() {
    run("TarXz");
}
