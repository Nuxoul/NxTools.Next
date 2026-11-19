use super::*;
use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use tempfile::TempDir;

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sfx-coordinates")
}
fn manifest() -> serde_json::Value {
    serde_json::from_slice(&fs::read(fixture_root().join("manifest.json")).expect("manifest bytes"))
        .expect("qualified manifest JSON")
}
fn family(profile: &str) -> (TempDir, PathBuf) {
    let temp = tempfile::Builder::new()
        .prefix("b7-ref20-stub-")
        .tempdir()
        .expect("owned temporary family");
    let value = manifest();
    let row = value["profiles"]
        .as_array()
        .expect("profiles")
        .iter()
        .find(|p| p["profile"] == profile)
        .expect("qualified profile");
    for part in row["parts"].as_array().expect("parts") {
        let name = part["name"].as_str().expect("part name");
        fs::copy(fixture_root().join(name), temp.path().join(name)).expect("copy own fixture");
    }
    let first = temp.path().join(format!("{profile}.exe"));
    (temp, first)
}
fn put(path: &Path, mutate: impl FnOnce(&mut Vec<u8>)) {
    let mut bytes = fs::read(path).expect("owned bytes");
    mutate(&mut bytes);
    fs::write(path, bytes).expect("write owned mutation");
}
fn signature(bytes: &[u8], value: &[u8], last: bool) -> usize {
    let mut positions = bytes
        .windows(value.len())
        .enumerate()
        .filter(|(_, b)| *b == value)
        .map(|(i, _)| i);
    if last {
        positions.next_back().expect("last fixture signature")
    } else {
        positions.next().expect("fixture signature")
    }
}
fn rejected(path: &Path) {
    let result = Archive::open(path, Limits::default()).and_then(|mut a| a.test());
    assert!(result.is_err(), "damaged family must be refused");
}
fn exclusive(path: &Path) {
    let mut options = OpenOptions::new();
    options.read(true).write(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(0);
    }
    drop(
        options
            .open(path)
            .expect("exclusive reopen proves released file"),
    );
}
fn profile(profile: &str) {
    let (temp, path) = family(profile);
    let value = manifest();
    let row = value["profiles"]
        .as_array()
        .expect("profiles")
        .iter()
        .find(|p| p["profile"] == profile)
        .expect("profile");
    let mut archive =
        Archive::open(&path, Limits::default()).expect("supported qualified coordinate family");
    assert_eq!(archive.entries.len(), 4);
    for (index, (entry, expected)) in archive
        .entries
        .iter()
        .zip(row["entries"].as_array().expect("entries"))
        .enumerate()
    {
        assert_eq!(entry.ordinal, index);
        assert_eq!(entry.name, expected["name"].as_str().expect("name"));
        assert_eq!(entry.size, expected["size"].as_u64().expect("size"));
        assert_eq!(
            u64::from(entry.method),
            expected["method"].as_u64().expect("method")
        );
        assert_eq!(
            u64::from(entry.disk),
            expected["disk"].as_u64().expect("disk")
        );
        assert_eq!(entry.offset, expected["offset"].as_u64().expect("offset"));
        assert_eq!(entry.directory, index == 0);
    }
    let summary = Summary {
        files: 3,
        directories: 1,
        bytes: 8230,
    };
    assert_eq!(archive.test().expect("full read-only test"), summary);
    assert!(!temp.path().join("output").exists());
    let output = temp.path().join("output");
    assert_eq!(
        archive.extract(&output).expect("fresh-root extraction"),
        summary
    );
    for expected in value["expected"].as_array().expect("expected") {
        let name = expected["name"].as_str().expect("name");
        if name.ends_with('/') {
            assert!(output.join(name).is_dir());
        } else {
            assert_eq!(
                fs::read(output.join(name)).expect("extracted file"),
                fs::read(fixture_root().join(expected["path"].as_str().expect("expected path")))
                    .expect("oracle bytes")
            );
        }
    }
    drop(archive);
    for part in row["parts"].as_array().expect("parts") {
        let file = temp.path().join(part["name"].as_str().expect("name"));
        exclusive(&file);
        fs::remove_file(file).expect("delete after test/extract");
    }
}
#[test]
fn ordinary_store() {
    profile("ordinary-store");
}
#[test]
fn ordinary_deflate() {
    profile("ordinary-deflate");
}
#[test]
fn split_store() {
    profile("split-store");
}
#[test]
fn split_deflate() {
    profile("split-deflate");
}
#[test]
fn ordinary_zip64() {
    profile("ordinary-deflate-zip64");
}
#[test]
fn split_zip64() {
    profile("split-deflate-zip64");
}

