use nxtools_archive::{ArchiveEngine, ArchiveError, ArchiveRequest, OpenRequest, OperationControl};
use std::{fs, path::Path};

fn record(name: &str, payload: &[u8], disk: u16, offset: u32) -> (Vec<u8>, Vec<u8>) {
    let crc = crc32fast::hash(payload);
    let mut local = Vec::new();
    local.extend_from_slice(b"PK\x03\x04");
    for value in [20u16, 0x800, 0, 0, 0x5021] {
        local.extend_from_slice(&value.to_le_bytes());
    }
    for value in [crc, payload.len() as u32, payload.len() as u32] {
        local.extend_from_slice(&value.to_le_bytes());
    }
    local.extend_from_slice(&(name.len() as u16).to_le_bytes());
    local.extend_from_slice(&0u16.to_le_bytes());
    local.extend_from_slice(name.as_bytes());
    local.extend_from_slice(payload);
    let mut central = Vec::new();
    central.extend_from_slice(b"PK\x01\x02");
    for value in [20u16, 20, 0x800, 0, 0, 0x5021] {
        central.extend_from_slice(&value.to_le_bytes());
    }
    for value in [crc, payload.len() as u32, payload.len() as u32] {
        central.extend_from_slice(&value.to_le_bytes());
    }
    for value in [name.len() as u16, 0, 0, disk, 0] {
        central.extend_from_slice(&value.to_le_bytes());
    }
    central.extend_from_slice(&0x20u32.to_le_bytes());
    central.extend_from_slice(&offset.to_le_bytes());
    central.extend_from_slice(name.as_bytes());
    (local, central)
}
fn volumes(root: &Path) {
    let (one, first) = record("one.txt", b"volume-one\n", 0, 4);
    let (two, second) = record("two.txt", b"volume-two\n", 1, 0);
    let mut disk0 = b"PK\x07\x08".to_vec();
    disk0.extend(one);
    let directory_offset = two.len() as u32;
    let mut disk1 = two;
    let directory_size = (first.len() + second.len()) as u32;
    disk1.extend(first);
    disk1.extend(second);
    disk1.extend_from_slice(b"PK\x05\x06");
    for value in [1u16, 1, 2, 2] {
        disk1.extend_from_slice(&value.to_le_bytes());
    }
    disk1.extend_from_slice(&directory_size.to_le_bytes());
    disk1.extend_from_slice(&directory_offset.to_le_bytes());
    disk1.extend_from_slice(&0u16.to_le_bytes());
    fs::write(root.join("real.z01"), disk0).expect("first volume");
    fs::write(root.join("real.zip"), disk1).expect("last volume");
}
#[test]
fn genuine_zip_volume_public_read_test_and_replacement_identity() {
    let root = tempfile::tempdir().expect("owned fixture");
    volumes(root.path());
    let engine = ArchiveEngine::default().with_zip_volumes();
    let control = OperationControl::default();
    let snapshot = engine
        .open(OpenRequest::new(root.path().join("real.zip")), &control)
        .expect("all genuine ZIP volumes available");
    assert_eq!(snapshot.entries().len(), 2);
    assert!(snapshot.capabilities().split_read);
    let outcome = engine
        .execute(ArchiveRequest::test(snapshot.clone()), &control)
        .expect("both volumes verified");
    assert_eq!(outcome.files_verified(), 2);
    assert_eq!(outcome.bytes_verified(), 22);
    fs::write(root.path().join("real.z01"), b"not-the-original-volume")
        .expect("replace first volume");
    assert_eq!(
        engine.execute(ArchiveRequest::test(snapshot), &control),
        Err(ArchiveError::SourceChanged)
    );
}
#[test]
fn genuine_zip_volume_missing_is_explicit_and_later_volume_selects_same_family() {
    let root = tempfile::tempdir().expect("owned fixture");
    volumes(root.path());
    let engine = ArchiveEngine::default().with_zip_volumes();
    let control = OperationControl::default();
    let first = engine
        .open(OpenRequest::new(root.path().join("real.z01")), &control)
        .expect("first volume routing");
    assert_eq!(first.entries().len(), 2);
    fs::remove_file(root.path().join("real.z01")).expect("remove owned first volume");
    assert!(matches!(
        engine.open(OpenRequest::new(root.path().join("real.zip")), &control),
        Err(ArchiveError::MissingVolume)
    ));
}

