//! Raw fixed-header fixtures: expected times are literals, never a product writer/readback.
use nxtools_archive::metadata::ModifiedTime;
use nxtools_archive::{ArchiveEngine, OpenRequest, OperationControl};

fn member(bytes: &mut Vec<u8>, name: &str, kind: u8, time: [u8; 12], payload: &[u8]) {
    let mut header = [0u8; 512];
    header[..name.len()].copy_from_slice(name.as_bytes());
    header[100..108].copy_from_slice(b"0000644\0");
    header[108..116].copy_from_slice(b"0000000\0");
    header[116..124].copy_from_slice(b"0000000\0");
    header[124..136].copy_from_slice(format!("{:011o}\0", payload.len()).as_bytes());
    header[136..148].copy_from_slice(&time);
    header[148..156].fill(b' ');
    header[156] = kind;
    header[257..263].copy_from_slice(b"ustar\0");
    header[263..265].copy_from_slice(b"00");
    let checksum: u32 = header.iter().map(|byte| u32::from(*byte)).sum();
    header[148..156].copy_from_slice(format!("{checksum:06o}\0 ").as_bytes());
    bytes.extend_from_slice(&header);
    bytes.extend_from_slice(payload);
    bytes.resize(bytes.len().next_multiple_of(512), 0);
}

fn unix(seconds: i64) -> ModifiedTime {
    ModifiedTime::UnixUtc {
        seconds,
        nanoseconds: 0,
        precision_ns: 1_000_000_000,
    }
}

#[test]
fn gnu_signed_base256_negative_mtime_reaches_public_metadata() {
    let owned = tempfile::tempdir().expect("owned TAR fixture");
    let path = owned.path().join("negative.tar");
    let mut bytes = Vec::new();
    member(&mut bytes, "payload.txt", b'0', [0xff; 12], b"x");
    bytes.extend_from_slice(&[0; 1024]);
    std::fs::write(&path, bytes).expect("raw TAR fixture");
    let snapshot = ArchiveEngine::default()
        .open(OpenRequest::new(path), &OperationControl::default())
        .expect("valid negative GNU TAR");
    assert_eq!(snapshot.entries().len(), 1);
    assert_eq!(snapshot.entries()[0].metadata().modified, Some(unix(-1)));
}

fn pax_time(value: &str) -> Vec<u8> {
    let record = format!("mtime={value}\n");
    let mut length = record.len() + 2;
    loop {
        let next = record.len() + 1 + length.to_string().len();
        if next == length {
            return format!("{length} {record}").into_bytes();
        }
        length = next;
    }
}

#[test]
fn pax_global_mtime_set_clear_restores_subsequent_header() {
    let owned = tempfile::tempdir().expect("owned PAX fixture");
    let path = owned.path().join("global-clear.tar");
    let mut bytes = Vec::new();
    member(
        &mut bytes,
        "global",
        b'g',
        *b"00000000000\0",
        &pax_time("1500000000"),
    );
    member(&mut bytes, "first.txt", b'0', *b"13727410000\0", b"x");
    member(&mut bytes, "clear", b'g', *b"00000000000\0", &pax_time(""));
    member(&mut bytes, "second.txt", b'0', *b"13727410001\0", b"y");
    bytes.extend_from_slice(&[0; 1024]);
    std::fs::write(&path, bytes).expect("raw PAX fixture");
    let snapshot = ArchiveEngine::default()
        .open(OpenRequest::new(path), &OperationControl::default())
        .expect("valid global removal");
    assert_eq!(snapshot.entries().len(), 2);
    assert_eq!(
        snapshot.entries()[0].metadata().modified,
        Some(unix(1_500_000_000))
    );
    assert_eq!(
        snapshot.entries()[1].metadata().modified,
        Some(unix(1_600_000_001))
    );
}

