use super::*;
use crate::mft::allocation_probe::measure;

#[allow(unsafe_code)]
fn process_memory() -> (usize, usize) {
    use std::mem::size_of;
    use windows_sys::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX,
    };
    let mut counters = PROCESS_MEMORY_COUNTERS_EX {
        cb: size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
        ..Default::default()
    };
    // SAFETY: The initialized extended structure contains the base counters prefix and
    // remains writable for the supplied extended size throughout this synchronous call.
    assert_ne!(
        unsafe {
            GetProcessMemoryInfo(
                GetCurrentProcess(),
                (&raw mut counters).cast::<PROCESS_MEMORY_COUNTERS>(),
                size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
            )
        },
        0
    );
    (counters.WorkingSetSize, counters.PrivateUsage)
}

#[test]
#[ignore = "synthetic million-row owned versus mapped snapshot memory measurement"]
fn l11_million_row_owned_and_mapped_memory() {
    let directory = tempfile::tempdir().expect("isolated memory measurement");
    let path = directory.path().join("synthetic.idx");
    let before_owned = process_memory();
    let (owned, owned_allocations) = measure(|| {
        MftSnapshot::from_entries(
            r"C:\",
            Some(17),
            Some(100),
            (0..1_000_000_u64)
                .map(|row| MftEntry {
                    identity: Some(MftIdentity {
                        file_reference: row + 1_000,
                        parent_reference: 5,
                    }),
                    path: format!(r"C:\synthetic-{row:07}.txt"),
                    name: format!("synthetic-{row:07}.txt"),
                    is_directory: false,
                    size_bytes: None,
                    modified_at_unix_ms: Some(1),
                })
                .collect(),
        )
    });
    let after_owned = process_memory();
    persist_snapshot(&owned, &path).expect("persist synthetic base");
    let file_bytes = fs::metadata(&path).expect("synthetic base size").len();
    drop(owned);
    let before_mapped = process_memory();
    let (mapped, mapped_allocations) = measure(|| {
        let (mapped, journal, cursor, visible) =
            MappedSnapshot::open(&path, r"C:\").expect("map base");
        MftSnapshot::from_mapped(r"C:\".into(), journal, cursor, visible, mapped, path)
    });
    let after_mapped = process_memory();
    let touched_bytes = (0..mapped.base_len())
        .map(|row| {
            let entry = mapped.base_entry(row).expect("mapped row");
            entry.path.len() + entry.name.len()
        })
        .sum::<usize>();
    std::hint::black_box(touched_bytes);
    let after_touch = process_memory();
    assert_eq!(mapped.visible_len(), 1_000_000);
    eprintln!(
        "L11 rows=1000000 file_bytes={file_bytes} owned_allocator={owned_allocations:?} mapped_allocator={mapped_allocations:?} process_bytes=(working_set,private_commit) before_owned={before_owned:?} after_owned={after_owned:?} before_mapped={before_mapped:?} after_mapped={after_mapped:?} after_row_iteration={after_touch:?}"
    );
}
