use super::*;
use crate::mft::{RawMftRecord, hydrate_record_times_while};

const REFERENCE: u64 = (3_u64 << 48) | 1;
const EPOCH: u64 = 116_444_736_000_000_000;

fn put16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}
fn put32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
fn put64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
fn file_record(reference: u64) -> Vec<u8> {
    let mut record = vec![0; 1024];
    record[..4].copy_from_slice(b"FILE");
    put16(&mut record, 4, 48);
    put16(&mut record, 6, 3);
    put16(&mut record, 16, (reference >> 48) as u16);
    put16(&mut record, 20, 56);
    put16(&mut record, 22, 1);
    put32(&mut record, 24, 160);
    put32(&mut record, 28, 1024);
    put32(&mut record, 44, reference as u32);
    put16(&mut record, 48, 0xabcd);
    put16(&mut record, 50, 0x1122);
    put16(&mut record, 52, 0x3344);
    put32(&mut record, 56, 0x10);
    put32(&mut record, 60, 96);
    put32(&mut record, 72, 72);
    put16(&mut record, 76, 24);
    // Creation, write, MFT-change and access times must not be confused.
    for (offset, ms) in [(80, 11), (88, 12345), (96, 33), (104, 44)] {
        put64(&mut record, offset, EPOCH + ms * 10000);
    }
    put32(&mut record, 152, u32::MAX);
    put16(&mut record, 510, 0xabcd);
    put16(&mut record, 1022, 0xabcd);
    record
}

struct Bytes {
    disk: Vec<u8>,
    map: Vec<Extent>,
    grown: Option<Vec<Extent>>,
    queries: usize,
    fail: bool,
    current_calls: Vec<u64>,
    current: Option<i64>,
}
impl Source for Bytes {
    fn current_time(&mut self, reference: u64) -> Option<i64> {
        self.current_calls.push(reference);
        self.current
    }
    fn read_at(&mut self, offset: u64, output: &mut [u8]) -> Result<()> {
        ensure!(!self.fail, "injected raw-volume read failure");
        assert_eq!(offset % 512, 0);
        assert_eq!(output.len() % 512, 0);
        assert_eq!(output.as_ptr() as usize % 512, 0);
        output.copy_from_slice(&self.disk[offset as usize..offset as usize + output.len()]);
        Ok(())
    }
    fn extents(&mut self) -> Result<Vec<Extent>> {
        self.queries += 1;
        if self.queries > 1
            && let Some(grown) = self.grown.take()
        {
            self.map = grown;
        }
        Ok(self.map.clone())
    }
}
fn reader(split: bool) -> BulkTimes<Bytes> {
    let record = file_record(REFERENCE);
    let mut disk = vec![0; 8192];
    let map = if split {
        // The record crosses two noncontiguous physical extents.
        disk[1024..1536].copy_from_slice(&record[..512]);
        disk[4096..4608].copy_from_slice(&record[512..]);
        vec![
            Extent {
                start: 0,
                end: 1536,
                physical: Some(0),
            },
            Extent {
                start: 1536,
                end: 2048,
                physical: Some(4096),
            },
        ]
    } else {
        disk[1024..2048].copy_from_slice(&record);
        vec![Extent {
            start: 0,
            end: 4096,
            physical: Some(0),
        }]
    };
    BulkTimes::new(
        Geometry {
            sector: 512,
            cluster: 512,
            record: 1024,
        },
        Bytes {
            disk,
            map,
            grown: None,
            queries: 0,
            fail: false,
            current_calls: Vec::new(),
            current: Some(9999),
        },
    )
    .expect("byte-backed metadata reader")
}

