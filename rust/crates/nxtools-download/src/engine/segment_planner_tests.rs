use super::{Claim, SegmentPlanner};
use crate::{DownloadSegment, DownloadSegmentState};
use std::time::Duration;

fn segment(index: u32, start: u64, end: u64, written: u64) -> DownloadSegment {
    DownloadSegment::new(
        index,
        start,
        end - 1,
        written,
        DownloadSegmentState::Waiting,
    )
    .expect("valid half-open test range")
}

fn assert_tiles(planner: &SegmentPlanner, end: u64) {
    let mut ranges: Vec<_> = planner
        .segments()
        .iter()
        .map(|segment| (segment.start, segment.end))
        .collect();
    ranges.sort_unstable();
    let mut next = 0;
    for (start, stop) in ranges {
        assert_eq!(start, next, "segment ranges must not overlap or leave gaps");
        assert!(stop > start);
        next = stop;
    }
    assert_eq!(next, end);
}

#[test]
fn planner_waiting_after_precedes_other_waiting_and_takeover() {
    let mut planner = SegmentPlanner::new(
        &[
            segment(3, 0, 16_384, 0),
            segment(8, 16_384, 32_768, 0),
            segment(12, 32_768, 49_152, 0),
        ],
        Duration::ZERO,
    );
    let first = planner.assign(1, None).expect("first waiting range");
    assert_eq!((first.index, first.from, first.to), (3, 0, 16_384));
    let second = planner
        .assign(2, Some(32_768))
        .expect("preferred waiting range");
    assert_eq!(second.index, 12);
    let third = planner
        .assign(4, Some(32_768))
        .expect("remaining waiting range");
    assert_eq!(third.index, 8);
    assert_eq!(third.preempted, None);
    assert_eq!(planner.active_workers(), 3);
    assert_eq!(planner.receiving_workers(), 0);
    assert_tiles(&planner, 49_152);
}

#[test]
fn planner_timed_adjacent_takeover_only_when_not_receiving() {
    let input = [segment(0, 0, 16_384, 0), segment(1, 16_384, 32_768, 0)];
    let mut early = SegmentPlanner::new(&input, Duration::from_secs(3600));
    early.assign(1, None).expect("first");
    early.assign(2, None).expect("second");
    assert!(early.assign(3, Some(16_384)).is_none());
    assert_eq!(early.segments()[1].worker, Some(2));

    let mut ready = SegmentPlanner::new(&input, Duration::ZERO);
    ready.assign(1, None).expect("first");
    ready.assign(2, None).expect("second");
    let takeover = ready.assign(3, Some(16_384)).expect("adjacent takeover");
    assert_eq!(
        (takeover.index, takeover.preempted, takeover.split),
        (1, Some(2), false)
    );
    assert_eq!(ready.claim(2, 16_384, 12), Claim::Lost);
    assert_eq!(ready.claim(3, 16_384, 12), Claim::Granted(12));
    assert_eq!(ready.receiving_workers(), 1);
    assert!(ready.assign(4, Some(16_384)).is_none());
    assert_eq!(ready.segments()[1].worker, Some(3));
}

#[test]
fn planner_largest_unclaimed_tail_splits_at_aligned_midpoint() {
    let mut planner = SegmentPlanner::new(&[segment(9, 0, 100_000, 0)], Duration::ZERO);
    planner.assign(1, None).expect("initial range");
    assert_eq!(planner.claim(1, 0, 5_000), Claim::Granted(5_000));
    let split = planner.assign(2, None).expect("tail split");
    assert_eq!(
        (split.index, split.from, split.to, split.written),
        (10, 49_152, 100_000, 0)
    );
    assert!(split.split);
    assert_eq!(split.preempted, None);
    assert_eq!(planner.segments()[0].end, split.from);
    assert_eq!(planner.segments()[0].worker, Some(1));
    assert_eq!(planner.claim(1, 5_000, 100_000), Claim::Granted(44_152));
    assert_eq!(
        planner.claim(2, split.from, 100_000),
        Claim::Granted(50_848)
    );
    assert_tiles(&planner, 100_000);
}

