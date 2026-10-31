use super::*;

fn source(header: &[u8], tail: &[u8]) -> Vec<u8> {
    let mut signature = vec![0x37, 0x7a, 0xbc, 0xaf, 0x27, 0x1c, 0, 4, 0, 0, 0, 0];
    signature.extend_from_slice(&0u64.to_le_bytes());
    signature.extend_from_slice(&(header.len() as u64).to_le_bytes());
    signature.extend_from_slice(&crc32fast::hash(header).to_le_bytes());
    let crc = crc32fast::hash(&signature[12..]);
    signature[8..12].copy_from_slice(&crc.to_le_bytes());
    signature.extend_from_slice(header);
    signature.extend_from_slice(tail);
    signature
}
fn inspect_bytes(bytes: Vec<u8>, control: &OperationControl) -> Result<HeaderFacts> {
    inspect(
        &mut Cursor::new(bytes),
        &ResourceLimits::default(),
        control,
        &sevenz_rust2::Password::empty(),
    )
}

#[test]
fn sevenz_header_raw_allowlist_and_unknown_files_property() {
    let control = OperationControl::default();
    inspect_bytes(source(&[1, 5, 0, 0, 0], &[]), &control).expect("empty raw header");
    assert_eq!(
        inspect_bytes(source(&[1, 5, 1, 0x18, 1, 0xff, 0, 0], &[]), &control).err(),
        Some(ArchiveError::UnsupportedOperation)
    );
    assert_eq!(
        inspect_bytes(source(&[1, 5, 0, 0x17, 0, 0, 0], &[]), &control).err(),
        Some(ArchiveError::UnsupportedOperation)
    );
    inspect_bytes(
        source(&[1, 5, 0, 0x19, 2, 0, 0, 0x19, 1, 0, 0, 0], &[]),
        &control,
    )
    .expect("7-Zip repeated zero-only alignment has no member semantics");
    assert_eq!(
        inspect_bytes(source(&[1, 5, 0, 0x19, 1, 0xff, 0, 0], &[]), &control).err(),
        Some(ArchiveError::UnsupportedOperation)
    );
}

#[test]
fn sevenz_header_encoded_unknown_property_is_not_hidden_by_decoder() {
    // One Copy coder, one stored synthetic metadata stream. The encoded header
    // must pass through the same strict FilesInfo admission as a raw header.
    let raw = [1, 5, 1, 0x18, 1, 0xff, 0, 0];
    let descriptor = [
        0x17,
        6,
        0,
        1,
        9,
        8,
        0,
        7,
        11,
        1,
        0,
        1,
        1,
        0,
        12,
        raw.len() as u8,
        0,
        8,
        0,
        0,
    ];
    let mut bytes = source(&descriptor, &[]);
    bytes.splice(32..32, raw);
    let offset = raw.len() as u64;
    bytes[12..20].copy_from_slice(&offset.to_le_bytes());
    let crc = crc32fast::hash(&descriptor);
    bytes[28..32].copy_from_slice(&crc.to_le_bytes());
    let crc = crc32fast::hash(&bytes[12..32]);
    bytes[8..12].copy_from_slice(&crc.to_le_bytes());
    assert_eq!(
        inspect_bytes(bytes, &OperationControl::default()).err(),
        Some(ArchiveError::UnsupportedOperation)
    );
}

#[test]
fn sevenz_header_crc_truncation_and_nonzero_tail_reject() {
    let valid = source(&[1, 5, 0, 0, 0], &[]);
    let mut bad_crc = valid.clone();
    bad_crc[32] ^= 1;
    assert_eq!(
        inspect_bytes(bad_crc, &OperationControl::default()).err(),
        Some(ArchiveError::CorruptArchive)
    );
    let mut short = valid.clone();
    short.pop();
    assert!(inspect_bytes(short, &OperationControl::default()).is_err());
    assert_eq!(
        inspect_bytes(
            source(&[1, 5, 0, 0, 0], &[0xff]),
            &OperationControl::default()
        )
        .err(),
        Some(ArchiveError::UnsupportedOperation)
    );
    assert_eq!(
        inspect_bytes(
            source(&[1, 5, 1, 17, 4, 0], &[]),
            &OperationControl::default()
        )
        .err(),
        Some(ArchiveError::CorruptArchive)
    );
}

