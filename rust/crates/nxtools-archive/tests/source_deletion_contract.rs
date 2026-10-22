use nxtools_archive::{
    ArchiveEngine, ArchiveError, OpenRequest, OperationControl,
    output::{ArchiveOutput, ExtractRequest},
    planning::{ExtractionPlan, ExtractionSelection, TargetStrategy},
};
use uuid::Uuid;

#[derive(Default)]
struct Output {
    commits: usize,
    fail_directory: bool,
    fail_metadata: bool,
    fail_commit: bool,
    on_directory: Option<Box<dyn FnMut()>>,
}
impl ArchiveOutput for Output {
    fn ensure_directory(&mut self, _: &str) -> nxtools_archive::Result<()> {
        Ok(())
    }
    fn begin_file(&mut self, _: &str) -> nxtools_archive::Result<Uuid> {
        Ok(Uuid::new_v4())
    }
    fn write_file(&mut self, _: Uuid, _: &[u8]) -> nxtools_archive::Result<()> {
        Ok(())
    }
    fn set_file_metadata(
        &mut self,
        _: Uuid,
        _: nxtools_archive::metadata::OutputMetadata,
    ) -> nxtools_archive::Result<()> {
        if self.fail_metadata {
            Err(ArchiveError::IoFailure)
        } else {
            Ok(())
        }
    }
    fn finalize_directory_metadata(
        &mut self,
        _: &str,
        _: nxtools_archive::metadata::OutputMetadata,
    ) -> nxtools_archive::Result<()> {
        if let Some(mut callback) = self.on_directory.take() {
            callback();
        }
        if self.fail_directory {
            Err(ArchiveError::IoFailure)
        } else {
            Ok(())
        }
    }
    fn commit_file(&mut self, _: Uuid) -> nxtools_archive::Result<()> {
        if self.fail_commit {
            return Err(ArchiveError::IoFailure);
        }
        self.commits += 1;
        Ok(())
    }
    fn abort_file(&mut self, _: Uuid) {}
}
fn snapshot(name: &str, control: &OperationControl) -> nxtools_archive::ArchiveSnapshot {
    ArchiveEngine::default()
        .open(
            OpenRequest::new(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures")
                    .join(name),
            ),
            control,
        )
        .expect("independent archive")
}
#[test]
fn complete_real_extraction_is_job_bound_default_off_and_once_only() {
    let control = OperationControl::default();
    let source = snapshot("deflated.zip", &control);
    let plan = ExtractionPlan::build(&source, ExtractionSelection::All, TargetStrategy::Here)
        .expect("full plan");
    let mut output = Output::default();
    ArchiveEngine::default()
        .extract(ExtractRequest::new(source, plan), &control, &mut output)
        .expect("real decode/metadata/finalize");
    assert_eq!(output.commits, 3);
    assert_eq!(
        control.source_delete_eligibility(false).reason_code(),
        "consent_missing"
    );
    let grant = control.source_delete_eligibility(true);
    assert_eq!(grant.reason_code(), "eligible");
    assert_eq!(grant.operation_id(), control.operation_id());
    assert_eq!(
        control.source_delete_eligibility(true).reason_code(),
        "already_consumed"
    );
}

#[test]
fn test_only_after_extraction_cannot_reuse_prior_success() {
    let control = OperationControl::default();
    let source = snapshot("deflated.zip", &control);
    let plan = ExtractionPlan::build(&source, ExtractionSelection::All, TargetStrategy::Here)
        .expect("plan");
    ArchiveEngine::default()
        .extract(
            ExtractRequest::new(source.clone(), plan),
            &control,
            &mut Output::default(),
        )
        .expect("extract");
    ArchiveEngine::default()
        .execute(nxtools_archive::ArchiveRequest::test(source), &control)
        .expect("test is not extraction");
    assert_eq!(
        control.source_delete_eligibility(true).reason_code(),
        "lifecycle_incomplete"
    );
}

#[test]
fn selected_failure_cancel_and_directory_finalization_never_grant() {
    for mode in [
        "selected",
        "crc",
        "metadata",
        "directory",
        "commit",
        "cancel",
    ] {
        let control = OperationControl::default();
        let source = snapshot(
            if mode == "crc" {
                "bad-crc.zip"
            } else {
                "deflated.zip"
            },
            &control,
        );
        let selection = if mode == "selected" {
            ExtractionSelection::Entries(source.entries().iter().map(|entry| entry.id()).collect())
        } else {
            ExtractionSelection::All
        };
        let plan = ExtractionPlan::build(&source, selection, TargetStrategy::Here).expect("plan");
        let mut output = Output {
            fail_metadata: mode == "metadata",
            fail_directory: mode == "directory",
            fail_commit: mode == "commit",
            ..Default::default()
        };
        if mode == "cancel" {
            control.cancel();
        }
        let result = ArchiveEngine::default().extract(
            ExtractRequest::new(source, plan),
            &control,
            &mut output,
        );
        assert_eq!(result.is_ok(), mode == "selected", "{mode}");
        assert_eq!(
            control.source_delete_eligibility(true).reason_code(),
            if mode == "selected" {
                "selected_only"
            } else if mode == "cancel" {
                "cancelled"
            } else {
                "lifecycle_incomplete"
            },
            "{mode}"
        );
        if mode == "crc" {
            assert_eq!(output.commits, 2);
        }
        if mode == "directory" {
            assert_eq!(output.commits, 3);
        }
    }
}

