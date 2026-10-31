use nxtools_archive::output::{ArchiveOutput, ExtractRequest};
use nxtools_archive::planning::{ExtractionPlan, ExtractionSelection, TargetStrategy};
use nxtools_archive::{
    ArchiveEngine, ArchiveError, ArchiveFormat, ArchiveRequest, OpenRequest, OperationControl,
};
use std::{collections::BTreeMap, path::PathBuf};
use uuid::Uuid;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}
#[derive(Default)]
struct Output {
    partials: BTreeMap<Uuid, Vec<u8>>,
    committed: Vec<Vec<u8>>,
    aborted: usize,
    skip: bool,
    skipped: u64,
}
impl ArchiveOutput for Output {
    fn ensure_directory(&mut self, _: &str) -> nxtools_archive::Result<()> {
        Ok(())
    }
    fn begin_file(&mut self, _: &str) -> nxtools_archive::Result<Uuid> {
        let id = Uuid::new_v4();
        self.partials.insert(id, Vec::new());
        Ok(id)
    }
    fn begin_extracted_file(
        &mut self,
        path: &str,
        _: Option<u64>,
    ) -> nxtools_archive::Result<Option<Uuid>> {
        if self.skip {
            self.skipped += 1;
            Ok(None)
        } else {
            self.begin_file(path).map(Some)
        }
    }
    fn skipped_files(&self) -> u64 {
        self.skipped
    }
    fn write_file(&mut self, id: Uuid, data: &[u8]) -> nxtools_archive::Result<()> {
        assert!(data.len() <= 65536);
        self.partials
            .get_mut(&id)
            .expect("owned partial")
            .extend_from_slice(data);
        Ok(())
    }
    fn set_file_metadata(
        &mut self,
        token: Uuid,
        metadata: nxtools_archive::metadata::OutputMetadata,
    ) -> nxtools_archive::Result<()> {
        assert!(self.partials.contains_key(&token));
        metadata.validate()
    }
    fn finalize_directory_metadata(
        &mut self,
        _: &str,
        metadata: nxtools_archive::metadata::OutputMetadata,
    ) -> nxtools_archive::Result<()> {
        metadata.validate()
    }
    fn commit_file(&mut self, id: Uuid) -> nxtools_archive::Result<()> {
        self.committed
            .push(self.partials.remove(&id).expect("owned partial"));
        Ok(())
    }
    fn abort_file(&mut self, id: Uuid) {
        self.partials.remove(&id);
        self.aborted += 1;
    }
}
#[test]
fn independent_gzip_streams_list_unknown_size_and_verify_all_members_before_publish() {
    for name in ["independent.gz", "multi.gz"] {
        let engine = ArchiveEngine::default();
        let control = OperationControl::default();
        let snapshot = engine
            .open(OpenRequest::new(fixture(name)), &control)
            .expect("independent gzip open");
        assert_eq!(snapshot.format(), ArchiveFormat::Gzip);
        assert_eq!(snapshot.entries().len(), 1);
        assert_eq!(snapshot.entries()[0].size(), None);
        let expected = if name == "multi.gz" {
            b"first\nsecond\n".to_vec()
        } else {
            let mut value = "独立 gzip fixture\n".as_bytes().to_vec();
            value.extend(0u8..=255);
            value
        };
        let verified = engine
            .execute(ArchiveRequest::test(snapshot.clone()), &control)
            .expect("full gzip integrity");
        assert_eq!(verified.bytes_verified(), expected.len() as u64);
        let plan = ExtractionPlan::build(&snapshot, ExtractionSelection::All, TargetStrategy::Here)
            .expect("unknown size plan");
        let mut output = Output::default();
        let outcome = engine
            .extract(ExtractRequest::new(snapshot, plan), &control, &mut output)
            .expect("gzip extraction");
        assert_eq!(outcome.files_committed, 1);
        assert_eq!(output.committed, vec![expected]);
        assert!(output.partials.is_empty());
    }
}
#[test]
fn gzip_crc_truncation_actual_budget_and_oversized_headers_never_publish() {
    let owned = tempfile::tempdir().expect("owned fixture");
    for mode in ["crc", "truncated", "budget", "header"] {
        let mut bytes = std::fs::read(fixture("independent.gz")).expect("independent stream");
        if mode == "crc" {
            let index = bytes.len() - 8;
            bytes[index] ^= 1;
        }
        if mode == "truncated" {
            bytes.truncate(bytes.len() - 3);
        }
        if mode == "header" {
            bytes[3] |= 8;
            bytes.splice(10..10, std::iter::repeat_n(b'x', 70000));
        }
        let path = owned.path().join(format!("{mode}.gz"));
        std::fs::write(&path, bytes).expect("bounded mutation");
        let engine = ArchiveEngine::with_limits(nxtools_archive::ResourceLimits {
            max_entry_bytes: if mode == "budget" { 1 } else { 1 << 20 },
            ..Default::default()
        });
        let control = OperationControl::default();
        let mut output = Output::default();
        let result = engine
            .open(OpenRequest::new(path), &control)
            .and_then(|snapshot| {
                let plan = ExtractionPlan::build(
                    &snapshot,
                    ExtractionSelection::All,
                    TargetStrategy::Here,
                )?;
                engine.extract(ExtractRequest::new(snapshot, plan), &control, &mut output)
            });
        assert_eq!(
            result,
            Err(if mode == "budget" || mode == "header" {
                ArchiveError::ResourceLimit
            } else {
                ArchiveError::CorruptArchive
            }),
            "{mode}"
        );
        assert!(output.committed.is_empty());
        assert!(output.partials.is_empty());
    }
}

