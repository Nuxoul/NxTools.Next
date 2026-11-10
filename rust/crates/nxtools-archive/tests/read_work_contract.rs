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
