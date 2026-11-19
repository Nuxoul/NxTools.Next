use std::time::{Duration, Instant};

use crate::{DownloadSegment, DownloadSegmentError, DownloadSegmentState};

pub(crate) type WorkerId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Claim {
    Granted(u64),
    Lost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Assignment {
    pub index: u32,
    pub from: u64,
    pub to: u64,
    pub written: u64,
    pub preempted: Option<WorkerId>,
    pub split: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct PlannedSegment {
    pub(crate) index: u32,
    pub(crate) start: u64,
    pub(crate) cursor: u64,
    pub(crate) reserved: u64,
    pub(crate) synced_cursor: u64,
    pub(crate) end: u64,
    pub(crate) worker: Option<WorkerId>,
    pub(crate) receiving: bool,
    pub(crate) given_at: Option<Instant>,
}

pub(crate) struct SegmentPlanner {
    segments: Vec<PlannedSegment>,
    preempt_after: Duration,
}

impl SegmentPlanner {
    pub(crate) fn new(segments: &[DownloadSegment], preempt_after: Duration) -> Self {
        Self {
            segments: segments
                .iter()
                .map(|segment| {
                    let start = segment.start_byte();
                    let cursor = start + segment.downloaded_bytes();
                    PlannedSegment {
                        index: segment.index(),
                        start,
                        cursor,
                        reserved: cursor,
                        synced_cursor: cursor,
                        end: segment.end_byte() + 1,
                        worker: None,
                        receiving: false,
                        given_at: None,
                    }
                })
                .collect(),
            preempt_after,
        }
    }

    pub(crate) fn assign(&mut self, worker: WorkerId, after: Option<u64>) -> Option<Assignment> {
        self.release(worker);
        let now = Instant::now();

        // Continue at the adjacent waiting range before taking the first other one.
        let waiting = self
            .segments
            .iter()
            .position(|segment| {
                segment.worker.is_none()
                    && segment.reserved < segment.end
                    && after == Some(segment.start)
            })
            .or_else(|| {
                self.segments
                    .iter()
                    .position(|segment| segment.worker.is_none() && segment.reserved < segment.end)
            });
        if let Some(slot) = waiting {
            let segment = &mut self.segments[slot];
            segment.worker = Some(worker);
            segment.receiving = false;
            segment.given_at = Some(now);
            return Some(Self::assignment(segment, None, false));
        }

        if let Some(after) = after
            && let Some(segment) = self.segments.iter_mut().find(|segment| {
                segment.start == after
                    && segment.worker.is_some()
                    && !segment.receiving
                    && segment
                        .given_at
                        .is_some_and(|given_at| now.duration_since(given_at) >= self.preempt_after)
            })
        {
            let previous = segment.worker.replace(worker);
            segment.given_at = Some(now);
            return Some(Self::assignment(segment, previous, false));
        }

        let (slot, segment) =
            self.segments
                .iter()
                .enumerate()
                .max_by(|(left_slot, left), (right_slot, right)| {
                    (left.end - left.reserved)
                        .cmp(&(right.end - right.reserved))
                        .then_with(|| right_slot.cmp(left_slot))
                })?;
        let half = (segment.end - segment.reserved) / 2;
        const MIN_SPLIT: u64 = 12 * 1024;
        if half < MIN_SPLIT {
            return None;
        }
        let midpoint = segment.reserved + half;
        let aligned = midpoint / 4096 * 4096;
        let midpoint = if aligned - segment.reserved >= MIN_SPLIT {
            aligned
        } else {
            midpoint
        };
        let end = segment.end;
        let index = self.segments.iter().map(|segment| segment.index).max()? + 1;
        self.segments[slot].end = midpoint;
        let next = PlannedSegment {
            index,
            start: midpoint,
            cursor: midpoint,
            reserved: midpoint,
            synced_cursor: midpoint,
            end,
            worker: Some(worker),
            receiving: false,
            given_at: Some(now),
        };
        let assignment = Self::assignment(&next, None, true);
        self.segments.insert(slot + 1, next);
        Some(assignment)
    }

    fn assignment(
        segment: &PlannedSegment,
        preempted: Option<WorkerId>,
        split: bool,
    ) -> Assignment {
        Assignment {
            index: segment.index,
            from: segment.cursor,
            to: segment.end,
            written: segment.cursor - segment.start,
            preempted,
            split,
        }
    }

    pub(crate) fn claim(&mut self, worker: WorkerId, pos: u64, len: u64) -> Claim {
        let Some(segment) = self
            .segments
            .iter_mut()
            .find(|segment| segment.worker == Some(worker))
        else {
            return Claim::Lost;
        };
        if pos != segment.reserved {
            return Claim::Lost;
        }
        let count = len.min(segment.end - segment.reserved);
        segment.reserved += count;
        if count > 0 {
            segment.receiving = true;
        }
        Claim::Granted(count)
    }

    pub(crate) fn commit(&mut self, worker: WorkerId, upto: u64) -> Option<u64> {
        let segment = self
            .segments
            .iter_mut()
            .find(|segment| segment.worker == Some(worker))?;
        segment.cursor = upto.clamp(segment.cursor, segment.reserved);
        if segment.cursor == segment.end {
            segment.worker = None;
            segment.receiving = false;
            segment.given_at = None;
            return Some(segment.end);
        }
        None
    }

    pub(crate) fn release(&mut self, worker: WorkerId) {
        if let Some(segment) = self
            .segments
            .iter_mut()
            .find(|segment| segment.worker == Some(worker))
        {
            segment.reserved = segment.cursor;
            segment.worker = None;
            segment.receiving = false;
            segment.given_at = None;
        }
    }

    pub(crate) fn room(&self, worker: WorkerId, at: u64) -> Option<u64> {
        self.segments
            .iter()
            .find(|segment| segment.worker == Some(worker))
            .map(|segment| segment.end.saturating_sub(at))
    }

    pub(crate) fn mark_synced(&mut self, index: u32, upto: u64) {
        if let Some(segment) = self
            .segments
            .iter_mut()
            .find(|segment| segment.index == index)
        {
            segment.synced_cursor = upto.clamp(segment.synced_cursor, segment.cursor);
        }
    }

    pub(crate) fn segments(&self) -> &[PlannedSegment] {
        &self.segments
    }

    pub(crate) fn active_workers(&self) -> usize {
        self.segments
            .iter()
            .filter(|segment| segment.worker.is_some())
            .count()
    }

    pub(crate) fn receiving_workers(&self) -> usize {
        self.segments
            .iter()
            .filter(|segment| segment.worker.is_some() && segment.receiving)
            .count()
    }

    pub(crate) fn is_complete(&self) -> bool {
        self.segments
            .iter()
            .all(|segment| segment.cursor == segment.end)
    }

    pub(crate) fn checkpoints(&self) -> Result<Vec<DownloadSegment>, DownloadSegmentError> {
        self.segments
            .iter()
            .map(|segment| {
                let state = if segment.synced_cursor == segment.end {
                    DownloadSegmentState::Completed
                } else if segment.worker.is_some() {
                    DownloadSegmentState::Active
                } else {
                    DownloadSegmentState::Waiting
                };
                DownloadSegment::new(
                    segment.index,
                    segment.start,
                    segment.end - 1,
                    segment.synced_cursor - segment.start,
                    state,
                )
            })
            .collect()
    }

    pub(crate) fn progress_segments(&self) -> Vec<(u32, u64, u64, u64)> {
        self.segments
            .iter()
            .map(|segment| {
                (
                    segment.index,
                    segment.start,
                    segment.end - segment.start,
                    segment.cursor - segment.start,
                )
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "segment_planner_tests.rs"]
mod tests;
