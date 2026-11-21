# Native material performance comparison

## Boundary

Compare Solid and Acrylic in the same isolated GPUI executable and Debug profile, using the approved fixed-client native material proof. This is a synthetic resource comparison, not installed-product, Release, physical-input, power or full-theme acceptance. The selected whole-application theme scope remains unchanged.

The fixture uses Kit/Base/Component/Assets0.8.0 and gpui-pre0.3.9, the private UnderBorder build and Sarasa UI SC. No production dependency patch, product setting, service or real clipboard/history data is changed. See [native proof](native-material-preview.md) and the [measurement plan](../../.trellis/tasks/11-21-desktop-motion-acrylic-style/performance-plan.md).

## Workloads

| Scenario | GPUI process | Owned background |
| --- | --- | --- |
| `idle` | Workbench900×600, stable InputState, actual1000-row uniform_list; no benchmark update task | Static |
| `background` | Same Workbench | Moving synthetic gradient/grid |
| `input` | Same Workbench,16-digit synthetic InputState value;50ms target interval | Static |
| `scroll` | Same Workbench, actual virtualized scrolling;16ms target interval | Static |
| `multiwindow` | Workbench900×600, Clipboard480×540 and Search620×60 in one process | Moving synthetic gradient/grid |

Each process warms for8s, runs one finite phase and closes its own windows after a1s tail. Input/scroll work stops by task cancellation and generation retirement. Logs contain only correlation, phase and counts. The scroll evidence is the greatest actually rendered first-row index, not merely the requested scroll position. Target timer intervals are not claims of achieved update frequency.

The benchmark layout retains official Large InputGroup/Input and official ListItem content; GPUI uniform_list owns virtualization and scrolling. An explicit horizontal content child arranges icon/text. No custom input, row interaction framework or product worker is introduced.

## Resource collection

```powershell
# Requires the separately built isolated candidate, not the installed product.
.\tests\performance\native-material-benchmark.ps1 `
  -OutputDirectory "$env:PI_SCRATCH_DIR\material-runs" `
  -CandidateExecutable "$PWD\target\debug\preview-native-material\quick_launch_surface.exe" `
  -Seconds 15 -Repetitions 3
```

Use `-Seconds 2 -Repetitions 1` for a short lifecycle trial. Default scenarios run sequentially; each repetition alternates Solid/Acrylic order. The runner does not elevate itself. `-FrameControlDirectory` connects an already authorized temporary capture observer.

The C# sampler returns absolute process CPU time, private committed bytes, working set, handles, per-process GPU engine utilization and dedicated/shared GPU memory. GPU engine contexts are grouped by adapter/physical engine; the process scalar is the busiest engine, not a sum across engines. Missing observations retain status/native error and null values.

The background shares the ordinary PowerShell helper process with its sampler; report that combined role separately. The elevated observer and both PresentMon processes are also separate measurement roles. Their cost is not application cost and is not silently subtracted from DWM.

## Temporary frame capture

`tests/performance/native-material-frame-capture.ps1` requires explicit administrator authorization. Start one observer with `-ControlDirectory`, the exact `-CandidateExecutable`, the downloaded `-PresentMonExecutable` and `-ExpectedSessionId`. Confirm the Windows UAC prompt. It accepts correlated requests only for that candidate path and the selected session's DWM, using two independent PID-filtered PresentMon sessions.

PresentMon2.6.0x64 source: https://github.com/GameTechDev/PresentMon/releases/tag/v2.6.0. No installer, user-group change or persistent permission is needed. Each trace has a unique session name and a bounded duration; existing ETW sessions are not stopped. Put an empty `stop` file in the control directory after the runner completes to close the observer normally. It also has a25-minute lifetime limit.

`--no_track_input --qpc_time --v2_metrics` records frame timing without keyboard/mouse payload capture. The runner preserves CSV, process samples, phase events, native receipts and metadata. Successful trace startup is not sufficient: check exit codes, PID columns and phase-aligned rows.

## Analysis

```powershell
python tests/performance/analyze-native-material.py `
  "$env:PI_SCRATCH_DIR\material-runs" --logical-processors 12 `
  --output "$env:PI_SCRATCH_DIR\material-analysis.json"
```

Use the machine's actual logical processor count. The analyzer trims0.5s from each parent-observed phase boundary, selects complete sample intervals, and uses actual QPC duration for process CPU deltas. One-core CPU percentage and whole-machine percentage remain distinct. GPU engine averages are time-weighted; private memory/working set remain separate distributions. Paired differences are computed within each repetition before reporting median/range.

Frame metrics retain separate swapchains, including all three windows. FrameTime includes deliberate pacing. PresentMon CPUBusy describes wall time outside Present waiting, not scheduled CPU execution: event-loop/timer waits can appear there. Use process CPU deltas for CPU utilization. GPU busy, display latency and observed frame intervals have their own units and availability; idle caret frames are not a60Hz animation test.

## Checks

- `native-material-benchmark-driver-tests.ps1`: sampler structures, status/null behavior, aggregation and actual self-process reads.
- `native-material-benchmark-tests.ps1`: correlated event parsing, paired order, shared-log reading and owned multi-HWND preparation.
- `native-material-analysis-tests.py`: CPU normalization, missing values, paired deltas, QPC/PID filtering, swapchain separation and time-weighted GPU averages.
- Isolated example tests: stable input identity, actual input/scroll work, actual virtual-list rendering and retirement to static idle, alongside existing material/close tests.

## Limitations

DWM serves the entire desktop, so paired variation includes unrelated desktop work. Debug scheduling and instrumentation overhead affect the achieved workload; they are recorded rather than treated as Release behavior. A moving synthetic background measures one controlled composition load, not every wallpaper, video or monitor topology. This comparison does not establish Win10/mixed-DPI behavior, physical keyboard/IME latency, power consumption, long-duration leak freedom or performance of the full application after integration.
