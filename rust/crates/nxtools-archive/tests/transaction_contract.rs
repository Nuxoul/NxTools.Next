use nxtools_archive::{
    ArchiveEngine, ArchiveError, OpenRequest, OperationControl,
    output::{ArchiveOutput, ExtractRequest},
    planning::{ExtractionPlan, ExtractionSelection, TargetStrategy},
};
use std::{collections::BTreeMap, path::PathBuf};
use uuid::Uuid;
#[derive(Default)]
struct MemoryOutput {
    partials: BTreeMap<Uuid, (String, Vec<u8>)>,
    committed: BTreeMap<String, Vec<u8>>,
    aborted: usize,
    commit_count: usize,
    fail_write: bool,
    cancel_after_write: Option<OperationControl>,
    skip_files: bool,
    skipped: u64,
}
impl ArchiveOutput for MemoryOutput {
    fn ensure_directory(&mut self, _: &str) -> nxtools_archive::Result<()> {
        Ok(())
    }
    fn begin_file(&mut self, path: &str) -> nxtools_archive::Result<Uuid> {
        let token = Uuid::new_v4();
        self.partials.insert(token, (path.into(), Vec::new()));
        Ok(token)
    }
    fn begin_extracted_file(
        &mut self,
        path: &str,
        _: Option<u64>,
    ) -> nxtools_archive::Result<Option<Uuid>> {
        if self.skip_files {
            self.skipped += 1;
            Ok(None)
        } else {
            self.begin_file(path).map(Some)
        }
    }
    fn skipped_files(&self) -> u64 {
        self.skipped
    }
    fn write_file(&mut self, token: Uuid, bytes: &[u8]) -> nxtools_archive::Result<()> {
        if self.fail_write {
            return Err(ArchiveError::IoFailure);
        }
        self.partials
            .get_mut(&token)
            .expect("owned token")
            .1
            .extend_from_slice(bytes);
        if let Some(control) = &self.cancel_after_write {
            control.cancel();
        }
        Ok(())
    }
    fn finalize_directory_metadata(
        &mut self,
        _: &str,
        metadata: nxtools_archive::metadata::OutputMetadata,
    ) -> nxtools_archive::Result<()> {
        metadata.validate()
    }
    fn set_file_metadata(
        &mut self,
        token: Uuid,
        metadata: nxtools_archive::metadata::OutputMetadata,
    ) -> nxtools_archive::Result<()> {
        assert!(self.partials.contains_key(&token));
        metadata.validate()
    }
    fn commit_file(&mut self, token: Uuid) -> nxtools_archive::Result<()> {
        let (path, bytes) = self.partials.remove(&token).expect("owned token");
        if self.committed.contains_key(&path) {
            return Err(ArchiveError::OutputCollision);
        }
        self.committed.insert(path, bytes);
        self.commit_count += 1;
        Ok(())
    }
    fn abort_file(&mut self, token: Uuid) {
        self.partials.remove(&token);
        self.aborted += 1;
    }
}
fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}
fn request(name: &str) -> ExtractRequest {
    let snapshot = ArchiveEngine::default()
        .open(
            OpenRequest::new(fixture(name)),
            &OperationControl::default(),
        )
        .expect("independent metadata");
    let plan = ExtractionPlan::build(&snapshot, ExtractionSelection::All, TargetStrategy::Here)
        .expect("validated output plan");
    ExtractRequest::new(snapshot, plan)
}
#[test]
fn independent_zip_extracts_exact_bytes_and_commits_only_verified_files() {
    let mut output = MemoryOutput::default();
    let result = ArchiveEngine::default()
        .extract(
            request("deflated.zip"),
            &OperationControl::default(),
            &mut output,
        )
        .expect("stream to output capability");
    assert_eq!((result.files_committed, result.bytes_committed), (3, 275));
    assert_eq!(
        output.committed["资料/说明.txt"],
        "你好，压缩包\n".as_bytes()
    );
    assert_eq!(
        output.committed["bytes.bin"],
        (0u8..=255).collect::<Vec<_>>()
    );
    assert!(output.partials.is_empty());
    assert_eq!(output.aborted, 0);
}
#[test]
fn corrupt_current_file_never_commits_and_previous_verified_files_are_not_rolled_back() {
    let mut output = MemoryOutput::default();
    assert_eq!(
        ArchiveEngine::default().extract(
            request("bad-crc.zip"),
            &OperationControl::default(),
            &mut output
        ),
        Err(ArchiveError::CorruptArchive)
    );
    assert_eq!(
        output.commit_count, 2,
        "prior per-file commits are not a global rollback"
    );
    assert!(!output.committed.contains_key("bytes.bin"));
    assert!(output.partials.is_empty());
    assert_eq!(output.aborted, 1);
}
#[test]
fn output_failure_aborts_owned_partial_and_cancel_before_open_has_no_effects() {
    let mut output = MemoryOutput {
        fail_write: true,
        ..Default::default()
    };
    assert_eq!(
        ArchiveEngine::default().extract(
            request("stored.zip"),
            &OperationControl::default(),
            &mut output
        ),
        Err(ArchiveError::IoFailure)
    );
    assert!(output.partials.is_empty() && output.committed.is_empty());
    assert_eq!(output.aborted, 1);
    let control = OperationControl::default();
    control.cancel();
    let mut output = MemoryOutput::default();
    assert_eq!(
        ArchiveEngine::default().extract(request("stored.zip"), &control, &mut output),
        Err(ArchiveError::Cancelled)
    );
    assert_eq!((output.commit_count, output.aborted), (0, 0));
}

#[test]
fn foreign_plan_is_rejected_before_output_and_cancel_after_last_write_never_commits() {
    let engine = ArchiveEngine::default();
    let control = OperationControl::default();
    let first = engine
        .open(OpenRequest::new(fixture("stored.zip")), &control)
        .expect("first session");
    let second = engine
        .open(OpenRequest::new(fixture("stored.zip")), &control)
        .expect("second session");
    let plan = ExtractionPlan::build(&first, ExtractionSelection::All, TargetStrategy::Here)
        .expect("first plan");
    let mut output = MemoryOutput::default();
    assert_eq!(
        engine.extract(ExtractRequest::new(second, plan), &control, &mut output),
        Err(ArchiveError::SourceChanged)
    );
    assert!(output.partials.is_empty() && output.committed.is_empty());
    let control = OperationControl::default();
    let mut output = MemoryOutput {
        cancel_after_write: Some(control.clone()),
        ..Default::default()
    };
    assert_eq!(
        engine.extract(request("stored.zip"), &control, &mut output),
        Err(ArchiveError::Cancelled)
    );
    assert!(output.partials.is_empty() && output.committed.is_empty());
    assert_eq!(output.aborted, 1);
}

#[test]
fn skipped_zip_files_have_no_partial_or_commit_and_cannot_qualify_source_deletion() {
    let mut output = MemoryOutput {
        skip_files: true,
        ..Default::default()
    };
    let control = OperationControl::default();
    let outcome = ArchiveEngine::default()
        .extract(request("deflated.zip"), &control, &mut output)
        .expect("completed with skipped members");
    assert_eq!(
        (
            outcome.files_committed,
            outcome.bytes_committed,
            output.skipped
        ),
        (0, 0, 3)
    );
    assert!(output.partials.is_empty() && output.committed.is_empty());
    assert_eq!(
        control.source_delete_eligibility(true).reason_code(),
        "members_skipped"
    );
}