#[test]
fn independent_bzip2_zip_preserves_bounded_zip_validation_and_exact_payload() {
    let engine = ArchiveEngine::default();
    let control = OperationControl::default();
    let snapshot = engine
        .open(OpenRequest::new(fixture("independent-bzip2.zip")), &control)
        .expect("independent ZIP12 metadata");
    assert_eq!(snapshot.entries()[0].method(), 12);
    let verified = engine
        .execute(ArchiveRequest::test(snapshot.clone()), &control)
        .expect("ZIP12 integrity");
    let mut expected = "独立 gzip fixture\n".as_bytes().to_vec();
    expected.extend(0u8..=255);
    assert_eq!(verified.bytes_verified(), expected.len() as u64);
    let plan = ExtractionPlan::build(&snapshot, ExtractionSelection::All, TargetStrategy::Here)
        .expect("ZIP12 plan");
    let mut output = Output::default();
    engine
        .extract(ExtractRequest::new(snapshot, plan), &control, &mut output)
        .expect("ZIP12 output");
    assert_eq!(output.committed, vec![expected]);
    assert!(output.partials.is_empty());
}

#[test]
fn bzip2_zip_rejects_unfinished_streams_corruption_and_insufficient_work_before_publication() {
    let owned = tempfile::tempdir().expect("owned mutation");
    for mode in ["work", "corrupt", "unfinished"] {
        let mut bytes = std::fs::read(fixture("independent-bzip2.zip")).expect("independent ZIP12");
        let central = bytes
            .windows(4)
            .position(|value| value == b"PK\x01\x02")
            .expect("central");
        if mode == "corrupt" {
            bytes[70] ^= 0x80;
        }
        if mode == "unfinished" {
            let packed = u32::from_le_bytes(bytes[18..22].try_into().expect("packed")) - 4;
            bytes[18..22].copy_from_slice(&packed.to_le_bytes());
            bytes[central + 20..central + 24].copy_from_slice(&packed.to_le_bytes());
            bytes.drain(central - 4..central);
            let eocd = bytes.len() - 22;
            bytes[eocd + 16..eocd + 20].copy_from_slice(&((central - 4) as u32).to_le_bytes());
        }
        let path = owned.path().join(format!("{mode}.zip"));
        std::fs::write(&path, bytes).expect("bounded mutant");
        let engine = ArchiveEngine::with_limits(nxtools_archive::ResourceLimits {
            max_working_bytes: if mode == "work" { 1 << 20 } else { 16 << 20 },
            ..Default::default()
        });
        let control = OperationControl::default();
        let snapshot = engine
            .open(OpenRequest::new(path), &control)
            .expect("bounded metadata");
        let plan = ExtractionPlan::build(&snapshot, ExtractionSelection::All, TargetStrategy::Here)
            .expect("bounded plan");
        let mut output = Output::default();
        assert_eq!(
            engine.extract(ExtractRequest::new(snapshot, plan), &control, &mut output),
            Err(if mode == "work" {
                ArchiveError::ResourceLimit
            } else {
                ArchiveError::CorruptArchive
            }),
            "{mode}"
        );
        assert!(output.committed.is_empty());
        assert!(output.partials.is_empty());
    }
}

