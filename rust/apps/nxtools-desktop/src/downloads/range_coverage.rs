use super::*;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::plot::{Plot, PlotElement};
use nxtools_download::DownloadSegmentProgress;

type Span = (u64, u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RangeCoverage {
    total: Option<u64>,
    completed: Vec<Span>,
    written: Vec<Span>,
    missing_ranges: bool,
}

#[derive(Debug, PartialEq)]
struct CoverageBin {
    completed: f32,
    written: f32,
}

fn merge_spans(mut spans: Vec<Span>) -> Vec<Span> {
    spans.sort_unstable();
    let mut merged: Vec<Span> = Vec::with_capacity(spans.len());
    for (start, end) in spans {
        if let Some(last) = merged.last_mut()
            && start <= last.1
        {
            last.1 = last.1.max(end);
        } else {
            merged.push((start, end));
        }
    }
    merged
}

impl RangeCoverage {
    pub(super) fn new(total: Option<u64>, segments: &[DownloadSegmentProgress]) -> Self {
        let mut coverage = Self {
            total: total.filter(|total| *total > 0),
            completed: Vec::new(),
            written: Vec::new(),
            missing_ranges: segments.is_empty(),
        };
        for segment in segments {
            let (Some(start), Some(length)) = (segment.start_byte, segment.total_bytes) else {
                coverage.missing_ranges = true;
                continue;
            };
            let Some(end) = start.checked_add(length) else {
                coverage.missing_ranges = true;
                continue;
            };
            let end = coverage.total.map_or(end, |total| end.min(total));
            if start >= end {
                continue;
            }
            let written_end = start + segment.downloaded_bytes.min(end - start);
            if written_end > start {
                coverage.written.push((start, written_end));
                if segment.downloaded_bytes >= length {
                    coverage.completed.push((start, end));
                }
            }
        }
        coverage.completed = merge_spans(coverage.completed);
        coverage.written = merge_spans(coverage.written);
        coverage
    }

    fn label(&self) -> String {
        if self.total.is_none() {
            return "总大小未知 · 不显示覆盖比例".into();
        }
        if self.missing_ranges {
            return "部分区间未记录 · 仅显示已知写入".into();
        }
        "已完成 · 已写入前缀 · 空白为未写入".into()
    }

    fn bins(&self, width: usize) -> Vec<CoverageBin> {
        let Some(total) = self.total else {
            return Vec::new();
        };
        let count = width
            .min(2048)
            .min(usize::try_from(total).unwrap_or(usize::MAX));
        if count == 0 {
            return Vec::new();
        }
        let mut complete_cursor = 0;
        let mut written_cursor = 0;
        (0..count)
            .map(|index| {
                let start = (index as u128 * u128::from(total) / count as u128) as u64;
                let end = ((index + 1) as u128 * u128::from(total) / count as u128) as u64;
                let length = (end - start) as f32;
                CoverageBin {
                    completed: span_bytes(&self.completed, start, end, &mut complete_cursor) as f32
                        / length,
                    written: span_bytes(&self.written, start, end, &mut written_cursor) as f32
                        / length,
                }
            })
            .collect()
    }
}

fn span_bytes(spans: &[Span], start: u64, end: u64, cursor: &mut usize) -> u64 {
    while *cursor < spans.len() && spans[*cursor].1 <= start {
        *cursor += 1;
    }
    let mut bytes = 0;
    for &(left, right) in &spans[*cursor..] {
        if left >= end {
            break;
        }
        bytes += right.min(end) - left.max(start);
    }
    bytes
}

struct CoveragePlot {
    coverage: RangeCoverage,
    theme: Theme,
}

impl IntoElement for CoveragePlot {
    type Element = PlotElement<Self>;

    fn into_element(self) -> Self::Element {
        PlotElement::new(self)
    }
}

impl Plot for CoveragePlot {
    fn paint(
        &mut self,
        bounds: gpui_kit::Bounds<gpui_kit::Pixels>,
        window: &mut Window,
        _: &mut App,
    ) {
        let bins = self
            .coverage
            .bins(bounds.size.width.as_f32().ceil().max(0.0) as usize);
        if bins.is_empty() {
            return;
        }
        let width = bounds.size.width.as_f32() / bins.len() as f32;
        for (index, bin) in bins.iter().enumerate() {
            let left = bounds.origin.x + px(index as f32 * width);
            for (fraction, color) in [
                (bin.written, self.theme.accent),
                (bin.completed, self.theme.success),
            ] {
                if fraction > 0.0 {
                    window.paint_quad(gpui_kit::fill(
                        gpui_kit::Bounds::new(
                            gpui_kit::point(left, bounds.origin.y),
                            size(px(width * fraction), bounds.size.height),
                        ),
                        color,
                    ));
                }
            }
        }
    }
}

pub(super) fn range_coverage_slot(
    total: Option<u64>,
    segments: &[DownloadSegmentProgress],
    theme: Theme,
) -> AnyElement {
    let coverage = RangeCoverage::new(total, segments);
    let label = coverage.label();
    div()
        .id("download-window-range-coverage")
        .debug_selector(|| "download-window-range-coverage".into())
        .test_support()
        .aria_label(format!("真实字节区间：{label}"))
        .w_full()
        .h(px(26.0))
        .flex_none()
        .flex()
        .flex_col()
        .child(
            div()
                .w_full()
                .h(px(8.0))
                .flex_none()
                .bg(theme.surface_hover)
                .child(CoveragePlot { coverage, theme }),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.text_secondary)
                .truncate()
                .child(label),
        )
        .into_any_element()
}

