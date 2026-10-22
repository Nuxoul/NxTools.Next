use nxtools_archive::{
    ArchiveEngine, ArchiveFormat, ArchiveRequest, OpenRequest, OperationControl,
};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn independent_python_zip_is_listed_and_every_payload_is_verified() {
    for name in ["stored.zip", "deflated.zip"] {
        let engine = ArchiveEngine::default();
        let control = OperationControl::default();
        let snapshot = engine
            .open(OpenRequest::new(fixture(name)), &control)
            .expect("open Python ZIP");
        assert_eq!(snapshot.format(), ArchiveFormat::Zip);
        assert_eq!(snapshot.entries().len(), 4);
        assert_eq!(snapshot.entries()[1].display_path(), "资料/说明.txt");
        assert_eq!(snapshot.entries()[1].size(), Some(19));
        assert_eq!(snapshot.comment(), "independent-python-fixture");
        let outcome = engine
            .execute(ArchiveRequest::test(snapshot), &control)
            .expect("verify all ZIP streams");
        assert_eq!(outcome.files_verified(), 3);
        assert_eq!(outcome.bytes_verified(), 275);
    }
}

#[test]
fn unsafe_names_are_visible_and_never_sanitized_into_valid_outputs() {
    let snapshot = ArchiveEngine::default()
        .open(
            OpenRequest::new(fixture("unsafe.zip")),
            &OperationControl::default(),
        )
        .expect("unsafe names must stay visible");
    assert_eq!(snapshot.entries().len(), 16);
    assert_eq!(snapshot.entries()[0].display_path(), "../escape");
    for entry in snapshot.entries() {
        assert!(
            entry
                .warnings()
                .contains(&nxtools_archive::EntryWarning::UnsafePath),
            "missing unsafe-path warning at ordinal {}",
            entry.id().ordinal
        );
    }
}

#[test]
fn duplicates_case_and_file_directory_conflicts_remain_visible() {
    let engine = ArchiveEngine::default();
    let control = OperationControl::default();
    let snapshot = engine
        .open(OpenRequest::new(fixture("collisions.zip")), &control)
        .expect("list all original ordinals");
    assert_eq!(snapshot.entries().len(), 8);
    assert_ne!(snapshot.entries()[0].id(), snapshot.entries()[1].id());
    for entry in snapshot.entries() {
        assert!(
            entry
                .warnings()
                .contains(&nxtools_archive::EntryWarning::OutputCollision),
            "missing collision warning at ordinal {}",
            entry.id().ordinal
        );
    }
    assert_eq!(
        engine.execute(ArchiveRequest::test(snapshot), &control),
        Err(nxtools_archive::ArchiveError::OutputCollision)
    );
}

#[test]
fn corrupted_payload_is_never_a_successful_test_or_generic_io_failure() {
    let engine = ArchiveEngine::default();
    let control = OperationControl::default();
    let snapshot = engine
        .open(OpenRequest::new(fixture("bad-crc.zip")), &control)
        .expect("CRC does not prevent metadata browsing");
    assert_eq!(
        engine.execute(ArchiveRequest::test(snapshot), &control),
        Err(nxtools_archive::ArchiveError::CorruptArchive)
    );
}

#[test]
fn manual_code_pages_and_authoritative_unicode_keep_their_provenance() {
    use nxtools_archive::{NameEncoding, NameProvenance};
    let engine = ArchiveEngine::default();
    for (name, encoding, expected) in [
        ("gbk.zip", NameEncoding::Gbk, "中文.txt"),
        ("shift_jis.zip", NameEncoding::ShiftJis, "日本語.txt"),
        ("cp437.zip", NameEncoding::Cp437, "café.txt"),
    ] {
        let mut request = OpenRequest::new(fixture(name));
        request.encoding = encoding;
        let snapshot = engine
            .open(request, &OperationControl::default())
            .expect("explicit code page");
        assert_eq!(snapshot.entries()[0].display_path(), expected);
        assert_eq!(
            snapshot.entries()[0].name_provenance(),
            NameProvenance::Explicit(encoding)
        );
    }
    for (name, expected, provenance) in [
        (
            "unicode-extra.zip",
            "权威名称.txt",
            NameProvenance::UnicodeExtra,
        ),
        (
            "unicode-invalid-crc.zip",
            "旧名.txt",
            NameProvenance::Explicit(NameEncoding::Gbk),
        ),
        ("stored.zip", "资料/", NameProvenance::Utf8Flag),
    ] {
        let mut request = OpenRequest::new(fixture(name));
        request.encoding = NameEncoding::Gbk;
        let snapshot = engine
            .open(request, &OperationControl::default())
            .expect("Unicode authority");
        assert_eq!(snapshot.entries()[0].display_path(), expected);
        assert_eq!(snapshot.entries()[0].name_provenance(), provenance);
    }
    let snapshot = engine
        .open(
            OpenRequest::new(fixture("gbk.zip")),
            &OperationControl::default(),
        )
        .expect("uncertain auto fallback");
    assert_eq!(
        snapshot.entries()[0].name_provenance(),
        NameProvenance::UncertainCp437
    );
    assert!(
        snapshot.entries()[0]
            .warnings()
            .contains(&nxtools_archive::EntryWarning::UncertainEncoding)
    );
}

