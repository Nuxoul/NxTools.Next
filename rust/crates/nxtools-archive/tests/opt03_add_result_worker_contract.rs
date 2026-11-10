//! Actual worker/controlled native reply boundary and independent decoded candidate oracle.
#![cfg(feature = "isolated-codecs")]
use std::{path::PathBuf, process::Command};

#[test]
fn opt03_actual_add_result_follows_selected_published_members() {
    let base = std::env::var_os("PI_SCRATCH_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let worker = std::env::var_os("NXTOOLS_OPT03_ADD_RESULT_WORKER")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_nxtools-archive-worker")));
    for format in ["Zip", "SevenZ", "Tar", "TarGz", "TarXz"] {
        for (scenario, decision, expected) in [
            (
                "original",
                "overwrite",
                vec![("folder/same.txt", "overwritten")],
            ),
            (
                "original",
                "auto_rename",
                vec![
                    ("folder/same (2).txt", "renamed"),
                    ("folder/same (3).txt", "renamed"),
                ],
            ),
            ("original", "skip", vec![]),
            ("new-empty", "skip", vec![("folder/empty/", "added")]),
            ("merge-only", "skip", vec![]),
            ("commit-unknown", "skip", vec![]),
            ("display-overflow", "skip", vec![]),
        ] {
            let root = base.join(format!("opt03-add-result-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir(&root).expect("owned worker fixture root");
            let output = Command::new("python")
                .arg(
                    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("tests/fixtures/add-namespace.py"),
                )
                .arg(&worker)
                .arg(&root)
                .args([format, scenario, decision])
                .output()
                .expect("worker fixture");
            assert!(
                output.status.success(),
                "{format}/{scenario}/{decision}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let wire: Vec<serde_json::Value> = serde_json::from_slice(
                &std::fs::read(root.join("wire.json")).expect("worker wire"),
            )
            .expect("worker JSON");
            let results: Vec<_> = wire
                .iter()
                .filter(|frame| frame["op"] == "add_result")
                .collect();
            if scenario == "display-overflow" {
                assert_eq!(
                    results.len(),
                    1,
                    "one bounded result after full publication"
                );
                let result = &results[0]["add_result"];
                assert_eq!(result["total"].as_u64(), Some(301));
                let members = result["members"].as_array().expect("display subset");
                assert_eq!(members.len(), 256);
                assert!(members.iter().all(|member| member["outcome"] == "added"));
            } else if expected.is_empty() {
                assert!(
                    results.is_empty(),
                    "no publication must not claim Add results: {format}/{scenario}"
                );
            } else {
                assert_eq!(
                    results.len(),
                    1,
                    "published Add must report actual selected result: {format}/{scenario}"
                );
                let result = &results[0]["add_result"];
                assert_eq!(result["total"].as_u64(), Some(expected.len() as u64));
                let members = result["members"].as_array().expect("bounded members");
                assert_eq!(members.len(), expected.len());
                for (member, (path, outcome)) in members.iter().zip(expected) {
                    assert_eq!(member["path"], path);
                    assert_eq!(member["outcome"], outcome);
                }
            }
            std::fs::remove_dir_all(root).expect("remove owned fixture");
        }
    }
}