#[test]
fn bulk_records_preserve_true_write_identity_and_cache_work() {
    let mut reader = reader(false);
    let mut records = (0..64)
        .map(|_| RawMftRecord {
            last_usn: 0,
            file_reference: REFERENCE,
            parent_reference: 5,
            name: "fixture".into(),
            is_directory: false,
            modified_at_unix_ms: None,
        })
        .collect::<Vec<_>>();
    assert!(
        hydrate_record_times_while(
            &mut records,
            |reference, usn| reader.last_write(reference, usn),
            &|| true
        )
        .expect("hydrate cached fixture batch")
    );
    assert!(
        records
            .iter()
            .all(|record| record.modified_at_unix_ms == Some(12345))
    );
    assert_eq!(reader.reads, 1, "one block, not one volume read per row");
    assert_eq!(reader.unknown, 0);
    assert_eq!(
        reader
            .last_write((4_u64 << 48) | 1, 0)
            .expect("read reused slot"),
        None,
        "reused slot is another file"
    );
}

#[test]
fn fragmented_and_cross_block_records_are_reassembled_before_fixup() {
    let mut fragmented = reader(true);
    assert_eq!(
        fragmented
            .last_write(REFERENCE, 0)
            .expect("read fragmented record"),
        Some(12345)
    );
    assert_eq!(fragmented.reads, 2);
    let mut blocked = reader(false);
    blocked.block_bytes = 512;
    assert_eq!(
        blocked
            .last_write(REFERENCE, 0)
            .expect("read cross-block record"),
        Some(12345)
    );
    assert_eq!(blocked.reads, 2);
}

#[test]
fn malformed_or_unavailable_standard_information_stays_unknown() {
    let original = file_record(REFERENCE);
    let mutations: &[fn(&mut [u8])] = &[
        |r| r[..4].copy_from_slice(b"BAAD"),
        |r| put16(r, 22, 0),
        |r| put64(r, 32, REFERENCE),
        |r| put32(r, 44, 2),
        |r| put16(r, 1022, 0),
        |r| put16(r, 6, 4),
        |r| put16(r, 4, 54),
        |r| put32(r, 24, 1025),
        |r| put32(r, 60, 0),
        |r| put32(r, 60, 1040),
        |r| r[64] = 1,
        |r| put16(r, 76, 95),
        |r| put32(r, 72, 8),
        |r| put64(r, 88, 0),
        // A plausible FILE_NAME value never substitutes for STANDARD_INFORMATION.
        |r| put32(r, 56, 0x30),
    ];
    for (index, mutate) in mutations.iter().enumerate() {
        let mut record = original.clone();
        mutate(&mut record);
        assert_eq!(
            record_last_write(&mut record, 512, REFERENCE),
            None,
            "mutation {index}"
        );
    }
}

#[test]
fn growth_refresh_is_bounded_and_io_failure_is_not_successful_unknown() {
    let mut reader = reader(false);
    reader.extents[0].end = 1024;
    assert_eq!(
        reader.last_write(REFERENCE, 0).expect("read grown extent"),
        Some(12345)
    );
    assert_eq!(reader.source.queries, 2);
    assert_eq!(
        reader
            .last_write((3_u64 << 48) | 100, 0)
            .expect("missing record is unknown"),
        None
    );
    assert_eq!(
        reader
            .last_write((3_u64 << 48) | 101, 0)
            .expect("missing record does not refresh repeatedly"),
        None
    );
    assert_eq!(reader.source.queries, 2);
    reader.cached = None;
    reader.source.fail = true;
    let mut rows = vec![RawMftRecord {
        last_usn: 0,
        file_reference: REFERENCE,
        parent_reference: 5,
        name: "fixture".into(),
        is_directory: false,
        modified_at_unix_ms: None,
    }];
    assert!(
        hydrate_record_times_while(
            &mut rows,
            |reference, usn| reader.last_write(reference, usn),
            &|| { true }
        )
        .is_err()
    );
}

#[test]
fn invalid_geometry_and_discontinuous_extents_are_rejected() {
    assert!(
        validate_extents(
            &[Extent {
                start: 512,
                end: 1024,
                physical: Some(0)
            }],
            512
        )
        .is_err()
    );
    assert!(
        validate_extents(
            &[Extent {
                start: 0,
                end: 1024,
                physical: Some(1)
            }],
            512
        )
        .is_err()
    );
    let source = reader(false).source;
    assert!(
        BulkTimes::new(
            Geometry {
                sector: 4096,
                cluster: 4096,
                record: 1024
            },
            source
        )
        .is_err()
    );
}