#[test]
fn pax_local_empty_suppresses_global_and_header_for_current_member_only() {
    let owned = tempfile::tempdir().expect("owned local PAX fixture");
    let path = owned.path().join("local-clear.tar");
    let mut bytes = Vec::new();
    member(
        &mut bytes,
        "global",
        b'g',
        *b"00000000000\0",
        &pax_time("1500000000"),
    );
    member(&mut bytes, "local", b'x', *b"00000000000\0", &pax_time(""));
    member(&mut bytes, "first.txt", b'0', *b"13727410000\0", b"x");
    member(&mut bytes, "second.txt", b'0', *b"13727410001\0", b"y");
    bytes.extend_from_slice(&[0; 1024]);
    std::fs::write(&path, bytes).expect("raw local PAX fixture");
    let snapshot = ArchiveEngine::default()
        .open(OpenRequest::new(path), &OperationControl::default())
        .expect("valid local deletion");
    assert_eq!(snapshot.entries()[0].metadata().modified, None);
    assert_eq!(
        snapshot.entries()[1].metadata().modified,
        Some(unix(1_500_000_000))
    );
}

fn binary_time(seconds: i64) -> [u8; 12] {
    let mut bytes = [if seconds < 0 { 0xff } else { 0 }; 12];
    bytes[0] = if seconds < 0 { 0xff } else { 0x80 };
    bytes[4..].copy_from_slice(&seconds.to_be_bytes());
    bytes
}

#[test]
fn fixed_header_time_distinguishes_signed_octal_absence_invalid_and_range() {
    let owned = tempfile::tempdir().expect("owned numeric fixtures");
    let mut positive_overflow = [0; 12];
    positive_overflow[0] = 0x80;
    positive_overflow[3] = 1; // 2^64, not zero from its low64 bits.
    let mut negative_overflow = [0xff; 12];
    negative_overflow[3] = 0xfe; // Below i64::MIN despite the low64 -1.
    let mut invalid_marker = [0xff; 12];
    invalid_marker[0] = 0x81;
    let mut cases = vec![
        (*b"00000000000\0", Some(unix(0))),
        (*b"13727410001\0", Some(unix(1_600_000_001))),
        (*b"77777777777\0", Some(unix(8_589_934_591))),
        (*b" 000000001 \0", Some(unix(1))),
        ([0; 12], None),
        ([b' '; 12], None),
        (*b"00000000008\0", Some(ModifiedTime::Invalid)),
        (*b"000\x000000001\0", Some(ModifiedTime::Invalid)),
        (*b"\x000000000001\0", Some(ModifiedTime::Invalid)),
        (*b"-0000000001\0", Some(ModifiedTime::Invalid)),
        (positive_overflow, Some(ModifiedTime::Invalid)),
        (negative_overflow, Some(ModifiedTime::Invalid)),
        (invalid_marker, Some(ModifiedTime::Invalid)),
    ];
    for seconds in [
        -1,
        -86_401,
        -2_208_988_800,
        0,
        1,
        1_600_000_001,
        -11_644_473_600,
        -11_644_473_601,
        253_402_300_799,
        253_402_300_800,
        i64::MIN,
        i64::MAX,
    ] {
        cases.push((binary_time(seconds), Some(unix(seconds))));
    }
    for (index, (raw, expected)) in cases.into_iter().enumerate() {
        let path = owned.path().join(format!("case-{index}.tar"));
        let mut bytes = Vec::new();
        member(&mut bytes, "payload.txt", b'0', raw, b"x");
        bytes.extend_from_slice(&[0; 1024]);
        std::fs::write(&path, bytes).expect("bounded numeric fixture");
        let snapshot = ArchiveEngine::default()
            .open(OpenRequest::new(path), &OperationControl::default())
            .expect("structure independent from time validity");
        let metadata = snapshot.entries()[0].metadata();
        assert_eq!(metadata.modified, expected, "raw numeric case {index}");
        if let Some(ModifiedTime::UnixUtc { seconds, .. }) = expected {
            assert_eq!(
                metadata.validate().is_ok(),
                (-11_644_473_600..=253_402_300_799).contains(&seconds)
            );
        } else {
            assert_eq!(metadata.validate().is_ok(), expected.is_none());
        }
    }
}

