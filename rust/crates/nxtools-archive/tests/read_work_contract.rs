use nxtools_archive::worker::{WorkerFrame, WorkerRequest, read_frame, write_frame};
use nxtools_archive::{
    ArchiveEngine, ArchiveError, ArchiveModificationRefusal, OpenRequest, OperationControl,
};
use std::{
    fs::File,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};
use uuid::Uuid;

fn fixture(path: &Path) {
    let mut zip = zip::ZipWriter::new(File::create(path).expect("owned fixture"));
    for index in 0..16 {
        zip.start_file(
            format!("member-{index}.bin"),
            zip::write::SimpleFileOptions::default(),
        )
        .expect("member");
        zip.write_all(&vec![index as u8; 65536]).expect("payload");
    }
    zip.finish().expect("complete fixture");
}

#[test]
fn browser_open_shares_zip_directory_and_revision_with_edit_admission() {
    let root = tempfile::tempdir().expect("owned root");
    let path = root.path().join("read.zip");
    fixture(&path);
    let control = OperationControl::default();
    let snapshot = ArchiveEngine::default()
        .open(OpenRequest::new(&path), &control)
        .expect("browser open");
    assert!(snapshot.capabilities().add);
    let work = control.work_observation();
    assert_eq!(
        work.catalog_loads, 1,
        "browser edit qualification must reuse its catalog"
    );
    assert_eq!(
        work.revision_passes, 2,
        "initial and final revision enclose shared admission"
    );
    assert_eq!(
        work.revision_bytes,
        2 * path.metadata().expect("source size").len()
    );
}

#[test]
fn public_independent_edit_admission_retains_source_change_detection() {
    let root = tempfile::tempdir().expect("owned root");
    let path = root.path().join("read.zip");
    fixture(&path);
    let engine = ArchiveEngine::default();
    let control = OperationControl::default();
    let snapshot = engine
        .open(OpenRequest::new(&path), &control)
        .expect("snapshot");
    File::create(&path)
        .expect("replace owned unleased source")
        .write_all(b"changed")
        .expect("change");
    assert_eq!(
        engine.modification_eligibility(&snapshot, &control),
        Err(ArchiveModificationRefusal::SourceChanged)
    );
    assert!(matches!(
        engine.execute(nxtools_archive::ArchiveRequest::test(snapshot), &control),
        Err(ArchiveError::SourceChanged)
    ));
}

#[test]
fn actual_test_worker_reads_catalog_without_edit_admission() {
    let root = tempfile::tempdir().expect("owned root");
    let path = root.path().join("read.zip");
    fixture(&path);
    let mut child = Command::new(env!("CARGO_BIN_EXE_nxtools-archive-worker"))
        .arg("--archive-worker")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("original worker");
    let mut request = WorkerRequest::open(Uuid::new_v4(), path.to_string_lossy().into_owned());
    request.operation = "test".into();
    write_frame(child.stdin.as_mut().expect("stdin"), &request).expect("start");
    let terminal = loop {
        let frame: WorkerFrame = read_frame(child.stdout.as_mut().expect("stdout"))
            .expect("frame")
            .expect("before EOF");
        if matches!(frame.op.as_str(), "complete" | "failure") {
            break frame;
        }
    };
    drop(child.stdin.take());
    let exited = child.wait_with_output().expect("worker cleanup");
    assert!(exited.status.success() && exited.stderr.is_empty());
    assert_eq!(terminal.op, "complete");
    assert_eq!(terminal.files_verified, Some(16));
    assert_eq!(terminal.bytes_verified, Some(16 * 65536));
    let work = terminal.work.expect("actual terminal work");
    assert_eq!(
        work.eligibility_checks, 0,
        "Test must not discover unused edit capability"
    );
    assert_eq!(work.catalog_loads, 1);
}