#[test]
fn sparse_mft_region_is_unknown_without_raw_io() {
    let mut reader = reader(false);
    reader.extents[0].physical = None;
    assert_eq!(
        reader
            .last_write(REFERENCE, 0)
            .expect("sparse record is unknown"),
        None
    );
    assert_eq!(reader.reads, 0);
    assert_eq!(reader.unknown, 1);
}

#[test]
fn standard_information_time_uses_repaired_sector_trailer_bytes() {
    let mut record = file_record(REFERENCE);
    let attribute = record[56..152].to_vec();
    record[472..568].copy_from_slice(&attribute);
    put16(&mut record, 20, 472);
    put32(&mut record, 24, 576);
    put32(&mut record, 568, u32::MAX);
    let original_tail = u16_at(&record, 510).expect("fixture sector trailer");
    put16(&mut record, 50, original_tail);
    put16(&mut record, 510, 0xabcd);
    assert_eq!(record_last_write(&mut record, 512, REFERENCE), Some(12345));
}

#[test]
fn pre_cursor_dirty_metadata_uses_current_full_id_time_only_for_stale_records() {
    let mut reader = reader(false);
    assert_eq!(
        reader
            .last_write(REFERENCE, 0)
            .expect("read current disk metadata"),
        Some(12345)
    );
    assert!(reader.source.current_calls.is_empty());
    // The latest USN was already observed before the saved replay cursor. Raw disk
    // still contains the previous write time, so later replay cannot repair it.
    assert_eq!(
        reader
            .last_write(REFERENCE, 201)
            .expect("read current attribute exception"),
        Some(9999)
    );
    assert_eq!(reader.source.current_calls, [REFERENCE]);
    reader.source.current = None;
    assert_eq!(
        reader
            .last_write(REFERENCE, 202)
            .expect("failed attributes are unknown"),
        None
    );
    assert_eq!(reader.source.current_calls, [REFERENCE, REFERENCE]);
    assert_eq!(
        reader
            .last_write((4_u64 << 48) | 1, 201)
            .expect("reused identity is unknown"),
        None
    );
    assert_eq!(reader.source.current_calls.len(), 2);
    assert_eq!(reader.fallback_reads, 2);
    // A resident legacy-sized standard-information value has no USN witness.
    reader.cached = None;
    put32(&mut reader.source.disk, 1024 + 72, 48);
    reader.source.current = Some(8888);
    assert_eq!(
        reader
            .last_write(REFERENCE, 0)
            .expect("legacy standard information uses current attributes"),
        Some(8888)
    );
    assert_eq!(reader.fallback_reads, 3);
}

#[test]
fn sequential_metadata_uses_mib_bounded_reads_without_changing_record_results() {
    let mut disk = vec![0; 2 * 1024 * 1024];
    for number in 1..=1025_u64 {
        let reference = (3_u64 << 48) | number;
        let offset = number as usize * 1024;
        disk[offset..offset + 1024].copy_from_slice(&file_record(reference));
    }
    let mut reader = BulkTimes::new(
        Geometry {
            sector: 512,
            cluster: 4096,
            record: 1024,
        },
        Bytes {
            disk,
            map: vec![Extent {
                start: 0,
                end: 2 * 1024 * 1024,
                physical: Some(0),
            }],
            grown: None,
            queries: 0,
            fail: false,
            current_calls: Vec::new(),
            current: Some(9999),
        },
    )
    .expect("sequential metadata reader");
    for number in 1..=1025_u64 {
        assert_eq!(
            reader
                .last_write((3_u64 << 48) | number, 0)
                .expect("sequential record"),
            Some(12345)
        );
    }
    assert_eq!(reader.unknown, 0);
    assert_eq!(reader.fallback_reads, 0);
    assert!(reader.block_bytes <= 1024 * 1024);
    assert!(reader.storage.len() <= 1024 * 1024 + reader.geometry.sector);
    assert_eq!(
        reader.reads, 2,
        "batch sequential metadata within the existing 1 MiB ceiling"
    );
}
