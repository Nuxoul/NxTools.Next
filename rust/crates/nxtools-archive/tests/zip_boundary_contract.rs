use nxtools_archive::output::{ArchiveOutput, ExtractRequest};
use nxtools_archive::planning::{ExtractionPlan, ExtractionSelection, TargetStrategy};
use nxtools_archive::{
    ArchiveEngine, ArchiveError, ArchiveRequest, OpenRequest, OperationControl, Result,
};
use std::path::PathBuf;
use uuid::Uuid;
fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}
#[derive(Default)]
struct Output {
    directories: Vec<String>,
    files: usize,
}
impl ArchiveOutput for Output {
    fn ensure_directory(&mut self, path: &str) -> Result<()> {
        self.directories.push(path.into());
        Ok(())
    }
    fn begin_file(&mut self, _: &str) -> Result<Uuid> {
        self.files += 1;
        Ok(Uuid::new_v4())
    }
    fn write_file(&mut self, _: Uuid, _: &[u8]) -> Result<()> {
        Ok(())
    }
    fn finalize_directory_metadata(
        &mut self,
        _: &str,
        metadata: nxtools_archive::metadata::OutputMetadata,
    ) -> Result<()> {
        metadata.validate()
    }
    fn set_file_metadata(
        &mut self,
        _: Uuid,
        metadata: nxtools_archive::metadata::OutputMetadata,
    ) -> Result<()> {
        metadata.validate()
    }
    fn commit_file(&mut self, _: Uuid) -> Result<()> {
        Ok(())
    }
    fn abort_file(&mut self, _: Uuid) {}
}
#[test]
fn explicit_directory_payload_test_and_extract_agree_before_directory_creation() {
    let owned = tempfile::tempdir().expect("owned directories");
    for mode in ["normal", "corrupt", "truncated", "unsupported"] {
        let mut bytes =
            std::fs::read(fixture("independent-directory.zip")).expect("CPython directory");
        let central = bytes
            .windows(4)
            .position(|b| b == b"PK\x01\x02")
            .expect("central");
        let offset = 30 + u16::from_le_bytes([bytes[26], bytes[27]]) as usize;
        match mode {
            "corrupt" => bytes[offset] = 0xff,
            "truncated" => {
                bytes[18..22].copy_from_slice(&1u32.to_le_bytes());
                bytes[central + 20..central + 24].copy_from_slice(&1u32.to_le_bytes());
            }
            "unsupported" => {
                bytes[8] = 222;
                bytes[central + 10] = 222;
            }
            _ => {}
        }
        let path = owned.path().join("directory.zip");
        std::fs::write(&path, bytes).expect("owned mutation");
        let engine = ArchiveEngine::default();
        let control = OperationControl::default();
        let snapshot = engine
            .open(OpenRequest::new(path), &control)
            .expect("directory metadata");
        let result = engine.execute(ArchiveRequest::test(snapshot.clone()), &control);
        let expected = if mode == "unsupported" {
            ArchiveError::UnsupportedMethod
        } else {
            ArchiveError::CorruptArchive
        };
        if mode == "normal" {
            assert_eq!(
                result
                    .expect("valid compressed empty directory")
                    .files_verified(),
                1
            );
        } else {
            assert_eq!(result, Err(expected));
        }
        let plan = ExtractionPlan::build(&snapshot, ExtractionSelection::All, TargetStrategy::Here)
            .expect("plan");
        let mut output = Output::default();
        let result = engine.extract(
            ExtractRequest::new(snapshot.clone(), plan),
            &control,
            &mut output,
        );
        if mode == "normal" {
            assert_eq!(result.expect("valid directory output").files_committed, 1);
            assert_eq!(output.directories, ["folder"]);
        } else {
            assert_eq!(result, Err(expected), "{mode}");
            assert!(output.directories.is_empty());
            assert_eq!(output.files, 0);
        }
        // An unrelated unselected explicit directory is not decoded or materialized.
        let plan = ExtractionPlan::build(
            &snapshot,
            ExtractionSelection::Entries(vec![snapshot.entries()[1].id()]),
            TargetStrategy::Here,
        )
        .expect("selected file plan");
        let mut output = Output::default();
        assert_eq!(
            engine
                .extract(ExtractRequest::new(snapshot, plan), &control, &mut output)
                .expect("selected file only")
                .files_committed,
            1
        );
        assert!(output.directories.is_empty());
    }
}

fn with_comment(mut bytes: Vec<u8>, comment: &[u8]) -> Vec<u8> {
    let end = bytes.len() - 22;
    bytes[end + 20..end + 22].copy_from_slice(&(comment.len() as u16).to_le_bytes());
    bytes.extend(comment);
    bytes
}
#[test]
fn eocd_comment_raw_lossy_and_central_share_one_metadata_reservation() {
    use nxtools_archive::ResourceLimits;
    let owned = tempfile::tempdir().expect("owned comments");
    for (mode, source, comment, budget, success) in [
        ("empty", "empty.zip", Vec::new(), 1, true),
        ("ascii_under", "empty.zip", vec![b'a'; 65535], 1, false),
        (
            "ascii_fit",
            "empty.zip",
            vec![b'a'; 65535],
            65535 * 4 + 64,
            true,
        ),
        ("lossy_under", "empty.zip", vec![0xff; 1000], 3000, false),
        ("lossy_fit", "empty.zip", vec![0xff; 1000], 4064, true),
        (
            "combined_under",
            "independent-directory.zip",
            vec![b'a'; 1000],
            5000,
            false,
        ),
        (
            "combined_fit",
            "independent-directory.zip",
            vec![b'a'; 1000],
            16384,
            true,
        ),
    ] {
        let path = owned.path().join(format!("{mode}.zip"));
        std::fs::write(
            &path,
            with_comment(
                std::fs::read(fixture(source)).expect("independent ZIP"),
                &comment,
            ),
        )
        .expect("owned comment");
        let control = OperationControl::default();
        let baseline = ArchiveEngine::default()
            .open(OpenRequest::new(&path), &control)
            .expect("valid comment structure");
        let engine = ArchiveEngine::with_limits(ResourceLimits {
            max_metadata_bytes: budget,
            ..Default::default()
        });
        let opened = engine.open(OpenRequest::new(path), &control);
        if success {
            assert_eq!(
                opened.expect("admitted combined comment").comment(),
                String::from_utf8_lossy(&comment)
            );
            assert!(
                engine
                    .execute(ArchiveRequest::test(baseline), &control)
                    .is_ok()
            );
        } else {
            assert_eq!(
                opened.err(),
                Some(ArchiveError::ResourceLimit),
                "open {mode}"
            );
            assert_eq!(
                engine.execute(ArchiveRequest::test(baseline.clone()), &control),
                Err(ArchiveError::ResourceLimit),
                "execute {mode}"
            );
            let plan =
                ExtractionPlan::build(&baseline, ExtractionSelection::All, TargetStrategy::Here)
                    .expect("plan");
            let mut output = Output::default();
            assert_eq!(
                engine.extract(ExtractRequest::new(baseline, plan), &control, &mut output),
                Err(ArchiveError::ResourceLimit),
                "extract {mode}"
            );
            assert!(output.directories.is_empty());
            assert_eq!(output.files, 0);
        }
    }
}
