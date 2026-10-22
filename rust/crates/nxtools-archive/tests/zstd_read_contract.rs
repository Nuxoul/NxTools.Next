use nxtools_archive::{
    ArchiveEngine, ArchiveFormat, ArchiveRequest, OpenRequest, OperationControl,
};
use std::path::PathBuf;

#[test]
fn independent_zstd_single_multi_and_tar_public_read_requires_complete_frames() {
    for (name, format, size) in [
        (
            "independent.zst",
            ArchiveFormat::Zstd,
            "独立 gzip fixture\n".len() as u64 + 256,
        ),
        ("multi.zst", ArchiveFormat::Zstd, 13),
        (
            "independent.tar.zst",
            ArchiveFormat::TarZstd,
            "独立 gzip fixture\n".len() as u64 + 256,
        ),
    ] {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        let engine = ArchiveEngine::default();
        let control = OperationControl::default();
        let snapshot = engine
            .open(OpenRequest::new(path), &control)
            .expect("independent Zstd public open");
        assert_eq!(snapshot.format(), format);
        let outcome = engine
            .execute(ArchiveRequest::test(snapshot), &control)
            .expect("complete frames");
        assert_eq!(outcome.files_verified(), 1);
        assert_eq!(outcome.bytes_verified(), size);
    }
}
