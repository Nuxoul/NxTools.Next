use nxtools_download::{
    DownloadSegment, DownloadSegmentState, SegmentedArtifact, SegmentedArtifactError,
};

#[tokio::test]
async fn independent_segment_writers_reconstruct_one_file_out_of_order() {
    let directory = tempfile::tempdir().expect("create artifact directory");
    let path = directory.path().join(".fixture.nxtools-part");
    let artifact = SegmentedArtifact::create(&path, 16)
        .await
        .expect("create pre-sized artifact");
    let first = DownloadSegment::new(0, 0, 7, 0, DownloadSegmentState::Active)
        .expect("valid first segment");
    let second = DownloadSegment::new(1, 8, 15, 0, DownloadSegmentState::Active)
        .expect("valid second segment");

    let mut second_writer = artifact
        .open_segment(&second)
        .await
        .expect("open second segment writer");
    second_writer
        .write_all(b"89abcdef")
        .await
        .expect("write second segment");
    second_writer.sync().await.expect("sync second segment");

    let mut first_writer = artifact
        .open_segment(&first)
        .await
        .expect("open first segment writer");
    first_writer
        .write_all(b"01234567")
        .await
        .expect("write first segment");
    first_writer.sync().await.expect("sync first segment");

    assert_eq!(
        tokio::fs::read(&path).await.expect("read artifact"),
        b"0123456789abcdef"
    );
}

#[tokio::test]
async fn a_segment_writer_rejects_bytes_outside_its_owned_range_before_writing() {
    let directory = tempfile::tempdir().expect("create artifact directory");
    let path = directory.path().join(".fixture.nxtools-part");
    let artifact = SegmentedArtifact::create(&path, 16)
        .await
        .expect("create pre-sized artifact");
    let segment = DownloadSegment::new(0, 4, 7, 2, DownloadSegmentState::Active)
        .expect("valid resumed segment");
    let mut writer = artifact
        .open_segment(&segment)
        .await
        .expect("open resumed writer");

    let error = writer
        .write_all(b"678")
        .await
        .expect_err("three bytes exceed the two-byte remainder");

    assert!(matches!(
        error,
        SegmentedArtifactError::SegmentWriteOutOfBounds {
            segment_index: 0,
            remaining_bytes: 2,
            attempted_bytes: 3,
        }
    ));
    assert_eq!(
        tokio::fs::read(&path)
            .await
            .expect("read unchanged artifact"),
        vec![0; 16]
    );
}

#[cfg(windows)]
#[tokio::test]
async fn a_new_segmented_artifact_uses_the_windows_sparse_attribute() {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_SPARSE_FILE: u32 = 0x0000_0200;

    let directory = tempfile::tempdir().expect("create artifact directory");
    let path = directory.path().join(".sparse.nxtools-part");
    SegmentedArtifact::create(&path, 32 * 1024 * 1024)
        .await
        .expect("create sparse artifact");

    let attributes = std::fs::metadata(&path)
        .expect("read sparse artifact metadata")
        .file_attributes();
    assert_ne!(
        attributes & FILE_ATTRIBUTE_SPARSE_FILE,
        0,
        "Windows must report the incomplete artifact as sparse"
    );
}

#[cfg(windows)]
#[tokio::test]
async fn an_external_exclusive_lock_is_reported_by_the_segment_writer_boundary() {
    use std::os::windows::fs::OpenOptionsExt;

    let directory = tempfile::tempdir().expect("create artifact directory");
    let path = directory.path().join(".locked.nxtools-part");
    let artifact = SegmentedArtifact::create(&path, 16)
        .await
        .expect("create artifact");
    let exclusive = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0)
        .open(&path)
        .expect("hold exclusive artifact lock");
    let segment =
        DownloadSegment::new(0, 0, 15, 0, DownloadSegmentState::Active).expect("valid segment");

    let error = artifact
        .open_segment(&segment)
        .await
        .expect_err("exclusive foreign lock must reject a writer");
    let SegmentedArtifactError::Io(error) = error else {
        panic!("expected an I/O sharing violation");
    };
    assert!(
        matches!(error.raw_os_error(), Some(32 | 33)),
        "unexpected native error: {error:?}"
    );

    drop(exclusive);
}