#[test]
fn independent_pax_tar_keeps_unicode_empty_directories_and_never_materializes_links() {
    let engine = ArchiveEngine::default();
    let control = OperationControl::default();
    let snapshot = engine
        .open(OpenRequest::new(fixture("independent.tar")), &control)
        .expect("independent PAX TAR");
    assert_eq!(snapshot.format(), ArchiveFormat::Tar);
    assert_eq!(snapshot.entries().len(), 3);
    assert_eq!(snapshot.entries()[0].display_path(), "资料/empty/");
    assert_eq!(
        snapshot.entries()[2].kind(),
        nxtools_archive::EntryKind::Link
    );
    assert!(
        snapshot.entries()[2]
            .warnings()
            .contains(&nxtools_archive::EntryWarning::LinkNotExtracted)
    );
    let verified = engine
        .execute(ArchiveRequest::test(snapshot.clone()), &control)
        .expect("full structural TAR test");
    let mut expected = "独立 gzip fixture\n".as_bytes().to_vec();
    expected.extend(0u8..=255);
    assert_eq!(verified.files_verified(), 1);
    assert_eq!(verified.bytes_verified(), expected.len() as u64);
    let plan = ExtractionPlan::build(&snapshot, ExtractionSelection::All, TargetStrategy::Here)
        .expect("TAR plan");
    assert!(!plan.permits_source_delete_request());
    assert!(plan.directories().contains(&"资料/empty".into()));
    let mut output = Output::default();
    let extracted = engine
        .extract(ExtractRequest::new(snapshot, plan), &control, &mut output)
        .expect("TAR extraction");
    assert_eq!(extracted.skipped_special, 1);
    assert_eq!(output.committed, vec![expected]);
}

#[test]
fn tar_bad_checksum_truncation_and_expansion_metadata_budget_have_typed_failures() {
    let owned = tempfile::tempdir().expect("owned TAR mutations");
    for mode in ["checksum", "truncated", "metadata"] {
        let mut bytes = std::fs::read(fixture("independent.tar")).expect("independent TAR");
        if mode == "checksum" {
            bytes[0] ^= 1;
        }
        if mode == "truncated" {
            bytes.truncate(2300);
        }
        let path = owned.path().join(format!("{mode}.tar"));
        std::fs::write(&path, bytes).expect("bounded mutation");
        let engine = ArchiveEngine::with_limits(nxtools_archive::ResourceLimits {
            max_metadata_bytes: if mode == "metadata" { 128 } else { 1 << 20 },
            ..Default::default()
        });
        let result = engine.open(OpenRequest::new(path), &OperationControl::default());
        assert_eq!(
            result.err(),
            Some(if mode == "metadata" {
                ArchiveError::ResourceLimit
            } else {
                ArchiveError::CorruptArchive
            }),
            "{mode}"
        );
    }
}