#[test]
fn descriptor_and_pe_boundary_are_exact() {
    for (offset, byte) in [
        (512, 0),
        (528, 2),
        (532, 47),
        (536, 0),
        (540, 1),
        (544, 0),
        (552, 1),
        (296, 1),
    ] {
        let (_temp, path) = family("ordinary-store");
        put(&path, |b| b[offset] = byte);
        rejected(&path);
        exclusive(&path);
    }
    let (_temp, path) = family("ordinary-store");
    put(&path, |b| {
        b.splice(512..512, [0u8]);
    });
    rejected(&path);
}
#[test]
fn missing_reordered_corrupt_and_truncated_disks() {
    let (temp, path) = family("split-deflate");
    fs::remove_file(temp.path().join("split-deflate.z02")).expect("remove own disk");
    rejected(&path);
    exclusive(&path);
    let (temp, path) = family("split-deflate");
    let a = temp.path().join("split-deflate.z02");
    let b = temp.path().join("split-deflate.z03");
    let first = fs::read(&a).expect("disk a");
    let second = fs::read(&b).expect("disk b");
    fs::write(&a, second).expect("reorder a");
    fs::write(&b, first).expect("reorder b");
    rejected(&path);
    let (temp, path) = family("split-store");
    put(&temp.path().join("split-store.z05"), |b| b[12] ^= 128);
    rejected(&path);
    let (temp, path) = family("split-deflate-zip64");
    put(&temp.path().join("split-deflate-zip64.zip"), |b| {
        b.pop();
    });
    rejected(&path);
}
#[test]
fn local_central_and_descriptor_agreement() {
    for field in [6, 8, 10, 14, 18, 22, 30] {
        let (_temp, path) = family("ordinary-store");
        put(&path, |b| {
            let local = 613;
            b[local + field] ^= 1;
        });
        rejected(&path);
    }
    for wide in [false, true] {
        let (_temp, path) = family(if wide {
            "ordinary-deflate-zip64"
        } else {
            "ordinary-deflate"
        });
        put(&path, |b| {
            let descriptor = signature(b, b"PK\x07\x08", true);
            b[descriptor + 4] ^= 1;
        });
        rejected(&path);
    }
    let (_temp, path) = family("ordinary-deflate");
    put(&path, |b| {
        let start = 8872;
        let descriptor = start + 30 + 9 + 2;
        b[descriptor + 8] ^= 1;
    });
    rejected(&path);
}
#[test]
fn crc_failure_keeps_completed_files_and_cleans_only_partial() {
    let (temp, path) = family("ordinary-store");
    put(&path, |b| {
        b[8961] ^= 1;
    });
    let mut archive = Archive::open(&path, Limits::default()).expect("catalog still valid");
    assert_eq!(archive.test().expect_err("CRC failure").kind, Kind::Corrupt);
    let output = temp.path().join("out");
    assert_eq!(
        archive
            .extract(&output)
            .expect_err("CRC extraction failure")
            .kind,
        Kind::Corrupt
    );
    assert!(output.join("资料/data.bin").is_file());
    assert!(output.join("empty.txt").is_file());
    assert!(!output.join("hello.txt").exists());
    assert_eq!(fs::read_dir(&output).expect("output directory").count(), 2);
    drop(archive);
    exclusive(&path);
    fs::remove_file(path).expect("delete after failure");
}
#[test]
fn deflate_must_consume_all_packed_bytes() {
    let (_temp, path) = family("ordinary-deflate");
    put(&path, |b| {
        let central = signature(b, b"PK\x01\x02", true);
        let local = 8929;
        let packed =
            u32::from_le_bytes(b[local + 18..local + 22].try_into().expect("packed bytes"));
        let end = local + 30 + 9 + packed as usize;
        b[local + 18..local + 22].copy_from_slice(&(packed + 1).to_le_bytes());
        b[central + 20..central + 24].copy_from_slice(&(packed + 1).to_le_bytes());
        b[end + 8..end + 12].copy_from_slice(&(packed + 1).to_le_bytes());
        let eocd = b.len() - 22;
        let off = u32::from_le_bytes(
            b[eocd + 16..eocd + 20]
                .try_into()
                .expect("central coordinate"),
        );
        b[eocd + 16..eocd + 20].copy_from_slice(&(off + 1).to_le_bytes());
        b.insert(end, 0);
    });
    let mut archive = Archive::open(&path, Limits::default())
        .expect("consistent metadata with unused packed byte");
    assert_eq!(
        archive.test().expect_err("unused Deflate input").kind,
        Kind::Corrupt
    );
}
#[test]
fn names_are_refused_without_sanitizing() {
    for name in [
        "",
        "/a",
        "\\a",
        "C:a",
        "C:/a",
        "//server/a",
        "../a",
        "a/../b",
        "a/./b",
        "a//b",
        "a\\b",
        "a\0b",
        "a\nb",
        "NUL",
        "con.txt",
        "LPT1.bin",
        "COM9",
        "a.",
        "a ",
        "a/b:",
        "a//",
        "a/ /b",
    ] {
        assert_eq!(
            validate_names(&[(name.into(), name.ends_with('/'))], 4096)
                .expect_err("unsafe Windows path")
                .kind,
            Kind::UnsafeName,
            "{name:?}"
        );
    }
    validate_names(
        &[
            ("资料/".into(), true),
            ("资料/data.bin".into(), false),
            ("a..b".into(), false),
        ],
        4096,
    )
    .expect("valid names");
}
#[test]
fn collisions_include_case_and_file_prefixes() {
    for names in [
        vec![("a".into(), false), ("A".into(), false)],
        vec![("a".into(), false), ("a/b".into(), false)],
        vec![("a/b".into(), false), ("A".into(), false)],
        vec![("d/".into(), true), ("D/".into(), true)],
    ] {
        assert_eq!(
            validate_names(&names, 4096)
                .expect_err("catalog collision")
                .kind,
            Kind::Collision
        );
    }
}
#[test]
fn unsafe_catalog_has_no_output_effects() {
    let (temp, path) = family("ordinary-store");
    put(&path, |b| {
        for at in [8922 + 30, signature(b, b"PK\x01\x02", true) + 46] {
            b[at..at + 9].copy_from_slice(b"../aa.txt");
        }
    });
    assert_eq!(
        Archive::open(&path, Limits::default())
            .err()
            .expect("unsafe catalog")
            .kind,
        Kind::UnsafeName
    );
    assert!(!temp.path().join("out").exists());
}
#[test]
fn finite_limits_are_enforced() {
    for limits in [
        Limits {
            entries: 3,
            ..Limits::default()
        },
        Limits {
            metadata: 64,
            ..Limits::default()
        },
        Limits {
            name: 8,
            ..Limits::default()
        },
    ] {
        let (_temp, path) = family("ordinary-store");
        assert_eq!(
            Archive::open(&path, limits)
                .err()
                .expect("finite admission")
                .kind,
            Kind::Limit
        );
    }
    let (_temp, path) = family("split-store");
    assert_eq!(
        Archive::open(
            &path,
            Limits {
                disks: 2,
                ..Limits::default()
            }
        )
        .err()
        .expect("disk admission")
        .kind,
        Kind::Limit
    );
    assert_eq!(
        validate_names(&[("a".repeat(4097), false)], 4096)
            .expect_err("name admission")
            .kind,
        Kind::Limit
    );
}
#[test]
fn new_root_is_required_before_writes() {
    let (temp, path) = family("ordinary-store");
    let mut archive = Archive::open(&path, Limits::default()).expect("open");
    let output = temp.path().join("existing");
    fs::create_dir(&output).expect("existing root");
    fs::write(output.join("sentinel"), b"untouched").expect("sentinel");
    assert_eq!(
        archive.extract(&output).expect_err("existing root").kind,
        Kind::TargetExists
    );
    assert_eq!(
        fs::read(output.join("sentinel")).expect("sentinel"),
        b"untouched"
    );
    assert_eq!(fs::read_dir(&output).expect("directory").count(), 1);
}
#[test]
fn temporary_names_cannot_clobber_catalog_files() {
    let (temp, path) = family("ordinary-store");
    put(&path, |b| {
        for at in [8922 + 30, signature(b, b"PK\x01\x02", true) + 46] {
            b[at..at + 9].copy_from_slice(b"0.partial");
        }
    });
    let mut archive =
        Archive::open(&path, Limits::default()).expect("valid candidate-like catalog");
    archive
        .extract(&temp.path().join("out"))
        .expect("all files verified");
    assert_eq!(
        fs::read(temp.path().join("out/0.partial")).expect("member"),
        fs::read(fixture_root().join("expected/3.bin")).expect("expected")
    );
}
#[test]
fn encryption_methods_and_links_have_no_fallback() {
    for (field, value) in [(8, 99u16), (8, 12), (6, 0x809)] {
        let (_temp, path) = family("ordinary-store");
        put(&path, |b| {
            let central = signature(b, b"PK\x01\x02", false);
            b[560 + field..560 + field + 2].copy_from_slice(&value.to_le_bytes());
            let cfield = if field == 6 { 8 } else { 10 };
            b[central + cfield..central + cfield + 2].copy_from_slice(&value.to_le_bytes());
        });
        assert_eq!(
            Archive::open(&path, Limits::default())
                .err()
                .expect("unsupported profile")
                .kind,
            Kind::Unsupported
        );
    }
    let (_temp, path) = family("ordinary-store");
    put(&path, |b| {
        let central = signature(b, b"PK\x01\x02", true);
        b[central + 5] = 3;
        b[central + 38..central + 42].copy_from_slice(&(0o120777u32 << 16).to_le_bytes());
    });
    assert_eq!(
        Archive::open(&path, Limits::default())
            .err()
            .expect("symlink")
            .kind,
        Kind::Unsupported
    );
}
#[test]
fn final_end_disk_and_zip64_locator_must_match_descriptor() {
    for wide in [false, true] {
        let (temp, path) = family(if wide {
            "split-deflate-zip64"
        } else {
            "split-deflate"
        });
        let last = temp.path().join(if wide {
            "split-deflate-zip64.zip"
        } else {
            "split-deflate.zip"
        });
        put(&last, |b| {
            let end = b.len() - 22;
            b[end + 4..end + 6].copy_from_slice(&0u16.to_le_bytes());
        });
        rejected(&path);
    }
    let (_temp, path) = family("ordinary-deflate-zip64");
    put(&path, |b| {
        let locator = b.len() - 42;
        b[locator + 16..locator + 20].copy_from_slice(&2u32.to_le_bytes());
    });
    rejected(&path);
}