#[test]
fn links_skipped_in_real_tar_never_qualify_and_source_change_is_refused() {
    let control = OperationControl::default();
    let source = snapshot("independent.tar", &control);
    let plan = ExtractionPlan::build(&source, ExtractionSelection::All, TargetStrategy::Here)
        .expect("TAR plan");
    assert!(plan.skipped_special() > 0, "independent TAR includes link");
    ArchiveEngine::default()
        .extract(
            ExtractRequest::new(source, plan),
            &control,
            &mut Output::default(),
        )
        .expect("ordinary files succeed");
    assert_eq!(
        control.source_delete_eligibility(true).reason_code(),
        "members_skipped"
    );
    let owned = tempfile::tempdir().expect("owned source");
    let path = owned.path().join("changed.zip");
    std::fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/deflated.zip"),
        &path,
    )
    .expect("fixture copy");
    let source = ArchiveEngine::default()
        .open(OpenRequest::new(&path), &control)
        .expect("snapshot");
    let plan = ExtractionPlan::build(&source, ExtractionSelection::All, TargetStrategy::Here)
        .expect("plan");
    std::fs::write(&path, b"externally replaced").expect("synthetic replacement");
    assert_eq!(
        ArchiveEngine::default().extract(
            ExtractRequest::new(source, plan),
            &control,
            &mut Output::default()
        ),
        Err(ArchiveError::SourceChanged)
    );
    assert_eq!(
        control.source_delete_eligibility(true).reason_code(),
        "lifecycle_incomplete"
    );
}

#[test]
fn concurrent_consumers_observe_only_one_real_extraction_grant() {
    let control = OperationControl::default();
    let source = snapshot("deflated.zip", &control);
    let plan = ExtractionPlan::build(&source, ExtractionSelection::All, TargetStrategy::Here)
        .expect("plan");
    ArchiveEngine::default()
        .extract(
            ExtractRequest::new(source, plan),
            &control,
            &mut Output::default(),
        )
        .expect("extract");
    let tasks: Vec<_> = (0..8)
        .map(|_| {
            let control = control.clone();
            std::thread::spawn(move || control.source_delete_eligibility(true).reason_code())
        })
        .collect();
    let results: Vec<_> = tasks
        .into_iter()
        .map(|task| task.join().expect("consumer"))
        .collect();
    assert_eq!(
        results
            .iter()
            .filter(|reason| **reason == "eligible")
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|reason| **reason == "already_consumed")
            .count(),
        7
    );
}

#[test]
fn late_extraction_finish_cannot_restore_facts_invalidated_by_reentrant_test() {
    let control = OperationControl::default();
    let source = snapshot("deflated.zip", &control);
    let plan = ExtractionPlan::build(&source, ExtractionSelection::All, TargetStrategy::Here)
        .expect("original full plan");
    let later_control = control.clone();
    let later_source = source.clone();
    let injected = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let observed = injected.clone();
    let mut output = Output {
        on_directory: Some(Box::new(move || {
            ArchiveEngine::default()
                .execute(
                    nxtools_archive::ArchiveRequest::test(later_source.clone()),
                    &later_control,
                )
                .expect("later real test");
            assert_eq!(
                later_control.source_delete_eligibility(true).reason_code(),
                "lifecycle_incomplete"
            );
            observed.store(true, std::sync::atomic::Ordering::SeqCst);
        })),
        ..Default::default()
    };
    ArchiveEngine::default()
        .extract(ExtractRequest::new(source, plan), &control, &mut output)
        .expect("earlier extraction still succeeds");
    assert!(
        injected.load(std::sync::atomic::Ordering::SeqCst),
        "real finalization callback reached"
    );
    assert_eq!(output.commits, 3);
    assert_eq!(
        control.source_delete_eligibility(true).reason_code(),
        "lifecycle_incomplete",
        "older extraction must not revive eligibility"
    );
}

