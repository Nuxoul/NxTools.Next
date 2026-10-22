use nxtools_extensions::{ImportPreview, PackageError, PackageRegistry};
use std::{fs, path::Path};

const WEB: &[u8] = br#"{"schema_version":1,"id":"example.zip","name":"ZIP","version":"1","handlers":[{"id":"web","title":"Web","feature":"quick_launch","entry":"command","keywords":["zip"],"action":{"type":"website","url_template":"https://example.test/?q={query}"}}]}"#;
const SCRIPT: &[u8] = br#"{"schema_version":1,"id":"example.zip","name":"ZIP","version":"1","runtime":{"program":"python"},"files":["src/main.py"],"handlers":[{"id":"run","title":"Run","feature":"tools","entry":"action","action":{"type":"script","entry_file":"src/main.py"}}]}"#;

#[derive(Clone)]
struct Entry<'a> {
    name: &'a str,
    bytes: &'a [u8],
    mode: u32,
    flags: u16,
    method: u16,
    declared: Option<u32>,
}
fn entry<'a>(name: &'a str, bytes: &'a [u8]) -> Entry<'a> {
    Entry {
        name,
        bytes,
        mode: 0o100644,
        flags: 0x800,
        method: 0,
        declared: None,
    }
}
fn crc(bytes: &[u8]) -> u32 {
    let mut value = !0u32;
    for byte in bytes {
        value ^= u32::from(*byte);
        for _ in 0..8 {
            value = (value >> 1) ^ (0xedb88320 & 0u32.wrapping_sub(value & 1));
        }
    }
    !value
}
fn u16le(out: &mut Vec<u8>, value: u16) {
    out.extend(value.to_le_bytes());
}
fn u32le(out: &mut Vec<u8>, value: u32) {
    out.extend(value.to_le_bytes());
}
// Raw owned fixtures deliberately bypass ZipWriter's duplicate/type admission checks.
fn archive(path: &Path, entries: &[Entry<'_>]) {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for e in entries {
        let offset = out.len() as u32;
        u32le(&mut out, 0x04034b50);
        for v in [20, e.flags, e.method, 0, 0] {
            u16le(&mut out, v);
        }
        u32le(&mut out, crc(e.bytes));
        u32le(&mut out, e.bytes.len() as u32);
        u32le(&mut out, e.declared.unwrap_or(e.bytes.len() as u32));
        u16le(&mut out, e.name.len() as u16);
        u16le(&mut out, 0);
        out.extend(e.name.as_bytes());
        out.extend(e.bytes);
        u32le(&mut central, 0x02014b50);
        for v in [0x314, 20, e.flags, e.method, 0, 0] {
            u16le(&mut central, v);
        }
        u32le(&mut central, crc(e.bytes));
        u32le(&mut central, e.bytes.len() as u32);
        u32le(&mut central, e.declared.unwrap_or(e.bytes.len() as u32));
        for v in [e.name.len() as u16, 0, 0, 0, 0] {
            u16le(&mut central, v);
        }
        u32le(&mut central, e.mode << 16);
        u32le(&mut central, offset);
        central.extend(e.name.as_bytes());
    }
    let offset = out.len() as u32;
    let size = central.len() as u32;
    out.extend(central);
    u32le(&mut out, 0x06054b50);
    for v in [0, 0, entries.len() as u16, entries.len() as u16] {
        u16le(&mut out, v);
    }
    u32le(&mut out, size);
    u32le(&mut out, offset);
    u16le(&mut out, 0);
    fs::write(path, out).expect("owned ZIP");
}