#[test]
fn tar_extension_and_path_safety_are_bounded_before_output() {
    let engine = ArchiveEngine::default();
    let control = OperationControl::default();
    for (name, expected) in [
        ("sparse.tar", ArchiveError::UnsupportedMethod),
        ("huge-pax.tar", ArchiveError::ResourceLimit),
        ("orphan-pax.tar", ArchiveError::CorruptArchive),
    ] {
        assert_eq!(
            engine.open(OpenRequest::new(fixture(name)), &control).err(),
            Some(expected),
            "{name}"
        );
    }
    for (name, expected) in [
        ("unsafe.tar", ArchiveError::UnsafePath),
        ("collision.tar", ArchiveError::OutputCollision),
    ] {
        let snapshot = engine
            .open(OpenRequest::new(fixture(name)), &control)
            .expect("unsafe metadata remains visible");
        assert_eq!(
            ExtractionPlan::build(&snapshot, ExtractionSelection::All, TargetStrategy::Here).err(),
            Some(expected),
            "{name}"
        );
    }
    let snapshot = engine
        .open(OpenRequest::new(fixture("gnu.tar")), &control)
        .expect("GNU longname");
    assert_eq!(
        snapshot.entries()[0].display_path(),
        format!("long/{}", "a".repeat(150))
    );
    assert_eq!(
        engine
            .execute(ArchiveRequest::test(snapshot), &control)
            .expect("GNU full test")
            .bytes_verified(),
        3
    );
}

#[test]
fn independent_tar_gzip_checks_outer_integrity_before_any_inner_output() {
    let owned = tempfile::tempdir().expect("owned compound fixture");
    for mode in ["normal", "outer_crc", "spool_budget"] {
        let mut bytes =
            std::fs::read(fixture("independent.tar.gz")).expect("Python compound stream");
        if mode == "outer_crc" {
            let index = bytes.len() - 8;
            bytes[index] ^= 1;
        }
        let path = owned.path().join(format!("{mode}.tar.gz"));
        std::fs::write(&path, bytes).expect("bounded compound copy");
        let engine = ArchiveEngine::with_limits(nxtools_archive::ResourceLimits {
            max_output_bytes: if mode == "spool_budget" { 512 } else { 1 << 20 },
            ..Default::default()
        });
        let control = OperationControl::default();
        let mut output = Output::default();
        let result = engine
            .open(OpenRequest::new(path), &control)
            .and_then(|snapshot| {
                assert_eq!(snapshot.format(), ArchiveFormat::TarGz);
                assert_eq!(snapshot.entries().len(), 3);
                let plan = ExtractionPlan::build(
                    &snapshot,
                    ExtractionSelection::All,
                    TargetStrategy::Here,
                )?;
                engine.extract(ExtractRequest::new(snapshot, plan), &control, &mut output)
            });
        if mode == "normal" {
            assert_eq!(result.expect("compound extraction").files_committed, 1);
            assert_eq!(output.committed.len(), 1);
        } else {
            assert_eq!(
                result,
                Err(if mode == "spool_budget" {
                    ArchiveError::ResourceLimit
                } else {
                    ArchiveError::CorruptArchive
                })
            );
            assert!(output.committed.is_empty());
            assert!(output.partials.is_empty());
        }
    }
}

#[test]
fn independent_bzip2_single_multi_and_tar_streams_have_truthful_metadata_and_full_integrity() {
    for (name, format, count) in [
        ("independent.bz2", ArchiveFormat::Bzip2, 1),
        ("multi.bz2", ArchiveFormat::Bzip2, 1),
        ("independent.tar.bz2", ArchiveFormat::TarBz2, 3),
    ] {
        let engine = ArchiveEngine::default();
        let control = OperationControl::default();
        let snapshot = engine
            .open(OpenRequest::new(fixture(name)), &control)
            .expect("independent BZip2 format");
        assert_eq!(snapshot.format(), format);
        assert_eq!(snapshot.entries().len(), count);
        if format == ArchiveFormat::Bzip2 {
            assert_eq!(snapshot.entries()[0].size(), None);
        }
        let plan = ExtractionPlan::build(&snapshot, ExtractionSelection::All, TargetStrategy::Here)
            .expect("BZip2 plan");
        let mut output = Output::default();
        engine
            .extract(ExtractRequest::new(snapshot, plan), &control, &mut output)
            .expect("BZip2 full integrity/output");
        let expected = if name == "multi.bz2" {
            b"first\nsecond\n".to_vec()
        } else {
            let mut value = "独立 gzip fixture\n".as_bytes().to_vec();
            value.extend(0u8..=255);
            value
        };
        assert_eq!(output.committed, vec![expected]);
        assert!(output.partials.is_empty());
    }
}