#[test]
fn later_open_or_failed_extract_in_callback_owns_invalidation() {
    for mode in ["open_ok", "open_failed", "extract_plan", "extract_identity"] {
        let owned = tempfile::tempdir().expect("owned interleaving source");
        let path = owned.path().join("source.zip");
        std::fs::copy(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/deflated.zip"),
            &path,
        )
        .expect("owned copy");
        let control = OperationControl::default();
        let engine = ArchiveEngine::default();
        let source = engine
            .open(OpenRequest::new(&path), &control)
            .expect("original snapshot");
        let foreign = engine
            .open(OpenRequest::new(&path), &OperationControl::default())
            .expect("different snapshot session");
        let foreign_plan =
            ExtractionPlan::build(&foreign, ExtractionSelection::All, TargetStrategy::Here)
                .expect("foreign plan");
        let plan = ExtractionPlan::build(&source, ExtractionSelection::All, TargetStrategy::Here)
            .expect("original plan");
        let later_control = control.clone();
        let later_source = source.clone();
        let injected = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let observed = injected.clone();
        let mut foreign_plan = Some(foreign_plan);
        let mut output = Output {
            on_directory: Some(Box::new(move || {
                let engine = ArchiveEngine::default();
                match mode {
                    "open_ok" => {
                        engine
                            .open(OpenRequest::new(&path), &later_control)
                            .expect("later successful open");
                    }
                    "open_failed" => {
                        assert!(
                            engine
                                .open(
                                    OpenRequest::new(path.with_extension("missing")),
                                    &later_control
                                )
                                .is_err()
                        );
                    }
                    "extract_plan" => {
                        assert_eq!(
                            engine.extract(
                                ExtractRequest::new(
                                    later_source.clone(),
                                    foreign_plan.take().expect("one foreign plan")
                                ),
                                &later_control,
                                &mut Output::default()
                            ),
                            Err(ArchiveError::SourceChanged)
                        );
                    }
                    "extract_identity" => {
                        let later_plan = ExtractionPlan::build(
                            &later_source,
                            ExtractionSelection::All,
                            TargetStrategy::Here,
                        )
                        .expect("later plan");
                        std::fs::write(
                            &path,
                            b"changed after earlier prefinalization source check",
                        )
                        .expect("synthetic changed source");
                        assert_eq!(
                            engine.extract(
                                ExtractRequest::new(later_source.clone(), later_plan),
                                &later_control,
                                &mut Output::default()
                            ),
                            Err(ArchiveError::SourceChanged)
                        );
                    }
                    _ => unreachable!(),
                }
                assert_eq!(
                    later_control.source_delete_eligibility(true).reason_code(),
                    "lifecycle_incomplete",
                    "{mode}"
                );
                observed.store(true, std::sync::atomic::Ordering::SeqCst);
            })),
            ..Default::default()
        };
        engine
            .extract(ExtractRequest::new(source, plan), &control, &mut output)
            .expect("earlier lifecycle returned");
        assert!(injected.load(std::sync::atomic::Ordering::SeqCst));
        assert_eq!(output.commits, 3);
        assert_eq!(
            control.source_delete_eligibility(true).reason_code(),
            "lifecycle_incomplete",
            "older completion restored after {mode}"
        );
    }
}

#[test]
fn later_valid_extraction_wins_and_operation_consumption_survives_new_lifecycle() {
    let control = OperationControl::default();
    let source = snapshot("deflated.zip", &control);
    let plan = ExtractionPlan::build(
        &source,
        ExtractionSelection::Entries(source.entries().iter().map(|entry| entry.id()).collect()),
        TargetStrategy::Here,
    )
    .expect("earlier selected plan");
    let later_control = control.clone();
    let later_source = source.clone();
    let mut output = Output {
        on_directory: Some(Box::new(move || {
            let plan = ExtractionPlan::build(
                &later_source,
                ExtractionSelection::All,
                TargetStrategy::Here,
            )
            .expect("later full plan");
            ArchiveEngine::default()
                .extract(
                    ExtractRequest::new(later_source.clone(), plan),
                    &later_control,
                    &mut Output::default(),
                )
                .expect("later full extraction");
        })),
        ..Default::default()
    };
    ArchiveEngine::default()
        .extract(
            ExtractRequest::new(source.clone(), plan),
            &control,
            &mut output,
        )
        .expect("older selected lifecycle returned");
    assert_eq!(
        control.source_delete_eligibility(true).reason_code(),
        "eligible",
        "older selected completion overwrote newer full success"
    );
    let plan = ExtractionPlan::build(&source, ExtractionSelection::All, TargetStrategy::Here)
        .expect("sequential plan");
    ArchiveEngine::default()
        .extract(
            ExtractRequest::new(source.clone(), plan),
            &control,
            &mut Output::default(),
        )
        .expect("next sequential extraction");
    assert_eq!(
        control.source_delete_eligibility(true).reason_code(),
        "already_consumed",
        "begin reset operation consumption"
    );
    ArchiveEngine::default()
        .execute(nxtools_archive::ArchiveRequest::test(source), &control)
        .expect("later sequential test");
    control.cancel();
    assert_eq!(
        control.source_delete_eligibility(true).reason_code(),
        "already_consumed"
    );
}

#[test]
fn cancellation_during_finalization_cannot_be_reversed_by_old_finish() {
    let control = OperationControl::default();
    let source = snapshot("deflated.zip", &control);
    let plan = ExtractionPlan::build(&source, ExtractionSelection::All, TargetStrategy::Here)
        .expect("full plan");
    let later_control = control.clone();
    let mut output = Output {
        on_directory: Some(Box::new(move || {
            later_control.cancel();
        })),
        ..Default::default()
    };
    let _outcome =
        ArchiveEngine::default().extract(ExtractRequest::new(source, plan), &control, &mut output);
    assert_eq!(
        control.source_delete_eligibility(true).reason_code(),
        "cancelled"
    );
}