#[test]
fn root_and_wrapper_snapshot_only_declared_files_and_preserve_old_hashes() {
    let temp = tempfile::tempdir().expect("fixture");
    let zip = temp.path().join("source.zip");
    let packages = temp.path().join("packages");
    for prefix in ["", "wrapper/"] {
        archive(
            &zip,
            &[
                entry(&format!("{prefix}extension.json"), WEB),
                entry(&format!("{prefix}README.md"), b"bounded extra"),
            ],
        );
        let preview = ImportPreview::from_archive(&zip).expect("declarative ZIP");
        assert!(!packages.exists(), "preview does not stage");
        assert_eq!(
            preview.content_hash,
            ImportPreview::from_json(WEB)
                .expect("legacy JSON")
                .content_hash
        );
        let staged = preview.stage_into(&packages).expect("declarative stage");
        let snapshot = staged
            .code_directory
            .as_ref()
            .expect("managed declarative snapshot");
        assert!(snapshot.join("extension.json").is_file());
        assert!(!snapshot.join("README.md").exists());
        let mut registry = PackageRegistry::default();
        registry
            .install(staged, "owned.zip".into(), false)
            .expect("install declarative snapshot");
        registry
            .validate()
            .expect("new and legacy metadata compatible");
        fs::remove_dir_all(&packages).expect("reset owned staging");
    }
    archive(
        &zip,
        &[
            entry("wrapper/extension.json", SCRIPT),
            entry("wrapper/src/main.py", b"never executed"),
            entry("wrapper/unlisted.exe", b"never staged"),
        ],
    );
    let preview = ImportPreview::from_archive(&zip).expect("script ZIP");
    fs::remove_file(&zip).expect("source removal before confirm");
    let staged = preview
        .stage_into(&packages)
        .expect("immutable bytes stage");
    let snapshot = staged.code_directory.expect("script snapshot");
    assert_eq!(
        fs::read(snapshot.join("src/main.py")).expect("code"),
        b"never executed"
    );
    assert!(!snapshot.join("unlisted.exe").exists());
}

#[test]
fn dangerous_names_anywhere_duplicates_aliases_collisions_and_roots_are_rejected_without_writes() {
    let temp = tempfile::tempdir().expect("fixture");
    let zip = temp.path().join("source.zip");
    for name in [
        "../escape",
        "/absolute",
        "C:/drive",
        "\\\\server\\share",
        "a:ads",
        "CON",
        "aux.txt",
        "COM1.txt",
        "LPT¹.txt",
        "dir./file",
        "file ",
        "a/../b",
        "a//b",
        "a/./b",
        "a\u{1}b",
        "AUX .txt",
        "CONIN$",
        "a<bad",
    ] {
        archive(&zip, &[entry("extension.json", WEB), entry(name, b"bad")]);
        assert!(
            ImportPreview::from_archive(&zip).is_err(),
            "unsafe name {name:?}"
        );
    }
    for entries in [
        vec![entry("extension.json", WEB), entry("extension.json", WEB)],
        vec![entry("extension.json", WEB), entry("EXTENSION.JSON", WEB)],
        vec![
            entry("extension.json", WEB),
            entry("a", b"file"),
            entry("a/b", b"child"),
        ],
        vec![
            entry("extension.json", WEB),
            entry("a/x", b"file"),
            entry("A/y", b"alias"),
        ],
        vec![
            entry("one/extension.json", WEB),
            entry("two/extension.json", WEB),
        ],
        vec![
            entry("extension.json", WEB),
            entry("nested/extension.json", WEB),
        ],
        vec![
            entry("one/extension.json", WEB),
            entry("outside.txt", b"other root"),
        ],
        vec![entry("one/two/extension.json", WEB)],
        vec![entry("readme", b"no manifest")],
        vec![entry("extension.json", SCRIPT)],
    ] {
        archive(&zip, &entries);
        assert!(
            ImportPreview::from_archive(&zip).is_err(),
            "ambiguous/invalid ZIP"
        );
    }
    assert_eq!(
        fs::read_dir(temp.path()).expect("owned directory").count(),
        1,
        "no extraction"
    );
}

