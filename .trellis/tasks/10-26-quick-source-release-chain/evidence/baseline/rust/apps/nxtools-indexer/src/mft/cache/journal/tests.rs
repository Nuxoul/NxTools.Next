use super::*;

fn entry(reference: u64, timestamp: i64) -> MftEntry {
    MftEntry {
        identity: Some(MftIdentity {
            file_reference: reference,
            parent_reference: 5,
        }),
        path: format!("C:\\fixture-{reference}.txt"),
        name: format!("fixture-{reference}.txt"),
        is_directory: false,
        size_bytes: Some(4),
        modified_at_unix_ms: Some(timestamp),
    }
}
fn fixture() -> (tempfile::TempDir, PathBuf, MftSnapshot) {
    let directory = tempfile::tempdir().expect("isolated journal");
    let base = directory.path().join("volume.idx");
    let mut snapshot = MftSnapshot::from_entries(r"C:\", Some(17), Some(100), vec![entry(11, 1)]);
    persist_snapshot(&snapshot, &base).expect("persist base");
    snapshot.set_cache_path(base.clone());
    (directory, base, snapshot)
}
fn reopen(base: &Path) -> MftSnapshot {
    let (mapped, journal, cursor, visible) =
        MappedSnapshot::open(base, r"C:\").expect("mapped base");
    let mut snapshot = MftSnapshot::from_mapped(
        r"C:\".into(),
        journal,
        cursor,
        visible,
        mapped,
        base.to_owned(),
    );
    let delta =
        super::super::load_delta_for_base(base, r"C:\", journal, cursor, snapshot.base_len())
            .expect("replay committed journal");
    *snapshot.delta_checkpoint.lock().expect("durable state") = delta.checkpoint;
    snapshot.restore_delta(
        delta.journal_id,
        delta.next_usn,
        delta.visible_count,
        delta.overlay,
    );
    snapshot
}

#[test]
fn idle_io_reopen_resumes_append_and_replays_delete_restore_and_cursor_only() {
    let (_directory, base, mut snapshot) = fixture();
    snapshot.overlay.replacements.insert(11, entry(11, 2));
    persist_system_delta(&snapshot).expect("checkpoint");
    let initial_len = fs::metadata(delta_path(&base)).expect("journal").len();
    let initial_bytes = fs::read(delta_path(&base)).expect("checkpoint bytes");
    let mut snapshot = reopen(&base);
    snapshot.overlay.replacements.remove(&11);
    snapshot.overlay.deleted.insert(11);
    snapshot.record_visible_delete();
    snapshot.next_usn = Some(110);
    persist_system_delta(&snapshot).expect("delete");
    assert!(reopen(&base).visible_entry(11).is_none());
    snapshot.overlay.deleted.remove(&11);
    snapshot.record_visible_insert();
    snapshot.next_usn = Some(120);
    persist_system_delta(&snapshot).expect("restore base row");
    let restored = reopen(&base);
    assert_eq!(
        restored
            .visible_entry(11)
            .expect("original base restored")
            .modified_at_unix_ms,
        Some(1)
    );
    assert_eq!(restored.visible_len(), 1);
    assert!(restored.overlay.replacements.is_empty() && restored.overlay.deleted.is_empty());
    let mut cursor_only = restored;
    cursor_only.next_usn = Some(130);
    let before = fs::metadata(delta_path(&base))
        .expect("before cursor")
        .len();
    persist_system_delta(&cursor_only).expect("cursor commit");
    let after = fs::metadata(delta_path(&base)).expect("after cursor").len();
    assert_eq!(after - before, (FRAME_HEADER + CHECKSUM) as u64);
    persist_system_delta(&cursor_only).expect("unchanged no-op");
    assert_eq!(
        fs::metadata(delta_path(&base)).expect("after no-op").len(),
        after
    );
    assert_eq!(reopen(&base).next_usn, Some(130));
    assert_eq!(
        &fs::read(delta_path(&base)).expect("journal bytes")[..initial_len as usize],
        initial_bytes.as_slice(),
        "small commits preserve initial checkpoint bytes"
    );
}

#[test]
fn idle_io_legacy_delta_migrates_once_without_rebuilding_base() {
    let (_directory, base, mut snapshot) = fixture();
    snapshot.next_usn = Some(110);
    snapshot.overlay.replacements.insert(11, entry(11, 2));
    super::super::persist_delta(&snapshot, &delta_path(&base)).expect("version3legacy delta");
    let base_bytes = fs::read(&base).expect("base unchanged");
    let legacy = reopen(&base);
    assert_eq!(legacy.next_usn, Some(110));
    assert_eq!(
        legacy
            .visible_entry(11)
            .expect("legacy row")
            .modified_at_unix_ms,
        Some(2)
    );
    persist_system_delta(&legacy).expect("one atomic migration");
    assert!(is_journal(&fs::read(delta_path(&base)).expect("new delta")));
    assert_eq!(fs::read(&base).expect("base after migration"), base_bytes);
    assert_eq!(reopen(&base).next_usn, Some(110));
    let checkpoint_bytes = fs::read(delta_path(&base)).expect("migrated checkpoint");
    let mut continued = reopen(&base);
    continued.next_usn = Some(120);
    continued.overlay.replacements.insert(11, entry(11, 3));
    persist_system_delta(&continued).expect("append after one-time migration");
    let appended = fs::read(delta_path(&base)).expect("appended migration journal");
    assert_eq!(
        &appended[..checkpoint_bytes.len()],
        checkpoint_bytes.as_slice()
    );
    assert_eq!(reopen(&base).next_usn, Some(120));
}

#[test]
fn idle_io_every_truncated_append_restores_only_complete_commit_then_retries() {
    let (_directory, base, mut snapshot) = fixture();
    persist_system_delta(&snapshot).expect("initial checkpoint");
    let committed = fs::read(delta_path(&base)).expect("committed base frame");
    snapshot.next_usn = Some(110);
    snapshot.overlay.replacements.insert(11, entry(11, 2));
    let frame = encode_frame(&snapshot, Some(&MftOverlay::default())).expect("append frame");
    for truncated in 0..frame.len() {
        let mut bytes = committed.clone();
        bytes.extend_from_slice(&frame[..truncated]);
        fs::write(delta_path(&base), bytes).expect("inject interrupted append");
        let restored = reopen(&base);
        assert_eq!(restored.next_usn, Some(100), "truncated={truncated}");
        assert_eq!(
            restored
                .visible_entry(11)
                .expect("last-good row")
                .modified_at_unix_ms,
            Some(1)
        );
    }
    let mut recovered = reopen(&base);
    recovered.next_usn = Some(120);
    recovered.overlay.replacements.insert(11, entry(11, 3));
    persist_system_delta(&recovered).expect("discard torn tail then append current state");
    assert_eq!(reopen(&base).next_usn, Some(120));
    assert_eq!(
        reopen(&base)
            .visible_entry(11)
            .expect("current row")
            .modified_at_unix_ms,
        Some(3)
    );
}

#[test]
fn idle_io_complete_corruption_and_wrong_journal_fail_closed() {
    let (_directory, base, snapshot) = fixture();
    persist_system_delta(&snapshot).expect("checkpoint");
    let mut bytes = fs::read(delta_path(&base)).expect("journal bytes");
    bytes[JOURNAL_HEADER + 16] ^= 1;
    assert!(
        decode(&bytes, Some(17)).is_err(),
        "full corrupt frame cannot expose newer cursor"
    );
    assert!(decode(&fs::read(delta_path(&base)).expect("good bytes"), Some(18)).is_err());
    assert!(
        decode(&bytes[..JOURNAL_HEADER], Some(17)).is_err(),
        "no complete initial checkpoint"
    );
}

#[test]
fn idle_io_failed_append_keeps_checkpoint_and_retry_commits_latest_state() {
    let (_directory, base, mut snapshot) = fixture();
    persist_system_delta(&snapshot).expect("initial checkpoint");
    let delta = delta_path(&base);
    let retained = base.with_extension("retained");
    fs::rename(&delta, &retained).expect("retain old good delta");
    fs::create_dir(&delta).expect("block append target");
    snapshot.next_usn = Some(110);
    snapshot.overlay.replacements.insert(11, entry(11, 2));
    assert!(persist_system_delta(&snapshot).is_err());
    assert_eq!(
        snapshot
            .delta_checkpoint
            .lock()
            .expect("last successful checkpoint")
            .as_ref()
            .expect("checkpoint")
            .next_usn,
        Some(100)
    );
    fs::remove_dir(&delta).expect("remove obstruction");
    fs::rename(&retained, &delta).expect("restore target");
    snapshot.next_usn = Some(120);
    snapshot.overlay.replacements.insert(11, entry(11, 3));
    persist_system_delta(&snapshot).expect("retry newest state");
    assert_eq!(reopen(&base).next_usn, Some(120));
    assert_eq!(
        reopen(&base)
            .visible_entry(11)
            .expect("new row")
            .modified_at_unix_ms,
        Some(3)
    );
}

#[test]
fn idle_io_repeated_updates_checkpoint_only_after_bounded_growth() {
    let (_directory, base, mut snapshot) = fixture();
    let mut value = entry(11, 1);
    value.path = format!("C:\\{}", "x".repeat(32_768));
    snapshot.overlay.replacements.insert(11, value);
    persist_system_delta(&snapshot).expect("checkpoint");
    let initial = fs::metadata(delta_path(&base))
        .expect("initial bytes")
        .len();
    let mut prior = initial;
    let mut checkpoints = 0;
    for tick in 1..=70 {
        snapshot.next_usn = Some(100 + tick);
        snapshot
            .overlay
            .replacements
            .get_mut(&11)
            .expect("row")
            .modified_at_unix_ms = Some(tick);
        persist_system_delta(&snapshot).expect("bounded update");
        let length = fs::metadata(delta_path(&base)).expect("journal size").len();
        if length < prior {
            checkpoints += 1;
        }
        assert!(length <= initial + MIN_CHECKPOINT_OVERHEAD);
        prior = length;
    }
    assert!(
        (1..=3).contains(&checkpoints),
        "not full rewrite each tick: checkpoints={checkpoints}"
    );
    assert_eq!(reopen(&base).next_usn, Some(170));
}

#[test]
fn idle_io_stale_snapshot_cannot_overwrite_newer_committed_cursor() {
    let (_directory, base, mut snapshot) = fixture();
    persist_system_delta(&snapshot).expect("initial commit");
    let stale = snapshot.clone();
    snapshot.next_usn = Some(110);
    snapshot.overlay.replacements.insert(11, entry(11, 2));
    persist_system_delta(&snapshot).expect("new current commit");
    assert!(
        persist_system_delta(&stale).is_err(),
        "retired snapshot shares durability guard"
    );
    assert_eq!(reopen(&base).next_usn, Some(110));
    assert_eq!(
        reopen(&base)
            .visible_entry(11)
            .expect("new row retained")
            .modified_at_unix_ms,
        Some(2)
    );
}

#[test]
fn idle_io_second_checkpoint_inside_append_stream_is_rejected() {
    let (_directory, base, mut snapshot) = fixture();
    persist_system_delta(&snapshot).expect("initial commit");
    let mut bytes = fs::read(delta_path(&base)).expect("committed stream");
    snapshot.next_usn = Some(110);
    bytes.extend_from_slice(&encode_frame(&snapshot, None).expect("misplaced complete checkpoint"));
    assert!(
        decode(&bytes, Some(17)).is_err(),
        "only atomic replacement may introduce a new full checkpoint"
    );
}

#[test]
fn idle_io_review_missing_or_short_delta_recreates_from_current_memory() {
    for shortened in [false, true] {
        let (_directory, base, mut snapshot) = fixture();
        persist_system_delta(&snapshot).expect("first durable commit");
        if shortened {
            fs::write(delta_path(&base), b"short").expect("truncate backing delta");
        } else {
            fs::remove_file(delta_path(&base)).expect("remove backing delta");
        }
        snapshot.next_usn = Some(110);
        snapshot.overlay.replacements.insert(11, entry(11, 2));
        persist_system_delta(&snapshot)
            .expect("recover missing/short backing file from current view");
        assert_eq!(reopen(&base).next_usn, Some(110));
        assert_eq!(
            reopen(&base)
                .visible_entry(11)
                .expect("recovered row")
                .modified_at_unix_ms,
            Some(2)
        );
    }
}

#[test]
fn idle_io_review_same_journal_wrong_base_delta_is_rejected() {
    let (directory, base, snapshot) = fixture();
    persist_system_delta(&snapshot).expect("source delta");
    let other = directory.path().join("different-base.idx");
    let other_snapshot = MftSnapshot::from_entries(r"C:\", Some(17), Some(90), vec![entry(22, 1)]);
    persist_snapshot(&other_snapshot, &other).expect("different immutable base");
    fs::copy(delta_path(&base), delta_path(&other)).expect("misassociate same-journal sidecar");
    assert!(
        super::super::load_delta_for_base(&other, r"C:\", Some(17), Some(90), 1).is_err(),
        "journal ID alone cannot authorize another base's overlay"
    );
}

#[test]
fn idle_io_review_newer_base_mtime_cannot_discard_newer_committed_delta() {
    let (directory, _base, mut authoritative) = fixture();
    let older = directory.path().join("volume-test-older.idx");
    persist_snapshot(&authoritative, &older).expect("older base cursor100");
    authoritative.set_cache_path(older.clone());
    authoritative.next_usn = Some(130);
    authoritative.overlay.replacements.insert(11, entry(11, 3));
    persist_system_delta(&authoritative).expect("latest committed delta cursor130");
    std::thread::sleep(std::time::Duration::from_millis(30));
    let newer = directory.path().join("volume-test-newer.idx");
    let stale = MftSnapshot::from_entries(r"C:\", Some(17), Some(120), vec![entry(11, 2)]);
    persist_snapshot(&stale, &newer).expect("later-written stale compaction cursor120");
    let restored = super::super::load_cached_snapshot(directory.path(), "volume-test", r"C:\")
        .expect("select committed base/delta pair")
        .expect("restored index");
    assert_eq!(
        restored.next_usn,
        Some(130),
        "mtime is not the authoritative effective cursor"
    );
    assert!(
        older.is_file() && delta_path(&older).is_file(),
        "cleanup must preserve authoritative pair"
    );
}

#[test]
fn idle_io_review_unsynced_complete_frame_residue_is_replaced_before_latest_retry() {
    let (_directory, base, mut snapshot) = fixture();
    persist_system_delta(&snapshot).expect("initial synced checkpoint");
    let mut failed = snapshot.clone();
    failed.next_usn = Some(110);
    failed.overlay.replacements.insert(11, entry(11, 2));
    let frame = encode_frame(&failed, Some(&snapshot.overlay)).expect("failed sync residue frame");
    OpenOptions::new()
        .append(true)
        .open(delta_path(&base))
        .expect("append residue")
        .write_all(&frame)
        .expect("complete write before failed sync acknowledgement");
    assert_eq!(
        snapshot
            .delta_checkpoint
            .lock()
            .expect("unchanged durable owner")
            .as_ref()
            .expect("checkpoint")
            .next_usn,
        Some(100)
    );
    snapshot.next_usn = Some(120);
    snapshot.overlay.replacements.insert(11, entry(11, 3));
    persist_system_delta(&snapshot).expect("truncate failed transaction and commit latest");
    assert_eq!(reopen(&base).next_usn, Some(120));
    assert_eq!(
        reopen(&base)
            .visible_entry(11)
            .expect("latest row")
            .modified_at_unix_ms,
        Some(3)
    );
}
