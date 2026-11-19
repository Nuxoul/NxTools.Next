use super::*;
use std::fs;

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sfx-coordinates")
}
fn family() -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::Builder::new()
        .prefix("b7-ref20-stub-publication-")
        .tempdir()
        .expect("owned temporary directory");
    let path = temp.path().join("ordinary-store.exe");
    fs::copy(fixture_root().join("ordinary-store.exe"), &path).expect("copy qualified family");
    (temp, path)
}

#[test]
fn verified_file_publication_never_replaces_an_existing_final() {
    let (temp, path) = family();
    let mut archive = Archive::open(&path, Limits::default()).expect("qualified catalog");
    let root = temp.path().join("out");
    fs::create_dir(&root).expect("owned output");
    fs::write(root.join("hello.txt"), b"preserve existing bytes").expect("existing final");
    let catalog = archive
        .entries
        .iter()
        .map(|e| names::key(&e.name))
        .collect();
    let result = extract_file(&mut archive.parts, &archive.entries[3], &root, &catalog);
    assert!(
        result.is_err(),
        "no-replace publication must refuse the existing final"
    );
    assert_eq!(
        fs::read(root.join("hello.txt")).expect("existing bytes"),
        b"preserve existing bytes"
    );
    assert_eq!(fs::read_dir(root).expect("output").count(), 1);
}

#[test]
fn actual_candidate_names_and_implicit_directories_are_reserved() {
    let (temp, path) = family();
    let mut archive = Archive::open(&path, Limits::default()).expect("qualified catalog");
    let root = temp.path().join("out");
    fs::create_dir(&root).expect("owned output");
    let mut catalog: BTreeSet<_> = archive
        .entries
        .iter()
        .map(|e| names::key(&e.name))
        .collect();
    catalog.insert(names::key(".nxtools-sfx-3-0.partial"));
    catalog.insert(names::key(".nxtools-sfx-3-1.partial/child"));
    fs::write(root.join(".nxtools-sfx-3-0.partial"), b"catalog member").expect("member");
    fs::create_dir(root.join(".nxtools-sfx-3-1.partial")).expect("implicit directory");
    extract_file(&mut archive.parts, &archive.entries[3], &root, &catalog)
        .expect("skip reserved candidates");
    assert_eq!(
        fs::read(root.join(".nxtools-sfx-3-0.partial")).expect("catalog member"),
        b"catalog member"
    );
    assert!(root.join(".nxtools-sfx-3-1.partial").is_dir());
    assert_eq!(
        fs::read(root.join("hello.txt")).expect("verified member"),
        fs::read(fixture_root().join("expected/3.bin")).expect("expected")
    );
    assert!(!root.join(".nxtools-sfx-3-2.partial").exists());
}

#[test]
fn special_types_are_refused_for_all_creator_hosts() {
    for host in [0, 3, 19] {
        let (_temp, path) = family();
        let mut bytes = fs::read(&path).expect("owned bytes");
        let central = bytes
            .windows(4)
            .rposition(|b| b == b"PK\x01\x02")
            .expect("central record");
        bytes[central + 5] = host;
        bytes[central + 38..central + 42].copy_from_slice(&(0o120777u32 << 16).to_le_bytes());
        fs::write(&path, bytes).expect("owned mutation");
        assert_eq!(
            Archive::open(&path, Limits::default())
                .err()
                .expect("symlink refusal")
                .kind,
            Kind::Unsupported
        );
    }
}