pub(super) fn persisted_range_progress(
    segments: &[DownloadSegment],
) -> Vec<DownloadSegmentProgress> {
    segments
        .iter()
        .map(|segment| DownloadSegmentProgress {
            segment_index: segment.index(),
            start_byte: Some(segment.start_byte()),
            total_bytes: Some(segment.len()),
            downloaded_bytes: segment.downloaded_bytes(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(
        index: u32,
        start: Option<u64>,
        length: Option<u64>,
        written: u64,
    ) -> DownloadSegmentProgress {
        DownloadSegmentProgress {
            segment_index: index,
            start_byte: start,
            total_bytes: length,
            downloaded_bytes: written,
        }
    }

    #[test]
    fn b4_ref10_sparse_overlap_and_allocation_use_only_written_bytes() {
        let coverage = RangeCoverage::new(
            Some(100),
            &[
                segment(0, Some(0), Some(10), 10),
                segment(1, Some(20), Some(20), 5),
                segment(2, Some(22), Some(15), 5),
                segment(3, Some(70), Some(30), 0),
            ],
        );
        assert_eq!(coverage.completed, vec![(0, 10)]);
        assert_eq!(coverage.written, vec![(0, 10), (20, 27)]);
        let bins = coverage.bins(10);
        assert_eq!(bins.len(), 10);
        assert_eq!(
            bins[0],
            CoverageBin {
                completed: 1.0,
                written: 1.0
            }
        );
        assert_eq!(bins[1].written, 0.0);
        assert!((bins[2].written - 0.7).abs() < f32::EPSILON);
        assert!(bins[3..].iter().all(|bin| bin.written == 0.0));
    }

    #[test]
    fn b4_ref10_missing_and_unknown_do_not_become_zero_or_complete() {
        let missing = RangeCoverage::new(Some(100), &[segment(0, None, Some(100), 100)]);
        assert!(missing.missing_ranges);
        assert!(missing.written.is_empty());
        assert!(missing.label().contains("未记录"));
        let unknown = RangeCoverage::new(None, &[segment(0, Some(0), Some(100), 25)]);
        assert!(unknown.bins(400).is_empty());
        assert!(unknown.label().contains("总大小未知"));
        assert_eq!(unknown.written, vec![(0, 25)]);
    }

    #[test]
    fn b4_ref10_long_many_ranges_are_viewport_bounded_and_clamped() {
        let segments = (0..10_000)
            .map(|index| segment(index, Some(u64::from(index) * 100), Some(100), 50))
            .collect::<Vec<_>>();
        let coverage = RangeCoverage::new(Some(1_000_000), &segments);
        assert_eq!(coverage.bins(320).len(), 320);
        assert_eq!(coverage.bins(100_000).len(), 2048);
        assert!(
            coverage
                .bins(320)
                .iter()
                .all(|bin| bin.written >= 0.0 && bin.written <= 1.0)
        );
        let clamped = RangeCoverage::new(Some(100), &[segment(0, Some(90), Some(20), 40)]);
        assert_eq!(clamped.written, vec![(90, 100)]);
        assert_eq!(clamped.completed, vec![(90, 100)]);
    }
}
