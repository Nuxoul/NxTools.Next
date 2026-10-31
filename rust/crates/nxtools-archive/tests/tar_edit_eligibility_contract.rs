use nxtools_archive::{
    ArchiveEngine, ArchiveFormat, ArchiveModificationRefusal as R, ArchiveOperation as O,
    OpenRequest, OperationControl,
};
use std::process::Command;
#[test]
fn tar_edit_actual_source_qualification_is_separate_from_readability_and_suffix() {
    let root = tempfile::tempdir().expect("owned");
    for (format, suffix) in [("Tar", "tar"), ("TarGz", "tgz"), ("TarXz", "txz")] {
        let path = root.path().join(format!("source.{suffix}"));
        assert!(Command::new("python").args(["-c","import sys,tarfile,io; t=tarfile.open(sys.argv[1],{'Tar':'w','TarGz':'w:gz','TarXz':'w:xz'}[sys.argv[2]],format=tarfile.USTAR_FORMAT); h=tarfile.TarInfo('a/');h.type=tarfile.DIRTYPE;t.addfile(h);h=tarfile.TarInfo('a/file');h.size=1;t.addfile(h,io.BytesIO(b'x'));t.close()"]).arg(&path).arg(format).status().expect("Python seed").success());
        let engine = ArchiveEngine::default();
        let control = OperationControl::default();
        let snapshot = engine
            .open(OpenRequest::new(&path), &control)
            .expect("readable");
        assert_eq!(
            engine.modification_eligibility(&snapshot, &control),
            Ok(()),
            "actual {format} header catalog matches original ordinal projection"
        );
        for operation in [
            O::Add,
            O::Delete,
            O::ReplaceContent,
            O::RenameFile,
            O::RenameDirectorySubtree,
            O::NewDirectory,
        ] {
            assert_eq!(snapshot.capabilities().admit(operation), Ok(()));
        }
        assert_eq!(
            snapshot.capabilities().admit(O::Comment),
            Err(R::BackendMissing)
        );
    }
    let path = root.path().join("links.tar");
    assert!(Command::new("python").args(["-c","import sys,tarfile; t=tarfile.open(sys.argv[1],'w');h=tarfile.TarInfo('link');h.type=tarfile.SYMTYPE;h.linkname='target';t.addfile(h);t.close()"]).arg(&path).status().expect("link seed").success());
    let snapshot = ArchiveEngine::default()
        .open(OpenRequest::new(&path), &OperationControl::default())
        .expect("ordinary reader remains healthy");
    assert!(snapshot.capabilities().browse);
    assert_eq!(
        snapshot.capabilities().admit(O::Add),
        Err(R::UnsupportedStructure)
    );
    for format in [
        ArchiveFormat::TarBz2,
        ArchiveFormat::TarZstd,
        ArchiveFormat::Gzip,
        ArchiveFormat::Xz,
    ] {
        assert_eq!(
            nxtools_archive::ArchiveCapabilities::opened(format, None).admit(O::Add),
            Err(R::BackendMissing)
        );
    }
}