#[test]
fn finite_metadata_and_actual_stream_budgets_are_enforced() {
    use nxtools_archive::{ArchiveError, ResourceLimits};
    let control = OperationControl::default();
    for limits in [
        ResourceLimits {
            max_entries: 3,
            ..Default::default()
        },
        ResourceLimits {
            max_metadata_bytes: 64,
            ..Default::default()
        },
    ] {
        let result = ArchiveEngine::with_limits(limits)
            .open(OpenRequest::new(fixture("stored.zip")), &control);
        assert!(matches!(result, Err(ArchiveError::ResourceLimit)));
    }
    for limits in [
        ResourceLimits {
            max_entry_bytes: 255,
            ..Default::default()
        },
        ResourceLimits {
            max_output_bytes: 274,
            ..Default::default()
        },
    ] {
        let engine = ArchiveEngine::with_limits(limits);
        let snapshot = engine
            .open(OpenRequest::new(fixture("stored.zip")), &control)
            .expect("listing is not extraction");
        assert_eq!(
            engine.execute(ArchiveRequest::test(snapshot), &control),
            Err(ArchiveError::ResourceLimit)
        );
    }
}

#[test]
fn old_snapshot_cannot_address_a_replaced_source() {
    let temporary = tempfile::tempdir().expect("owned fixture root");
    let path = temporary.path().join("source.zip");
    std::fs::copy(fixture("stored.zip"), &path).expect("copy original");
    let engine = ArchiveEngine::default();
    let control = OperationControl::default();
    let snapshot = engine
        .open(OpenRequest::new(&path), &control)
        .expect("original revision");
    std::fs::copy(fixture("deflated.zip"), &path).expect("replace source");
    assert_eq!(
        engine.execute(ArchiveRequest::test(snapshot), &control),
        Err(nxtools_archive::ArchiveError::SourceChanged)
    );
}

#[test]
fn cancellation_wakes_a_paused_real_open_without_reading_or_hanging() {
    use std::{
        sync::mpsc,
        thread,
        time::{Duration, Instant},
    };
    let control = OperationControl::default();
    control.set_paused(true);
    let worker_control = control.clone();
    let (started_tx, started_rx) = mpsc::channel();
    let (finished_tx, finished_rx) = mpsc::channel();
    let worker = thread::spawn(move || {
        started_tx.send(()).expect("signal owned worker");
        let result =
            ArchiveEngine::default().open(OpenRequest::new(fixture("stored.zip")), &worker_control);
        finished_tx
            .send(matches!(
                result,
                Err(nxtools_archive::ArchiveError::Cancelled)
            ))
            .expect("report worker cancellation");
    });
    started_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("worker started");
    assert!(
        finished_rx.recv_timeout(Duration::from_millis(20)).is_err(),
        "paused operation must not complete"
    );
    let before = Instant::now();
    control.cancel();
    assert!(
        finished_rx
            .recv_timeout(Duration::from_millis(200))
            .expect("cooperative cancellation deadline")
    );
    assert!(before.elapsed() < Duration::from_millis(200));
    worker.join().expect("worker exits normally");
}

#[test]
fn local_header_cannot_silently_disagree_with_the_listed_central_record() {
    let engine = ArchiveEngine::default();
    let control = OperationControl::default();
    let snapshot = engine
        .open(OpenRequest::new(fixture("local-mismatch.zip")), &control)
        .expect("retain central metadata for inspection");
    assert_eq!(
        engine.execute(ArchiveRequest::test(snapshot), &control),
        Err(nxtools_archive::ArchiveError::CorruptArchive)
    );
}

#[test]
fn deflate_requires_stream_end_even_when_truncated_payload_has_full_size_and_crc() {
    let engine = ArchiveEngine::default();
    let control = OperationControl::default();
    let snapshot = engine
        .open(OpenRequest::new(fixture("truncated-deflate.zip")), &control)
        .expect("browse truncated stream metadata");
    assert_eq!(
        engine.execute(ArchiveRequest::test(snapshot), &control),
        Err(nxtools_archive::ArchiveError::CorruptArchive)
    );
}

#[test]
fn shift_jis_trail_byte_is_not_a_directory_separator() {
    let engine = ArchiveEngine::default();
    let control = OperationControl::default();
    let mut request = OpenRequest::new(fixture("shift-jis-trailing-5c.zip"));
    request.encoding = nxtools_archive::NameEncoding::ShiftJis;
    let snapshot = engine
        .open(request, &control)
        .expect("valid Shift-JIS regular file");
    assert_eq!(snapshot.entries()[0].display_path(), "表");
    assert_eq!(
        snapshot.entries()[0].kind(),
        nxtools_archive::EntryKind::File
    );
    assert_eq!(
        engine
            .execute(ArchiveRequest::test(snapshot), &control)
            .expect("verify multibyte file")
            .files_verified(),
        1
    );
}

