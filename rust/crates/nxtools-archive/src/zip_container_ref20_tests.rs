use super::*;
use crate::{ArchiveEngine, ArchiveRequest, NameEncoding, OpenRequest};
use serde_json::Value;

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sfx-coordinates")
}
fn manifest() -> Value {
    serde_json::from_slice(
        &std::fs::read(fixture_root().join("manifest.json")).expect("qualified manifest"),
    )
    .expect("manifest")
}
fn expected() -> Vec<(String, Vec<u8>)> {
    manifest()["expected"]
        .as_array()
        .expect("expected members")
        .iter()
        .map(|entry| {
            (
                entry["name"].as_str().expect("name").to_owned(),
                std::fs::read(fixture_root().join(entry["path"].as_str().expect("expected path")))
                    .expect("qualified content"),
            )
        })
        .collect()
}
fn exact_payloads(file: &mut File, control: &OperationControl) {
    let limits = ResourceLimits::default();
    let (entries, _, safe) = crate::formats::zip::list(file, NameEncoding::Auto, &limits, control)
        .expect("original ZIP catalog");
    assert!(safe);
    let expected = expected();
    assert_eq!(entries.len(), expected.len());
    let mut reader =
        crate::formats::zip::ValidatedZip::new(file, &entries, &limits, control, false)
            .expect("original record preflight");
    let mut total = 0;
    for (entry, (name, bytes)) in entries.iter().zip(expected) {
        assert_eq!(entry.display_path(), name);
        let mut actual = Vec::new();
        reader
            .stream(
                entry.id.ordinal,
                &limits,
                control,
                None,
                &mut total,
                |chunk| {
                    actual.extend_from_slice(chunk);
                    Ok(())
                },
            )
            .expect("original payload stream");
        assert_eq!(actual, bytes, "exact member {name}");
    }
    assert_eq!(total, 8230);
}
fn read_profile(name: &str) {
    let value = manifest();
    let profile = value["profiles"]
        .as_array()
        .expect("profiles")
        .iter()
        .find(|profile| profile["profile"] == name)
        .expect("qualified profile");
    let parts: Vec<String> = profile["parts"]
        .as_array()
        .expect("parts")
        .iter()
        .map(|part| {
            fixture_root()
                .join(part["name"].as_str().expect("part"))
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    for entry in [parts.first().expect("first"), parts.last().expect("last")] {
        let control = OperationControl::default();
        let engine = ArchiveEngine::default().with_held_zip_volumes(&parts);
        let snapshot = engine
            .open(OpenRequest::new(entry), &control)
            .expect("qualified own SFX catalog");
        assert_eq!(snapshot.entries().len(), 4);
        let capabilities = snapshot.capabilities();
        assert!(capabilities.test && capabilities.extract);
        assert!(
            !capabilities.modify
                && !capabilities.add
                && !capabilities.delete
                && !capabilities.rename_file
        );
        assert_eq!(capabilities.split_read, parts.len() > 1);
        let mut input =
            crate::engine::open_snapshot(&snapshot, &ResourceLimits::default(), &control)
                .expect("same full source revision");
        exact_payloads(&mut input, &control);
        let outcome = engine
            .execute(ArchiveRequest::test(snapshot), &control)
            .expect("original SFX Test");
        assert_eq!(outcome.bytes_verified(), 8230);
        assert_eq!(outcome.files_verified(), 3);
    }
}
#[test]
fn ordinary_store_coordinates() {
    read_profile("ordinary-store");
}
#[test]
fn ordinary_deflate_coordinates() {
    read_profile("ordinary-deflate");
}
#[test]
fn ordinary_zip64_coordinates() {
    read_profile("ordinary-deflate-zip64");
}
#[test]
fn split_store_coordinates() {
    read_profile("split-store");
}
#[test]
fn split_deflate_coordinates() {
    read_profile("split-deflate");
}
#[test]
fn split_zip64_coordinates() {
    read_profile("split-deflate-zip64");
}

fn encoded(method: ::zip::CompressionMethod, wide: bool) -> File {
    let mut file = tempfile::tempfile().expect("encoded source");
    let mut writer = ::zip::ZipWriter::new_stream(&mut file);
    for (name, bytes) in expected() {
        let settings = ::zip::write::SimpleFileOptions::default()
            .compression_method(if name.ends_with('/') {
                ::zip::CompressionMethod::Stored
            } else {
                method
            })
            .large_file(wide && !name.ends_with('/'));
        writer
            .start_file(name, settings)
            .expect("original encoder member");
        writer.write_all(&bytes).expect("literal member bytes");
    }
    writer.finish().expect("original encoder finish");
    file.rewind().expect("encoded rewind");
    exact_payloads(&mut file, &OperationControl::default());
    file
}
fn layout(split: bool, wide: bool) {
    let control = OperationControl::default();
    let stub = include_bytes!("../assets/zip-sfx-x64.bin");
    let maximum = stub.len() as u64 + 48 + 256;
    for method in [
        ::zip::CompressionMethod::Stored,
        ::zip::CompressionMethod::Deflated,
    ] {
        let mut input = encoded(method, wide);
        let mut files = sfx_encoded(
            &mut input,
            split.then_some(maximum),
            &ResourceLimits::default(),
            &control,
        )
        .expect("coordinate-aware SFX layout");
        assert_eq!(files.len() > 1, split);
        assert!(files[0].metadata().expect("first length").len() <= u64::from(u32::MAX));
        if split {
            assert!(
                files
                    .iter()
                    .all(|file| file.metadata().expect("length").len() <= maximum)
            );
        }
        let mut first = vec![0; stub.len() + 48];
        files[0].rewind().expect("first rewind");
        files[0]
            .read_exact(&mut first)
            .expect("complete known prefix");
        assert_eq!(&first[..stub.len()], stub);
        let descriptor = &first[stub.len()..];
        assert_eq!(&descriptor[..16], b"NXTOOLS-ZIP-SFX1");
        assert_eq!(dword(descriptor, 24), files.len() as u32);
        assert_eq!(qword(descriptor, 32), (stub.len() + 48) as u64);
        let mut set =
            candidate_set(&files, &ResourceLimits::default()).expect("held candidate set");
        let mut normalized = normalize(&mut set, &ResourceLimits::default(), &control)
            .expect("original physical coordinate normalization");
        exact_payloads(&mut normalized, &control);
        crate::zip_edit::verify(&mut normalized, &ResourceLimits::default(), &control, None)
            .expect("full original final candidate verification");
    }
}
#[test]
fn ordinary_layout_preserves_store_deflate() {
    layout(false, false);
}
#[test]
fn split_layout_crosses_payload_and_keeps_complete_prefix() {
    layout(true, false);
}
#[test]
fn zip64_local_layout_preserves_store_deflate() {
    layout(true, true);
}
#[test]
fn complete_prefix_and_loader_limits_are_admitted_before_layout() {
    let prefix = include_bytes!("../assets/zip-sfx-x64.bin").len() as u64 + 48;
    let mut input = encoded(::zip::CompressionMethod::Stored, false);
    for maximum in [prefix + 3, u64::from(u32::MAX) + 1] {
        assert!(matches!(
            sfx_encoded(
                &mut input,
                Some(maximum),
                &ResourceLimits::default(),
                &OperationControl::default()
            ),
            Err(ArchiveError::ResourceLimit)
        ));
    }
    let limits = ResourceLimits {
        max_output_bytes: prefix - 1,
        ..ResourceLimits::default()
    };
    assert!(matches!(
        sfx_encoded(&mut input, None, &limits, &OperationControl::default()),
        Err(ArchiveError::ResourceLimit)
    ));
}
#[test]
fn layout_observes_original_cancellation() {
    let mut input = encoded(::zip::CompressionMethod::Stored, false);
    let control = OperationControl::default();
    control.cancel();
    assert!(matches!(
        sfx_encoded(&mut input, None, &ResourceLimits::default(), &control),
        Err(ArchiveError::Cancelled)
    ));
}
#[test]
fn missing_middle_is_typed_without_unleased_fallback() {
    let root = tempfile::tempdir().expect("owned family");
    let value = manifest();
    let profile = value["profiles"]
        .as_array()
        .expect("profiles")
        .iter()
        .find(|profile| profile["profile"] == "split-store")
        .expect("profile");
    for part in profile["parts"].as_array().expect("parts") {
        let name = part["name"].as_str().expect("part");
        if !name.ends_with(".z02") {
            std::fs::copy(fixture_root().join(name), root.path().join(name))
                .expect("owned data-only part");
        }
    }
    let result = ArchiveEngine::default().with_zip_volumes().open(
        OpenRequest::new(root.path().join("split-store.zip")),
        &OperationControl::default(),
    );
    assert!(matches!(result, Err(ArchiveError::MissingVolume)));
}
#[test]
fn held_order_cannot_discover_unadmitted_siblings() {
    let first = fixture_root().join("split-store.exe");
    let held = [
        first.to_string_lossy().into_owned(),
        fixture_root()
            .join("split-store.zip")
            .to_string_lossy()
            .into_owned(),
    ];
    let result = ArchiveEngine::default()
        .with_held_zip_volumes(&held)
        .open(OpenRequest::new(&first), &OperationControl::default());
    assert!(matches!(
        result,
        Err(ArchiveError::MissingVolume | ArchiveError::SourceChanged)
    ));
}
#[test]
fn own_descriptor_corruption_is_not_an_arbitrary_prefixed_zip() {
    let root = tempfile::tempdir().expect("owned corruption");
    let mut bytes =
        std::fs::read(fixture_root().join("ordinary-store.exe")).expect("qualified source");
    bytes[512 + 16] = 2;
    let path = root.path().join("invalid.exe");
    std::fs::write(&path, bytes).expect("owned malformed data");
    let result = ArchiveEngine::default()
        .with_zip_volumes()
        .open(OpenRequest::new(path), &OperationControl::default());
    assert!(matches!(result, Err(ArchiveError::CorruptArchive)));
}

#[test]
fn stream_directory_has_truthful_recipient_attributes() {
    let control = OperationControl::default();
    let limits = ResourceLimits::default();
    let mut input = encoded(::zip::CompressionMethod::Stored, false);
    let files = sfx_encoded(&mut input, None, &limits, &control).expect("SFX layout");
    let mut set = candidate_set(&files, &limits).expect("complete candidate set");
    let mut normalized = normalize(&mut set, &limits, &control).expect("original ZIP records");
    exact_payloads(&mut normalized, &control);
    let set = candidate_set(&[normalized.try_clone().expect("held normalized")], &limits)
        .expect("normalized set");
    let catalog = directory(&mut normalized, &set, &limits, &control).expect("central records");
    let directories: Vec<_> = catalog
        .records
        .iter()
        .filter(|row| {
            let size = usize::from(word(&row.central, 28));
            row.central[46..46 + size].ends_with(b"/")
        })
        .collect();
    assert_eq!(directories.len(), 1);
    let attributes = dword(&directories[0].central, 38);
    assert_eq!(
        (attributes >> 16) & 0xf000,
        0x4000,
        "recipient directory type"
    );
    assert_ne!(attributes & 0x10, 0, "DOS directory attribute");
}
