use crate::{ArchiveEntry, ArchiveFormat, ArchiveSnapshot, EntryId, EntryKind, NameProvenance};

pub(crate) fn snapshot(names: impl IntoIterator<Item = (String, EntryKind)>) -> ArchiveSnapshot {
    let session = uuid::Uuid::new_v4();
    let entries = names
        .into_iter()
        .enumerate()
        .map(|(ordinal, (name, kind))| ArchiveEntry {
            id: EntryId {
                session,
                ordinal: ordinal as u64,
            },
            raw_name: name.as_bytes().to_vec(),
            display_path: name,
            provenance: NameProvenance::FormatUnicode,
            kind,
            size: Some(0),
            compressed_size: Some(0),
            method: 0,
            encrypted: false,
            crc32: Some(0),
            dos_modified: None,
            metadata: Default::default(),
            warnings: Vec::new(),
            zip: None,
        })
        .collect::<Vec<_>>();
    ArchiveSnapshot {
        revision: crate::model::SourceRevision {
            path: "owned-memory-catalog.zip".into(),
            size: 0,
            modified: std::time::UNIX_EPOCH,
            digest: [0; 32],
            volumes: Vec::new(),
        },
        session,
        format: ArchiveFormat::Zip,
        capabilities: Default::default(),
        entries: entries.into(),
        comment: String::new(),
        backend_safe: true,
    }
}

pub(crate) fn large() -> (ArchiveSnapshot, Vec<EntryId>) {
    let mut names = Vec::with_capacity(65_536);
    for root in (0..4096).rev() {
        names.push((format!("r{root:04}/"), EntryKind::Directory));
        for file in 0..15 {
            names.push((format!("r{root:04}/f{file:02}"), EntryKind::File));
        }
    }
    let snapshot = snapshot(names);
    let ids = snapshot
        .entries
        .iter()
        .filter(|entry| entry.kind == EntryKind::Directory)
        .map(|entry| entry.id)
        .collect::<Vec<_>>();
    assert_eq!(snapshot.entries.len(), 65_536);
    assert_eq!(ids.len(), 4_096);
    (snapshot, ids)
}

pub(crate) fn delete_intent(ordinals: Vec<u64>) -> nxtools_protocol::archive_job::ArchiveJobModify {
    serde_json::from_value(serde_json::json!({
        "browser": {"snapshot_id": uuid::Uuid::new_v4(), "format":"Tar", "comment":"",
            "source":{"native":"v1:fixture", "digest":vec![0;32], "size":0, "modified_unix_ns":0}},
        "action":"delete", "ordinals":ordinals,
    }))
    .expect("original modify intent")
}
