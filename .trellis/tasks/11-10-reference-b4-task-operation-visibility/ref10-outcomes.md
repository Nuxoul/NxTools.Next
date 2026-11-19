# REF-10 actual written-range coverage

Status: **accepted at the [coordinator's stated source/TestPlatform/owned live HTTP-writer boundaries](coordinator-acceptance.md)**. Original implementation evidence remains below.

## Original owners

Original HTTP segment writer callback → original progress count map plus actual `(start,length)` range map → existing coalesced `DownloadProgress.segments` → original Desktop event/window refresh → same keyed DownloadTaskWindowView → passive official Plot/PlotElement. Segment index is identity, not position. Existing `total_bytes` is segment length; optional `start_byte` is the real byte-axis origin. Split/merge update both original maps together under progress→range locks before workers restart; original scheduler, persistence and transfer lifecycle remain.

Original SQLite plan → `command_task_detail` → `load_download_task_detail` at original window open/status transition → cached actual DownloadSegment prefixes. One open-time detail read supplies ranges and final-artifact metadata. Paused and reopened windows prefer persisted ranges; resumed Downloading state retains them until a nonempty live projection exists. Actual live fields are never guessed from stale ranges. Original selected-detail cache adopts the current known live layout after rebalance. No render storage/network calls, second sampler/timer or scheduler.

`RangeCoverage` merges actual written prefixes and completed spans, clamps to known total and bounds Plot marks to visible width/2048bins. Allocated unwritten ranges paint no progress. Missing start/length stays unrecorded; unknown total produces no coverage ratio. Original official aggregate Progress, theme, Server/metrics, speed-limit input, segment table,440×184/280 window budget and fixed footer remain.

## Behavioral evidence

Logs: `D:\.pi-desktop\data\scratch\0ffe6ffa-9e52-420b-af61-a29fdd0f37b4`.

- Genuine engine RED `b4-ref10-projection-red-resumed`: LIST1/RUN1/PASS0/FAIL1/ignored0 at `live written projection must carry true plan start, never infer from segment index`. Before that assertion, the original parallel writer completed exact fixture bytes, real tail rebalance produced at least4range requests, and original partial multi-segment progress was observed.
- Same selection GREEN `b4-ref10-projection-green` and preservation `b4-ref10-projection-preserve`: each LIST1/RUN1/PASS1/FAIL0/ignored0. True serialized origins/lengths, bounded written counts and nonoverlapping current ranges pass. Repeated selection is not an additive distinct total.
- Genuine production-window RED `b4-ref10-window-red`: LIST1/RUN1/PASS0/FAIL1/ignored0 at `real paused written ranges must appear in original expanded window`, after actual HTTP partial writer, SQLite pause and original detail disclosure.
- Focused `b4-ui-range-template-green2`: LIST28/RUN28/PASS28/FAIL0/ignored0, containing6 REF-10 tests. Final `b4-final-ui`: Desktop bin filter `b4_`, LIST29/RUN29/PASS29/FAIL0/ignored0, containing7 REF-10 tests including the full live chain. These selections overlap Header/REF-08/REF-09 and are not additive totals.
- The original written paused task's displayed range cache exactly equals original persisted TaskDetail segments, with nonzero actual bytes. Original detail disclosure displays coverage plus aggregate Progress and relink action, within fixed footer bounds. Closing/reopening the same task hydrates the same ranges.
- Additional first-pass coverage: sparse completed spans and partially written prefixes; overlapping display inputs merged without double-counting; unwritten allocations remain blank;10,000ranges reduced to viewport-bounded bins; long/clamped inputs; missing offsets and unknown total; actual440×280unknown projection with original metrics/capabilities/table/Input entity retained; original selected-detail cache applies reduced/added rebalanced ranges without a storage query. These are not separate claimed feature REDs.
- First-event regression `b4-ref10-pre-event-red2`: LIST1/RUN1/PASS0/FAIL1/ignored0 at `resumed window retains known persisted prefixes before first live range event`: actual SQLite prefix was1/16bytes but presentation returned an empty list. `b4-ref10-pre-event-green`: identical selection LIST1/RUN1/PASS1/FAIL0/ignored0. It supplements the original real paused-writer/window test with the supported resume presentation transition and absent live projection.
- Full live chain `b4-ref10-live-window-green`: LIST1/RUN1/PASS1/FAIL0/ignored0. Original parallel HTTP/writer/rebalance → original DownloadApplication event pump → same keyed production window, with no forged progress. A real partial event contains at least2current segments, a new rebalance segment ID and at least4real range requests; origins/lengths are bounded and nonoverlapping, and written sum equals aggregate bytes below total. Actual detail disclosure retains coverage/aggregate Progress/metrics/capabilities/Server/table above the fixed footer and the original active action inside440×280. Window/view and speed-input entities stay stable. Exact final8MiB fixture bytes, HTTP connection workers reaching zero, listener closure and owned runtime drop pass. This is supplemental first-pass business coverage, not another missing-feature RED.

Commands use scratch `b4-run-rust.ps1`, exact nonzero LIST then identical RUN with `-- --nocapture --test-threads=1` and the required single-lane environment. The engine selection is `cargo test --locked --offline -p nxtools-download --test b4_coverage_contract b4_ref10_`; production/model selection is Desktop bin `b4_`.

## Limitations

Source/TestPlatform and owned HTTP/SQLite/writer evidence does not certify physical Windows IME/DPI/OS-close, human visual, installed/Release or whole-workspace acceptance. Final verification and independent acceptance are recorded in outcomes.md.