#[test]
fn links_special_entries_encryption_unsupported_crc_and_forged_sizes_fail_closed() {
    let temp = tempfile::tempdir().expect("fixture");
    let zip = temp.path().join("source.zip");
    for mode in [0o120777, 0o020600, 0o060600, 0o010600, 0o140600] {
        let mut special = entry("extra", b"target");
        special.mode = mode;
        archive(&zip, &[entry("extension.json", WEB), special]);
        assert_eq!(
            ImportPreview::from_archive(&zip),
            Err(PackageError::ArchiveUnsafe)
        );
    }
    for (flags, method) in [(0x801, 0), (0x800, 99), (0x840, 0)] {
        let mut unsupported = entry("extra", b"data");
        unsupported.flags = flags;
        unsupported.method = method;
        archive(&zip, &[entry("extension.json", WEB), unsupported]);
        assert!(ImportPreview::from_archive(&zip).is_err());
    }
    archive(
        &zip,
        &[entry("extension.json", WEB), entry("extra", b"payload")],
    );
    let mut bytes = fs::read(&zip).expect("ZIP bytes");
    let at = bytes
        .windows(7)
        .position(|v| v == b"payload")
        .expect("payload offset");
    bytes[at] ^= 1;
    fs::write(&zip, bytes).expect("corrupt ignored extra CRC");
    assert_eq!(
        ImportPreview::from_archive(&zip),
        Err(PackageError::ArchiveInvalid)
    );
    let mut oversized = entry("extra", b"small");
    oversized.declared = Some(256 * 1024 * 1024 + 1);
    archive(&zip, &[entry("extension.json", WEB), oversized]);
    assert_eq!(
        ImportPreview::from_archive(&zip),
        Err(PackageError::ArchiveTooLarge)
    );
    let mut forged = entry("extra", b"actual bytes");
    forged.declared = Some(1);
    archive(&zip, &[entry("extension.json", WEB), forged]);
    assert_eq!(
        ImportPreview::from_archive(&zip),
        Err(PackageError::ArchiveInvalid)
    );
    fs::write(&zip, b"not zip").expect("invalid bytes");
    assert_eq!(
        ImportPreview::from_archive(&zip),
        Err(PackageError::ArchiveInvalid)
    );
}

#[test]
fn archive_file_and_directory_counts_and_sparse_input_are_bounded() {
    let temp = tempfile::tempdir().expect("fixture");
    let zip = temp.path().join("source.zip");
    let names: Vec<_> = (0..4097).map(|n| format!("file-{n}")).collect();
    let mut entries = vec![entry("extension.json", WEB)];
    entries.extend(names.iter().map(|name| entry(name, b"")));
    archive(&zip, &entries);
    assert_eq!(
        ImportPreview::from_archive(&zip),
        Err(PackageError::ArchiveTooLarge)
    );
    let names: Vec<_> = (0..4097).map(|n| format!("dir-{n}/")).collect();
    let mut entries = vec![entry("extension.json", WEB)];
    entries.extend(names.iter().map(|name| {
        let mut e = entry(name, b"");
        e.mode = 0o040755;
        e
    }));
    archive(&zip, &entries);
    assert_eq!(
        ImportPreview::from_archive(&zip),
        Err(PackageError::ArchiveTooLarge)
    );
    fs::File::create(&zip)
        .expect("sparse archive")
        .set_len(256 * 1024 * 1024 + 1)
        .expect("bounded sparse fixture");
    assert_eq!(
        ImportPreview::from_archive(&zip),
        Err(PackageError::ArchiveTooLarge)
    );
}

