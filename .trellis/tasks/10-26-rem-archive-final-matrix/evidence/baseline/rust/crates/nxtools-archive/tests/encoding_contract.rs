use nxtools_archive::{ArchiveEngine, EntryWarning, NameEncoding, OpenRequest, OperationControl};
use std::{path::PathBuf, process::Command};

fn fixtures() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("owned tiny ZIPs");
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/zip_encoding.py");
    assert!(
        Command::new("python")
            .arg(script)
            .arg(root.path())
            .status()
            .expect("Python fixture producer")
            .success()
    );
    root
}

#[test]
fn auto_uses_script_evidence_not_decode_success_and_keeps_real_ambiguity_manual() {
    let root = fixtures();
    let engine = ArchiveEngine::default();
    for (fixture, expected) in [
        ("auto-gbk", "中文文件资料.txt"),
        ("auto-sjis", "日本語のファイルｱ.txt"),
        ("auto-utf8", "文件名称测试.txt"),
    ] {
        let snapshot = engine
            .open(
                OpenRequest::new(root.path().join(format!("{fixture}.zip"))),
                &OperationControl::default(),
            )
            .expect("Auto listing");
        assert_eq!(snapshot.entries()[0].display_path(), expected, "{fixture}");
        assert!(
            snapshot.entries()[0]
                .warnings()
                .contains(&EntryWarning::UncertainEncoding),
            "heuristic is not authoritative"
        );
    }
    let mut request = OpenRequest::new(root.path().join("ambiguous.zip"));
    let auto = engine
        .open(request, &OperationControl::default())
        .expect("ambiguous Auto");
    assert_eq!(auto.entries()[0].display_path(), "╓╨╬─.txt");
    assert_eq!(auto.entries()[0].raw_name(), b"\xd6\xd0\xce\xc4.txt");
    request = OpenRequest::new(root.path().join("ambiguous.zip"));
    request.encoding = NameEncoding::Gbk;
    let manual = engine
        .open(request, &OperationControl::default())
        .expect("manual disambiguation");
    assert_eq!(manual.entries()[0].display_path(), "中文.txt");
    let competing = engine
        .open(
            OpenRequest::new(root.path().join("ambiguous-long.zip")),
            &OperationControl::default(),
        )
        .expect("two plausible scripts");
    assert_eq!(
        competing.entries()[0].name_provenance(),
        nxtools_archive::NameProvenance::UncertainCp437
    );
}

#[test]
fn reprojection_keeps_identity_and_rechecks_unsafe_case_and_file_prefix() {
    use nxtools_archive::{
        ArchiveError,
        planning::{ExtractionPlan, ExtractionSelection, TargetStrategy},
    };
    let root = fixtures();
    let engine = ArchiveEngine::default();
    let control = OperationControl::default();
    for (name, error) in [
        ("switch-unsafe", ArchiveError::UnsafePath),
        ("switch-case", ArchiveError::OutputCollision),
        ("switch-prefix", ArchiveError::OutputCollision),
    ] {
        let mut request = OpenRequest::new(root.path().join(format!("{name}.zip")));
        request.encoding = NameEncoding::Cp437;
        let original = engine
            .open(request, &control)
            .expect("safe CP437 projection");
        ExtractionPlan::build(&original, ExtractionSelection::All, TargetStrategy::Here)
            .expect("original plan");
        let changed = engine
            .reproject(&original, NameEncoding::Windows1252, &control)
            .expect("names remain visible");
        assert_eq!(changed.session_id(), original.session_id());
        for (before, after) in original.entries().iter().zip(changed.entries()) {
            assert_eq!(before.id(), after.id());
            assert_eq!(before.raw_name(), after.raw_name());
        }
        assert!(
            matches!(ExtractionPlan::build(&changed, ExtractionSelection::All, TargetStrategy::Here), Err(actual) if actual == error)
        );
        let restored = engine
            .reproject(&changed, NameEncoding::Cp437, &control)
            .expect("restore safe projection");
        ExtractionPlan::build(&restored, ExtractionSelection::All, TargetStrategy::Here)
            .expect("no stale collision warnings");
    }
    let auto = engine
        .open(OpenRequest::new(root.path().join("auto-gbk.zip")), &control)
        .expect("Auto source");
    let changed = engine
        .reproject(&auto, NameEncoding::Gbk, &control)
        .expect("manual projection");
    let plan = ExtractionPlan::build(&changed, ExtractionSelection::All, TargetStrategy::Here)
        .expect("fresh manual output plan");
    assert_eq!(plan.files()[0].relative_path(), "中文文件资料.txt");
    assert_eq!(plan.files()[0].id(), auto.entries()[0].id());
}