#[test]
fn browser_catalog_budget_does_not_require_optional_edit_records() {
    let root = tempfile::tempdir().expect("owned root");
    let path = root.path().join("small.zip");
    let mut zip = zip::ZipWriter::new(File::create(&path).expect("owned fixture"));
    zip.start_file("a", zip::write::SimpleFileOptions::default())
        .expect("member");
    zip.write_all(b"a").expect("payload");
    zip.finish().expect("complete fixture");
    let engine = ArchiveEngine::with_limits(nxtools_archive::ResourceLimits {
        max_metadata_bytes: 1084,
        ..Default::default()
    });
    let control = OperationControl::default();
    let snapshot = engine
        .open(OpenRequest::new(&path), &control)
        .expect("catalog reservation fits without optional edit records");
    assert_eq!(snapshot.entries().len(), 1);
    assert!(!snapshot.capabilities().add);
    assert_eq!(
        snapshot.capabilities().modification_refusal,
        Some(ArchiveModificationRefusal::UnverifiedMetadata)
    );
    assert!(snapshot.capabilities().test && snapshot.capabilities().extract);
    assert_eq!(control.work_observation().revision_passes, 2);
    let too_small = ArchiveEngine::with_limits(nxtools_archive::ResourceLimits {
        max_metadata_bytes: 1,
        ..Default::default()
    });
    assert!(matches!(
        too_small.open(OpenRequest::new(&path), &OperationControl::default()),
        Err(ArchiveError::ResourceLimit)
    ));
}

#[cfg(feature = "isolated-codecs")]
mod sevenz_metadata_workspace {
    use super::*;
    use serde_json::{Value, json};

    const FIXTURE: &[u8] = include_bytes!("fixtures/sevenz-workspace.7z");

    fn run(
        bytes: &[u8],
        operation: &str,
        configure: impl FnOnce(&mut WorkerRequest),
    ) -> Vec<Value> {
        let root = tempfile::tempdir().expect("owned 7z root");
        let path = root.path().join("workspace.7z");
        std::fs::write(&path, bytes).expect("independent fixture");
        let id = Uuid::new_v4();
        let mut request = WorkerRequest::open(id, path.to_string_lossy().into_owned());
        request.version = 2;
        request.operation = operation.into();
        request.limits.max_working_bytes = 64 << 20;
        request.limits.max_metadata_bytes = 128 << 20;
        request.limits.max_entries = 100_000;
        configure(&mut request);
        request.bulk_transfer = (operation == "extract").then_some(1);
        request.validate().expect("original request admission");
        let mut child = Command::new(env!("CARGO_BIN_EXE_nxtools-archive-worker"))
            .arg("--archive-worker")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("original isolated worker");
        write_frame(child.stdin.as_mut().expect("stdin"), &request).expect("start");
        let mut frames = Vec::new();
        loop {
            let frame: Value = read_frame(child.stdout.as_mut().expect("stdout"))
                .expect("bounded frame")
                .expect("terminal before EOF");
            if frame["op"] == "native_call" {
                let native = &frame["native"];
                assert!(
                    matches!(
                        native["action"].as_str(),
                        Some("source_acquire" | "source_validated" | "codec_opened")
                    ),
                    "workspace refusal/metadata Open must precede payload/output: {frame}"
                );
                write_frame(
                    child.stdin.as_mut().expect("stdin"),
                    &json!({
                        "version":1, "operation_id":id, "op":"native_result",
                        "call_id":native["call_id"], "success":true
                    }),
                )
                .expect("original native control");
            }
            let terminal = matches!(
                frame["op"].as_str(),
                Some("complete" | "failure" | "cancelled")
            );
            frames.push(frame);
            if terminal {
                break;
            }
        }
        drop(child.stdin.take());
        let exited = child.wait_with_output().expect("worker exit");
        assert!(exited.status.success() && exited.stderr.is_empty());
        let terminal = frames.last().expect("terminal");
        let work = &terminal["work"];
        assert_eq!(work["sevenz_block_decodes"], 0);
        assert_eq!(work["output_begin_calls"], 0);
        assert_eq!(work["output_write_bytes"], 0);
        frames
    }

