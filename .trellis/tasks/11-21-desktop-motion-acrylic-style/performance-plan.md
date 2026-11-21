# Native material resource comparison

## Boundary

Measure the material increment in the isolated native GPUI proof, not the installed application. Keep the same pinned build/profile, dimensions, DPI, palette/font and synthetic content for Solid/Acrylic. Do not start the business runtime, access real clipboard/search data, package/install, change system settings or terminate user processes. Only owned candidate/background windows may be closed normally for a clean measurement.

Extend only the explicit benchmark fixture and performance tools. Existing ordinary/native previews retain their behavior. A benchmark-only1000-row uniform_list provides real virtualized scrolling; bounded numeric InputState updates provide a renderer/input-state load, not physical keyboard/IME acceptance. Three-window measurement uses one GPUI process/shared font and assets, not three independent runtimes.

## Scenarios

- Static idle: no benchmark update task; ordinary focused-input caret behavior remains.
- Changing background: own static-scene host animates behind the target; measure host CPU separately from the GPUI process and DWM.
- Input-state updates: bounded synthetic numeric value,20Hz; no OS keyboard layout or clipboard changes.
- Virtualized scroll: real1000-row GPUI uniform_list, target60Hz; log only phase transitions and final update/scroll counts.
- Multiple windows: Workbench900x600, Clipboard480x540 and Search620x60 in the same process, non-overlapping within primary work area; changing-background load.

Use three paired repetitions with alternating Solid/Acrylic order. Each owned process has an8s warmup, a15s measured phase and a1s normal-close tail. Poll counters and correlated phase logs every250ms; trim0.5s from each observed phase boundary for analysis. Preserve target duration, observed boundary duration, actual inner CPU interval and sample counts separately. Run sequentially with a single build writer and one foreground benchmark target. No unrelated windows are closed or moved.

## Measurements

- Absolute CPU time deltas, private committed bytes, working set and handle count for GPUI, benchmark/helper host and the same-session DWM process.
- CPU conventions: one-core percentage and whole-machine percentage using recorded logical processor count. Private bytes and working set remain separate.
- GPU process memory: dedicated/shared bytes where available. GPU engine values: maximum physical-engine utilization perprocess/sample after grouping duplicate contexts, not an invalid sum across different engines.
- PID-filtered PresentMon traces where supported; preserve raw CSV in scratch. Use absolute QPC boundaries to assign frames to phases. Report deliberate pacing separately from busy/API/GPU durations;50ms input update intervals are not50ms stalls, and idle presents are not a60Hz workload.
- Benchmark phase/update/visible-row evidence must establish that input/scroll work actually ran. Missing counters/trace data are unavailable, never zero or a pass.
- One explicitly authorized temporary elevated observer runs separate, uniquely named PID-filtered PresentMon sessions for the candidate and same-session DWM. It also samples DWM and records observer/PresentMon overhead. No permanent permission, user-group, service or system-setting changes.
- Preserve swapchain separation, especially for the three-window process. PresentMon CPUBusy is a frame wall-time definition, not scheduled CPU utilization; event-loop waiting outside Present can appear in that field. FrameTime includes deliberate pacing. Use process CPU deltas for CPU utilization and report GPU busy/display observations independently.
- Each dynamic workload records its update count; virtualized scrolling additionally records the greatest actually rendered first-row index. The public logical_scroll_top_index can remain0 for programmatic strict scrolling and is not sufficient movement evidence.

DWM is shared desktop work. Match area/load and record paired variance, but do not attribute unrelated desktop activity to the material. These Debug fixture deltas are directional; full-product and Release acceptance follow integration. Power is unmeasured without a suitable controlled energy source.

## Decision

Use paired deltas, medians and observed ranges, not a universal score or arbitrary MB/% threshold. Investigate repeatable idle work, growth after close/reopen or sustained frame/workload regressions before product adoption. Optimize within the approved whole-app theme scope; do not silently shrink it to selected panels.