#[test]
fn sevenz_header_rejects_special_files_info_and_unsupported_graphs() {
    let control = OperationControl::default();
    // Anti property cannot be preserved; malformed known properties cannot be
    // silently accepted merely because their outer length fits the header.
    for header in [
        vec![1, 5, 1, 16, 1, 0x80, 0, 0],
        vec![1, 5, 1, 17, 1, 0xff, 0, 0],
        vec![1, 5, 1, 14, 0, 0, 0],
        vec![1, 5, 1, 18, 1, 1, 0, 0],
        vec![1, 2, 0],
        vec![1, 4, 7, 11, 1, 1, 0],
    ] {
        assert!(
            inspect_bytes(source(&header, &[]), &control).is_err(),
            "header {header:?}"
        );
    }
}

#[test]
fn sevenz_header_cancel_before_inspection() {
    let control = OperationControl::default();
    control.cancel();
    assert_eq!(
        inspect_bytes(source(&[1, 5, 0, 0, 0], &[]), &control).err(),
        Some(ArchiveError::Cancelled)
    );
}

// EncodedHeader StreamsInfo may omit the optional SubStreamsInfo: pinned
// 7-Zip/NanaZip define one unpack stream per folder in that case. Copy keeps
// this fixture independent of the Rust encoder and isolates the bridge itself.
fn hz4_encoded_metadata(substreams: bool, folder_crc: bool, pack_pos: u8) -> Vec<u8> {
    let raw = [1, 5, 0, 0, 0];
    let mut descriptor = vec![
        0x17,
        6,
        pack_pos,
        1,
        9,
        raw.len() as u8,
        0,
        7,
        11,
        1,
        0,
        1,
        1,
        0,
        12,
        raw.len() as u8,
    ];
    if folder_crc {
        descriptor.extend_from_slice(&[10, 1]);
        descriptor.extend_from_slice(&crc32fast::hash(&raw).to_le_bytes());
    }
    descriptor.push(0); // UnpackInfo end.
    if substreams {
        descriptor.extend_from_slice(&[8, 0]);
    }
    descriptor.push(0); // StreamsInfo end.
    let mut bytes = source(&descriptor, &[]);
    let packed: Vec<_> = std::iter::repeat_n(0xa5, usize::from(pack_pos))
        .chain(raw)
        .collect();
    bytes.splice(32..32, packed.iter().copied());
    bytes[12..20].copy_from_slice(&(packed.len() as u64).to_le_bytes());
    let crc = crc32fast::hash(&bytes[12..32]);
    bytes[8..12].copy_from_slice(&crc.to_le_bytes());
    bytes
}

#[test]
fn hz4_encoded_metadata_omitted_substreams_default_one() {
    for folder_crc in [false, true] {
        for pack_pos in [0, 7] {
            let facts = inspect_bytes(
                hz4_encoded_metadata(false, folder_crc, pack_pos),
                &OperationControl::default(),
            )
            .expect("optional absent SubStreamsInfo still has one encoded metadata stream");
            assert!(!facts.encrypted);
        }
    }
}

#[test]
fn hz4_encoded_metadata_explicit_substreams_control_and_crc() {
    for folder_crc in [false, true] {
        for pack_pos in [0, 7] {
            inspect_bytes(
                hz4_encoded_metadata(true, folder_crc, pack_pos),
                &OperationControl::default(),
            )
            .expect("explicit single metadata stream control");
        }
    }
    for substreams in [false, true] {
        let mut changed = hz4_encoded_metadata(substreams, true, 7);
        changed[32 + 7 + 2] ^= 1; // FilesInfo count, still structurally scannable.
        assert!(
            inspect_bytes(changed, &OperationControl::default()).is_err(),
            "defined encoded metadata CRC mismatch must reject"
        );
    }
}
