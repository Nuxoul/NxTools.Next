use nxtools_archive::{
    ArchiveEngine, ArchiveError, ArchiveRequest, ArchiveWorkObservation, OpenRequest,
    OperationControl,
    output::{ArchiveOutput, ExtractRequest},
    planning::{ExtractionPlan, ExtractionSelection, TargetStrategy},
    zip_edit::ZipEdit,
};
use serde::Serialize;
use std::{
    fs::{self, File},
    io::{Read, Seek, Write},
    path::Path,
    time::Instant,
};
use uuid::Uuid;

// Independent classic ZIP records, including signed data descriptors and separate UT extras.
fn raw_zip(count: usize, size: usize, first: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut central = Vec::new();
    for index in first..first + count {
        let name = format!("folder/member-{index:05}.bin");
        let payload = vec![(index % 251) as u8; size];
        let crc = crc32fast::hash(&payload);
        let offset = bytes.len() as u32;
        let flags = 0x800u16 | if index % 2 == 0 { 8 } else { 0 };
        let extra = [0x55, 0x54, 5, 0, 1, 0, 0xe1, 0x0b, 0x5e];
        bytes.extend_from_slice(b"PK\x03\x04");
        bytes.extend_from_slice(&20u16.to_le_bytes());
        bytes.extend_from_slice(&flags.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&0x21u16.to_le_bytes());
        for field in [crc, size as u32, size as u32] {
            bytes.extend_from_slice(&(if flags & 8 != 0 { 0 } else { field }).to_le_bytes());
        }
        bytes.extend_from_slice(&(name.len() as u16).to_le_bytes());
        bytes.extend_from_slice(&(extra.len() as u16).to_le_bytes());
        bytes.extend_from_slice(name.as_bytes());
        bytes.extend_from_slice(&extra);
        bytes.extend_from_slice(&payload);
        if flags & 8 != 0 {
            bytes.extend_from_slice(b"PK\x07\x08");
            for field in [crc, size as u32, size as u32] {
                bytes.extend_from_slice(&field.to_le_bytes());
            }
        }
        let mut row = vec![0; 46];
        row[..4].copy_from_slice(b"PK\x01\x02");
        row[4..6].copy_from_slice(&20u16.to_le_bytes());
        row[6..8].copy_from_slice(&20u16.to_le_bytes());
        row[8..10].copy_from_slice(&flags.to_le_bytes());
        row[14..16].copy_from_slice(&0x21u16.to_le_bytes());
        row[16..20].copy_from_slice(&crc.to_le_bytes());
        row[20..24].copy_from_slice(&(size as u32).to_le_bytes());
        row[24..28].copy_from_slice(&(size as u32).to_le_bytes());
        row[28..30].copy_from_slice(&(name.len() as u16).to_le_bytes());
        row[30..32].copy_from_slice(&(extra.len() as u16).to_le_bytes());
        row[42..46].copy_from_slice(&offset.to_le_bytes());
        row.extend_from_slice(name.as_bytes());
        row.extend_from_slice(&extra);
        central.extend_from_slice(&row);
    }
    let offset = bytes.len() as u32;
    let length = central.len() as u32;
    bytes.extend_from_slice(&central);
    bytes.extend_from_slice(b"PK\x05\x06");
    bytes.extend_from_slice(&[0; 4]);
    bytes.extend_from_slice(&(count as u16).to_le_bytes());
    bytes.extend_from_slice(&(count as u16).to_le_bytes());
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(&offset.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes
}
fn seed(path: &Path, bytes: &[u8]) {
    if path.exists() {
        assert_eq!(fs::read(path).expect("retained fixture"), bytes);
    } else {
        File::options()
            .write(true)
            .create_new(true)
            .open(path)
            .expect("fresh fixture")
            .write_all(bytes)
            .expect("seed");
    }
}
#[derive(Default)]
struct SelectedOutput {
    pending: Option<Uuid>,
    bytes: Vec<u8>,
    committed: bool,
}
impl ArchiveOutput for SelectedOutput {
    fn ensure_directory(&mut self, _: &str) -> nxtools_archive::Result<()> {
        Ok(())
    }
    fn begin_file(&mut self, _: &str) -> nxtools_archive::Result<Uuid> {
        assert!(self.pending.is_none());
        let id = Uuid::new_v4();
        self.pending = Some(id);
        Ok(id)
    }
    fn write_file(&mut self, id: Uuid, bytes: &[u8]) -> nxtools_archive::Result<()> {
        assert_eq!(self.pending, Some(id));
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    fn set_file_metadata(
        &mut self,
        id: Uuid,
        _: nxtools_archive::metadata::OutputMetadata,
    ) -> nxtools_archive::Result<()> {
        assert_eq!(self.pending, Some(id));
        Ok(())
    }
    fn commit_file(&mut self, id: Uuid) -> nxtools_archive::Result<()> {
        assert_eq!(self.pending.take(), Some(id));
        self.committed = true;
        Ok(())
    }
    fn abort_file(&mut self, id: Uuid) {
        assert_eq!(self.pending.take(), Some(id));
    }
}
#[derive(Serialize)]
struct Trial {
    operation: String,
    trial: usize,
    elapsed_us: u128,
    candidate_bytes: Option<u64>,
    work: ArchiveWorkObservation,
}
fn edit_case(engine: &ArchiveEngine<'_>, path: &Path, operation: &str, trial: usize) -> Trial {
    let snapshot = engine
        .open(OpenRequest::new(path), &OperationControl::default())
        .expect("source admission");
    let mut additions = tempfile::tempfile().expect("anonymous additions");
    let mut edits = Vec::new();
    match operation {
        "rename" => edits.push(ZipEdit::Rename {
            entry: snapshot.entries()[0].id(),
            name: "renamed.bin".into(),
        }),
        "comment" => edits.push(ZipEdit::Comment {
            text: "comment".into(),
        }),
        "delete" | "overwrite" => edits.push(ZipEdit::Delete {
            entries: vec![snapshot.entries()[0].id()],
        }),
        "add" => {}
        _ => panic!("known operation"),
    }
    let added = match operation {
        "overwrite" => Some(0),
        "add" => Some(512),
        _ => None,
    };
    if let Some(first) = added {
        additions
            .write_all(&raw_zip(1, 65536, first))
            .expect("addition bytes");
        additions.rewind().expect("addition rewind");
    }
    let control = OperationControl::default();
    let start = Instant::now();
    let candidate = engine
        .zip_edit_candidate(&snapshot, &edits, added.map(|_| &mut additions), &control)
        .expect("complete verified candidate");
    let elapsed_us = start.elapsed().as_micros();
    let candidate_bytes = Some(candidate.metadata().expect("candidate size").len());
    assert!(candidate_bytes.is_some_and(|length| length > 0));
    drop(candidate);
    Trial {
        operation: operation.into(),
        trial,
        elapsed_us,
        candidate_bytes,
        work: control.work_observation(),
    }
}
#[test]
fn opt05_zip_measure_original_engine_workflows() {
    let temporary = tempfile::tempdir().expect("owned root");
    let configured = std::env::var_os("NXTOOLS_OPT05_ZIP_ROOT").map(std::path::PathBuf::from);
    let root = configured.as_deref().unwrap_or(temporary.path());
    if configured.is_some() {
        assert_eq!(
            fs::read(root.join("owned-opt05-zip")).expect("owned marker"),
            b"owned OPT05 ZIP fixture\n"
        );
    }
    let path = root.join("payload.zip");
    let original = raw_zip(128, 65536, 0);
    seed(&path, &original);
    let metadata = root.join("metadata.zip");
    seed(&metadata, &raw_zip(1024, 8, 0));
    let engine = ArchiveEngine::default();
    let mut trials = Vec::new();
    for trial in 1..=3 {
        let control = OperationControl::default();
        let start = Instant::now();
        let snapshot = engine
            .open(OpenRequest::new(&metadata), &control)
            .expect("metadata open");
        assert_eq!(snapshot.entries().len(), 1024);
        trials.push(Trial {
            operation: "metadata_open".into(),
            trial,
            elapsed_us: start.elapsed().as_micros(),
            candidate_bytes: None,
            work: control.work_observation(),
        });
        let control = OperationControl::default();
        let start = Instant::now();
        let tested = engine
            .execute(ArchiveRequest::test(snapshot.clone()), &control)
            .expect("metadata Test");
        assert_eq!(tested.files_verified(), 1024);
        assert_eq!(tested.bytes_verified(), 8192);
        trials.push(Trial {
            operation: "metadata_test".into(),
            trial,
            elapsed_us: start.elapsed().as_micros(),
            candidate_bytes: None,
            work: control.work_observation(),
        });
        let plan = ExtractionPlan::build(
            &snapshot,
            ExtractionSelection::Entries(vec![snapshot.entries()[17].id()]),
            TargetStrategy::Here,
        )
        .expect("selected original ordinal");
        let control = OperationControl::default();
        let start = Instant::now();
        let mut output = SelectedOutput::default();
        let result = engine
            .extract(ExtractRequest::new(snapshot, plan), &control, &mut output)
            .expect("selected extraction");
        assert_eq!(result.files_committed, 1);
        assert!(output.committed && output.pending.is_none());
        assert_eq!(output.bytes, vec![17; 8]);
        trials.push(Trial {
            operation: "metadata_extract_one".into(),
            trial,
            elapsed_us: start.elapsed().as_micros(),
            candidate_bytes: None,
            work: control.work_observation(),
        });
        for operation in ["rename", "comment", "add", "overwrite", "delete"] {
            trials.push(edit_case(&engine, &path, operation, trial));
        }
    }
    assert_eq!(fs::read(&path).expect("source retained"), original);
    if let Some(report) = std::env::var_os("NXTOOLS_OPT05_ZIP_REPORT") {
        let mut file = File::options()
            .write(true)
            .create_new(true)
            .open(report)
            .expect("new evidence, never overwrite");
        serde_json::to_writer_pretty(&mut file, &trials).expect("complete trials");
        file.write_all(b"\n").expect("report end");
    }
    for row in &trials {
        println!(
            "OPT05 {} trial={} us={} local={} descriptor={} record_seeks={} source={} addition={} candidate={}",
            row.operation,
            row.trial,
            row.elapsed_us,
            row.work.zip_local_checks,
            row.work.zip_descriptor_checks,
            row.work.zip_record_seeks,
            row.work.zip_source_payload_decodes,
            row.work.zip_addition_payload_decodes,
            row.work.zip_candidate_payload_decodes
        );
    }
}
#[test]
fn opt05_zip_reuses_validated_records_and_payload_responsibility() {
    let root = tempfile::tempdir().expect("owned root");
    let path = root.path().join("source.zip");
    seed(&path, &raw_zip(128, 65536, 0));
    let engine = ArchiveEngine::default();
    let mut failures = Vec::new();
    for operation in ["rename", "comment", "add", "overwrite", "delete"] {
        let row = edit_case(&engine, &path, operation, 1);
        let removed = u64::from(matches!(operation, "delete" | "overwrite"));
        let added = u64::from(matches!(operation, "add" | "overwrite"));
        let final_count = 128 - removed + added;
        if row.work.zip_source_payload_decodes != removed
            || row.work.zip_addition_payload_decodes != 0
            || row.work.zip_candidate_payload_decodes != final_count
            || row.work.zip_local_checks != 128 + added + final_count
            || row.work.zip_descriptor_checks != 128 + added + final_count
        {
            failures.push(format!(
                "{operation}: local={} descriptor={} source={} addition={} final={}",
                row.work.zip_local_checks,
                row.work.zip_descriptor_checks,
                row.work.zip_source_payload_decodes,
                row.work.zip_addition_payload_decodes,
                row.work.zip_candidate_payload_decodes
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "original ZIP attempt must reuse structural facts and verify payload at its owning source/final stage: {failures:?}"
    );
}
#[test]
fn opt05_zip_test_and_extract_use_one_structural_plan() {
    let root = tempfile::tempdir().expect("owned root");
    let path = root.path().join("source.zip");
    seed(&path, &raw_zip(16, 32, 0));
    let engine = ArchiveEngine::default();
    let snapshot = engine
        .open(OpenRequest::new(&path), &OperationControl::default())
        .expect("catalog");
    let control = OperationControl::default();
    engine
        .execute(ArchiveRequest::test(snapshot.clone()), &control)
        .expect("Test");
    assert_eq!(
        control.work_observation().zip_local_checks,
        16,
        "Test reuses admitted local facts"
    );
    let control = OperationControl::default();
    let plan = ExtractionPlan::build(
        &snapshot,
        ExtractionSelection::Entries(vec![snapshot.entries()[1].id()]),
        TargetStrategy::Here,
    )
    .expect("plan");
    let mut output = SelectedOutput::default();
    engine
        .extract(ExtractRequest::new(snapshot, plan), &control, &mut output)
        .expect("extract");
    assert_eq!(
        control.work_observation().zip_local_checks,
        16,
        "extract reuses admitted local facts"
    );
    assert_eq!(output.bytes, vec![1; 32]);
}
#[test]
fn opt05_zip_dropped_and_retained_corruption_and_bad_additions_refuse() {
    let root = tempfile::tempdir().expect("owned root");
    let path = root.path().join("source.zip");
    let engine = ArchiveEngine::default();
    for removed in [false, true] {
        let mut bytes = raw_zip(2, 64, 0);
        let data = 30 + "folder/member-00000.bin".len() + 9;
        bytes[data] ^= 1;
        fs::write(&path, bytes).expect("damaged source");
        let snapshot = engine
            .open(OpenRequest::new(&path), &OperationControl::default())
            .expect("structurally readable damaged payload");
        let edits = if removed {
            vec![ZipEdit::Delete {
                entries: vec![snapshot.entries()[0].id()],
            }]
        } else {
            vec![ZipEdit::Comment { text: "x".into() }]
        };
        assert!(matches!(
            engine.zip_edit_candidate(&snapshot, &edits, None, &OperationControl::default()),
            Err(ArchiveError::CorruptArchive)
        ));
    }
    fs::write(&path, raw_zip(2, 64, 0)).expect("valid source");
    let snapshot = engine
        .open(OpenRequest::new(&path), &OperationControl::default())
        .expect("catalog");
    for kind in ["crc", "truncated", "local_name", "length"] {
        let mut bytes = raw_zip(1, 64, 512);
        let data = 30 + "folder/member-00512.bin".len() + 9;
        match kind {
            "crc" => bytes[data] ^= 1,
            "truncated" => {
                bytes.truncate(bytes.len() - 1);
            }
            "local_name" => bytes[30] ^= 1,
            "length" => {
                let central = bytes
                    .windows(4)
                    .position(|v| v == b"PK\x01\x02")
                    .expect("central");
                bytes[central + 24..central + 28].copy_from_slice(&65u32.to_le_bytes());
            }
            _ => unreachable!(),
        }
        let mut added = tempfile::tempfile().expect("addition");
        added.write_all(&bytes).expect("bad addition");
        added.rewind().expect("rewind");
        assert!(
            engine
                .zip_edit_candidate(
                    &snapshot,
                    &[],
                    Some(&mut added),
                    &OperationControl::default()
                )
                .is_err(),
            "bad additions {kind} must not return candidate"
        );
    }
    let mut reader = File::open(&path).expect("unchanged source");
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).expect("source");
    assert_eq!(bytes, raw_zip(2, 64, 0));
}

#[test]
fn opt05_zip_plan_and_encrypted_codec_share_working_budget() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/independent-zipcrypto.zip");
    let snapshot = ArchiveEngine::default()
        .open(OpenRequest::new(path), &OperationControl::default())
        .expect("independent encrypted catalog");
    for (working, admitted) in [(1 << 20, false), ((1 << 20) + 4096, true)] {
        let engine = ArchiveEngine::with_limits(nxtools_archive::ResourceLimits {
            max_working_bytes: working,
            ..Default::default()
        })
        .with_password("synthetic-private-ZIP-secret");
        let result = engine.execute(
            ArchiveRequest::test(snapshot.clone()),
            &OperationControl::default(),
        );
        if admitted {
            assert!(
                result.is_ok(),
                "codec plus ordinal plan fit together: {result:?}"
            );
        } else {
            assert_eq!(
                result,
                Err(ArchiveError::ResourceLimit),
                "the plan remains live during encrypted decoding"
            );
        }
    }
}

#[test]
fn opt05_zip_edit_reserves_source_snapshot_during_candidate_admission() {
    let root = tempfile::tempdir().expect("owned root");
    let path = root.path().join("source.zip");
    seed(&path, &raw_zip(128, 8, 0));
    let snapshot = ArchiveEngine::default()
        .open(OpenRequest::new(&path), &OperationControl::default())
        .expect("source catalog");
    for (metadata, admitted) in [(400_000, false), (1 << 20, true)] {
        let engine = ArchiveEngine::with_limits(nxtools_archive::ResourceLimits {
            max_metadata_bytes: metadata,
            ..Default::default()
        });
        let mut added = tempfile::tempfile().expect("anonymous additions");
        added.write_all(&raw_zip(128, 8, 512)).expect("additions");
        added.rewind().expect("rewind");
        let result = engine.zip_edit_candidate(
            &snapshot,
            &[],
            Some(&mut added),
            &OperationControl::default(),
        );
        if admitted {
            assert!(
                result.is_ok(),
                "source snapshot and candidate plan fit together"
            );
        } else {
            assert!(
                matches!(result, Err(ArchiveError::ResourceLimit)),
                "simultaneously retained source metadata must be reserved"
            );
        }
    }
    assert_eq!(fs::read(&path).expect("source"), raw_zip(128, 8, 0));
}

#[test]
fn opt05_zip_attribute_directory_rename_uses_component_boundary() {
    let root = tempfile::tempdir().expect("owned root");
    let path = root.path().join("attribute-directory.zip");
    let mut writer = zip::ZipWriter::new(File::create(&path).expect("source"));
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, payload) in [("a", ""), ("a/file", "child"), ("ab/file", "peer")] {
        writer.start_file(name, options).expect("entry");
        writer.write_all(payload.as_bytes()).expect("payload");
    }
    writer.finish().expect("complete ZIP");
    let mut original = fs::read(&path).expect("source records");
    let central = original
        .windows(4)
        .position(|bytes| bytes == b"PK\x01\x02")
        .expect("first central record");
    original[central + 38..central + 42]
        .copy_from_slice(&((0o40755u32 << 16) | 0x10).to_le_bytes());
    fs::write(&path, &original).expect("explicit attribute directory without name suffix");
    let engine = ArchiveEngine::default();
    let snapshot = engine
        .open(OpenRequest::new(&path), &OperationControl::default())
        .expect("supported directory");
    assert_eq!(
        snapshot.entries()[0].kind(),
        nxtools_archive::EntryKind::Directory
    );
    let candidate = engine
        .zip_edit_candidate(
            &snapshot,
            &[ZipEdit::Rename {
                entry: snapshot.entries()[0].id(),
                name: "b/".into(),
            }],
            None,
            &OperationControl::default(),
        )
        .expect("component-ended directory rename");
    let mut decoded = zip::ZipArchive::new(candidate).expect("independent candidate catalog");
    assert!(decoded.by_name("b/").is_ok());
    let mut child = String::new();
    decoded
        .by_name("b/file")
        .expect("child")
        .read_to_string(&mut child)
        .expect("child payload");
    assert_eq!(child, "child");
    let mut peer = String::new();
    decoded
        .by_name("ab/file")
        .expect("unrelated peer")
        .read_to_string(&mut peer)
        .expect("peer payload");
    assert_eq!(peer, "peer");
    assert_eq!(decoded.len(), 3);
    assert_eq!(fs::read(&path).expect("unchanged source"), original);
}