    #[test]
    fn sevenz_metadata_open_does_not_reserve_payload_dictionary() {
        let frames = run(FIXTURE, "open", |_| {});
        let terminal = frames.last().expect("terminal");
        assert_eq!(terminal["op"], "complete", "{terminal}");
        let entries: Vec<_> = frames
            .iter()
            .filter_map(|frame| frame["entries"].as_array())
            .flatten()
            .collect();
        assert_eq!(entries.len(), 2);
        for (index, (name, size)) in [("a.txt", 6), ("b.txt", 5)].into_iter().enumerate() {
            assert_eq!(entries[index]["ordinal"], index);
            assert_eq!(entries[index]["name"], name);
            assert_eq!(entries[index]["size"], size);
            assert_eq!(entries[index]["kind"], "File");
            assert_eq!(entries[index]["encrypted"], false);
        }
        assert_eq!(terminal["work"]["catalog_loads"], 1);
    }

    #[test]
    fn sevenz_payload_operations_keep_workspace_admission_before_decode_or_output() {
        for operation in ["test", "extract", "selected", "retained"] {
            let frames = run(
                FIXTURE,
                if operation == "test" {
                    "test"
                } else {
                    "extract"
                },
                |request| {
                    if matches!(operation, "selected" | "retained") {
                        request.browser_entry = Some(serde_json::from_value(json!({
                        "ordinal":0, "expected":{
                            "snapshot_id":Uuid::new_v4(), "format":"SevenZ", "comment":"",
                            "source":{"native":"v1:fixture", "digest":([0u8;32]), "size":134, "modified_unix_ns":0}
                        }
                    })).expect("valid selected context"));
                    }
                    if operation == "retained" {
                        request.drag_materialization =
                            Some(nxtools_archive::worker::DragMaterialization {
                                version: 1,
                                members: vec![nxtools_archive::worker::DragMember {
                                    index: 0,
                                    ordinal: 0,
                                }],
                            });
                    }
                },
            );
            let terminal = frames.last().expect("terminal");
            assert_eq!(terminal["op"], "failure", "{operation}: {terminal}");
            assert_eq!(
                terminal["error_code"], "ResourceLimit",
                "{operation}: {terminal}"
            );
            assert!(!frames.iter().any(|frame| matches!(
                frame["op"].as_str(),
                Some("entry_batch" | "drag_ready" | "drag_member_ready")
            )));
        }
    }

    #[test]
    fn sevenz_encoded_header_dictionary_still_limits_metadata_open() {
        let mut bytes = FIXTURE.to_vec();
        let next = 32 + u64::from_le_bytes(bytes[12..20].try_into().expect("offset")) as usize;
        let marker = [0x21, 0x21, 1, 0];
        let at = bytes[next..]
            .windows(marker.len())
            .position(|part| part == marker)
            .expect("header LZMA2 properties");
        bytes[next + at + 3] = 31;
        let crc = crc32fast::hash(&bytes[next..]);
        bytes[28..32].copy_from_slice(&crc.to_le_bytes());
        let crc = crc32fast::hash(&bytes[12..32]);
        bytes[8..12].copy_from_slice(&crc.to_le_bytes());
        let frames = run(&bytes, "open", |_| {});
        let terminal = frames.last().expect("terminal");
        assert_eq!(terminal["op"], "failure");
        assert_eq!(terminal["error_code"], "ResourceLimit");
    }

    #[test]
    fn sevenz_metadata_open_still_validates_payload_coders() {
        for (bytes, code) in [
            (
                include_bytes!("fixtures/sevenz-invalid-properties.7z").as_slice(),
                "CorruptArchive",
            ),
            (
                include_bytes!("fixtures/sevenz-unsupported-coder.7z").as_slice(),
                "UnsupportedMethod",
            ),
        ] {
            let frames = run(bytes, "open", |_| {});
            let terminal = frames.last().expect("terminal");
            assert_eq!(terminal["op"], "failure");
            assert_eq!(terminal["error_code"], code);
        }
    }
}
