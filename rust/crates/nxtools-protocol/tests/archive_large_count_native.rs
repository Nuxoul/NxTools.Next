use nxtools_protocol::{Envelope, Message, PlatformEvent};

#[test]
#[ignore = "requires original C# large-count Job fixture transcripts in an explicit owned directory"]
fn original_native_large_count_terminals_match_the_rust_contract() {
    let directory = std::env::var_os("NXTOOLS_ARCHIVE_NATIVE_TERMINALS")
        .map(std::path::PathBuf::from)
        .expect("explicit native fixture transcript directory");
    for (name, count, outcome) in [
        ("archive-large-count-terminal.json", 70_000, "succeeded"),
        ("archive-large-cancel-terminal.json", 4_200, "cancelled"),
        ("archive-large-corrupt-terminal.json", 4_999, "failed"),
    ] {
        let bytes = std::fs::read(directory.join(name)).expect("original native Job transcript");
        assert!(bytes.len() <= 1_048_576, "original 1MiB frame bound");
        let wire = std::str::from_utf8(&bytes).expect("UTF8 native envelope");
        let envelope = Envelope::decode_line(wire).expect("C# envelope matches Rust wire");
        let Message::PlatformEvent(PlatformEvent::ArchiveJob(event)) = envelope.message else {
            panic!("original archive Job terminal");
        };
        event
            .validate()
            .expect("native aggregates and bounded detail are consistent");
        let terminal = event.terminal.expect("terminal");
        assert_eq!(terminal.files_committed, count);
        assert_eq!(terminal.bytes_committed, count);
        assert!(terminal.cleanup_complete);
        assert_eq!(
            serde_json::to_value(terminal.outcome).expect("outcome"),
            outcome
        );
        let summary = terminal.journal_summary.expect("negotiated native summary");
        assert!(!summary.detail_complete);
        assert!(summary.total_records >= count);
        assert!(terminal.journal.len() <= 256);
    }
}
