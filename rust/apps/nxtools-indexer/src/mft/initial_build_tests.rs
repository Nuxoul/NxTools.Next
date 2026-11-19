#![allow(unsafe_code)]

use super::*;
use std::fs::File;
use std::os::windows::io::AsRawHandle;
use windows_sys::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct FileIdentityAndTime {
    pub(super) reference: u64,
    pub(super) modified: Option<i64>,
}

pub(super) fn system_file_metadata(path: &Path) -> FileIdentityAndTime {
    let file = File::open(path).expect("open a read-only system-file witness");
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: file owns the live handle; output has the API's exact structure and size.
    assert_ne!(
        unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) },
        0
    );
    let ticks = (u64::from(info.ftLastWriteTime.dwHighDateTime) << 32)
        | u64::from(info.ftLastWriteTime.dwLowDateTime);
    FileIdentityAndTime {
        reference: (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
        modified: time_metadata::filetime_to_unix_ms(ticks as i64),
    }
}

#[test]
#[ignore = "requires explicit C: read-only scan authorization and an elevated test process"]
fn initial_build_native_avoids_per_file_metadata_calls() {
    let mount = std::env::var("NXTOOLS_INITIAL_SCAN_VOLUME")
        .expect("set the explicitly authorized scan volume");
    assert_eq!(mount, r"C:\", "this fixture is authorized only for C:");
    let system_root = PathBuf::from(std::env::var_os("SystemRoot").expect("Windows system root"));
    assert!(
        system_root
            .to_string_lossy()
            .to_ascii_lowercase()
            .starts_with(r"c:\")
    );
    let witnesses = ["kernel32.dll", "cmd.exe", "notepad.exe"].map(|name| {
        let path = system_root.join("System32").join(name);
        let before = system_file_metadata(&path);
        (path, before)
    });
    let reads_before = time_metadata::attribute_read_count();
    let fallbacks_before = super::bulk_times::fallback_read_count();
    let started = Instant::now();
    let snapshot = scan_volume(&mount).expect("authorized read-only initial volume scan");
    let elapsed_ms = started.elapsed().as_millis();
    let attribute_reads = time_metadata::attribute_read_count() - reads_before;
    let freshness_fallbacks = super::bulk_times::fallback_read_count() - fallbacks_before;
    assert!(snapshot.visible_len() > 0);
    let mut verified = 0;
    for (path, before) in witnesses {
        if system_file_metadata(&path) != before {
            continue;
        }
        let entry = (0..snapshot.base_len())
            .filter_map(|row| snapshot.base_entry(row))
            .find(|entry| {
                entry
                    .identity
                    .is_some_and(|identity| identity.file_reference == before.reference)
            })
            .expect("stable system-file reference remains in the initial snapshot");
        assert_eq!(
            entry.modified_at_unix_ms, before.modified,
            "true LastWriteTime must match the independent handle API"
        );
        verified += 1;
    }
    assert!(
        verified > 0,
        "at least one independent system-file witness must remain stable"
    );
    println!(
        "INITIAL_SCAN_NATIVE rows={} attribute_reads={attribute_reads} freshness_fallbacks={freshness_fallbacks} elapsed_ms={elapsed_ms} verified_system_files={verified}",
        snapshot.visible_len()
    );
    assert_eq!(
        attribute_reads, freshness_fallbacks,
        "only raw/enumerated USN freshness exceptions may issue per-file attribute reads"
    );
}