#[test]
fn planner_takeover_precedes_split_of_a_large_tail() {
    let mut planner = SegmentPlanner::new(
        &[segment(0, 0, 16_384, 0), segment(1, 16_384, 216_384, 0)],
        Duration::ZERO,
    );
    planner.assign(1, None).expect("first range");
    planner.assign(2, None).expect("adjacent range");
    let assignment = planner.assign(3, Some(16_384)).expect("takeover");
    assert_eq!(
        (assignment.index, assignment.preempted, assignment.split),
        (1, Some(2), false)
    );
    assert_eq!(planner.segments().len(), 2);
    assert_tiles(&planner, 216_384);
}

#[test]
fn planner_split_respects_minimum_and_uses_unaligned_midpoint_when_needed() {
    let mut small = SegmentPlanner::new(&[segment(0, 0, 24_575, 0)], Duration::ZERO);
    small.assign(1, None).expect("initial range");
    assert!(small.assign(2, None).is_none()); // half < 12 KiB
    assert_tiles(&small, 24_575);

    let mut boundary = SegmentPlanner::new(&[segment(0, 0, 24_576, 0)], Duration::ZERO);
    boundary.assign(1, None).expect("initial range");
    assert_eq!(
        boundary.assign(2, None).expect("exact minimum").from,
        12_288
    );

    let mut unaligned = SegmentPlanner::new(&[segment(0, 0, 60_000, 0)], Duration::ZERO);
    unaligned.assign(1, None).expect("initial range");
    assert_eq!(unaligned.claim(1, 0, 9_000), Claim::Granted(9_000));
    // The 9,000 + 25,500 midpoint aligns to 32,768; check the fallback when
    // rounding down would leave fewer than 12 KiB after an unaligned reservation.
    let mut fallback = SegmentPlanner::new(&[segment(0, 0, 34_000, 0)], Duration::ZERO);
    fallback.assign(1, None).expect("initial range");
    assert_eq!(fallback.claim(1, 0, 9_000), Claim::Granted(9_000));
    assert_eq!(
        fallback.assign(2, None).expect("fallback midpoint").from,
        21_500
    );
    assert_eq!(
        unaligned.assign(2, None).expect("aligned midpoint").from,
        32_768
    );
}

#[test]
fn planner_reserved_bytes_survive_split_until_commit_or_release() {
    let mut planner = SegmentPlanner::new(&[segment(0, 0, 80_000, 0)], Duration::ZERO);
    planner.assign(1, None).expect("initial range");
    assert_eq!(planner.claim(1, 0, 40_000), Claim::Granted(40_000));
    assert_eq!(planner.claim(1, 39_999, 1), Claim::Lost);
    assert_eq!(planner.segments()[0].reserved, 40_000);
    assert_eq!(planner.room(999, 40_000), None);
    let split = planner.assign(2, None).expect("split unclaimed tail");
    assert!(split.from >= 40_000);
    assert_eq!(planner.room(1, 40_000), Some(split.from - 40_000));
    assert_eq!(
        planner.claim(1, 40_000, 80_000),
        Claim::Granted(split.from - 40_000)
    );
    assert_eq!(planner.commit(1, 20_000), None);
    planner.release(1);
    assert_eq!(planner.segments()[0].reserved, 20_000);
    assert_eq!(
        planner.assign(3, None).expect("released prefix").from,
        20_000
    );
    assert_eq!(planner.claim(1, 20_000, 10), Claim::Lost);
    assert_tiles(&planner, 80_000);
}

