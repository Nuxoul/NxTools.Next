use nxtools_protocol::{
    archive_browser::{ArchiveBrowserContext, ArchiveSourceBinding},
    archive_read::{ArchiveReadBatch, ArchiveReadEntry, ArchiveReadError},
};
use uuid::Uuid;

fn batch(format: &str) -> ArchiveReadBatch {
    ArchiveReadBatch {
        snapshot_id: Uuid::new_v4(),
        sequence: 0,
        format: format.into(),
        entries: vec![ArchiveReadEntry {
            ordinal: 0,
            name: "text.txt".into(),
            kind: "File".into(),
            size: Some(3600),
            packed_size: Some(3600),
            encrypted: Some(false),
            warning_codes: vec![],
            provenance: "FormatUnicode".into(),
            codepage: Some(65001),
            modified: None,
            method: Some(0),
            crc32: Some(0x2c19aec0),
            attributes: None,
        }],
    }
}
fn browser(format: &str) -> ArchiveBrowserContext {
    ArchiveBrowserContext {
        snapshot_id: Uuid::new_v4(),
        format: format.into(),
        comment: String::new(),
        source: ArchiveSourceBinding {
            native: "v1:owned-native-family".into(),
            digest: [1; 32],
            size: 160200,
            modified_unix_ns: 1,
        },
        sevenz_edit: None,
        tar_edit: None,
    }
}

#[test]
fn ref19_read_batches_accept_actual_alz_and_egg_labels() {
    for format in ["Alz", "Egg"] {
        let batch = batch(format);
        for version in [1, 2] {
            assert!(batch.validate_for(version).is_ok(), "{format} v{version}");
        }
        let encoded = serde_json::to_vec(&batch).expect("public batch encoding");
        let decoded: ArchiveReadBatch =
            serde_json::from_slice(&encoded).expect("public batch decoding");
        assert_eq!(decoded, batch);
    }
}
#[test]
fn ref19_browser_context_keeps_complete_source_binding() {
    for format in ["Alz", "Egg"] {
        let browser = browser(format);
        assert_eq!(browser.validate(), Ok(()), "{format}");
        let encoded = serde_json::to_vec(&browser).expect("Browser encoding");
        let decoded: ArchiveBrowserContext =
            serde_json::from_slice(&encoded).expect("Browser decoding");
        assert_eq!(decoded, browser);
        assert!(decoded.sevenz_edit.is_none() && decoded.tar_edit.is_none());
    }
}
#[test]
fn ref19_auto_cp949_metadata_keeps_heuristic_provenance() {
    let mut batch = batch("Zip");
    batch.entries[0].name = "뷁테스트.txt".into();
    batch.entries[0].provenance = "Heuristic".into();
    batch.entries[0].codepage = Some(949);
    batch.entries[0].warning_codes = vec!["UncertainEncoding".into()];
    for version in [1, 2] {
        assert!(batch.validate_for(version).is_ok());
    }
    let encoded = serde_json::to_vec(&batch).expect("Auto metadata encoding");
    let decoded: ArchiveReadBatch =
        serde_json::from_slice(&encoded).expect("Auto metadata decoding");
    assert_eq!(decoded.entries[0].provenance, "Heuristic");
    assert_eq!(decoded.entries[0].codepage, Some(949));
}
#[test]
fn ref19_unknown_format_and_heuristic_codepage_remain_rejected() {
    assert_eq!(
        batch("Unknown").validate(),
        Err(ArchiveReadError::ProtocolViolation)
    );
    assert_eq!(
        browser("Unknown").validate(),
        Err(ArchiveReadError::ProtocolViolation)
    );
    for codepage in [None, Some(42), Some(437), Some(950)] {
        let mut batch = batch("Zip");
        batch.entries[0].provenance = "Heuristic".into();
        batch.entries[0].codepage = codepage;
        assert_eq!(batch.validate(), Err(ArchiveReadError::ProtocolViolation));
    }
}
