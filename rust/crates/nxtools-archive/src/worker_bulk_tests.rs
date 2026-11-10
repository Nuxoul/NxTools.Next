use super::{Binding, FLAG, HEADER, Packet, SOURCE, read};
use std::io::{self, Read, Write};
use uuid::Uuid;

fn literal(actual: u32) -> Vec<u8> {
    let mut bytes = Vec::from((FLAG | (HEADER as u32 + actual)).to_le_bytes());
    bytes.extend_from_slice(&[1, 2, 0, 0]);
    for start in [1u8, 17, 33] {
        bytes.extend(start..start + 16);
    }
    bytes.extend_from_slice(&7u64.to_le_bytes());
    bytes.extend_from_slice(&123u64.to_le_bytes());
    bytes.extend_from_slice(&65536u32.to_le_bytes());
    bytes.extend_from_slice(&actual.to_le_bytes());
    bytes.extend((0..actual).map(|i| (i % 251) as u8));
    bytes
}
struct Short<T>(T);
impl<T: Read> Read for Short<T> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let count = bytes.len().min(3);
        self.0.read(&mut bytes[..count])
    }
}
impl<T: Write> Write for Short<T> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.write(&bytes[..bytes.len().min(3)])
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}
#[test]
fn independent_literal_roundtrips_network_uuids_short_reads_and_writes() {
    for actual in [0, 1, 65536] {
        let bytes = literal(actual);
        let Some(Packet::Binary(binding, payload)) =
            read(&mut Short(bytes.as_slice())).expect("short reads")
        else {
            panic!("binary record");
        };
        assert!(
            binding.kind == SOURCE
                && binding.sequence == 7
                && binding.offset == 123
                && binding.length == 65536
        );
        assert_eq!(
            binding.operation,
            Uuid::from_bytes(std::array::from_fn(|i| i as u8 + 1))
        );
        assert_eq!(
            binding.call,
            Uuid::from_bytes(std::array::from_fn(|i| i as u8 + 17))
        );
        assert_eq!(
            binding.token,
            Uuid::from_bytes(std::array::from_fn(|i| i as u8 + 33))
        );
        assert_eq!(payload.len(), actual as usize);
        let mut encoded = Short(Vec::new());
        binding.write(&mut encoded, &payload).expect("short writes");
        assert_eq!(encoded.0, bytes);
    }
}
#[test]
fn malformed_size_header_and_truncation_fail_before_payload_allocation() {
    for encoded in [0, 1048577, FLAG | 75, FLAG | 65613] {
        assert!(read(&mut encoded.to_le_bytes().as_slice()).is_err());
    }
    for (offset, value) in [(4, 2), (5, 4), (6, 1), (56, 0), (72, 0)] {
        let mut bytes = literal(1);
        bytes[offset] = value;
        if offset == 56 {
            bytes[56..64].fill(0);
        }
        if offset == 72 {
            bytes[72..76].fill(0);
        }
        assert!(read(&mut bytes.as_slice()).is_err(), "field {offset}");
    }
    let bytes = literal(1);
    for length in [1, 3, 4, 40, 79, 80] {
        assert!(
            read(&mut &bytes[..length]).is_err(),
            "truncated at {length}"
        );
    }
    let mut nil = literal(0);
    nil[40..56].fill(0);
    assert!(read(&mut nil.as_slice()).is_err());
}
#[test]
fn invalid_output_short_write_and_payload_bounds_are_rejected() {
    let binding = Binding {
        kind: 1,
        operation: Uuid::new_v4(),
        call: Uuid::new_v4(),
        token: Uuid::new_v4(),
        sequence: 1,
        offset: 0,
        length: 2,
    };
    assert!(binding.write(&mut Vec::new(), &[1]).is_err());
    let oversized = Binding {
        length: 65537,
        ..binding
    };
    assert!(oversized.write(&mut Vec::new(), &[1]).is_err());
}