#[test]
fn planner_clamps_written_and_synced_independently_and_checkpoints_only_sync() {
    let mut planner = SegmentPlanner::new(&[segment(4, 0, 32_768, 4_000)], Duration::ZERO);
    let assigned = planner.assign(7, None).expect("resumed segment");
    assert_eq!((assigned.from, assigned.written), (4_000, 4_000));
    assert_eq!(planner.claim(7, 4_000, 20_000), Claim::Granted(20_000));
    planner.mark_synced(4, 32_768);
    assert_eq!(planner.segments()[0].synced_cursor, 4_000);
    assert_eq!(
        planner.checkpoints().expect("assigned checkpoint")[0].state,
        DownloadSegmentState::Active
    );
    assert_eq!(planner.commit(7, 40_000), None);
    assert_eq!(
        planner.checkpoints().expect("unflushed checkpoint")[0].downloaded_bytes(),
        4_000
    );
    assert_eq!(planner.segments()[0].cursor, 24_000);
    planner.mark_synced(4, 100_000);
    assert_eq!(planner.segments()[0].synced_cursor, 24_000);
    planner.mark_synced(4, 0);
    planner.mark_synced(999, 32_768);
    assert_eq!(planner.segments()[0].synced_cursor, 24_000);
    assert_eq!(planner.commit(7, 0), None);
    assert_eq!(planner.segments()[0].cursor, 24_000);
    assert_eq!(planner.progress_segments(), vec![(4, 0, 32_768, 24_000)]);
    planner.release(7);
    let saved = planner.checkpoints().expect("valid checkpoint");
    assert_eq!(saved[0].downloaded_bytes(), 24_000);
    assert_eq!(saved[0].state, DownloadSegmentState::Waiting);
    assert_eq!(
        planner.assign(9, None).expect("unwritten suffix").from,
        24_000
    );
    assert_eq!(planner.claim(9, 24_000, 8_768), Claim::Granted(8_768));
    assert_eq!(planner.commit(9, 32_768), Some(32_768));
    assert!(planner.is_complete());
    assert_eq!(planner.active_workers(), 0);
    assert_eq!(
        planner.checkpoints().expect("unsynced complete")[0].state,
        DownloadSegmentState::Waiting
    );
    planner.mark_synced(4, 32_768);
    assert_eq!(
        planner.checkpoints().expect("durably complete")[0].state,
        DownloadSegmentState::Completed
    );
}

#[test]
fn planner_operation_sequences_preserve_complete_tiling() {
    let mut planner = SegmentPlanner::new(
        &[segment(1, 0, 65_536, 0), segment(7, 65_536, 131_072, 0)],
        Duration::ZERO,
    );
    planner.assign(1, None).expect("first");
    assert_tiles(&planner, 131_072);
    planner.assign(2, None).expect("second");
    assert_tiles(&planner, 131_072);
    assert_eq!(planner.claim(1, 0, 17_000), Claim::Granted(17_000));
    planner.commit(1, 12_000);
    assert_tiles(&planner, 131_072);
    let tail = planner
        .assign(3, None)
        .expect("split largest unclaimed tail");
    assert!(tail.split);
    assert_tiles(&planner, 131_072);
    planner.release(1);
    assert_tiles(&planner, 131_072);
    let prefix = planner.assign(4, Some(0)).expect("recover released prefix");
    assert_eq!(prefix.from, 12_000);
    assert_tiles(&planner, 131_072);
    for worker in [2, 3, 4] {
        loop {
            let segment = planner
                .segments()
                .iter()
                .find(|segment| segment.worker == Some(worker))
                .expect("assigned worker");
            let (pos, end, index) = (segment.reserved, segment.end, segment.index);
            let available = end - pos;
            assert_eq!(
                planner.claim(worker, pos, available),
                Claim::Granted(available)
            );
            assert_eq!(planner.commit(worker, end), Some(end));
            planner.mark_synced(index, end);
            assert_tiles(&planner, 131_072);
            if worker == 4 || planner.assign(worker, None).is_none() {
                break;
            }
        }
    }
    assert!(planner.is_complete());
    assert!(
        planner
            .checkpoints()
            .expect("all synced")
            .iter()
            .all(|segment| segment.state == DownloadSegmentState::Completed)
    );
}
