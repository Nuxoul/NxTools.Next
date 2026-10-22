# R1/R2 monitor and confirmation reference / implementation record

## Verified reference before edits
- ABDM desktop `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/singleDownloadPage/ProgressDownloadPage.kt`: `RenderInfo`/`RenderPropertyItem` (529–579), expandable `RenderPartInfo`/`PartInfoCells` (332–525): part number, status, downloaded, total. Its automatic marquee is intentionally NOT adopted (approved user amendment: at most two filename lines plus full accessible name/Tooltip).
- ABDM `downloader/monitor/src/commonMain/kotlin/ir/amirab/downloader/monitor/DownloadMonitor.kt` (90–149): byte-count deltas once per second, optional five-sample display averaging; monitor collection is subscription-owned. NxTools.Next samples timestamped byte counters, NOT already-averaged engine rates; no rendering timer or render-time state mutation.
- ABDM `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/singledownloadpage/BaseSingleDownloadComponent.kt` (107–147,180–228): same task component switches completion phase, disclosure local state, original pause/resume/cancel owners; closing presentation is not an implicit transfer action.
- Legacy `D:\Code\NxTools\Views\DownloadProgressWindow.axaml.cs`: constructors (26–41), real OnOpened/OnClosed (43–60), `ShowCompletionState`/`ResizePreservingCenter` (84–136). `docs/reference-code/download.md` indexes `MainWindow.axaml.cs` keyed registry/show boundary. Keep same keyed HWND, phase/generation, centered native resize, explicit actions and presentation-only close.
- Legacy `D:\Code\NxTools\Views\AddDownloadWindow.axaml.cs`: constructor/OnOpened/OnClosed (20–40), stage/media/error notification → queued measure (73–89) → `RefreshContentSize` (91–122), bounded working-area height (124–135), center-preserving size change (137–170). Correct excess confirmation space through state/content-aware allocation, not shrinking probe/media/error states blindly.

## Released Kit 0.7.1 selection
Complete official component index read. Published sources inspected under `C:\Users\Administrator\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f\gpui-component-0.7.1\src\`:
- Monitor: `description_list.rs::DescriptionList::{horizontal,columns,label_width,bordered,item}` + `Sizable::small`, public AnyElement label/value slots: adopt two-column compact label/value rows. `table/table.rs::Table/TableHeader/TableBody/TableRow/TableHead/TableCell`: adopt stateless semantic part table; public Styled widths/minimums support narrow number column. No DataTable selection/sorting/controller is needed.
- Confirmation: `form/form.rs::Form::{horizontal,columns,label_width,child}` remains real existing compound, original InputState/picker/submission owners unchanged.
- Category: `popover.rs::Popover::{open,on_open_change,track_focus,trigger_style,content}` remains sole overlay owner. `list/list_item.rs::ListItem::render` (198–255) inserts internal h_flex/full-width children and 12px horizontal padding; caller outer flex_col does NOT control those internals. Adapt through public child slot using explicitly bounded two-line content width derived from 320px outer minus Kit padding/border and row padding; retain released ListItem paint/selection/one on_click. Existing search/arrow/highlight/scroll/Enter/Escape/return-focus stay owned by NewDownloadView.

## Required invariants / regression plan
- 440×184 collapsed and 440×280 expanded monitor clients, 34px caption/44px footer; bounded readable two-line filename, noncompeting progress/percent, downloaded/total, speed, ETA, status, resume and rate-limit/connection state. Only progress URL/destination and minimal dots are intentionally superseded; completed/failed paths remain.
- Rate/ETA are updated only in new/replace_snapshot; pause/terminal/byte rewind resets immediately. No double averaging or high-frequency diagnostics.
- Part status is honestly byte-derived because current DownloadSegmentProgress has only index/bytes/total; unknown length MUST NOT be declared complete. All rows remain reachable in one bounded vertical viewport.
- Confirmation keeps probe→fixed-target handoff/generation/cancel/defer/native centered placement. Media/FFmpeg/error reserves stay distinct.
- Red-capable production bounds/control assertions precede edits; compiled execution delegated to parent (no child Cargo/fmt/build, no package/native/Shell/service/data operations).

## Verification status
Implementation and exact deferred integration details will be appended after the source slice. No compiled, visual, native or installed acceptance is claimed here.