#[test]
fn bzip2_stream_damage_truncation_garbage_and_budgets_never_publish() {
    let owned = tempfile::tempdir().expect("owned stream mutants");
    for mode in ["corrupt", "truncated", "garbage", "work", "output"] {
        let mut bytes = std::fs::read(fixture("independent.bz2")).expect("independent stream");
        if mode == "corrupt" {
            bytes[10] ^= 1;
        }
        if mode == "truncated" {
            bytes.truncate(bytes.len() - 4);
        }
        if mode == "garbage" {
            bytes.extend_from_slice(b"garbage");
        }
        let path = owned.path().join(format!("{mode}.bz2"));
        std::fs::write(&path, bytes).expect("bounded mutation");
        let engine = ArchiveEngine::with_limits(nxtools_archive::ResourceLimits {
            max_working_bytes: if mode == "work" { 1 << 20 } else { 16 << 20 },
            max_output_bytes: if mode == "output" { 1 } else { 1 << 20 },
            ..Default::default()
        });
        let control = OperationControl::default();
        let mut output = Output::default();
        let result = engine
            .open(OpenRequest::new(path), &control)
            .and_then(|snapshot| {
                let plan = ExtractionPlan::build(
                    &snapshot,
                    ExtractionSelection::All,
                    TargetStrategy::Here,
                )?;
                engine.extract(ExtractRequest::new(snapshot, plan), &control, &mut output)
            });
        assert_eq!(
            result,
            Err(if mode == "work" || mode == "output" {
                ArchiveError::ResourceLimit
            } else {
                ArchiveError::CorruptArchive
            }),
            "{mode}"
        );
        assert!(output.committed.is_empty());
        assert!(output.partials.is_empty());
    }
}

#[test]
fn independent_xz_single_multi_and_tar_streams_require_complete_outer_integrity() {
    for (name, format) in [
        ("independent.xz", ArchiveFormat::Xz),
        ("multi.xz", ArchiveFormat::Xz),
        ("independent.tar.xz", ArchiveFormat::TarXz),
    ] {
        let engine = ArchiveEngine::default();
        let control = OperationControl::default();
        let snapshot = engine
            .open(OpenRequest::new(fixture(name)), &control)
            .expect("independent XZ metadata");
        assert_eq!(snapshot.format(), format);
        if format == ArchiveFormat::Xz {
            assert_eq!(snapshot.entries()[0].size(), None);
        }
        let plan = ExtractionPlan::build(&snapshot, ExtractionSelection::All, TargetStrategy::Here)
            .expect("XZ plan");
        let mut output = Output::default();
        engine
            .extract(ExtractRequest::new(snapshot, plan), &control, &mut output)
            .expect("XZ integrity/native capability");
        let expected = if name == "multi.xz" {
            b"first\nsecond\n".to_vec()
        } else {
            let mut value = "独立 gzip fixture\n".as_bytes().to_vec();
            value.extend(0u8..=255);
            value
        };
        assert_eq!(output.committed, vec![expected]);
        assert!(output.partials.is_empty());
    }
}

#[test]
fn tar_skip_discards_payload_without_partial_and_reports_members_skipped() {
    let control = OperationControl::default();
    let engine = ArchiveEngine::default();
    let snapshot = engine
        .open(OpenRequest::new(fixture("independent.tar")), &control)
        .expect("independent TAR");
    let plan = ExtractionPlan::build(&snapshot, ExtractionSelection::All, TargetStrategy::Here)
        .expect("safe original plan");
    let mut output = Output {
        skip: true,
        ..Default::default()
    };
    let result = engine
        .extract(ExtractRequest::new(snapshot, plan), &control, &mut output)
        .expect("TAR completed with Skip");
    assert_eq!(
        (
            result.files_committed,
            result.bytes_committed,
            output.skipped
        ),
        (0, 0, 1)
    );
    assert!(output.partials.is_empty() && output.committed.is_empty() && output.aborted == 0);
    assert_eq!(
        control.source_delete_eligibility(true).reason_code(),
        "members_skipped"
    );
}