#[test]
fn pax_last_record_precedence_scope_and_nonempty_invalid_remain_explicit() {
    let owned = tempfile::tempdir().expect("owned PAX precedence fixture");
    let path = owned.path().join("precedence.tar");
    let zero = *b"00000000000\0";
    let header = *b"13727410001\0";
    let mut bytes = Vec::new();
    member(&mut bytes, "global", b'g', zero, &pax_time("1500000000"));
    let mut local = pax_time("1400000000");
    local.extend(pax_time(""));
    member(&mut bytes, "delete-last", b'x', zero, &local);
    member(&mut bytes, "deleted.txt", b'0', header, b"x");
    member(&mut bytes, "global.txt", b'0', header, b"x");
    let mut local = pax_time("");
    local.extend(pax_time("-0.0000000001"));
    member(&mut bytes, "set-last", b'x', zero, &local);
    member(&mut bytes, "fraction.txt", b'0', header, b"x");
    member(&mut bytes, "bad-local", b'x', zero, &pax_time("not-a-time"));
    member(&mut bytes, "invalid-local.txt", b'0', header, b"x");
    member(&mut bytes, "bad-global", b'g', zero, &pax_time("1e2"));
    member(&mut bytes, "invalid-global.txt", b'0', header, b"x");
    member(&mut bytes, "local-override", b'x', zero, &pax_time("0"));
    member(&mut bytes, "override.txt", b'0', header, b"x");
    member(&mut bytes, "invalid-global-again.txt", b'0', header, b"x");
    member(&mut bytes, "global-clear", b'g', zero, &pax_time(""));
    member(&mut bytes, "header.txt", b'0', header, b"x");
    // Local deletion suppresses even a malformed raw time, without parsing/fallback.
    member(&mut bytes, "local-clear", b'x', zero, &pax_time(""));
    member(&mut bytes, "suppressed.txt", b'0', *b"00000000008\0", b"x");
    member(&mut bytes, "bad-header.txt", b'0', *b"00000000008\0", b"x");
    bytes.extend_from_slice(&[0; 1024]);
    std::fs::write(&path, bytes).expect("raw precedence fixture");
    let snapshot = ArchiveEngine::default()
        .open(OpenRequest::new(path), &OperationControl::default())
        .expect("bounded precedence metadata");
    let expected = [
        None,
        Some(unix(1_500_000_000)),
        Some(ModifiedTime::UnixUtc {
            seconds: -1,
            nanoseconds: 999_999_999,
            precision_ns: 1,
        }),
        Some(ModifiedTime::Invalid),
        Some(ModifiedTime::Invalid),
        Some(unix(0)),
        Some(ModifiedTime::Invalid),
        Some(unix(1_600_000_001)),
        None,
        Some(ModifiedTime::Invalid),
    ];
    assert_eq!(snapshot.entries().len(), expected.len());
    for (entry, expected) in snapshot.entries().iter().zip(expected) {
        assert_eq!(
            entry.metadata().modified,
            expected,
            "{}",
            entry.display_path()
        );
    }
}

#[test]
fn pax_empty_local_still_requires_a_real_member_and_valid_record_extent() {
    let owned = tempfile::tempdir().expect("owned PAX structure fixtures");
    for (label, payload) in [
        ("orphan", pax_time("")),
        ("bad-length", b"8 mtime=\n".to_vec()),
    ] {
        let path = owned.path().join(format!("{label}.tar"));
        let mut bytes = Vec::new();
        member(&mut bytes, "local-clear", b'x', *b"00000000000\0", &payload);
        bytes.extend_from_slice(&[0; 1024]);
        std::fs::write(&path, bytes).expect("bounded structure fixture");
        let result =
            ArchiveEngine::default().open(OpenRequest::new(path), &OperationControl::default());
        assert!(
            matches!(result, Err(nxtools_archive::ArchiveError::CorruptArchive)),
            "{label}"
        );
    }
}