#[test]
fn arbitrary_comment_may_contain_an_eocd_signature() {
    let snapshot = ArchiveEngine::default()
        .open(
            OpenRequest::new(fixture("comment-eocd.zip")),
            &OperationControl::default(),
        )
        .expect("comment is not a second archive");
    assert_eq!(snapshot.entries().len(), 1);
    assert!(snapshot.comment().starts_with("arbitrary-comment"));
}

#[test]
fn declared_budget_rejects_before_decode_and_terminal_progress_is_exact() {
    use nxtools_archive::{ArchiveError, ResourceLimits};
    let control = OperationControl::default();
    let engine = ArchiveEngine::with_limits(ResourceLimits {
        max_entry_bytes: 255,
        ..Default::default()
    });
    let snapshot = engine
        .open(OpenRequest::new(fixture("deflated.zip")), &control)
        .expect("bounded metadata");
    assert_eq!(
        engine.execute(ArchiveRequest::test(snapshot), &control),
        Err(ArchiveError::ResourceLimit)
    );
    assert_eq!(
        control.latest_progress(),
        None,
        "declared sizes preflight before any decode"
    );
    let engine = ArchiveEngine::default();
    let control = OperationControl::default();
    let snapshot = engine
        .open(OpenRequest::new(fixture("stored.zip")), &control)
        .expect("normal metadata");
    engine
        .execute(ArchiveRequest::test(snapshot), &control)
        .expect("valid payload");
    let latest = control
        .latest_progress()
        .expect("terminal progress snapshot");
    assert_eq!(
        (latest.bytes, latest.files, latest.total_bytes),
        (275, 3, Some(275))
    );
}

#[test]
fn default_budget_lists_one_hundred_thousand_independent_entries() {
    let root = tempfile::tempdir().expect("large metadata fixture root");
    let path = root.path().join("100k.zip");
    let result = std::process::Command::new("python")
        .arg(fixture("generate.py"))
        .arg(&path)
        .arg("100000")
        .output()
        .expect("independent Python fixture generator");
    assert!(result.status.success(), "fixture generation must succeed");
    let snapshot = ArchiveEngine::default()
        .open(OpenRequest::new(&path), &OperationControl::default())
        .expect("100k entries under default finite budget");
    assert_eq!(snapshot.entries().len(), 100_000);
    let engine = ArchiveEngine::with_limits(nxtools_archive::ResourceLimits {
        max_working_bytes: 256 * 1024,
        ..Default::default()
    });
    assert_eq!(
        engine.execute(ArchiveRequest::test(snapshot), &OperationControl::default()),
        Err(nxtools_archive::ArchiveError::ResourceLimit),
        "extent validation must account working allocation"
    );
}

#[test]
fn independent_zip_aliases_use_signature_and_misleading_inputs_keep_typed_errors() {
    use nxtools_archive::ArchiveError;
    let owned = tempfile::tempdir().expect("small alias fixtures");
    let engine = ArchiveEngine::default();
    let control = OperationControl::default();
    for extension in ["bin", "jar", "apk"] {
        let path = owned.path().join(format!("same.{extension}"));
        std::fs::copy(fixture("stored.zip"), &path).expect("independent Python ZIP unchanged");
        let snapshot = engine
            .open(OpenRequest::new(&path), &control)
            .expect("signature wins");
        assert_eq!(snapshot.format(), ArchiveFormat::Zip);
        assert_eq!(snapshot.entries().len(), 4);
        let outcome = engine
            .execute(ArchiveRequest::test(snapshot), &control)
            .expect("alias reads existing payloads");
        assert_eq!(
            (outcome.files_verified(), outcome.bytes_verified()),
            (3, 275)
        );
    }
    let random: Vec<u8> = (0u16..511).map(|i| ((i * 73 + 19) % 251) as u8).collect();
    let mut damaged_signature = std::fs::read(fixture("stored.zip")).expect("independent ZIP");
    damaged_signature[0] ^= 0x80;
    for (name, bytes, expected) in [
        (
            "random.bin",
            random.clone(),
            ArchiveError::UnsupportedFormat,
        ),
        (
            "damaged.zip",
            damaged_signature,
            ArchiveError::UnsupportedFormat,
        ),
        (
            "truncated.zip",
            b"PK\x03\x04bad".to_vec(),
            ArchiveError::CorruptArchive,
        ),
        (
            "random.tar.xz",
            random.clone(),
            ArchiveError::UnsupportedFormat,
        ),
        ("random.tar.gz", random, ArchiveError::UnsupportedFormat),
        (
            "gzip-is-not-tar.tar.gz",
            std::fs::read(fixture("independent.gz")).expect("independent gzip"),
            ArchiveError::CorruptArchive,
        ),
        (
            "xz-is-not-tar.tar.xz",
            std::fs::read(fixture("independent.xz")).expect("independent XZ"),
            ArchiveError::CorruptArchive,
        ),
    ] {
        let path = owned.path().join(name);
        std::fs::write(&path, bytes).expect("bounded invalid input");
        assert!(
            matches!(engine.open(OpenRequest::new(path), &control), Err(error) if error == expected),
            "typed rejection: {name}"
        );
    }
}
