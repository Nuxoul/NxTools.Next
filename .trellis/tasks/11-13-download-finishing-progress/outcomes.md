# Outcomes

## Delivered

- Additive event-only finishing stages distinguish actual fragment concatenation, FFmpeg muxing and final publication. Concatenation uses written bytes and real fragment lengths; muxing uses the child media clock and known duration. Unknown totals and publication are indeterminate.
- Transfer byte counters remain separate. The original100ms progress gate retains forced stage/terminal samples; task-window and taskbar presentation remain below completion until Completed.
- HLS and DASH use bounded copy loops and the original transfer control. Public pause/cancel accepts their active Finalizing state; pause retains durable fragments, cancellation removes owned staging, and partial merged output is removed before either stop settles.
- FFmpeg progress is parsed incrementally with bounded records, including split Windows terminal records. Original stderr bounds, timeout, kill/wait, reader drains, exact output and no-replace publication remain authoritative.
- Existing official Progress and Small primary/outlined actions retain the keyed task window and its placement/focus lifecycle.

## Verification

| Boundary | Result |
| --- | --- |
| Real HTTP/child/engine contracts |106 passed (`finishing-engine-contract.log`) |
| Media resolution/process/parser tests |10 passed (`finishing-media-final.log`) |
| Real HTTP HLS finishing pause/resume/cancel and exact bytes |1 passed, both stop outcomes (`finishing-hls-stop-green.log`) |
| Production GPUI finishing and taskbar projection |2 passed (`finishing-controls-green.log`) |
| Task-window lifecycle/interaction filter |29 passed (`finishing-task-windows.log`) |
| Strict Clippy, download and desktop all targets |passed (`finishing-clippy-final.log`) |
| Scoped rustfmt, source whitespace, spec contracts and download UI lifecycle |passed |
| GPUI release and pinned Markdown editor provenance gates |passed |

Logs are retained in the session scratch directory. Production-boundary regressions cover missing mux events, public HLS finishing stops and retained media controls.

## Known limitations

The independent B6 `ref13_successful_width_prevents_repeated_extra_connection_refusals` experiment retains its obsolete native-refusal-only assertion: the recorded full-library run has159 passed,1 failed and4 ignored (`finishing-download-lib.log`). The current fault-recovery contract accepts other typed extra-connection refusals; this slice does not change connection learning.

TestPlatform and source gates do not certify real HWND, pointer/DPI, visual or user acceptance. No package, installation or product launch was performed.