#[test]
fn unicode_authority_and_other_explicit_pages_use_independent_raw_literals() {
    use nxtools_archive::NameProvenance;
    let root = fixtures();
    let engine = ArchiveEngine::default();
    let control = OperationControl::default();
    for (file, page, raw, expected) in [
        (
            "big5",
            NameEncoding::Big5,
            &b"\xc1\x63\xc5\xe9\xa4\xa4\xa4\xe5.txt"[..],
            "繁體中文.txt",
        ),
        (
            "cp949",
            NameEncoding::Korean,
            &b"\xc7\xd1\xb1\xdb\xc6\xc4\xc0\xcf.txt"[..],
            "한글파일.txt",
        ),
        (
            "cp1250",
            NameEncoding::Windows1250,
            &b"P\xf8\xedli\x9a.txt"[..],
            "Příliš.txt",
        ),
        (
            "cp1251",
            NameEncoding::Windows1251,
            &b"\xcf\xf0\xe8\xe2\xe5\xf2.txt"[..],
            "Привет.txt",
        ),
        (
            "cp1252",
            NameEncoding::Windows1252,
            &b"caf\xe9.txt"[..],
            "café.txt",
        ),
    ] {
        let mut request = OpenRequest::new(root.path().join(format!("{file}.zip")));
        request.encoding = page;
        let snapshot = engine.open(request, &control).expect("explicit page");
        assert_eq!(snapshot.entries()[0].raw_name(), raw);
        assert_eq!(snapshot.entries()[0].display_path(), expected);
        assert_eq!(
            snapshot.entries()[0].name_provenance(),
            NameProvenance::Explicit(page)
        );
    }
    let snapshot = engine
        .open(
            OpenRequest::new(root.path().join("authority.zip")),
            &control,
        )
        .expect("authoritative names");
    let changed = engine
        .reproject(&snapshot, NameEncoding::Windows1252, &control)
        .expect("manual cannot replace Unicode");
    assert_eq!(changed.entries()[0].display_path(), "权威UTF8.txt");
    assert_eq!(
        changed.entries()[0].name_provenance(),
        NameProvenance::Utf8Flag
    );
    assert_eq!(changed.entries()[1].display_path(), "权威名称.txt");
    assert_eq!(
        changed.entries()[1].name_provenance(),
        NameProvenance::UnicodeExtra
    );
}

#[derive(Default)]
struct NoOutput(usize);
impl nxtools_archive::output::ArchiveOutput for NoOutput {
    fn ensure_directory(&mut self, _: &str) -> nxtools_archive::Result<()> {
        self.0 += 1;
        Err(nxtools_archive::ArchiveError::IoFailure)
    }
    fn begin_file(&mut self, _: &str) -> nxtools_archive::Result<uuid::Uuid> {
        self.0 += 1;
        Err(nxtools_archive::ArchiveError::IoFailure)
    }
    fn write_file(&mut self, _: uuid::Uuid, _: &[u8]) -> nxtools_archive::Result<()> {
        self.0 += 1;
        Err(nxtools_archive::ArchiveError::IoFailure)
    }
    fn commit_file(&mut self, _: uuid::Uuid) -> nxtools_archive::Result<()> {
        self.0 += 1;
        Err(nxtools_archive::ArchiveError::IoFailure)
    }
    fn abort_file(&mut self, _: uuid::Uuid) {
        self.0 += 1;
    }
}

#[test]
fn old_safe_plan_cannot_execute_a_changed_projection_and_reprojection_checks_revision() {
    use nxtools_archive::{
        ArchiveError,
        output::ExtractRequest,
        planning::{ExtractionPlan, ExtractionSelection, TargetStrategy},
    };
    let root = fixtures();
    let control = OperationControl::default();
    let engine = ArchiveEngine::default();
    let path = root.path().join("switch-unsafe.zip");
    let original = engine
        .open(OpenRequest::new(&path), &control)
        .expect("safe initial projection");
    let plan = ExtractionPlan::build(&original, ExtractionSelection::All, TargetStrategy::Here)
        .expect("safe plan");
    let changed = engine
        .reproject(&original, NameEncoding::Windows1252, &control)
        .expect("dangerous projection visible");
    let mut output = NoOutput::default();
    assert_eq!(
        engine.extract(ExtractRequest::new(changed, plan), &control, &mut output),
        Err(ArchiveError::SourceChanged)
    );
    assert_eq!(output.0, 0, "old plan must not reach any output capability");
    std::fs::copy(root.path().join("auto-gbk.zip"), &path)
        .expect("replace source after prior operations");
    assert!(matches!(
        engine.reproject(&original, NameEncoding::Gbk, &control),
        Err(ArchiveError::SourceChanged)
    ));
    let cancelled = OperationControl::default();
    cancelled.cancel();
    assert!(matches!(
        engine.reproject(&original, NameEncoding::Gbk, &cancelled),
        Err(ArchiveError::Cancelled)
    ));
}