#[test]
fn worker_default_does_not_opt_into_unleased_volume_set() {
    let root = tempfile::tempdir().expect("owned fixture");
    volumes(root.path());
    assert!(matches!(
        ArchiveEngine::default().open(
            OpenRequest::new(root.path().join("real.zip")),
            &OperationControl::default()
        ),
        Err(ArchiveError::MissingVolume)
    ));
}

#[test]
fn wrong_volume_order_count_and_encoded_spool_budget_are_rejected() {
    let root = tempfile::tempdir().expect("owned fixture");
    volumes(root.path());
    let first = fs::read(root.path().join("real.z01")).expect("first bytes");
    let last = fs::read(root.path().join("real.zip")).expect("last bytes");
    fs::write(root.path().join("real.z01"), &last).expect("wrong first");
    fs::write(root.path().join("real.zip"), &first).expect("wrong last");
    assert!(matches!(
        ArchiveEngine::default().with_zip_volumes().open(
            OpenRequest::new(root.path().join("real.zip")),
            &OperationControl::default()
        ),
        Err(ArchiveError::CorruptArchive)
    ));
    volumes(root.path());
    let mut invalid = fs::read(root.path().join("real.zip")).expect("last bytes");
    let end = invalid.len() - 22;
    invalid[end + 8..end + 10].copy_from_slice(&0u16.to_le_bytes());
    fs::write(root.path().join("real.zip"), invalid).expect("wrong directory count");
    assert!(matches!(
        ArchiveEngine::default().with_zip_volumes().open(
            OpenRequest::new(root.path().join("real.zip")),
            &OperationControl::default()
        ),
        Err(ArchiveError::CorruptArchive)
    ));
    volumes(root.path());
    let limits = nxtools_archive::ResourceLimits {
        max_output_bytes: 64,
        ..Default::default()
    };
    assert!(matches!(
        ArchiveEngine::with_limits(limits).with_zip_volumes().open(
            OpenRequest::new(root.path().join("real.zip")),
            &OperationControl::default()
        ),
        Err(ArchiveError::ResourceLimit)
    ));
}

#[test]
fn numbered_member_uses_platform_case_semantics_and_exact_family_ordinal() {
    let root = tempfile::tempdir().expect("owned fixture");
    volumes(root.path());
    let engine = ArchiveEngine::default().with_zip_volumes();
    let control = OperationControl::default();
    let lower = engine
        .open(OpenRequest::new(root.path().join("real.z01")), &control)
        .expect("lowercase actual member");
    assert_eq!(lower.entries().len(), 2);
    let upper = engine.open(OpenRequest::new(root.path().join("real.Z01")), &control);
    #[cfg(windows)]
    {
        let upper = upper.expect("Windows uppercase extension resolves same exact member");
        assert_eq!(upper.entries().len(), lower.entries().len());
        let verified = engine
            .execute(ArchiveRequest::test(upper), &control)
            .expect("same volume bytes");
        assert_eq!(verified.bytes_verified(), 22);
    }
    #[cfg(not(windows))]
    assert!(
        upper.is_err(),
        "case-sensitive platform must not alias absent uppercase member"
    );
    for name in ["real.z00", "real.z02", "real.z001", "other.z01"] {
        assert!(
            engine
                .open(OpenRequest::new(root.path().join(name)), &control)
                .is_err(),
            "invalid exact-family member must not be accepted"
        );
    }
}