#[test]
fn released_zip_writer_deflate_unicode_directories_and_streaming_forgery() {
    use std::io::Write;
    use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};
    let temp = tempfile::tempdir().expect("fixture");
    let path = temp.path().join("deflated.zip");
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut writer = ZipWriter::new(fs::File::create(&path).expect("owned ZIP"));
    writer.add_directory("包/", options).expect("wrapper");
    writer
        .start_file("包/extension.json", options)
        .expect("manifest entry");
    writer.write_all(WEB).expect("manifest bytes");
    writer
        .start_file("包/说明.md", options)
        .expect("Unicode extra");
    writer
        .write_all(b"bounded ignored documentation")
        .expect("extra");
    writer.finish().expect("finish ZIP");
    ImportPreview::from_archive(&path).expect("released ZIP writer compatibility");

    let mut writer = ZipWriter::new(fs::File::create(&path).expect("forged fixture"));
    writer
        .start_file("extension.json", options)
        .expect("compressed manifest");
    writer.write_all(WEB).expect("manifest");
    writer
        .start_file("extra", options)
        .expect("compressed extra");
    writer
        .write_all(&vec![0; 2 * 1024 * 1024])
        .expect("compressible owned bytes");
    writer.finish().expect("finish fixture");
    let mut bytes = fs::read(&path).expect("ZIP bytes");
    let central = bytes
        .windows(4)
        .enumerate()
        .filter_map(|(at, signature)| (signature == b"PK\x01\x02").then_some(at))
        .next_back()
        .expect("extra central entry");
    let local = u32::from_le_bytes(
        bytes[central + 42..central + 46]
            .try_into()
            .expect("local offset"),
    ) as usize;
    bytes[central + 24..central + 28].copy_from_slice(&1u32.to_le_bytes());
    bytes[local + 22..local + 26].copy_from_slice(&1u32.to_le_bytes());
    fs::write(&path, bytes).expect("forged metadata");
    assert_eq!(
        ImportPreview::from_archive(&path),
        Err(PackageError::ArchiveInvalid),
        "actual streamed bytes cannot hide behind forged uncompressed size"
    );
}

#[test]
fn corrupt_manifest_strict_json_and_local_name_mismatch_never_preview() {
    let temp = tempfile::tempdir().expect("fixture");
    let path = temp.path().join("invalid.zip");
    for manifest in [
        b"broken".to_vec(),
        String::from_utf8_lossy(WEB)
            .replace("\"version\":\"1\"", "\"version\":\"1\",\"version\":\"2\"")
            .into_bytes(),
        String::from_utf8_lossy(WEB)
            .replace("\"schema_version\":1", "\"schema_version\":2")
            .into_bytes(),
    ] {
        archive(&path, &[entry("extension.json", &manifest)]);
        assert!(ImportPreview::from_archive(&path).is_err());
    }
    archive(&path, &[entry("extension.json", WEB)]);
    let mut bytes = fs::read(&path).expect("ZIP bytes");
    bytes[30] = b'X';
    fs::write(&path, bytes).expect("mismatched local name");
    assert_eq!(
        ImportPreview::from_archive(&path),
        Err(PackageError::ArchiveInvalid)
    );
}

#[test]
fn stage_failure_never_overwrites_preexisting_snapshots_or_unrelated_directories() {
    let temp = tempfile::tempdir().expect("fixture");
    let path = temp.path().join("source.zip");
    let packages = temp.path().join("packages");
    archive(&path, &[entry("extension.json", WEB)]);
    let preview = ImportPreview::from_archive(&path).expect("preview");
    let staged = preview.stage_into(&packages).expect("snapshot");
    let snapshot = staged.code_directory.expect("snapshot path");
    fs::write(
        snapshot.join("extension.json"),
        b"preexisting invalid snapshot",
    )
    .expect("owned tamper");
    let unrelated = packages.join("unrelated");
    fs::create_dir(&unrelated).expect("unrelated directory");
    fs::write(unrelated.join("keep"), b"do not touch").expect("unrelated bytes");
    assert!(preview.stage_into(&packages).is_err());
    assert_eq!(
        fs::read(snapshot.join("extension.json")).expect("existing snapshot unchanged"),
        b"preexisting invalid snapshot"
    );
    assert_eq!(
        fs::read(unrelated.join("keep")).expect("unrelated unchanged"),
        b"do not touch"
    );
    assert!(
        fs::read_dir(snapshot.parent().expect("owner directory"))
            .expect("entries")
            .all(|entry| !entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .starts_with(".stage-"))
    );
}
